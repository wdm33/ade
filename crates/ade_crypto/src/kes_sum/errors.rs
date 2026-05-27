// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Closed parse-time error surface for the Sum_n KES serde
//! (PHASE4-N-P S3).
//!
//! Per the S1 proof obligation
//! ([`period-from-zeroed-sum6-tree-shape-proof.md`](../../../docs/clusters/PHASE4-N-P/period-from-zeroed-sum6-tree-shape-proof.md)
//! §5), every variant carries only non-secret metadata: `level:
//! u32` and `actual: usize`. No raw key bytes; no hex; no decimal
//! seed runs.

/// Closed error surface for parsing 608-byte expanded Sum_n
/// signing-key payloads and 448-byte signatures.
///
/// `Debug` is derived because every variant payload is a non-secret
/// primitive (`u32` / `usize`); no manual redaction needed.
#[derive(Debug, PartialEq, Eq)]
pub enum KesParseError {
    /// Payload size does not match the canonical algorithm size
    /// (608 bytes for a Sum6 signing key; 448 bytes for a Sum6
    /// signature; analogously for sub-Sum sizes during recursion).
    /// `actual` is the observed buffer length.
    WrongPayloadSize { actual: usize },

    /// Leaf Ed25519 signing-key seed is all zeros. Either an
    /// exhausted key or a malformed payload; both fail-closed.
    LeafSignKeyAllZero,

    /// At level `level`, `vk_left` from the bytes does not match the
    /// recursively-derived sub-tree VK. Only verifiable when the
    /// level's seed is non-zero (left sub-tree active); when the
    /// seed is zero the original left sub-tree is forward-secrecy-
    /// gone and `vk_left` is trusted as given.
    InconsistentSubtreeVkLeft { level: u32 },

    /// At level `level`, `vk_right` from the bytes does not match
    /// the recursively-derived sub-tree VK (from `r1_seed` when
    /// left sub-tree active; from the current child when right
    /// sub-tree active).
    InconsistentSubtreeVkRight { level: u32 },

    /// `level` is outside `1..=6` for a Sum6 deserialization.
    /// Defense-in-depth; should not arise from a 608-byte payload
    /// because the offsets are fixed.
    LevelOutOfRange { level: u32 },

    /// Ed25519 signature inside a Sum_n signature was not 64
    /// bytes (only arises during signature deserialization).
    InvalidEd25519SignatureLength { actual: usize },
}
