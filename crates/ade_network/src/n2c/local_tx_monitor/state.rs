// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// LocalTxMonitor state machine types — pure values, no I/O, no async.
//
// `LocalTxMonitorState` encodes the five protocol states from the
// Ouroboros local-tx-monitor mini-protocol per cardano-node 10.6.2.
// `LocalTxMonitorOutput` carries an event per client/server message
// (Reply is not separately modeled — every transition either yields an
// event or terminates the session). `LocalTxMonitorError` is
// structured — every variant carries typed context, no `String`.

use crate::codec::version::LocalTxMonitorVersion;
use crate::n2c::local_tx_monitor::agency::LocalTxMonitorAgency;
use crate::n2c::local_tx_monitor::event::LocalTxMonitorEvent;

/// Closed LocalTxMonitor protocol state per Ouroboros mini-protocol
/// spec.
///
/// State graph:
///   Idle      -- client Acquire             --> Acquiring
///   Idle      -- client Done                --> Done
///   Acquiring -- server AwaitAcquire        --> Acquiring
///   Acquiring -- server Acquired{slot}      --> Acquired
///   Acquired  -- client Query(payload)      --> Querying
///   Querying  -- server Reply(payload)      --> Acquired
///   Acquired  -- client Release             --> Idle
///   Acquired  -- client Done                --> Done
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalTxMonitorState {
    Idle,
    Acquiring,
    Acquired,
    Querying,
    Done,
}

/// Output of a single LocalTxMonitor transition.
///
/// `Event` carries a `LocalTxMonitorEvent` derived from the incoming
/// message. `Done` is emitted only on the client Done terminal
/// transition. The state machine does not decode query/reply payloads
/// or mutate mempool state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalTxMonitorOutput {
    Event(LocalTxMonitorEvent),
    Done,
}

/// Structured LocalTxMonitor errors. No `String`, no `anyhow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalTxMonitorError {
    /// A (state, message, agency) triple that the protocol grammar
    /// forbids.
    IllegalTransition {
        state: LocalTxMonitorState,
        message_tag: &'static str,
        agency: LocalTxMonitorAgency,
    },
    /// Message variant valid in the grammar but rejected by the
    /// selected protocol version. Carries the version newtype and the
    /// tag of the offending message.
    InvalidForVersion {
        version: LocalTxMonitorVersion,
        message_tag: &'static str,
    },
    /// Structurally-valid message that fails protocol-grammar
    /// invariants the codec does not check.
    MalformedMessage { reason: &'static str },
}
