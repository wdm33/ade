// GREEN — minimal helpers for reading consensus corpus files. Used by
// integration tests in `ade_core/tests/` and `ade_runtime/tests/`.
// Non-authoritative.

use std::path::PathBuf;

/// Resolve a path inside `corpus/consensus/<dir>/<name>`. The path is
/// relative to the workspace root, derived from `CARGO_MANIFEST_DIR`.
///
/// This helper takes the manifest dir of the calling test crate as a
/// parameter so it works equally well from `ade_core/tests/` and
/// `ade_runtime/tests/`.
pub fn corpus_path(manifest_dir: &str, dir: &str, name: &str) -> PathBuf {
    let mut p = PathBuf::from(manifest_dir);
    // crates/<crate>/  -> workspace root
    p.pop();
    p.pop();
    p.push("corpus");
    p.push("consensus");
    p.push(dir);
    p.push(name);
    p
}

/// Convenience: resolve `corpus/consensus/nonce_evolution/<name>`.
pub fn nonce_evolution_corpus_path(manifest_dir: &str, name: &str) -> PathBuf {
    corpus_path(manifest_dir, "nonce_evolution", name)
}
