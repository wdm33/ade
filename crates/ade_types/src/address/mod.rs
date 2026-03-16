// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Cardano address — 6 variants distinguished by the header byte.
///
/// The header byte encodes both the address type (bits 4-7) and
/// network ID (bits 0-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Address {
    /// Type 0-3: Base address (payment + staking credential).
    Base(Vec<u8>),
    /// Type 4-5: Pointer address (payment + chain pointer).
    Pointer(Vec<u8>),
    /// Type 6-7: Enterprise address (payment only, no staking).
    Enterprise(Vec<u8>),
    /// Type 8: Byron/Bootstrap address (legacy, Base58check encoded).
    Byron(Vec<u8>),
    /// Type 14-15: Reward address (staking credential only).
    Reward(Vec<u8>),
}

impl Address {
    /// The raw bytes of this address.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Address::Base(b)
            | Address::Pointer(b)
            | Address::Enterprise(b)
            | Address::Byron(b)
            | Address::Reward(b) => b,
        }
    }
}

/// Network identifier from the address header byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkId(pub u8);

/// Credential: either a key hash or script hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Credential {
    KeyHash(Vec<u8>),
    ScriptHash(Vec<u8>),
}
