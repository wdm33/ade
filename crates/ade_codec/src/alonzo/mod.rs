// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::error::CodecError;
use crate::preserved::PreservedCbor;
use ade_types::alonzo::AlonzoBlock;

/// Named decode chokepoint for Alonzo blocks (era tag 5).
///
/// Alonzo shares Shelley's block structure. The semantic differences
/// (Plutus scripts, datums, redeemers, execution units) are in the opaque
/// tx bodies and witnesses.
pub fn decode_alonzo_block(data: &[u8]) -> Result<PreservedCbor<AlonzoBlock>, CodecError> {
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
