// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_codec::cbor;
use ade_codec::shelley::tx::decode_shelley_tx_body;
use ade_crypto::{blake2b_256, credential_hash, verify_ed25519, Ed25519VerificationKey, Ed25519Signature};
use ade_types::shelley::block::ShelleyBlock;
use ade_types::shelley::tx::ShelleyTxBody;
use ade_types::tx::{Coin, TxIn};
use ade_types::{Hash32, SlotNo};

use crate::error::{
    ConservationError, FeeError, LedgerError, ValidityError, WitnessAlgorithm, WitnessError,
};
use crate::pparams::ProtocolParameters;
use crate::state::LedgerState;
use crate::utxo::{utxo_delete, utxo_insert, TxOut, UTxOState};
use crate::value::Value;

/// Validate and apply a Shelley block to ledger state.
pub fn validate_shelley_block(
    state: &LedgerState,
    block: &ShelleyBlock,
    current_slot: SlotNo,
) -> Result<LedgerState, LedgerError> {
    let mut utxo_state = state.utxo_state.clone();

    // Decode tx bodies and witness sets from the block
    let tx_bodies = decode_tx_bodies_from_block(block)?;
    let witness_sets = decode_witness_sets_from_block(block)?;

    for (i, (tx_body_wire, tx_body)) in tx_bodies.iter().enumerate() {
        let empty_witnesses: Vec<VKeyWitness> = Vec::new();
        let witnesses = if i < witness_sets.len() {
            &witness_sets[i]
        } else {
            &empty_witnesses
        };

        utxo_state = validate_shelley_tx(
            &utxo_state,
            tx_body,
            tx_body_wire,
            witnesses,
            current_slot,
            &state.protocol_params,
        )?;
    }

    Ok(LedgerState {
        utxo_state,
        epoch_state: state.epoch_state.clone(),
        protocol_params: state.protocol_params.clone(),
        era: state.era,
    })
}

/// Decoded VKey witness: verification key + signature.
#[derive(Debug, Clone)]
pub struct VKeyWitness {
    pub vkey: Vec<u8>,
    pub signature: Vec<u8>,
}

/// Validate a single Shelley transaction.
fn validate_shelley_tx(
    utxo_state: &UTxOState,
    tx_body: &ShelleyTxBody,
    tx_body_wire: &[u8],
    witnesses: &[VKeyWitness],
    current_slot: SlotNo,
    pparams: &ProtocolParameters,
) -> Result<UTxOState, LedgerError> {
    // 1. TTL check
    if current_slot.0 > tx_body.ttl.0 {
        return Err(LedgerError::ExpiredTransaction(ValidityError {
            current_slot,
            bound: tx_body.ttl,
        }));
    }

    // 2. Resolve inputs and compute consumed value
    let (new_utxo, consumed_coin) = resolve_shelley_inputs(utxo_state, tx_body)?;

    // 3. Compute produced value
    let produced_coin = compute_shelley_outputs_coin(tx_body)?;

    // 4. Check outputs have minimum value
    for output in &tx_body.outputs {
        if output.coin.0 < pparams.min_utxo_value.0 && pparams.min_utxo_value.0 > 0 {
            return Err(LedgerError::Conservation(ConservationError {
                consumed_coin,
                produced_coin: output.coin,
            }));
        }
    }

    // 5. Fee check
    let fee = tx_body.fee;
    let min_fee = shelley_min_fee(tx_body_wire.len(), pparams);
    if fee < min_fee {
        return Err(LedgerError::InsufficientFee(FeeError {
            required: min_fee,
            provided: fee,
        }));
    }

    // 6. Conservation check: consumed = produced + fee
    let produced_plus_fee = produced_coin.checked_add(fee).ok_or(
        LedgerError::Conservation(ConservationError {
            consumed_coin,
            produced_coin,
        }),
    )?;

    if consumed_coin != produced_plus_fee {
        return Err(LedgerError::Conservation(ConservationError {
            consumed_coin,
            produced_coin: produced_plus_fee,
        }));
    }

    // 7. Verify witnesses
    let tx_body_hash = blake2b_256(tx_body_wire);
    verify_shelley_witnesses(witnesses, &tx_body_hash)?;

    // 8. Add produced outputs
    let tx_id = blake2b_256(tx_body_wire);
    let mut final_utxo = new_utxo;
    for (idx, output) in tx_body.outputs.iter().enumerate() {
        let tx_in = TxIn {
            tx_hash: tx_id.clone(),
            index: idx as u16,
        };
        let tx_out = TxOut::ShelleyMary {
            address: output.address.clone(),
            value: Value::from_coin(output.coin),
        };
        final_utxo = utxo_insert(&final_utxo, tx_in, tx_out);
    }

    Ok(final_utxo)
}

/// Compute Shelley minimum fee: `fee >= a * tx_size + b`
pub fn shelley_min_fee(tx_size_bytes: usize, pparams: &ProtocolParameters) -> Coin {
    let size_fee = pparams.min_fee_a.0.saturating_mul(tx_size_bytes as u64);
    Coin(size_fee.saturating_add(pparams.min_fee_b.0))
}

/// Resolve Shelley inputs from UTxO, returning new state and consumed coin.
fn resolve_shelley_inputs(
    utxo_state: &UTxOState,
    tx_body: &ShelleyTxBody,
) -> Result<(UTxOState, Coin), LedgerError> {
    let mut state = utxo_state.clone();
    let mut consumed = Coin(0);

    for tx_in in &tx_body.inputs {
        let (new_state, tx_out) = utxo_delete(&state, tx_in)?;
        consumed = consumed.checked_add(tx_out.coin()).ok_or(
            LedgerError::Conservation(ConservationError {
                consumed_coin: consumed,
                produced_coin: Coin(0),
            }),
        )?;
        state = new_state;
    }

    Ok((state, consumed))
}

/// Compute total output coin value.
fn compute_shelley_outputs_coin(tx_body: &ShelleyTxBody) -> Result<Coin, LedgerError> {
    let mut total = Coin(0);
    for output in &tx_body.outputs {
        total = total.checked_add(output.coin).ok_or(
            LedgerError::Conservation(ConservationError {
                consumed_coin: Coin(0),
                produced_coin: total,
            }),
        )?;
    }
    Ok(total)
}

/// Verify Shelley VKey witnesses against tx body hash.
fn verify_shelley_witnesses(
    witnesses: &[VKeyWitness],
    tx_body_hash: &Hash32,
) -> Result<(), LedgerError> {
    for witness in witnesses {
        let key_hash = credential_hash(&witness.vkey);

        let vk = Ed25519VerificationKey::from_bytes(&witness.vkey).map_err(|_| {
            LedgerError::InvalidWitness(WitnessError {
                key_hash: key_hash.clone(),
                algorithm: WitnessAlgorithm::Ed25519,
            })
        })?;

        let sig = Ed25519Signature::from_bytes(&witness.signature).map_err(|_| {
            LedgerError::InvalidWitness(WitnessError {
                key_hash: key_hash.clone(),
                algorithm: WitnessAlgorithm::Ed25519,
            })
        })?;

        let valid = verify_ed25519(&vk, &tx_body_hash.0, &sig).map_err(|_| {
            LedgerError::InvalidWitness(WitnessError {
                key_hash: key_hash.clone(),
                algorithm: WitnessAlgorithm::Ed25519,
            })
        })?;

        if !valid {
            return Err(LedgerError::InvalidWitness(WitnessError {
                key_hash,
                algorithm: WitnessAlgorithm::Ed25519,
            }));
        }
    }

    Ok(())
}

/// Decode transaction bodies from a Shelley block.
fn decode_tx_bodies_from_block(
    block: &ShelleyBlock,
) -> Result<Vec<(Vec<u8>, ShelleyTxBody)>, LedgerError> {
    let mut offset = 0;
    let data = &block.tx_bodies;
    let enc = cbor::read_array_header(data, &mut offset)?;

    let mut results = Vec::new();
    match enc {
        cbor::ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                let body_start = offset;
                let body = decode_shelley_tx_body(data, &mut offset)?;
                let wire = data[body_start..offset].to_vec();
                results.push((wire, body));
            }
        }
        cbor::ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, offset)? {
                let body_start = offset;
                let body = decode_shelley_tx_body(data, &mut offset)?;
                let wire = data[body_start..offset].to_vec();
                results.push((wire, body));
            }
        }
    }

    Ok(results)
}

/// Decode witness sets from a Shelley block.
///
/// Witness sets are stored as an array of maps. Each map has keys:
/// 0 = vkey_witnesses, 1 = multisig_scripts, 2 = bootstrap_witnesses
fn decode_witness_sets_from_block(
    block: &ShelleyBlock,
) -> Result<Vec<Vec<VKeyWitness>>, LedgerError> {
    let mut offset = 0;
    let data = &block.witness_sets;
    let enc = cbor::read_array_header(data, &mut offset)?;

    let mut results = Vec::new();
    let count = match enc {
        cbor::ContainerEncoding::Definite(n, _) => n,
        cbor::ContainerEncoding::Indefinite => {
            let mut ws = Vec::new();
            while !cbor::is_break(data, offset)? {
                ws.push(decode_single_witness_set(data, &mut offset)?);
            }
            return Ok(ws);
        }
    };

    for _ in 0..count {
        results.push(decode_single_witness_set(data, &mut offset)?);
    }

    Ok(results)
}

/// Decode a single witness set from CBOR map.
fn decode_single_witness_set(
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<VKeyWitness>, LedgerError> {
    let enc = cbor::read_map_header(data, offset)?;
    let map_len = match enc {
        cbor::ContainerEncoding::Definite(n, _) => n,
        cbor::ContainerEncoding::Indefinite => {
            // Handle indefinite map
            let mut witnesses = Vec::new();
            while !cbor::is_break(data, *offset)? {
                let (key, _) = cbor::read_uint(data, offset)?;
                if key == 0 {
                    witnesses = decode_vkey_witnesses(data, offset)?;
                } else {
                    let _ = cbor::skip_item(data, offset)?;
                }
            }
            *offset += 1;
            return Ok(witnesses);
        }
    };

    let mut witnesses = Vec::new();
    for _ in 0..map_len {
        let (key, _) = cbor::read_uint(data, offset)?;
        if key == 0 {
            witnesses = decode_vkey_witnesses(data, offset)?;
        } else {
            let _ = cbor::skip_item(data, offset)?;
        }
    }

    Ok(witnesses)
}

/// Decode VKey witnesses from CBOR array.
fn decode_vkey_witnesses(
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<VKeyWitness>, LedgerError> {
    let enc = cbor::read_array_header(data, offset)?;
    let count = match enc {
        cbor::ContainerEncoding::Definite(n, _) => n,
        cbor::ContainerEncoding::Indefinite => {
            let mut wits = Vec::new();
            while !cbor::is_break(data, *offset)? {
                wits.push(decode_vkey_witness(data, offset)?);
            }
            *offset += 1;
            return Ok(wits);
        }
    };

    let mut wits = Vec::with_capacity(count as usize);
    for _ in 0..count {
        wits.push(decode_vkey_witness(data, offset)?);
    }
    Ok(wits)
}

/// Decode a single VKey witness: `[vkey, signature]`
fn decode_vkey_witness(
    data: &[u8],
    offset: &mut usize,
) -> Result<VKeyWitness, LedgerError> {
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        cbor::ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(LedgerError::Decoding(crate::error::DecodingError {
                offset: *offset,
                reason: crate::error::DecodingFailureReason::InvalidStructure,
            }));
        }
    }

    let (vkey, _) = cbor::read_bytes(data, offset)?;
    let (signature, _) = cbor::read_bytes(data, offset)?;

    Ok(VKeyWitness { vkey, signature })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn shelley_min_fee_calculation() {
        let pparams = ProtocolParameters {
            min_fee_a: Coin(44),
            min_fee_b: Coin(155381),
            min_utxo_value: Coin(1_000_000),
            max_tx_size: 16384,
            ..ProtocolParameters::default()
        };
        let fee = shelley_min_fee(300, &pparams);
        assert_eq!(fee, Coin(44 * 300 + 155381));
    }

    #[test]
    fn expired_transaction_rejected() {
        let utxo = UTxOState::new();
        let tx_body = ShelleyTxBody {
            inputs: std::collections::BTreeSet::new(),
            outputs: vec![],
            fee: Coin(200_000),
            ttl: SlotNo(100),
            certs: None,
            withdrawals: None,
            update: None,
            metadata_hash: None,
        };

        let result = validate_shelley_tx(
            &utxo,
            &tx_body,
            &[0xa0], // minimal wire bytes
            &[],
            SlotNo(200), // current slot > ttl
            &ProtocolParameters::default(),
        );
        assert!(matches!(result, Err(LedgerError::ExpiredTransaction(_))));
    }
}
