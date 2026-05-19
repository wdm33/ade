// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Praos fork-choice — BLUE authoritative transition.
//!
//! Pinned reference (DC-CONS-03 auditable artifact): cardano-node
//! **10.6.2** ships with ouroboros-consensus **~0.22.x**
//! (`ouroboros-consensus` package version visible in cardano-node
//! 10.6.2's `cabal.project.freeze`). The `SelectView` ordering encoded
//! here matches the `PraosTiebreaker` shape from that release:
//! `(BlockNo, then TiebreakerView (slot, issuer, op_cert_counter,
//! vrf_output_first_8))`. Chain-length-weight ordering is reserved
//! for Genesis / catch-up and is explicitly forbidden in this
//! module — see the CI check that greps for the forbidden term.
//!
//! BLUE contract: `select_best_chain` consumes a borrowed
//! `ChainSelectorState` and a slice of `CandidateFragment`s and
//! returns a new state plus a single `ChainEvent`. It never reads
//! a chain store, a network mux, or any I/O surface. It performs
//! only integer comparisons and lexicographic byte comparisons —
//! no float, no `HashMap`, no wall-clock.
//!
//! The cluster's primary invariant says fork-choice is a pure
//! function over `(candidate_fragments, EraSchedule, ledger_view,
//! protocol_params)`. In practice the BLUE comparison uses only
//! block-number + `TiebreakerView`, neither of which needs
//! `EraSchedule`, `LedgerView`, or `ProtocolParameters` directly.
//! S-B10's orchestrator threads those through to the upstream
//! materializer that produces these typed inputs; the BLUE surface
//! here stays minimal so the comparison is auditable in one place.

// no-density: this module forbids density-based ordering; the only
// no-density: permissible mention of the word d-e-n-s-i-t-y in this
// no-density: file is on lines beginning with this audit marker.

use std::cmp::Ordering;

use crate::consensus::candidate::{
    tiebreaker_prefer, CandidateFragment, ChainSelectorState, TiebreakerView,
};
use crate::consensus::events::{ChainEvent, ChainSelectionReject};
use ade_types::BlockNo;

/// Errors that halt `select_best_chain` before any candidate is
/// considered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForkChoiceError {
    NoCandidates,
}

/// One eligible candidate's projection used for comparison.
struct EligibleCandidate<'a> {
    tip_block_no: BlockNo,
    tip_tiebreaker: &'a TiebreakerView,
    tip_point: crate::consensus::events::Point,
}

/// Compare two `(block_no, tiebreaker)` pairs. `Less` means the *first*
/// pair is preferred.
fn compare_select_view(
    lhs_block_no: BlockNo,
    lhs_tb: &TiebreakerView,
    rhs_block_no: BlockNo,
    rhs_tb: &TiebreakerView,
) -> Ordering {
    // Higher block number is preferred → reverse compare.
    match rhs_block_no.0.cmp(&lhs_block_no.0) {
        Ordering::Equal => tiebreaker_prefer(lhs_tb, rhs_tb),
        non_eq => non_eq,
    }
}

/// Best-chain selection. Pure: same inputs always produce the same
/// `(new_state, event)`.
///
/// Algorithm:
///   1. Partition candidates into eligible / ineligible. Ineligible
///      causes: fork before immutable tip, rollback exceeds k.
///   2. If at least one candidate is eligible, compare each to the
///      current tip by `(block_no, tiebreaker)`. The best candidate
///      among the eligible set is chosen.
///   3. If the best eligible candidate is strictly preferred over the
///      current tip, emit `ChainSelected`; otherwise emit `Rejected`
///      with `TiebreakerLossKeepCurrent`.
///   4. If no candidate is eligible but at least one was supplied,
///      emit `Rejected` with the reject reason of the highest-block-no
///      ineligible candidate.
///   5. If `candidates` is empty, return `ForkChoiceError::NoCandidates`.
pub fn select_best_chain(
    state: &ChainSelectorState,
    candidates: &[CandidateFragment],
) -> Result<(ChainSelectorState, ChainEvent), ForkChoiceError> {
    if candidates.is_empty() {
        return Err(ForkChoiceError::NoCandidates);
    }

    let mut eligible: Vec<EligibleCandidate<'_>> = Vec::new();
    let mut ineligible: Vec<(BlockNo, ChainSelectionReject)> = Vec::new();

    for c in candidates {
        // Ineligibility check 1: fork rooted before the immutable tip.
        if c.anchor.slot.0 < state.immutable_tip.slot.0 {
            let candidate_tip_block_no = candidate_tip_block_no(c);
            ineligible.push((
                candidate_tip_block_no,
                ChainSelectionReject::ForkBeforeImmutableTip {
                    immutable_tip: state.immutable_tip.clone(),
                    candidate_intersection: c.anchor.clone(),
                    rollback_depth: c.rollback_depth,
                    security_param: state.security_param,
                },
            ));
            continue;
        }
        // Ineligibility check 2: rollback depth exceeds the security
        // parameter k.
        if c.rollback_depth.0 > state.security_param.0 {
            let candidate_tip_block_no = candidate_tip_block_no(c);
            ineligible.push((
                candidate_tip_block_no,
                ChainSelectionReject::ExceededRollback {
                    requested: c.rollback_depth,
                    max: state.security_param,
                },
            ));
            continue;
        }

        // Eligible — compute tip projection.
        let tip_block_no = candidate_tip_block_no(c);
        let tip_point = candidate_tip_point(c);
        eligible.push(EligibleCandidate {
            tip_block_no,
            tip_tiebreaker: &c.select_view,
            tip_point,
        });
    }

    if eligible.is_empty() {
        // All candidates were ineligible — report the reject reason of
        // the candidate with the highest tip block number. Ties broken
        // by first-occurrence order. `ineligible` is non-empty here
        // because `candidates` is non-empty (we returned early on the
        // empty case) and no candidate was eligible.
        let mut best_idx: usize = 0;
        let mut best_block_no = ineligible[0].0.0;
        for (i, (bn, _)) in ineligible.iter().enumerate().skip(1) {
            if bn.0 > best_block_no {
                best_block_no = bn.0;
                best_idx = i;
            }
        }
        let reason = ineligible[best_idx].1.clone();
        return Ok((state.clone(), ChainEvent::Rejected { reason }));
    }

    // Pick the maximally-preferred eligible candidate.
    let mut best_idx: usize = 0;
    for (i, cand) in eligible.iter().enumerate().skip(1) {
        let cmp = compare_select_view(
            cand.tip_block_no,
            cand.tip_tiebreaker,
            eligible[best_idx].tip_block_no,
            eligible[best_idx].tip_tiebreaker,
        );
        if cmp == Ordering::Less {
            best_idx = i;
        }
    }
    let best = &eligible[best_idx];

    // Compare the best eligible candidate to the current tip.
    let cmp_to_current = compare_select_view(
        best.tip_block_no,
        best.tip_tiebreaker,
        state.current_tip_block_no,
        &state.current_tiebreaker,
    );

    if cmp_to_current == Ordering::Less {
        // Strictly preferred → adopt.
        let new_state = ChainSelectorState {
            current_tip: best.tip_point.clone(),
            current_tip_block_no: best.tip_block_no,
            current_tiebreaker: best.tip_tiebreaker.clone(),
            immutable_tip: state.immutable_tip.clone(),
            immutable_tip_block_no: state.immutable_tip_block_no,
            security_param: state.security_param,
        };
        let event = ChainEvent::ChainSelected {
            new_tip: best.tip_point.clone(),
            replaced_tip: Some(state.current_tip.clone()),
        };
        Ok((new_state, event))
    } else {
        // Equal or worse → keep current, emit TiebreakerLossKeepCurrent.
        Ok((
            state.clone(),
            ChainEvent::Rejected {
                reason: ChainSelectionReject::TiebreakerLossKeepCurrent {
                    current_tip: state.current_tip.clone(),
                    candidate_tip: best.tip_point.clone(),
                },
            },
        ))
    }
}

fn candidate_tip_block_no(c: &CandidateFragment) -> BlockNo {
    // Tip block number = anchor block_no + number of headers in the
    // fragment. headers is non-empty for a valid candidate; if empty,
    // the tip is the anchor itself.
    BlockNo(c.anchor_block_no.0.saturating_add(c.headers.len() as u64))
}

fn candidate_tip_point(c: &CandidateFragment) -> crate::consensus::events::Point {
    if let Some(last) = c.headers.last() {
        crate::consensus::events::Point {
            slot: last.slot,
            hash: last.body_hash.clone(),
        }
    } else {
        c.anchor.clone()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::consensus::candidate::TiebreakerView;
    use crate::consensus::events::{BlockDistance, Point, SecurityParam};
    use ade_crypto::vrf::VrfOutput;
    use ade_types::{Hash28, Hash32, SlotNo};

    fn tv(slot: u64, issuer: u8, counter: u64, vrf_first: u8) -> TiebreakerView {
        TiebreakerView {
            slot: SlotNo(slot),
            issuer_hash: Hash28([issuer; 28]),
            op_cert_counter: counter,
            leader_vrf_output_first_8: [vrf_first; 8],
        }
    }

    fn state(
        current_slot: u64,
        current_block_no: u64,
        immutable_slot: u64,
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
                slot: SlotNo(immutable_slot),
                hash: Hash32([0x00; 32]),
            },
            immutable_tip_block_no: BlockNo(immutable_block_no),
            security_param: SecurityParam(k),
        }
    }

    fn header(slot: u64, block_no: u64, hash: u8) -> crate::consensus::header_summary::ValidatedHeaderSummary {
        crate::consensus::header_summary::ValidatedHeaderSummary {
            slot: SlotNo(slot),
            block_no: BlockNo(block_no),
            body_hash: Hash32([hash; 32]),
            issuer_pool: Hash28([hash; 28]),
            op_cert_counter: 0,
            vrf_leader_output: VrfOutput([0u8; 64]),
        }
    }

    #[test]
    fn tiebreaker_prefer_lower_slot_wins() {
        let earlier = tv(100, 0xaa, 5, 0x01);
        let later = tv(101, 0xaa, 5, 0x01);
        assert_eq!(tiebreaker_prefer(&earlier, &later), Ordering::Less);
        assert_eq!(tiebreaker_prefer(&later, &earlier), Ordering::Greater);
    }

    #[test]
    fn tiebreaker_prefer_higher_op_cert_wins_on_equal_slot_and_issuer() {
        let lo = tv(100, 0xaa, 5, 0x01);
        let hi = tv(100, 0xaa, 9, 0x01);
        assert_eq!(tiebreaker_prefer(&hi, &lo), Ordering::Less);
        assert_eq!(tiebreaker_prefer(&lo, &hi), Ordering::Greater);
    }

    #[test]
    fn tiebreaker_prefer_lower_vrf_value_wins_on_full_tie() {
        let lower_vrf = tv(100, 0xaa, 5, 0x01);
        let higher_vrf = tv(100, 0xaa, 5, 0x02);
        assert_eq!(tiebreaker_prefer(&lower_vrf, &higher_vrf), Ordering::Less);
        assert_eq!(tiebreaker_prefer(&higher_vrf, &lower_vrf), Ordering::Greater);
    }

    #[test]
    fn no_candidates_returns_no_candidates_error() {
        let s = state(100, 50, 50, 25, 2160);
        let r = select_best_chain(&s, &[]);
        assert_eq!(r, Err(ForkChoiceError::NoCandidates));
    }

    #[test]
    fn equal_to_current_keeps_current_via_tiebreaker_loss() {
        let s = state(100, 50, 50, 25, 2160);
        // Candidate: same block_no, identical tiebreaker → equal → keep current.
        let frag = CandidateFragment {
            anchor: Point {
                slot: SlotNo(95),
                hash: Hash32([0x22; 32]),
            },
            anchor_block_no: BlockNo(49),
            headers: vec![header(100, 50, 0x33)],
            select_view: tv(100, 0xaa, 5, 0x01),
            rollback_depth: BlockDistance(1),
        };
        let (new_state, evt) = select_best_chain(&s, std::slice::from_ref(&frag)).unwrap();
        // state unchanged
        assert_eq!(new_state, s);
        match evt {
            ChainEvent::Rejected {
                reason: ChainSelectionReject::TiebreakerLossKeepCurrent { .. },
            } => {}
            other => panic!("expected TiebreakerLossKeepCurrent, got {:?}", other),
        }
    }
}
