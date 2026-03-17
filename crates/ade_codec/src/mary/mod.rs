// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

pub mod tx;

use crate::error::CodecError;
use crate::preserved::PreservedCbor;
use ade_types::mary::MaryBlock;

/// Named decode chokepoint for Mary blocks (era tag 4).
///
/// Mary shares Shelley's block structure. The semantic differences
/// (MultiAsset values) are in the opaque tx bodies.
pub fn decode_mary_block(data: &[u8]) -> Result<PreservedCbor<MaryBlock>, CodecError> {
    let mut offset = 0;
    let decoded = crate::shelley::block::decode_shelley_block_inner(data, &mut offset)?;
    if offset != data.len() {
        return Err(CodecError::TrailingBytes {
            consumed: offset,
            total: data.len(),
        });
    }
    Ok(PreservedCbor::new(data.to_vec(), decoded))
}
