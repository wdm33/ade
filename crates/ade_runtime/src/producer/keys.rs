// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED disk-side key loader for cardano-cli text-envelope `*.skey`
//! files (PHASE4-N-C S1).
//!
//! Parses the JSON envelope (`type`, `description`, `cborHex`), decodes
//! the CBOR byte string, validates the `type` field against the
//! expected algorithm string, and constructs the matching RED secret
//! wrapper. Path strings never appear in `KeyLoadError`; the surface
//! returns `std::io::ErrorKind` rather than the path so filesystem
//! layout cannot leak into logs.
//!
//! Synthetic test fixtures store the *seed* (32 bytes for KES + cold,
//! 64 bytes for VRF libsodium-secret form) inside the CBOR byte string.
//! Genuine cardano-cli `.skey` interop with the full Sum6KES signing-key
//! serialization is OQ-1 future work and not part of this slice's
//! mechanical scope; the synthetic round-trip proves the envelope +
//! cborHex + type-validation pathway end-to-end.

use std::io::Read;
use std::path::Path;

use super::signing::{ColdSigningKey, KesSecret, SigningError, VrfSigningKey};

// =========================================================================
// Envelope type-string constants (mirror cardano-crypto::key::text_envelope).
// Hardcoded to avoid pulling the optional `key` feature into ade_crypto /
// ade_runtime (preserves the feature-set hard prohibition for S1).
// =========================================================================

pub const VRF_SIGNING_KEY_TYPE: &str = "VrfSigningKey_PraosVRF";
pub const KES_SIGNING_KEY_TYPE: &str = "KesSigningKey_ed25519_kes_2^6";
pub const POOL_SIGNING_KEY_TYPE: &str = "StakePoolSigningKey_ed25519";

// =========================================================================
// Loaders
// =========================================================================

/// Load a VRF signing key from a cardano-cli text-envelope `.skey` file.
pub fn load_vrf_signing_key_skey(path: &Path) -> Result<VrfSigningKey, KeyLoadError> {
    let payload = read_envelope_payload(path, VRF_SIGNING_KEY_TYPE)?;
    VrfSigningKey::from_bytes_zeroizing(&payload).map_err(KeyLoadError::Crypto)
}

/// Load a Sum6KES signing key from a cardano-cli text-envelope `.skey`
/// file. Returns a `KesSecret` initialized at period 0 with the full
/// `SUM6_MAX_PERIOD` evolutions remaining.
pub fn load_kes_signing_key_skey(path: &Path) -> Result<KesSecret, KeyLoadError> {
    let payload = read_envelope_payload(path, KES_SIGNING_KEY_TYPE)?;
    KesSecret::from_bytes_zeroizing(&payload).map_err(KeyLoadError::Crypto)
}

/// Load an Ed25519 cold signing key from a cardano-cli text-envelope
/// `.skey` file. The envelope `type` must be `StakePoolSigningKey_ed25519`.
pub fn load_cold_signing_key_skey(path: &Path) -> Result<ColdSigningKey, KeyLoadError> {
    let payload = read_envelope_payload(path, POOL_SIGNING_KEY_TYPE)?;
    ColdSigningKey::from_bytes_zeroizing(&payload).map_err(KeyLoadError::Crypto)
}

// =========================================================================
// KeyLoadError — no path strings, no key bytes.
// =========================================================================

/// Closed loader error surface. Carries only `std::io::ErrorKind`
/// (never the path), static `&'static str` detail strings, and
/// envelope-type strings — never the raw key bytes.
#[derive(Debug)]
pub enum KeyLoadError {
    /// Filesystem error. `ErrorKind` only — the path is intentionally
    /// not stored, so log lines cannot leak filesystem layout.
    Io(std::io::ErrorKind),
    /// Envelope JSON did not match the expected shape (missing or
    /// extra fields, malformed strings, etc.). Detail is a static
    /// literal.
    MalformedEnvelope { detail: &'static str },
    /// Envelope `type` field did not match the expected algorithm
    /// string. `expected` is the static algorithm constant, `found` is
    /// the bytes from the envelope.
    UnexpectedType {
        expected: &'static str,
        found: String,
    },
    /// `cborHex` field failed hex / CBOR-bytestring decoding.
    CborHexDecode { detail: &'static str },
    /// Underlying RED crypto wrapper rejected the decoded bytes
    /// (wrong length, malformed seed, etc.).
    Crypto(SigningError),
}

// =========================================================================
// Envelope reader
// =========================================================================

fn read_envelope_payload(
    path: &Path,
    expected_type: &'static str,
) -> Result<Vec<u8>, KeyLoadError> {
    let mut file = std::fs::File::open(path).map_err(|e| KeyLoadError::Io(e.kind()))?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)
        .map_err(|e| KeyLoadError::Io(e.kind()))?;

    let env: TextEnvelope =
        serde_json::from_str(&buf).map_err(|_| KeyLoadError::MalformedEnvelope {
            detail: "JSON parse failure",
        })?;

    if env.type_field != expected_type {
        return Err(KeyLoadError::UnexpectedType {
            expected: expected_type,
            found: env.type_field,
        });
    }

    // Hex-decode the cborHex field, then strip the leading CBOR
    // byte-string header. A single CBOR byte-string is encoded as
    // major-type 2 (0b010 | additional-info length-encoding).
    let raw = hex_decode(&env.cbor_hex).map_err(|d| KeyLoadError::CborHexDecode { detail: d })?;
    decode_cbor_byte_string(&raw).map_err(|d| KeyLoadError::CborHexDecode { detail: d })
}

#[derive(serde::Deserialize)]
struct TextEnvelope {
    #[serde(rename = "type")]
    type_field: String,
    #[serde(default, rename = "description")]
    _description: String,
    #[serde(rename = "cborHex")]
    cbor_hex: String,
}

// =========================================================================
// Hex + CBOR byte-string decoders (hand-rolled; no new deps in RED).
// =========================================================================

fn hex_decode(s: &str) -> Result<Vec<u8>, &'static str> {
    if s.len() % 2 != 0 {
        return Err("odd-length hex");
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

fn hex_nibble(c: u8) -> Result<u8, &'static str> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err("non-hex character"),
    }
}

/// Decode a single CBOR byte string. Returns the contents (excluding
/// the major-type-2 header). Accepts only definite-length encodings;
/// indefinite-length byte strings (major-type 2 with additional-info
/// 31) are rejected because cardano-cli never emits them for `.skey`
/// payloads.
fn decode_cbor_byte_string(buf: &[u8]) -> Result<Vec<u8>, &'static str> {
    if buf.is_empty() {
        return Err("empty CBOR payload");
    }
    let initial = buf[0];
    let major = initial >> 5;
    if major != 2 {
        return Err("CBOR major type is not byte string (expected 2)");
    }
    let ai = initial & 0x1F;
    let (len, header_len) = match ai {
        0..=23 => (ai as usize, 1),
        24 => {
            if buf.len() < 2 {
                return Err("truncated CBOR length (u8)");
            }
            (buf[1] as usize, 2)
        }
        25 => {
            if buf.len() < 3 {
                return Err("truncated CBOR length (u16)");
            }
            (u16::from_be_bytes([buf[1], buf[2]]) as usize, 3)
        }
        26 => {
            if buf.len() < 5 {
                return Err("truncated CBOR length (u32)");
            }
            (
                u32::from_be_bytes([buf[1], buf[2], buf[3], buf[4]]) as usize,
                5,
            )
        }
        27 => {
            if buf.len() < 9 {
                return Err("truncated CBOR length (u64)");
            }
            // Reject u64 lengths; key bytes never exceed 4 GiB and a
            // CBOR-encoded length of more than u32::MAX is per se a
            // malformed envelope.
            return Err("CBOR length width u64 not supported");
        }
        31 => return Err("indefinite-length byte strings not accepted"),
        _ => return Err("reserved CBOR additional-info"),
    };
    if buf.len() != header_len + len {
        return Err("CBOR byte-string length mismatch (trailing bytes or truncation)");
    }
    Ok(buf[header_len..].to_vec())
}

/// CBOR-encode a byte slice as a definite-length major-type-2 byte
/// string. Only the test fixture-builder uses this; production loaders
/// only *decode* envelopes.
#[cfg(test)]
fn encode_cbor_byte_string(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 5);
    let len = bytes.len();
    if len < 24 {
        out.push(0x40 | (len as u8));
    } else if len < 256 {
        out.push(0x58);
        out.push(len as u8);
    } else if len < 65536 {
        out.push(0x59);
        out.extend_from_slice(&(len as u16).to_be_bytes());
    } else if (len as u64) < u32::MAX as u64 {
        out.push(0x5A);
        out.extend_from_slice(&(len as u32).to_be_bytes());
    } else {
        // Unreachable for our key sizes.
        out.push(0x5B);
        out.extend_from_slice(&(len as u64).to_be_bytes());
    }
    out.extend_from_slice(bytes);
    out
}

#[cfg(test)]
fn hex_encode(bytes: &[u8]) -> String {
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
    use std::io::Write;

    fn write_envelope(
        dir: &std::path::Path,
        name: &str,
        type_str: &str,
        payload_bytes: &[u8],
    ) -> std::path::PathBuf {
        let path = dir.join(name);
        let cbor_hex = hex_encode(&encode_cbor_byte_string(payload_bytes));
        let json = format!(
            "{{\"type\":\"{}\",\"description\":\"S1 fixture\",\"cborHex\":\"{}\"}}",
            type_str, cbor_hex
        );
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(json.as_bytes()).unwrap();
        path
    }

    #[test]
    fn cardano_cli_skey_envelope_round_trips_through_keys_loader() {
        use ade_crypto::kes::{verify_kes_signature, KesPeriod, KesVerificationKey};
        use ade_crypto::vrf::{verify_vrf, VrfVerificationKey};
        use cardano_crypto::kes::{KesAlgorithm, Sum6Kes};
        use cardano_crypto::vrf::VrfDraft03;

        let dir = tempfile::tempdir().unwrap();

        // ---- VRF: full 64-byte libsodium secret in cborHex ----
        let vrf_seed = [0x42u8; 32];
        let (vrf_sk_raw, vrf_vk_raw) = VrfDraft03::keypair_from_seed(&vrf_seed);
        let vrf_path = write_envelope(dir.path(), "vrf.skey", VRF_SIGNING_KEY_TYPE, &vrf_sk_raw);
        let vrf_sk = load_vrf_signing_key_skey(&vrf_path).unwrap();
        let alpha = b"S1 round-trip alpha";
        let (proof, _output) = super::super::signing::vrf_prove(&vrf_sk, alpha).unwrap();
        verify_vrf(&VrfVerificationKey(vrf_vk_raw), &proof, alpha).unwrap();

        // ---- KES: 32-byte seed in cborHex ----
        let kes_seed = [0x42u8; 32];
        let kes_path = write_envelope(dir.path(), "kes.skey", KES_SIGNING_KEY_TYPE, &kes_seed);
        let kes_sk = load_kes_signing_key_skey(&kes_path).unwrap();
        let msg = b"S1 round-trip msg";
        let sig = super::super::signing::kes_sign(&kes_sk, KesPeriod(0), msg).unwrap();
        let raw = Sum6Kes::gen_key_kes_from_seed_bytes(&kes_seed).unwrap();
        let vk_bytes = Sum6Kes::raw_serialize_verification_key_kes(
            &Sum6Kes::derive_verification_key(&raw).unwrap(),
        );
        let mut vk_arr = [0u8; 32];
        vk_arr.copy_from_slice(&vk_bytes);
        verify_kes_signature(&KesVerificationKey(vk_arr), KesPeriod(0), msg, &sig).unwrap();

        // ---- Cold: 32-byte seed in cborHex ----
        let cold_seed = [0x42u8; 32];
        let cold_path = write_envelope(dir.path(), "cold.skey", POOL_SIGNING_KEY_TYPE, &cold_seed);
        let cold_sk = load_cold_signing_key_skey(&cold_path).unwrap();
        let vk_expected = {
            use ed25519_dalek::SigningKey;
            SigningKey::from_bytes(&cold_seed)
                .verifying_key()
                .to_bytes()
        };
        assert_eq!(cold_sk.derive_verification_key_bytes(), vk_expected);
    }

    #[test]
    fn keys_loader_rejects_wrong_envelope_type() {
        let dir = tempfile::tempdir().unwrap();
        // Build a KES envelope and ask the VRF loader to consume it.
        let kes_seed = [0x11u8; 32];
        let path = write_envelope(dir.path(), "wrong.skey", KES_SIGNING_KEY_TYPE, &kes_seed);
        let err = load_vrf_signing_key_skey(&path).unwrap_err();
        match err {
            KeyLoadError::UnexpectedType { expected, found } => {
                assert_eq!(expected, VRF_SIGNING_KEY_TYPE);
                assert_eq!(found, KES_SIGNING_KEY_TYPE);
            }
            other => panic!("expected UnexpectedType, got {:?}", other),
        }
    }

    #[test]
    fn keys_loader_rejects_malformed_cbor_hex() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.skey");
        // Garbled cborHex: odd-length hex, then non-hex characters.
        let json = format!(
            "{{\"type\":\"{}\",\"description\":\"bad\",\"cborHex\":\"5820zzzzz\"}}",
            VRF_SIGNING_KEY_TYPE
        );
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(json.as_bytes()).unwrap();
        }
        let err = load_vrf_signing_key_skey(&path).unwrap_err();
        assert!(matches!(err, KeyLoadError::CborHexDecode { .. }));
    }

    #[test]
    fn key_load_error_io_carries_no_path_bytes() {
        // Construct a path that contains a recognizable byte sequence
        // and confirm the error formatting omits it. The path is
        // intentionally unreachable: the loader's I/O error returns
        // only `ErrorKind`, never the path.
        let exotic_path = std::path::PathBuf::from("/tmp/__N_C_S1_DOES_NOT_EXIST_SECRET_42__");
        let err = load_vrf_signing_key_skey(&exotic_path).unwrap_err();
        let s = format!("{:?}", err);
        assert!(matches!(err, KeyLoadError::Io(_)));
        assert!(!s.contains("__N_C_S1_DOES_NOT_EXIST_SECRET_42__"));
        assert!(!s.contains("/tmp/"));
    }
}
