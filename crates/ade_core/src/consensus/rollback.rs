// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Praos rollback authority — BLUE pure transition.
//!
//! `apply_rollback` is independent of fork-choice: it operates on a
//! `ChainSelectorState` and a `RollBackRequest` and returns a
//! `RollBackApplied` carrying either the rolled-back state plus a
//! `ChainEvent::RolledBack`, or the unchanged state plus a
//! `ChainEvent::Rejected` with one of two structured reasons —
//! `ForkBeforeImmutableTip` or `ExceededRollback`.
//!
//! Refusal shape: the transition does NOT return `Err` for rejects.
//! The caller (chain-selector orchestrator, S-B10) receives both the
//! event and the state in one shape, recording the reject without
//! losing the prior state.
//!
//! BLUE never reads a chain store, a network mux, or any I/O surface.
//! The caller supplies the chain-dep state and tiebreaker view at the
//! rolled-back point — typically materialized from an N-D snapshot or
//! replay. This module never advances `immutable_tip`; that rule
//! lives in S-B10's orchestrator.

use crate::consensus::candidate::{ChainSelectorState, TiebreakerView};
use crate::consensus::events::{BlockDistance, ChainEvent, ChainSelectionReject, Point};
use crate::consensus::praos_state::PraosChainDepState;
use ade_types::BlockNo;

/// Request to roll back to an ancestor point.
///
/// `depth` is supplied by the caller from its chain history. The
/// transition uses it for the k-bound check and surfaces it in the
/// reject reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollBackRequest {
    pub to_point: Point,
    pub to_block_no: BlockNo,
    pub depth: BlockDistance,
}

/// The single output shape of `apply_rollback` — carries the new
/// (or unchanged-on-reject) state, the chain-dep state, and the
/// `ChainEvent` produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollBackApplied {
    pub new_state: ChainSelectorState,
    pub new_chain_dep: PraosChainDepState,
    pub event: ChainEvent,
}

/// Pure rollback transition.
///
/// Inputs:
///   - `state`: current `ChainSelectorState`
///   - `chain_dep`: current `PraosChainDepState` (used on reject so
///     the returned `new_chain_dep` mirrors the caller's state
///     unchanged)
///   - `rolled_back_chain_dep`: `PraosChainDepState` at the rolled-back
///     point, supplied by the caller (N-D snapshot or replay)
///   - `rolled_back_tiebreaker`: `TiebreakerView` at the rolled-back
///     point, supplied by the caller
///   - `request`: `RollBackRequest`
///
/// Algorithm:
///   1. If `request.to_block_no < state.immutable_tip_block_no` →
///      return state + chain_dep unchanged with
///      `ChainEvent::Rejected { ForkBeforeImmutableTip { .. } }`.
///   2. If `request.depth > state.security_param` → return state +
///      chain_dep unchanged with
///      `ChainEvent::Rejected { ExceededRollback { .. } }`.
///   3. Otherwise: apply the rollback. `new_state` adopts the
///      `request.to_point`, `request.to_block_no`, and supplied
///      tiebreaker; `immutable_tip{,_block_no}` and `security_param`
///      are preserved verbatim. `new_chain_dep` is the supplied
///      `rolled_back_chain_dep`. Event is
///      `ChainEvent::RolledBack { to_point, depth }`.
pub fn apply_rollback(
    state: &ChainSelectorState,
    chain_dep: &PraosChainDepState,
    rolled_back_chain_dep: &PraosChainDepState,
    rolled_back_tiebreaker: &TiebreakerView,
    request: &RollBackRequest,
) -> RollBackApplied {
    // Step 1: refuse a rollback that would cross the immutable tip.
    if request.to_block_no.0 < state.immutable_tip_block_no.0 {
        return RollBackApplied {
            new_state: state.clone(),
            new_chain_dep: chain_dep.clone(),
            event: ChainEvent::Rejected {
                reason: ChainSelectionReject::ForkBeforeImmutableTip {
                    immutable_tip: state.immutable_tip.clone(),
                    candidate_intersection: request.to_point.clone(),
                    rollback_depth: request.depth,
                    security_param: state.security_param,
                },
            },
        };
    }

    // Step 2: refuse a rollback that exceeds the security parameter k.
    if request.depth.0 > state.security_param.0 {
        return RollBackApplied {
            new_state: state.clone(),
            new_chain_dep: chain_dep.clone(),
            event: ChainEvent::Rejected {
                reason: ChainSelectionReject::ExceededRollback {
                    requested: request.depth,
                    max: state.security_param,
                },
            },
        };
    }

    // Step 3: apply. `immutable_tip{,_block_no}` and `security_param`
    // are read-only here per the slice's design note.
    let new_state = ChainSelectorState {
        current_tip: request.to_point.clone(),
        current_tip_block_no: request.to_block_no,
        current_tiebreaker: rolled_back_tiebreaker.clone(),
        immutable_tip: state.immutable_tip.clone(),
        immutable_tip_block_no: state.immutable_tip_block_no,
        security_param: state.security_param,
    };
    let event = ChainEvent::RolledBack {
        to_point: request.to_point.clone(),
        depth: request.depth,
    };
    RollBackApplied {
        new_state,
        new_chain_dep: rolled_back_chain_dep.clone(),
        event,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::consensus::candidate::ChainSelectorState;
    use crate::consensus::events::SecurityParam;
    use crate::consensus::praos_state::{Nonce, PraosChainDepState};
    use ade_types::{Hash28, Hash32, SlotNo};

    fn tv(slot: u64, issuer: u8, counter: u64, vrf: u8) -> TiebreakerView {
        TiebreakerView {
            slot: SlotNo(slot),
            issuer_hash: Hash28([issuer; 28]),
            op_cert_counter: counter,
            leader_vrf_output_first_8: [vrf; 8],
        }
    }

    fn state(
        current_slot: u64,
        current_block_no: u64,
        immutable_block_no: u64,
        k: u64,
    ) -> ChainSelectorState {
        ChainSelectorState {
            current_tip: Point {
                slot: SlotNo(current_slot),
                hash: Hash32([0x11; 32]),
            },
            current_tip_block_no: BlockNo(current_block_no),
            current_tiebreaker: tv(current_slot, 0xaa, 5, 0x01),
            immutable_tip: Point {
                slot: SlotNo(immutable_block_no * 2),
                hash: Hash32([0x00; 32]),
            },
            immutable_tip_block_no: BlockNo(immutable_block_no),
            security_param: SecurityParam(k),
        }
    }

    fn chain_dep(byte: u8) -> PraosChainDepState {
        PraosChainDepState::genesis(Nonce(Hash32([byte; 32])))
    }

    fn point(slot: u64, byte: u8) -> Point {
        Point {
            slot: SlotNo(slot),
            hash: Hash32([byte; 32]),
        }
    }

    #[test]
    fn rollback_preserves_immutable_tip() {
        let s = state(100, 50, 25, 2160);
        let cd = chain_dep(0xaa);
        let cd_back = chain_dep(0xbb);
        let req = RollBackRequest {
            to_point: point(80, 0x77),
            to_block_no: BlockNo(40),
            depth: BlockDistance(10),
        };
        let r = apply_rollback(&s, &cd, &cd_back, &tv(80, 0xbb, 4, 0x02), &req);
        assert_eq!(r.new_state.immutable_tip, s.immutable_tip);
        assert_eq!(r.new_state.immutable_tip_block_no, s.immutable_tip_block_no);
    }

    #[test]
    fn rollback_preserves_security_param() {
        let s = state(100, 50, 25, 2160);
        let cd = chain_dep(0xaa);
        let cd_back = chain_dep(0xbb);
        let req = RollBackRequest {
            to_point: point(80, 0x77),
            to_block_no: BlockNo(40),
            depth: BlockDistance(10),
        };
        let r = apply_rollback(&s, &cd, &cd_back, &tv(80, 0xbb, 4, 0x02), &req);
        assert_eq!(r.new_state.security_param, s.security_param);
    }

    #[test]
    fn rollback_with_zero_depth_is_noop() {
        // depth = 0 is the degenerate "roll back to where we are"; it
        // is still a valid event (the orchestrator may treat it as a
        // no-op upstream). The transition itself adopts the request's
        // to_point as the new tip even when depth is zero.
        let s = state(100, 50, 25, 2160);
        let cd = chain_dep(0xaa);
        let cd_back = chain_dep(0xbb);
        let req = RollBackRequest {
            to_point: s.current_tip.clone(),
            to_block_no: s.current_tip_block_no,
            depth: BlockDistance(0),
        };
        let r = apply_rollback(&s, &cd, &cd_back, &s.current_tiebreaker, &req);
        match r.event {
            ChainEvent::RolledBack { depth, .. } => assert_eq!(depth, BlockDistance(0)),
            other => panic!("expected RolledBack, got {:?}", other),
        }
        // State adopts the request: same as `s` modulo chain_dep, since
        // the request mirrored the current tip exactly.
        assert_eq!(r.new_state, s);
    }

    #[test]
    fn rollback_to_equal_block_no_as_immutable_succeeds() {
        // immutable_tip_block_no == request.to_block_no — boundary case;
        // exclusively strict `<` triggers the refusal.
        let s = state(100, 50, 25, 2160);
        let cd = chain_dep(0xaa);
        let cd_back = chain_dep(0xbb);
        let req = RollBackRequest {
            to_point: point(50, 0x33),
            to_block_no: BlockNo(25),
            depth: BlockDistance(25),
        };
        let r = apply_rollback(&s, &cd, &cd_back, &tv(50, 0xbb, 4, 0x02), &req);
        match r.event {
            ChainEvent::RolledBack { .. } => {}
            other => panic!("expected RolledBack, got {:?}", other),
        }
        assert_eq!(r.new_state.current_tip_block_no, BlockNo(25));
    }

    #[test]
    fn rollback_to_one_below_immutable_rejected() {
        let s = state(100, 50, 25, 2160);
        let cd = chain_dep(0xaa);
        let cd_back = chain_dep(0xbb);
        let req = RollBackRequest {
            to_point: point(48, 0x44),
            to_block_no: BlockNo(24),
            depth: BlockDistance(26),
        };
        let snapshot_state = s.clone();
        let snapshot_chain_dep = cd.clone();
        let r = apply_rollback(&s, &cd, &cd_back, &tv(48, 0xbb, 4, 0x02), &req);
        match r.event {
            ChainEvent::Rejected {
                reason: ChainSelectionReject::ForkBeforeImmutableTip { .. },
            } => {}
            other => panic!("expected ForkBeforeImmutableTip, got {:?}", other),
        }
        // State + chain_dep unchanged on reject.
        assert_eq!(r.new_state, snapshot_state);
        assert_eq!(r.new_chain_dep, snapshot_chain_dep);
    }
}
