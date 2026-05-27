// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! `period_from_zeroed_sum6_tree_shape` — period inference from
//! 608-byte expanded Sum6KES skey payloads (PHASE4-N-P S3).
//!
//! Implements the function specified in
//! [`docs/clusters/PHASE4-N-P/period-from-zeroed-sum6-tree-shape-proof.md`](../../../docs/clusters/PHASE4-N-P/period-from-zeroed-sum6-tree-shape-proof.md)
//! §3. The pseudocode there is the spec; this is the implementation.
//!
//! Walks levels 6 → 5 → … → 1, accumulating `2^(level-1)` at each
//! level where the level's seed is zero (right sub-tree active).
//! Leaf-zero is fail-closed via
//! [`KesParseError::LeafSignKeyAllZero`].

use super::errors::KesParseError;

/// Compute the current period of a fresh-from-disk 608-byte
/// expanded Sum6KES signing-key payload by walking which sub-seeds
/// are zeroed.
///
/// Returns:
/// - `Ok(period)` where `period ∈ 0..=63`.
/// - `Err(KesParseError::LeafSignKeyAllZero)` if `bytes[0..32]` is
///   all zeros (exhausted or malformed key).
///
/// **Does not** perform vk-consistency checks; those live in
/// `Sum6Kes::raw_deserialize_signing_key_kes`. This function is the
/// shape-only inference; the deserializer is the structural
/// validator.
pub fn period_from_zeroed_sum6_tree_shape(
    bytes: &[u8; 608],
) -> Result<u32, KesParseError> {
    // Leaf (Sum0) lives at bytes[0..32). All-zero leaf = malformed
    // (active leaf is always a non-zero ed25519 seed).
    if bytes[0..32].iter().all(|&b| b == 0) {
        return Err(KesParseError::LeafSignKeyAllZero);
    }

    let mut period: u32 = 0;

    // Level 6: seed at [512..544). Contribution = 2^5 = 32.
    if bytes[512..544].iter().all(|&b| b == 0) {
        period += 32;
    }
    // Level 5: seed at [416..448). Contribution = 2^4 = 16.
    if bytes[416..448].iter().all(|&b| b == 0) {
        period += 16;
    }
    // Level 4: seed at [320..352). Contribution = 2^3 = 8.
    if bytes[320..352].iter().all(|&b| b == 0) {
        period += 8;
    }
    // Level 3: seed at [224..256). Contribution = 2^2 = 4.
    if bytes[224..256].iter().all(|&b| b == 0) {
        period += 4;
    }
    // Level 2: seed at [128..160). Contribution = 2^1 = 2.
    if bytes[128..160].iter().all(|&b| b == 0) {
        period += 2;
    }
    // Level 1: seed at [32..64). Contribution = 2^0 = 1.
    if bytes[32..64].iter().all(|&b| b == 0) {
        period += 1;
    }

    // Sum6KES encodes periods 0..=63 by structure; no overflow
    // possible from a 608-byte payload.
    debug_assert!(period <= 63);
    Ok(period)
}
