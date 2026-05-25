// RED — CE-N-G-8 closure-gate test. `#[ignore]` by default (needs a
// private cardano-node peer); the test asserts the
// `live_block_fetch_session` binary builds and starts in the
// hermetic default mode.
//
// Live evidence capture (operator-action; not run in CI):
//
//     cargo run -p ade_core_interop --bin live_block_fetch_session -- \
//         --connect --network preprod --target <host:port>
//
// This drives the n2n_server reducers against a real cardano-node
// peer issuing RequestRange over an Ade-forged block (or, pre-stake
// provisioning, over a synthetic pre-captured AcceptedBlock); see
// docs/clusters/PHASE4-N-G/CE-N-G-8_PROCEDURE.md for the operator
// procedure. The deterministic CI gate is the offline
// cross_impl_server_pipeline + server_paths_transcript_replay tests
// in crates/ade_runtime/tests/.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::process::Command;

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn live_block_fetch_session_hermetic_default_prints_readiness() {
    // Hermetic gate: assert the binary builds and starts in default
    // (no --connect) mode without opening a socket.
    let bin = manifest_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join("debug")
        .join("live_block_fetch_session");
    if !bin.exists() {
        let build = Command::new(env!("CARGO"))
            .args(["build", "-p", "ade_core_interop", "--bin", "live_block_fetch_session"])
            .status()
            .expect("cargo build");
        assert!(build.success(), "binary build failed");
    }
    let out = Command::new(&bin)
        .output()
        .expect("run live_block_fetch_session in hermetic mode");
    assert!(out.status.success(), "hermetic run must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("live_block_fetch_session ready"),
        "expected readiness banner, got: {stdout}"
    );
    assert!(
        stdout.contains("pass --connect for the operator live pass"),
        "expected --connect hint, got: {stdout}"
    );
}
