// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Canonical types consumed by the BLUE fork-choice transition.
//!
//! All comparison shapes live here so the auditable surface for "which
//! chain is best" is a small, flat data set.
//!
//! `tiebreaker_prefer` is implemented as an explicit `cmp` function
//! rather than via `derive(Ord)` so the lexicographic ordering is
//! immediately visible in source — see DC-CONS-03.
//!
//! BLUE never reads a chain store, a network mux, or any I/O
//! surface. The GREEN materializer
//! `ade_runtime::consensus::candidate_fragment` is responsible for
//! assembling a `CandidateFragment` from N-D / N-A state and
//! handing it to BLUE.

use std::cmp::Ordering;

use ade_types::{BlockNo, Hash28, SlotNo};

use crate::consensus::events::{BlockDistance, Point};
use crate::consensus::header_summary::ValidatedHeaderSummary;

/// Tiebreaker view per ouroboros-consensus `PraosTiebreaker`:
/// `(slot, issuer_hash, op_cert_counter, vrf_output_first_8)`.
///
/// Total preference order (see `tiebreaker_prefer`):
///   primary = slot                       ascending  (lower slot preferred)
///   then    = issuer_hash                ascending  (lexicographic; deterministic only)
///   then    = op_cert_counter            descending (higher counter preferred)
///   then    = leader_vrf_output_first_8  ascending  (lower lottery value preferred)
///
/// CRITICAL: the comparison must match ouroboros-consensus exactly.
/// The implementation uses an explicit `Ord` function, not `derive`,
/// so the semantics are immediately auditable from source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TiebreakerView {
    pub slot: SlotNo,
    pub issuer_hash: Hash28,
    pub op_cert_counter: u64,
    pub leader_vrf_output_first_8: [u8; 8],
}

/// Returns `Ordering::Less` when `a` is *preferred over* `b`. Returns
/// `Ordering::Greater` when `b` is preferred over `a`. `Equal` only
/// when every field matches.
pub fn tiebreaker_prefer(a: &TiebreakerView, b: &TiebreakerView) -> Ordering {
    match a.slot.0.cmp(&b.slot.0) {
        Ordering::Equal => {}
        non_eq => return non_eq,
    }
    match a.issuer_hash.0.cmp(&b.issuer_hash.0) {
        Ordering::Equal => {}
        non_eq => return non_eq,
    }
    // op_cert_counter: higher is preferred → reverse compare so the
    // higher value yields Ordering::Less.
    match b.op_cert_counter.cmp(&a.op_cert_counter) {
        Ordering::Equal => {}
        non_eq => return non_eq,
    }
    a.leader_vrf_output_first_8
        .cmp(&b.leader_vrf_output_first_8)
}

/// One candidate chain fragment — a sequence of validated headers
/// rooted at a common anchor point with the current chain.
///
/// `anchor_block_no` is carried alongside `anchor` so block-number
/// comparison does not require a `Point → BlockNo` lookup, keeping
/// BLUE fork-choice free of any side-channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateFragment {
    pub anchor: Point,
    pub anchor_block_no: BlockNo,
    pub headers: Vec<ValidatedHeaderSummary>,
    pub select_view: TiebreakerView,
    pub rollback_depth: BlockDistance,
}

/// Authoritative selector state — owned by N-B, persisted by N-D.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainSelectorState {
    pub current_tip: Point,
    pub current_tip_block_no: BlockNo,
    pub current_tiebreaker: TiebreakerView,
    pub immutable_tip: Point,
    pub immutable_tip_block_no: BlockNo,
    pub security_param: crate::consensus::events::SecurityParam,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use ade_crypto::vrf::VrfOutput;
    use ade_types::Hash32;

    fn tv(
        slot: u64,
        issuer: [u8; 28],
        op_cert_counter: u64,
        vrf_first_8: [u8; 8],
    ) -> TiebreakerView {
        TiebreakerView {
            slot: SlotNo(slot),
            issuer_hash: Hash28(issuer),
            op_cert_counter,
            leader_vrf_output_first_8: vrf_first_8,
        }
    }

    #[test]
    fn tiebreaker_view_eq_is_field_wise() {
        let a = tv(10, [1u8; 28], 5, [9u8; 8]);
        let b = tv(10, [1u8; 28], 5, [9u8; 8]);
        assert_eq!(a, b);
        // Differ only in the last byte of the VRF prefix.
        let mut vrf_alt = [9u8; 8];
        vrf_alt[7] = 8;
        let c = tv(10, [1u8; 28], 5, vrf_alt);
        assert_ne!(a, c);
        // Differ only in slot.
        let d = tv(11, [1u8; 28], 5, [9u8; 8]);
        assert_ne!(a, d);
        // Differ only in issuer.
        let e = tv(10, [2u8; 28], 5, [9u8; 8]);
        assert_ne!(a, e);
        // Differ only in op_cert_counter.
        let f = tv(10, [1u8; 28], 6, [9u8; 8]);
        assert_ne!(a, f);
    }

    #[test]
    fn candidate_fragment_carries_anchor_block_no() {
        let anchor = Point {
            slot: SlotNo(50),
            hash: Hash32([7u8; 32]),
        };
        let header = ValidatedHeaderSummary {
            slot: SlotNo(60),
            block_no: BlockNo(101),
            body_hash: Hash32([2u8; 32]),
            issuer_pool: Hash28([3u8; 28]),
            op_cert_counter: 4,
            vrf_leader_output: VrfOutput([0u8; 64]),
        };
        let frag = CandidateFragment {
            anchor: anchor.clone(),
            anchor_block_no: BlockNo(100),
            headers: vec![header.clone()],
            select_view: tv(60, [3u8; 28], 4, [0u8; 8]),
            rollback_depth: BlockDistance(0),
        };
        assert_eq!(frag.anchor_block_no, BlockNo(100));
        assert_eq!(frag.anchor, anchor);
        assert_eq!(frag.headers, vec![header]);
    }
}
