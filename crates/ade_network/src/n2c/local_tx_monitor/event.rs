// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// LocalTxMonitor event taxonomy emitted by the state machine.
//
// Per slice S-A8 §9: N-A produces values, downstream interprets effects.
// The state machine does not decode `LocalTxMonitorQuery` /
// `LocalTxMonitorReply`, does not touch mempool state, and does not
// perform query evaluation — it emits a `LocalTxMonitorEvent` value
// derived from each client/server message.
//
// `LocalTxMonitorQuery` and `LocalTxMonitorReply` are re-exported from
// the S-A2 codec module so every consumer references the same
// canonical types.

use ade_types::SlotNo;

pub use crate::codec::local_tx_monitor::{LocalTxMonitorQuery, LocalTxMonitorReply};

/// LocalTxMonitor event taxonomy. Closed enum; consumers exhaustively
/// match.
///
/// `QueryRequested.payload` and `QueryReplied.payload` are opaque —
/// the exact bytes the client or server sent on the wire, passed
/// through verbatim. Mempool-semantic interpretation belongs to a
/// future cluster (N-F). `MempoolAcquired.slot` is the snapshot slot
/// the server pinned the mempool view at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalTxMonitorEvent {
    AcquireRequested,
    AwaitingAcquisition,
    MempoolAcquired { slot: SlotNo },
    QueryRequested { payload: LocalTxMonitorQuery },
    QueryReplied { payload: LocalTxMonitorReply },
    MempoolReleased,
}
