// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Crash-safety contract test interfaces (S-34).
//!
//! S-34 ships the *interfaces* and a no-op fault-injection harness;
//! S-37 will replace the no-op with real `kill -9` against a child
//! process to close CE-N-D-1 (1,000-kill-9 corruption-free
//! invariant). The interfaces live here so S-37 has a stable target.
//!
//! See `docs/active/S-34_obligation_discharge.md` §O-34.5.

use ade_types::primitives::{Hash32, SlotNo};

use super::{ChainDb, StoredBlock};

/// A crash window. Implementations of [`KillStrategy`] simulate a
/// crash at the moment the strategy's method is called, returning
/// `true` if a kill actually occurred.
pub trait KillStrategy<D: ChainDb> {
    /// Called immediately after a `put_block` returns `Ok`. If this
    /// returns `true`, the test runner skips assertions that depend
    /// on the in-process db handle and reopens via `make_db`.
    fn after_put(&self, db: &D) -> bool;

    /// Called mid-`rollback_to_slot` (S-37 wires this to a child
    /// process; in-process kills aren't possible without unsafe).
    fn during_rollback(&self, db: &D) -> bool;
}

/// No-op fault injection. Real fault injection is S-37 scope.
pub struct NoKill;

impl<D: ChainDb> KillStrategy<D> for NoKill {
    fn after_put(&self, _: &D) -> bool {
        false
    }
    fn during_rollback(&self, _: &D) -> bool {
        false
    }
}

fn block(slot: u64, hash_byte: u8) -> StoredBlock {
    StoredBlock {
        slot: SlotNo(slot),
        hash: Hash32([hash_byte; 32]),
        bytes: vec![hash_byte; 64],
    }
}

/// Run the crash-safety obligations. With `NoKill`, this is
/// effectively a re-run of the contract suite with extra reopen
/// assertions interleaved. With a real `KillStrategy` (S-37), it
/// validates crash windows.
///
/// `make_db` must produce an impl that opens / reopens the same
/// underlying storage on each call. For in-memory impls this means
/// a shared backing store (which is why crash-safety isn't usually
/// run against `InMemoryChainDb`); for persistent impls it means
/// opening the same file path.
pub fn run_crash_safety_tests<D, F, K>(make_db: F, kill: K)
where
    D: ChainDb,
    F: Fn() -> D,
    K: KillStrategy<D>,
{
    put_then_kill_then_reopen_observes_block(&make_db, &kill);
    repeated_put_same_block_idempotent_across_reopens(&make_db);
    rollback_persists_across_reopen(&make_db);
}

fn put_then_kill_then_reopen_observes_block<D, F, K>(make_db: &F, kill: &K)
where
    D: ChainDb,
    F: Fn() -> D,
    K: KillStrategy<D>,
{
    let db = make_db();
    let b = block(100, 0xa1);
    db.put_block(&b).expect("put");
    let _killed = kill.after_put(&db);
    drop(db);

    let reopened = make_db();
    let got = reopened
        .get_block_by_slot(SlotNo(100))
        .expect("get")
        .expect("survives reopen");
    assert_eq!(got, b);
}

fn repeated_put_same_block_idempotent_across_reopens<D, F>(make_db: &F)
where
    D: ChainDb,
    F: Fn() -> D,
{
    let b = block(7, 0x07);
    {
        let db = make_db();
        db.put_block(&b).expect("first put");
    }
    {
        let db = make_db();
        db.put_block(&b).expect("idempotent re-put across reopen");
        let got = db
            .get_block_by_slot(SlotNo(7))
            .expect("get")
            .expect("present");
        assert_eq!(got, b);
    }
}

fn rollback_persists_across_reopen<D, F>(make_db: &F)
where
    D: ChainDb,
    F: Fn() -> D,
{
    {
        let db = make_db();
        db.put_block(&block(10, 0x10)).expect("put 10");
        db.put_block(&block(20, 0x20)).expect("put 20");
        db.put_block(&block(30, 0x30)).expect("put 30");
        db.rollback_to_slot(SlotNo(15)).expect("rollback");
    }
    let db = make_db();
    assert!(
        db.get_block_by_slot(SlotNo(10)).expect("get").is_some(),
        "kept block before rollback target",
    );
    assert!(
        db.get_block_by_slot(SlotNo(20)).expect("get").is_none(),
        "rollback persisted across reopen",
    );
    assert!(
        db.get_block_by_slot(SlotNo(30)).expect("get").is_none(),
        "rollback persisted across reopen",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chaindb::{PersistentChainDb, PersistentChainDbOptions};
    use std::sync::Arc;
    use tempfile::TempDir;

    #[test]
    fn persistent_passes_crash_safety_with_no_kill() {
        let tmp = Arc::new(TempDir::new().expect("tempdir"));
        let path = tmp.path().join("crash.chaindb");
        let make = || {
            PersistentChainDb::open(PersistentChainDbOptions::at(&path))
                .expect("open")
        };
        run_crash_safety_tests(make, NoKill);
    }
}
