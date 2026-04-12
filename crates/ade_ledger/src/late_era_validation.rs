// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! State-backed late-era validation — Slice S-27 (Cluster P-A).
//!
//! Pure check functions implementing the Alonzo/Babbage/Conway ledger
//! rules that require resolved UTxO state. Each function is state-free
//! in its signature: inputs are either tx fields or resolved outputs,
//! never mutable state. Callers are responsible for resolving inputs
//! against `UTxOState` before invoking.
//!
//! All checks mirror the Haskell cardano-ledger rules exactly, per the
//! citations in `docs/active/S-27_obligation_discharge.md`. Error
//! constructors are 1:1 with the Haskell variants
//! (`BadInputsUTxO`, `InsufficientCollateral`, etc.) for future
//! wire-level agreement.

use std::collections::{BTreeMap, BTreeSet};

use ade_types::tx::{Coin, TxIn};

use crate::error::{
    BadInputsError, IncorrectTotalCollateralError, InsufficientCollateralError, LedgerError,
};

// ---------------------------------------------------------------------------
// Input resolution (O-27.3)
// ---------------------------------------------------------------------------

/// Resolve a set of inputs against the UTxO, returning all missing inputs.
///
/// Mirrors Shelley's `validateBadInputsUTxO` predicate:
/// `failureOnNonEmptySet (inputs ∖ dom utxo) BadInputsUTxO`.
///
/// Used for spend inputs (all eras), collateral inputs (Alonzo+), and
/// reference inputs (Babbage+). The Haskell ledger treats all three
/// with the same constructor; callers may merge sets and call once.
///
/// On success, returns `Ok(())`. On any missing input, returns
/// `LedgerError::BadInputs` carrying the full missing set (not just
/// the first one — mirrors Haskell's `NonEmptySet` payload).
pub fn check_inputs_present<V>(
    inputs: &BTreeSet<TxIn>,
    utxo: &BTreeMap<TxIn, V>,
) -> Result<(), LedgerError> {
    let mut missing: BTreeSet<TxIn> = BTreeSet::new();
    for tx_in in inputs {
        if !utxo.contains_key(tx_in) {
            missing.insert(tx_in.clone());
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(LedgerError::BadInputs(BadInputsError { missing }))
    }
}

// ---------------------------------------------------------------------------
// Collateral checks (O-27.1, O-27.2)
// ---------------------------------------------------------------------------

/// Enforce that the collateral inputs set is non-empty when required.
///
/// Required whenever a tx uses Plutus scripts (`script_data_hash`
/// present). Mirrors Haskell `NoCollateralInputs` from Alonzo Utxo.
pub fn check_collateral_non_empty(collateral_inputs: &BTreeSet<TxIn>) -> Result<(), LedgerError> {
    if collateral_inputs.is_empty() {
        Err(LedgerError::NoCollateralInputs)
    } else {
        Ok(())
    }
}

/// Enforce the collateral percent rule: `100 * balance >= percent * fee`.
///
/// From O-27.1 discharge:
/// - `balance = sum(collateral_inputs.coin) − collateral_return.coin` (Babbage+)
/// - `percent` = protocol parameter `collateralPercentage` (mainnet: 150)
/// - `fee` = tx body fee field
///
/// Implementation uses `i128` cross-multiplication — no division, no
/// rounding in the predicate. Matches the Haskell `Val.scale`-based
/// check exactly. Overflow-safe for any well-typed `u64` fee because
/// `u64 * u16` fits in `i128` with room to spare.
///
/// The `required` field of the error payload is reporting-only,
/// computed as `ceiling((percent * fee) / 100)`. The validity
/// decision itself never rounds.
pub fn check_collateral_percent(
    balance: i128,
    percent: u16,
    fee: Coin,
) -> Result<(), LedgerError> {
    let fee_lovelace = fee.0 as i128;
    let percent_i128 = percent as i128;
    // 100 * balance >= percent * fee
    let lhs = balance.saturating_mul(100);
    let rhs = percent_i128.saturating_mul(fee_lovelace);
    if lhs >= rhs {
        Ok(())
    } else {
        // Reporting-only ceiling of required collateral.
        let required = ceil_div_u128(
            (percent as u128).saturating_mul(fee.0 as u128),
            100u128,
        );
        Err(LedgerError::InsufficientCollateral(
            InsufficientCollateralError {
                balance,
                required: u128_to_u64_clamped(required),
                percent,
                fee: fee.0,
            },
        ))
    }
}

/// Enforce that `totalCollateral` (when declared) matches the computed balance.
///
/// From O-27.2 discharge: Babbage's `validateCollateralEqBalance`
/// requires `sum(collateral_inputs.coin) − collateral_return.coin ==
/// totalCollateral`. Pre-Babbage eras do not support `totalCollateral`;
/// callers pass `None` for those.
pub fn check_total_collateral(
    balance: i128,
    declared: Option<Coin>,
) -> Result<(), LedgerError> {
    match declared {
        None => Ok(()),
        Some(d) if balance == d.0 as i128 => Ok(()),
        Some(d) => Err(LedgerError::IncorrectTotalCollateral(
            IncorrectTotalCollateralError {
                balance,
                declared: d.0,
            },
        )),
    }
}

/// Enforce that collateral inputs contain no non-ADA assets unless a
/// collateral return output is provided that can absorb them.
///
/// From O-27.2 discharge: `validateCollateralContainsNonADA` raises
/// `CollateralContainsNonADA` when collateral inputs carry native
/// assets and no collateral return is provided (the non-ADA cannot be
/// paid as fee and would be lost).
///
/// Pre-Babbage eras cannot provide a collateral return, so this
/// function reduces to "collateral must be pure ADA" for Alonzo.
pub fn check_collateral_contains_non_ada(
    any_collateral_has_non_ada: bool,
    has_collateral_return: bool,
) -> Result<(), LedgerError> {
    if any_collateral_has_non_ada && !has_collateral_return {
        Err(LedgerError::CollateralContainsNonADA)
    } else {
        Ok(())
    }
}

/// Compute the collateral ADA balance.
///
/// `balance = sum(collateral_inputs.coin) − collateral_return.coin`.
///
/// Returns `i128` because adversarial input values could theoretically
/// sum beyond `u64::MAX`, and a negative balance (return greater than
/// inputs) is a valid error-reportable state rather than an overflow
/// panic. The Haskell `DeltaCoin` is signed `Integer`-backed for the
/// same reason.
pub fn compute_collateral_balance(
    collateral_inputs_coin_sum: u128,
    collateral_return_coin: u64,
) -> i128 {
    (collateral_inputs_coin_sum as i128) - (collateral_return_coin as i128)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ceil_div_u128(n: u128, d: u128) -> u128 {
    // d is never zero in this module's call sites (always 100).
    if d == 0 {
        return 0;
    }
    (n + d - 1) / d
}

fn u128_to_u64_clamped(v: u128) -> u64 {
    if v > u64::MAX as u128 {
        u64::MAX
    } else {
        v as u64
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_types::Hash32;

    fn tx_in(hash_byte: u8, index: u16) -> TxIn {
        TxIn {
            tx_hash: Hash32([hash_byte; 32]),
            index,
        }
    }

    // -----------------------------------------------------------------------
    // check_inputs_present (O-27.3)
    // -----------------------------------------------------------------------

    #[test]
    fn inputs_present_empty_set_passes() {
        let utxo: BTreeMap<TxIn, ()> = BTreeMap::new();
        let inputs: BTreeSet<TxIn> = BTreeSet::new();
        assert!(check_inputs_present(&inputs, &utxo).is_ok());
    }

    #[test]
    fn inputs_present_all_resolved_passes() {
        let mut utxo = BTreeMap::new();
        utxo.insert(tx_in(0x01, 0), ());
        utxo.insert(tx_in(0x02, 0), ());

        let mut inputs = BTreeSet::new();
        inputs.insert(tx_in(0x01, 0));
        inputs.insert(tx_in(0x02, 0));

        assert!(check_inputs_present(&inputs, &utxo).is_ok());
    }

    #[test]
    fn inputs_present_missing_one_reports_it() {
        let mut utxo = BTreeMap::new();
        utxo.insert(tx_in(0x01, 0), ());

        let mut inputs = BTreeSet::new();
        inputs.insert(tx_in(0x01, 0));
        inputs.insert(tx_in(0x99, 0));

        match check_inputs_present(&inputs, &utxo) {
            Err(LedgerError::BadInputs(e)) => {
                assert_eq!(e.missing.len(), 1);
                assert!(e.missing.contains(&tx_in(0x99, 0)));
            }
            other => panic!("expected BadInputs, got {other:?}"),
        }
    }

    #[test]
    fn inputs_present_missing_all_reports_all() {
        let utxo: BTreeMap<TxIn, ()> = BTreeMap::new();

        let mut inputs = BTreeSet::new();
        inputs.insert(tx_in(0x01, 0));
        inputs.insert(tx_in(0x02, 0));
        inputs.insert(tx_in(0x03, 0));

        match check_inputs_present(&inputs, &utxo) {
            Err(LedgerError::BadInputs(e)) => {
                assert_eq!(e.missing.len(), 3);
            }
            other => panic!("expected BadInputs, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // check_collateral_non_empty (O-27.2)
    // -----------------------------------------------------------------------

    #[test]
    fn collateral_non_empty_passes_when_present() {
        let mut col = BTreeSet::new();
        col.insert(tx_in(0xaa, 0));
        assert!(check_collateral_non_empty(&col).is_ok());
    }

    #[test]
    fn collateral_non_empty_fails_when_empty() {
        let col: BTreeSet<TxIn> = BTreeSet::new();
        assert!(matches!(
            check_collateral_non_empty(&col),
            Err(LedgerError::NoCollateralInputs)
        ));
    }

    // -----------------------------------------------------------------------
    // check_collateral_percent (O-27.1)
    // -----------------------------------------------------------------------

    #[test]
    fn percent_150_fee_100_balance_150_passes() {
        // 100 * 150 == 150 * 100 → equality, >= holds
        assert!(check_collateral_percent(150, 150, Coin(100)).is_ok());
    }

    #[test]
    fn percent_150_fee_100_balance_149_fails() {
        // 100 * 149 = 14900 < 15000 = 150 * 100
        match check_collateral_percent(149, 150, Coin(100)) {
            Err(LedgerError::InsufficientCollateral(e)) => {
                assert_eq!(e.balance, 149);
                assert_eq!(e.required, 150); // ceil(15000/100) = 150
                assert_eq!(e.percent, 150);
                assert_eq!(e.fee, 100);
            }
            other => panic!("expected InsufficientCollateral, got {other:?}"),
        }
    }

    #[test]
    fn percent_150_fee_101_required_ceiling_153() {
        // ceil(150 * 101 / 100) = ceil(151.5) = 152
        match check_collateral_percent(0, 150, Coin(101)) {
            Err(LedgerError::InsufficientCollateral(e)) => {
                assert_eq!(e.required, 152);
            }
            other => panic!("expected InsufficientCollateral, got {other:?}"),
        }
    }

    #[test]
    fn zero_fee_any_balance_passes() {
        // 100 * 0 >= 150 * 0 → trivially true
        assert!(check_collateral_percent(0, 150, Coin(0)).is_ok());
    }

    #[test]
    fn negative_balance_fails() {
        // balance = -1 (return exceeded inputs)
        match check_collateral_percent(-1, 150, Coin(100)) {
            Err(LedgerError::InsufficientCollateral(e)) => {
                assert_eq!(e.balance, -1);
            }
            other => panic!("expected InsufficientCollateral, got {other:?}"),
        }
    }

    #[test]
    fn large_fee_does_not_overflow() {
        // Near-u64::MAX fee should still evaluate without overflow.
        // u64::MAX = 18_446_744_073_709_551_615
        // 150 * u64::MAX fits in i128 easily (< 2^71)
        let fee = u64::MAX;
        let balance_enough = (fee as i128) * 150 / 100 + 1;
        // Sanity: passes
        assert!(check_collateral_percent(balance_enough, 150, Coin(fee)).is_ok());
        // balance 0 fails
        assert!(check_collateral_percent(0, 150, Coin(fee)).is_err());
    }

    #[test]
    fn percent_5_boundary_inclusive() {
        // 100 * 5 == 5 * 100 — inclusive >= should pass
        assert!(check_collateral_percent(5, 5, Coin(100)).is_ok());
        // one less fails
        assert!(check_collateral_percent(4, 5, Coin(100)).is_err());
    }

    // -----------------------------------------------------------------------
    // check_total_collateral (O-27.2)
    // -----------------------------------------------------------------------

    #[test]
    fn total_collateral_absent_always_passes() {
        assert!(check_total_collateral(100, None).is_ok());
        assert!(check_total_collateral(-1, None).is_ok());
    }

    #[test]
    fn total_collateral_matches_passes() {
        assert!(check_total_collateral(150, Some(Coin(150))).is_ok());
    }

    #[test]
    fn total_collateral_mismatch_fails() {
        match check_total_collateral(150, Some(Coin(149))) {
            Err(LedgerError::IncorrectTotalCollateral(e)) => {
                assert_eq!(e.balance, 150);
                assert_eq!(e.declared, 149);
            }
            other => panic!("expected IncorrectTotalCollateral, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // check_collateral_contains_non_ada (O-27.2)
    // -----------------------------------------------------------------------

    #[test]
    fn non_ada_without_return_fails() {
        assert!(matches!(
            check_collateral_contains_non_ada(true, false),
            Err(LedgerError::CollateralContainsNonADA)
        ));
    }

    #[test]
    fn non_ada_with_return_passes() {
        assert!(check_collateral_contains_non_ada(true, true).is_ok());
    }

    #[test]
    fn pure_ada_always_passes() {
        assert!(check_collateral_contains_non_ada(false, false).is_ok());
        assert!(check_collateral_contains_non_ada(false, true).is_ok());
    }

    // -----------------------------------------------------------------------
    // compute_collateral_balance (O-27.2)
    // -----------------------------------------------------------------------

    #[test]
    fn balance_no_return() {
        assert_eq!(compute_collateral_balance(1000, 0), 1000);
    }

    #[test]
    fn balance_with_return() {
        assert_eq!(compute_collateral_balance(1000, 200), 800);
    }

    #[test]
    fn balance_return_exceeds_inputs_is_negative() {
        assert_eq!(compute_collateral_balance(100, 200), -100);
    }

    #[test]
    fn balance_large_inputs_fits_i128() {
        // 1000 inputs of 10B ADA each = 10^16 lovelace — well within i128
        let sum: u128 = 1000u128 * 10_000_000_000_000_000u128;
        let bal = compute_collateral_balance(sum, 0);
        assert_eq!(bal, sum as i128);
    }

    // -----------------------------------------------------------------------
    // Determinism
    // -----------------------------------------------------------------------

    #[test]
    fn all_functions_deterministic() {
        // Same inputs → same outputs, bit-identical on repeat calls.
        let mut utxo = BTreeMap::new();
        utxo.insert(tx_in(0x01, 0), ());
        let mut ins = BTreeSet::new();
        ins.insert(tx_in(0x02, 0));

        let r1 = check_inputs_present(&ins, &utxo);
        let r2 = check_inputs_present(&ins, &utxo);
        assert_eq!(format!("{r1:?}"), format!("{r2:?}"));

        let c1 = check_collateral_percent(149, 150, Coin(100));
        let c2 = check_collateral_percent(149, 150, Coin(100));
        assert_eq!(format!("{c1:?}"), format!("{c2:?}"));
    }
}
