// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Real-capture corpus test for N2N ChainSync (S-A9 partial closure
// for CE-N-A-2). Fixtures captured against preprod testnet via the
// `ade_chain_sync_capture` binary. Each fixture is a mux frame
// (8-byte header + CBOR payload). We strip the mux header, decode
// the ChainSync message with our codec, re-encode it, and assert
// byte-identical round-trip — proving the codec is on the same
// wire grammar as the cardano-node Haskell reference for
// IntersectFound / RollBackward / RollForward (all observed eras
// via the opaque-header skip_item path).

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::fs;
use std::path::Path;

use ade_network::codec::chain_sync::{
    decode_chain_sync_message, encode_chain_sync_message, ChainSyncMessage,
};

const MUX_HEADER_LEN: usize = 8;

fn load_chain_sync_payload(path: &Path) -> Vec<u8> {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    assert!(
        bytes.len() > MUX_HEADER_LEN,
        "{path:?}: capture is shorter than mux header"
    );
    bytes[MUX_HEADER_LEN..].to_vec()
}

fn round_trip_chain_sync_payload(path: &Path) -> ChainSyncMessage {
    let payload = load_chain_sync_payload(path);
    let decoded = decode_chain_sync_message(&payload)
        .unwrap_or_else(|e| panic!("decode {path:?}: {e:?}"));
    let re_encoded = encode_chain_sync_message(&decoded);
    assert_eq!(
        re_encoded,
        payload,
        "{path:?}: encode(decode(bytes)) != bytes — codec is not on the cardano-node wire grammar"
    );
    decoded
}

fn corpus_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .parent()
        .expect("repo root")
        .join("corpus/network/n2n/chain_sync")
}

#[test]
fn real_capture_round_trips_byte_identical() {
    let dir = corpus_dir();
    assert!(dir.is_dir(), "expected corpus directory at {dir:?}");

    let mut frames = 0u32;
    let mut intersect_seen = false;
    let mut rollforward_seen = false;
    let mut rollbackward_seen = false;

    for entry in fs::read_dir(&dir).expect("read corpus dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with("_recv.cbor") {
            continue;
        }
        let msg = round_trip_chain_sync_payload(&path);
        frames += 1;
        match msg {
            ChainSyncMessage::IntersectFound { .. } => intersect_seen = true,
            ChainSyncMessage::RollForward { .. } => rollforward_seen = true,
            ChainSyncMessage::RollBackward { .. } => rollbackward_seen = true,
            _ => {}
        }
    }

    assert!(
        frames >= 4,
        "expected at least 4 captured chain-sync frames; got {frames}"
    );
    assert!(intersect_seen, "expected at least one IntersectFound capture");
    assert!(rollbackward_seen, "expected at least one RollBackward capture");
    assert!(rollforward_seen, "expected at least one RollForward capture");
}

#[test]
fn rollforward_header_opaque_pass_through() {
    let dir = corpus_dir();
    for entry in fs::read_dir(&dir).expect("read corpus dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !(name.ends_with("_recv.cbor") && name.contains("frame_")) {
            continue;
        }
        let msg = round_trip_chain_sync_payload(&path);
        if let ChainSyncMessage::RollForward { header, tip: _ } = msg {
            // Header must be non-empty (a real block header is at least
            // a few bytes of wrapped CBOR).
            assert!(
                !header.is_empty(),
                "RollForward header bytes empty for {path:?}"
            );
            // The wrapping always begins with a CBOR array — first byte
            // major type 4 means top 3 bits are 100. 0x80..=0x9f is the
            // small-array range; 0x9f indefinite; 0x98..=0x9b are
            // length-prefixed. We just need to assert it's *some* array.
            let first = header[0];
            let major = first >> 5;
            assert_eq!(
                major, 4,
                "RollForward header for {path:?} does not start with a CBOR array (first byte 0x{first:02x})"
            );
        }
    }
}

#[test]
fn conway_rollforward_header_served_shape_matches_oracle() {
    // CE-X-4 (CN-WIRE-08): pin the SERVED ChainSync RollForward header
    // shape against the real captured Conway frame from the docker
    // preprod cardano-node 11.0.1
    // (preprod_conway_rollforward_frame_01_recv.cbor). The header MUST be
    // `[era_idx, tag24(bytes(header_cbor))]` with era_idx == 6 (the
    // CONSENSUS index for Conway — NOT the EBB-aware storage 7), and
    // ade's own `compose_rollforward_header(Conway, inner)` MUST
    // reproduce the captured wire bytes BYTE-IDENTICALLY.
    use ade_network::codec::chain_sync::{
        compose_rollforward_header, decompose_rollforward_header,
    };
    use ade_types::CardanoEra;

    let path = corpus_dir().join("preprod_conway_rollforward_frame_01_recv.cbor");
    let payload = load_chain_sync_payload(&path);
    let ChainSyncMessage::RollForward { header, .. } =
        decode_chain_sync_message(&payload).expect("decode")
    else {
        panic!("frame_01 is not a RollForward");
    };

    // Real wire shape: era_idx then a tag-24 wrapped header_cbor.
    let (era_idx, inner) =
        decompose_rollforward_header(&header).expect("real header decomposes");
    assert_eq!(era_idx, 6, "Conway ChainSync header era index must be 6 (consensus)");
    // The tag-24 inner is the bare era-specific header: [header_body, kes_sig].
    assert_eq!(&inner[0..2], &[0x82, 0x8a], "inner is [header_body(array 10), kes_sig]");

    // ade's serve composition reproduces the EXACT captured wire bytes.
    let recomposed = compose_rollforward_header(CardanoEra::Conway, inner);
    assert_eq!(
        recomposed, header,
        "compose_rollforward_header(Conway, inner) must equal the real cardano-node wire header byte-for-byte"
    );
}
