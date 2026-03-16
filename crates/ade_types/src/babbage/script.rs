// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Plutus V2 script bytecode. Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlutusV2Script {
    pub raw: Vec<u8>,
}

/// Script reference (inline script in a UTxO). Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptRef {
    pub raw: Vec<u8>,
}
