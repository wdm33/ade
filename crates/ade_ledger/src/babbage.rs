// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::{BTreeMap, BTreeSet};

use ade_types::babbage::tx::BabbageTxBody;
use ade_types::tx::TxIn;
use ade_types::CardanoEra;

use crate::error::LedgerError;
use crate::late_era_validation::{
    check_address_network, check_collateral_contains_non_ada, check_collateral_non_empty,
    check_collateral_percent, check_inputs_present, check_required_signers, check_total_collateral,
    check_tx_ex_units_within_cap, check_tx_network_id, compute_collateral_balance,
};
use crate::scripts::ScriptPosture;
use crate::utxo::TxOut;
use crate::witness::WitnessInfo;

/// Classify the script posture of a Babbage transaction body.
pub fn classify_babbage_script_posture(body: &BabbageTxBody) -> ScriptPosture {
    if body.script_data_hash.is_some() {
        ScriptPosture::PlutusPresentDeferred
    } else {
        ScriptPosture::NonPlutusScriptsOnly
    }
}

/// Validate the structural legality of a Babbage transaction body.
pub fn validate_babbage_structure(body: &BabbageTxBody) -> Result<(), LedgerError> {
    crate::alonzo::validate_common_structure(
        body.inputs.is_empty(),
        body.outputs.is_empty(),
        body.fee,
        body.outputs.iter().any(|o| o.coin.0 == 0),
        CardanoEra::Babbage,
    )
}

// ---------------------------------------------------------------------------
// Babbage state-backed validation (S-27 + S-28 composer)
// ---------------------------------------------------------------------------

/// State-backed late-era validation for a Babbage transaction body.
///
/// Extends the Alonzo composer with Babbage-specific additions:
/// - Reference inputs participate in the input-resolution check
///   (same constructor as spend inputs; `BadInputsUTxO`).
/// - Collateral balance subtracts `collateral_return.coin` when present.
/// - `total_collateral` (when declared) must equal the computed balance.
/// - Output address network check widens to include `collateral_return`.
///
/// Disjointness between `inputs` and `reference_inputs` is NOT
/// enforced in Babbage — that rule is Conway-gated (`PV >= 9`). Babbage
/// silently accepts overlap; see `validate_conway_state_backed`.
///
/// Intentionally NOT wired into `apply_block` in this slice (see Alonzo
/// composer docstring for rationale). S-32 integrates.
pub fn validate_babbage_state_backed(
    body: &BabbageTxBody,
    utxo: &BTreeMap<TxIn, TxOut>,
    witness_info: &WitnessInfo,
    collateral_percent: u16,
    current_network: u8,
    max_tx_ex_units: (i64, i64),
) -> Result<(), LedgerError> {
    // 0. Tx-level ex_units cap (O-30.3).
    check_tx_ex_units_within_cap(
        witness_info.total_ex_units.mem,
        witness_info.total_ex_units.cpu,
        max_tx_ex_units.0,
        max_tx_ex_units.1,
    )?;

    // 1. Input resolution (spend + collateral + reference) — all use
    //    the same `BadInputsUTxO` constructor per O-28.1.
    let mut all_inputs: BTreeSet<TxIn> = body.inputs.iter().cloned().collect();
    if let Some(col) = &body.collateral_inputs {
        for tx_in in col {
            all_inputs.insert(tx_in.clone());
        }
    }
    if let Some(refs) = &body.reference_inputs {
        for tx_in in refs {
            all_inputs.insert(tx_in.clone());
        }
    }
    check_inputs_present(&all_inputs, utxo)?;

    // 2. Plutus-gated collateral non-empty
    if body.script_data_hash.is_some() {
        let empty = BTreeSet::new();
        let col = body.collateral_inputs.as_ref().unwrap_or(&empty);
        check_collateral_non_empty(col)?;
    }

    // 3. Collateral checks (when provided)
    if let Some(col) = &body.collateral_inputs {
        if !col.is_empty() {
            let (sum_coin, any_non_ada) = crate::alonzo::sum_collateral(col, utxo);
            let return_coin = body.collateral_return.as_ref().map(|o| o.coin.0).unwrap_or(0);
            let balance = compute_collateral_balance(sum_coin, return_coin);
            check_collateral_percent(balance, collateral_percent, body.fee)?;
            check_collateral_contains_non_ada(any_non_ada, body.collateral_return.is_some())?;
            check_total_collateral(balance, body.total_collateral)?;
        }
    }

    // 4. Required signers
    if let Some(req) = &body.required_signers {
        check_required_signers(req, &witness_info.available_key_hashes)?;
    }

    // 5. Tx-body network_id
    check_tx_network_id(body.network_id, current_network)?;

    // 6. Output address networks (including collateral_return per
    //    Babbage's `allOutputs` widening).
    for out in &body.outputs {
        check_address_network(&out.address, current_network)?;
    }
    if let Some(ret) = &body.collateral_return {
        check_address_network(&ret.address, current_network)?;
    }

    Ok(())
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

    fn minimal_body() -> BabbageTxBody {
        let mut inputs = BTreeSet::new();
        inputs.insert(TxIn {
            tx_hash: Hash32([0x01; 32]),
            index: 0,
        });
        BabbageTxBody {
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
            update: None,
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
        }
    }

    #[test]
    fn no_scripts_classifies_non_plutus() {
        assert_eq!(
            classify_babbage_script_posture(&minimal_body()),
            ScriptPosture::NonPlutusScriptsOnly
        );
    }

    #[test]
    fn plutus_classifies_deferred() {
        let mut body = minimal_body();
        body.script_data_hash = Some(Hash32([0xAA; 32]));
        body.collateral_inputs = Some(BTreeSet::new());
        assert_eq!(
            classify_babbage_script_posture(&body),
            ScriptPosture::PlutusPresentDeferred
        );
    }

    #[test]
    fn structural_ok_clean() {
        assert!(validate_babbage_structure(&minimal_body()).is_ok());
    }

    #[test]
    fn structural_ok_with_collateral_return() {
        let mut body = minimal_body();
        body.collateral_return = Some(BabbageTxOut {
            address: vec![0x00; 29],
            coin: Coin(500_000),
            multi_asset: None,
            datum_option: None,
            script_ref: None,
        });
        assert!(validate_babbage_structure(&body).is_ok());
    }

    #[test]
    fn structural_ok_with_reference_inputs() {
        let mut body = minimal_body();
        body.reference_inputs = Some(BTreeSet::new());
        assert!(validate_babbage_structure(&body).is_ok());
    }

    #[test]
    fn reject_empty_inputs() {
        let mut body = minimal_body();
        body.inputs = BTreeSet::new();
        assert!(matches!(
            validate_babbage_structure(&body),
            Err(LedgerError::StructuralViolation(StructuralError {
                reason: StructuralFailureReason::EmptyInputs, ..
            }))
        ));
    }

    #[test]
    fn empty_outputs_accepted() {
        let mut body = minimal_body();
        body.outputs = Vec::new();
        assert!(validate_babbage_structure(&body).is_ok());
    }

    #[test]
    fn structural_validation_deterministic() {
        let body = minimal_body();
        assert_eq!(validate_babbage_structure(&body), validate_babbage_structure(&body));
    }

    // -----------------------------------------------------------------------
    // Babbage state-backed validation (S-28.5 composer)
    // -----------------------------------------------------------------------

    use std::collections::BTreeMap;
    use ade_types::tx::Coin as CoinT;
    use crate::utxo::TxOut;
    use crate::value::{MultiAsset, Value};
    use crate::witness::WitnessInfo;

    const MAINNET_PERCENT: u16 = 150;
    const MAINNET_NET: u8 = 1;

    fn mainnet_addr() -> Vec<u8> {
        let mut v = vec![0x61u8];
        v.extend_from_slice(&[0xaa; 28]);
        v
    }

    fn utxo_with(entries: &[(TxIn, u64)]) -> BTreeMap<TxIn, TxOut> {
        let mut u = BTreeMap::new();
        for (tx_in, coin) in entries {
            u.insert(
                tx_in.clone(),
                TxOut::ShelleyMary {
                    address: mainnet_addr(),
                    value: Value {
                        coin: CoinT(*coin),
                        multi_asset: MultiAsset::new(),
                    },
                },
            );
        }
        u
    }

    fn empty_witness() -> WitnessInfo {
        WitnessInfo {
            available_key_hashes: BTreeSet::new(),
            native_scripts: Vec::new(),
            has_plutus_v1: false,
            has_plutus_v2: false,
            has_plutus_v3: false,
            total_ex_units: Default::default(),
        }
    }

    fn babbage_body() -> BabbageTxBody {
        let mut body = minimal_body();
        body.outputs[0].address = mainnet_addr();
        body
    }

    #[test]
    fn babbage_state_backed_happy_path() {
        let body = babbage_body();
        let utxo = utxo_with(&[(TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 5_000_000)]);
        assert!(validate_babbage_state_backed(
            &body, &utxo, &empty_witness(), MAINNET_PERCENT, MAINNET_NET, (i64::MAX, i64::MAX),
        ).is_ok());
    }

    #[test]
    fn babbage_reference_input_missing_fails() {
        let mut body = babbage_body();
        let mut refs = BTreeSet::new();
        refs.insert(TxIn { tx_hash: Hash32([0x99; 32]), index: 0 });
        body.reference_inputs = Some(refs);
        let utxo = utxo_with(&[(TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 5_000_000)]);
        assert!(matches!(
            validate_babbage_state_backed(
                &body, &utxo, &empty_witness(), MAINNET_PERCENT, MAINNET_NET, (i64::MAX, i64::MAX),
            ),
            Err(LedgerError::BadInputs(_))
        ));
    }

    #[test]
    fn babbage_reference_input_overlap_accepted_at_babbage_pv() {
        // Babbage PV 7/8 silently accepts overlap (only Conway PV 9+ gates).
        // The composer delegates to check_reference_input_disjoint at Conway
        // only; Babbage composer doesn't run that check at all — so overlap
        // passes through input resolution normally.
        let mut body = babbage_body();
        let shared = TxIn { tx_hash: Hash32([0x01; 32]), index: 0 };
        let mut refs = BTreeSet::new();
        refs.insert(shared.clone());
        body.reference_inputs = Some(refs);
        // UTxO must have the shared input
        let utxo = utxo_with(&[(shared, 5_000_000)]);
        assert!(validate_babbage_state_backed(
            &body, &utxo, &empty_witness(), MAINNET_PERCENT, MAINNET_NET, (i64::MAX, i64::MAX),
        ).is_ok());
    }

    #[test]
    fn babbage_total_collateral_mismatch_fails() {
        let mut body = babbage_body();
        body.script_data_hash = Some(Hash32([0xAA; 32]));
        let col_in = TxIn { tx_hash: Hash32([0x99; 32]), index: 0 };
        let mut col = BTreeSet::new();
        col.insert(col_in.clone());
        body.collateral_inputs = Some(col);
        body.fee = CoinT(100_000);
        body.total_collateral = Some(CoinT(999_999)); // doesn't match 1M
        let utxo = utxo_with(&[
            (TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 5_000_000),
            (col_in, 1_000_000),
        ]);
        assert!(matches!(
            validate_babbage_state_backed(
                &body, &utxo, &empty_witness(), MAINNET_PERCENT, MAINNET_NET, (i64::MAX, i64::MAX),
            ),
            Err(LedgerError::IncorrectTotalCollateral(_))
        ));
    }

    #[test]
    fn babbage_collateral_return_reduces_balance() {
        let mut body = babbage_body();
        body.script_data_hash = Some(Hash32([0xAA; 32]));
        let col_in = TxIn { tx_hash: Hash32([0x99; 32]), index: 0 };
        let mut col = BTreeSet::new();
        col.insert(col_in.clone());
        body.collateral_inputs = Some(col);
        body.fee = CoinT(100_000); // 150% = 150_000 required
        body.collateral_return = Some(BabbageTxOut {
            address: mainnet_addr(),
            coin: CoinT(2_850_000), // leaves 150_000 balance
            multi_asset: None,
            datum_option: None,
            script_ref: None,
        });
        let utxo = utxo_with(&[
            (TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 5_000_000),
            (col_in, 3_000_000), // 3M - 2.85M = 150k, exactly required
        ]);
        assert!(validate_babbage_state_backed(
            &body, &utxo, &empty_witness(), MAINNET_PERCENT, MAINNET_NET, (i64::MAX, i64::MAX),
        ).is_ok());
    }

    #[test]
    fn babbage_collateral_return_over_consumption_fails() {
        let mut body = babbage_body();
        body.script_data_hash = Some(Hash32([0xAA; 32]));
        let col_in = TxIn { tx_hash: Hash32([0x99; 32]), index: 0 };
        let mut col = BTreeSet::new();
        col.insert(col_in.clone());
        body.collateral_inputs = Some(col);
        body.fee = CoinT(100_000);
        body.collateral_return = Some(BabbageTxOut {
            address: mainnet_addr(),
            coin: CoinT(3_000_000), // return = 3M, but input is 1M → negative balance
            multi_asset: None,
            datum_option: None,
            script_ref: None,
        });
        let utxo = utxo_with(&[
            (TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 5_000_000),
            (col_in, 1_000_000),
        ]);
        match validate_babbage_state_backed(
            &body, &utxo, &empty_witness(), MAINNET_PERCENT, MAINNET_NET, (i64::MAX, i64::MAX),
        ) {
            Err(LedgerError::InsufficientCollateral(e)) => {
                assert_eq!(e.balance, -2_000_000);
            }
            other => panic!("expected InsufficientCollateral, got {other:?}"),
        }
    }

    #[test]
    fn babbage_collateral_return_wrong_network_fails() {
        let mut body = babbage_body();
        body.collateral_return = Some(BabbageTxOut {
            address: vec![0x60u8, 0xaa], // testnet
            coin: CoinT(100_000),
            multi_asset: None,
            datum_option: None,
            script_ref: None,
        });
        let utxo = utxo_with(&[(TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 5_000_000)]);
        assert!(matches!(
            validate_babbage_state_backed(
                &body, &utxo, &empty_witness(), MAINNET_PERCENT, MAINNET_NET, (i64::MAX, i64::MAX),
            ),
            Err(LedgerError::WrongNetworkInOutput(_))
        ));
    }
}
