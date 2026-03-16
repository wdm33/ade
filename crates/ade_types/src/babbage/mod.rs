// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

pub mod output;
pub mod script;
pub mod tx;

/// Babbage block reuses Shelley's block structure (same array(4), header array(15)).
/// The semantic differences (inline datums, reference scripts, reference inputs)
/// are in the tx bodies and outputs, which are opaque in Phase 1.
pub type BabbageBlock = crate::shelley::block::ShelleyBlock;
