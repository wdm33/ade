// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE closed error sums for the rollback driver (PHASE4-N-I S1).

use ade_types::{CardanoEra, SlotNo};

use crate::block_validity::BlockValidityError;
use crate::receive::ChainWriteError;

/// Closed materialize-error sum. Three variants; no `String`, no
/// `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq)]
pub enum MaterializeError {
    /// No snapshot ≤ `target_slot` exists. `oldest_snapshot` is the
    /// smallest available snapshot slot, or `None` if the cache is
    /// empty. Receive state stays unchanged; orchestrator halts the
    /// peer.
    RollbackTooDeep {
        target_slot: SlotNo,
        oldest_snapshot: Option<SlotNo>,
    },
    /// Replay-forward encountered a block at `slot` whose
    /// `apply_block_with_verdicts` returned `error`. Receive state
    /// stays unchanged.
    ReplayFailedAt {
        slot: SlotNo,
        error: BlockValidityError,
    },
    /// Pre-Conway era encountered during replay (PHASE4-N-I scope
    /// limited to Conway). Receive state stays unchanged.
    EraNotSupported {
        era: CardanoEra,
        slot: SlotNo,
    },
}

/// Closed commit-rollback-error sum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitRollbackError {
    /// The underlying ChainDb's rollback or write failed. Receive
    /// state stays unchanged (irreversible step is checked first).
    ChainDb(ChainWriteError),
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::receive::ChainWriteErrorKind;
    use ade_types::Hash32;

    #[test]
    fn materialize_error_round_trips_through_pattern_match() {
        let errs = vec![
            MaterializeError::RollbackTooDeep {
                target_slot: SlotNo(100),
                oldest_snapshot: Some(SlotNo(50)),
            },
            MaterializeError::EraNotSupported {
                era: CardanoEra::ByronEbb,
                slot: SlotNo(10),
            },
        ];
        for e in errs {
            // Exhaustive match — fourth variant would not compile
            // without an arm here.
            match e {
                MaterializeError::RollbackTooDeep { .. } => {}
                MaterializeError::ReplayFailedAt { .. } => {}
                MaterializeError::EraNotSupported { .. } => {}
            }
        }
    }

    #[test]
    fn commit_rollback_error_round_trips_through_pattern_match() {
        let errs = vec![CommitRollbackError::ChainDb(
            ChainWriteError::SlotConflict {
                slot: SlotNo(100),
                hash: Hash32([0xAB; 32]),
            },
        )];
        for e in errs {
            match e {
                CommitRollbackError::ChainDb(_) => {}
            }
        }
        // Also exercise the kind enum to keep the type referenced.
        let _ = ChainWriteErrorKind::Io;
    }
}
