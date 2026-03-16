// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::error::CodecError;
use crate::preserved::PreservedCbor;
use ade_types::conway::ConwayBlock;

/// Named decode chokepoint for Conway blocks (era tag 7).
///
/// Conway shares Shelley's block structure. The semantic differences
/// (governance actions, voting, DReps, Plutus V3) are in the opaque
/// tx bodies.
pub fn decode_conway_block(data: &[u8]) -> Result<PreservedCbor<ConwayBlock>, CodecError> {
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
