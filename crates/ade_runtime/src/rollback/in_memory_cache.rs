// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN in-memory snapshot cache (PHASE4-N-I S4).
//!
//! `InMemorySnapshotCache` is a `BTreeMap<SlotNo, (LedgerState,
//! PraosChainDepState)>` impl of `SnapshotReader`. The persistent
//! variant (round-trippable encoder + decoder over `SnapshotStore`)
//! is the follow-on cluster's deliverable (DC-CONS-21).

use std::collections::BTreeMap;

use ade_core::consensus::praos_state::PraosChainDepState;
use ade_ledger::receive::ReceiveState;
use ade_ledger::rollback::SnapshotReader;
use ade_ledger::state::LedgerState;
use ade_types::SlotNo;

/// In-memory snapshot cache. BTreeMap-backed canonical iteration.
#[derive(Debug, Default, Clone)]
pub struct InMemorySnapshotCache {
    entries: BTreeMap<SlotNo, (LedgerState, PraosChainDepState)>,
}

impl InMemorySnapshotCache {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Insert (or overwrite) a snapshot at `slot`. Used by the
    /// snapshot-write orchestration (S5) after each admission that
    /// the cadence policy elects.
    pub fn admit(
        &mut self,
        slot: SlotNo,
        ledger: LedgerState,
        chain_dep: PraosChainDepState,
    ) {
        self.entries.insert(slot, (ledger, chain_dep));
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Smallest snapshot slot present, or None.
    pub fn oldest(&self) -> Option<SlotNo> {
        self.entries.keys().next().copied()
    }

    /// Largest snapshot slot present, or None. Used by the
    /// snapshot-write hook (S5) to seed the cadence policy's
    /// `last_snapshot` argument.
    pub fn most_recent(&self) -> Option<SlotNo> {
        self.entries.keys().next_back().copied()
    }

    /// All slot keys in ascending order. Read-only.
    pub fn slots(&self) -> Vec<SlotNo> {
        self.entries.keys().copied().collect()
    }

    /// Iterate (slot, ()) pairs for test inspection. Avoids
    /// exposing the inner state tuple read-only externally — most
    /// tests just need the slot set.
    pub fn iter_for_test(&self) -> Vec<(SlotNo, ())> {
        self.entries.keys().map(|s| (*s, ())).collect()
    }

    /// Convenience: capture the per-peer receive state's (ledger,
    /// chain_dep) into a snapshot at `slot`. Used by S5's
    /// orchestrator hook.
    pub fn capture_from(&mut self, slot: SlotNo, state: &ReceiveState) {
        self.admit(slot, state.ledger.clone(), state.chain_dep.clone());
    }
}

impl SnapshotReader for InMemorySnapshotCache {
    fn nearest_le(
        &self,
        target_slot: SlotNo,
    ) -> Option<(SlotNo, LedgerState, PraosChainDepState)> {
        // BTreeMap::range gives in-order iteration; take the
        // largest key ≤ target_slot.
        let mut last: Option<(SlotNo, &(LedgerState, PraosChainDepState))> = None;
        for (k, v) in self.entries.range(..=target_slot) {
            last = Some((*k, v));
        }
        last.map(|(k, (l, c))| (k, l.clone(), c.clone()))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use ade_types::{CardanoEra, EpochNo, Hash32};

    fn make_state(epoch: u64) -> (LedgerState, PraosChainDepState) {
        let mut l = LedgerState::new(CardanoEra::Conway);
        l.epoch_state.epoch = EpochNo(epoch);
        let mut s = PraosChainDepState::empty();
        s.epoch_nonce = ade_core::consensus::Nonce(Hash32([epoch as u8; 32]));
        (l, s)
    }

    #[test]
    fn in_memory_snapshot_cache_nearest_le_returns_largest_key() {
        let mut cache = InMemorySnapshotCache::new();
        cache.admit(SlotNo(100), make_state(576).0, make_state(576).1);
        cache.admit(SlotNo(200), make_state(577).0, make_state(577).1);
        cache.admit(SlotNo(300), make_state(578).0, make_state(578).1);

        let (slot, _l, cd) = cache.nearest_le(SlotNo(250)).expect("found");
        assert_eq!(slot, SlotNo(200));
        assert_eq!(cd.epoch_nonce.0 .0[0], (577u16 as u8));

        let (slot, _, _) = cache.nearest_le(SlotNo(300)).expect("found");
        assert_eq!(slot, SlotNo(300));

        let (slot, _, _) = cache.nearest_le(SlotNo(99999)).expect("found");
        assert_eq!(slot, SlotNo(300));

        assert!(cache.nearest_le(SlotNo(50)).is_none());
    }

    #[test]
    fn in_memory_snapshot_cache_iteration_is_btreemap_ordered() {
        let mut cache = InMemorySnapshotCache::new();
        cache.admit(SlotNo(300), make_state(578).0, make_state(578).1);
        cache.admit(SlotNo(100), make_state(576).0, make_state(576).1);
        cache.admit(SlotNo(200), make_state(577).0, make_state(577).1);
        let slots: Vec<SlotNo> = cache.entries.keys().copied().collect();
        assert_eq!(slots, vec![SlotNo(100), SlotNo(200), SlotNo(300)]);
    }

    #[test]
    fn in_memory_snapshot_cache_empty_returns_none() {
        let cache = InMemorySnapshotCache::new();
        assert!(cache.nearest_le(SlotNo(0)).is_none());
        assert!(cache.is_empty());
        assert!(cache.oldest().is_none());
    }

    #[test]
    fn in_memory_snapshot_cache_oldest_returns_smallest_slot() {
        let mut cache = InMemorySnapshotCache::new();
        cache.admit(SlotNo(300), make_state(578).0, make_state(578).1);
        cache.admit(SlotNo(100), make_state(576).0, make_state(576).1);
        assert_eq!(cache.oldest(), Some(SlotNo(100)));
    }
}
