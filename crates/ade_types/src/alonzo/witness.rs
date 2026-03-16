// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Alonzo witness set (vkey sigs, scripts, datums, redeemers). Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlonzoWitnesses {
    pub raw: Vec<u8>,
}
