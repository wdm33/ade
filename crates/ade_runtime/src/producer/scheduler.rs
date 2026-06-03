// Core Contract:
// - Deterministic: same (state, input, ledger_view) => same (state', effects)
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types; closed sums on every surface
// - Pure RED state transition; wall-clock and I/O live in the outer driver

//! RED scheduler core (PHASE4-N-C S6).
//!
//! `scheduler_step` is a pure value transition mirroring N-B's
//! `process_stream_input` shape. The outer driver (the binary layer)
//! feeds a slot number from a wall-clock source; this function consumes
//! that value and produces a new state plus a closed list of effects.
//!
//! Determinism: identical `(state, input, ledger_view)` -> identical
//! `(state', effects)`. The scheduler does not call into RED signing
//! primitives — those have already been invoked to produce the
//! `TickInputs` carried inside `SchedulerInput::SlotTick`.

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_ledger::mempool::admit::MempoolState;
use ade_ledger::producer::forge::{forge_block, ForgeError, ForgeEffects};
use ade_ledger::producer::{self_accept, AcceptedBlock, SelfAcceptError};
use ade_ledger::state::LedgerState;

use crate::producer::tick_assembler::{assemble_tick, TickAssemblyError, TickInputs};

/// Closed scheduler input. RED only — never crosses into BLUE.
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulerInput {
    /// A slot has elapsed; the producer attempts forging at this slot.
    SlotTick { slot: u64, inputs: TickInputs },
    /// The chain selector advanced; refresh the ledger / chain_dep /
    /// mempool baseline that the next SlotTick starts from.
    ChainAdvanced {
        ledger: LedgerState,
        chain_dep: PraosChainDepState,
        mempool: MempoolState,
    },
}

/// Closed scheduler effect. RED dispatches.
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulerEffect {
    /// Forge succeeded and self-accepted; queue for broadcast.
    EnqueueBroadcast(AcceptedBlock),
    /// Slot was non-leader; the producer is silent. Observable for tests.
    SilentNonLeader { slot: u64 },
    /// Forge or self-accept failed; the scheduler halts at this slot.
    HaltOnInvariant { slot: u64, reason: SchedulerHaltReason },
    /// Tick assembly produced a structurally inconsistent tick. Defensive.
    HaltOnAssembly { slot: u64, reason: TickAssemblyError },
}

/// Closed halt-reason sum.
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulerHaltReason {
    Forge(ForgeError),
    SelfAccept(SelfAcceptError),
}

/// Closed scheduler state.
#[derive(Debug, Clone, PartialEq)]
pub struct SchedulerState {
    pub ledger: LedgerState,
    pub chain_dep: PraosChainDepState,
    pub mempool: MempoolState,
    pub era_schedule: EraSchedule,
    pub last_seen_slot: Option<u64>,
    pub prev_opcert_counter: Option<u64>,
    /// Once `Some`, subsequent SlotTick inputs are ignored: the original
    /// halt reason is re-emitted via `SchedulerEffect::HaltOnInvariant`.
    pub halted: Option<SchedulerHaltReason>,
}

/// Pure RED state transition. No I/O, no clock — the outer driver feeds
/// the slot number from a wall-clock source; this function consumes
/// that value. Determinism: identical `(state, input)` -> identical
/// `(state', effects)`.
pub fn scheduler_step<L: LedgerView>(
    state: SchedulerState,
    input: SchedulerInput,
    ledger_view: &L,
) -> (SchedulerState, Vec<SchedulerEffect>) {
    match input {
        SchedulerInput::ChainAdvanced {
            ledger,
            chain_dep,
            mempool,
        } => {
            let mut next = state;
            next.ledger = ledger;
            next.chain_dep = chain_dep;
            next.mempool = mempool;
            (next, Vec::new())
        }
        SchedulerInput::SlotTick { slot, inputs } => slot_tick(state, slot, inputs, ledger_view),
    }
}

fn slot_tick<L: LedgerView>(
    state: SchedulerState,
    slot: u64,
    inputs: TickInputs,
    ledger_view: &L,
) -> (SchedulerState, Vec<SchedulerEffect>) {
    // Once halted, re-emit the original halt reason and ignore the tick.
    if let Some(reason) = state.halted.clone() {
        return (state, vec![SchedulerEffect::HaltOnInvariant { slot, reason }]);
    }

    let mut next = state;
    next.last_seen_slot = Some(slot);

    // 1. Assemble the canonical ProducerTick.
    let tick = match assemble_tick(slot, &next.ledger, &next.mempool, &inputs) {
        Ok(t) => t,
        Err(reason) => {
            return (
                next,
                vec![SchedulerEffect::HaltOnAssembly { slot, reason }],
            );
        }
    };

    // 2. Forge. NotLeader is the canonical non-leader silence path; any
    //    other forge error halts the scheduler.
    let (forged, effects) = match forge_block(&tick) {
        Ok(pair) => pair,
        Err(ForgeError::NotLeader { slot: _ }) => {
            return (next, vec![SchedulerEffect::SilentNonLeader { slot }]);
        }
        Err(other) => {
            let reason = SchedulerHaltReason::Forge(other);
            next.halted = Some(reason.clone());
            return (
                next,
                vec![SchedulerEffect::HaltOnInvariant { slot, reason }],
            );
        }
    };

    // 3. Self-accept. Any rejection halts the scheduler.
    let accepted = match self_accept(
        &forged.bytes,
        &next.ledger,
        &next.chain_dep,
        &next.era_schedule,
        ledger_view,
    ) {
        Ok(a) => a,
        Err(e) => {
            let reason = SchedulerHaltReason::SelfAccept(e);
            next.halted = Some(reason.clone());
            return (
                next,
                vec![SchedulerEffect::HaltOnInvariant { slot, reason }],
            );
        }
    };

    // 4. Pull the opcert counter advance off the forge effects.
    for eff in &effects {
        match eff {
            ForgeEffects::ReadyForSelfAccept {
                next_prev_opcert_counter,
            } => {
                next.prev_opcert_counter = Some(*next_prev_opcert_counter);
            }
        }
    }

    (next, vec![SchedulerEffect::EnqueueBroadcast(accepted)])
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use ade_core::consensus::era_schedule::EraSchedule;
    use ade_core::consensus::vrf_cert::{ActiveSlotsCoeff, ExpectedVrfInput};
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
    use ade_crypto::ed25519::Ed25519VerificationKey;
    use ade_crypto::kes::{KesPeriod, KesSignature, SUM6_KES_SIG_LEN};
    use ade_crypto::vrf::VrfProof;
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::mempool::admit::MempoolState;
    use ade_ledger::state::LedgerState;
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::shelley::block::{OperationalCert, PrevHash, ProtocolVersion};
    use ade_types::{BlockNo, CardanoEra, EpochNo, Hash28, Hash32, SlotNo};
    use ed25519_dalek::{Signer, SigningKey as DalekSk};

    use crate::producer::tick_assembler::TickInputs;

    pub(crate) const EPOCH_576: EpochNo = EpochNo(576);
    pub(crate) const EPOCH_577_START: u64 = 163_900_800;
    pub(crate) const MAINNET_EPOCH_LENGTH: u64 = 432_000;

    pub(crate) fn schedule() -> EraSchedule {
        let start_576 = EPOCH_577_START - MAINNET_EPOCH_LENGTH;
        let eras = vec![EraSummary {
            era: CardanoEra::Conway,
            start_slot: SlotNo(start_576),
            start_epoch: EPOCH_576,
            slot_length_ms: 1_000,
            epoch_length_slots: MAINNET_EPOCH_LENGTH as u32,
            safe_zone_slots: MAINNET_EPOCH_LENGTH as u32,
        }];
        EraSchedule::new(BootstrapAnchorHash(Hash32([0u8; 32])), 0, eras)
            .expect("schedule is well-formed")
    }

    pub(crate) fn state_with_eta0(eta0: [u8; 32]) -> PraosChainDepState {
        let mut s = PraosChainDepState::empty();
        s.epoch_nonce = Nonce(Hash32(eta0));
        s.evolving_nonce = Nonce(Hash32(eta0));
        s
    }

    pub(crate) fn ledger_at_576() -> LedgerState {
        let mut l = LedgerState::new(CardanoEra::Conway);
        l.epoch_state.epoch = EPOCH_576;
        l
    }

    pub(crate) fn corpus() -> ConwayValidityCorpus {
        ConwayValidityCorpus::load().expect("corpus loads")
    }

    pub(crate) fn view(c: &ConwayValidityCorpus) -> PoolDistrView {
        let total = c.pd_total_active_stake;
        let asc = ActiveSlotsCoeff {
            numer: c.asc.numer as u32,
            denom: c.asc.denom as u32,
        };
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        for (pool_id, p) in &c.pools {
            assert!(p.sigma.denom != 0, "zero denom in corpus pool");
            assert!(
                total % p.sigma.denom == 0,
                "corpus denom does not divide total"
            );
            let scale = total / p.sigma.denom;
            let active_stake = p.sigma.numer * scale;
            pools.insert(
                Hash28(*pool_id),
                PoolEntry {
                    active_stake,
                    vrf_keyhash: Hash32(p.vrf_keyhash),
                },
            );
        }
        PoolDistrView::new(EPOCH_576, total, asc, pools)
    }

    // -----------------------------------------------------------------
    // Synthesized scheduler fixtures.
    //
    // The scheduler tests do not require a full BLUE-accepting forge —
    // they exercise the closed scheduler-effect surface: silent /
    // halt-on-self-accept / halt-persists / chain-advanced. A leader-true
    // synthesized tick reaches forge_block; the forge encodes a block
    // value; self_accept then rejects it (the synthetic block does not
    // match the corpus ledger). That rejection drives the SelfAccept
    // halt path — exactly what the test names assert.
    // -----------------------------------------------------------------

    pub(crate) fn synth_opcert(
        cold_seed: [u8; 32],
        hot_vkey: [u8; 32],
        sequence_number: u64,
        kes_period: u64,
    ) -> (OperationalCert, Ed25519VerificationKey) {
        let cold = DalekSk::from_bytes(&cold_seed);
        let cold_vk_bytes = *cold.verifying_key().as_bytes();
        let mut signable = Vec::with_capacity(48);
        signable.extend_from_slice(&hot_vkey);
        signable.extend_from_slice(&sequence_number.to_be_bytes());
        signable.extend_from_slice(&kes_period.to_be_bytes());
        let sigma = cold.sign(&signable);
        (
            OperationalCert {
                hot_vkey: hot_vkey.to_vec(),
                sequence_number,
                kes_period,
                sigma: sigma.to_bytes().to_vec(),
            },
            Ed25519VerificationKey::from_bytes(&cold_vk_bytes).unwrap(),
        )
    }

    pub(crate) fn leader_always() -> LeaderScheduleAnswer {
        LeaderScheduleAnswer {
            slot: SlotNo(100),
            pool: Hash28([0xAA; 28]),
            epoch: EpochNo(0),
            expected_vrf_input: ExpectedVrfInput::Praos([0u8; 32]),
            stake_fraction: (1, 2),
            asc: ActiveSlotsCoeff { numer: 1, denom: 1 },
        }
    }

    pub(crate) fn leader_never() -> LeaderScheduleAnswer {
        LeaderScheduleAnswer {
            slot: SlotNo(100),
            pool: Hash28([0xAA; 28]),
            epoch: EpochNo(0),
            expected_vrf_input: ExpectedVrfInput::Praos([0u8; 32]),
            stake_fraction: (0, 1),
            asc: ActiveSlotsCoeff { numer: 1, denom: 2 },
        }
    }

    pub(crate) fn base_tick_inputs(leader: LeaderScheduleAnswer) -> TickInputs {
        let (opcert, cold_vk) = synth_opcert([0x42; 32], [0x43; 32], 7, 42);
        // A real VRF proof from a deterministic seed — vrf_proof_to_hash
        // accepts it, satisfying the tick-assembler proof-shape check.
        use cardano_crypto::vrf::VrfDraft03;
        let (sk, _vk) = VrfDraft03::keypair_from_seed(&[0xA5; 32]);
        let mut alpha = Vec::with_capacity(16);
        alpha.extend_from_slice(&100u64.to_be_bytes());
        alpha.extend_from_slice(b"vrf-input-stub");
        let proof_bytes = VrfDraft03::prove(&sk, &alpha).expect("prove");
        TickInputs {
            vrf_proof: VrfProof(proof_bytes),
            kes_period: KesPeriod(42),
            kes_signature: KesSignature([0u8; SUM6_KES_SIG_LEN]),
            opcert,
            cold_vk,
            vrf_vkey: vec![0u8; 32],
            leader_answer: leader,
            pparams: Default::default(),
            mempool_tx_bytes: Vec::new(),
            prev_opcert_counter: None,
            block_number: BlockNo(1),
            prev_hash: PrevHash::Block(Hash32([0u8; 32])),
            protocol_version: ProtocolVersion { major: 9, minor: 0 },
        }
    }

    use ade_core::consensus::leader_schedule::LeaderScheduleAnswer;

    pub(crate) fn build_scheduler_state() -> SchedulerState {
        SchedulerState {
            ledger: ledger_at_576(),
            chain_dep: state_with_eta0(corpus().epoch_nonce),
            mempool: MempoolState::new(ledger_at_576()),
            era_schedule: schedule(),
            last_seen_slot: None,
            prev_opcert_counter: None,
            halted: None,
        }
    }

    // -----------------------------------------------------------------
    // §11 / §12 named tests.
    // -----------------------------------------------------------------

    #[test]
    fn producer_scheduler_silent_on_non_leader_slots() {
        let c = corpus();
        let v = view(&c);
        let state = build_scheduler_state();
        let inputs = base_tick_inputs(leader_never());
        let (next, effects) = scheduler_step(
            state,
            SchedulerInput::SlotTick { slot: 100, inputs },
            &v,
        );
        assert_eq!(effects, vec![SchedulerEffect::SilentNonLeader { slot: 100 }]);
        assert!(next.halted.is_none(), "non-leader must not halt");
        assert_eq!(next.last_seen_slot, Some(100));
    }

    #[test]
    fn producer_scheduler_self_accept_failure_halts_clean() {
        // The synthesized tick forges a block whose ledger context does
        // not match the corpus snapshot supplied to self_accept; the
        // self_accept gate rejects with a structured BlockValidityError,
        // which the scheduler surfaces as HaltOnInvariant(SelfAccept(_)).
        let c = corpus();
        let v = view(&c);
        let state = build_scheduler_state();
        let inputs = base_tick_inputs(leader_always());
        let (next, effects) = scheduler_step(
            state,
            SchedulerInput::SlotTick { slot: 100, inputs },
            &v,
        );
        assert_eq!(effects.len(), 1, "exactly one halt effect");
        match &effects[0] {
            SchedulerEffect::HaltOnInvariant {
                slot: 100,
                reason: SchedulerHaltReason::SelfAccept(_),
            } => {}
            other => panic!("expected HaltOnInvariant(SelfAccept), got {other:?}"),
        }
        assert!(
            matches!(next.halted, Some(SchedulerHaltReason::SelfAccept(_))),
            "halted field must capture the SelfAccept reason"
        );
    }

    #[test]
    fn producer_scheduler_halted_state_ignores_future_ticks() {
        let c = corpus();
        let v = view(&c);
        let state = build_scheduler_state();
        let inputs = base_tick_inputs(leader_always());

        // First tick halts on self-accept.
        let (after_first, first_effects) = scheduler_step(
            state,
            SchedulerInput::SlotTick {
                slot: 100,
                inputs: inputs.clone(),
            },
            &v,
        );
        let original_reason = match &first_effects[0] {
            SchedulerEffect::HaltOnInvariant { reason, .. } => reason.clone(),
            other => panic!("expected first effect HaltOnInvariant, got {other:?}"),
        };
        assert!(after_first.halted.is_some());

        // Subsequent tick at a later slot re-emits the ORIGINAL reason.
        let (after_second, second_effects) = scheduler_step(
            after_first,
            SchedulerInput::SlotTick {
                slot: 101,
                inputs: inputs.clone(),
            },
            &v,
        );
        assert_eq!(second_effects.len(), 1);
        match &second_effects[0] {
            SchedulerEffect::HaltOnInvariant { slot: 101, reason } => {
                assert_eq!(reason, &original_reason, "halt reason must persist");
            }
            other => panic!("expected HaltOnInvariant(slot=101), got {other:?}"),
        }
        assert_eq!(after_second.halted, Some(original_reason));
    }

    #[test]
    fn producer_scheduler_chain_advanced_refreshes_baseline() {
        let c = corpus();
        let v = view(&c);
        let state = build_scheduler_state();

        let mut new_ledger = LedgerState::new(CardanoEra::Conway);
        new_ledger.epoch_state.epoch = EpochNo(577);
        let new_chain_dep = state_with_eta0([0xEE; 32]);
        let new_mempool = MempoolState::new(new_ledger.clone());

        let (next, effects) = scheduler_step(
            state,
            SchedulerInput::ChainAdvanced {
                ledger: new_ledger.clone(),
                chain_dep: new_chain_dep.clone(),
                mempool: new_mempool.clone(),
            },
            &v,
        );
        assert!(effects.is_empty(), "ChainAdvanced emits no effects");
        assert_eq!(next.ledger, new_ledger);
        assert_eq!(next.chain_dep, new_chain_dep);
        assert_eq!(next.mempool, new_mempool);
    }

}
