// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::{BlockNo, Hash32, SlotNo};

use crate::consensus::errors::HeaderValidationError;

/// Forward-declared point identifier. Refined in S-B7/S-B8.
///
/// `Hash32` is not `Copy` in this codebase (32-byte arrays are
/// cheap to clone but not `Copy`-friendly across the workspace),
/// so `Point` is `Clone` but not `Copy`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Point {
    pub slot: SlotNo,
    pub hash: Hash32,
}

/// Forward-declared chain-tip identifier.
pub type ChainHash = Hash32;

/// Distance between two points expressed in blocks (not slots).
/// Rollback depth is measured in blocks per DC-CONS-05.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockDistance(pub u64);

/// Security parameter k (block-count rollback bound).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SecurityParam(pub u64);

/// Reasons fork-choice / rollback reject a candidate.
/// CLOSED — every variant is exhaustive; no `Other` or `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainSelectionReject {
    ForkBeforeImmutableTip {
        immutable_tip: Point,
        candidate_intersection: Point,
        rollback_depth: BlockDistance,
        security_param: SecurityParam,
    },
    ExceededRollback {
        requested: BlockDistance,
        max: SecurityParam,
    },
    HeaderInvalid {
        at_point: Point,
        reason: HeaderValidationError,
    },
    TiebreakerLossKeepCurrent {
        current_tip: Point,
        candidate_tip: Point,
    },
}

/// Output of the fork-choice / rollback transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainEvent {
    ChainExtended {
        new_tip: Point,
        block_no: BlockNo,
    },
    RolledBack {
        to_point: Point,
        depth: BlockDistance,
    },
    RolledForward {
        from: Point,
        to: Point,
    },
    ChainSelected {
        new_tip: Point,
        replaced_tip: Option<Point>,
    },
    Rejected {
        reason: ChainSelectionReject,
    },
}
