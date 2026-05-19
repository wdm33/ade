// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2C Handshake mini-protocol message codec (BLUE).
//
// The wire shape mirrors the N2N handshake but the version table is
// keyed by `N2CVersion`. The per-protocol closed enum is deliberately
// distinct from `HandshakeMessage` so the type system rejects mixing
// N2N and N2C handshakes across session boundaries.

use crate::codec::error::{CodecError, ProtocolKind};
use crate::codec::primitives::{
    decode_array_header, decode_text, decode_u32, decode_u64, encode_array_header, encode_text,
    encode_u64, require_consumed,
};
use crate::codec::version::N2CVersion;

const PROTOCOL: ProtocolKind = ProtocolKind::N2cHandshake;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct N2cVersionParams(pub Vec<u8>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct N2cVersionTable(pub Vec<(N2CVersion, N2cVersionParams)>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum N2cRefuseReason {
    VersionMismatch(Vec<N2CVersion>),
    HandshakeDecodeError { version: N2CVersion, reason: String },
    Refused { version: N2CVersion, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum N2cHandshakeMessage {
    ProposeVersions(N2cVersionTable),
    AcceptVersion(N2CVersion, N2cVersionParams),
    Refuse(N2cRefuseReason),
    QueryReply(N2cVersionTable),
}

fn encode_table(buf: &mut Vec<u8>, table: &N2cVersionTable) {
    let len = table.0.len() as u64;
    ade_codec::cbor_primitives::write_map_header(
        buf,
        ade_codec::cbor_primitives::ContainerEncoding::Definite(
            len,
            ade_codec::cbor_primitives::canonical_width(len),
        ),
    );
    for (ver, params) in &table.0 {
        encode_u64(buf, ver.get() as u64);
        buf.extend_from_slice(&params.0);
    }
}

fn encode_refuse(buf: &mut Vec<u8>, reason: &N2cRefuseReason) {
    match reason {
        N2cRefuseReason::VersionMismatch(vs) => {
            encode_array_header(buf, 2);
            encode_u64(buf, 0);
            encode_array_header(buf, vs.len() as u64);
            for v in vs {
                encode_u64(buf, v.get() as u64);
            }
        }
        N2cRefuseReason::HandshakeDecodeError { version, reason } => {
            encode_array_header(buf, 3);
            encode_u64(buf, 1);
            encode_u64(buf, version.get() as u64);
            encode_text(buf, reason);
        }
        N2cRefuseReason::Refused { version, reason } => {
            encode_array_header(buf, 3);
            encode_u64(buf, 2);
            encode_u64(buf, version.get() as u64);
            encode_text(buf, reason);
        }
    }
}

pub fn encode_n2c_handshake_message(msg: &N2cHandshakeMessage) -> Vec<u8> {
    let mut buf = Vec::new();
    match msg {
        N2cHandshakeMessage::ProposeVersions(table) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 0);
            encode_table(&mut buf, table);
        }
        N2cHandshakeMessage::AcceptVersion(ver, params) => {
            encode_array_header(&mut buf, 3);
            encode_u64(&mut buf, 1);
            encode_u64(&mut buf, ver.get() as u64);
            buf.extend_from_slice(&params.0);
        }
        N2cHandshakeMessage::Refuse(reason) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 2);
            encode_refuse(&mut buf, reason);
        }
        N2cHandshakeMessage::QueryReply(table) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 3);
            encode_table(&mut buf, table);
        }
    }
    buf
}

fn decode_table(data: &[u8], offset: &mut usize) -> Result<N2cVersionTable, CodecError> {
    let enc = ade_codec::cbor_primitives::read_map_header(data, offset)
        .map_err(|source| CodecError::MalformedCbor { protocol: PROTOCOL, source })?;
    let len = match enc {
        ade_codec::cbor_primitives::ContainerEncoding::Definite(n, _) => n,
        ade_codec::cbor_primitives::ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidProtocolMessage {
                protocol: PROTOCOL,
                reason: "indefinite-length version table not allowed",
            });
        }
    };
    let mut entries = Vec::with_capacity(len as usize);
    for _ in 0..len {
        let v = decode_u32(PROTOCOL, data, offset, "version number")?;
        if v > u16::MAX as u32 {
            return Err(CodecError::InvalidIntegerRange {
                protocol: PROTOCOL,
                field: "version number",
                value: v as u64,
            });
        }
        let start = *offset;
        let (_, end) = ade_codec::cbor_primitives::skip_item(data, offset)
            .map_err(|source| CodecError::MalformedCbor { protocol: PROTOCOL, source })?;
        entries.push((N2CVersion::new(v as u16), N2cVersionParams(data[start..end].to_vec())));
    }
    Ok(N2cVersionTable(entries))
}

fn decode_refuse(data: &[u8], offset: &mut usize) -> Result<N2cRefuseReason, CodecError> {
    let arr_len = decode_array_header(PROTOCOL, data, offset)?;
    if arr_len < 1 {
        return Err(CodecError::InvalidProtocolMessage {
            protocol: PROTOCOL,
            reason: "refuse reason has zero-length array",
        });
    }
    let tag = decode_u64(PROTOCOL, data, offset)?;
    match (tag, arr_len) {
        (0, 2) => {
            let n = decode_array_header(PROTOCOL, data, offset)?;
            let mut vs = Vec::with_capacity(n as usize);
            for _ in 0..n {
                let v = decode_u32(PROTOCOL, data, offset, "version number")?;
                if v > u16::MAX as u32 {
                    return Err(CodecError::InvalidIntegerRange {
                        protocol: PROTOCOL,
                        field: "version number",
                        value: v as u64,
                    });
                }
                vs.push(N2CVersion::new(v as u16));
            }
            Ok(N2cRefuseReason::VersionMismatch(vs))
        }
        (1, 3) => {
            let v = decode_u32(PROTOCOL, data, offset, "version number")?;
            if v > u16::MAX as u32 {
                return Err(CodecError::InvalidIntegerRange {
                    protocol: PROTOCOL,
                    field: "version number",
                    value: v as u64,
                });
            }
            let reason = decode_text(PROTOCOL, data, offset, "handshake decode error reason")?;
            Ok(N2cRefuseReason::HandshakeDecodeError {
                version: N2CVersion::new(v as u16),
                reason,
            })
        }
        (2, 3) => {
            let v = decode_u32(PROTOCOL, data, offset, "version number")?;
            if v > u16::MAX as u32 {
                return Err(CodecError::InvalidIntegerRange {
                    protocol: PROTOCOL,
                    field: "version number",
                    value: v as u64,
                });
            }
            let reason = decode_text(PROTOCOL, data, offset, "refused reason")?;
            Ok(N2cRefuseReason::Refused { version: N2CVersion::new(v as u16), reason })
        }
        (other, _) => Err(CodecError::UnknownTag { protocol: PROTOCOL, tag: other }),
    }
}

pub fn decode_n2c_handshake_message(bytes: &[u8]) -> Result<N2cHandshakeMessage, CodecError> {
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
        (0, 2) => N2cHandshakeMessage::ProposeVersions(decode_table(bytes, &mut offset)?),
        (1, 3) => {
            let v = decode_u32(PROTOCOL, bytes, &mut offset, "version number")?;
            if v > u16::MAX as u32 {
                return Err(CodecError::InvalidIntegerRange {
                    protocol: PROTOCOL,
                    field: "version number",
                    value: v as u64,
                });
            }
            let start = offset;
            let (_, end) = ade_codec::cbor_primitives::skip_item(bytes, &mut offset)
                .map_err(|source| CodecError::MalformedCbor { protocol: PROTOCOL, source })?;
            N2cHandshakeMessage::AcceptVersion(
                N2CVersion::new(v as u16),
                N2cVersionParams(bytes[start..end].to_vec()),
            )
        }
        (2, 2) => N2cHandshakeMessage::Refuse(decode_refuse(bytes, &mut offset)?),
        (3, 2) => N2cHandshakeMessage::QueryReply(decode_table(bytes, &mut offset)?),
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

    fn params_uint(v: u64) -> N2cVersionParams {
        let mut buf = Vec::new();
        encode_u64(&mut buf, v);
        N2cVersionParams(buf)
    }

    fn sample_messages() -> Vec<N2cHandshakeMessage> {
        let table = N2cVersionTable(vec![
            (N2CVersion::new(15), params_uint(1)),
            (N2CVersion::new(16), params_uint(2)),
        ]);
        vec![
            N2cHandshakeMessage::ProposeVersions(table.clone()),
            N2cHandshakeMessage::AcceptVersion(N2CVersion::new(16), params_uint(3)),
            N2cHandshakeMessage::Refuse(N2cRefuseReason::VersionMismatch(vec![
                N2CVersion::new(14),
                N2CVersion::new(15),
            ])),
            N2cHandshakeMessage::Refuse(N2cRefuseReason::HandshakeDecodeError {
                version: N2CVersion::new(15),
                reason: "bad params".to_string(),
            }),
            N2cHandshakeMessage::Refuse(N2cRefuseReason::Refused {
                version: N2CVersion::new(16),
                reason: "policy".to_string(),
            }),
            N2cHandshakeMessage::QueryReply(table),
        ]
    }

    #[test]
    fn roundtrip_every_variant() {
        for msg in sample_messages() {
            let bytes = encode_n2c_handshake_message(&msg);
            let decoded = decode_n2c_handshake_message(&bytes).expect("decode");
            assert_eq!(decoded, msg);
            assert_eq!(encode_n2c_handshake_message(&decoded), bytes);
        }
    }

    #[test]
    fn decode_rejects_unknown_tag() {
        let bytes = vec![0x82, 0x18, 0x42, 0x00];
        match decode_n2c_handshake_message(&bytes) {
            Err(CodecError::UnknownTag { protocol: ProtocolKind::N2cHandshake, tag: 0x42 }) => {}
            other => panic!("expected UnknownTag, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_truncated_input() {
        let full = encode_n2c_handshake_message(&N2cHandshakeMessage::AcceptVersion(
            N2CVersion::new(16),
            params_uint(3),
        ));
        for n in 0..full.len() {
            let slice = &full[..n];
            let err = decode_n2c_handshake_message(slice).expect_err("must reject truncated");
            match err {
                CodecError::Truncated { .. }
                | CodecError::MalformedCbor { .. }
                | CodecError::InvalidProtocolMessage { .. } => {}
                other => panic!("expected truncation-class error, got {other:?}"),
            }
        }
    }

    #[test]
    fn decode_rejects_invalid_utf8_in_text_fields() {
        let mut buf = Vec::new();
        encode_array_header(&mut buf, 2);
        encode_u64(&mut buf, 2);
        encode_array_header(&mut buf, 3);
        encode_u64(&mut buf, 2);
        encode_u64(&mut buf, 16);
        buf.push(0x62);
        buf.push(0xff);
        buf.push(0xfe);
        match decode_n2c_handshake_message(&buf) {
            Err(CodecError::InvalidUtf8 { protocol: ProtocolKind::N2cHandshake, .. }) => {}
            other => panic!("expected InvalidUtf8, got {other:?}"),
        }
    }
}
