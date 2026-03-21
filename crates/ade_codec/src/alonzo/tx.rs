// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::cbor;
use crate::error::CodecError;
use ade_types::alonzo::tx::AlonzoTxBody;

/// Decode an Alonzo transaction body from CBOR.
///
/// Alonzo extends Mary with keys 11–15 (script data hash, collateral
/// inputs, required signers, network ID, collateral return). In Phase 1
/// the body is opaque — we capture the raw CBOR bytes after validating
/// structural well-formedness via skip_item.
pub fn decode_alonzo_tx_body(
    data: &[u8],
    offset: &mut usize,
) -> Result<AlonzoTxBody, CodecError> {
    let (start, end) = cbor::skip_item(data, offset)?;
    Ok(AlonzoTxBody {
        raw: data[start..end].to_vec(),
    })
}
