// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Worker binary for the S-37 stress harness.
//!
//! Opens the persistent chaindb at the path passed as argv[1] and
//! writes synthetic blocks in a tight loop until killed by SIGKILL
//! from outside. Hash is derived from slot so idempotent re-puts
//! after restart succeed (same slot → same hash → trait's
//! idempotent path).
//!
//! Not a user-facing binary; only intended to be invoked by the
//! `tests/stress_kill_harness.rs` integration test.

use std::path::PathBuf;

use ade_runtime::chaindb::{
    ChainDb, PersistentChainDb, PersistentChainDbOptions, StoredBlock,
};
use ade_types::primitives::{Hash32, SlotNo};

fn slot_hash(slot: u64) -> Hash32 {
    let mut h = [0u8; 32];
    h[..8].copy_from_slice(&slot.to_le_bytes());
    Hash32(h)
}

fn main() {
    let path: PathBuf = std::env::args()
        .nth(1)
        .expect("argv[1] must be the chaindb path")
        .into();

    let db = match PersistentChainDb::open(PersistentChainDbOptions::at(&path)) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("worker: open failed: {e}");
            std::process::exit(2);
        }
    };

    // Resume from where the previous (possibly-killed) iteration left off.
    let start_slot = match db.tip() {
        Ok(Some(tip)) => tip.slot.0.saturating_add(1),
        Ok(None) => 1,
        Err(e) => {
            eprintln!("worker: tip failed: {e}");
            std::process::exit(3);
        }
    };

    let mut slot = start_slot;
    loop {
        let block = StoredBlock {
            slot: SlotNo(slot),
            hash: slot_hash(slot),
            bytes: vec![(slot & 0xff) as u8; 64],
        };
        match db.put_block(&block) {
            Ok(()) => {}
            Err(e) => {
                // Idempotent re-put should succeed; any other error is
                // unexpected. Print and exit non-zero so the harness
                // can distinguish kill (exit code from SIGKILL) vs
                // legitimate worker failure.
                eprintln!("worker: put_block at slot {slot} failed: {e}");
                std::process::exit(4);
            }
        }
        slot = slot.wrapping_add(1);
        if slot == 0 {
            // u64 overflow guard — synthetic; in practice we never
            // reach this in 1000 iterations.
            slot = 1;
        }
    }
}
