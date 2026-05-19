// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Batch-delivery event taxonomy emitted by the block-fetch transition.
//
// Per slice S-A5 §9: N-A produces values, N-B interprets effects. The
// block-fetch state machine does not decode `block_bytes`, does not
// touch ChainDb, and does not perform block validation — it emits a
// `BatchDeliveryEvent` value per server reply, and the N-B consensus
// runtime consumes the event stream.
//
// `Point` and `Range` are re-exported from the S-A2 codec module so
// every consumer references the same canonical types.

pub use crate::codec::block_fetch::{Point, Range};

/// Batch-delivery taxonomy. Closed enum; consumers exhaustively match.
///
/// `BlockDelivered.block_bytes` is opaque — the exact bytes the server
/// sent on the wire, passed through verbatim. Decoding lives in N-B's
/// block-body pipeline via `ade_codec` era decoders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchDeliveryEvent {
    BatchStarted,
    BlockDelivered { block_bytes: Vec<u8> },
    NoBlocks,
    BatchCompleted,
}
