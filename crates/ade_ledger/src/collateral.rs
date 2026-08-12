// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BND-2b (INV-BND-2b) — the ADA a phase-2-invalid transaction actually consumes.
//!
//! Mirrors `Cardano.Ledger.Babbage.Collateral`, whose rule is:
//!
//! ```text
//! collAdaBalance txBody utxoCollateral = toDeltaCoin $
//!   case txBody ^. collateralReturnTxBodyL of
//!     SNothing    -> colbal
//!     SJust txOut -> colbal <-> (txOut ^. coinTxOutL)
//!   where colbal = sumAllCoin utxoCollateral
//! ```
//!
//! Two properties of that definition drive this module's shape:
//!
//! 1. `utxoCollateral` is the **resolved** UTxO entries for the collateral inputs — the consumed
//!    amount is a property of the entries, not of the transaction. So a resolver is required, and
//!    [`CollateralValueResolver`] is how the rule asks for it. The trait points INWARD: it is
//!    declared here (BLUE) and implemented by the storage layer, so the accumulator never holds,
//!    indexes or reconstructs a UTxO map.
//! 2. It is ADA only (`sumAllCoin` / `coinTxOutL`). That is exactly what Ade's reduced UTxO
//!    authority retains per entry, so the reduced form is sufficient by construction.
//!
//! `total_collateral` (body field 17) is **never** consulted. Per the reference it is a declared
//! assertion the UTXO rule checks (`IncorrectTotalCollateralField`) when present and which
//! constrains nothing when absent — never the source of the value.

use ade_types::tx::{Coin, TxIn};

/// The UTxO authority's answer for ONE collateral input: its ADA value, or `None` if the authority
/// does not hold it.
///
/// `None` is an ADMISSION OF IGNORANCE and callers must treat it as a refusal
/// ([`CollateralBalanceError::UnresolvedCollateralInput`]) — never as zero and never as a skipped
/// contribution. Substituting a value here would put a wrong number into the fee pot, which is the
/// precise failure the accumulator's fail-closed guards exist to prevent.
pub trait CollateralValueResolver {
    fn collateral_value(&self, txin: &TxIn) -> Option<Coin>;
}

/// Why a collateral balance could not be computed. Every variant is a REFUSAL — there is no
/// fallback value, by construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollateralBalanceError {
    /// The UTxO authority does not hold this collateral input, so the consumed amount is unknown.
    UnresolvedCollateralInput { tx_index: u64, txin: TxIn },
    /// Summing the resolved collateral values overflowed.
    ArithmeticOverflow { tx_index: u64 },
    /// The declared collateral return exceeds the resolved collateral. Cardano models the balance
    /// as a `DeltaCoin` and enforces sufficiency in the UTXO rule; Ade has no negative fee, so this
    /// fails closed rather than underflowing.
    ReturnExceedsCollateral {
        tx_index: u64,
        collateral: u64,
        returned: u64,
    },
}

/// `collAdaBalance` for one phase-2-invalid transaction: the sum of its resolved collateral input
/// values, minus its collateral return (which is read from the canonical block, not resolved).
///
/// Pure and total: the same inputs and the same resolver answers yield the same result, and every
/// failure is a typed refusal. An EMPTY collateral-input list yields `Coin(0)` — that is the honest
/// reading of `sumAllCoin ∅`, and it is not a fallback: it is only reachable for a transaction that
/// declares no collateral at all.
pub fn collateral_balance(
    tx_index: u64,
    collateral_inputs: &[TxIn],
    collateral_return_coin: Option<Coin>,
    resolver: &dyn CollateralValueResolver,
) -> Result<Coin, CollateralBalanceError> {
    let mut total: u64 = 0;
    for txin in collateral_inputs {
        let value = resolver.collateral_value(txin).ok_or_else(|| {
            CollateralBalanceError::UnresolvedCollateralInput {
                tx_index,
                txin: txin.clone(),
            }
        })?;
        total = total
            .checked_add(value.0)
            .ok_or(CollateralBalanceError::ArithmeticOverflow { tx_index })?;
    }
    match collateral_return_coin {
        None => Ok(Coin(total)),
        Some(returned) => {
            // Subtracted EXACTLY ONCE, and only here.
            let net = total.checked_sub(returned.0).ok_or(
                CollateralBalanceError::ReturnExceedsCollateral {
                    tx_index,
                    collateral: total,
                    returned: returned.0,
                },
            )?;
            Ok(Coin(net))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ade_types::Hash32;
    use std::collections::BTreeMap;

    struct MapResolver(BTreeMap<TxIn, Coin>);
    impl CollateralValueResolver for MapResolver {
        fn collateral_value(&self, txin: &TxIn) -> Option<Coin> {
            self.0.get(txin).copied()
        }
    }
    /// A resolver that knows nothing — models an authority that does not hold the entry.
    struct EmptyResolver;
    impl CollateralValueResolver for EmptyResolver {
        fn collateral_value(&self, _txin: &TxIn) -> Option<Coin> {
            None
        }
    }

    fn txin(tag: u8, index: u16) -> TxIn {
        TxIn {
            tx_hash: Hash32([tag; 32]),
            index,
        }
    }

    /// CE-2b-1 — multiple collateral inputs sum, deterministically.
    #[test]
    fn multiple_collateral_inputs_sum_deterministically() {
        let ins = vec![txin(1, 0), txin(2, 3), txin(3, 7)];
        let r = MapResolver(
            [
                (ins[0].clone(), Coin(1_000_000)),
                (ins[1].clone(), Coin(2_500_000)),
                (ins[2].clone(), Coin(7)),
            ]
            .into_iter()
            .collect(),
        );
        let a = collateral_balance(0, &ins, None, &r).expect("balance");
        assert_eq!(a, Coin(3_500_007));
        // CE-2b-6: replay yields a byte-identical scalar.
        let b = collateral_balance(0, &ins, None, &r).expect("balance");
        assert_eq!(a, b);
    }

    /// CE-2b-2 — the collateral return is subtracted EXACTLY ONCE.
    #[test]
    fn the_collateral_return_is_subtracted_exactly_once() {
        let ins = vec![txin(1, 0), txin(2, 1)];
        let r = MapResolver(
            [(ins[0].clone(), Coin(5_000_000)), (ins[1].clone(), Coin(1_000_000))]
                .into_iter()
                .collect(),
        );
        assert_eq!(
            collateral_balance(0, &ins, Some(Coin(4_000_000)), &r).expect("balance"),
            Coin(2_000_000),
            "6,000,000 collateral - 4,000,000 returned"
        );
        // Subtracting twice would give 0 here; adding would give 10,000,000. Both are excluded.
        assert_ne!(
            collateral_balance(0, &ins, Some(Coin(4_000_000)), &r).expect("balance"),
            Coin(0)
        );
    }

    /// CE-2b-3 — an unresolvable collateral input is a TYPED REFUSAL, never zero, never skipped.
    #[test]
    fn an_unresolved_collateral_input_is_a_typed_refusal_not_zero() {
        let ins = vec![txin(9, 1)];
        let err = collateral_balance(4, &ins, None, &EmptyResolver).expect_err("must refuse");
        assert_eq!(
            err,
            CollateralBalanceError::UnresolvedCollateralInput {
                tx_index: 4,
                txin: ins[0].clone()
            }
        );
    }

    /// CE-2b-3 (partial resolution) — ONE unresolved input among several still refuses. A rule that
    /// summed what it could would return a plausible, wrong number.
    #[test]
    fn one_unresolved_input_among_several_still_refuses() {
        let ins = vec![txin(1, 0), txin(2, 0), txin(3, 0)];
        let r = MapResolver(
            [(ins[0].clone(), Coin(10)), (ins[2].clone(), Coin(30))]
                .into_iter()
                .collect(),
        );
        let err = collateral_balance(0, &ins, None, &r).expect_err("must refuse");
        assert!(matches!(
            err,
            CollateralBalanceError::UnresolvedCollateralInput { .. }
        ));
    }

    /// A return exceeding the resolved collateral fails closed rather than underflowing.
    #[test]
    fn a_return_exceeding_the_collateral_fails_closed() {
        let ins = vec![txin(1, 0)];
        let r = MapResolver([(ins[0].clone(), Coin(100))].into_iter().collect());
        let err = collateral_balance(2, &ins, Some(Coin(101)), &r).expect_err("must refuse");
        assert_eq!(
            err,
            CollateralBalanceError::ReturnExceedsCollateral {
                tx_index: 2,
                collateral: 100,
                returned: 101
            }
        );
    }

    /// CE-2b-6 — the error path replays identically too, not just the success path.
    #[test]
    fn the_refusal_is_replay_identical() {
        let ins = vec![txin(9, 1)];
        let a = collateral_balance(1, &ins, None, &EmptyResolver).expect_err("refuse");
        let b = collateral_balance(1, &ins, None, &EmptyResolver).expect_err("refuse");
        assert_eq!(a, b);
    }
}
