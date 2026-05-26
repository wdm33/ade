// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE BootstrapAnchor — closed provenance record for the
//! oracle-seed bootstrap path (PHASE4-N-M-A S2).

pub mod anchor;
pub mod error;

pub use anchor::{
    decode_bootstrap_anchor, encode_bootstrap_anchor, BootstrapAnchor, SeedPoint, SCHEMA_VERSION,
};
pub use error::BootstrapAnchorError;
