// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED N2N outbound dialer (PHASE4-N-L S7).
//!
//! Composes:
//!   1. `tokio::net::TcpStream::connect` — TCP dial.
//!   2. `ade_network::session::handshake_driver::run_n2n_handshake_initiator`
//!      — sync handshake state-machine driver (CN-SESS-02), bridged
//!      to async via `tokio::task::spawn_blocking` so the sync
//!      transport trait survives.
//!   3. `MuxPump` (S6) — drives the post-handshake session.
//!   4. Emits `OrchestratorEvent::PeerConnected { role: UpstreamClient }`
//!      to the orchestrator inbox.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use ade_network::codec::handshake::VersionTable;
use ade_network::codec::version::{BlockFetchVersion, ChainSyncVersion};
use ade_network::handshake::state::HandshakeError;
use ade_network::mux::transport::{
    spawn_duplex, DuplexCapacity, MuxTransportHandle, TransportError,
};
use ade_network::session::{
    run_n2n_handshake_initiator, ConnectedState, NegotiatedN2n, SessionState, Transport,
    TransportError as SessionTransportError,
};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::orchestrator::event::{OrchestratorEvent, PeerId, PeerRole};
use crate::orchestrator::n2n_server_pump::PeerIdGenerator;

use super::mux_pump::MuxPump;

/// Closed dial-error sum.
#[derive(Debug)]
pub enum DialError {
    Io(io::ErrorKind),
    Handshake(HandshakeError),
    Transport(TransportError),
    OrchestratorDropped,
}

/// Outbound dialer. Resolves a peer address, performs the
/// handshake, and spawns the per-connection pump.
pub struct N2nDialer {
    pub peer_addr: SocketAddr,
    pub our_versions: VersionTable,
    pub peer_id_generator: Arc<PeerIdGenerator>,
    pub events_out: mpsc::Sender<OrchestratorEvent>,
}

impl N2nDialer {
    pub async fn dial(self) -> Result<PeerId, DialError> {
        let stream = TcpStream::connect(self.peer_addr)
            .await
            .map_err(|e| DialError::Io(e.kind()))?;
        let handle = spawn_duplex(stream, DuplexCapacity::DEFAULT);
        let MuxTransportHandle {
            inbound,
            outbound,
            reader_handle,
            writer_handle,
        } = handle;

        // The handshake driver is sync over `Transport`. We bridge to
        // async by running it inside a blocking task. The
        // `BlockingTransport` adapter owns the inbound + outbound
        // channel halves and runs `recv`/`send` synchronously via
        // a small bounded buffer; this only spans the handshake
        // window (one round-trip), so the blocking-pool footprint
        // is bounded.
        let our_versions = self.our_versions;
        let (transport_back_in, transport_back_out, negotiated) =
            tokio::task::spawn_blocking(move || {
                let mut bt = BlockingTransport::new(inbound, outbound);
                let result = run_n2n_handshake_initiator(&mut bt, our_versions);
                let (inbound, outbound) = bt.into_halves();
                (inbound, outbound, result)
            })
            .await
            .map_err(|_| DialError::OrchestratorDropped)?;

        let negotiated: NegotiatedN2n = match negotiated {
            Ok(n) => n,
            Err(SessionTransportError::Handshake(e)) => return Err(DialError::Handshake(e)),
            Err(SessionTransportError::Mux(_)) => {
                return Err(DialError::Handshake(HandshakeError::MalformedMessage {
                    reason: "mux decode error during handshake",
                }))
            }
            Err(SessionTransportError::Io) => return Err(DialError::Io(io::ErrorKind::Other)),
            Err(SessionTransportError::Eof) => return Err(DialError::Io(io::ErrorKind::UnexpectedEof)),
        };

        let peer_id = self.peer_id_generator.next();
        let event = OrchestratorEvent::PeerConnected {
            peer_id,
            chain_sync_version: ChainSyncVersion::new(negotiated.version),
            block_fetch_version: BlockFetchVersion::new(negotiated.version),
            role: PeerRole::UpstreamClient,
        };
        if self.events_out.send(event).await.is_err() {
            return Err(DialError::OrchestratorDropped);
        }

        // Hand the post-handshake session to a MuxPump task.
        let session_state = SessionState::Connected(ConnectedState::new(
            negotiated.version,
            negotiated.params,
        ));
        let pump = MuxPump {
            peer_id,
            transport: MuxTransportHandle {
                inbound: transport_back_in,
                outbound: transport_back_out,
                reader_handle,
                writer_handle,
            },
            session_state,
            events_out: self.events_out,
            peer_role: PeerRole::UpstreamClient,
            outbound_relay: None,
        };
        tokio::spawn(pump.run());

        Ok(peer_id)
    }
}

/// Sync transport bridge over the duplex transport's bounded
/// channels. Held only for the handshake window (single
/// round-trip).
struct BlockingTransport {
    inbound: mpsc::Receiver<Vec<u8>>,
    outbound: mpsc::Sender<Vec<u8>>,
    inbound_buffer: Vec<u8>,
}

impl BlockingTransport {
    fn new(inbound: mpsc::Receiver<Vec<u8>>, outbound: mpsc::Sender<Vec<u8>>) -> Self {
        Self {
            inbound,
            outbound,
            inbound_buffer: Vec::new(),
        }
    }

    fn into_halves(self) -> (mpsc::Receiver<Vec<u8>>, mpsc::Sender<Vec<u8>>) {
        (self.inbound, self.outbound)
    }
}

impl Transport for BlockingTransport {
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), SessionTransportError> {
        while self.inbound_buffer.len() < buf.len() {
            // blocking_recv requires being inside a spawn_blocking task,
            // which is exactly where the dialer calls this method.
            match self.inbound.blocking_recv() {
                Some(chunk) => self.inbound_buffer.extend_from_slice(&chunk),
                None => return Err(SessionTransportError::Eof),
            }
        }
        let drained: Vec<u8> = self.inbound_buffer.drain(..buf.len()).collect();
        buf.copy_from_slice(&drained);
        Ok(())
    }

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), SessionTransportError> {
        self.outbound
            .blocking_send(bytes.to_vec())
            .map_err(|_| SessionTransportError::Io)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use ade_network::codec::handshake::VersionParams;
    use ade_network::codec::N2NVersion;
    use ade_network::handshake::version_table::N2N_SUPPORTED;
    use ade_network::session::run_n2n_handshake_responder;
    use tokio::net::TcpListener;

    fn versions_14_to_16() -> VersionTable {
        VersionTable(vec![
            (N2NVersion::new(14), VersionParams(vec![0x01])),
            (N2NVersion::new(15), VersionParams(vec![0x01])),
            (N2NVersion::new(16), VersionParams(vec![0x01])),
        ])
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn n2n_dialer_loopback_handshake_succeeds() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");

        // Responder task: accept one connection, run the responder
        // handshake to completion using the project's N2N_SUPPORTED.
        let responder = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let handle = spawn_duplex(stream, DuplexCapacity::DEFAULT);
            let MuxTransportHandle {
                inbound, outbound, ..
            } = handle;
            tokio::task::spawn_blocking(move || {
                let mut bt = BlockingTransport::new(inbound, outbound);
                run_n2n_handshake_responder(&mut bt, N2N_SUPPORTED)
            })
            .await
            .expect("spawn_blocking")
        });

        let (events_tx, mut events_rx) = mpsc::channel(8);
        let dialer = N2nDialer {
            peer_addr: addr,
            our_versions: versions_14_to_16(),
            peer_id_generator: Arc::new(PeerIdGenerator::new()),
            events_out: events_tx,
        };
        let peer_id = dialer.dial().await.expect("dial");
        let resp_result = responder.await.expect("responder join");
        assert!(resp_result.is_ok(), "responder must accept");

        let ev = events_rx.recv().await.expect("event");
        match ev {
            OrchestratorEvent::PeerConnected {
                peer_id: pid,
                role,
                ..
            } => {
                assert_eq!(pid, peer_id);
                assert!(matches!(role, PeerRole::UpstreamClient));
            }
            other => panic!("expected PeerConnected, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn n2n_dialer_returns_io_error_on_unreachable_target() {
        // Bind, then immediately close the listener — TCP connect on
        // a now-closed port should error.
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        drop(listener);

        let (events_tx, _events_rx) = mpsc::channel(4);
        let dialer = N2nDialer {
            peer_addr: addr,
            our_versions: versions_14_to_16(),
            peer_id_generator: Arc::new(PeerIdGenerator::new()),
            events_out: events_tx,
        };
        match dialer.dial().await {
            Err(DialError::Io(_)) => {}
            other => panic!("expected DialError::Io, got {other:?}"),
        }
    }
}
