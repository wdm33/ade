// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Peer-sharing event taxonomy emitted by the peer-sharing transition.
//
// Per slice S-A7 §9: N-A produces values, the RED session layer
// interprets effects. The peer-sharing state machine does not mutate
// any peer book — it emits a `PeerSharingEvent` value per legal
// message, and a future RED session-level cluster feeds the
// `PeersShared { peers }` payload into the peer-book.
//
// `PeerAddress` is re-exported from the S-A2 codec module so every
// consumer references the same canonical type.

pub use crate::codec::peer_sharing::PeerAddress;

/// Peer-sharing event taxonomy. Closed enum; consumers exhaustively
/// match.
///
/// The state machine certifies only the reply-size invariant
/// (`peers.len() <= amount`); peer-book population is a future RED
/// cluster's job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerSharingEvent {
    SharingRequested { amount: u8 },
    PeersShared { peers: Vec<PeerAddress> },
}
