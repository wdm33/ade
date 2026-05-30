// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration test — PHASE4-N-M-A S4 (DC-WAL-03).
//!
//! Headline replay-equivalence harness: builds a fresh
//! `FileWalStore` on disk, appends N entries derived from the
//! sample (anchor, block-hash) inputs, runs
//! `wal::replay_from_anchor` twice over the persisted entries
//! plus the matching block-bytes map, asserts both runs produce
//! the same final ledger fingerprint AND that fingerprint equals
//! the last entry's `post_fp`.
//!
//! Future sub-cluster B replaces the synthetic
//! `(prior_fp, post_fp)` pairs with real fingerprints computed
//! per-entry via `block_validity`; the harness shape carries
//! forward unchanged.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use ade_ledger::wal::{encode_wal_entry, replay_from_anchor, BlockVerdictTag, WalEntry, WalStore};
use ade_runtime::wal::FileWalStore;
use ade_types::{Hash32, SlotNo};
use tempfile::tempdir;

fn mk_entry(prior_fp: u8, post_fp: u8, block_hash_byte: u8, slot: u64) -> WalEntry {
    WalEntry::AdmitBlock {
        prior_fp: Hash32([prior_fp; 32]),
        block_hash: Hash32([block_hash_byte; 32]),
        slot: SlotNo(slot),
        verdict: BlockVerdictTag::Valid,
        post_fp: Hash32([post_fp; 32]),
    }
}

fn synthetic_block_bytes(block_hash_byte: u8) -> Vec<u8> {
    // Future sub-cluster B uses real Conway corpus block bytes
    // here; for this slice synthetic bytes are sufficient since
    // the replay reducer only checks presence, not validity.
    vec![block_hash_byte; 64]
}

fn anchor_initial_ledger_fp() -> Hash32 {
    Hash32([0x01; 32])
}

fn build_three_entry_chain() -> (Vec<WalEntry>, BTreeMap<Hash32, Vec<u8>>) {
    let entries = vec![
        mk_entry(0x01, 0x02, 0xA1, 100),
        mk_entry(0x02, 0x03, 0xA2, 101),
        mk_entry(0x03, 0x04, 0xA3, 102),
    ];
    let mut block_bytes = BTreeMap::new();
    for byte in [0xA1u8, 0xA2, 0xA3] {
        block_bytes.insert(Hash32([byte; 32]), synthetic_block_bytes(byte));
    }
    (entries, block_bytes)
}

#[test]
fn wal_replay_from_anchor_two_runs_byte_identical() {
    let dir = tempdir().expect("tmpdir");
    let mut store = FileWalStore::open(dir.path()).expect("open");
    let (entries, block_bytes) = build_three_entry_chain();
    for e in &entries {
        store.append(e.clone()).expect("append");
    }
    let anchor_fp = anchor_initial_ledger_fp();

    let read = store.read_all().expect("read_all");
    let a = replay_from_anchor(&anchor_fp, &read, &block_bytes).expect("run a");
    let b = replay_from_anchor(&anchor_fp, &read, &block_bytes).expect("run b");
    assert_eq!(a, b, "two replays must be byte-identical");
    assert_eq!(
        a.tail_fp,
        Hash32([0x04; 32]),
        "final fingerprint must equal WAL tail's post_fp"
    );
}

#[test]
fn wal_replay_from_anchor_post_fp_matches_wal_tail() {
    let dir = tempdir().expect("tmpdir");
    let mut store = FileWalStore::open(dir.path()).expect("open");
    let (entries, block_bytes) = build_three_entry_chain();
    for e in &entries {
        store.append(e.clone()).expect("append");
    }
    let anchor_fp = anchor_initial_ledger_fp();
    let read = store.read_all().expect("read_all");
    let final_fp = replay_from_anchor(&anchor_fp, &read, &block_bytes).expect("ok");
    let expected = match read.last().expect("non-empty") {
        WalEntry::AdmitBlock { post_fp, .. } => post_fp.clone(),
        WalEntry::SeedEpochConsensusInputsImported { .. } => {
            panic!("this chain has no provenance entry")
        }
    };
    assert_eq!(final_fp.tail_fp, expected);
}

#[test]
fn wal_replay_from_anchor_rejects_chain_break() {
    let dir = tempdir().expect("tmpdir");
    let mut store = FileWalStore::open(dir.path()).expect("open");
    store
        .append(mk_entry(0x01, 0x02, 0xA1, 100))
        .expect("append 1");
    // Bad: prior_fp 0x99 instead of 0x02.
    store
        .append(mk_entry(0x99, 0x03, 0xA2, 101))
        .expect("append 2");
    let mut block_bytes = BTreeMap::new();
    for byte in [0xA1u8, 0xA2] {
        block_bytes.insert(Hash32([byte; 32]), synthetic_block_bytes(byte));
    }
    let anchor_fp = anchor_initial_ledger_fp();
    let read = store.read_all().expect("read_all");
    let err = replay_from_anchor(&anchor_fp, &read, &block_bytes).expect_err("must break");
    match err {
        ade_ledger::wal::WalError::ChainBreak { entry_index: 1, .. } => {}
        other => panic!("expected ChainBreak@1, got {other:?}"),
    }
}

#[test]
fn wal_replay_from_anchor_rejects_missing_block_bytes() {
    let dir = tempdir().expect("tmpdir");
    let mut store = FileWalStore::open(dir.path()).expect("open");
    store
        .append(mk_entry(0x01, 0x02, 0xA1, 100))
        .expect("append 1");
    store
        .append(mk_entry(0x02, 0x03, 0xA2, 101))
        .expect("append 2");
    // Only the first block's bytes are present.
    let mut block_bytes = BTreeMap::new();
    block_bytes.insert(Hash32([0xA1; 32]), synthetic_block_bytes(0xA1));
    let anchor_fp = anchor_initial_ledger_fp();
    let read = store.read_all().expect("read_all");
    let err = replay_from_anchor(&anchor_fp, &read, &block_bytes).expect_err("must fail");
    match err {
        ade_ledger::wal::WalError::BlockBytesMissing { block_hash } => {
            assert_eq!(block_hash, Hash32([0xA2; 32]));
        }
        other => panic!("expected BlockBytesMissing, got {other:?}"),
    }
}

#[test]
fn wal_replay_from_anchor_persists_across_reopen() {
    let dir = tempdir().expect("tmpdir");
    let (entries, block_bytes) = build_three_entry_chain();
    {
        let mut store = FileWalStore::open(dir.path()).expect("open");
        for e in &entries {
            store.append(e.clone()).expect("append");
        }
    }
    // Reopen and replay.
    let store2 = FileWalStore::open(dir.path()).expect("reopen");
    let anchor_fp = anchor_initial_ledger_fp();
    let read = store2.read_all().expect("read_all");
    assert_eq!(read.len(), 3);
    let result = replay_from_anchor(&anchor_fp, &read, &block_bytes).expect("ok");
    assert_eq!(result.tail_fp, Hash32([0x04; 32]));
}

#[test]
fn wal_entry_encoded_bytes_byte_identical_across_runs() {
    let e = mk_entry(0x10, 0x20, 0xB1, 555);
    let a = encode_wal_entry(&e);
    let b = encode_wal_entry(&e);
    assert_eq!(a, b);
}
