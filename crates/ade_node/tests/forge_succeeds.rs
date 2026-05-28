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
//! ## OQ4 honest fallback (HARD RULE invoked)
//!
//! With the setup fully aligned, the forge does NOT yet reach
//! `ForgeSucceeded`. `self_accept` rejects with
//! `BlockValidityError::Header(VrfCert(VerificationFailed))` at header
//! validation step 6 (the VRF proof check). The cause is a producer-side
//! BLUE protocol mismatch, NOT a test-setup error:
//!
//! - The forge / leader-check / leader-schedule authorities build the VRF
//!   proof over the **TPraos role-tagged** input
//!   `vrf_input(slot, eta0, LeaderEligibility)` =
//!   `slot_be8 ‖ eta0_32 ‖ 0x4C` (see `run_real_forge` step 1,
//!   `verify_and_evaluate_leader`, `query_leader_schedule`,
//!   `LeaderScheduleAnswer.expected_vrf_input`).
//! - Conway is **Praos**: `validate_and_apply_header` verifies the single
//!   combined proof over `praos_vrf_input(slot, eta0)` =
//!   `blake2b256(slot_be8 ‖ eta0_32)` and derives the leader value via the
//!   `vrfLeaderValue` range-extension (`header_validate.rs` step 6,
//!   `verify_praos_vrf`).
//!
//! The two alphas differ, so `VrfDraft03::verify` rejects the proof. No
//! value the test controls can reconcile them — the producer's VRF
//! construction for a Conway block is TPraos-shaped where the validator's
//! is Praos-shaped. Fixing it is coordinated surgery across the
//! producer-side leader-eligibility authorities (forge VRF prove,
//! `verify_and_evaluate_leader`, `query_leader_schedule`, and the
//! `LeaderScheduleAnswer.expected_vrf_input` contract) — real forge
//! authority work beyond an in-slice correction. Per the S3 HARD RULE the
//! success is NOT faked and the assertion is NOT loosened: the test pins
//! the honest current outcome and the blocker is reported for re-scope.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::collections::BTreeMap;

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::leader_schedule::LeaderScheduleAnswer;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::vrf_cert::{vrf_input, ActiveSlotsCoeff, VrfRole};
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
            expected_vrf_input: vrf_input(
                ade_types::SlotNo(slot),
                &eta0,
                VrfRole::LeaderEligibility,
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

/// HONEST-FALLBACK NAME: this is NOT yet a `ForgeSucceeded`. The
/// consistent eligible-leader tick reaches header validation step 6 and is
/// rejected at the VRF proof check because the producer builds a TPraos
/// role-tagged VRF proof while Conway header validation verifies a Praos
/// combined proof. See the module doc for the full root cause. When the
/// producer-side leader-eligibility VRF construction is migrated to Praos
/// (a follow-on cluster), this test must be promoted to assert
/// `ForgeSucceeded` (the `forge_to_self_accept_succeeds` shape).
#[test]
fn forge_to_self_accept_blocked_on_praos_vrf_construction() {
    let epoch = EpochNo(0);
    let slot = 1u64;

    let mut shell = synth_shell(0x77, 0x88, 0x99);
    let fixture = EligibleFixture::build(&shell, slot, epoch);
    let ctx = fixture.ctx();

    let event = run_real_forge(slot, /* kes_period = */ 0, &ctx, &mut shell);
    match event {
        CoordinatorEvent::ForgeFailed { slot: s, reason } => {
            assert_eq!(s, slot, "ForgeFailed must preserve the slot");
            // The block decodes (S2 envelope), the leader claim is
            // eligible, the KES sig is real (N-S-A), and self_accept is
            // reached — it rejects at the VRF proof step. `run_real_forge`
            // maps a self_accept rejection to `SelfAcceptRejected`.
            assert_eq!(
                reason,
                ForgeFailureReason::SelfAcceptRejected,
                "expected the self_accept rejection path (VRF-proof mismatch), \
                 not an earlier-pipeline failure",
            );
        }
        CoordinatorEvent::ForgeSucceeded { .. } => {
            // If this fires, the producer-side Praos VRF construction has
            // landed: promote this test to assert ForgeSucceeded and flip
            // CN-FORGE-01 / CE-V-6 to the success outcome.
            panic!(
                "ForgeSucceeded reached — producer-side Praos VRF construction \
                 has landed; promote this test to the success assertion (CE-V-6)"
            );
        }
        CoordinatorEvent::ForgeNotLeader { .. } => {
            panic!("expected the eligible path; got ForgeNotLeader (setup bug)");
        }
        other => panic!("expected ForgeFailed {{ SelfAcceptRejected }}, got {:?}", other),
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
