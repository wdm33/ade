// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Mary transaction body. Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaryTxBody {
    pub raw: Vec<u8>,
}
