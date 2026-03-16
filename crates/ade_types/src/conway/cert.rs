// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Conway certificate (stake registration, delegation, DRep registration, etc.).
/// Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConwayCert {
    pub raw: Vec<u8>,
}

/// Delegated representative. Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DRep {
    pub raw: Vec<u8>,
}
