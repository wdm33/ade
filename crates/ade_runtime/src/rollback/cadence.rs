// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN snapshot cadence policy (PHASE4-N-I S4).
//!
//! Pure decision function `should_snapshot_after_block(slot,
//! block_no, cadence, last_snapshot)`. `SnapshotCadence` is a
//! BLUE-structural parameter (every N blocks). No runtime-mutable
//! input; operator-tunable cadence is explicitly out of scope per
//! DC-STORE-07 and the cluster's scope decisions.

use ade_types::{BlockNo, SlotNo};

/// Snapshot cadence parameters. BLUE-structural: the only field is
/// `every_n_blocks` (default 100). Pinning a default at the type
/// level — operator-tunable cadence would weaken DC-STORE-07's
/// replay-equivalence guarantee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotCadence {
    pub every_n_blocks: u32,
}

impl SnapshotCadence {
    pub const DEFAULT: Self = Self {
        every_n_blocks: 100,
    };
}

impl Default for SnapshotCadence {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Decide whether to snapshot after applying block at
/// `(slot, block_no)`. Pure; same inputs → same decision.
///
/// Cadence rule (block-based): snapshot iff
/// `block_no.0 % cadence.every_n_blocks == 0` AND
/// `last_snapshot.is_none() || last_snapshot.unwrap() < slot`.
pub fn should_snapshot_after_block(
    slot: SlotNo,
    block_no: BlockNo,
    cadence: SnapshotCadence,
    last_snapshot: Option<SlotNo>,
) -> bool {
    if cadence.every_n_blocks == 0 {
        return false;
    }
    if block_no.0 % cadence.every_n_blocks as u64 != 0 {
        return false;
    }
    match last_snapshot {
        Some(last) if last.0 >= slot.0 => false,
        _ => true,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn should_snapshot_after_block_every_n_returns_true_at_cadence() {
        let cadence = SnapshotCadence { every_n_blocks: 10 };
        assert!(should_snapshot_after_block(
            SlotNo(1000),
            BlockNo(100),
            cadence,
            None,
        ));
        assert!(should_snapshot_after_block(
            SlotNo(1000),
            BlockNo(110),
            cadence,
            Some(SlotNo(900)),
        ));
    }

    #[test]
    fn should_snapshot_after_block_returns_false_off_cadence() {
        let cadence = SnapshotCadence { every_n_blocks: 10 };
        assert!(!should_snapshot_after_block(
            SlotNo(1000),
            BlockNo(101),
            cadence,
            None,
        ));
        assert!(!should_snapshot_after_block(
            SlotNo(1000),
            BlockNo(105),
            cadence,
            None,
        ));
    }

    #[test]
    fn should_snapshot_after_block_returns_false_when_already_at_or_after_slot() {
        let cadence = SnapshotCadence { every_n_blocks: 10 };
        // last_snapshot at the same slot → no.
        assert!(!should_snapshot_after_block(
            SlotNo(1000),
            BlockNo(100),
            cadence,
            Some(SlotNo(1000)),
        ));
        // last_snapshot beyond → no.
        assert!(!should_snapshot_after_block(
            SlotNo(1000),
            BlockNo(100),
            cadence,
            Some(SlotNo(2000)),
        ));
    }

    #[test]
    fn should_snapshot_after_block_is_pure() {
        // Same inputs → same decision across many calls.
        let cadence = SnapshotCadence::DEFAULT;
        let inputs: Vec<(SlotNo, BlockNo, Option<SlotNo>)> = (0u64..200)
            .map(|i| (SlotNo(i * 10), BlockNo(i), Some(SlotNo((i / 100) * 1000))))
            .collect();
        let run = || -> Vec<bool> {
            inputs
                .iter()
                .map(|(s, b, last)| should_snapshot_after_block(*s, *b, cadence, *last))
                .collect()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn snapshot_cadence_default_is_100_blocks() {
        assert_eq!(SnapshotCadence::DEFAULT.every_n_blocks, 100);
    }
}
