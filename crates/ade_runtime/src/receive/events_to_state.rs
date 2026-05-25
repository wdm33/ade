// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN adapter (PHASE4-N-H S3): lift N-A signals/events into the
//! BLUE [`ReceiveEvent`] stream.
//!
//! Variants that aren't state-changing for the receive bridge
//! (BatchStarted, NoBlocks, BatchCompleted, Intersected,
//! NoIntersection) return `None`. The orchestrator (S4) filters
//! these out before calling `receive_apply`.
//!
//! Pass-through discipline: `header_bytes` and `block_bytes` are
//! NEVER decoded here — the BLUE reducer's `BlockDelivered` branch
//! is the canonical decode site.

use ade_ledger::receive::{ReceiveEvent, TargetPoint, TipPoint};
use ade_network::block_fetch::event::{BatchDeliveryEvent, Point as BfPoint};
use ade_network::chain_sync::signal::{ForkChoiceSignal, Point as CsPoint, Tip};

/// Lift a chain-sync `ForkChoiceSignal` into a `ReceiveEvent`.
///
/// `RollForward { header_bytes, tip }` carries the announced block
/// at `tip.point` — Cardano chain-sync semantics: the RollForward
/// header IS the new tip block's header. We extract (slot, hash)
/// from `tip.point` (Point::Block) and pass `header_bytes` through
/// verbatim. If `tip.point` is `Point::Origin` (only legal at the
/// chain start), we cannot form a `ReceiveEvent::RollForward` (no
/// slot+hash) — return `None` so the orchestrator skips it.
///
/// `RollBackward { point, tip }`: lift to
/// `ReceiveEvent::RollBackward`. Origin point is treated as
/// `(SlotNo(0), Hash32([0; 32]))` for the target — the reducer
/// returns `RollbackOutOfScope` for either case (Path A scope edge).
///
/// `Intersected` / `NoIntersection`: return `None` — orchestrator
/// concerns, not state-changing.
pub fn lift_chain_sync_signal(sig: ForkChoiceSignal) -> Option<ReceiveEvent> {
    match sig {
        ForkChoiceSignal::RollForward { header_bytes, tip } => {
            let (slot, hash) = point_to_slot_hash_cs(&tip.point)?;
            Some(ReceiveEvent::RollForward {
                slot,
                hash,
                header_bytes,
                tip: tip_to_tip_point_cs(&tip),
            })
        }
        ForkChoiceSignal::RollBackward { point, tip } => {
            let (slot, hash) = match point_to_slot_hash_cs(&point) {
                Some(sh) => sh,
                None => (ade_types::SlotNo(0), ade_types::Hash32([0u8; 32])),
            };
            Some(ReceiveEvent::RollBackward {
                target_point: TargetPoint { slot, hash },
                tip: tip_to_tip_point_cs(&tip),
            })
        }
        ForkChoiceSignal::Intersected { .. } | ForkChoiceSignal::NoIntersection { .. } => None,
    }
}

/// Lift a block-fetch `BatchDeliveryEvent` into a `ReceiveEvent`.
///
/// `BlockDelivered { block_bytes }`: lift directly. The reducer
/// decodes on this branch — the adapter does NOT.
///
/// `BatchStarted` / `NoBlocks` / `BatchCompleted`: return `None`.
pub fn lift_block_fetch_event(ev: BatchDeliveryEvent) -> Option<ReceiveEvent> {
    match ev {
        BatchDeliveryEvent::BlockDelivered { block_bytes } => {
            Some(ReceiveEvent::BlockDelivered { block_bytes })
        }
        BatchDeliveryEvent::BatchStarted
        | BatchDeliveryEvent::NoBlocks
        | BatchDeliveryEvent::BatchCompleted => None,
    }
}

fn point_to_slot_hash_cs(p: &CsPoint) -> Option<(ade_types::SlotNo, ade_types::Hash32)> {
    match p {
        CsPoint::Block { slot, hash } => Some((*slot, hash.clone())),
        CsPoint::Origin => None,
    }
}

fn tip_to_tip_point_cs(tip: &Tip) -> TipPoint {
    let (slot, hash) = match &tip.point {
        CsPoint::Block { slot, hash } => (*slot, hash.clone()),
        CsPoint::Origin => (ade_types::SlotNo(0), ade_types::Hash32([0u8; 32])),
    };
    TipPoint {
        slot,
        hash,
        block_no: tip.block_no,
    }
}

// Suppress unused-import warning; BfPoint is exposed for future
// adapter extensions but not used at this slice.
#[allow(dead_code)]
fn _bf_point_marker(_p: &BfPoint) {}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use ade_network::chain_sync::signal::Tip as CsTip;
    use ade_types::{Hash32, SlotNo};

    fn tip(slot: u64, h: u8, block_no: u64) -> CsTip {
        CsTip {
            point: CsPoint::Block {
                slot: SlotNo(slot),
                hash: Hash32([h; 32]),
            },
            block_no,
        }
    }

    #[test]
    fn lift_chain_sync_signal_roll_forward_yields_receive_event() {
        let sig = ForkChoiceSignal::RollForward {
            header_bytes: vec![0x46, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
            tip: tip(100, 0xAA, 7),
        };
        let ev = lift_chain_sync_signal(sig).expect("Some");
        match ev {
            ReceiveEvent::RollForward { slot, hash, header_bytes, tip } => {
                assert_eq!(slot, SlotNo(100));
                assert_eq!(hash, Hash32([0xAA; 32]));
                assert_eq!(header_bytes, vec![0x46, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
                assert_eq!(tip.block_no, 7);
            }
            other => panic!("expected RollForward, got {other:?}"),
        }
    }

    #[test]
    fn lift_chain_sync_signal_roll_backward_yields_receive_event() {
        let sig = ForkChoiceSignal::RollBackward {
            point: CsPoint::Block {
                slot: SlotNo(50),
                hash: Hash32([0xBB; 32]),
            },
            tip: tip(100, 0xAA, 7),
        };
        let ev = lift_chain_sync_signal(sig).expect("Some");
        match ev {
            ReceiveEvent::RollBackward { target_point, tip } => {
                assert_eq!(target_point.slot, SlotNo(50));
                assert_eq!(target_point.hash, Hash32([0xBB; 32]));
                assert_eq!(tip.block_no, 7);
            }
            other => panic!("expected RollBackward, got {other:?}"),
        }
    }

    #[test]
    fn lift_chain_sync_signal_intersected_yields_none() {
        let sig = ForkChoiceSignal::Intersected {
            point: CsPoint::Origin,
            tip: tip(0, 0x00, 0),
        };
        assert!(lift_chain_sync_signal(sig).is_none());
    }

    #[test]
    fn lift_chain_sync_signal_no_intersection_yields_none() {
        let sig = ForkChoiceSignal::NoIntersection {
            tip: tip(0, 0x00, 0),
        };
        assert!(lift_chain_sync_signal(sig).is_none());
    }

    #[test]
    fn lift_block_fetch_event_block_delivered_yields_receive_event() {
        let ev = BatchDeliveryEvent::BlockDelivered {
            block_bytes: vec![0x40, 0x01, 0x02],
        };
        let r = lift_block_fetch_event(ev).expect("Some");
        match r {
            ReceiveEvent::BlockDelivered { block_bytes } => {
                assert_eq!(block_bytes, vec![0x40, 0x01, 0x02]);
            }
            other => panic!("expected BlockDelivered, got {other:?}"),
        }
    }

    #[test]
    fn lift_block_fetch_event_batch_started_yields_none() {
        assert!(lift_block_fetch_event(BatchDeliveryEvent::BatchStarted).is_none());
    }

    #[test]
    fn lift_block_fetch_event_no_blocks_yields_none() {
        assert!(lift_block_fetch_event(BatchDeliveryEvent::NoBlocks).is_none());
    }

    #[test]
    fn lift_block_fetch_event_batch_completed_yields_none() {
        assert!(lift_block_fetch_event(BatchDeliveryEvent::BatchCompleted).is_none());
    }
}
