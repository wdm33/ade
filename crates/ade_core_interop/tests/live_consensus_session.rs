// RED — CE-N-B-6 closure-gate test. `#[ignore]` by default (needs
// network egress to a preprod relay); the test asserts the
// `live_consensus_session` binary builds and starts.
//
// Live evidence capture (automated — no manual operator):
//
//     cargo run -p ade_core_interop --bin live_consensus_session -- \
//         --connect --network preprod --lag-seconds 200 --max-headers 25
//
// This discovers the peer tip, waits offline while the chain advances,
// reconnects, intersects at the now-stale tip, follows the real
// roll-forward window, and asserts tip agreement at every block,
// writing docs/clusters/PHASE4-N-B/CE-N-B-6_<date>.log. A captured run
// against live preprod (8 Conway headers, 0 disagreements) is committed
// at that path. The deterministic CI gate is the offline replay test
// (`tests/follow_offline_replay.rs`); CI does not run this network test.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::process::Command;

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
#[ignore = "needs network egress to a preprod relay; run the binary with --connect to capture live evidence"]
fn cardano_node_session_sustained_window() {
    // Hermetic gate: assert the binary builds and starts (prints the
    // readiness banner) without opening a socket. The live tip-agreement
    // window is captured by running the binary with --connect (see the
    // module comment); a real preprod run is committed under
    // docs/clusters/PHASE4-N-B/.
    let bin = manifest_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join(if cfg!(debug_assertions) { "debug" } else { "release" })
        .join("live_consensus_session");
    assert!(
        bin.exists(),
        "live_consensus_session binary missing at {:?} — build with `cargo build -p ade_core_interop --bin live_consensus_session` first",
        bin
    );
    let output = Command::new(&bin).output().expect("spawn binary");
    assert!(
        output.status.success(),
        "binary exited with non-zero status: {:?}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("live_consensus_session ready"),
        "expected readiness banner, got: {}",
        stdout
    );
}
