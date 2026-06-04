// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2N ChainSync mini-protocol message codec (BLUE).
//
// Wire shape:
//   chainSyncMessage =
//       [0]                         ; MsgRequestNext
//     / [1]                         ; MsgAwaitReply
//     / [2, header, tip]            ; MsgRollForward
//     / [3, point, tip]             ; MsgRollBackward
//     / [4, [point*]]               ; MsgFindIntersect
//     / [5, point, tip]             ; MsgIntersectFound
//     / [6, tip]                    ; MsgIntersectNotFound
//     / [7]                         ; MsgDone
//
//   point = [] | [slot, hash32]
//   tip   = [point, blockNo]
//   header = bytes                  ; opaque era-specific header bytes
//
// The header body is carried as opaque bytes — block parsing lives in
// ade_codec's era decoders; chain-sync only needs to preserve the
// wire-bytes for forwarding to the ledger.

use ade_types::{CardanoEra, Hash32, SlotNo};

use crate::codec::error::{CodecError, ProtocolKind};
use crate::codec::primitives::{
    decode_array_head_two_form, decode_array_header, decode_bytes, decode_u64, encode_array_header,
    encode_bytes, encode_u64, require_consumed, try_consume_break, ArrayHead,
};

const PROTOCOL: ProtocolKind = ProtocolKind::ChainSync;

/// Either the genesis pseudo-point or a slot-hash pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Point {
    Origin,
    Block { slot: SlotNo, hash: Hash32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tip {
    pub point: Point,
    pub block_no: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainSyncMessage {
    RequestNext,
    AwaitReply,
    RollForward { header: Vec<u8>, tip: Tip },
    RollBackward { point: Point, tip: Tip },
    FindIntersect { points: Vec<Point> },
    IntersectFound { point: Point, tip: Tip },
    IntersectNotFound { tip: Tip },
    Done,
}

// ---------------------------------------------------------------------------
// Encode
// ---------------------------------------------------------------------------

fn encode_point(buf: &mut Vec<u8>, p: &Point) {
    match p {
        Point::Origin => {
            encode_array_header(buf, 0);
        }
        Point::Block { slot, hash } => {
            encode_array_header(buf, 2);
            encode_u64(buf, slot.0);
            encode_bytes(buf, &hash.0);
        }
    }
}

fn encode_tip(buf: &mut Vec<u8>, t: &Tip) {
    encode_array_header(buf, 2);
    encode_point(buf, &t.point);
    encode_u64(buf, t.block_no);
}

pub fn encode_chain_sync_message(msg: &ChainSyncMessage) -> Vec<u8> {
    let mut buf = Vec::new();
    match msg {
        ChainSyncMessage::RequestNext => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 0);
        }
        ChainSyncMessage::AwaitReply => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 1);
        }
        ChainSyncMessage::RollForward { header, tip } => {
            // N2N RollForward carries the era-specific header wrapped as
            //   [era_idx, tag24(bytes(header_cbor))]   (Shelley-based eras)
            // where era_idx is the cardano-node CONSENSUS era index
            // (Byron=0, Shelley=1, ... Conway=6) — NOT the EBB-aware
            // BlockFetch/storage discriminant (Conway=7). Verified against
            // the real preprod Conway capture
            // (corpus/network/n2n/chain_sync/preprod_conway_rollforward_*).
            // `header` holds the ENTIRE wrapped item (starting with the
            // array(2) header) — the serve path composes it via
            // `compose_rollforward_header` (the CN-WIRE-08 tag-24
            // authority). The codec carries it verbatim for byte-identical
            // round-trip; the decoder slices the same range out.
            encode_array_header(&mut buf, 3);
            encode_u64(&mut buf, 2);
            buf.extend_from_slice(header);
            encode_tip(&mut buf, tip);
        }
        ChainSyncMessage::RollBackward { point, tip } => {
            encode_array_header(&mut buf, 3);
            encode_u64(&mut buf, 3);
            encode_point(&mut buf, point);
            encode_tip(&mut buf, tip);
        }
        ChainSyncMessage::FindIntersect { points } => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 4);
            encode_array_header(&mut buf, points.len() as u64);
            for p in points {
                encode_point(&mut buf, p);
            }
        }
        ChainSyncMessage::IntersectFound { point, tip } => {
            encode_array_header(&mut buf, 3);
            encode_u64(&mut buf, 5);
            encode_point(&mut buf, point);
            encode_tip(&mut buf, tip);
        }
        ChainSyncMessage::IntersectNotFound { tip } => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 6);
            encode_tip(&mut buf, tip);
        }
        ChainSyncMessage::Done => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 7);
        }
    }
    buf
}

// ---------------------------------------------------------------------------
// Decode
// ---------------------------------------------------------------------------

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

fn decode_tip(data: &[u8], offset: &mut usize) -> Result<Tip, CodecError> {
    let n = decode_array_header(PROTOCOL, data, offset)?;
    if n != 2 {
        return Err(CodecError::InvalidProtocolMessage {
            protocol: PROTOCOL,
            reason: "tip array must be 2 elements",
        });
    }
    let point = decode_point(data, offset)?;
    let block_no = decode_u64(PROTOCOL, data, offset)?;
    Ok(Tip { point, block_no })
}

/// Decode the `MsgFindIntersect` points list. A real cardano-node encodes
/// this list as a CBOR INDEFINITE-length array (`9f … ff`); Ade encodes it
/// definite-length. Accept BOTH canonical forms (`CN-WIRE-11`) — scoped to
/// THIS list ONLY; `decode_array_header` stays definite-only everywhere
/// else. Each element is the existing closed [`decode_point`]; the
/// indefinite form requires the `0xff` break. No catch-all, no other shape.
fn decode_find_intersect_points(
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<Point>, CodecError> {
    match decode_array_head_two_form(PROTOCOL, data, offset)? {
        ArrayHead::Definite(n) => {
            let mut points = Vec::with_capacity((n as usize).min(data.len()));
            for _ in 0..n {
                points.push(decode_point(data, offset)?);
            }
            Ok(points)
        }
        ArrayHead::Indefinite => {
            let mut points = Vec::new();
            loop {
                if try_consume_break(data, offset) {
                    break;
                }
                if *offset >= data.len() {
                    return Err(CodecError::InvalidProtocolMessage {
                        protocol: PROTOCOL,
                        reason: "indefinite FindIntersect points list missing break",
                    });
                }
                points.push(decode_point(data, offset)?);
            }
            Ok(points)
        }
    }
}

pub fn decode_chain_sync_message(bytes: &[u8]) -> Result<ChainSyncMessage, CodecError> {
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
        (0, 1) => ChainSyncMessage::RequestNext,
        (1, 1) => ChainSyncMessage::AwaitReply,
        (2, 3) => {
            // N2N RollForward header is wrapped per the cardano-node
            // hard-fork-combinator era discriminator. Shape varies by
            // era (e.g. Byron-EBB wraps differently than Shelley+);
            // we treat it as opaque CBOR pass-through and consume one
            // whole CBOR item, capturing its bytes verbatim for
            // byte-identical round-trip.
            let start = offset;
            ade_codec::cbor_primitives::skip_item(bytes, &mut offset)
                .map_err(|e| CodecError::MalformedCbor { protocol: PROTOCOL, source: e })?;
            let header = bytes[start..offset].to_vec();
            let tip = decode_tip(bytes, &mut offset)?;
            ChainSyncMessage::RollForward { header, tip }
        }
        (3, 3) => {
            let point = decode_point(bytes, &mut offset)?;
            let tip = decode_tip(bytes, &mut offset)?;
            ChainSyncMessage::RollBackward { point, tip }
        }
        (4, 2) => {
            let points = decode_find_intersect_points(bytes, &mut offset)?;
            ChainSyncMessage::FindIntersect { points }
        }
        (5, 3) => {
            let point = decode_point(bytes, &mut offset)?;
            let tip = decode_tip(bytes, &mut offset)?;
            ChainSyncMessage::IntersectFound { point, tip }
        }
        (6, 2) => {
            let tip = decode_tip(bytes, &mut offset)?;
            ChainSyncMessage::IntersectNotFound { tip }
        }
        (7, 1) => ChainSyncMessage::Done,
        (other, _) => return Err(CodecError::UnknownTag { protocol: PROTOCOL, tag: other }),
    };
    require_consumed(PROTOCOL, bytes, offset)?;
    Ok(msg)
}

// ---------------------------------------------------------------------
// Per-protocol tag-24 composition (CN-WIRE-08)
// ---------------------------------------------------------------------
//
// ChainSync's RollForward header wrap is `[era_idx, tag24(header_cbor)]`
// — the era index sits OUTSIDE the tag-24, and uses the cardano-node
// CONSENSUS era index, distinct from the BlockFetch (EBB-aware) scheme.
// This module owns that protocol composition; the tag-24 byte
// wrap/unwrap is the single `ade_codec` authority.

/// Map an ade storage `CardanoEra` to the cardano-node N2N ChainSync
/// header era index: the CONSENSUS 0-based index (Byron=0, Shelley=1,
/// Allegra=2, Mary=3, Alonzo=4, Babbage=5, Conway=6) = ade's storage
/// discriminant MINUS ONE (both Byron variants collapse to 0).
///
/// This is deliberately NOT the BlockFetch era index, which is the
/// EBB-aware storage discriminant (Conway=7). The two N2N surfaces use
/// different era-index schemes — verified against the real preprod
/// captures.
pub fn chain_sync_wire_era_index(era: CardanoEra) -> u8 {
    era.as_u8().saturating_sub(1)
}

/// Compose a ChainSync RollForward header payload from the bare
/// era-specific `header_cbor` (the `[header_body, kes_signature]` array
/// projected by `accepted_block_header_bytes`):
/// `[era_idx, tag24(bytes(header_cbor))]`, where `era_idx` is the
/// consensus era index. Single tag-24 authority — delegates the wrap to
/// `ade_codec::wrap_tag24` (`CN-WIRE-08`). The serve path calls this so
/// no bare header reaches a peer.
pub fn compose_rollforward_header(era: CardanoEra, header_cbor: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(header_cbor.len() + 8);
    encode_array_header(&mut buf, 2);
    encode_u64(&mut buf, chain_sync_wire_era_index(era) as u64);
    buf.extend_from_slice(&ade_codec::wrap_tag24(header_cbor));
    buf
}

/// Inverse of [`compose_rollforward_header`]: strip the
/// `[era_idx, tag24(...)]` wrap, returning the consensus era index and a
/// zero-copy borrow of the inner `header_cbor`. Fails closed on a
/// non-array(2) outer, a non-tag-24 inner (e.g. a Byron era-0 header,
/// which is not tag-24-wrapped), or trailing bytes.
pub fn decompose_rollforward_header(payload: &[u8]) -> Result<(u8, &[u8]), CodecError> {
    let mut off = 0usize;
    let arr = decode_array_header(PROTOCOL, payload, &mut off)?;
    if arr != 2 {
        return Err(CodecError::InvalidProtocolMessage {
            protocol: PROTOCOL,
            reason: "rollforward header must be a 2-element [era_idx, tag24] array",
        });
    }
    let era_idx = decode_u64(PROTOCOL, payload, &mut off)?;
    let inner = ade_codec::unwrap_tag24(&payload[off..]).map_err(|_| {
        CodecError::InvalidProtocolMessage {
            protocol: PROTOCOL,
            reason: "rollforward header inner is not tag24(bytes(..))",
        }
    })?;
    Ok((era_idx as u8, inner))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn sample_tip() -> Tip {
        Tip {
            point: Point::Block { slot: SlotNo(1234), hash: Hash32([0xAA; 32]) },
            block_no: 5678,
        }
    }

    /// Build a synthetic wrapped header matching the real N2N wire
    /// format: `[era_idx, tag(24, bytes(inner))]` — the first element is
    /// the consensus era index (verified against the captured Conway
    /// frame), NOT a serialisationInfo word.
    fn wrapped_header(era_idx: u64, inner: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_array_header(&mut buf, 2);
        encode_u64(&mut buf, era_idx);
        buf.push(0xd8);
        buf.push(0x18);
        encode_bytes(&mut buf, inner);
        buf
    }

    fn sample_messages() -> Vec<ChainSyncMessage> {
        vec![
            ChainSyncMessage::RequestNext,
            ChainSyncMessage::AwaitReply,
            ChainSyncMessage::RollForward {
                header: wrapped_header(1, &[0x01, 0x02, 0x03, 0x04]),
                tip: sample_tip(),
            },
            ChainSyncMessage::RollBackward {
                point: Point::Origin,
                tip: sample_tip(),
            },
            ChainSyncMessage::FindIntersect {
                points: vec![
                    Point::Origin,
                    Point::Block { slot: SlotNo(99), hash: Hash32([0xBB; 32]) },
                ],
            },
            ChainSyncMessage::IntersectFound {
                point: Point::Block { slot: SlotNo(42), hash: Hash32([0xCC; 32]) },
                tip: sample_tip(),
            },
            ChainSyncMessage::IntersectNotFound { tip: sample_tip() },
            ChainSyncMessage::Done,
        ]
    }

    #[test]
    fn roundtrip_every_variant() {
        for msg in sample_messages() {
            let bytes = encode_chain_sync_message(&msg);
            let decoded = decode_chain_sync_message(&bytes).expect("decode");
            assert_eq!(decoded, msg);
            assert_eq!(encode_chain_sync_message(&decoded), bytes);
        }
    }

    #[test]
    fn decode_rejects_unknown_tag() {
        // [99] — outer tag 99 is not in {0..7}
        let bytes = vec![0x81, 0x18, 0x63];
        match decode_chain_sync_message(&bytes) {
            Err(CodecError::UnknownTag { protocol: ProtocolKind::ChainSync, tag: 99 }) => {}
            other => panic!("expected UnknownTag, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_truncated_input() {
        let full = encode_chain_sync_message(&ChainSyncMessage::RollForward {
            header: wrapped_header(1, &[0x01, 0x02, 0x03, 0x04]),
            tip: sample_tip(),
        });
        for n in 0..full.len() {
            let slice = &full[..n];
            let err = decode_chain_sync_message(slice).expect_err("must reject truncated");
            match err {
                CodecError::Truncated { .. }
                | CodecError::MalformedCbor { .. }
                | CodecError::InvalidProtocolMessage { .. } => {}
                other => panic!("expected truncation-class error, got {other:?}"),
            }
        }
    }

    #[test]
    fn chain_sync_wire_era_index_is_consensus_index() {
        // Consensus index = ade storage discriminant minus one; both
        // Byron variants collapse to 0. Conway (storage 7) → 6.
        assert_eq!(chain_sync_wire_era_index(CardanoEra::ByronEbb), 0);
        assert_eq!(chain_sync_wire_era_index(CardanoEra::ByronRegular), 0);
        assert_eq!(chain_sync_wire_era_index(CardanoEra::Shelley), 1);
        assert_eq!(chain_sync_wire_era_index(CardanoEra::Allegra), 2);
        assert_eq!(chain_sync_wire_era_index(CardanoEra::Mary), 3);
        assert_eq!(chain_sync_wire_era_index(CardanoEra::Alonzo), 4);
        assert_eq!(chain_sync_wire_era_index(CardanoEra::Babbage), 5);
        assert_eq!(chain_sync_wire_era_index(CardanoEra::Conway), 6);
    }

    #[test]
    fn compose_decompose_rollforward_header_round_trips() {
        let inner = [0x82u8, 0x8a, 0x01, 0x02, 0x03];
        let wire = compose_rollforward_header(CardanoEra::Conway, &inner);
        // Shape: [6, tag24(bytes(inner))]
        assert_eq!(wire[0], 0x82, "array(2)");
        assert_eq!(wire[1], 0x06, "Conway consensus era index");
        assert_eq!(&wire[2..4], &[0xd8, 0x18], "tag(24)");
        let (era_idx, back) = decompose_rollforward_header(&wire).expect("decompose");
        assert_eq!(era_idx, 6);
        assert_eq!(back, &inner[..]);
    }

    #[test]
    fn decompose_rollforward_header_rejects_non_tag24_inner() {
        // A Byron-style `[0, byron_header]` (no tag-24) must fail closed.
        let mut buf = Vec::new();
        encode_array_header(&mut buf, 2);
        encode_u64(&mut buf, 0);
        encode_array_header(&mut buf, 1); // bare array, not tag-24 bytes
        encode_u64(&mut buf, 7);
        assert!(matches!(
            decompose_rollforward_header(&buf),
            Err(CodecError::InvalidProtocolMessage { .. })
        ));
    }
}
