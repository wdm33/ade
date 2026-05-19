// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2C LocalStateQuery mini-protocol state machine (BLUE) — S-A8.
//
// Pure local-state-query transition that consumes the codec types
// defined in S-A2 (`LocalStateQueryMessage`, `Point`, `AcquireFailure`,
// `QueryPayload`, `ResultPayload`) and emits `LocalStateQueryEvent`
// values. The state machine owns the closed wire grammar of LSQ but
// NOT the ledger-semantic interpretation of query/result payloads —
// those are opaque `Vec<u8>` at this layer (DC-PROTO-06).
//
// Per-protocol agency type per locked §7 #7: `LocalStateQueryAgency`
// is non-interchangeable with any other per-protocol agency. The
// selected version is threaded as an explicit input (DC-PROTO-06).

pub mod agency;
pub mod event;
pub mod state;
pub mod transition;

pub use agency::LocalStateQueryAgency;
pub use event::{AcquireFailure, LocalStateQueryEvent, Point, QueryPayload, ResultPayload};
pub use state::{LocalStateQueryError, LocalStateQueryOutput, LocalStateQueryState};
pub use transition::local_state_query_transition;
