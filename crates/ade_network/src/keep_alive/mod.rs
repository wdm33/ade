// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Keep-alive mini-protocol state machine (BLUE) — S-A7.
//
// Pure keep-alive transition that consumes/produces the codec types
// defined in S-A2 (`KeepAliveMessage`, `KeepAliveCookie`) and emits
// `KeepAliveEvent` values for the RED session layer to interpret
// (latency metrics, dead-peer detection). The state machine carries
// the in-flight cookie in `ServerHasAgency { cookie }` so it can
// reject mismatched responses; it never reads a wall-clock and never
// mutates connection-health metrics — those are RED concerns.
//
// Per-protocol agency type per locked §7 #7: `KeepAliveAgency` is
// non-interchangeable with any other per-protocol agency. The selected
// version is threaded as an explicit input (DC-PROTO-06).

pub mod agency;
pub mod event;
pub mod state;
pub mod transition;

pub use agency::KeepAliveAgency;
pub use event::{KeepAliveCookie, KeepAliveEvent};
pub use state::{KeepAliveError, KeepAliveOutput, KeepAliveState};
pub use transition::keep_alive_transition;
