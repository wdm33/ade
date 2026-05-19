// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2C LocalStateQuery mini-protocol message codec (BLUE).
//
// Per the locked decision (PHASE4-N-A_invariants.md §7 #3) the codec
// owns the *closed wire grammar* — envelope shape, agency, version
// gating, structured errors — but does NOT interpret query semantics.
// `QueryPayload(Vec<u8>)` and `ResultPayload(Vec<u8>)` carry opaque
// CBOR bytes. Ledger-specific interpretation belongs to cluster N-F.
//
// Wire shape:
//   localStateQueryMessage =
//       [0, [pointOpt]]            ; MsgAcquire (point or current)
//     / [1]                        ; MsgAcquired
//     / [2, failure(u8)]           ; MsgFailure
//     / [3, queryPayload]          ; MsgQuery
//     / [4, resultPayload]         ; MsgResult
//     / [5]                        ; MsgRelease
//     / [6, [pointOpt]]            ; MsgReAcquire
//     / [7]                        ; MsgDone
//
//   pointOpt = [] | [slot, hash32]
//
// `failure` is a small enumerated tag carried as a CBOR unsigned int:
//   0 = AcquireFailurePointTooOld
//   1 = AcquireFailurePointNotOnChain

use ade_types::{Hash32, SlotNo};

use crate::codec::error::{CodecError, ProtocolKind};
use crate::codec::primitives::{
    decode_array_header, decode_bytes, decode_u64, encode_array_header, encode_bytes, encode_u64,
    require_consumed,
};

const PROTOCOL: ProtocolKind = ProtocolKind::LocalStateQuery;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Point {
    Origin,
    Block { slot: SlotNo, hash: Hash32 },
}

/// Closed failure taxonomy. Newer cardano-node versions may extend the
/// failure space; per locked decision §7 #3 the codec rejects unknown
/// failure discriminants explicitly rather than silently accepting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquireFailure {
    PointTooOld,
    PointNotOnChain,
}

/// Opaque query bytes (closed wire grammar, semantic content out of
/// scope for the codec).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryPayload(pub Vec<u8>);

/// Opaque result bytes (closed wire grammar, semantic content out of
/// scope for the codec).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultPayload(pub Vec<u8>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalStateQueryMessage {
    /// Acquire a snapshot at a specific Point. Wire form `[0, point]`.
    Acquire(Point),
    /// Acquire the current immutable tip (no point given). Wire form `[8]`.
    AcquireNoPoint,
    Acquired,
    Failure(AcquireFailure),
    Query(QueryPayload),
    Result(ResultPayload),
    Release,
    /// Re-acquire at a specific Point. Wire form `[6, point]`.
    ReAcquire(Point),
    /// Re-acquire the current immutable tip. Wire form `[9]`.
    ReAcquireNoPoint,
    Done,
}

/// Encode a Point per Ouroboros LSQ wire form:
///   Point::Origin   → `[]` (encodeListLen 0)
///   Point::Block    → `[slot, hash32]` (encodeListLen 2)
/// No Option-wrapper on the wire — `Maybe Point` is expressed by
/// using a different message tag (8/9 for No-Point variants).
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

pub fn encode_local_state_query_message(msg: &LocalStateQueryMessage) -> Vec<u8> {
    let mut buf = Vec::new();
    match msg {
        LocalStateQueryMessage::Acquire(point) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 0);
            encode_point(&mut buf, point);
        }
        LocalStateQueryMessage::Acquired => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 1);
        }
        LocalStateQueryMessage::Failure(f) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 2);
            let code: u64 = match f {
                AcquireFailure::PointTooOld => 0,
                AcquireFailure::PointNotOnChain => 1,
            };
            encode_u64(&mut buf, code);
        }
        LocalStateQueryMessage::Query(q) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 3);
            encode_bytes(&mut buf, &q.0);
        }
        LocalStateQueryMessage::Result(r) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 4);
            encode_bytes(&mut buf, &r.0);
        }
        LocalStateQueryMessage::Release => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 5);
        }
        LocalStateQueryMessage::ReAcquire(point) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 6);
            encode_point(&mut buf, point);
        }
        LocalStateQueryMessage::Done => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 7);
        }
        LocalStateQueryMessage::AcquireNoPoint => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 8);
        }
        LocalStateQueryMessage::ReAcquireNoPoint => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 9);
        }
    }
    buf
}

pub fn decode_local_state_query_message(
    bytes: &[u8],
) -> Result<LocalStateQueryMessage, CodecError> {
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
        (0, 2) => LocalStateQueryMessage::Acquire(decode_point(bytes, &mut offset)?),
        (1, 1) => LocalStateQueryMessage::Acquired,
        (2, 2) => {
            let code = decode_u64(PROTOCOL, bytes, &mut offset)?;
            let f = match code {
                0 => AcquireFailure::PointTooOld,
                1 => AcquireFailure::PointNotOnChain,
                other => {
                    return Err(CodecError::InvalidIntegerRange {
                        protocol: PROTOCOL,
                        field: "acquire failure code",
                        value: other,
                    })
                }
            };
            LocalStateQueryMessage::Failure(f)
        }
        (3, 2) => {
            let q = decode_bytes(PROTOCOL, bytes, &mut offset)?;
            LocalStateQueryMessage::Query(QueryPayload(q))
        }
        (4, 2) => {
            let r = decode_bytes(PROTOCOL, bytes, &mut offset)?;
            LocalStateQueryMessage::Result(ResultPayload(r))
        }
        (5, 1) => LocalStateQueryMessage::Release,
        (6, 2) => LocalStateQueryMessage::ReAcquire(decode_point(bytes, &mut offset)?),
        (7, 1) => LocalStateQueryMessage::Done,
        (8, 1) => LocalStateQueryMessage::AcquireNoPoint,
        (9, 1) => LocalStateQueryMessage::ReAcquireNoPoint,
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

    fn sample_messages() -> Vec<LocalStateQueryMessage> {
        vec![
            LocalStateQueryMessage::AcquireNoPoint,
            LocalStateQueryMessage::Acquire(Point::Origin),
            LocalStateQueryMessage::Acquire(Point::Block {
                slot: SlotNo(99),
                hash: Hash32([0xAB; 32]),
            }),
            LocalStateQueryMessage::Acquired,
            LocalStateQueryMessage::Failure(AcquireFailure::PointTooOld),
            LocalStateQueryMessage::Failure(AcquireFailure::PointNotOnChain),
            LocalStateQueryMessage::Query(QueryPayload(vec![0xCA, 0xFE])),
            LocalStateQueryMessage::Result(ResultPayload(vec![0xBA, 0xBE])),
            LocalStateQueryMessage::Release,
            LocalStateQueryMessage::ReAcquireNoPoint,
            LocalStateQueryMessage::ReAcquire(Point::Block {
                slot: SlotNo(42),
                hash: Hash32([0xCD; 32]),
            }),
            LocalStateQueryMessage::Done,
        ]
    }

    #[test]
    fn roundtrip_every_variant() {
        for msg in sample_messages() {
            let bytes = encode_local_state_query_message(&msg);
            let decoded = decode_local_state_query_message(&bytes).expect("decode");
            assert_eq!(decoded, msg);
            assert_eq!(encode_local_state_query_message(&decoded), bytes);
        }
    }

    #[test]
    fn decode_rejects_unknown_tag() {
        let bytes = vec![0x81, 0x18, 0x63];
        match decode_local_state_query_message(&bytes) {
            Err(CodecError::UnknownTag { protocol: ProtocolKind::LocalStateQuery, tag: 99 }) => {}
            other => panic!("expected UnknownTag, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_truncated_input() {
        let full = encode_local_state_query_message(&LocalStateQueryMessage::Query(QueryPayload(
            vec![0xCA, 0xFE, 0xBA, 0xBE],
        )));
        for n in 0..full.len() {
            let slice = &full[..n];
            let err = decode_local_state_query_message(slice).expect_err("must reject truncated");
            match err {
                CodecError::Truncated { .. }
                | CodecError::MalformedCbor { .. }
                | CodecError::InvalidProtocolMessage { .. } => {}
                other => panic!("expected truncation-class error, got {other:?}"),
            }
        }
    }
}
