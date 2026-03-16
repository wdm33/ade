// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::cbor::{self, ContainerEncoding, IntWidth};
use crate::error::CodecError;
use crate::traits::{AdeEncode, CodecContext};
use ade_types::byron::block::*;

// ---------------------------------------------------------------------------
// Byron EBB decode/encode
// ---------------------------------------------------------------------------

pub(crate) fn decode_ebb_block(
    data: &[u8],
    offset: &mut usize,
) -> Result<ByronEbbBlock, CodecError> {
    // array(3): [header, body, extra]
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(3, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "EBB block must be array(3)",
            });
        }
    }

    let header = decode_ebb_header(data, offset)?;

    let (body_start, body_end) = cbor::skip_item(data, offset)?;
    let body = data[body_start..body_end].to_vec();

    let (extra_start, extra_end) = cbor::skip_item(data, offset)?;
    let extra = data[extra_start..extra_end].to_vec();

    Ok(ByronEbbBlock {
        header,
        body,
        extra,
    })
}

impl AdeEncode for ByronEbbBlock {
    fn ade_encode(&self, buf: &mut Vec<u8>, ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_array_header(buf, ContainerEncoding::Definite(3, IntWidth::Inline));
        self.header.ade_encode(buf, ctx)?;
        buf.extend_from_slice(&self.body);
        buf.extend_from_slice(&self.extra);
        Ok(())
    }
}

fn decode_ebb_header(data: &[u8], offset: &mut usize) -> Result<ByronEbbHeader, CodecError> {
    // array(5): [protocol_magic, prev_hash, body_proof, consensus_data, extra_data]
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(5, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "EBB header must be array(5)",
            });
        }
    }

    let (pm, _) = cbor::read_uint(data, offset)?;
    let protocol_magic = pm as u32;

    let prev_hash = super::read_hash32(data, offset)?;
    let body_proof = super::read_hash32(data, offset)?;

    // consensus_data: array(2) [epoch, array(1) [chain_difficulty]]
    let cd_enc = cbor::read_array_header(data, offset)?;
    match cd_enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "EBB consensus data must be array(2)",
            });
        }
    }
    let (epoch, _) = cbor::read_uint(data, offset)?;

    // chain_difficulty: array(1) [uint]
    let diff_enc = cbor::read_array_header(data, offset)?;
    match diff_enc {
        ContainerEncoding::Definite(1, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "chain difficulty must be array(1)",
            });
        }
    }
    let (chain_difficulty, _) = cbor::read_uint(data, offset)?;

    // extra_data: opaque
    let (ed_start, ed_end) = cbor::skip_item(data, offset)?;
    let extra_data = data[ed_start..ed_end].to_vec();

    Ok(ByronEbbHeader {
        protocol_magic,
        prev_hash,
        body_proof,
        epoch,
        chain_difficulty,
        extra_data,
    })
}

impl AdeEncode for ByronEbbHeader {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_array_header(buf, ContainerEncoding::Definite(5, IntWidth::Inline));
        cbor::write_uint_canonical(buf, u64::from(self.protocol_magic));
        cbor::write_bytes_canonical(buf, &self.prev_hash.0);
        cbor::write_bytes_canonical(buf, &self.body_proof.0);

        // consensus_data: array(2) [epoch, array(1) [chain_difficulty]]
        cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        cbor::write_uint_canonical(buf, self.epoch);
        cbor::write_array_header(buf, ContainerEncoding::Definite(1, IntWidth::Inline));
        cbor::write_uint_canonical(buf, self.chain_difficulty);

        buf.extend_from_slice(&self.extra_data);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Byron Regular Block decode/encode
// ---------------------------------------------------------------------------

pub(crate) fn decode_regular_block(
    data: &[u8],
    offset: &mut usize,
) -> Result<ByronRegularBlock, CodecError> {
    // array(3): [header, body, extra]
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(3, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "regular block must be array(3)",
            });
        }
    }

    let header = decode_regular_header(data, offset)?;

    let (body_start, body_end) = cbor::skip_item(data, offset)?;
    let body = data[body_start..body_end].to_vec();

    let (extra_start, extra_end) = cbor::skip_item(data, offset)?;
    let extra = data[extra_start..extra_end].to_vec();

    Ok(ByronRegularBlock {
        header,
        body,
        extra,
    })
}

impl AdeEncode for ByronRegularBlock {
    fn ade_encode(&self, buf: &mut Vec<u8>, ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_array_header(buf, ContainerEncoding::Definite(3, IntWidth::Inline));
        self.header.ade_encode(buf, ctx)?;
        buf.extend_from_slice(&self.body);
        buf.extend_from_slice(&self.extra);
        Ok(())
    }
}

fn decode_regular_header(
    data: &[u8],
    offset: &mut usize,
) -> Result<ByronRegularHeader, CodecError> {
    // array(5): [protocol_magic, prev_hash, body_proof, consensus_data, extra_data]
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(5, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "regular header must be array(5)",
            });
        }
    }

    let (pm, _) = cbor::read_uint(data, offset)?;
    let protocol_magic = pm as u32;

    let prev_hash = super::read_hash32(data, offset)?;

    // body_proof: opaque (4-element proof array)
    let (bp_start, bp_end) = cbor::skip_item(data, offset)?;
    let body_proof = data[bp_start..bp_end].to_vec();

    let consensus_data = decode_consensus_data(data, offset)?;

    // extra_data: opaque
    let (ed_start, ed_end) = cbor::skip_item(data, offset)?;
    let extra_data = data[ed_start..ed_end].to_vec();

    Ok(ByronRegularHeader {
        protocol_magic,
        prev_hash,
        body_proof,
        consensus_data,
        extra_data,
    })
}

impl AdeEncode for ByronRegularHeader {
    fn ade_encode(&self, buf: &mut Vec<u8>, ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_array_header(buf, ContainerEncoding::Definite(5, IntWidth::Inline));
        cbor::write_uint_canonical(buf, u64::from(self.protocol_magic));
        cbor::write_bytes_canonical(buf, &self.prev_hash.0);
        buf.extend_from_slice(&self.body_proof);
        self.consensus_data.ade_encode(buf, ctx)?;
        buf.extend_from_slice(&self.extra_data);
        Ok(())
    }
}

fn decode_consensus_data(
    data: &[u8],
    offset: &mut usize,
) -> Result<ByronConsensusData, CodecError> {
    // array(4): [slot_id, delegator_pubkey, chain_difficulty, block_sig]
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(4, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "consensus data must be array(4)",
            });
        }
    }

    // slot_id: array(2) [epoch, slot_in_epoch]
    let slot_enc = cbor::read_array_header(data, offset)?;
    match slot_enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "slot_id must be array(2)",
            });
        }
    }
    let (epoch, _) = cbor::read_uint(data, offset)?;
    let (slot_in_epoch, _) = cbor::read_uint(data, offset)?;

    // delegator_pubkey: byte string
    let (pubkey, _) = cbor::read_bytes(data, offset)?;

    // chain_difficulty: array(1) [uint]
    let diff_enc = cbor::read_array_header(data, offset)?;
    match diff_enc {
        ContainerEncoding::Definite(1, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "chain difficulty must be array(1)",
            });
        }
    }
    let (chain_difficulty, _) = cbor::read_uint(data, offset)?;

    // block_sig: opaque
    let (sig_start, sig_end) = cbor::skip_item(data, offset)?;
    let block_sig = data[sig_start..sig_end].to_vec();

    Ok(ByronConsensusData {
        epoch,
        slot_in_epoch,
        delegator_pubkey: pubkey,
        chain_difficulty,
        block_sig,
    })
}

impl AdeEncode for ByronConsensusData {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_array_header(buf, ContainerEncoding::Definite(4, IntWidth::Inline));

        // slot_id: array(2) [epoch, slot_in_epoch]
        cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        cbor::write_uint_canonical(buf, self.epoch);
        cbor::write_uint_canonical(buf, self.slot_in_epoch);

        // delegator_pubkey
        cbor::write_bytes_canonical(buf, &self.delegator_pubkey);

        // chain_difficulty: array(1) [uint]
        cbor::write_array_header(buf, ContainerEncoding::Definite(1, IntWidth::Inline));
        cbor::write_uint_canonical(buf, self.chain_difficulty);

        // block_sig: opaque
        buf.extend_from_slice(&self.block_sig);
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn ebb_ctx() -> CodecContext {
        CodecContext {
            era: ade_types::CardanoEra::ByronEbb,
        }
    }

    fn reg_ctx() -> CodecContext {
        CodecContext {
            era: ade_types::CardanoEra::ByronRegular,
        }
    }

    #[test]
    fn ebb_header_encode_decode_round_trip() {
        let header = ByronEbbHeader {
            protocol_magic: 764824073,
            prev_hash: ade_types::Hash32([0xab; 32]),
            body_proof: ade_types::Hash32([0xcd; 32]),
            epoch: 42,
            chain_difficulty: 100,
            extra_data: vec![0x81, 0xa0], // array(1)[map(0)]
        };
        let mut buf = Vec::new();
        header.ade_encode(&mut buf, &ebb_ctx()).unwrap();

        let mut offset = 0;
        let decoded = decode_ebb_header(&buf, &mut offset).unwrap();
        assert_eq!(decoded.protocol_magic, header.protocol_magic);
        assert_eq!(decoded.prev_hash, header.prev_hash);
        assert_eq!(decoded.body_proof, header.body_proof);
        assert_eq!(decoded.epoch, header.epoch);
        assert_eq!(decoded.chain_difficulty, header.chain_difficulty);
        assert_eq!(decoded.extra_data, header.extra_data);
        assert_eq!(offset, buf.len());
    }

    #[test]
    fn regular_consensus_encode_decode_round_trip() {
        let cd = ByronConsensusData {
            epoch: 5,
            slot_in_epoch: 1234,
            delegator_pubkey: vec![0x42; 64],
            chain_difficulty: 5678,
            block_sig: vec![0x82, 0x00, 0x40], // array(2)[0, bytes(0)]
        };
        let mut buf = Vec::new();
        cd.ade_encode(&mut buf, &reg_ctx()).unwrap();

        let mut offset = 0;
        let decoded = decode_consensus_data(&buf, &mut offset).unwrap();
        assert_eq!(decoded.epoch, cd.epoch);
        assert_eq!(decoded.slot_in_epoch, cd.slot_in_epoch);
        assert_eq!(decoded.delegator_pubkey, cd.delegator_pubkey);
        assert_eq!(decoded.chain_difficulty, cd.chain_difficulty);
        assert_eq!(decoded.block_sig, cd.block_sig);
    }

    #[test]
    fn ebb_block_encode_decode_round_trip() {
        let block = ByronEbbBlock {
            header: ByronEbbHeader {
                protocol_magic: 764824073,
                prev_hash: ade_types::Hash32([0xaa; 32]),
                body_proof: ade_types::Hash32([0xbb; 32]),
                epoch: 0,
                chain_difficulty: 0,
                extra_data: vec![0x81, 0xa0],
            },
            body: vec![0x80],  // empty array
            extra: vec![0xa0], // empty map
        };
        let mut buf = Vec::new();
        block.ade_encode(&mut buf, &ebb_ctx()).unwrap();

        let mut offset = 0;
        let decoded = decode_ebb_block(&buf, &mut offset).unwrap();
        assert_eq!(decoded.header.protocol_magic, block.header.protocol_magic);
        assert_eq!(decoded.body, block.body);
        assert_eq!(decoded.extra, block.extra);

        // Re-encode should produce identical bytes
        let mut buf2 = Vec::new();
        decoded.ade_encode(&mut buf2, &ebb_ctx()).unwrap();
        assert_eq!(buf, buf2);
    }
}
