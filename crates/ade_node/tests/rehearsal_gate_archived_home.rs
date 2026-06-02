// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration test — PHASE4-N-F-G-D S4 (rehearsal leak gate archived-home hardening).
//!
//! Durable regression guard for non-promotability barrier (b): the rehearsal
//! manifest gate (`ci/ci_check_rehearsal_manifest_schema.sh`) MUST catch a
//! rehearsal marker committed under ANY real G-C bounty-evidence home — including
//! the ARCHIVED home `docs/clusters/completed/PHASE4-N-F-G-C/`. Before S4 the gate
//! scanned only the (non-existent) active home behind an `[[ -d ]]` guard, so the
//! check silently skipped after G-C's archival; the absence of THIS test is what
//! let that ship green. The test shells the real gate and asserts it fails on an
//! archived-home smuggle, cleaning the fixture via a Drop guard (clean on panic).

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
        .arg(root.join("ci/ci_check_rehearsal_manifest_schema.sh"))
        .current_dir(root)
        .output()
        .expect("run ci_check_rehearsal_manifest_schema.sh via bash")
}

/// Removes the planted fixture on drop — clean even if an assertion panics.
struct FixtureGuard(PathBuf);
impl Drop for FixtureGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Non-promotability barrier (b): a rehearsal marker under the ARCHIVED G-C
/// bounty home must make the gate fail closed. Also asserts the gate is green on
/// the clean tree (vacuous — no rehearsal manifest committed). Both runs live in
/// one test fn so the clean-tree run cannot race the planted fixture.
#[test]
fn rehearsal_gate_fails_on_archived_home_leak() {
    let root = repo_root();
    let archived = root.join("docs/clusters/completed/PHASE4-N-F-G-C");
    assert!(
        archived.is_dir(),
        "precondition: the archived G-C home must exist at {}",
        archived.display()
    );

    // (1) Clean tree: the gate is green (vacuous — no rehearsal manifest committed,
    // no marker under either bounty home).
    let clean = run_gate(&root);
    assert!(
        clean.status.success(),
        "gate must be green on the clean tree; stdout={} stderr={}",
        String::from_utf8_lossy(&clean.stdout),
        String::from_utf8_lossy(&clean.stderr)
    );

    // (2) Archived-home smuggle: a rehearsal-marked .toml under the ARCHIVED G-C
    // home must make the gate FAIL closed. (This exact smuggle PASSED before S4.)
    let smuggle = archived.join(format!("CE-G-C-LIVE_s4negtest_{}.toml", std::process::id()));
    let _guard = FixtureGuard(smuggle.clone());
    std::fs::write(&smuggle, "is_rehearsal = true\nnot_bounty_evidence = true\n")
        .expect("write smuggle fixture");

    let leaked = run_gate(&root);
    assert!(
        !leaked.status.success(),
        "gate MUST fail when a rehearsal marker is under the archived bounty home {}; \
         stdout={} stderr={}",
        smuggle.display(),
        String::from_utf8_lossy(&leaked.stdout),
        String::from_utf8_lossy(&leaked.stderr)
    );
    // `_guard` removes the fixture on scope exit (including on a panic above).
}
