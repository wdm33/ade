#![allow(clippy::disallowed_types)]
// RED — `live_consensus_session` binary. Operator evidence-capture pass
// for CE-N-B-6.
//
// Drives a real N2N session against a Cardano relay:
//   1. handshake (reusing the capture_chain_sync session pattern),
//   2. FindIntersect at the peer's CURRENT tip, so every following
//      header is current-era (Praos) — not a historical replay,
//   3. RequestNext loop: each RollForward header is projected through
//      the RED follow bridge (`ade_core_interop::follow`) and fed to
//      BLUE fork-choice; each RollBackward is fed to BLUE rollback,
//   4. after catch-up, Ade's selected tip is compared to the peer Tip.
//
// Follow mode is RED, peer-trusted, selection-only: it runs fork-choice
// + rollback ONLY. It does NOT validate VRF / leader / nonce / KES and
// builds no LedgerView — that is workstream B. See the
// `ade_core_interop::follow` module docstring.
//
// Output: a transcript and a peer-tip-comparison log written to
// docs/clusters/PHASE4-N-B/CE-N-B-6_<date>.log. The sustained run is
// fully automated — `--connect` discovers the peer tip, waits
// `--lag-seconds` offline so the chain advances, reconnects, intersects
// at the now-stale tip, and follows the real roll-forward window,
// asserting tip agreement at every block. No manual operator needed.
//
// The closure-gate test (`tests/live_consensus_session.rs`, gated
// `#[ignore]` because it needs network egress to a preprod relay)
// asserts this binary builds and starts. This file performs NO network
// connection unless run with `--connect`; the default `main` prints
// readiness and exits so the `#[ignore]` build-and-start test stays
// hermetic and the deterministic CI gate is the offline replay test.

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

use ade_core::consensus::candidate::TiebreakerView;
use ade_core::consensus::events::{ChainEvent, Point, SecurityParam};
use ade_core::consensus::praos_state::Nonce;
use ade_core_interop::follow::{
    agreement_status, ingest_rollbackward, ingest_rollforward,
    project_header_from_n2n_rollforward, AgreementStatus, FollowState, PeerTip,
};
use ade_network::codec::chain_sync::{
    decode_chain_sync_message, encode_chain_sync_message, ChainSyncMessage, Point as WirePoint,
    Tip as WireTip,
};
use ade_network::codec::handshake::{
    decode_handshake_message, encode_handshake_message, HandshakeMessage, VersionParams,
    VersionTable,
};
use ade_network::codec::primitives::{encode_array_header, encode_bool, encode_u64};
use ade_network::codec::version::N2NVersion;
use ade_network::mux::frame::{encode_frame, MiniProtocolId, MuxFrame, MuxHeader, MuxMode, HEADER_LEN};
use ade_types::{BlockNo, Hash32, SlotNo};

const MAINNET_MAGIC: u32 = 764_824_073;
const PREPROD_MAGIC: u32 = 1;
const HANDSHAKE_PROTOCOL_ID: u16 = 0;
const CHAIN_SYNC_PROTOCOL_ID: u16 = 2;
const MAINNET_K: u64 = 2160;

fn main() {
    let args: Vec<String> = env::args().collect();
    if !args.iter().any(|a| a == "--connect") {
        // Default hermetic path: assert the bridge wires up, print
        // readiness, exit. This is what the `#[ignore]` build-and-start
        // test invokes — no socket is opened.
        let cfg = SessionConfig::from_args(&args);
        println!(
            "ade_core_interop live_consensus_session ready — network={} magic={} max_headers={} k={} (pass --connect for the operator live pass)",
            cfg.network, cfg.magic, cfg.max_headers, MAINNET_K
        );
        return;
    }
    if let Err(e) = run_live(&args) {
        eprintln!("[live] session error: {e}");
        std::process::exit(1);
    }
}

struct SessionConfig {
    network: String,
    magic: u32,
    max_headers: usize,
    max_seconds: u64,
    lag_seconds: u64,
    peer: String,
    out: PathBuf,
}

impl SessionConfig {
    fn from_args(args: &[String]) -> Self {
        let network = arg_value(args, "--network").unwrap_or_else(|| "preprod".into());
        let magic = match network.as_str() {
            "mainnet" => MAINNET_MAGIC,
            _ => PREPROD_MAGIC,
        };
        let max_headers = arg_value(args, "--max-headers")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1000usize);
        let max_seconds = arg_value(args, "--max-seconds")
            .and_then(|s| s.parse().ok())
            .unwrap_or(480u64);
        let lag_seconds = arg_value(args, "--lag-seconds")
            .and_then(|s| s.parse().ok())
            .unwrap_or(75u64);
        let peer = arg_value(args, "--peer").unwrap_or_else(|| match network.as_str() {
            "mainnet" => "backbone.cardano.iog.io:3001".into(),
            _ => "preprod-node.play.dev.cardano.org:3001".into(),
        });
        let out = arg_value(args, "--out")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("docs/clusters/PHASE4-N-B"));
        SessionConfig {
            network,
            magic,
            max_headers,
            max_seconds,
            lag_seconds,
            peer,
            out,
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn run_live(args: &[String]) -> io::Result<()> {
    let cfg = SessionConfig::from_args(args);
    fs::create_dir_all(&cfg.out)?;
    let mut transcript = String::new();
    let log = |t: &mut String, line: String| {
        eprintln!("{line}");
        t.push_str(&line);
        t.push('\n');
    };

    // The real peer address goes to stderr for the operator, but is
    // redacted from the committed transcript (no hostnames in-repo).
    eprintln!("[live] connecting to peer {}", cfg.peer);
    log(
        &mut transcript,
        format!(
            "[live] follow-mode (RED, peer-trusted, selection-only) network={} magic={} peer=<{}-relay> max_headers={}",
            cfg.network, cfg.magic, cfg.network, cfg.max_headers
        ),
    );

    // ---- Connection 1: handshake, discover the peer tip, disconnect ----
    // We deliberately drop the socket during the lag. Holding an idle
    // connection open gets reaped by the relay at a variable timeout, so we
    // wait OFFLINE and reconnect to intersect at the now-stale tip.
    let (mut probe, negotiated) = connect_and_handshake(&cfg.peer, cfg.magic).await?;
    log(&mut transcript, format!("[live] handshake accepted at v{negotiated}"));
    let peer_tip = discover_peer_tip(&mut probe).await?;
    let _ = send_msg(&mut probe, &ChainSyncMessage::Done).await;
    drop(probe);
    log(
        &mut transcript,
        format!(
            "[live] peer tip at connect: block_no={} point={:?}",
            peer_tip.block_no.0, peer_tip.point
        ),
    );

    // ---- Wait offline, then reconnect and roll forward from the stale tip ----
    // After the wait the chain has advanced; intersecting at the stale tip
    // makes the peer roll us back to it and then forward through every block
    // forged since — a real roll-forward window of current-era (Praos)
    // headers, with no fragile idle hold on the connection.
    log(
        &mut transcript,
        format!("[live] waiting {}s offline for the chain to advance", cfg.lag_seconds),
    );
    sleep(Duration::from_secs(cfg.lag_seconds)).await;

    let (mut stream, negotiated2) = connect_and_handshake(&cfg.peer, cfg.magic).await?;
    log(&mut transcript, format!("[live] reconnected, handshake at v{negotiated2}"));

    let intersect_point = wire_point(&peer_tip.point);
    let (anchor_point, anchor_block_no) = (peer_tip.point.clone(), peer_tip.block_no);
    send_msg(
        &mut stream,
        &ChainSyncMessage::FindIntersect {
            points: vec![intersect_point],
        },
    )
    .await?;
    let (_p, _m, payload) = read_frame(&mut stream).await?;
    match decode_chain_sync_message(&payload)
        .map_err(|e| io::Error::other(format!("intersect decode: {e:?}")))?
    {
        ChainSyncMessage::IntersectFound { .. } => {}
        other => return Err(io::Error::other(format!("expected IntersectFound, got {other:?}"))),
    }

    let mut state = FollowState::new(
        anchor_point,
        anchor_block_no,
        anchor_tiebreaker(&peer_tip),
        SecurityParam(MAINNET_K),
        Nonce(Hash32([0u8; 32])),
    );

    // ---- RequestNext follow loop ----
    // At the tip the peer answers RequestNext with AwaitReply, then pushes
    // the next RollForward unsolicited once a block is forged. We keep
    // reading frames after AwaitReply (without re-sending RequestNext)
    // until a new block arrives, bounded by --max-seconds so the run
    // always terminates even on a quiet chain.
    let mut headers_seen = 0usize;
    let deadline = Instant::now() + Duration::from_secs(cfg.max_seconds);
    while headers_seen < cfg.max_headers {
        let now = Instant::now();
        if now >= deadline {
            log(&mut transcript, format!("[live] time budget ({}s) reached", cfg.max_seconds));
            break;
        }
        send_msg(&mut stream, &ChainSyncMessage::RequestNext).await?;
        let remaining = deadline.saturating_duration_since(now);
        let (_p, _m, payload) = match timeout(remaining, read_frame(&mut stream)).await {
            Ok(Ok(frame)) => frame,
            Ok(Err(e)) => {
                // Peer closed or a partial frame — end the follow window and
                // preserve whatever agreement evidence we gathered so far.
                log(
                    &mut transcript,
                    format!("[live] read ended after {headers_seen} header(s): {e}"),
                );
                break;
            }
            Err(_) => {
                log(
                    &mut transcript,
                    format!("[live] time budget ({}s) reached while awaiting next block", cfg.max_seconds),
                );
                break;
            }
        };
        let msg = decode_chain_sync_message(&payload)
            .map_err(|e| io::Error::other(format!("chain-sync decode: {e:?}")))?;
        match msg {
            ChainSyncMessage::RollForward { header, tip } => {
                let projected = match project_header_from_n2n_rollforward(&header) {
                    Ok(p) => p,
                    Err(e) => {
                        log(
                            &mut transcript,
                            format!("[live] header projection ended window after {headers_seen} header(s): {e:?}"),
                        );
                        break;
                    }
                };
                let pt = peer_tip_from_wire(&tip);
                let (next, event) = ingest_rollforward(state, &projected, pt)
                    .map_err(|e| io::Error::other(format!("rollforward: {e:?}")))?;
                state = next;
                headers_seen += 1;
                if let ChainEvent::ChainSelected { new_tip, .. } = event {
                    log(
                        &mut transcript,
                        format!(
                            "[live] +block {} slot {} -> tip {:?}",
                            state.current_tip_block_no().0,
                            new_tip.slot.0,
                            new_tip.hash.0[..4].to_vec()
                        ),
                    );
                }
            }
            ChainSyncMessage::RollBackward { point, tip } => {
                let to = wire_to_core_point(&point);
                let pt = peer_tip_from_wire(&tip);
                let (next, event) = ingest_rollbackward(state, to, pt)
                    .map_err(|e| io::Error::other(format!("rollback: {e:?}")))?;
                state = next;
                log(&mut transcript, format!("[live] rollback event: {event:?}"));
            }
            ChainSyncMessage::AwaitReply => {
                // Consumed the whole backlog up to the peer tip — caught up.
                log(&mut transcript, "[live] caught up to peer tip".to_string());
                break;
            }
            other => return Err(io::Error::other(format!("unexpected reply: {other:?}"))),
        }
    }

    // ---- Peer-tip agreement ----
    let status = agreement_status(&mut state);
    let agreement_line = match &status {
        AgreementStatus::Agreed { block_no, .. } => {
            format!("[live] AGREEMENT at block_no={}", block_no.0)
        }
        AgreementStatus::CatchingUp {
            ade_block_no,
            peer_block_no,
        } => format!(
            "[live] still catching up: ade={} peer={}",
            ade_block_no.0, peer_block_no.0
        ),
        AgreementStatus::Disagree { block_no, .. } => {
            format!("[live] DISAGREEMENT at block_no={} (hard failure)", block_no.0)
        }
        AgreementStatus::NoPeerTipYet => "[live] no peer tip observed".into(),
    };
    log(&mut transcript, agreement_line);
    log(
        &mut transcript,
        format!(
            "[live] headers_seen={} disagreements={} ade_tip_block_no={}",
            headers_seen,
            state.disagreements(),
            state.current_tip_block_no().0
        ),
    );

    let _ = send_msg(&mut stream, &ChainSyncMessage::Done).await;

    let ts = utc_stamp();
    let transcript_path = cfg.out.join(format!("CE-N-B-6_{ts}.log"));
    fs::write(&transcript_path, &transcript)?;
    eprintln!("[live] transcript written to {transcript_path:?}");

    if state.disagreements() > 0 {
        std::process::exit(2);
    }
    Ok(())
}

fn anchor_tiebreaker(peer_tip: &PeerTip) -> TiebreakerView {
    // The anchor is the peer's current tip; we have no tiebreaker fields
    // for it (only the point), so seed a minimal view at its slot. The
    // first real header always has a strictly higher block number, so it
    // is selected regardless of this seed.
    TiebreakerView {
        slot: peer_tip.point.slot,
        issuer_hash: ade_types::Hash28([0u8; 28]),
        op_cert_counter: 0,
        leader_vrf_output_first_8: [0u8; 8],
    }
}

async fn connect_and_handshake(peer: &str, magic: u32) -> io::Result<(TcpStream, u16)> {
    let mut stream = timeout(Duration::from_secs(10), TcpStream::connect(peer))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "tcp connect timed out"))??;
    let proposed: Vec<u16> = vec![11, 12, 13, 14, 15, 16];
    let mut entries = Vec::with_capacity(proposed.len());
    for v in &proposed {
        entries.push((N2NVersion::new(*v), version_params_for_n2n(*v, magic)));
    }
    let propose = encode_handshake_message(&HandshakeMessage::ProposeVersions(VersionTable(entries)));
    write_frame(&mut stream, propose, HANDSHAKE_PROTOCOL_ID).await?;
    let (_proto, _mode, payload) = read_frame(&mut stream).await?;
    let negotiated = match decode_handshake_message(&payload)
        .map_err(|e| io::Error::other(format!("handshake decode: {e:?}")))?
    {
        HandshakeMessage::AcceptVersion(v, _) => v.get(),
        other => return Err(io::Error::other(format!("handshake refused: {other:?}"))),
    };
    Ok((stream, negotiated))
}

async fn discover_peer_tip(stream: &mut TcpStream) -> io::Result<PeerTip> {
    send_msg(
        stream,
        &ChainSyncMessage::FindIntersect {
            points: vec![WirePoint::Origin],
        },
    )
    .await?;
    let (_p, _m, payload) = read_frame(stream).await?;
    match decode_chain_sync_message(&payload)
        .map_err(|e| io::Error::other(format!("intersect decode: {e:?}")))?
    {
        ChainSyncMessage::IntersectFound { tip, .. }
        | ChainSyncMessage::IntersectNotFound { tip } => Ok(peer_tip_from_wire(&tip)),
        other => Err(io::Error::other(format!(
            "expected intersect reply, got {other:?}"
        ))),
    }
}

fn peer_tip_from_wire(tip: &WireTip) -> PeerTip {
    PeerTip {
        point: wire_to_core_point(&tip.point),
        block_no: BlockNo(tip.block_no),
    }
}

fn wire_to_core_point(p: &WirePoint) -> Point {
    match p {
        WirePoint::Origin => Point {
            slot: SlotNo(0),
            hash: Hash32([0u8; 32]),
        },
        WirePoint::Block { slot, hash } => Point {
            slot: *slot,
            hash: hash.clone(),
        },
    }
}

fn wire_point(p: &Point) -> WirePoint {
    WirePoint::Block {
        slot: p.slot,
        hash: p.hash.clone(),
    }
}

// ---- transport helpers (lifted from capture_chain_sync) ----

async fn send_msg(stream: &mut TcpStream, msg: &ChainSyncMessage) -> io::Result<()> {
    let payload = encode_chain_sync_message(msg);
    write_frame(stream, payload, CHAIN_SYNC_PROTOCOL_ID).await
}

async fn write_frame(stream: &mut TcpStream, payload: Vec<u8>, protocol_id: u16) -> io::Result<()> {
    let frame = MuxFrame {
        header: MuxHeader {
            timestamp: now_micros_mod_u32(),
            mode: MuxMode::Initiator,
            mini_protocol_id: MiniProtocolId::new(protocol_id).expect("id in range"),
            length: payload.len() as u16,
        },
        payload,
    };
    let bytes = encode_frame(&frame).expect("encode frame");
    stream.write_all(&bytes).await
}

async fn read_frame(stream: &mut TcpStream) -> io::Result<(u16, MuxMode, Vec<u8>)> {
    let mut header_buf = [0u8; HEADER_LEN];
    timeout(Duration::from_secs(20), stream.read_exact(&mut header_buf))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "frame header read"))??;
    let id_word = u16::from_be_bytes([header_buf[4], header_buf[5]]);
    let mini_id = id_word & 0x7FFF;
    let mode = if id_word & 0x8000 != 0 {
        MuxMode::Responder
    } else {
        MuxMode::Initiator
    };
    let payload_len = u16::from_be_bytes([header_buf[6], header_buf[7]]) as usize;
    let mut payload = vec![0u8; payload_len];
    if payload_len > 0 {
        timeout(Duration::from_secs(20), stream.read_exact(&mut payload))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "frame payload read"))??;
    }
    Ok((mini_id, mode, payload))
}

fn version_params_for_n2n(version: u16, magic: u32) -> VersionParams {
    let mut buf = Vec::new();
    let field_count: u64 = if version >= 16 { 5 } else { 4 };
    encode_array_header(&mut buf, field_count);
    encode_u64(&mut buf, magic as u64);
    encode_bool(&mut buf, true);
    encode_u64(&mut buf, 0);
    encode_bool(&mut buf, false);
    if version >= 16 {
        encode_bool(&mut buf, false);
    }
    VersionParams(buf)
}

fn now_micros_mod_u32() -> u32 {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    (dur.as_micros() as u64 & 0xFFFF_FFFF) as u32
}

fn utc_stamp() -> String {
    // Coarse UTC date stamp for the evidence filename; the operator pass
    // is the authoritative record, so a date-only stamp suffices.
    let _ = Instant::now();
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    let days = dur.as_secs() / 86_400;
    let mut year = 1970u64;
    let mut d = days;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let yd = if leap { 366 } else { 365 };
        if d < yd {
            break;
        }
        d -= yd;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let mdays: [u64; 12] = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 0usize;
    while month < 12 && d >= mdays[month] {
        d -= mdays[month];
        month += 1;
    }
    format!("{year:04}-{:02}-{:02}", month + 1, d + 1)
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == flag {
            return iter.next().cloned();
        }
    }
    None
}
