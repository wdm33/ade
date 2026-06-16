// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;

use ade_types::conway::cert::{CertDisposition, CoinSource, DepositEffect};
use ade_types::conway::tx::ConwayTxBody;
use ade_types::tx::{Coin, TxIn};
use ade_types::CardanoEra;

use crate::cert_classify::classify;
use crate::delegation::CertState;
use crate::error::{EraInvalidCertificateError, LedgerError};
use crate::late_era_validation::{
    check_address_network, check_collateral_contains_non_ada, check_collateral_non_empty,
    check_collateral_percent, check_inputs_present, check_reference_input_disjoint,
    check_required_signers, check_total_collateral, check_tx_ex_units_within_cap,
    check_tx_network_id, compute_collateral_balance,
};
use crate::pparams::ConwayDepositParams;
use crate::scripts::ScriptPosture;
use crate::witness::WitnessInfo;

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

// ---------------------------------------------------------------------------
// Conway state-backed validation (S-27 + S-28 composer)
// ---------------------------------------------------------------------------

/// State-backed late-era validation for a Conway transaction body.
///
/// Extends Babbage with one Conway-era addition:
/// - `inputs ∩ reference_inputs == ∅` enforced when `PV >= 9 && PV < 11`
///   (O-28.1, Haskell `disjointRefInputs`).
///
/// Governance fields (voting_procedures, proposal_procedures,
/// treasury_value, donation) are handled structurally only in this
/// slice; their state-backed semantics live in the governance module
/// and the eventual S-32 integration.
///
/// Intentionally NOT wired into `apply_block` in this slice (see
/// Alonzo composer docstring). S-32 integrates.
#[allow(clippy::too_many_arguments)]
pub fn validate_conway_state_backed(
    body: &ConwayTxBody,
    utxo: &impl crate::utxo::UtxoStore,
    witness_info: &WitnessInfo,
    collateral_percent: u16,
    current_network: u8,
    protocol_version_major: u16,
    max_tx_ex_units: (i64, i64),
    deposit_params: &ConwayDepositParams,
    cert_state: &CertState,
) -> Result<(), LedgerError> {
    // 0. Tx-level ex_units cap (O-30.3).
    check_tx_ex_units_within_cap(
        witness_info.total_ex_units.mem,
        witness_info.total_ex_units.cpu,
        max_tx_ex_units.0,
        max_tx_ex_units.1,
    )?;

    // 1. Input resolution (spend + collateral + reference)
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

    // 2. Conway-gated reference-input disjointness (O-28.1)
    let empty_refs = BTreeSet::new();
    let refs = body.reference_inputs.as_ref().unwrap_or(&empty_refs);
    check_reference_input_disjoint(&body.inputs, refs, protocol_version_major)?;

    // 3. Plutus-gated collateral non-empty — gate on redeemers, not
    //    script_data_hash (see alonzo.rs / babbage.rs for rationale).
    let has_redeemers = witness_info.total_ex_units.mem > 0
        || witness_info.total_ex_units.cpu > 0;
    if has_redeemers {
        let empty = BTreeSet::new();
        let col = body.collateral_inputs.as_ref().unwrap_or(&empty);
        check_collateral_non_empty(col)?;
    }

    // 4. Collateral checks (when provided)
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

    // 5. Required signers
    if let Some(req) = &body.required_signers {
        check_required_signers(req, &witness_info.available_key_hashes)?;
    }

    // 5b. Coin-level preservation of value (the no-false-accept gap closed in
    //     PHASE4-B2-S4). The Conway/Babbage/Alonzo state-backed path previously
    //     verified input presence, collateral, network, and required signers
    //     but never the value equation, so a tx whose outputs + fee did not
    //     equal its inputs was accepted at track_utxo=true. See
    //     `check_conway_coin_conservation` for the exact (conservative) scope.
    check_conway_coin_conservation(body, utxo, deposit_params, cert_state)?;

    // 6. Tx-body network_id
    check_tx_network_id(body.network_id, current_network)?;

    // 7. Output address networks (including collateral_return)
    for out in &body.outputs {
        check_address_network(&out.address, current_network)?;
    }
    if let Some(ret) = &body.collateral_return {
        check_address_network(&ret.address, current_network)?;
    }

    Ok(())
}

/// Coin-level preservation of value for a Conway transaction body.
///
/// The Cardano `UTXO` rule requires `consumed == produced`, where (coin level):
///
/// ```text
/// consumed = Σ(resolved input coins) + Σ(withdrawals) + refunded_deposits
/// produced = Σ(output coins) + fee + donation + new_deposits
/// accept ⟺ consumed == produced
/// ```
///
/// This is the full equation: certs and withdrawals are decoded and accounted,
/// never skipped (PHASE4-B3-S4 removed the deposit/withdrawal early-out that was
/// the cluster's known false-accept path). `donation` (key 22) is a produced
/// term; `treasury_value` (key 21) carries no conservation weight.
///
/// The accounting pipeline runs in §9.1 precedence order so the rejected reason
/// is deterministic and independent of evaluation order:
///   1. decode failure (certs / withdrawals) — `CodecError` → `Decoding`;
///   2. era-invalid cert (`CertDisposition::NotValidInConway`, tags 5/6);
///   3. (missing validation environment — handled at the S1 view assembly upstream);
///   4. unsupported state-dependent accounting (`classify` reject);
///   5. value not conserved (`ConservationError`).
///
/// A tx triggering several of these rejects with the lowest-numbered reason;
/// no later check may mask an earlier failure.
///
/// Inputs are guaranteed resolvable: `check_inputs_present` runs earlier in the
/// composer, so a missing input is already a `BadInputs` rejection by the time
/// this is reached. `i128` arithmetic throughout; no float, no rounding.
fn check_conway_coin_conservation(
    body: &ConwayTxBody,
    utxo: &impl crate::utxo::UtxoStore,
    deposit_params: &ConwayDepositParams,
    cert_state: &CertState,
) -> Result<(), LedgerError> {
    // 1. Decode withdrawals (consumed term) — decode failure is the
    //    highest-precedence reject.
    let withdrawals_total: i128 = match &body.withdrawals {
        Some(bytes) => {
            let map = ade_codec::conway::withdrawals::decode_withdrawals(bytes)?;
            ade_codec::conway::withdrawals::withdrawals_sum(&map)
        }
        None => 0,
    };

    // 1. Decode certs, then 2. era-validity, then 4. classify accounting.
    //    Decode runs to completion before any classification so a decode
    //    failure cannot be masked by a later accounting reject.
    let mut new_deposits: i128 = 0;
    let mut refunded_deposits: i128 = 0;
    if let Some(bytes) = &body.certs {
        let certs = ade_codec::conway::cert::decode_conway_certs(bytes)?;
        // 2. Era-validity sweep across ALL certs before any accounting, so a
        //    removed tag is reported ahead of a state-dependent or value reject
        //    regardless of cert ordering.
        for (idx, cert) in certs.iter().enumerate() {
            if let CertDisposition::NotValidInConway = classify(cert, deposit_params, cert_state)? {
                return Err(LedgerError::EraInvalidCertificate(
                    EraInvalidCertificateError {
                        cert_index: idx as u16,
                        removed_tag: removed_tag_of(cert),
                    },
                ));
            }
        }
        // 4. Accounting fold — `classify` already proven non-Err and non-removed.
        for cert in &certs {
            match classify(cert, deposit_params, cert_state)? {
                CertDisposition::Accountable(DepositEffect::NewDeposit(src)) => {
                    new_deposits = new_deposits.saturating_add(coin_of(&src) as i128);
                }
                CertDisposition::Accountable(DepositEffect::Refund(src)) => {
                    refunded_deposits = refunded_deposits.saturating_add(coin_of(&src) as i128);
                }
                CertDisposition::Neutral => {}
                CertDisposition::NotValidInConway => unreachable!("era-validity swept above"),
            }
        }
    }

    let mut consumed: i128 = 0;
    for input in &body.inputs {
        // Present by construction (check_inputs_present ran first); a defensive
        // miss contributes nothing and is caught upstream, never silently here.
        if let Some(out) = utxo.get(input) {
            consumed = consumed.saturating_add(out.coin().0 as i128);
        }
    }
    consumed = consumed.saturating_add(withdrawals_total);
    consumed = consumed.saturating_add(refunded_deposits);

    let mut produced: i128 = body.fee.0 as i128;
    for out in &body.outputs {
        produced = produced.saturating_add(out.coin.0 as i128);
    }
    if let Some(donation) = body.donation {
        produced = produced.saturating_add(donation.0 as i128);
    }
    produced = produced.saturating_add(new_deposits);

    // 5. Value conservation — the final, lowest-precedence reject.
    if consumed != produced {
        return Err(LedgerError::Conservation(crate::error::ConservationError {
            consumed_coin: Coin(consumed.max(0).min(u64::MAX as i128) as u64),
            produced_coin: Coin(produced.max(0).min(u64::MAX as i128) as u64),
        }));
    }
    Ok(())
}

/// Extract the resolved coin from a [`CoinSource`]; every variant carries it.
fn coin_of(src: &CoinSource) -> u64 {
    match src {
        CoinSource::ExplicitInCert(c)
        | CoinSource::DepositParam(c)
        | CoinSource::RegistrationState(c) => c.0,
    }
}

/// The CDDL tag of a known-but-removed Conway certificate (tags 5/6).
fn removed_tag_of(cert: &ade_types::conway::cert::ConwayCert) -> u64 {
    match cert {
        ade_types::conway::cert::ConwayCert::RemovedInConway { tag } => *tag,
        _ => unreachable!("removed_tag_of called on a non-removed cert"),
    }
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
        // OQ-8 (PROPOSAL-PROCEDURES-DECODE): the old placeholder
        // `Some(vec![0x80])` for proposal_procedures was a presence-marker
        // only (not real proposal data); the field is now typed
        // `Option<Vec<ProposalProcedure>>` and constructing one here
        // would pull the closed decoder into a test that does not care
        // about proposal content. The other governance fields exercise
        // the "governance present" path.
        let mut body = minimal_body();
        body.voting_procedures = Some(vec![0x80]);
        body.proposal_procedures = None;
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

    // -----------------------------------------------------------------------
    // Conway state-backed validation (S-28.5 composer)
    // -----------------------------------------------------------------------

    use std::collections::BTreeMap;
    use ade_types::tx::Coin as CoinT;
    use crate::utxo::TxOut;
    use crate::value::{MultiAsset, Value};
    use crate::witness::WitnessInfo;

    const MAINNET_PERCENT: u16 = 150;
    const MAINNET_NET: u8 = 1;
    const PV_CONWAY: u16 = 9;

    fn deposit_params() -> ConwayDepositParams {
        ConwayDepositParams {
            key_deposit: Coin(2_000_000),
            pool_deposit: Coin(500_000_000),
            drep_deposit: Coin(500_000_000),
            gov_action_deposit: Coin(100_000_000_000),
        }
    }

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

    fn conway_body() -> ConwayTxBody {
        let mut body = minimal_body();
        body.outputs[0].address = mainnet_addr();
        body
    }

    #[test]
    fn conway_state_backed_happy_path() {
        let body = conway_body();
        // Input must balance output(1_000_000) + fee(200_000) = 1_200_000 to
        // satisfy coin-level preservation of value (PHASE4-B2-S4).
        let utxo = utxo_with(&[(TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 1_200_000)]);
        assert!(validate_conway_state_backed(
            &body, &utxo, &empty_witness(), MAINNET_PERCENT, MAINNET_NET, PV_CONWAY, (i64::MAX, i64::MAX), &deposit_params(), &CertState::new(),
        ).is_ok());
    }

    #[test]
    fn conway_reference_input_overlap_rejected() {
        // Conway PV 9: overlap between inputs and reference_inputs is
        // disallowed via BabbageNonDisjointRefInputs (O-28.1).
        let mut body = conway_body();
        let shared = TxIn { tx_hash: Hash32([0x01; 32]), index: 0 };
        let mut refs = BTreeSet::new();
        refs.insert(shared.clone());
        body.reference_inputs = Some(refs);
        let utxo = utxo_with(&[(shared, 5_000_000)]);
        assert!(matches!(
            validate_conway_state_backed(
                &body, &utxo, &empty_witness(), MAINNET_PERCENT, MAINNET_NET, PV_CONWAY, (i64::MAX, i64::MAX), &deposit_params(), &CertState::new(),
            ),
            Err(LedgerError::NonDisjointRefInputs(_))
        ));
    }

    #[test]
    fn conway_disjoint_reference_inputs_pass() {
        let mut body = conway_body();
        let ref_in = TxIn { tx_hash: Hash32([0x77; 32]), index: 0 };
        let mut refs = BTreeSet::new();
        refs.insert(ref_in.clone());
        body.reference_inputs = Some(refs);
        // Spend input balances output + fee; the reference input is not
        // consumed, so it does not contribute to preservation of value.
        let utxo = utxo_with(&[
            (TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 1_200_000),
            (ref_in, 1_000_000),
        ]);
        assert!(validate_conway_state_backed(
            &body, &utxo, &empty_witness(), MAINNET_PERCENT, MAINNET_NET, PV_CONWAY, (i64::MAX, i64::MAX), &deposit_params(), &CertState::new(),
        ).is_ok());
    }

    #[test]
    fn conway_overlap_accepted_at_future_pv_11() {
        // PV 11+ is outside the Haskell gate (disjointRefInputs's `< 11`
        // bound). Ade mirrors this exactly; overlap passes silently at
        // PV 11 until a future era re-enables the check.
        let mut body = conway_body();
        let shared = TxIn { tx_hash: Hash32([0x01; 32]), index: 0 };
        let mut refs = BTreeSet::new();
        refs.insert(shared.clone());
        body.reference_inputs = Some(refs);
        // Balanced spend (output + fee) so the conservation check passes; this
        // test isolates the PV-11 reference-input overlap behavior.
        let utxo = utxo_with(&[(shared, 1_200_000)]);
        assert!(validate_conway_state_backed(
            &body, &utxo, &empty_witness(), MAINNET_PERCENT, MAINNET_NET, 11, (i64::MAX, i64::MAX), &deposit_params(), &CertState::new(),
        ).is_ok());
    }

    #[test]
    fn conway_governance_fields_not_affecting_utxo_rule() {
        // Voting/proposal procedures + treasury_value + donation are
        // processed elsewhere (governance module); the UTXO composer
        // must not reject a tx purely because those fields are set.
        let mut body = conway_body();
        body.voting_procedures = Some(vec![0x80]);
        // OQ-8: proposal_procedures is now typed; this test does not load-bear
        // on proposal content (it tests the UTXO composer accepts a body with
        // governance fields). None is the honest substitute for the prior
        // placeholder `Some(vec![0x80])`.
        body.proposal_procedures = None;
        body.treasury_value = Some(CoinT(1_000_000));
        body.donation = Some(CoinT(500));
        // Balance: output(1_000_000) + fee(200_000) + donation(500). treasury_value
        // is a governance read, not a produced-value term.
        let utxo = utxo_with(&[(TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 1_200_500)]);
        assert!(validate_conway_state_backed(
            &body, &utxo, &empty_witness(), MAINNET_PERCENT, MAINNET_NET, PV_CONWAY, (i64::MAX, i64::MAX), &deposit_params(), &CertState::new(),
        ).is_ok());
    }
}
