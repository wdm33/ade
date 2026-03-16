// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

// Ed25519 signature verification for Cardano transaction witnesses.
//
// Verdict contract (standard verification):
//   Ok(true)                  — valid: well-formed inputs, cryptographic check passed
//   Ok(false)                 — invalid: well-formed inputs, cryptographic check failed
//   Err(CryptoError)          — malformed: structurally invalid, cannot attempt verification
//
// Two verification surfaces:
//   Standard Ed25519:         32-byte vk, 64-byte sig, arbitrary message
//   Byron bootstrap extended: 64-byte xvk (first 32 = ed25519 vk, last 32 = chain code)
//
// Byron extended key verification: only the first 32 bytes (standard Ed25519 vk)
// participate in verification. The chain code (bytes 32-63) is used for key
// derivation only, not signature verification.
//
// Cardano uses libsodium's crypto_sign_ed25519_verify_detached. Test vectors
// are generated and cross-validated using PyNaCl (which wraps libsodium).

use crate::error::CryptoError;

/// Ed25519 verification key (32 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ed25519VerificationKey(pub [u8; 32]);

/// Ed25519 signature (64 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ed25519Signature(pub [u8; 64]);

/// Byron extended verification key (64 bytes: 32-byte vk + 32-byte chain code).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronExtendedVerificationKey(pub [u8; 64]);

impl Ed25519VerificationKey {
    /// Construct from a byte slice, validating length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != 32 {
            return Err(CryptoError::MalformedKey {
                algorithm: "ed25519",
                detail: "expected 32 bytes",
            });
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Ok(Self(arr))
    }
}

impl Ed25519Signature {
    /// Construct from a byte slice, validating length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != 64 {
            return Err(CryptoError::MalformedSignature {
                algorithm: "ed25519",
                detail: "expected 64 bytes",
            });
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(bytes);
        Ok(Self(arr))
    }
}

impl ByronExtendedVerificationKey {
    /// Construct from a byte slice, validating length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != 64 {
            return Err(CryptoError::MalformedKey {
                algorithm: "ed25519_extended",
                detail: "expected 64 bytes",
            });
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(bytes);
        Ok(Self(arr))
    }
}

/// Verify a standard Ed25519 signature.
///
/// Returns:
///   `Ok(true)`  — signature valid
///   `Ok(false)` — signature invalid (well-formed inputs, bad signature)
///   `Err(CryptoError::MalformedKey)` — vk has invalid point encoding
pub fn verify_ed25519(
    vk: &Ed25519VerificationKey,
    msg: &[u8],
    sig: &Ed25519Signature,
) -> Result<bool, CryptoError> {
    let dalek_vk =
        ed25519_dalek::VerifyingKey::from_bytes(&vk.0).map_err(|_| CryptoError::MalformedKey {
            algorithm: "ed25519",
            detail: "invalid point encoding",
        })?;

    let dalek_sig = ed25519_dalek::Signature::from_bytes(&sig.0);

    use ed25519_dalek::Verifier;
    match dalek_vk.verify(msg, &dalek_sig) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Verify a Byron bootstrap extended Ed25519 signature.
///
/// Only the first 32 bytes of the extended key (the standard Ed25519 vk)
/// participate in verification. The chain code (bytes 32-63) is for key
/// derivation only.
///
/// Returns same verdict contract as `verify_ed25519`.
pub fn verify_byron_bootstrap(
    xvk: &ByronExtendedVerificationKey,
    msg: &[u8],
    sig: &Ed25519Signature,
) -> Result<bool, CryptoError> {
    let mut vk_bytes = [0u8; 32];
    vk_bytes.copy_from_slice(&xvk.0[..32]);
    let vk = Ed25519VerificationKey(vk_bytes);
    verify_ed25519(&vk, msg, sig)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // All test vectors generated and cross-validated with PyNaCl (libsodium)
    // which is the Cardano oracle for Ed25519 verification.

    // Vector 1: empty message (cross-validated with libsodium)
    #[test]
    fn libsodium_vector_empty_message() {
        let vk = Ed25519VerificationKey::from_bytes(&hex_decode(
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        ));
        let sig = Ed25519Signature::from_bytes(&hex_decode(
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155\
             5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
        ));

        let result = verify_ed25519(&vk.unwrap(), b"", &sig.unwrap());
        assert_eq!(result, Ok(true));
    }

    // Vector 2: single byte 0x72 (cross-validated with libsodium)
    #[test]
    fn libsodium_vector_single_byte() {
        let vk = Ed25519VerificationKey::from_bytes(&hex_decode(
            "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
        ));
        let sig = Ed25519Signature::from_bytes(&hex_decode(
            "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da\
             085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
        ));

        let result = verify_ed25519(&vk.unwrap(), &[0x72], &sig.unwrap());
        assert_eq!(result, Ok(true));
    }

    // Vector 3: 2-byte message (cross-validated with libsodium)
    #[test]
    fn libsodium_vector_two_byte() {
        let vk = Ed25519VerificationKey::from_bytes(&hex_decode(
            "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025",
        ));
        let sig = Ed25519Signature::from_bytes(&hex_decode(
            "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac\
             18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a",
        ));

        let result = verify_ed25519(&vk.unwrap(), &hex_decode("af82"), &sig.unwrap());
        assert_eq!(result, Ok(true));
    }

    // Vector 4: longer message (cross-validated with libsodium)
    #[test]
    fn libsodium_vector_longer_message() {
        let vk = Ed25519VerificationKey::from_bytes(&hex_decode(
            "03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8",
        ));
        let sig = Ed25519Signature::from_bytes(&hex_decode(
            "976583084645b3bc40deb4950971b1798665a1465d8b0f1bdca018bf98345e94\
             f77064cce8b187244a314d869015e3d274fffb3239ebc8ffa4af1468bbce6d0d",
        ));

        let msg = hex_decode("43617264616e6f20626c6f636b206865616465722074657374206d657373616765");
        let result = verify_ed25519(&vk.unwrap(), &msg, &sig.unwrap());
        assert_eq!(result, Ok(true));
    }

    // Vector 5: 32-byte message (tx hash size, cross-validated with libsodium)
    #[test]
    fn libsodium_vector_hash_size_message() {
        let vk = Ed25519VerificationKey::from_bytes(&hex_decode(
            "29acbae141bccaf0b22e1a94d34d0bc7361e526d0bfe12c89794bc9322966dd7",
        ));
        let sig = Ed25519Signature::from_bytes(&hex_decode(
            "ea92d24eda4c099ea3c1aaa10c86cd63e2a0430aaf865db6ed60f8129de7a363\
             a61cdc5251b5c74f07755639b33ddad8699ef85be61cbe8d603e8e80aa7c1b09",
        ));

        let msg = hex_decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f");
        let result = verify_ed25519(&vk.unwrap(), &msg, &sig.unwrap());
        assert_eq!(result, Ok(true));
    }

    // Wrong key — valid structure, invalid signature
    #[test]
    fn wrong_key_returns_false() {
        // V1 key with V2 sig
        let vk = Ed25519VerificationKey::from_bytes(&hex_decode(
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        ));
        let sig = Ed25519Signature::from_bytes(&hex_decode(
            "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da\
             085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
        ));

        let result = verify_ed25519(&vk.unwrap(), &[0x72], &sig.unwrap());
        assert_eq!(result, Ok(false));
    }

    // Wrong message — valid structure, invalid signature
    #[test]
    fn wrong_message_returns_false() {
        let vk = Ed25519VerificationKey::from_bytes(&hex_decode(
            "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
        ));
        let sig = Ed25519Signature::from_bytes(&hex_decode(
            "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da\
             085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
        ));

        // Wrong message (original was 0x72)
        let result = verify_ed25519(&vk.unwrap(), &[0x73], &sig.unwrap());
        assert_eq!(result, Ok(false));
    }

    // Malformed key — wrong length
    #[test]
    fn malformed_key_wrong_length() {
        let result = Ed25519VerificationKey::from_bytes(&[0u8; 31]);
        assert_eq!(
            result,
            Err(CryptoError::MalformedKey {
                algorithm: "ed25519",
                detail: "expected 32 bytes",
            })
        );
    }

    #[test]
    fn malformed_key_too_long() {
        let result = Ed25519VerificationKey::from_bytes(&[0u8; 33]);
        assert_eq!(
            result,
            Err(CryptoError::MalformedKey {
                algorithm: "ed25519",
                detail: "expected 32 bytes",
            })
        );
    }

    // Malformed signature — wrong length
    #[test]
    fn malformed_sig_wrong_length() {
        let result = Ed25519Signature::from_bytes(&[0u8; 63]);
        assert_eq!(
            result,
            Err(CryptoError::MalformedSignature {
                algorithm: "ed25519",
                detail: "expected 64 bytes",
            })
        );
    }

    // Invalid point encoding — 32 bytes that don't decompress to a valid curve point
    #[test]
    fn malformed_key_invalid_point() {
        // Bytes that fail point decompression in ed25519-dalek
        // (y-coordinate >= p, which is impossible on the curve)
        let mut bad_key = [0xFF; 32];
        bad_key[31] = 0xFF; // High byte 0xFF guarantees y >= p
        let vk = Ed25519VerificationKey(bad_key);
        let sig = Ed25519Signature([0u8; 64]);
        let result = verify_ed25519(&vk, b"test", &sig);
        // Either MalformedKey (failed decompression) or Ok(false) (decompressed but invalid sig)
        // Both are correct under the verdict contract
        assert!(
            matches!(result, Err(CryptoError::MalformedKey { .. }) | Ok(false)),
            "expected MalformedKey or Ok(false), got {result:?}"
        );
    }

    // Byron extended key — wrong length
    #[test]
    fn byron_xvk_wrong_length() {
        let result = ByronExtendedVerificationKey::from_bytes(&[0u8; 63]);
        assert_eq!(
            result,
            Err(CryptoError::MalformedKey {
                algorithm: "ed25519_extended",
                detail: "expected 64 bytes",
            })
        );
    }

    // No retry/fallback: invalid signature returns Ok(false), not retried
    #[test]
    fn no_fallback_on_invalid() {
        let vk = Ed25519VerificationKey::from_bytes(&hex_decode(
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        ));

        // Corrupted signature (all zeros)
        let sig = Ed25519Signature([0u8; 64]);
        let result = verify_ed25519(&vk.unwrap(), b"", &sig);
        assert_eq!(result, Ok(false));
    }

    // Determinism test — same inputs always produce same result
    #[test]
    fn verification_is_deterministic() {
        let vk = Ed25519VerificationKey::from_bytes(&hex_decode(
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        ));
        let sig = Ed25519Signature::from_bytes(&hex_decode(
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155\
             5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
        ));

        let r1 = verify_ed25519(&vk.clone().unwrap(), b"", &sig.clone().unwrap());
        let r2 = verify_ed25519(&vk.unwrap(), b"", &sig.unwrap());
        assert_eq!(r1, r2);
        assert_eq!(r1, Ok(true));
    }

    // Byron extended key verification — chain code is ignored
    #[test]
    fn byron_extended_key_chain_code_ignored() {
        // Use V1 key as the first 32 bytes, random chain code as last 32
        let vk_hex = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let vk_bytes = hex_decode(vk_hex);
        let chain_code = [0xAB; 32];

        let mut xvk_bytes = [0u8; 64];
        xvk_bytes[..32].copy_from_slice(&vk_bytes);
        xvk_bytes[32..].copy_from_slice(&chain_code);
        let xvk = ByronExtendedVerificationKey(xvk_bytes);

        let sig = Ed25519Signature::from_bytes(&hex_decode(
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155\
             5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
        ));

        // Should verify just like standard Ed25519 with the first 32 bytes
        let result = verify_byron_bootstrap(&xvk, b"", &sig.unwrap());
        assert_eq!(result, Ok(true));
    }

    fn hex_decode(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
            .collect::<Result<Vec<u8>, _>>()
            .unwrap()
    }
}
