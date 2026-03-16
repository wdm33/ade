// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Transaction input reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxIn {
    pub raw: Vec<u8>,
}

/// Shelley transaction output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelleyTxOut {
    pub raw: Vec<u8>,
}

/// Nonce for randomness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nonce {
    pub raw: Vec<u8>,
}
