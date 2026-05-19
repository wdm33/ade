// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// LocalTxMonitor state machine types — pure values, no I/O, no async.
//
// `LocalTxMonitorState` encodes the protocol states from the Ouroboros
// local-tx-monitor mini-protocol per cardano-node 11.0.1. The Busy
// state is parameterised by `BusyKind` to capture which query the
// client issued, so the state machine knows which Reply variant is
// legal next. `LocalTxMonitorOutput` carries an event per
// client/server message. `LocalTxMonitorError` is structured — every
// variant carries typed context, no `String`.

use crate::codec::version::LocalTxMonitorVersion;
use crate::n2c::local_tx_monitor::agency::LocalTxMonitorAgency;
use crate::n2c::local_tx_monitor::event::LocalTxMonitorEvent;

/// Closed LocalTxMonitor protocol state per Ouroboros mini-protocol
/// spec.
///
/// State graph:
///   Idle              -- client Done                  --> Done
///   Idle              -- client Acquire               --> Acquiring
///   Acquiring         -- server Acquired{slot}        --> Acquired
///   Acquired          -- client Acquire (re-acquire)  --> Acquiring
///   Acquired          -- client Release               --> Idle
///   Acquired          -- client NextTx                --> Busy{NextTx}
///   Acquired          -- client HasTx{tx_id}          --> Busy{HasTx}
///   Acquired          -- client GetSizes              --> Busy{GetSizes}
///   Acquired          -- client GetMeasures [v2+]     --> Busy{GetMeasures}
///   Busy{NextTx}      -- server ReplyNextTx           --> Acquired
///   Busy{HasTx}       -- server ReplyHasTx            --> Acquired
///   Busy{GetSizes}    -- server ReplyGetSizes         --> Acquired
///   Busy{GetMeasures} -- server ReplyGetMeasures [v2+]--> Acquired
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalTxMonitorState {
    Idle,
    Acquiring,
    Acquired,
    Busy { kind: BusyKind },
    Done,
}

/// Which query is in flight in a `Busy` state. The codec emits four
/// distinct reply messages — `BusyKind` selects which one is legal as
/// the next server-agency transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusyKind {
    NextTx,
    HasTx,
    GetSizes,
    GetMeasures,
}

/// Output of a single LocalTxMonitor transition.
///
/// `Event` carries a `LocalTxMonitorEvent` derived from the incoming
/// message. `Done` is emitted only on the client Done terminal
/// transition. The state machine does not interpret mempool state.
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
