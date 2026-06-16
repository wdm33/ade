// Core Contract:
// - Deterministic: same ops => byte-identical effective set + fingerprint
// - No wall-clock, randomness, HashMap/HashSet, or floats
// - Canonical (BTreeMap) ordering for all iteration

//! MEM-OPT-UTXO-DISK S2a: the overlay-capable UTxO representation.
//!
//! An `Arc`-shared immutable **anchor** + a bounded in-memory **overlay** of the
//! diffs since the anchor. The effective set = the anchor with the overlay
//! applied. This replaces the raw `BTreeMap` clone-per-mutation model:
//!   - **clone** is O(overlay) (the anchor `Arc` is shared, not copied),
//!   - **mutation** is amortized O(1) (append to the overlay -- a tombstone for a
//!     delete), never a full-map clone,
//!   - **lookup** is overlay -> anchor (owned value, the S1 interface).
//!
//! The anchor is in-memory here (S2a: `Arc<BTreeMap>`); S2b swaps it for the
//! on-disk redb table behind the same surface. The overlay is BOUNDED
//! (`DC-MEM-07`): exceeding `MAX_OVERLAY_ENTRIES` folds it into a fresh anchor
//! (compaction), keeping the in-memory diff bounded. Determinism: every iteration
//! is canonical `TxIn` order (BTreeMap), independent of the anchor/overlay split.

use std::collections::BTreeMap;
use std::sync::Arc;

use ade_types::tx::TxIn;

use crate::utxo::TxOut;

/// `DC-MEM-07`: the fixed, closed, non-configurable bound on the in-memory
/// overlay. The k-deep changelog never grows past this before compacting.
pub const MAX_OVERLAY_ENTRIES: usize = 100_000;

/// An anchor (shared, immutable) + a bounded overlay of diffs since the anchor.
#[derive(Clone, Debug)]
pub struct OverlayUtxo {
    anchor: Arc<BTreeMap<TxIn, TxOut>>,
    /// `Some(out)` = inserted/updated; `None` = deleted (tombstone).
    overlay: BTreeMap<TxIn, Option<TxOut>>,
}

impl OverlayUtxo {
    /// An empty UTxO set.
    pub fn new() -> Self {
        OverlayUtxo {
            anchor: Arc::new(BTreeMap::new()),
            overlay: BTreeMap::new(),
        }
    }

    /// Build from a full map (e.g. seed import / snapshot decode) -- the map
    /// becomes the anchor; the overlay starts empty.
    pub fn from_map(map: BTreeMap<TxIn, TxOut>) -> Self {
        OverlayUtxo {
            anchor: Arc::new(map),
            overlay: BTreeMap::new(),
        }
    }

    /// Resolve an input to its output (owned), or `None` if absent/deleted.
    pub fn get(&self, tx_in: &TxIn) -> Option<TxOut> {
        match self.overlay.get(tx_in) {
            Some(Some(out)) => Some(out.clone()),
            Some(None) => None, // tombstone -- deleted in the overlay
            None => self.anchor.get(tx_in).cloned(),
        }
    }

    /// Whether a live entry exists.
    pub fn contains(&self, tx_in: &TxIn) -> bool {
        self.get(tx_in).is_some()
    }

    /// Insert/update -- append to the overlay (amortized O(1)).
    pub fn insert(&mut self, tx_in: TxIn, tx_out: TxOut) {
        self.overlay.insert(tx_in, Some(tx_out));
        self.maybe_compact();
    }

    /// Remove -- a tombstone in the overlay; returns the removed value (or `None`
    /// if the entry was absent, in which case nothing is recorded).
    pub fn remove(&mut self, tx_in: &TxIn) -> Option<TxOut> {
        let current = self.get(tx_in);
        if current.is_some() {
            self.overlay.insert(tx_in.clone(), None);
            self.maybe_compact();
        }
        current
    }

    /// The number of live entries (O(overlay) -- the overlay is bounded).
    pub fn len(&self) -> usize {
        let mut n = self.anchor.len();
        for (tx_in, slot) in &self.overlay {
            match (slot, self.anchor.contains_key(tx_in)) {
                (Some(_), false) => n += 1, // a new insert
                (None, true) => n -= 1,     // a delete of an anchor entry
                _ => {}                     // an update, or a no-op
            }
        }
        n
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The effective sorted set -- the anchor with the overlay applied. O(anchor +
    /// overlay). Used for iteration (fingerprint / snapshot -- checkpoint-time,
    /// never the per-block hot path, which is O(1)/O(overlay)).
    pub fn to_map(&self) -> BTreeMap<TxIn, TxOut> {
        let mut map = (*self.anchor).clone();
        for (tx_in, slot) in &self.overlay {
            match slot {
                Some(out) => {
                    map.insert(tx_in.clone(), out.clone());
                }
                None => {
                    map.remove(tx_in);
                }
            }
        }
        map
    }

    /// Iterate the effective set in canonical `TxIn` order (owned entries).
    pub fn iter(&self) -> impl Iterator<Item = (TxIn, TxOut)> {
        self.to_map().into_iter()
    }

    /// Fold the overlay into a FRESH anchor and clear it. The effective set is
    /// unchanged; the in-memory overlay returns to empty. Called automatically
    /// when the overlay exceeds the bound, and available explicitly for the
    /// per-block durable-commit point (S2b).
    pub fn compact(&mut self) {
        if self.overlay.is_empty() {
            return;
        }
        self.anchor = Arc::new(self.to_map());
        self.overlay.clear();
    }

    fn maybe_compact(&mut self) {
        if self.overlay.len() > MAX_OVERLAY_ENTRIES {
            self.compact();
        }
    }
}

impl Default for OverlayUtxo {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for OverlayUtxo {
    /// Equality is on the EFFECTIVE set -- two stores with the same live entries
    /// are equal regardless of their anchor/overlay split (so replay-equivalence
    /// and fingerprints never depend on the split).
    fn eq(&self, other: &Self) -> bool {
        self.to_map() == other.to_map()
    }
}
impl Eq for OverlayUtxo {}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_types::address::Address;
    use ade_types::tx::Coin;
    use ade_types::Hash32;

    fn txin(h: u8, i: u16) -> TxIn {
        TxIn {
            tx_hash: Hash32([h; 32]),
            index: i,
        }
    }
    fn out(c: u64, t: u8) -> TxOut {
        TxOut::Byron {
            address: Address::Byron(vec![t]),
            coin: Coin(c),
        }
    }

    /// The load-bearing S2a proof: across an arbitrary insert/remove sequence the
    /// overlay store's effective set + every observation matches a reference
    /// BTreeMap. Includes compaction interleaved (forced) -- which must NOT change
    /// the effective set.
    #[test]
    fn overlay_matches_btreemap_across_a_sequence() {
        let mut store = OverlayUtxo::new();
        let mut model: BTreeMap<TxIn, TxOut> = BTreeMap::new();

        // op = (is_insert, h, i, coin)
        let ops: &[(bool, u8, u16, u64)] = &[
            (true, 0x01, 0, 100),
            (true, 0x02, 0, 200),
            (true, 0x03, 7, 300),
            (false, 0x01, 0, 0), // remove 0x01
            (true, 0x10, 0, 40),
            (true, 0x10, 1, 60),
            (true, 0x02, 0, 999), // update 0x02
            (false, 0x10, 0, 0),  // remove 0x10:0
            (false, 0xff, 9, 0),  // remove absent -- no-op
            (true, 0x20, 3, 95),
        ];
        for (idx, (is_insert, h, i, c)) in ops.iter().enumerate() {
            let ti = txin(*h, *i);
            if *is_insert {
                store.insert(ti.clone(), out(*c, *h));
                model.insert(ti.clone(), out(*c, *h));
            } else {
                let a = store.remove(&ti);
                let b = model.remove(&ti);
                assert_eq!(a, b, "remove return mismatch at op {idx}");
            }
            // force a compaction halfway -- the effective set must be invariant.
            if idx == 4 {
                store.compact();
            }
            assert_eq!(store.to_map(), model, "effective set mismatch at op {idx}");
            assert_eq!(store.len(), model.len(), "len mismatch at op {idx}");
            for (k, v) in &model {
                assert_eq!(store.get(k).as_ref(), Some(v), "get mismatch at op {idx}");
                assert!(store.contains(k));
            }
            assert_eq!(store.get(&txin(0xee, 0)), None, "absent get must be None");
        }
        // iteration order is canonical (== the BTreeMap's).
        let iterated: Vec<_> = store.iter().collect();
        let expected: Vec<_> = model.into_iter().collect();
        assert_eq!(iterated, expected, "iteration order must be canonical");
    }

    #[test]
    fn compact_preserves_effective_set_and_clears_overlay() {
        let mut store = OverlayUtxo::from_map(
            [(txin(0xaa, 0), out(1, 0xaa)), (txin(0xbb, 0), out(2, 0xbb))]
                .into_iter()
                .collect(),
        );
        store.insert(txin(0xcc, 0), out(3, 0xcc));
        store.remove(&txin(0xaa, 0));
        let before = store.to_map();
        store.compact();
        assert_eq!(store.to_map(), before, "compaction must not change the set");
        assert_eq!(store.overlay.len(), 0, "overlay cleared after compaction");
        assert!(store.contains(&txin(0xcc, 0)) && !store.contains(&txin(0xaa, 0)));
    }

    #[test]
    fn clone_shares_anchor_and_is_independent() {
        let store = OverlayUtxo::from_map([(txin(0x01, 0), out(10, 1))].into_iter().collect());
        let mut cloned = store.clone();
        // the clone shares the same anchor Arc (cheap clone).
        assert!(Arc::ptr_eq(&store.anchor, &cloned.anchor));
        cloned.insert(txin(0x02, 0), out(20, 2));
        // mutating the clone does not affect the original.
        assert!(store.get(&txin(0x02, 0)).is_none());
        assert!(cloned.get(&txin(0x02, 0)).is_some());
    }
}
