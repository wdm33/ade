// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Seed expansion for Sum_n KES (PHASE4-N-P S2).
//!
//! Domain-separated Blake2b256 expansion matching Haskell `cardano-base`'s
//! `expandSeed` and the upstream `cardano-crypto` 1.0.8 Rust reference:
//!
//! ```text
//! left_seed  = Blake2b256(0x00 || seed)
//! right_seed = Blake2b256(0x01 || seed)
//! ```
//!
//! The prefix bytes are byte-load-bearing — they're part of the Sum6KES
//! algorithm's canonical surface. Cross-implementation agreement
//! (DC-CRYPTO-08) requires byte-identical expansion.

use crate::blake2b::Blake2b256;
use crate::traits::HashAlgorithm;

/// Expand a 32-byte seed into the (left, right) sub-tree seeds using
/// domain-separated Blake2b256 hashes. Pure, deterministic.
pub(crate) fn expand_seed(seed: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let mut left_input = [0u8; 33];
    left_input[0] = 0x00;
    left_input[1..].copy_from_slice(seed);

    let mut right_input = [0u8; 33];
    right_input[0] = 0x01;
    right_input[1..].copy_from_slice(seed);

    (
        Blake2b256::hash(&left_input),
        Blake2b256::hash(&right_input),
    )
}

/// Hash two 32-byte chunks under Blake2b256 to produce a 32-byte vk
/// for a Sum_n internal node: `vk_n = Blake2b256(vk_left || vk_right)`.
pub(super) fn hash_concat_vk(vk_left: &[u8; 32], vk_right: &[u8; 32]) -> [u8; 32] {
    let mut input = [0u8; 64];
    input[..32].copy_from_slice(vk_left);
    input[32..].copy_from_slice(vk_right);
    Blake2b256::hash(&input)
}
