// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2C LocalTxMonitor mini-protocol message codec (BLUE).
//
// Wire grammar (cardano-node 11.0.1
// `ouroboros-network/protocols/lib/Ouroboros/Network/Protocol/LocalTxMonitor/Codec.hs`):
//
//   localTxMonitorMessage =
//       [0]                                           ; MsgDone
//     / [1]                                           ; MsgAcquire             (Idle, Client)
//     / [1]                                           ; MsgAwaitAcquire        (Acquired, Client) — same tag
//     / [2, slot]                                     ; MsgAcquired
//     / [3]                                           ; MsgRelease
//     / [5]                                           ; MsgNextTx
//     / [6]                                           ; MsgReplyNextTx Nothing
//     / [6, txBytes]                                  ; MsgReplyNextTx (Just tx)
//     / [7, txid]                                     ; MsgHasTx
//     / [8, bool]                                     ; MsgReplyHasTx
//     / [9]                                           ; MsgGetSizes
//     / [10, [capBytes, sizeBytes, txCount]]          ; MsgReplyGetSizes
//     / [11]                                          ; MsgGetMeasures         [v2+]
//     / [12, txCount, measureMap]                     ; MsgReplyGetMeasures    [v2+]
//
// `MsgAcquire` and `MsgAwaitAcquire` share tag 1 on the wire; the
// upstream codec distinguishes them by agency/state alone. To keep
// the codec state-free we emit a single `Acquire` variant on decode;
// the state machine reinterprets `(Acquired, Client, Acquire)` as
// re-acquire semantics (ReAcquireRequested event).

use ade_types::{Hash32, SlotNo, TxId};
use std::collections::BTreeMap;

use crate::codec::error::{CodecError, ProtocolKind};
use crate::codec::primitives::{
    decode_array_header, decode_array_of_len, decode_bool, decode_bytes, decode_u32, decode_u64,
    encode_array_header, encode_bool, encode_bytes, encode_text, encode_u64, require_consumed,
};

const PROTOCOL: ProtocolKind = ProtocolKind::LocalTxMonitor;

/// Mempool size and capacity, as carried by `MsgReplyGetSizes`.
/// Upstream is `SizeAndCapacity Word32` (three `Word32` fields).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MempoolSizeAndCapacity {
    pub capacity_bytes: u32,
    pub size_bytes: u32,
    pub tx_count: u32,
}

/// Measure name on the wire (CBOR Text). The inner buffer is always
/// valid UTF-8 — the type's constructors are the only way in, and both
/// validate. This makes the encode path total without an unwrap.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MeasureName(Vec<u8>);

impl MeasureName {
    pub fn new(s: &str) -> Self {
        MeasureName(s.as_bytes().to_vec())
    }

    pub fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, CodecError> {
        match core::str::from_utf8(&bytes) {
            Ok(_) => Ok(MeasureName(bytes)),
            Err(_) => Err(CodecError::InvalidUtf8 {
                protocol: PROTOCOL,
                field: "measure name",
            }),
        }
    }

    pub fn as_str(&self) -> &str {
        debug_assert!(
            core::str::from_utf8(&self.0).is_ok(),
            "MeasureName invariant violated: inner buffer must be valid UTF-8"
        );
        // Safe: constructors above are the only way to populate `self.0`,
        // and both verify UTF-8.
        core::str::from_utf8(&self.0).unwrap_or_default()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// One measure's size and capacity, as carried inside the measure map
/// of `MsgReplyGetMeasures`. Upstream is `SizeAndCapacity Integer`;
/// `u64` is sufficient for every measure cardano-node currently emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeasureSizeAndCapacity {
    pub size: u64,
    pub capacity: u64,
}

/// `MempoolMeasures` payload of `MsgReplyGetMeasures` (V2+).
///
/// `measures` is a `BTreeMap` for deterministic iteration order —
/// `HashMap` is forbidden under DC-CORE-01.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MempoolMeasures {
    pub tx_count: u32,
    pub measures: BTreeMap<MeasureName, MeasureSizeAndCapacity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalTxMonitorMessage {
    Done,
    Acquire,
    Acquired { slot: SlotNo },
    Release,

    NextTx,
    ReplyNextTx { tx_bytes: Option<Vec<u8>> },

    HasTx { tx_id: TxId },
    ReplyHasTx { present: bool },

    GetSizes,
    ReplyGetSizes(MempoolSizeAndCapacity),

    GetMeasures,
    ReplyGetMeasures(MempoolMeasures),
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

fn encode_sizes(buf: &mut Vec<u8>, sizes: &MempoolSizeAndCapacity) {
    encode_array_header(buf, 3);
    encode_u64(buf, sizes.capacity_bytes as u64);
    encode_u64(buf, sizes.size_bytes as u64);
    encode_u64(buf, sizes.tx_count as u64);
}

fn decode_sizes(data: &[u8], offset: &mut usize) -> Result<MempoolSizeAndCapacity, CodecError> {
    decode_array_of_len(PROTOCOL, data, offset, 3)?;
    let capacity_bytes = decode_u32(PROTOCOL, data, offset, "capacityBytes")?;
    let size_bytes = decode_u32(PROTOCOL, data, offset, "sizeBytes")?;
    let tx_count = decode_u32(PROTOCOL, data, offset, "txCount")?;
    Ok(MempoolSizeAndCapacity {
        capacity_bytes,
        size_bytes,
        tx_count,
    })
}

fn encode_measure_name(buf: &mut Vec<u8>, name: &MeasureName) {
    encode_text(buf, name.as_str());
}

fn decode_measure_name(data: &[u8], offset: &mut usize) -> Result<MeasureName, CodecError> {
    // Decode as text but preserve as bytes for ordering stability.
    match ade_codec::cbor_primitives::read_text(data, offset) {
        Ok((s, _)) => Ok(MeasureName::new(&s)),
        Err(ade_codec::CodecError::InvalidCborStructure {
            detail: "invalid UTF-8 in text string",
            ..
        }) => Err(CodecError::InvalidUtf8 {
            protocol: PROTOCOL,
            field: "measure name",
        }),
        Err(source) => Err(CodecError::MalformedCbor {
            protocol: PROTOCOL,
            source,
        }),
    }
}

fn encode_measures(buf: &mut Vec<u8>, m: &MempoolMeasures) {
    encode_u64(buf, m.tx_count as u64);
    let len = m.measures.len() as u64;
    ade_codec::cbor_primitives::write_map_header(
        buf,
        ade_codec::cbor_primitives::ContainerEncoding::Definite(
            len,
            ade_codec::cbor_primitives::canonical_width(len),
        ),
    );
    for (name, sc) in &m.measures {
        encode_measure_name(buf, name);
        encode_array_header(buf, 2);
        encode_u64(buf, sc.size);
        encode_u64(buf, sc.capacity);
    }
}

fn decode_measures(data: &[u8], offset: &mut usize) -> Result<MempoolMeasures, CodecError> {
    let tx_count = decode_u32(PROTOCOL, data, offset, "txCount")?;
    let enc = ade_codec::cbor_primitives::read_map_header(data, offset).map_err(|source| {
        CodecError::MalformedCbor {
            protocol: PROTOCOL,
            source,
        }
    })?;
    let len = match enc {
        ade_codec::cbor_primitives::ContainerEncoding::Definite(n, _) => n,
        ade_codec::cbor_primitives::ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidProtocolMessage {
                protocol: PROTOCOL,
                reason: "indefinite-length measure map not allowed",
            });
        }
    };
    let mut measures = BTreeMap::new();
    for _ in 0..len {
        let name = decode_measure_name(data, offset)?;
        decode_array_of_len(PROTOCOL, data, offset, 2)?;
        let size = decode_u64(PROTOCOL, data, offset)?;
        let capacity = decode_u64(PROTOCOL, data, offset)?;
        measures.insert(name, MeasureSizeAndCapacity { size, capacity });
    }
    Ok(MempoolMeasures { tx_count, measures })
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
        LocalTxMonitorMessage::Release => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 3);
        }
        LocalTxMonitorMessage::NextTx => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 5);
        }
        LocalTxMonitorMessage::ReplyNextTx { tx_bytes } => match tx_bytes {
            None => {
                encode_array_header(&mut buf, 1);
                encode_u64(&mut buf, 6);
            }
            Some(tx) => {
                encode_array_header(&mut buf, 2);
                encode_u64(&mut buf, 6);
                encode_bytes(&mut buf, tx);
            }
        },
        LocalTxMonitorMessage::HasTx { tx_id } => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 7);
            encode_tx_id(&mut buf, tx_id);
        }
        LocalTxMonitorMessage::ReplyHasTx { present } => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 8);
            encode_bool(&mut buf, *present);
        }
        LocalTxMonitorMessage::GetSizes => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 9);
        }
        LocalTxMonitorMessage::ReplyGetSizes(sizes) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 10);
            encode_sizes(&mut buf, sizes);
        }
        LocalTxMonitorMessage::GetMeasures => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 11);
        }
        LocalTxMonitorMessage::ReplyGetMeasures(measures) => {
            encode_array_header(&mut buf, 3);
            encode_u64(&mut buf, 12);
            encode_measures(&mut buf, measures);
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
        (3, 1) => LocalTxMonitorMessage::Release,
        (5, 1) => LocalTxMonitorMessage::NextTx,
        (6, 1) => LocalTxMonitorMessage::ReplyNextTx { tx_bytes: None },
        (6, 2) => {
            let tx = decode_bytes(PROTOCOL, bytes, &mut offset)?;
            LocalTxMonitorMessage::ReplyNextTx { tx_bytes: Some(tx) }
        }
        (7, 2) => {
            let tx_id = decode_tx_id(bytes, &mut offset)?;
            LocalTxMonitorMessage::HasTx { tx_id }
        }
        (8, 2) => {
            let present = decode_bool(PROTOCOL, bytes, &mut offset)?;
            LocalTxMonitorMessage::ReplyHasTx { present }
        }
        (9, 1) => LocalTxMonitorMessage::GetSizes,
        (10, 2) => {
            let sizes = decode_sizes(bytes, &mut offset)?;
            LocalTxMonitorMessage::ReplyGetSizes(sizes)
        }
        (11, 1) => LocalTxMonitorMessage::GetMeasures,
        (12, 3) => {
            let measures = decode_measures(bytes, &mut offset)?;
            LocalTxMonitorMessage::ReplyGetMeasures(measures)
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

    fn tx_id(seed: u8) -> TxId {
        TxId(Hash32([seed; 32]))
    }

    fn sample_measures() -> MempoolMeasures {
        let mut measures = BTreeMap::new();
        measures.insert(
            MeasureName::new("txs"),
            MeasureSizeAndCapacity {
                size: 12,
                capacity: 1024,
            },
        );
        measures.insert(
            MeasureName::new("bytes"),
            MeasureSizeAndCapacity {
                size: 4096,
                capacity: 65536,
            },
        );
        MempoolMeasures {
            tx_count: 12,
            measures,
        }
    }

    fn sample_messages() -> Vec<LocalTxMonitorMessage> {
        vec![
            LocalTxMonitorMessage::Done,
            LocalTxMonitorMessage::Acquire,
            LocalTxMonitorMessage::Acquired { slot: SlotNo(987654) },
            LocalTxMonitorMessage::Release,
            LocalTxMonitorMessage::NextTx,
            LocalTxMonitorMessage::ReplyNextTx { tx_bytes: None },
            LocalTxMonitorMessage::ReplyNextTx {
                tx_bytes: Some(vec![0xCA, 0xFE, 0xBA, 0xBE]),
            },
            LocalTxMonitorMessage::HasTx { tx_id: tx_id(0x11) },
            LocalTxMonitorMessage::ReplyHasTx { present: true },
            LocalTxMonitorMessage::ReplyHasTx { present: false },
            LocalTxMonitorMessage::GetSizes,
            LocalTxMonitorMessage::ReplyGetSizes(MempoolSizeAndCapacity {
                capacity_bytes: 1_048_576,
                size_bytes: 12_345,
                tx_count: 7,
            }),
            LocalTxMonitorMessage::GetMeasures,
            LocalTxMonitorMessage::ReplyGetMeasures(sample_measures()),
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
            Err(CodecError::UnknownTag {
                protocol: ProtocolKind::LocalTxMonitor,
                tag: 99,
            }) => {}
            other => panic!("expected UnknownTag, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_truncated_input() {
        let full = encode_local_tx_monitor_message(&LocalTxMonitorMessage::ReplyGetSizes(
            MempoolSizeAndCapacity {
                capacity_bytes: 1_048_576,
                size_bytes: 12_345,
                tx_count: 7,
            },
        ));
        for n in 0..full.len() {
            let slice = &full[..n];
            let err =
                decode_local_tx_monitor_message(slice).expect_err("must reject truncated");
            match err {
                CodecError::Truncated { .. }
                | CodecError::MalformedCbor { .. }
                | CodecError::InvalidProtocolMessage { .. } => {}
                other => panic!("expected truncation-class error, got {other:?}"),
            }
        }
    }

    #[test]
    fn decode_rejects_invalid_utf8_measure_name() {
        // Build a ReplyGetMeasures with a measure-name text string
        // containing invalid UTF-8 bytes.
        let mut buf = Vec::new();
        encode_array_header(&mut buf, 3);
        encode_u64(&mut buf, 12);
        encode_u64(&mut buf, 0); // tx_count
        ade_codec::cbor_primitives::write_map_header(
            &mut buf,
            ade_codec::cbor_primitives::ContainerEncoding::Definite(
                1,
                ade_codec::cbor_primitives::canonical_width(1),
            ),
        );
        // CBOR text-string of length 2, with invalid UTF-8 bytes.
        buf.push(0x62);
        buf.push(0xff);
        buf.push(0xfe);
        // Pair value (array of length 2, two u64s).
        encode_array_header(&mut buf, 2);
        encode_u64(&mut buf, 0);
        encode_u64(&mut buf, 0);

        match decode_local_tx_monitor_message(&buf) {
            Err(CodecError::InvalidUtf8 {
                protocol: ProtocolKind::LocalTxMonitor,
                field: "measure name",
            }) => {}
            other => panic!("expected InvalidUtf8, got {other:?}"),
        }
    }
}
