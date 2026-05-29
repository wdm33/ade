// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE closed BootstrapAnchor error sum (PHASE4-N-M-A S2).

use ade_codec::CodecError;

/// Closed error sum for `BootstrapAnchor` encode/decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapAnchorError {
    /// CBOR primitive read/write error.
    Cbor(CodecError),
    /// Decoded schema version did not match `ANCHOR_SCHEMA_VERSION`.
    UnknownVersion { expected: u32, found: u32 },
    /// Decoded buffer did not match the expected closed CBOR
    /// shape (wrong array length, wrong hash byte width, etc.).
    Structural { reason: &'static str },
    /// Trailing bytes after the expected anchor structure.
    TrailingBytes { extra: usize },
}

impl From<CodecError> for BootstrapAnchorError {
    fn from(e: CodecError) -> Self {
        Self::Cbor(e)
    }
}
