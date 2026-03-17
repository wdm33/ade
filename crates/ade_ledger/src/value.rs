// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeMap;
use ade_types::tx::Coin;
use ade_types::Hash28;
use crate::error::{ConservationError, LedgerError, NegativeValueError};

/// Asset name (0-32 bytes).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetName(pub Vec<u8>);

/// Multi-asset bundle: PolicyId → AssetName → quantity.
///
/// Quantities are i64 to support minting (positive) and burning (negative)
/// in intermediate calculations. Final output values must be non-negative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiAsset(pub BTreeMap<Hash28, BTreeMap<AssetName, i64>>);

impl MultiAsset {
    pub fn new() -> Self {
        MultiAsset(BTreeMap::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Remove all entries with zero quantity. Removes empty policies.
    pub fn prune_zeros(&mut self) {
        self.0.retain(|_, assets| {
            assets.retain(|_, qty| *qty != 0);
            !assets.is_empty()
        });
    }
}

impl Default for MultiAsset {
    fn default() -> Self {
        Self::new()
    }
}

/// Cardano value: coin + optional multi-asset bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    pub coin: Coin,
    pub multi_asset: MultiAsset,
}

impl Value {
    pub fn from_coin(coin: Coin) -> Self {
        Value {
            coin,
            multi_asset: MultiAsset::new(),
        }
    }
}

/// Overflow-checked value addition.
///
/// Adds coin amounts and per-policy per-asset quantities.
pub fn value_add(a: &Value, b: &Value) -> Result<Value, LedgerError> {
    let coin = a.coin.checked_add(b.coin).ok_or(
        LedgerError::Conservation(ConservationError {
            consumed_coin: a.coin,
            produced_coin: b.coin,
        })
    )?;

    let multi_asset = multi_asset_add(&a.multi_asset, &b.multi_asset)?;

    Ok(Value { coin, multi_asset })
}

/// Underflow-checked value subtraction.
///
/// Subtracts coin amounts and per-policy per-asset quantities.
/// Coin underflow returns ConservationError. Asset quantities can go negative
/// (for intermediate mint/burn calculations).
pub fn value_sub(a: &Value, b: &Value) -> Result<Value, LedgerError> {
    let coin = a.coin.checked_sub(b.coin).ok_or(
        LedgerError::Conservation(ConservationError {
            consumed_coin: a.coin,
            produced_coin: b.coin,
        })
    )?;

    let multi_asset = multi_asset_sub(&a.multi_asset, &b.multi_asset);

    Ok(Value { coin, multi_asset })
}

/// Check that consumed == produced + fee.
pub fn check_conservation(
    consumed: &Value,
    produced: &Value,
    fee: Coin,
) -> Result<(), LedgerError> {
    // Check coin conservation: consumed == produced + fee
    let produced_plus_fee = produced.coin.checked_add(fee).ok_or(
        LedgerError::Conservation(ConservationError {
            consumed_coin: consumed.coin,
            produced_coin: produced.coin,
        })
    )?;

    if consumed.coin != produced_plus_fee {
        return Err(LedgerError::Conservation(ConservationError {
            consumed_coin: consumed.coin,
            produced_coin: produced_plus_fee,
        }));
    }

    // Multi-asset conservation: consumed_ma == produced_ma (no fee for native assets)
    if consumed.multi_asset != produced.multi_asset {
        return Err(LedgerError::Conservation(ConservationError {
            consumed_coin: consumed.coin,
            produced_coin: produced_plus_fee,
        }));
    }

    Ok(())
}

/// Check that a value has no negative coin or asset quantities.
pub fn check_non_negative(value: &Value) -> Result<(), LedgerError> {
    // Coin is u64 so always >= 0

    for assets in value.multi_asset.0.values() {
        for qty in assets.values() {
            if *qty < 0 {
                return Err(LedgerError::NegativeValue(NegativeValueError {
                    coin: value.coin,
                }));
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Multi-asset arithmetic
// ---------------------------------------------------------------------------

fn multi_asset_add(a: &MultiAsset, b: &MultiAsset) -> Result<MultiAsset, LedgerError> {
    let mut result = a.0.clone();

    for (policy, b_assets) in &b.0 {
        let entry = result.entry(policy.clone()).or_default();
        for (name, qty) in b_assets {
            let current = entry.entry(name.clone()).or_insert(0);
            *current = current.checked_add(*qty).ok_or(
                LedgerError::Conservation(ConservationError {
                    consumed_coin: Coin(0),
                    produced_coin: Coin(0),
                })
            )?;
        }
    }

    let mut ma = MultiAsset(result);
    ma.prune_zeros();
    Ok(ma)
}

fn multi_asset_sub(a: &MultiAsset, b: &MultiAsset) -> MultiAsset {
    let mut result = a.0.clone();

    for (policy, b_assets) in &b.0 {
        let entry = result.entry(policy.clone()).or_default();
        for (name, qty) in b_assets {
            let current = entry.entry(name.clone()).or_insert(0);
            *current -= qty;
        }
    }

    let mut ma = MultiAsset(result);
    ma.prune_zeros();
    ma
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn coin(n: u64) -> Coin {
        Coin(n)
    }

    fn pure_coin(n: u64) -> Value {
        Value::from_coin(coin(n))
    }

    #[test]
    fn add_pure_coin() {
        let a = pure_coin(100);
        let b = pure_coin(200);
        let result = value_add(&a, &b).unwrap();
        assert_eq!(result.coin, coin(300));
        assert!(result.multi_asset.is_empty());
    }

    #[test]
    fn sub_pure_coin() {
        let a = pure_coin(300);
        let b = pure_coin(100);
        let result = value_sub(&a, &b).unwrap();
        assert_eq!(result.coin, coin(200));
    }

    #[test]
    fn sub_underflow_returns_conservation_error() {
        let a = pure_coin(50);
        let b = pure_coin(100);
        let result = value_sub(&a, &b);
        assert!(matches!(result, Err(LedgerError::Conservation(_))));
    }

    #[test]
    fn add_overflow_returns_error() {
        let a = pure_coin(u64::MAX);
        let b = pure_coin(1);
        let result = value_add(&a, &b);
        assert!(matches!(result, Err(LedgerError::Conservation(_))));
    }

    #[test]
    fn conservation_check_passes() {
        let consumed = pure_coin(1_000_000);
        let produced = pure_coin(800_000);
        let fee = coin(200_000);
        assert!(check_conservation(&consumed, &produced, fee).is_ok());
    }

    #[test]
    fn conservation_check_fails() {
        let consumed = pure_coin(1_000_000);
        let produced = pure_coin(800_000);
        let fee = coin(100_000); // too low
        assert!(check_conservation(&consumed, &produced, fee).is_err());
    }

    #[test]
    fn round_trip_add_sub() {
        let a = pure_coin(500);
        let b = pure_coin(300);
        let sum = value_add(&a, &b).unwrap();
        let diff = value_sub(&sum, &b).unwrap();
        assert_eq!(diff, a);
    }

    #[test]
    fn multi_asset_add_and_sub() {
        let policy = Hash28([0xaa; 28]);
        let name = AssetName(b"token".to_vec());

        let mut ma_a = BTreeMap::new();
        let mut inner_a = BTreeMap::new();
        inner_a.insert(name.clone(), 100i64);
        ma_a.insert(policy.clone(), inner_a);

        let mut ma_b = BTreeMap::new();
        let mut inner_b = BTreeMap::new();
        inner_b.insert(name.clone(), 50i64);
        ma_b.insert(policy.clone(), inner_b);

        let a = Value {
            coin: coin(1000),
            multi_asset: MultiAsset(ma_a),
        };
        let b = Value {
            coin: coin(500),
            multi_asset: MultiAsset(ma_b),
        };

        let sum = value_add(&a, &b).unwrap();
        assert_eq!(sum.coin, coin(1500));
        assert_eq!(sum.multi_asset.0[&policy][&name], 150);

        let diff = value_sub(&sum, &b).unwrap();
        assert_eq!(diff, a);
    }

    #[test]
    fn zero_quantity_pruned() {
        let policy = Hash28([0xbb; 28]);
        let name = AssetName(b"tok".to_vec());

        let mut ma = BTreeMap::new();
        let mut inner = BTreeMap::new();
        inner.insert(name.clone(), 100i64);
        ma.insert(policy.clone(), inner);

        let a = Value {
            coin: coin(100),
            multi_asset: MultiAsset(ma.clone()),
        };
        let b = Value {
            coin: coin(0),
            multi_asset: MultiAsset(ma),
        };

        let result = value_sub(&a, &b).unwrap();
        // 100 - 100 = 0, should be pruned
        assert!(result.multi_asset.is_empty());
    }

    #[test]
    fn empty_value_arithmetic() {
        let a = Value::from_coin(Coin::ZERO);
        let b = Value::from_coin(Coin::ZERO);
        let sum = value_add(&a, &b).unwrap();
        assert_eq!(sum.coin, Coin::ZERO);
        assert!(sum.multi_asset.is_empty());
    }

    #[test]
    fn check_non_negative_passes_for_positive() {
        let policy = Hash28([0xcc; 28]);
        let name = AssetName(b"pos".to_vec());
        let mut ma = BTreeMap::new();
        let mut inner = BTreeMap::new();
        inner.insert(name, 42i64);
        ma.insert(policy, inner);

        let v = Value {
            coin: coin(100),
            multi_asset: MultiAsset(ma),
        };
        assert!(check_non_negative(&v).is_ok());
    }

    #[test]
    fn check_non_negative_fails_for_negative() {
        let policy = Hash28([0xdd; 28]);
        let name = AssetName(b"neg".to_vec());
        let mut ma = BTreeMap::new();
        let mut inner = BTreeMap::new();
        inner.insert(name, -1i64);
        ma.insert(policy, inner);

        let v = Value {
            coin: coin(100),
            multi_asset: MultiAsset(ma),
        };
        assert!(check_non_negative(&v).is_err());
    }
}
