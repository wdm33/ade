// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

// Sum6KES signature verification and operational certificate verification.
//
// Sum6KES: binary-tree Merkle KES with depth 6, supporting 2^6 = 64 periods.
// Uses Blake2b-256 for internal node hashing, matching the Haskell cardano-node.
//
// Operational certificate: Ed25519 signature by the cold key over the signable
// encoding (hot vkey || counter || KES period as CBOR).
//
// Uses the `cardano-crypto` crate's pure Rust KES implementation which provides
// byte-level compatibility with Cardano's Haskell KES.
//
// Verdict contract (standard verification):
//   Ok(true)  — valid: well-formed inputs, cryptographic check passed
//   Ok(false) — invalid: well-formed inputs, cryptographic check failed
//   Err(CryptoError) — malformed: structurally invalid, cannot attempt verification

use crate::ed25519::{verify_ed25519, Ed25519Signature, Ed25519VerificationKey};
use crate::error::CryptoError;

/// KES verification key (32 bytes) — the Merkle root of the Sum6KES tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KesVerificationKey(pub [u8; 32]);

/// KES period (0..63 for Sum6KES).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KesPeriod(pub u32);

/// Sum6KES maximum period (2^6 - 1 = 63).
pub const SUM6_MAX_PERIOD: u32 = 63;

/// Sum6KES signature size in bytes.
/// Base Ed25519 sig (64) + 6 levels * 2 sibling VKs (2 * 32 = 64 per level) = 64 + 6*64 = 448
/// Actually: computed from the recursive formula in the cardano-crypto crate.
const SUM6_SIGNATURE_SIZE: usize = get_sum6_sig_size();

/// Compute Sum6KES signature size at compile time.
/// SingleKES sig = 64 (Ed25519)
/// SumKES(d) sig = SumKES(d-1) sig + 2 * VK_SIZE(d-1)
/// VK at each level is 32 bytes (Blake2b-256 hash or Ed25519 vk).
const fn get_sum6_sig_size() -> usize {
    // Sum0Kes = SingleKes: sig=64, vk=32
    // Sum1Kes = SumKes<Sum0Kes>: sig = 64 + 2*32 = 128, vk = 32
    // Sum2Kes = SumKes<Sum1Kes>: sig = 128 + 2*32 = 192, vk = 32
    // Sum3Kes = SumKes<Sum2Kes>: sig = 192 + 2*32 = 256, vk = 32
    // Sum4Kes = SumKes<Sum3Kes>: sig = 256 + 2*32 = 320, vk = 32
    // Sum5Kes = SumKes<Sum4Kes>: sig = 320 + 2*32 = 384, vk = 32
    // Sum6Kes = SumKes<Sum5Kes>: sig = 384 + 2*32 = 448, vk = 32
    448
}

impl KesVerificationKey {
    /// Construct from a byte slice, validating length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != 32 {
            return Err(CryptoError::MalformedKey {
                algorithm: "kes_sum6",
                detail: "expected 32 bytes",
            });
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Ok(Self(arr))
    }
}

impl KesPeriod {
    /// Construct a KES period, validating range.
    pub fn new(period: u32) -> Result<Self, CryptoError> {
        if period > SUM6_MAX_PERIOD {
            return Err(CryptoError::KesExpiredPeriod {
                current: period,
                max: SUM6_MAX_PERIOD,
            });
        }
        Ok(Self(period))
    }
}

/// Verify a Sum6KES signature.
///
/// Returns:
///   `Ok(true)`  — valid signature for the given period and message
///   `Ok(false)` — well-formed but invalid (Merkle root mismatch, bad leaf sig)
///   `Err(CryptoError)` — malformed inputs (bad period, wrong-size sig, etc.)
pub fn verify_kes(
    vk: &KesVerificationKey,
    period: KesPeriod,
    sig_bytes: &[u8],
    msg: &[u8],
) -> Result<bool, CryptoError> {
    use cardano_crypto::kes::{KesAlgorithm, Sum6Kes};

    if sig_bytes.len() != SUM6_SIGNATURE_SIZE {
        return Err(CryptoError::MalformedSignature {
            algorithm: "kes_sum6",
            detail: "wrong signature length",
        });
    }

    let signature = Sum6Kes::raw_deserialize_signature_kes(sig_bytes).ok_or(
        CryptoError::MalformedSignature {
            algorithm: "kes_sum6",
            detail: "failed to deserialize KES signature",
        },
    )?;

    let kes_vk = vk.0.to_vec();

    match Sum6Kes::verify_kes(&(), &kes_vk, period.0 as u64, msg, &signature) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Operational certificate data for verification.
#[derive(Debug, Clone)]
pub struct OperationalCertData {
    /// Hot KES verification key (32 bytes).
    pub hot_vkey: KesVerificationKey,
    /// Operational certificate sequence number.
    pub sequence_number: u64,
    /// KES period at which this cert becomes active.
    pub kes_period: u64,
    /// Cold key signature over the signable encoding.
    pub cold_signature: Ed25519Signature,
}

/// Verify an operational certificate.
///
/// The cold verification key signs the "signable" encoding of:
///   hot_vkey (32 bytes) || sequence_number (8 bytes BE) || kes_period (8 bytes BE)
///
/// Note: The exact signable encoding must match the Haskell OCertSignable toCBOR.
/// The Haskell encoding uses CBOR serialization of (vk_hot, counter, kes_period).
/// This is: CBOR bytes(32) for vk_hot || CBOR uint for counter || CBOR uint for kes_period
///
/// Returns:
///   `Ok(true)`  — valid: cold key signed this opcert
///   `Ok(false)` — invalid: signature check failed
///   `Err(CryptoError)` — malformed cold key
pub fn verify_opcert(
    cold_vk: &Ed25519VerificationKey,
    opcert: &OperationalCertData,
) -> Result<bool, CryptoError> {
    let signable = build_opcert_signable(opcert);
    verify_ed25519(cold_vk, &signable, &opcert.cold_signature)
}

/// Build the signable bytes for an operational certificate.
///
/// Matches the Haskell OCertSignable toCBOR encoding:
/// CBOR encoding of (vk_hot_bytes, counter, kes_period)
///
/// The CBOR structure is:
///   bytes(32) [hot vkey] || uint [sequence_number] || uint [kes_period]
fn build_opcert_signable(opcert: &OperationalCertData) -> alloc::vec::Vec<u8> {
    let mut buf = alloc::vec::Vec::with_capacity(64);

    // CBOR bytes(32) for hot vkey: major type 2, length 32 = 0x5820
    buf.push(0x58);
    buf.push(0x20);
    buf.extend_from_slice(&opcert.hot_vkey.0);

    // CBOR uint for sequence_number
    cbor_encode_uint(&mut buf, opcert.sequence_number);

    // CBOR uint for kes_period
    cbor_encode_uint(&mut buf, opcert.kes_period);

    buf
}

/// Encode a u64 as CBOR unsigned integer.
fn cbor_encode_uint(buf: &mut alloc::vec::Vec<u8>, value: u64) {
    if value < 24 {
        buf.push(value as u8);
    } else if value <= 0xFF {
        buf.push(0x18);
        buf.push(value as u8);
    } else if value <= 0xFFFF {
        buf.push(0x19);
        buf.extend_from_slice(&(value as u16).to_be_bytes());
    } else if value <= 0xFFFF_FFFF {
        buf.push(0x1A);
        buf.extend_from_slice(&(value as u32).to_be_bytes());
    } else {
        buf.push(0x1B);
        buf.extend_from_slice(&value.to_be_bytes());
    }
}

extern crate alloc;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // KesPeriod validation

    #[test]
    fn kes_period_valid_range() {
        assert!(KesPeriod::new(0).is_ok());
        assert!(KesPeriod::new(63).is_ok());
    }

    #[test]
    fn kes_period_out_of_range() {
        let result = KesPeriod::new(64);
        assert_eq!(
            result,
            Err(CryptoError::KesExpiredPeriod {
                current: 64,
                max: SUM6_MAX_PERIOD,
            })
        );
    }

    #[test]
    fn kes_period_large_value() {
        let result = KesPeriod::new(1000);
        assert!(matches!(result, Err(CryptoError::KesExpiredPeriod { .. })));
    }

    // KesVerificationKey validation

    #[test]
    fn kes_vk_wrong_length() {
        let result = KesVerificationKey::from_bytes(&[0u8; 31]);
        assert_eq!(
            result,
            Err(CryptoError::MalformedKey {
                algorithm: "kes_sum6",
                detail: "expected 32 bytes",
            })
        );
    }

    // KES signature malformed

    #[test]
    fn kes_sig_wrong_length() {
        let vk = KesVerificationKey([0u8; 32]);
        let period = KesPeriod::new(0).unwrap();
        let result = verify_kes(&vk, period, &[0u8; 100], b"msg");
        assert_eq!(
            result,
            Err(CryptoError::MalformedSignature {
                algorithm: "kes_sum6",
                detail: "wrong signature length",
            })
        );
    }

    // Self-consistency: generate KES key, sign, and verify
    #[test]
    fn generate_sign_verify_period_0() {
        use cardano_crypto::kes::{KesAlgorithm, Sum6Kes};

        let seed = [42u8; 32];
        let signing_key = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
        let vk_bytes = Sum6Kes::raw_serialize_verification_key_kes(
            &Sum6Kes::derive_verification_key(&signing_key).unwrap(),
        );

        let msg = b"Block header at period 0";
        let sig = Sum6Kes::sign_kes(&(), 0, msg, &signing_key).unwrap();
        let sig_bytes = Sum6Kes::raw_serialize_signature_kes(&sig);

        assert_eq!(sig_bytes.len(), SUM6_SIGNATURE_SIZE);

        let mut vk_arr = [0u8; 32];
        vk_arr.copy_from_slice(&vk_bytes);
        let vk = KesVerificationKey(vk_arr);
        let period = KesPeriod::new(0).unwrap();

        let result = verify_kes(&vk, period, &sig_bytes, msg);
        assert_eq!(result, Ok(true));
    }

    // Wrong period returns Ok(false)
    #[test]
    fn wrong_period_returns_false() {
        use cardano_crypto::kes::{KesAlgorithm, Sum6Kes};

        let seed = [7u8; 32];
        let signing_key = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
        let vk_bytes = Sum6Kes::raw_serialize_verification_key_kes(
            &Sum6Kes::derive_verification_key(&signing_key).unwrap(),
        );

        let msg = b"Block header at period 0";
        let sig = Sum6Kes::sign_kes(&(), 0, msg, &signing_key).unwrap();
        let sig_bytes = Sum6Kes::raw_serialize_signature_kes(&sig);

        let mut vk_arr = [0u8; 32];
        vk_arr.copy_from_slice(&vk_bytes);
        let vk = KesVerificationKey(vk_arr);

        // Verify with wrong period
        let period = KesPeriod::new(1).unwrap();
        let result = verify_kes(&vk, period, &sig_bytes, msg);
        assert_eq!(result, Ok(false));
    }

    // Wrong message returns Ok(false)
    #[test]
    fn wrong_message_returns_false() {
        use cardano_crypto::kes::{KesAlgorithm, Sum6Kes};

        let seed = [99u8; 32];
        let signing_key = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
        let vk_bytes = Sum6Kes::raw_serialize_verification_key_kes(
            &Sum6Kes::derive_verification_key(&signing_key).unwrap(),
        );

        let msg = b"Block header at period 0";
        let sig = Sum6Kes::sign_kes(&(), 0, msg, &signing_key).unwrap();
        let sig_bytes = Sum6Kes::raw_serialize_signature_kes(&sig);

        let mut vk_arr = [0u8; 32];
        vk_arr.copy_from_slice(&vk_bytes);
        let vk = KesVerificationKey(vk_arr);
        let period = KesPeriod::new(0).unwrap();

        let result = verify_kes(&vk, period, &sig_bytes, b"different message");
        assert_eq!(result, Ok(false));
    }

    // Determinism: same inputs always produce same result
    #[test]
    fn kes_verification_is_deterministic() {
        use cardano_crypto::kes::{KesAlgorithm, Sum6Kes};

        let seed = [11u8; 32];
        let signing_key = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
        let vk_bytes = Sum6Kes::raw_serialize_verification_key_kes(
            &Sum6Kes::derive_verification_key(&signing_key).unwrap(),
        );

        let msg = b"determinism test";
        let sig = Sum6Kes::sign_kes(&(), 0, msg, &signing_key).unwrap();
        let sig_bytes = Sum6Kes::raw_serialize_signature_kes(&sig);

        let mut vk_arr = [0u8; 32];
        vk_arr.copy_from_slice(&vk_bytes);
        let vk = KesVerificationKey(vk_arr);
        let period = KesPeriod::new(0).unwrap();

        let r1 = verify_kes(&vk, period, &sig_bytes, msg);
        let r2 = verify_kes(&vk, period, &sig_bytes, msg);
        assert_eq!(r1, r2);
        assert_eq!(r1, Ok(true));
    }

    // CBOR uint encoding

    #[test]
    fn cbor_encode_small() {
        let mut buf = alloc::vec::Vec::new();
        cbor_encode_uint(&mut buf, 0);
        assert_eq!(buf, [0x00]);

        buf.clear();
        cbor_encode_uint(&mut buf, 23);
        assert_eq!(buf, [23]);
    }

    #[test]
    fn cbor_encode_one_byte() {
        let mut buf = alloc::vec::Vec::new();
        cbor_encode_uint(&mut buf, 24);
        assert_eq!(buf, [0x18, 24]);

        buf.clear();
        cbor_encode_uint(&mut buf, 255);
        assert_eq!(buf, [0x18, 0xFF]);
    }

    #[test]
    fn cbor_encode_two_byte() {
        let mut buf = alloc::vec::Vec::new();
        cbor_encode_uint(&mut buf, 256);
        assert_eq!(buf, [0x19, 0x01, 0x00]);
    }

    // Opcert signable encoding
    #[test]
    fn opcert_signable_format() {
        let opcert = OperationalCertData {
            hot_vkey: KesVerificationKey([0xAB; 32]),
            sequence_number: 0,
            kes_period: 0,
            cold_signature: Ed25519Signature([0u8; 64]),
        };

        let signable = build_opcert_signable(&opcert);
        // CBOR bytes(32): 0x5820 + 32 bytes + CBOR uint(0) + CBOR uint(0) = 34 + 1 + 1 = 36
        assert_eq!(signable.len(), 36);
        assert_eq!(signable[0], 0x58); // CBOR bytes prefix
        assert_eq!(signable[1], 0x20); // length 32
        assert_eq!(signable[2], 0xAB); // first byte of hot vkey
        assert_eq!(signable[34], 0x00); // sequence_number = 0
        assert_eq!(signable[35], 0x00); // kes_period = 0
    }

    #[test]
    fn sum6_signature_size_is_correct() {
        use cardano_crypto::kes::KesAlgorithm;
        use cardano_crypto::kes::Sum6Kes;
        assert_eq!(Sum6Kes::SIGNATURE_SIZE, SUM6_SIGNATURE_SIZE);
    }
}
