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
    // Real cardano-node MsgBlock body is the era-discriminated
    // Hard-Fork-Combinator-wrapped block; our codec captures the
    // whole wrapped item verbatim via skip_item. The body must
    // therefore (a) be non-empty and (b) start with a major-type-6
    // CBOR tag (0xd8..=0xdb) for tag24 wrapping or major-type-4
    // array for inline era envelope — both shapes are valid HFC
    // wrappings depending on era.
    let dir = corpus_dir();
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
            assert!(!bytes.is_empty(), "MsgBlock body empty for {path:?}");
            let first = bytes[0];
            let major = first >> 5;
            // Major type 4 (array) for inline era envelope, or major
            // type 6 (tag) for tag24-wrapped serialised block. Both
            // are observed in cardano-node HFC encoders.
            assert!(
                major == 4 || major == 6,
                "MsgBlock body for {path:?} doesn't start with array or tag (first byte 0x{first:02x})"
            );
        }
    }
}
