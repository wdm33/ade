// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED per-peer tokio task (PHASE4-N-K S4).
//!
//! Owns a `tokio::sync::mpsc::Receiver<PeerInboundFrame>` (frames
//! arrive from the socket layer) and forwards them to the
//! orchestrator's inbox as `OrchestratorEvent`s. One task per
//! connected peer; no shared mutable state across tasks.
//!
//! Per-peer isolation is enforced structurally:
//!   - Each task owns its own `mpsc::Receiver`.
//!   - Errors on `events_out.send()` (orchestrator dropped) exit
//!     this task only — sibling peer tasks are unaffected.
//!   - The orchestrator core's per-peer state map (S2) keys by
//!     `PeerId`; a `PeerSessionHalted` effect removes only that
//!     peer's entry.
//!
//! DC-NODE-01: enforced by `ci/ci_check_peer_session_isolation.sh`.

use tokio::sync::mpsc;

use super::event::{OrchestratorEvent, PeerId};

/// One inbound frame from a peer socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerInboundFrame {
    pub kind: PeerInboundFrameKind,
    pub bytes: Vec<u8>,
}

/// Closed sum of inbound frame kinds. The orchestrator dispatches
/// on this discriminant when translating into `OrchestratorEvent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerInboundFrameKind {
    /// Upstream peer's chain-sync server response (we are client).
    ChainSyncClient,
    /// Upstream peer's block-fetch server response (we are client).
    BlockFetchClient,
    /// Downstream peer's chain-sync client message (we are server).
    ChainSyncServer,
    /// Downstream peer's block-fetch client message (we are server).
    BlockFetchServer,
}

/// Per-peer task. Drives one `mpsc::Receiver<PeerInboundFrame>`,
/// produces `OrchestratorEvent`s on the orchestrator inbox channel.
pub struct PeerSession {
    pub peer_id: PeerId,
    pub inbound: mpsc::Receiver<PeerInboundFrame>,
    pub events_out: mpsc::Sender<OrchestratorEvent>,
}

impl PeerSession {
    /// Run the session loop. Exits cleanly when:
    ///   - `inbound` closes (peer socket dropped → `recv()` returns `None`); OR
    ///   - `events_out` closes (orchestrator dropped → `send()` errors).
    pub async fn run(mut self) {
        while let Some(frame) = self.inbound.recv().await {
            let event = match frame.kind {
                PeerInboundFrameKind::ChainSyncClient => {
                    OrchestratorEvent::PeerChainSyncFrame {
                        peer_id: self.peer_id,
                        bytes: frame.bytes,
                    }
                }
                PeerInboundFrameKind::BlockFetchClient => {
                    OrchestratorEvent::PeerBlockFetchFrame {
                        peer_id: self.peer_id,
                        bytes: frame.bytes,
                    }
                }
                PeerInboundFrameKind::ChainSyncServer => {
                    OrchestratorEvent::PeerN2nServerChainSyncFrame {
                        peer_id: self.peer_id,
                        bytes: frame.bytes,
                    }
                }
                PeerInboundFrameKind::BlockFetchServer => {
                    OrchestratorEvent::PeerN2nServerBlockFetchFrame {
                        peer_id: self.peer_id,
                        bytes: frame.bytes,
                    }
                }
            };
            if self.events_out.send(event).await.is_err() {
                // Orchestrator dropped; nothing more to do.
                break;
            }
        }
        // Notify orchestrator the peer has disconnected. If the
        // orchestrator is already gone, swallow the error.
        let _ = self
            .events_out
            .send(OrchestratorEvent::PeerDisconnected {
                peer_id: self.peer_id,
            })
            .await;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn peer_session_routes_chain_sync_to_orchestrator_event() {
        let (inbound_tx, inbound_rx) = mpsc::channel(4);
        let (events_tx, mut events_rx) = mpsc::channel(4);
        let session = PeerSession {
            peer_id: PeerId(1),
            inbound: inbound_rx,
            events_out: events_tx,
        };
        let handle = tokio::spawn(session.run());

        inbound_tx
            .send(PeerInboundFrame {
                kind: PeerInboundFrameKind::ChainSyncClient,
                bytes: vec![0x01, 0x02, 0x03],
            })
            .await
            .expect("send");
        drop(inbound_tx);

        let ev = events_rx.recv().await.expect("event");
        match ev {
            OrchestratorEvent::PeerChainSyncFrame { peer_id, bytes } => {
                assert_eq!(peer_id, PeerId(1));
                assert_eq!(bytes, vec![0x01, 0x02, 0x03]);
            }
            other => panic!("expected PeerChainSyncFrame, got {other:?}"),
        }
        // Disconnect notification on inbound close.
        let disc = events_rx.recv().await.expect("disconnect");
        assert!(matches!(disc, OrchestratorEvent::PeerDisconnected { peer_id } if peer_id == PeerId(1)));
        handle.await.expect("join");
    }

    #[tokio::test]
    async fn peer_session_isolation_across_two_concurrent_tasks() {
        let (inbound_a, rx_a) = mpsc::channel(4);
        let (inbound_b, rx_b) = mpsc::channel(4);
        let (events_tx, mut events_rx) = mpsc::channel(16);

        let session_a = PeerSession {
            peer_id: PeerId(1),
            inbound: rx_a,
            events_out: events_tx.clone(),
        };
        let session_b = PeerSession {
            peer_id: PeerId(2),
            inbound: rx_b,
            events_out: events_tx.clone(),
        };
        drop(events_tx); // sole producers are the sessions

        let h_a = tokio::spawn(session_a.run());
        let h_b = tokio::spawn(session_b.run());

        // Peer A "fails" by dropping its inbound channel (socket
        // dropped). Peer B sends a frame.
        drop(inbound_a);
        inbound_b
            .send(PeerInboundFrame {
                kind: PeerInboundFrameKind::ChainSyncServer,
                bytes: vec![0xAA],
            })
            .await
            .expect("send");
        drop(inbound_b);

        let mut seen_a_disc = false;
        let mut seen_b_frame = false;
        let mut seen_b_disc = false;
        while let Some(ev) = events_rx.recv().await {
            match ev {
                OrchestratorEvent::PeerDisconnected { peer_id: PeerId(1) } => {
                    seen_a_disc = true;
                }
                OrchestratorEvent::PeerN2nServerChainSyncFrame {
                    peer_id: PeerId(2),
                    bytes,
                } => {
                    assert_eq!(bytes, vec![0xAA]);
                    seen_b_frame = true;
                }
                OrchestratorEvent::PeerDisconnected { peer_id: PeerId(2) } => {
                    seen_b_disc = true;
                }
                _ => {}
            }
        }
        h_a.await.expect("join a");
        h_b.await.expect("join b");
        assert!(seen_a_disc, "peer A disconnect must arrive");
        assert!(seen_b_frame, "peer B frame must arrive");
        assert!(seen_b_disc, "peer B disconnect must arrive");
    }

    #[tokio::test]
    async fn peer_session_exits_when_orchestrator_drops() {
        let (inbound_tx, inbound_rx) = mpsc::channel(4);
        let (events_tx, events_rx) = mpsc::channel(1);
        // Drop the consumer immediately.
        drop(events_rx);
        let session = PeerSession {
            peer_id: PeerId(42),
            inbound: inbound_rx,
            events_out: events_tx,
        };
        let handle = tokio::spawn(session.run());
        inbound_tx
            .send(PeerInboundFrame {
                kind: PeerInboundFrameKind::ChainSyncClient,
                bytes: vec![0xFF],
            })
            .await
            .expect("send");
        // The session sees send-error and exits.
        handle.await.expect("join");
    }
}
