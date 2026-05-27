// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Ade-owned BLUE Sum_n KES algorithm (PHASE4-N-P S2).
//!
//! Reimplements `Sum6KES Ed25519DSIGN` from first principles, matching
//! Haskell `cardano-base`'s `Sum6KES` byte-for-byte and the upstream
//! `cardano-crypto` 1.0.8 Rust reference. After PHASE4-N-P S5,
//! `KesSecret.inner` in `ade_runtime::producer::signing` uses
//! `Sum6Kes::SigningKey` defined here, and `cardano-crypto` is demoted
//! to a `#[cfg(test)]` oracle.
//!
//! S2 ships the algorithm only — no expanded-skey serde (that is S3,
//! gated by the period-from-zeroed-tree-shape proof obligation at
//! `docs/clusters/PHASE4-N-P/period-from-zeroed-sum6-tree-shape-proof.md`).
//!
//! See `docs/clusters/PHASE4-N-P/cluster.md` §1 for the primary
//! invariant and §5 for the N9 hard prohibition (no upstream-shim
//! compatibility hack via unsafe / transmute / vendored pub(crate)
//! access). cardano-crypto MUST NOT be imported in `kes_sum` outside
//! `#[cfg(test)]`.
//!
//! ### Verification-key pinning
//!
//! Every Sum_n in this chain has `VerificationKey = [u8; 32]`:
//! - `Sum0Kes` = `SingleKes<Ed25519>` — Ed25519 public key (32 bytes).
//! - `Sum_n` for `n >= 1` — Blake2b256 hash of (vk_left || vk_right).
//!
//! Pinning `[u8; 32]` directly on the trait removes a generic VK
//! type parameter and the AsRef coherence dance it required. If a
//! future Sum_n with a different hash width is ever needed (out of
//! scope for N-P), this is the place to generalize.

mod hash;
mod single;
mod sum;

#[cfg(test)]
mod tests;

pub use single::{Sum0Kes, Sum0Signature, Sum0SigningKey};
pub use sum::{SumKes, SumSignature, SumSigningKey};

// =========================================================================
// Public type aliases — the Sum_n chain anchored at Sum0 = SingleKes<Ed25519>.
// =========================================================================

pub type Sum1Kes = SumKes<Sum0Kes>;
pub type Sum2Kes = SumKes<Sum1Kes>;
pub type Sum3Kes = SumKes<Sum2Kes>;
pub type Sum4Kes = SumKes<Sum3Kes>;
pub type Sum5Kes = SumKes<Sum4Kes>;
pub type Sum6Kes = SumKes<Sum5Kes>;

// =========================================================================
// KesAlgorithm trait — the closed surface for Sum_n KES.
// =========================================================================

/// Closed BLUE surface for a Sum_n KES algorithm. Implementations are
/// pure: no I/O, no wall clock, no `HashMap`, no floats, no RNG. Every
/// function is total or returns a closed [`KesError`] variant.
///
/// Type-level invariants:
/// - `SigningKey` carries hot secret material; the implementor MUST
///   hand-roll `Drop` to best-effort zeroize the inner buffers and
///   MUST NOT expose public byte accessors.
/// - `VerificationKey` is pinned to `[u8; 32]` for this chain (Ed25519
///   pk at the leaf; Blake2b256 hash at every Sum level).
/// - `Signature` is the structured tuple `(sigma_d, vk0, vk1)` for
///   Sum levels, or a 64-byte Ed25519 signature at the leaf.
pub trait KesAlgorithm: 'static {
    /// Signing-key type. Holds hot secret material. Implementors
    /// MUST hand-roll `Drop` for best-effort zeroize.
    type SigningKey;

    /// Signature type. Structured (not raw bytes) at this BLUE
    /// layer; raw-byte serde is the PHASE4-N-P S3 deliverable.
    type Signature: Clone + Eq + core::fmt::Debug;

    /// Human-readable name (debug only; never used as a wire
    /// discriminator).
    const ALGORITHM_NAME: &'static str;

    /// Seed size in bytes (always 32 for our chain — Blake2b256
    /// output width).
    const SEED_SIZE: usize;

    /// Signing-key serialized size in bytes — the size that the
    /// PHASE4-N-P S3 `raw_serialize_signing_key_kes` will emit.
    /// Sum0 = 32, Sum_n = Sum_(n-1) + 96.
    const SIGNING_KEY_SIZE: usize;

    /// Signature serialized size in bytes. Sum0 = 64, Sum_n =
    /// Sum_(n-1) + 64.
    const SIGNATURE_SIZE: usize;

    /// Verification-key size in bytes (always 32 for our chain).
    const VERIFICATION_KEY_SIZE: usize = 32;

    /// Total number of periods this algorithm supports.
    /// Sum0 = 1; Sum_n = 2 * Sum_(n-1). Sum6 = 64.
    fn total_periods() -> u32;

    /// Construct a fresh signing key from a 32-byte seed at period
    /// 0. Returns `KesError::InvalidSeedLength` if the seed is the
    /// wrong size.
    fn gen_key_kes_from_seed_bytes(seed: &[u8]) -> Result<Self::SigningKey, KesError>;

    /// Derive the verification key from a signing key. Pure; non-
    /// secret output.
    fn derive_verification_key(sk: &Self::SigningKey) -> [u8; 32];

    /// Evolve the signing key to the next period. `period` is the
    /// **current** period of `sk`; the returned key is at
    /// `period + 1`.
    ///
    /// - `Ok(Some(new_sk))` — successful evolution.
    /// - `Ok(None)` — key has reached its final period and cannot
    ///   evolve further.
    /// - `Err(KesError::PeriodOutOfRange)` — `period` is past the
    ///   algorithm's `total_periods()`.
    fn update_kes(
        sk: Self::SigningKey,
        period: u32,
    ) -> Result<Option<Self::SigningKey>, KesError>;

    /// Sign `msg` at `period` using `sk`. Returns
    /// `KesError::PeriodOutOfRange` if `period >= total_periods()`.
    fn sign_kes(
        sk: &Self::SigningKey,
        period: u32,
        msg: &[u8],
    ) -> Result<Self::Signature, KesError>;

    /// Verify a `Sum_n` signature. Returns
    /// `KesError::VerificationFailed` on any mismatch (recursive
    /// VK hash mismatch, Ed25519 verification failure, period out
    /// of range, etc.).
    fn verify_kes(
        vk: &[u8; 32],
        period: u32,
        msg: &[u8],
        sig: &Self::Signature,
    ) -> Result<(), KesError>;
}

// =========================================================================
// Closed runtime-error surface (S2). Parse-time errors land in S3 via the
// separate `KesParseError` enum.
// =========================================================================

/// Closed error surface for runtime Sum_n operations. Variants carry
/// only non-secret metadata — period numbers, expected/actual lengths,
/// algorithm names. Never key bytes / seeds / hex representations of
/// secret material.
#[derive(Debug, PartialEq, Eq)]
pub enum KesError {
    /// Seed slice was not exactly `SEED_SIZE` bytes.
    InvalidSeedLength {
        expected: usize,
        actual: usize,
    },
    /// `period >= total_periods()` for the algorithm.
    PeriodOutOfRange {
        period: u32,
        max_period: u32,
    },
    /// Signature failed verification — recursive VK hash mismatch or
    /// leaf Ed25519 check failed.
    VerificationFailed,
    /// `update_kes` was called on a key already at its final period.
    KeyExpired,
    /// Ed25519-dalek rejected a structurally invalid input. Detail is
    /// a static literal; never contains key bytes.
    Ed25519(&'static str),
}

// Compile-time size verification: confirm Sum_n sizes match the
// recurrence. Off by one byte at any level is a forge-validity defect.
const _: () = {
    assert!(Sum0Kes::SIGNING_KEY_SIZE == 32);
    assert!(Sum0Kes::SIGNATURE_SIZE == 64);
    assert!(Sum0Kes::VERIFICATION_KEY_SIZE == 32);
    assert!(Sum0Kes::SEED_SIZE == 32);
    // Sum1 = 32 + 96 = 128
    assert!(Sum1Kes::SIGNING_KEY_SIZE == 128);
    assert!(Sum1Kes::SIGNATURE_SIZE == 128);
    // Sum6 = 32 + 6 * 96 = 608
    assert!(Sum6Kes::SIGNING_KEY_SIZE == 608);
    assert!(Sum6Kes::SIGNATURE_SIZE == 448);
    assert!(Sum6Kes::VERIFICATION_KEY_SIZE == 32);
};
