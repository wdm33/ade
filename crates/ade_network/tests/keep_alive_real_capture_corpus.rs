// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Real-capture corpus test for N2N KeepAlive (S-A9 closure for
// CE-N-A-5). Fixtures captured against a local cardano-node 11.0.1
// on preprod via the `ade_keep_alive_capture` binary.
//
// Each fixture is the mux-reassembled CBOR payload of one KeepAlive
// message. We decode, re-encode, and assert byte-identical
// round-trip — proving the codec is on the cardano-node wire grammar
// for MsgKeepAlive / MsgResponseKeepAlive.
//
// Additionally we assert the COOKIE ECHO invariant: for each round,
// the cookie in the recv `_response` message equals the cookie in
// the immediately-preceding send `_keep_alive` message. This is the
// minimum semantic check the protocol guarantees and the only thing
// the responder owes us beyond raw byte-shape.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use ade_network::codec::keep_alive::{
    decode_keep_alive_message, encode_keep_alive_message, KeepAliveCookie, KeepAliveMessage,
};

fn round_trip(path: &Path) -> KeepAliveMessage {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    let decoded = decode_keep_alive_message(&bytes)
        .unwrap_or_else(|e| panic!("decode {path:?}: {e:?}"));
    let re_encoded = encode_keep_alive_message(&decoded);
    assert_eq!(
        re_encoded, bytes,
        "{path:?}: encode(decode(bytes)) != bytes"
    );
    decoded
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .parent()
        .expect("repo root")
        .join("corpus/network/n2n/keep_alive")
}

fn sorted_msg_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .expect("read corpus dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(".cbor"))
                .unwrap_or(false)
        })
        .collect();
    files.sort();
    files
}

#[test]
fn real_capture_round_trips_byte_identical() {
    let dir = corpus_dir();
    assert!(dir.is_dir(), "expected corpus directory at {dir:?}");

    let files = sorted_msg_files(&dir);
    assert!(
        files.len() >= 2,
        "expected at least one send+recv pair; got {} files",
        files.len()
    );

    let mut send_seen = false;
    let mut recv_seen = false;
    for path in &files {
        let msg = round_trip(path);
        match msg {
            KeepAliveMessage::KeepAlive(_) => send_seen = true,
            KeepAliveMessage::ResponseKeepAlive(_) => recv_seen = true,
            KeepAliveMessage::Done => {}
        }
    }
    assert!(send_seen, "expected at least one MsgKeepAlive capture");
    assert!(recv_seen, "expected at least one MsgResponseKeepAlive capture");
}

#[test]
fn cookie_echo_invariant_holds() {
    // Filenames are <scenario>_msg_NN_<send_keep_alive|recv_response>.cbor;
    // sort order pairs each send with the next recv. Each pair must
    // share a cookie.
    let dir = corpus_dir();
    let files = sorted_msg_files(&dir);
    let mut pending_cookie: Option<KeepAliveCookie> = None;
    let mut pairs_checked = 0u32;
    for path in &files {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let msg = round_trip(path);
        match (&msg, name) {
            (KeepAliveMessage::KeepAlive(c), _) if name.contains("_send_") => {
                pending_cookie = Some(*c);
            }
            (KeepAliveMessage::ResponseKeepAlive(c), _) if name.contains("_recv_") => {
                let sent = pending_cookie.take().unwrap_or_else(|| {
                    panic!("recv {path:?} with no preceding send")
                });
                assert_eq!(
                    sent.0, c.0,
                    "cookie mismatch for {path:?}: sent 0x{:04x}, got 0x{:04x}",
                    sent.0, c.0
                );
                pairs_checked += 1;
            }
            _ => {}
        }
    }
    assert!(
        pairs_checked >= 1,
        "expected at least one send/recv pair; checked {pairs_checked}"
    );
}
