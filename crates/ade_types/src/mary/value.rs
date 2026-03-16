// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Cardano value: either pure Lovelace or Lovelace + MultiAsset.
/// Opaque in Phase 1 — type stubs for Phase 2+.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    pub raw: Vec<u8>,
}

/// Multi-asset bundle: PolicyId → AssetName → quantity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiAsset {
    pub raw: Vec<u8>,
}

/// 28-byte policy ID hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyId(pub [u8; 28]);

/// Asset name (0-32 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetName(pub Vec<u8>);
