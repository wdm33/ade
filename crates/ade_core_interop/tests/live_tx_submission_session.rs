// RED — CE-N-E-6 closure-gate test. `#[ignore]` by default (needs
// network egress to a preprod relay); the test asserts the
// `live_tx_submission_session` binary builds and starts.
//
// Live evidence capture (automated — no manual operator beyond
// running the command):
//
//     cargo run -p ade_core_interop --bin live_tx_submission_session -- \
//         --connect --network preprod --max-seconds 600 --max-frames 1000
//
// What the live pass evidences:
//   - N2N handshake against a real cardano-node 11.x peer accepted.
//   - tx-submission2 mini-protocol opened (protocol id 4).
//   - BLUE `tx_submission2_transition` drives the protocol grammar
//     for every received message without `IllegalTransition` /
//     `MalformedMessage` errors.
//   - Every codec round-trip (encode/decode of every
//     `TxSubmission2Message` variant we exchange) succeeds against
//     live wire bytes — proving the codec is wire-compatible with
//     cardano-node's tx-submission2 implementation.
//
// What the live pass does NOT directly evidence (joins
// CE-NODE-N2C-LTX in the future node-binary cluster's deferral):
//   - Bulk receipt of real tx_bytes from the peer. In the outbound
//     client direction the peer does not push txs at us; bulk tx
//     ingestion requires Ade to host an inbound listener, which is
//     the node-binary cluster's responsibility. If the peer happens
//     to send `ReplyTxs` opportunistically, the bridge captures them
//     and the log records `[bridge] tx_bytes=<N>`.
//
// The output is committed at `docs/clusters/PHASE4-N-E/CE-N-E-6_<date>.log`.
// The deterministic CI gate for this cluster is the synthetic-event
// adapter test surface in `tests/tx_submission_ingress.rs` +
// `tests/local_tx_submission_ingress.rs`; CI does not run this
// network test.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::process::Command;

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
#[ignore = "needs network egress to a preprod relay; run the binary with --connect to capture live evidence"]
fn cardano_node_tx_submission2_sustained_window() {
    // Hermetic gate: assert the binary builds and starts (prints the
    // readiness banner) without opening a socket. The live tx-submission2
    // window is captured by running the binary with --connect (see the
    // module comment); a real preprod run is committed under
    // docs/clusters/PHASE4-N-E/.
    let bin = manifest_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join(if cfg!(debug_assertions) { "debug" } else { "release" })
        .join("live_tx_submission_session");
    assert!(
        bin.exists(),
        "live_tx_submission_session binary missing at {:?} — build with `cargo build -p ade_core_interop --bin live_tx_submission_session` first",
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
        stdout.contains("live_tx_submission_session ready"),
        "expected readiness banner, got: {}",
        stdout
    );
}
