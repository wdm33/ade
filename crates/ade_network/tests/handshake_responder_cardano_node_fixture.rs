// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! PHASE4-N-F-G-L S2 (CN-WIRE-10): Ade's serve-side N2N handshake RESPONDER must emit the
//! `versionData` a real cardano-node decodes. Pinned against captured real-cardano-node fixtures
//! (`corpus/network/n2n/handshake/*_v11_v16_propose_recv.cbor`) -- NOT an Ade<->Ade round-trip
//! (which already passed against the prior `[0x01]` placeholder and so cannot catch this).
//!
//! The live failure (G-K rerun) was `HandshakeDecodeError NodeToNodeV_15 "unknown encoding: TInt 1"`
//! -- the responder sent a bare CBOR int `0x01` (TInt 1) instead of the 4-element versionData array.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use ade_network::codec::handshake::{encode_handshake_message, HandshakeMessage};
use ade_network::codec::version::N2NVersion;
use ade_network::handshake::version_table::encode_n2n_version_params;

fn corpus_handshake_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("corpus/network/n2n/handshake")
}

/// The captured fixture is an 8-byte mux header + the handshake payload (the MsgAcceptVersion CBOR).
/// The mux header carries a per-capture timestamp, so we compare the PAYLOAD (bytes after the header).
fn fixture_payload(name: &str) -> Vec<u8> {
    let recv = std::fs::read(corpus_handshake_dir().join(name))
        .unwrap_or_else(|e| panic!("handshake fixture {name} present: {e:?}"));
    assert!(recv.len() > 8, "fixture has a mux header + payload");
    recv[8..].to_vec()
}

/// CE-G-L-1: Ade's serve responder V15 accept is byte-identical to the captured real cardano-node
/// reply (public preprod, magic 1). This is the proof the prior `[0x01]` placeholder failed.
#[test]
fn responder_v15_accept_matches_real_cardano_node_preprod_fixture() {
    let payload = fixture_payload("preprod_v11_v16_propose_recv.cbor");
    let ours = encode_handshake_message(&HandshakeMessage::AcceptVersion(
        N2NVersion::new(15),
        encode_n2n_version_params(15, 1),
    ));
    assert_eq!(
        ours, payload,
        "Ade serve responder V15 accept must byte-match the captured real cardano-node reply \
         (NOT an Ade<->Ade round-trip). ours={ours:02x?} fixture={payload:02x?}"
    );
}

/// CE-G-L-1 (the EXACT failing peer): byte-match the C1 private-net fixture (magic 42) whose dial
/// produced the live `unknown encoding: TInt 1`.
#[test]
fn responder_v15_accept_matches_failing_c1_peer_fixture() {
    let payload = fixture_payload("c1privnet_v11_v16_propose_recv.cbor");
    let ours = encode_handshake_message(&HandshakeMessage::AcceptVersion(
        N2NVersion::new(15),
        encode_n2n_version_params(15, 42),
    ));
    assert_eq!(
        ours, payload,
        "Ade serve responder V15 accept must byte-match the C1 failing-peer fixture. \
         ours={ours:02x?} fixture={payload:02x?}"
    );
}

/// Regression: the V15 versionData must be a 4-element CBOR array (0x84), NEVER the bare int the
/// real peer rejected (`0x01` = TInt 1). The accept payload is `[83 01 0f <versionData...>]`, so
/// byte 3 is the versionData head.
#[test]
fn responder_v15_versiondata_is_a_four_element_array_not_a_bare_int() {
    let ours = encode_handshake_message(&HandshakeMessage::AcceptVersion(
        N2NVersion::new(15),
        encode_n2n_version_params(15, 1),
    ));
    assert_eq!(&ours[0..3], &[0x83, 0x01, 0x0f], "MsgAcceptVersion head [1, 15, ...]");
    assert_eq!(
        ours[3], 0x84,
        "V15 versionData must be a 4-element CBOR array (0x84), never the old TInt-1 placeholder"
    );
}
