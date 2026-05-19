// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Real-capture corpus test for N2N PeerSharing (S-A9 closure for
// CE-N-A-6). Fixtures captured against a local cardano-node 11.0.1
// on preprod with `PeerSharing: true` enabled in node config, via
// the `ade_peer_sharing_capture` binary.
//
// Each fixture is the mux-reassembled CBOR payload of one
// PeerSharing message. We decode, re-encode, and assert
// byte-identical round-trip — proving the codec is on the
// cardano-node wire grammar for MsgShareRequest / MsgSharePeers.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use ade_network::codec::peer_sharing::{
    decode_peer_sharing_message, encode_peer_sharing_message, PeerSharingMessage,
};

fn round_trip(path: &Path) -> PeerSharingMessage {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    let decoded = decode_peer_sharing_message(&bytes)
        .unwrap_or_else(|e| panic!("decode {path:?}: {e:?}"));
    let re_encoded = encode_peer_sharing_message(&decoded);
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
        .join("corpus/network/n2n/peer_sharing")
}

#[test]
fn real_capture_round_trips_byte_identical() {
    let dir = corpus_dir();
    assert!(dir.is_dir(), "expected corpus directory at {dir:?}");

    let mut request_seen = false;
    let mut reply_seen = false;
    let mut frames = 0u32;

    for entry in fs::read_dir(&dir).expect("read corpus dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".cbor") {
            continue;
        }
        let msg = round_trip(&path);
        frames += 1;
        match msg {
            PeerSharingMessage::ShareRequest { .. } => request_seen = true,
            PeerSharingMessage::SharePeers { .. } => reply_seen = true,
            PeerSharingMessage::Done => {}
        }
    }

    assert!(frames >= 2, "expected ≥2 captured frames; got {frames}");
    assert!(request_seen, "expected at least one MsgShareRequest capture");
    assert!(reply_seen, "expected at least one MsgSharePeers capture");
}
