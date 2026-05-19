// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

pub mod era_schedule;
pub mod errors;

pub use era_schedule::{
    BootstrapAnchorHash, EraLocation, EraSchedule, EraSummary,
};
pub use errors::{HFCError, OutsideForecastRange, SlotTimeError};
