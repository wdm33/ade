// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Real-capture corpus tests for the 4 N2C mini-protocols
// (LocalStateQuery, LocalChainSync, LocalTxSubmission, LocalTxMonitor).
// Fixtures captured against a local cardano-node 11.0.1 on preprod
// via the `ade_n2c_protocols_capture` binary using the bind-mounted
// Unix socket /ipc/node.socket from the Docker container.
//
// Each test asserts byte-identical round-trip through our codec and
// that at least one message of each expected message kind is present
// in the corpus.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use ade_network::codec::local_chain_sync::{
    decode_local_chain_sync_message, encode_local_chain_sync_message, LocalChainSyncMessage,
};
use ade_network::codec::local_state_query::{
    decode_local_state_query_message, encode_local_state_query_message, LocalStateQueryMessage,
};
use ade_network::codec::local_tx_monitor::{
    decode_local_tx_monitor_message, encode_local_tx_monitor_message, LocalTxMonitorMessage,
};
use ade_network::codec::local_tx_submission::{
    decode_local_tx_submission_message, encode_local_tx_submission_message,
    LocalTxSubmissionMessage,
};

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .parent()
        .expect("repo root")
        .join("corpus/network/n2c")
}

fn read_cbor_files(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return out;
    }
    let mut entries: Vec<_> = fs::read_dir(dir)
        .expect("read dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(".cbor"))
                .unwrap_or(false)
        })
        .collect();
    entries.sort();
    for path in entries {
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        out.push((path, bytes));
    }
    out
}

#[test]
fn local_state_query_real_capture_round_trips() {
    let dir = corpus_root().join("local_state_query");
    let files = read_cbor_files(&dir);
    assert!(!files.is_empty(), "no LSQ captures in {dir:?}");
    let mut acquire_no_point_seen = false;
    let mut acquired_seen = false;
    let mut release_seen = false;
    let mut done_seen = false;
    for (path, bytes) in &files {
        let msg = decode_local_state_query_message(bytes)
            .unwrap_or_else(|e| panic!("decode {path:?}: {e:?}"));
        let reenc = encode_local_state_query_message(&msg);
        assert_eq!(reenc, *bytes, "{path:?}: not byte-identical");
        match msg {
            LocalStateQueryMessage::AcquireNoPoint => acquire_no_point_seen = true,
            LocalStateQueryMessage::Acquired => acquired_seen = true,
            LocalStateQueryMessage::Release => release_seen = true,
            LocalStateQueryMessage::Done => done_seen = true,
            _ => {}
        }
    }
    assert!(acquire_no_point_seen, "expected AcquireNoPoint");
    assert!(acquired_seen, "expected Acquired");
    assert!(release_seen, "expected Release");
    assert!(done_seen, "expected Done");
}

#[test]
fn local_chain_sync_real_capture_round_trips() {
    let dir = corpus_root().join("local_chain_sync");
    let files = read_cbor_files(&dir);
    assert!(!files.is_empty(), "no LocalChainSync captures in {dir:?}");
    let mut find_seen = false;
    let mut intersect_seen = false;
    for (path, bytes) in &files {
        let msg = decode_local_chain_sync_message(bytes)
            .unwrap_or_else(|e| panic!("decode {path:?}: {e:?}"));
        let reenc = encode_local_chain_sync_message(&msg);
        assert_eq!(reenc, *bytes, "{path:?}: not byte-identical");
        match msg {
            LocalChainSyncMessage::FindIntersect { .. } => find_seen = true,
            LocalChainSyncMessage::IntersectFound { .. } => intersect_seen = true,
            _ => {}
        }
    }
    assert!(find_seen, "expected FindIntersect");
    assert!(intersect_seen, "expected IntersectFound");
}

#[test]
fn local_tx_submission_real_capture_round_trips() {
    let dir = corpus_root().join("local_tx_submission");
    let files = read_cbor_files(&dir);
    assert!(!files.is_empty(), "no LTS captures in {dir:?}");
    let mut submit_seen = false;
    for (path, bytes) in &files {
        let msg = decode_local_tx_submission_message(bytes)
            .unwrap_or_else(|e| panic!("decode {path:?}: {e:?}"));
        let reenc = encode_local_tx_submission_message(&msg);
        assert_eq!(reenc, *bytes, "{path:?}: not byte-identical");
        if matches!(msg, LocalTxSubmissionMessage::SubmitTx { .. }) {
            submit_seen = true;
        }
    }
    assert!(submit_seen, "expected SubmitTx");
}

#[test]
fn local_tx_monitor_real_capture_round_trips() {
    let dir = corpus_root().join("local_tx_monitor");
    let files = read_cbor_files(&dir);
    assert!(!files.is_empty(), "no LTM captures in {dir:?}");
    let mut acquire_seen = false;
    let mut acquired_seen = false;
    let mut get_sizes_seen = false;
    let mut reply_sizes_seen = false;
    for (path, bytes) in &files {
        let msg = decode_local_tx_monitor_message(bytes)
            .unwrap_or_else(|e| panic!("decode {path:?}: {e:?}"));
        let reenc = encode_local_tx_monitor_message(&msg);
        assert_eq!(reenc, *bytes, "{path:?}: not byte-identical");
        match msg {
            LocalTxMonitorMessage::Acquire => acquire_seen = true,
            LocalTxMonitorMessage::Acquired { .. } => acquired_seen = true,
            LocalTxMonitorMessage::GetSizes => get_sizes_seen = true,
            LocalTxMonitorMessage::ReplyGetSizes(_) => reply_sizes_seen = true,
            _ => {}
        }
    }
    assert!(acquire_seen, "expected Acquire");
    assert!(acquired_seen, "expected Acquired");
    assert!(get_sizes_seen, "expected GetSizes");
    assert!(reply_sizes_seen, "expected ReplyGetSizes");
}
