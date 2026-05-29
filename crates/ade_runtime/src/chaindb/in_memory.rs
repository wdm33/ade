// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! In-memory `ChainDb` implementation.
//!
//! Test-only; satisfies the trait's logical durability obligations
//! trivially because nothing leaves process memory. Used to validate
//! the contract test suite and as a stand-in for slices that don't
//! yet need persistence.

use std::collections::BTreeMap;
use std::sync::Mutex;

use ade_types::primitives::{Hash32, SlotNo};

use super::{BlockIter, ChainDb, ChainDbError, ChainTip, SnapshotStore, StoredBlock};

/// In-memory chain database.
///
/// Backed by two BTreeMaps: slot → block (the canonical store) and
/// hash → slot (the lookup index). Both live behind a single Mutex
/// to honor the single-writer multi-reader contract. The Mutex is an
/// implementation detail; callers do not see it.
#[derive(Debug, Default)]
pub struct InMemoryChainDb {
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    by_slot: BTreeMap<SlotNo, StoredBlock>,
    by_hash: BTreeMap<Hash32, SlotNo>,
    snapshots: BTreeMap<SlotNo, Vec<u8>>,
    // Anchor-fp-keyed seed-epoch consensus-inputs sidecar (A2). A map
    // disjoint from `snapshots` above, keyed by the 32-byte anchor
    // fingerprint — never a reserved sentinel slot.
    seed_consensus_inputs: BTreeMap<Hash32, Vec<u8>>,
}

impl InMemoryChainDb {
    pub fn new() -> Self {
        Self::default()
    }
}

fn lock_poisoned<T>(_: std::sync::PoisonError<T>) -> ChainDbError {
    ChainDbError::Corruption("in-memory chaindb mutex poisoned".to_string())
}

impl ChainDb for InMemoryChainDb {
    fn put_block(&self, block: &StoredBlock) -> Result<(), ChainDbError> {
        let mut inner = self.inner.lock().map_err(lock_poisoned)?;
        if let Some(existing) = inner.by_slot.get(&block.slot) {
            if existing.hash != block.hash {
                return Err(ChainDbError::InvalidOperation(format!(
                    "slot {} already occupied by a different block",
                    block.slot.0,
                )));
            }
            // Same block re-put is idempotent.
            return Ok(());
        }
        inner.by_slot.insert(block.slot, block.clone());
        inner.by_hash.insert(block.hash.clone(), block.slot);
        Ok(())
    }

    fn get_block_by_hash(
        &self,
        hash: &Hash32,
    ) -> Result<Option<StoredBlock>, ChainDbError> {
        let inner = self.inner.lock().map_err(lock_poisoned)?;
        Ok(inner
            .by_hash
            .get(hash)
            .and_then(|slot| inner.by_slot.get(slot))
            .cloned())
    }

    fn get_block_by_slot(
        &self,
        slot: SlotNo,
    ) -> Result<Option<StoredBlock>, ChainDbError> {
        let inner = self.inner.lock().map_err(lock_poisoned)?;
        Ok(inner.by_slot.get(&slot).cloned())
    }

    fn tip(&self) -> Result<Option<ChainTip>, ChainDbError> {
        let inner = self.inner.lock().map_err(lock_poisoned)?;
        Ok(inner.by_slot.values().next_back().map(|b| ChainTip {
            hash: b.hash.clone(),
            slot: b.slot,
        }))
    }

    fn iter_from_slot(&self, from: SlotNo) -> Result<BlockIter<'_>, ChainDbError> {
        // Snapshot the matching range while holding the lock; release before
        // returning so the iterator doesn't pin the mutex.
        let inner = self.inner.lock().map_err(lock_poisoned)?;
        let snapshot: Vec<StoredBlock> = inner
            .by_slot
            .range(from..)
            .map(|(_, b)| b.clone())
            .collect();
        drop(inner);
        Ok(Box::new(snapshot.into_iter().map(Ok)))
    }

    fn rollback_to_slot(&self, slot: SlotNo) -> Result<(), ChainDbError> {
        let mut inner = self.inner.lock().map_err(lock_poisoned)?;
        // Collect first (can't mutate while iterating the same map).
        let to_remove: Vec<SlotNo> = inner
            .by_slot
            .range((std::ops::Bound::Excluded(slot), std::ops::Bound::Unbounded))
            .map(|(s, _)| *s)
            .collect();
        for s in to_remove {
            if let Some(block) = inner.by_slot.remove(&s) {
                inner.by_hash.remove(&block.hash);
            }
        }
        Ok(())
    }
}

impl SnapshotStore for InMemoryChainDb {
    fn put_snapshot(
        &self,
        slot: SlotNo,
        bytes: &[u8],
    ) -> Result<(), ChainDbError> {
        let mut inner = self.inner.lock().map_err(lock_poisoned)?;
        if let Some(existing) = inner.snapshots.get(&slot) {
            if existing.as_slice() != bytes {
                return Err(ChainDbError::InvalidOperation(format!(
                    "snapshot at slot {} already occupied by different bytes",
                    slot.0,
                )));
            }
            return Ok(());
        }
        inner.snapshots.insert(slot, bytes.to_vec());
        Ok(())
    }

    fn get_snapshot(&self, slot: SlotNo) -> Result<Option<Vec<u8>>, ChainDbError> {
        let inner = self.inner.lock().map_err(lock_poisoned)?;
        Ok(inner.snapshots.get(&slot).cloned())
    }

    fn latest_snapshot(
        &self,
    ) -> Result<Option<(SlotNo, Vec<u8>)>, ChainDbError> {
        let inner = self.inner.lock().map_err(lock_poisoned)?;
        Ok(inner
            .snapshots
            .iter()
            .next_back()
            .map(|(s, b)| (*s, b.clone())))
    }

    fn list_snapshot_slots(&self) -> Result<Vec<SlotNo>, ChainDbError> {
        let inner = self.inner.lock().map_err(lock_poisoned)?;
        Ok(inner.snapshots.keys().copied().collect())
    }

    fn delete_snapshot(&self, slot: SlotNo) -> Result<(), ChainDbError> {
        let mut inner = self.inner.lock().map_err(lock_poisoned)?;
        inner.snapshots.remove(&slot);
        Ok(())
    }

    fn put_seed_epoch_consensus_inputs(
        &self,
        anchor_fp: &Hash32,
        bytes: &[u8],
    ) -> Result<(), ChainDbError> {
        let mut inner = self.inner.lock().map_err(lock_poisoned)?;
        if let Some(existing) = inner.seed_consensus_inputs.get(anchor_fp) {
            if existing.as_slice() != bytes {
                return Err(ChainDbError::InvalidOperation(format!(
                    "seed-epoch consensus inputs for anchor_fp {anchor_fp} already occupied by different bytes",
                )));
            }
            return Ok(());
        }
        inner
            .seed_consensus_inputs
            .insert(anchor_fp.clone(), bytes.to_vec());
        Ok(())
    }

    fn get_seed_epoch_consensus_inputs(
        &self,
        anchor_fp: &Hash32,
    ) -> Result<Option<Vec<u8>>, ChainDbError> {
        let inner = self.inner.lock().map_err(lock_poisoned)?;
        Ok(inner.seed_consensus_inputs.get(anchor_fp).cloned())
    }
}
