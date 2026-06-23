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

/// A UTxO OUTPUT asset quantity: the non-negative Cardano Word64 domain (0 ..= 2^64-1).
///
/// Non-negative BY CONSTRUCTION; the canonical output encoding is a CBOR unsigned
/// integer (u64); only checked add/sub is offered, so an output quantity can never
/// silently wrap or go negative; and a value of this type cannot be passed where a
/// mint/burn delta ([`MintBurnQuantity`]) is expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct OutputAssetQuantity(pub u64);

impl OutputAssetQuantity {
    pub const ZERO: Self = OutputAssetQuantity(0);

    pub fn checked_add(self, o: Self) -> Option<Self> {
        self.0.checked_add(o.0).map(OutputAssetQuantity)
    }

    pub fn checked_sub(self, o: Self) -> Option<Self> {
        self.0.checked_sub(o.0).map(OutputAssetQuantity)
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }
}

/// A mint/burn DELTA: the signed domain. DORMANT until mint decoding (S-13).
///
/// Cannot enter UTxO output state (it is never placed in a [`MultiAsset`]). Defined
/// here only to fix the future boundary so the non-negative-stored-output vs
/// signed-mint/burn distinction is explicit in the type system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MintBurnQuantity(pub i64);

/// Multi-asset bundle: PolicyId (Hash28) -> AssetName -> output quantity.
///
/// Quantities are the non-negative Cardano Word64 domain ([`OutputAssetQuantity`]):
/// this bundle is an OUTPUT representation, so a quantity cannot be negative by type.
/// Mint/burn deltas are the distinct signed [`MintBurnQuantity`] and never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiAsset(pub BTreeMap<Hash28, BTreeMap<AssetName, OutputAssetQuantity>>);

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
