// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Tx-submission2 state machine types — pure values, no I/O, no async.
//
// `TxSubmission2State` encodes the six protocol states from the
// Ouroboros tx-submission2 mini-protocol per cardano-node 11.0.1 (10.6.2 forward-compatible). The
// `TxIdsBlocking` and `TxIdsNonBlocking` variants are kept separate
// (rather than collapsed to a single `TxIdsAwaiting { blocking: bool }`)
// because the grammar differs: blocking replies must be non-empty,
// non-blocking replies may be empty. Encoding the distinction in the
// type keeps the transition table explicit. Memory is bounded — the
// state only carries integer counts, never any tx IDs.
//
// `TxSubmission2Output` distinguishes inventory events (consumer-facing
// values consumed by the future mempool) from session termination.
// `TxSubmission2Error` is structured — every variant carries typed
// context, no `String`.

use crate::codec::version::TxSubmission2Version;
use crate::tx_submission::agency::TxSubmission2Agency;
use crate::tx_submission::event::InventoryEvent;

/// Closed tx-submission2 protocol state per Ouroboros mini-protocol spec.
///
/// State graph:
///   Init               -- client Init                      --> Idle
///   Idle               -- server RequestTxIds{blocking:T}  --> TxIdsBlocking{req}
///   Idle               -- server RequestTxIds{blocking:F}  --> TxIdsNonBlocking{req}
///   Idle               -- server RequestTxs(ids)           --> TxsRequested{req_count}
///   Idle               -- server Done                      --> Done
///   TxIdsBlocking{req} -- client ReplyTxIds(entries)       --> Idle
///   TxIdsNonBlocking{req} -- client ReplyTxIds(entries)    --> Idle
///   TxsRequested{n}    -- client ReplyTxs(tx_bytes)        --> Idle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxSubmission2State {
    Init,
    Idle,
    TxIdsBlocking { req: u16 },
    TxIdsNonBlocking { req: u16 },
    TxsRequested { req_count: usize },
    Done,
}

/// Output of a single tx-submission2 transition.
///
/// `Event` carries an `InventoryEvent` derived from the wire message;
/// the future mempool cluster interprets the event. The state machine
/// does not decode tx bodies or mutate inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxSubmission2Output {
    Event(InventoryEvent),
    Done,
}

/// Structured tx-submission2 errors. No `String`, no `anyhow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxSubmission2Error {
    /// A (state, message, agency) triple that the protocol grammar
    /// forbids — e.g. server-originated `RequestTxIds` arriving while
    /// the state machine is `TxsRequested`, or `Init` paired with
    /// `Server` agency.
    IllegalTransition {
        state: TxSubmission2State,
        message_tag: &'static str,
        agency: TxSubmission2Agency,
    },
    /// Message variant valid in the grammar but rejected by the
    /// selected protocol version. Carries the version newtype and the
    /// tag of the offending message.
    InvalidForVersion {
        version: TxSubmission2Version,
        message_tag: &'static str,
    },
    /// Structurally-valid message that fails protocol-grammar invariants
    /// the codec does not check: blocking `ReplyTxIds` must be
    /// non-empty, `ReplyTxIds` count must not exceed the advertised
    /// `req`, `RequestTxs` must request at least one tx, `ReplyTxs`
    /// count must not exceed the outstanding `req_count`.
    MalformedMessage { reason: &'static str },
}
