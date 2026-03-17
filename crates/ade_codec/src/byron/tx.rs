// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::cbor::{self, ContainerEncoding, IntWidth};
use crate::error::CodecError;
use crate::preserved::PreservedCbor;
use crate::traits::{AdeEncode, CodecContext};
use ade_types::address::Address;
use ade_types::byron::tx::*;
use ade_types::tx::Coin;

/// Decode a Byron transaction body from CBOR.
///
/// Byron tx body is: `array(3) [inputs, outputs, attributes]`
/// where inputs is `array [input, ...]` and outputs is `array [output, ...]`.
pub fn decode_byron_tx_body(data: &[u8], offset: &mut usize) -> Result<ByronTxBody, CodecError> {
    let body_enc = cbor::read_array_header(data, offset)?;
    match body_enc {
        ContainerEncoding::Definite(3, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Byron tx body must be array(3)",
            });
        }
    }

    // inputs: array of ByronTxIn
    let inputs = decode_byron_tx_inputs(data, offset)?;

    // outputs: array of ByronTxOut
    let outputs = decode_byron_tx_outputs(data, offset)?;

    // attributes: opaque map (preserved for round-trip)
    let (attr_start, attr_end) = cbor::skip_item(data, offset)?;
    let attributes = data[attr_start..attr_end].to_vec();

    Ok(ByronTxBody {
        inputs,
        outputs,
        attributes,
    })
}

fn decode_byron_tx_inputs(
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<ByronTxIn>, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    let count = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Byron tx inputs must be definite-length array",
            });
        }
    };

    let mut inputs = Vec::with_capacity(count as usize);
    for _ in 0..count {
        inputs.push(decode_byron_tx_in(data, offset)?);
    }
    Ok(inputs)
}

/// Decode a single Byron transaction input.
///
/// Wire format: `[tag(0), tag(24, cbor_bytes([tx_hash, index]))]`
/// The input is a 2-element array where:
/// - element 0: uint type tag (0 for regular inputs)
/// - element 1: tag(24) wrapping a CBOR byte string containing [hash, index]
fn decode_byron_tx_in(data: &[u8], offset: &mut usize) -> Result<ByronTxIn, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Byron tx input must be array(2)",
            });
        }
    }

    // Type tag (0 = TxInUtxo)
    let (_type_tag, _) = cbor::read_uint(data, offset)?;

    // tag(24) wrapping a CBOR byte string
    let (tag_val, _) = cbor::read_tag(data, offset)?;
    if tag_val != 24 {
        return Err(CodecError::InvalidCborStructure {
            offset: *offset,
            detail: "Byron tx input expected tag(24)",
        });
    }

    // Read the embedded CBOR bytes
    let (inner_bytes, _) = cbor::read_bytes(data, offset)?;

    // Decode the inner [tx_hash, index] from the embedded CBOR
    let mut inner_offset = 0;
    let inner_enc = cbor::read_array_header(&inner_bytes, &mut inner_offset)?;
    match inner_enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Byron tx input inner must be array(2)",
            });
        }
    }

    let tx_hash = crate::byron::read_hash32(&inner_bytes, &mut inner_offset)?;
    let (index, _) = cbor::read_uint(&inner_bytes, &mut inner_offset)?;

    Ok(ByronTxIn {
        tx_hash,
        index: index as u32,
    })
}

fn decode_byron_tx_outputs(
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<ByronTxOut>, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    let count = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Byron tx outputs must be definite-length array",
            });
        }
    };

    let mut outputs = Vec::with_capacity(count as usize);
    for _ in 0..count {
        outputs.push(decode_byron_tx_out(data, offset)?);
    }
    Ok(outputs)
}

/// Decode a single Byron transaction output.
///
/// Wire format: `[address, coin]`
fn decode_byron_tx_out(data: &[u8], offset: &mut usize) -> Result<ByronTxOut, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Byron tx output must be array(2)",
            });
        }
    }

    // address: Byron addresses are CBOR-in-CBOR (tag(24) wrapping bytes)
    // Capture the full address bytes for Address::Byron
    let addr_start = *offset;
    let _ = cbor::skip_item(data, offset)?;
    let address = Address::Byron(data[addr_start..*offset].to_vec());

    // coin: unsigned integer
    let (coin_val, _) = cbor::read_uint(data, offset)?;
    let coin = Coin(coin_val);

    Ok(ByronTxOut { address, coin })
}

/// Decode a Byron witness.
///
/// Wire format: `[type_tag, tag(24, cbor_bytes([xvk, signature]))]`
pub fn decode_byron_witness(data: &[u8], offset: &mut usize) -> Result<ByronWitness, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Byron witness must be array(2)",
            });
        }
    }

    let (witness_type, _) = cbor::read_uint(data, offset)?;

    // tag(24) wrapping embedded CBOR bytes
    let (tag_val, _) = cbor::read_tag(data, offset)?;
    if tag_val != 24 {
        return Err(CodecError::InvalidCborStructure {
            offset: *offset,
            detail: "Byron witness expected tag(24)",
        });
    }

    let (inner_bytes, _) = cbor::read_bytes(data, offset)?;

    // Decode inner: [xvk, signature]
    let mut inner_offset = 0;
    let inner_enc = cbor::read_array_header(&inner_bytes, &mut inner_offset)?;
    match inner_enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Byron witness inner must be array(2)",
            });
        }
    }

    let (xvk, _) = cbor::read_bytes(&inner_bytes, &mut inner_offset)?;
    let (signature, _) = cbor::read_bytes(&inner_bytes, &mut inner_offset)?;

    Ok(ByronWitness {
        witness_type: witness_type as u8,
        xvk,
        signature,
    })
}

/// Decode a full Byron transaction from the tx_payload.
///
/// Each entry in the Byron block body tx_payload is:
/// `[tx_body, [witnesses]]`
pub fn decode_byron_tx(data: &[u8], offset: &mut usize) -> Result<ByronTx, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Byron tx must be array(2) [body, witnesses]",
            });
        }
    }

    let body = decode_byron_tx_body(data, offset)?;

    // witnesses: array of witnesses
    let wit_enc = cbor::read_array_header(data, offset)?;
    let wit_count = match wit_enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Byron witnesses must be definite-length array",
            });
        }
    };

    let mut witnesses = Vec::with_capacity(wit_count as usize);
    for _ in 0..wit_count {
        witnesses.push(decode_byron_witness(data, offset)?);
    }

    Ok(ByronTx { body, witnesses })
}

/// Decode all transactions from a Byron block body.
///
/// Byron block body is: `array(4) [tx_payload, ssc_payload, dlg_payload, upd_payload]`
/// tx_payload is: `array [[tx, witnesses], ...]`
///
/// Returns a list of `PreservedCbor<ByronTx>` — each tx preserves its wire bytes.
pub fn decode_byron_block_txs(body_cbor: &[u8]) -> Result<Vec<PreservedCbor<ByronTx>>, CodecError> {
    let mut offset = 0;

    // body: array(4)
    let body_enc = cbor::read_array_header(body_cbor, &mut offset)?;
    match body_enc {
        ContainerEncoding::Definite(4, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset,
                detail: "Byron block body must be array(4)",
            });
        }
    }

    // tx_payload: array of [tx, [witnesses]] — may be indefinite-length
    let tx_enc = cbor::read_array_header(body_cbor, &mut offset)?;
    let mut txs = Vec::new();
    match tx_enc {
        ContainerEncoding::Definite(n, _) => {
            txs.reserve(n as usize);
            for _ in 0..n {
                let tx_start = offset;
                let tx = decode_byron_tx(body_cbor, &mut offset)?;
                let wire = body_cbor[tx_start..offset].to_vec();
                txs.push(PreservedCbor::new(wire, tx));
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(body_cbor, offset)? {
                let tx_start = offset;
                let tx = decode_byron_tx(body_cbor, &mut offset)?;
                let wire = body_cbor[tx_start..offset].to_vec();
                txs.push(PreservedCbor::new(wire, tx));
            }
            let _break = offset + 1; // break byte consumed
        }
    }

    Ok(txs)
}

// ---------------------------------------------------------------------------
// Encoding (for round-trip)
// ---------------------------------------------------------------------------

impl AdeEncode for ByronTxBody {
    fn ade_encode(&self, buf: &mut Vec<u8>, ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_array_header(buf, ContainerEncoding::Definite(3, IntWidth::Inline));

        // inputs
        cbor::write_array_header(
            buf,
            ContainerEncoding::Definite(self.inputs.len() as u64, IntWidth::Inline),
        );
        for input in &self.inputs {
            input.ade_encode(buf, ctx)?;
        }

        // outputs
        cbor::write_array_header(
            buf,
            ContainerEncoding::Definite(self.outputs.len() as u64, IntWidth::Inline),
        );
        for output in &self.outputs {
            output.ade_encode(buf, ctx)?;
        }

        // attributes (opaque)
        buf.extend_from_slice(&self.attributes);

        Ok(())
    }
}

impl AdeEncode for ByronTxIn {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));

        // type tag 0
        cbor::write_uint_canonical(buf, 0);

        // tag(24) wrapping CBOR bytes containing [tx_hash, index]
        cbor::write_tag(buf, 24, IntWidth::I8);

        let mut inner = Vec::new();
        cbor::write_array_header(&mut inner, ContainerEncoding::Definite(2, IntWidth::Inline));
        cbor::write_bytes_canonical(&mut inner, &self.tx_hash.0);
        cbor::write_uint_canonical(&mut inner, u64::from(self.index));

        cbor::write_bytes_canonical(buf, &inner);

        Ok(())
    }
}

impl AdeEncode for ByronTxOut {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));

        // address (preserved raw bytes)
        buf.extend_from_slice(self.address.as_bytes());

        // coin
        cbor::write_uint_canonical(buf, self.coin.0);

        Ok(())
    }
}

impl AdeEncode for ByronWitness {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));

        cbor::write_uint_canonical(buf, u64::from(self.witness_type));

        // tag(24) wrapping embedded CBOR
        cbor::write_tag(buf, 24, IntWidth::I8);

        let mut inner = Vec::new();
        cbor::write_array_header(&mut inner, ContainerEncoding::Definite(2, IntWidth::Inline));
        cbor::write_bytes_canonical(&mut inner, &self.xvk);
        cbor::write_bytes_canonical(&mut inner, &self.signature);

        cbor::write_bytes_canonical(buf, &inner);

        Ok(())
    }
}

impl AdeEncode for ByronTx {
    fn ade_encode(&self, buf: &mut Vec<u8>, ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));

        self.body.ade_encode(buf, ctx)?;

        cbor::write_array_header(
            buf,
            ContainerEncoding::Definite(self.witnesses.len() as u64, IntWidth::Inline),
        );
        for witness in &self.witnesses {
            witness.ade_encode(buf, ctx)?;
        }

        Ok(())
    }
}
