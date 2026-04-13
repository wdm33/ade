// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;

use crate::alonzo::tx::read_hash28;
use crate::cbor::{self, ContainerEncoding, IntWidth};
use crate::error::CodecError;
use crate::shelley::tx::decode_tx_inputs;
use crate::traits::{AdeEncode, CodecContext};
use ade_types::babbage::tx::{BabbageTxBody, BabbageTxOut};
use ade_types::tx::{Coin, TxIn};
use ade_types::{Hash28, Hash32, SlotNo};

/// Decode a Babbage transaction body from CBOR map.
///
/// Extends Alonzo with keys 16 (collateral_return), 17 (total_collateral),
/// 18 (reference_inputs). Outputs can be array or map format.
pub fn decode_babbage_tx_body(
    data: &[u8],
    offset: &mut usize,
) -> Result<BabbageTxBody, CodecError> {
    let map_enc = cbor::read_map_header(data, offset)?;
    let map_len = match map_enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Babbage tx body must be definite-length map",
            });
        }
    };

    let mut inputs: Option<BTreeSet<TxIn>> = None;
    let mut outputs: Option<Vec<BabbageTxOut>> = None;
    let mut fee: Option<Coin> = None;
    let mut ttl: Option<SlotNo> = None;
    let mut certs: Option<Vec<u8>> = None;
    let mut withdrawals: Option<Vec<u8>> = None;
    let mut update: Option<Vec<u8>> = None;
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

    for _ in 0..map_len {
        let (key, _) = cbor::read_uint(data, offset)?;
        match key {
            0 => inputs = Some(decode_tx_inputs(data, offset)?),
            1 => outputs = Some(decode_babbage_outputs(data, offset)?),
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
            6 => {
                let (start, end) = cbor::skip_item(data, offset)?;
                update = Some(data[start..end].to_vec());
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
            13 => collateral_inputs = Some(decode_tx_inputs(data, offset)?),
            14 => required_signers = Some(decode_required_signers(data, offset)?),
            15 => {
                let (v, _) = cbor::read_uint(data, offset)?;
                network_id = Some(v as u8);
            }
            16 => collateral_return = Some(decode_babbage_tx_out(data, offset)?),
            17 => {
                let (v, _) = cbor::read_uint(data, offset)?;
                total_collateral = Some(Coin(v));
            }
            18 => reference_inputs = Some(decode_tx_inputs(data, offset)?),
            _ => {
                let _ = cbor::skip_item(data, offset)?;
            }
        }
    }

    let inputs = inputs.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Babbage tx body missing inputs (key 0)",
    })?;
    let outputs = outputs.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Babbage tx body missing outputs (key 1)",
    })?;
    let fee = fee.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Babbage tx body missing fee (key 2)",
    })?;

    Ok(BabbageTxBody {
        inputs,
        outputs,
        fee,
        ttl,
        certs,
        withdrawals,
        update,
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
    })
}

fn decode_required_signers(
    data: &[u8],
    offset: &mut usize,
) -> Result<BTreeSet<Hash28>, CodecError> {
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

/// Decode Babbage tx outputs array.
fn decode_babbage_outputs(
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

/// Decode a single Babbage tx output — array or map format.
///
/// Array format: `[address, value, datum_option?, script_ref?]`
/// Map format: `{0: address, 1: value, 2?: datum_option, 3?: script_ref}`
pub(crate) fn decode_babbage_tx_out(
    data: &[u8],
    offset: &mut usize,
) -> Result<BabbageTxOut, CodecError> {
    let major = cbor::peek_major(data, *offset)?;

    if major == cbor::MAJOR_MAP {
        decode_babbage_tx_out_map(data, offset)
    } else if major == cbor::MAJOR_ARRAY {
        decode_babbage_tx_out_array(data, offset)
    } else {
        Err(CodecError::UnexpectedCborType {
            offset: *offset,
            expected: "array or map for Babbage tx output",
            actual: major,
        })
    }
}

/// Decode Babbage tx output in map format: `{0: address, 1: value, ...}`
fn decode_babbage_tx_out_map(
    data: &[u8],
    offset: &mut usize,
) -> Result<BabbageTxOut, CodecError> {
    let map_enc = cbor::read_map_header(data, offset)?;
    let map_len = match map_enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Babbage tx output map must be definite-length",
            });
        }
    };

    let mut address: Option<Vec<u8>> = None;
    let mut coin = Coin(0);
    let mut multi_asset: Option<Vec<u8>> = None;
    let mut datum_option: Option<Vec<u8>> = None;
    let mut script_ref: Option<Vec<u8>> = None;

    for _ in 0..map_len {
        let (key, _) = cbor::read_uint(data, offset)?;
        match key {
            0 => {
                let (addr, _) = cbor::read_bytes(data, offset)?;
                address = Some(addr);
            }
            1 => {
                let major = cbor::peek_major(data, *offset)?;
                if major == cbor::MAJOR_UNSIGNED {
                    let (v, _) = cbor::read_uint(data, offset)?;
                    coin = Coin(v);
                } else if major == cbor::MAJOR_ARRAY {
                    let val_enc = cbor::read_array_header(data, offset)?;
                    match val_enc {
                        ContainerEncoding::Definite(2, _) => {}
                        _ => {
                            return Err(CodecError::InvalidCborStructure {
                                offset: *offset,
                                detail: "Babbage value array must be array(2)",
                            });
                        }
                    }
                    let (v, _) = cbor::read_uint(data, offset)?;
                    coin = Coin(v);
                    let (ma_start, ma_end) = cbor::skip_item(data, offset)?;
                    multi_asset = Some(data[ma_start..ma_end].to_vec());
                } else {
                    return Err(CodecError::UnexpectedCborType {
                        offset: *offset,
                        expected: "uint or array for value",
                        actual: major,
                    });
                }
            }
            2 => {
                let (start, end) = cbor::skip_item(data, offset)?;
                datum_option = Some(data[start..end].to_vec());
            }
            3 => {
                let (start, end) = cbor::skip_item(data, offset)?;
                script_ref = Some(data[start..end].to_vec());
            }
            _ => {
                let _ = cbor::skip_item(data, offset)?;
            }
        }
    }

    let address = address.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Babbage tx output missing address (key 0)",
    })?;

    Ok(BabbageTxOut {
        address,
        coin,
        multi_asset,
        datum_option,
        script_ref,
    })
}

/// Encode a Babbage transaction body in canonical map form.
///
/// Extends the Alonzo body layout with keys 16 (collateral_return),
/// 17 (total_collateral), and 18 (reference_inputs). See
/// `encode_alonzo_tx_body` for notes on round-trip fidelity.
pub fn encode_babbage_tx_body(
    buf: &mut Vec<u8>,
    body: &BabbageTxBody,
    ctx: &CodecContext,
) -> Result<(), CodecError> {
    let mut count: u64 = 3; // inputs, outputs, fee mandatory
    if body.ttl.is_some() { count += 1; }
    if body.certs.is_some() { count += 1; }
    if body.withdrawals.is_some() { count += 1; }
    if body.update.is_some() { count += 1; }
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

    cbor::write_map_header(
        buf,
        ContainerEncoding::Definite(count, cbor::canonical_width(count)),
    );

    // Key 0: inputs
    cbor::write_uint_canonical(buf, 0);
    crate::shelley::tx::encode_tx_inputs(buf, &body.inputs, ctx)?;

    // Key 1: outputs (Babbage map form for each)
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

    // Key 3: ttl
    if let Some(SlotNo(v)) = body.ttl {
        cbor::write_uint_canonical(buf, 3);
        cbor::write_uint_canonical(buf, v);
    }

    // Keys 4-6: opaque
    if let Some(ref b) = body.certs {
        cbor::write_uint_canonical(buf, 4);
        buf.extend_from_slice(b);
    }
    if let Some(ref b) = body.withdrawals {
        cbor::write_uint_canonical(buf, 5);
        buf.extend_from_slice(b);
    }
    if let Some(ref b) = body.update {
        cbor::write_uint_canonical(buf, 6);
        buf.extend_from_slice(b);
    }

    // Key 7: metadata_hash
    if let Some(ref h) = body.metadata_hash {
        cbor::write_uint_canonical(buf, 7);
        cbor::write_bytes_canonical(buf, &h.0);
    }

    // Key 8: validity_interval_start
    if let Some(SlotNo(v)) = body.validity_interval_start {
        cbor::write_uint_canonical(buf, 8);
        cbor::write_uint_canonical(buf, v);
    }

    // Key 9: mint (opaque)
    if let Some(ref b) = body.mint {
        cbor::write_uint_canonical(buf, 9);
        buf.extend_from_slice(b);
    }

    // Key 11: script_data_hash
    if let Some(ref h) = body.script_data_hash {
        cbor::write_uint_canonical(buf, 11);
        cbor::write_bytes_canonical(buf, &h.0);
    }

    // Key 13: collateral_inputs
    if let Some(ref col) = body.collateral_inputs {
        cbor::write_uint_canonical(buf, 13);
        crate::shelley::tx::encode_tx_inputs(buf, col, ctx)?;
    }

    // Key 14: required_signers
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

    // Key 15: network_id
    if let Some(nid) = body.network_id {
        cbor::write_uint_canonical(buf, 15);
        cbor::write_uint_canonical(buf, nid as u64);
    }

    // Key 16: collateral_return (map-form output)
    if let Some(ref ret) = body.collateral_return {
        cbor::write_uint_canonical(buf, 16);
        encode_babbage_tx_out_map(buf, ret)?;
    }

    // Key 17: total_collateral
    if let Some(Coin(v)) = body.total_collateral {
        cbor::write_uint_canonical(buf, 17);
        cbor::write_uint_canonical(buf, v);
    }

    // Key 18: reference_inputs
    if let Some(ref refs) = body.reference_inputs {
        cbor::write_uint_canonical(buf, 18);
        crate::shelley::tx::encode_tx_inputs(buf, refs, ctx)?;
    }

    Ok(())
}

impl AdeEncode for BabbageTxBody {
    fn ade_encode(&self, buf: &mut Vec<u8>, ctx: &CodecContext) -> Result<(), CodecError> {
        encode_babbage_tx_body(buf, self, ctx)
    }
}

/// Encode a Babbage tx output in canonical map form.
///
/// `{0: address, 1: value, ?2: datum_option_raw, ?3: script_ref_raw}`
/// Value is `uint(coin)` when multi_asset is absent, else `[coin, multi_asset]`.
pub fn encode_babbage_tx_out_map(
    buf: &mut Vec<u8>,
    output: &BabbageTxOut,
) -> Result<(), CodecError> {
    let mut count: u64 = 2;
    if output.datum_option.is_some() {
        count += 1;
    }
    if output.script_ref.is_some() {
        count += 1;
    }
    cbor::write_map_header(buf, ContainerEncoding::Definite(count, IntWidth::Inline));

    cbor::write_uint_canonical(buf, 0);
    cbor::write_bytes_canonical(buf, &output.address);

    cbor::write_uint_canonical(buf, 1);
    if let Some(ref ma) = output.multi_asset {
        cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        cbor::write_uint_canonical(buf, output.coin.0);
        buf.extend_from_slice(ma);
    } else {
        cbor::write_uint_canonical(buf, output.coin.0);
    }

    if let Some(ref d) = output.datum_option {
        cbor::write_uint_canonical(buf, 2);
        buf.extend_from_slice(d);
    }

    if let Some(ref s) = output.script_ref {
        cbor::write_uint_canonical(buf, 3);
        buf.extend_from_slice(s);
    }

    Ok(())
}

/// Encode a Babbage tx output in legacy array form.
///
/// `[address, value, ?datum_option, ?script_ref]` — used when a mainnet tx
/// was originally decoded from an array-form output and byte-identity matters.
pub fn encode_babbage_tx_out_array(
    buf: &mut Vec<u8>,
    output: &BabbageTxOut,
) -> Result<(), CodecError> {
    let mut count: u64 = 2;
    if output.datum_option.is_some() {
        count += 1;
    }
    if output.script_ref.is_some() {
        count += 1;
    }
    cbor::write_array_header(buf, ContainerEncoding::Definite(count, IntWidth::Inline));

    cbor::write_bytes_canonical(buf, &output.address);

    if let Some(ref ma) = output.multi_asset {
        cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        cbor::write_uint_canonical(buf, output.coin.0);
        buf.extend_from_slice(ma);
    } else {
        cbor::write_uint_canonical(buf, output.coin.0);
    }

    if let Some(ref d) = output.datum_option {
        buf.extend_from_slice(d);
    }

    if let Some(ref s) = output.script_ref {
        buf.extend_from_slice(s);
    }

    Ok(())
}

impl AdeEncode for BabbageTxOut {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        encode_babbage_tx_out_map(buf, self)
    }
}

/// Decode Babbage tx output in legacy array format: `[address, value, datum_option?, script_ref?]`
fn decode_babbage_tx_out_array(
    data: &[u8],
    offset: &mut usize,
) -> Result<BabbageTxOut, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    let arr_len = match enc {
        ContainerEncoding::Definite(n, _) if (2..=4).contains(&n) => n,
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Babbage tx output array must have 2-4 elements",
            });
        }
    };

    let (address, _) = cbor::read_bytes(data, offset)?;

    let major = cbor::peek_major(data, *offset)?;
    let (coin, multi_asset) = if major == cbor::MAJOR_UNSIGNED {
        let (v, _) = cbor::read_uint(data, offset)?;
        (Coin(v), None)
    } else if major == cbor::MAJOR_ARRAY {
        let val_enc = cbor::read_array_header(data, offset)?;
        match val_enc {
            ContainerEncoding::Definite(2, _) => {}
            _ => {
                return Err(CodecError::InvalidCborStructure {
                    offset: *offset,
                    detail: "Babbage value array must be array(2)",
                });
            }
        }
        let (v, _) = cbor::read_uint(data, offset)?;
        let (ma_start, ma_end) = cbor::skip_item(data, offset)?;
        (Coin(v), Some(data[ma_start..ma_end].to_vec()))
    } else {
        return Err(CodecError::UnexpectedCborType {
            offset: *offset,
            expected: "uint or array for value",
            actual: major,
        });
    };

    let datum_option = if arr_len >= 3 {
        let (start, end) = cbor::skip_item(data, offset)?;
        Some(data[start..end].to_vec())
    } else {
        None
    };

    let script_ref = if arr_len >= 4 {
        let (start, end) = cbor::skip_item(data, offset)?;
        Some(data[start..end].to_vec())
    } else {
        None
    };

    Ok(BabbageTxOut {
        address,
        coin,
        multi_asset,
        datum_option,
        script_ref,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use ade_types::CardanoEra;

    fn ctx() -> CodecContext {
        CodecContext {
            era: CardanoEra::Babbage,
        }
    }

    fn round_trip_map(data: &[u8]) {
        let mut offset = 0;
        let out = decode_babbage_tx_out(data, &mut offset).unwrap();
        assert_eq!(offset, data.len(), "decoder must consume all bytes");
        let mut buf = Vec::new();
        out.ade_encode(&mut buf, &ctx()).unwrap();
        assert_eq!(buf.as_slice(), data, "map-form encode must be byte-identical");
    }

    fn round_trip_array(data: &[u8]) {
        let mut offset = 0;
        let out = decode_babbage_tx_out(data, &mut offset).unwrap();
        assert_eq!(offset, data.len(), "decoder must consume all bytes");
        let mut buf = Vec::new();
        encode_babbage_tx_out_array(&mut buf, &out).unwrap();
        assert_eq!(buf.as_slice(), data, "array-form encode must be byte-identical");
    }

    #[test]
    fn map_round_trip_coin_only() {
        // {0: bstr(3)[01,02,03], 1: uint(42)}
        let data = [0xa2, 0x00, 0x43, 0x01, 0x02, 0x03, 0x01, 0x18, 0x2a];
        round_trip_map(&data);
    }

    #[test]
    fn map_round_trip_multi_asset() {
        // {0: addr, 1: [uint(10), {}]}
        let data = [
            0xa2, 0x00, 0x43, 0x01, 0x02, 0x03, 0x01, 0x82, 0x0a, 0xa0,
        ];
        round_trip_map(&data);
    }

    #[test]
    fn map_round_trip_with_datum_option() {
        // {0: addr, 1: uint(42), 2: [0, bstr(32)]}
        let mut data: Vec<u8> = vec![
            0xa3, 0x00, 0x43, 0x01, 0x02, 0x03, 0x01, 0x18, 0x2a, 0x02, 0x82, 0x00, 0x58, 0x20,
        ];
        data.extend_from_slice(&[0xCC; 32]);
        round_trip_map(&data);
    }

    #[test]
    fn map_round_trip_with_datum_and_script_ref() {
        // {0: addr, 1: uint(42), 2: [1, bstr(...)], 3: bstr(wrapped_script)}
        let data: Vec<u8> = vec![
            0xa4, 0x00, 0x43, 0x01, 0x02, 0x03, 0x01, 0x18, 0x2a,
            0x02, 0x82, 0x01, 0x41, 0x99,
            0x03, 0x44, 0xaa, 0xbb, 0xcc, 0xdd,
        ];
        round_trip_map(&data);
    }

    #[test]
    fn array_round_trip_coin_only() {
        // [addr, uint(42)]
        let data = [0x82, 0x43, 0x01, 0x02, 0x03, 0x18, 0x2a];
        round_trip_array(&data);
    }

    #[test]
    fn array_round_trip_with_datum() {
        // [addr, uint(42), bstr(32)] — legacy Alonzo-compatible form
        let mut data: Vec<u8> = vec![0x83, 0x43, 0x01, 0x02, 0x03, 0x18, 0x2a, 0x58, 0x20];
        data.extend_from_slice(&[0xDD; 32]);
        round_trip_array(&data);
    }

    fn babbage_body_round_trip(body: BabbageTxBody) {
        let mut buf = Vec::new();
        <BabbageTxBody as AdeEncode>::ade_encode(&body, &mut buf, &ctx()).unwrap();
        let mut off = 0;
        let decoded = decode_babbage_tx_body(&buf, &mut off).unwrap();
        assert_eq!(off, buf.len(), "decoder must consume all bytes");
        assert_eq!(body, decoded, "body round-trip must preserve fields");
    }

    fn minimal_babbage_body() -> BabbageTxBody {
        let mut inputs = BTreeSet::new();
        inputs.insert(TxIn {
            tx_hash: Hash32([0xAA; 32]),
            index: 0,
        });
        BabbageTxBody {
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
    fn body_babbage_minimal() {
        babbage_body_round_trip(minimal_babbage_body());
    }

    #[test]
    fn body_babbage_with_plutus_and_babbage_fields() {
        let mut b = minimal_babbage_body();
        b.script_data_hash = Some(Hash32([0xCC; 32]));
        let mut col = BTreeSet::new();
        col.insert(TxIn {
            tx_hash: Hash32([0x99; 32]),
            index: 0,
        });
        b.collateral_inputs = Some(col.clone());
        b.reference_inputs = Some({
            let mut r = BTreeSet::new();
            r.insert(TxIn {
                tx_hash: Hash32([0x88; 32]),
                index: 1,
            });
            r
        });
        b.collateral_return = Some(BabbageTxOut {
            address: vec![0x60, 0xDE, 0xAD],
            coin: Coin(50_000),
            multi_asset: None,
            datum_option: None,
            script_ref: None,
        });
        b.total_collateral = Some(Coin(100_000));
        babbage_body_round_trip(b);
    }

    #[test]
    fn array_round_trip_full() {
        // [addr, [uint(10), {}], datum, script_ref]
        let data = [
            0x84, 0x43, 0x01, 0x02, 0x03,
            0x82, 0x0a, 0xa0,
            0x41, 0x11,
            0x42, 0xaa, 0xbb,
        ];
        round_trip_array(&data);
    }
}
