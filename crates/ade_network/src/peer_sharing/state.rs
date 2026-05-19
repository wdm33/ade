// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Peer-sharing state machine types — pure values, no I/O, no async.
//
// `PeerSharingState` encodes the three protocol states from the
// Ouroboros peer-sharing mini-protocol per cardano-node 11.0.1 (10.6.2 forward-compatible). The
// `Busy { amount }` variant carries the requested amount so the state
// machine can reject overlarge replies without consulting any ambient
// session state. The amount is a u8 — bounded by the on-wire grammar
// (`amount: u8` in `ShareRequest`).
//
// `PeerSharingOutput` distinguishes per-message events (consumer-facing
// values consumed by the RED session layer for peer-book population)
// from session termination. `PeerSharingError` is structured — every
// variant carries typed context, no `String`.

use crate::codec::version::PeerSharingVersion;
use crate::peer_sharing::agency::PeerSharingAgency;
use crate::peer_sharing::event::PeerSharingEvent;

/// Closed peer-sharing protocol state per Ouroboros mini-protocol spec.
///
/// State graph:
///   Idle           -- client ShareRequest{amount}    --> Busy{amount}
///   Idle           -- client Done                    --> Done
///   Busy{amount}   -- server SharePeers(peers)       --> Idle  (requires peers.len() <= amount)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerSharingState {
    Idle,
    Busy { amount: u8 },
    Done,
}

/// Output of a single peer-sharing transition.
///
/// `Event` carries a `PeerSharingEvent` derived from the wire message;
/// the RED session layer consumes the event. The state machine does
/// not mutate the peer book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerSharingOutput {
    Event(PeerSharingEvent),
    Done,
}

/// Structured peer-sharing errors. No `String`, no `anyhow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerSharingError {
    /// A (state, message, agency) triple that the protocol grammar
    /// forbids — e.g. server sending `ShareRequest`, or `SharePeers`
    /// arriving while the state machine is `Idle`.
    IllegalTransition {
        state: PeerSharingState,
        message_tag: &'static str,
        agency: PeerSharingAgency,
    },
    /// Message variant valid in the grammar but rejected by the
    /// selected protocol version. Carries the version newtype and the
    /// tag of the offending message.
    InvalidForVersion {
        version: PeerSharingVersion,
        message_tag: &'static str,
    },
    /// Structurally-valid message that fails protocol-grammar
    /// invariants the codec does not check: the `SharePeers` reply
    /// count must be `<= amount` from the matching `ShareRequest`.
    MalformedMessage { reason: &'static str },
}
