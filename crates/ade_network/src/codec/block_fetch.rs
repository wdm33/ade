// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2N BlockFetch mini-protocol message codec (BLUE).
//
// Wire shape (matches Ouroboros.Network.Protocol.BlockFetch.Codec):
//   blockFetchMessage =
//       [0, point, point]          ; MsgRequestRange — FLAT 3-element array
//     / [1]                        ; MsgClientDone
//     / [2]                        ; MsgStartBatch
//     / [3]                        ; MsgNoBlocks
//     / [4, block_bytes]           ; MsgBlock
//     / [5]                        ; MsgBatchDone
//
//   point = [] | [slot, hash32]    ; identical shape to chain-sync
//
// The MsgRequestRange (from, to) is a flat triple — there is no
// nested `range` wrapper on the wire. Internally we keep a Range
// struct for ergonomics, but the encoder emits the two points
// inline at the top level. Round-trip with cardano-node verified
// against a local node (DeserialiseFailure if 2-element form is
// emitted: "unexpected key (0, 2)").
//
// The block body is opaque bytes — era-aware parsing belongs to
// ade_codec, not the block-fetch protocol codec.

use ade_types::{Hash32, SlotNo};

use crate::codec::error::{CodecError, ProtocolKind};
use crate::codec::primitives::{
    decode_array_header, decode_bytes, decode_u64, encode_array_header, encode_bytes, encode_u64,
    require_consumed,
};

const PROTOCOL: ProtocolKind = ProtocolKind::BlockFetch;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Point {
    Origin,
    Block { slot: SlotNo, hash: Hash32 },
}

/// Inclusive (from, to) point range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    pub from: Point,
    pub to: Point,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockFetchMessage {
    RequestRange(Range),
    ClientDone,
    StartBatch,
    NoBlocks,
    Block { bytes: Vec<u8> },
    BatchDone,
}

fn encode_point(buf: &mut Vec<u8>, p: &Point) {
    match p {
        Point::Origin => encode_array_header(buf, 0),
        Point::Block { slot, hash } => {
            encode_array_header(buf, 2);
            encode_u64(buf, slot.0);
            encode_bytes(buf, &hash.0);
        }
    }
}

pub fn encode_block_fetch_message(msg: &BlockFetchMessage) -> Vec<u8> {
    let mut buf = Vec::new();
    match msg {
        BlockFetchMessage::RequestRange(r) => {
            // Wire format is FLAT: [0, from_point, to_point] — three
            // top-level elements. Real interop against cardano-node
            // 11.0.1 rejects a nested-range encoding with
            // DeserialiseFailure "unexpected key (0, 2)".
            encode_array_header(&mut buf, 3);
            encode_u64(&mut buf, 0);
            encode_point(&mut buf, &r.from);
            encode_point(&mut buf, &r.to);
        }
        BlockFetchMessage::ClientDone => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 1);
        }
        BlockFetchMessage::StartBatch => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 2);
        }
        BlockFetchMessage::NoBlocks => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 3);
        }
        BlockFetchMessage::Block { bytes } => {
            // The block body is wrapped per the cardano-node hard-fork-
            // combinator era discriminator: [serialisationInfo, tag24(bytes)]
            // or similar nested shape varying by era. We carry the FULL
            // wrapped CBOR item as opaque bytes for byte-identical
            // round-trip; the decoder slices the same range.
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 4);
            buf.extend_from_slice(bytes);
        }
        BlockFetchMessage::BatchDone => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 5);
        }
    }
    buf
}

fn decode_point(data: &[u8], offset: &mut usize) -> Result<Point, CodecError> {
    let arr_len = decode_array_header(PROTOCOL, data, offset)?;
    match arr_len {
        0 => Ok(Point::Origin),
        2 => {
            let slot = decode_u64(PROTOCOL, data, offset)?;
            let hash_bytes = decode_bytes(PROTOCOL, data, offset)?;
            if hash_bytes.len() != 32 {
                return Err(CodecError::InvalidProtocolMessage {
                    protocol: PROTOCOL,
                    reason: "point hash not 32 bytes",
                });
            }
            let mut h = [0u8; 32];
            h.copy_from_slice(&hash_bytes);
            Ok(Point::Block { slot: SlotNo(slot), hash: Hash32(h) })
        }
        _ => Err(CodecError::InvalidProtocolMessage {
            protocol: PROTOCOL,
            reason: "point array must be 0 or 2 elements",
        }),
    }
}

pub fn decode_block_fetch_message(bytes: &[u8]) -> Result<BlockFetchMessage, CodecError> {
    if bytes.is_empty() {
        return Err(CodecError::Truncated { needed: 1, got: 0 });
    }
    let mut offset = 0usize;
    let arr_len = decode_array_header(PROTOCOL, bytes, &mut offset)?;
    if arr_len < 1 {
        return Err(CodecError::InvalidProtocolMessage {
            protocol: PROTOCOL,
            reason: "empty outer array",
        });
    }
    let tag = decode_u64(PROTOCOL, bytes, &mut offset)?;
    let msg = match (tag, arr_len) {
        (0, 3) => {
            // Flat 3-element wire form: [0, from_point, to_point].
            let from = decode_point(bytes, &mut offset)?;
            let to = decode_point(bytes, &mut offset)?;
            BlockFetchMessage::RequestRange(Range { from, to })
        }
        (1, 1) => BlockFetchMessage::ClientDone,
        (2, 1) => BlockFetchMessage::StartBatch,
        (3, 1) => BlockFetchMessage::NoBlocks,
        (4, 2) => {
            // See encode comment: block body is era-discriminated wrapped
            // CBOR. Consume one whole item via skip_item, capture its
            // bytes verbatim for byte-identical round-trip.
            let start = offset;
            ade_codec::cbor_primitives::skip_item(bytes, &mut offset)
                .map_err(|e| CodecError::MalformedCbor { protocol: PROTOCOL, source: e })?;
            let body = bytes[start..offset].to_vec();
            BlockFetchMessage::Block { bytes: body }
        }
        (5, 1) => BlockFetchMessage::BatchDone,
        (other, _) => return Err(CodecError::UnknownTag { protocol: PROTOCOL, tag: other }),
    };
    require_consumed(PROTOCOL, bytes, offset)?;
    Ok(msg)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    /// Build a synthetic wrapped block matching the N2N wire shape:
    /// `[serialisationInfo, tag(24, bytes(inner))]`.
    fn wrapped_block(info_word: u64, inner: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_array_header(&mut buf, 2);
        encode_u64(&mut buf, info_word);
        buf.push(0xd8);
        buf.push(0x18);
        encode_bytes(&mut buf, inner);
        buf
    }

    fn sample_messages() -> Vec<BlockFetchMessage> {
        vec![
            BlockFetchMessage::RequestRange(Range {
                from: Point::Block { slot: SlotNo(100), hash: Hash32([0x11; 32]) },
                to: Point::Block { slot: SlotNo(200), hash: Hash32([0x22; 32]) },
            }),
            BlockFetchMessage::ClientDone,
            BlockFetchMessage::StartBatch,
            BlockFetchMessage::NoBlocks,
            BlockFetchMessage::Block { bytes: wrapped_block(1, &[0xDE, 0xAD, 0xBE, 0xEF]) },
            BlockFetchMessage::BatchDone,
        ]
    }

    #[test]
    fn roundtrip_every_variant() {
        for msg in sample_messages() {
            let bytes = encode_block_fetch_message(&msg);
            let decoded = decode_block_fetch_message(&bytes).expect("decode");
            assert_eq!(decoded, msg);
            assert_eq!(encode_block_fetch_message(&decoded), bytes);
        }
    }

    #[test]
    fn decode_rejects_unknown_tag() {
        let bytes = vec![0x81, 0x18, 0x63];
        match decode_block_fetch_message(&bytes) {
            Err(CodecError::UnknownTag { protocol: ProtocolKind::BlockFetch, tag: 99 }) => {}
            other => panic!("expected UnknownTag, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_truncated_input() {
        let full = encode_block_fetch_message(&BlockFetchMessage::RequestRange(Range {
            from: Point::Block { slot: SlotNo(100), hash: Hash32([0x11; 32]) },
            to: Point::Block { slot: SlotNo(200), hash: Hash32([0x22; 32]) },
        }));
        for n in 0..full.len() {
            let slice = &full[..n];
            let err = decode_block_fetch_message(slice).expect_err("must reject truncated");
            match err {
                CodecError::Truncated { .. }
                | CodecError::MalformedCbor { .. }
                | CodecError::InvalidProtocolMessage { .. } => {}
                other => panic!("expected truncation-class error, got {other:?}"),
            }
        }
    }
}
