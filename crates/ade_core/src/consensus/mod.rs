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
pub mod header_summary;
pub mod header_validate;
pub mod leader_schedule;
pub mod ledger_view;
pub mod nonce;
pub mod op_cert;
pub mod praos_state;
pub mod vrf_cert;

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
pub use header_summary::{HeaderInput, ValidatedHeaderSummary};
pub use header_validate::{validate_and_apply_header, HeaderApplied};
pub use leader_schedule::{
    is_leader_for_vrf_output, query_leader_schedule, LeaderScheduleAnswer, LeaderScheduleQuery,
};
pub use ledger_view::LedgerView;
pub use nonce::{apply_nonce_input, NonceInput};
pub use op_cert::{apply_op_cert, OpCertObservation};
pub use praos_state::{Nonce, OpCertCounterMap, PraosChainDepState};
pub use vrf_cert::{
    check_leader_claim, is_leader, leader_value_bytes, verify_vrf_cert, ActiveSlotsCoeff,
    StakeFraction, VerifiedVrf, VrfRole, VRF_INPUT_LEN,
};
