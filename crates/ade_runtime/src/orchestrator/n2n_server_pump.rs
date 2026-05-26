// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED N2N listening-socket pump (PHASE4-N-K S6).
//!
//! Accepts incoming TCP connections on a bound `TcpListener`. For
//! each accepted connection, allocates a deterministic
//! `PeerId` via a shared monotonic [`PeerIdGenerator`], emits a
//! `PeerConnected { role: DownstreamServer, ... }` event to the
//! orchestrator inbox, and structurally spawns a per-peer session
//! task. The actual Ouroboros mux + frame parsing above
//! `MuxTransport::read_raw` / `write_raw` is operator-action work
//! tracked by RO-LIVE-01 / RO-LIVE-02; this pump's correctness
//! contract is the spawn-per-connection isolation, not the wire
//! frame parsing.
//!
//! The pump matches the project's existing live-binary discipline
//! (`live_block_follow_session`): the orchestrator core is real,
//! the mux session driver is operator-action work.

use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use ade_network::codec::version::{BlockFetchVersion, ChainSyncVersion};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use super::event::{OrchestratorEvent, PeerId, PeerRole};
use super::peer_session::{PeerInboundFrame, PeerSession};

/// Monotonic peer-id allocator. Shared across the pump and any
/// other component that allocates peer ids (in practice only the
/// pump itself). Per-id allocation is deterministic at the value
/// level: the n-th accepted connection always gets id n.
pub struct PeerIdGenerator {
    counter: AtomicU64,
}

impl PeerIdGenerator {
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(1),
        }
    }

    pub fn next(&self) -> PeerId {
        PeerId(self.counter.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for PeerIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// N2N server pump. Owns the bound `TcpListener` and the
/// orchestrator-inbox sender; allocates a `mpsc::Sender` per
/// accepted connection so the peer session can forward inbound
/// frames the moment the mux session driver is wired.
pub struct N2nServerPump {
    pub listener: TcpListener,
    pub events_out: mpsc::Sender<OrchestratorEvent>,
    pub next_peer_id: Arc<PeerIdGenerator>,
    /// Negotiated version defaults; the production runner can pin
    /// these per accepted connection. Operator-action layer.
    pub default_chain_sync_version: ChainSyncVersion,
    pub default_block_fetch_version: BlockFetchVersion,
}

impl N2nServerPump {
    /// Drive the listening loop. For each accepted connection,
    /// spawn a per-peer session and notify the orchestrator. Exits
    /// on listener error (port closed, etc.).
    pub async fn run(self) -> io::Result<()> {
        loop {
            let (_stream, _addr) = self.listener.accept().await?;
            let peer_id = self.next_peer_id.next();
            let (inbound_tx, inbound_rx) = mpsc::channel::<PeerInboundFrame>(32);
            let session = PeerSession {
                peer_id,
                inbound: inbound_rx,
                events_out: self.events_out.clone(),
            };
            // The session task owns the per-connection channel; the
            // mux driver (operator-action) will fill `inbound_tx`
            // with parsed frames once the wire layer above
            // `MuxTransport` is wired. Until then the session
            // sits quiescent and exits when this end of the channel
            // drops.
            tokio::spawn(session.run());
            // The per-connection inbound sender is intentionally
            // dropped here; the operator wires the actual frame
            // parser. Mechanical evidence covers the per-peer
            // isolation; live wire evidence is RO-LIVE-01 /
            // RO-LIVE-02.
            drop(inbound_tx);

            let event = OrchestratorEvent::PeerConnected {
                peer_id,
                chain_sync_version: self.default_chain_sync_version,
                block_fetch_version: self.default_block_fetch_version,
                role: PeerRole::DownstreamServer,
            };
            if self.events_out.send(event).await.is_err() {
                return Ok(());
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use tokio::net::TcpStream;

    #[test]
    fn peer_id_generator_is_monotonic_and_deterministic_per_seed() {
        let ids_gen = PeerIdGenerator::new();
        let ids: Vec<PeerId> = (0..5).map(|_| ids_gen.next()).collect();
        assert_eq!(
            ids,
            vec![PeerId(1), PeerId(2), PeerId(3), PeerId(4), PeerId(5)]
        );
    }

    #[tokio::test]
    async fn n2n_server_pump_spawns_per_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");

        let (events_tx, mut events_rx) = mpsc::channel(8);
        let pump = N2nServerPump {
            listener,
            events_out: events_tx,
            next_peer_id: Arc::new(PeerIdGenerator::new()),
            default_chain_sync_version: ChainSyncVersion::new(9),
            default_block_fetch_version: BlockFetchVersion::new(9),
        };
        let pump_handle = tokio::spawn(pump.run());

        // Open two connections.
        let _c1 = TcpStream::connect(addr).await.expect("connect 1");
        let _c2 = TcpStream::connect(addr).await.expect("connect 2");

        // Expect two PeerConnected events with distinct ids.
        let ev1 = events_rx.recv().await.expect("conn 1");
        let ev2 = events_rx.recv().await.expect("conn 2");
        let (id_a, role_a) = match ev1 {
            OrchestratorEvent::PeerConnected { peer_id, role, .. } => (peer_id, role),
            other => panic!("expected PeerConnected, got {other:?}"),
        };
        let (id_b, role_b) = match ev2 {
            OrchestratorEvent::PeerConnected { peer_id, role, .. } => (peer_id, role),
            other => panic!("expected PeerConnected, got {other:?}"),
        };
        assert_ne!(id_a, id_b, "per-connection peer ids must differ");
        assert!(matches!(role_a, PeerRole::DownstreamServer));
        assert!(matches!(role_b, PeerRole::DownstreamServer));

        pump_handle.abort();
    }

    #[tokio::test]
    async fn n2n_server_pump_exits_when_orchestrator_drops() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let (events_tx, events_rx) = mpsc::channel(1);
        let pump = N2nServerPump {
            listener,
            events_out: events_tx,
            next_peer_id: Arc::new(PeerIdGenerator::new()),
            default_chain_sync_version: ChainSyncVersion::new(9),
            default_block_fetch_version: BlockFetchVersion::new(9),
        };
        let handle = tokio::spawn(pump.run());
        drop(events_rx);
        let _ = TcpStream::connect(addr).await.expect("connect");
        // Pump's send fails; it returns Ok(()) and exits cleanly.
        let result = handle.await.expect("join");
        assert!(result.is_ok());
    }
}
