// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! EPOCH-CONSENSUS-VIEW Slice 1 — worker binary for the transient-store SIGKILL
//! crash harness.
//!
//! Pointed at a TRANSIENT [`UtxoAnchor`] window (NOT `PersistentChainDb`): it
//! materializes — or disposes — the bounded transient replay window in a tight
//! loop until killed by SIGKILL from outside. It NEVER touches any durable
//! artifact (ChainDb / WAL / checkpoint); the harness owns those and asserts
//! they are byte-unchanged after every kill (the GATE-CRASH four-part
//! assertion).
//!
//! argv[1] = the transient subtree root (an owned `transient-epoch-view/` dir).
//! argv[2] = mode: `materialize` (killed mid-write) or `dispose` (window built,
//!           then killed mid-disposal).
//!
//! Not user-facing; only invoked by `tests/transient_view_kill_harness.rs`.

use std::path::PathBuf;

use ade_runtime::chaindb::{
    purge_transient_root, window_key, AnchorPosition, TransientEpochViewStore,
};
use ade_ledger::utxo::TxOut;
use ade_types::address::Address;
use ade_types::tx::{Coin, TxIn};
use ade_types::Hash32;

/// The committed corpus size for the crash worker (small + deterministic; the
/// memory-bound corpus is larger and lives in the ade_node measurement).
const CRASH_CORPUS_N: u64 = 2_000;

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

/// The fixed deterministic window key the harness also reconstructs to probe the
/// subtree. No rand/uuid.
fn corpus_window_key() -> String {
    window_key(1, 7, 1331, b"crash-harness-src-point", b"crash-harness-ckpt")
}

fn main() {
    let root: PathBuf = std::env::args()
        .nth(1)
        .expect("argv[1] must be the transient subtree root")
        .into();
    let mode = std::env::args().nth(2).unwrap_or_else(|| "materialize".to_string());

    // Fail-closed purge before any materialization (D3) — the worker resumes
    // nothing; a transient store is never resumable. A foreign artifact halts
    // here (structured terminal failure), which the harness asserts never
    // happens on a clean own subtree.
    if let Err(e) = purge_transient_root(&root) {
        eprintln!("worker: purge failed: {e}");
        std::process::exit(2);
    }

    let key = corpus_window_key();
    let store = match TransientEpochViewStore::open(&root, &key) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("worker: open failed: {e}");
            std::process::exit(3);
        }
    };

    match mode.as_str() {
        "materialize" => materialize_loop(&store),
        "dispose" => dispose_loop(store, &root, &key),
        other => {
            eprintln!("worker: unknown mode {other:?}");
            std::process::exit(4);
        }
    }
}

/// Materialize the window in small batches forever (re-purging + rebuilding each
/// full pass), so a SIGKILL at any delay lands mid-materialize. The transient
/// store is the ONLY thing touched.
fn materialize_loop(store: &TransientEpochViewStore) -> ! {
    const BATCH: u64 = 64;
    let mut slot = 0u64;
    loop {
        let start = (slot * BATCH) % CRASH_CORPUS_N;
        let produced: Vec<(TxIn, TxOut)> = (start..start + BATCH).map(|i| (txin(i), out(i))).collect();
        if let Err(e) = store.materialize_batch(&produced, &position(slot)) {
            eprintln!("worker: materialize_batch failed: {e}");
            std::process::exit(5);
        }
        slot = slot.wrapping_add(1);
        if slot == 0 {
            slot = 1;
        }
    }
}

/// Build the full window once, then repeatedly dispose + reopen + rebuild so a
/// SIGKILL lands mid-dispose. Each dispose removes the window file + fsyncs the
/// owned subtree; the durable artifacts are never touched.
fn dispose_loop(mut store: TransientEpochViewStore, root: &PathBuf, key: &str) -> ! {
    loop {
        // Materialize a bounded window.
        let produced: Vec<(TxIn, TxOut)> = (0..CRASH_CORPUS_N).map(|i| (txin(i), out(i))).collect();
        if let Err(e) = store.materialize_batch(&produced, &position(0)) {
            eprintln!("worker: dispose-loop materialize failed: {e}");
            std::process::exit(6);
        }
        // Dispose it (the kill-during-dispose window).
        if let Err(e) = store.dispose() {
            eprintln!("worker: dispose failed: {e}");
            std::process::exit(7);
        }
        // Re-purge (clean) and reopen for the next pass.
        if let Err(e) = purge_transient_root(root) {
            eprintln!("worker: dispose-loop re-purge failed: {e}");
            std::process::exit(8);
        }
        store = match TransientEpochViewStore::open(root, key) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("worker: dispose-loop reopen failed: {e}");
                std::process::exit(9);
            }
        };
    }
}
