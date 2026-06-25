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

use super::{
    BlockIter, CappedSlotRange, ChainDb, ChainDbError, ChainTip, SnapshotStore, StoredBlock,
};

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
    // ECA-5: anchor-fp-keyed bootstrap bridge authority (the seed+1 leadership). Disjoint map.
    bootstrap_bridge: BTreeMap<Hash32, Vec<u8>>,
    // Anchor-fp-keyed recovered anchor-point provenance record (AK-S1,
    // DC-NODE-31). Disjoint from both `snapshots` and
    // `seed_consensus_inputs`; keyed by the same 32-byte anchor fingerprint.
    recovered_anchor_points: BTreeMap<Hash32, Vec<u8>>,
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

    fn range_bytes_capped(
        &self,
        from: SlotNo,
        to: SlotNo,
        max: usize,
    ) -> Result<CappedSlotRange, ChainDbError> {
        // An inverted range (from > to) is a malformed (peer-controllable)
        // request — return empty, NEVER let it reach `BTreeMap::range`, which
        // panics when start > end. Keeps parity with PersistentChainDb (redb
        // treats start > end as empty). DC-SERVEMEM-01.
        if from > to {
            return Ok(CappedSlotRange::default());
        }
        let inner = self.inner.lock().map_err(lock_poisoned)?;
        // Bounded, hash-free: read at most `max` in-range blocks (stop after a
        // (max+1)-th proves the request exceeded the cap). DC-SERVEMEM-01.
        let mut out: Vec<(SlotNo, Vec<u8>)> = Vec::new();
        let mut truncated = false;
        for (slot, b) in inner.by_slot.range(from..=to) {
            if out.len() == max {
                truncated = true;
                break;
            }
            out.push((*slot, b.bytes.clone()));
        }
        Ok(CappedSlotRange { blocks: out, truncated })
    }

    fn last_block_bytes(&self) -> Result<Option<(SlotNo, Vec<u8>)>, ChainDbError> {
        let inner = self.inner.lock().map_err(lock_poisoned)?;
        Ok(inner
            .by_slot
            .values()
            .next_back()
            .map(|b| (b.slot, b.bytes.clone())))
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

    fn put_bootstrap_next_epoch_authority(
        &self,
        anchor_fp: &Hash32,
        bytes: &[u8],
    ) -> Result<(), ChainDbError> {
        let mut inner = self.inner.lock().map_err(lock_poisoned)?;
        if let Some(existing) = inner.bootstrap_bridge.get(anchor_fp) {
            if existing.as_slice() != bytes {
                return Err(ChainDbError::InvalidOperation(format!(
                    "bootstrap bridge for anchor_fp {anchor_fp} already occupied by different bytes",
                )));
            }
            return Ok(());
        }
        inner
            .bootstrap_bridge
            .insert(anchor_fp.clone(), bytes.to_vec());
        Ok(())
    }

    fn get_bootstrap_next_epoch_authority(
        &self,
        anchor_fp: &Hash32,
    ) -> Result<Option<Vec<u8>>, ChainDbError> {
        let inner = self.inner.lock().map_err(lock_poisoned)?;
        Ok(inner.bootstrap_bridge.get(anchor_fp).cloned())
    }

    fn list_seed_epoch_consensus_anchor_fps(&self) -> Result<Vec<Hash32>, ChainDbError> {
        let inner = self.inner.lock().map_err(lock_poisoned)?;
        // BTreeMap keys are already ascending; clone into an owned Vec.
        Ok(inner.seed_consensus_inputs.keys().cloned().collect())
    }

    fn put_recovered_anchor_point(
        &self,
        anchor_fp: &Hash32,
        bytes: &[u8],
    ) -> Result<(), ChainDbError> {
        let mut inner = self.inner.lock().map_err(lock_poisoned)?;
        if let Some(existing) = inner.recovered_anchor_points.get(anchor_fp) {
            if existing.as_slice() != bytes {
                return Err(ChainDbError::InvalidOperation(format!(
                    "recovered anchor point for anchor_fp {anchor_fp} already occupied by different bytes",
                )));
            }
            return Ok(());
        }
        inner
            .recovered_anchor_points
            .insert(anchor_fp.clone(), bytes.to_vec());
        Ok(())
    }

    fn get_recovered_anchor_point(
        &self,
        anchor_fp: &Hash32,
    ) -> Result<Option<Vec<u8>>, ChainDbError> {
        let inner = self.inner.lock().map_err(lock_poisoned)?;
        Ok(inner.recovered_anchor_points.get(anchor_fp).cloned())
    }
}
