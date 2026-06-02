// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration test — BA-02 bounty gate archived-home regression
//! (fix(ci): scan archived G-C home in BA-02 evidence gate).
//!
//! Mirrors the PHASE4-N-F-G-D S4 rehearsal-gate test. The BA-02 evidence gate
//! (`ci/ci_check_ba02_evidence_manifest_schema.sh`) MUST validate a committed
//! `CE-G-C-LIVE_*.toml` manifest under ANY real G-C home — including the ARCHIVED
//! home `docs/clusters/completed/PHASE4-N-F-G-C/`. Before the fix it `find`d only
//! the (now-archived, absent) active home behind a swallow-all guard, so a
//! manifest under the archived home was silently un-validated. This test plants a
//! malformed manifest under the archived home and asserts the gate fails,
//! cleaning the fixture via a Drop guard (clean on panic).

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
        .arg(root.join("ci/ci_check_ba02_evidence_manifest_schema.sh"))
        .current_dir(root)
        .output()
        .expect("run ci_check_ba02_evidence_manifest_schema.sh via bash")
}

/// Removes the planted fixture on drop — clean even if an assertion panics.
struct FixtureGuard(PathBuf);
impl Drop for FixtureGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// The BA-02 gate validates manifests under the ARCHIVED G-C home. Clean tree =>
/// green (vacuous, no manifest committed); a malformed `CE-G-C-LIVE_*.toml` under
/// the archived home => gate fails (it would have PASSED, silently un-validated,
/// before the archived-home fix).
#[test]
fn ba02_gate_validates_archived_home_manifest() {
    let root = repo_root();
    let archived = root.join("docs/clusters/completed/PHASE4-N-F-G-C");
    assert!(
        archived.is_dir(),
        "precondition: the archived G-C home must exist at {}",
        archived.display()
    );

    // (1) Clean tree: the gate is green (vacuous — no CE-G-C-LIVE manifest committed
    // under either home).
    let clean = run_gate(&root);
    assert!(
        clean.status.success(),
        "gate must be green on the clean tree; stdout={} stderr={}",
        String::from_utf8_lossy(&clean.stdout),
        String::from_utf8_lossy(&clean.stderr)
    );

    // (2) Archived-home malformed manifest: a CE-G-C-LIVE_*.toml missing required
    // fields under the ARCHIVED home must make the gate FAIL closed (proving it is
    // now scanned + validated there).
    let smuggle = archived.join(format!("CE-G-C-LIVE_gfnegtest_{}.toml", std::process::id()));
    let _guard = FixtureGuard(smuggle.clone());
    // schema_version present, everything else required (block_hash, slot, peer_log_file,
    // peer_log_file_sha256, peer_log_capture_command, peer_log_filter, accept_event_kind)
    // absent => the gate's required-fields check fails.
    std::fs::write(&smuggle, "schema_version = 1\n").expect("write malformed manifest fixture");

    let malformed = run_gate(&root);
    assert!(
        !malformed.status.success(),
        "gate MUST fail on a malformed CE-G-C-LIVE manifest under the archived home {}; \
         stdout={} stderr={}",
        smuggle.display(),
        String::from_utf8_lossy(&malformed.stdout),
        String::from_utf8_lossy(&malformed.stderr)
    );
    // `_guard` removes the fixture on scope exit (including on a panic above).
}
