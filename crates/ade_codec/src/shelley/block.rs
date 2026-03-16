// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::cbor::{self, ContainerEncoding, IntWidth};
use crate::error::CodecError;
use crate::traits::{AdeEncode, CodecContext};
use ade_types::shelley::block::*;

pub fn decode_shelley_block_inner(
    data: &[u8],
    offset: &mut usize,
) -> Result<ShelleyBlock, CodecError> {
    // array(4): [header, tx_bodies, witness_sets, metadata]
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(4, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Shelley block must be array(4)",
            });
        }
    }

    let header = decode_header(data, offset)?;

    // tx_bodies: read array header for tx_count, then capture as opaque
    let tx_start = *offset;
    let tx_enc = cbor::read_array_header(data, offset)?;
    let tx_count = match tx_enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            // Count items in indefinite array
            let mut count = 0u64;
            while !cbor::is_break(data, *offset)? {
                let _ = cbor::skip_item(data, offset)?;
                count += 1;
            }
            *offset += 1; // skip break byte
            count
        }
    };
    // For definite, skip all tx body items
    if matches!(tx_enc, ContainerEncoding::Definite(_, _)) {
        for _ in 0..tx_count {
            let _ = cbor::skip_item(data, offset)?;
        }
    }
    let tx_bodies = data[tx_start..*offset].to_vec();

    // witness_sets: opaque
    let (ws_start, ws_end) = cbor::skip_item(data, offset)?;
    let witness_sets = data[ws_start..ws_end].to_vec();

    // metadata: opaque
    let (md_start, md_end) = cbor::skip_item(data, offset)?;
    let metadata = data[md_start..md_end].to_vec();

    Ok(ShelleyBlock {
        header,
        tx_count,
        tx_bodies,
        witness_sets,
        metadata,
    })
}

impl AdeEncode for ShelleyBlock {
    fn ade_encode(&self, buf: &mut Vec<u8>, ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_array_header(buf, ContainerEncoding::Definite(4, IntWidth::Inline));
        self.header.ade_encode(buf, ctx)?;
        buf.extend_from_slice(&self.tx_bodies);
        buf.extend_from_slice(&self.witness_sets);
        buf.extend_from_slice(&self.metadata);
        Ok(())
    }
}

fn decode_header(data: &[u8], offset: &mut usize) -> Result<ShelleyHeader, CodecError> {
    // array(2): [header_body, kes_signature]
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Shelley header must be array(2)",
            });
        }
    }

    let body = decode_header_body(data, offset)?;

    let (sig_start, sig_end) = cbor::skip_item(data, offset)?;
    let kes_signature = data[sig_start..sig_end].to_vec();

    Ok(ShelleyHeader {
        body,
        kes_signature,
    })
}

impl AdeEncode for ShelleyHeader {
    fn ade_encode(&self, buf: &mut Vec<u8>, ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        self.body.ade_encode(buf, ctx)?;
        buf.extend_from_slice(&self.kes_signature);
        Ok(())
    }
}

fn decode_header_body(data: &[u8], offset: &mut usize) -> Result<ShelleyHeaderBody, CodecError> {
    // array(15): [block_number, slot, prev_hash, issuer_vkey, vrf_vkey,
    //             nonce_vrf, leader_vrf, body_size, body_hash,
    //             hot_vkey, sequence_number, kes_period, sigma,
    //             major_version, minor_version]
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(15, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "Shelley header body must be array(15)",
            });
        }
    }

    let (block_number, _) = cbor::read_uint(data, offset)?;
    let (slot, _) = cbor::read_uint(data, offset)?;
    let prev_hash = crate::byron::read_hash32(data, offset)?;

    let (issuer_vkey, _) = cbor::read_bytes(data, offset)?;
    let (vrf_vkey, _) = cbor::read_bytes(data, offset)?;

    // nonce_vrf: opaque (array(2) [bytes, bytes])
    let (nv_start, nv_end) = cbor::skip_item(data, offset)?;
    let nonce_vrf = data[nv_start..nv_end].to_vec();

    // leader_vrf: opaque
    let (lv_start, lv_end) = cbor::skip_item(data, offset)?;
    let leader_vrf = data[lv_start..lv_end].to_vec();

    let (body_size, _) = cbor::read_uint(data, offset)?;
    let body_hash = crate::byron::read_hash32(data, offset)?;

    // operational_cert fields (inlined): hot_vkey, sequence_number, kes_period, sigma
    let (hot_vkey, _) = cbor::read_bytes(data, offset)?;
    let (sequence_number, _) = cbor::read_uint(data, offset)?;
    let (kes_period, _) = cbor::read_uint(data, offset)?;
    let (sigma, _) = cbor::read_bytes(data, offset)?;

    // protocol_version (inlined): major, minor
    let (major, _) = cbor::read_uint(data, offset)?;
    let (minor, _) = cbor::read_uint(data, offset)?;

    Ok(ShelleyHeaderBody {
        block_number,
        slot,
        prev_hash,
        issuer_vkey,
        vrf_vkey,
        nonce_vrf,
        leader_vrf,
        body_size,
        body_hash,
        operational_cert: OperationalCert {
            hot_vkey,
            sequence_number,
            kes_period,
            sigma,
        },
        protocol_version: ProtocolVersion { major, minor },
    })
}

impl AdeEncode for ShelleyHeaderBody {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_array_header(buf, ContainerEncoding::Definite(15, IntWidth::Inline));
        cbor::write_uint_canonical(buf, self.block_number);
        cbor::write_uint_canonical(buf, self.slot);
        cbor::write_bytes_canonical(buf, &self.prev_hash.0);
        cbor::write_bytes_canonical(buf, &self.issuer_vkey);
        cbor::write_bytes_canonical(buf, &self.vrf_vkey);
        buf.extend_from_slice(&self.nonce_vrf);
        buf.extend_from_slice(&self.leader_vrf);
        cbor::write_uint_canonical(buf, self.body_size);
        cbor::write_bytes_canonical(buf, &self.body_hash.0);
        // operational_cert (inlined)
        cbor::write_bytes_canonical(buf, &self.operational_cert.hot_vkey);
        cbor::write_uint_canonical(buf, self.operational_cert.sequence_number);
        cbor::write_uint_canonical(buf, self.operational_cert.kes_period);
        cbor::write_bytes_canonical(buf, &self.operational_cert.sigma);
        // protocol_version (inlined)
        cbor::write_uint_canonical(buf, self.protocol_version.major);
        cbor::write_uint_canonical(buf, self.protocol_version.minor);
        Ok(())
    }
}
