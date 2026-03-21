// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::cbor;
use crate::error::CodecError;
use ade_types::babbage::tx::BabbageTxBody;

/// Decode a Babbage transaction body from CBOR.
///
/// Babbage extends Alonzo with inline datums, reference scripts, and
/// reference inputs. In Phase 1 the body is opaque — we capture the raw
/// CBOR bytes after validating structural well-formedness via skip_item.
pub fn decode_babbage_tx_body(
    data: &[u8],
    offset: &mut usize,
) -> Result<BabbageTxBody, CodecError> {
    let (start, end) = cbor::skip_item(data, offset)?;
    Ok(BabbageTxBody {
        raw: data[start..end].to_vec(),
    })
}
