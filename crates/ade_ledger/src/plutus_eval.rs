// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Per-tx Plutus evaluation wire-in (S-32 item 2 / CE-89 precursor).
//!
//! Bridges the ade_ledger composer path to `ade_plutus::eval_tx_phase_two`.
//! Responsibilities:
//!   1. Assemble full-tx CBOR `[body, witness_set, is_valid, aux_data]`
//!      from body_bytes + witness_bytes slices.
//!   2. Build `resolved_utxos` pairs `(TxIn_cbor, TxOut_cbor)` by walking
//!      the tx's input set and looking each up in the resolved UTxO.
//!   3. Short-circuit to `Ineligible` when any input doesn't resolve —
//!      the pre-block UTxO may legitimately not cover historical inputs
//!      that predate the replay window.
//!
//! Non-goals (explicitly deferred):
//!   - aux_data inclusion: set to CBOR `null` for now. aiken's phase-2
//!     path uses aux_data only for integrity-hash validation, which is
//!     a phase-1 concern. If aiken starts rejecting null aux_data, plumb
//!     the real bytes from the block's metadata map.
//!   - is_valid lookup from block.invalid_txs: defaulted to `true`
//!     (the typical mainnet case). If needed later, parse invalid_txs
//!     and pass the correct flag per tx_idx.
//!   - Script_ref / reference-input-sourced scripts: handled naturally
//!     by aiken as long as the reference input's output is in the
//!     resolved_utxos list with its ScriptRef encoded.
//!   - Actual phase-2 state delta (collateral consumption). Counting
//!     only; state-delta apply is a separate commit.

use std::collections::{BTreeMap, BTreeSet};

use ade_types::tx::TxIn;
use ade_types::CardanoEra;

use crate::utxo::TxOut;

/// Outcome of attempting to evaluate a single Plutus tx.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlutusEvalOutcome {
    /// Not all inputs (spend / collateral / reference) resolved in the
    /// provided UTxO. Mirrors the BadInputs silent-skip policy used by
    /// Phase-1 composers when replaying without full chain history.
    Ineligible,
    /// aiken reported success for every script in the tx.
    Passed {
        /// Sum of `cpu` across scripts.
        total_cpu: i64,
        /// Sum of `mem` across scripts.
        total_mem: i64,
        /// Number of scripts executed.
        script_count: usize,
    },
    /// aiken returned an error (decode / eval / budget / context).
    Failed {
        reason: String,
    },
}

/// Assemble a full Alonzo+ transaction CBOR from body + witness slices.
///
/// Wire format: `[body, witness_set, is_valid(true), aux_data(null)]`
/// — a 4-element definite-length array.
pub fn assemble_full_tx_cbor(body_bytes: &[u8], witness_bytes: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(body_bytes.len() + witness_bytes.len() + 3);
    buf.push(0x84); // array(4)
    buf.extend_from_slice(body_bytes);
    buf.extend_from_slice(witness_bytes);
    buf.push(0xf5); // is_valid = true
    buf.push(0xf6); // aux_data = null
    buf
}

/// Encode a single resolved (TxIn, TxOut) pair to the CBOR form aiken
/// expects in `resolved_utxos`.
///
/// TxIn: `[tx_id(bstr 32), index(uint)]`
/// TxOut: era-specific encoding from ade_codec's AdeEncode impls.
///
/// Byron outputs are not encodable in Alonzo+ form (different CDDL);
/// callers that hit a Byron UTxO entry should short-circuit to
/// `Ineligible` — Plutus txs cannot validly spend a Byron output anyway.
pub fn encode_resolved_pair(
    tx_in: &TxIn,
    tx_out: &TxOut,
    era: CardanoEra,
) -> Option<(Vec<u8>, Vec<u8>)> {
    let input_cbor = encode_tx_in(tx_in);

    let ctx = ade_codec::traits::CodecContext { era };
    let output_cbor = match tx_out {
        TxOut::Byron { .. } => return None,
        TxOut::ShelleyMary { address, value } => {
            // Shelley/Allegra/Mary outputs predate datum_hash/script_ref,
            // so a coin-only reconstruction is semantically complete for
            // those eras. Plutus txs that spend these UTxOs don't
            // reference datum or script_ref fields on them.
            let alonzo = ade_types::alonzo::tx::AlonzoTxOut {
                address: address.clone(),
                coin: value.coin,
                multi_asset: None,
                datum_hash: None,
            };
            let mut buf = Vec::new();
            use ade_codec::traits::AdeEncode;
            alonzo.ade_encode(&mut buf, &ctx).ok()?;
            buf
        }
        TxOut::AlonzoPlus { raw, .. } => {
            // Preserved byte-for-byte from the producing tx's body.
            // Contains the full datum_hash / datum_option / script_ref /
            // multi_asset surface aiken needs for ScriptContext.
            raw.clone()
        }
    };

    Some((input_cbor, output_cbor))
}

/// Encode a TxIn as CBOR `[tx_hash(bstr 32), index(uint)]`.
fn encode_tx_in(tx_in: &TxIn) -> Vec<u8> {
    let mut buf = Vec::with_capacity(36);
    buf.push(0x82); // array(2)
    // bstr(32) containing tx_hash bytes
    buf.push(0x58); // bstr with 1-byte length
    buf.push(32);
    buf.extend_from_slice(&tx_in.tx_hash.0);
    // index as canonical uint
    ade_codec::cbor::write_uint_canonical(&mut buf, tx_in.index as u64);
    buf
}

/// Build the full `resolved_utxos` list for a tx's input / collateral /
/// reference-input sets. Returns `None` if any input doesn't resolve.
pub fn build_resolved_utxos(
    inputs: &BTreeSet<TxIn>,
    collateral: Option<&BTreeSet<TxIn>>,
    reference_inputs: Option<&BTreeSet<TxIn>>,
    utxo: &BTreeMap<TxIn, TxOut>,
    era: CardanoEra,
) -> Option<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut all: BTreeSet<TxIn> = inputs.iter().cloned().collect();
    if let Some(c) = collateral {
        all.extend(c.iter().cloned());
    }
    if let Some(r) = reference_inputs {
        all.extend(r.iter().cloned());
    }

    let mut pairs = Vec::with_capacity(all.len());
    for tx_in in &all {
        let tx_out = utxo.get(tx_in)?;
        let pair = encode_resolved_pair(tx_in, tx_out, era)?;
        pairs.push(pair);
    }
    Some(pairs)
}

/// Run aiken's phase-2 evaluator for a single tx. Short-circuits to
/// `Ineligible` when any input doesn't resolve.
pub fn try_evaluate_tx(
    body_bytes: &[u8],
    witness_bytes: &[u8],
    inputs: &BTreeSet<TxIn>,
    collateral: Option<&BTreeSet<TxIn>>,
    reference_inputs: Option<&BTreeSet<TxIn>>,
    utxo: &BTreeMap<TxIn, TxOut>,
    era: CardanoEra,
    initial_budget: (u64, u64),
) -> PlutusEvalOutcome {
    let resolved =
        match build_resolved_utxos(inputs, collateral, reference_inputs, utxo, era) {
            Some(p) => p,
            None => return PlutusEvalOutcome::Ineligible,
        };

    let tx_cbor = assemble_full_tx_cbor(body_bytes, witness_bytes);

    match ade_plutus::eval_tx_phase_two(
        &tx_cbor,
        &resolved,
        None, // cost models: aiken defaults (fine for classification; fine-tuning is CE-86)
        initial_budget,
        ade_plutus::tx_eval::MAINNET_SLOT_CONFIG,
    ) {
        Ok(result) => {
            let total_cpu: i64 = result.scripts.iter().map(|s| s.cpu).sum();
            let total_mem: i64 = result.scripts.iter().map(|s| s.mem).sum();
            PlutusEvalOutcome::Passed {
                total_cpu,
                total_mem,
                script_count: result.scripts.len(),
            }
        }
        Err(e) => PlutusEvalOutcome::Failed {
            reason: format!("{e:?}"),
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_types::Hash32;

    #[test]
    fn assemble_full_tx_cbor_has_correct_prefix_and_suffix() {
        let body = vec![0xa3, 0x00, 0x01, 0x01, 0x02, 0x02, 0x03]; // minimal map-ish stub
        let witness = vec![0xa0]; // empty map
        let full = assemble_full_tx_cbor(&body, &witness);
        assert_eq!(full[0], 0x84, "must start with array(4)");
        assert_eq!(full[full.len() - 2], 0xf5, "second-to-last = is_valid=true");
        assert_eq!(full[full.len() - 1], 0xf6, "last = aux_data=null");
        assert_eq!(
            full.len(),
            body.len() + witness.len() + 3,
            "no padding beyond prefix+suffix",
        );
    }

    #[test]
    fn encode_tx_in_canonical() {
        let tx_in = TxIn {
            tx_hash: Hash32([0x11; 32]),
            index: 0,
        };
        let bytes = encode_tx_in(&tx_in);
        assert_eq!(bytes[0], 0x82, "array(2)");
        assert_eq!(bytes[1], 0x58, "bstr 1-byte length");
        assert_eq!(bytes[2], 32, "length = 32");
        assert_eq!(&bytes[3..35], &[0x11; 32]);
        assert_eq!(bytes[35], 0x00, "index 0 encoded inline");
    }

    #[test]
    fn build_resolved_utxos_ineligible_when_input_missing() {
        let mut inputs = BTreeSet::new();
        inputs.insert(TxIn {
            tx_hash: Hash32([0xAA; 32]),
            index: 0,
        });
        let utxo = BTreeMap::new();
        let pairs = build_resolved_utxos(&inputs, None, None, &utxo, CardanoEra::Alonzo);
        assert!(pairs.is_none(), "missing input → None");
    }

    #[test]
    fn build_resolved_utxos_ineligible_on_byron_utxo() {
        // Plutus txs cannot spend Byron outputs, but if our UTxO map has
        // a Byron entry for an input that the tx cites, we correctly
        // short-circuit rather than silently skipping it.
        let tx_in = TxIn {
            tx_hash: Hash32([0xAA; 32]),
            index: 0,
        };
        let mut inputs = BTreeSet::new();
        inputs.insert(tx_in.clone());
        let mut utxo = BTreeMap::new();
        utxo.insert(
            tx_in,
            TxOut::Byron {
                address: ade_types::address::Address::Byron(vec![0x82, 0xaa]),
                coin: ade_types::tx::Coin(1000),
            },
        );
        let pairs = build_resolved_utxos(&inputs, None, None, &utxo, CardanoEra::Alonzo);
        assert!(pairs.is_none(), "Byron UTxO for Plutus tx → Ineligible");
    }
}
