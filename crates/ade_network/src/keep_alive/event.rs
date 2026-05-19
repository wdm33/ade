// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Keep-alive event taxonomy emitted by the keep-alive transition.
//
// Per slice S-A7 §9: N-A produces values, the RED session layer
// interprets effects. The keep-alive state machine does not measure
// latency, does not flag dead peers, and does not touch any health
// metric — it emits a `KeepAliveEvent` value per legal message, and
// the session layer attaches send/receive timestamps and derives
// connection health from the resulting event stream.
//
// `KeepAliveCookie` is re-exported from the S-A2 codec module so
// every consumer references the same canonical type.

pub use crate::codec::keep_alive::KeepAliveCookie;

/// Keep-alive event taxonomy. Closed enum; consumers exhaustively match.
///
/// The cookie is a 16-bit nonce, not a timestamp. The session layer
/// (RED) is responsible for any latency accounting; the state machine
/// only certifies that the response cookie matches the outstanding
/// request cookie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepAliveEvent {
    PingSent { cookie: KeepAliveCookie },
    PongReceived { cookie: KeepAliveCookie },
}
