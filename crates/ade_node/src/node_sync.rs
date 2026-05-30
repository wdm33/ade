// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED `--mode node` sync path (PHASE4-N-F-C L4).
//!
//! L4a (this slice step): the verdict-decoupled block-bytes SOURCE the
//! lifecycle sync path consumes. One ordered source only (E1): either a
//! single peer's `run_admission_wire_pump` event stream, or a
//! deterministic in-memory feed for the hermetic test. The source yields
//! ONLY block bytes — it never derives, surfaces, or depends on an
//! agreement verdict, tip-agreement, or follow decision (E2). A
//! `TipUpdate` is a comparison input for admission's verdict loop, NOT a
//! block and NOT a tip authority for sync, so it is skipped; a clean
//! `Disconnected` (or a closed channel) ends the feed.
//!
//! What L4a is NOT: it is not a verdict flow (no `derive_verdict` /
//! `run_admission`), not a follower (`ade_core_interop::follow` is not
//! validating sync), and it advances no tip. The durable apply +
//! tip-snapshot capture (L4b, via `forward_sync::pump_block` +
//! `PersistentSnapshotCache::capture`) and the kill→warm-start recovery
//! proof (L4c) build on this source in later slice steps; the tip is a
//! durable-apply fact, never an agreement verdict.

use std::collections::VecDeque;

use ade_runtime::admission::AdmissionPeerEvent;
use tokio::sync::mpsc;

/// Closed, verdict-decoupled ordered block-bytes source for the
/// `--mode node` lifecycle sync path (PHASE4-N-F-C L4a).
///
/// One ordered source only (E1). [`NodeBlockSource::next_block`] yields
/// ONLY `AdmissionPeerEvent::Block` payloads, in arrival order; it never
/// surfaces a verdict / tip-agreement / follow decision (E2).
pub enum NodeBlockSource {
    /// One peer's `run_admission_wire_pump` event stream. The pump is
    /// the N2N `BlockFetch` source; this taps its raw `Block` events —
    /// NOT admission's verdict runner (`run_admission`).
    WirePump(mpsc::Receiver<AdmissionPeerEvent>),
    /// Deterministic in-memory ordered feed (hermetic test / loopback).
    /// Exactly the "a live socket is not required" shape `pump_block`
    /// was designed for.
    InMemory(VecDeque<Vec<u8>>),
}

impl NodeBlockSource {
    /// Build an in-memory source from an ordered block-bytes sequence.
    pub fn in_memory(blocks: Vec<Vec<u8>>) -> Self {
        Self::InMemory(VecDeque::from(blocks))
    }

    /// Wrap one peer's wire-pump event receiver as the source.
    pub fn from_wire_pump(rx: mpsc::Receiver<AdmissionPeerEvent>) -> Self {
        Self::WirePump(rx)
    }

    /// Next ordered block bytes, or `None` at clean end-of-feed.
    ///
    /// Selects ONLY `AdmissionPeerEvent::Block`. `TipUpdate` is skipped
    /// (a comparison input for the verdict loop, not a block and not a
    /// sync tip authority). `Disconnected` and a closed channel both end
    /// the feed (a clean disconnect is not a tip authority). No verdict
    /// is ever derived or surfaced here (E2 / no verdict-as-sync).
    pub async fn next_block(&mut self) -> Option<Vec<u8>> {
        match self {
            Self::InMemory(q) => q.pop_front(),
            Self::WirePump(rx) => loop {
                match rx.recv().await {
                    Some(AdmissionPeerEvent::Block { block_bytes, .. }) => {
                        return Some(block_bytes);
                    }
                    // Not a block; not a sync tip authority. Skip.
                    Some(AdmissionPeerEvent::TipUpdate { .. }) => continue,
                    // Clean disconnect ends the feed.
                    Some(AdmissionPeerEvent::Disconnected { .. }) => return None,
                    // Sender dropped: end of feed.
                    None => return None,
                }
            },
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ade_network::codec::chain_sync::{Point, Tip};
    use ade_types::{Hash32, SlotNo};

    fn block(b: u8) -> Vec<u8> {
        vec![b; 4]
    }

    fn tip_update(peer: &str) -> AdmissionPeerEvent {
        AdmissionPeerEvent::TipUpdate {
            peer: peer.to_string(),
            tip: Tip {
                point: Point::Block {
                    slot: SlotNo(1),
                    hash: Hash32([0u8; 32]),
                },
                block_no: 1,
            },
        }
    }

    #[tokio::test]
    async fn in_memory_source_yields_blocks_in_order_then_none() {
        let mut src = NodeBlockSource::in_memory(vec![block(0xA1), block(0xA2), block(0xA3)]);
        assert_eq!(src.next_block().await, Some(block(0xA1)));
        assert_eq!(src.next_block().await, Some(block(0xA2)));
        assert_eq!(src.next_block().await, Some(block(0xA3)));
        assert_eq!(src.next_block().await, None);
        // Idempotent at end-of-feed.
        assert_eq!(src.next_block().await, None);
    }

    #[tokio::test]
    async fn wire_pump_source_selects_blocks_and_skips_tipupdate() {
        let (tx, rx) = mpsc::channel::<AdmissionPeerEvent>(16);
        // Interleave TipUpdate noise with the ordered blocks.
        tx.send(tip_update("p")).await.unwrap();
        tx.send(AdmissionPeerEvent::Block {
            peer: "p".to_string(),
            block_bytes: block(0xB1),
        })
        .await
        .unwrap();
        tx.send(tip_update("p")).await.unwrap();
        tx.send(AdmissionPeerEvent::Block {
            peer: "p".to_string(),
            block_bytes: block(0xB2),
        })
        .await
        .unwrap();
        drop(tx); // close the channel after the ordered blocks

        let mut src = NodeBlockSource::from_wire_pump(rx);
        assert_eq!(src.next_block().await, Some(block(0xB1)));
        assert_eq!(src.next_block().await, Some(block(0xB2)));
        assert_eq!(src.next_block().await, None, "closed channel ends the feed");
    }

    #[tokio::test]
    async fn wire_pump_source_ends_on_disconnect_ignoring_later_blocks() {
        // A clean disconnect ends the feed even if more Block events are
        // queued behind it — a disconnect is not a tip authority, and a
        // single ordered source stops at its peer's disconnect (E1/E2).
        let (tx, rx) = mpsc::channel::<AdmissionPeerEvent>(16);
        tx.send(AdmissionPeerEvent::Block {
            peer: "p".to_string(),
            block_bytes: block(0xC1),
        })
        .await
        .unwrap();
        tx.send(AdmissionPeerEvent::Disconnected {
            peer: "p".to_string(),
        })
        .await
        .unwrap();
        // This block is queued AFTER the disconnect; it must NOT surface.
        tx.send(AdmissionPeerEvent::Block {
            peer: "p".to_string(),
            block_bytes: block(0xC2),
        })
        .await
        .unwrap();
        drop(tx);

        let mut src = NodeBlockSource::from_wire_pump(rx);
        assert_eq!(src.next_block().await, Some(block(0xC1)));
        assert_eq!(
            src.next_block().await,
            None,
            "disconnect ends the feed; later queued blocks are not surfaced"
        );
    }
}
