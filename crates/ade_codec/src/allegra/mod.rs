// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

pub mod script;
pub mod tx;

use crate::error::CodecError;
use crate::preserved::PreservedCbor;
use ade_types::allegra::AllegraBlock;

/// Named decode chokepoint for Allegra blocks (era tag 3).
///
/// Allegra shares Shelley's block structure. The semantic differences
/// (ValidityInterval, TimelockScript) are in the opaque tx bodies.
pub fn decode_allegra_block(data: &[u8]) -> Result<PreservedCbor<AllegraBlock>, CodecError> {
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
