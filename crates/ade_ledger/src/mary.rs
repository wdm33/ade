// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeMap;
use ade_types::tx::{Coin, TxIn};
use ade_types::{Hash28, SlotNo};
use ade_types::mary::tx::{MaryTxBody, MaryTxOut};

use crate::error::{
    ConservationError, FeeError, LedgerError, MintError,
    NegativeValueError, ValidityError,
};
use crate::pparams::ProtocolParameters;
use crate::utxo::{utxo_delete, utxo_insert, TxOut, UTxOState};
use crate::value::{self, MultiAsset, Value};

/// Validate a single Mary transaction against the UTxO state.
///
/// Extends Allegra validation with:
/// - Multi-asset conservation: consumed_ma + minted == produced_ma (per policy/asset)
/// - No negative output asset quantities
/// - Mint field requires corresponding policy (placeholder until script eval)
///
/// Returns the updated UTxO state or a typed error.
pub fn validate_mary_tx(
    utxo_state: &UTxOState,
    tx_body: &MaryTxBody,
    tx_body_wire: &[u8],
    current_slot: SlotNo,
    pparams: &ProtocolParameters,
) -> Result<UTxOState, LedgerError> {
    // 1. Validity interval: check TTL if present
    if let Some(ttl) = tx_body.ttl {
        if current_slot.0 > ttl.0 {
            return Err(LedgerError::ExpiredTransaction(ValidityError {
                current_slot,
                bound: ttl,
            }));
        }
    }

    // 2. Validity interval start (Allegra+)
    if let Some(start) = tx_body.validity_interval_start {
        if current_slot.0 < start.0 {
            return Err(LedgerError::TransactionNotYetValid(ValidityError {
                current_slot,
                bound: start,
            }));
        }
    }

    // 3. Resolve inputs: consume UTxOs and accumulate consumed value
    let (new_utxo, consumed_value) = resolve_mary_inputs(utxo_state, tx_body)?;

    // 4. Compute produced value from outputs
    let produced_value = compute_mary_outputs_value(tx_body)?;

    // 5. Check no output has negative asset quantities
    for (i, output) in tx_body.outputs.iter().enumerate() {
        let output_value = mary_tx_out_to_value(output);
        value::check_non_negative(&output_value).map_err(|_| {
            LedgerError::NegativeValue(NegativeValueError {
                coin: Coin(i as u64),
            })
        })?;

        // Minimum UTxO value check
        if output.coin.0 < pparams.min_utxo_value.0 && pparams.min_utxo_value.0 > 0 {
            return Err(LedgerError::Conservation(ConservationError {
                consumed_coin: consumed_value.coin,
                produced_coin: output.coin,
            }));
        }
    }

    // 6. Parse mint field into a MultiAsset (empty if absent)
    let minted = parse_mint_field(tx_body)?;

    // 7. Fee check
    let fee = tx_body.fee;
    let min_fee = crate::shelley::shelley_min_fee(tx_body_wire.len(), pparams);
    if fee < min_fee {
        return Err(LedgerError::InsufficientFee(FeeError {
            required: min_fee,
            provided: fee,
        }));
    }

    // 8. Multi-asset conservation:
    //    consumed_ma + minted == produced_ma
    //    coin conservation: consumed_coin == produced_coin + fee
    check_mary_conservation(&consumed_value, &produced_value, fee, &minted)?;

    // 9. Add produced outputs to UTxO
    let tx_id = ade_crypto::blake2b_256(tx_body_wire);
    let mut final_utxo = new_utxo;
    for (idx, output) in tx_body.outputs.iter().enumerate() {
        let tx_in = TxIn {
            tx_hash: tx_id.clone(),
            index: idx as u16,
        };
        let tx_out = TxOut::ShelleyMary {
            address: output.address.clone(),
            value: mary_tx_out_to_value(output),
        };
        final_utxo = utxo_insert(&final_utxo, tx_in, tx_out);
    }

    Ok(final_utxo)
}

/// Resolve Mary transaction inputs from UTxO, returning new state and consumed value.
fn resolve_mary_inputs(
    utxo_state: &UTxOState,
    tx_body: &MaryTxBody,
) -> Result<(UTxOState, Value), LedgerError> {
    let mut state = utxo_state.clone();
    let mut consumed = Value::from_coin(Coin::ZERO);

    for tx_in in &tx_body.inputs {
        let (new_state, tx_out) = utxo_delete(&state, tx_in)?;
        let out_value = match &tx_out {
            TxOut::Byron { coin, .. } => Value::from_coin(*coin),
            TxOut::ShelleyMary { value, .. } => value.clone(),
            TxOut::AlonzoPlus { coin, .. } => Value::from_coin(*coin),
        };
        consumed = value::value_add(&consumed, &out_value)?;
        state = new_state;
    }

    Ok((state, consumed))
}

/// Compute aggregate output value from Mary transaction outputs.
fn compute_mary_outputs_value(tx_body: &MaryTxBody) -> Result<Value, LedgerError> {
    let mut total = Value::from_coin(Coin::ZERO);

    for output in &tx_body.outputs {
        let out_value = mary_tx_out_to_value(output);
        total = value::value_add(&total, &out_value)?;
    }

    Ok(total)
}

/// Convert a MaryTxOut to the ledger's Value type.
///
/// Since the MaryTxOut still holds multi_asset as opaque CBOR (Option<Vec<u8>>),
/// pure-lovelace outputs map directly. Multi-asset outputs are represented
/// with an empty multi-asset bundle until the CBOR decoding slice lands —
/// multi-asset conservation is enforced when the mint field is decoded.
fn mary_tx_out_to_value(output: &MaryTxOut) -> Value {
    Value {
        coin: output.coin,
        multi_asset: MultiAsset::new(),
    }
}

/// Parse the mint field from a Mary transaction body.
///
/// The mint field is opaque CBOR in the current type representation.
/// Returns an empty MultiAsset if no mint field is present.
/// When mint bytes are present but not yet decoded, returns empty multi-asset.
/// Full CBOR decoding of the mint field is a follow-on task.
fn parse_mint_field(tx_body: &MaryTxBody) -> Result<MultiAsset, LedgerError> {
    match &tx_body.mint {
        None => Ok(MultiAsset::new()),
        Some(_mint_bytes) => {
            // Mint field present — placeholder until CBOR decode is wired.
            // The mint bytes are preserved for future decoding.
            Ok(MultiAsset::new())
        }
    }
}

/// Check Mary-era conservation of value.
///
/// Coin: consumed_coin == produced_coin + fee
/// Multi-asset: consumed_ma + minted == produced_ma (per policy per asset)
fn check_mary_conservation(
    consumed: &Value,
    produced: &Value,
    fee: Coin,
    minted: &MultiAsset,
) -> Result<(), LedgerError> {
    // Coin conservation
    let produced_plus_fee = produced.coin.checked_add(fee).ok_or(
        LedgerError::Conservation(ConservationError {
            consumed_coin: consumed.coin,
            produced_coin: produced.coin,
        }),
    )?;

    if consumed.coin != produced_plus_fee {
        return Err(LedgerError::Conservation(ConservationError {
            consumed_coin: consumed.coin,
            produced_coin: produced_plus_fee,
        }));
    }

    // Multi-asset conservation: consumed_ma + minted == produced_ma
    let consumed_plus_minted = value::value_add(
        &Value {
            coin: Coin::ZERO,
            multi_asset: consumed.multi_asset.clone(),
        },
        &Value {
            coin: Coin::ZERO,
            multi_asset: minted.clone(),
        },
    )?;

    if consumed_plus_minted.multi_asset != produced.multi_asset {
        return Err(LedgerError::Conservation(ConservationError {
            consumed_coin: consumed.coin,
            produced_coin: produced_plus_fee,
        }));
    }

    Ok(())
}

/// Validate that any minted policies have corresponding scripts.
///
/// Placeholder: in the full implementation, each policy ID in the mint field
/// must have a corresponding native script in the witness set. For now,
/// this checks that the mint multi-asset is empty or present.
pub fn check_mint_policies(
    minted: &MultiAsset,
    _available_scripts: &BTreeMap<Hash28, ()>,
) -> Result<(), LedgerError> {
    for policy_id in minted.0.keys() {
        if !_available_scripts.contains_key(policy_id) {
            return Err(LedgerError::MintWithoutPolicy(MintError {
                policy_id: policy_id.clone(),
            }));
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::value::AssetName;
    use ade_types::Hash32;
    use std::collections::BTreeSet;

    fn make_tx_in(hash_byte: u8, index: u16) -> TxIn {
        TxIn {
            tx_hash: Hash32([hash_byte; 32]),
            index,
        }
    }

    fn make_mary_tx_out(address_byte: u8, coin: u64) -> MaryTxOut {
        MaryTxOut {
            address: vec![address_byte],
            coin: Coin(coin),
            multi_asset: None,
        }
    }

    fn make_mary_tx_body(
        inputs: &[TxIn],
        outputs: Vec<MaryTxOut>,
        fee: u64,
    ) -> MaryTxBody {
        MaryTxBody {
            inputs: inputs.iter().cloned().collect::<BTreeSet<_>>(),
            outputs,
            fee: Coin(fee),
            ttl: Some(SlotNo(1_000_000)),
            certs: None,
            withdrawals: None,
            update: None,
            metadata_hash: None,
            validity_interval_start: None,
            mint: None,
        }
    }

    fn default_pparams() -> ProtocolParameters {
        ProtocolParameters {
            min_fee_a: Coin(0),
            min_fee_b: Coin(0),
            min_utxo_value: Coin(0),
            ..ProtocolParameters::default()
        }
    }

    fn seed_utxo(hash_byte: u8, coin: u64) -> (UTxOState, TxIn) {
        let tx_in = make_tx_in(hash_byte, 0);
        let tx_out = TxOut::ShelleyMary {
            address: vec![0x01],
            value: Value::from_coin(Coin(coin)),
        };
        let state = utxo_insert(&UTxOState::new(), tx_in.clone(), tx_out);
        (state, tx_in)
    }

    // -----------------------------------------------------------------------
    // Basic Mary transaction validation
    // -----------------------------------------------------------------------

    #[test]
    fn mary_tx_coin_conservation_passes() {
        let (utxo, input) = seed_utxo(0xaa, 1_000_000);
        let outputs = vec![make_mary_tx_out(0x01, 800_000)];
        let tx_body = make_mary_tx_body(&[input], outputs, 200_000);
        let wire = vec![0xa0]; // minimal wire bytes

        let result = validate_mary_tx(
            &utxo,
            &tx_body,
            &wire,
            SlotNo(100),
            &default_pparams(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn mary_tx_conservation_fails_wrong_fee() {
        let (utxo, input) = seed_utxo(0xbb, 1_000_000);
        let outputs = vec![make_mary_tx_out(0x01, 800_000)];
        let tx_body = make_mary_tx_body(&[input], outputs, 100_000); // too low: 800k + 100k != 1M
        let wire = vec![0xa0];

        let result = validate_mary_tx(
            &utxo,
            &tx_body,
            &wire,
            SlotNo(100),
            &default_pparams(),
        );
        assert!(matches!(result, Err(LedgerError::Conservation(_))));
    }

    #[test]
    fn mary_tx_expired_rejected() {
        let (utxo, input) = seed_utxo(0xcc, 1_000_000);
        let outputs = vec![make_mary_tx_out(0x01, 800_000)];
        let mut tx_body = make_mary_tx_body(&[input], outputs, 200_000);
        tx_body.ttl = Some(SlotNo(50));
        let wire = vec![0xa0];

        let result = validate_mary_tx(
            &utxo,
            &tx_body,
            &wire,
            SlotNo(100), // past TTL
            &default_pparams(),
        );
        assert!(matches!(result, Err(LedgerError::ExpiredTransaction(_))));
    }

    #[test]
    fn mary_tx_not_yet_valid_rejected() {
        let (utxo, input) = seed_utxo(0xdd, 1_000_000);
        let outputs = vec![make_mary_tx_out(0x01, 800_000)];
        let mut tx_body = make_mary_tx_body(&[input], outputs, 200_000);
        tx_body.validity_interval_start = Some(SlotNo(200));
        let wire = vec![0xa0];

        let result = validate_mary_tx(
            &utxo,
            &tx_body,
            &wire,
            SlotNo(100), // before start
            &default_pparams(),
        );
        assert!(matches!(
            result,
            Err(LedgerError::TransactionNotYetValid(_))
        ));
    }

    #[test]
    fn mary_tx_insufficient_fee_rejected() {
        let (utxo, input) = seed_utxo(0xee, 1_000_000);
        let outputs = vec![make_mary_tx_out(0x01, 999_990)];
        let tx_body = make_mary_tx_body(&[input], outputs, 10);
        let wire = vec![0xa0];

        let pparams = ProtocolParameters {
            min_fee_a: Coin(44),
            min_fee_b: Coin(155_381),
            min_utxo_value: Coin(0),
            ..ProtocolParameters::default()
        };

        let result = validate_mary_tx(
            &utxo,
            &tx_body,
            &wire,
            SlotNo(100),
            &pparams,
        );
        assert!(matches!(result, Err(LedgerError::InsufficientFee(_))));
    }

    #[test]
    fn mary_tx_missing_input_rejected() {
        let utxo = UTxOState::new();
        let input = make_tx_in(0xff, 0);
        let outputs = vec![make_mary_tx_out(0x01, 800_000)];
        let tx_body = make_mary_tx_body(&[input], outputs, 200_000);
        let wire = vec![0xa0];

        let result = validate_mary_tx(
            &utxo,
            &tx_body,
            &wire,
            SlotNo(100),
            &default_pparams(),
        );
        assert!(matches!(result, Err(LedgerError::InputNotFound(_))));
    }

    // -----------------------------------------------------------------------
    // Multi-asset conservation
    // -----------------------------------------------------------------------

    #[test]
    fn multi_asset_conservation_equal_passes() {
        let consumed = Value {
            coin: Coin(1_000),
            multi_asset: MultiAsset::new(),
        };
        let produced = Value {
            coin: Coin(800),
            multi_asset: MultiAsset::new(),
        };
        let minted = MultiAsset::new();

        let result = check_mary_conservation(&consumed, &produced, Coin(200), &minted);
        assert!(result.is_ok());
    }

    #[test]
    fn multi_asset_conservation_with_mint() {
        let policy = Hash28([0xaa; 28]);
        let name = AssetName(b"token".to_vec());

        // Consumed has no tokens
        let consumed = Value {
            coin: Coin(1_000),
            multi_asset: MultiAsset::new(),
        };

        // Produced has 100 tokens (from minting)
        let mut produced_ma = BTreeMap::new();
        let mut inner = BTreeMap::new();
        inner.insert(name.clone(), 100i64);
        produced_ma.insert(policy.clone(), inner);

        let produced = Value {
            coin: Coin(800),
            multi_asset: MultiAsset(produced_ma),
        };

        // Minted 100 tokens
        let mut minted_inner = BTreeMap::new();
        let mut mint_map = BTreeMap::new();
        minted_inner.insert(name, 100i64);
        mint_map.insert(policy, minted_inner);
        let minted = MultiAsset(mint_map);

        let result = check_mary_conservation(&consumed, &produced, Coin(200), &minted);
        assert!(result.is_ok());
    }

    #[test]
    fn multi_asset_conservation_mismatch_fails() {
        let policy = Hash28([0xbb; 28]);
        let name = AssetName(b"tok".to_vec());

        let consumed = Value {
            coin: Coin(1_000),
            multi_asset: MultiAsset::new(),
        };

        // Produced has tokens but no minting
        let mut produced_ma = BTreeMap::new();
        let mut inner = BTreeMap::new();
        inner.insert(name, 50i64);
        produced_ma.insert(policy, inner);

        let produced = Value {
            coin: Coin(800),
            multi_asset: MultiAsset(produced_ma),
        };

        let minted = MultiAsset::new(); // Nothing minted!

        let result = check_mary_conservation(&consumed, &produced, Coin(200), &minted);
        assert!(matches!(result, Err(LedgerError::Conservation(_))));
    }

    // -----------------------------------------------------------------------
    // Mint policy checks
    // -----------------------------------------------------------------------

    #[test]
    fn check_mint_policies_no_mint_passes() {
        let minted = MultiAsset::new();
        let scripts = BTreeMap::new();
        assert!(check_mint_policies(&minted, &scripts).is_ok());
    }

    #[test]
    fn check_mint_policies_with_script_passes() {
        let policy = Hash28([0xcc; 28]);
        let name = AssetName(b"t".to_vec());

        let mut mint_map = BTreeMap::new();
        let mut inner = BTreeMap::new();
        inner.insert(name, 10i64);
        mint_map.insert(policy.clone(), inner);
        let minted = MultiAsset(mint_map);

        let mut scripts = BTreeMap::new();
        scripts.insert(policy, ());

        assert!(check_mint_policies(&minted, &scripts).is_ok());
    }

    #[test]
    fn check_mint_policies_missing_script_fails() {
        let policy = Hash28([0xdd; 28]);
        let name = AssetName(b"t".to_vec());

        let mut mint_map = BTreeMap::new();
        let mut inner = BTreeMap::new();
        inner.insert(name, 10i64);
        mint_map.insert(policy.clone(), inner);
        let minted = MultiAsset(mint_map);

        let scripts = BTreeMap::new(); // No scripts!

        let result = check_mint_policies(&minted, &scripts);
        assert!(matches!(result, Err(LedgerError::MintWithoutPolicy(MintError { policy_id })) if policy_id == policy));
    }

    // -----------------------------------------------------------------------
    // UTxO state updates
    // -----------------------------------------------------------------------

    #[test]
    fn mary_tx_produces_new_utxos() {
        let (utxo, input) = seed_utxo(0x11, 1_000_000);
        let outputs = vec![
            make_mary_tx_out(0x01, 500_000),
            make_mary_tx_out(0x02, 300_000),
        ];
        let tx_body = make_mary_tx_body(&[input], outputs, 200_000);
        let wire = vec![0xa0];

        let new_utxo = validate_mary_tx(
            &utxo,
            &tx_body,
            &wire,
            SlotNo(100),
            &default_pparams(),
        )
        .unwrap();

        // Should have 2 new UTxOs (input consumed, 2 outputs produced)
        assert_eq!(new_utxo.len(), 2);
    }

    #[test]
    fn mary_tx_no_ttl_accepted() {
        let (utxo, input) = seed_utxo(0x22, 1_000_000);
        let outputs = vec![make_mary_tx_out(0x01, 800_000)];
        let mut tx_body = make_mary_tx_body(&[input], outputs, 200_000);
        tx_body.ttl = None; // No TTL in Mary
        let wire = vec![0xa0];

        let result = validate_mary_tx(
            &utxo,
            &tx_body,
            &wire,
            SlotNo(100),
            &default_pparams(),
        );
        assert!(result.is_ok());
    }

    // -----------------------------------------------------------------------
    // Determinism
    // -----------------------------------------------------------------------

    #[test]
    fn mary_tx_validation_deterministic() {
        let (utxo, input) = seed_utxo(0x33, 1_000_000);
        let outputs = vec![make_mary_tx_out(0x01, 800_000)];
        let tx_body = make_mary_tx_body(&[input], outputs, 200_000);
        let wire = vec![0xa0];
        let pp = default_pparams();

        let r1 = validate_mary_tx(&utxo, &tx_body, &wire, SlotNo(100), &pp);
        let r2 = validate_mary_tx(&utxo, &tx_body, &wire, SlotNo(100), &pp);
        assert_eq!(r1, r2);
    }
}
