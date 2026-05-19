// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Keep-alive state machine types — pure values, no I/O, no async.
//
// `KeepAliveState` encodes the three protocol states from the
// Ouroboros keep-alive mini-protocol per cardano-node 10.6.2. The
// `ServerHasAgency` variant carries the request cookie so the state
// machine can reject mismatched server responses without consulting
// any ambient session state. The cookie is a u16 — a single 2-byte
// value; carries no replay risk.
//
// `KeepAliveOutput` distinguishes per-message events (consumer-facing
// values consumed by the RED session layer for latency/health) from
// session termination. `KeepAliveError` is structured — every variant
// carries typed context, no `String`.

use crate::codec::keep_alive::KeepAliveCookie;
use crate::codec::version::KeepAliveVersion;
use crate::keep_alive::agency::KeepAliveAgency;
use crate::keep_alive::event::KeepAliveEvent;

/// Closed keep-alive protocol state per Ouroboros mini-protocol spec.
///
/// State graph:
///   ClientIdle              -- client KeepAlive(cookie)      --> ServerHasAgency{cookie}
///   ClientIdle              -- client Done                   --> Done
///   ServerHasAgency{cookie} -- server ResponseKeepAlive(c')  --> ClientIdle  (requires c' == cookie)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepAliveState {
    ClientIdle,
    ServerHasAgency { cookie: KeepAliveCookie },
    Done,
}

/// Output of a single keep-alive transition.
///
/// `Event` carries a `KeepAliveEvent` derived from the wire message;
/// the RED session layer consumes the event. The state machine does
/// not measure latency or mutate connection-health metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepAliveOutput {
    Event(KeepAliveEvent),
    Done,
}

/// Structured keep-alive errors. No `String`, no `anyhow`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepAliveError {
    /// A (state, message, agency) triple that the protocol grammar
    /// forbids — e.g. server sending `KeepAlive`, or
    /// `ResponseKeepAlive` arriving while the state machine is
    /// `ClientIdle`.
    IllegalTransition {
        state: KeepAliveState,
        message_tag: &'static str,
        agency: KeepAliveAgency,
    },
    /// Message variant valid in the grammar but rejected by the
    /// selected protocol version. Carries the version newtype and the
    /// tag of the offending message.
    InvalidForVersion {
        version: KeepAliveVersion,
        message_tag: &'static str,
    },
    /// Structurally-valid message that fails protocol-grammar
    /// invariants the codec does not check: the response cookie must
    /// equal the cookie carried in `ServerHasAgency`.
    MalformedMessage { reason: &'static str },
}
