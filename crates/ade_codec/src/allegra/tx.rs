// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;

use crate::cbor::{self, ContainerEncoding};
use crate::error::CodecError;
use crate::shelley::tx::{decode_shelley_tx_out, decode_tx_inputs};
use crate::traits::{AdeEncode, CodecContext};
use ade_types::allegra::tx::AllegraTxBody;
use ade_types::shelley::tx::ShelleyTxOut;
use ade_types::tx::{Coin, TxIn};
use ade_types::{Hash32, SlotNo};

/// Decode an Allegra transaction body from CBOR map.
///
/// Same as Shelley but with key 8 (validity_interval_start) added,
/// and ttl (key 3) becomes optional.
pub fn decode_allegra_tx_body(
    data: &[u8],
    offset: &mut usize,
) -> Result<AllegraTxBody, CodecError> {
    let map_enc = cbor::read_map_header(data, offset)?;
    let map_len = match map_enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Allegra tx body must be definite-length map",
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
    let mut validity_interval_start: Option<SlotNo> = None;

    for _ in 0..map_len {
        let (key, _) = cbor::read_uint(data, offset)?;
        match key {
            0 => {
                inputs = Some(decode_tx_inputs(data, offset)?);
            }
            1 => {
                outputs = Some(decode_allegra_outputs(data, offset)?);
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
            _ => {
                let _ = cbor::skip_item(data, offset)?;
            }
        }
    }

    let inputs = inputs.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Allegra tx body missing inputs (key 0)",
    })?;
    let outputs = outputs.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Allegra tx body missing outputs (key 1)",
    })?;
    let fee = fee.ok_or(CodecError::InvalidCborStructure {
        offset: *offset,
        detail: "Allegra tx body missing fee (key 2)",
    })?;

    Ok(AllegraTxBody {
        inputs,
        outputs,
        fee,
        ttl,
        certs,
        withdrawals,
        update,
        metadata_hash,
        validity_interval_start,
    })
}

fn decode_allegra_outputs(
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<ShelleyTxOut>, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    let count = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            let mut outputs = Vec::new();
            while !cbor::is_break(data, *offset)? {
                outputs.push(decode_shelley_tx_out(data, offset)?);
            }
            *offset += 1;
            return Ok(outputs);
        }
    };

    let mut outputs = Vec::with_capacity(count as usize);
    for _ in 0..count {
        outputs.push(decode_shelley_tx_out(data, offset)?);
    }
    Ok(outputs)
}

// ---------------------------------------------------------------------------
// Encoding (for round-trip)
// ---------------------------------------------------------------------------

impl AdeEncode for AllegraTxBody {
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

        // key 3: ttl (optional)
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

        Ok(())
    }
}

impl AdeEncode for ShelleyTxOut {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_array_header(
            buf,
            ContainerEncoding::Definite(2, cbor::IntWidth::Inline),
        );
        cbor::write_bytes_canonical(buf, &self.address);
        cbor::write_uint_canonical(buf, self.coin.0);
        Ok(())
    }
}
