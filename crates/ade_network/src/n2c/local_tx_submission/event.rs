// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// LocalTxSubmission event taxonomy emitted by the state machine.
//
// Per slice S-A8 §9: N-A produces values, downstream interprets effects.
// The state machine does not decode `tx_bytes`, does not touch the
// mempool, and does not perform tx validation — it emits a
// `LocalTxSubmissionEvent` value per client/server message.
//
// `TxRejection` is re-exported from the S-A2 codec module so every
// consumer references the same canonical type.

pub use crate::codec::local_tx_submission::TxRejection;

/// LocalTxSubmission event taxonomy. Closed enum; consumers
/// exhaustively match.
///
/// `TxSubmitted.tx_bytes` is opaque — the exact bytes the client sent
/// on the wire, passed through verbatim. `TxRejected.rejection` carries
/// the ledger-defined rejection reason bytes verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalTxSubmissionEvent {
    TxSubmitted { tx_bytes: Vec<u8> },
    TxAccepted,
    TxRejected { rejection: TxRejection },
}
