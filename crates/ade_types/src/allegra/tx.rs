// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Allegra transaction body with ValidityInterval. Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllegraTxBody {
    pub raw: Vec<u8>,
}

/// Timelock script (6 variants). Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelockScript {
    pub raw: Vec<u8>,
}
