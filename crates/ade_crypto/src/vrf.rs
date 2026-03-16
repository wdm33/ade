// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

// VRF (Verifiable Random Function) verification.
//
// ECVRF-ED25519-SHA512-Elligator2 (IETF draft-irtf-cfrg-vrf-03)
// matching Cardano's Praos consensus VRF.
//
// Uses the `cardano-crypto` crate's pure Rust implementation which provides
// byte-level compatibility with Cardano's IOHK libsodium VRF fork.
// This eliminates the need for unsafe FFI while maintaining oracle equivalence.
//
// Verdict contract (extractive verification):
//   Ok(VrfOutput)                          — valid proof, output extracted
//   Err(CryptoError::VerificationFailed)   — well-formed inputs, proof failed
//   Err(CryptoError::MalformedProof/Key)   — structurally invalid, pre-verify reject

use crate::error::CryptoError;

/// VRF verification key (32 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VrfVerificationKey(pub [u8; 32]);

/// VRF proof (80 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VrfProof(pub [u8; 80]);

/// VRF output (64 bytes) — extracted from a valid proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VrfOutput(pub [u8; 64]);

impl VrfVerificationKey {
    /// Construct from a byte slice, validating length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != 32 {
            return Err(CryptoError::MalformedKey {
                algorithm: "vrf_ecvrf",
                detail: "expected 32 bytes",
            });
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Ok(Self(arr))
    }
}

impl VrfProof {
    /// Construct from a byte slice, validating length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != 80 {
            return Err(CryptoError::MalformedProof {
                detail: "expected 80 bytes",
            });
        }
        let mut arr = [0u8; 80];
        arr.copy_from_slice(bytes);
        Ok(Self(arr))
    }
}

/// Verify a VRF proof and extract the output.
///
/// On success, returns the 64-byte VRF output that is bit-identical to
/// what the Haskell cardano-node produces for the same (vk, proof, alpha).
///
/// This is extractive verification: valid proofs return `Ok(VrfOutput)`,
/// not `Ok(true)`.
pub fn verify_vrf(
    vk: &VrfVerificationKey,
    proof: &VrfProof,
    alpha: &[u8],
) -> Result<VrfOutput, CryptoError> {
    use cardano_crypto::vrf::VrfDraft03;

    match VrfDraft03::verify(&vk.0, &proof.0, alpha) {
        Ok(output) => Ok(VrfOutput(output)),
        Err(_) => Err(CryptoError::VerificationFailed {
            algorithm: "vrf_ecvrf",
        }),
    }
}

/// Extract VRF output from a proof without verification.
///
/// This is a deterministic conversion from proof bytes to output bytes.
/// It does NOT verify the proof against a key — use `verify_vrf` for that.
pub fn vrf_proof_to_hash(proof: &VrfProof) -> Result<VrfOutput, CryptoError> {
    use cardano_crypto::vrf::VrfDraft03;

    match VrfDraft03::proof_to_hash(&proof.0) {
        Ok(output) => Ok(VrfOutput(output)),
        Err(_) => Err(CryptoError::VrfOutputExtractionFailed {
            detail: "proof_to_hash returned error",
        }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // Malformed inputs — pre-verification rejection

    #[test]
    fn malformed_proof_wrong_length() {
        let result = VrfProof::from_bytes(&[0u8; 79]);
        assert_eq!(
            result,
            Err(CryptoError::MalformedProof {
                detail: "expected 80 bytes",
            })
        );
    }

    #[test]
    fn malformed_proof_too_long() {
        let result = VrfProof::from_bytes(&[0u8; 81]);
        assert_eq!(
            result,
            Err(CryptoError::MalformedProof {
                detail: "expected 80 bytes",
            })
        );
    }

    #[test]
    fn malformed_key_wrong_length() {
        let result = VrfVerificationKey::from_bytes(&[0u8; 31]);
        assert_eq!(
            result,
            Err(CryptoError::MalformedKey {
                algorithm: "vrf_ecvrf",
                detail: "expected 32 bytes",
            })
        );
    }

    // Invalid proof — correct lengths, rejected by verifier
    #[test]
    fn invalid_proof_rejected() {
        let vk = VrfVerificationKey([0u8; 32]);
        let proof = VrfProof([0u8; 80]);
        let result = verify_vrf(&vk, &proof, b"test alpha");
        assert!(matches!(
            result,
            Err(CryptoError::VerificationFailed {
                algorithm: "vrf_ecvrf"
            })
        ));
    }

    // Self-consistency: generate a proof and verify it
    #[test]
    fn generate_and_verify() {
        use cardano_crypto::vrf::VrfDraft03;

        let seed = [42u8; 32];
        let (sk, vk_bytes) = VrfDraft03::keypair_from_seed(&seed);

        let alpha = b"Cardano block slot 12345";
        let proof_bytes = VrfDraft03::prove(&sk, alpha).unwrap();

        let vk = VrfVerificationKey(vk_bytes);
        let proof = VrfProof(proof_bytes);

        let result = verify_vrf(&vk, &proof, alpha);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_ne!(output.0, [0u8; 64]);
    }

    // Determinism: same inputs always produce same result
    #[test]
    fn verification_is_deterministic() {
        use cardano_crypto::vrf::VrfDraft03;

        let seed = [99u8; 32];
        let (sk, vk_bytes) = VrfDraft03::keypair_from_seed(&seed);
        let alpha = b"determinism test";
        let proof_bytes = VrfDraft03::prove(&sk, alpha).unwrap();

        let vk = VrfVerificationKey(vk_bytes);
        let proof = VrfProof(proof_bytes);

        let r1 = verify_vrf(&vk, &proof, alpha);
        let r2 = verify_vrf(&vk, &proof, alpha);
        assert_eq!(r1, r2);
    }

    // Output bit-identical between verify and proof_to_hash
    #[test]
    fn verify_and_proof_to_hash_match() {
        use cardano_crypto::vrf::VrfDraft03;

        let seed = [7u8; 32];
        let (sk, vk_bytes) = VrfDraft03::keypair_from_seed(&seed);
        let alpha = b"output consistency test";
        let proof_bytes = VrfDraft03::prove(&sk, alpha).unwrap();

        let vk = VrfVerificationKey(vk_bytes);
        let proof = VrfProof(proof_bytes);

        let verify_output = verify_vrf(&vk, &proof, alpha).unwrap();
        let hash_output = vrf_proof_to_hash(&proof).unwrap();

        assert_eq!(verify_output, hash_output);
    }

    // No retry: verification failure is terminal
    #[test]
    fn no_retry_on_failure() {
        let vk = VrfVerificationKey([1u8; 32]);
        let proof = VrfProof([1u8; 80]);
        let r1 = verify_vrf(&vk, &proof, b"alpha");
        let r2 = verify_vrf(&vk, &proof, b"alpha");
        assert_eq!(r1, r2);
        assert!(matches!(r1, Err(CryptoError::VerificationFailed { .. })));
    }

    // Wrong key — valid proof for different key rejected
    #[test]
    fn wrong_key_rejected() {
        use cardano_crypto::vrf::VrfDraft03;

        let seed1 = [1u8; 32];
        let seed2 = [2u8; 32];
        let (sk1, _) = VrfDraft03::keypair_from_seed(&seed1);
        let (_, vk2_bytes) = VrfDraft03::keypair_from_seed(&seed2);

        let alpha = b"wrong key test";
        let proof_bytes = VrfDraft03::prove(&sk1, alpha).unwrap();

        let vk2 = VrfVerificationKey(vk2_bytes);
        let proof = VrfProof(proof_bytes);

        let result = verify_vrf(&vk2, &proof, alpha);
        assert!(matches!(
            result,
            Err(CryptoError::VerificationFailed { .. })
        ));
    }

    // Wrong alpha — valid proof for different message rejected
    #[test]
    fn wrong_alpha_rejected() {
        use cardano_crypto::vrf::VrfDraft03;

        let seed = [3u8; 32];
        let (sk, vk_bytes) = VrfDraft03::keypair_from_seed(&seed);

        let alpha = b"correct alpha";
        let proof_bytes = VrfDraft03::prove(&sk, alpha).unwrap();

        let vk = VrfVerificationKey(vk_bytes);
        let proof = VrfProof(proof_bytes);

        let result = verify_vrf(&vk, &proof, b"wrong alpha");
        assert!(matches!(
            result,
            Err(CryptoError::VerificationFailed { .. })
        ));
    }
}
