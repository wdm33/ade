// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE receive-side event / effect / error closed sums
//! (PHASE4-N-H S1).
//!
//! The receive reducer consumes [`ReceiveEvent`] values — a closed
//! sum lifting the receive-relevant subset of N-A signals/events:
//!   - [`ReceiveEvent::RollForward`] from
//!     `ade_network::chain_sync::ForkChoiceSignal::RollForward`
//!   - [`ReceiveEvent::RollBackward`] from
//!     `ade_network::chain_sync::ForkChoiceSignal::RollBackward`
//!   - [`ReceiveEvent::BlockDelivered`] from
//!     `ade_network::block_fetch::BatchDeliveryEvent::BlockDelivered`
//!
//! Locally-originated chain-sync/block-fetch outputs (client requests
//! the orchestrator sends — RequestNext, RequestRange, ClientDone,
//! FindIntersect, Done) are NOT constructible here — that is the
//! CN-PROTO-07 closure.
//!
//! [`ReceiveEffect`] reports what the reducer did. [`ReceiveError`]
//! is the closed failure surface.

use ade_types::{Hash32, SlotNo};

use crate::block_validity::verdict::BlockValidityError;

/// Target point of a `RollBackward` — the (slot, hash) the peer
/// instructs us to roll back to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetPoint {
    pub slot: SlotNo,
    pub hash: Hash32,
}

/// Tip metadata carried by `RollForward` and `RollBackward`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TipPoint {
    pub slot: SlotNo,
    pub hash: Hash32,
    pub block_no: u64,
}

/// Closed receive-event sum. Three variants only; no
/// `#[non_exhaustive]`, no `String`-bearing variant.
///
/// CN-PROTO-07: the orchestrator may not construct a `ReceiveEvent`
/// from a locally-originated chain-sync or block-fetch message — the
/// public API has no such constructor. The events here are exactly
/// the peer-originated stream subset the reducer needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiveEvent {
    /// Peer announces a new tip via chain-sync RollForward. Header
    /// bytes are the wire-bytes from the peer's reply; (slot, hash)
    /// identify the announced block.
    RollForward {
        slot: SlotNo,
        hash: Hash32,
        header_bytes: Vec<u8>,
        tip: TipPoint,
    },
    /// Peer instructs rollback to `target_point`. In the receive
    /// cluster (Path A), the reducer returns
    /// `Err(ReceiveError::RollbackOutOfScope)` for this branch.
    RollBackward {
        target_point: TargetPoint,
        tip: TipPoint,
    },
    /// Block body delivered via block-fetch. `block_bytes` is the
    /// full era-tagged envelope — same format `block_validity`
    /// consumes.
    BlockDelivered { block_bytes: Vec<u8> },
}

/// Closed receive-effect sum. What the reducer did on a successful
/// event apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiveEffect {
    /// Block admitted into ChainDb + ledger + chain_dep.
    Admitted { slot: SlotNo, hash: Hash32 },
    /// Header cached pending body delivery.
    Cached { slot: SlotNo, hash: Hash32 },
    /// Rollback applied (unreachable in Path A; reserved for the
    /// follow-on rollback cluster).
    RolledBack { to_slot: SlotNo },
    /// Event observed; reducer chose no-op (e.g., duplicate header
    /// announcement at the same key with the same bytes).
    NoOp { reason: NoOpReason },
}

/// Closed reason-tag for `ReceiveEffect::NoOp`. No `String`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoOpReason {
    /// `RollForward` for a key already cached with the same bytes.
    HeaderAlreadyCached,
}

/// Closed receive-error sum. Total over the reducer's failure modes.
#[derive(Debug, Clone, PartialEq)]
pub enum ReceiveError {
    /// `BlockDelivered` arrived but no cached header matches the
    /// decoded body's (slot, hash) — or the cached header bytes do
    /// not match. DC-CONS-19 enforcement.
    HeaderBodyMismatch {
        decoded_slot: SlotNo,
        decoded_hash: Hash32,
    },
    /// `block_validity` returned Invalid; the upstream typed error is
    /// carried verbatim.
    Validity(BlockValidityError),
    /// Path A scope edge: `RollBackward` is not yet supported. The
    /// orchestrator halts the peer pipeline. Full rollback authority
    /// is a follow-on cluster's deliverable.
    RollbackOutOfScope { target_point: TargetPoint },
    /// ChainDb-write failure surfaced by the trait impl.
    ChainDb(crate::receive::chain_write::ChainWriteError),
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn sample_tip() -> TipPoint {
        TipPoint {
            slot: SlotNo(100),
            hash: Hash32([0xAA; 32]),
            block_no: 42,
        }
    }

    #[test]
    fn receive_event_round_trips_through_pattern_match() {
        let events = vec![
            ReceiveEvent::RollForward {
                slot: SlotNo(1),
                hash: Hash32([0x01; 32]),
                header_bytes: vec![0x40],
                tip: sample_tip(),
            },
            ReceiveEvent::RollBackward {
                target_point: TargetPoint {
                    slot: SlotNo(0),
                    hash: Hash32([0x00; 32]),
                },
                tip: sample_tip(),
            },
            ReceiveEvent::BlockDelivered {
                block_bytes: vec![0x40],
            },
        ];
        for e in events {
            // Exhaustive match — if a fourth variant is added, this
            // test fails to compile, surfacing the closure regression.
            match e {
                ReceiveEvent::RollForward { .. } => {}
                ReceiveEvent::RollBackward { .. } => {}
                ReceiveEvent::BlockDelivered { .. } => {}
            }
        }
    }

    #[test]
    fn receive_effect_round_trips_through_pattern_match() {
        let effects = vec![
            ReceiveEffect::Admitted {
                slot: SlotNo(1),
                hash: Hash32([0x01; 32]),
            },
            ReceiveEffect::Cached {
                slot: SlotNo(1),
                hash: Hash32([0x01; 32]),
            },
            ReceiveEffect::RolledBack {
                to_slot: SlotNo(0),
            },
            ReceiveEffect::NoOp {
                reason: NoOpReason::HeaderAlreadyCached,
            },
        ];
        for e in effects {
            match e {
                ReceiveEffect::Admitted { .. } => {}
                ReceiveEffect::Cached { .. } => {}
                ReceiveEffect::RolledBack { .. } => {}
                ReceiveEffect::NoOp { .. } => {}
            }
        }
    }

    #[test]
    fn receive_error_round_trips_through_pattern_match() {
        let errs = vec![
            ReceiveError::HeaderBodyMismatch {
                decoded_slot: SlotNo(1),
                decoded_hash: Hash32([0x01; 32]),
            },
            ReceiveError::RollbackOutOfScope {
                target_point: TargetPoint {
                    slot: SlotNo(0),
                    hash: Hash32([0x00; 32]),
                },
            },
        ];
        for e in errs {
            match e {
                ReceiveError::HeaderBodyMismatch { .. } => {}
                ReceiveError::Validity(_) => {}
                ReceiveError::RollbackOutOfScope { .. } => {}
                ReceiveError::ChainDb(_) => {}
            }
        }
    }
}
