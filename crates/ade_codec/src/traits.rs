// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::error::CodecError;
use ade_types::CardanoEra;

/// Era-aware encoding/decoding context.
///
/// Carried through encode/decode operations to enable era-specific
/// behavior where the same CBOR structure is interpreted differently
/// across eras.
#[derive(Debug, Clone, Copy)]
pub struct CodecContext {
    pub era: CardanoEra,
}

/// Project-canonical encoding.
///
/// Implementations produce deterministic canonical bytes for a type.
/// Canonical encoding is used for internal replay and evidence surfaces,
/// NOT for hash-critical computation (which uses `.wire_bytes()`).
pub trait AdeEncode {
    fn ade_encode(&self, buf: &mut Vec<u8>, ctx: &CodecContext) -> Result<(), CodecError>;
}

/// Decoding from CBOR bytes.
///
/// Implementations consume bytes starting at `offset`, advancing it past
/// the decoded item. Multiple wire encodings of the same semantic value
/// are tolerated (e.g., non-minimal integer widths).
pub trait AdeDecode: Sized {
    fn ade_decode(data: &[u8], offset: &mut usize, ctx: &CodecContext) -> Result<Self, CodecError>;
}
