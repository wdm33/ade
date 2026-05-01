// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::primitives::{Hash32, SlotNo};

/// A block as stored in the chain database.
///
/// `bytes` is the wire-byte authoritative representation — what the
/// node received, what its hash was computed over, what gets re-served
/// on chain-sync. Hash and slot are derived metadata indexed for
/// lookup; storing them alongside avoids re-decoding on every access.
///
/// Equality is structural across all three fields. A db that returns
/// a `StoredBlock` with the same `hash` and `slot` but different
/// `bytes` than what was put has corrupted the wire-byte contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredBlock {
    pub hash: Hash32,
    pub slot: SlotNo,
    pub bytes: Vec<u8>,
}

/// Identification of the highest stored block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainTip {
    pub hash: Hash32,
    pub slot: SlotNo,
}
