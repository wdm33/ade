// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED Ade-native KES envelope codec (PHASE4-N-O S1).
//!
//! The closed on-disk format for the hot-signing KES seed. Replaces
//! cardano-cli's `KesSigningKey_ed25519_kes_2^6` envelope as the sole
//! accepted Ade KES skey shape. The cardano-cli expanded `Sum6KES`
//! deserialization path is deferred to PHASE4-N-P.
//!
//! Grammar (closed):
//!
//! ```json
//! {
//!   "format":         "ade.kes.seed.v1",
//!   "role":           "kes_hot_signing_key",
//!   "crypto":         "Sum6KES-Ed25519DSIGN",
//!   "seed_32":        "<64 lowercase hex chars>",
//!   "period_idx":     <integer 0..=63>,
//!   "format_version": 1
//! }
//! ```
//!
//! Optional metadata fields (`genesis_hash`, `network_magic`,
//! `created_at_slot`, `created_by`) may appear and are silently ignored
//! by N-O; they do not change signing semantics. Any *unknown
//! load-bearing field* — i.e., any field outside the documented set —
//! is rejected via `#[serde(deny_unknown_fields)]`.
//!
//! This module is pure: no file I/O, no clock, no entropy. Inputs are
//! byte slices, outputs are byte vectors and error variants.

use serde::{Deserialize, Serialize};

use ade_crypto::kes::SUM6_MAX_PERIOD;

// =========================================================================
// Closed constants — the envelope grammar.
// =========================================================================

pub const ADE_KES_ENVELOPE_FORMAT: &str = "ade.kes.seed.v1";
pub const ADE_KES_ROLE: &str = "kes_hot_signing_key";
pub const ADE_KES_CRYPTO: &str = "Sum6KES-Ed25519DSIGN";
pub const ADE_KES_FORMAT_VERSION: u32 = 1;

// =========================================================================
// In-memory envelope (load-bearing fields only).
// =========================================================================

#[derive(Debug, PartialEq, Eq)]
pub struct AdeKesEnvelope {
    pub seed_32: [u8; 32],
    pub period_idx: u32,
}

// =========================================================================
// Closed error surface.
// =========================================================================

#[derive(Debug, PartialEq, Eq)]
pub enum AdeKesEnvelopeError {
    UnknownEnvelopeFormat,
    WrongKeyRole,
    UnsupportedCryptoTag,
    MissingSeed32,
    MalformedSeed32 { detail: &'static str },
    MalformedPeriodIdx,
    PeriodIdxOutOfRange,
    UnsupportedFormatVersion,
    MalformedJson { detail: &'static str },
}

// =========================================================================
// On-disk shapes (serde-only). Keep these separate from the in-memory
// type so the public API never leaks serde details, and so seed_32
// stays a hex string at the JSON boundary.
// =========================================================================

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvelopeOnDisk {
    format: String,
    role: String,
    crypto: String,
    seed_32: String,
    period_idx: u32,
    format_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    genesis_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    network_magic: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    created_at_slot: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    created_by: Option<String>,
}

// =========================================================================
// Parser
// =========================================================================

/// Parse an Ade KES envelope from raw bytes (JSON).
///
/// Returns the in-memory `AdeKesEnvelope` on success. Every shape that is
/// not the exact closed grammar maps to a structured `AdeKesEnvelopeError`
/// variant; the function never returns `Ok` for an unrecognized shape.
pub fn parse(buf: &[u8]) -> Result<AdeKesEnvelope, AdeKesEnvelopeError> {
    let env: EnvelopeOnDisk = serde_json::from_slice(buf).map_err(|e| {
        if e.classify() == serde_json::error::Category::Data {
            classify_serde_data_error(&e)
        } else {
            AdeKesEnvelopeError::MalformedJson {
                detail: "JSON parse failure",
            }
        }
    })?;

    if env.format != ADE_KES_ENVELOPE_FORMAT {
        return Err(AdeKesEnvelopeError::UnknownEnvelopeFormat);
    }
    if env.role != ADE_KES_ROLE {
        return Err(AdeKesEnvelopeError::WrongKeyRole);
    }
    if env.crypto != ADE_KES_CRYPTO {
        return Err(AdeKesEnvelopeError::UnsupportedCryptoTag);
    }
    if env.format_version != ADE_KES_FORMAT_VERSION {
        return Err(AdeKesEnvelopeError::UnsupportedFormatVersion);
    }
    if env.period_idx > SUM6_MAX_PERIOD {
        return Err(AdeKesEnvelopeError::PeriodIdxOutOfRange);
    }

    let seed_32 = decode_seed_hex(&env.seed_32)?;
    Ok(AdeKesEnvelope {
        seed_32,
        period_idx: env.period_idx,
    })
}

/// Classify a serde_json data error. `EnvelopeOnDisk` is opaque at the
/// serde layer, but the error message text follows a predictable pattern
/// for missing fields. We surface the load-bearing missing-field cases
/// as closed variants and fall back to `MalformedJson` for everything
/// else. The classification is conservative: any non-`seed_32` /
/// non-`period_idx` shape ends up as `MalformedJson` rather than a
/// guessed wrong-role / wrong-crypto label.
fn classify_serde_data_error(e: &serde_json::Error) -> AdeKesEnvelopeError {
    let msg = e.to_string();
    if msg.contains("missing field `seed_32`") {
        AdeKesEnvelopeError::MissingSeed32
    } else if msg.contains("missing field `period_idx`")
        || msg.contains("invalid type")
            && msg.contains("for key `period_idx`")
        || msg.contains("invalid value")
            && msg.contains("for key `period_idx`")
    {
        AdeKesEnvelopeError::MalformedPeriodIdx
    } else if msg.contains("unknown field") {
        AdeKesEnvelopeError::MalformedJson {
            detail: "unknown load-bearing field",
        }
    } else if msg.contains("invalid type") || msg.contains("invalid value") {
        // Catches non-integer period_idx, non-string crypto, etc.
        if msg.contains("period_idx") {
            AdeKesEnvelopeError::MalformedPeriodIdx
        } else {
            AdeKesEnvelopeError::MalformedJson {
                detail: "field type mismatch",
            }
        }
    } else {
        AdeKesEnvelopeError::MalformedJson {
            detail: "structural JSON error",
        }
    }
}

fn decode_seed_hex(s: &str) -> Result<[u8; 32], AdeKesEnvelopeError> {
    if s.len() != 64 {
        return Err(AdeKesEnvelopeError::MalformedSeed32 {
            detail: "seed_32 must be exactly 64 lowercase hex chars",
        });
    }
    let mut out = [0u8; 32];
    let bytes = s.as_bytes();
    for (i, pair) in bytes.chunks(2).enumerate() {
        let hi = decode_lowercase_nibble(pair[0])?;
        let lo = decode_lowercase_nibble(pair[1])?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

fn decode_lowercase_nibble(c: u8) -> Result<u8, AdeKesEnvelopeError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        _ => Err(AdeKesEnvelopeError::MalformedSeed32 {
            detail: "seed_32 must contain only lowercase hex characters",
        }),
    }
}

// =========================================================================
// Serializer
// =========================================================================

/// Serialize an `AdeKesEnvelope` to JSON bytes with the canonical key
/// order (verbatim from the operator spec): `format`, `role`, `crypto`,
/// `seed_32`, `period_idx`, `format_version`. No optional metadata
/// emitted by this serializer — N-O does not mechanically enforce
/// metadata semantics; emitting it would be an extension surface.
pub fn serialize(env: &AdeKesEnvelope) -> Vec<u8> {
    let mut out = String::with_capacity(256);
    out.push_str("{\n");
    out.push_str("  \"format\": \"");
    out.push_str(ADE_KES_ENVELOPE_FORMAT);
    out.push_str("\",\n");
    out.push_str("  \"role\": \"");
    out.push_str(ADE_KES_ROLE);
    out.push_str("\",\n");
    out.push_str("  \"crypto\": \"");
    out.push_str(ADE_KES_CRYPTO);
    out.push_str("\",\n");
    out.push_str("  \"seed_32\": \"");
    out.push_str(&hex_encode_lowercase(&env.seed_32));
    out.push_str("\",\n");
    out.push_str("  \"period_idx\": ");
    out.push_str(&env.period_idx.to_string());
    out.push_str(",\n");
    out.push_str("  \"format_version\": ");
    out.push_str(&ADE_KES_FORMAT_VERSION.to_string());
    out.push_str("\n}\n");
    out.into_bytes()
}

fn hex_encode_lowercase(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn canonical_envelope() -> AdeKesEnvelope {
        AdeKesEnvelope {
            seed_32: [0x42; 32],
            period_idx: 0,
        }
    }

    #[test]
    fn parse_round_trips_serialize() {
        let env = canonical_envelope();
        let bytes = serialize(&env);
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed, env);
    }

    #[test]
    fn parse_round_trips_at_nonzero_period() {
        let env = AdeKesEnvelope {
            seed_32: [0x07; 32],
            period_idx: 17,
        };
        let bytes = serialize(&env);
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed, env);
    }

    fn synth_envelope(
        format: &str,
        role: &str,
        crypto: &str,
        seed_32_hex: &str,
        period_idx: &str,
        format_version: &str,
        extra_field: Option<(&str, &str)>,
    ) -> Vec<u8> {
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str(&format!("  \"format\": \"{}\",\n", format));
        s.push_str(&format!("  \"role\": \"{}\",\n", role));
        s.push_str(&format!("  \"crypto\": \"{}\",\n", crypto));
        s.push_str(&format!("  \"seed_32\": \"{}\",\n", seed_32_hex));
        s.push_str(&format!("  \"period_idx\": {},\n", period_idx));
        if let Some((k, v)) = extra_field {
            s.push_str(&format!("  \"{}\": {},\n", k, v));
        }
        s.push_str(&format!("  \"format_version\": {}\n", format_version));
        s.push_str("}\n");
        s.into_bytes()
    }

    const VALID_HEX: &str = "4242424242424242424242424242424242424242424242424242424242424242";

    #[test]
    fn parse_rejects_unknown_format() {
        let buf = synth_envelope(
            "other.format.v1",
            ADE_KES_ROLE,
            ADE_KES_CRYPTO,
            VALID_HEX,
            "0",
            "1",
            None,
        );
        assert_eq!(parse(&buf), Err(AdeKesEnvelopeError::UnknownEnvelopeFormat));
    }

    #[test]
    fn parse_rejects_wrong_role() {
        let buf = synth_envelope(
            ADE_KES_ENVELOPE_FORMAT,
            "vrf_signing_key",
            ADE_KES_CRYPTO,
            VALID_HEX,
            "0",
            "1",
            None,
        );
        assert_eq!(parse(&buf), Err(AdeKesEnvelopeError::WrongKeyRole));
    }

    #[test]
    fn parse_rejects_unsupported_crypto() {
        let buf = synth_envelope(
            ADE_KES_ENVELOPE_FORMAT,
            ADE_KES_ROLE,
            "Sum7KES-Ed25519DSIGN",
            VALID_HEX,
            "0",
            "1",
            None,
        );
        assert_eq!(parse(&buf), Err(AdeKesEnvelopeError::UnsupportedCryptoTag));
    }

    #[test]
    fn parse_rejects_unsupported_format_version() {
        let buf = synth_envelope(
            ADE_KES_ENVELOPE_FORMAT,
            ADE_KES_ROLE,
            ADE_KES_CRYPTO,
            VALID_HEX,
            "0",
            "2",
            None,
        );
        assert_eq!(
            parse(&buf),
            Err(AdeKesEnvelopeError::UnsupportedFormatVersion)
        );
    }

    #[test]
    fn parse_rejects_missing_seed_32() {
        let buf = b"{\"format\":\"ade.kes.seed.v1\",\"role\":\"kes_hot_signing_key\",\"crypto\":\"Sum6KES-Ed25519DSIGN\",\"period_idx\":0,\"format_version\":1}".to_vec();
        assert_eq!(parse(&buf), Err(AdeKesEnvelopeError::MissingSeed32));
    }

    #[test]
    fn parse_rejects_malformed_seed_32_length() {
        let short_hex = "42424242";
        let buf = synth_envelope(
            ADE_KES_ENVELOPE_FORMAT,
            ADE_KES_ROLE,
            ADE_KES_CRYPTO,
            short_hex,
            "0",
            "1",
            None,
        );
        match parse(&buf) {
            Err(AdeKesEnvelopeError::MalformedSeed32 { .. }) => (),
            other => panic!("expected MalformedSeed32, got {:?}", other),
        }
    }

    #[test]
    fn parse_rejects_uppercase_seed_hex() {
        // The grammar requires lowercase hex; uppercase A-F is rejected
        // to keep the on-disk form unambiguous.
        let upper_hex = "4242424242424242424242424242424242424242424242424242424242424242"
            .replace("4", "F");
        let buf = synth_envelope(
            ADE_KES_ENVELOPE_FORMAT,
            ADE_KES_ROLE,
            ADE_KES_CRYPTO,
            &upper_hex,
            "0",
            "1",
            None,
        );
        match parse(&buf) {
            Err(AdeKesEnvelopeError::MalformedSeed32 { .. }) => (),
            other => panic!("expected MalformedSeed32, got {:?}", other),
        }
    }

    #[test]
    fn parse_rejects_period_idx_overflow() {
        let buf = synth_envelope(
            ADE_KES_ENVELOPE_FORMAT,
            ADE_KES_ROLE,
            ADE_KES_CRYPTO,
            VALID_HEX,
            "65",
            "1",
            None,
        );
        assert_eq!(parse(&buf), Err(AdeKesEnvelopeError::PeriodIdxOutOfRange));
    }

    #[test]
    fn parse_accepts_period_idx_boundary() {
        let buf = synth_envelope(
            ADE_KES_ENVELOPE_FORMAT,
            ADE_KES_ROLE,
            ADE_KES_CRYPTO,
            VALID_HEX,
            "63",
            "1",
            None,
        );
        let env = parse(&buf).unwrap();
        assert_eq!(env.period_idx, 63);
    }

    #[test]
    fn parse_rejects_unknown_load_bearing_field() {
        let buf = synth_envelope(
            ADE_KES_ENVELOPE_FORMAT,
            ADE_KES_ROLE,
            ADE_KES_CRYPTO,
            VALID_HEX,
            "0",
            "1",
            Some(("seed_42", "\"abc\"")),
        );
        match parse(&buf) {
            Err(AdeKesEnvelopeError::MalformedJson { .. }) => (),
            other => panic!("expected MalformedJson (unknown field), got {:?}", other),
        }
    }

    #[test]
    fn parse_accepts_optional_metadata_fields() {
        // genesis_hash / network_magic / created_at_slot / created_by are
        // documented optional metadata; the parser must accept them and
        // ignore them (load-bearing semantics unchanged).
        let json = format!(
            r#"{{
  "format": "{fmt}",
  "role": "{role}",
  "crypto": "{crypto}",
  "seed_32": "{seed}",
  "period_idx": 0,
  "format_version": 1,
  "genesis_hash": "deadbeef",
  "network_magic": 1,
  "created_at_slot": 123456,
  "created_by": "ade_node v0.1"
}}"#,
            fmt = ADE_KES_ENVELOPE_FORMAT,
            role = ADE_KES_ROLE,
            crypto = ADE_KES_CRYPTO,
            seed = VALID_HEX
        );
        let env = parse(json.as_bytes()).unwrap();
        assert_eq!(env.seed_32, [0x42; 32]);
        assert_eq!(env.period_idx, 0);
    }

    #[test]
    fn parse_rejects_malformed_json() {
        let buf = b"{ not json at all }".to_vec();
        match parse(&buf) {
            Err(AdeKesEnvelopeError::MalformedJson { .. }) => (),
            other => panic!("expected MalformedJson, got {:?}", other),
        }
    }

    #[test]
    fn serialize_does_not_leak_extra_fields() {
        let env = canonical_envelope();
        let bytes = serialize(&env);
        let s = std::str::from_utf8(&bytes).unwrap();
        // The serialized form contains only the load-bearing keys.
        assert!(s.contains("\"format\":"));
        assert!(s.contains("\"role\":"));
        assert!(s.contains("\"crypto\":"));
        assert!(s.contains("\"seed_32\":"));
        assert!(s.contains("\"period_idx\":"));
        assert!(s.contains("\"format_version\":"));
        assert!(!s.contains("genesis_hash"));
        assert!(!s.contains("network_magic"));
    }

    #[test]
    fn envelope_error_carries_no_seed_bytes() {
        // Construct each rejecting variant against an envelope carrying
        // a known seed pattern; ensure the formatted error does not
        // include any seed bytes (hex or decimal).
        let seed_hex = "9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c";
        let cases = [
            (
                synth_envelope(
                    "wrong.format",
                    ADE_KES_ROLE,
                    ADE_KES_CRYPTO,
                    seed_hex,
                    "0",
                    "1",
                    None,
                ),
                "UnknownEnvelopeFormat",
            ),
            (
                synth_envelope(
                    ADE_KES_ENVELOPE_FORMAT,
                    "wrong_role",
                    ADE_KES_CRYPTO,
                    seed_hex,
                    "0",
                    "1",
                    None,
                ),
                "WrongKeyRole",
            ),
            (
                synth_envelope(
                    ADE_KES_ENVELOPE_FORMAT,
                    ADE_KES_ROLE,
                    "Sum7KES-Ed25519DSIGN",
                    seed_hex,
                    "0",
                    "1",
                    None,
                ),
                "UnsupportedCryptoTag",
            ),
        ];
        for (buf, label) in &cases {
            let err = parse(buf).unwrap_err();
            let formatted = format!("{:?}", err);
            assert!(!formatted.contains(seed_hex), "{}: seed leaked", label);
            assert!(
                !formatted.contains("156, 156, 156"),
                "{}: decimal seed leaked",
                label
            );
        }
    }
}
