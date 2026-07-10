// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration test — LIVE-LEDGER-EPOCH-TRANSITION S4-pre-1c (DC-EPOCH-25).
//!
//! Durable regression guard for the frozen-leadership authority CI gate
//! (`ci/ci_check_frozen_leadership_authority.sh`): the quarantined go+active-params leadership builder
//! `from_accumulator_go_active_params_for_test_only` (a DISPROVEN hypothesis — active params drop a
//! retired-but-leadership-relevant pool's VRF) must be referenced ONLY from its definition + test / oracle /
//! negative-regression code, never a production authority path. This test shells the real gate and asserts
//! (1) it is green on the clean tree and (2) it fails closed on a planted production leak, cleaning the
//! fixture via a Drop guard (clean even on a panic).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Repo root = `<CARGO_MANIFEST_DIR>/../..` (CARGO_MANIFEST_DIR is `<repo>/crates/ade_node`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("canonicalize repo root")
}

fn run_gate(root: &Path) -> Output {
    Command::new("bash")
        .arg(root.join("ci/ci_check_frozen_leadership_authority.sh"))
        .current_dir(root)
        .output()
        .expect("run ci_check_frozen_leadership_authority.sh via bash")
}

/// Removes the planted fixture on drop — clean even if an assertion panics.
struct FixtureGuard(PathBuf);
impl Drop for FixtureGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// The gate is green on the clean tree, and fails closed when a non-test / non-definition production code site
/// references the quarantined builder. Both runs live in one test fn so the clean-tree run cannot race the
/// planted fixture.
#[test]
fn frozen_leadership_gate_fails_on_production_leak() {
    let root = repo_root();

    // (1) Clean tree: the gate is green (the builder is referenced only from its definition + tests).
    let clean = run_gate(&root);
    assert!(
        clean.status.success(),
        "gate must be green on the clean tree; stdout={} stderr={}",
        String::from_utf8_lossy(&clean.stdout),
        String::from_utf8_lossy(&clean.stderr)
    );

    // (2) Production leak: a non-test, non-definition `.rs` reference to the quarantined builder must make the
    // gate FAIL closed. Planted at `crates/` root — inside the gate's `crates/ --include='*.rs'` scan but
    // OUTSIDE any crate's compiled module tree, so cargo never touches it.
    let leak = root.join(format!(
        "crates/__frozen_leadership_gate_selftest_{}.rs",
        std::process::id()
    ));
    let _guard = FixtureGuard(leak.clone());
    std::fs::write(
        &leak,
        "fn leak() { let _ = PoolDistrView::from_accumulator_go_active_params_for_test_only(&a, b); }\n",
    )
    .expect("write leak fixture");

    let leaked = run_gate(&root);
    assert!(
        !leaked.status.success(),
        "gate MUST fail when a production code site references the quarantined builder ({}); stdout={} stderr={}",
        leak.display(),
        String::from_utf8_lossy(&leaked.stdout),
        String::from_utf8_lossy(&leaked.stderr)
    );
    // `_guard` removes the fixture on scope exit (including on a panic above).
}
