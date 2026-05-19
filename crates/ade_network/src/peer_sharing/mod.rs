// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Peer-sharing mini-protocol state machine (BLUE) — S-A7.
//
// Pure peer-sharing transition that consumes/produces the codec types
// defined in S-A2 (`PeerSharingMessage`, `PeerAddress`) and emits
// `PeerSharingEvent` values for the RED session layer to consume.
// The state machine carries the outstanding `amount` in
// `Busy { amount }` so it can reject overlarge replies; it does not
// mutate any peer book (population is a RED concern in a future
// cluster per the cluster TCB map).
//
// Per-protocol agency type per locked §7 #7: `PeerSharingAgency` is
// non-interchangeable with any other per-protocol agency. The selected
// version is threaded as an explicit input (DC-PROTO-06).

pub mod agency;
pub mod event;
pub mod state;
pub mod transition;

pub use agency::PeerSharingAgency;
pub use event::{PeerAddress, PeerSharingEvent};
pub use state::{PeerSharingError, PeerSharingOutput, PeerSharingState};
pub use transition::peer_sharing_transition;
