// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED wire-only live smoke pass (PHASE4-N-L-LIVE S2).
//!
//! Composes:
//!   1. TCP `connect`.
//!   2. `ade_network::mux::transport::spawn_duplex` — bounded
//!      full-duplex socket.
//!   3. `ade_network::session::run_n2n_handshake_initiator`
//!      (sync; ridden inside `tokio::task::spawn_blocking`).
//!   4. One chain-sync `FindIntersect(Origin)` frame on the wire.
//!   5. Read one `IntersectFound` / `IntersectNotFound` reply
//!      → emit `LiveLogEvent::PeerTipRead`.
//!   6. Send chain-sync `Done`, close cleanly.
//!
//! **DOES NOT** call `bootstrap_initial_state`. **DOES NOT** emit
//! any of `agreement_verdict` / `admitted_block` / `ledger_applied`
//! / `projection_updated` — the closed `LiveLogEvent` enum forbids
//! it (DC-NET-equivalent type-level + CI grep).

use std::io::Write;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use ade_network::codec::chain_sync::{
    decode_chain_sync_message, encode_chain_sync_message, ChainSyncMessage, Point as CsPoint,
};
use ade_network::codec::handshake::{VersionParams, VersionTable};
use ade_network::codec::version::N2NVersion;
use ade_network::handshake::version_table::{
    MAINNET_NETWORK_MAGIC, N2N_SUPPORTED,
};
use ade_network::mux::frame::{
    decode_frame, encode_frame, MiniProtocolId, MuxError, MuxFrame, MuxHeader, MuxMode, HEADER_LEN,
};
use ade_network::mux::transport::{
    spawn_duplex, DuplexCapacity, MuxTransportHandle, TransportError,
};
use ade_network::session::{
    run_n2n_handshake_initiator, Transport, TransportError as SessionTransportError,
};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch, Mutex};

use crate::cli::{Cli, Mode};
use crate::live_log::{
    LiveLogEvent, LiveLogWriter, ModeTag, PeerDialFailureKind, WireOnlyShutdownReason,
};

/// Live-pass peer-failure exit code. EXIT_SUCCESS (0) iff every
/// dialed peer completed its handshake + tip-read.
pub const EXIT_LIVE_PASS_PEER_FAILURE: i32 = 20;

/// Per-peer outcome — the per-peer task returns this so the outer
/// loop can aggregate counts for `WireSmokeComplete`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerOutcome {
    Succeeded,
    Failed,
}

/// Drive a complete wire-only run. Returns an exit code; the
/// `main.rs` wrapper passes it through to `ExitCode`.
///
/// The writer is held behind an `Arc<Mutex<...>>` so per-peer
/// tasks can share it; the JSON serializer flushes after every
/// emit so concurrent writes interleave at line granularity but
/// never within a line.
pub async fn run_wire_only<W: Write + Send + 'static>(
    cli: &Cli,
    writer: LiveLogWriter<W>,
    shutdown: watch::Receiver<bool>,
) -> ExitCode {
    let writer = Arc::new(Mutex::new(writer));

    emit(
        &writer,
        LiveLogEvent::NodeStarted {
            mode: ModeTag::WireOnly,
            peer_count: cli.peer_addrs.len() as u32,
        },
    )
    .await;

    let timeout = Duration::from_secs(cli.tip_read_timeout_secs as u64);
    let network_magic = network_name_to_magic(&cli.network);

    let mut per_peer = Vec::new();
    for peer in &cli.peer_addrs {
        let peer_addr = peer.clone();
        let writer = writer.clone();
        let shutdown = shutdown.clone();
        per_peer.push(tokio::spawn(async move {
            wire_only_peer_session(peer_addr, network_magic, timeout, writer, shutdown).await
        }));
    }

    let mut ok = 0u32;
    let mut failed = 0u32;
    for h in per_peer {
        match h.await.unwrap_or(PeerOutcome::Failed) {
            PeerOutcome::Succeeded => ok += 1,
            PeerOutcome::Failed => failed += 1,
        }
    }

    emit(
        &writer,
        LiveLogEvent::WireSmokeComplete {
            admission_enabled: false,
            peer_count_ok: ok,
            peer_count_failed: failed,
        },
    )
    .await;

    let reason = if *shutdown.borrow() {
        WireOnlyShutdownReason::SignalReceived
    } else if failed > 0 {
        WireOnlyShutdownReason::PeerDialFailure
    } else {
        WireOnlyShutdownReason::TipReadComplete
    };
    emit(&writer, LiveLogEvent::NodeShutdown { reason }).await;

    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(EXIT_LIVE_PASS_PEER_FAILURE as u8)
    }
}

/// Fail-closed entry point for `--mode admission` until the
/// ledger-seed cluster lands. Emits a structured shutdown event
/// and exits with the generic-startup code.
pub async fn run_admission_unavailable<W: Write + Send + 'static>(
    writer: LiveLogWriter<W>,
) -> ExitCode {
    let writer = Arc::new(Mutex::new(writer));
    emit(
        &writer,
        LiveLogEvent::NodeStarted {
            mode: ModeTag::WireOnly,
            peer_count: 0,
        },
    )
    .await;
    emit(
        &writer,
        LiveLogEvent::NodeShutdown {
            reason: WireOnlyShutdownReason::LedgerSeedUnavailable,
        },
    )
    .await;
    ExitCode::from(crate::node::EXIT_GENERIC_STARTUP as u8)
}

async fn emit<W: Write + Send>(
    writer: &Arc<Mutex<LiveLogWriter<W>>>,
    event: LiveLogEvent,
) {
    let mut w = writer.lock().await;
    let _ = w.emit(&event);
}

async fn wire_only_peer_session<W: Write + Send + 'static>(
    peer: String,
    network_magic: u32,
    timeout: Duration,
    writer: Arc<Mutex<LiveLogWriter<W>>>,
    _shutdown: watch::Receiver<bool>,
) -> PeerOutcome {
    emit(
        &writer,
        LiveLogEvent::PeerDialStarted { peer: peer.clone() },
    )
    .await;

    // 1. TCP connect.
    let stream = match TcpStream::connect(&peer).await {
        Ok(s) => s,
        Err(e) => {
            emit(
                &writer,
                LiveLogEvent::PeerDialFailed {
                    peer: peer.clone(),
                    kind: PeerDialFailureKind::TcpConnectFailed,
                    detail: format!("{:?}", e.kind()),
                },
            )
            .await;
            return PeerOutcome::Failed;
        }
    };

    // 2. Spawn duplex transport.
    let MuxTransportHandle {
        inbound,
        outbound,
        reader_handle,
        writer_handle,
    } = spawn_duplex(stream, DuplexCapacity::DEFAULT);

    // 3. Handshake initiator (sync; ridden inside spawn_blocking).
    let our_versions = our_n2n_versions(network_magic);
    let (inbound, outbound, hs_result) = tokio::task::spawn_blocking(move || {
        let mut bt = BlockingTransport::new(inbound, outbound);
        let r = run_n2n_handshake_initiator(&mut bt, our_versions);
        let (i, o) = bt.into_halves();
        (i, o, r)
    })
    .await
    .unwrap_or_else(|_| {
        // join-error path — treat as orchestrator-dropped.
        (
            mpsc::channel::<Vec<u8>>(1).1,
            mpsc::channel::<Vec<u8>>(1).0,
            Err(SessionTransportError::Io),
        )
    });

    let negotiated_version = match hs_result {
        Ok(n) => n.version,
        Err(e) => {
            emit(
                &writer,
                LiveLogEvent::PeerDialFailed {
                    peer: peer.clone(),
                    kind: PeerDialFailureKind::HandshakeRejected,
                    detail: format!("{e:?}"),
                },
            )
            .await;
            reader_handle.abort();
            writer_handle.abort();
            return PeerOutcome::Failed;
        }
    };
    emit(
        &writer,
        LiveLogEvent::HandshakeOk {
            peer: peer.clone(),
            negotiated_version,
        },
    )
    .await;

    // 4. Send chain-sync FindIntersect(Origin) frame.
    let cs_find_intersect = encode_chain_sync_message(&ChainSyncMessage::FindIntersect {
        points: vec![CsPoint::Origin],
    });
    let frame_bytes = match encode_chain_sync_mux_frame(cs_find_intersect, MuxMode::Initiator) {
        Ok(b) => b,
        Err(e) => {
            emit(
                &writer,
                LiveLogEvent::PeerDialFailed {
                    peer: peer.clone(),
                    kind: PeerDialFailureKind::TipReadProtocolError,
                    detail: format!("encode_frame {e:?}"),
                },
            )
            .await;
            reader_handle.abort();
            writer_handle.abort();
            return PeerOutcome::Failed;
        }
    };
    if outbound.send(frame_bytes).await.is_err() {
        emit(
            &writer,
            LiveLogEvent::PeerDialFailed {
                peer: peer.clone(),
                kind: PeerDialFailureKind::OrchestratorDropped,
                detail: "outbound channel dropped".to_string(),
            },
        )
        .await;
        reader_handle.abort();
        writer_handle.abort();
        return PeerOutcome::Failed;
    }

    // 5. Read tip from the reply, bounded by --tip-read-timeout-secs.
    let tip_outcome = tokio::time::timeout(
        timeout,
        read_first_chain_sync_intersect_reply(inbound),
    )
    .await;

    let (tip_slot, tip_hash_hex, tip_block_no) = match tip_outcome {
        Ok(Ok(t)) => t,
        Ok(Err(kind)) => {
            emit(
                &writer,
                LiveLogEvent::PeerDialFailed {
                    peer: peer.clone(),
                    kind,
                    detail: String::new(),
                },
            )
            .await;
            reader_handle.abort();
            writer_handle.abort();
            return PeerOutcome::Failed;
        }
        Err(_) => {
            emit(
                &writer,
                LiveLogEvent::PeerDialFailed {
                    peer: peer.clone(),
                    kind: PeerDialFailureKind::TipReadTimeout,
                    detail: format!("{}s", timeout.as_secs()),
                },
            )
            .await;
            reader_handle.abort();
            writer_handle.abort();
            return PeerOutcome::Failed;
        }
    };

    emit(
        &writer,
        LiveLogEvent::PeerTipRead {
            peer: peer.clone(),
            slot: tip_slot,
            hash_hex: tip_hash_hex,
            block_no: tip_block_no,
        },
    )
    .await;

    // 6. Send chain-sync Done, close cleanly.
    let done_bytes = match encode_chain_sync_mux_frame(
        encode_chain_sync_message(&ChainSyncMessage::Done),
        MuxMode::Initiator,
    ) {
        Ok(b) => b,
        Err(_) => Vec::new(),
    };
    let _ = outbound.send(done_bytes).await;
    drop(outbound);
    reader_handle.abort();
    writer_handle.abort();

    PeerOutcome::Succeeded
}

fn our_n2n_versions(network_magic: u32) -> VersionTable {
    // Build per-version structured NodeToNodeVersionData per the
    // ouroboros-network spec. Mirrors the canonical encoder in
    // `crates/ade_network/src/bin/capture_handshake.rs::version_params_for_n2n`:
    //   V11..V15: array(4) [magic, initiatorOnlyDiffusion(true),
    //                       peerSharing(NoPeerSharing=0), query(false)]
    //   V16+:     array(5) above + perasSupport(false)
    // The codec passes VersionParams bytes verbatim; the responder
    // decodes them as a CBOR record matching its supported
    // VersionData schema.
    use ade_network::codec::primitives::{encode_array_header, encode_bool, encode_u64};
    VersionTable(
        N2N_SUPPORTED
            .iter()
            .map(|(v, _)| {
                let mut buf = Vec::new();
                let field_count: u64 = if *v >= 16 { 5 } else { 4 };
                encode_array_header(&mut buf, field_count);
                encode_u64(&mut buf, network_magic as u64);
                encode_bool(&mut buf, true); // initiatorOnlyDiffusionMode
                encode_u64(&mut buf, 0);     // peerSharing = NoPeerSharing
                encode_bool(&mut buf, false); // query
                if *v >= 16 {
                    encode_bool(&mut buf, false); // perasSupport
                }
                (N2NVersion::new(*v), VersionParams(buf))
            })
            .collect(),
    )
}

fn network_name_to_magic(name: &str) -> u32 {
    match name {
        "mainnet" => MAINNET_NETWORK_MAGIC,
        "preprod" => 1,
        "preview" => 2,
        // Operator-supplied magic numbers (parsed as a decimal string)
        // are honored if the name is itself a number.
        other => other.parse::<u32>().unwrap_or(MAINNET_NETWORK_MAGIC),
    }
}

fn encode_chain_sync_mux_frame(
    payload: Vec<u8>,
    mode: MuxMode,
) -> Result<Vec<u8>, MuxError> {
    let length = payload.len() as u16;
    let frame = MuxFrame {
        header: MuxHeader {
            timestamp: 0,
            mode,
            mini_protocol_id: MiniProtocolId::new(2).expect("chain-sync id=2 in range"),
            length,
        },
        payload,
    };
    encode_frame(&frame)
}

/// Read the first chain-sync IntersectFound / IntersectNotFound
/// reply from a stream of inbound byte chunks. Returns
/// `(slot, hash_hex, block_no)` on success.
async fn read_first_chain_sync_intersect_reply(
    mut inbound: mpsc::Receiver<Vec<u8>>,
) -> Result<(u64, String, u64), PeerDialFailureKind> {
    let mut buffer: Vec<u8> = Vec::new();
    loop {
        // Try to pop one full chain-sync mux frame from the buffer.
        if buffer.len() >= HEADER_LEN {
            match decode_frame(&buffer) {
                Ok((frame, rest_len)) => {
                    let consumed = buffer.len() - rest_len.len();
                    if frame.header.mini_protocol_id.get() == 2 {
                        let msg = decode_chain_sync_message(&frame.payload)
                            .map_err(|_| PeerDialFailureKind::TipReadProtocolError)?;
                        match msg {
                            ChainSyncMessage::IntersectFound { tip, .. }
                            | ChainSyncMessage::IntersectNotFound { tip } => {
                                let (slot, hash_hex) = match tip.point {
                                    CsPoint::Origin => (0u64, String::new()),
                                    CsPoint::Block { slot, hash } => {
                                        (slot.0, hex_lowercase(&hash.0))
                                    }
                                };
                                return Ok((slot, hash_hex, tip.block_no));
                            }
                            _ => {
                                // Unexpected mid-handshake-of-tip-read
                                // chain-sync msg → protocol error.
                                return Err(PeerDialFailureKind::TipReadProtocolError);
                            }
                        }
                    }
                    // Not a chain-sync frame; drop it (it's likely
                    // a duplicate handshake frame or keep-alive).
                    buffer.drain(..consumed);
                }
                Err(MuxError::Truncated { .. }) => {
                    // Need more bytes.
                }
                Err(_) => return Err(PeerDialFailureKind::TipReadProtocolError),
            }
        }

        // Pull more bytes.
        match inbound.recv().await {
            Some(chunk) => buffer.extend_from_slice(&chunk),
            None => return Err(PeerDialFailureKind::OrchestratorDropped),
        }
    }
}

fn hex_lowercase(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xF) as usize] as char);
    }
    out
}

// Sync transport bridge over the duplex transport's bounded
// channels, lifted for the handshake window only.
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

#[allow(dead_code)]
fn _touch_mode(m: Mode) {
    // Silence unused-import warnings on Mode; the real consumer is
    // main.rs's match on cli.mode.
    let _ = m;
}

#[allow(dead_code)]
fn _touch_transport_error(e: TransportError) {
    let _ = e;
}
