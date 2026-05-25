// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN test harness for the PHASE4-N-C producer surface.
//!
//! Exposes `reference_vectors` (S1), `fixtures` + `replay` (S3, S4),
//! and `cross_impl_adapter` (S7 — mechanical half of CN-CONS-06).

pub mod cross_impl_adapter;
pub mod fixtures;
pub mod reference_vectors;
pub mod replay;
