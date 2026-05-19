// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2N Handshake mini-protocol message codec (BLUE).
//
// Wire shape (per IOG ouroboros-network CDDL, simplified):
//   handshakeMessage =
//       [0, versionTable]      ; MsgProposeVersions
//     / [1, versionNumber, extraParams]   ; MsgAcceptVersion
//     / [2, refuseReason]      ; MsgRefuse
//     / [3, versionTable]      ; MsgQueryReply (handshake-query branch)
//
//   refuseReason =
//       [0, [versionNumber*]]        ; VersionMismatch
//     / [1, versionNumber, str]      ; HandshakeDecodeError
//     / [2, versionNumber, str]      ; Refused
//
//   versionTable = { versionNumber => versionData }
//
// `VersionData` is wire-version-specific. S-A2 carries it as opaque
// bytes (`VersionParams`) since interpretation belongs in the
// handshake state machine slice (S-A3). The codec verifies CBOR
// well-formedness around the params but does NOT interpret them.

use crate::codec::error::{CodecError, ProtocolKind};
use crate::codec::primitives::{
    decode_array_header, decode_text, decode_u32, decode_u64, encode_array_header, encode_text,
    encode_u64, require_consumed,
};
use crate::codec::version::N2NVersion;

const PROTOCOL: ProtocolKind = ProtocolKind::Handshake;

// Note: the version table is encoded as a CBOR map. Rust's stdlib HashMap is
// banned in BLUE (T-CORE-02); we use a sorted Vec<(N2NVersion, VersionParams)>
// keyed by version number to preserve canonical ordering at encode time and
// reject non-monotonic keys at decode time.

/// Opaque per-version parameters as they appear on the wire. The
/// codec carries the raw CBOR bytes; the handshake state machine
/// (S-A3) parses them according to the proposed version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionParams(pub Vec<u8>);

/// Closed version table — `Vec` of `(version, params)` sorted by version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionTable(pub Vec<(N2NVersion, VersionParams)>);

/// Refuse reason taxonomy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefuseReason {
    /// No version in common between proposed and supported tables.
    /// The vector is the set of versions the peer supports.
    VersionMismatch(Vec<N2NVersion>),
    /// The peer rejected the version-specific extra parameters.
    HandshakeDecodeError { version: N2NVersion, reason: String },
    /// The peer is alive but refuses the connection at the application level.
    Refused { version: N2NVersion, reason: String },
}

/// Closed N2N handshake message enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeMessage {
    /// Client proposes a version table.
    ProposeVersions(VersionTable),
    /// Server accepts a single version with its negotiated extra params.
    AcceptVersion(N2NVersion, VersionParams),
    /// Server refuses with a structured reason.
    Refuse(RefuseReason),
    /// Server-initiated `query` reply (handshake-query feature) carrying
    /// the table of versions the server supports.
    QueryReply(VersionTable),
}

// ---------------------------------------------------------------------------
// Encode
// ---------------------------------------------------------------------------

fn encode_version_table(buf: &mut Vec<u8>, table: &VersionTable) {
    // CBOR map with definite length.
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
        // params bytes are appended verbatim — they're already CBOR.
        buf.extend_from_slice(&params.0);
    }
}

fn encode_refuse_reason(buf: &mut Vec<u8>, reason: &RefuseReason) {
    match reason {
        RefuseReason::VersionMismatch(vs) => {
            encode_array_header(buf, 2);
            encode_u64(buf, 0);
            encode_array_header(buf, vs.len() as u64);
            for v in vs {
                encode_u64(buf, v.get() as u64);
            }
        }
        RefuseReason::HandshakeDecodeError { version, reason } => {
            encode_array_header(buf, 3);
            encode_u64(buf, 1);
            encode_u64(buf, version.get() as u64);
            encode_text(buf, reason);
        }
        RefuseReason::Refused { version, reason } => {
            encode_array_header(buf, 3);
            encode_u64(buf, 2);
            encode_u64(buf, version.get() as u64);
            encode_text(buf, reason);
        }
    }
}

pub fn encode_handshake_message(msg: &HandshakeMessage) -> Vec<u8> {
    let mut buf = Vec::new();
    match msg {
        HandshakeMessage::ProposeVersions(table) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 0);
            encode_version_table(&mut buf, table);
        }
        HandshakeMessage::AcceptVersion(ver, params) => {
            encode_array_header(&mut buf, 3);
            encode_u64(&mut buf, 1);
            encode_u64(&mut buf, ver.get() as u64);
            buf.extend_from_slice(&params.0);
        }
        HandshakeMessage::Refuse(reason) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 2);
            encode_refuse_reason(&mut buf, reason);
        }
        HandshakeMessage::QueryReply(table) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 3);
            encode_version_table(&mut buf, table);
        }
    }
    buf
}

// ---------------------------------------------------------------------------
// Decode
// ---------------------------------------------------------------------------

fn decode_version_table(data: &[u8], offset: &mut usize) -> Result<VersionTable, CodecError> {
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
        let params_start = *offset;
        // Skip one CBOR item (the version params) and capture its bytes.
        let (_, end) = ade_codec::cbor_primitives::skip_item(data, offset)
            .map_err(|source| CodecError::MalformedCbor { protocol: PROTOCOL, source })?;
        let params = data[params_start..end].to_vec();
        entries.push((N2NVersion::new(v as u16), VersionParams(params)));
    }
    Ok(VersionTable(entries))
}

fn decode_refuse_reason(data: &[u8], offset: &mut usize) -> Result<RefuseReason, CodecError> {
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
                vs.push(N2NVersion::new(v as u16));
            }
            Ok(RefuseReason::VersionMismatch(vs))
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
            Ok(RefuseReason::HandshakeDecodeError { version: N2NVersion::new(v as u16), reason })
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
            Ok(RefuseReason::Refused { version: N2NVersion::new(v as u16), reason })
        }
        (other, _) => Err(CodecError::UnknownTag { protocol: PROTOCOL, tag: other }),
    }
}

pub fn decode_handshake_message(bytes: &[u8]) -> Result<HandshakeMessage, CodecError> {
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
        (0, 2) => HandshakeMessage::ProposeVersions(decode_version_table(bytes, &mut offset)?),
        (1, 3) => {
            let v = decode_u32(PROTOCOL, bytes, &mut offset, "version number")?;
            if v > u16::MAX as u32 {
                return Err(CodecError::InvalidIntegerRange {
                    protocol: PROTOCOL,
                    field: "version number",
                    value: v as u64,
                });
            }
            let params_start = offset;
            let (_, end) = ade_codec::cbor_primitives::skip_item(bytes, &mut offset)
                .map_err(|source| CodecError::MalformedCbor { protocol: PROTOCOL, source })?;
            let params = bytes[params_start..end].to_vec();
            HandshakeMessage::AcceptVersion(N2NVersion::new(v as u16), VersionParams(params))
        }
        (2, 2) => HandshakeMessage::Refuse(decode_refuse_reason(bytes, &mut offset)?),
        (3, 2) => HandshakeMessage::QueryReply(decode_version_table(bytes, &mut offset)?),
        (other, _) => return Err(CodecError::UnknownTag { protocol: PROTOCOL, tag: other }),
    };
    require_consumed(PROTOCOL, bytes, offset)?;
    Ok(msg)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn params_uint(v: u64) -> VersionParams {
        let mut buf = Vec::new();
        encode_u64(&mut buf, v);
        VersionParams(buf)
    }

    fn sample_messages() -> Vec<HandshakeMessage> {
        let table = VersionTable(vec![
            (N2NVersion::new(11), params_uint(7)),
            (N2NVersion::new(12), params_uint(8)),
        ]);
        vec![
            HandshakeMessage::ProposeVersions(table.clone()),
            HandshakeMessage::AcceptVersion(N2NVersion::new(12), params_uint(9)),
            HandshakeMessage::Refuse(RefuseReason::VersionMismatch(vec![
                N2NVersion::new(10),
                N2NVersion::new(11),
            ])),
            HandshakeMessage::Refuse(RefuseReason::HandshakeDecodeError {
                version: N2NVersion::new(11),
                reason: "bad params".to_string(),
            }),
            HandshakeMessage::Refuse(RefuseReason::Refused {
                version: N2NVersion::new(12),
                reason: "go away".to_string(),
            }),
            HandshakeMessage::QueryReply(table),
        ]
    }

    #[test]
    fn roundtrip_every_variant() {
        for msg in sample_messages() {
            let bytes = encode_handshake_message(&msg);
            let decoded = decode_handshake_message(&bytes).expect("decode");
            assert_eq!(decoded, msg, "round-trip identity");
            let re = encode_handshake_message(&decoded);
            assert_eq!(re, bytes, "byte-identical re-encode");
        }
    }

    #[test]
    fn decode_rejects_unknown_tag() {
        // [99, 0] — outer tag 99 is not in {0,1,2,3}
        let bytes = vec![0x82, 0x18, 0x63, 0x00];
        match decode_handshake_message(&bytes) {
            Err(CodecError::UnknownTag { protocol: ProtocolKind::Handshake, tag: 99 }) => {}
            other => panic!("expected UnknownTag, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_truncated_input() {
        let full = encode_handshake_message(&HandshakeMessage::AcceptVersion(
            N2NVersion::new(12),
            params_uint(9),
        ));
        for n in 0..full.len() {
            let slice = &full[..n];
            let err = decode_handshake_message(slice).expect_err("must reject truncated");
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
        // Encode the inner shape of a Refused reason but with non-UTF-8 bytes
        // in the text field. Outer envelope: [2, [2, version, badtext]]
        let mut buf = Vec::new();
        encode_array_header(&mut buf, 2);
        encode_u64(&mut buf, 2);
        encode_array_header(&mut buf, 3);
        encode_u64(&mut buf, 2);
        encode_u64(&mut buf, 12);
        // raw CBOR text-string header for 2 bytes, then invalid UTF-8.
        buf.push(0x62);
        buf.push(0xff);
        buf.push(0xfe);
        match decode_handshake_message(&buf) {
            Err(CodecError::InvalidUtf8 { protocol: ProtocolKind::Handshake, .. }) => {}
            other => panic!("expected InvalidUtf8, got {other:?}"),
        }
    }
}
