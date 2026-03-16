// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

// Blake2b hash functions — the single authoritative source for all Blake2b
// hashing in the project.
//
// Domain separation on Cardano's hash-critical surfaces is by output length
// and typed context (Hash32 vs Hash28), matching the Haskell cardano-node
// construction exactly. No personalization string, no custom IV, no prefix byte.
//
// CN-CRYPTO-02 satisfied: 32-byte and 28-byte hashes inhabit different typed
// contexts and are never confused.

use ade_types::{Hash28, Hash32};
use blake2::digest::Digest;

use crate::traits::HashAlgorithm;

/// Blake2b-256 hash algorithm (32-byte output).
pub struct Blake2b256;

impl HashAlgorithm for Blake2b256 {
    type Output = [u8; 32];

    fn hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = blake2::Blake2b::<blake2::digest::consts::U32>::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut output = [0u8; 32];
        output.copy_from_slice(&result);
        output
    }
}

/// Blake2b-224 hash algorithm (28-byte output).
pub struct Blake2b224;

impl HashAlgorithm for Blake2b224 {
    type Output = [u8; 28];

    fn hash(data: &[u8]) -> [u8; 28] {
        let mut hasher = blake2::Blake2b::<blake2::digest::consts::U28>::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut output = [0u8; 28];
        output.copy_from_slice(&result);
        output
    }
}

/// Compute Blake2b-256 (32-byte output) of arbitrary data.
///
/// Pure and infallible — any byte slice is valid input.
pub fn blake2b_256(data: &[u8]) -> Hash32 {
    Hash32(Blake2b256::hash(data))
}

/// Compute Blake2b-224 (28-byte output) of arbitrary data.
///
/// Pure and infallible — any byte slice is valid input.
pub fn blake2b_224(data: &[u8]) -> Hash28 {
    Hash28(Blake2b224::hash(data))
}

/// Compute block header hash: Blake2b-256 of header wire bytes.
///
/// Callers on hash-critical paths MUST pass bytes from `PreservedCbor<T>.wire_bytes()`.
pub fn block_header_hash(header_wire_bytes: &[u8]) -> Hash32 {
    blake2b_256(header_wire_bytes)
}

/// Compute transaction ID: Blake2b-256 of transaction body wire bytes.
///
/// Callers on hash-critical paths MUST pass bytes from `PreservedCbor<T>.wire_bytes()`.
pub fn transaction_id(tx_body_wire_bytes: &[u8]) -> Hash32 {
    blake2b_256(tx_body_wire_bytes)
}

/// Compute script hash: Blake2b-224 of script bytes.
pub fn script_hash(script_bytes: &[u8]) -> Hash28 {
    blake2b_224(script_bytes)
}

/// Compute credential hash: Blake2b-224 of verification key bytes.
pub fn credential_hash(vk_bytes: &[u8]) -> Hash28 {
    blake2b_224(vk_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 7693 Appendix A — Blake2b-256 test vectors

    #[test]
    fn blake2b_256_empty() {
        let hash = blake2b_256(b"");
        // Blake2b-256 of empty input — cross-validated with Python hashlib.blake2b
        assert_eq!(
            format!("{hash}"),
            "0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8"
        );
    }

    #[test]
    fn blake2b_256_abc() {
        let hash = blake2b_256(b"abc");
        assert_eq!(
            format!("{hash}"),
            "bddd813c634239723171ef3fee98579b94964e3bb1cb3e427262c8c068d52319"
        );
    }

    #[test]
    fn blake2b_256_single_byte() {
        let hash = blake2b_256(&[0x00]);
        assert_eq!(
            format!("{hash}"),
            "03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314"
        );
    }

    #[test]
    fn blake2b_256_multi_block() {
        // 128 bytes of 0xFF — spans multiple Blake2b blocks (block size = 128)
        let data = vec![0xFF; 128];
        let hash = blake2b_256(&data);
        // Cross-validated with Python hashlib.blake2b
        assert_eq!(
            format!("{hash}"),
            "d3f35cd80b65c482e3026da32b729e9e7fd75065aca6677b16e488a58f5625f7"
        );
    }

    #[test]
    fn blake2b_256_large() {
        // 1024 bytes of incrementing pattern
        let data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
        let hash = blake2b_256(&data);
        // Deterministic: same input always produces same output
        let hash2 = blake2b_256(&data);
        assert_eq!(hash, hash2);
        // Non-trivial: not all zeros
        assert_ne!(hash.0, [0u8; 32]);
    }

    // Blake2b-224 test vectors

    #[test]
    fn blake2b_224_empty() {
        let hash = blake2b_224(b"");
        assert_eq!(
            format!("{hash}"),
            "836cc68931c2e4e3e838602eca1902591d216837bafddfe6f0c8cb07"
        );
    }

    #[test]
    fn blake2b_224_abc() {
        let hash = blake2b_224(b"abc");
        // Cross-validated with Python hashlib.blake2b(digest_size=28)
        assert_eq!(
            format!("{hash}"),
            "9bd237b02a29e43bdd6738afa5b53ff0eee178d6210b618e4511aec8"
        );
    }

    // Domain wrapper tests

    #[test]
    fn block_header_hash_is_blake2b_256() {
        let data = b"test header bytes";
        assert_eq!(block_header_hash(data), blake2b_256(data));
    }

    #[test]
    fn transaction_id_is_blake2b_256() {
        let data = b"test tx body bytes";
        assert_eq!(transaction_id(data), blake2b_256(data));
    }

    #[test]
    fn script_hash_is_blake2b_224() {
        let data = b"test script bytes";
        assert_eq!(script_hash(data), blake2b_224(data));
    }

    #[test]
    fn credential_hash_is_blake2b_224() {
        let data = b"test vk bytes";
        assert_eq!(credential_hash(data), blake2b_224(data));
    }

    // HashAlgorithm trait tests

    #[test]
    fn blake2b256_trait_matches_function() {
        let data = b"trait test";
        let via_trait = Blake2b256::hash(data);
        let via_fn = blake2b_256(data);
        assert_eq!(via_trait, via_fn.0);
    }

    #[test]
    fn blake2b224_trait_matches_function() {
        let data = b"trait test";
        let via_trait = Blake2b224::hash(data);
        let via_fn = blake2b_224(data);
        assert_eq!(via_trait, via_fn.0);
    }
}
