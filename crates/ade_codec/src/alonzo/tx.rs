// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;

use crate::cbor::{self, ContainerEncoding, IntWidth};
use crate::error::CodecError;
use crate::shelley::tx::decode_tx_inputs;
use crate::traits::{AdeEncode, CodecContext};
use ade_types::alonzo::tx::{AlonzoTxBody, AlonzoTxOut};
use ade_types::tx::{Coin, TxIn};
use ade_types::{Hash28, Hash32, SlotNo};

/// Decode an Alonzo transaction body from CBOR map.
///
/// Extends Mary with keys 11 (script_data_hash), 13 (collateral),
/// 14 (required_signers), 15 (network_id). Outputs gain optional datum_hash.
pub fn decode_alonzo_tx_body(
    data: &[u8],
    offset: &mut usize,
) -> Result<AlonzoTxBody, CodecError> {
    let map_enc = cbor::read_map_header(data, offset)?;
    let map_len = match map_enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Alonzo tx body must be definite-length map",
            });
        }
    };

    let mut inputs: Option<BTreeSet<TxIn>> = None;
    let mut outputs: Option<Vec<AlonzoTxOut>> = None;
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

    for _ in 0..map_len {
        let (key, _) = cbor::read_uint(data, offset)?;
        match key {
            0 => inputs = Some(decode_tx_inputs(data, offset)?),
            1 => outputs = Some(decode_alonzo_outputs(data, offset)?),
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
            _ => {
                let _ = cbor::skip_item(data, offset)?;
            }
        }
    }

    let inputs = inputs.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Alonzo tx body missing inputs (key 0)",
    })?;
    let outputs = outputs.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Alonzo tx body missing outputs (key 1)",
    })?;
    let fee = fee.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Alonzo tx body missing fee (key 2)",
    })?;

    Ok(AlonzoTxBody {
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
    })
}

/// Decode Alonzo tx outputs — each output is [address, value] or [address, value, datum_hash].
fn decode_alonzo_outputs(
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<AlonzoTxOut>, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    let count = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            let mut outputs = Vec::new();
            while !cbor::is_break(data, *offset)? {
                outputs.push(decode_alonzo_tx_out(data, offset)?);
            }
            *offset += 1;
            return Ok(outputs);
        }
    };

    let mut outputs = Vec::with_capacity(count as usize);
    for _ in 0..count {
        outputs.push(decode_alonzo_tx_out(data, offset)?);
    }
    Ok(outputs)
}

/// Decode a single Alonzo tx output.
///
/// Wire format: `[address, value]` or `[address, value, datum_hash]`
/// Value is either `uint` (coin) or `[uint, multiasset]`.
fn decode_alonzo_tx_out(
    data: &[u8],
    offset: &mut usize,
) -> Result<AlonzoTxOut, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    let arr_len = match enc {
        ContainerEncoding::Definite(n, _) if n == 2 || n == 3 => n,
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Alonzo tx output must be array(2) or array(3)",
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
                    detail: "Alonzo value array must be array(2)",
                });
            }
        }
        let (coin_val, _) = cbor::read_uint(data, offset)?;
        let (ma_start, ma_end) = cbor::skip_item(data, offset)?;
        (Coin(coin_val), Some(data[ma_start..ma_end].to_vec()))
    } else {
        return Err(CodecError::UnexpectedCborType {
            offset: *offset,
            expected: "uint or array for value",
            actual: major,
        });
    };

    let datum_hash = if arr_len == 3 {
        Some(crate::byron::read_hash32(data, offset)?)
    } else {
        None
    };

    Ok(AlonzoTxOut {
        address,
        coin,
        multi_asset,
        datum_hash,
    })
}

/// Decode a set of required signer key hashes (28-byte Hash28).
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

pub(crate) fn read_hash28(
    data: &[u8],
    offset: &mut usize,
) -> Result<Hash28, CodecError> {
    let (bytes, _) = cbor::read_bytes(data, offset)?;
    if bytes.len() != 28 {
        return Err(CodecError::InvalidLength {
            offset: *offset - bytes.len(),
            detail: "expected 28-byte key hash",
        });
    }
    let mut arr = [0u8; 28];
    arr.copy_from_slice(&bytes);
    Ok(Hash28(arr))
}

impl AdeEncode for AlonzoTxOut {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        let arr_len: u64 = if self.datum_hash.is_some() { 3 } else { 2 };
        cbor::write_array_header(
            buf,
            ContainerEncoding::Definite(arr_len, IntWidth::Inline),
        );
        cbor::write_bytes_canonical(buf, &self.address);

        if let Some(ref ma) = self.multi_asset {
            cbor::write_array_header(
                buf,
                ContainerEncoding::Definite(2, IntWidth::Inline),
            );
            cbor::write_uint_canonical(buf, self.coin.0);
            buf.extend_from_slice(ma);
        } else {
            cbor::write_uint_canonical(buf, self.coin.0);
        }

        if let Some(ref dh) = self.datum_hash {
            cbor::write_bytes_canonical(buf, &dh.0);
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::traits::CodecContext;
    use ade_types::CardanoEra;

    fn ctx() -> CodecContext {
        CodecContext {
            era: CardanoEra::Alonzo,
        }
    }

    fn round_trip(data: &[u8]) {
        let mut offset = 0;
        let out = decode_alonzo_tx_out(data, &mut offset).unwrap();
        assert_eq!(offset, data.len(), "decoder must consume all bytes");
        let mut buf = Vec::new();
        out.ade_encode(&mut buf, &ctx()).unwrap();
        assert_eq!(buf.as_slice(), data, "encode must be byte-identical to input");
    }

    #[test]
    fn round_trip_coin_only_no_datum() {
        // [address(bstr(3)), uint(42)]
        let data = [0x82, 0x43, 0x01, 0x02, 0x03, 0x18, 0x2a];
        round_trip(&data);
    }

    #[test]
    fn round_trip_multi_asset_no_datum() {
        // [address(bstr(3)), [uint(10), {}]]
        let data = [0x82, 0x43, 0x01, 0x02, 0x03, 0x82, 0x0a, 0xa0];
        round_trip(&data);
    }

    #[test]
    fn round_trip_coin_with_datum() {
        // [address(bstr(3)), uint(42), bstr(32)[...]]
        let mut data: Vec<u8> = vec![0x83, 0x43, 0x01, 0x02, 0x03, 0x18, 0x2a, 0x58, 0x20];
        data.extend_from_slice(&[0xAA; 32]);
        round_trip(&data);
    }

    #[test]
    fn round_trip_multi_asset_with_datum() {
        // [address(bstr(3)), [uint(10), {1:...}], bstr(32)]
        let mut data: Vec<u8> = vec![
            0x83, 0x43, 0x01, 0x02, 0x03,
            0x82, 0x0a,
            0xa1, 0x41, 0x11, 0xa1, 0x41, 0x22, 0x18, 0x64,
            0x58, 0x20,
        ];
        data.extend_from_slice(&[0xBB; 32]);
        round_trip(&data);
    }
}
