// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! PHASE4-N-V S3 — first in-process forge → self_accept drive.
//!
//! `run_real_forge` is driven by a **consistent eligible-leader** tick:
//! the operator's pool is registered in `pool_distr_view` with the exact
//! recipes Conway header validation binds — `pool_id =
//! blake2b_224(cold_vk)` (the issuer-pool derivation at
//! `header_input.rs`), `vrf_keyhash = blake2b_256(vrf_vk)` (the VRF
//! keyhash binding at `header_validate.rs` step 5) — at positive stake
//! with ASC 1/1, and the eta0 nonce is shared between the forge VRF input
//! and validation. S2's envelope wrap means the forged bytes now decode,
//! so the tick reaches `self_accept`'s header validation.
//!
//! ## PHASE4-N-W S2: producer Praos VRF now matches the validator
//!
//! With the producer-side leader-eligibility VRF migrated to Praos
//! (CN-FORGE-04), the aligned tick now reaches `ForgeSucceeded`. The
//! producer builds its leader VRF proof over the **Praos** input
//! `praos_vrf_input(slot, eta0) = blake2b256(slot_be8 ‖ eta0_32)` (sourced
//! from `LeaderScheduleAnswer.expected_vrf_input`, the single
//! `leader_vrf_input` authority) and evaluates eligibility from
//! `praos_leader_value(output)`. `validate_and_apply_header` then verifies
//! the single combined proof over the **same** `praos_vrf_input` via
//! `verify_praos_vrf` (`header_validate.rs` `HeaderVrf::Praos` branch), so
//! `self_accept` accepts the forged Conway block — producer self-accept ≡
//! receive-path verification (R2). This is the first in-process forge →
//! self-accept success.
//!
//! Before N-W the producer built the **TPraos** role-tagged input
//! `slot ‖ eta0 ‖ 0x4C`, which the Praos validator rejected at the VRF
//! proof step; N-W removed that producer/validator asymmetry.

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
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::state::LedgerState;
use ade_runtime::producer::coordinator::CoordinatorEvent;
use ade_runtime::producer::producer_log::ForgeFailureReason;
use ade_runtime::producer::producer_shell::ProducerShell;
use ade_types::shelley::block::{OperationalCert, ProtocolVersion};
use ade_types::{BlockNo, CardanoEra, EpochNo, Hash28, Hash32};

use ade_node::produce_mode::{run_real_forge, ForgeRequestContext};

// =========================================================================
// Synthetic-corpus helpers (mirror forge_handler_variants::synth_shell)
// =========================================================================

fn synth_shell(cold_seed: u8, vrf_seed: u8, kes_seed: u8) -> ProducerShell {
    use ade_runtime::producer::signing::{ColdSigningKey, VrfSigningKey};
    use cardano_crypto::vrf::VrfDraft03;

    let cold_bytes = [cold_seed; 32];
    let cold = ColdSigningKey::from_bytes_zeroizing(&cold_bytes).unwrap();

    let (vrf_sk_bytes, vrf_vk_bytes) = VrfDraft03::keypair_from_seed(&[vrf_seed; 32]);
    let vrf = VrfSigningKey::from_bytes_zeroizing(&vrf_sk_bytes).unwrap();

    let kes_seed_bytes = [kes_seed; 32];
    let kes = ade_runtime::producer::signing::KesSecret::from_seed_at_period(&kes_seed_bytes, 0)
        .unwrap();

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

fn era_schedule(epoch: EpochNo) -> EraSchedule {
    EraSchedule::new(
        BootstrapAnchorHash(Hash32([0u8; 32])),
        0,
        vec![EraSummary {
            era: CardanoEra::Conway,
            start_slot: ade_types::SlotNo(0),
            start_epoch: epoch,
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 129_600,
        }],
    )
    .expect("era schedule")
}

/// Build the consistent eligible-leader `ForgeRequestContext` plus its
/// owned backing values. The returned tuple keeps every borrowed input
/// alive for the duration of the `run_real_forge` call.
struct EligibleFixture {
    eta0_holder: PraosChainDepState,
    vrf_vk: VrfVerificationKey,
    leader_answer: LeaderScheduleAnswer,
    pparams: ProtocolParameters,
    base_state: LedgerState,
    era_schedule: EraSchedule,
    pool_distr_view: PoolDistrView,
}

impl EligibleFixture {
    fn build(shell: &ProducerShell, slot: u64, epoch: EpochNo) -> Self {
        let eta0 = Nonce(Hash32([0xCD; 32]));
        let vrf_vk = shell.vrf_verification_key();
        let cold_vk = shell.cold_vk();

        // pool_id = blake2b_224(cold_vk): the issuer pool the header
        // validator derives from the issuer vkey (header_input.rs).
        let pool_id: Hash28 = ade_crypto::blake2b::blake2b_224(&cold_vk.0);
        // vrf_keyhash = blake2b_256(vrf_vk): the recipe header validation
        // binds at step 5 (header_validate.rs VRF keyhash binding).
        let vrf_keyhash: Hash32 = ade_crypto::blake2b::blake2b_256(&vrf_vk.0);

        // total = 1 + ASC 1/1 ⇒ always eligible regardless of VRF output.
        let total: u64 = 1;
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        pools.insert(
            pool_id.clone(),
            PoolEntry {
                active_stake: total,
                vrf_keyhash,
            },
        );
        let pool_distr_view =
            PoolDistrView::new(epoch, total, ActiveSlotsCoeff { numer: 1, denom: 1 }, pools);

        let leader_answer = LeaderScheduleAnswer {
            slot: ade_types::SlotNo(slot),
            pool: pool_id,
            epoch,
            expected_vrf_input: leader_vrf_input(
                CardanoEra::Conway,
                ade_types::SlotNo(slot),
                &eta0,
            ),
            stake_fraction: (1, 1),
            asc: ActiveSlotsCoeff { numer: 1, denom: 1 },
        };

        // The same nonce feeds both the forge VRF input and validation.
        let mut eta0_holder = PraosChainDepState::empty();
        eta0_holder.epoch_nonce = eta0;

        let mut base_state = LedgerState::new(CardanoEra::Conway);
        base_state.epoch_state.epoch = epoch;

        EligibleFixture {
            eta0_holder,
            vrf_vk,
            leader_answer,
            pparams: ProtocolParameters::default(),
            base_state,
            era_schedule: era_schedule(epoch),
            pool_distr_view,
        }
    }

    fn ctx(&self) -> ForgeRequestContext<'_> {
        ForgeRequestContext {
            eta0: &self.eta0_holder.epoch_nonce,
            vrf_vk: &self.vrf_vk,
            leader_schedule_answer: &self.leader_answer,
            pparams: &self.pparams,
            base_state: &self.base_state,
            chain_dep_state: &self.eta0_holder,
            era_schedule: &self.era_schedule,
            pool_distr_view: &self.pool_distr_view,
            block_number: BlockNo(0),
            prev_hash: Hash32([0u8; 32]),
            protocol_version: ProtocolVersion { major: 9, minor: 0 },
            prev_opcert_counter: None,
        }
    }
}

// =========================================================================
// CE-V-6 — OQ4 honest fallback: ForgeFailed with the next blocker pinned
// =========================================================================

/// PHASE4-N-W S2: the producer-side Praos VRF construction now matches the
/// validator (CN-FORGE-04). The consistent eligible-leader tick forges a
/// Conway/Praos block whose single combined VRF certificate the SAME
/// receive-path `verify_praos_vrf` accepts, so `self_accept` reaches
/// `ForgeSucceeded` — the first in-process forge → self-accept success
/// (CE-W-3, CE-W-4, CE-W-5; closes the deferred CN-FORGE-01 "ForgeSucceeded
/// reachable" strengthening).
#[test]
fn forge_to_self_accept_succeeds() {
    let epoch = EpochNo(0);
    let slot = 1u64;

    let mut shell = synth_shell(0x77, 0x88, 0x99);
    let fixture = EligibleFixture::build(&shell, slot, epoch);
    let ctx = fixture.ctx();

    let event = run_real_forge(slot, /* kes_period = */ 0, &ctx, &mut shell);
    match event {
        CoordinatorEvent::ForgeSucceeded { slot: s, .. } => {
            assert_eq!(s, slot, "ForgeSucceeded must preserve the slot");
        }
        CoordinatorEvent::ForgeFailed { slot: _, reason } => {
            panic!(
                "expected ForgeSucceeded — the producer Praos VRF now matches the \
                 validator's verify_praos_vrf; got ForgeFailed {{ {:?} }}. If a \
                 deeper producer/validator asymmetry has surfaced, pin it as a \
                 follow-on cluster (N-X) per the CE-W-5 contingency.",
                reason
            );
        }
        CoordinatorEvent::ForgeNotLeader { .. } => {
            panic!("expected the eligible path; got ForgeNotLeader (setup bug)");
        }
        other => panic!("expected ForgeSucceeded, got {:?}", other),
    }
}

// =========================================================================
// CE-W-6 (PHASE4-N-W S1) — TPraos producer-forge fail-closed
// =========================================================================

/// A producer-forge request whose era schedule locates a non-Praos era
/// (Shelley = TPraos) must fail closed with the structured
/// `ForgeFailureReason::UnsupportedProducerEra` — the sketch's
/// `UnsupportedEra::ProducerForge` policy (I6 / N5). The guard fires before
/// any VRF/KES key use. TPraos *validation* is unaffected (this slice does
/// not touch `vrf_input` / `VrfRole`).
#[test]
fn tpraos_producer_forge_fails_closed_with_unsupported_era() {
    let epoch = EpochNo(0);
    let slot = 1u64;

    let mut shell = synth_shell(0x77, 0x88, 0x99);
    let fixture = EligibleFixture::build(&shell, slot, epoch);

    // A Shelley (TPraos) era schedule located at the forge slot.
    let shelley_schedule = EraSchedule::new(
        BootstrapAnchorHash(Hash32([0u8; 32])),
        0,
        vec![EraSummary {
            era: CardanoEra::Shelley,
            start_slot: ade_types::SlotNo(0),
            start_epoch: epoch,
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 129_600,
        }],
    )
    .expect("shelley era schedule");

    let ctx = ForgeRequestContext {
        eta0: &fixture.eta0_holder.epoch_nonce,
        vrf_vk: &fixture.vrf_vk,
        leader_schedule_answer: &fixture.leader_answer,
        pparams: &fixture.pparams,
        base_state: &fixture.base_state,
        chain_dep_state: &fixture.eta0_holder,
        era_schedule: &shelley_schedule,
        pool_distr_view: &fixture.pool_distr_view,
        block_number: BlockNo(0),
        prev_hash: Hash32([0u8; 32]),
        protocol_version: ProtocolVersion { major: 9, minor: 0 },
        prev_opcert_counter: None,
    };

    let event = run_real_forge(slot, /* kes_period = */ 0, &ctx, &mut shell);
    match event {
        CoordinatorEvent::ForgeFailed { slot: s, reason } => {
            assert_eq!(s, slot, "ForgeFailed must preserve the slot");
            assert_eq!(
                reason,
                ForgeFailureReason::UnsupportedProducerEra,
                "a non-Praos (Shelley) producer-forge request must fail closed \
                 with UnsupportedProducerEra, not attempt a forge",
            );
        }
        other => panic!("expected ForgeFailed {{ UnsupportedProducerEra }}, got {:?}", other),
    }
}
