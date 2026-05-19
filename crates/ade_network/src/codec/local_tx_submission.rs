// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2C LocalTxSubmission mini-protocol message codec (BLUE).
//
// Wire shape:
//   localTxSubmissionMessage =
//       [0, txBytes]               ; MsgSubmitTx
//     / [1]                        ; MsgAcceptTx
//     / [2, rejectionBytes]        ; MsgRejectTx
//     / [3]                        ; MsgDone
//
// The submitted transaction and the rejection reason are carried as
// opaque CBOR bytes. The codec models the closed wire grammar; ledger
// interpretation of the submitted tx (or of the rejection variant)
// belongs to the slices following S-A2.

use crate::codec::error::{CodecError, ProtocolKind};
use crate::codec::primitives::{
    decode_array_header, decode_u64, encode_array_header, encode_u64, require_consumed,
};

const PROTOCOL: ProtocolKind = ProtocolKind::LocalTxSubmission;

/// Submitter-side acceptance acknowledgement. Carries no payload but
/// remains an explicit type so callers cannot conflate "accepted" with
/// the ambient `()` unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxAcceptance;

/// Opaque rejection bytes — the ledger-specific reject reason CBOR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxRejection(pub Vec<u8>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalTxSubmissionMessage {
    SubmitTx { tx_bytes: Vec<u8> },
    AcceptTx(TxAcceptance),
    RejectTx(TxRejection),
    Done,
}

pub fn encode_local_tx_submission_message(msg: &LocalTxSubmissionMessage) -> Vec<u8> {
    let mut buf = Vec::new();
    match msg {
        LocalTxSubmissionMessage::SubmitTx { tx_bytes } => {
            // Inner tx is an era-discriminated Hard-Fork-Combinator CBOR
            // item `[era_index, encoded_tx]`, NOT a CBOR byte string.
            // Real interop against cardano-node 11.0.1 closes the
            // connection on `[0, bytes(...)]`.
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 0);
            buf.extend_from_slice(tx_bytes);
        }
        LocalTxSubmissionMessage::AcceptTx(_) => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 1);
        }
        LocalTxSubmissionMessage::RejectTx(reason) => {
            // Reject reason is also an era-discriminated opaque CBOR
            // item (ApplyTxErr), not a byte string.
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 2);
            buf.extend_from_slice(&reason.0);
        }
        LocalTxSubmissionMessage::Done => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 3);
        }
    }
    buf
}

pub fn decode_local_tx_submission_message(
    bytes: &[u8],
) -> Result<LocalTxSubmissionMessage, CodecError> {
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
        (0, 2) => {
            let start = offset;
            ade_codec::cbor_primitives::skip_item(bytes, &mut offset)
                .map_err(|e| CodecError::MalformedCbor { protocol: PROTOCOL, source: e })?;
            LocalTxSubmissionMessage::SubmitTx { tx_bytes: bytes[start..offset].to_vec() }
        }
        (1, 1) => LocalTxSubmissionMessage::AcceptTx(TxAcceptance),
        (2, 2) => {
            let start = offset;
            ade_codec::cbor_primitives::skip_item(bytes, &mut offset)
                .map_err(|e| CodecError::MalformedCbor { protocol: PROTOCOL, source: e })?;
            LocalTxSubmissionMessage::RejectTx(TxRejection(bytes[start..offset].to_vec()))
        }
        (3, 1) => LocalTxSubmissionMessage::Done,
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

    /// Build a synthetic HFC-era-wrapped tx: `[era_idx, h'<inner>']`.
    fn wrapped_tx(era: u64, inner: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_array_header(&mut buf, 2);
        encode_u64(&mut buf, era);
        // bytes(...) header
        if inner.len() < 24 {
            buf.push(0x40 | (inner.len() as u8));
        } else {
            buf.push(0x58);
            buf.push(inner.len() as u8);
        }
        buf.extend_from_slice(inner);
        buf
    }

    fn sample_messages() -> Vec<LocalTxSubmissionMessage> {
        vec![
            LocalTxSubmissionMessage::SubmitTx {
                tx_bytes: wrapped_tx(6, &[0x01, 0x02, 0x03, 0x04]),
            },
            LocalTxSubmissionMessage::AcceptTx(TxAcceptance),
            LocalTxSubmissionMessage::RejectTx(TxRejection(wrapped_tx(6, &[0xDE, 0xAD]))),
            LocalTxSubmissionMessage::Done,
        ]
    }

    #[test]
    fn roundtrip_every_variant() {
        for msg in sample_messages() {
            let bytes = encode_local_tx_submission_message(&msg);
            let decoded = decode_local_tx_submission_message(&bytes).expect("decode");
            assert_eq!(decoded, msg);
            assert_eq!(encode_local_tx_submission_message(&decoded), bytes);
        }
    }

    #[test]
    fn decode_rejects_unknown_tag() {
        let bytes = vec![0x81, 0x18, 0x63];
        match decode_local_tx_submission_message(&bytes) {
            Err(CodecError::UnknownTag {
                protocol: ProtocolKind::LocalTxSubmission,
                tag: 99,
            }) => {}
            other => panic!("expected UnknownTag, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_truncated_input() {
        let full = encode_local_tx_submission_message(&LocalTxSubmissionMessage::SubmitTx {
            tx_bytes: wrapped_tx(6, &[0x01, 0x02, 0x03, 0x04]),
        });
        for n in 0..full.len() {
            let slice = &full[..n];
            let err =
                decode_local_tx_submission_message(slice).expect_err("must reject truncated");
            match err {
                CodecError::Truncated { .. }
                | CodecError::MalformedCbor { .. }
                | CodecError::InvalidProtocolMessage { .. } => {}
                other => panic!("expected truncation-class error, got {other:?}"),
            }
        }
    }
}
