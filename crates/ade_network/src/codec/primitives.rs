// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Thin wrappers over `ade_codec::cbor_primitives` for the
// mini-protocol codec layer. Centralising the wrapping keeps direct
// `minicbor::`/raw-CBOR imports out of every protocol module — the
// ingress chokepoint script enforces this constraint. Every helper
// here adapts `ade_codec::CodecError` into a `CodecError` annotated
// with the protocol it belongs to.

use crate::codec::error::{CodecError, ProtocolKind};
use ade_codec::cbor_primitives as cbp;

// ---------------------------------------------------------------------------
// Decode helpers
// ---------------------------------------------------------------------------

/// Decode an array header and return the expected element count. Rejects
/// indefinite-length arrays — mini-protocol messages are encoded as
/// fixed-shape arrays with known element counts.
pub fn decode_array_header(
    protocol: ProtocolKind,
    data: &[u8],
    offset: &mut usize,
) -> Result<u64, CodecError> {
    let enc = cbp::read_array_header(data, offset)
        .map_err(|source| CodecError::MalformedCbor { protocol, source })?;
    match enc {
        cbp::ContainerEncoding::Definite(count, _) => Ok(count),
        cbp::ContainerEncoding::Indefinite => Err(CodecError::InvalidProtocolMessage {
            protocol,
            reason: "indefinite-length array not allowed",
        }),
    }
}

/// Decode an array header and require the exact element count.
pub fn decode_array_of_len(
    protocol: ProtocolKind,
    data: &[u8],
    offset: &mut usize,
    expected: u64,
) -> Result<(), CodecError> {
    let got = decode_array_header(protocol, data, offset)?;
    if got != expected {
        Err(CodecError::InvalidProtocolMessage {
            protocol,
            reason: "wrong array length",
        })
    } else {
        Ok(())
    }
}

/// Decode an unsigned integer as `u64`.
pub fn decode_u64(
    protocol: ProtocolKind,
    data: &[u8],
    offset: &mut usize,
) -> Result<u64, CodecError> {
    let (val, _) = cbp::read_uint(data, offset)
        .map_err(|source| CodecError::MalformedCbor { protocol, source })?;
    Ok(val)
}

/// Decode an unsigned integer as `u16`, range-checking against `u16::MAX`.
pub fn decode_u16(
    protocol: ProtocolKind,
    data: &[u8],
    offset: &mut usize,
    field: &'static str,
) -> Result<u16, CodecError> {
    let v = decode_u64(protocol, data, offset)?;
    if v > u16::MAX as u64 {
        Err(CodecError::InvalidIntegerRange { protocol, field, value: v })
    } else {
        Ok(v as u16)
    }
}

/// Decode an unsigned integer as `u32`, range-checking against `u32::MAX`.
pub fn decode_u32(
    protocol: ProtocolKind,
    data: &[u8],
    offset: &mut usize,
    field: &'static str,
) -> Result<u32, CodecError> {
    let v = decode_u64(protocol, data, offset)?;
    if v > u32::MAX as u64 {
        Err(CodecError::InvalidIntegerRange { protocol, field, value: v })
    } else {
        Ok(v as u32)
    }
}

/// Decode a CBOR byte string.
pub fn decode_bytes(
    protocol: ProtocolKind,
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<u8>, CodecError> {
    let (bytes, _) = cbp::read_bytes(data, offset)
        .map_err(|source| CodecError::MalformedCbor { protocol, source })?;
    Ok(bytes)
}

/// Decode a CBOR text string with explicit UTF-8 validation.
pub fn decode_text(
    protocol: ProtocolKind,
    data: &[u8],
    offset: &mut usize,
    field: &'static str,
) -> Result<String, CodecError> {
    match cbp::read_text(data, offset) {
        Ok((s, _)) => Ok(s),
        Err(ade_codec::CodecError::InvalidCborStructure {
            detail: "invalid UTF-8 in text string",
            ..
        }) => Err(CodecError::InvalidUtf8 { protocol, field }),
        Err(source) => Err(CodecError::MalformedCbor { protocol, source }),
    }
}

/// Decode a CBOR boolean.
pub fn decode_bool(
    protocol: ProtocolKind,
    data: &[u8],
    offset: &mut usize,
) -> Result<bool, CodecError> {
    cbp::read_bool(data, offset).map_err(|source| CodecError::MalformedCbor { protocol, source })
}

/// Assert that `data` is fully consumed up to `offset`.
pub fn require_consumed(
    protocol: ProtocolKind,
    data: &[u8],
    offset: usize,
) -> Result<(), CodecError> {
    if offset != data.len() {
        Err(CodecError::InvalidProtocolMessage {
            protocol,
            reason: "trailing bytes after message",
        })
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Encode helpers
// ---------------------------------------------------------------------------

/// Write a fixed-length array header with canonical width.
pub fn encode_array_header(buf: &mut Vec<u8>, len: u64) {
    cbp::write_array_header(
        buf,
        cbp::ContainerEncoding::Definite(len, cbp::canonical_width(len)),
    );
}

/// Write an unsigned integer using canonical (minimal) width.
pub fn encode_u64(buf: &mut Vec<u8>, value: u64) {
    cbp::write_uint_canonical(buf, value);
}

/// Write a CBOR byte string with canonical length encoding.
pub fn encode_bytes(buf: &mut Vec<u8>, bytes: &[u8]) {
    cbp::write_bytes_canonical(buf, bytes);
}

/// Write a CBOR text string with canonical length encoding.
pub fn encode_text(buf: &mut Vec<u8>, text: &str) {
    cbp::write_text_canonical(buf, text);
}

/// Write a CBOR boolean.
pub fn encode_bool(buf: &mut Vec<u8>, b: bool) {
    cbp::write_bool(buf, b);
}

/// Write a CBOR null.
pub fn encode_null(buf: &mut Vec<u8>) {
    cbp::write_null(buf);
}
