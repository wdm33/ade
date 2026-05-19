// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::SlotNo;

/// Hard Fork Combinator schedule construction and query errors.
///
/// All variants are flat and value-typed; replay corpora compare reject
/// reasons byte-for-byte without traversing strings or trait objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HFCError {
    EmptyEraList,
    NonMonotonicEras {
        prev_start: SlotNo,
        next_start: SlotNo,
    },
    ZeroSlotLength {
        era_index: u8,
    },
    ZeroEpochLength {
        era_index: u8,
    },
    SlotBeforeSystemStart {
        slot: SlotNo,
        first_era_start: SlotNo,
    },
    SlotAfterLastEra {
        slot: SlotNo,
        last_era_end: SlotNo,
    },
}

/// Slot-to-time translation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotTimeError {
    OutOfRange { slot: SlotNo },
    HFC(HFCError),
    Overflow,
}

/// Returned when a consensus query asks for a slot strictly past the
/// stable forecast horizon (`last_era.start_slot + last_era.safe_zone_slots`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutsideForecastRange {
    pub requested: SlotNo,
    pub horizon: SlotNo,
}
