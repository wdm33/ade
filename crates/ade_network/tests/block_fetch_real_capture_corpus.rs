// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Real-capture corpus test for N2N BlockFetch (S-A9 closure for
// CE-N-A-3). Fixtures captured against a local cardano-node 11.0.1
// running on preprod (network magic 1) via the
// `ade_block_fetch_capture` binary.
//
// Each fixture is the mux-reassembled CBOR payload of one BlockFetch
// message (no mux header — `capture_block_fetch.rs` reassembles
// fragmented frames before writing). We decode it with our codec,
// re-encode, and assert byte-identical round-trip — proving the
// codec is on the same wire grammar as the cardano-node Haskell
// reference for the MsgStartBatch / MsgBlock / MsgBatchDone server
// trio that follows a RequestRange.
//
// Real interop also exposed the codec bug that synthetic vectors
// missed: MsgRequestRange wire form is FLAT `[0, from, to]`, not the
// nested `[0, [from, to]]` we originally emitted. cardano-node
// reports `DeserialiseFailure "unexpected key (0, 2)"` for the
// nested form; the flat form is accepted and the server replies
// with the trio captured here.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use ade_network::codec::block_fetch::{
    decode_block_fetch_message, encode_block_fetch_message, BlockFetchMessage,
};

fn round_trip(path: &Path) -> BlockFetchMessage {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    let decoded = decode_block_fetch_message(&bytes)
        .unwrap_or_else(|e| panic!("decode {path:?}: {e:?}"));
    let re_encoded = encode_block_fetch_message(&decoded);
    assert_eq!(
        re_encoded, bytes,
        "{path:?}: encode(decode(bytes)) != bytes — codec is not on the cardano-node wire grammar"
    );
    decoded
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .parent()
        .expect("repo root")
        .join("corpus/network/n2n/block_fetch")
}

#[test]
fn real_capture_round_trips_byte_identical() {
    let dir = corpus_dir();
    assert!(dir.is_dir(), "expected corpus directory at {dir:?}");

    let mut start_batch_seen = false;
    let mut block_seen = false;
    let mut batch_done_seen = false;
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
            BlockFetchMessage::StartBatch => start_batch_seen = true,
            BlockFetchMessage::Block { .. } => block_seen = true,
            BlockFetchMessage::BatchDone => batch_done_seen = true,
            _ => {}
        }
    }

    assert!(
        frames >= 3,
        "expected at least 3 captured block-fetch frames; got {frames}"
    );
    assert!(start_batch_seen, "expected at least one MsgStartBatch capture");
    assert!(block_seen, "expected at least one MsgBlock capture");
    assert!(batch_done_seen, "expected at least one MsgBatchDone capture");
}

#[test]
fn block_body_is_wrapped_cbor_item() {
    // CE-X-2 (CN-WIRE-08): the REAL cardano-node 11.0.1 MsgBlock body is
    // a bare tag-24 CBOR-in-CBOR wrap of the era-tagged storage block:
    // `tag24(bytes([era, block]))` — first bytes `0xd8 0x18`, NO
    // serialisationInfo word. We pin the exact shape against the
    // captured oracle (not the looser "array OR tag" hedge the prior
    // assertion allowed): the payload MUST strip via the shared tag-24
    // authority (`decompose_blockfetch_block`) and the inner MUST decode
    // through the canonical `[era, block]` envelope authority.
    use ade_network::codec::block_fetch::decompose_blockfetch_block;

    let dir = corpus_dir();
    let mut checked = 0u32;
    for entry in fs::read_dir(&dir).expect("read corpus dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !(name.ends_with(".cbor") && name.contains("_block.")) {
            continue;
        }
        let msg = round_trip(&path);
        if let BlockFetchMessage::Block { bytes } = msg {
            assert!(bytes.len() >= 2, "MsgBlock body too short for {path:?}");
            // Bare tag-24 marker, NOT a [serialisationInfo, ...] array.
            assert_eq!(
                &bytes[0..2],
                &[0xd8, 0x18],
                "MsgBlock body for {path:?} must start with tag(24) 0xd8 0x18 \
                 (first bytes 0x{:02x} 0x{:02x}); a serialisationInfo array would be wrong",
                bytes[0],
                bytes[1],
            );
            let inner = decompose_blockfetch_block(&bytes)
                .unwrap_or_else(|e| panic!("{path:?}: tag-24 unwrap failed: {e:?}"));
            let env = ade_codec::cbor::envelope::decode_block_envelope(inner)
                .unwrap_or_else(|e| panic!("{path:?}: inner is not a valid [era,block]: {e:?}"));
            assert!(
                env.block_end > env.block_start,
                "{path:?}: decoded envelope has empty inner block"
            );
            checked += 1;
        }
    }
    assert!(checked >= 1, "expected at least one captured MsgBlock to pin");
}
#[test]
fn captured_block_era_index_matches_ade_storage_scheme() {
    // Wire-vs-storage era-index check (CN-WIRE-08 / proof obligation #1).
    // The tag-24 inner is the HFC `[era_index, block]` envelope. The
    // cardano-node N2N block-fetch era index uses the SAME EBB-aware
    // 0..=7 numbering as ade's storage envelope (ByronEbb=0 .. Allegra=3
    // .. Conway=7), so the served block needs NO era-index translation —
    // `compose_blockfetch_block(storage_bytes)` is wire-correct.
    //
    // This S-A9 fixture is a historical Allegra-era preprod block
    // (era index 3): the inner decodes through the SAME
    // decode_block_envelope authority and yields era == Allegra. (Full
    // decode_block is Babbage/Conway-only, so it is expected to reject an
    // Allegra body — that is a decoder-coverage boundary, not a wire-shape
    // mismatch.) Conway tip blocks travel as era index 7 on the wire, as
    // proven by the N-M-FRAG/FOLLOW live admission path.
    use ade_network::codec::block_fetch::decompose_blockfetch_block;
    use ade_types::CardanoEra;

    let path = corpus_dir().join("local_preprod_tip_msg_01_block.cbor");
    let bytes = fs::read(&path).expect("read fixture");
    let BlockFetchMessage::Block { bytes: payload } =
        decode_block_fetch_message(&bytes).expect("decode")
    else {
        panic!("fixture is not a MsgBlock");
    };
    let inner = decompose_blockfetch_block(&payload).expect("tag-24 unwrap");
    let env = ade_codec::cbor::envelope::decode_block_envelope(inner).expect("envelope");
    // The captured historical block is Allegra (era index 3 in BOTH the
    // wire and ade's storage scheme — proving the two indices coincide).
    assert_eq!(
        env.era,
        CardanoEra::Allegra,
        "captured era index must map through ade's storage scheme unchanged"
    );
}
