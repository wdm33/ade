// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// LocalTxSubmission state machine types — pure values, no I/O, no
// async.
//
// `LocalTxSubmissionState` encodes the three protocol states from the
// Ouroboros local-tx-submission mini-protocol per cardano-node 11.0.1 (10.6.2 forward-compatible).
// `LocalTxSubmissionOutput` carries an event per client/server message
// (Reply is not separately modeled — every transition either yields an
// event or terminates the session). `LocalTxSubmissionError` is
// structured — every variant carries typed context, no `String`.

use crate::codec::version::LocalTxSubmissionVersion;
use crate::n2c::local_tx_submission::agency::LocalTxSubmissionAgency;
use crate::n2c::local_tx_submission::event::LocalTxSubmissionEvent;

/// Closed LocalTxSubmission protocol state per Ouroboros mini-protocol
/// spec.
///
/// State graph:
///   Idle -- client SubmitTx{tx_bytes} --> Busy
///   Idle -- client Done               --> Done
///   Busy -- server AcceptTx(_)        --> Idle
///   Busy -- server RejectTx(reason)   --> Idle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalTxSubmissionState {
    Idle,
    Busy,
    Done,
}

/// Output of a single LocalTxSubmission transition.
///
/// `Event` carries a `LocalTxSubmissionEvent` derived from the incoming
/// message. `Done` is emitted only on the client Done terminal
/// transition. The state machine does not decode tx bytes or mutate
/// mempool state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalTxSubmissionOutput {
    Event(LocalTxSubmissionEvent),
    Done,
}

/// Structured LocalTxSubmission errors. No `String`, no `anyhow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalTxSubmissionError {
    /// A (state, message, agency) triple that the protocol grammar
    /// forbids — e.g. server sending `SubmitTx`, or `AcceptTx`
    /// arriving while the state machine is `Idle`.
    IllegalTransition {
        state: LocalTxSubmissionState,
        message_tag: &'static str,
        agency: LocalTxSubmissionAgency,
    },
    /// Message variant valid in the grammar but rejected by the
    /// selected protocol version. Carries the version newtype and the
    /// tag of the offending message.
    InvalidForVersion {
        version: LocalTxSubmissionVersion,
        message_tag: &'static str,
    },
    /// Structurally-valid message that fails protocol-grammar
    /// invariants the codec does not check.
    MalformedMessage { reason: &'static str },
}
