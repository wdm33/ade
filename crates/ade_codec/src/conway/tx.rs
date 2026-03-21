// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::cbor;
use crate::error::CodecError;
use ade_types::conway::tx::ConwayTxBody;

/// Decode a Conway transaction body from CBOR.
///
/// Conway extends Babbage with governance actions, voting, DReps, and
/// Plutus V3. In Phase 1 the body is opaque — we capture the raw CBOR
/// bytes after validating structural well-formedness via skip_item.
pub fn decode_conway_tx_body(
    data: &[u8],
    offset: &mut usize,
) -> Result<ConwayTxBody, CodecError> {
    let (start, end) = cbor::skip_item(data, offset)?;
    Ok(ConwayTxBody {
        raw: data[start..end].to_vec(),
    })
}
