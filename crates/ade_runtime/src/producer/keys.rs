// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED disk-side key loader for cardano-cli text-envelope `*.skey`
//! files (PHASE4-N-C S1) **and** the Ade-native KES envelope
//! `ade.kes.seed.v1` (PHASE4-N-O S1).
//!
//! Loaders parse the JSON envelope (`type`, `description`, `cborHex`
//! for cardano-cli; `format`, `role`, `crypto`, `seed_32`, `period_idx`,
//! `format_version` for Ade-native), decode the payload, and construct
//! the matching RED secret wrapper. Path strings never appear in
//! `KeyLoadError`; the surface returns `std::io::ErrorKind` rather than
//! the path so filesystem layout cannot leak into logs.
//!
//! ### KES envelope policy (PHASE4-N-O)
//!
//! The cardano-cli `KesSigningKey_ed25519_kes_2^6` envelope is recognized
//! only to be fail-closed via `KeyLoadError::UnsupportedExpandedKesKeyFormat`.
//! cardano-cli emits the 608-byte Sum6KES expanded-tree serialization, for
//! which `cardano-crypto` 1.0.8 exposes no public constructor; rehydrating
//! it requires a full `ade_crypto::kes_sum` reimplementation (deferred to
//! PHASE4-N-P). For the challenge build the operator generates Ade-native
//! envelopes via `ade_node key-gen-KES`; that envelope is the sole shape
//! accepted by `load_ade_kes_signing_key`.
//!
//! VRF and cold (Ed25519) loaders continue to consume the cardano-cli
//! text-envelope shape unchanged — neither is affected by the Sum6KES
//! expanded-format gap.

use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use super::ade_kes_envelope::{self, AdeKesEnvelope, AdeKesEnvelopeError};
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

/// Load a cardano-cli `KesSigningKey_ed25519_kes_2^6` envelope into an
/// Ade `KesSecret`.
///
/// PHASE4-N-P policy (replaces PHASE4-N-O's unconditional fail-close):
/// the 608-byte structurally-valid expanded `Sum6KES` payload is
/// accepted via the Ade-owned BLUE deserializer (`ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes`).
/// Any other payload shape — 32 bytes, 612 bytes, malformed sub-tree,
/// inconsistent vk hash, leaf-all-zero — fail-closes via the closed
/// `KeyLoadError` variants. No fallback parser; the deserializer is
/// the structural validator (see
/// `docs/clusters/PHASE4-N-P/period-from-zeroed-sum6-tree-shape-proof.md`
/// §5 for the closed `KesParseError` surface).
pub fn load_kes_signing_key_skey(path: &Path) -> Result<KesSecret, KeyLoadError> {
    use ade_crypto::kes_sum::{KesAlgorithm as AdeKesAlgorithm, Sum6Kes};

    let payload = read_envelope_payload(path, KES_SIGNING_KEY_TYPE)?;
    // Any payload size != 608 is fail-closed via UnsupportedExpandedKesKeyFormat;
    // the new BLUE deserializer would also reject these with
    // KesParseError::WrongPayloadSize, but emitting the narrower
    // variant preserves the N-O closed surface for the size-mismatch
    // case.
    if payload.len() != Sum6Kes::SIGNING_KEY_SIZE {
        return Err(KeyLoadError::UnsupportedExpandedKesKeyFormat);
    }
    let inner = Sum6Kes::raw_deserialize_signing_key_kes(&payload)
        .map_err(KeyLoadError::KesParse)?;
    let current_period = Sum6Kes::current_period_of_signing_key(&inner);
    Ok(KesSecret::from_blue_signing_key(
        inner,
        ade_crypto::kes::KesPeriod(current_period),
    ))
}

/// Load an Ade-native KES signing key from an `ade.kes.seed.v1` envelope
/// file. Returns a [`KesSecret`] advanced to the envelope's embedded
/// `period_idx`.
pub fn load_ade_kes_signing_key(path: &Path) -> Result<KesSecret, KeyLoadError> {
    let buf = read_file_bytes(path)?;
    let env = ade_kes_envelope::parse(&buf).map_err(KeyLoadError::AdeEnvelope)?;
    KesSecret::from_seed_at_period(&env.seed_32, env.period_idx)
        .map_err(KeyLoadError::Crypto)
}

/// Load an Ed25519 cold signing key from a cardano-cli text-envelope
/// `.skey` file. The envelope `type` must be `StakePoolSigningKey_ed25519`.
pub fn load_cold_signing_key_skey(path: &Path) -> Result<ColdSigningKey, KeyLoadError> {
    let payload = read_envelope_payload(path, POOL_SIGNING_KEY_TYPE)?;
    ColdSigningKey::from_bytes_zeroizing(&payload).map_err(KeyLoadError::Crypto)
}

// =========================================================================
// Writer
// =========================================================================

/// Write an Ade-native KES envelope (`ade.kes.seed.v1`) to `path` with
/// permissions `0o600`. Used by `ade_node key-gen-KES`. The file is
/// truncated if it exists. Errors collapse to
/// `KeyLoadError::Io(ErrorKind)` so the path / contents never appear in
/// logs.
pub fn write_ade_kes_envelope(
    path: &Path,
    seed: &[u8; 32],
    period_idx: u32,
) -> Result<(), KeyLoadError> {
    use std::io::Write;
    let envelope = AdeKesEnvelope {
        seed_32: *seed,
        period_idx,
    };
    let bytes = ade_kes_envelope::serialize(&envelope);
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| KeyLoadError::Io(e.kind()))?;
    f.write_all(&bytes).map_err(|e| KeyLoadError::Io(e.kind()))?;
    f.sync_all().map_err(|e| KeyLoadError::Io(e.kind()))?;
    Ok(())
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
    /// cardano-cli expanded Sum6KES envelope had a payload size other
    /// than the canonical 608 bytes (PHASE4-N-O introduced; PHASE4-N-P
    /// narrowed to the size-mismatch case only — structurally-valid
    /// 608-byte payloads are now imported via [`KesParse`]).
    UnsupportedExpandedKesKeyFormat,
    /// Ade-native KES envelope parse error.
    AdeEnvelope(AdeKesEnvelopeError),
    /// cardano-cli expanded Sum6KES payload was 608 bytes but
    /// structurally invalid (truncated sub-tree, inconsistent vk
    /// hash, leaf-all-zero, etc.). Closed surface from the Ade-owned
    /// `ade_crypto::kes_sum::raw_deserialize_signing_key_kes` path
    /// (PHASE4-N-P S5).
    KesParse(ade_crypto::kes_sum::KesParseError),
}

// =========================================================================
// Envelope reader (cardano-cli text-envelope shape)
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

fn read_file_bytes(path: &Path) -> Result<Vec<u8>, KeyLoadError> {
    let mut file = std::fs::File::open(path).map_err(|e| KeyLoadError::Io(e.kind()))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| KeyLoadError::Io(e.kind()))?;
    Ok(buf)
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
    use ade_crypto::kes::{verify_kes_signature, KesPeriod, KesVerificationKey};
    use ade_crypto::vrf::{verify_vrf, VrfVerificationKey};
    use cardano_crypto::vrf::VrfDraft03;

    // S5: KES test paths use the Ade-owned BLUE algorithm. The kes
    // module alias removes cardano-crypto from the KES test surface;
    // VRF tests above continue using `cardano_crypto::vrf` (which is
    // out of N-P scope).
    use ade_crypto::kes_sum as ade_blue;
    use ade_blue::KesAlgorithm as _AdeKesAlgorithm;
    use std::io::Write;
    use std::os::unix::fs::MetadataExt;

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
    fn vrf_and_cold_cardano_cli_skey_envelopes_round_trip() {
        let dir = tempfile::tempdir().unwrap();

        // ---- VRF: full 64-byte libsodium secret in cborHex ----
        let vrf_seed = [0x42u8; 32];
        let (vrf_sk_raw, vrf_vk_raw) = VrfDraft03::keypair_from_seed(&vrf_seed);
        let vrf_path = write_envelope(dir.path(), "vrf.skey", VRF_SIGNING_KEY_TYPE, &vrf_sk_raw);
        let vrf_sk = load_vrf_signing_key_skey(&vrf_path).unwrap();
        let alpha = b"S1 round-trip alpha";
        let (proof, _output) = super::super::signing::vrf_prove(&vrf_sk, alpha).unwrap();
        verify_vrf(&VrfVerificationKey(vrf_vk_raw), &proof, alpha).unwrap();

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

    // =====================================================================
    // PHASE4-N-O — Ade-native KES envelope round-trip + signature interop
    // =====================================================================

    #[test]
    fn ade_envelope_round_trips_through_loader_at_period_0() {
        let dir = tempfile::tempdir().unwrap();
        let seed = [0x42u8; 32];
        let path = dir.path().join("kes.ade.skey");
        write_ade_kes_envelope(&path, &seed, 0).unwrap();
        let kes_sk = load_ade_kes_signing_key(&path).unwrap();
        assert_eq!(kes_sk.current_period().0, 0);

        let msg = b"S1 round-trip msg";
        let sig = super::super::signing::kes_sign(&kes_sk, KesPeriod(0), msg).unwrap();

        // PHASE4-N-P S5: VK derived via the Ade-owned BLUE algorithm
        // (matches what KesSecret produced internally).
        let raw_ade = ade_blue::Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
        let vk_arr = ade_blue::Sum6Kes::derive_verification_key(&raw_ade);
        verify_kes_signature(&KesVerificationKey(vk_arr), KesPeriod(0), msg, &sig).unwrap();
    }

    #[test]
    fn ade_envelope_loader_returns_kes_at_loaded_period() {
        for period_idx in [0u32, 5, 17, 63] {
            let dir = tempfile::tempdir().unwrap();
            let seed = [0x11u8; 32];
            let path = dir.path().join("kes.ade.skey");
            write_ade_kes_envelope(&path, &seed, period_idx).unwrap();
            let kes_sk = load_ade_kes_signing_key(&path).unwrap();
            assert_eq!(kes_sk.current_period().0, period_idx);
            // Signing at the loaded period must round-trip through verify.
            let msg = b"loaded-period probe";
            let sig =
                super::super::signing::kes_sign(&kes_sk, KesPeriod(period_idx), msg).unwrap();
            let raw_ade = ade_blue::Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
            let vk_arr = ade_blue::Sum6Kes::derive_verification_key(&raw_ade);
            verify_kes_signature(
                &KesVerificationKey(vk_arr),
                KesPeriod(period_idx),
                msg,
                &sig,
            )
            .unwrap();
        }
    }

    #[test]
    fn ade_envelope_loader_rejects_signing_at_past_period() {
        let dir = tempfile::tempdir().unwrap();
        let seed = [0x22u8; 32];
        let path = dir.path().join("kes.ade.skey");
        write_ade_kes_envelope(&path, &seed, 5).unwrap();
        let kes_sk = load_ade_kes_signing_key(&path).unwrap();
        // After load at period 5, signing at period 0..=4 is fail-closed
        // via SigningError::PeriodBackwards.
        let err =
            super::super::signing::kes_sign(&kes_sk, KesPeriod(0), b"past").unwrap_err();
        match err {
            SigningError::PeriodBackwards { .. } => (),
            other => panic!("expected PeriodBackwards, got {:?}", other),
        }
    }

    // =====================================================================
    // PHASE4-N-O — cardano-cli expanded path fail-closed
    // =====================================================================

    // -----------------------------------------------------------------
    // PHASE4-N-P S5: cardano-cli loader now accepts structurally-valid
    // 608-byte payloads. Wrong-size payloads stay fail-closed via
    // `UnsupportedExpandedKesKeyFormat`; 608-byte payloads with
    // structural defects fail-close via the new `KesParse` variant.
    // -----------------------------------------------------------------

    #[test]
    fn cardano_cli_kes_envelope_rejects_32_byte_payload() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_envelope(dir.path(), "kes.skey", KES_SIGNING_KEY_TYPE, &[0x42u8; 32]);
        let err = load_kes_signing_key_skey(&path).unwrap_err();
        assert!(matches!(err, KeyLoadError::UnsupportedExpandedKesKeyFormat));
    }

    #[test]
    fn cardano_cli_kes_envelope_rejects_synthetic_608_byte_payload() {
        // 608 bytes of repeated 0xAB has the right SIZE but is not a
        // structurally-valid Sum6KES tree. After PHASE4-N-P S5, the
        // loader runs the BLUE deserializer; it fails with a
        // structured `KesParseError` (leaf-zero-check passes since
        // 0xAB != 0, then vk-consistency walk fails at level 1).
        let dir = tempfile::tempdir().unwrap();
        let path = write_envelope(dir.path(), "kes.skey", KES_SIGNING_KEY_TYPE, &[0xABu8; 608]);
        let err = load_kes_signing_key_skey(&path).unwrap_err();
        assert!(
            matches!(err, KeyLoadError::KesParse(_)),
            "expected KesParse for synthetic 608-byte payload, got {:?}",
            err
        );
    }

    #[test]
    fn cardano_cli_kes_envelope_accepts_real_608_byte_payload() {
        // PHASE4-N-P S5: a 608-byte payload that IS a valid Sum6KES
        // tree (produced by our own serializer from a real seed) now
        // round-trips through the loader. Construct one via our BLUE
        // gen_key+serialize, write it into a cardano-cli envelope,
        // and load.
        let dir = tempfile::tempdir().unwrap();
        let seed = [0x42u8; 32];
        let sk = ade_blue::Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
        let payload = ade_blue::Sum6Kes::raw_serialize_signing_key_kes(&sk);
        assert_eq!(payload.len(), 608);
        let path = write_envelope(dir.path(), "kes.skey", KES_SIGNING_KEY_TYPE, &payload);

        let kes_sk = load_kes_signing_key_skey(&path).expect("608-byte valid payload accepted");
        assert_eq!(kes_sk.current_period().0, 0);

        // Sign + verify round-trip.
        let msg = b"S5 cardano-cli load round-trip";
        let sig = super::super::signing::kes_sign(&kes_sk, KesPeriod(0), msg).unwrap();
        let vk = ade_blue::Sum6Kes::derive_verification_key(&sk);
        verify_kes_signature(&KesVerificationKey(vk), KesPeriod(0), msg, &sig).unwrap();
    }

    #[test]
    fn cardano_cli_kes_envelope_rejects_612_byte_payload() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_envelope(dir.path(), "kes.skey", KES_SIGNING_KEY_TYPE, &[0xCDu8; 612]);
        let err = load_kes_signing_key_skey(&path).unwrap_err();
        assert!(matches!(err, KeyLoadError::UnsupportedExpandedKesKeyFormat));
    }

    #[test]
    fn cardano_cli_kes_envelope_rejects_608_byte_leaf_zero_payload() {
        // Construct a real Sum6KES skey then zero its leaf (bytes
        // [0..32)). The deserializer must reject via LeafSignKeyAllZero.
        let dir = tempfile::tempdir().unwrap();
        let seed = [0x77u8; 32];
        let sk = ade_blue::Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
        let mut payload = ade_blue::Sum6Kes::raw_serialize_signing_key_kes(&sk);
        for b in payload[0..32].iter_mut() {
            *b = 0;
        }
        let path = write_envelope(dir.path(), "kes.skey", KES_SIGNING_KEY_TYPE, &payload);
        let err = load_kes_signing_key_skey(&path).unwrap_err();
        assert!(
            matches!(
                err,
                KeyLoadError::KesParse(ade_crypto::kes_sum::KesParseError::LeafSignKeyAllZero)
            ),
            "got {:?}",
            err
        );
    }

    // =====================================================================
    // PHASE4-N-O — closed Ade envelope error surfaces
    // =====================================================================

    #[test]
    fn ade_envelope_loader_returns_unknown_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("kes.ade.skey");
        let json = r#"{"format":"other.format.v1","role":"kes_hot_signing_key","crypto":"Sum6KES-Ed25519DSIGN","seed_32":"4242424242424242424242424242424242424242424242424242424242424242","period_idx":0,"format_version":1}"#;
        std::fs::File::create(&path)
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();
        let err = load_ade_kes_signing_key(&path).unwrap_err();
        assert!(matches!(
            err,
            KeyLoadError::AdeEnvelope(AdeKesEnvelopeError::UnknownEnvelopeFormat)
        ));
    }

    #[test]
    fn ade_envelope_loader_returns_wrong_role() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("kes.ade.skey");
        let json = r#"{"format":"ade.kes.seed.v1","role":"vrf_signing_key","crypto":"Sum6KES-Ed25519DSIGN","seed_32":"4242424242424242424242424242424242424242424242424242424242424242","period_idx":0,"format_version":1}"#;
        std::fs::File::create(&path)
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();
        let err = load_ade_kes_signing_key(&path).unwrap_err();
        assert!(matches!(
            err,
            KeyLoadError::AdeEnvelope(AdeKesEnvelopeError::WrongKeyRole)
        ));
    }

    #[test]
    fn ade_envelope_loader_returns_unsupported_crypto() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("kes.ade.skey");
        let json = r#"{"format":"ade.kes.seed.v1","role":"kes_hot_signing_key","crypto":"Sum7KES-Ed25519DSIGN","seed_32":"4242424242424242424242424242424242424242424242424242424242424242","period_idx":0,"format_version":1}"#;
        std::fs::File::create(&path)
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();
        let err = load_ade_kes_signing_key(&path).unwrap_err();
        assert!(matches!(
            err,
            KeyLoadError::AdeEnvelope(AdeKesEnvelopeError::UnsupportedCryptoTag)
        ));
    }

    #[test]
    fn ade_envelope_loader_returns_missing_seed_32() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("kes.ade.skey");
        let json = r#"{"format":"ade.kes.seed.v1","role":"kes_hot_signing_key","crypto":"Sum6KES-Ed25519DSIGN","period_idx":0,"format_version":1}"#;
        std::fs::File::create(&path)
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();
        let err = load_ade_kes_signing_key(&path).unwrap_err();
        assert!(matches!(
            err,
            KeyLoadError::AdeEnvelope(AdeKesEnvelopeError::MissingSeed32)
        ));
    }

    #[test]
    fn ade_envelope_loader_returns_period_idx_overflow() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("kes.ade.skey");
        let json = r#"{"format":"ade.kes.seed.v1","role":"kes_hot_signing_key","crypto":"Sum6KES-Ed25519DSIGN","seed_32":"4242424242424242424242424242424242424242424242424242424242424242","period_idx":65,"format_version":1}"#;
        std::fs::File::create(&path)
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();
        let err = load_ade_kes_signing_key(&path).unwrap_err();
        assert!(matches!(
            err,
            KeyLoadError::AdeEnvelope(AdeKesEnvelopeError::PeriodIdxOutOfRange)
        ));
    }

    #[test]
    fn ade_envelope_loader_returns_malformed_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("kes.ade.skey");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(b"{ not json at all }")
            .unwrap();
        let err = load_ade_kes_signing_key(&path).unwrap_err();
        assert!(matches!(
            err,
            KeyLoadError::AdeEnvelope(AdeKesEnvelopeError::MalformedJson { .. })
        ));
    }

    #[test]
    fn write_ade_kes_envelope_sets_0600_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("kes.ade.skey");
        let seed = [0x33u8; 32];
        write_ade_kes_envelope(&path, &seed, 0).unwrap();
        let meta = std::fs::metadata(&path).unwrap();
        let mode = meta.mode() & 0o777;
        assert_eq!(mode, 0o600, "expected 0o600, got {:o}", mode);
    }

    // =====================================================================
    // VRF / cold negative paths (unchanged from N-C S1)
    // =====================================================================

    #[test]
    fn keys_loader_rejects_wrong_envelope_type() {
        let dir = tempfile::tempdir().unwrap();
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
        let exotic_path = std::path::PathBuf::from("/tmp/__N_O_S1_DOES_NOT_EXIST_SECRET_42__");
        let err = load_vrf_signing_key_skey(&exotic_path).unwrap_err();
        let s = format!("{:?}", err);
        assert!(matches!(err, KeyLoadError::Io(_)));
        assert!(!s.contains("__N_O_S1_DOES_NOT_EXIST_SECRET_42__"));
        assert!(!s.contains("/tmp/"));
    }

    #[test]
    fn ade_envelope_load_error_io_carries_no_path_bytes() {
        let exotic_path = std::path::PathBuf::from("/tmp/__N_O_S1_ADE_SECRET_88__");
        let err = load_ade_kes_signing_key(&exotic_path).unwrap_err();
        let s = format!("{:?}", err);
        assert!(matches!(err, KeyLoadError::Io(_)));
        assert!(!s.contains("__N_O_S1_ADE_SECRET_88__"));
        assert!(!s.contains("/tmp/"));
    }
}
