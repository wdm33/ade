// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeMap;
use ade_types::mary::value::OutputAssetQuantity;
use ade_types::tx::Coin;
use ade_types::Hash28;
use crate::error::{AssetUnderflowError, ConservationError, LedgerError};

/// Asset name (0-32 bytes).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetName(pub Vec<u8>);

/// Multi-asset bundle: PolicyId → AssetName → output quantity.
///
/// Quantities are the non-negative Cardano Word64 domain
/// (`ade_types::mary::value::OutputAssetQuantity`): this is an OUTPUT
/// representation, so an asset quantity cannot be negative by type, and output
/// arithmetic is checked (overflow/underflow → a structured `LedgerError`).
/// Mint/burn deltas are the distinct signed `MintBurnQuantity` and never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiAsset(pub BTreeMap<Hash28, BTreeMap<AssetName, OutputAssetQuantity>>);

impl MultiAsset {
    pub fn new() -> Self {
        MultiAsset(BTreeMap::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Remove all entries whose quantity is exactly zero, then drop emptied policies.
    ///
    /// Canonical Cardano value normalization: the specified empty-bundle form drops a
    /// zero-quantity asset rather than carrying it. This is NOT a silent deletion of
    /// value — it only ever runs after a CHECKED add/sub has produced an exact zero.
    pub fn prune_zeros(&mut self) {
        self.0.retain(|_, assets| {
            assets.retain(|_, qty| !qty.is_zero());
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
/// Subtracts coin amounts and per-policy per-asset output quantities. Coin underflow
/// returns `ConservationError`. An output asset quantity underflow (subtrahend qty >
/// minuend qty) returns `AssetUnderflow` — it never wraps and never produces a
/// negative entry (output quantities are the non-negative Word64 domain).
pub fn value_sub(a: &Value, b: &Value) -> Result<Value, LedgerError> {
    let coin = a.coin.checked_sub(b.coin).ok_or(
        LedgerError::Conservation(ConservationError {
            consumed_coin: a.coin,
            produced_coin: b.coin,
        })
    )?;

    let multi_asset = multi_asset_sub(&a.multi_asset, &b.multi_asset)?;

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

// ---------------------------------------------------------------------------
// Multi-asset arithmetic
// ---------------------------------------------------------------------------

fn multi_asset_add(a: &MultiAsset, b: &MultiAsset) -> Result<MultiAsset, LedgerError> {
    let mut result = a.0.clone();

    for (policy, b_assets) in &b.0 {
        let entry = result.entry(policy.clone()).or_default();
        for (name, qty) in b_assets {
            let current = entry.entry(name.clone()).or_insert(OutputAssetQuantity::ZERO);
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

fn multi_asset_sub(a: &MultiAsset, b: &MultiAsset) -> Result<MultiAsset, LedgerError> {
    let mut result = a.0.clone();

    for (policy, b_assets) in &b.0 {
        let entry = result.entry(policy.clone()).or_default();
        for (name, qty) in b_assets {
            let current = entry.entry(name.clone()).or_insert(OutputAssetQuantity::ZERO);
            // Checked output subtraction: a subtrahend quantity larger than the
            // minuend is a structured authoritative underflow, never a wrap to a
            // huge value and never a negative entry.
            *current = current.checked_sub(*qty).ok_or_else(|| {
                LedgerError::AssetUnderflow(AssetUnderflowError {
                    policy: policy.clone(),
                    name: name.0.clone(),
                })
            })?;
        }
    }

    let mut ma = MultiAsset(result);
    ma.prune_zeros();
    Ok(ma)
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

    fn ma_one(policy: Hash28, name: AssetName, qty: OutputAssetQuantity) -> MultiAsset {
        let mut inner = BTreeMap::new();
        inner.insert(name, qty);
        let mut outer = BTreeMap::new();
        outer.insert(policy, inner);
        MultiAsset(outer)
    }

    #[test]
    fn multi_asset_add_and_sub() {
        let policy = Hash28([0xaa; 28]);
        let name = AssetName(b"token".to_vec());

        let a = Value {
            coin: coin(1000),
            multi_asset: ma_one(policy.clone(), name.clone(), OutputAssetQuantity(100)),
        };
        let b = Value {
            coin: coin(500),
            multi_asset: ma_one(policy.clone(), name.clone(), OutputAssetQuantity(50)),
        };

        let sum = value_add(&a, &b).unwrap();
        assert_eq!(sum.coin, coin(1500));
        assert_eq!(sum.multi_asset.0[&policy][&name], OutputAssetQuantity(150));

        let diff = value_sub(&sum, &b).unwrap();
        assert_eq!(diff, a);
    }

    #[test]
    fn zero_quantity_pruned() {
        let policy = Hash28([0xbb; 28]);
        let name = AssetName(b"tok".to_vec());
        let ma = ma_one(policy, name, OutputAssetQuantity(100));

        let a = Value {
            coin: coin(100),
            multi_asset: ma.clone(),
        };
        let b = Value {
            coin: coin(0),
            multi_asset: ma,
        };

        let result = value_sub(&a, &b).unwrap();
        // 100 - 100 = 0, should be pruned to the canonical empty-bundle form.
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
    fn multi_asset_sub_underflow_returns_asset_underflow() {
        // Subtracting more of an asset than is present is a structured authoritative
        // underflow — never a wrap to a huge u64 and never a negative entry.
        let policy = Hash28([0xcc; 28]);
        let name = AssetName(b"tok".to_vec());
        let a = Value {
            coin: coin(100),
            multi_asset: ma_one(policy.clone(), name.clone(), OutputAssetQuantity(5)),
        };
        let b = Value {
            coin: coin(0),
            multi_asset: ma_one(policy.clone(), name.clone(), OutputAssetQuantity(6)),
        };
        match value_sub(&a, &b) {
            Err(LedgerError::AssetUnderflow(e)) => {
                assert_eq!(e.policy, policy);
                assert_eq!(e.name, name.0);
            }
            other => panic!("expected AssetUnderflow, got {other:?}"),
        }
    }

    #[test]
    fn multi_asset_add_overflow_returns_error() {
        // u64::MAX + 1 of an asset overflows the Word64 domain → structured reject,
        // never a wrap.
        let policy = Hash28([0xdd; 28]);
        let name = AssetName(b"tok".to_vec());
        let a = Value {
            coin: Coin::ZERO,
            multi_asset: ma_one(policy.clone(), name.clone(), OutputAssetQuantity(u64::MAX)),
        };
        let b = Value {
            coin: Coin::ZERO,
            multi_asset: ma_one(policy, name, OutputAssetQuantity(1)),
        };
        assert!(matches!(value_add(&a, &b), Err(LedgerError::Conservation(_))));
    }

    #[test]
    fn multi_asset_word64_add_sub_round_trips_above_i64_max() {
        // The upper-half Word64 domain is representable and arithmetic is exact:
        // (i64::MAX+1) + 1 - 1 == (i64::MAX+1), with no wrap anywhere.
        let policy = Hash28([0xee; 28]);
        let name = AssetName(b"big".to_vec());
        let base = OutputAssetQuantity(i64::MAX as u64 + 1);
        let a = Value {
            coin: Coin::ZERO,
            multi_asset: ma_one(policy.clone(), name.clone(), base),
        };
        let one = Value {
            coin: Coin::ZERO,
            multi_asset: ma_one(policy.clone(), name.clone(), OutputAssetQuantity(1)),
        };
        let sum = value_add(&a, &one).unwrap();
        assert_eq!(
            sum.multi_asset.0[&policy][&name],
            OutputAssetQuantity(i64::MAX as u64 + 2)
        );
        let back = value_sub(&sum, &one).unwrap();
        assert_eq!(back.multi_asset.0[&policy][&name], base);
    }

    #[test]
    fn negative_output_quantity_is_unrepresentable() {
        // Type-level guarantee: an output quantity is u64, so a negative literal
        // does not compile. The closest constructible boundary is 0; assert it is
        // the canonical zero (and that the signed domain lives in MintBurnQuantity).
        let zero = OutputAssetQuantity::default();
        assert_eq!(zero, OutputAssetQuantity::ZERO);
        assert!(zero.is_zero());
        // MintBurnQuantity carries the signed domain and is NOT a MultiAsset value
        // type (this line would not compile if a MultiAsset accepted it).
        let _burn = ade_types::mary::value::MintBurnQuantity(-1);
    }
}
