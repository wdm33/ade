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
//       [6]                                                ; MsgInit
//     / [0, blocking(bool), ackTxIds(u16), reqTxIds(u16)]  ; MsgRequestTxIds
//     / [1, txIdSeq]                                       ; MsgReplyTxIds
//     / [2, txIdSeq2]                                      ; MsgRequestTxs
//     / [3, txSeq]                                         ; MsgReplyTxs
//     / [4]                                                ; MsgDone
//
//   txIdSeq  = SEQ of [txId, size]      ; the (id, byte-size) advertisements
//   txIdSeq2 = SEQ of txId              ; the ids whose bodies are requested
//   txSeq    = SEQ of eraTx             ; era-wrapped tx bytes (opaque here)
//   txId     = [eraIndex, bytes]        ; HardFork-wrapped, ERA-TAGGED txid
//                                       ; e.g. [6, h'..32'] for Conway
//   SEQ      = a definite OR indefinite-length CBOR array
//
// Two real cardano-node wire facts, confirmed by the server-side capture
// (corpus/network/n2n/tx_submission2/), that synthetic round-trip tests using
// definite arrays + bare 32-byte txids had MISSED:
//   1. cardano-node emits the txId/tx sequences as CBOR INDEFINITE-length
//      arrays (`9f .. ff`), not definite.
//   2. each txId is ERA-TAGGED `[eraIndex, hash32]` (the HardFork GenTxId),
//      not a bare 32-byte byte string.
//
// Byte-authority doctrine: the decoder accepts BOTH definite and indefinite
// sequences; the encoder reproduces cardano-node's indefinite form so a
// captured frame re-encodes BYTE-IDENTICALLY. The era tag is preserved
// explicitly (never stripped, never guessed) so a requester echoes the exact
// txid the provider advertised.

use ade_types::{Hash32, TxId};

use crate::codec::error::{CodecError, ProtocolKind};
use crate::codec::primitives::{
    decode_array_header, decode_bool, decode_bytes, decode_u16, decode_u32, decode_u64,
    encode_array_header, encode_bool, encode_bytes, encode_u64, require_consumed,
};
use ade_codec::cbor_primitives as cbp;

const PROTOCOL: ProtocolKind = ProtocolKind::TxSubmission2;

/// CBOR break byte that terminates an indefinite-length array.
const CBOR_BREAK: u8 = 0xFF;

/// A HardFork-wrapped, ERA-TAGGED transaction id as it appears on the N2N
/// tx-submission2 wire: `[eraIndex, hash32]`. cardano-node tags every txid
/// with its HardFork era index (e.g. 6 = Conway); the tag is preserved so a
/// requester can echo the exact advertised id back in MsgRequestTxs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxSubmissionTxId {
    /// HardFork era index the id is tagged with (e.g. 6 = Conway consensus index).
    pub era: u64,
    /// The 32-byte Blake2b-256 transaction-body hash.
    pub id: TxId,
}

/// Pairing of an (era-tagged) transaction id with its serialised byte length,
/// as advertised in MsgReplyTxIds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxIdAndSize {
    pub tx_id: TxSubmissionTxId,
    pub size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxSubmission2Message {
    Init,
    RequestTxIds { blocking: bool, ack: u16, req: u16 },
    ReplyTxIds(Vec<TxIdAndSize>),
    RequestTxs(Vec<TxSubmissionTxId>),
    ReplyTxs(Vec<Vec<u8>>),
    Done,
}

// ---------------------------------------------------------------------------
// txid + sequence helpers
// ---------------------------------------------------------------------------

fn encode_tx_submission_txid(buf: &mut Vec<u8>, id: &TxSubmissionTxId) {
    // [eraIndex, bytes(hash32)]
    encode_array_header(buf, 2);
    encode_u64(buf, id.era);
    encode_bytes(buf, id.id.as_bytes());
}

fn decode_tx_submission_txid(
    data: &[u8],
    offset: &mut usize,
) -> Result<TxSubmissionTxId, CodecError> {
    let arr = decode_array_header(PROTOCOL, data, offset)?;
    if arr != 2 {
        return Err(CodecError::InvalidProtocolMessage {
            protocol: PROTOCOL,
            reason: "tx-submission2 txid must be [eraIndex, hash]",
        });
    }
    let era = decode_u64(PROTOCOL, data, offset)?;
    let bytes = decode_bytes(PROTOCOL, data, offset)?;
    if bytes.len() != 32 {
        return Err(CodecError::InvalidProtocolMessage {
            protocol: PROTOCOL,
            reason: "tx-submission2 txid hash not 32 bytes",
        });
    }
    let mut h = [0u8; 32];
    h.copy_from_slice(&bytes);
    Ok(TxSubmissionTxId {
        era,
        id: TxId(Hash32(h)),
    })
}

/// Begin a tx-submission sequence. cardano-node emits the txid/tx sequences as
/// CBOR indefinite-length arrays; we reproduce that form so captured frames
/// re-encode byte-identically.
fn write_seq_header(buf: &mut Vec<u8>) {
    cbp::write_array_header(buf, cbp::ContainerEncoding::Indefinite);
}

fn write_seq_footer(buf: &mut Vec<u8>) {
    cbp::write_break(buf);
}

/// Decode a definite- OR indefinite-length sequence, invoking `decode_item`
/// per element. cardano-node emits indefinite (`9f .. ff`); a definite array is
/// also accepted for robustness. The break byte (`0xFF`) is never a valid
/// element start here, so it unambiguously terminates the indefinite form.
fn decode_seq<T, F>(
    data: &[u8],
    offset: &mut usize,
    mut decode_item: F,
) -> Result<Vec<T>, CodecError>
where
    F: FnMut(&[u8], &mut usize) -> Result<T, CodecError>,
{
    let enc = cbp::read_array_header(data, offset)
        .map_err(|source| CodecError::MalformedCbor { protocol: PROTOCOL, source })?;
    let mut out = Vec::new();
    match enc {
        cbp::ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                out.push(decode_item(data, offset)?);
            }
        }
        cbp::ContainerEncoding::Indefinite => loop {
            match data.get(*offset) {
                Some(&CBOR_BREAK) => {
                    *offset += 1;
                    break;
                }
                Some(_) => out.push(decode_item(data, offset)?),
                None => {
                    return Err(CodecError::Truncated {
                        needed: *offset + 1,
                        got: data.len(),
                    })
                }
            }
        },
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// encode / decode
// ---------------------------------------------------------------------------

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
            write_seq_header(&mut buf);
            for e in entries {
                encode_array_header(&mut buf, 2);
                encode_tx_submission_txid(&mut buf, &e.tx_id);
                encode_u64(&mut buf, e.size as u64);
            }
            write_seq_footer(&mut buf);
        }
        TxSubmission2Message::RequestTxs(ids) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 2);
            write_seq_header(&mut buf);
            for id in ids {
                encode_tx_submission_txid(&mut buf, id);
            }
            write_seq_footer(&mut buf);
        }
        TxSubmission2Message::ReplyTxs(txs) => {
            // Each tx is an era-discriminated HFC-wrapped CBOR item
            // `[era_idx, tag24(bytes)]`, carried verbatim — opaque to this
            // codec layer (same wire form as LocalTxSubmission MsgSubmitTx).
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 3);
            write_seq_header(&mut buf);
            for tx in txs {
                buf.extend_from_slice(tx);
            }
            write_seq_footer(&mut buf);
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
            let entries = decode_seq(bytes, &mut offset, |d, o| {
                let pair_len = decode_array_header(PROTOCOL, d, o)?;
                if pair_len != 2 {
                    return Err(CodecError::InvalidProtocolMessage {
                        protocol: PROTOCOL,
                        reason: "txid/size pair must be 2 elements",
                    });
                }
                let tx_id = decode_tx_submission_txid(d, o)?;
                let size = decode_u32(PROTOCOL, d, o, "tx size")?;
                Ok(TxIdAndSize { tx_id, size })
            })?;
            TxSubmission2Message::ReplyTxIds(entries)
        }
        (2, 2) => {
            let ids = decode_seq(bytes, &mut offset, decode_tx_submission_txid)?;
            TxSubmission2Message::RequestTxs(ids)
        }
        (3, 2) => {
            let txs = decode_seq(bytes, &mut offset, |d, o| {
                let start = *o;
                cbp::skip_item(d, o)
                    .map_err(|source| CodecError::MalformedCbor { protocol: PROTOCOL, source })?;
                Ok(d[start..*o].to_vec())
            })?;
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

    fn txid(era: u64, seed: u8) -> TxSubmissionTxId {
        TxSubmissionTxId {
            era,
            id: TxId(Hash32([seed; 32])),
        }
    }

    fn sample_messages() -> Vec<TxSubmission2Message> {
        vec![
            TxSubmission2Message::Init,
            TxSubmission2Message::RequestTxIds { blocking: true, ack: 3, req: 7 },
            TxSubmission2Message::RequestTxIds { blocking: false, ack: 0, req: 0 },
            TxSubmission2Message::ReplyTxIds(vec![
                TxIdAndSize { tx_id: txid(6, 0x01), size: 200 },
                TxIdAndSize { tx_id: txid(6, 0x02), size: 300 },
            ]),
            TxSubmission2Message::RequestTxs(vec![txid(6, 0x11), txid(6, 0x22)]),
            // Synthetic HFC-wrapped txs: `[era_idx, tag24(bytes)]`.
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
    fn encoder_emits_indefinite_sequences() {
        // cardano-node form: the txid/tx sequence is an indefinite array.
        let bytes = encode_tx_submission_message(&TxSubmission2Message::ReplyTxIds(vec![
            TxIdAndSize { tx_id: txid(6, 0x01), size: 604 },
        ]));
        assert_eq!(&bytes[0..3], &[0x82, 0x01, 0x9f], "outer [1, indefinite-seq]");
        assert_eq!(*bytes.last().unwrap(), CBOR_BREAK, "seq terminated by break");
    }

    #[test]
    fn real_cardano_reply_txids_decodes_and_re_encodes_byte_identical() {
        // The exact MsgReplyTxIds payload captured from the docker preprod
        // cardano-node 11.0.1 (server-side capture). Shape:
        //   [1, 9f [ [6, h'..32'], 604 ] ff]
        // i.e. indefinite entries seq + era-tagged (Conway=6) txid. This is the
        // frame Ade's previous codec false-rejected ("indefinite-length array
        // not allowed").
        let wire = [
            0x82, 0x01, 0x9f, 0x82, 0x82, 0x06, 0x58, 0x20, 0x22, 0x61, 0x82, 0xcf, 0x9e, 0x60,
            0x95, 0x4c, 0x5e, 0xb4, 0x61, 0xcf, 0xbd, 0x7e, 0x9f, 0x1d, 0x25, 0xaa, 0x7e, 0xe0,
            0x08, 0x63, 0x85, 0xba, 0x80, 0x00, 0x3f, 0x20, 0xa1, 0x52, 0x01, 0xae, 0x19, 0x02,
            0x5c, 0xff,
        ];
        let decoded =
            decode_tx_submission_message(&wire).expect("real cardano ReplyTxIds must decode");
        match &decoded {
            TxSubmission2Message::ReplyTxIds(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].tx_id.era, 6, "Conway era tag preserved");
                assert_eq!(entries[0].size, 604);
            }
            other => panic!("expected ReplyTxIds, got {other:?}"),
        }
        assert_eq!(
            encode_tx_submission_message(&decoded),
            wire,
            "captured frame must re-encode byte-identically"
        );
    }

    #[test]
    fn decode_accepts_definite_sequence_form() {
        // Robustness: a definite-length seq must also decode to the same
        // message (cardano-node emits indefinite, but we accept both).
        // [1, 81 [ [6, h'01*32'], 200 ]]
        let mut wire = vec![0x82, 0x01, 0x81, 0x82, 0x82, 0x06, 0x58, 0x20];
        wire.extend_from_slice(&[0x01; 32]);
        wire.extend_from_slice(&[0x18, 0xC8]); // size 200
        let decoded = decode_tx_submission_message(&wire).expect("definite seq decodes");
        assert_eq!(
            decoded,
            TxSubmission2Message::ReplyTxIds(vec![TxIdAndSize {
                tx_id: txid(6, 0x01),
                size: 200
            }])
        );
    }

    #[test]
    fn decode_rejects_bare_txid() {
        // A bare 32-byte byte-string where an era-tagged [era,hash] is required.
        // [1, 9f [ h'01*32', 200 ] ff]
        let mut wire = vec![0x82, 0x01, 0x9f, 0x82, 0x58, 0x20];
        wire.extend_from_slice(&[0x01; 32]);
        wire.extend_from_slice(&[0x18, 0xC8, CBOR_BREAK]);
        match decode_tx_submission_message(&wire) {
            Err(CodecError::MalformedCbor { .. }) | Err(CodecError::InvalidProtocolMessage { .. }) => {}
            other => panic!("bare (non-era-tagged) txid must be rejected, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_wrong_txid_hash_length() {
        // Era-tagged but the hash is 31 bytes, not 32.
        // [1, 9f [ [6, h'01*31'], 200 ] ff]
        let mut wire = vec![0x82, 0x01, 0x9f, 0x82, 0x82, 0x06, 0x58, 0x1F];
        wire.extend_from_slice(&[0x01; 31]);
        wire.extend_from_slice(&[0x18, 0xC8, CBOR_BREAK]);
        match decode_tx_submission_message(&wire) {
            Err(CodecError::InvalidProtocolMessage { reason, .. }) => {
                assert!(reason.contains("32 bytes"), "got: {reason}");
            }
            other => panic!("31-byte txid hash must be rejected, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_unterminated_indefinite_sequence() {
        // [1, 9f [ [6, h'01*32'], 200 ]   (no break byte — truncated)
        let mut wire = vec![0x82, 0x01, 0x9f, 0x82, 0x82, 0x06, 0x58, 0x20];
        wire.extend_from_slice(&[0x01; 32]);
        wire.extend_from_slice(&[0x18, 0xC8]); // size 200, then EOF (no 0xFF)
        match decode_tx_submission_message(&wire) {
            Err(CodecError::Truncated { .. }) => {}
            other => panic!("unterminated indefinite seq must be Truncated, got {other:?}"),
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
