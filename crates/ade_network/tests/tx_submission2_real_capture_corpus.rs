// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// Real-capture corpus test for N2N TxSubmission2 (closes the DC-PROTO-02
// tx-submission2 gap). Fixtures were captured against the docker preprod
// cardano-node 11.0.1 via the SERVER-SIDE harness `ade_tx_submission2_server_capture`
// (option B: Ade listens, the node dials Ade as a localRoots peer and, as the
// tx-submission2 CLIENT/provider, sends the rich messages — MsgInit, then its
// real mempool MsgReplyTxIds / MsgReplyTxs — while Ade plays the SERVER/consumer).
//
// Each fixture is a full mux frame (8-byte header + CBOR payload). We strip the
// mux header, decode the tx-submission2 message with our codec, re-encode it,
// and assert byte-identical round-trip — proving the codec is on the same wire
// grammar as the cardano-node Haskell reference for the node-originated rich
// messages. (The Request* messages are Ade-originated in this direction and are
// live-validated by the node accepting them; their wire form is also covered by
// the synthetic roundtrip_every_variant codec test.)

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use ade_network::codec::tx_submission::{
    decode_tx_submission_message, encode_tx_submission_message, TxSubmission2Message,
};

const MUX_HEADER_LEN: usize = 8;

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .parent()
        .expect("repo root")
        .join("corpus/network/n2n/tx_submission2")
}

/// Round-trip a node-captured tx-submission2 frame: strip the mux header,
/// decode, re-encode, assert byte-identical.
fn round_trip(path: &Path) -> TxSubmission2Message {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    assert!(
        bytes.len() > MUX_HEADER_LEN,
        "{path:?}: capture shorter than mux header"
    );
    let payload = &bytes[MUX_HEADER_LEN..];
    let decoded = decode_tx_submission_message(payload)
        .unwrap_or_else(|e| panic!("decode {path:?}: {e:?}"));
    let re = encode_tx_submission_message(&decoded);
    assert_eq!(
        re, payload,
        "{path:?}: encode(decode(bytes)) != bytes — codec is not on the cardano-node wire grammar"
    );
    decoded
}

/// All node-captured tx-submission2 frames in the corpus: files named
/// `*_txsub_*_recv.cbor` (the server harness's node-originated captures). The
/// older `local_preprod_msg_00_send_init.cbor` (Ade's own Init, payload-only,
/// no mux header) and `*_handshake_propose.cbor` are intentionally excluded.
fn captured_frames() -> Vec<PathBuf> {
    let dir = corpus_dir();
    let mut frames: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read corpus dir {dir:?}: {e}"))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains("_txsub_") && n.ends_with("_recv.cbor"))
                .unwrap_or(false)
        })
        .collect();
    frames.sort();
    frames
}

#[test]
fn real_capture_round_trips_byte_identical() {
    let frames = captured_frames();
    assert!(
        !frames.is_empty(),
        "expected real tx-submission2 captures (run ade_tx_submission2_server_capture)"
    );

    let mut saw_init = false;
    let mut saw_reply_txids_nonempty = false;
    let mut saw_reply_txs = false;

    for path in &frames {
        match round_trip(path) {
            TxSubmission2Message::Init => saw_init = true,
            TxSubmission2Message::ReplyTxIds(entries) => {
                if !entries.is_empty() {
                    saw_reply_txids_nonempty = true;
                }
            }
            TxSubmission2Message::ReplyTxs(_) => saw_reply_txs = true,
            _ => {}
        }
    }

    // The node, as the tx-submission2 provider, originates MsgInit and offers
    // its mempool via MsgReplyTxIds (real, era-tagged tx ids in an indefinite
    // array) and MsgReplyTxs (real era-wrapped tx bodies). All three are from
    // the live full exchange (Init -> RequestTxIds -> ReplyTxIds -> RequestTxs
    // -> ReplyTxs) against the docker public-preprod node, and all re-encode
    // byte-identically above.
    assert!(saw_init, "expected the node's MsgInit among the captures");
    assert!(
        saw_reply_txids_nonempty,
        "expected a non-empty MsgReplyTxIds (real mempool tx ids) capture"
    );
    assert!(
        saw_reply_txs,
        "expected a MsgReplyTxs (real era-wrapped tx body) capture",
    );
}

#[test]
fn reply_txids_entries_are_real_32_byte_txids() {
    // Every captured ReplyTxIds entry must carry a 32-byte tx id (Blake2b-256)
    // and a non-zero advertised size — the real mempool shape.
    for path in captured_frames() {
        if let TxSubmission2Message::ReplyTxIds(entries) = round_trip(&path) {
            for e in &entries {
                assert_eq!(e.tx_id.id.as_bytes().len(), 32, "{path:?}: tx id not 32 bytes");
                assert!(e.size > 0, "{path:?}: advertised tx size is zero");
            }
        }
    }
}
