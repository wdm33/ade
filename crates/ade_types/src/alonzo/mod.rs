// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

pub mod output;
pub mod plutus;
pub mod tx;
pub mod witness;

/// Alonzo block reuses Shelley's block structure (same array(4), header array(15)).
/// The semantic differences (Plutus scripts, datums, redeemers, execution units)
/// are in the tx bodies and witnesses, which are opaque in Phase 1.
pub type AlonzoBlock = crate::shelley::block::ShelleyBlock;
