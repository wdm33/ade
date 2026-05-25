// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN block source adapter over `ChainDb` (PHASE4-N-I S4).
//!
//! Wraps any `ChainDb` impl and exposes the BLUE `BlockSource`
//! interface. `blocks_in_range(from_exclusive, to_inclusive)` calls
//! `ChainDb::iter_from_slot(from_exclusive + 1)` and collects until
//! the iterator yields a slot > to_inclusive.

use ade_ledger::rollback::BlockSource;
use ade_types::SlotNo;

use crate::chaindb::ChainDb;

/// Block source backed by a borrowed `ChainDb`. Produces the
/// `(slot, bytes)` pairs in BTreeMap-equivalent order.
pub struct ChainDbBlockSource<'a, D: ChainDb> {
    pub db: &'a D,
}

impl<'a, D: ChainDb> ChainDbBlockSource<'a, D> {
    pub fn new(db: &'a D) -> Self {
        Self { db }
    }
}

impl<'a, D: ChainDb> BlockSource for ChainDbBlockSource<'a, D> {
    fn blocks_in_range(
        &self,
        from_exclusive: SlotNo,
        to_inclusive: SlotNo,
    ) -> Vec<(SlotNo, Vec<u8>)> {
        // ChainDb::iter_from_slot is inclusive on the lower bound;
        // we want STRICTLY greater than from_exclusive. Start at
        // `from_exclusive + 1` (saturating).
        let from_inclusive = SlotNo(from_exclusive.0.saturating_add(1));
        let mut out: Vec<(SlotNo, Vec<u8>)> = Vec::new();
        let iter = match self.db.iter_from_slot(from_inclusive) {
            Ok(it) => it,
            Err(_) => return out,
        };
        for item in iter {
            let stored = match item {
                Ok(b) => b,
                Err(_) => break,
            };
            if stored.slot.0 > to_inclusive.0 {
                break;
            }
            out.push((stored.slot, stored.bytes));
        }
        out
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::chaindb::{InMemoryChainDb, StoredBlock};
    use ade_types::Hash32;

    fn block(slot: u64, h: u8) -> StoredBlock {
        StoredBlock {
            slot: SlotNo(slot),
            hash: Hash32([h; 32]),
            bytes: vec![h, slot as u8],
        }
    }

    #[test]
    fn chaindb_block_source_inclusive_upper_exclusive_lower() {
        let db = InMemoryChainDb::new();
        for s in [10u64, 20, 30, 40] {
            db.put_block(&block(s, s as u8)).expect("put");
        }
        let source = ChainDbBlockSource::new(&db);
        let got: Vec<u64> = source
            .blocks_in_range(SlotNo(20), SlotNo(40))
            .into_iter()
            .map(|(s, _)| s.0)
            .collect();
        assert_eq!(got, vec![30, 40]);
    }

    #[test]
    fn chaindb_block_source_empty_when_no_blocks() {
        let db = InMemoryChainDb::new();
        let source = ChainDbBlockSource::new(&db);
        assert!(source
            .blocks_in_range(SlotNo(0), SlotNo(100))
            .is_empty());
    }

    #[test]
    fn chaindb_block_source_returns_bytes_byte_identical() {
        let db = InMemoryChainDb::new();
        let b = block(50, 0xAB);
        let original = b.bytes.clone();
        db.put_block(&b).expect("put");
        let source = ChainDbBlockSource::new(&db);
        let got = source.blocks_in_range(SlotNo(49), SlotNo(50));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1, original);
    }
}
