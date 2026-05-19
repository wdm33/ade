// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Tx-submission2 mini-protocol state machine (BLUE) — S-A6.
//
// Pure tx-submission2 transition that consumes/produces the codec types
// defined in S-A2 (`TxSubmission2Message`, `TxIdAndSize`) and emits
// `InventoryEvent` values for the mempool (future cluster) to interpret.
// The state machine does not decode `tx_bytes` (DC-PROTO-06: opaque
// pass-through), does not maintain a tx ID inventory (only the request
// count is carried in `TxsRequested`), and does not touch mempool state
// — events are the entire contract with the future mempool.
//
// Per-protocol agency type per locked §7 #7: `TxSubmission2Agency` is
// non-interchangeable with `ChainSyncAgency`, `BlockFetchAgency`,
// `HandshakeAgency`, or any other per-protocol agency. The selected
// version is threaded as an explicit input (DC-PROTO-06).
//
// Inverted client-server semantics note: in tx-submission2 the Client
// (initiator) RESPONDS to Server (responder) requests — opposite of
// chain-sync. The agency labels in this state machine match the
// Ouroboros spec, not intuition about "who's in charge."

pub mod agency;
pub mod event;
pub mod state;
pub mod transition;

pub use agency::TxSubmission2Agency;
pub use event::{InventoryEvent, TxIdAndSize};
pub use state::{TxSubmission2Error, TxSubmission2Output, TxSubmission2State};
pub use transition::tx_submission2_transition;
