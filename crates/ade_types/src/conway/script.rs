// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Plutus V3 script bytecode. Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlutusV3Script {
    pub raw: Vec<u8>,
}
