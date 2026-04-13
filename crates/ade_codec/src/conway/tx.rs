// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;

use crate::alonzo::tx::read_hash28;
use crate::babbage::tx::{decode_babbage_tx_out, encode_babbage_tx_out_map};
use crate::cbor::{self, ContainerEncoding, IntWidth};
use crate::error::CodecError;
use crate::shelley::tx::decode_tx_inputs;
use crate::traits::{AdeEncode, CodecContext};

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

/// Encode a Conway transaction body in canonical map form.
///
/// Extends Babbage with keys 19 (voting_procedures), 20
/// (proposal_procedures), 21 (treasury_value), 22 (donation).
/// Conway removes key 6 (protocol param update).
pub fn encode_conway_tx_body(
    buf: &mut Vec<u8>,
    body: &ConwayTxBody,
    ctx: &CodecContext,
) -> Result<(), CodecError> {
    let mut count: u64 = 3;
    if body.ttl.is_some() { count += 1; }
    if body.certs.is_some() { count += 1; }
    if body.withdrawals.is_some() { count += 1; }
    if body.metadata_hash.is_some() { count += 1; }
    if body.validity_interval_start.is_some() { count += 1; }
    if body.mint.is_some() { count += 1; }
    if body.script_data_hash.is_some() { count += 1; }
    if body.collateral_inputs.is_some() { count += 1; }
    if body.required_signers.is_some() { count += 1; }
    if body.network_id.is_some() { count += 1; }
    if body.collateral_return.is_some() { count += 1; }
    if body.total_collateral.is_some() { count += 1; }
    if body.reference_inputs.is_some() { count += 1; }
    if body.voting_procedures.is_some() { count += 1; }
    if body.proposal_procedures.is_some() { count += 1; }
    if body.treasury_value.is_some() { count += 1; }
    if body.donation.is_some() { count += 1; }

    cbor::write_map_header(
        buf,
        ContainerEncoding::Definite(count, cbor::canonical_width(count)),
    );

    // Key 0: inputs
    cbor::write_uint_canonical(buf, 0);
    crate::shelley::tx::encode_tx_inputs(buf, &body.inputs, ctx)?;

    // Key 1: outputs
    cbor::write_uint_canonical(buf, 1);
    cbor::write_array_header(
        buf,
        ContainerEncoding::Definite(body.outputs.len() as u64, cbor::canonical_width(body.outputs.len() as u64)),
    );
    for o in &body.outputs {
        encode_babbage_tx_out_map(buf, o)?;
    }

    // Key 2: fee
    cbor::write_uint_canonical(buf, 2);
    cbor::write_uint_canonical(buf, body.fee.0);

    if let Some(SlotNo(v)) = body.ttl {
        cbor::write_uint_canonical(buf, 3);
        cbor::write_uint_canonical(buf, v);
    }
    if let Some(ref b) = body.certs {
        cbor::write_uint_canonical(buf, 4);
        buf.extend_from_slice(b);
    }
    if let Some(ref b) = body.withdrawals {
        cbor::write_uint_canonical(buf, 5);
        buf.extend_from_slice(b);
    }
    // Conway removed key 6 (update).
    if let Some(ref h) = body.metadata_hash {
        cbor::write_uint_canonical(buf, 7);
        cbor::write_bytes_canonical(buf, &h.0);
    }
    if let Some(SlotNo(v)) = body.validity_interval_start {
        cbor::write_uint_canonical(buf, 8);
        cbor::write_uint_canonical(buf, v);
    }
    if let Some(ref b) = body.mint {
        cbor::write_uint_canonical(buf, 9);
        buf.extend_from_slice(b);
    }
    if let Some(ref h) = body.script_data_hash {
        cbor::write_uint_canonical(buf, 11);
        cbor::write_bytes_canonical(buf, &h.0);
    }
    if let Some(ref col) = body.collateral_inputs {
        cbor::write_uint_canonical(buf, 13);
        crate::shelley::tx::encode_tx_inputs(buf, col, ctx)?;
    }
    if let Some(ref signers) = body.required_signers {
        cbor::write_uint_canonical(buf, 14);
        cbor::write_array_header(
            buf,
            ContainerEncoding::Definite(signers.len() as u64, cbor::canonical_width(signers.len() as u64)),
        );
        for s in signers {
            cbor::write_bytes_canonical(buf, &s.0);
        }
    }
    if let Some(nid) = body.network_id {
        cbor::write_uint_canonical(buf, 15);
        cbor::write_uint_canonical(buf, nid as u64);
    }
    if let Some(ref ret) = body.collateral_return {
        cbor::write_uint_canonical(buf, 16);
        encode_babbage_tx_out_map(buf, ret)?;
    }
    if let Some(Coin(v)) = body.total_collateral {
        cbor::write_uint_canonical(buf, 17);
        cbor::write_uint_canonical(buf, v);
    }
    if let Some(ref refs) = body.reference_inputs {
        cbor::write_uint_canonical(buf, 18);
        crate::shelley::tx::encode_tx_inputs(buf, refs, ctx)?;
    }
    if let Some(ref b) = body.voting_procedures {
        cbor::write_uint_canonical(buf, 19);
        buf.extend_from_slice(b);
    }
    if let Some(ref b) = body.proposal_procedures {
        cbor::write_uint_canonical(buf, 20);
        buf.extend_from_slice(b);
    }
    if let Some(Coin(v)) = body.treasury_value {
        cbor::write_uint_canonical(buf, 21);
        cbor::write_uint_canonical(buf, v);
    }
    if let Some(Coin(v)) = body.donation {
        cbor::write_uint_canonical(buf, 22);
        cbor::write_uint_canonical(buf, v);
    }

    // Silence unused-import warning when no optional body fields fire.
    let _ = IntWidth::Inline;

    Ok(())
}

impl AdeEncode for ConwayTxBody {
    fn ade_encode(&self, buf: &mut Vec<u8>, ctx: &CodecContext) -> Result<(), CodecError> {
        encode_conway_tx_body(buf, self, ctx)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use ade_types::CardanoEra;
    use std::collections::BTreeSet;

    fn ctx() -> CodecContext {
        CodecContext {
            era: CardanoEra::Conway,
        }
    }

    fn round_trip(body: ConwayTxBody) {
        let mut buf = Vec::new();
        <ConwayTxBody as AdeEncode>::ade_encode(&body, &mut buf, &ctx()).unwrap();
        let mut off = 0;
        let decoded = decode_conway_tx_body(&buf, &mut off).unwrap();
        assert_eq!(off, buf.len(), "decoder must consume all bytes");
        assert_eq!(body, decoded, "body round-trip must preserve fields");
    }

    fn minimal() -> ConwayTxBody {
        let mut inputs = BTreeSet::new();
        inputs.insert(TxIn {
            tx_hash: Hash32([0xAA; 32]),
            index: 0,
        });
        ConwayTxBody {
            inputs,
            outputs: vec![BabbageTxOut {
                address: vec![0x60, 0x01, 0x02, 0x03, 0x04],
                coin: Coin(1_000_000),
                multi_asset: None,
                datum_option: None,
                script_ref: None,
            }],
            fee: Coin(200_000),
            ttl: None,
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
    fn body_conway_minimal() {
        round_trip(minimal());
    }

    #[test]
    fn body_conway_with_governance_fields() {
        let mut b = minimal();
        b.treasury_value = Some(Coin(1_000_000_000));
        b.donation = Some(Coin(500_000));
        round_trip(b);
    }

    #[test]
    fn body_conway_with_plutus_and_refs() {
        let mut b = minimal();
        b.script_data_hash = Some(Hash32([0xCC; 32]));
        let mut col = BTreeSet::new();
        col.insert(TxIn { tx_hash: Hash32([0x99; 32]), index: 0 });
        b.collateral_inputs = Some(col);
        let mut refs = BTreeSet::new();
        refs.insert(TxIn { tx_hash: Hash32([0x88; 32]), index: 0 });
        b.reference_inputs = Some(refs);
        round_trip(b);
    }
}
