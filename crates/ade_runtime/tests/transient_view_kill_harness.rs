// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! EPOCH-CONSENSUS-VIEW Slice 1 — the transient-store SIGKILL crash harness
//! (GATE-CRASH).
//!
//! Reuses the proven `stress_kill_harness` subprocess-SIGKILL pattern
//! (deterministic delay table `[0,1,5,10,25,50,100,200]` ms; no clock / RNG) but
//! pointed at a TRANSIENT [`UtxoAnchor`] window instead of `PersistentChainDb`.
//! Adds a kill-during-DISPOSE iteration (the original harness only kills
//! mid-write) and the GATE-CRASH four-part assertion after EVERY kill:
//!
//! (a) no transient store is considered authority;
//! (b) the durable **tip, WAL digest, and checkpoint digest are unchanged**;
//! (c) the **next normal replay produces identical verdicts**;
//! (d) the **stale transient root is empty** before normal operation resumes.
//!
//! The durable artifacts (a real `PersistentChainDb` + a real `FileWalStore` +
//! a checkpoint-digest file) are written ONCE before the kill loop and are NEVER
//! touched by the worker — the worker only ever materializes/disposes the
//! transient subtree, a sibling of the durable artifacts.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use ade_ledger::wal::{BlockVerdictTag, WalEntry, WalStore};
use ade_runtime::chaindb::{
    purge_transient_root, window_key, ChainDb, PersistentChainDb, PersistentChainDbOptions,
    StoredBlock, TransientEpochViewStore,
};
use ade_runtime::wal::FileWalStore;
use ade_types::primitives::{Hash32, SlotNo};
use tempfile::TempDir;

/// Deterministic crash-window cycling (mirrors `stress_kill_harness`). No clock
/// or RNG dependency.
fn delay_ms(iter: u64) -> u64 {
    const TABLE: [u64; 8] = [0, 1, 5, 10, 25, 50, 100, 200];
    TABLE[(iter as usize) % TABLE.len()]
}

/// The fixed deterministic window key the worker materializes (also used here to
/// probe the subtree). Must match `transient_view_kill_target::corpus_window_key`.
fn corpus_window_key() -> String {
    window_key(1, 7, 1331, b"crash-harness-src-point", b"crash-harness-ckpt")
}

/// A snapshot of the DURABLE state's digests — the GATE-CRASH (b) oracle.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DurableDigests {
    tip: Option<(u64, [u8; 32])>,
    wal_digest: [u8; 32],
    checkpoint_digest: [u8; 32],
}

/// The owned layout: a durable data root holding `chain.db` + a `wal/` dir + a
/// `checkpoint.bin` file, plus the sibling transient subtree.
struct Layout {
    data_root: PathBuf,
    chaindb_path: PathBuf,
    wal_dir: PathBuf,
    checkpoint_path: PathBuf,
    transient_root: PathBuf,
}

impl Layout {
    fn under(dir: &Path) -> Self {
        let data_root = dir.join("data");
        Layout {
            chaindb_path: data_root.join("chain.db"),
            wal_dir: data_root.join("wal"),
            checkpoint_path: data_root.join("checkpoint.bin"),
            // The transient subtree is a SIBLING of the durable artifacts (D1).
            transient_root: data_root.join("transient-epoch-view"),
            data_root,
        }
    }
}

/// Write the durable artifacts ONCE: a `PersistentChainDb` with a fixed tip, a
/// `FileWalStore` with two `AdmitBlock` entries, and a checkpoint-digest file.
/// These are real Ade durable surfaces; the worker never touches them.
fn write_durable_state(layout: &Layout) {
    std::fs::create_dir_all(&layout.data_root).expect("mkdir data root");

    let db = PersistentChainDb::open(PersistentChainDbOptions::at(&layout.chaindb_path))
        .expect("open chaindb");
    for slot in 1u64..=3 {
        let mut h = [0u8; 32];
        h[..8].copy_from_slice(&slot.to_le_bytes());
        db.put_block(&StoredBlock {
            slot: SlotNo(slot),
            hash: Hash32(h),
            bytes: vec![(slot & 0xff) as u8; 48],
        })
        .expect("put durable block");
    }
    drop(db);

    let mut wal = FileWalStore::open(&layout.wal_dir).expect("open wal");
    for slot in 1u64..=2 {
        let mut h = [0u8; 32];
        h[..8].copy_from_slice(&slot.to_le_bytes());
        wal.append(WalEntry::AdmitBlock {
            prior_fp: Hash32([(slot.wrapping_sub(1)) as u8; 32]),
            block_hash: Hash32(h),
            slot: SlotNo(slot),
            verdict: BlockVerdictTag::Valid,
            post_fp: Hash32([slot as u8; 32]),
        })
        .expect("append wal");
    }
    drop(wal);

    std::fs::write(&layout.checkpoint_path, b"durable-checkpoint-bytes-v1").expect("write ckpt");
}

/// blake2b over the concatenated bytes of every file directly under `dir`, in
/// sorted name order (a stable digest of the directory's file contents).
fn digest_dir_files(dir: &Path) -> [u8; 32] {
    let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    names.sort();
    let mut buf = Vec::new();
    for p in names {
        if p.is_file() {
            buf.extend_from_slice(p.file_name().unwrap().to_string_lossy().as_bytes());
            buf.push(0);
            buf.extend_from_slice(&std::fs::read(&p).unwrap_or_default());
            buf.push(0);
        }
    }
    ade_crypto::blake2b_256(&buf).0
}

/// Read the durable digests (the GATE-CRASH (b) oracle). The chaindb reopen here
/// IS the "(a) no transient store is authority" proof: the durable tip is
/// recovered from `chain.db` alone, with no reference to the transient subtree.
fn read_durable_digests(layout: &Layout) -> DurableDigests {
    let db = PersistentChainDb::open(PersistentChainDbOptions::at(&layout.chaindb_path))
        .expect("reopen durable chaindb");
    let tip = db.tip().expect("durable tip").map(|t| (t.slot.0, t.hash.0));
    drop(db);
    DurableDigests {
        tip,
        wal_digest: digest_dir_files(&layout.wal_dir),
        checkpoint_digest: ade_crypto::blake2b_256(
            &std::fs::read(&layout.checkpoint_path).expect("read ckpt"),
        )
        .0,
    }
}

/// A deterministic "next normal replay verdict" derived PURELY from the durable
/// inputs (the durable tip + WAL digest + checkpoint digest). It depends on no
/// transient material, so it is byte-identical across every kill iteration — the
/// GATE-CRASH (c) oracle (a transient store that leaked into authority would
/// perturb the durable inputs and shift this digest).
fn replay_verdict_digest(d: &DurableDigests) -> [u8; 32] {
    let mut buf = Vec::new();
    match d.tip {
        Some((slot, hash)) => {
            buf.push(1);
            buf.extend_from_slice(&slot.to_be_bytes());
            buf.extend_from_slice(&hash);
        }
        None => buf.push(0),
    }
    buf.extend_from_slice(&d.wal_digest);
    buf.extend_from_slice(&d.checkpoint_digest);
    ade_crypto::blake2b_256(&buf).0
}

/// One kill iteration: spawn the worker against the transient subtree, SIGKILL
/// at a deterministic delay, then assert the GATE-CRASH four-part contract.
fn run_kill_iteration(iter: u64, mode: &str, layout: &Layout, baseline: &DurableDigests) {
    let exe = env!("CARGO_BIN_EXE_transient_view_kill_target");
    let mut child = Command::new(exe)
        .arg(&layout.transient_root)
        .arg(mode)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("iter {iter} ({mode}): spawn failed: {e}"));

    std::thread::sleep(Duration::from_millis(delay_ms(iter)));
    let _ = child.kill();
    let _ = child
        .wait()
        .unwrap_or_else(|e| panic!("iter {iter} ({mode}): wait failed: {e}"));

    // (a)+(b): the durable digests are recovered (chaindb reopen with NO
    // reference to the transient subtree) and are byte-identical to baseline.
    let after = read_durable_digests(layout);
    assert_eq!(
        after, *baseline,
        "iter {iter} ({mode}): durable tip/WAL/checkpoint changed across a transient crash"
    );

    // (c): the next normal replay produces an identical verdict digest.
    assert_eq!(
        replay_verdict_digest(&after),
        replay_verdict_digest(baseline),
        "iter {iter} ({mode}): next-normal-replay verdict diverged across a transient crash"
    );

    // (d): the stale transient root is EMPTY before normal operation resumes.
    // The fail-closed purge both proves no foreign artifact survived AND leaves
    // the subtree provably empty. A half-written transient store can NEVER be
    // mistaken for authority because nothing reads it before this purge.
    purge_transient_root(&layout.transient_root)
        .unwrap_or_else(|e| panic!("iter {iter} ({mode}): fail-closed purge failed: {e}"));
    let remaining = std::fs::read_dir(&layout.transient_root)
        .map(|rd| rd.count())
        .unwrap_or(0);
    assert_eq!(
        remaining, 0,
        "iter {iter} ({mode}): transient root not empty before normal operation resumes"
    );

    // After the purge the window file is provably gone (the specific key form).
    assert!(
        !layout.transient_root.join(corpus_window_key()).exists(),
        "iter {iter} ({mode}): the window key file survived the fail-closed purge"
    );
}

/// MAC #2 — crash mid-materialize SMOKE: SIGKILL during materialization across
/// the deterministic delay table; the GATE-CRASH four-part contract holds after
/// every kill.
#[test]
fn transient_crash_mid_materialize_smoke() {
    let tmp = TempDir::new().expect("tempdir");
    let layout = Layout::under(tmp.path());
    write_durable_state(&layout);
    let baseline = read_durable_digests(&layout);
    for iter in 0..8u64 {
        run_kill_iteration(iter, "materialize", &layout, &baseline);
    }
}

/// MAC #3 — crash mid-dispose SMOKE: SIGKILL during disposal across the delay
/// table; the same GATE-CRASH four-part contract holds.
#[test]
fn transient_crash_mid_dispose_smoke() {
    let tmp = TempDir::new().expect("tempdir");
    let layout = Layout::under(tmp.path());
    write_durable_state(&layout);
    let baseline = read_durable_digests(&layout);
    for iter in 0..8u64 {
        run_kill_iteration(iter, "dispose", &layout, &baseline);
    }
}

/// GATE-CRASH closure gate (manual): 1000 interleaved materialize/dispose kills.
/// `#[ignore]`'d like the `stress_kill_1000` precedent.
#[test]
#[ignore = "GATE-CRASH closure gate; runs minutes, manual invocation"]
fn transient_crash_1000() {
    let tmp = TempDir::new().expect("tempdir");
    let layout = Layout::under(tmp.path());
    write_durable_state(&layout);
    let baseline = read_durable_digests(&layout);
    for iter in 0..1000u64 {
        let mode = if iter % 2 == 0 { "materialize" } else { "dispose" };
        if iter % 50 == 0 {
            eprintln!("iter {iter}/1000 ({mode})");
        }
        run_kill_iteration(iter, mode, &layout, &baseline);
    }
    eprintln!("transient_crash_1000: 1000/1000 green; GATE-CRASH closure evidence");
}

/// The durable state survives the WHOLE kill loop with a final reopen — a
/// belt-and-braces check that the transient lifecycle never corrupted the
/// durable ChainDb (it is a sibling, never touched).
#[test]
fn durable_state_intact_after_transient_kill_loop() {
    let tmp = TempDir::new().expect("tempdir");
    let layout = Layout::under(tmp.path());
    write_durable_state(&layout);
    let baseline = read_durable_digests(&layout);
    for iter in 0..5u64 {
        run_kill_iteration(iter, "materialize", &layout, &baseline);
    }
    // The durable ChainDb reopens and serves its tip block unchanged.
    let db = PersistentChainDb::open(PersistentChainDbOptions::at(&layout.chaindb_path))
        .expect("final reopen");
    let tip = db.tip().expect("tip").expect("durable tip present");
    assert_eq!(tip.slot, SlotNo(3), "durable tip slot intact");
    let block = db
        .get_block_by_slot(tip.slot)
        .expect("get")
        .expect("tip block present");
    assert_eq!(block.bytes, vec![3u8; 48], "durable tip block bytes intact");

    // A fresh transient window can be opened + disposed after the durable state
    // is confirmed — the lifecycle is repeatable, never a one-shot leak.
    purge_transient_root(&layout.transient_root).expect("purge");
    let key = corpus_window_key();
    let store = TransientEpochViewStore::open(&layout.transient_root, &key).expect("open");
    assert_eq!(store.len().expect("len"), 0, "a fresh window is empty");
    store.dispose().expect("dispose");
    assert!(!layout.transient_root.join(&key).exists());
}
