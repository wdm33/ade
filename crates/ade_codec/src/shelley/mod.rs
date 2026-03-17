// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

pub mod block;
pub mod tx;

use crate::error::CodecError;
use crate::preserved::PreservedCbor;
use ade_types::shelley::block::ShelleyBlock;

/// Named decode chokepoint for Shelley blocks (era tag 2).
pub fn decode_shelley_block(data: &[u8]) -> Result<PreservedCbor<ShelleyBlock>, CodecError> {
    let mut offset = 0;
    let decoded = block::decode_shelley_block_inner(data, &mut offset)?;
    if offset != data.len() {
        return Err(CodecError::TrailingBytes {
            consumed: offset,
            total: data.len(),
        });
    }
    Ok(PreservedCbor::new(data.to_vec(), decoded))
}
