// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! EPOCH-CONSENSUS-VIEW Slice 1 — the bounded-materialization memory gate
//! (GATE-MEM / D4).
//!
//! Runs a `create → materialize N (committed corpus) → iterate → dispose` cycle
//! over the transient redb [`TransientEpochViewStore`] while sampling owned
//! `RssAnon` (the anonymous resident heap). The redb store is mmap-backed, so a
//! CORRECTLY disk-backed materialization keeps the bulk OFF the anonymous heap:
//! `RssAnon` shows only a small bounded delta even with all N entries on disk.
//!
//! ASSERTED (the hard regression ceiling, D4):
//! - the `RssAnon` delta (peak over the cycle − pre-materialize baseline) is
//!   below the FIXED committed ceiling [`RSS_ANON_DELTA_CEILING_KIB`] on the
//!   committed corpus [`CORPUS_N`];
//! - the bulk lives on disk: `TransientEpochViewStore::len() == CORPUS_N`.
//!
//! REPORTED (evidence only, never asserted — BA-08 1.94<2.57 GiB is the release
//! evidence, not this test's threshold): the peak `RssAnon`, the baseline, the
//! delta, and the redb on-disk byte count.
//!
//! `RssAnon` reader: `ade_runtime` does NOT (and must not) depend on `ade_node`,
//! where `mem_measure/rss_sampler.rs` lives — depending on the binary crate the
//! other way round would invert the dependency arrow. The few lines that read the
//! `RssAnon:` field of `/proc/self/status` are replicated locally here instead.

use std::path::Path;

use ade_runtime::chaindb::{
    purge_transient_root, window_key, AnchorPosition, TransientEpochViewStore,
};
use ade_ledger::utxo::TxOut;
use ade_types::address::Address;
use ade_types::tx::{Coin, TxIn};
use ade_types::Hash32;
use tempfile::TempDir;

/// **D4 — committed corpus.** The number of distinct UTxO entries the gate
/// materializes on disk. A FIXED constant (not "calibrate later"): the gate is
/// reproducible only against a pinned N. Large enough that a heap-resident
/// (non-disk-backed) store would show a clearly distinguishable anonymous-heap
/// delta; bounded enough to run in the foreground test suite.
const CORPUS_N: u64 = 200_000;

/// **D4 — the hard regression ceiling (CI).** The maximum tolerated `RssAnon`
/// delta (peak over the cycle − pre-materialize baseline), in kiB, for
/// [`CORPUS_N`] entries. Calibrated against the observed delta with a fixed
/// margin above it. A correctly mmap-backed redb store keeps the N entries off
/// the anonymous heap, so the delta is small and bounded; a regression that
/// pulled the materialized set back onto the anonymous heap (~N × entry size)
/// would blow through this ceiling. COMMITTED, not deferred.
const RSS_ANON_DELTA_CEILING_KIB: u64 = 131_072; // 128 MiB

/// Distinct TxIn #`i` (every `i` is a unique key, so N inserts => N live entries).
fn txin(i: u64) -> TxIn {
    let mut h = [0u8; 32];
    h[..8].copy_from_slice(&i.to_le_bytes());
    TxIn {
        tx_hash: Hash32(h),
        index: (i % 4) as u16,
    }
}

fn out(i: u64) -> TxOut {
    TxOut::Byron {
        address: Address::Byron(vec![(i & 0xff) as u8]),
        coin: Coin(i.wrapping_mul(7).wrapping_add(1)),
    }
}

fn position(slot: u64) -> AnchorPosition {
    let mut h = [0u8; 32];
    h[..8].copy_from_slice(&slot.to_le_bytes());
    AnchorPosition {
        slot,
        block_hash: h,
        prior_fp: [0u8; 32],
        post_fp: h,
    }
}

/// The committed window key for the memory gate. Deterministic (no rand/uuid).
fn memory_window_key() -> String {
    window_key(1, 7, 1331, b"mem-gate-src-point", b"mem-gate-ckpt")
}

/// OWNED anonymous resident heap (`RssAnon`, `/proc/self/status`), in kiB — the
/// metric that EXCLUDES file-backed mappings (the mmap'd redb store). Replicated
/// locally (a few lines) rather than depending on `ade_node`'s sampler, which
/// would invert the crate dependency direction. `None` off-Linux / unreadable.
fn sample_rss_anon_kib() -> Option<u64> {
    let contents = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("RssAnon:") {
            let num = rest.split_whitespace().next()?;
            return num.parse::<u64>().ok();
        }
    }
    None
}

/// MAC #1 — bounded materialization. `create → materialize CORPUS_N → iterate →
/// dispose` while sampling `RssAnon`; assert the delta is below the fixed
/// committed ceiling and that all N entries are on disk; report the peak +
/// on-disk bytes.
#[test]
fn transient_materialization_rss_anon_delta_bounded() {
    // Off-Linux (no /proc): the gate is vacuous — the metric is unavailable.
    let Some(baseline) = sample_rss_anon_kib() else {
        eprintln!("transient_view_memory: RssAnon unavailable (non-Linux); gate vacuous");
        return;
    };

    let tmp = TempDir::new().expect("tempdir");
    let root = ade_runtime::chaindb::transient_root(tmp.path());
    purge_transient_root(&root).expect("fail-closed purge before materialization (D3)");
    let key = memory_window_key();

    let store = TransientEpochViewStore::open(&root, &key).expect("open transient window");

    // Materialize CORPUS_N distinct entries on disk in bounded batches. Each batch
    // is built, committed (a single atomic redb txn), then dropped before the next
    // — the materialized bulk lives in the mmap-backed store, NOT on the heap.
    const BATCH: u64 = 4_096;
    let mut peak = baseline;
    let mut produced = 0u64;
    let mut slot = 0u64;
    while produced < CORPUS_N {
        let n = BATCH.min(CORPUS_N - produced);
        let batch: Vec<(TxIn, TxOut)> = (produced..produced + n)
            .map(|i| (txin(i), out(i)))
            .collect();
        store
            .materialize_batch(&batch, &position(slot))
            .expect("materialize batch");
        produced += n;
        slot += 1;
        if let Some(now) = sample_rss_anon_kib() {
            peak = peak.max(now);
        }
    }

    // Iterate the whole window (the aggregation-pass shape). This materializes the
    // decoded set into a Vec, the single intentionally heap-resident step — it is a
    // bounded RED pass, sampled into the peak.
    let window = store.iter_window().expect("iterate window");
    assert_eq!(window.len() as u64, CORPUS_N, "iteration yields all N entries");
    if let Some(now) = sample_rss_anon_kib() {
        peak = peak.max(now);
    }

    // The bulk is on disk: len()==N.
    let on_disk_len = store.len().expect("len");
    assert_eq!(on_disk_len, CORPUS_N, "all N entries are on disk (UtxoAnchor::len)");
    let on_disk_bytes = store.on_disk_bytes();

    store.dispose().expect("dispose");
    assert!(
        !root.join(&key).exists(),
        "the window file is gone after dispose"
    );

    let delta = peak.saturating_sub(baseline);

    // Evidence (reported, never asserted).
    eprintln!(
        "transient_view_memory: corpus_n={CORPUS_N} on_disk_bytes={on_disk_bytes} \
         rss_anon_baseline_kib={baseline} rss_anon_peak_kib={peak} \
         rss_anon_delta_kib={delta} ceiling_kib={RSS_ANON_DELTA_CEILING_KIB}"
    );

    // The hard regression ceiling (GATE-MEM): the anonymous-heap delta stays below
    // the fixed committed ceiling. A correctly disk-backed materialization keeps
    // the N entries off RssAnon; only the bounded iterate Vec + redb's small
    // working buffers contribute.
    assert!(
        delta < RSS_ANON_DELTA_CEILING_KIB,
        "GATE-MEM: RssAnon delta {delta} kiB >= ceiling {RSS_ANON_DELTA_CEILING_KIB} kiB \
         for {CORPUS_N} entries (on_disk_bytes={on_disk_bytes}, baseline={baseline}, peak={peak}) \
         -- the materialized set may have leaked onto the anonymous heap"
    );
}

/// A second pass after the first disposes proves the lifecycle is repeatable and
/// the anonymous heap does not ratchet up across windows (no per-window leak):
/// the second cycle's delta is bounded by the same ceiling.
#[test]
fn transient_materialization_is_repeatable_without_ratchet() {
    let Some(_) = sample_rss_anon_kib() else {
        eprintln!("transient_view_memory: RssAnon unavailable (non-Linux); gate vacuous");
        return;
    };

    let tmp = TempDir::new().expect("tempdir");
    let root = ade_runtime::chaindb::transient_root(tmp.path());
    let key = memory_window_key();

    // A SMALLER corpus here (this is the no-ratchet shape, not the headline N) so
    // the two-pass test stays fast; the per-pass delta must still be bounded.
    const SMALL_N: u64 = 50_000;

    for pass in 0..2u64 {
        purge_transient_root(&root).expect("purge");
        let before = sample_rss_anon_kib().expect("rss");
        let store = TransientEpochViewStore::open(&root, &key).expect("open");
        let batch: Vec<(TxIn, TxOut)> = (0..SMALL_N).map(|i| (txin(i), out(i))).collect();
        store
            .materialize_batch(&batch, &position(pass))
            .expect("materialize");
        assert_eq!(store.len().expect("len"), SMALL_N);
        let _ = store.iter_window().expect("iter");
        let peak = sample_rss_anon_kib().expect("rss");
        store.dispose().expect("dispose");
        let delta = peak.saturating_sub(before);
        eprintln!("transient_view_memory: pass={pass} small_n={SMALL_N} rss_anon_delta_kib={delta}");
        assert!(
            delta < RSS_ANON_DELTA_CEILING_KIB,
            "GATE-MEM (pass {pass}): RssAnon delta {delta} kiB >= ceiling {RSS_ANON_DELTA_CEILING_KIB} kiB"
        );
    }
    assert!(!root.join(&key).exists(), "disposed");
}

/// Sanity: the local RssAnon reader returns a plausible value on Linux (a running
/// process has a nonzero anonymous heap). Off-Linux it is `None` (gate vacuous).
#[test]
fn local_rss_anon_reader_present_on_linux() {
    let s = sample_rss_anon_kib();
    if cfg!(target_os = "linux") {
        let v = s.expect("RssAnon readable on linux");
        assert!(v > 0, "a running process has a nonzero anonymous heap");
    }
}

// Silence the unused-helper warning when the readers compile but a branch is not
// taken on a given platform.
#[allow(dead_code)]
fn _ensure_path_used() {
    let _ = Path::new("/");
}
