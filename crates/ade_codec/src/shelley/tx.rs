// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;

use crate::cbor::{self, ContainerEncoding, IntWidth};
use crate::error::CodecError;
use crate::traits::{AdeEncode, CodecContext};
use ade_types::shelley::tx::{ShelleyTxBody, ShelleyTxOut};
use ade_types::tx::{Coin, TxIn};
use ade_types::{Hash32, SlotNo};

/// Decode a Shelley transaction body from CBOR map.
///
/// Shelley tx body is a CBOR map with integer keys 0–7:
/// - 0: inputs (set of [tx_hash, index])
/// - 1: outputs (array of [address, coin])
/// - 2: fee (uint)
/// - 3: ttl (uint)
/// - 4: certs (optional, opaque)
/// - 5: withdrawals (optional, opaque)
/// - 6: update (optional, opaque)
/// - 7: metadata_hash (optional, 32-byte hash)
pub fn decode_shelley_tx_body(
    data: &[u8],
    offset: &mut usize,
) -> Result<ShelleyTxBody, CodecError> {
    let map_enc = cbor::read_map_header(data, offset)?;
    let map_len = match map_enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Shelley tx body must be definite-length map",
            });
        }
    };

    let mut inputs: Option<BTreeSet<TxIn>> = None;
    let mut outputs: Option<Vec<ShelleyTxOut>> = None;
    let mut fee: Option<Coin> = None;
    let mut ttl: Option<SlotNo> = None;
    let mut certs: Option<Vec<u8>> = None;
    let mut withdrawals: Option<Vec<u8>> = None;
    let mut update: Option<Vec<u8>> = None;
    let mut metadata_hash: Option<Hash32> = None;

    for _ in 0..map_len {
        let (key, _) = cbor::read_uint(data, offset)?;
        match key {
            0 => {
                inputs = Some(decode_tx_inputs(data, offset)?);
            }
            1 => {
                outputs = Some(decode_shelley_tx_outputs(data, offset)?);
            }
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
            7 => {
                metadata_hash = Some(crate::byron::read_hash32(data, offset)?);
            }
            _ => {
                // Unknown key — skip the value
                let _ = cbor::skip_item(data, offset)?;
            }
        }
    }

    let inputs = inputs.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Shelley tx body missing inputs (key 0)",
    })?;
    let outputs = outputs.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Shelley tx body missing outputs (key 1)",
    })?;
    let fee = fee.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Shelley tx body missing fee (key 2)",
    })?;
    let ttl = ttl.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Shelley tx body missing ttl (key 3)",
    })?;

    Ok(ShelleyTxBody {
        inputs,
        outputs,
        fee,
        ttl,
        certs,
        withdrawals,
        update,
        metadata_hash,
    })
}

/// Decode a set of transaction inputs from CBOR.
///
/// Wire format: `set [input, ...]` where each input is `[tx_hash, index]`.
pub fn decode_tx_inputs(
    data: &[u8],
    offset: &mut usize,
) -> Result<BTreeSet<TxIn>, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    let count = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            // Handle indefinite-length arrays
            let mut inputs = BTreeSet::new();
            while !cbor::is_break(data, *offset)? {
                inputs.insert(decode_tx_in(data, offset)?);
            }
            *offset += 1; // consume break byte
            return Ok(inputs);
        }
    };

    let mut inputs = BTreeSet::new();
    for _ in 0..count {
        inputs.insert(decode_tx_in(data, offset)?);
    }
    Ok(inputs)
}

/// Decode a single post-Byron transaction input.
///
/// Wire format: `[tx_hash, index]`
pub fn decode_tx_in(data: &[u8], offset: &mut usize) -> Result<TxIn, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "tx input must be array(2)",
            });
        }
    }

    let tx_hash = crate::byron::read_hash32(data, offset)?;
    let (index, _) = cbor::read_uint(data, offset)?;

    Ok(TxIn {
        tx_hash,
        index: index as u16,
    })
}

fn decode_shelley_tx_outputs(
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<ShelleyTxOut>, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(n, _) => {
            let mut outputs = Vec::with_capacity(n as usize);
            for _ in 0..n {
                outputs.push(decode_shelley_tx_out(data, offset)?);
            }
            Ok(outputs)
        }
        ContainerEncoding::Indefinite => {
            let mut outputs = Vec::new();
            while !cbor::is_break(data, *offset)? {
                outputs.push(decode_shelley_tx_out(data, offset)?);
            }
            *offset += 1;
            Ok(outputs)
        }
    }
}

/// Decode a Shelley transaction output.
///
/// Wire format: `[address, coin]`
pub fn decode_shelley_tx_out(
    data: &[u8],
    offset: &mut usize,
) -> Result<ShelleyTxOut, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Shelley tx output must be array(2)",
            });
        }
    }

    // address: byte string
    let (address, _) = cbor::read_bytes(data, offset)?;

    // coin: uint
    let (coin_val, _) = cbor::read_uint(data, offset)?;

    Ok(ShelleyTxOut {
        address,
        coin: Coin(coin_val),
    })
}

// ---------------------------------------------------------------------------
// Encoding (for round-trip)
// ---------------------------------------------------------------------------

impl AdeEncode for ShelleyTxBody {
    fn ade_encode(&self, buf: &mut Vec<u8>, ctx: &CodecContext) -> Result<(), CodecError> {
        // Count how many map entries we need
        let mut count = 4u64; // inputs, outputs, fee, ttl are mandatory
        if self.certs.is_some() {
            count += 1;
        }
        if self.withdrawals.is_some() {
            count += 1;
        }
        if self.update.is_some() {
            count += 1;
        }
        if self.metadata_hash.is_some() {
            count += 1;
        }

        cbor::write_map_header(
            buf,
            ContainerEncoding::Definite(count, cbor::canonical_width(count)),
        );

        // key 0: inputs
        cbor::write_uint_canonical(buf, 0);
        encode_tx_inputs(buf, &self.inputs, ctx)?;

        // key 1: outputs
        cbor::write_uint_canonical(buf, 1);
        encode_shelley_tx_outputs(buf, &self.outputs, ctx)?;

        // key 2: fee
        cbor::write_uint_canonical(buf, 2);
        cbor::write_uint_canonical(buf, self.fee.0);

        // key 3: ttl
        cbor::write_uint_canonical(buf, 3);
        cbor::write_uint_canonical(buf, self.ttl.0);

        // key 4: certs (optional)
        if let Some(ref certs) = self.certs {
            cbor::write_uint_canonical(buf, 4);
            buf.extend_from_slice(certs);
        }

        // key 5: withdrawals (optional)
        if let Some(ref withdrawals) = self.withdrawals {
            cbor::write_uint_canonical(buf, 5);
            buf.extend_from_slice(withdrawals);
        }

        // key 6: update (optional)
        if let Some(ref update) = self.update {
            cbor::write_uint_canonical(buf, 6);
            buf.extend_from_slice(update);
        }

        // key 7: metadata_hash (optional)
        if let Some(ref hash) = self.metadata_hash {
            cbor::write_uint_canonical(buf, 7);
            cbor::write_bytes_canonical(buf, &hash.0);
        }

        Ok(())
    }
}

pub(crate) fn encode_tx_inputs(
    buf: &mut Vec<u8>,
    inputs: &BTreeSet<TxIn>,
    _ctx: &CodecContext,
) -> Result<(), CodecError> {
    let n = inputs.len() as u64;
    cbor::write_array_header(buf, ContainerEncoding::Definite(n, cbor::canonical_width(n)));
    for input in inputs {
        encode_tx_in(buf, input)?;
    }
    Ok(())
}

pub(crate) fn encode_tx_in(buf: &mut Vec<u8>, input: &TxIn) -> Result<(), CodecError> {
    cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    cbor::write_bytes_canonical(buf, &input.tx_hash.0);
    cbor::write_uint_canonical(buf, u64::from(input.index));
    Ok(())
}

fn encode_shelley_tx_outputs(
    buf: &mut Vec<u8>,
    outputs: &[ShelleyTxOut],
    _ctx: &CodecContext,
) -> Result<(), CodecError> {
    let n = outputs.len() as u64;
    cbor::write_array_header(buf, ContainerEncoding::Definite(n, cbor::canonical_width(n)));
    for output in outputs {
        encode_shelley_tx_out(buf, output)?;
    }
    Ok(())
}

fn encode_shelley_tx_out(buf: &mut Vec<u8>, output: &ShelleyTxOut) -> Result<(), CodecError> {
    cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    cbor::write_bytes_canonical(buf, &output.address);
    cbor::write_uint_canonical(buf, output.coin.0);
    Ok(())
}
