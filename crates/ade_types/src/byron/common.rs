// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Lovelace amount (smallest unit of ADA).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Lovelace(pub u64);

/// Byron-era address. Opaque bytes in Phase 1.
///
/// Byron addresses use Base58check encoding on the human-readable side
/// and double CBOR wrapping on the wire side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronAddress {
    pub raw: Vec<u8>,
}
