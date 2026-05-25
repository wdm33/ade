// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED snapshot-write hook for the receive flow (PHASE4-N-I S5).
//!
//! Pure-function hook the caller invokes after each
//! `dispatch_*_inbound` call. Inspects the `ReceiveEffect`; on
//! `Admitted`, consults the cadence policy and captures the per-
//! peer (ledger, chain_dep) into the in-memory cache when due.
//!
//! Classified RED because it composes RED dispatch outcomes with
//! the GREEN cadence + cache. The decision logic itself is pure
//! (no I/O, no clock); BLUE-eligible mechanically but RED-shaped
//! by location.

use ade_ledger::receive::{ReceiveEffect, ReceiveState};

use crate::rollback::cadence::{should_snapshot_after_block, SnapshotCadence};
use crate::rollback::in_memory_cache::InMemorySnapshotCache;

/// If `effect` is `Admitted` and the cadence policy elects to
/// snapshot at this block, capture `(state.ledger, state.chain_dep)`
/// into `cache` at the admitted slot.
///
/// Returns `true` if a snapshot was captured. Pure decision +
/// in-memory write — no I/O, no clock.
pub fn maybe_capture_snapshot(
    cache: &mut InMemorySnapshotCache,
    cadence: SnapshotCadence,
    effect: &ReceiveEffect,
    state: &ReceiveState,
) -> bool {
    let slot = match effect {
        ReceiveEffect::Admitted { slot, .. } => *slot,
        _ => return false,
    };
    let block_no = match state.chain_dep.last_block_no {
        Some(bn) => bn,
        None => return false,
    };
    let last_snapshot = cache.most_recent();
    if should_snapshot_after_block(slot, block_no, cadence, last_snapshot) {
        cache.capture_from(slot, state);
        return true;
    }
    false
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use ade_core::consensus::praos_state::PraosChainDepState;
    use ade_ledger::receive::ReceiveState;
    use ade_ledger::state::LedgerState;
    use ade_types::{BlockNo, CardanoEra, EpochNo, Hash32, SlotNo};

    fn make_state(block_no: u64) -> ReceiveState {
        let mut ledger = LedgerState::new(CardanoEra::Conway);
        ledger.epoch_state.epoch = EpochNo(576);
        let mut chain_dep = PraosChainDepState::empty();
        chain_dep.last_block_no = Some(BlockNo(block_no));
        chain_dep.last_slot = Some(SlotNo(block_no * 2));
        ReceiveState::new(ledger, chain_dep)
    }

    #[test]
    fn maybe_capture_snapshot_captures_at_cadence() {
        let mut cache = InMemorySnapshotCache::new();
        let cadence = SnapshotCadence { every_n_blocks: 10 };
        let state = make_state(100);
        let effect = ReceiveEffect::Admitted {
            slot: SlotNo(1000),
            hash: Hash32([0xAB; 32]),
        };
        let captured = maybe_capture_snapshot(&mut cache, cadence, &effect, &state);
        assert!(captured);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.most_recent(), Some(SlotNo(1000)));
    }

    #[test]
    fn maybe_capture_snapshot_skips_off_cadence() {
        let mut cache = InMemorySnapshotCache::new();
        let cadence = SnapshotCadence { every_n_blocks: 10 };
        let state = make_state(105);
        let effect = ReceiveEffect::Admitted {
            slot: SlotNo(2000),
            hash: Hash32([0xAB; 32]),
        };
        let captured = maybe_capture_snapshot(&mut cache, cadence, &effect, &state);
        assert!(!captured);
        assert!(cache.is_empty());
    }

    #[test]
    fn maybe_capture_snapshot_only_on_admitted_effect() {
        let mut cache = InMemorySnapshotCache::new();
        let cadence = SnapshotCadence::DEFAULT;
        let state = make_state(100);
        // Cached effect — not Admitted.
        let cached_effect = ReceiveEffect::Cached {
            slot: SlotNo(1000),
            hash: Hash32([0xAB; 32]),
        };
        let captured = maybe_capture_snapshot(&mut cache, cadence, &cached_effect, &state);
        assert!(!captured);
        assert!(cache.is_empty());
    }

    #[test]
    fn maybe_capture_snapshot_deterministic_over_admission_sequence() {
        // Simulate N admissions; assert the set of captured slots
        // is exactly floor(N / every_n_blocks) entries, deterministic
        // across two runs.
        let run = || -> Vec<SlotNo> {
            let mut cache = InMemorySnapshotCache::new();
            let cadence = SnapshotCadence { every_n_blocks: 5 };
            for block_no in 1..=50u64 {
                let state = make_state(block_no);
                let effect = ReceiveEffect::Admitted {
                    slot: SlotNo(block_no * 10),
                    hash: Hash32([(block_no as u8); 32]),
                };
                maybe_capture_snapshot(&mut cache, cadence, &effect, &state);
            }
            cache
                .iter_for_test()
                .into_iter()
                .map(|(s, _)| s)
                .collect()
        };
        let a = run();
        let b = run();
        assert_eq!(a, b, "captures must be deterministic");
        // 50 blocks, cadence 5 → 10 captures at blocks 5,10,...,50.
        assert_eq!(a.len(), 10);
    }
}
