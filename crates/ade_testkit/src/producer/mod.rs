// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN test harness for the PHASE4-N-C producer surface.
//!
//! Currently exposes only `reference_vectors` (S1). Replay harness and
//! cross-impl adapter follow in later slices (S3, S4, S7).

pub mod fixtures;
pub mod reference_vectors;
pub mod replay;
