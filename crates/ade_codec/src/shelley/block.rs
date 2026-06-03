// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::cbor::{self, ContainerEncoding, IntWidth};
use crate::error::CodecError;
use crate::shelley::opcert::{read_opcert_fields_from, write_opcert_fields_into};
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

/// Decode the header_body `prev_hash` field — the closed grammar `$hash32 / null`
/// (cardano-ledger `PrevHash`). POSITION-BLIND: the `Genesis | Block` decision is a
/// pure function of the CBOR token (null `0xf6` vs a 32-byte string), never the
/// `block_number`. The block-position rule (block 0 requires `Genesis`) lives in the
/// validator, not in this decoder.
fn decode_prev_hash(data: &[u8], offset: &mut usize) -> Result<PrevHash, CodecError> {
    if data.get(*offset) == Some(&0xf6) {
        *offset += 1;
        Ok(PrevHash::Genesis)
    } else {
        Ok(PrevHash::Block(crate::byron::read_hash32(data, offset)?))
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
    let prev_hash = decode_prev_hash(data, offset)?;
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
        // Inlined: 4 fields directly in the array.
        read_opcert_fields_from(data, offset).map_err(|_| CodecError::InvalidCborStructure {
            offset: *offset,
            detail: "operational cert fields (inline)",
        })?
    } else {
        // Nested: array(4) [hot_vkey, seq, kes_period, sigma].
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
        read_opcert_fields_from(data, offset).map_err(|_| CodecError::InvalidCborStructure {
            offset: *offset,
            detail: "operational cert fields (nested)",
        })?
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
        match &self.prev_hash {
            PrevHash::Genesis => cbor::write_null(buf),
            PrevHash::Block(h) => cbor::write_bytes_canonical(buf, &h.0),
        }
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
            // Inlined operational cert.
            write_opcert_fields_into(buf, &self.operational_cert);
            // Inlined protocol version
            cbor::write_uint_canonical(buf, self.protocol_version.major);
            cbor::write_uint_canonical(buf, self.protocol_version.minor);
        } else {
            // Nested operational cert: array(4).
            cbor::write_array_header(buf, ContainerEncoding::Definite(4, IntWidth::Inline));
            write_opcert_fields_into(buf, &self.operational_cert);
            // Nested protocol version: array(2)
            cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            cbor::write_uint_canonical(buf, self.protocol_version.major);
            cbor::write_uint_canonical(buf, self.protocol_version.minor);
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod prevhash_codec_tests {
    use super::*;
    use ade_types::{CardanoEra, Hash32};

    fn ctx() -> CodecContext {
        CodecContext {
            era: CardanoEra::Conway,
        }
    }

    /// A round-trippable Babbage-Conway (array(10)) header body with the given
    /// `prev_hash`. `vrf_result` is one valid CBOR item (the decoder does
    /// `skip_item`); `hot_vkey`/`sigma` are `read_bytes`-symmetric.
    fn sample_body(prev_hash: PrevHash) -> ShelleyHeaderBody {
        let mut vrf_result = Vec::new();
        cbor::write_bytes_canonical(&mut vrf_result, &[0x33u8; 32]);
        ShelleyHeaderBody {
            block_number: 0,
            slot: 0,
            prev_hash,
            issuer_vkey: vec![0x11; 32],
            vrf_vkey: vec![0x22; 32],
            vrf: VrfData::Combined { vrf_result },
            body_size: 0,
            body_hash: Hash32([0x44; 32]),
            operational_cert: OperationalCert {
                hot_vkey: vec![0x55; 32],
                sequence_number: 3,
                kes_period: 10,
                sigma: vec![0x66; 64],
            },
            protocol_version: ProtocolVersion { major: 9, minor: 0 },
        }
    }

    fn encode(hb: &ShelleyHeaderBody) -> Vec<u8> {
        let mut buf = Vec::new();
        hb.ade_encode(&mut buf, &ctx()).unwrap();
        buf
    }

    #[test]
    fn prevhash_genesis_round_trips_as_null() {
        let mut off = 0;
        assert_eq!(
            decode_prev_hash(&[0xf6], &mut off).unwrap(),
            PrevHash::Genesis
        );
        assert_eq!(off, 1, "null consumes exactly one byte");
        let hb = sample_body(PrevHash::Genesis);
        let mut o = 0;
        assert_eq!(
            decode_header_body(&encode(&hb), &mut o).unwrap().prev_hash,
            PrevHash::Genesis
        );
    }

    #[test]
    fn prevhash_block_round_trips_as_hash32() {
        let h = Hash32([0xAB; 32]);
        let mut enc = Vec::new();
        cbor::write_bytes_canonical(&mut enc, &h.0);
        let mut off = 0;
        assert_eq!(
            decode_prev_hash(&enc, &mut off).unwrap(),
            PrevHash::Block(h.clone())
        );
        let hb = sample_body(PrevHash::Block(h.clone()));
        let mut o = 0;
        assert_eq!(
            decode_header_body(&encode(&hb), &mut o).unwrap().prev_hash,
            PrevHash::Block(h)
        );
    }

    #[test]
    fn prevhash_codec_is_position_blind() {
        // `decode_prev_hash` takes NO block_number: the Genesis|Block decision is a
        // pure function of the CBOR token. A null token decodes to Genesis even on a
        // body whose block_number is > 0 (the position rule lives in the validator, S3).
        let h = Hash32([0x01; 32]);
        let mut hbytes = Vec::new();
        cbor::write_bytes_canonical(&mut hbytes, &h.0);
        let (mut o1, mut o2) = (0usize, 0usize);
        assert_eq!(
            decode_prev_hash(&[0xf6], &mut o1).unwrap(),
            PrevHash::Genesis
        );
        assert_eq!(
            decode_prev_hash(&hbytes, &mut o2).unwrap(),
            PrevHash::Block(h)
        );
        let mut body = sample_body(PrevHash::Genesis);
        body.block_number = 999;
        let mut o = 0;
        assert_eq!(
            decode_header_body(&encode(&body), &mut o).unwrap().prev_hash,
            PrevHash::Genesis
        );
    }

    #[test]
    fn genesis_successor_header_round_trips_with_null_prev() {
        let hb = sample_body(PrevHash::Genesis);
        let bytes = encode(&hb);
        // array(10)=0x8a, block_number 0=0x00, slot 0=0x00, prev_hash at index 3 = null.
        assert_eq!(bytes[3], 0xf6, "genesis-successor prev_hash is CBOR null");
        let mut o = 0;
        let decoded = decode_header_body(&bytes, &mut o).unwrap();
        assert_eq!(decoded, hb);
        assert_eq!(encode(&decoded), bytes, "re-encode is byte-identical");
    }

    #[test]
    fn block_header_prev_hash_byte_identical_after_migration() {
        // The Block(h) case encodes EXACTLY as the pre-migration flat Hash32 did: a
        // canonical 32-byte bytestring, never null. (Real-block corpus round-trips in
        // tests/{shelley,allegra_mary,full_corpus}_round_trip.rs corroborate on real data.)
        let h = Hash32([0xCD; 32]);
        let hb = sample_body(PrevHash::Block(h.clone()));
        let bytes = encode(&hb);
        let mut expected_prev = Vec::new();
        cbor::write_bytes_canonical(&mut expected_prev, &h.0);
        assert_ne!(bytes[3], 0xf6, "Block prev_hash is never null");
        assert_eq!(
            &bytes[3..3 + expected_prev.len()],
            &expected_prev[..],
            "Block prev_hash encodes as canonical hash32"
        );
        let mut o = 0;
        assert_eq!(
            decode_header_body(&bytes, &mut o).unwrap().prev_hash,
            PrevHash::Block(h)
        );
    }
}
