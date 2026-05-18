// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! S-37 stress harness — subprocess kill-9 loop against
//! `PersistentChainDb`.
//!
//! Smoke variant runs every `cargo test`. Gate variant
//! (`stress_kill_1000`) is `#[ignore]`'d for manual CE-N-D-1 closure.
//!
//! Per `docs/active/S-37_obligation_discharge.md`.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use ade_runtime::chaindb::{
    ChainDb, PersistentChainDb, PersistentChainDbOptions, SnapshotStore,
};
use ade_types::primitives::{Hash32, SlotNo};
use tempfile::TempDir;

fn delay_ms(iter: u64) -> u64 {
    // Per O-37.2: deterministic LCG-style cycling through interesting
    // crash windows. No clock or RNG dependency.
    const TABLE: [u64; 8] = [0, 1, 5, 10, 25, 50, 100, 200];
    TABLE[(iter as usize) % TABLE.len()]
}

fn run_iteration(iter: u64, path: &PathBuf) {
    let exe = env!("CARGO_BIN_EXE_chaindb_kill_target");
    let mut child = Command::new(exe)
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("iter {iter}: spawn failed: {e}"));

    std::thread::sleep(Duration::from_millis(delay_ms(iter)));
    let _ = child.kill();
    let _status = child
        .wait()
        .unwrap_or_else(|e| panic!("iter {iter}: wait failed: {e}"));
    // We don't assert the status — SIGKILL leaves the child in
    // signal-terminated state; that's expected.

    verify_invariants(iter, path);
}

fn verify_invariants(iter: u64, path: &PathBuf) {
    // 1. Reopen succeeds.
    let db = PersistentChainDb::open(PersistentChainDbOptions::at(path))
        .unwrap_or_else(|e| {
            panic!("iter {iter}: reopen failed: {e}");
        });

    // 2. Tip is consistent (or absent for early iterations).
    let tip = db
        .tip()
        .unwrap_or_else(|e| panic!("iter {iter}: tip() failed: {e}"));

    if let Some(tip) = tip {
        // 3. Tip block is readable by both indices.
        let by_slot = db
            .get_block_by_slot(tip.slot)
            .unwrap_or_else(|e| panic!("iter {iter}: get_by_slot failed: {e}"))
            .unwrap_or_else(|| {
                panic!("iter {iter}: tip slot {} not in slot index", tip.slot.0)
            });
        let by_hash = db
            .get_block_by_hash(&tip.hash)
            .unwrap_or_else(|e| panic!("iter {iter}: get_by_hash failed: {e}"))
            .unwrap_or_else(|| {
                panic!("iter {iter}: tip hash not in hash index")
            });
        assert_eq!(
            by_slot, by_hash,
            "iter {iter}: tip block from slot index != hash index",
        );

        // 4. Tip's hash matches what's expected for that slot
        // (worker derives hash from slot deterministically).
        let expected_hash = {
            let mut h = [0u8; 32];
            h[..8].copy_from_slice(&tip.slot.0.to_le_bytes());
            Hash32(h)
        };
        assert_eq!(
            tip.hash, expected_hash,
            "iter {iter}: tip hash mismatch — corruption or bug",
        );
    }

    // 5. Full iteration is unconditional: every block in the slot
    // index can be read back without error. This walks the whole
    // table, which is the strongest in-process integrity check we
    // have without the schema-level checksum redb already provides.
    db.iter_from_slot(SlotNo(0))
        .unwrap_or_else(|e| panic!("iter {iter}: iter_from_slot failed: {e}"))
        .enumerate()
        .for_each(|(idx, r)| {
            r.unwrap_or_else(|e| {
                panic!("iter {iter}: iter entry {idx} failed: {e}")
            });
        });
}

#[test]
fn stress_kill_smoke() {
    let tmp = TempDir::new().expect("tempdir");
    let path = tmp.path().join("smoke.chaindb");
    for iter in 0..10u64 {
        run_iteration(iter, &path);
    }
}

#[test]
#[ignore = "CE-N-D-1 closure gate; runs ~2-5 min, manual invocation"]
fn stress_kill_1000() {
    let tmp = TempDir::new().expect("tempdir");
    let path = tmp.path().join("gate.chaindb");
    for iter in 0..1000u64 {
        if iter % 50 == 0 {
            eprintln!("iter {iter}/1000");
        }
        run_iteration(iter, &path);
    }
    eprintln!("stress_kill_1000: 1000/1000 iterations green; CE-N-D-1 closure evidence");
}

/// Sanity check that the SnapshotStore surface is exercised by the
/// reopen path even though the worker doesn't write snapshots.
/// Confirms the schema upgrade machinery survives crashes during
/// pure block-write workloads.
#[test]
fn snapshot_table_intact_after_kill_loop() {
    let tmp = TempDir::new().expect("tempdir");
    let path = tmp.path().join("snap_intact.chaindb");
    for iter in 0..5u64 {
        run_iteration(iter, &path);
    }
    let db = PersistentChainDb::open(PersistentChainDbOptions::at(&path))
        .expect("reopen");
    db.put_snapshot(SlotNo(1), b"post-stress snapshot")
        .expect("put_snapshot");
    let got = db
        .get_snapshot(SlotNo(1))
        .expect("get_snapshot")
        .expect("present");
    assert_eq!(got, b"post-stress snapshot");
}
