// RED — CE-N-H-6 closure-gate test. The hermetic default mode is
// fully testable in CI; the live `--connect` mode is operator-
// action and not run in CI.
//
// Live evidence capture (operator-action; not run in CI):
//
//     cargo run -p ade_core_interop --bin live_block_follow_session -- \
//         --connect --network preprod --target <host:port>
//
// See docs/clusters/PHASE4-N-H/CE-N-H-6_PROCEDURE.md for the
// operator procedure. The deterministic CI gate is the offline
// receive_pipeline_corpus_drive + receive_session_transcript_replay
// tests in crates/ade_runtime/tests/.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::process::Command;

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn live_block_follow_session_hermetic_default_prints_readiness() {
    let bin = manifest_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join("debug")
        .join("live_block_follow_session");
    if !bin.exists() {
        let build = Command::new(env!("CARGO"))
            .args([
                "build",
                "-p",
                "ade_core_interop",
                "--bin",
                "live_block_follow_session",
            ])
            .status()
            .expect("cargo build");
        assert!(build.success(), "binary build failed");
    }
    let out = Command::new(&bin)
        .output()
        .expect("run live_block_follow_session in hermetic mode");
    assert!(out.status.success(), "hermetic run must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("live_block_follow_session ready"),
        "expected readiness banner, got: {stdout}"
    );
    assert!(
        stdout.contains("pass --connect for the operator live pass"),
        "expected --connect hint, got: {stdout}"
    );
}
