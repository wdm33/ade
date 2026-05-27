// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED per-connection mux pump (PHASE4-N-L S6).
//!
//! Bridges a `MuxTransportHandle` (S5 full-duplex transport) to the
//! GREEN session reducer (`session::core::step`, S2). One tokio
//! task per connected peer. The pump:
//!   - reads inbound byte chunks from `transport.inbound`,
//!   - feeds each chunk to `session::core::step`,
//!   - forwards `SessionEffect::SendBytes` to `transport.outbound`,
//!   - lifts `SessionEffect::DeliverPeerFrame` into the matching
//!     `OrchestratorEvent::Peer{ChainSync,BlockFetch,...}Frame`
//!     and forwards it to the orchestrator inbox,
//!   - emits `OrchestratorEvent::PeerConnected` on
//!     `SessionEffect::HandshakeComplete`,
//!   - on `SessionError` or transport error, emits
//!     `OrchestratorEvent::PeerDisconnected` and exits.
//!
//! Per-peer isolation: each pump owns its own `MuxTransportHandle`
//! + `SessionState`; no shared mutable state across pumps.

use ade_network::codec::block_fetch::encode_block_fetch_message;
use ade_network::codec::chain_sync::encode_chain_sync_message;
use ade_network::mux::frame::MuxMode;
use ade_network::mux::transport::{MuxTransportHandle, TransportError};
use ade_network::session::{
    step, AcceptedMiniProtocol, ByteChunkIn, SessionEffect, SessionError, SessionState,
};
use tokio::sync::mpsc;

use crate::network::outbound_command::{CloseReason, OutboundCommand};
use crate::orchestrator::event::{
    OrchestratorEvent, PeerHaltReason, PeerId, PeerRole,
};

/// Per-connection mux pump. Owns transport + session state.
pub struct MuxPump {
    pub peer_id: PeerId,
    pub transport: MuxTransportHandle,
    pub session_state: SessionState,
    pub events_out: mpsc::Sender<OrchestratorEvent>,
    pub peer_role: PeerRole,
    /// PHASE4-N-S-B: per-peer outbound command receiver.
    /// `None` in dialer mode (existing behavior preserved).
    /// `Some` in server mode after the listener creates a
    /// per-peer mpsc::channel and registers the sender side
    /// in `PerPeerOutbound`.
    pub outbound_relay: Option<mpsc::Receiver<OutboundCommand>>,
}

impl MuxPump {
    /// Drive the pump until inbound EOF, session-fatal error, or
    /// orchestrator drop.
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                chunk = self.transport.inbound.recv() => {
                    match chunk {
                        Some(c) => {
                            let effects = match step(&mut self.session_state, ByteChunkIn::Inbound(c)) {
                                Ok(e) => e,
                                Err(err) => {
                                    let _ = self
                                        .emit_peer_disconnected(Some(session_err_to_halt(&err)))
                                        .await;
                                    return;
                                }
                            };
                            for effect in effects {
                                if !self.route_effect(effect).await {
                                    return;
                                }
                            }
                        }
                        None => {
                            // Inbound channel closed → reader task exited.
                            let reader_handle = std::mem::replace(
                                &mut self.transport.reader_handle,
                                tokio::spawn(async { Ok(()) }),
                            );
                            let reason = match reader_handle.await {
                                Ok(Ok(())) | Ok(Err(TransportError::Eof)) => None,
                                Ok(Err(TransportError::BackpressureExceeded)) => {
                                    Some(PeerHaltReason::ChainSyncDecodeError)
                                }
                                Ok(Err(TransportError::Io(_))) => Some(PeerHaltReason::ChainSyncDecodeError),
                                Err(_) => Some(PeerHaltReason::ChainSyncDecodeError),
                            };
                            let _ = self.emit_peer_disconnected(reason).await;
                            return;
                        }
                    }
                }
                cmd = async {
                    match self.outbound_relay.as_mut() {
                        Some(rx) => rx.recv().await,
                        None => std::future::pending().await,
                    }
                } => {
                    match cmd {
                        Some(c) => {
                            if !self.handle_outbound_command(c).await {
                                return;
                            }
                        }
                        None => {
                            // outbound_relay sender side dropped — treat
                            // as orchestrator-driven shutdown signal.
                            let _ = self.emit_peer_disconnected(None).await;
                            return;
                        }
                    }
                }
            }
        }
    }

    /// PHASE4-N-S-B: handle one OutboundCommand by encoding the
    /// typed reply via the existing codec, driving session::step
    /// with `ByteChunkIn::OutboundFrame` (which wraps the mux
    /// frame), then routing the resulting `SendBytes` via
    /// `route_effect`. The session-aware encoder is the sole
    /// producer of wire-byte streams — `produce_mode` cannot
    /// bypass this path.
    ///
    /// Returns `false` if the pump should exit (graceful close
    /// or unrecoverable failure).
    async fn handle_outbound_command(&mut self, cmd: OutboundCommand) -> bool {
        // Cross-peer leakage gate (CN-PEER-OUTBOUND-MAP-01).
        if cmd.peer() != self.peer_id {
            // Reject silently — the sender targeted a different
            // peer. Should be unreachable if PerPeerOutbound is
            // consistent.
            return true;
        }
        match cmd {
            OutboundCommand::ChainSync { reply, .. } => {
                let msg = reply.into_message();
                let payload = encode_chain_sync_message(&msg);
                self.dispatch_outbound_frame(AcceptedMiniProtocol::ChainSync, payload).await
            }
            OutboundCommand::BlockFetch { reply, .. } => {
                let msg = reply.into_message();
                let payload = encode_block_fetch_message(&msg);
                self.dispatch_outbound_frame(AcceptedMiniProtocol::BlockFetch, payload).await
            }
            OutboundCommand::ClosePeer { reason, .. } => {
                let halt = match reason {
                    CloseReason::Graceful => None,
                    CloseReason::ProtocolViolation => Some(PeerHaltReason::ChainSyncDecodeError),
                };
                let _ = self.emit_peer_disconnected(halt).await;
                false
            }
        }
    }

    async fn dispatch_outbound_frame(
        &mut self,
        mini_protocol: AcceptedMiniProtocol,
        payload: Vec<u8>,
    ) -> bool {
        let mode = match self.peer_role {
            PeerRole::UpstreamClient => MuxMode::Initiator,
            PeerRole::DownstreamServer => MuxMode::Responder,
        };
        let effects = match step(
            &mut self.session_state,
            ByteChunkIn::OutboundFrame {
                mini_protocol,
                payload,
                mode,
                timestamp: 0,
            },
        ) {
            Ok(e) => e,
            Err(err) => {
                let _ = self
                    .emit_peer_disconnected(Some(session_err_to_halt(&err)))
                    .await;
                return false;
            }
        };
        for effect in effects {
            if !self.route_effect(effect).await {
                return false;
            }
        }
        true
    }

    async fn route_effect(&mut self, effect: SessionEffect) -> bool {
        match effect {
            SessionEffect::SendBytes(bytes) => {
                if self.transport.outbound.send(bytes).await.is_err() {
                    let _ = self.emit_peer_disconnected(None).await;
                    return false;
                }
                true
            }
            SessionEffect::DeliverPeerFrame {
                mini_protocol,
                payload,
            } => {
                let event = match mini_protocol {
                    AcceptedMiniProtocol::ChainSync => match self.peer_role {
                        PeerRole::UpstreamClient => OrchestratorEvent::PeerChainSyncFrame {
                            peer_id: self.peer_id,
                            bytes: payload,
                        },
                        PeerRole::DownstreamServer => {
                            OrchestratorEvent::PeerN2nServerChainSyncFrame {
                                peer_id: self.peer_id,
                                bytes: payload,
                            }
                        }
                    },
                    AcceptedMiniProtocol::BlockFetch => match self.peer_role {
                        PeerRole::UpstreamClient => OrchestratorEvent::PeerBlockFetchFrame {
                            peer_id: self.peer_id,
                            bytes: payload,
                        },
                        PeerRole::DownstreamServer => {
                            OrchestratorEvent::PeerN2nServerBlockFetchFrame {
                                peer_id: self.peer_id,
                                bytes: payload,
                            }
                        }
                    },
                    // Other accepted protocols (KeepAlive, TxSubmission,
                    // PeerSharing, N2C family) are not routed into the
                    // orchestrator core in this cluster — they need
                    // additional `OrchestratorEvent` variants. For now
                    // the pump silently drops them after counting via the
                    // deliver-bytes path. Future cluster lands those
                    // discriminants additively.
                    AcceptedMiniProtocol::Handshake
                    | AcceptedMiniProtocol::KeepAlive
                    | AcceptedMiniProtocol::TxSubmission
                    | AcceptedMiniProtocol::LocalChainSync
                    | AcceptedMiniProtocol::LocalTxSubmission
                    | AcceptedMiniProtocol::LocalStateQuery
                    | AcceptedMiniProtocol::LocalTxMonitor
                    | AcceptedMiniProtocol::PeerSharing => return true,
                };
                if self.events_out.send(event).await.is_err() {
                    return false;
                }
                true
            }
            SessionEffect::HandshakeComplete { .. } => {
                // The dialer (S7) is the canonical emitter of
                // PeerConnected; the pump observes the effect for
                // logging only.
                true
            }
        }
    }

    async fn emit_peer_disconnected(
        &mut self,
        _reason: Option<PeerHaltReason>,
    ) -> bool {
        self.events_out
            .send(OrchestratorEvent::PeerDisconnected {
                peer_id: self.peer_id,
            })
            .await
            .is_ok()
    }
}

fn session_err_to_halt(err: &SessionError) -> PeerHaltReason {
    // Map every session-error variant onto an existing PeerHaltReason
    // discriminant. The orchestrator core's reason taxonomy is the
    // single source; if a variant doesn't fit a wire-side bucket, we
    // reuse the closest match.
    match err {
        SessionError::UnknownMiniProtocolId { .. } => PeerHaltReason::ChainSyncDecodeError,
        SessionError::PreHandshakeMiniProtocolFrame { .. } => {
            PeerHaltReason::ChainSyncDecodeError
        }
        SessionError::PostHandshakeHandshakeFrame => {
            PeerHaltReason::ChainSyncDecodeError
        }
        SessionError::Mux(_) => PeerHaltReason::ChainSyncDecodeError,
        SessionError::Handshake(_) => PeerHaltReason::ChainSyncDecodeError,
        SessionError::OutboundPayloadTooLarge { .. } => {
            PeerHaltReason::ChainSyncDecodeError
        }
        SessionError::ProtocolPayloadMalformed { .. } => {
            PeerHaltReason::ChainSyncDecodeError
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use ade_network::codec::N2NVersion;
    use ade_network::handshake::state::{PeerSharingFlag, VersionData};
    use ade_network::handshake::version_table::MAINNET_NETWORK_MAGIC;
    use ade_network::mux::frame::{encode_frame, MiniProtocolId, MuxFrame, MuxHeader, MuxMode};
    use ade_network::mux::transport::{spawn_duplex, DuplexCapacity};
    use ade_network::session::ConnectedState;
    use tokio::net::{TcpListener, TcpStream};

    async fn loopback_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let connect_fut = TcpStream::connect(addr);
        let accept_fut = async {
            let (s, _) = listener.accept().await.expect("accept");
            s
        };
        let (a, b) = tokio::join!(connect_fut, accept_fut);
        (a.expect("connect"), b)
    }

    fn fake_connected_state() -> SessionState {
        SessionState::Connected(ConnectedState::new(
            14,
            VersionData {
                network_magic: MAINNET_NETWORK_MAGIC,
                initiator_only_diffusion: false,
                peer_sharing: PeerSharingFlag::NoPeerSharing,
                query: false,
                peras_support: false,
            },
        ))
    }

    fn chain_sync_frame(payload: Vec<u8>) -> Vec<u8> {
        let f = MuxFrame {
            header: MuxHeader {
                timestamp: 0,
                mode: MuxMode::Responder,
                mini_protocol_id: MiniProtocolId::new(2).expect("2"),
                length: payload.len() as u16,
            },
            payload,
        };
        encode_frame(&f).expect("encode")
    }

    #[tokio::test]
    async fn mux_pump_routes_chain_sync_frame_over_loopback() {
        let (a, b) = loopback_pair().await;
        let mut handle_a = spawn_duplex(a, DuplexCapacity::DEFAULT);
        let handle_b = spawn_duplex(b, DuplexCapacity::DEFAULT);
        let (events_tx, mut events_rx) = mpsc::channel(8);

        // Peer A's pump: post-handshake state, upstream client role.
        let pump = MuxPump {
            peer_id: PeerId(1),
            transport: MuxTransportHandle {
                inbound: std::mem::replace(
                    &mut handle_a.inbound,
                    mpsc::channel::<Vec<u8>>(1).1,
                ),
                outbound: handle_a.outbound.clone(),
                reader_handle: tokio::spawn(async { Ok(()) }),
                writer_handle: tokio::spawn(async { Ok(()) }),
            },
            session_state: fake_connected_state(),
            events_out: events_tx,
            peer_role: PeerRole::UpstreamClient,
            outbound_relay: None,
        };
        let pump_handle = tokio::spawn(pump.run());

        // Send a chain-sync frame from b → a. Under FRAG (CN-SESS-04
        // / DC-SESS-06), the session reducer now requires the
        // mini-protocol payload to be a valid CBOR item; deliver
        // bytes that decode as a complete chain-sync RequestNext
        // (`0x81 0x00` = `array(1)[uint(0)]`).
        let payload = vec![0x81u8, 0x00];
        handle_b
            .outbound
            .send(chain_sync_frame(payload.clone()))
            .await
            .expect("send");

        // Expect a PeerChainSyncFrame event on the orchestrator inbox.
        let ev = events_rx.recv().await.expect("event");
        match ev {
            OrchestratorEvent::PeerChainSyncFrame { peer_id, bytes } => {
                assert_eq!(peer_id, PeerId(1));
                assert_eq!(bytes, payload);
            }
            other => panic!("expected PeerChainSyncFrame, got {other:?}"),
        }

        pump_handle.abort();
        drop(handle_b);
        let _ = N2NVersion::new(14); // touch import
    }
}
