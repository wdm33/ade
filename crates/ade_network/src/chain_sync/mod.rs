// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Chain-sync mini-protocol state machine (BLUE) — S-A4.
//
// Pure chain-sync transition that consumes/produces the codec types
// defined in S-A2 (`ChainSyncMessage`, `Point`, `Tip`) and emits
// `ForkChoiceSignal` values for the consensus runtime (N-B) to
// interpret. The state machine does not decode header bytes
// (DC-PROTO-06: opaque pass-through), does not touch ChainDb (N-B
// owns mutation), and does not perform fork-choice (signals are
// values, not effects).
//
// Per-protocol agency type per locked §7 #7: `ChainSyncAgency` is
// non-interchangeable with `HandshakeAgency` or any other per-protocol
// agency. The selected version is threaded as an explicit input
// (DC-PROTO-06).

pub mod agency;
pub mod server;
pub mod signal;
pub mod state;
pub mod transition;

pub use agency::ChainSyncAgency;
pub use signal::{ForkChoiceSignal, Point, Tip};
pub use state::{ChainSyncError, ChainSyncOutput, ChainSyncState};
pub use transition::chain_sync_transition;
