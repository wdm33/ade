// GREEN — chain-selector orchestrator. Threads N-A `HeaderInput` /
// `RollBackRequest` / `EpochBoundary` inputs through BLUE transitions
// (`validate_and_apply_header`, `select_best_chain`, `apply_rollback`,
// `apply_nonce_input`). Non-authoritative: BLUE owns every comparison
// and every state-shape decision; this file only sequences calls and
// maintains the in-memory rollback snapshot list.
//
// FC/IS boundary: BLUE never receives N-A events directly. The
// orchestrator translates each `StreamInput` into the canonical BLUE
// input (`HeaderInput`, `&[CandidateFragment]`, `RollBackRequest`,
// `NonceInput::EpochBoundary`) and stores the BLUE-returned new state.
//
// No HashMap. No async. No wall-clock. BTreeMap and Vec only.

use ade_core::consensus::candidate::{
    CandidateFragment, ChainSelectorState, TiebreakerView,
};
use ade_core::consensus::errors::HeaderValidationError;
use ade_core::consensus::events::{BlockDistance, ChainEvent, ChainSelectionReject};
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::fork_choice::select_best_chain;
use ade_core::consensus::header_summary::HeaderInput;
use ade_core::consensus::header_validate::validate_and_apply_header;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::nonce::{apply_nonce_input, NonceInput};
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_core::consensus::rollback::{apply_rollback, RollBackRequest};
use ade_types::{BlockNo, EpochNo};

/// One ordered input to the chain-selector orchestrator.
///
/// Closed enum — every external trigger that can advance Ade's chain
/// state is one of these three shapes.
///
/// `HeaderInput` is intentionally inlined (not boxed) — slice §9 pins
/// the shape `HeaderArrival(HeaderInput)`. The size disparity with the
/// other two variants is accepted; the orchestrator never builds
/// dense arrays of this enum so the overhead is per-input.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum StreamInput {
    HeaderArrival(HeaderInput),
    RollBack(RollBackRequest),
    EpochBoundary {
        new_epoch: EpochNo,
        last_block_of_prev_epoch: Option<EpochNo>,
    },
}

/// Fail-fast errors the orchestrator surfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorError {
    HeaderInvalid(HeaderValidationError),
    NonceEvolution(ade_core::consensus::errors::NonceEvolutionError),
}

/// Default rollback-snapshot retention — k = 2160 (Cardano mainnet
/// security parameter). Tests may construct with a smaller bound.
pub const DEFAULT_SNAPSHOT_LIMIT: usize = 2160;

/// One rollback snapshot. Indexed by `block_no`. `chain_dep` is the
/// `PraosChainDepState` snapshot taken *after* applying the header at
/// that `block_no`; `tiebreaker` is the selector tiebreaker for the
/// same point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackSnapshot {
    pub block_no: BlockNo,
    pub chain_dep: PraosChainDepState,
    pub tiebreaker: TiebreakerView,
}

/// Orchestrator state: BLUE-owned `chain_dep` and `selector` plus a
/// bounded rollback snapshot ring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestratorState {
    pub chain_dep: PraosChainDepState,
    pub selector: ChainSelectorState,
    pub recent_snapshots: Vec<RollbackSnapshot>,
    snapshot_limit: usize,
}

impl OrchestratorState {
    /// Build a new orchestrator state with the default snapshot
    /// retention (`DEFAULT_SNAPSHOT_LIMIT`).
    pub fn new(chain_dep: PraosChainDepState, selector: ChainSelectorState) -> Self {
        Self::with_snapshot_limit(chain_dep, selector, DEFAULT_SNAPSHOT_LIMIT)
    }

    /// Build with a caller-specified snapshot retention. `0` disables
    /// snapshot retention (every rollback is rejected with
    /// `ExceededRollback`).
    pub fn with_snapshot_limit(
        chain_dep: PraosChainDepState,
        selector: ChainSelectorState,
        snapshot_limit: usize,
    ) -> Self {
        Self {
            chain_dep,
            selector,
            recent_snapshots: Vec::new(),
            snapshot_limit,
        }
    }

    pub fn snapshot_limit(&self) -> usize {
        self.snapshot_limit
    }
}

/// Process one stream input. Returns the emitted `ChainEvent` (or
/// `None` for an epoch boundary, which advances internal state without
/// changing the best-chain identity).
///
/// On any header-validation error the orchestrator state is left
/// untouched and `Err(OrchestratorError::HeaderInvalid(...))` is
/// returned, per the slice's fail-fast contract.
pub fn process_stream_input(
    state: &mut OrchestratorState,
    input: &StreamInput,
    ledger_view: &dyn LedgerView,
    era_schedule: &EraSchedule,
) -> Result<Option<ChainEvent>, OrchestratorError> {
    match input {
        StreamInput::HeaderArrival(h) => process_header_arrival(state, h, ledger_view, era_schedule)
            .map(Some),
        StreamInput::RollBack(req) => Ok(Some(process_rollback(state, req))),
        StreamInput::EpochBoundary {
            new_epoch,
            last_block_of_prev_epoch,
        } => {
            process_epoch_boundary(state, *new_epoch, *last_block_of_prev_epoch)?;
            Ok(None)
        }
    }
}

fn process_header_arrival(
    state: &mut OrchestratorState,
    header: &HeaderInput,
    ledger_view: &dyn LedgerView,
    era_schedule: &EraSchedule,
) -> Result<ChainEvent, OrchestratorError> {
    let applied = validate_and_apply_header(&state.chain_dep, header, ledger_view, era_schedule)
        .map_err(OrchestratorError::HeaderInvalid)?;

    // Build a single-header CandidateFragment rooted at the current
    // selector tip. anchor_block_no = current tip block_no so the
    // fragment's tip block_no = current + 1 (BLUE fork-choice computes
    // tip_block_no = anchor_block_no + headers.len()).
    let select_view = TiebreakerView {
        slot: applied.summary.slot,
        issuer_hash: applied.summary.issuer_pool.clone(),
        op_cert_counter: applied.summary.op_cert_counter,
        leader_vrf_output_first_8: {
            let mut prefix = [0u8; 8];
            prefix.copy_from_slice(&applied.summary.vrf_leader_output.0[0..8]);
            prefix
        },
    };
    let fragment = CandidateFragment {
        anchor: state.selector.current_tip.clone(),
        anchor_block_no: state.selector.current_tip_block_no,
        headers: vec![applied.summary.clone()],
        select_view: select_view.clone(),
        rollback_depth: BlockDistance(0),
    };

    let (new_selector, event) = match select_best_chain(&state.selector, &[fragment]) {
        Ok(pair) => pair,
        // select_best_chain only errors on empty candidates; impossible here.
        Err(_) => unreachable!("single-header fragment is non-empty"),
    };

    // Adopt BLUE's new chain-dep regardless of whether the selector
    // advanced — the header was validated and the BLUE chain-dep
    // transition has applied. If the selector rejected (e.g.
    // TiebreakerLossKeepCurrent), the chain-dep advance still stands;
    // this matches ouroboros-consensus where header validation
    // strictly precedes fork-choice and is not undone on a tiebreak
    // loss.
    state.chain_dep = applied.new_state;
    state.selector = new_selector;

    // On a chain-selected event, push a rollback snapshot.
    if matches!(event, ChainEvent::ChainSelected { .. }) {
        push_snapshot(
            state,
            RollbackSnapshot {
                block_no: applied.summary.block_no,
                chain_dep: state.chain_dep.clone(),
                tiebreaker: select_view,
            },
        );
    }

    Ok(event)
}

fn process_rollback(state: &mut OrchestratorState, req: &RollBackRequest) -> ChainEvent {
    // Step 1: locate the snapshot. If absent → ExceededRollback.
    let snapshot = state
        .recent_snapshots
        .iter()
        .find(|s| s.block_no == req.to_block_no)
        .cloned();
    let snapshot = match snapshot {
        Some(s) => s,
        None => {
            return ChainEvent::Rejected {
                reason: ChainSelectionReject::ExceededRollback {
                    requested: req.depth,
                    max: state.selector.security_param,
                },
            };
        }
    };

    let applied = apply_rollback(
        &state.selector,
        &state.chain_dep,
        &snapshot.chain_dep,
        &snapshot.tiebreaker,
        req,
    );

    // BLUE rollback may surface its own structured reject (e.g.
    // ForkBeforeImmutableTip / ExceededRollback). On reject the
    // returned state mirrors the caller's state unchanged, so it is
    // safe to assign back unconditionally.
    state.selector = applied.new_state;
    state.chain_dep = applied.new_chain_dep;

    if matches!(applied.event, ChainEvent::RolledBack { .. }) {
        // Pop any snapshots strictly newer than the rolled-back
        // point. The retained tail ends at `to_block_no` inclusive.
        state
            .recent_snapshots
            .retain(|s| s.block_no.0 <= req.to_block_no.0);
    }

    applied.event
}

fn process_epoch_boundary(
    state: &mut OrchestratorState,
    new_epoch: EpochNo,
    last_block_of_prev_epoch: Option<EpochNo>,
) -> Result<(), OrchestratorError> {
    let new_chain_dep = apply_nonce_input(
        &state.chain_dep,
        &NonceInput::EpochBoundary {
            new_epoch,
            last_block_of_prev_epoch,
        },
    )
    .map_err(OrchestratorError::NonceEvolution)?;
    state.chain_dep = new_chain_dep;
    Ok(())
}

fn push_snapshot(state: &mut OrchestratorState, snap: RollbackSnapshot) {
    state.recent_snapshots.push(snap);
    // Bounded retention. snapshot_limit == 0 means no retention.
    if state.recent_snapshots.len() > state.snapshot_limit {
        let excess = state.recent_snapshots.len() - state.snapshot_limit;
        state.recent_snapshots.drain(0..excess);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use ade_core::consensus::candidate::ChainSelectorState;
    use ade_core::consensus::era_schedule::{BootstrapAnchorHash, EraSummary};
    use ade_core::consensus::header_summary::HeaderVrf;
    use ade_core::consensus::events::{Point, SecurityParam};
    use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
    use ade_core::consensus::vrf_cert::{vrf_input, ActiveSlotsCoeff, VrfRole};
    use ade_crypto::vrf::{VrfProof, VrfVerificationKey};
    use ade_testkit::consensus::ledger_view_stub::{
        EpochStakeFixture, LedgerViewStub, PoolFixture,
    };
    use ade_types::{CardanoEra, Hash28, Hash32, SlotNo};
    use cardano_crypto::vrf::VrfDraft03;

    fn schedule() -> EraSchedule {
        let eras = vec![EraSummary {
            era: CardanoEra::Shelley,
            start_slot: SlotNo(0),
            start_epoch: EpochNo(0),
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 129_600,
        }];
        EraSchedule::new(BootstrapAnchorHash(Hash32([0u8; 32])), 0, eras).expect("schedule")
    }

    fn pool() -> Hash28 {
        Hash28([0xAA; 28])
    }

    fn keypair() -> ([u8; 64], VrfVerificationKey) {
        let (sk, vk_bytes) = VrfDraft03::keypair_from_seed(&[7u8; 32]);
        (sk, VrfVerificationKey(vk_bytes))
    }

    fn prove(sk: &[u8; 64], slot: SlotNo, epoch_nonce: &Nonce, role: VrfRole) -> VrfProof {
        let alpha = vrf_input(slot, epoch_nonce, role);
        let bytes = VrfDraft03::prove(sk, &alpha).expect("prove");
        VrfProof(bytes)
    }

    fn ledger(vk: VrfVerificationKey) -> LedgerViewStub {
        let mut pools = BTreeMap::new();
        pools.insert(
            pool(),
            PoolFixture {
                active_stake: 1,
                vrf_keyhash: ade_crypto::blake2b::blake2b_256(&vk.0),
            },
        );
        // asc = 1/1 + sigma = 1/1 → every VRF output trivially leads.
        // Same shape as S-B7 header_validate_compose tests.
        let mut stub = LedgerViewStub::new().with_epoch(
            EpochNo(0),
            EpochStakeFixture {
                total_active_stake: 1,
                asc: ActiveSlotsCoeff { numer: 1, denom: 1 },
                pools: pools.clone(),
            },
        );
        // Same fixture for epoch 1 so post-epoch-boundary headers
        // still locate a stake snapshot.
        stub = stub.with_epoch(
            EpochNo(1),
            EpochStakeFixture {
                total_active_stake: 1,
                asc: ActiveSlotsCoeff { numer: 1, denom: 1 },
                pools,
            },
        );
        stub
    }

    fn genesis_chain_dep() -> PraosChainDepState {
        let mut s = PraosChainDepState::empty();
        s.epoch_nonce = Nonce(Hash32([0xCD; 32]));
        s.evolving_nonce = Nonce(Hash32([0xEE; 32]));
        s.candidate_nonce = Nonce(Hash32([0xCD; 32]));
        s
    }

    fn genesis_selector() -> ChainSelectorState {
        ChainSelectorState {
            current_tip: Point {
                slot: SlotNo(0),
                hash: Hash32([0u8; 32]),
            },
            current_tip_block_no: BlockNo(0),
            current_tiebreaker: TiebreakerView {
                slot: SlotNo(0),
                issuer_hash: Hash28([0u8; 28]),
                op_cert_counter: 0,
                leader_vrf_output_first_8: [0u8; 8],
            },
            immutable_tip: Point {
                slot: SlotNo(0),
                hash: Hash32([0u8; 32]),
            },
            immutable_tip_block_no: BlockNo(0),
            security_param: SecurityParam(2160),
        }
    }

    fn header_at(
        sk: &[u8; 64],
        vk: &VrfVerificationKey,
        chain_dep: &PraosChainDepState,
        slot: SlotNo,
        block_no: BlockNo,
        op_cert_counter: u64,
    ) -> HeaderInput {
        HeaderInput {
            slot,
            block_no,
            body_hash: Hash32([0x55; 32]),
            issuer_pool: pool(),
            op_cert_kes_period: 0,
            op_cert_counter,
            vrf_vk: vk.clone(),
            vrf: HeaderVrf::Tpraos {
                nonce_proof: prove(sk, slot, &chain_dep.epoch_nonce, VrfRole::NonceContribution),
                leader_proof: prove(sk, slot, &chain_dep.epoch_nonce, VrfRole::LeaderEligibility),
            },
            kes: None,
        }
    }

    #[test]
    fn header_arrival_updates_state_and_selector() {
        let (sk, vk) = keypair();
        let mut state = OrchestratorState::new(genesis_chain_dep(), genesis_selector());
        let header = header_at(&sk, &vk, &state.chain_dep, SlotNo(1), BlockNo(1), 0);

        let evt = process_stream_input(
            &mut state,
            &StreamInput::HeaderArrival(header),
            &ledger(vk),
            &schedule(),
        )
        .expect("happy path");
        assert!(matches!(evt, Some(ChainEvent::ChainSelected { .. })));
        assert_eq!(state.selector.current_tip_block_no, BlockNo(1));
        assert_eq!(state.chain_dep.last_block_no, Some(BlockNo(1)));
        assert_eq!(state.chain_dep.last_slot, Some(SlotNo(1)));
        assert_eq!(state.recent_snapshots.len(), 1);
        assert_eq!(state.recent_snapshots[0].block_no, BlockNo(1));
    }

    #[test]
    fn rollback_walks_back_via_recent_snapshots() {
        let (sk, vk) = keypair();
        let mut state =
            OrchestratorState::with_snapshot_limit(genesis_chain_dep(), genesis_selector(), 100);
        let ldg = ledger(vk.clone());
        let sched = schedule();

        for i in 1..=3u64 {
            // op_cert_counter strictly increases per (pool, kes_period) —
            // pre-incremented per block to satisfy the BLUE op-cert
            // monotonicity check.
            let h = header_at(&sk, &vk, &state.chain_dep, SlotNo(i), BlockNo(i), i);
            process_stream_input(&mut state, &StreamInput::HeaderArrival(h), &ldg, &sched).unwrap();
        }
        assert_eq!(state.selector.current_tip_block_no, BlockNo(3));
        let chain_dep_at_2 = state.recent_snapshots[1].chain_dep.clone();

        let req = RollBackRequest {
            to_point: Point {
                slot: SlotNo(2),
                hash: Hash32([0x55; 32]),
            },
            to_block_no: BlockNo(2),
            depth: BlockDistance(1),
        };
        let evt = process_stream_input(&mut state, &StreamInput::RollBack(req), &ldg, &sched)
            .expect("rollback");
        assert!(matches!(evt, Some(ChainEvent::RolledBack { .. })));
        assert_eq!(state.selector.current_tip_block_no, BlockNo(2));
        assert_eq!(state.chain_dep, chain_dep_at_2);
        // Snapshots strictly newer than block 2 are dropped.
        assert_eq!(state.recent_snapshots.len(), 2);
        assert_eq!(state.recent_snapshots.last().unwrap().block_no, BlockNo(2));
    }

    #[test]
    fn rollback_to_block_older_than_snapshots_rejected() {
        // snapshot_limit = 2 → after 5 headers, only blocks {4, 5} are
        // retained. A rollback to block 1 must surface
        // `ExceededRollback`.
        let (sk, vk) = keypair();
        let mut state =
            OrchestratorState::with_snapshot_limit(genesis_chain_dep(), genesis_selector(), 2);
        let ldg = ledger(vk.clone());
        let sched = schedule();
        for i in 1..=5u64 {
            let h = header_at(&sk, &vk, &state.chain_dep, SlotNo(i), BlockNo(i), i);
            process_stream_input(&mut state, &StreamInput::HeaderArrival(h), &ldg, &sched).unwrap();
        }
        assert_eq!(state.recent_snapshots.len(), 2);
        assert_eq!(state.recent_snapshots[0].block_no, BlockNo(4));

        let req = RollBackRequest {
            to_point: Point {
                slot: SlotNo(1),
                hash: Hash32([0x99; 32]),
            },
            to_block_no: BlockNo(1),
            depth: BlockDistance(4),
        };
        let evt = process_stream_input(&mut state, &StreamInput::RollBack(req), &ldg, &sched)
            .expect("rollback returns event, not Err");
        match evt {
            Some(ChainEvent::Rejected {
                reason: ChainSelectionReject::ExceededRollback { .. },
            }) => {}
            other => panic!("expected ExceededRollback, got {:?}", other),
        }
        // Selector & chain_dep unchanged.
        assert_eq!(state.selector.current_tip_block_no, BlockNo(5));
    }

    #[test]
    fn epoch_boundary_emits_no_event() {
        let (sk, vk) = keypair();
        let mut state = OrchestratorState::new(genesis_chain_dep(), genesis_selector());
        let ldg = ledger(vk.clone());
        let sched = schedule();

        let h = header_at(&sk, &vk, &state.chain_dep, SlotNo(1), BlockNo(1), 0);
        process_stream_input(&mut state, &StreamInput::HeaderArrival(h), &ldg, &sched).unwrap();
        let prior_epoch_nonce = state.chain_dep.epoch_nonce.clone();
        let prior_candidate = state.chain_dep.candidate_nonce.clone();

        let evt = process_stream_input(
            &mut state,
            &StreamInput::EpochBoundary {
                new_epoch: EpochNo(1),
                last_block_of_prev_epoch: Some(EpochNo(0)),
            },
            &ldg,
            &sched,
        )
        .expect("epoch boundary");
        assert_eq!(evt, None);
        // Internal state evolved: epoch_nonce now equals prior candidate;
        // previous_epoch_nonce equals prior epoch_nonce.
        assert_eq!(state.chain_dep.epoch_nonce, prior_candidate);
        assert_eq!(state.chain_dep.previous_epoch_nonce, prior_epoch_nonce);
        // Selector unchanged on epoch boundary.
        assert_eq!(state.selector.current_tip_block_no, BlockNo(1));
    }
}
