// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED cardano-cli opcert envelope parser (PHASE4-N-R-C C1).
//!
//! Parses a `NodeOperationalCertificate` text envelope (the
//! JSON wrapper around a CBOR payload that `cardano-cli node
//! issue-op-cert` emits) and produces a canonical
//! `OperationalCert`.
//!
//! **Envelope shape** (locked by N-R-A A1 OQ4 fixture capture
//! against cardano-cli 11.0.0.0 / cardano-node 11.0.1):
//!
//! ```json
//! {
//!   "type": "NodeOperationalCertificate",
//!   "description": "",
//!   "cborHex": "<hex of CBOR array(2)>"
//! }
//! ```
//!
//! The `cborHex` decodes to CBOR `array(2)`:
//! - Element 0: `array(4)` of
//!   `[hot_vkey(bytes(32)), sequence_number(uint),
//!     kes_period(uint), sigma(bytes(64))]`.
//! - Element 1: `bytes(32)` — cold verification key.
//!
//! The N-Q `OperationalCert` struct maps to element 0. C1's
//! parser extracts element 0 and discards element 1 (cold VK
//! is verified externally if needed).
//!
//! Doctrine: see [[feedback-fail-closed-validation]] —
//! every shape mismatch returns a structured error; no
//! permissive fallback.

use ade_codec::cbor::{read_array_header, read_bytes, read_uint, ContainerEncoding};
use ade_types::shelley::block::OperationalCert;

/// Closed parser error surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpCertParseError {
    /// JSON envelope structurally invalid (missing fields,
    /// wrong types, non-JSON input).
    JsonShape,
    /// Envelope `type` field is not `NodeOperationalCertificate`.
    WrongEnvelopeType,
    /// `cborHex` field contains non-hex characters or is
    /// odd-length.
    MalformedCborHex,
    /// CBOR-decode failure inside the cborHex payload (wrong
    /// outer arity, wrong inner arity, wrong field types,
    /// truncated input).
    MalformedCbor,
    /// `hot_vkey` field is not 32 bytes.
    HotVkeyWrongLength { found: usize },
    /// `sigma` field is not 64 bytes.
    SigmaWrongLength { found: usize },
    /// `cold_vk` field (envelope element 1) is not 32 bytes.
    ColdVkWrongLength { found: usize },
}

/// Successfully decoded envelope. Carries both element 0 (the
/// canonical opcert) and element 1 (the cold VK) so callers
/// can optionally cross-check the sigma signature against the
/// named cold key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedOpCertEnvelope {
    pub opcert: OperationalCert,
    pub cold_vk: [u8; 32],
}

/// Parse a cardano-cli opcert envelope from raw JSON bytes.
///
/// Closed pipeline:
/// 1. JSON-decode + extract `type`, `cborHex` fields.
/// 2. Envelope-type check: must equal `NodeOperationalCertificate`.
/// 3. Hex-decode `cborHex` (lowercase or uppercase accepted).
/// 4. CBOR array(2) header.
/// 5. Element 0 — array(4) of
///    `[bytes(32), uint, uint, bytes(64)]`.
/// 6. Element 1 — bytes(32) cold VK.
/// 7. Construct `OperationalCert` from element 0.
pub fn parse_opcert_envelope(json_bytes: &[u8]) -> Result<DecodedOpCertEnvelope, OpCertParseError> {
    // Step 1-2: JSON envelope shape + type check.
    let json: serde_json::Value =
        serde_json::from_slice(json_bytes).map_err(|_| OpCertParseError::JsonShape)?;
    let obj = json.as_object().ok_or(OpCertParseError::JsonShape)?;
    let ty = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or(OpCertParseError::JsonShape)?;
    if ty != "NodeOperationalCertificate" {
        return Err(OpCertParseError::WrongEnvelopeType);
    }
    let cbor_hex = obj
        .get("cborHex")
        .and_then(|v| v.as_str())
        .ok_or(OpCertParseError::JsonShape)?;

    // Step 3: hex-decode.
    let cbor_bytes = decode_hex(cbor_hex).map_err(|_| OpCertParseError::MalformedCborHex)?;

    // Step 4-6: CBOR decode.
    let mut offset = 0;
    let outer = read_array_header(&cbor_bytes, &mut offset)
        .map_err(|_| OpCertParseError::MalformedCbor)?;
    match outer {
        ContainerEncoding::Definite(2, _) => {}
        _ => return Err(OpCertParseError::MalformedCbor),
    }

    let inner = read_array_header(&cbor_bytes, &mut offset)
        .map_err(|_| OpCertParseError::MalformedCbor)?;
    match inner {
        ContainerEncoding::Definite(4, _) => {}
        _ => return Err(OpCertParseError::MalformedCbor),
    }

    let (hot_vkey, _) = read_bytes(&cbor_bytes, &mut offset)
        .map_err(|_| OpCertParseError::MalformedCbor)?;
    if hot_vkey.len() != 32 {
        return Err(OpCertParseError::HotVkeyWrongLength {
            found: hot_vkey.len(),
        });
    }
    let (sequence_number, _) = read_uint(&cbor_bytes, &mut offset)
        .map_err(|_| OpCertParseError::MalformedCbor)?;
    let (kes_period, _) = read_uint(&cbor_bytes, &mut offset)
        .map_err(|_| OpCertParseError::MalformedCbor)?;
    let (sigma, _) = read_bytes(&cbor_bytes, &mut offset)
        .map_err(|_| OpCertParseError::MalformedCbor)?;
    if sigma.len() != 64 {
        return Err(OpCertParseError::SigmaWrongLength { found: sigma.len() });
    }

    let (cold_vk_bytes, _) = read_bytes(&cbor_bytes, &mut offset)
        .map_err(|_| OpCertParseError::MalformedCbor)?;
    if cold_vk_bytes.len() != 32 {
        return Err(OpCertParseError::ColdVkWrongLength {
            found: cold_vk_bytes.len(),
        });
    }
    let mut cold_vk = [0u8; 32];
    cold_vk.copy_from_slice(&cold_vk_bytes);

    Ok(DecodedOpCertEnvelope {
        opcert: OperationalCert {
            hot_vkey,
            sequence_number,
            kes_period,
            sigma,
        },
        cold_vk,
    })
}

fn decode_hex(s: &str) -> Result<Vec<u8>, ()> {
    if s.len() % 2 != 0 {
        return Err(());
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_nibble(bytes[i])?;
        let lo = hex_nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn hex_nibble(c: u8) -> Result<u8, ()> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(()),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    const FIXTURE_DIR: &str = "tests/fixtures/opcert";

    fn fixture_bytes(name: &str) -> Vec<u8> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURE_DIR)
            .join(name);
        std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {:?}", path.display(), e))
    }

    #[test]
    fn accepted_envelope_decodes_to_expected_opcert() {
        let bytes = fixture_bytes("accepted-cardano-cli-11.0.0.opcert.json");
        let decoded = parse_opcert_envelope(&bytes).expect("accepted fixture parses");

        // Per OQ4 fixture metadata:
        // hot_vkey = 0c6ea1d8de23bf345996c6b26e0699f81a8e3fe79021b764ba3727c0eeb62314
        let expected_hot_vkey: Vec<u8> =
            decode_hex("0c6ea1d8de23bf345996c6b26e0699f81a8e3fe79021b764ba3727c0eeb62314").unwrap();
        assert_eq!(decoded.opcert.hot_vkey, expected_hot_vkey);
        assert_eq!(decoded.opcert.sequence_number, 0);
        assert_eq!(decoded.opcert.kes_period, 0);
        assert_eq!(decoded.opcert.sigma.len(), 64);
        // cold_vk = 180537b7910f1dcb35bed2bcbc2d374f0f8a68f4f63cd0662afa38d3c4499d93
        let expected_cold_vk: Vec<u8> =
            decode_hex("180537b7910f1dcb35bed2bcbc2d374f0f8a68f4f63cd0662afa38d3c4499d93").unwrap();
        assert_eq!(&decoded.cold_vk[..], &expected_cold_vk[..]);
    }

    #[test]
    fn malformed_type_envelope_emits_wrong_envelope_type() {
        let bytes = fixture_bytes("malformed-type.opcert.json");
        let err = parse_opcert_envelope(&bytes).unwrap_err();
        assert_eq!(err, OpCertParseError::WrongEnvelopeType);
    }

    #[test]
    fn malformed_cbor_hex_envelope_emits_malformed_cbor_hex() {
        let bytes = fixture_bytes("malformed-cborhex.opcert.json");
        let err = parse_opcert_envelope(&bytes).unwrap_err();
        assert_eq!(err, OpCertParseError::MalformedCborHex);
    }

    #[test]
    fn wrong_arity_envelope_emits_malformed_cbor() {
        let bytes = fixture_bytes("wrong-arity.opcert.json");
        let err = parse_opcert_envelope(&bytes).unwrap_err();
        assert_eq!(err, OpCertParseError::MalformedCbor);
    }

    #[test]
    fn parser_is_byte_identical_across_two_runs() {
        let bytes = fixture_bytes("accepted-cardano-cli-11.0.0.opcert.json");
        let a = parse_opcert_envelope(&bytes).unwrap();
        let b = parse_opcert_envelope(&bytes).unwrap();
        assert_eq!(a, b);
    }
}
