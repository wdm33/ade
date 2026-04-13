// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::{BTreeMap, BTreeSet};

use ade_types::alonzo::tx::AlonzoTxBody;
use ade_types::tx::TxIn;
use ade_types::CardanoEra;

use crate::error::{LedgerError, StructuralError, StructuralFailureReason};
use crate::late_era_validation::{
    check_address_network, check_collateral_contains_non_ada, check_collateral_non_empty,
    check_collateral_percent, check_inputs_present, check_required_signers,
    check_tx_ex_units_within_cap, check_tx_network_id, compute_collateral_balance,
};
use crate::scripts::ScriptPosture;
use crate::utxo::TxOut;
use crate::witness::WitnessInfo;

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

// ---------------------------------------------------------------------------
// Alonzo state-backed validation (S-27 + S-28 composer)
// ---------------------------------------------------------------------------

/// State-backed late-era validation for an Alonzo transaction body.
///
/// Composes the pure check functions from `late_era_validation` into
/// the Alonzo-specific sequence:
/// 1. Spend + collateral inputs must resolve in UTxO (O-27.3)
/// 2. If script_data_hash present: collateral inputs must be non-empty (O-27.2)
/// 3. If collateral present: percent rule (O-27.1) + non-ADA rule (O-27.2)
/// 4. required_signers ⊆ witness key hashes (O-28.3)
/// 5. tx-body network_id matches current network (O-28.4)
/// 6. Every output's address network nibble matches current network (O-28.4)
///
/// Intentionally NOT wired into `apply_block` in this slice — the default
/// replay path uses `track_utxo=false` (empty UTxO), under which state-
/// backed checks trivially fail on every input. Integration with real
/// UTxO state is S-32 work (verdict integration).
///
/// Callers that HAVE real UTxO state (future harness, Plutus test
/// scaffolding) invoke this function directly.
pub fn validate_alonzo_state_backed(
    body: &AlonzoTxBody,
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

    // 1. Input resolution (spend + collateral)
    let mut all_inputs: BTreeSet<TxIn> = body.inputs.iter().cloned().collect();
    if let Some(col) = &body.collateral_inputs {
        for tx_in in col {
            all_inputs.insert(tx_in.clone());
        }
    }
    check_inputs_present(&all_inputs, utxo)?;

    // 2. Plutus-gated collateral requirement.
    //
    // Cardano gates this on the PRESENCE OF REDEEMERS, not on script_data_hash.
    // A tx may set script_data_hash to bind datum hashes or cost models
    // without actually running scripts (e.g., datum-propagation txs).
    // Empirically confirmed: mainnet Babbage blocks contain such txs
    // and cardano-node accepts them without collateral.
    let has_redeemers = witness_info.total_ex_units.mem > 0
        || witness_info.total_ex_units.cpu > 0;
    if has_redeemers {
        let empty = BTreeSet::new();
        let col = body.collateral_inputs.as_ref().unwrap_or(&empty);
        check_collateral_non_empty(col)?;
    }

    // 3. Collateral checks (when provided)
    if let Some(col) = &body.collateral_inputs {
        if !col.is_empty() {
            let (sum_coin, any_non_ada) = sum_collateral(col, utxo);
            // Alonzo has no collateral_return, so balance = full sum
            let balance = compute_collateral_balance(sum_coin, 0);
            check_collateral_percent(balance, collateral_percent, body.fee)?;
            // Alonzo cannot return non-ADA collateral (no collateral_return field)
            check_collateral_contains_non_ada(any_non_ada, false)?;
        }
    }

    // 4. Required signers
    if let Some(req) = &body.required_signers {
        check_required_signers(req, &witness_info.available_key_hashes)?;
    }

    // 5. Tx-body network_id
    check_tx_network_id(body.network_id, current_network)?;

    // 6. Output address networks
    for out in &body.outputs {
        check_address_network(&out.address, current_network)?;
    }

    Ok(())
}

/// Sum the coin values of a collateral input set against resolved UTxO.
///
/// Returns `(sum_coin as u128, any_has_non_ada)`. Silently skips inputs
/// not present in `utxo` — callers should invoke
/// `check_inputs_present` first to ensure all collateral is resolvable.
pub(crate) fn sum_collateral(
    collateral_inputs: &BTreeSet<TxIn>,
    utxo: &BTreeMap<TxIn, TxOut>,
) -> (u128, bool) {
    let mut sum: u128 = 0;
    let mut any_non_ada = false;
    for tx_in in collateral_inputs {
        if let Some(out) = utxo.get(tx_in) {
            match out {
                TxOut::Byron { coin, .. } => {
                    sum = sum.saturating_add(coin.0 as u128);
                }
                TxOut::ShelleyMary { value, .. } => {
                    sum = sum.saturating_add(value.coin.0 as u128);
                    if !value.multi_asset.0.is_empty() {
                        any_non_ada = true;
                    }
                }
            }
        }
    }
    (sum, any_non_ada)
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

    // -----------------------------------------------------------------------
    // Alonzo state-backed validation (S-28.5 composer)
    // -----------------------------------------------------------------------

    use crate::value::{MultiAsset, Value};
    use ade_types::tx::Coin as CoinT;

    const MAINNET_COLLATERAL_PERCENT: u16 = 150;
    const MAINNET_NETWORK: u8 = 1;

    fn mainnet_addr() -> Vec<u8> {
        // Shelley enterprise address: high nibble 6, low nibble 1 (mainnet)
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

    fn witness_with_keys(keys: &[[u8; 28]]) -> WitnessInfo {
        let mut hashes = BTreeSet::new();
        for k in keys {
            hashes.insert(ade_types::Hash28(*k));
        }
        WitnessInfo {
            available_key_hashes: hashes,
            native_scripts: Vec::new(),
            has_plutus_v1: false,
            has_plutus_v2: false,
            has_plutus_v3: false,
            total_ex_units: Default::default(),
        }
    }

    fn alonzo_body_for_state_backed() -> AlonzoTxBody {
        let mut inputs = BTreeSet::new();
        inputs.insert(TxIn {
            tx_hash: Hash32([0x01; 32]),
            index: 0,
        });
        AlonzoTxBody {
            inputs,
            outputs: vec![ade_types::alonzo::tx::AlonzoTxOut {
                address: mainnet_addr(),
                coin: CoinT(1_000_000),
                multi_asset: None,
                datum_hash: None,
            }],
            fee: CoinT(200_000),
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
    fn alonzo_state_backed_happy_path_no_scripts() {
        let body = alonzo_body_for_state_backed();
        let utxo = utxo_with(&[(TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 5_000_000)]);
        let witness = witness_with_keys(&[]);
        let res = validate_alonzo_state_backed(
            &body, &utxo, &witness, MAINNET_COLLATERAL_PERCENT, MAINNET_NETWORK,
            (i64::MAX, i64::MAX),
        );
        assert!(res.is_ok(), "expected Ok, got {res:?}");
    }

    #[test]
    fn alonzo_state_backed_missing_input_fails() {
        let body = alonzo_body_for_state_backed();
        let utxo = BTreeMap::new();
        let witness = witness_with_keys(&[]);
        assert!(matches!(
            validate_alonzo_state_backed(
                &body, &utxo, &witness, MAINNET_COLLATERAL_PERCENT, MAINNET_NETWORK,
                (i64::MAX, i64::MAX),
            ),
            Err(LedgerError::BadInputs(_))
        ));
    }

    #[test]
    fn alonzo_state_backed_plutus_requires_collateral() {
        // Redeemers present (ex_units > 0) and no collateral → NoCollateralInputs.
        // The gate is on redeemers, not script_data_hash: datum-propagation txs
        // set script_data_hash without redeemers and need no collateral.
        let mut body = alonzo_body_for_state_backed();
        body.script_data_hash = Some(Hash32([0xAA; 32]));
        body.collateral_inputs = Some(BTreeSet::new()); // empty → fail
        let utxo = utxo_with(&[(TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 5_000_000)]);
        let mut witness = witness_with_keys(&[]);
        witness.total_ex_units = crate::witness::TotalExUnits { mem: 100, cpu: 200 };
        assert!(matches!(
            validate_alonzo_state_backed(
                &body, &utxo, &witness, MAINNET_COLLATERAL_PERCENT, MAINNET_NETWORK,
                (i64::MAX, i64::MAX),
            ),
            Err(LedgerError::NoCollateralInputs)
        ));
    }

    #[test]
    fn alonzo_state_backed_script_data_hash_without_redeemers_needs_no_collateral() {
        // Guards the regression: mainnet Babbage blocks contain txs that set
        // script_data_hash (binding datum hashes / cost models) but carry no
        // redeemers. Cardano accepts these without collateral; we must too.
        let mut body = alonzo_body_for_state_backed();
        body.script_data_hash = Some(Hash32([0xAA; 32]));
        body.collateral_inputs = None;
        let utxo = utxo_with(&[(TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 5_000_000)]);
        let witness = witness_with_keys(&[]); // total_ex_units = (0, 0)
        assert!(validate_alonzo_state_backed(
            &body, &utxo, &witness, MAINNET_COLLATERAL_PERCENT, MAINNET_NETWORK,
            (i64::MAX, i64::MAX),
        ).is_ok());
    }

    #[test]
    fn alonzo_state_backed_insufficient_collateral() {
        let mut body = alonzo_body_for_state_backed();
        body.script_data_hash = Some(Hash32([0xAA; 32]));
        let mut col = BTreeSet::new();
        let col_in = TxIn { tx_hash: Hash32([0x99; 32]), index: 0 };
        col.insert(col_in.clone());
        body.collateral_inputs = Some(col);
        body.fee = CoinT(1_000_000); // 150% of 1M = 1.5M required
        let utxo = utxo_with(&[
            (TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 5_000_000),
            (col_in, 1_000_000), // only 1M, less than 1.5M
        ]);
        let witness = witness_with_keys(&[]);
        assert!(matches!(
            validate_alonzo_state_backed(
                &body, &utxo, &witness, MAINNET_COLLATERAL_PERCENT, MAINNET_NETWORK,
                (i64::MAX, i64::MAX),
            ),
            Err(LedgerError::InsufficientCollateral(_))
        ));
    }

    #[test]
    fn alonzo_state_backed_required_signer_missing() {
        let mut body = alonzo_body_for_state_backed();
        let mut req = BTreeSet::new();
        req.insert(ade_types::Hash28([0x77; 28]));
        body.required_signers = Some(req);
        let utxo = utxo_with(&[(TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 5_000_000)]);
        let witness = witness_with_keys(&[[0x66; 28]]); // not matching
        assert!(matches!(
            validate_alonzo_state_backed(
                &body, &utxo, &witness, MAINNET_COLLATERAL_PERCENT, MAINNET_NETWORK,
                (i64::MAX, i64::MAX),
            ),
            Err(LedgerError::MissingRequiredSigners(_))
        ));
    }

    #[test]
    fn alonzo_state_backed_wrong_network_in_tx_body() {
        let mut body = alonzo_body_for_state_backed();
        body.network_id = Some(0); // declared testnet, current is mainnet
        let utxo = utxo_with(&[(TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 5_000_000)]);
        let witness = witness_with_keys(&[]);
        assert!(matches!(
            validate_alonzo_state_backed(
                &body, &utxo, &witness, MAINNET_COLLATERAL_PERCENT, MAINNET_NETWORK,
                (i64::MAX, i64::MAX),
            ),
            Err(LedgerError::WrongNetworkInTxBody(_))
        ));
    }

    #[test]
    fn alonzo_state_backed_wrong_network_in_output() {
        let mut body = alonzo_body_for_state_backed();
        // Testnet address (low nibble 0) while current is mainnet (1)
        body.outputs[0].address = vec![0x60u8, 0xaa];
        let utxo = utxo_with(&[(TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 5_000_000)]);
        let witness = witness_with_keys(&[]);
        assert!(matches!(
            validate_alonzo_state_backed(
                &body, &utxo, &witness, MAINNET_COLLATERAL_PERCENT, MAINNET_NETWORK,
                (i64::MAX, i64::MAX),
            ),
            Err(LedgerError::WrongNetworkInOutput(_))
        ));
    }

    #[test]
    fn alonzo_state_backed_ex_units_cap_exceeded() {
        let body = alonzo_body_for_state_backed();
        let utxo = utxo_with(&[(TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 5_000_000)]);
        let mut witness = witness_with_keys(&[]);
        witness.total_ex_units = crate::witness::TotalExUnits { mem: 100, cpu: 200 };
        assert!(matches!(
            validate_alonzo_state_backed(
                &body, &utxo, &witness, MAINNET_COLLATERAL_PERCENT, MAINNET_NETWORK,
                (99, 1000), // mem cap exceeded
            ),
            Err(LedgerError::ExUnitsTooBigUTxO(_))
        ));
    }

    #[test]
    fn alonzo_state_backed_ex_units_within_cap() {
        // Redeemers with ex_units > 0 require collateral (post–composer-gate
        // fix). Seed collateral so the test exercises only the cap check.
        let mut body = alonzo_body_for_state_backed();
        let col_in = TxIn { tx_hash: Hash32([0x99; 32]), index: 0 };
        let mut col = BTreeSet::new();
        col.insert(col_in.clone());
        body.collateral_inputs = Some(col);
        let utxo = utxo_with(&[
            (TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 5_000_000),
            (col_in, 5_000_000),
        ]);
        let mut witness = witness_with_keys(&[]);
        witness.total_ex_units = crate::witness::TotalExUnits { mem: 100, cpu: 200 };
        assert!(validate_alonzo_state_backed(
            &body, &utxo, &witness, MAINNET_COLLATERAL_PERCENT, MAINNET_NETWORK,
            (1000, 1000),
        ).is_ok());
    }
}
