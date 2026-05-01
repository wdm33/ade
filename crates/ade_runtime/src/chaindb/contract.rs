// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Reusable contract test suite for any [`ChainDb`] implementation.
//!
//! Per S-33 obligation discharge §O-33.5: the trait's logical
//! durability and lookup obligations are encoded here so any
//! `ChainDb` impl — in-memory now, persistent later — can be
//! validated against the same gate. Call from a `#[test]` function
//! inside the impl's crate or this crate's test suite.
//!
//! ```ignore
//! #[test]
//! fn my_chaindb_passes_contract() {
//!     ade_runtime::chaindb::run_contract_tests(MyChainDb::new);
//! }
//! ```

use ade_types::primitives::{Hash32, SlotNo};

use super::{ChainDb, StoredBlock};

fn block(slot: u64, hash_byte: u8) -> StoredBlock {
    StoredBlock {
        slot: SlotNo(slot),
        hash: Hash32([hash_byte; 32]),
        bytes: vec![hash_byte; 64],
    }
}

/// Run every assertion in the contract suite against a fresh
/// `ChainDb` produced by `make_db`. Panics on first failure (the
/// usual `#[test]` convention).
///
/// `make_db` is `FnMut` so callers can share state (e.g., a counter
/// for unique persistent paths) across invocations.
pub fn run_contract_tests<D, F>(mut make_db: F)
where
    D: ChainDb,
    F: FnMut() -> D,
{
    empty_db_has_no_tip(&make_db());
    put_then_get_by_hash(&make_db());
    put_then_get_by_slot(&make_db());
    tip_is_highest_slot(&make_db());
    iter_from_slot_yields_in_order(&make_db());
    iter_from_slot_skips_lower(&make_db());
    rollback_removes_higher_blocks(&make_db());
    rollback_preserves_lower_blocks(&make_db());
    rollback_clears_hash_index(&make_db());
    rollback_beyond_tip_is_noop(&make_db());
    same_block_reput_is_idempotent(&make_db());
    conflicting_slot_put_errors(&make_db());
    not_found_is_ok_none(&make_db());
}

fn empty_db_has_no_tip<D: ChainDb>(db: &D) {
    assert!(db.tip().expect("tip ok").is_none());
    assert!(db.iter_from_slot(SlotNo(0)).expect("iter ok").next().is_none());
}

fn put_then_get_by_hash<D: ChainDb>(db: &D) {
    let b = block(10, 0xaa);
    db.put_block(&b).expect("put");
    let got = db.get_block_by_hash(&b.hash).expect("get").expect("present");
    assert_eq!(got, b);
}

fn put_then_get_by_slot<D: ChainDb>(db: &D) {
    let b = block(20, 0xbb);
    db.put_block(&b).expect("put");
    let got = db.get_block_by_slot(b.slot).expect("get").expect("present");
    assert_eq!(got, b);
}

fn tip_is_highest_slot<D: ChainDb>(db: &D) {
    db.put_block(&block(5, 0x01)).expect("put 5");
    db.put_block(&block(20, 0x02)).expect("put 20");
    db.put_block(&block(10, 0x03)).expect("put 10");
    let tip = db.tip().expect("tip").expect("non-empty");
    assert_eq!(tip.slot, SlotNo(20));
    assert_eq!(tip.hash, Hash32([0x02; 32]));
}

fn iter_from_slot_yields_in_order<D: ChainDb>(db: &D) {
    for s in [3u64, 1, 4, 1, 5, 9, 2, 6] {
        let _ = db.put_block(&block(s, s as u8));
    }
    let slots: Vec<u64> = db
        .iter_from_slot(SlotNo(0))
        .expect("iter")
        .map(|r| r.expect("ok").slot.0)
        .collect();
    assert_eq!(slots, vec![1, 2, 3, 4, 5, 6, 9]);
}

fn iter_from_slot_skips_lower<D: ChainDb>(db: &D) {
    for s in [10u64, 20, 30, 40] {
        db.put_block(&block(s, s as u8)).expect("put");
    }
    let slots: Vec<u64> = db
        .iter_from_slot(SlotNo(25))
        .expect("iter")
        .map(|r| r.expect("ok").slot.0)
        .collect();
    assert_eq!(slots, vec![30, 40]);
}

fn rollback_removes_higher_blocks<D: ChainDb>(db: &D) {
    for s in [10u64, 20, 30, 40] {
        db.put_block(&block(s, s as u8)).expect("put");
    }
    db.rollback_to_slot(SlotNo(25)).expect("rollback");
    assert!(db.get_block_by_slot(SlotNo(30)).expect("get").is_none());
    assert!(db.get_block_by_slot(SlotNo(40)).expect("get").is_none());
}

fn rollback_preserves_lower_blocks<D: ChainDb>(db: &D) {
    for s in [10u64, 20, 30, 40] {
        db.put_block(&block(s, s as u8)).expect("put");
    }
    db.rollback_to_slot(SlotNo(25)).expect("rollback");
    assert!(db.get_block_by_slot(SlotNo(10)).expect("get").is_some());
    assert!(db.get_block_by_slot(SlotNo(20)).expect("get").is_some());
}

fn rollback_clears_hash_index<D: ChainDb>(db: &D) {
    db.put_block(&block(10, 0xaa)).expect("put");
    db.put_block(&block(20, 0xbb)).expect("put");
    db.rollback_to_slot(SlotNo(15)).expect("rollback");
    let hash_bb = Hash32([0xbb; 32]);
    assert!(db.get_block_by_hash(&hash_bb).expect("get").is_none());
}

fn rollback_beyond_tip_is_noop<D: ChainDb>(db: &D) {
    db.put_block(&block(10, 0xaa)).expect("put");
    db.rollback_to_slot(SlotNo(1000)).expect("rollback noop");
    assert!(db.get_block_by_slot(SlotNo(10)).expect("get").is_some());
}

fn same_block_reput_is_idempotent<D: ChainDb>(db: &D) {
    let b = block(10, 0xaa);
    db.put_block(&b).expect("put 1");
    db.put_block(&b).expect("put 2 (idempotent)");
    assert_eq!(
        db.get_block_by_slot(b.slot).expect("get").expect("present"),
        b,
    );
}

fn conflicting_slot_put_errors<D: ChainDb>(db: &D) {
    db.put_block(&block(10, 0xaa)).expect("put");
    let conflict = db.put_block(&block(10, 0xbb));
    assert!(matches!(
        conflict,
        Err(super::ChainDbError::InvalidOperation(_))
    ));
}

fn not_found_is_ok_none<D: ChainDb>(db: &D) {
    assert!(db.get_block_by_slot(SlotNo(999)).expect("get").is_none());
    assert!(
        db.get_block_by_hash(&Hash32([0xff; 32]))
            .expect("get")
            .is_none()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chaindb::InMemoryChainDb;

    #[test]
    fn in_memory_passes_contract() {
        run_contract_tests(InMemoryChainDb::new);
    }
}
