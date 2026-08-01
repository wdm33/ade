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
use std::time::Duration;

use ade_network::codec::block_fetch::{
    decode_block_fetch_message, decompose_blockfetch_block, encode_block_fetch_message,
    BlockFetchMessage,
    Point as BfPoint, Range,
};
use ade_network::codec::chain_sync::{
    decode_chain_sync_message, encode_chain_sync_message, ChainSyncMessage, Point, Tip,
};
use ade_network::codec::handshake::VersionTable;
use ade_network::codec::keep_alive::{
    decode_keep_alive_message, encode_keep_alive_message, KeepAliveCookie, KeepAliveMessage,
};
use ade_network::codec::version::KeepAliveVersion;
use ade_network::keep_alive::{
    keep_alive_transition, KeepAliveAgency, KeepAliveError, KeepAliveEvent, KeepAliveOutput,
    KeepAliveState,
};
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
    /// PHASE4-N-AI AI-S4a: the peer announced a chain-sync rollback to
    /// `point` (always a concrete `Block` point — `Origin` fails closed at the
    /// pump). The closed authority signal for fork-choice / durable rollback;
    /// the live loop consumes it in AI-S4b (latent until then). `tip` is the
    /// peer's post-rollback tip, carried for transcript / consistency parity
    /// with `TipUpdate`. A rollback is NEVER represented as a `TipUpdate` only.
    RollBackward { peer: String, point: Point, tip: Tip },
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
    /// PHASE4-N-AI AI-S4a: the peer sent a chain-sync `RollBackward` to
    /// `Origin` (rollback-to-genesis). Unsupported for single-best-peer within
    /// k — fail closed (drop the peer) rather than surface a rollback point
    /// this rung cannot apply.
    UnsupportedRollbackPoint,
    /// DC-PUMP-03 (PHASE4-N-AM): an inbound keep-alive frame violated the
    /// keep-alive grammar (cookie mismatch, illegal transition, an
    /// out-of-version message, or an undecodable payload). Wire-only
    /// liveness fault — drop the peer rather than continue on a broken
    /// keep-alive exchange.
    KeepAlive(KeepAliveError),
    /// LIVE-WIRE-LIVENESS S1 (INV-WL-3): the peer kept delivering
    /// chain-sync / block-fetch frames while the pump was waiting for
    /// `events_out` capacity, past `MAX_DEFERRED_PEER_FRAMES`. Since the pump
    /// issues no new requests while backpressured (INV-WL-4), a well-behaved
    /// peer quiesces after its in-flight batch; exceeding the cap means
    /// unbounded buffering would be required, so we fail closed instead.
    DeferredFrameOverflow,
}

/// LIVE-WIRE-LIVENESS S1 (INV-WL-3). Cap on inbound peer frames held while the
/// pump waits for downstream `events_out` capacity. Bounded by the peer's
/// in-flight batch because no new demand is issued while backpressured
/// (INV-WL-4); the cap is the fail-closed backstop, not the expected depth.
const MAX_DEFERRED_PEER_FRAMES: usize = 4096;

/// Keep-alive cadence (DC-PUMP-03 / PHASE4-N-AM). The wire pump sends
/// `MsgKeepAlive` every `KEEP_ALIVE_CADENCE` during inbound quiescence —
/// STRICTLY under the peer's ~97s keep-alive timeout (observed live: the
/// relay `ShutdownPeer`s a silent client at ~97s with `ExceededTimeLimit
/// (KeepAlive) ClientHasAgency`). ~20s leaves ~3 missed-tick margin and is
/// quieter than the 10s ecosystem default. Wall-clock cadence is a RED
/// transport concern; it never reaches the BLUE core.
const KEEP_ALIVE_CADENCE: Duration = Duration::from_secs(20);

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

    // DC-PUMP-03 (PHASE4-N-AM): N2N keep-alive CLIENT state. The cadence
    // sends `MsgKeepAlive` during inbound quiescence so the peer's
    // keep-alive responder does not `ShutdownPeer` us at the ~97s timeout.
    // The cookie is a monotonic u16 (deterministic — no rand); the BLUE
    // `keep_alive_transition` carries the in-flight cookie and validates the
    // echo. Wire-only: this state never produces an `AdmissionPeerEvent`.
    //
    // LIVE-WIRE-LIVENESS S1: the cadence timer is a LOCAL held disjointly from
    // `keep_alive`, so a `select!` can borrow the timer while its branch body
    // borrows the lane. The lane is serviced from the main loop AND from
    // `emit_cooperative`, which is what makes it immune to a stalled consumer
    // (INV-WL-1).
    let mut keep_alive = KeepAliveLane {
        state: KeepAliveState::ClientIdle,
        next_cookie: 0,
        version: KeepAliveVersion::new(negotiated_version),
    };
    let mut keep_alive_timer = tokio::time::interval(KEEP_ALIVE_CADENCE);
    keep_alive_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // Consume the immediate first tick so the first keep-alive fires after
    // one full cadence, not at startup.
    keep_alive_timer.tick().await;

    // LIVE-WIRE-LIVENESS S1: the single ordered queue of peer frames awaiting
    // dispatch. Frames decoded from a chunk are appended here rather than
    // dispatched inline, and `emit_cooperative` appends frames it reads while
    // parked on backpressure. Because exactly one frame is dispatched per loop
    // iteration and the queue is drained before any new read, wire order — and
    // therefore emitted-event order — is unchanged (INV-WL-2).
    let mut pending_frames: VecDeque<(AcceptedMiniProtocol, Vec<u8>)> = VecDeque::new();

    loop {
        // 1. Flush every queued outbound payload first.
        while let Some(out_event) = outbox_payloads.pop_front() {
            match flush_outbound(&mut state, out_event, &transport.outbound, &peer_addr).await {
                Ok(()) => {}
                Err(res) => return finalize(&peer_addr, res, &events_out).await,
            }
        }

        // 1b. Dispatch exactly one already-received frame before reading more.
        //     Draining to empty before the next read is what bounds
        //     `pending_frames`: no fresh chain-sync / block-fetch request is
        //     issued while frames are outstanding, so a well-behaved peer
        //     quiesces after its in-flight batch (INV-WL-4).
        if let Some((mini_protocol, payload)) = pending_frames.pop_front() {
            if let Err(res) = dispatch_peer_frame(
                mini_protocol,
                payload,
                &peer_addr,
                &events_out,
                &mut state,
                &mut transport.inbound,
                &transport.outbound,
                &mut keep_alive,
                &mut keep_alive_timer,
                &mut pending_frames,
                &mut outbox_payloads,
                &mut chain_sync_in_flight,
                &mut block_fetch_in_flight,
            )
            .await
            {
                return finalize(&peer_addr, res, &events_out).await;
            }
            continue;
        }

        // 2. Read the next inbound chunk, OR fire a keep-alive on the
        //    cadence during inbound quiescence (DC-PUMP-03). The inbound
        //    arm is byte-identical to the pre-AM behaviour; mpsc `recv()`
        //    is cancel-safe, so a keep-alive tick never drops a chunk.
        let chunk = tokio::select! {
            maybe_chunk = transport.inbound.recv() => match maybe_chunk {
                Some(c) => c,
                None => {
                    return finalize(&peer_addr, AdmissionWirePumpResult::Eof, &events_out).await;
                }
            },
            _ = keep_alive_timer.tick() => {
                // Cadence fired during a quiescent inbound. Send
                // `MsgKeepAlive` iff the client holds agency (no keepalive
                // in flight — respect `ServerHasAgency`). Wire-only — no
                // event emitted. Shares `keep_alive_cadence_send` with
                // `emit_cooperative` so both paths behave identically.
                if let Err(res) = keep_alive_cadence_send(
                    &mut keep_alive,
                    &mut state,
                    &transport.outbound,
                    &peer_addr,
                )
                .await
                {
                    return finalize(&peer_addr, res, &events_out).await;
                }
                continue;
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
                } => {
                    // LIVE-WIRE-LIVENESS S1: enqueue rather than dispatch
                    // inline. One chunk can carry several frames; dispatching
                    // them here would let frames deferred by `emit_cooperative`
                    // (appended to the back) interleave ahead of this chunk's
                    // remaining frames. Enqueuing keeps a single FIFO, and step
                    // 1b dispatches exactly one per iteration in wire order
                    // (INV-WL-2).
                    pending_frames.push_back((mini_protocol, payload));
                }
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

/// LIVE-WIRE-LIVENESS S1: takes `&Sender` rather than `&mut MuxTransportHandle`
/// (it only ever writes) so a caller can hold `&mut transport.inbound` and
/// `&transport.outbound` at once as disjoint field borrows — required by
/// `emit_cooperative`, which must read inbound and write keep-alive frames
/// while parked on downstream backpressure.
async fn flush_outbound(
    state: &mut SessionState,
    chunk_in: ByteChunkIn,
    outbound: &mpsc::Sender<Vec<u8>>,
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
                if outbound.send(bytes).await.is_err() {
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

/// LIVE-WIRE-LIVENESS S1 (INV-WL-1): the N2N keep-alive CLIENT lane. Bundled so
/// the identical cadence logic can run from the main loop AND from
/// `emit_cooperative` while the pump is parked on downstream backpressure. The
/// `Interval` is deliberately NOT a field: keeping it a separate local lets a
/// `select!` borrow the timer while the branch body borrows the lane.
struct KeepAliveLane {
    state: KeepAliveState,
    next_cookie: u16,
    version: KeepAliveVersion,
}

/// Send `MsgKeepAlive` iff the client holds agency (no keep-alive in flight —
/// respect `ServerHasAgency`). The BLUE `keep_alive_transition` advances the
/// state machine and carries the cookie.
///
/// LIVE-WIRE-LIVENESS S1: the frame is flushed IMMEDIATELY rather than queued on
/// `outbox_payloads`. The outbox is drained only at the top of the pump's outer
/// loop, so a queued keep-alive would never reach the wire during the very
/// stall it exists to survive.
async fn keep_alive_cadence_send(
    ka: &mut KeepAliveLane,
    session: &mut SessionState,
    outbound: &mpsc::Sender<Vec<u8>>,
    peer_addr: &str,
) -> Result<(), AdmissionWirePumpResult> {
    if ka.state != KeepAliveState::ClientIdle {
        return Ok(());
    }
    let cookie = KeepAliveCookie(ka.next_cookie);
    ka.next_cookie = ka.next_cookie.wrapping_add(1);
    match keep_alive_transition(
        ka.state,
        KeepAliveAgency::Client,
        ka.version,
        KeepAliveMessage::KeepAlive(cookie),
    ) {
        Ok((new_state, _output)) => {
            ka.state = new_state;
            // Wire-only diagnostic (DC-PUMP-03 / CE-AM-LIVE observability):
            // stderr only, no AdmissionPeerEvent.
            eprintln!("keep_alive: ping cookie={} sent (cadence) peer={peer_addr}", cookie.0);
            flush_outbound(
                session,
                ByteChunkIn::OutboundFrame {
                    mini_protocol: AcceptedMiniProtocol::KeepAlive,
                    payload: encode_keep_alive_message(&KeepAliveMessage::KeepAlive(cookie)),
                    mode: MuxMode::Initiator,
                    timestamp: 0,
                },
                outbound,
                peer_addr,
            )
            .await
        }
        Err(e) => Err(AdmissionWirePumpResult::Error(
            AdmissionWirePumpError::KeepAlive(e),
        )),
    }
}

/// LIVE-WIRE-LIVENESS S1 (INV-WL-1). Deliver one `AdmissionPeerEvent`
/// downstream without ever going silent on the wire.
///
/// The pre-slice pump awaited `events_out.send(..)` directly. That call sits
/// OUTSIDE the pump's `select!`, so while the consumer stalled (block
/// application, epoch-boundary fold, checkpoint capture) the keep-alive cadence
/// could not fire and the peer shut the connection down with
/// `ExceededTimeLimit (KeepAlive) ClientHasAgency` — observed live 2026-08-01
/// at 03:44:11Z, 2m52s after handshake.
///
/// Here the wait itself is a `select!`, so during an arbitrarily long stall we
/// keep the keep-alive protocol live in BOTH directions: fire the cadence, and
/// consume the peer's echoed `MsgResponseKeepAlive` so the BLUE state returns to
/// `ClientIdle` and later ticks are not suppressed by the agency guard. Handling
/// only the timer would send a single ping and then stall in `ClientAwaiting`,
/// buying ~117s instead of unbounded tolerance.
///
/// Non-keep-alive frames read while waiting are appended to `pending_frames` in
/// wire order (INV-WL-2), under a fixed cap (INV-WL-3). All three `select!`
/// arms are cancel-safe (`reserve`, `Interval::tick`, `Receiver::recv`).
#[allow(clippy::too_many_arguments)]
async fn emit_cooperative(
    events_out: &mpsc::Sender<AdmissionPeerEvent>,
    event: AdmissionPeerEvent,
    session: &mut SessionState,
    inbound: &mut mpsc::Receiver<Vec<u8>>,
    outbound: &mpsc::Sender<Vec<u8>>,
    ka: &mut KeepAliveLane,
    ka_timer: &mut tokio::time::Interval,
    pending_frames: &mut VecDeque<(AcceptedMiniProtocol, Vec<u8>)>,
    peer_addr: &str,
) -> Result<(), AdmissionWirePumpResult> {
    let permit = loop {
        tokio::select! {
            reserved = events_out.reserve() => match reserved {
                Ok(p) => break p,
                Err(_) => return Err(AdmissionWirePumpResult::EventsChannelDropped),
            },
            _ = ka_timer.tick() => {
                keep_alive_cadence_send(ka, session, outbound, peer_addr).await?;
            }
            maybe_chunk = inbound.recv() => {
                let Some(chunk) = maybe_chunk else {
                    return Err(AdmissionWirePumpResult::Eof);
                };
                let effects = step(session, ByteChunkIn::Inbound(chunk)).map_err(|err| {
                    AdmissionWirePumpResult::Error(AdmissionWirePumpError::Session(err))
                })?;
                for effect in effects {
                    match effect {
                        SessionEffect::SendBytes(bytes) => {
                            if outbound.send(bytes).await.is_err() {
                                return Err(AdmissionWirePumpResult::Error(
                                    AdmissionWirePumpError::TransportWrite,
                                ));
                            }
                        }
                        SessionEffect::DeliverPeerFrame { mini_protocol, payload } => {
                            if mini_protocol == AcceptedMiniProtocol::KeepAlive {
                                // The whole point: validate the echo NOW so the
                                // lane returns to `ClientIdle` and the next
                                // cadence tick is not suppressed.
                                handle_keep_alive(&payload, &mut ka.state, ka.version).map_err(
                                    |e| {
                                        AdmissionWirePumpResult::Error(
                                            AdmissionWirePumpError::KeepAlive(e),
                                        )
                                    },
                                )?;
                            } else {
                                if pending_frames.len() >= MAX_DEFERRED_PEER_FRAMES {
                                    return Err(AdmissionWirePumpResult::Error(
                                        AdmissionWirePumpError::DeferredFrameOverflow,
                                    ));
                                }
                                pending_frames.push_back((mini_protocol, payload));
                            }
                        }
                        SessionEffect::HandshakeComplete { .. } => {}
                    }
                }
            }
        }
    };
    permit.send(event);
    Ok(())
}

/// LIVE-WIRE-LIVENESS S1: decode + handle exactly one peer frame, then deliver
/// the events it produced through `emit_cooperative`. Shared by the outer loop's
/// step 1b so deferred and freshly-read frames take an identical path.
#[allow(clippy::too_many_arguments)]
async fn dispatch_peer_frame(
    mini_protocol: AcceptedMiniProtocol,
    payload: Vec<u8>,
    peer_addr: &str,
    events_out: &mpsc::Sender<AdmissionPeerEvent>,
    session: &mut SessionState,
    inbound: &mut mpsc::Receiver<Vec<u8>>,
    outbound: &mpsc::Sender<Vec<u8>>,
    ka: &mut KeepAliveLane,
    ka_timer: &mut tokio::time::Interval,
    pending_frames: &mut VecDeque<(AcceptedMiniProtocol, Vec<u8>)>,
    outbox: &mut VecDeque<ByteChunkIn>,
    chain_sync_in_flight: &mut bool,
    block_fetch_in_flight: &mut bool,
) -> Result<(), AdmissionWirePumpResult> {
    let mut out_events: VecDeque<AdmissionPeerEvent> = VecDeque::new();
    match mini_protocol {
        AcceptedMiniProtocol::ChainSync => {
            let msg = decode_chain_sync_message(&payload).map_err(|_| {
                AdmissionWirePumpResult::Error(AdmissionWirePumpError::ChainSyncDecode)
            })?;
            handle_chain_sync(
                msg,
                peer_addr,
                &mut out_events,
                outbox,
                chain_sync_in_flight,
                block_fetch_in_flight,
            )?;
        }
        AcceptedMiniProtocol::BlockFetch => {
            let msg = decode_block_fetch_message(&payload).map_err(|_| {
                AdmissionWirePumpResult::Error(AdmissionWirePumpError::BlockFetchDecode)
            })?;
            handle_block_fetch(
                msg,
                peer_addr,
                &mut out_events,
                outbox,
                chain_sync_in_flight,
                block_fetch_in_flight,
            )?;
        }
        AcceptedMiniProtocol::KeepAlive => {
            // DC-PUMP-03 (PHASE4-N-AM): consume the peer's
            // `MsgResponseKeepAlive` to advance the BLUE keep-alive state
            // machine back to `ClientIdle`, validating the echoed cookie.
            // Wire-only — emits NO `AdmissionPeerEvent`. A grammar violation
            // (cookie mismatch / illegal / undecodable) fails closed.
            handle_keep_alive(&payload, &mut ka.state, ka.version).map_err(|e| {
                AdmissionWirePumpResult::Error(AdmissionWirePumpError::KeepAlive(e))
            })?;
        }
        AcceptedMiniProtocol::Handshake
        | AcceptedMiniProtocol::TxSubmission
        | AcceptedMiniProtocol::LocalChainSync
        | AcceptedMiniProtocol::LocalTxSubmission
        | AcceptedMiniProtocol::LocalStateQuery
        | AcceptedMiniProtocol::LocalTxMonitor
        | AcceptedMiniProtocol::PeerSharing => {
            // Honest-scope: the admission pump only listens for chain-sync +
            // block-fetch (plus the keep-alive client above) in this cluster.
            // Other accepted mini-protocol frames are silently dropped; the
            // runner has no consumer for them.
        }
    }
    while let Some(ev) = out_events.pop_front() {
        emit_cooperative(
            events_out,
            ev,
            session,
            inbound,
            outbound,
            ka,
            ka_timer,
            pending_frames,
            peer_addr,
        )
        .await?;
    }
    Ok(())
}

/// LIVE-WIRE-LIVENESS S1: emission is HOISTED out of this handler. It now
/// appends to `out_events` (order-preserving) instead of awaiting a bounded
/// `events_out.send`, so the caller can drain them through `emit_cooperative`
/// with the keep-alive machinery in scope (INV-WL-1/2). Sequencing
/// (`*_in_flight`, outbox queuing) is unchanged, and events are still appended
/// BEFORE any request is queued, so the on-wire order is byte-identical.
fn handle_chain_sync(
    msg: ChainSyncMessage,
    peer_addr: &str,
    out_events: &mut VecDeque<AdmissionPeerEvent>,
    outbox: &mut VecDeque<ByteChunkIn>,
    chain_sync_in_flight: &mut bool,
    block_fetch_in_flight: &mut bool,
) -> Result<(), AdmissionWirePumpResult> {
    match msg {
        ChainSyncMessage::IntersectFound { point: _, tip } => {
            out_events.push_back(tip_update(peer_addr, tip.clone()));
            *chain_sync_in_flight = false;
            // PHASE4-N-M-FOLLOW: do NOT block-fetch the peer's
            // tip here. Chain-sync starts walking forward FROM
            // the intersect point; block-fetching the tip would
            // jump ahead and cause subsequent RollForward
            // points (which start at intersect+1) to be
            // rejected as `SlotBeforeLastApplied`. Instead,
            // request the next chain-sync message — each
            // `RollForward` will block-fetch its own point.
            queue_chain_sync_request_next(outbox);
            *chain_sync_in_flight = true;
            // Suppress unused-variable warning on block_fetch_in_flight
            // when this arm fires before any block-fetch round.
            let _ = block_fetch_in_flight;
            Ok(())
        }
        ChainSyncMessage::IntersectNotFound { tip } => {
            out_events.push_back(tip_update(peer_addr, tip));
            *chain_sync_in_flight = false;
            // Producer has nothing for us at the requested
            // intersect — request the next message anyway so the
            // protocol stays live until the upstream sends Done.
            queue_chain_sync_request_next(outbox);
            *chain_sync_in_flight = true;
            Ok(())
        }
        ChainSyncMessage::RollForward { header, tip } => {
            out_events.push_back(tip_update(peer_addr, tip));
            *chain_sync_in_flight = false;
            // PHASE4-N-M-FOLLOW: extract the rolled-forward
            // block's point from the header envelope, then
            // block-fetch it. Sequencing: do NOT queue another
            // chain-sync RequestNext here — the block-fetch
            // BatchDone handler will queue it once we've
            // received the block. This sequences chain-sync
            // and block-fetch so we never pipeline two
            // chain-sync requests while a fetch is outstanding.
            match extract_chain_sync_header_point(&header) {
                Ok(point) => {
                    queue_block_fetch_request(outbox, &point);
                    *block_fetch_in_flight = true;
                    Ok(())
                }
                Err(_) => {
                    // Header-envelope malformed → fail-closed.
                    // (Per `[[feedback-shell-must-not-overstate-semantic-truth]]`:
                    // a peer sending an undecodable header is a
                    // wire-layer violation; we exit rather than
                    // silently skip the block and break chain
                    // continuity.)
                    Err(AdmissionWirePumpResult::Error(
                        AdmissionWirePumpError::ChainSyncDecode,
                    ))
                }
            }
        }
        ChainSyncMessage::RollBackward { point, tip } => {
            // PHASE4-N-AI AI-S4a: preserve the rollback POINT as a closed event
            // (was: discarded -> TipUpdate only). A rollback is NEVER
            // represented as a TipUpdate only. `Origin` (rollback-to-genesis)
            // is unsupported for single-best-peer within k -> fail closed.
            // Latent: the live loop consumes this in AI-S4b; the pump only
            // preserves the signal here -- no orchestrator / ChainDb / forge.
            match point {
                point @ Point::Block { .. } => {
                    out_events.push_back(AdmissionPeerEvent::RollBackward {
                        peer: peer_addr.to_string(),
                        point,
                        tip,
                    });
                }
                Point::Origin => {
                    return Err(AdmissionWirePumpResult::Error(
                        AdmissionWirePumpError::UnsupportedRollbackPoint,
                    ));
                }
            }
            *chain_sync_in_flight = false;
            // Don't block-fetch; request the next chain-sync message so we can
            // pick up where the peer is going.
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

/// LIVE-WIRE-LIVENESS S1: emission hoisted to the caller (see
/// `handle_chain_sync`). Appends to `out_events` instead of awaiting a bounded
/// send, so a full downstream channel can never park the pump here.
fn handle_block_fetch(
    msg: BlockFetchMessage,
    peer_addr: &str,
    out_events: &mut VecDeque<AdmissionPeerEvent>,
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
            // CN-WIRE-12 (PHASE4-N-F-G-O): the BlockFetch wire delivers the
            // block tag-24-wrapped (`tag24(bytes([era, block]))` — the serve
            // side composes it via CN-WIRE-08). Strip the wrapper through the
            // SINGLE `ade_codec` authority before delivery, so the feed/runner
            // consumers receive bare `[era, block]` for the authoritative
            // decode. Fail-closed on a non-tag-24 / malformed payload
            // (`BlockFetchDecode` → drop the peer; never a silent pass-through).
            let bare = decompose_blockfetch_block(&bytes).map_err(|_| {
                AdmissionWirePumpResult::Error(AdmissionWirePumpError::BlockFetchDecode)
            })?;
            out_events.push_back(AdmissionPeerEvent::Block {
                peer: peer_addr.to_string(),
                block_bytes: bare.to_vec(),
            });
            Ok(())
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

/// Consume an inbound keep-alive frame (DC-PUMP-03 / PHASE4-N-AM).
///
/// The peer's `MsgResponseKeepAlive` advances the BLUE keep-alive state
/// machine from `ServerHasAgency{cookie}` back to `ClientIdle`, which
/// validates that the echoed cookie matches the outstanding request.
/// Wire-only: this NEVER emits an `AdmissionPeerEvent`. The agency is
/// fixed `Server` — the peer is the keep-alive SERVER on this connection
/// (Ade is the client), so a client-originated message from the peer
/// (`MsgKeepAlive` / `MsgDone`) is an `IllegalTransition` and fails
/// closed; an undecodable payload is a malformed keep-alive frame.
/// (NB: a responder over THIS outbound follow connection is NOT the adoption
/// path — cardano-node never runs client roles over a unidirectional inbound
/// link, so it never pulls from Ade here. Adoption is the localRoot DIAL: the
/// relay dials Ade's `--listen` serve listener (proven live 2026-06-20). See
/// docs/clusters/PRODUCER-PARTICIPANT-FOLLOW/ADOPTION-CHANNEL-TRANSPORT-ROLE-DISCOVERY.md.)
fn handle_keep_alive(
    payload: &[u8],
    keep_alive_state: &mut KeepAliveState,
    version: KeepAliveVersion,
) -> Result<(), KeepAliveError> {
    let msg = decode_keep_alive_message(payload).map_err(|_| KeepAliveError::MalformedMessage {
        reason: "keep-alive frame failed to decode",
    })?;
    let (new_state, output) =
        keep_alive_transition(*keep_alive_state, KeepAliveAgency::Server, version, msg)?;
    *keep_alive_state = new_state;
    // Wire-only diagnostic (DC-PUMP-03 / CE-AM-LIVE observability): the BLUE
    // transition validated the echoed cookie back to ClientIdle. stderr only,
    // no AdmissionPeerEvent.
    if let KeepAliveOutput::Event(KeepAliveEvent::PongReceived { cookie }) = output {
        eprintln!("keep_alive: pong cookie={} validated", cookie.0);
    }
    Ok(())
}

/// Extract the `Point` of the block referenced by a chain-sync
/// `RollForward` header envelope. PHASE4-N-M-FOLLOW.
///
/// Wire shape (cardano-node, all Praos eras):
/// ```text
/// header = [serialisationInfo: uint, encodedHeader: tag(24, bytes(header_cbor))]
/// header_cbor = [header_body, body_signature]
/// header_body = [block_no, slot, prev_hash, issuer_vkey, vrf_vkey,
///                vrf_cert, body_size, body_hash, op_cert, protocol_version]
/// ```
///
/// Returns `Point::Block { slot, hash }` where:
/// - `slot` is the header_body's 2nd uint field.
/// - `hash` is `blake2b_256(header_cbor)` — the canonical
///   block hash.
///
/// Returns `Err(())` on any structural decode failure. Honest
/// scope: this slice supports Babbage/Conway Praos headers
/// only (`array(2)[header_body, signature]` outer). Pre-Babbage
/// TPraos headers (where `header_body` is the SECOND element,
/// not the first) would parse incorrectly here — the operator
/// pass is Conway-only by design (`¬P-C5`).
fn extract_chain_sync_header_point(envelope_bytes: &[u8]) -> Result<Point, ()> {
    use ade_codec::cbor::{read_array_header, read_bytes, read_tag, read_uint, ContainerEncoding};
    use ade_types::{Hash32, SlotNo};

    let mut offset = 0usize;
    // Outer envelope: array(2) [serialisationInfo, tag(24, bytes)]
    let outer = read_array_header(envelope_bytes, &mut offset).map_err(|_| ())?;
    if !matches!(outer, ContainerEncoding::Definite(2, _)) {
        return Err(());
    }
    // Skip serialisationInfo (era discriminator uint).
    let _ = read_uint(envelope_bytes, &mut offset).map_err(|_| ())?;
    // Read tag(24).
    let tag = read_tag(envelope_bytes, &mut offset).map_err(|_| ())?;
    if tag.0 != 24 {
        return Err(());
    }
    // Read the wrapped header_cbor bytes.
    let header_cbor = read_bytes(envelope_bytes, &mut offset).map_err(|_| ())?.0;

    // block_hash = blake2b_256(header_cbor)
    let block_hash = Hash32(ade_crypto::blake2b::blake2b_256(&header_cbor).0);

    // Parse the header to extract the slot.
    let mut h_off = 0usize;
    // array(2): [header_body, body_signature]
    let h_outer = read_array_header(&header_cbor, &mut h_off).map_err(|_| ())?;
    if !matches!(h_outer, ContainerEncoding::Definite(2, _)) {
        return Err(());
    }
    // header_body: array(N) — N varies by era (e.g. Conway = 10),
    // but the first two fields are always [block_no, slot, ...].
    let _hb = read_array_header(&header_cbor, &mut h_off).map_err(|_| ())?;
    // block_no (skip)
    let _ = read_uint(&header_cbor, &mut h_off).map_err(|_| ())?;
    // slot
    let (slot, _) = read_uint(&header_cbor, &mut h_off).map_err(|_| ())?;

    Ok(Point::Block {
        slot: SlotNo(slot),
        hash: block_hash,
    })
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

async fn finalize(
    peer_addr: &str,
    result: AdmissionWirePumpResult,
    events_out: &mpsc::Sender<AdmissionPeerEvent>,
) -> AdmissionWirePumpResult {
    // Diagnostic: tag the pump exit cause on stderr so an
    // operator running with stderr captured can see WHY the pump
    // ended (PHASE4-N-M-C / A1.1 follow-up debugging). The
    // session's AdmissionPeerEvent vocabulary doesn't carry an
    // error tag; this is the smallest-footprint diagnostic.
    eprintln!("admission_wire_pump: peer={peer_addr} exit={result:?}");
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
    use ade_network::codec::block_fetch::compose_blockfetch_block;
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

    // PHASE4-N-AI AI-S4a — wire rollback signal preservation.

    #[tokio::test]
    async fn wire_pump_rollbackward_block_preserves_point() {
        let mut evs: VecDeque<AdmissionPeerEvent> = VecDeque::new();
        let mut outbox: VecDeque<ByteChunkIn> = VecDeque::new();
        let (mut csf, mut bff) = (false, false);
        let point = Point::Block {
            slot: SlotNo(100),
            hash: Hash32([0xF0; 32]),
        };
        let tip = fake_tip(105);
        let r = handle_chain_sync(
            ChainSyncMessage::RollBackward {
                point: point.clone(),
                tip: tip.clone(),
            },
            "peer",
            &mut evs,
            &mut outbox,
            &mut csf,
            &mut bff,
        );
        assert!(r.is_ok());
        match evs.pop_front().expect("a RollBackward event is emitted") {
            AdmissionPeerEvent::RollBackward {
                point: got_point,
                tip: got_tip,
                ..
            } => {
                assert_eq!(got_point, point, "the rollback point is preserved verbatim");
                assert_eq!(got_tip, tip);
            }
            _ => panic!("expected AdmissionPeerEvent::RollBackward"),
        }
    }

    #[tokio::test]
    async fn wire_pump_rollbackward_origin_fails_closed() {
        let mut evs: VecDeque<AdmissionPeerEvent> = VecDeque::new();
        let mut outbox: VecDeque<ByteChunkIn> = VecDeque::new();
        let (mut csf, mut bff) = (false, false);
        let r = handle_chain_sync(
            ChainSyncMessage::RollBackward {
                point: Point::Origin,
                tip: fake_tip(105),
            },
            "peer",
            &mut evs,
            &mut outbox,
            &mut csf,
            &mut bff,
        );
        assert!(
            matches!(
                r,
                Err(AdmissionWirePumpResult::Error(
                    AdmissionWirePumpError::UnsupportedRollbackPoint
                ))
            ),
            "rollback-to-Origin fails closed"
        );
        assert!(evs.is_empty(), "no event is emitted on Origin fail-closed");
    }

    #[tokio::test]
    async fn wire_pump_intersectfound_still_emits_tipupdate_unchanged() {
        // The TipUpdate path is unchanged by AI-S4a (only RollBackward changed).
        let mut evs: VecDeque<AdmissionPeerEvent> = VecDeque::new();
        let mut outbox: VecDeque<ByteChunkIn> = VecDeque::new();
        let (mut csf, mut bff) = (false, false);
        let tip = fake_tip(42);
        let r = handle_chain_sync(
            ChainSyncMessage::IntersectFound {
                point: Point::Origin,
                tip: tip.clone(),
            },
            "peer",
            &mut evs,
            &mut outbox,
            &mut csf,
            &mut bff,
        );
        assert!(r.is_ok());
        match evs.pop_front().expect("a TipUpdate event") {
            AdmissionPeerEvent::TipUpdate { tip: got, .. } => assert_eq!(got, tip),
            _ => panic!("expected AdmissionPeerEvent::TipUpdate"),
        }
    }

    /// Synthesize a Praos-shaped chain-sync RollForward header
    /// envelope with a given slot. Used by PHASE4-N-M-FOLLOW
    /// tests.
    ///
    /// Layout:
    ///   envelope = `82 01 D8 18 4F <inner_cbor>`
    ///   inner_cbor (15 bytes) =
    ///     `82 8A 01 19 SS SS 00 00 00 00 00 00 00 00 00`
    ///   = array(2)[array(10)[uint(1), uint(slot_u16), 8x uint(0)], uint(0)]
    fn synth_rollforward_header(slot_u16: u16) -> Vec<u8> {
        let mut inner = Vec::with_capacity(15);
        inner.push(0x82); // array(2)
        inner.push(0x8A); // array(10)
        inner.push(0x01); // block_no = 1
        inner.push(0x19); // uint(u16) follows
        inner.extend_from_slice(&slot_u16.to_be_bytes()); // slot
        // 8 placeholder fields for header_body, each a single
        // uint(0) byte.
        inner.extend(std::iter::repeat(0x00).take(8));
        // body_signature: uint(0).
        inner.push(0x00);
        assert_eq!(inner.len(), 15);

        let mut env = Vec::with_capacity(4 + 1 + inner.len());
        env.push(0x82); // array(2)
        env.push(0x01); // serialisationInfo uint(1)
        env.push(0xD8); // tag(24) prefix
        env.push(0x18); // tag(24) suffix
        env.push(0x4F); // bytes(15) header
        env.extend_from_slice(&inner);
        env
    }

    #[test]
    fn extract_chain_sync_header_point_returns_slot_and_hash() {
        let envelope = synth_rollforward_header(0x1234);
        let point = extract_chain_sync_header_point(&envelope).expect("extract");
        match point {
            Point::Block { slot, hash } => {
                assert_eq!(slot.0, 0x1234);
                // The hash is blake2b_256 of the inner CBOR
                // (envelope bytes 5..end). Recompute the
                // expected hash from the test fixture so we
                // pin the canonical formula, not a magic
                // value.
                let inner = &envelope[5..];
                let expected = ade_crypto::blake2b::blake2b_256(inner).0;
                assert_eq!(hash.0, expected);
            }
            Point::Origin => panic!("expected Block point, got Origin"),
        }
    }

    #[test]
    fn extract_chain_sync_header_point_rejects_malformed_envelope() {
        // Outer is not array(2).
        assert!(extract_chain_sync_header_point(&[0x83, 0x01, 0x02, 0x03]).is_err());
        // Wrong tag (not 24).
        let bad_tag = vec![0x82, 0x01, 0xD8, 0x42, 0x41, 0x00];
        assert!(extract_chain_sync_header_point(&bad_tag).is_err());
        // Empty input.
        assert!(extract_chain_sync_header_point(&[]).is_err());
        // Header inner not array(2).
        let mut inner = Vec::new();
        inner.push(0x81); // array(1) — wrong (must be 2)
        inner.push(0x00);
        let mut env = vec![0x82, 0x01, 0xD8, 0x18, 0x42];
        env.extend_from_slice(&inner);
        assert!(extract_chain_sync_header_point(&env).is_err());
    }

    #[tokio::test]
    async fn pump_emits_tip_update_and_request_next_on_intersect_found_no_block_fetch() {
        // PHASE4-N-M-FOLLOW: under in-order admission, an
        // IntersectFound MUST NOT trigger an immediate
        // block-fetch — it must only emit TipUpdate and
        // request the next chain-sync message. Block-fetch
        // happens on each subsequent RollForward.
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

        // Drain the initial outbound FindIntersect.
        let _ = server_transport
            .inbound
            .recv()
            .await
            .expect("client sent FindIntersect");

        // Server replies IntersectFound at tip slot=42.
        let tip = fake_tip(42);
        let if_frame = responder_frame(
            AcceptedMiniProtocol::CHAIN_SYNC_ID,
            encode_chain_sync_message(&ChainSyncMessage::IntersectFound {
                point: Point::Origin,
                tip: tip.clone(),
            }),
        );
        server_transport.outbound.send(if_frame).await.expect("send IF");

        // Pump emits TipUpdate.
        let evt = tokio::time::timeout(
            std::time::Duration::from_millis(2000),
            events_rx.recv(),
        )
        .await
        .expect("tip update")
        .expect("event");
        match evt {
            AdmissionPeerEvent::TipUpdate { tip: got, .. } => assert_eq!(got, tip),
            other => panic!("expected TipUpdate, got {other:?}"),
        }

        // Pump's next outbound MUST be a chain-sync
        // RequestNext (NOT a block-fetch). Block-fetch only
        // fires on each subsequent RollForward.
        let next_outbound = tokio::time::timeout(
            std::time::Duration::from_millis(2000),
            server_transport.inbound.recv(),
        )
        .await
        .expect("client sent next outbound")
        .expect("frame bytes");
        let (frame, _) = ade_network::mux::frame::decode_frame(&next_outbound)
            .expect("frame decodes");
        assert_eq!(
            frame.header.mini_protocol_id.get(),
            AcceptedMiniProtocol::CHAIN_SYNC_ID,
            "post-IntersectFound outbound must be chain-sync RequestNext, not block-fetch"
        );
        let cs_msg = decode_chain_sync_message(&frame.payload).expect("decode cs");
        assert!(matches!(cs_msg, ChainSyncMessage::RequestNext));

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

    // PHASE4-N-AM (DC-PUMP-03) — wire-pump keep-alive client.

    #[tokio::test(start_paused = true)]
    async fn wire_pump_sends_keep_alive_on_quiescent_cadence() {
        // CE-AM-1: during inbound quiescence the pump sends MsgKeepAlive on
        // the cadence so the peer's keep-alive responder does not time us
        // out at ~97s. Under start_paused the virtual clock auto-advances to
        // the interval deadline; the keep-alive then travels the loopback
        // socket. Proves AM-1 (the pump sends MsgKeepAlive under the
        // deadline) + AM-2 (wire-only: no AdmissionPeerEvent).
        let (client_stream, server_stream) = loopback_pair().await;
        let client_transport = spawn_duplex(client_stream, DuplexCapacity::DEFAULT);
        let mut server_transport = spawn_duplex(server_stream, DuplexCapacity::DEFAULT);

        let (events_tx, mut events_rx) = mpsc::channel::<AdmissionPeerEvent>(16);

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

        // Drain the initial FindIntersect, then stay QUIESCENT (no reply).
        let _ = server_transport
            .inbound
            .recv()
            .await
            .expect("client sent FindIntersect");

        // After the keep-alive cadence elapses (virtual clock auto-advances),
        // the next outbound is a keep-alive frame on mini-protocol id 8
        // carrying MsgKeepAlive. The 60s virtual timeout is a clean failure
        // mode — the 20s interval fires first under auto-advance.
        let ka_outbound = tokio::time::timeout(
            Duration::from_secs(60),
            server_transport.inbound.recv(),
        )
        .await
        .expect("keep-alive sent within the cadence")
        .expect("frame bytes");
        let (frame, _) =
            ade_network::mux::frame::decode_frame(&ka_outbound).expect("frame decodes");
        assert_eq!(
            frame.header.mini_protocol_id.get(),
            AcceptedMiniProtocol::KEEP_ALIVE_ID,
            "quiescent-cadence outbound must be a keep-alive frame (proto 8)"
        );
        match decode_keep_alive_message(&frame.payload).expect("decode keep-alive") {
            KeepAliveMessage::KeepAlive(_) => {}
            other => panic!("expected MsgKeepAlive, got {other:?}"),
        }

        // Wire-only: the keep-alive send emitted NO AdmissionPeerEvent.
        assert!(
            events_rx.try_recv().is_err(),
            "keep-alive is wire-only — no AdmissionPeerEvent on send"
        );

        drop(server_transport);
        let _ = pump_handle.await;
    }

    // LIVE-WIRE-LIVENESS S1 (INV-WL-1/2/3) — keep-alive under downstream
    // backpressure. Regression cover for the 2026-08-01 03:44:11Z live preview
    // shutdown: `ExceededTimeLimit (KeepAlive) ClientHasAgency`, 2m52s after
    // handshake, because the pump was parked on a bounded `events_out.send`
    // OUTSIDE its `select!` while the consumer folded an epoch boundary.

    /// CE-WL-1. With `events_out` full and NEVER drained, `emit_cooperative`
    /// must still send `MsgKeepAlive` on cadence AND consume the echoed
    /// response, so ping after ping goes out for an unbounded stall.
    ///
    /// The strictness is in the COOKIES. Servicing only the cadence timer would
    /// emit exactly one ping and then wedge in `ClientAwaiting` (the agency
    /// guard suppresses every later tick) — which buys ~117s and then dies.
    /// Three pings with strictly increasing cookies can only happen if each
    /// echoed response was consumed *while parked on a full `events_out`*.
    ///
    /// Driven over in-memory channels rather than a loopback socket: under
    /// `start_paused` the virtual clock auto-advances past real TCP delivery,
    /// so a socket-based version races. This exercises the same code path.
    #[tokio::test(start_paused = true)]
    async fn ce_wl_1_keep_alive_survives_unbounded_downstream_stall() {
        // Capacity 1, pre-filled, never drained => `reserve()` pends forever.
        let (events_tx, _events_rx) = mpsc::channel::<AdmissionPeerEvent>(1);
        events_tx
            .send(tip_update("peer", fake_tip(1)))
            .await
            .expect("prefill to capacity");

        let (in_tx, mut in_rx) = mpsc::channel::<Vec<u8>>(16);
        let (out_tx, mut out_rx) = mpsc::channel::<Vec<u8>>(64);

        let mut session = post_handshake_state(14, MAINNET_NETWORK_MAGIC);
        let mut ka = KeepAliveLane {
            state: KeepAliveState::ClientIdle,
            next_cookie: 0,
            version: KeepAliveVersion::new(14),
        };
        let mut timer = tokio::time::interval(KEEP_ALIVE_CADENCE);
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        timer.tick().await;
        let mut pending: VecDeque<(AcceptedMiniProtocol, Vec<u8>)> = VecDeque::new();

        // This never returns: capacity never frees. The timeout ends it.
        let emit_fut = tokio::time::timeout(
            Duration::from_secs(600),
            emit_cooperative(
                &events_tx,
                tip_update("peer", fake_tip(2)),
                &mut session,
                &mut in_rx,
                &out_tx,
                &mut ka,
                &mut timer,
                &mut pending,
                "peer",
            ),
        );

        // Peer side: echo every keep-alive so the BLUE lane returns to
        // `ClientIdle`, exactly as a real relay responder does.
        let echo_fut = async {
            let mut cookies: Vec<u16> = Vec::new();
            while cookies.len() < 3 {
                let Some(bytes) = out_rx.recv().await else { break };
                let (frame, _) =
                    ade_network::mux::frame::decode_frame(&bytes).expect("frame decodes");
                if frame.header.mini_protocol_id.get() != AcceptedMiniProtocol::KEEP_ALIVE_ID {
                    continue;
                }
                match decode_keep_alive_message(&frame.payload).expect("decode keep-alive") {
                    KeepAliveMessage::KeepAlive(c) => {
                        cookies.push(c.0);
                        let resp =
                            encode_keep_alive_message(&KeepAliveMessage::ResponseKeepAlive(c));
                        in_tx
                            .send(responder_frame(AcceptedMiniProtocol::KEEP_ALIVE_ID, resp))
                            .await
                            .expect("peer echoes the keep-alive");
                    }
                    other => panic!("expected MsgKeepAlive, got {other:?}"),
                }
            }
            cookies
        };

        let (_, cookies) = tokio::join!(emit_fut, echo_fut);

        assert_eq!(
            cookies.len(),
            3,
            "keep-alive must keep flowing while parked on a full events_out, got {cookies:?}"
        );
        assert!(
            cookies[0] < cookies[1] && cookies[1] < cookies[2],
            "cookies must strictly increase ({cookies:?}) — proving each echoed \
             response was consumed WHILE parked. A send-only fix wedges in \
             ClientAwaiting after the first ping."
        );
    }

    /// CE-WL-2. Backpressure must not reorder the event stream: a slow consumer
    /// (capacity 1, drained one at a time) sees exactly the same
    /// `AdmissionPeerEvent` sequence as an unstalled run.
    #[tokio::test(start_paused = true)]
    async fn ce_wl_2_backpressure_preserves_event_order() {
        let (client_stream, server_stream) = loopback_pair().await;
        let client_transport = spawn_duplex(client_stream, DuplexCapacity::DEFAULT);
        let mut server_transport = spawn_duplex(server_stream, DuplexCapacity::DEFAULT);

        let (events_tx, mut events_rx) = mpsc::channel::<AdmissionPeerEvent>(1);

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

        let _ = server_transport
            .inbound
            .recv()
            .await
            .expect("client sent FindIntersect");

        // Push several tip-bearing replies back-to-back so the bounded channel
        // is saturated and the pump must defer frames.
        let slots: Vec<u64> = (100..106).collect();
        for slot in &slots {
            let payload = encode_chain_sync_message(&ChainSyncMessage::IntersectFound {
                point: Point::Origin,
                tip: fake_tip(*slot),
            });
            server_transport
                .outbound
                .send(responder_frame(AcceptedMiniProtocol::CHAIN_SYNC_ID, payload))
                .await
                .expect("server sends IntersectFound");
        }

        // Drain one at a time — each recv frees exactly one permit.
        let mut seen: Vec<u64> = Vec::new();
        while seen.len() < slots.len() {
            let evt = tokio::time::timeout(Duration::from_secs(120), events_rx.recv())
                .await
                .expect("events keep flowing")
                .expect("channel open");
            if let AdmissionPeerEvent::TipUpdate { tip, .. } = evt {
                if let Point::Block { slot, .. } = tip.point {
                    seen.push(slot.0);
                }
            }
        }
        assert_eq!(
            seen, slots,
            "TipUpdate order under backpressure must match wire order exactly"
        );

        drop(server_transport);
        let _ = pump_handle.await;
    }

    /// CE-WL-3. The deferral queue is bounded: once `MAX_DEFERRED_PEER_FRAMES`
    /// frames are outstanding, a further deferred frame fails closed rather
    /// than growing without limit.
    #[tokio::test(start_paused = true)]
    async fn ce_wl_3_deferred_frames_are_bounded_and_fail_closed() {
        // events_out full => `emit_cooperative` parks and takes the defer path.
        let (events_tx, _events_rx) = mpsc::channel::<AdmissionPeerEvent>(1);
        events_tx
            .send(tip_update("peer", fake_tip(1)))
            .await
            .expect("prefill to capacity");

        let (in_tx, mut in_rx) = mpsc::channel::<Vec<u8>>(8);
        let (out_tx, _out_rx) = mpsc::channel::<Vec<u8>>(64);

        let mut session = post_handshake_state(14, MAINNET_NETWORK_MAGIC);
        let mut ka = KeepAliveLane {
            state: KeepAliveState::ClientIdle,
            next_cookie: 0,
            version: KeepAliveVersion::new(14),
        };
        let mut timer = tokio::time::interval(KEEP_ALIVE_CADENCE);
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        timer.tick().await;

        // Pre-fill the queue to the cap, then hand the pump one more frame.
        let mut pending: VecDeque<(AcceptedMiniProtocol, Vec<u8>)> = VecDeque::new();
        for _ in 0..MAX_DEFERRED_PEER_FRAMES {
            pending.push_back((AcceptedMiniProtocol::ChainSync, vec![]));
        }
        let payload = encode_chain_sync_message(&ChainSyncMessage::IntersectFound {
            point: Point::Origin,
            tip: fake_tip(7),
        });
        in_tx
            .send(responder_frame(AcceptedMiniProtocol::CHAIN_SYNC_ID, payload))
            .await
            .expect("queue one more inbound frame");

        let res = emit_cooperative(
            &events_tx,
            tip_update("peer", fake_tip(2)),
            &mut session,
            &mut in_rx,
            &out_tx,
            &mut ka,
            &mut timer,
            &mut pending,
            "peer",
        )
        .await;

        assert!(
            matches!(
                res,
                Err(AdmissionWirePumpResult::Error(
                    AdmissionWirePumpError::DeferredFrameOverflow
                ))
            ),
            "deferral past the cap must fail closed, got {res:?}"
        );
        assert_eq!(
            pending.len(),
            MAX_DEFERRED_PEER_FRAMES,
            "the overflowing frame is NOT buffered"
        );
    }

    #[test]
    fn wire_pump_keep_alive_response_validates_cookie_no_event() {
        // CE-AM-2: an inbound MsgResponseKeepAlive matching the outstanding
        // cookie advances the BLUE state machine back to ClientIdle. The
        // handler has NO event channel parameter — wire-only by construction
        // (it cannot emit an AdmissionPeerEvent).
        let cookie = KeepAliveCookie(0x1234);
        let mut state = KeepAliveState::ServerHasAgency { cookie };
        let version = KeepAliveVersion::new(14);
        let payload = encode_keep_alive_message(&KeepAliveMessage::ResponseKeepAlive(cookie));
        handle_keep_alive(&payload, &mut state, version).expect("matched cookie validates");
        assert_eq!(
            state,
            KeepAliveState::ClientIdle,
            "back to ClientIdle after a valid pong"
        );
    }

    #[test]
    fn wire_pump_keep_alive_cookie_mismatch_fails_closed() {
        // CE-AM-3: every keep-alive grammar violation fails closed.
        let version = KeepAliveVersion::new(14);

        // (a) a mismatched cookie against an outstanding request.
        let mut state = KeepAliveState::ServerHasAgency {
            cookie: KeepAliveCookie(0x1111),
        };
        let bogus =
            encode_keep_alive_message(&KeepAliveMessage::ResponseKeepAlive(KeepAliveCookie(0x2222)));
        let err = handle_keep_alive(&bogus, &mut state, version)
            .expect_err("mismatched cookie must fail closed");
        assert!(matches!(err, KeepAliveError::MalformedMessage { .. }));

        // (b) a response while ClientIdle (no outstanding request).
        let mut idle = KeepAliveState::ClientIdle;
        let resp =
            encode_keep_alive_message(&KeepAliveMessage::ResponseKeepAlive(KeepAliveCookie(7)));
        let err2 = handle_keep_alive(&resp, &mut idle, version)
            .expect_err("unsolicited response must fail closed");
        assert!(matches!(err2, KeepAliveError::IllegalTransition { .. }));

        // (c) an undecodable payload.
        let mut st = KeepAliveState::ServerHasAgency {
            cookie: KeepAliveCookie(1),
        };
        let err3 = handle_keep_alive(&[], &mut st, version)
            .expect_err("undecodable payload must fail closed");
        assert!(matches!(err3, KeepAliveError::MalformedMessage { .. }));
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

    /// PHASE4-N-M-FOLLOW: after the initial IntersectFound +
    /// block-fetch round, a chain-sync `RollForward` MUST cause
    /// the pump to block-fetch the rolled-forward block AND
    /// hold off on chain-sync RequestNext until the
    /// block-fetch BatchDone arrives.
    #[tokio::test]
    async fn rollforward_drives_block_fetch_then_request_next() {
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

        // 1. Drain the initial FindIntersect frame.
        let _ = server_transport
            .inbound
            .recv()
            .await
            .expect("client sent FindIntersect");

        // 2. Server replies IntersectFound + Tip(slot=42).
        let tip0 = fake_tip(42);
        let if_frame = responder_frame(
            AcceptedMiniProtocol::CHAIN_SYNC_ID,
            encode_chain_sync_message(&ChainSyncMessage::IntersectFound {
                point: Point::Origin,
                tip: tip0.clone(),
            }),
        );
        server_transport.outbound.send(if_frame).await.expect("send IF");

        // Pump emits TipUpdate(tip0).
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(2000),
            events_rx.recv(),
        )
        .await
        .expect("tip0")
        .expect("event");

        // 3. Drain the initial block-fetch RequestRange.
        let _ = server_transport
            .inbound
            .recv()
            .await
            .expect("client sent initial BF RequestRange");

        // 4. Server replies StartBatch + Block(B0) + BatchDone.
        let block0_bytes = {
            // A tag-24-wrapped MsgBlock payload:
            // tag(24, bytes(4))[DE AD BE EF]. The WIRE PUMP
            // strips the tag-24 wrapper (CN-WIRE-12), emitting
            // the bare inner bytes; this test only asserts a
            // Block event is emitted (not the runner-side decode
            // of the inner bytes).
            let mut b = Vec::new();
            b.push(0xD8);
            b.push(0x18);
            b.push(0x44);
            b.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
            b
        };
        for msg in [
            BlockFetchMessage::StartBatch,
            BlockFetchMessage::Block {
                bytes: block0_bytes.clone(),
            },
            BlockFetchMessage::BatchDone,
        ] {
            let f = responder_frame(
                AcceptedMiniProtocol::BLOCK_FETCH_ID,
                encode_block_fetch_message(&msg),
            );
            server_transport.outbound.send(f).await.expect("send BF");
        }

        // Pump emits Block(B0).
        let evt = tokio::time::timeout(
            std::time::Duration::from_millis(2000),
            events_rx.recv(),
        )
        .await
        .expect("block0")
        .expect("event");
        assert!(matches!(evt, AdmissionPeerEvent::Block { .. }));

        // 5. After BatchDone, pump issues chain-sync RequestNext.
        let _ = server_transport
            .inbound
            .recv()
            .await
            .expect("client sent RequestNext post-BatchDone");

        // 6. Server replies RollForward { header_with_slot=4660, tip1 }.
        let header_env = synth_rollforward_header(0x1234);
        let tip1 = fake_tip(4660);
        let rf_frame = responder_frame(
            AcceptedMiniProtocol::CHAIN_SYNC_ID,
            encode_chain_sync_message(&ChainSyncMessage::RollForward {
                header: header_env,
                tip: tip1.clone(),
            }),
        );
        server_transport.outbound.send(rf_frame).await.expect("send RF");

        // Pump emits TipUpdate(tip1).
        let evt = tokio::time::timeout(
            std::time::Duration::from_millis(2000),
            events_rx.recv(),
        )
        .await
        .expect("tip1")
        .expect("event");
        match evt {
            AdmissionPeerEvent::TipUpdate { tip: got, .. } => {
                assert_eq!(got, tip1);
            }
            other => panic!("expected TipUpdate, got {other:?}"),
        }

        // 7. KEY ASSERTION: pump must send block-fetch
        // RequestRange for the rolled-forward block — NOT a
        // chain-sync RequestNext.
        let next_outbound = tokio::time::timeout(
            std::time::Duration::from_millis(2000),
            server_transport.inbound.recv(),
        )
        .await
        .expect("client sent next outbound")
        .expect("frame bytes");
        // Decode the mux frame to learn which protocol the
        // pump used.
        let (frame, _) = ade_network::mux::frame::decode_frame(&next_outbound)
            .expect("frame parses");
        assert_eq!(
            frame.header.mini_protocol_id.get(),
            AcceptedMiniProtocol::BLOCK_FETCH_ID,
            "after RollForward, pump must block-fetch the rolled-forward block, not RequestNext"
        );
        // Decode the payload to confirm it's RequestRange.
        let bf_msg = decode_block_fetch_message(&frame.payload).expect("decode bf");
        match bf_msg {
            BlockFetchMessage::RequestRange(Range { from, to }) => {
                // The point must match the header's extracted
                // (slot=4660, hash=blake2b256(inner_cbor)).
                match (from, to) {
                    (BfPoint::Block { slot: from_slot, .. }, BfPoint::Block { slot: to_slot, .. }) => {
                        assert_eq!(from_slot.0, 4660);
                        assert_eq!(to_slot.0, 4660);
                    }
                    other => panic!("expected Block points, got {other:?}"),
                }
            }
            other => panic!("expected RequestRange, got {other:?}"),
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

    /// CN-WIRE-12 (PHASE4-N-F-G-O): the block-fetch receive handler strips the
    /// protocol tag-24 wrapper via the single `ade_codec` authority before
    /// delivery, so the emitted `AdmissionPeerEvent::Block` carries the BARE
    /// `[era, block]` storage bytes — NOT the wrapped wire payload. (The serve
    /// side composes the wrapper via `compose_blockfetch_block` / CN-WIRE-08;
    /// this is its receive-side mirror.)
    #[tokio::test]
    async fn block_fetch_unwraps_tag24_emitting_bare_block() {
        let mut evs: VecDeque<AdmissionPeerEvent> = VecDeque::new();
        let mut outbox: VecDeque<ByteChunkIn> = VecDeque::new();
        let mut cs_in_flight = false;
        let mut bf_in_flight = true;

        // Bare [era 7, []] stand-in storage bytes, wrapped by the serve-side
        // authority — the exact shape the BlockFetch wire delivers.
        let bare = vec![0x82u8, 0x07, 0x80];
        let wrapped = compose_blockfetch_block(&bare);
        assert_eq!(
            &wrapped[0..2],
            &[0xd8, 0x18],
            "the wire payload is tag-24-wrapped (d8 18)"
        );

        handle_block_fetch(
            BlockFetchMessage::Block { bytes: wrapped },
            "127.0.0.1:0",
            &mut evs,
            &mut outbox,
            &mut cs_in_flight,
            &mut bf_in_flight,
        )
        .expect("a valid tag-24 block-fetch payload unwraps and emits");

        match evs.pop_front().expect("a Block event is emitted") {
            AdmissionPeerEvent::Block { block_bytes, .. } => assert_eq!(
                block_bytes, bare,
                "the emitted block is the tag-24-UNWRAPPED bare [era, block] \
                 (CN-WIRE-12), never the wire wrapper"
            ),
            other => panic!("expected a Block event, got {other:?}"),
        }
    }

    /// CN-WIRE-12 fail-closed: a non-tag-24 `MsgBlock` payload (where the
    /// BlockFetch protocol requires the tag-24 wrapper) is rejected as a
    /// structured `BlockFetchDecode` error — never a silent pass-through of the
    /// unwrapped-but-not-actually-wrapped bytes, and no `Block` event emitted.
    #[tokio::test]
    async fn block_fetch_fails_closed_on_non_tag24_payload() {
        let mut evs: VecDeque<AdmissionPeerEvent> = VecDeque::new();
        let mut outbox: VecDeque<ByteChunkIn> = VecDeque::new();
        let mut cs_in_flight = false;
        let mut bf_in_flight = true;

        // Bare [era, []] with NO tag-24 wrapper — a protocol violation on the
        // BlockFetch receive path.
        let non_tag24 = vec![0x82u8, 0x07, 0x80];

        let res = handle_block_fetch(
            BlockFetchMessage::Block { bytes: non_tag24 },
            "127.0.0.1:0",
            &mut evs,
            &mut outbox,
            &mut cs_in_flight,
            &mut bf_in_flight,
        );

        assert!(
            matches!(
                res,
                Err(AdmissionWirePumpResult::Error(
                    AdmissionWirePumpError::BlockFetchDecode
                ))
            ),
            "a non-tag-24 payload must fail closed with BlockFetchDecode, got {res:?}"
        );
        assert!(
            evs.is_empty(),
            "no Block event is emitted on a fail-closed unwrap"
        );
    }
}
