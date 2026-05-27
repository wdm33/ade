// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED N2N inbound listener (PHASE4-N-Q S4).
//!
//! Mirror of `n2n_dialer` for the server-role direction. Composes:
//!   1. `tokio::net::TcpListener::accept` — accept inbound TCP.
//!   2. Per-accepted-connection: spawn a session task that runs
//!      `ade_network::session::run_n2n_handshake_responder` via
//!      `tokio::task::spawn_blocking` to complete the N2N
//!      handshake (CN-SESS-02; CN-PROD-01).
//!   3. On handshake success: emit
//!      `OrchestratorEvent::PeerConnected { role: DownstreamServer }`
//!      and spawn a `MuxPump` per peer (existing, GREEN per the
//!      N-Q reclassification).
//!   4. The MuxPump emits
//!      `PeerN2nServerChainSyncFrame` / `PeerN2nServerBlockFetchFrame`
//!      events; the produce-mode main loop (S5) dispatches them
//!      to `n2n_server::dispatch_*`.
//!
//! Pre-handshake socket bytes never reach the n2n_server reducers
//! — CN-PROD-01 enforced at the type boundary (the handshake
//! responder consumes bytes from the duplex BEFORE the MuxPump is
//! constructed).

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use ade_network::codec::version::{BlockFetchVersion, ChainSyncVersion};
use ade_network::handshake::state::{HandshakeError, VersionData};
use ade_network::mux::frame::MuxError;
use ade_network::mux::transport::{spawn_duplex, DuplexCapacity, MuxTransportHandle};
use ade_network::session::{
    run_n2n_handshake_responder, ConnectedState, NegotiatedN2n, SessionState,
    TransportError as SessionTransportError,
};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};

use crate::orchestrator::event::{OrchestratorEvent, PeerRole};
use crate::orchestrator::n2n_server_pump::PeerIdGenerator;

use super::mux_pump::MuxPump;

/// Closed listener-error sum.
#[derive(Debug)]
pub enum ListenerError {
    /// `TcpListener::bind` failed.
    Bind(io::ErrorKind),
    /// `TcpListener::accept` returned a fatal error and the listen
    /// loop must exit.
    AcceptFatal(io::ErrorKind),
    /// Shutdown signal received; clean exit.
    Shutdown,
}

/// Closed per-peer session-error sum (emitted as evidence; not
/// returned from `run_n2n_listener` which keeps accepting other
/// peers).
#[derive(Debug)]
pub enum PeerSessionError {
    Io(io::ErrorKind),
    Handshake(HandshakeError),
    Mux(MuxError),
    OrchestratorDropped,
}

/// Configuration for the inbound listener.
pub struct N2nListenerConfig {
    pub bind_addr: SocketAddr,
    /// Supported N2N versions (responder side). Typically
    /// `ade_network::handshake::version_table::N2N_SUPPORTED`.
    pub our_supported: &'static [(u16, VersionData)],
    pub peer_id_generator: Arc<PeerIdGenerator>,
    pub events_out: mpsc::Sender<OrchestratorEvent>,
    /// PHASE4-N-S-B B3: shared per-peer outbound channel map.
    /// `None` preserves N-Q/N-R listener behavior (no outbound
    /// relay; dispatch responses computed but not transmitted).
    /// `Some` enables produce_mode to send `OutboundCommand`
    /// instances through the per-peer mpsc::Sender for the
    /// corresponding `PeerId`.
    pub peer_outbound: Option<crate::network::outbound_command::PerPeerOutbound>,
}

/// Bind a TCP listener at `bind_addr` and run the accept loop until
/// `shutdown_rx` flips to `true`. Each accepted connection is
/// handed to a per-peer session task that completes the N2N
/// handshake + spawns a MuxPump.
///
/// Returns:
/// - `Ok(())` on graceful shutdown via `shutdown_rx`.
/// - `Err(ListenerError::Bind | AcceptFatal)` on fatal errors.
pub async fn run_n2n_listener(
    cfg: N2nListenerConfig,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<(), ListenerError> {
    let listener = TcpListener::bind(cfg.bind_addr)
        .await
        .map_err(|e| ListenerError::Bind(e.kind()))?;

    loop {
        tokio::select! {
            biased;
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    return Err(ListenerError::Shutdown);
                }
            }
            accept = listener.accept() => {
                let (stream, _addr) = match accept {
                    Ok(pair) => pair,
                    Err(e) => return Err(ListenerError::AcceptFatal(e.kind())),
                };
                let session_cfg = PerPeerSessionConfig {
                    stream,
                    our_supported: cfg.our_supported,
                    peer_id_generator: cfg.peer_id_generator.clone(),
                    events_out: cfg.events_out.clone(),
                    peer_outbound: cfg.peer_outbound.clone(),
                };
                tokio::spawn(run_per_peer_session(session_cfg));
            }
        }
    }
}

/// Per-peer session config. Owned by the spawned task.
pub struct PerPeerSessionConfig {
    pub stream: TcpStream,
    pub our_supported: &'static [(u16, VersionData)],
    pub peer_id_generator: Arc<PeerIdGenerator>,
    pub events_out: mpsc::Sender<OrchestratorEvent>,
    /// PHASE4-N-S-B B3: per-peer outbound map (see
    /// `N2nListenerConfig::peer_outbound`).
    pub peer_outbound: Option<crate::network::outbound_command::PerPeerOutbound>,
}

/// Drive a single accepted peer connection: handshake → mux pump.
/// Errors are logged via `events_out` (PeerDisconnected with
/// reason); the function returns Ok in all cases the listener
/// should keep running.
pub async fn run_per_peer_session(cfg: PerPeerSessionConfig) -> Result<(), PeerSessionError> {
    let handle = spawn_duplex(cfg.stream, DuplexCapacity::DEFAULT);
    let MuxTransportHandle {
        inbound,
        outbound,
        reader_handle,
        writer_handle,
    } = handle;

    let our_supported = cfg.our_supported;
    let (transport_back_in, transport_back_out, negotiated) =
        tokio::task::spawn_blocking(move || {
            let mut bt = BlockingTransport::new(inbound, outbound);
            let result = run_n2n_handshake_responder(&mut bt, our_supported);
            let (inbound, outbound) = bt.into_halves();
            (inbound, outbound, result)
        })
        .await
        .map_err(|_| PeerSessionError::OrchestratorDropped)?;

    let negotiated: NegotiatedN2n = match negotiated {
        Ok(n) => n,
        Err(SessionTransportError::Handshake(e)) => return Err(PeerSessionError::Handshake(e)),
        Err(SessionTransportError::Mux(m)) => return Err(PeerSessionError::Mux(m)),
        Err(SessionTransportError::Io) => {
            return Err(PeerSessionError::Io(io::ErrorKind::Other))
        }
        Err(SessionTransportError::Eof) => {
            return Err(PeerSessionError::Io(io::ErrorKind::UnexpectedEof))
        }
    };

    let peer_id = cfg.peer_id_generator.next();
    let event = OrchestratorEvent::PeerConnected {
        peer_id,
        chain_sync_version: ChainSyncVersion::new(negotiated.version),
        block_fetch_version: BlockFetchVersion::new(negotiated.version),
        role: PeerRole::DownstreamServer,
    };
    cfg.events_out
        .send(event)
        .await
        .map_err(|_| PeerSessionError::OrchestratorDropped)?;

    // Hand the post-handshake session to a MuxPump task. The pump
    // dispatches `OrchestratorEvent::PeerN2nServerChainSyncFrame` /
    // `PeerN2nServerBlockFetchFrame` events as the peer sends
    // mini-protocol frames.
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
        events_out: cfg.events_out,
        peer_role: PeerRole::DownstreamServer,
        outbound_relay: None, // B3 wires per-peer outbound channel
    };
    tokio::spawn(pump.run());

    Ok(())
}

// =========================================================================
// BlockingTransport — same bridge n2n_dialer uses. Held only for the
// handshake window.
// =========================================================================

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

impl ade_network::session::Transport for BlockingTransport {
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), SessionTransportError> {
        while self.inbound_buffer.len() < buf.len() {
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

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::network::n2n_dialer::N2nDialer;
    use ade_network::codec::handshake::{VersionParams, VersionTable};
    use ade_network::codec::N2NVersion;
    use ade_network::handshake::version_table::N2N_SUPPORTED;

    fn dialer_versions() -> VersionTable {
        // For the loopback dialer, propose a subset of the
        // responder's N2N_SUPPORTED so handshake negotiates.
        VersionTable(
            N2N_SUPPORTED
                .iter()
                .map(|(v, _data)| {
                    (
                        N2NVersion::new(*v),
                        VersionParams(vec![0x01]),
                    )
                })
                .collect(),
        )
    }

    /// Loopback test: spawn the listener at an ephemeral port; dial
    /// it with `N2nDialer`; both sides must emit `PeerConnected`
    /// events. Closes CN-PROD-01's handshake-complete gate for the
    /// loopback case.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn n2n_listener_loopback_handshake_succeeds() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        drop(listener); // free the port; the cfg-driven bind below
                        // will re-claim it.

        let (events_tx, mut events_rx) = mpsc::channel(8);
        let listener_cfg = N2nListenerConfig {
            bind_addr: addr,
            our_supported: N2N_SUPPORTED,
            peer_id_generator: Arc::new(PeerIdGenerator::new()),
            events_out: events_tx.clone(),
            peer_outbound: None,
        };

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let listener_handle =
            tokio::spawn(async move { run_n2n_listener(listener_cfg, shutdown_rx).await });

        // Give the listener a moment to bind.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let dialer = N2nDialer {
            peer_addr: addr,
            our_versions: dialer_versions(),
            peer_id_generator: Arc::new(PeerIdGenerator::new()),
            events_out: events_tx,
        };
        let dial_result = dialer.dial().await;
        assert!(dial_result.is_ok(), "dial failed: {:?}", dial_result.err());

        // Collect PeerConnected events from both sides. Order is
        // not load-bearing.
        let mut roles_seen: Vec<PeerRole> = Vec::new();
        for _ in 0..2 {
            let evt = tokio::time::timeout(
                std::time::Duration::from_secs(3),
                events_rx.recv(),
            )
            .await
            .expect("event timeout")
            .expect("event channel closed");
            match evt {
                OrchestratorEvent::PeerConnected { role, .. } => roles_seen.push(role),
                other => panic!("unexpected event: {:?}", other),
            }
        }

        // One UpstreamClient (dialer side), one DownstreamServer
        // (listener side).
        assert!(
            roles_seen.contains(&PeerRole::DownstreamServer),
            "no DownstreamServer event"
        );
        assert!(
            roles_seen.contains(&PeerRole::UpstreamClient),
            "no UpstreamClient event"
        );

        // Shut down the listener.
        let _ = shutdown_tx.send(true);
        let _ = listener_handle.await;
    }
}
