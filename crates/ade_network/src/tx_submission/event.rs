// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Inventory event taxonomy emitted by the tx-submission2 transition.
//
// Per slice S-A6 §9: N-A produces values, the mempool (future cluster)
// interprets effects. The tx-submission2 state machine does not decode
// `tx_bytes`, does not maintain a tx-ID inventory, and does not mutate
// mempool state — it emits an `InventoryEvent` value per message, and
// the mempool consumes the event stream.
//
// `TxIdAndSize` is re-exported from the S-A2 codec module so every
// consumer references the same canonical type.

pub use crate::codec::tx_submission::{TxIdAndSize, TxSubmissionTxId};

/// Inventory event taxonomy. Closed enum; consumers exhaustively match.
///
/// `TxsDelivered.tx_bytes` is opaque — the exact bytes the client sent
/// on the wire, passed through verbatim. Tx-body decoding and validation
/// live in the mempool / ledger pipeline, not in the state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryEvent {
    ServerOpened,
    IdsRequested {
        blocking: bool,
        ack: u16,
        req: u16,
    },
    IdsDelivered {
        entries: Vec<TxIdAndSize>,
    },
    TxsRequested {
        ids: Vec<TxSubmissionTxId>,
    },
    TxsDelivered {
        tx_bytes: Vec<Vec<u8>>,
    },
}
