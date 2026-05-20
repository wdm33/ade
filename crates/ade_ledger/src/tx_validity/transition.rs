// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// `tx_validity` composes the existing per-tx authorities — the B2-S1 witness
// closure, the shared state-backed phase-1, and the Plutus phase-2 dispatch —
// into one total, fail-fast verdict, paralleling `block_validity`:
//
//   Phase1Valid ∧ (Phase2Valid when the tx carries Plutus scripts)
//
// Fail-fast: phase-1 is decided first; phase-2 NEVER runs if phase-1 fails
// (DC-TXV-02). On any Invalid outcome the input state is returned unchanged —
// no partial mutation (DC-TXV-04). `tx_id` is `blake2b_256` of the PRESERVED
// body bytes, never a re-encode (T-ENC-01). No new validation rule is added
// here: this is composition only.

use ade_types::CardanoEra;

use crate::error::LedgerError;
use crate::state::LedgerState;

use super::phase1::{decode_tx, tx_phase_one};
use super::verdict::{TxValidityError, TxValidityVerdict};

/// The outcome of [`tx_validity`]: the verdict plus the (possibly evolved)
/// state. `applied` equals the input clone on any Invalid outcome and the
/// state evolved by the spend on Valid.
pub struct TxValidityOutcome {
    pub verdict: TxValidityVerdict,
    pub applied: LedgerState,
}

/// Decide whether `tx_cbor` is a valid transaction atop `ledger`.
///
/// Total and fail-fast: the first failing stage produces the verdict and the
/// input state is returned unchanged. A `Valid` verdict returns the state
/// evolved by the spend (inputs consumed, outputs produced under the tx id).
pub fn tx_validity(ledger: &LedgerState, tx_cbor: &[u8]) -> TxValidityOutcome {
    // Step 1: decode (preserved body slice → tx id; witness set).
    let decoded = match decode_tx(tx_cbor) {
        Ok(d) => d,
        Err(error) => return invalid(ledger, error),
    };

    // Step 2: phase-1 (witness closure + state-backed checks). FAIL-FAST —
    // phase-2 is not reached if this fails.
    if let Err(error) = tx_phase_one(ledger, &decoded) {
        return invalid(ledger, error);
    }

    // Step 3: phase-2 (Plutus), only when the tx carries Plutus scripts.
    if decoded.witness_info.has_plutus() {
        let pp = &ledger.protocol_params;
        let budget = (pp.max_tx_ex_units_cpu, pp.max_tx_ex_units_mem);
        let outcome = crate::plutus_eval::try_evaluate_tx(
            &decoded.body_bytes,
            &decoded.witness_set_bytes,
            &decoded.body.inputs,
            decoded.body.collateral_inputs.as_ref(),
            decoded.body.reference_inputs.as_ref(),
            &ledger.utxo_state.utxos,
            CardanoEra::Conway,
            budget,
            pp.cost_models_cbor.as_deref(),
        );
        match outcome {
            crate::plutus_eval::PlutusEvalOutcome::Passed { .. }
            | crate::plutus_eval::PlutusEvalOutcome::Ineligible => {}
            crate::plutus_eval::PlutusEvalOutcome::Failed { .. } => {
                // Map aiken's String-bearing failure into the closed phase-2
                // error taxonomy (no owned String on the verdict surface). The
                // coarse class is what the oracle compares; the exact aiken
                // diagnostic is out of the canonical surface by design.
                return invalid(
                    ledger,
                    TxValidityError::Phase2(LedgerError::PlutusExecutionFailed(
                        crate::error::PlutusExecutionError {
                            redeemer_index: 0,
                            budget_exhausted: false,
                        },
                    )),
                );
            }
        }
    }

    // Step 4: Valid — evolve the UTxO by the spend.
    let utxo_state = match crate::rules::apply_conway_tx_to_utxo(
        &ledger.utxo_state,
        &decoded.body,
        &decoded.body_bytes,
        &decoded.tx_id,
    ) {
        Ok(u) => u,
        Err(e) => return invalid(ledger, TxValidityError::Phase1(e)),
    };
    let mut applied = ledger.clone();
    applied.utxo_state = utxo_state;

    TxValidityOutcome {
        verdict: TxValidityVerdict::Valid {
            tx_id: decoded.tx_id,
            applied: applied.clone(),
        },
        applied,
    }
}

/// Build an Invalid outcome with the input state cloned unchanged.
fn invalid(ledger: &LedgerState, error: TxValidityError) -> TxValidityOutcome {
    let class = error.class();
    TxValidityOutcome {
        verdict: TxValidityVerdict::Invalid { class, error },
        applied: ledger.clone(),
    }
}
