// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

pub mod encoding;
pub mod era_schedule;
pub mod errors;
pub mod events;
pub mod praos_state;

pub use encoding::{
    decode_chain_dep_state, decode_chain_event, encode_chain_dep_state, encode_chain_event,
    DecodeError,
};
pub use era_schedule::{
    BootstrapAnchorHash, EraLocation, EraSchedule, EraSummary,
};
pub use errors::{
    HFCError, HeaderValidationError, LeaderScheduleError, NonceEvolutionError,
    OpCertCounterError, OutsideForecastRange, SlotTimeError, VrfCertError,
};
pub use events::{
    BlockDistance, ChainEvent, ChainHash, ChainSelectionReject, Point, SecurityParam,
};
pub use praos_state::{Nonce, OpCertCounterMap, PraosChainDepState};
