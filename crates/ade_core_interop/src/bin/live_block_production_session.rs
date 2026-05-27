// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! **DEPRECATED shim** — PHASE4-N-R-C C3.
//!
//! This binary was the PHASE4-N-C operator-evidence harness for
//! CE-N-C-8. It has been **retired in favor of**
//! `ade_node --mode produce`, which composes the real forge
//! pipeline (`run_real_forge` in `ade_node::produce_mode`)
//! shipped in PHASE4-N-R-A and the per-peer dispatch +
//! `ServedChainHandle` shipped in PHASE4-N-R-B.
//!
//! No alternate flags. No alternate defaults. No duplicate
//! parser logic. This shim prints a deprecation banner and
//! exits with a non-zero status to surface that the binary
//! has moved.
//!
//! Per DQ-C3 (locked at N-R planning): the binary name is
//! **kept** (`live_block_production_session`) to preserve
//! operator muscle memory. The invariant is "no independent
//! legacy production path," not "no old binary name."

fn main() {
    eprintln!(
        "DEPRECATED: live_block_production_session is a shim; \
         invoking produce_mode::run_produce_mode."
    );
    eprintln!();
    eprintln!("This binary has been retired. The operator-pass evidence");
    eprintln!("harness now lives in `ade_node --mode produce`, composing");
    eprintln!("the PHASE4-N-R-A real-forge pipeline + PHASE4-N-R-B");
    eprintln!("served-snapshot + per-peer dispatch.");
    eprintln!();
    eprintln!("Run:");
    eprintln!("  cargo run --bin ade_node -- --mode produce \\");
    eprintln!("    --listen 127.0.0.1:3001 \\");
    eprintln!("    --cold-skey <path> --kes-skey <path> --vrf-skey <path> \\");
    eprintln!("    --opcert <path> --genesis-file <path> \\");
    eprintln!("    --evidence-log <path>");
    eprintln!();
    eprintln!("See docs/active/cn-cons-06-operator-runbook.md.");
    std::process::exit(2);
}
