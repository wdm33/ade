// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Alonzo transaction body. Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlonzoTxBody {
    pub raw: Vec<u8>,
}

/// Alonzo transaction (body + witnesses + validity + auxiliary). Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlonzoTx {
    pub raw: Vec<u8>,
}
