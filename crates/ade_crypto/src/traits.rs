// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

// Library isolation trait for hash backends.
//
// HashAlgorithm wraps the concrete hash implementation so that ade_crypto
// is the single authoritative source for hashing and the backend can be
// swapped without changing any call sites.

/// Trait abstracting a hash algorithm with a fixed output size.
///
/// Implementors are infallible on arbitrary `&[u8]` input.
pub trait HashAlgorithm {
    /// Fixed-size output type (e.g., `[u8; 32]` for Blake2b-256).
    type Output: AsRef<[u8]>;

    /// Compute the hash of `data`.
    fn hash(data: &[u8]) -> Self::Output;
}
