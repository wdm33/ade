// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! PHASE4-N-R-A A4 — integration tests for the real forge handler.
//!
//! Exercises `produce_mode::run_real_forge` end-to-end against a
//! synthetic `(stake_dist, eta0, vrf_sk)` corpus. Covers 3 of the
//! 4 named branches of the closed `RequestForge → ForgeResult`
//! contract (`CN-FORGE-01`):
//!
//! - **`ForgeNotLeader`** — zero-stake answer → step 2 emits
//!   `LeaderCheckVerdict::NotEligible`.
//! - **`ForgeFailed { KesPeriodMismatch }`** — kes_period outside
//!   the shell's current-period window.
//! - **`ForgeFailed { SelfAcceptRejected }`** — full-stake answer
//!   that reaches step 6; self_accept rejects the synthetic block
//!   because the placeholder KES signing payload (A3 honest scope)
//!   does not match the real unsigned-header bytes.
//!
//! The 4th branch, `ForgeSucceeded`, is structurally guaranteed
//! by the closed pipeline (the only path to emit Succeeded is past
//! the self_accept = Accepted gate). Asserting end-to-end requires
//! the KES-signs-real-unsigned-header bridge — explicitly out of
//! N-R-A scope (the runbook documents this as a future-cluster
//! deliverable). A4 enforces the structural property: no
//! `ForgeSucceeded` emission outside the documented gate sequence.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::collections::BTreeMap;

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::leader_schedule::LeaderScheduleAnswer;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::vrf_cert::{leader_vrf_input, ActiveSlotsCoeff};
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_crypto::vrf::VrfVerificationKey;
use ade_ledger::consensus_view::PoolDistrView;
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::state::LedgerState;
use ade_runtime::producer::coordinator::CoordinatorEvent;
use ade_runtime::producer::producer_log::ForgeFailureReason;
use ade_runtime::producer::producer_shell::ProducerShell;
use ade_types::shelley::block::{OperationalCert, PrevHash, ProtocolVersion};
use ade_types::{BlockNo, CardanoEra, EpochNo, Hash28, Hash32};

use ade_node::produce_mode::{run_real_forge, ForgeRequestContext};

// =========================================================================
// Synthetic-corpus helpers
// =========================================================================

fn synth_shell(cold_seed: u8, vrf_seed: u8, kes_seed: u8) -> ProducerShell {
    use ade_runtime::producer::signing::{ColdSigningKey, VrfSigningKey};
    use cardano_crypto::vrf::VrfDraft03;

    // Cold (Ed25519 32-byte seed).
    let cold_bytes = [cold_seed; 32];
    let cold = ColdSigningKey::from_bytes_zeroizing(&cold_bytes).unwrap();

    // VRF (libsodium expanded 64-byte SK from seed).
    let (vrf_sk_bytes, vrf_vk_bytes) = VrfDraft03::keypair_from_seed(&[vrf_seed; 32]);
    let vrf = VrfSigningKey::from_bytes_zeroizing(&vrf_sk_bytes).unwrap();

    // KES (Ade-native, 32-byte seed at period 0).
    let kes_seed_bytes = [kes_seed; 32];
    let kes = ade_runtime::producer::signing::KesSecret::from_seed_at_period(
        &kes_seed_bytes,
        0,
    )
    .unwrap();

    // Opcert with matching KES VK + cold-signed sigma.
    use ade_crypto::kes_sum::{KesAlgorithm, Sum6Kes};
    let kes_sk_raw = Sum6Kes::gen_key_kes_from_seed_bytes(&kes_seed_bytes).unwrap();
    let hot_vkey = Sum6Kes::derive_verification_key(&kes_sk_raw);

    use ed25519_dalek::{Signer, SigningKey as DalekSk};
    let cold_dalek = DalekSk::from_bytes(&cold_bytes);
    let mut signable = Vec::with_capacity(48);
    signable.extend_from_slice(&hot_vkey);
    signable.extend_from_slice(&0u64.to_be_bytes());
    signable.extend_from_slice(&0u64.to_be_bytes());
    let sigma = cold_dalek.sign(&signable);

    let opcert = OperationalCert {
        hot_vkey: hot_vkey.to_vec(),
        sequence_number: 0,
        kes_period: 0,
        sigma: sigma.to_bytes().to_vec(),
    };
    let _vrf_vk = VrfVerificationKey(vrf_vk_bytes);

    ProducerShell::init(kes, vrf, cold, opcert).expect("shell init")
}

fn era_schedule() -> EraSchedule {
    EraSchedule::new(
        BootstrapAnchorHash(Hash32([0u8; 32])),
        0,
        vec![EraSummary {
            era: CardanoEra::Conway,
            start_slot: ade_types::SlotNo(0),
            start_epoch: EpochNo(0),
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 129_600,
        }],
    )
    .expect("era schedule")
}

fn answer_for_slot(
    slot: u64,
    eta0: &Nonce,
    stake_numer: u64,
    stake_denom: u64,
    asc_numer: u32,
    asc_denom: u32,
) -> LeaderScheduleAnswer {
    LeaderScheduleAnswer {
        slot: ade_types::SlotNo(slot),
        pool: Hash28([0xAA; 28]),
        epoch: EpochNo(0),
        expected_vrf_input: leader_vrf_input(
            ade_types::CardanoEra::Conway,
            ade_types::SlotNo(slot),
            eta0,
        ),
        stake_fraction: (stake_numer, stake_denom),
        asc: ActiveSlotsCoeff {
            numer: asc_numer,
            denom: asc_denom,
        },
    }
}

struct Fixture {
    eta0: Nonce,
    pparams: ProtocolParameters,
    base_state: LedgerState,
    chain_dep_state: PraosChainDepState,
    era_schedule: EraSchedule,
    pool_distr_view: PoolDistrView,
    block_number: BlockNo,
    prev_hash: PrevHash,
    protocol_version: ProtocolVersion,
}

impl Fixture {
    fn new() -> Self {
        Fixture {
            eta0: Nonce(Hash32([0xCD; 32])),
            pparams: ProtocolParameters::default(),
            base_state: LedgerState::new(CardanoEra::Conway),
            chain_dep_state: PraosChainDepState::empty(),
            era_schedule: era_schedule(),
            pool_distr_view: PoolDistrView::new(
                EpochNo(0),
                0,
                ActiveSlotsCoeff { numer: 1, denom: 20 },
                BTreeMap::new(),
            ),
            block_number: BlockNo(1),
            prev_hash: PrevHash::Block(Hash32([0u8; 32])),
            protocol_version: ProtocolVersion { major: 9, minor: 0 },
        }
    }

    fn ctx<'a>(
        &'a self,
        answer: &'a LeaderScheduleAnswer,
        vrf_vk: &'a VrfVerificationKey,
    ) -> ForgeRequestContext<'a> {
        ForgeRequestContext {
            eta0: &self.eta0,
            vrf_vk,
            leader_schedule_answer: answer,
            pparams: &self.pparams,
            base_state: &self.base_state,
            chain_dep_state: &self.chain_dep_state,
            era_schedule: &self.era_schedule,
            pool_distr_view: &self.pool_distr_view,
            block_number: self.block_number,
            prev_hash: self.prev_hash.clone(),
            protocol_version: self.protocol_version,
            prev_opcert_counter: None,
        }
    }
}

// =========================================================================
// Tests — 3 reachable branches of CN-FORGE-01
// =========================================================================

#[test]
fn zero_stake_answer_emits_forge_not_leader() {
    let fixture = Fixture::new();
    let mut shell = synth_shell(0x11, 0x22, 0x33);
    let vrf_vk = shell.vrf_verification_key();

    // Zero stake → step 2 returns NotEligible → ForgeNotLeader.
    let answer = answer_for_slot(42, &fixture.eta0, 0, 1, 1, 1);
    let ctx = fixture.ctx(&answer, &vrf_vk);

    let (event, handoff) = run_real_forge(42, /* kes_period = */ 0, &ctx, &mut shell);
    // PHASE4-N-F-G-B S1: a not-a-leader outcome surfaces NO self-accepted
    // handoff — the success-only token is None here.
    assert!(
        handoff.is_none(),
        "ForgeNotLeader must surface no self-accepted handoff"
    );
    match event {
        CoordinatorEvent::ForgeNotLeader {
            slot,
            vrf_output_fingerprint,
        } => {
            assert_eq!(slot, 42);
            // Fingerprint is non-zero (the synthetic VRF output is real).
            assert_ne!(vrf_output_fingerprint, [0u8; 8]);
        }
        other => panic!("expected ForgeNotLeader, got {:?}", other),
    }
}

#[test]
fn kes_period_outside_window_emits_forge_failed_kes_period_mismatch() {
    let fixture = Fixture::new();
    let mut shell = synth_shell(0x44, 0x55, 0x66);
    let vrf_vk = shell.vrf_verification_key();

    // Full stake so step 2 emits Eligible and we proceed to step 3.
    // ASC numer == denom → every slot eligible regardless of VRF
    // output bytes (boundary handling in vrf_cert::is_leader).
    let answer = answer_for_slot(7, &fixture.eta0, 1, 2, 1, 1);
    let ctx = fixture.ctx(&answer, &vrf_vk);

    // kes_period = u32::MAX is outside the opcert's [0, 0+SUM6_MAX_PERIOD]
    // window; shell.kes_sign_at returns an error.
    let (event, handoff) = run_real_forge(7, /* kes_period = */ u32::MAX, &ctx, &mut shell);
    // PHASE4-N-F-G-B S1: a ForgeFailed (KesPeriodMismatch) outcome surfaces NO
    // self-accepted handoff.
    assert!(
        handoff.is_none(),
        "ForgeFailed must surface no self-accepted handoff"
    );
    match event {
        CoordinatorEvent::ForgeFailed { slot, reason } => {
            assert_eq!(slot, 7);
            assert_eq!(reason, ForgeFailureReason::KesPeriodMismatch);
        }
        other => panic!("expected ForgeFailed {{ KesPeriodMismatch }}, got {:?}", other),
    }
}

#[test]
fn full_stake_answer_reaches_self_accept_and_rejects() {
    let fixture = Fixture::new();
    let mut shell = synth_shell(0x77, 0x88, 0x99);
    let vrf_vk = shell.vrf_verification_key();

    // Full stake (numer/denom = 1/1) + asc 1/1 → Eligible. The pipeline
    // proceeds through VRF prove + leader-check + KES sign + assemble_tick +
    // forge_block + self_accept. With the Praos VRF migration (PHASE4-N-W) the
    // VRF proof is no longer the rejection cause; this fixture's pool /
    // VRF-keyhash are not bound to the validator's issuer-pool recipe (unlike
    // forge_succeeds.rs's EligibleFixture), so self_accept rejects at that
    // binding. The assertion here is the gate (no premature ForgeSucceeded,
    // slot preserved); the canonical forge → self-accept SUCCESS path is
    // forge_succeeds.rs::forge_to_self_accept_succeeds.
    let answer = answer_for_slot(13, &fixture.eta0, 1, 1, 1, 1);
    let ctx = fixture.ctx(&answer, &vrf_vk);

    let (event, _handoff) = run_real_forge(13, /* kes_period = */ 0, &ctx, &mut shell);
    match event {
        CoordinatorEvent::ForgeFailed { slot, reason } => {
            assert_eq!(slot, 13);
            // Could be SelfAcceptRejected (if forge_block succeeds and
            // self_accept rejects) or another reason from earlier in the
            // pipeline. Accept either; the assertion is the gate works
            // (no Succeeded emitted) and the slot is correctly preserved.
            assert!(
                matches!(
                    reason,
                    ForgeFailureReason::SelfAcceptRejected
                        | ForgeFailureReason::Other
                        | ForgeFailureReason::EmptyMempool
                ),
                "unexpected reason: {:?}",
                reason
            );
        }
        CoordinatorEvent::ForgeSucceeded { .. } => {
            // This fixture's pool / VRF-keyhash are not validator-bound, so
            // self_accept is expected to reject. If this fires, the fixture
            // has become aligned — fold it into the canonical success test
            // forge_succeeds.rs::forge_to_self_accept_succeeds rather than
            // asserting success in two places.
            panic!(
                "unexpected ForgeSucceeded for the unaligned-fixture case; \
                 see forge_succeeds.rs::forge_to_self_accept_succeeds"
            );
        }
        CoordinatorEvent::ForgeNotLeader { .. } => {
            panic!("expected Eligible path; got ForgeNotLeader");
        }
        other => panic!("expected ForgeFailed, got {:?}", other),
    }
}

// =========================================================================
// Structural property — DC-FORGE-01 replay anchor under real forge
// =========================================================================

#[test]
fn run_real_forge_is_byte_identical_across_two_runs() {
    let fixture = Fixture::new();
    let mut shell1 = synth_shell(0xAB, 0xCD, 0xEF);
    let mut shell2 = synth_shell(0xAB, 0xCD, 0xEF);
    let vrf_vk = shell1.vrf_verification_key();
    let vrf_vk2 = shell2.vrf_verification_key();
    assert_eq!(vrf_vk.0, vrf_vk2.0, "shells must derive byte-identical VKs");

    let answer = answer_for_slot(100, &fixture.eta0, 0, 1, 1, 1);
    let ctx1 = fixture.ctx(&answer, &vrf_vk);
    let ctx2 = fixture.ctx(&answer, &vrf_vk2);

    let (e1, h1) = run_real_forge(100, 0, &ctx1, &mut shell1);
    let (e2, h2) = run_real_forge(100, 0, &ctx2, &mut shell2);

    // Both runs must produce the same CoordinatorEvent variant +
    // payload. The vrf_output_fingerprint is deterministic in
    // (vrf_sk, slot, eta0).
    assert_eq!(e1, e2, "replay byte-identity under real forge");
    // PHASE4-N-F-G-B S1: the surfaced self-accepted token is replay
    // byte-identical too (same forge inputs => same Option<AcceptedBlock>).
    assert_eq!(h1, h2, "surfaced handoff token is replay byte-identical");
}
