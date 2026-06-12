// Core Contract:
// - RED scheduling discipline ONLY: this layer decides delivery OPPORTUNITY, never
//   fork-choice. The merge order may affect which lane is polled first per round;
//   it MUST NOT decide the selected chain (select_best_chain stays arrival-order
//   independent, CN-CONS-01).
// - Deterministic order derived from the configured --peer list (an explicit Vec),
//   never HashMap/HashSet iteration, never wall-clock / rand.

//! Multi-peer wire-pump fairness (PHASE4-N-AO S8, `DC-PUMP-04`).
//!
//! The S7 live retry surfaced: when several peers feed ONE shared bounded channel,
//! a continuously-producing peer monopolises it and starves the others — so a
//! competing peer's branch never reaches the participant dispatch. This module gives
//! each peer its OWN bounded lane and drains them with a fair round-robin merge:
//! a hot peer fills only its own lane (self-backpressure), while the merge keeps
//! servicing the quiet peers. A disconnected lane is retired in place, leaving the
//! remaining peers' order stable. The merged stream the `NodeBlockSource::WirePump`
//! consumer reads is unchanged in shape (one peer-attributed event sequence).

use std::collections::BTreeMap;
use std::future::poll_fn;
use std::task::Poll;

use ade_runtime::admission::AdmissionPeerEvent;
use tokio::sync::mpsc;

/// Capacity of each PER-PEER lane (bounded → per-peer backpressure; a hot peer
/// self-blocks its own pump, never the shared path). Matches the single-peer
/// budget the pre-S8 shared channel used.
pub const PER_PEER_LANE_CAP: usize = 64;

/// Fairly receive the next event from any live per-peer lane.
///
/// Round-robin over the lanes in their fixed (configured `--peer`) order, starting
/// at `*start` and rotating past the serviced lane so no lane is starved. A lane
/// whose pump has ended (`Poll::Ready(None)`) is **retired in place** (`None`),
/// leaving the remaining lanes' indices — hence their relative order — unchanged.
/// Returns `None` only when every lane is closed. No wall-clock, no rand; the only
/// ordering input is the explicit lane `Vec` + the rotating cursor.
pub async fn fair_recv(
    lanes: &mut [Option<mpsc::Receiver<AdmissionPeerEvent>>],
    start: &mut usize,
) -> Option<AdmissionPeerEvent> {
    poll_fn(|cx| {
        let n = lanes.len();
        if n == 0 {
            return Poll::Ready(None);
        }
        for k in 0..n {
            let i = (*start + k) % n;
            if let Some(rx) = lanes[i].as_mut() {
                match rx.poll_recv(cx) {
                    Poll::Ready(Some(ev)) => {
                        // Fairness: the NEXT pass favours the lane after this one.
                        *start = (i + 1) % n;
                        return Poll::Ready(Some(ev));
                    }
                    // Lane closed — retire in place; remaining lanes keep their index.
                    Poll::Ready(None) => lanes[i] = None,
                    Poll::Pending => {}
                }
            }
        }
        if lanes.iter().all(Option::is_none) {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    })
    .await
}

/// RED fair-merge loop: drain the per-peer lanes fairly into the single merged
/// output the `NodeBlockSource::WirePump` consumer reads. Ends when every lane
/// closes or the consumer drops. Per-peer backpressure is inherent (each lane is
/// bounded; the merge awaits `out.send` only when the CONSUMER is slow — that is
/// global pacing by the consumer, never one peer starving another).
pub async fn fair_merge(
    mut lanes: Vec<Option<mpsc::Receiver<AdmissionPeerEvent>>>,
    out: mpsc::Sender<AdmissionPeerEvent>,
) {
    let mut start = 0usize;
    while let Some(ev) = fair_recv(&mut lanes, &mut start).await {
        if out.send(ev).await.is_err() {
            return; // consumer gone — stop draining
        }
    }
}

/// The peer label every `AdmissionPeerEvent` carries (closed match — all variants).
fn event_peer(ev: &AdmissionPeerEvent) -> &str {
    match ev {
        AdmissionPeerEvent::Block { peer, .. }
        | AdmissionPeerEvent::TipUpdate { peer, .. }
        | AdmissionPeerEvent::RollBackward { peer, .. }
        | AdmissionPeerEvent::Disconnected { peer } => peer,
    }
}

/// GREEN deterministic evidence: per-peer delivered-event counts (by peer label),
/// over a forwarded-event log. Evidence/fairness-assertion only — never affects
/// scheduling or selection. `BTreeMap` (sorted, deterministic).
pub fn per_peer_delivered_counts(events: &[AdmissionPeerEvent]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for ev in events {
        *counts.entry(event_peer(ev).to_string()).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn blk(peer: &str, n: u8) -> AdmissionPeerEvent {
        AdmissionPeerEvent::Block {
            peer: peer.to_string(),
            block_bytes: vec![n],
        }
    }

    fn peer_of(ev: &AdmissionPeerEvent) -> &str {
        event_peer(ev)
    }

    /// Run fair_merge to completion over the given lane contents (each inner Vec is
    /// one lane, in order) and return the merged output sequence.
    async fn run_merge(lane_contents: Vec<Vec<AdmissionPeerEvent>>) -> Vec<AdmissionPeerEvent> {
        let mut lanes = Vec::new();
        for content in lane_contents {
            let (tx, rx) = mpsc::channel(PER_PEER_LANE_CAP);
            for ev in content {
                tx.try_send(ev).expect("lane cap sized for the test");
            }
            drop(tx); // close the lane so the merge terminates
            lanes.push(Some(rx));
        }
        let (out_tx, mut out_rx) = mpsc::channel(1024);
        fair_merge(lanes, out_tx).await;
        let mut got = Vec::new();
        while let Ok(ev) = out_rx.try_recv() {
            got.push(ev);
        }
        got
    }

    #[tokio::test]
    async fn hot_peer_cannot_starve_quiet_peer() {
        // Lane 0 (hot) holds 10 blocks; lane 1 (quiet) holds 1. Round-robin must
        // surface the quiet peer's block EARLY (2nd item), not after all 10 hot ones.
        let hot: Vec<_> = (0..10).map(|i| blk("hot", i)).collect();
        let quiet = vec![blk("quiet", 99)];
        let out = run_merge(vec![hot, quiet]).await;
        assert_eq!(out.len(), 11, "every block delivered, none dropped");
        let quiet_pos = out.iter().position(|e| peer_of(e) == "quiet").unwrap();
        assert!(
            quiet_pos <= 1,
            "quiet peer must not be starved (delivered at index {quiet_pos}, expected <=1)"
        );
        let counts = per_peer_delivered_counts(&out);
        assert_eq!(counts["hot"], 10);
        assert_eq!(counts["quiet"], 1);
    }

    #[tokio::test]
    async fn per_peer_backpressure_not_global() {
        // A FULL lane backpressures only its own sender; a sibling lane still accepts.
        let (hot_tx, _hot_rx) = mpsc::channel::<AdmissionPeerEvent>(1);
        let (quiet_tx, _quiet_rx) = mpsc::channel::<AdmissionPeerEvent>(1);
        hot_tx.try_send(blk("hot", 0)).unwrap(); // fill the hot lane
        assert!(
            hot_tx.try_send(blk("hot", 1)).is_err(),
            "a full lane backpressures its OWN sender"
        );
        assert!(
            quiet_tx.try_send(blk("quiet", 0)).is_ok(),
            "a sibling lane is NOT blocked by the hot lane being full (no global starvation)"
        );
    }

    #[tokio::test]
    async fn peer_identity_preserved_through_merge() {
        let out = run_merge(vec![vec![blk("A", 1)], vec![blk("B", 2)]]).await;
        let counts = per_peer_delivered_counts(&out);
        assert_eq!(counts.get("A"), Some(&1));
        assert_eq!(counts.get("B"), Some(&1));
        // The peer label rides with the payload — no cross-attribution.
        for ev in &out {
            if let AdmissionPeerEvent::Block { peer, block_bytes } = ev {
                let expect = if peer == "A" { 1 } else { 2 };
                assert_eq!(block_bytes, &vec![expect], "peer {peer} kept its own payload");
            }
        }
    }

    #[tokio::test]
    async fn deterministic_peer_order_from_config() {
        // Same lane contents in the same order => byte-identical merged sequence.
        let mk = || vec![vec![blk("A", 1), blk("A", 2)], vec![blk("B", 1), blk("B", 2)]];
        let a = run_merge(mk()).await;
        let b = run_merge(mk()).await;
        let seq = |o: &[AdmissionPeerEvent]| {
            o.iter().map(|e| peer_of(e).to_string()).collect::<Vec<_>>()
        };
        assert_eq!(seq(&a), seq(&b), "merge order is deterministic for the same lanes");
        // Round-robin interleave: A,B,A,B (not A,A,B,B).
        assert_eq!(seq(&a), vec!["A", "B", "A", "B"]);
    }

    #[tokio::test]
    async fn closed_lane_removed_without_reordering_remaining_peers() {
        // 3 lanes; the MIDDLE one (index 1) is empty+closed. The remaining peers
        // (index 0 and 2) must still deliver, fairly and in their stable order.
        let out = run_merge(vec![
            vec![blk("p0", 1), blk("p0", 2)],
            vec![], // p1: closed with nothing
            vec![blk("p2", 1), blk("p2", 2)],
        ])
        .await;
        let seq: Vec<_> = out.iter().map(|e| peer_of(e).to_string()).collect();
        // p1 retired in place → round-robin over {p0, p2} stays p0,p2,p0,p2.
        assert_eq!(seq, vec!["p0", "p2", "p0", "p2"]);
        let counts = per_peer_delivered_counts(&out);
        assert_eq!(counts.get("p1"), None, "closed empty lane contributes nothing");
        assert_eq!(counts["p0"], 2);
        assert_eq!(counts["p2"], 2);
    }

    #[tokio::test]
    async fn single_peer_behaviour_unchanged() {
        // One lane → pure passthrough, in order (the pre-S8 single-peer behavior).
        let out = run_merge(vec![vec![blk("solo", 1), blk("solo", 2), blk("solo", 3)]]).await;
        let payloads: Vec<_> = out
            .iter()
            .map(|e| match e {
                AdmissionPeerEvent::Block { block_bytes, .. } => block_bytes[0],
                _ => 0,
            })
            .collect();
        assert_eq!(payloads, vec![1, 2, 3], "single lane is an in-order passthrough");
    }
}
