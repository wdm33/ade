// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Fork-choice signal taxonomy emitted by the chain-sync transition.
//
// Per slice S-A4 §9: N-A produces values, N-B interprets effects. The
// chain-sync state machine does not decode `header_bytes`, does not
// touch ChainDb, and does not perform fork-choice — it emits the
// `ForkChoiceSignal` value derived from a server reply, and the N-B
// consensus runtime consumes the signal stream.
//
// `Point` and `Tip` are re-exported from the S-A2 codec module so
// every consumer references the same canonical types.

pub use crate::codec::chain_sync::{Point, Tip};

/// Fork-choice taxonomy. Closed enum; consumers exhaustively match.
///
/// `RollForward.header_bytes` is opaque — the exact bytes the server
/// sent on the wire, passed through verbatim. Decoding lives in N-B's
/// header pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForkChoiceSignal {
    RollForward { header_bytes: Vec<u8>, tip: Tip },
    RollBackward { point: Point, tip: Tip },
    Intersected { point: Point, tip: Tip },
    NoIntersection { tip: Tip },
}
