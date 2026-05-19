// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Real-capture corpus test (S-A9 partial closure).
//
// Loads bytes captured by `ade_handshake_capture` against public
// mainnet relays, strips the 8-byte mux header, decodes the inner
// HandshakeMessage CBOR with our codec, re-encodes it, and asserts
// byte-identical round-trip. Proves our codec is on the same wire
// grammar as the cardano-node Haskell reference.
//
// Fixtures live under corpus/network/n2n/handshake/<scenario>_{sent,recv}.cbor.
// Each pair carries an 8-byte mux header followed by handshake CBOR.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::fs;
use std::path::Path;

use ade_network::codec::handshake::{
    decode_handshake_message, encode_handshake_message, HandshakeMessage,
};

const MUX_HEADER_LEN: usize = 8;

fn load_handshake_payload(path: &Path) -> Vec<u8> {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    assert!(
        bytes.len() > MUX_HEADER_LEN,
        "{path:?}: capture is shorter than mux header"
    );
    bytes[MUX_HEADER_LEN..].to_vec()
}

fn round_trip_handshake_payload(path: &Path) -> HandshakeMessage {
    let payload = load_handshake_payload(path);
    let decoded = decode_handshake_message(&payload)
        .unwrap_or_else(|e| panic!("decode {path:?}: {e:?}"));
    let re_encoded = encode_handshake_message(&decoded);
    assert_eq!(
        re_encoded,
        payload,
        "{path:?}: encode(decode(bytes)) != bytes — codec is not on the cardano-node wire grammar"
    );
    decoded
}

#[test]
fn real_capture_round_trips_byte_identical() {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .parent()
        .expect("repo root")
        .join("corpus/network/n2n/handshake");
    assert!(
        corpus_dir.is_dir(),
        "expected corpus directory at {corpus_dir:?}"
    );

    let mut pairs = 0u32;
    for entry in fs::read_dir(&corpus_dir).expect("read corpus dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !(name.ends_with("_sent.cbor") || name.ends_with("_recv.cbor")) {
            continue;
        }
        let _msg = round_trip_handshake_payload(&path);
        pairs += 1;
    }

    assert!(
        pairs >= 6,
        "expected at least 6 captured fixture files (3 sent + 3 recv); got {pairs}. Run `cargo run -p ade_network --bin ade_handshake_capture --release` against mainnet relays to populate the corpus."
    );
}

#[test]
fn iog_capture_accepted_at_known_version() {
    let recv_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .parent()
        .expect("repo root")
        .join("corpus/network/n2n/handshake/mainnet_v11_v16_propose_recv.cbor");
    if !recv_path.exists() {
        // Skip if the IOG capture isn't present; the round-trip test
        // above still exercises any captures that ARE there.
        return;
    }
    let msg = round_trip_handshake_payload(&recv_path);
    match msg {
        HandshakeMessage::AcceptVersion(v, _params) => {
            // backbone.cardano.iog.io picked V15 against our V11..V16
            // proposal (cardano-node 10.7.x supports up to V15). When
            // mainnet rolls to 11.0.1 this expected value flips to V16.
            assert!(
                v.get() == 14 || v.get() == 15 || v.get() == 16,
                "IOG relay accepted at unexpected version {}",
                v.get()
            );
        }
        other => panic!("expected AcceptVersion from IOG, got {other:?}"),
    }
}

#[test]
fn refuse_capture_carries_supported_set() {
    let recv_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .parent()
        .expect("repo root")
        .join("corpus/network/n2n/handshake/iog_refuse_v1_only_recv.cbor");
    if !recv_path.exists() {
        return;
    }
    let msg = round_trip_handshake_payload(&recv_path);
    match msg {
        HandshakeMessage::Refuse(reason) => {
            use ade_network::codec::handshake::RefuseReason;
            match reason {
                RefuseReason::VersionMismatch(supported) => {
                    // Mainnet relay told us its supported set when we
                    // proposed V1-only. Just assert it's non-empty and
                    // contains real-looking cardano-node versions.
                    assert!(!supported.is_empty(), "supported set was empty");
                    for v in &supported {
                        assert!(
                            v.get() >= 11 && v.get() <= 23,
                            "supported set contained out-of-range version {}",
                            v.get()
                        );
                    }
                }
                other => panic!("expected VersionMismatch, got {other:?}"),
            }
        }
        other => panic!("expected Refuse from IOG, got {other:?}"),
    }
}
