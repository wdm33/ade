// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// LocalStateQuery event taxonomy emitted by the state machine.
//
// Per slice S-A8 §9: N-A produces values, downstream interprets effects.
// The state machine does not decode `QueryPayload` / `ResultPayload`,
// does not touch ledger state, and does not perform query evaluation —
// it emits a `LocalStateQueryEvent` value derived from each
// client/server message.
//
// `Point`, `AcquireFailure`, `QueryPayload`, and `ResultPayload` are
// re-exported from the S-A2 codec module so every consumer references
// the same canonical types.

pub use crate::codec::local_state_query::{AcquireFailure, Point, QueryPayload, ResultPayload};

/// LocalStateQuery event taxonomy. Closed enum; consumers exhaustively
/// match.
///
/// `QueryRequested.payload` and `QueryReplied.payload` are opaque —
/// the exact bytes the client or server sent on the wire, passed
/// through verbatim. Ledger-semantic interpretation belongs to a
/// future cluster (N-F).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalStateQueryEvent {
    AcquireRequested { point: Option<Point> },
    SnapshotAcquired,
    AcquireFailed { reason: AcquireFailure },
    QueryRequested { payload: QueryPayload },
    QueryReplied { payload: ResultPayload },
    SnapshotReleased,
    ReAcquireRequested { point: Option<Point> },
}
