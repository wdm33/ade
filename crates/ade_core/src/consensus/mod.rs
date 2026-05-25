// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

pub mod candidate;
pub mod encoding;
pub mod era_schedule;
pub mod errors;
pub mod events;
pub mod fork_choice;
pub mod header_summary;
pub mod header_validate;
pub mod kes_check;
pub mod leader_schedule;
pub mod ledger_view;
pub mod nonce;
pub mod op_cert;
pub mod opcert_validate;
pub mod praos_state;
pub mod rollback;
pub mod vrf_cert;

pub use candidate::{
    tiebreaker_prefer, CandidateFragment, ChainSelectorState, TiebreakerView,
};
pub use encoding::{
    decode_chain_dep_state, decode_chain_event, encode_chain_dep_state, encode_chain_event,
    DecodeError,
};
pub use fork_choice::{select_best_chain, ForkChoiceError};
pub use era_schedule::{
    BootstrapAnchorHash, EraLocation, EraSchedule, EraSummary,
};
pub use errors::{
    FieldError, FieldKind, HFCError, HeaderValidationError, LeaderScheduleError,
    NonceEvolutionError, OpCertCounterError, OutsideForecastRange, SlotTimeError, VrfCertError,
};
pub use events::{
    BlockDistance, ChainEvent, ChainHash, ChainSelectionReject, Point, SecurityParam,
};
pub use header_summary::{HeaderInput, HeaderKes, HeaderVrf, ValidatedHeaderSummary};
pub use header_validate::{validate_and_apply_header, HeaderApplied};
pub use kes_check::{expect_size, verify_header_kes, SLOTS_PER_KES_PERIOD};
pub use leader_schedule::{
    is_leader_for_vrf_output, query_leader_schedule, LeaderScheduleAnswer, LeaderScheduleQuery,
};
pub use ledger_view::LedgerView;
pub use nonce::{apply_nonce_input, NonceInput};
pub use op_cert::{apply_op_cert, OpCertObservation};
pub use opcert_validate::{opcert_validate, OpCertError};
pub use praos_state::{Nonce, OpCertCounterMap, PraosChainDepState};
pub use rollback::{apply_rollback, RollBackApplied, RollBackRequest};
pub use vrf_cert::{
    check_leader_claim, is_leader, leader_value_bytes, praos_leader_value, praos_nonce_value,
    praos_vrf_input, verify_praos_vrf, verify_vrf_cert, ActiveSlotsCoeff, StakeFraction,
    VerifiedVrf, VrfRole, VRF_INPUT_LEN,
};
