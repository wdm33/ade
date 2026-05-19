// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2C LocalTxMonitor mini-protocol message codec (BLUE).
//
// Wire shape (closed wire grammar — query/reply payloads are opaque):
//   localTxMonitorMessage =
//       [0]                        ; MsgDone
//     / [1]                        ; MsgAcquire
//     / [2, slot]                  ; MsgAcquired
//     / [3]                        ; MsgAwaitAcquire
//     / [4]                        ; MsgRelease
//     / [5, queryPayload]          ; MsgQuery
//     / [6, replyPayload]          ; MsgReply
//
// The query envelope (MsgQuery / MsgReply) carries opaque CBOR bytes
// — the codec verifies grammar but does not interpret semantics. The
// `slot` carried by MsgAcquired is the snapshot slot for the acquired
// mempool view.

use ade_types::SlotNo;

use crate::codec::error::{CodecError, ProtocolKind};
use crate::codec::primitives::{
    decode_array_header, decode_bytes, decode_u64, encode_array_header, encode_bytes, encode_u64,
    require_consumed,
};

const PROTOCOL: ProtocolKind = ProtocolKind::LocalTxMonitor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalTxMonitorQuery(pub Vec<u8>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalTxMonitorReply(pub Vec<u8>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalTxMonitorMessage {
    Done,
    Acquire,
    Acquired { slot: SlotNo },
    AwaitAcquire,
    Release,
    Query(LocalTxMonitorQuery),
    Reply(LocalTxMonitorReply),
}

pub fn encode_local_tx_monitor_message(msg: &LocalTxMonitorMessage) -> Vec<u8> {
    let mut buf = Vec::new();
    match msg {
        LocalTxMonitorMessage::Done => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 0);
        }
        LocalTxMonitorMessage::Acquire => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 1);
        }
        LocalTxMonitorMessage::Acquired { slot } => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 2);
            encode_u64(&mut buf, slot.0);
        }
        LocalTxMonitorMessage::AwaitAcquire => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 3);
        }
        LocalTxMonitorMessage::Release => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 4);
        }
        LocalTxMonitorMessage::Query(q) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 5);
            encode_bytes(&mut buf, &q.0);
        }
        LocalTxMonitorMessage::Reply(r) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 6);
            encode_bytes(&mut buf, &r.0);
        }
    }
    buf
}

pub fn decode_local_tx_monitor_message(bytes: &[u8]) -> Result<LocalTxMonitorMessage, CodecError> {
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
        (0, 1) => LocalTxMonitorMessage::Done,
        (1, 1) => LocalTxMonitorMessage::Acquire,
        (2, 2) => {
            let slot = decode_u64(PROTOCOL, bytes, &mut offset)?;
            LocalTxMonitorMessage::Acquired { slot: SlotNo(slot) }
        }
        (3, 1) => LocalTxMonitorMessage::AwaitAcquire,
        (4, 1) => LocalTxMonitorMessage::Release,
        (5, 2) => {
            let q = decode_bytes(PROTOCOL, bytes, &mut offset)?;
            LocalTxMonitorMessage::Query(LocalTxMonitorQuery(q))
        }
        (6, 2) => {
            let r = decode_bytes(PROTOCOL, bytes, &mut offset)?;
            LocalTxMonitorMessage::Reply(LocalTxMonitorReply(r))
        }
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

    fn sample_messages() -> Vec<LocalTxMonitorMessage> {
        vec![
            LocalTxMonitorMessage::Done,
            LocalTxMonitorMessage::Acquire,
            LocalTxMonitorMessage::Acquired { slot: SlotNo(987654) },
            LocalTxMonitorMessage::AwaitAcquire,
            LocalTxMonitorMessage::Release,
            LocalTxMonitorMessage::Query(LocalTxMonitorQuery(vec![0xCA, 0xFE])),
            LocalTxMonitorMessage::Reply(LocalTxMonitorReply(vec![0xBA, 0xBE])),
        ]
    }

    #[test]
    fn roundtrip_every_variant() {
        for msg in sample_messages() {
            let bytes = encode_local_tx_monitor_message(&msg);
            let decoded = decode_local_tx_monitor_message(&bytes).expect("decode");
            assert_eq!(decoded, msg);
            assert_eq!(encode_local_tx_monitor_message(&decoded), bytes);
        }
    }

    #[test]
    fn decode_rejects_unknown_tag() {
        let bytes = vec![0x81, 0x18, 0x63];
        match decode_local_tx_monitor_message(&bytes) {
            Err(CodecError::UnknownTag { protocol: ProtocolKind::LocalTxMonitor, tag: 99 }) => {}
            other => panic!("expected UnknownTag, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_truncated_input() {
        let full = encode_local_tx_monitor_message(&LocalTxMonitorMessage::Query(
            LocalTxMonitorQuery(vec![0xCA, 0xFE, 0xBA, 0xBE]),
        ));
        for n in 0..full.len() {
            let slice = &full[..n];
            let err = decode_local_tx_monitor_message(slice).expect_err("must reject truncated");
            match err {
                CodecError::Truncated { .. }
                | CodecError::MalformedCbor { .. }
                | CodecError::InvalidProtocolMessage { .. } => {}
                other => panic!("expected truncation-class error, got {other:?}"),
            }
        }
    }
}
