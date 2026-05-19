// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// LocalChainSync state machine types — pure values, no I/O, no async.
//
// `LocalChainSyncState` encodes the five protocol states from the
// Ouroboros local-chain-sync mini-protocol per cardano-node 11.0.1 (10.6.2 forward-compatible).
// `LocalChainSyncOutput` distinguishes wire replies (locally-originated
// messages to be encoded by the S-A2 codec) from local-chain-sync
// events (consumer-interface values). `LocalChainSyncError` is
// structured — every variant carries typed context, no `String`.

use crate::codec::local_chain_sync::LocalChainSyncMessage;
use crate::codec::version::LocalChainSyncVersion;
use crate::n2c::local_chain_sync::agency::LocalChainSyncAgency;
use crate::n2c::local_chain_sync::event::LocalChainSyncEvent;

/// Closed LocalChainSync protocol state per Ouroboros mini-protocol
/// spec.
///
/// State graph:
///   Idle      -- client RequestNext     --> CanAwait
///   Idle      -- client FindIntersect   --> Intersect
///   Idle      -- client Done            --> Done
///   CanAwait  -- server RollForward     --> Idle
///   CanAwait  -- server RollBackward    --> Idle
///   CanAwait  -- server AwaitReply      --> MustReply
///   MustReply -- server RollForward     --> Idle
///   MustReply -- server RollBackward    --> Idle
///   Intersect -- server IntersectFound  --> Idle
///   Intersect -- server IntersectNotFound --> Idle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalChainSyncState {
    Idle,
    CanAwait,
    MustReply,
    Intersect,
    Done,
}

/// Output of a single LocalChainSync transition.
///
/// `Reply` carries the next on-wire message *value* the local side
/// originates — encoding to bytes is the S-A2 codec's job. `Event`
/// carries a `LocalChainSyncEvent` derived from a server reply. The
/// state machine does not decode block bytes or mutate chain state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalChainSyncOutput {
    Reply(LocalChainSyncMessage),
    Event(LocalChainSyncEvent),
    Done,
}

/// Structured LocalChainSync errors. No `String`, no `anyhow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalChainSyncError {
    /// A (state, message, agency) triple that the protocol grammar
    /// forbids — e.g. server sending `RequestNext`, or `RollForward`
    /// arriving while the state machine is `Idle`.
    IllegalTransition {
        state: LocalChainSyncState,
        message_tag: &'static str,
        agency: LocalChainSyncAgency,
    },
    /// Message variant valid in the grammar but rejected by the
    /// selected protocol version. Carries the version newtype and the
    /// tag of the offending message.
    InvalidForVersion {
        version: LocalChainSyncVersion,
        message_tag: &'static str,
    },
    /// Structurally-valid message that fails protocol-grammar
    /// invariants the codec does not check (e.g. an empty intersect
    /// point list).
    MalformedMessage { reason: &'static str },
}
