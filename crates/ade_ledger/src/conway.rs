// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::conway::tx::ConwayTxBody;
use ade_types::CardanoEra;

use crate::error::LedgerError;
use crate::scripts::ScriptPosture;

/// Classify the script posture of a Conway transaction body.
pub fn classify_conway_script_posture(body: &ConwayTxBody) -> ScriptPosture {
    if body.script_data_hash.is_some() {
        ScriptPosture::PlutusPresentDeferred
    } else {
        ScriptPosture::NonPlutusScriptsOnly
    }
}

/// Validate the structural legality of a Conway transaction body.
pub fn validate_conway_structure(body: &ConwayTxBody) -> Result<(), LedgerError> {
    crate::alonzo::validate_common_structure(
        body.inputs.is_empty(),
        body.outputs.is_empty(),
        body.fee,
        body.outputs.iter().any(|o| o.coin.0 == 0),
        CardanoEra::Conway,
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use crate::error::{StructuralError, StructuralFailureReason};
    use ade_types::babbage::tx::BabbageTxOut;
    use ade_types::tx::{Coin, TxIn};
    use ade_types::{Hash32, SlotNo};

    fn minimal_body() -> ConwayTxBody {
        let mut inputs = BTreeSet::new();
        inputs.insert(TxIn {
            tx_hash: Hash32([0x01; 32]),
            index: 0,
        });
        ConwayTxBody {
            inputs,
            outputs: vec![BabbageTxOut {
                address: vec![0x00; 29],
                coin: Coin(1_000_000),
                multi_asset: None,
                datum_option: None,
                script_ref: None,
            }],
            fee: Coin(200_000),
            ttl: Some(SlotNo(100)),
            certs: None,
            withdrawals: None,
            metadata_hash: None,
            validity_interval_start: None,
            mint: None,
            script_data_hash: None,
            collateral_inputs: None,
            required_signers: None,
            network_id: None,
            collateral_return: None,
            total_collateral: None,
            reference_inputs: None,
            voting_procedures: None,
            proposal_procedures: None,
            treasury_value: None,
            donation: None,
        }
    }

    #[test]
    fn no_scripts_classifies_non_plutus() {
        assert_eq!(
            classify_conway_script_posture(&minimal_body()),
            ScriptPosture::NonPlutusScriptsOnly
        );
    }

    #[test]
    fn structural_ok_clean() {
        assert!(validate_conway_structure(&minimal_body()).is_ok());
    }

    #[test]
    fn structural_ok_with_governance() {
        let mut body = minimal_body();
        body.voting_procedures = Some(vec![0x80]);
        body.proposal_procedures = Some(vec![0x80]);
        body.treasury_value = Some(Coin(1_000_000));
        body.donation = Some(Coin(500));
        assert!(validate_conway_structure(&body).is_ok());
    }

    #[test]
    fn structural_ok_with_script_data_hash() {
        let mut body = minimal_body();
        body.script_data_hash = Some(Hash32([0xAA; 32]));
        assert!(validate_conway_structure(&body).is_ok());
    }

    #[test]
    fn reject_empty_inputs() {
        let mut body = minimal_body();
        body.inputs = BTreeSet::new();
        assert!(matches!(
            validate_conway_structure(&body),
            Err(LedgerError::StructuralViolation(StructuralError {
                reason: StructuralFailureReason::EmptyInputs, ..
            }))
        ));
    }

    #[test]
    fn empty_outputs_accepted() {
        let mut body = minimal_body();
        body.outputs = Vec::new();
        assert!(validate_conway_structure(&body).is_ok());
    }

    #[test]
    fn structural_validation_deterministic() {
        let body = minimal_body();
        let r1 = validate_conway_structure(&body);
        let r2 = validate_conway_structure(&body);
        assert_eq!(r1, r2);
    }
}
