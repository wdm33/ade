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

/// Bounded, hash-free result of [`super::ChainDb::range_bytes_capped`]
/// (PHASE4-N-AA, DC-SERVEMEM-01).
///
/// `blocks` holds at most `max` `(slot, bytes)` pairs in slot-ascending order;
/// `truncated` is `true` when the requested range contained MORE than `max`
/// blocks — i.e. the per-request serve cap was exceeded. The serve uses
/// `truncated` to fail closed and to distinguish "cap exceeded" from "genuinely
/// empty" (both encode to the same wire `NoBlocks`, but the internal reason
/// differs for diagnostics + tests). Hash-free: the serve derives each block's
/// hash from its own bytes via the BLUE decode authority, so no `SLOT_BY_HASH`
/// scan is performed here.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CappedSlotRange {
    pub blocks: Vec<(SlotNo, Vec<u8>)>,
    pub truncated: bool,
}
