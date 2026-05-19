// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// LocalTxMonitor event taxonomy emitted by the state machine.
//
// Per slice S-A8b: the state machine owns the closed wire grammar of
// LocalTxMonitor and emits a `LocalTxMonitorEvent` value per
// client/server message. Mempool-semantic interpretation (e.g.
// looking up transactions by id, materialising measures) belongs to
// downstream consumers.

use ade_types::{SlotNo, TxId};

pub use crate::codec::local_tx_monitor::{MempoolMeasures, MempoolSizeAndCapacity};

/// LocalTxMonitor event taxonomy. Closed enum; consumers exhaustively
/// match.
///
/// `ReAcquireRequested` is emitted when the client sends an `Acquire`
/// while in the `Acquired` state — on the wire this is the
/// `MsgAwaitAcquire` form (same tag-1 encoding) and tells the server
/// to re-acquire a fresh mempool snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalTxMonitorEvent {
    AcquireRequested,
    ReAcquireRequested,
    MempoolAcquired { slot: SlotNo },
    MempoolReleased,
    NextTxRequested,
    NextTxReplied { tx_bytes: Option<Vec<u8>> },
    HasTxRequested { tx_id: TxId },
    HasTxReplied { present: bool },
    SizesRequested,
    SizesReplied(MempoolSizeAndCapacity),
    MeasuresRequested,
    MeasuresReplied(MempoolMeasures),
}
