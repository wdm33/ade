// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Shelley transaction body. Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelleyTxBody {
    pub raw: Vec<u8>,
}

/// Shelley transaction (body + witnesses). Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelleyTx {
    pub raw: Vec<u8>,
}

/// Shelley witness set. Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelleyWitnesses {
    pub raw: Vec<u8>,
}
