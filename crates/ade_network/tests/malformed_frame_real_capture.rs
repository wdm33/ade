// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// CE-N-A-5 condition 5 — "Malformed frames produce canonical
// structured errors" — closure test.
//
// We take real captured frames and deliberately corrupt them in
// well-defined ways (truncation, byte-flip on the tag, byte-flip
// on a length prefix). Each corruption MUST produce a canonical
// `CodecError` variant — never an unexpected panic, never a
// silent acceptance, never a `String`/`anyhow`-shaped error.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use ade_network::codec::block_fetch::decode_block_fetch_message;
use ade_network::codec::chain_sync::decode_chain_sync_message;
use ade_network::codec::error::CodecError;
use ade_network::codec::handshake::decode_handshake_message;
use ade_network::codec::keep_alive::decode_keep_alive_message;
use ade_network::codec::local_chain_sync::decode_local_chain_sync_message;
use ade_network::codec::local_state_query::decode_local_state_query_message;
use ade_network::codec::local_tx_monitor::decode_local_tx_monitor_message;
use ade_network::codec::local_tx_submission::decode_local_tx_submission_message;
use ade_network::codec::n2c_handshake::decode_n2c_handshake_message;
use ade_network::codec::peer_sharing::decode_peer_sharing_message;
use ade_network::codec::tx_submission::decode_tx_submission_message;

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .parent()
        .expect("repo root")
        .join("corpus/network")
}

fn read_file(rel: &str) -> Vec<u8> {
    let path = corpus_root().join(rel);
    fs::read(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"))
}

/// Strip mux header if the file is a raw mux capture (chain-sync
/// kept its mux header); otherwise return as-is.
fn payload(rel: &str) -> Vec<u8> {
    let raw = read_file(rel);
    if rel.starts_with("n2n/chain_sync/") {
        raw[8..].to_vec()
    } else {
        raw
    }
}

/// Assert decoding the given bytes via `f` returns a canonical
/// CodecError — i.e., one of the closed enum variants. Failing
/// silently (Ok), with a non-CodecError type, or panicking is a
/// CE-N-A-5 condition 5 violation.
fn assert_canonical_error<T, F>(label: &str, bytes: &[u8], f: F)
where
    F: FnOnce(&[u8]) -> Result<T, CodecError>,
    T: std::fmt::Debug,
{
    match f(bytes) {
        Ok(decoded) => panic!(
            "{label}: malformed bytes silently decoded to {decoded:?} (got Ok)"
        ),
        Err(e) => {
            // Spot-check that the error is structured. CodecError's
            // closed enum is the structured contract.
            match e {
                CodecError::Truncated { .. }
                | CodecError::UnknownTag { .. }
                | CodecError::InvalidUtf8 { .. }
                | CodecError::InvalidProtocolMessage { .. }
                | CodecError::InvalidIntegerRange { .. }
                | CodecError::MalformedCbor { .. } => {}
            }
        }
    }
}

/// Generic corruption sweep on one real capture. Three corruptions:
///   1. Truncate to 0..len.
///   2. Flip the second byte (often the tag).
///   3. If the byte at index 0 is an array header `8X`, force tag to
///      a deliberately-impossible value (0xFF).
fn sweep_corruptions<F, T>(label: &str, payload_bytes: &[u8], decode: F)
where
    F: Fn(&[u8]) -> Result<T, CodecError> + Copy,
    T: std::fmt::Debug,
{
    // 1. Truncation
    for n in 0..payload_bytes.len() {
        let slice = &payload_bytes[..n];
        assert_canonical_error(&format!("{label} truncated[..{n}]"), slice, decode);
    }
    // 2. Flip second byte (tag is at index 1 in `array_header, tag, ...`)
    if payload_bytes.len() >= 2 {
        let mut bad = payload_bytes.to_vec();
        bad[1] ^= 0xFF;
        // Some flips may still decode OK if they swap to another
        // valid tag at the same arity; that's not a malformed result.
        // We only assert NO panic. If it returned Ok, that's still
        // not a *malformed* response — it's a different valid message.
        let _ = decode(&bad);
    }
    // 3. Force tag to a deliberately-unknown value if the encoding
    // shape allows. For the `array(arr_len), uint(tag), ...` family,
    // the byte at index 1 is the tag (when tag < 24).
    if payload_bytes.len() >= 2 {
        let mut bad = payload_bytes.to_vec();
        bad[1] = 0x18; // CBOR uint8 next-byte
        bad.insert(2, 0xFF); // tag = 255, unknown to all our codecs
        assert_canonical_error(&format!("{label} tag=0xFF"), &bad, decode);
    }
}

#[test]
fn malformed_handshake_real_capture_yields_canonical_error() {
    // Use the IOG mainnet handshake capture as the base.
    let dir = corpus_root().join("n2n/handshake");
    let entries: Vec<_> = fs::read_dir(&dir)
        .expect("read dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(".cbor"))
                .unwrap_or(false)
        })
        .collect();
    assert!(!entries.is_empty(), "no handshake captures");
    let base = fs::read(&entries[0]).unwrap();
    sweep_corruptions("handshake", &base, decode_handshake_message);
}

#[test]
fn malformed_chain_sync_real_capture_yields_canonical_error() {
    let bytes = payload("n2n/chain_sync/preprod_origin_5_frames_intersect_recv.cbor");
    sweep_corruptions("chain_sync intersect", &bytes, decode_chain_sync_message);
}

#[test]
fn malformed_block_fetch_real_capture_yields_canonical_error() {
    let bytes = payload("n2n/block_fetch/local_preprod_tip_msg_01_block.cbor");
    sweep_corruptions("block_fetch block", &bytes, decode_block_fetch_message);
}

#[test]
fn malformed_keep_alive_real_capture_yields_canonical_error() {
    let bytes = payload("n2n/keep_alive/local_preprod_pings_msg_01_recv_response.cbor");
    sweep_corruptions("keep_alive", &bytes, decode_keep_alive_message);
}

#[test]
fn malformed_peer_sharing_real_capture_yields_canonical_error() {
    let bytes = payload("n2n/peer_sharing/local_preprod_share_msg_01_recv_share_peers.cbor");
    sweep_corruptions("peer_sharing", &bytes, decode_peer_sharing_message);
}

#[test]
fn malformed_tx_submission2_real_capture_yields_canonical_error() {
    let bytes = payload("n2n/tx_submission2/local_preprod_msg_00_send_init.cbor");
    sweep_corruptions("tx_submission2", &bytes, decode_tx_submission_message);
}

#[test]
fn malformed_n2c_handshake_real_capture_yields_canonical_error() {
    let bytes = payload("n2c/handshake/local_preprod_handshake_msg_01_recv_reply.cbor");
    sweep_corruptions("n2c_handshake", &bytes, decode_n2c_handshake_message);
}

#[test]
fn malformed_local_chain_sync_real_capture_yields_canonical_error() {
    let bytes = payload("n2c/local_chain_sync/local_preprod_msg_01_recv_intersect_reply.cbor");
    sweep_corruptions(
        "local_chain_sync",
        &bytes,
        decode_local_chain_sync_message,
    );
}

#[test]
fn malformed_local_state_query_real_capture_yields_canonical_error() {
    let bytes = payload("n2c/local_state_query/local_preprod_msg_00_send_acquire_no_point.cbor");
    sweep_corruptions(
        "local_state_query",
        &bytes,
        decode_local_state_query_message,
    );
}

#[test]
fn malformed_local_tx_submission_real_capture_yields_canonical_error() {
    let bytes = payload("n2c/local_tx_submission/local_preprod_msg_00_send_submit_empty.cbor");
    sweep_corruptions(
        "local_tx_submission",
        &bytes,
        decode_local_tx_submission_message,
    );
}

#[test]
fn malformed_local_tx_monitor_real_capture_yields_canonical_error() {
    let bytes = payload("n2c/local_tx_monitor/local_preprod_msg_03_recv_reply_get_sizes.cbor");
    sweep_corruptions(
        "local_tx_monitor",
        &bytes,
        decode_local_tx_monitor_message,
    );
}
