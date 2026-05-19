// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2N TxSubmission2 mini-protocol message codec (BLUE).
//
// Wire shape (cardano-node 11.0.1 (10.6.2 forward-compatible) NodeToNodeV13+):
//   txSubmission2Message =
//       [6]                                       ; MsgInit
//     / [0, blocking(bool), ackTxIds(u16), reqTxIds(u16)] ; MsgRequestTxIds
//     / [1, [(txId, size)*]]                      ; MsgReplyTxIds
//     / [2, [txId*]]                              ; MsgRequestTxs
//     / [3, [txBytes*]]                           ; MsgReplyTxs
//     / [4]                                       ; MsgDone
//
// `TxId` is `ade_types::TxId` (Blake2b-256 of the tx body).

use ade_types::{Hash32, TxId};

use crate::codec::error::{CodecError, ProtocolKind};
use crate::codec::primitives::{
    decode_array_header, decode_bool, decode_bytes, decode_u16, decode_u32, decode_u64,
    encode_array_header, encode_bool, encode_bytes, encode_u64, require_consumed,
};

const PROTOCOL: ProtocolKind = ProtocolKind::TxSubmission2;

/// Pairing of a transaction id with its serialised byte length, as
/// advertised in MsgReplyTxIds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxIdAndSize {
    pub tx_id: TxId,
    pub size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxSubmission2Message {
    Init,
    RequestTxIds { blocking: bool, ack: u16, req: u16 },
    ReplyTxIds(Vec<TxIdAndSize>),
    RequestTxs(Vec<TxId>),
    ReplyTxs(Vec<Vec<u8>>),
    Done,
}

fn encode_tx_id(buf: &mut Vec<u8>, id: &TxId) {
    encode_bytes(buf, id.as_bytes());
}

fn decode_tx_id(data: &[u8], offset: &mut usize) -> Result<TxId, CodecError> {
    let bytes = decode_bytes(PROTOCOL, data, offset)?;
    if bytes.len() != 32 {
        return Err(CodecError::InvalidProtocolMessage {
            protocol: PROTOCOL,
            reason: "TxId not 32 bytes",
        });
    }
    let mut h = [0u8; 32];
    h.copy_from_slice(&bytes);
    Ok(TxId(Hash32(h)))
}

pub fn encode_tx_submission_message(msg: &TxSubmission2Message) -> Vec<u8> {
    let mut buf = Vec::new();
    match msg {
        TxSubmission2Message::Init => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 6);
        }
        TxSubmission2Message::RequestTxIds { blocking, ack, req } => {
            encode_array_header(&mut buf, 4);
            encode_u64(&mut buf, 0);
            encode_bool(&mut buf, *blocking);
            encode_u64(&mut buf, *ack as u64);
            encode_u64(&mut buf, *req as u64);
        }
        TxSubmission2Message::ReplyTxIds(entries) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 1);
            encode_array_header(&mut buf, entries.len() as u64);
            for e in entries {
                encode_array_header(&mut buf, 2);
                encode_tx_id(&mut buf, &e.tx_id);
                encode_u64(&mut buf, e.size as u64);
            }
        }
        TxSubmission2Message::RequestTxs(ids) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 2);
            encode_array_header(&mut buf, ids.len() as u64);
            for id in ids {
                encode_tx_id(&mut buf, id);
            }
        }
        TxSubmission2Message::ReplyTxs(txs) => {
            // Each tx is an era-discriminated HFC-wrapped CBOR item
            // `[era_idx, tag24(bytes)]`, NOT a byte string. Same wire
            // form as LocalTxSubmission MsgSubmitTx. We carry the
            // wrapped item verbatim — opaque to this codec layer.
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 3);
            encode_array_header(&mut buf, txs.len() as u64);
            for tx in txs {
                buf.extend_from_slice(tx);
            }
        }
        TxSubmission2Message::Done => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 4);
        }
    }
    buf
}

pub fn decode_tx_submission_message(bytes: &[u8]) -> Result<TxSubmission2Message, CodecError> {
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
        (6, 1) => TxSubmission2Message::Init,
        (0, 4) => {
            let blocking = decode_bool(PROTOCOL, bytes, &mut offset)?;
            let ack = decode_u16(PROTOCOL, bytes, &mut offset, "ack")?;
            let req = decode_u16(PROTOCOL, bytes, &mut offset, "req")?;
            TxSubmission2Message::RequestTxIds { blocking, ack, req }
        }
        (1, 2) => {
            let n = decode_array_header(PROTOCOL, bytes, &mut offset)?;
            let mut entries = Vec::with_capacity((n as usize).min(bytes.len()));
            for _ in 0..n {
                let pair_len = decode_array_header(PROTOCOL, bytes, &mut offset)?;
                if pair_len != 2 {
                    return Err(CodecError::InvalidProtocolMessage {
                        protocol: PROTOCOL,
                        reason: "txid/size pair must be 2 elements",
                    });
                }
                let tx_id = decode_tx_id(bytes, &mut offset)?;
                let size = decode_u32(PROTOCOL, bytes, &mut offset, "tx size")?;
                entries.push(TxIdAndSize { tx_id, size });
            }
            TxSubmission2Message::ReplyTxIds(entries)
        }
        (2, 2) => {
            let n = decode_array_header(PROTOCOL, bytes, &mut offset)?;
            let mut ids = Vec::with_capacity((n as usize).min(bytes.len()));
            for _ in 0..n {
                ids.push(decode_tx_id(bytes, &mut offset)?);
            }
            TxSubmission2Message::RequestTxs(ids)
        }
        (3, 2) => {
            let n = decode_array_header(PROTOCOL, bytes, &mut offset)?;
            let mut txs = Vec::with_capacity((n as usize).min(bytes.len()));
            for _ in 0..n {
                let start = offset;
                ade_codec::cbor_primitives::skip_item(bytes, &mut offset)
                    .map_err(|e| CodecError::MalformedCbor { protocol: PROTOCOL, source: e })?;
                txs.push(bytes[start..offset].to_vec());
            }
            TxSubmission2Message::ReplyTxs(txs)
        }
        (4, 1) => TxSubmission2Message::Done,
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

    fn tx_id(seed: u8) -> TxId {
        TxId(Hash32([seed; 32]))
    }

    fn sample_messages() -> Vec<TxSubmission2Message> {
        vec![
            TxSubmission2Message::Init,
            TxSubmission2Message::RequestTxIds { blocking: true, ack: 3, req: 7 },
            TxSubmission2Message::RequestTxIds { blocking: false, ack: 0, req: 0 },
            TxSubmission2Message::ReplyTxIds(vec![
                TxIdAndSize { tx_id: tx_id(0x01), size: 200 },
                TxIdAndSize { tx_id: tx_id(0x02), size: 300 },
            ]),
            TxSubmission2Message::RequestTxs(vec![tx_id(0x11), tx_id(0x22)]),
            // Synthetic HFC-wrapped txs: `[era_idx, tag24(bytes)]`.
            // Encoded bytes for `[6, tag(24, h'aabb')]` and
            // `[6, tag(24, h'cc')]` respectively.
            TxSubmission2Message::ReplyTxs(vec![
                vec![0x82, 0x06, 0xd8, 0x18, 0x42, 0xAA, 0xBB],
                vec![0x82, 0x06, 0xd8, 0x18, 0x41, 0xCC],
            ]),
            TxSubmission2Message::Done,
        ]
    }

    #[test]
    fn roundtrip_every_variant() {
        for msg in sample_messages() {
            let bytes = encode_tx_submission_message(&msg);
            let decoded = decode_tx_submission_message(&bytes).expect("decode");
            assert_eq!(decoded, msg);
            assert_eq!(encode_tx_submission_message(&decoded), bytes);
        }
    }

    #[test]
    fn decode_rejects_unknown_tag() {
        let bytes = vec![0x81, 0x18, 0x63];
        match decode_tx_submission_message(&bytes) {
            Err(CodecError::UnknownTag { protocol: ProtocolKind::TxSubmission2, tag: 99 }) => {}
            other => panic!("expected UnknownTag, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_truncated_input() {
        let full = encode_tx_submission_message(&TxSubmission2Message::RequestTxIds {
            blocking: true,
            ack: 3,
            req: 7,
        });
        for n in 0..full.len() {
            let slice = &full[..n];
            let err = decode_tx_submission_message(slice).expect_err("must reject truncated");
            match err {
                CodecError::Truncated { .. }
                | CodecError::MalformedCbor { .. }
                | CodecError::InvalidProtocolMessage { .. } => {}
                other => panic!("expected truncation-class error, got {other:?}"),
            }
        }
    }
}
