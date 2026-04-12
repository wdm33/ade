// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Two-phase validation decision + state-delta machinery (Phase 3
//! Cluster P-D, slice S-32).
//!
//! Cardano's Alonzo+ two-phase validation model:
//!
//! - **Phase-1**: structural + state-backed ledger rules (UTXO,
//!   UTXOW). Any failure → tx rejected outright, NO state delta,
//!   block-invalid if included.
//!
//! - **Phase-2**: script evaluation (UTXOS). When scripts run and
//!   disagree with the tx's declared `isValid` flag, a collateral-
//!   only state delta applies: collateral inputs removed from UTxO,
//!   collateral-return output added (Babbage+), fees credited with
//!   the balance. No outputs, certs, mint, withdrawals, or
//!   treasury-donation effects.
//!
//! This module provides:
//!   - `ValidationPhase` enum
//!   - `classify_failure_phase(&LedgerError) -> ValidationPhase`
//!   - `apply_phase_2_failure(state, ...) -> LedgerState`
//!
//! Discharge: docs/active/S-32_obligation_discharge.md §O-32.1.

use std::collections::BTreeSet;

use ade_types::tx::{Coin, TxIn};

use crate::error::LedgerError;
use crate::state::LedgerState;
use crate::utxo::{utxo_delete, utxo_insert, TxOut};

/// The two-phase validation model's classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationPhase {
    /// Tx rejected outright; no state delta; block-invalid if
    /// included. Structural / state-backed failure before any
    /// script execution.
    Phase1,
    /// Tx stays in block; collateral-only state delta applies.
    /// Plutus script evaluation disagreed with tx's `isValid`
    /// declaration.
    Phase2,
}

/// Classify a ledger error by the two-phase model.
///
/// From O-32.1 discharge: **only two LedgerError categories are
/// phase-2** — Plutus-specific evaluation failures that map to
/// Haskell's `ValidationTagMismatch (IsValid True) FailedUnexpectedly`
/// and `CollectErrors`. Every other `LedgerError` variant is
/// phase-1.
///
/// Note: Ade doesn't yet have dedicated `LedgerError` variants for
/// Plutus execution failures (those arrive with S-32 integration).
/// The function currently classifies every existing LedgerError as
/// phase-1. When `PlutusExecutionFailed` / `PlutusContextBuildFailed`
/// variants are added to `LedgerError`, they route here to phase-2.
pub fn classify_failure_phase(err: &LedgerError) -> ValidationPhase {
    match err {
        // Every existing LedgerError variant is phase-1. Plutus-
        // specific phase-2 variants are added by S-32 integration
        // (PlutusExecutionFailed, PlutusContextBuildFailed).
        //
        // Listed explicitly for audit — any new variant added to
        // LedgerError triggers a compile error here, forcing a
        // phase decision rather than silently defaulting.
        LedgerError::InputNotFound(_) => ValidationPhase::Phase1,
        LedgerError::DuplicateInput(_) => ValidationPhase::Phase1,
        LedgerError::Conservation(_) => ValidationPhase::Phase1,
        LedgerError::NegativeValue(_) => ValidationPhase::Phase1,
        LedgerError::InsufficientFee(_) => ValidationPhase::Phase1,
        LedgerError::MissingWitness(_) => ValidationPhase::Phase1,
        LedgerError::InvalidWitness(_) => ValidationPhase::Phase1,
        LedgerError::BootstrapWitnessMismatch(_) => ValidationPhase::Phase1,
        LedgerError::ExpiredTransaction(_) => ValidationPhase::Phase1,
        LedgerError::TransactionNotYetValid(_) => ValidationPhase::Phase1,
        LedgerError::NativeScriptFailed(_) => ValidationPhase::Phase1,
        LedgerError::MintWithoutPolicy(_) => ValidationPhase::Phase1,
        LedgerError::InvalidCertificate(_) => ValidationPhase::Phase1,
        LedgerError::EpochTransition(_) => ValidationPhase::Phase1,
        LedgerError::Translation(_) => ValidationPhase::Phase1,
        LedgerError::RuleNotYetEnforced(_) => ValidationPhase::Phase1,
        LedgerError::StructuralViolation(_) => ValidationPhase::Phase1,
        LedgerError::BadInputs(_) => ValidationPhase::Phase1,
        LedgerError::NoCollateralInputs => ValidationPhase::Phase1,
        LedgerError::InsufficientCollateral(_) => ValidationPhase::Phase1,
        LedgerError::CollateralContainsNonADA => ValidationPhase::Phase1,
        LedgerError::IncorrectTotalCollateral(_) => ValidationPhase::Phase1,
        LedgerError::NonDisjointRefInputs(_) => ValidationPhase::Phase1,
        LedgerError::MissingRequiredDatums(_) => ValidationPhase::Phase1,
        LedgerError::MissingRequiredSigners(_) => ValidationPhase::Phase1,
        LedgerError::WrongNetworkInTxBody(_) => ValidationPhase::Phase1,
        LedgerError::WrongNetworkInOutput(_) => ValidationPhase::Phase1,
        LedgerError::ExUnitsTooBigUTxO(_) => ValidationPhase::Phase1,

        // Phase-2: the TWO Plutus-specific variants from O-32.1.
        LedgerError::PlutusExecutionFailed(_) => ValidationPhase::Phase2,
        LedgerError::PlutusContextBuildFailed(_) => ValidationPhase::Phase2,

        LedgerError::Decoding(_) => ValidationPhase::Phase1,
    }
}

/// Apply the phase-2 collateral-only state delta (Babbage+).
///
/// From O-32.1 discharge:
/// 1. Remove all `collateral_inputs` from UTxO.
/// 2. Add `collateral_return` output at `TxIn(tx_id, outputs_len)`
///    if present.
/// 3. Credit `fees_added` = `total_collateral` if declared, else
///    `sum(col_in.coin) − collateral_return.coin`.
///
/// No regular outputs, no cert effects, no mint, no withdrawals,
/// no treasury donation — all discarded.
///
/// Pre-Babbage (Alonzo-only) callers pass `collateral_return =
/// None` and `total_collateral = None`; the function handles the
/// Alonzo case correctly (credit = sum of collateral input coins).
///
/// Pure function: returns a new `LedgerState`, does not mutate.
///
/// Returns `None` if any collateral input is not present in the
/// current UTxO (caller must verify with `check_inputs_present`
/// first — this is a phase-1 check and would prevent reaching
/// phase-2).
pub fn apply_phase_2_failure(
    state: &LedgerState,
    collateral_inputs: &BTreeSet<TxIn>,
    collateral_return_ref: Option<(TxIn, TxOut)>,
    total_collateral: Option<Coin>,
) -> Option<LedgerState> {
    // 1. Compute the fee credit from resolved collateral.
    let mut sum_collateral: u64 = 0;
    for tx_in in collateral_inputs {
        let out = state.utxo_state.utxos.get(tx_in)?;
        let coin = match out {
            TxOut::Byron { coin, .. } => coin.0,
            TxOut::ShelleyMary { value, .. } => value.coin.0,
        };
        sum_collateral = sum_collateral.saturating_add(coin);
    }
    let return_coin = collateral_return_ref
        .as_ref()
        .map(|(_, o)| match o {
            TxOut::Byron { coin, .. } => coin.0,
            TxOut::ShelleyMary { value, .. } => value.coin.0,
        })
        .unwrap_or(0);
    let computed_credit = sum_collateral.saturating_sub(return_coin);
    let fee_credit = total_collateral.map(|c| c.0).unwrap_or(computed_credit);

    // 2. Remove collateral inputs from UTxO.
    let mut new_utxo = state.utxo_state.clone();
    for tx_in in collateral_inputs {
        let (updated, _consumed) = utxo_delete(&new_utxo, tx_in).ok()?;
        new_utxo = updated;
    }

    // 3. Add collateral_return output (if present) at its TxIn.
    if let Some((tx_in, tx_out)) = collateral_return_ref {
        new_utxo = utxo_insert(&new_utxo, tx_in, tx_out);
    }

    // 4. Credit fees (epoch fees pot).
    let mut new_state = state.clone();
    new_state.utxo_state = new_utxo;
    new_state.epoch_state.epoch_fees = Coin(
        new_state
            .epoch_state
            .epoch_fees
            .0
            .saturating_add(fee_credit),
    );

    Some(new_state)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_types::tx::TxIn;
    use ade_types::{CardanoEra, Hash32};

    use crate::error::{
        BadInputsError, ExUnitsTooBigError, IncorrectTotalCollateralError,
        InsufficientCollateralError, NonDisjointRefInputsError,
        PlutusContextBuildError, PlutusContextBuildReason, PlutusExecutionError,
    };
    use crate::value::{MultiAsset, Value};

    fn tx_in(b: u8) -> TxIn {
        TxIn {
            tx_hash: Hash32([b; 32]),
            index: 0,
        }
    }

    // -----------------------------------------------------------------------
    // classify_failure_phase
    // -----------------------------------------------------------------------

    #[test]
    fn phase1_for_bad_inputs() {
        let mut missing = BTreeSet::new();
        missing.insert(tx_in(0x01));
        let err = LedgerError::BadInputs(BadInputsError { missing });
        assert_eq!(classify_failure_phase(&err), ValidationPhase::Phase1);
    }

    #[test]
    fn phase1_for_insufficient_collateral() {
        let err = LedgerError::InsufficientCollateral(InsufficientCollateralError {
            balance: 10,
            required: 150,
            percent: 150,
            fee: 100,
        });
        assert_eq!(classify_failure_phase(&err), ValidationPhase::Phase1);
    }

    #[test]
    fn phase1_for_ex_units_cap() {
        let err = LedgerError::ExUnitsTooBigUTxO(ExUnitsTooBigError {
            declared_mem: 100,
            declared_cpu: 200,
            max_mem: 99,
            max_cpu: 1000,
        });
        assert_eq!(classify_failure_phase(&err), ValidationPhase::Phase1);
    }

    #[test]
    fn phase1_for_incorrect_total_collateral() {
        let err = LedgerError::IncorrectTotalCollateral(IncorrectTotalCollateralError {
            balance: 150,
            declared: 100,
        });
        assert_eq!(classify_failure_phase(&err), ValidationPhase::Phase1);
    }

    #[test]
    fn phase1_for_ref_input_overlap() {
        let mut intersection = BTreeSet::new();
        intersection.insert(tx_in(0x01));
        let err = LedgerError::NonDisjointRefInputs(NonDisjointRefInputsError { intersection });
        assert_eq!(classify_failure_phase(&err), ValidationPhase::Phase1);
    }

    #[test]
    fn phase1_for_no_collateral_inputs() {
        assert_eq!(
            classify_failure_phase(&LedgerError::NoCollateralInputs),
            ValidationPhase::Phase1
        );
    }

    #[test]
    fn phase1_for_collateral_contains_non_ada() {
        assert_eq!(
            classify_failure_phase(&LedgerError::CollateralContainsNonADA),
            ValidationPhase::Phase1
        );
    }

    // -----------------------------------------------------------------------
    // Phase-2 classification (the only two Plutus-specific error classes)
    // -----------------------------------------------------------------------

    #[test]
    fn phase2_for_plutus_execution_failure() {
        let err = LedgerError::PlutusExecutionFailed(PlutusExecutionError {
            redeemer_index: 0,
            budget_exhausted: false,
        });
        assert_eq!(classify_failure_phase(&err), ValidationPhase::Phase2);
    }

    #[test]
    fn phase2_for_plutus_budget_exhaustion() {
        let err = LedgerError::PlutusExecutionFailed(PlutusExecutionError {
            redeemer_index: 2,
            budget_exhausted: true,
        });
        assert_eq!(classify_failure_phase(&err), ValidationPhase::Phase2);
    }

    #[test]
    fn phase2_for_plutus_context_build_missing_redeemer() {
        let err = LedgerError::PlutusContextBuildFailed(PlutusContextBuildError {
            reason: PlutusContextBuildReason::MissingRedeemer,
        });
        assert_eq!(classify_failure_phase(&err), ValidationPhase::Phase2);
    }

    #[test]
    fn phase2_for_plutus_context_build_missing_cost_model() {
        let err = LedgerError::PlutusContextBuildFailed(PlutusContextBuildError {
            reason: PlutusContextBuildReason::MissingCostModel,
        });
        assert_eq!(classify_failure_phase(&err), ValidationPhase::Phase2);
    }

    #[test]
    fn phase2_for_plutus_context_build_bad_translation() {
        let err = LedgerError::PlutusContextBuildFailed(PlutusContextBuildError {
            reason: PlutusContextBuildReason::BadTranslation,
        });
        assert_eq!(classify_failure_phase(&err), ValidationPhase::Phase2);
    }

    // -----------------------------------------------------------------------
    // apply_phase_2_failure
    // -----------------------------------------------------------------------

    fn mk_state_with_utxo(entries: &[(TxIn, u64)]) -> LedgerState {
        let mut state = LedgerState::new(CardanoEra::Babbage);
        for (tx_in, coin) in entries {
            state.utxo_state = utxo_insert(
                &state.utxo_state,
                tx_in.clone(),
                TxOut::ShelleyMary {
                    address: vec![0x61; 29],
                    value: Value {
                        coin: Coin(*coin),
                        multi_asset: MultiAsset::new(),
                    },
                },
            );
        }
        state
    }

    fn coin_output(coin: u64) -> TxOut {
        TxOut::ShelleyMary {
            address: vec![0x61; 29],
            value: Value {
                coin: Coin(coin),
                multi_asset: MultiAsset::new(),
            },
        }
    }

    #[test]
    fn phase2_alonzo_consumes_all_collateral_as_fee() {
        // Alonzo: no collateral_return, so full sum goes to fees.
        let col1 = tx_in(0xAA);
        let col2 = tx_in(0xBB);
        let state = mk_state_with_utxo(&[(col1.clone(), 1_000_000), (col2.clone(), 500_000)]);

        let mut collateral = BTreeSet::new();
        collateral.insert(col1.clone());
        collateral.insert(col2.clone());

        let new_state = apply_phase_2_failure(&state, &collateral, None, None).unwrap();

        // Collateral inputs removed.
        assert!(new_state.utxo_state.utxos.get(&col1).is_none());
        assert!(new_state.utxo_state.utxos.get(&col2).is_none());
        // Fees credited 1.5M.
        assert_eq!(new_state.epoch_state.epoch_fees.0, 1_500_000);
    }

    #[test]
    fn phase2_babbage_with_collateral_return() {
        // Babbage: collateral = 1M, return = 700k, fee credit = 300k.
        let col = tx_in(0xAA);
        let state = mk_state_with_utxo(&[(col.clone(), 1_000_000)]);

        let mut collateral = BTreeSet::new();
        collateral.insert(col.clone());

        let return_ref = (tx_in(0xFF), coin_output(700_000));

        let new_state = apply_phase_2_failure(&state, &collateral, Some(return_ref.clone()), None)
            .unwrap();

        // Collateral input removed.
        assert!(new_state.utxo_state.utxos.get(&col).is_none());
        // Collateral return added at its TxIn.
        assert!(new_state.utxo_state.utxos.get(&return_ref.0).is_some());
        // Fee = 1M - 700k = 300k
        assert_eq!(new_state.epoch_state.epoch_fees.0, 300_000);
    }

    #[test]
    fn phase2_total_collateral_overrides_computed_fee() {
        // Declared totalCollateral = 250k; computed would be 300k.
        // Declared takes precedence.
        let col = tx_in(0xAA);
        let state = mk_state_with_utxo(&[(col.clone(), 1_000_000)]);

        let mut collateral = BTreeSet::new();
        collateral.insert(col);

        let return_ref = (tx_in(0xFF), coin_output(700_000));
        let declared_total = Some(Coin(250_000));

        let new_state =
            apply_phase_2_failure(&state, &collateral, Some(return_ref), declared_total).unwrap();

        assert_eq!(new_state.epoch_state.epoch_fees.0, 250_000);
    }

    #[test]
    fn phase2_missing_collateral_input_returns_none() {
        let state = LedgerState::new(CardanoEra::Babbage);
        let mut collateral = BTreeSet::new();
        collateral.insert(tx_in(0x99)); // not in UTxO
        assert!(apply_phase_2_failure(&state, &collateral, None, None).is_none());
    }

    #[test]
    fn phase2_no_outputs_no_certs_no_mint_applied() {
        // Regression: phase-2 must leave everything except UTxO-collateral
        // and fees unchanged. Start with a populated cert_state and
        // verify it's preserved.
        let col = tx_in(0xAA);
        let mut state = mk_state_with_utxo(&[(col.clone(), 1_000_000)]);
        state.epoch_state.reserves = Coin(999_999);
        state.epoch_state.treasury = Coin(888_888);

        let mut collateral = BTreeSet::new();
        collateral.insert(col);

        let new_state = apply_phase_2_failure(&state, &collateral, None, None).unwrap();

        // Reserves / treasury untouched.
        assert_eq!(new_state.epoch_state.reserves, state.epoch_state.reserves);
        assert_eq!(new_state.epoch_state.treasury, state.epoch_state.treasury);
        // Cert state untouched.
        assert_eq!(new_state.cert_state, state.cert_state);
        // Era untouched.
        assert_eq!(new_state.era, state.era);
    }

    #[test]
    fn phase2_deterministic() {
        let col = tx_in(0xAA);
        let state = mk_state_with_utxo(&[(col.clone(), 1_000_000)]);
        let mut collateral = BTreeSet::new();
        collateral.insert(col);

        let r1 = apply_phase_2_failure(&state, &collateral, None, None);
        let r2 = apply_phase_2_failure(&state, &collateral, None, None);
        assert_eq!(r1, r2);
    }

}
