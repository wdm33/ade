// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeMap;
use crate::Hash28;

/// Asset name (0-32 bytes).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetName(pub Vec<u8>);

/// Multi-asset bundle: PolicyId (Hash28) -> AssetName -> quantity.
///
/// Quantities are i64 to support minting (positive) and burning (negative)
/// in the mint field. Output quantities must be non-negative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiAsset(pub BTreeMap<Hash28, BTreeMap<AssetName, i64>>);

impl MultiAsset {
    pub fn new() -> Self {
        MultiAsset(BTreeMap::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for MultiAsset {
    fn default() -> Self {
        Self::new()
    }
}

/// Cardano value: coin + optional multi-asset bundle.
///
/// In the Mary era, transaction outputs can carry native tokens alongside ADA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    /// Lovelace amount.
    pub coin: u64,
    /// Multi-asset bundle (may be empty for pure-ADA outputs).
    pub multi_asset: MultiAsset,
}

/// 28-byte policy ID hash.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PolicyId(pub Hash28);
