// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Block-fetch mini-protocol state machine (BLUE) — S-A5.
//
// Pure block-fetch transition that consumes/produces the codec types
// defined in S-A2 (`BlockFetchMessage`, `Point`, `Range`) and emits
// `BatchDeliveryEvent` values for the consensus runtime (N-B) to
// interpret. The state machine does not decode block bytes
// (DC-PROTO-06: opaque pass-through), does not touch ChainDb (N-B
// owns mutation), and does not accumulate blocks (events are emitted
// per Block message as they arrive).
//
// Per-protocol agency type per locked §7 #7: `BlockFetchAgency` is
// non-interchangeable with `ChainSyncAgency`, `HandshakeAgency`, or any
// other per-protocol agency. The selected version is threaded as an
// explicit input (DC-PROTO-06).

pub mod agency;
pub mod event;
pub mod state;
pub mod transition;

pub use agency::BlockFetchAgency;
pub use event::{BatchDeliveryEvent, Point, Range};
pub use state::{BlockFetchError, BlockFetchOutput, BlockFetchState};
pub use transition::block_fetch_transition;
