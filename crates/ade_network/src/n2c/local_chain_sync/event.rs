// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// LocalChainSync event taxonomy emitted by the state machine.
//
// Per slice S-A8 §9: N-A produces values, downstream interprets effects.
// The state machine does not decode `block_bytes`, does not touch
// ChainDb, and does not perform fork-choice — it emits a
// `LocalChainSyncEvent` value derived from a server reply.
//
// `Point` and `Tip` are re-exported from the S-A2 codec module so every
// consumer references the same canonical types.

pub use crate::codec::local_chain_sync::{Point, Tip};

/// LocalChainSync event taxonomy. Closed enum; consumers exhaustively
/// match.
///
/// `RollForward.block_bytes` is opaque — the exact full-block bytes
/// the server sent on the wire, passed through verbatim. Decoding
/// lives in downstream consumers (block-body pipeline).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalChainSyncEvent {
    RollForward { block_bytes: Vec<u8>, tip: Tip },
    RollBackward { point: Point, tip: Tip },
    Intersected { point: Point, tip: Tip },
    NoIntersection { tip: Tip },
}
