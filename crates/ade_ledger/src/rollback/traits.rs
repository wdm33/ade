// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE narrow read-only traits for the rollback driver
//! (PHASE4-N-I S1).
//!
//! Both traits are minimal — single method each — and read-only.
//! Production impls live in `ade_runtime::rollback` (S4 GREEN).

use ade_core::consensus::praos_state::PraosChainDepState;
use ade_types::SlotNo;

use crate::state::LedgerState;

/// Snapshot lookup: given a target slot, return the largest
/// `(snapshot_slot, ledger, chain_dep)` triple whose
/// `snapshot_slot ≤ target_slot`, or `None`.
///
/// Returns owned `LedgerState` + `PraosChainDepState` because the
/// materialize driver folds `apply_block_with_verdicts` over them
/// and needs mutability. Production impl (`InMemorySnapshotCache`)
/// clones on lookup; persistent impls would decode from bytes.
pub trait SnapshotReader {
    fn nearest_le(
        &self,
        target_slot: SlotNo,
    ) -> Option<(SlotNo, LedgerState, PraosChainDepState)>;
}

/// Block iterator for replay-forward: ordered `(slot, block_bytes)`
/// for slots strictly greater than `from_exclusive` and ≤
/// `to_inclusive`. The materialize driver consumes the bytes via
/// `apply_block_with_verdicts`.
pub trait BlockSource {
    fn blocks_in_range(
        &self,
        from_exclusive: SlotNo,
        to_inclusive: SlotNo,
    ) -> Vec<(SlotNo, Vec<u8>)>;
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    /// Mock impls to prove the traits are object-safe.
    struct EmptyReader;
    impl SnapshotReader for EmptyReader {
        fn nearest_le(
            &self,
            _target_slot: SlotNo,
        ) -> Option<(SlotNo, LedgerState, PraosChainDepState)> {
            None
        }
    }

    struct EmptySource;
    impl BlockSource for EmptySource {
        fn blocks_in_range(
            &self,
            _from_exclusive: SlotNo,
            _to_inclusive: SlotNo,
        ) -> Vec<(SlotNo, Vec<u8>)> {
            Vec::new()
        }
    }

    #[test]
    fn snapshot_reader_trait_is_object_safe() {
        let r = EmptyReader;
        let _: &dyn SnapshotReader = &r;
        assert!(r.nearest_le(SlotNo(0)).is_none());
    }

    #[test]
    fn block_source_trait_is_object_safe() {
        let s = EmptySource;
        let _: &dyn BlockSource = &s;
        assert!(s.blocks_in_range(SlotNo(0), SlotNo(100)).is_empty());
    }
}
