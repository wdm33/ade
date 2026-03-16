// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

pub mod cert;
pub mod governance;
pub mod script;
pub mod tx;

/// Conway block reuses Shelley's block structure (same array(4), header array(15)).
/// The semantic differences (governance actions, voting, DReps, Plutus V3)
/// are in the tx bodies, which are opaque in Phase 1.
pub type ConwayBlock = crate::shelley::block::ShelleyBlock;
