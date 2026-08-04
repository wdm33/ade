//! PREPROD-ENTRY-AUTHORITY P6 (DC-STORE-10/11) — the store-semantics version gate, proven against
//! REAL redb artifacts rather than the pure checker alone.
//!
//! The pure checker is unit-tested in `ade_ledger::store_semantics`. What these tests prove is the
//! part that actually failed in P4: that a durable artifact which **parses perfectly** is still
//! rejected when its authority semantics do not match the binary. Every case here writes a genuine
//! store on disk, tampers only with the marker, and reopens it.
//!
//! The legacy case is the one that matters most: a store with NO marker is exactly a pre-P6 store,
//! and it must fail closed with no stamp path.

use std::path::{Path, PathBuf};

use ade_ledger::store_semantics::{
    AuthorityArtifact, FoundSemanticsVersion, RemediationAction, STORE_SEMANTICS_VERSION,
};
use ade_runtime::chaindb::{
    ChainDbError, EpochAccumulatorStore, EpochAccumulatorStoreError, PersistentChainDb,
    PersistentChainDbOptions, ReducedCheckpointError, ReducedUtxoCheckpoint,
};
use redb::{Database, TableDefinition};

const CHAINDB_META: TableDefinition<&str, &[u8]> = TableDefinition::new("meta");
const ACC_META: TableDefinition<&str, &[u8]> = TableDefinition::new("epoch_acc_meta");
const REDUCED_META: TableDefinition<&str, &[u8]> = TableDefinition::new("reduced_meta");
const SEMANTICS_KEY: &str = "store_semantics_version";

fn tmpdir(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("ade-p6-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).expect("mkdir");
    base
}

/// Rewrite (or remove) the marker on a closed redb artifact, leaving every other byte intact.
/// This is how a "legacy" or "version-skewed" store is simulated WITHOUT corrupting anything —
/// the point being that the store still parses.
fn set_marker(path: &Path, table: TableDefinition<&str, &[u8]>, value: Option<u32>, be: bool) {
    let db = Database::create(path).expect("open for tamper");
    let txn = db.begin_write().expect("txn");
    {
        let mut meta = txn.open_table(table).expect("meta");
        match value {
            None => {
                let _ = meta.remove(SEMANTICS_KEY).expect("remove");
            }
            Some(v) => {
                let bytes = if be { v.to_be_bytes() } else { v.to_le_bytes() };
                meta.insert(SEMANTICS_KEY, &bytes[..]).expect("insert");
            }
        }
    }
    txn.commit().expect("commit");
    drop(db);
}

// ---------------------------------------------------------------------------
// chain.db
// ---------------------------------------------------------------------------

fn open_chaindb(path: &Path) -> Result<PersistentChainDb, ChainDbError> {
    PersistentChainDb::open(PersistentChainDbOptions::at(path))
}

/// CE-P6-1 + CE-P6-6: a fresh store is stamped at creation and reopens cleanly. Without this the
/// rest of the suite could pass vacuously by rejecting everything.
#[test]
fn fresh_chaindb_is_stamped_and_reopens() {
    let dir = tmpdir("chaindb-fresh");
    let path = dir.join("chain.db");
    drop(open_chaindb(&path).expect("fresh store must open"));
    open_chaindb(&path).expect("a freshly stamped store must reopen");
}

/// CE-P6-3: a store with NO marker is a pre-P6 (legacy) store. It parses perfectly and is rejected
/// anyway — this is the P4 case, and the whole reason the slice exists.
#[test]
fn unmarked_chaindb_is_rejected_as_legacy() {
    let dir = tmpdir("chaindb-legacy");
    let path = dir.join("chain.db");
    drop(open_chaindb(&path).expect("fresh"));
    set_marker(&path, CHAINDB_META, None, false);

    match open_chaindb(&path) {
        Err(ChainDbError::StoreSemantics(e)) => {
            assert_eq!(e.artifact, AuthorityArtifact::ChainDb);
            assert_eq!(e.found, FoundSemanticsVersion::Absent);
            assert_eq!(e.action, RemediationAction::RebootstrapRequired);
        }
        other => panic!("an unmarked chaindb must fail closed, got {other:?}"),
    }
}

/// CE-P6-4: an OLDER marker -> re-bootstrap required, never an implicit migration.
#[test]
fn older_marked_chaindb_is_rejected() {
    let dir = tmpdir("chaindb-old");
    let path = dir.join("chain.db");
    drop(open_chaindb(&path).expect("fresh"));
    set_marker(
        &path,
        CHAINDB_META,
        Some(STORE_SEMANTICS_VERSION - 1),
        false,
    );

    match open_chaindb(&path) {
        Err(ChainDbError::StoreSemantics(e)) => {
            assert_eq!(
                e.found,
                FoundSemanticsVersion::Version(STORE_SEMANTICS_VERSION - 1)
            );
            assert_eq!(e.action, RemediationAction::RebootstrapRequired);
        }
        other => panic!("an older-marked chaindb must fail closed, got {other:?}"),
    }
}

/// CE-P6-5: a FUTURE marker fails closed too — an older binary must not guess at a newer binary's
/// semantics.
#[test]
fn future_marked_chaindb_is_rejected() {
    let dir = tmpdir("chaindb-future");
    let path = dir.join("chain.db");
    drop(open_chaindb(&path).expect("fresh"));
    set_marker(
        &path,
        CHAINDB_META,
        Some(STORE_SEMANTICS_VERSION + 7),
        false,
    );

    match open_chaindb(&path) {
        Err(ChainDbError::StoreSemantics(e)) => assert_eq!(
            e.found,
            FoundSemanticsVersion::Version(STORE_SEMANTICS_VERSION + 7)
        ),
        other => panic!("a future-marked chaindb must fail closed, got {other:?}"),
    }
}

/// The operator-facing message must say what to DO and must not imply an override exists.
#[test]
fn the_terminal_names_rebootstrap_and_offers_no_override() {
    let dir = tmpdir("chaindb-msg");
    let path = dir.join("chain.db");
    drop(open_chaindb(&path).expect("fresh"));
    set_marker(&path, CHAINDB_META, None, false);

    let msg = match open_chaindb(&path) {
        Err(e) => e.to_string(),
        Ok(_) => panic!("must fail"),
    };
    assert!(
        msg.contains("re-bootstrap"),
        "message must name the remediation: {msg}"
    );
    assert!(
        msg.contains("no stamp path"),
        "message must be explicit that there is no override: {msg}"
    );
}

// ---------------------------------------------------------------------------
// epoch-accumulator.redb — independent provenance (opened from snapshot_dir)
// ---------------------------------------------------------------------------

/// CE-P6-1: a fresh accumulator stamps and reopens.
#[test]
fn fresh_accumulator_is_stamped_and_reopens() {
    let dir = tmpdir("acc-fresh");
    let path = dir.join("epoch-accumulator.redb");
    assert!(
        EpochAccumulatorStore::open(&path).is_ok(),
        "fresh accumulator must open"
    );
    assert!(
        EpochAccumulatorStore::open(&path).is_ok(),
        "a freshly stamped accumulator must reopen"
    );
}

/// CE-P6-8 (cross-artifact): the accumulator carries its OWN marker precisely because it is opened
/// from `snapshot_dir` while chain.db comes from the data dir — so a stale accumulator must not be
/// able to ride along with a current chain.db.
#[test]
fn stale_accumulator_is_rejected_independently_of_the_chaindb() {
    let dir = tmpdir("acc-stale");
    let chain = dir.join("chain.db");
    let acc = dir.join("epoch-accumulator.redb");
    drop(open_chaindb(&chain).expect("fresh chaindb"));
    assert!(
        EpochAccumulatorStore::open(&acc).is_ok(),
        "fresh accumulator"
    );

    // The ChainDb stays CURRENT; only the sibling is rolled back.
    set_marker(&acc, ACC_META, Some(STORE_SEMANTICS_VERSION - 1), true);

    open_chaindb(&chain).expect("the current chaindb still opens");
    match EpochAccumulatorStore::open(&acc) {
        Err(EpochAccumulatorStoreError::StoreSemantics(e)) => {
            assert_eq!(e.artifact, AuthorityArtifact::EpochAccumulator);
            assert_eq!(
                e.found,
                FoundSemanticsVersion::Version(STORE_SEMANTICS_VERSION - 1)
            );
        }
        Err(other) => {
            panic!("a stale accumulator must fail closed on its own marker, got {other:?}")
        }
        Ok(_) => panic!("a stale accumulator must fail closed on its own marker, got Ok"),
    }
}

// ---------------------------------------------------------------------------
// reduced-checkpoint.redb — independent provenance (opened from snapshot_dir)
// ---------------------------------------------------------------------------

#[test]
fn fresh_reduced_checkpoint_is_stamped_and_reopens() {
    let dir = tmpdir("red-fresh");
    let path = dir.join("reduced-checkpoint.redb");
    assert!(
        ReducedUtxoCheckpoint::open(&path).is_ok(),
        "fresh checkpoint must open"
    );
    assert!(
        ReducedUtxoCheckpoint::open(&path).is_ok(),
        "a freshly stamped checkpoint must reopen"
    );
}

/// CE-P6-8 (cross-artifact), other sibling.
#[test]
fn stale_reduced_checkpoint_is_rejected_independently() {
    let dir = tmpdir("red-stale");
    let path = dir.join("reduced-checkpoint.redb");
    assert!(ReducedUtxoCheckpoint::open(&path).is_ok(), "fresh");
    set_marker(&path, REDUCED_META, Some(STORE_SEMANTICS_VERSION + 1), true);

    match ReducedUtxoCheckpoint::open(&path) {
        Err(ReducedCheckpointError::StoreSemantics(e)) => {
            assert_eq!(e.artifact, AuthorityArtifact::ReducedCheckpoint);
        }
        Err(other) => panic!("a version-skewed checkpoint must fail closed, got {other:?}"),
        Ok(_) => panic!("a version-skewed checkpoint must fail closed, got Ok"),
    }
}
