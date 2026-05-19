// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2C LocalChainSync mini-protocol message codec (BLUE).
//
// Mirrors N2N ChainSync's wire shape but carries a *block* body in
// RollForward rather than just a header — N2C consumers expect the
// full block payload because no separate BlockFetch protocol exists
// on the N2C surface. The closed enum is distinct from
// `ChainSyncMessage` so the type system rejects mixing N2N/N2C
// chain-sync messages across session boundaries.
//
// Wire shape:
//   localChainSyncMessage =
//       [0]                         ; MsgRequestNext
//     / [1]                         ; MsgAwaitReply
//     / [2, blockBytes, tip]        ; MsgRollForward
//     / [3, point, tip]             ; MsgRollBackward
//     / [4, [point*]]               ; MsgFindIntersect
//     / [5, point, tip]             ; MsgIntersectFound
//     / [6, tip]                    ; MsgIntersectNotFound
//     / [7]                         ; MsgDone

use ade_types::{Hash32, SlotNo};

use crate::codec::error::{CodecError, ProtocolKind};
use crate::codec::primitives::{
    decode_array_header, decode_bytes, decode_u64, encode_array_header, encode_bytes, encode_u64,
    require_consumed,
};

const PROTOCOL: ProtocolKind = ProtocolKind::LocalChainSync;

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
pub enum LocalChainSyncMessage {
    RequestNext,
    AwaitReply,
    RollForward { block: Vec<u8>, tip: Tip },
    RollBackward { point: Point, tip: Tip },
    FindIntersect { points: Vec<Point> },
    IntersectFound { point: Point, tip: Tip },
    IntersectNotFound { tip: Tip },
    Done,
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

fn encode_tip(buf: &mut Vec<u8>, t: &Tip) {
    encode_array_header(buf, 2);
    encode_point(buf, &t.point);
    encode_u64(buf, t.block_no);
}

pub fn encode_local_chain_sync_message(msg: &LocalChainSyncMessage) -> Vec<u8> {
    let mut buf = Vec::new();
    match msg {
        LocalChainSyncMessage::RequestNext => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 0);
        }
        LocalChainSyncMessage::AwaitReply => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 1);
        }
        LocalChainSyncMessage::RollForward { block, tip } => {
            encode_array_header(&mut buf, 3);
            encode_u64(&mut buf, 2);
            encode_bytes(&mut buf, block);
            encode_tip(&mut buf, tip);
        }
        LocalChainSyncMessage::RollBackward { point, tip } => {
            encode_array_header(&mut buf, 3);
            encode_u64(&mut buf, 3);
            encode_point(&mut buf, point);
            encode_tip(&mut buf, tip);
        }
        LocalChainSyncMessage::FindIntersect { points } => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 4);
            encode_array_header(&mut buf, points.len() as u64);
            for p in points {
                encode_point(&mut buf, p);
            }
        }
        LocalChainSyncMessage::IntersectFound { point, tip } => {
            encode_array_header(&mut buf, 3);
            encode_u64(&mut buf, 5);
            encode_point(&mut buf, point);
            encode_tip(&mut buf, tip);
        }
        LocalChainSyncMessage::IntersectNotFound { tip } => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 6);
            encode_tip(&mut buf, tip);
        }
        LocalChainSyncMessage::Done => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 7);
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

pub fn decode_local_chain_sync_message(bytes: &[u8]) -> Result<LocalChainSyncMessage, CodecError> {
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
        (0, 1) => LocalChainSyncMessage::RequestNext,
        (1, 1) => LocalChainSyncMessage::AwaitReply,
        (2, 3) => {
            let block = decode_bytes(PROTOCOL, bytes, &mut offset)?;
            let tip = decode_tip(bytes, &mut offset)?;
            LocalChainSyncMessage::RollForward { block, tip }
        }
        (3, 3) => {
            let point = decode_point(bytes, &mut offset)?;
            let tip = decode_tip(bytes, &mut offset)?;
            LocalChainSyncMessage::RollBackward { point, tip }
        }
        (4, 2) => {
            let n = decode_array_header(PROTOCOL, bytes, &mut offset)?;
            let mut points = Vec::with_capacity((n as usize).min(bytes.len()));
            for _ in 0..n {
                points.push(decode_point(bytes, &mut offset)?);
            }
            LocalChainSyncMessage::FindIntersect { points }
        }
        (5, 3) => {
            let point = decode_point(bytes, &mut offset)?;
            let tip = decode_tip(bytes, &mut offset)?;
            LocalChainSyncMessage::IntersectFound { point, tip }
        }
        (6, 2) => {
            let tip = decode_tip(bytes, &mut offset)?;
            LocalChainSyncMessage::IntersectNotFound { tip }
        }
        (7, 1) => LocalChainSyncMessage::Done,
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

    fn sample_tip() -> Tip {
        Tip {
            point: Point::Block { slot: SlotNo(1234), hash: Hash32([0xAA; 32]) },
            block_no: 5678,
        }
    }

    fn sample_messages() -> Vec<LocalChainSyncMessage> {
        vec![
            LocalChainSyncMessage::RequestNext,
            LocalChainSyncMessage::AwaitReply,
            LocalChainSyncMessage::RollForward {
                block: vec![0xDE, 0xAD, 0xBE, 0xEF],
                tip: sample_tip(),
            },
            LocalChainSyncMessage::RollBackward {
                point: Point::Origin,
                tip: sample_tip(),
            },
            LocalChainSyncMessage::FindIntersect {
                points: vec![
                    Point::Origin,
                    Point::Block { slot: SlotNo(99), hash: Hash32([0xBB; 32]) },
                ],
            },
            LocalChainSyncMessage::IntersectFound {
                point: Point::Block { slot: SlotNo(42), hash: Hash32([0xCC; 32]) },
                tip: sample_tip(),
            },
            LocalChainSyncMessage::IntersectNotFound { tip: sample_tip() },
            LocalChainSyncMessage::Done,
        ]
    }

    #[test]
    fn roundtrip_every_variant() {
        for msg in sample_messages() {
            let bytes = encode_local_chain_sync_message(&msg);
            let decoded = decode_local_chain_sync_message(&bytes).expect("decode");
            assert_eq!(decoded, msg);
            assert_eq!(encode_local_chain_sync_message(&decoded), bytes);
        }
    }

    #[test]
    fn decode_rejects_unknown_tag() {
        let bytes = vec![0x81, 0x18, 0x63];
        match decode_local_chain_sync_message(&bytes) {
            Err(CodecError::UnknownTag { protocol: ProtocolKind::LocalChainSync, tag: 99 }) => {}
            other => panic!("expected UnknownTag, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_truncated_input() {
        let full = encode_local_chain_sync_message(&LocalChainSyncMessage::RollForward {
            block: vec![0xDE, 0xAD, 0xBE, 0xEF],
            tip: sample_tip(),
        });
        for n in 0..full.len() {
            let slice = &full[..n];
            let err = decode_local_chain_sync_message(slice).expect_err("must reject truncated");
            match err {
                CodecError::Truncated { .. }
                | CodecError::MalformedCbor { .. }
                | CodecError::InvalidProtocolMessage { .. } => {}
                other => panic!("expected truncation-class error, got {other:?}"),
            }
        }
    }
}
