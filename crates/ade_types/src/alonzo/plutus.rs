// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Plutus datum (arbitrary CBOR data attached to UTxOs). Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Datum {
    pub raw: Vec<u8>,
}

/// Redeemer (script input + execution budget). Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redeemer {
    pub raw: Vec<u8>,
}

/// Execution units (memory + CPU steps). Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExUnits {
    pub raw: Vec<u8>,
}

/// Plutus V1 script bytecode. Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlutusV1Script {
    pub raw: Vec<u8>,
}

/// Cost model parameters for Plutus evaluation. Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostModel {
    pub raw: Vec<u8>,
}
