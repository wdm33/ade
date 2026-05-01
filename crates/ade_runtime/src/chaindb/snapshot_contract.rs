// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Reusable contract test suite for any [`SnapshotStore`] implementation
//! (S-35).

use ade_types::primitives::SlotNo;

use super::SnapshotStore;

/// Run every assertion in the snapshot contract suite against a fresh
/// store produced by `make_store`.
pub fn run_snapshot_contract_tests<S, F>(mut make_store: F)
where
    S: SnapshotStore,
    F: FnMut() -> S,
{
    empty_store_has_no_latest(&make_store());
    put_then_get(&make_store());
    latest_returns_highest_slot(&make_store());
    list_returns_ascending(&make_store());
    delete_removes_only_target(&make_store());
    delete_absent_is_ok(&make_store());
    same_bytes_reput_is_idempotent(&make_store());
    conflicting_bytes_at_same_slot_errors(&make_store());
    not_found_is_ok_none(&make_store());
}

fn empty_store_has_no_latest<S: SnapshotStore>(s: &S) {
    assert!(s.latest_snapshot().expect("latest").is_none());
    assert!(s.list_snapshot_slots().expect("list").is_empty());
}

fn put_then_get<S: SnapshotStore>(s: &S) {
    let bytes = vec![1u8, 2, 3, 4];
    s.put_snapshot(SlotNo(100), &bytes).expect("put");
    let got = s
        .get_snapshot(SlotNo(100))
        .expect("get")
        .expect("present");
    assert_eq!(got, bytes);
}

fn latest_returns_highest_slot<S: SnapshotStore>(s: &S) {
    s.put_snapshot(SlotNo(50), &[0x05]).expect("put 50");
    s.put_snapshot(SlotNo(200), &[0x20]).expect("put 200");
    s.put_snapshot(SlotNo(100), &[0x10]).expect("put 100");
    let (slot, bytes) = s.latest_snapshot().expect("latest").expect("non-empty");
    assert_eq!(slot, SlotNo(200));
    assert_eq!(bytes, vec![0x20]);
}

fn list_returns_ascending<S: SnapshotStore>(s: &S) {
    for slot in [40u64, 10, 30, 20] {
        s.put_snapshot(SlotNo(slot), &[slot as u8]).expect("put");
    }
    let slots = s.list_snapshot_slots().expect("list");
    let raw: Vec<u64> = slots.iter().map(|s| s.0).collect();
    assert_eq!(raw, vec![10, 20, 30, 40]);
}

fn delete_removes_only_target<S: SnapshotStore>(s: &S) {
    s.put_snapshot(SlotNo(10), &[0x10]).expect("put 10");
    s.put_snapshot(SlotNo(20), &[0x20]).expect("put 20");
    s.put_snapshot(SlotNo(30), &[0x30]).expect("put 30");
    s.delete_snapshot(SlotNo(20)).expect("delete 20");
    assert!(s.get_snapshot(SlotNo(10)).expect("get").is_some());
    assert!(s.get_snapshot(SlotNo(20)).expect("get").is_none());
    assert!(s.get_snapshot(SlotNo(30)).expect("get").is_some());
}

fn delete_absent_is_ok<S: SnapshotStore>(s: &S) {
    s.delete_snapshot(SlotNo(999)).expect("delete absent ok");
}

fn same_bytes_reput_is_idempotent<S: SnapshotStore>(s: &S) {
    let bytes = vec![0xff, 0xee, 0xdd];
    s.put_snapshot(SlotNo(7), &bytes).expect("put 1");
    s.put_snapshot(SlotNo(7), &bytes).expect("put 2 (idempotent)");
    let got = s.get_snapshot(SlotNo(7)).expect("get").expect("present");
    assert_eq!(got, bytes);
}

fn conflicting_bytes_at_same_slot_errors<S: SnapshotStore>(s: &S) {
    s.put_snapshot(SlotNo(5), &[0xaa]).expect("put");
    let conflict = s.put_snapshot(SlotNo(5), &[0xbb]);
    assert!(matches!(
        conflict,
        Err(super::ChainDbError::InvalidOperation(_))
    ));
}

fn not_found_is_ok_none<S: SnapshotStore>(s: &S) {
    assert!(s.get_snapshot(SlotNo(999)).expect("get").is_none());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chaindb::InMemoryChainDb;

    #[test]
    fn in_memory_passes_snapshot_contract() {
        run_snapshot_contract_tests(InMemoryChainDb::new);
    }
}
