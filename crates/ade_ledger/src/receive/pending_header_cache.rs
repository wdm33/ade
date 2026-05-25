// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE pending-header cache for the receive bridge (PHASE4-N-H S1).
//!
//! `BTreeMap<(SlotNo, Hash32), Vec<u8>>` — canonical iteration, no
//! `HashMap`. Insertion is idempotent on byte-identity at the same
//! key; byte-divergence at the same key is a structural conflict
//! (cryptographically unreachable under blake2b_256 header hashing,
//! but the invariant is explicit).
//!
//! Eviction is NOT a concern of this BLUE type — the orchestrator
//! (S4) decides eviction policy as canonical input and may call
//! `evict_below(slot)` to drop entries below a stale-cutoff.

use std::collections::BTreeMap;

use ade_types::{Hash32, SlotNo};

/// Canonical, BTreeMap-backed cache mapping `(slot, header_hash)` to
/// the announced header bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PendingHeaderCache {
    entries: BTreeMap<(SlotNo, Hash32), Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingHeaderCacheError {
    /// Inserting different bytes under an existing `(slot, hash)`
    /// key. Cryptographically unreachable under blake2b_256 header
    /// hashing; the variant exists to make the invariant explicit.
    ByteConflict { slot: SlotNo, hash: Hash32 },
}

impl PendingHeaderCache {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Insert a `(slot, hash) -> header_bytes` entry. Idempotent on
    /// byte-identity. Returns `ByteConflict` on byte-divergence at
    /// an existing key.
    pub fn insert(
        &mut self,
        slot: SlotNo,
        hash: Hash32,
        header_bytes: Vec<u8>,
    ) -> Result<(), PendingHeaderCacheError> {
        let key = (slot, hash.clone());
        if let Some(existing) = self.entries.get(&key) {
            if existing == &header_bytes {
                return Ok(());
            }
            return Err(PendingHeaderCacheError::ByteConflict { slot, hash });
        }
        self.entries.insert(key, header_bytes);
        Ok(())
    }

    /// Lookup the header bytes cached at `(slot, hash)`.
    pub fn get(&self, slot: SlotNo, hash: &Hash32) -> Option<&[u8]> {
        self.entries.get(&(slot, hash.clone())).map(Vec::as_slice)
    }

    /// Whether `(slot, hash)` is present.
    pub fn contains(&self, slot: SlotNo, hash: &Hash32) -> bool {
        self.entries.contains_key(&(slot, hash.clone()))
    }

    /// Remove a single `(slot, hash)` entry. Returns the bytes if
    /// present. Used by the reducer (S2) to evict a consumed header
    /// after successful admission.
    pub fn remove(&mut self, slot: SlotNo, hash: &Hash32) -> Option<Vec<u8>> {
        self.entries.remove(&(slot, hash.clone()))
    }

    /// Drop entries whose slot is strictly less than `slot`.
    /// Deterministic; used by the orchestrator (S4) as canonical
    /// eviction input.
    pub fn evict_below(&mut self, slot: SlotNo) {
        self.entries = self
            .entries
            .split_off(&(slot, Hash32([0u8; 32])));
    }

    /// Iterate `(slot, hash, bytes)` in BTreeMap order.
    pub fn iter(&self) -> impl Iterator<Item = (SlotNo, &'_ Hash32, &'_ [u8])> + '_ {
        self.entries
            .iter()
            .map(|((slot, hash), bytes)| (*slot, hash, bytes.as_slice()))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn pending_header_cache_insert_and_lookup() {
        let mut c = PendingHeaderCache::new();
        let h = Hash32([0x01; 32]);
        c.insert(SlotNo(10), h.clone(), vec![0x40]).expect("insert");
        assert_eq!(c.len(), 1);
        let got = c.get(SlotNo(10), &h).expect("present");
        assert_eq!(got, &[0x40][..]);
    }

    #[test]
    fn pending_header_cache_insert_is_idempotent_on_byte_identity() {
        let mut c = PendingHeaderCache::new();
        let h = Hash32([0x01; 32]);
        c.insert(SlotNo(10), h.clone(), vec![0x40]).expect("first");
        c.insert(SlotNo(10), h.clone(), vec![0x40]).expect("second-idempotent");
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn pending_header_cache_insert_rejects_byte_conflict() {
        let mut c = PendingHeaderCache::new();
        let h = Hash32([0x01; 32]);
        c.insert(SlotNo(10), h.clone(), vec![0x40]).expect("first");
        let err = c
            .insert(SlotNo(10), h.clone(), vec![0x41])
            .expect_err("conflict");
        match err {
            PendingHeaderCacheError::ByteConflict { slot, .. } => {
                assert_eq!(slot, SlotNo(10))
            }
        }
    }

    #[test]
    fn pending_header_cache_iteration_is_btreemap_ordered() {
        let mut c = PendingHeaderCache::new();
        c.insert(SlotNo(30), Hash32([0x33; 32]), vec![0x30]).unwrap();
        c.insert(SlotNo(10), Hash32([0x11; 32]), vec![0x10]).unwrap();
        c.insert(SlotNo(20), Hash32([0x22; 32]), vec![0x20]).unwrap();
        let slots: Vec<SlotNo> = c.iter().map(|(s, _, _)| s).collect();
        assert_eq!(slots, vec![SlotNo(10), SlotNo(20), SlotNo(30)]);
    }

    #[test]
    fn pending_header_cache_evict_below_drops_lower_slots() {
        let mut c = PendingHeaderCache::new();
        c.insert(SlotNo(10), Hash32([0x11; 32]), vec![0x10]).unwrap();
        c.insert(SlotNo(20), Hash32([0x22; 32]), vec![0x20]).unwrap();
        c.insert(SlotNo(30), Hash32([0x33; 32]), vec![0x30]).unwrap();
        c.evict_below(SlotNo(20));
        let slots: Vec<SlotNo> = c.iter().map(|(s, _, _)| s).collect();
        assert_eq!(slots, vec![SlotNo(20), SlotNo(30)]);
    }
}
