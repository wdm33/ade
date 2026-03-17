// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;

use crate::cbor::{self, ContainerEncoding};
use crate::error::CodecError;
use crate::shelley::tx::decode_tx_inputs;
use crate::traits::{AdeEncode, CodecContext};
use ade_types::mary::tx::{MaryTxBody, MaryTxOut};
use ade_types::tx::{Coin, TxIn};
use ade_types::{Hash32, SlotNo};

/// Decode a Mary transaction body from CBOR map.
///
/// Same as Allegra but with key 9 (mint) added, and outputs can be
/// multi-asset (value is either uint or [uint, multiasset_map]).
pub fn decode_mary_tx_body(
    data: &[u8],
    offset: &mut usize,
) -> Result<MaryTxBody, CodecError> {
    let map_enc = cbor::read_map_header(data, offset)?;
    let map_len = match map_enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Mary tx body must be definite-length map",
            });
        }
    };

    let mut inputs: Option<BTreeSet<TxIn>> = None;
    let mut outputs: Option<Vec<MaryTxOut>> = None;
    let mut fee: Option<Coin> = None;
    let mut ttl: Option<SlotNo> = None;
    let mut certs: Option<Vec<u8>> = None;
    let mut withdrawals: Option<Vec<u8>> = None;
    let mut update: Option<Vec<u8>> = None;
    let mut metadata_hash: Option<Hash32> = None;
    let mut validity_interval_start: Option<SlotNo> = None;
    let mut mint: Option<Vec<u8>> = None;

    for _ in 0..map_len {
        let (key, _) = cbor::read_uint(data, offset)?;
        match key {
            0 => {
                inputs = Some(decode_tx_inputs(data, offset)?);
            }
            1 => {
                outputs = Some(decode_mary_tx_outputs(data, offset)?);
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
            8 => {
                let (v, _) = cbor::read_uint(data, offset)?;
                validity_interval_start = Some(SlotNo(v));
            }
            9 => {
                let (start, end) = cbor::skip_item(data, offset)?;
                mint = Some(data[start..end].to_vec());
            }
            _ => {
                let _ = cbor::skip_item(data, offset)?;
            }
        }
    }

    let inputs = inputs.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Mary tx body missing inputs (key 0)",
    })?;
    let outputs = outputs.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Mary tx body missing outputs (key 1)",
    })?;
    let fee = fee.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Mary tx body missing fee (key 2)",
    })?;

    Ok(MaryTxBody {
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
    })
}

fn decode_mary_tx_outputs(
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<MaryTxOut>, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    let count = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Mary tx outputs must be definite-length array",
            });
        }
    };

    let mut outputs = Vec::with_capacity(count as usize);
    for _ in 0..count {
        outputs.push(decode_mary_tx_out(data, offset)?);
    }
    Ok(outputs)
}

/// Decode a Mary transaction output.
///
/// Wire format: `[address, value]`
/// where value is either `uint` (pure lovelace) or `[uint, multiasset_map]`.
pub fn decode_mary_tx_out(
    data: &[u8],
    offset: &mut usize,
) -> Result<MaryTxOut, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Mary tx output must be array(2)",
            });
        }
    }

    // address: byte string
    let (address, _) = cbor::read_bytes(data, offset)?;

    // value: uint OR [uint, multiasset_map]
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
                    detail: "Mary value array must be array(2)",
                });
            }
        }
        let (coin_val, _) = cbor::read_uint(data, offset)?;
        let (ma_start, ma_end) = cbor::skip_item(data, offset)?;
        (Coin(coin_val), Some(data[ma_start..ma_end].to_vec()))
    } else {
        return Err(CodecError::UnexpectedCborType {
            offset: *offset,
            expected: "uint or array for Mary value",
            actual: major,
        });
    };

    Ok(MaryTxOut {
        address,
        coin,
        multi_asset,
    })
}

// ---------------------------------------------------------------------------
// Encoding (for round-trip)
// ---------------------------------------------------------------------------

impl AdeEncode for MaryTxBody {
    fn ade_encode(&self, buf: &mut Vec<u8>, ctx: &CodecContext) -> Result<(), CodecError> {
        let mut count = 3u64; // inputs, outputs, fee mandatory
        if self.ttl.is_some() {
            count += 1;
        }
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
        if self.validity_interval_start.is_some() {
            count += 1;
        }
        if self.mint.is_some() {
            count += 1;
        }

        cbor::write_map_header(
            buf,
            ContainerEncoding::Definite(count, cbor::canonical_width(count)),
        );

        // key 0: inputs
        cbor::write_uint_canonical(buf, 0);
        let n = self.inputs.len() as u64;
        cbor::write_array_header(
            buf,
            ContainerEncoding::Definite(n, cbor::canonical_width(n)),
        );
        for input in &self.inputs {
            cbor::write_array_header(
                buf,
                ContainerEncoding::Definite(2, cbor::IntWidth::Inline),
            );
            cbor::write_bytes_canonical(buf, &input.tx_hash.0);
            cbor::write_uint_canonical(buf, u64::from(input.index));
        }

        // key 1: outputs
        cbor::write_uint_canonical(buf, 1);
        let n = self.outputs.len() as u64;
        cbor::write_array_header(
            buf,
            ContainerEncoding::Definite(n, cbor::canonical_width(n)),
        );
        for output in &self.outputs {
            output.ade_encode(buf, ctx)?;
        }

        // key 2: fee
        cbor::write_uint_canonical(buf, 2);
        cbor::write_uint_canonical(buf, self.fee.0);

        if let Some(ref ttl) = self.ttl {
            cbor::write_uint_canonical(buf, 3);
            cbor::write_uint_canonical(buf, ttl.0);
        }

        if let Some(ref certs) = self.certs {
            cbor::write_uint_canonical(buf, 4);
            buf.extend_from_slice(certs);
        }

        if let Some(ref withdrawals) = self.withdrawals {
            cbor::write_uint_canonical(buf, 5);
            buf.extend_from_slice(withdrawals);
        }

        if let Some(ref update) = self.update {
            cbor::write_uint_canonical(buf, 6);
            buf.extend_from_slice(update);
        }

        if let Some(ref hash) = self.metadata_hash {
            cbor::write_uint_canonical(buf, 7);
            cbor::write_bytes_canonical(buf, &hash.0);
        }

        if let Some(ref vis) = self.validity_interval_start {
            cbor::write_uint_canonical(buf, 8);
            cbor::write_uint_canonical(buf, vis.0);
        }

        if let Some(ref mint) = self.mint {
            cbor::write_uint_canonical(buf, 9);
            buf.extend_from_slice(mint);
        }

        Ok(())
    }
}

impl AdeEncode for MaryTxOut {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_array_header(
            buf,
            ContainerEncoding::Definite(2, cbor::IntWidth::Inline),
        );
        cbor::write_bytes_canonical(buf, &self.address);

        if let Some(ref ma) = self.multi_asset {
            cbor::write_array_header(
                buf,
                ContainerEncoding::Definite(2, cbor::IntWidth::Inline),
            );
            cbor::write_uint_canonical(buf, self.coin.0);
            buf.extend_from_slice(ma);
        } else {
            cbor::write_uint_canonical(buf, self.coin.0);
        }

        Ok(())
    }
}
