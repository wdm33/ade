// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED per-peer admission wire pump (PHASE4-N-M-C S3).
//!
//! Owns a post-handshake [`MuxTransportHandle`] and drives the
//! chain-sync + block-fetch mini-protocols (initiator side)
//! against a single upstream peer. Emits closed
//! [`AdmissionPeerEvent`] values into the admission runner's
//! `peer_events` channel.
//!
//! Authority + closure (PHASE4-N-M-C):
//!   - **CN-PUMP-01** — exactly one
//!     `pub async fn run_admission_wire_pump` across the
//!     workspace.
//!   - **DC-PUMP-01** — pump emits `AdmissionPeerEvent::{Block,
//!     TipUpdate, Disconnected}` only; never an
//!     `AgreementVerdict`. The verdict reducer remains
//!     downstream of the pump.
//!   - **DC-PUMP-02** — every chain-sync reply carrying a `Tip`
//!     (`IntersectFound`, `IntersectNotFound`, `RollForward`,
//!     `RollBackward`) yields an `AdmissionPeerEvent::TipUpdate`
//!     before any further processing.
//!
//! The pump is intentionally minimal: it drives chain-sync from
//! the operator-provided start point, requests blocks via
//! block-fetch one tip-range at a time, and never holds
//! authority state. Block-byte validation, verdict derivation,
//! and admission halting are all the runner's job
//! ([[feedback-evidence-reducers-are-green-not-authority]]).

use std::collections::VecDeque;
use std::io;
use std::net::SocketAddr;

use ade_network::codec::block_fetch::{
    decode_block_fetch_message, encode_block_fetch_message, BlockFetchMessage,
    Point as BfPoint, Range,
};
use ade_network::codec::chain_sync::{
    decode_chain_sync_message, encode_chain_sync_message, ChainSyncMessage, Point, Tip,
};
use ade_network::codec::handshake::VersionTable;
use ade_network::handshake::state::{HandshakeError, PeerSharingFlag, VersionData};
use ade_network::mux::frame::MuxMode;
use ade_network::mux::transport::{
    spawn_duplex, DuplexCapacity, MuxTransportHandle, TransportError,
};
use ade_network::session::{
    run_n2n_handshake_initiator, step, AcceptedMiniProtocol, ByteChunkIn, ConnectedState,
    NegotiatedN2n, SessionEffect, SessionError, SessionState, Transport,
    TransportError as SessionTransportError,
};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

/// Closed per-peer event sum the pump emits into the admission
/// runner's `peer_events` channel. Identical shape to
/// `ade_node::admission::AdmissionPeerEvent`; the runner adapts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionPeerEvent {
    /// A complete block CBOR delivered by the peer. Runner
    /// invokes `admit_via_block_validity` on the bytes.
    Block { peer: String, block_bytes: Vec<u8> },
    /// The peer's chain-sync tip changed. Used as the comparison
    /// input by the runner's next `verdict::derive` call.
    TipUpdate { peer: String, tip: Tip },
    /// Peer connection closed (clean EOF, protocol error, or
    /// transport drop). The runner uses this for clean-shutdown
    /// accounting.
    Disconnected { peer: String },
}

/// Closed pump-result discriminator. Each variant maps to a
/// pump-side termination cause; the runner treats every variant
/// as a final state for this peer (and emits `Disconnected`
/// itself if the channel still has capacity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionWirePumpResult {
    /// Clean EOF — peer closed its outbound half.
    Eof,
    /// Wire pump halted on an error. The variant carries a
    /// closed-sum reason.
    Error(AdmissionWirePumpError),
    /// Downstream `events_out` channel dropped. The runner is
    /// gone; the pump has no peer to report to.
    EventsChannelDropped,
}

/// Closed wire-pump error sum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionWirePumpError {
    /// The session reducer rejected an inbound chunk (mux frame
    /// decode failure, unknown mini-protocol id, etc).
    Session(SessionError),
    /// A chain-sync payload failed structured decoding (CBOR
    /// shape mismatch).
    ChainSyncDecode,
    /// A block-fetch payload failed structured decoding.
    BlockFetchDecode,
    /// Peer sent a chain-sync or block-fetch frame at a
    /// protocol-illegal point (e.g. RollForward while we never
    /// asked).
    UnexpectedProtocolMessage { protocol: &'static str },
    /// Transport-level error reading inbound bytes — the
    /// underlying mux reader exited.
    TransportRead,
    /// Transport-level error writing outbound bytes.
    TransportWrite,
}

/// SOLE per-peer wire-pump entry (CN-PUMP-01). Drives chain-sync
/// + block-fetch initiator state machines against `transport`
/// and emits typed events into `events_out`.
///
/// Behaviour summary:
///   1. Send `ChainSyncMessage::FindIntersect[start_point]`.
///   2. On `IntersectFound { point, tip }`: emit `TipUpdate`;
///      request the tip's single block via
///      `BlockFetchMessage::RequestRange { from: tip.point, to: tip.point }`.
///   3. On block-fetch `Block { bytes }`: emit
///      `AdmissionPeerEvent::Block`.
///   4. On block-fetch `BatchDone`: send chain-sync
///      `RequestNext`.
///   5. On chain-sync `RollForward { header, tip }`: emit
///      `TipUpdate`; send chain-sync `RequestNext`. (Block
///      fetching of in-flight rolls is deferred to a future
///      strengthening — the C5 acceptance criterion is met by
///      the initial Tip-block fetch in step 2.)
///   6. On chain-sync `RollBackward { tip, .. }` or
///      `IntersectNotFound { tip }`: emit `TipUpdate`; send
///      `RequestNext` (drain).
///   7. On EOF / protocol error: emit `Disconnected`; return.
pub async fn run_admission_wire_pump(
    mut transport: MuxTransportHandle,
    peer_addr: String,
    start_point: Point,
    negotiated_version: u16,
    network_magic: u32,
    events_out: mpsc::Sender<AdmissionPeerEvent>,
) -> AdmissionWirePumpResult {
    let mut state = post_handshake_state(negotiated_version, network_magic);
    let mut outbox_payloads: VecDeque<ByteChunkIn> = VecDeque::new();

    // Kick off chain-sync: FindIntersect[start_point].
    let initial = ChainSyncMessage::FindIntersect {
        points: vec![start_point.clone()],
    };
    outbox_payloads.push_back(ByteChunkIn::OutboundFrame {
        mini_protocol: AcceptedMiniProtocol::ChainSync,
        payload: encode_chain_sync_message(&initial),
        mode: MuxMode::Initiator,
        timestamp: 0,
    });

    let mut chain_sync_in_flight = true;
    let mut block_fetch_in_flight = false;

    loop {
        // 1. Flush every queued outbound payload first.
        while let Some(out_event) = outbox_payloads.pop_front() {
            match flush_outbound(&mut state, out_event, &mut transport, &peer_addr).await {
                Ok(()) => {}
                Err(res) => return finalize(&peer_addr, res, &events_out).await,
            }
        }

        // 2. Read the next inbound chunk.
        let chunk = match transport.inbound.recv().await {
            Some(c) => c,
            None => {
                return finalize(&peer_addr, AdmissionWirePumpResult::Eof, &events_out).await;
            }
        };

        let effects = match step(&mut state, ByteChunkIn::Inbound(chunk)) {
            Ok(e) => e,
            Err(err) => {
                return finalize(
                    &peer_addr,
                    AdmissionWirePumpResult::Error(AdmissionWirePumpError::Session(err)),
                    &events_out,
                )
                .await;
            }
        };

        for effect in effects {
            match effect {
                SessionEffect::SendBytes(bytes) => {
                    if transport.outbound.send(bytes).await.is_err() {
                        return finalize(
                            &peer_addr,
                            AdmissionWirePumpResult::Error(
                                AdmissionWirePumpError::TransportWrite,
                            ),
                            &events_out,
                        )
                        .await;
                    }
                }
                SessionEffect::DeliverPeerFrame {
                    mini_protocol,
                    payload,
                } => match mini_protocol {
                    AcceptedMiniProtocol::ChainSync => {
                        let msg = match decode_chain_sync_message(&payload) {
                            Ok(m) => m,
                            Err(_) => {
                                return finalize(
                                    &peer_addr,
                                    AdmissionWirePumpResult::Error(
                                        AdmissionWirePumpError::ChainSyncDecode,
                                    ),
                                    &events_out,
                                )
                                .await;
                            }
                        };
                        match handle_chain_sync(
                            msg,
                            &peer_addr,
                            &events_out,
                            &mut outbox_payloads,
                            &mut chain_sync_in_flight,
                            &mut block_fetch_in_flight,
                        )
                        .await
                        {
                            Ok(()) => {}
                            Err(res) => {
                                return finalize(&peer_addr, res, &events_out).await;
                            }
                        }
                    }
                    AcceptedMiniProtocol::BlockFetch => {
                        let msg = match decode_block_fetch_message(&payload) {
                            Ok(m) => m,
                            Err(_) => {
                                return finalize(
                                    &peer_addr,
                                    AdmissionWirePumpResult::Error(
                                        AdmissionWirePumpError::BlockFetchDecode,
                                    ),
                                    &events_out,
                                )
                                .await;
                            }
                        };
                        match handle_block_fetch(
                            msg,
                            &peer_addr,
                            &events_out,
                            &mut outbox_payloads,
                            &mut chain_sync_in_flight,
                            &mut block_fetch_in_flight,
                        )
                        .await
                        {
                            Ok(()) => {}
                            Err(res) => {
                                return finalize(&peer_addr, res, &events_out).await;
                            }
                        }
                    }
                    AcceptedMiniProtocol::KeepAlive
                    | AcceptedMiniProtocol::Handshake
                    | AcceptedMiniProtocol::TxSubmission
                    | AcceptedMiniProtocol::LocalChainSync
                    | AcceptedMiniProtocol::LocalTxSubmission
                    | AcceptedMiniProtocol::LocalStateQuery
                    | AcceptedMiniProtocol::LocalTxMonitor
                    | AcceptedMiniProtocol::PeerSharing => {
                        // Honest-scope: the admission pump only
                        // listens for chain-sync + block-fetch in
                        // this cluster. Other accepted
                        // mini-protocol frames are silently
                        // dropped; the runner has no consumer
                        // for them.
                    }
                },
                SessionEffect::HandshakeComplete { .. } => {
                    // The pump assumes a post-handshake state on
                    // entry; observing this effect here would be
                    // a configuration drift, not a peer fault.
                    // Treat as a no-op.
                }
            }
        }
    }
}

/// Construct a `SessionState::Connected` for a peer that has
/// already completed the N2N handshake. The pump enters with the
/// negotiated version + the operator-supplied network magic.
fn post_handshake_state(version: u16, network_magic: u32) -> SessionState {
    SessionState::Connected(ConnectedState::new(
        version,
        VersionData {
            network_magic,
            initiator_only_diffusion: false,
            peer_sharing: PeerSharingFlag::NoPeerSharing,
            query: false,
            peras_support: false,
        },
    ))
}

async fn flush_outbound(
    state: &mut SessionState,
    chunk_in: ByteChunkIn,
    transport: &mut MuxTransportHandle,
    _peer_addr: &str,
) -> Result<(), AdmissionWirePumpResult> {
    let effects = match step(state, chunk_in) {
        Ok(e) => e,
        Err(err) => {
            return Err(AdmissionWirePumpResult::Error(
                AdmissionWirePumpError::Session(err),
            ));
        }
    };
    for effect in effects {
        match effect {
            SessionEffect::SendBytes(bytes) => {
                if transport.outbound.send(bytes).await.is_err() {
                    return Err(AdmissionWirePumpResult::Error(
                        AdmissionWirePumpError::TransportWrite,
                    ));
                }
            }
            // A request to send an OutboundFrame should not
            // produce DeliverPeerFrame / HandshakeComplete — but
            // we tolerate both as no-ops to keep the closed sum
            // exhaustive without crashing.
            SessionEffect::DeliverPeerFrame { .. }
            | SessionEffect::HandshakeComplete { .. } => {}
        }
    }
    Ok(())
}

async fn handle_chain_sync(
    msg: ChainSyncMessage,
    peer_addr: &str,
    events_out: &mpsc::Sender<AdmissionPeerEvent>,
    outbox: &mut VecDeque<ByteChunkIn>,
    chain_sync_in_flight: &mut bool,
    block_fetch_in_flight: &mut bool,
) -> Result<(), AdmissionWirePumpResult> {
    match msg {
        ChainSyncMessage::IntersectFound { point: _, tip } => {
            emit(events_out, peer_addr, tip_update(peer_addr, tip.clone())).await?;
            *chain_sync_in_flight = false;
            // Issue a block-fetch RequestRange[tip,tip] to pull
            // the tip block bytes for admission.
            queue_block_fetch_request(outbox, &tip.point);
            *block_fetch_in_flight = true;
            Ok(())
        }
        ChainSyncMessage::IntersectNotFound { tip } => {
            emit(events_out, peer_addr, tip_update(peer_addr, tip)).await?;
            *chain_sync_in_flight = false;
            // Producer has nothing for us at the requested
            // intersect — request the next message anyway so the
            // protocol stays live until the upstream sends Done.
            queue_chain_sync_request_next(outbox);
            *chain_sync_in_flight = true;
            Ok(())
        }
        ChainSyncMessage::RollForward { header: _, tip } => {
            emit(events_out, peer_addr, tip_update(peer_addr, tip)).await?;
            *chain_sync_in_flight = false;
            // Block-fetch of the rolled-forward header point is
            // a future strengthening (would need a header-point
            // extractor). For C3, keep chain-sync alive so the
            // pump continues to surface fresh tips.
            queue_chain_sync_request_next(outbox);
            *chain_sync_in_flight = true;
            Ok(())
        }
        ChainSyncMessage::RollBackward { point: _, tip } => {
            emit(events_out, peer_addr, tip_update(peer_addr, tip)).await?;
            *chain_sync_in_flight = false;
            queue_chain_sync_request_next(outbox);
            *chain_sync_in_flight = true;
            Ok(())
        }
        ChainSyncMessage::AwaitReply => {
            // No tip in this message; nothing to emit. Producer
            // will send a follow-up RollForward/RollBackward.
            Ok(())
        }
        ChainSyncMessage::Done => {
            // Producer is done streaming. Nothing more to do; the
            // pump exits cleanly when the channel closes.
            Ok(())
        }
        // Client-originated chain-sync messages should never
        // arrive on the inbound path.
        ChainSyncMessage::RequestNext | ChainSyncMessage::FindIntersect { .. } => {
            Err(AdmissionWirePumpResult::Error(
                AdmissionWirePumpError::UnexpectedProtocolMessage {
                    protocol: "chain_sync",
                },
            ))
        }
    }
}

async fn handle_block_fetch(
    msg: BlockFetchMessage,
    peer_addr: &str,
    events_out: &mpsc::Sender<AdmissionPeerEvent>,
    outbox: &mut VecDeque<ByteChunkIn>,
    _chain_sync_in_flight: &mut bool,
    block_fetch_in_flight: &mut bool,
) -> Result<(), AdmissionWirePumpResult> {
    match msg {
        BlockFetchMessage::StartBatch => Ok(()),
        BlockFetchMessage::NoBlocks => {
            *block_fetch_in_flight = false;
            // Fall through to RequestNext so chain-sync keeps
            // streaming tip updates.
            queue_chain_sync_request_next(outbox);
            Ok(())
        }
        BlockFetchMessage::Block { bytes } => {
            emit(
                events_out,
                peer_addr,
                AdmissionPeerEvent::Block {
                    peer: peer_addr.to_string(),
                    block_bytes: bytes,
                },
            )
            .await
        }
        BlockFetchMessage::BatchDone => {
            *block_fetch_in_flight = false;
            queue_chain_sync_request_next(outbox);
            Ok(())
        }
        // Client-originated block-fetch messages should never
        // arrive on the inbound path.
        BlockFetchMessage::RequestRange(_) | BlockFetchMessage::ClientDone => {
            Err(AdmissionWirePumpResult::Error(
                AdmissionWirePumpError::UnexpectedProtocolMessage {
                    protocol: "block_fetch",
                },
            ))
        }
    }
}

fn queue_chain_sync_request_next(outbox: &mut VecDeque<ByteChunkIn>) {
    outbox.push_back(ByteChunkIn::OutboundFrame {
        mini_protocol: AcceptedMiniProtocol::ChainSync,
        payload: encode_chain_sync_message(&ChainSyncMessage::RequestNext),
        mode: MuxMode::Initiator,
        timestamp: 0,
    });
}

fn queue_block_fetch_request(outbox: &mut VecDeque<ByteChunkIn>, point: &Point) {
    let bf_point = chain_sync_point_to_block_fetch_point(point);
    outbox.push_back(ByteChunkIn::OutboundFrame {
        mini_protocol: AcceptedMiniProtocol::BlockFetch,
        payload: encode_block_fetch_message(&BlockFetchMessage::RequestRange(Range {
            from: bf_point.clone(),
            to: bf_point,
        })),
        mode: MuxMode::Initiator,
        timestamp: 0,
    });
}

fn chain_sync_point_to_block_fetch_point(point: &Point) -> BfPoint {
    match point {
        Point::Origin => BfPoint::Origin,
        Point::Block { slot, hash } => BfPoint::Block {
            slot: *slot,
            hash: hash.clone(),
        },
    }
}

fn tip_update(peer_addr: &str, tip: Tip) -> AdmissionPeerEvent {
    AdmissionPeerEvent::TipUpdate {
        peer: peer_addr.to_string(),
        tip,
    }
}

async fn emit(
    events_out: &mpsc::Sender<AdmissionPeerEvent>,
    _peer_addr: &str,
    event: AdmissionPeerEvent,
) -> Result<(), AdmissionWirePumpResult> {
    if events_out.send(event).await.is_err() {
        Err(AdmissionWirePumpResult::EventsChannelDropped)
    } else {
        Ok(())
    }
}

async fn finalize(
    peer_addr: &str,
    result: AdmissionWirePumpResult,
    events_out: &mpsc::Sender<AdmissionPeerEvent>,
) -> AdmissionWirePumpResult {
    // Best-effort Disconnected emit. If the channel is gone, the
    // runner has already noticed; nothing else to do.
    let _ = events_out
        .send(AdmissionPeerEvent::Disconnected {
            peer: peer_addr.to_string(),
        })
        .await;
    result
}

/// Closed dial-side error sum returned by
/// [`dial_for_admission`].
#[derive(Debug)]
pub enum AdmissionDialError {
    /// TCP connect failed.
    Io(io::ErrorKind),
    /// Handshake driver failure (initiator side).
    Handshake(HandshakeError),
    /// Underlying mux transport error during handshake.
    Transport(TransportError),
    /// `spawn_blocking` join error.
    BlockingJoin,
}

/// TCP dial + N2N handshake + post-handshake transport handoff
/// for the admission wire pump. Returns the bare
/// [`MuxTransportHandle`] + negotiated version so the caller can
/// pass them to [`run_admission_wire_pump`].
///
/// Distinct from [`crate::network::N2nDialer`] which is bound to
/// the orchestrator event channel. The admission pump consumes
/// its own [`AdmissionPeerEvent`] channel and does not route
/// through the orchestrator.
pub async fn dial_for_admission(
    peer_addr: SocketAddr,
    our_versions: VersionTable,
) -> Result<(MuxTransportHandle, u16), AdmissionDialError> {
    let stream = TcpStream::connect(peer_addr)
        .await
        .map_err(|e| AdmissionDialError::Io(e.kind()))?;
    let handle = spawn_duplex(stream, DuplexCapacity::DEFAULT);
    let MuxTransportHandle {
        inbound,
        outbound,
        reader_handle,
        writer_handle,
    } = handle;

    let result = tokio::task::spawn_blocking(move || {
        let mut bt = BlockingTransport::new(inbound, outbound);
        let res = run_n2n_handshake_initiator(&mut bt, our_versions);
        let (inbound, outbound) = bt.into_halves();
        (inbound, outbound, res)
    })
    .await
    .map_err(|_| AdmissionDialError::BlockingJoin)?;

    let (inbound, outbound, negotiated) = result;
    let negotiated: NegotiatedN2n = match negotiated {
        Ok(n) => n,
        Err(SessionTransportError::Handshake(e)) => return Err(AdmissionDialError::Handshake(e)),
        Err(SessionTransportError::Mux(_)) => {
            return Err(AdmissionDialError::Handshake(HandshakeError::MalformedMessage {
                reason: "mux decode error during handshake",
            }))
        }
        Err(SessionTransportError::Io) => {
            return Err(AdmissionDialError::Io(io::ErrorKind::Other))
        }
        Err(SessionTransportError::Eof) => {
            return Err(AdmissionDialError::Io(io::ErrorKind::UnexpectedEof))
        }
    };

    let transport = MuxTransportHandle {
        inbound,
        outbound,
        reader_handle,
        writer_handle,
    };
    Ok((transport, negotiated.version))
}

/// Sync transport bridge for the handshake driver. Identical
/// shape to the orchestrator dialer's adapter (a copy is fine —
/// the two dial paths must not share runtime state, only the
/// shape).
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
    use ade_network::handshake::version_table::MAINNET_NETWORK_MAGIC;
    use ade_network::mux::frame::{
        encode_frame, MiniProtocolId, MuxFrame, MuxHeader, MuxMode as TestMuxMode,
    };
    use ade_network::mux::transport::{spawn_duplex, DuplexCapacity};
    use ade_types::{Hash32, SlotNo};
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

    fn responder_frame(mp_id: u16, payload: Vec<u8>) -> Vec<u8> {
        let length = payload.len() as u16;
        let frame = MuxFrame {
            header: MuxHeader {
                timestamp: 0,
                mode: TestMuxMode::Responder,
                mini_protocol_id: MiniProtocolId::new(mp_id).expect("id"),
                length,
            },
            payload,
        };
        encode_frame(&frame).expect("encode")
    }

    fn fake_tip(slot: u64) -> Tip {
        Tip {
            point: Point::Block {
                slot: SlotNo(slot),
                hash: Hash32([0x11; 32]),
            },
            block_no: slot,
        }
    }

    #[tokio::test]
    async fn pump_emits_tip_update_and_block_on_initial_intersect_and_block_fetch() {
        // Wire two TCP halves and wrap each in a mux transport.
        let (client_stream, server_stream) = loopback_pair().await;
        let client_transport = spawn_duplex(client_stream, DuplexCapacity::DEFAULT);
        let mut server_transport = spawn_duplex(server_stream, DuplexCapacity::DEFAULT);

        let (events_tx, mut events_rx) = mpsc::channel::<AdmissionPeerEvent>(64);

        let pump_handle = tokio::spawn(async move {
            run_admission_wire_pump(
                client_transport,
                "127.0.0.1:0".into(),
                Point::Origin,
                14,
                MAINNET_NETWORK_MAGIC,
                events_tx,
            )
            .await
        });

        // Wait for the client's outbound FindIntersect to land on
        // the server side.
        let _first_in: Vec<u8> = server_transport
            .inbound
            .recv()
            .await
            .expect("client sent FindIntersect");

        // Server replies IntersectFound { point: Origin, tip:
        // (slot=42, hash=11..) }.
        let tip = fake_tip(42);
        let intersect_found = ChainSyncMessage::IntersectFound {
            point: Point::Origin,
            tip: tip.clone(),
        };
        let intersect_bytes = encode_chain_sync_message(&intersect_found);
        let intersect_frame = responder_frame(
            AcceptedMiniProtocol::CHAIN_SYNC_ID,
            intersect_bytes,
        );
        server_transport
            .outbound
            .send(intersect_frame)
            .await
            .expect("send IntersectFound");

        // First emitted event must be a TipUpdate that matches.
        let evt = tokio::time::timeout(
            std::time::Duration::from_millis(2000),
            events_rx.recv(),
        )
        .await
        .expect("tip update arrives")
        .expect("event");
        match evt {
            AdmissionPeerEvent::TipUpdate { tip: got_tip, .. } => {
                assert_eq!(got_tip, tip);
            }
            other => panic!("expected TipUpdate, got {:?}", other),
        }

        // Wait for the pump's BlockFetch RequestRange.
        let _bf_req: Vec<u8> = server_transport
            .inbound
            .recv()
            .await
            .expect("client sent RequestRange");

        // Server replies with the block-fetch happy path:
        // StartBatch → Block { bytes } → BatchDone. The bytes
        // field must be a single valid CBOR item (matching the
        // cardano-node N2N wrapped-block shape:
        // `[serialisationInfo, tag(24, bytes(inner))]`).
        let block_bytes = {
            let mut buf = Vec::new();
            // array(2)
            buf.push(0x82);
            // serialisationInfo: uint = 1
            buf.push(0x01);
            // tag(24) bytes(4) [DE AD BE EF]
            buf.push(0xd8);
            buf.push(0x18);
            buf.push(0x44);
            buf.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
            buf
        };
        for msg in [
            BlockFetchMessage::StartBatch,
            BlockFetchMessage::Block {
                bytes: block_bytes.clone(),
            },
            BlockFetchMessage::BatchDone,
        ] {
            let bytes = encode_block_fetch_message(&msg);
            let frame = responder_frame(AcceptedMiniProtocol::BLOCK_FETCH_ID, bytes);
            server_transport
                .outbound
                .send(frame)
                .await
                .expect("send block-fetch msg");
        }

        // The pump must emit AdmissionPeerEvent::Block with
        // exactly those bytes (DC-PUMP-01 + admission delivery).
        let evt = tokio::time::timeout(
            std::time::Duration::from_millis(2000),
            events_rx.recv(),
        )
        .await
        .expect("block event")
        .expect("event");
        match evt {
            AdmissionPeerEvent::Block {
                block_bytes: got, ..
            } => {
                assert_eq!(got, block_bytes);
            }
            other => panic!("expected Block, got {:?}", other),
        }

        // Drop the server side to trigger EOF on the client.
        drop(server_transport);

        // The pump must emit a final Disconnected.
        loop {
            let evt = tokio::time::timeout(
                std::time::Duration::from_millis(2000),
                events_rx.recv(),
            )
            .await
            .expect("disconnected")
            .expect("event");
            if let AdmissionPeerEvent::Disconnected { .. } = evt {
                break;
            }
        }

        let _ = pump_handle.await;
    }

    #[tokio::test]
    async fn pump_emits_tip_update_on_intersect_not_found() {
        let (client_stream, server_stream) = loopback_pair().await;
        let client_transport = spawn_duplex(client_stream, DuplexCapacity::DEFAULT);
        let mut server_transport = spawn_duplex(server_stream, DuplexCapacity::DEFAULT);

        let (events_tx, mut events_rx) = mpsc::channel::<AdmissionPeerEvent>(16);

        let pump_handle = tokio::spawn(async move {
            run_admission_wire_pump(
                client_transport,
                "127.0.0.1:0".into(),
                Point::Block {
                    slot: SlotNo(100),
                    hash: Hash32([0x99; 32]),
                },
                14,
                MAINNET_NETWORK_MAGIC,
                events_tx,
            )
            .await
        });

        let _first_in: Vec<u8> = server_transport
            .inbound
            .recv()
            .await
            .expect("client sent FindIntersect");

        let tip = fake_tip(200);
        let nf = ChainSyncMessage::IntersectNotFound { tip: tip.clone() };
        let bytes = encode_chain_sync_message(&nf);
        let frame = responder_frame(AcceptedMiniProtocol::CHAIN_SYNC_ID, bytes);
        server_transport
            .outbound
            .send(frame)
            .await
            .expect("send IntersectNotFound");

        let evt = tokio::time::timeout(
            std::time::Duration::from_millis(2000),
            events_rx.recv(),
        )
        .await
        .expect("tip update")
        .expect("event");
        match evt {
            AdmissionPeerEvent::TipUpdate { tip: got, .. } => assert_eq!(got, tip),
            other => panic!("expected TipUpdate, got {:?}", other),
        }

        drop(server_transport);

        loop {
            let evt = tokio::time::timeout(
                std::time::Duration::from_millis(2000),
                events_rx.recv(),
            )
            .await
            .expect("disconnected")
            .expect("event");
            if let AdmissionPeerEvent::Disconnected { .. } = evt {
                break;
            }
        }

        let _ = pump_handle.await;
    }
}
