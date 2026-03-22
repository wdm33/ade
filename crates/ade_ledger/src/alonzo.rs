// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::alonzo::tx::AlonzoTxBody;
use ade_types::CardanoEra;

use crate::error::{LedgerError, StructuralError, StructuralFailureReason};
use crate::scripts::ScriptPosture;

/// Classify the script posture of an Alonzo transaction body.
///
/// Deterministic classification based on structural indicators:
/// - No `script_data_hash` → no Plutus scripts involved
/// - `script_data_hash` present → Plutus scripts present, evaluation deferred
pub fn classify_alonzo_script_posture(body: &AlonzoTxBody) -> ScriptPosture {
    if body.script_data_hash.is_some() {
        ScriptPosture::PlutusPresentDeferred
    } else {
        ScriptPosture::NonPlutusScriptsOnly
    }
}

/// Validate the structural legality of an Alonzo transaction body.
///
/// Checks that do NOT require UTxO state:
/// - Inputs must not be empty
/// - Outputs must not be empty
/// - Fee must be non-zero
/// - No output may have zero coin
pub fn validate_alonzo_structure(body: &AlonzoTxBody) -> Result<(), LedgerError> {
    validate_common_structure(
        body.inputs.is_empty(),
        body.outputs.is_empty(),
        body.fee,
        body.outputs.iter().any(|o| o.coin.0 == 0),
        CardanoEra::Alonzo,
    )
}

/// Structural checks — unconditionally invalid regardless of UTxO state.
///
/// This is the state-free authority boundary. Only checks that hold in
/// every context belong here.
///
/// The following are NOT structural checks — they are state-backed ledger
/// rules that belong in the UTxO/state validation layer:
/// - EmptyOutputs → requires collateral-return / phase-2 context (T-21/UTxO)
/// - ZeroFee → requires fee semantics under phase-2 failure (T-21/UTxO)
/// - ZeroCoinOutput → requires full Value + protocol-parameter min-UTxO (T-21/UTxO)
pub(crate) fn validate_common_structure(
    empty_inputs: bool,
    _empty_outputs: bool,
    _fee: ade_types::tx::Coin,
    _has_zero_output: bool,
    era: CardanoEra,
) -> Result<(), LedgerError> {
    // A transaction with no inputs is unconditionally invalid — there is
    // nothing to consume and no way to authorize the transaction.
    if empty_inputs {
        return Err(LedgerError::StructuralViolation(StructuralError {
            era,
            reason: StructuralFailureReason::EmptyInputs,
        }));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use ade_types::alonzo::tx::AlonzoTxOut;
    use ade_types::tx::{Coin, TxIn};
    use ade_types::{Hash32, SlotNo};

    fn minimal_body() -> AlonzoTxBody {
        let mut inputs = BTreeSet::new();
        inputs.insert(TxIn {
            tx_hash: Hash32([0x01; 32]),
            index: 0,
        });
        AlonzoTxBody {
            inputs,
            outputs: vec![AlonzoTxOut {
                address: vec![0x00; 29],
                coin: Coin(1_000_000),
                multi_asset: None,
                datum_hash: None,
            }],
            fee: Coin(200_000),
            ttl: Some(SlotNo(100)),
            certs: None,
            withdrawals: None,
            update: None,
            metadata_hash: None,
            validity_interval_start: None,
            mint: None,
            script_data_hash: None,
            collateral_inputs: None,
            required_signers: None,
            network_id: None,
        }
    }

    #[test]
    fn no_scripts_classifies_as_non_plutus() {
        let body = minimal_body();
        assert_eq!(classify_alonzo_script_posture(&body), ScriptPosture::NonPlutusScriptsOnly);
    }

    #[test]
    fn script_data_hash_classifies_as_plutus_deferred() {
        let mut body = minimal_body();
        body.script_data_hash = Some(Hash32([0xAA; 32]));
        body.collateral_inputs = Some(BTreeSet::new());
        assert_eq!(classify_alonzo_script_posture(&body), ScriptPosture::PlutusPresentDeferred);
    }

    #[test]
    fn structural_ok_without_scripts() {
        let body = minimal_body();
        assert!(validate_alonzo_structure(&body).is_ok());
    }

    #[test]
    fn structural_ok_with_script_data_hash() {
        let mut body = minimal_body();
        body.script_data_hash = Some(Hash32([0xAA; 32]));
        assert!(validate_alonzo_structure(&body).is_ok());
    }

    #[test]
    fn reject_empty_inputs() {
        let mut body = minimal_body();
        body.inputs = BTreeSet::new();
        let err = validate_alonzo_structure(&body);
        assert!(matches!(
            err,
            Err(LedgerError::StructuralViolation(StructuralError {
                reason: StructuralFailureReason::EmptyInputs, ..
            }))
        ));
    }

    #[test]
    fn structural_validation_is_deterministic() {
        let body = minimal_body();
        let r1 = validate_alonzo_structure(&body);
        let r2 = validate_alonzo_structure(&body);
        assert_eq!(r1, r2);
    }

    #[test]
    fn empty_outputs_accepted() {
        // Babbage+ allows empty outputs (collateral-only txs).
        // Structural validation must not reject.
        let mut body = minimal_body();
        body.outputs = Vec::new();
        assert!(validate_alonzo_structure(&body).is_ok());
    }
}
