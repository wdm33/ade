// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN block-validity test harness (PHASE4-B1).
//!
//! Non-authoritative: loads the committed positive-validation corpus for the
//! B1 block-validity replay tests.

pub mod corpus;

pub use corpus::{ConwayValidityCorpus, CorpusLoadError, CorpusPool, CorpusRatio};
