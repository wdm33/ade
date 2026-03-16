// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Babbage transaction output (address + value + optional datum + optional script ref).
/// Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BabbageTxOut {
    pub raw: Vec<u8>,
}

/// Datum option: either inline datum or datum hash. Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatumOption {
    pub raw: Vec<u8>,
}
