// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Byron transaction. Opaque in Phase 1 — type stub for Phase 2+.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronTx {
    pub raw: Vec<u8>,
}

/// Byron transaction input. Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronTxIn {
    pub raw: Vec<u8>,
}

/// Byron transaction output. Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronTxOut {
    pub raw: Vec<u8>,
}

/// Byron transaction witness. Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronTxWitness {
    pub raw: Vec<u8>,
}
