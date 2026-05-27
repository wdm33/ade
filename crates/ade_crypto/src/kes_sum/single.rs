// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Sum0KES = `SingleKES Ed25519DSIGN` (PHASE4-N-P S2).
//!
//! The leaf KES: a single Ed25519 signing key, valid for exactly one
//! period (period 0). Signing / verification delegates to
//! `ed25519-dalek` without going through `cardano-crypto`. After
//! PHASE4-N-P, the Sum_n chain bottoms out here.

use super::KesAlgorithm;
use super::KesError;

// =========================================================================
// Sum0Kes — single-period KES wrapping Ed25519
// =========================================================================

/// Sum0KES = single-period KES at the leaf. Period 0 only.
pub struct Sum0Kes;

/// Sum0KES signing key — the 32-byte Ed25519 seed. Hand-rolled `Drop`
/// best-effort zeroizes on drop. No public byte accessors.
pub struct Sum0SigningKey {
    seed: [u8; 32],
}

impl Sum0SigningKey {
    /// Construct from a 32-byte slice. Returns
    /// `KesError::InvalidSeedLength` otherwise. The input is copied
    /// in; callers should also zeroize their copy.
    pub fn from_seed(seed: &[u8]) -> Result<Self, KesError> {
        if seed.len() != 32 {
            return Err(KesError::InvalidSeedLength {
                expected: 32,
                actual: seed.len(),
            });
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(seed);
        Ok(Self { seed: arr })
    }

}

impl Drop for Sum0SigningKey {
    fn drop(&mut self) {
        for b in self.seed.iter_mut() {
            *b = 0;
        }
        core::hint::black_box(&mut self.seed);
    }
}

impl core::fmt::Debug for Sum0SigningKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Sum0SigningKey(<redacted>)")
    }
}

/// Sum0KES signature = 64-byte Ed25519 signature.
#[derive(Clone, PartialEq, Eq)]
pub struct Sum0Signature {
    pub(super) bytes: [u8; 64],
}

impl Sum0Signature {
    /// Construct from a 64-byte slice. Returns
    /// `KesError::Ed25519("expected 64 bytes")` otherwise. Used by
    /// the recursive `SumKes::verify_kes` when reconstructing leaf
    /// signatures from raw bytes — not currently called outside
    /// tests and S3's deserializer.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KesError> {
        if bytes.len() != 64 {
            return Err(KesError::Ed25519("expected 64-byte signature"));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(bytes);
        Ok(Self { bytes: arr })
    }

    /// Borrow the 64-byte raw signature. Public — signatures are
    /// non-secret. Used by S3's `raw_serialize_signature_kes`.
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.bytes
    }
}

impl core::fmt::Debug for Sum0Signature {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Sum0Signature(64 bytes)")
    }
}

// =========================================================================
// KesAlgorithm impl
// =========================================================================

impl KesAlgorithm for Sum0Kes {
    type SigningKey = Sum0SigningKey;
    type Signature = Sum0Signature;

    const ALGORITHM_NAME: &'static str = "Sum0KES_Ed25519DSIGN";
    const SEED_SIZE: usize = 32;
    const SIGNING_KEY_SIZE: usize = 32;
    const SIGNATURE_SIZE: usize = 64;

    fn total_periods() -> u32 {
        1
    }

    fn gen_key_kes_from_seed_bytes(seed: &[u8]) -> Result<Self::SigningKey, KesError> {
        Sum0SigningKey::from_seed(seed)
    }

    fn derive_verification_key(sk: &Self::SigningKey) -> [u8; 32] {
        // ed25519-dalek 2.x: SigningKey::from_bytes is total over [u8; 32].
        let dalek_sk = ed25519_dalek::SigningKey::from_bytes(&sk.seed);
        dalek_sk.verifying_key().to_bytes()
    }

    fn sign_kes(
        sk: &Self::SigningKey,
        period: u32,
        msg: &[u8],
    ) -> Result<Self::Signature, KesError> {
        if period != 0 {
            return Err(KesError::PeriodOutOfRange {
                period,
                max_period: 0,
            });
        }
        let dalek_sk = ed25519_dalek::SigningKey::from_bytes(&sk.seed);
        // ed25519-dalek's `sign` is pure (RFC 8032 deterministic).
        use ed25519_dalek::Signer;
        let sig = dalek_sk.sign(msg);
        Ok(Sum0Signature {
            bytes: sig.to_bytes(),
        })
    }

    fn verify_kes(
        vk: &[u8; 32],
        period: u32,
        msg: &[u8],
        sig: &Self::Signature,
    ) -> Result<(), KesError> {
        if period != 0 {
            return Err(KesError::PeriodOutOfRange {
                period,
                max_period: 0,
            });
        }
        let dalek_vk = ed25519_dalek::VerifyingKey::from_bytes(vk)
            .map_err(|_| KesError::Ed25519("invalid verification key point"))?;
        let dalek_sig = ed25519_dalek::Signature::from_bytes(&sig.bytes);
        use ed25519_dalek::Verifier;
        dalek_vk
            .verify(msg, &dalek_sig)
            .map_err(|_| KesError::VerificationFailed)
    }

    fn update_kes(
        sk: Self::SigningKey,
        period: u32,
    ) -> Result<Option<Self::SigningKey>, KesError> {
        if period >= Self::total_periods() {
            return Err(KesError::PeriodOutOfRange {
                period,
                max_period: Self::total_periods() - 1,
            });
        }
        // Sum0 only has period 0; advancing from 0 exhausts the key.
        // `sk` is moved in and dropped here, which zeroizes the seed.
        drop(sk);
        Ok(None)
    }
}
