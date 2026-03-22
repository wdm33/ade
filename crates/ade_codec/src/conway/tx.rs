// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;

use crate::alonzo::tx::read_hash28;
use crate::babbage::tx::decode_babbage_tx_out;
use crate::cbor::{self, ContainerEncoding};
use crate::error::CodecError;
use crate::shelley::tx::decode_tx_inputs;

/// Skip an optional CBOR tag (e.g. tag 258 for sets in Conway).
/// Returns Ok(()) whether or not a tag was present.
fn skip_optional_tag(data: &[u8], offset: &mut usize) -> Result<(), CodecError> {
    if *offset < data.len() {
        let major = cbor::peek_major(data, *offset)?;
        if major == cbor::MAJOR_TAG {
            let _ = cbor::read_tag(data, offset)?;
        }
    }
    Ok(())
}
use ade_types::babbage::tx::BabbageTxOut;
use ade_types::conway::tx::ConwayTxBody;
use ade_types::tx::{Coin, TxIn};
use ade_types::{Hash28, Hash32, SlotNo};

/// Decode a Conway transaction body from CBOR map.
///
/// Extends Babbage with keys 19 (voting_procedures), 20 (proposal_procedures),
/// 21 (treasury_value), 22 (donation). Key 6 (update) is removed in Conway.
pub fn decode_conway_tx_body(
    data: &[u8],
    offset: &mut usize,
) -> Result<ConwayTxBody, CodecError> {
    let map_enc = cbor::read_map_header(data, offset)?;
    let map_len = match map_enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Conway tx body must be definite-length map",
            });
        }
    };

    let mut inputs: Option<BTreeSet<TxIn>> = None;
    let mut outputs: Option<Vec<BabbageTxOut>> = None;
    let mut fee: Option<Coin> = None;
    let mut ttl: Option<SlotNo> = None;
    let mut certs: Option<Vec<u8>> = None;
    let mut withdrawals: Option<Vec<u8>> = None;
    let mut metadata_hash: Option<Hash32> = None;
    let mut validity_interval_start: Option<SlotNo> = None;
    let mut mint: Option<Vec<u8>> = None;
    let mut script_data_hash: Option<Hash32> = None;
    let mut collateral_inputs: Option<BTreeSet<TxIn>> = None;
    let mut required_signers: Option<BTreeSet<Hash28>> = None;
    let mut network_id: Option<u8> = None;
    let mut collateral_return: Option<BabbageTxOut> = None;
    let mut total_collateral: Option<Coin> = None;
    let mut reference_inputs: Option<BTreeSet<TxIn>> = None;
    let mut voting_procedures: Option<Vec<u8>> = None;
    let mut proposal_procedures: Option<Vec<u8>> = None;
    let mut treasury_value: Option<Coin> = None;
    let mut donation: Option<Coin> = None;

    for _ in 0..map_len {
        let (key, _) = cbor::read_uint(data, offset)?;
        match key {
            0 => {
                skip_optional_tag(data, offset)?;
                inputs = Some(decode_tx_inputs(data, offset)?);
            }
            1 => outputs = Some(decode_conway_outputs(data, offset)?),
            2 => {
                let (v, _) = cbor::read_uint(data, offset)?;
                fee = Some(Coin(v));
            }
            3 => {
                let (v, _) = cbor::read_uint(data, offset)?;
                ttl = Some(SlotNo(v));
            }
            4 => {
                let (start, end) = cbor::skip_item(data, offset)?;
                certs = Some(data[start..end].to_vec());
            }
            5 => {
                let (start, end) = cbor::skip_item(data, offset)?;
                withdrawals = Some(data[start..end].to_vec());
            }
            7 => metadata_hash = Some(crate::byron::read_hash32(data, offset)?),
            8 => {
                let (v, _) = cbor::read_uint(data, offset)?;
                validity_interval_start = Some(SlotNo(v));
            }
            9 => {
                let (start, end) = cbor::skip_item(data, offset)?;
                mint = Some(data[start..end].to_vec());
            }
            11 => script_data_hash = Some(crate::byron::read_hash32(data, offset)?),
            13 => {
                skip_optional_tag(data, offset)?;
                collateral_inputs = Some(decode_tx_inputs(data, offset)?);
            }
            14 => {
                skip_optional_tag(data, offset)?;
                required_signers = Some(decode_required_signers(data, offset)?);
            }
            15 => {
                let (v, _) = cbor::read_uint(data, offset)?;
                network_id = Some(v as u8);
            }
            16 => collateral_return = Some(decode_babbage_tx_out(data, offset)?),
            17 => {
                let (v, _) = cbor::read_uint(data, offset)?;
                total_collateral = Some(Coin(v));
            }
            18 => {
                skip_optional_tag(data, offset)?;
                reference_inputs = Some(decode_tx_inputs(data, offset)?);
            }
            19 => {
                let (start, end) = cbor::skip_item(data, offset)?;
                voting_procedures = Some(data[start..end].to_vec());
            }
            20 => {
                let (start, end) = cbor::skip_item(data, offset)?;
                proposal_procedures = Some(data[start..end].to_vec());
            }
            21 => {
                let (v, _) = cbor::read_uint(data, offset)?;
                treasury_value = Some(Coin(v));
            }
            22 => {
                let (v, _) = cbor::read_uint(data, offset)?;
                donation = Some(Coin(v));
            }
            _ => {
                let _ = cbor::skip_item(data, offset)?;
            }
        }
    }

    let inputs = inputs.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Conway tx body missing inputs (key 0)",
    })?;
    let outputs = outputs.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Conway tx body missing outputs (key 1)",
    })?;
    let fee = fee.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Conway tx body missing fee (key 2)",
    })?;

    Ok(ConwayTxBody {
        inputs,
        outputs,
        fee,
        ttl,
        certs,
        withdrawals,
        metadata_hash,
        validity_interval_start,
        mint,
        script_data_hash,
        collateral_inputs,
        required_signers,
        network_id,
        collateral_return,
        total_collateral,
        reference_inputs,
        voting_procedures,
        proposal_procedures,
        treasury_value,
        donation,
    })
}

fn decode_required_signers(
    data: &[u8],
    offset: &mut usize,
) -> Result<BTreeSet<Hash28>, CodecError> {
    skip_optional_tag(data, offset)?;
    let enc = cbor::read_array_header(data, offset)?;
    let count = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            let mut signers = BTreeSet::new();
            while !cbor::is_break(data, *offset)? {
                signers.insert(read_hash28(data, offset)?);
            }
            *offset += 1;
            return Ok(signers);
        }
    };
    let mut signers = BTreeSet::new();
    for _ in 0..count {
        signers.insert(read_hash28(data, offset)?);
    }
    Ok(signers)
}

fn decode_conway_outputs(
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<BabbageTxOut>, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    let count = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            let mut outputs = Vec::new();
            while !cbor::is_break(data, *offset)? {
                outputs.push(decode_babbage_tx_out(data, offset)?);
            }
            *offset += 1;
            return Ok(outputs);
        }
    };
    let mut outputs = Vec::with_capacity(count as usize);
    for _ in 0..count {
        outputs.push(decode_babbage_tx_out(data, offset)?);
    }
    Ok(outputs)
}
