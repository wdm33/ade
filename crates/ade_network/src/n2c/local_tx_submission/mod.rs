// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2C LocalTxSubmission mini-protocol state machine (BLUE) — S-A8.
//
// Pure local-tx-submission transition that consumes the codec types
// defined in S-A2 (`LocalTxSubmissionMessage`, `TxRejection`) and emits
// `LocalTxSubmissionEvent` values. The state machine does not decode
// the submitted `tx_bytes` (DC-PROTO-06: opaque pass-through) and does
// not interpret the rejection reason (ledger-defined CBOR), and does
// not touch the mempool.
//
// Per-protocol agency type per locked §7 #7:
// `LocalTxSubmissionAgency` is non-interchangeable with any other
// per-protocol agency. The selected version is threaded as an explicit
// input (DC-PROTO-06).

pub mod agency;
pub mod event;
pub mod state;
pub mod transition;

pub use agency::LocalTxSubmissionAgency;
pub use event::{LocalTxSubmissionEvent, TxRejection};
pub use state::{LocalTxSubmissionError, LocalTxSubmissionOutput, LocalTxSubmissionState};
pub use transition::local_tx_submission_transition;
