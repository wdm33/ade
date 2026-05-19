// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Block-fetch state machine types — pure values, no I/O, no async.
//
// `BlockFetchState` encodes the four protocol states from the Ouroboros
// block-fetch mini-protocol per cardano-node 11.0.1 (10.6.2 forward-compatible). `BlockFetchOutput`
// distinguishes wire replies (locally-originated messages to be encoded
// by the S-A2 codec) from batch-delivery events (consensus-interface
// values consumed by N-B). `BlockFetchError` is structured — every
// variant carries typed context, no `String`.

use crate::block_fetch::agency::BlockFetchAgency;
use crate::block_fetch::event::BatchDeliveryEvent;
use crate::codec::block_fetch::BlockFetchMessage;
use crate::codec::version::BlockFetchVersion;

/// Closed block-fetch protocol state per Ouroboros mini-protocol spec.
///
/// State graph:
///   Idle      -- client RequestRange(range) --> Busy
///   Idle      -- client ClientDone          --> Done
///   Busy      -- server StartBatch          --> Streaming
///   Busy      -- server NoBlocks            --> Idle
///   Streaming -- server Block(bytes)        --> Streaming
///   Streaming -- server BatchDone           --> Idle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFetchState {
    Idle,
    Busy,
    Streaming,
    Done,
}

/// Output of a single block-fetch transition.
///
/// `Reply` carries the next on-wire message *value* the local side
/// originates — encoding to bytes is the S-A2 codec's job, not the
/// state machine's. `Event` carries a `BatchDeliveryEvent` derived from
/// a server reply; N-B (consensus runtime) interprets the event. The
/// state machine does not decode block bytes or mutate chain state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockFetchOutput {
    Reply(BlockFetchMessage),
    Event(BatchDeliveryEvent),
    Done,
}

/// Structured block-fetch errors. No `String`, no `anyhow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockFetchError {
    /// A (state, message, agency) triple that the protocol grammar
    /// forbids — e.g. server sending `RequestRange`, or `Block`
    /// arriving while the state machine is `Idle`.
    IllegalTransition {
        state: BlockFetchState,
        message_tag: &'static str,
        agency: BlockFetchAgency,
    },
    /// Message variant valid in the grammar but rejected by the
    /// selected protocol version. Carries the version newtype and the
    /// tag of the offending message.
    InvalidForVersion {
        version: BlockFetchVersion,
        message_tag: &'static str,
    },
    /// Structurally-valid message that fails protocol-grammar invariants
    /// the codec does not check (e.g. an inverted range where the
    /// `from` slot is greater than the `to` slot).
    MalformedMessage { reason: &'static str },
}
