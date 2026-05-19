// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Real-capture corpus test for N2C Handshake (S-A9 closure for the
// N2C handshake portion of the local-protocol stack). Fixtures
// captured against a local cardano-node 11.0.1 on preprod via the
// `ade_n2c_handshake_capture` binary talking to the Unix socket
// /ipc/node.socket inside the container (bind-mounted to the host).
//
// Real interop also exposed a codec bug that synthetic round-trip
// tests missed: cardano-node distinguishes N2C version numbers from
// N2N at the handshake layer by OR-ing 0x8000 into the wire integer.
// Semantic V_16 is encoded as CBOR integer 32784 (= 0x8000 + 16).
// Our codec was emitting the bare 16; the server replied with
// `Refuse(VersionMismatch[32784..32791])` listing its set in wire
// encoding. Fixed by adding version_to_wire / wire_to_version
// helpers in n2c_handshake.rs.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use ade_network::codec::n2c_handshake::{
    decode_n2c_handshake_message, encode_n2c_handshake_message, N2cHandshakeMessage,
};

fn round_trip(path: &Path) -> N2cHandshakeMessage {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    let decoded = decode_n2c_handshake_message(&bytes)
        .unwrap_or_else(|e| panic!("decode {path:?}: {e:?}"));
    let re_encoded = encode_n2c_handshake_message(&decoded);
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
        .join("corpus/network/n2c/handshake")
}

#[test]
fn real_capture_round_trips_byte_identical() {
    let dir = corpus_dir();
    assert!(dir.is_dir(), "expected corpus directory at {dir:?}");

    let mut propose_seen = false;
    let mut accept_seen = false;
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
            N2cHandshakeMessage::ProposeVersions(_) => propose_seen = true,
            N2cHandshakeMessage::AcceptVersion(_, _) => accept_seen = true,
            _ => {}
        }
    }

    assert!(frames >= 2, "expected ≥2 captured frames; got {frames}");
    assert!(propose_seen, "expected at least one ProposeVersions capture");
    assert!(accept_seen, "expected at least one AcceptVersion capture");
}

#[test]
fn accept_version_decodes_to_semantic_version_range() {
    // The captured AcceptVersion must decode to a semantic N2C
    // version in the V_16..=V_31 range (cardano-node ouroboros
    // currently uses up to V_23). Anything outside that range likely
    // means we forgot to strip the 0x8000 wire flag and the
    // semantic field still contains it.
    let dir = corpus_dir();
    for entry in fs::read_dir(&dir).expect("read corpus dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !(name.ends_with(".cbor") && name.contains("recv")) {
            continue;
        }
        let msg = round_trip(&path);
        if let N2cHandshakeMessage::AcceptVersion(v, _) = msg {
            let semantic = v.get();
            assert!(
                (16..=31).contains(&semantic),
                "AcceptVersion semantic version out of plausible N2C range: {semantic} in {path:?}"
            );
        }
    }
}
