// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// LocalStateQuery state machine types — pure values, no I/O, no async.
//
// `LocalStateQueryState` encodes the five protocol states from the
// Ouroboros local-state-query mini-protocol per cardano-node 11.0.1 (10.6.2 forward-compatible).
// `LocalStateQueryOutput` carries an event per client/server message
// (Reply is not separately modeled — every transition either yields an
// event or terminates the session). `LocalStateQueryError` is
// structured — every variant carries typed context, no `String`.

use crate::codec::version::LocalStateQueryVersion;
use crate::n2c::local_state_query::agency::LocalStateQueryAgency;
use crate::n2c::local_state_query::event::LocalStateQueryEvent;

/// Closed LocalStateQuery protocol state per Ouroboros mini-protocol
/// spec.
///
/// State graph:
///   Idle      -- client Acquire{point}    --> Acquiring
///   Idle      -- client Done              --> Done
///   Acquiring -- server Acquired          --> Acquired
///   Acquiring -- server Failure(reason)   --> Idle
///   Acquired  -- client Query(payload)    --> Querying
///   Querying  -- server Result(payload)   --> Acquired
///   Acquired  -- client Release           --> Idle
///   Acquired  -- client ReAcquire{point}  --> Acquiring
///   Acquired  -- client Done              --> Done
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalStateQueryState {
    Idle,
    Acquiring,
    Acquired,
    Querying,
    Done,
}

/// Output of a single LocalStateQuery transition.
///
/// `Event` carries a `LocalStateQueryEvent` derived from the incoming
/// message. `Done` is emitted only on the client Done terminal
/// transition. The state machine does not decode query/result payloads
/// or mutate ledger state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalStateQueryOutput {
    Event(LocalStateQueryEvent),
    Done,
}

/// Structured LocalStateQuery errors. No `String`, no `anyhow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalStateQueryError {
    /// A (state, message, agency) triple that the protocol grammar
    /// forbids.
    IllegalTransition {
        state: LocalStateQueryState,
        message_tag: &'static str,
        agency: LocalStateQueryAgency,
    },
    /// Message variant valid in the grammar but rejected by the
    /// selected protocol version. Carries the version newtype and the
    /// tag of the offending message.
    InvalidForVersion {
        version: LocalStateQueryVersion,
        message_tag: &'static str,
    },
    /// Structurally-valid message that fails protocol-grammar
    /// invariants the codec does not check.
    MalformedMessage { reason: &'static str },
}
