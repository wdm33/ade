// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

pub mod block;
pub mod tx;

use crate::cbor;
use crate::error::CodecError;
use crate::preserved::PreservedCbor;
use ade_types::byron::block::{ByronEbbBlock, ByronRegularBlock};

/// Named decode chokepoint for Byron EBB blocks (era tag 0).
///
/// Input: raw CBOR bytes of the inner block (after HFC envelope stripping).
/// Returns: `PreservedCbor<ByronEbbBlock>` with wire bytes preserved.
pub fn decode_byron_ebb_block(data: &[u8]) -> Result<PreservedCbor<ByronEbbBlock>, CodecError> {
    let mut offset = 0;
    let decoded = block::decode_ebb_block(data, &mut offset)?;
    if offset != data.len() {
        return Err(CodecError::TrailingBytes {
            consumed: offset,
            total: data.len(),
        });
    }
    Ok(PreservedCbor::new(data.to_vec(), decoded))
}

/// Named decode chokepoint for Byron regular blocks (era tag 1).
///
/// Input: raw CBOR bytes of the inner block (after HFC envelope stripping).
/// Returns: `PreservedCbor<ByronRegularBlock>` with wire bytes preserved.
pub fn decode_byron_regular_block(
    data: &[u8],
) -> Result<PreservedCbor<ByronRegularBlock>, CodecError> {
    let mut offset = 0;
    let decoded = block::decode_regular_block(data, &mut offset)?;
    if offset != data.len() {
        return Err(CodecError::TrailingBytes {
            consumed: offset,
            total: data.len(),
        });
    }
    Ok(PreservedCbor::new(data.to_vec(), decoded))
}

/// Decode a Byron block from its inner bytes (after envelope stripping),
/// dispatching on era tag.
pub fn decode_byron_block(
    era: ade_types::CardanoEra,
    data: &[u8],
) -> Result<ByronDecodedBlock, CodecError> {
    match era {
        ade_types::CardanoEra::ByronEbb => {
            let preserved = decode_byron_ebb_block(data)?;
            Ok(ByronDecodedBlock::Ebb(preserved))
        }
        ade_types::CardanoEra::ByronRegular => {
            let preserved = decode_byron_regular_block(data)?;
            Ok(ByronDecodedBlock::Regular(preserved))
        }
        _ => Err(CodecError::UnknownEraTag { tag: era.as_u8() }),
    }
}

/// Decoded Byron block — either EBB or regular.
#[derive(Debug, Clone)]
pub enum ByronDecodedBlock {
    Ebb(PreservedCbor<ByronEbbBlock>),
    Regular(PreservedCbor<ByronRegularBlock>),
}

impl ByronDecodedBlock {
    /// Wire bytes of the inner block.
    pub fn wire_bytes(&self) -> &[u8] {
        match self {
            ByronDecodedBlock::Ebb(p) => p.wire_bytes(),
            ByronDecodedBlock::Regular(p) => p.wire_bytes(),
        }
    }
}

/// Read a 32-byte hash from CBOR byte string.
pub(crate) fn read_hash32(
    data: &[u8],
    offset: &mut usize,
) -> Result<ade_types::Hash32, CodecError> {
    let (bytes, _width) = cbor::read_bytes(data, offset)?;
    if bytes.len() != 32 {
        return Err(CodecError::InvalidLength {
            offset: *offset - bytes.len(),
            detail: "expected 32-byte hash",
        });
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(ade_types::Hash32(arr))
}
