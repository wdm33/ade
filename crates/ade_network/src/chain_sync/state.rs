// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Chain-sync state machine types — pure values, no I/O, no async.
//
// `ChainSyncState` encodes the five protocol states from the Ouroboros
// chain-sync mini-protocol per cardano-node 11.0.1 (10.6.2 forward-compatible). `ChainSyncOutput`
// distinguishes wire replies (locally-originated messages to be encoded
// by the S-A2 codec) from fork-choice signals (consensus-interface
// values consumed by N-B). `ChainSyncError` is structured — every
// variant carries typed context, no `String`.

use crate::codec::chain_sync::ChainSyncMessage;
use crate::codec::version::ChainSyncVersion;

use crate::chain_sync::signal::ForkChoiceSignal;

/// Closed chain-sync protocol state per Ouroboros mini-protocol spec.
///
/// State graph:
///   Idle      -- client RequestNext     --> CanAwait
///   Idle      -- client FindIntersect   --> Intersect
///   Idle      -- client ClientDone      --> Done
///   CanAwait  -- server RollForward     --> Idle
///   CanAwait  -- server RollBackward    --> Idle
///   CanAwait  -- server AwaitReply      --> MustReply
///   MustReply -- server RollForward     --> Idle
///   MustReply -- server RollBackward    --> Idle
///   Intersect -- server IntersectFound  --> Idle
///   Intersect -- server IntersectNotFound --> Idle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainSyncState {
    Idle,
    CanAwait,
    MustReply,
    Intersect,
    Done,
}

/// Output of a single chain-sync transition.
///
/// `Reply` carries the next on-wire message *value* the local side
/// originates — encoding to bytes is the S-A2 codec's job, not the
/// state machine's. `Signal` carries a `ForkChoiceSignal` derived from
/// a server reply; N-B (consensus runtime) interprets the signal. The
/// state machine does not decode header bytes or mutate chain state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainSyncOutput {
    Reply(ChainSyncMessage),
    Signal(ForkChoiceSignal),
    Done,
}

/// Structured chain-sync errors. No `String`, no `anyhow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainSyncError {
    /// A (state, message, agency) triple that the protocol grammar
    /// forbids — e.g. server sending `RequestNext`, or `RollForward`
    /// arriving while the state machine is `Idle`.
    IllegalTransition {
        state: ChainSyncState,
        message_tag: &'static str,
        agency: &'static str,
    },
    /// Message variant valid in the grammar but rejected by the
    /// selected protocol version. Carries the version newtype and the
    /// tag of the offending message.
    InvalidForVersion {
        version: ChainSyncVersion,
        message_tag: &'static str,
    },
    /// Structurally-valid message that fails protocol-grammar invariants
    /// the codec does not check (e.g. an empty intersect point list).
    MalformedMessage { reason: &'static str },
}
