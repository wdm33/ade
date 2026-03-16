// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::error::CodecError;
use crate::preserved::PreservedCbor;
use ade_types::babbage::BabbageBlock;

/// Named decode chokepoint for Babbage blocks (era tag 6).
///
/// Babbage shares Shelley's block structure. The semantic differences
/// (inline datums, reference scripts, reference inputs) are in the opaque
/// tx bodies and outputs.
pub fn decode_babbage_block(data: &[u8]) -> Result<PreservedCbor<BabbageBlock>, CodecError> {
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
