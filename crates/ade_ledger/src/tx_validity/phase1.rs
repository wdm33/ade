// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Shared per-tx phase-1 (PHASE4-B2-S2, CE-B2-2).
//!
//! [`tx_phase_one`] is the single per-tx phase-1 authority that BOTH the
//! standalone [`super::transition::tx_validity`] entry and the block body
//! path converge on. It composes — it introduces NO new validation rule:
//!
//! 1. **Required-signer closure** (B2-S1): [`super::required_signers`] /
//!    [`super::tx_derived_required_signers`] + [`super::verify_required_witnesses`]
//!    over the PRESERVED tx body hash. Same UTxO-resolution policy as the
//!    block path's `verify_conway_witness_closure` (full input-credential
//!    coverage when every input resolves, tx-derived subset otherwise).
//! 2. **State-backed checks** ([`crate::conway::validate_conway_state_backed`]):
//!    input resolution, value/fee, collateral, network id — the SAME function
//!    the block loop's `decode_and_phase_one` calls.
//!
//! ## The `track_utxo` boundary (honest scope)
//!
//! This mirrors the B1 block path (`rules::apply_*_block_classified` +
//! `verify_conway_witness_closure`) exactly:
//!
//! - The **witness closure (step 1) runs unconditionally**, regardless of
//!   `track_utxo`: tx-derived required-signer coverage (explicit / withdrawal /
//!   cert / voter sources) plus supplied-witness Ed25519 verification over the
//!   preserved body hash, fail-closed. This is never gated.
//! - The **UTxO-dependent state-backed checks (step 2) run only when
//!   `ledger.track_utxo` is true**: input presence/resolution, value balance,
//!   fee, collateral, and input-credential coverage all need the resolved
//!   pre-tx UTxO, which is meaningless without UTxO tracking.
//!
//! Therefore `track_utxo = false` is the **PARTIAL corpus/replay mode**
//! (structural + witness closure; the UTxO-dependent checks are DEFERRED), and
//! `track_utxo = true` is full validation. `track_utxo = false` must NOT be
//! read as "full validity": it is a strict subset.
//!
//! **No-false-accept boundary**: because value-balance and input-resolution are
//! skipped at `track_utxo = false`, an adversarial tx with a value imbalance or
//! an unresolvable input would NOT be rejected here. Such adversarial cases
//! belong to the `track_utxo = true` path (synthetic UTxO) in B2-S4;
//! witness-mutation adversarial cases run against real txs. This is the same
//! honest boundary the B1 block path carries.
//!
//! Decode + witness-set parsing lives in [`decode_tx`], which lifts the
//! preserved body slice and the witness-set slice out of the full Conway
//! transaction CBOR `[body, witness_set, is_valid, aux_data]` so the tx id
//! is `blake2b_256(body_slice)` — never a re-encode (T-ENC-01).

use ade_codec::cbor::{self, ContainerEncoding};
use ade_types::conway::tx::ConwayTxBody;
use ade_types::{CardanoEra, Hash32};

use crate::error::LedgerError;
use crate::state::LedgerState;
use crate::utxo::utxo_lookup;
use crate::witness::WitnessInfo;

use super::required_signers::{required_signers, tx_derived_required_signers, ResolvedInputs, ResolvedOutput};
use super::verdict::TxValidityError;
use super::witness::VKeyWitnessRef;

/// A decoded Conway transaction: the typed body, the PRESERVED body byte
/// slice (for the tx id and witness closure), the parsed witness set, the
/// raw vkey witnesses, and the raw body/witness-set CBOR slices the Plutus
/// dispatch needs.
pub struct DecodedTx {
    pub body: ConwayTxBody,
    pub body_bytes: Vec<u8>,
    pub tx_id: Hash32,
    pub witness_info: WitnessInfo,
    pub vkey_witnesses: Vec<VKeyWitnessRef>,
    pub witness_set_bytes: Vec<u8>,
}

/// Decode a full Conway transaction CBOR into [`DecodedTx`].
///
/// Wire shape (Alonzo+): `[transaction_body, transaction_witness_set,
/// is_valid(bool), auxiliary_data/nil]`. The body slice is lifted byte-for-byte
/// so `tx_id = blake2b_256(body_slice)` is computed over preserved bytes, never
/// a re-encode. Conway-only here (the era envelope handling for older eras is
/// out of this slice's scope; B2-S3 exercises real multi-era extraction).
pub fn decode_tx(tx_cbor: &[u8]) -> Result<DecodedTx, TxValidityError> {
    let mut offset = 0;
    let enc = cbor::read_array_header(tx_cbor, &mut offset)
        .map_err(|e| TxValidityError::Decode(LedgerError::from(e)))?;
    // A Conway transaction is a 4-element definite array.
    match enc {
        ContainerEncoding::Definite(n, _) if n >= 2 => {}
        _ => {
            return Err(TxValidityError::Decode(LedgerError::from(
                ade_codec::CodecError::InvalidCborStructure {
                    offset,
                    detail: "Conway transaction must be a definite array of >= 2 elements",
                },
            )));
        }
    }

    // Element 0: transaction body — capture the preserved slice.
    let body_start = offset;
    let body = ade_codec::conway::tx::decode_conway_tx_body(tx_cbor, &mut offset)
        .map_err(|e| TxValidityError::Decode(LedgerError::from(e)))?;
    let body_end = offset;
    let body_bytes = tx_cbor[body_start..body_end].to_vec();
    let tx_id = ade_crypto::blake2b_256(&body_bytes);

    // Element 1: transaction witness set — capture the preserved slice and
    // decode both the vkey witnesses and the script-presence info.
    let witness_start = offset;
    cbor::skip_item(tx_cbor, &mut offset)
        .map_err(|e| TxValidityError::Decode(LedgerError::from(e)))?;
    let witness_end = offset;
    let witness_set_bytes = tx_cbor[witness_start..witness_end].to_vec();

    let witness_info =
        crate::witness::decode_witness_info_single(&witness_set_bytes).map_err(TxValidityError::Decode)?;
    let vkey_witnesses = crate::shelley::decode_conway_vkey_witness_set_single(&witness_set_bytes)
        .map_err(TxValidityError::Decode)?
        .into_iter()
        .map(|w| VKeyWitnessRef {
            vkey: w.vkey,
            signature: w.signature,
        })
        .collect();

    Ok(DecodedTx {
        body,
        body_bytes,
        tx_id,
        witness_info,
        vkey_witnesses,
        witness_set_bytes,
    })
}

/// Run the shared per-tx phase-1 over a decoded tx atop `ledger`.
///
/// Fail-fast: the FIRST failing check is returned as a structured
/// [`TxValidityError`]; an all-passing tx returns `Ok(())`. Introduces no
/// new validation rule — it composes the B2-S1 witness closure and the
/// existing [`crate::conway::validate_conway_state_backed`] authority.
pub fn tx_phase_one(ledger: &LedgerState, decoded: &DecodedTx) -> Result<(), TxValidityError> {
    // 1. Required-signer closure over the preserved body hash.
    let required = if ledger.track_utxo {
        // Resolve spend + collateral inputs against the ledger UTxO. If every
        // input resolves, the FULL closed enumeration (including input /
        // collateral payment-key sources) is checked; otherwise fall back to
        // the tx-derived subset (same policy as the block path).
        let mut resolved = ResolvedInputs::new();
        let mut all_resolved = true;
        for input in decoded
            .body
            .inputs
            .iter()
            .chain(decoded.body.collateral_inputs.iter().flat_map(|c| c.iter()))
        {
            match utxo_lookup(&ledger.utxo_state, input) {
                Some(out) => {
                    resolved.insert(
                        input.clone(),
                        ResolvedOutput {
                            address: out.address_bytes().to_vec(),
                        },
                    );
                }
                None => {
                    all_resolved = false;
                    break;
                }
            }
        }
        if all_resolved {
            required_signers(&decoded.body, &resolved, CardanoEra::Conway)
                .map_err(|e| TxValidityError::Phase1(LedgerError::RequiredSignerDerivation(e)))?
        } else {
            tx_derived_required_signers(&decoded.body, CardanoEra::Conway)
                .map_err(|e| TxValidityError::Phase1(LedgerError::RequiredSignerDerivation(e)))?
        }
    } else {
        tx_derived_required_signers(&decoded.body, CardanoEra::Conway)
            .map_err(|e| TxValidityError::Phase1(LedgerError::RequiredSignerDerivation(e)))?
    };

    super::verify_required_witnesses(&decoded.tx_id, &required, &decoded.vkey_witnesses)
        .map_err(TxValidityError::Witness)?;

    // 2. UTxO-dependent state-backed checks — the SAME authority the block
    //    loop calls (input resolution / value-fee balance / collateral /
    //    network id). Gated on `track_utxo`, mirroring the block path: the
    //    block loop only runs `run_phase_one_composers` (which calls
    //    `validate_conway_state_backed`) when `track_utxo` is on, because
    //    every check inside resolves against the pre-tx UTxO. At
    //    `track_utxo = false` the UTxO is empty, so these checks are deferred
    //    (see the module-level `track_utxo` boundary note).
    if ledger.track_utxo {
        let pp = &ledger.protocol_params;
        let max_ex_units: (i64, i64) = (
            pp.max_tx_ex_units_mem as i64,
            pp.max_tx_ex_units_cpu as i64,
        );
        // Assemble the canonical Conway deposit-param view; a Conway state
        // missing its deposit params is a validation-environment fault that
        // fails fast here (never a default substitution).
        let deposit_params = ledger
            .conway_deposit_view()
            .map_err(|e| TxValidityError::Phase1(LedgerError::ValidationEnvironment(e)))?;
        crate::conway::validate_conway_state_backed(
            &decoded.body,
            &ledger.utxo_state.utxos,
            &decoded.witness_info,
            pp.collateral_percent,
            pp.network_id,
            pp.protocol_major as u16,
            max_ex_units,
            &deposit_params,
            &ledger.cert_state,
        )
        .map_err(TxValidityError::Phase1)?;
    }

    Ok(())
}
