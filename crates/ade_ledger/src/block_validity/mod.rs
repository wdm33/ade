// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Closed verdict/error taxonomies for block validity + canonical CBOR for the
// replay/comparison surface. Types-only: no transition logic (B1-S4).

pub mod encoding;
pub mod header_input;
pub mod header_position;
pub mod transition;
pub mod unsigned_header_pre_image;
pub mod verdict;

pub use encoding::{
    decode_verdict_surface, encode_verdict_surface, SurfaceDecodeError, VerdictSurface,
};
pub use header_input::{
    accepted_block_header_bytes, block_header_bytes, decode_block, DecodedBlock,
};
pub use header_position::check_header_position;
pub use transition::{block_validity, BlockValidityOutcome};
pub use verdict::{
    BlockRejectClass, BlockValidityError, BlockValidityVerdict, FieldError, FieldKind, MissingInput,
};
