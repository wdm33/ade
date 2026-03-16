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

/// Decode a post-Byron block from inner CBOR bytes.
///
/// Handles both array(4) (Shelley/Allegra/Mary) and array(5) (Alonzo+)
/// block structures, and both array(15) and array(10) header bodies.
pub fn decode_shelley_block_inner(
    data: &[u8],
    offset: &mut usize,
) -> Result<ShelleyBlock, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    let block_len = match enc {
        ContainerEncoding::Definite(n @ 4 | n @ 5, _) => n,
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "post-Byron block must be array(4) or array(5)",
            });
        }
    };

    let header = decode_header(data, offset)?;

    // tx_bodies: read array header for tx_count, then capture as opaque
    let tx_start = *offset;
    let tx_enc = cbor::read_array_header(data, offset)?;
    let tx_count = match tx_enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                let _ = cbor::skip_item(data, offset)?;
            }
            n
        }
        ContainerEncoding::Indefinite => {
            let mut count = 0u64;
            while !cbor::is_break(data, *offset)? {
                let _ = cbor::skip_item(data, offset)?;
                count += 1;
            }
            *offset += 1;
            count
        }
    };
    let tx_bodies = data[tx_start..*offset].to_vec();

    let (ws_start, ws_end) = cbor::skip_item(data, offset)?;
    let witness_sets = data[ws_start..ws_end].to_vec();

    let (md_start, md_end) = cbor::skip_item(data, offset)?;
    let metadata = data[md_start..md_end].to_vec();

    let invalid_txs = if block_len == 5 {
        let (it_start, it_end) = cbor::skip_item(data, offset)?;
        Some(data[it_start..it_end].to_vec())
    } else {
        None
    };

    Ok(ShelleyBlock {
        header,
        tx_count,
        tx_bodies,
        witness_sets,
        metadata,
        invalid_txs,
    })
}

impl AdeEncode for ShelleyBlock {
    fn ade_encode(&self, buf: &mut Vec<u8>, ctx: &CodecContext) -> Result<(), CodecError> {
        let block_len = if self.invalid_txs.is_some() { 5u64 } else { 4 };
        cbor::write_array_header(
            buf,
            ContainerEncoding::Definite(block_len, IntWidth::Inline),
        );
        self.header.ade_encode(buf, ctx)?;
        buf.extend_from_slice(&self.tx_bodies);
        buf.extend_from_slice(&self.witness_sets);
        buf.extend_from_slice(&self.metadata);
        if let Some(ref it) = self.invalid_txs {
            buf.extend_from_slice(it);
        }
        Ok(())
    }
}

fn decode_header(data: &[u8], offset: &mut usize) -> Result<ShelleyHeader, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "header must be array(2)",
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
    let enc = cbor::read_array_header(data, offset)?;
    let hdr_len = match enc {
        ContainerEncoding::Definite(n @ 15 | n @ 10, _) => n,
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "header body must be array(15) or array(10)",
            });
        }
    };

    let (block_number, _) = cbor::read_uint(data, offset)?;
    let (slot, _) = cbor::read_uint(data, offset)?;
    let prev_hash = crate::byron::read_hash32(data, offset)?;
    let (issuer_vkey, _) = cbor::read_bytes(data, offset)?;
    let (vrf_vkey, _) = cbor::read_bytes(data, offset)?;

    let vrf = if hdr_len == 15 {
        // Shelley-Alonzo: two separate VRF certs
        let (nv_start, nv_end) = cbor::skip_item(data, offset)?;
        let nonce_vrf = data[nv_start..nv_end].to_vec();
        let (lv_start, lv_end) = cbor::skip_item(data, offset)?;
        let leader_vrf = data[lv_start..lv_end].to_vec();
        VrfData::Split {
            nonce_vrf,
            leader_vrf,
        }
    } else {
        // Babbage-Conway: single combined VRF result
        let (vr_start, vr_end) = cbor::skip_item(data, offset)?;
        let vrf_result = data[vr_start..vr_end].to_vec();
        VrfData::Combined { vrf_result }
    };

    let (body_size, _) = cbor::read_uint(data, offset)?;
    let body_hash = crate::byron::read_hash32(data, offset)?;

    let operational_cert = if hdr_len == 15 {
        // Inlined: 4 fields directly in the array
        let (hot_vkey, _) = cbor::read_bytes(data, offset)?;
        let (sequence_number, _) = cbor::read_uint(data, offset)?;
        let (kes_period, _) = cbor::read_uint(data, offset)?;
        let (sigma, _) = cbor::read_bytes(data, offset)?;
        OperationalCert {
            hot_vkey,
            sequence_number,
            kes_period,
            sigma,
        }
    } else {
        // Nested: array(4) [hot_vkey, seq, kes_period, sigma]
        let oc_enc = cbor::read_array_header(data, offset)?;
        match oc_enc {
            ContainerEncoding::Definite(4, _) => {}
            _ => {
                return Err(CodecError::InvalidCborStructure {
                    offset: *offset,
                    detail: "operational cert must be array(4)",
                });
            }
        }
        let (hot_vkey, _) = cbor::read_bytes(data, offset)?;
        let (sequence_number, _) = cbor::read_uint(data, offset)?;
        let (kes_period, _) = cbor::read_uint(data, offset)?;
        let (sigma, _) = cbor::read_bytes(data, offset)?;
        OperationalCert {
            hot_vkey,
            sequence_number,
            kes_period,
            sigma,
        }
    };

    let protocol_version = if hdr_len == 15 {
        // Inlined: 2 fields directly in the array
        let (major, _) = cbor::read_uint(data, offset)?;
        let (minor, _) = cbor::read_uint(data, offset)?;
        ProtocolVersion { major, minor }
    } else {
        // Nested: array(2) [major, minor]
        let pv_enc = cbor::read_array_header(data, offset)?;
        match pv_enc {
            ContainerEncoding::Definite(2, _) => {}
            _ => {
                return Err(CodecError::InvalidCborStructure {
                    offset: *offset,
                    detail: "protocol version must be array(2)",
                });
            }
        }
        let (major, _) = cbor::read_uint(data, offset)?;
        let (minor, _) = cbor::read_uint(data, offset)?;
        ProtocolVersion { major, minor }
    };

    Ok(ShelleyHeaderBody {
        block_number,
        slot,
        prev_hash,
        issuer_vkey,
        vrf_vkey,
        vrf,
        body_size,
        body_hash,
        operational_cert,
        protocol_version,
    })
}

impl AdeEncode for ShelleyHeaderBody {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        let is_split = matches!(self.vrf, VrfData::Split { .. });
        let hdr_len = if is_split { 15u64 } else { 10 };

        cbor::write_array_header(buf, ContainerEncoding::Definite(hdr_len, IntWidth::Inline));
        cbor::write_uint_canonical(buf, self.block_number);
        cbor::write_uint_canonical(buf, self.slot);
        cbor::write_bytes_canonical(buf, &self.prev_hash.0);
        cbor::write_bytes_canonical(buf, &self.issuer_vkey);
        cbor::write_bytes_canonical(buf, &self.vrf_vkey);

        match &self.vrf {
            VrfData::Split {
                nonce_vrf,
                leader_vrf,
            } => {
                buf.extend_from_slice(nonce_vrf);
                buf.extend_from_slice(leader_vrf);
            }
            VrfData::Combined { vrf_result } => {
                buf.extend_from_slice(vrf_result);
            }
        }

        cbor::write_uint_canonical(buf, self.body_size);
        cbor::write_bytes_canonical(buf, &self.body_hash.0);

        if is_split {
            // Inlined operational cert
            cbor::write_bytes_canonical(buf, &self.operational_cert.hot_vkey);
            cbor::write_uint_canonical(buf, self.operational_cert.sequence_number);
            cbor::write_uint_canonical(buf, self.operational_cert.kes_period);
            cbor::write_bytes_canonical(buf, &self.operational_cert.sigma);
            // Inlined protocol version
            cbor::write_uint_canonical(buf, self.protocol_version.major);
            cbor::write_uint_canonical(buf, self.protocol_version.minor);
        } else {
            // Nested operational cert: array(4)
            cbor::write_array_header(buf, ContainerEncoding::Definite(4, IntWidth::Inline));
            cbor::write_bytes_canonical(buf, &self.operational_cert.hot_vkey);
            cbor::write_uint_canonical(buf, self.operational_cert.sequence_number);
            cbor::write_uint_canonical(buf, self.operational_cert.kes_period);
            cbor::write_bytes_canonical(buf, &self.operational_cert.sigma);
            // Nested protocol version: array(2)
            cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            cbor::write_uint_canonical(buf, self.protocol_version.major);
            cbor::write_uint_canonical(buf, self.protocol_version.minor);
        }

        Ok(())
    }
}
