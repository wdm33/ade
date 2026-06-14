#![allow(clippy::disallowed_types)]
// RED — Imperative Shell SERVER-SIDE capture harness for N2N
// TxSubmission2 (protocol 4).
//
// The client-side capture binary (capture_tx_submission2.rs) dials the
// cardano-node and can only ever record `MsgInit`. This harness flips the
// direction: Ade LISTENS; the operator adds Ade to the node's `localRoots`
// so the node dials Ade, promotes it to a hot peer, and opens tx-submission2.
//
// Mini-protocol roles (confirmed live against cardano-node 11.0.1 via its own
// decoder traces): when the node dials Ade it runs the tx-submission2
// mux InitiatorDir, which is the protocol CLIENT (the tx *provider*). So:
//   - the node (CLIENT/provider) sends MsgInit, then offers its mempool with
//     MsgReplyTxIds (real tx ids) and MsgReplyTxs (real tx bodies);
//   - Ade (SERVER/consumer) sends MsgRequestTxIds / MsgRequestTxs.
// Ade therefore RECORDS the node-originated MsgInit / MsgReplyTxIds /
// MsgReplyTxs — the rich messages, carrying real preprod transaction data —
// and never submits anything to the node (it only pulls and discards).
//
// Mux mode convention (empirically confirmed by the chain_sync corpus, where
// the dialed node writes mode=Responder): the dialed party writes its frames
// with mode=Responder. Here Ade is the dialed party, so Ade writes its
// RequestTxIds/RequestTxs with mode=Responder and the node writes its
// Init/Reply* with mode=Initiator.
//
// The accept loop is MULTI-SHOT: the node's peer-selection governor promotes
// a local-root peer cold->warm->hot with backoff, and early connections can
// drop before tx-submission opens. The harness keeps accepting until it has
// recorded the rich messages.
//
// Captured node-originated frames are written as full mux frames (8-byte
// header + payload), matching the chain_sync real-capture corpus format
// consumed by the byte-identical round-trip test.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

use ade_network::codec::chain_sync::{
    decode_chain_sync_message, encode_chain_sync_message, ChainSyncMessage, Point, Tip,
};
use ade_network::codec::handshake::{
    decode_handshake_message, encode_handshake_message, HandshakeMessage,
};
use ade_network::codec::keep_alive::{
    decode_keep_alive_message, encode_keep_alive_message, KeepAliveMessage,
};
use ade_network::codec::tx_submission::{
    decode_tx_submission_message, encode_tx_submission_message, TxSubmission2Message,
};
use ade_network::mux::frame::{
    encode_frame, MiniProtocolId, MuxFrame, MuxHeader, MuxMode, HEADER_LEN,
};

const MAINNET_MAGIC: u32 = 764_824_073;
const PREPROD_MAGIC: u32 = 1;
const PREVIEW_MAGIC: u32 = 2;

const HANDSHAKE_PROTOCOL_ID: u16 = 0;
const CHAIN_SYNC_PROTOCOL_ID: u16 = 2;
const TX_SUBMISSION2_PROTOCOL_ID: u16 = 4;
const KEEP_ALIVE_PROTOCOL_ID: u16 = 8;

/// Highest N2N version Ade's handshake responder will accept.
const MAX_SUPPORTED_N2N_VERSION: u16 = 16;

/// How many tx ids to request per round (small window: ack each batch before
/// the next blocking request, keeping the node's unacked window satisfied).
const REQ_TXIDS: u16 = 2;

fn now_micros_mod_u32() -> u32 {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    (dur.as_micros() as u64 & 0xFFFF_FFFF) as u32
}

/// Wrap a mini-protocol payload in a mux frame. `mode` is the sender's
/// role; Ade (the dialed party) always writes `Responder`.
fn wrap(payload: Vec<u8>, protocol_id: u16, mode: MuxMode) -> Vec<u8> {
    let frame = MuxFrame {
        header: MuxHeader {
            timestamp: now_micros_mod_u32(),
            mode,
            mini_protocol_id: MiniProtocolId::new(protocol_id).expect("id in range"),
            length: payload.len() as u16,
        },
        payload,
    };
    encode_frame(&frame).expect("encode")
}

/// One mux frame off the wire: the raw 8-byte header (preserved verbatim so
/// the capture file is byte-identical to what the node sent), the
/// mini-protocol id, the sender mode, and the payload.
struct WireFrame {
    header: [u8; HEADER_LEN],
    mini_id: u16,
    mode: MuxMode,
    payload: Vec<u8>,
}

impl WireFrame {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_LEN + self.payload.len());
        out.extend_from_slice(&self.header);
        out.extend_from_slice(&self.payload);
        out
    }
}

async fn read_one_frame(stream: &mut TcpStream) -> io::Result<WireFrame> {
    let mut header = [0u8; HEADER_LEN];
    stream.read_exact(&mut header).await?;
    let id_word = u16::from_be_bytes([header[4], header[5]]);
    let mini_id = id_word & 0x7FFF;
    let mode = if id_word & 0x8000 != 0 {
        MuxMode::Responder
    } else {
        MuxMode::Initiator
    };
    let payload_len = u16::from_be_bytes([header[6], header[7]]) as usize;
    let mut payload = vec![0u8; payload_len];
    if payload_len > 0 {
        stream.read_exact(&mut payload).await?;
    }
    Ok(WireFrame {
        header,
        mini_id,
        mode,
        payload,
    })
}

/// A compact, non-spammy description of a tx-sub message (ReplyTxs bodies can
/// be large, so we summarise rather than Debug-print them).
fn describe(msg: &TxSubmission2Message) -> String {
    match msg {
        TxSubmission2Message::Init => "Init".into(),
        TxSubmission2Message::RequestTxIds { blocking, ack, req } => {
            format!("RequestTxIds{{blocking:{blocking}, ack:{ack}, req:{req}}}")
        }
        TxSubmission2Message::ReplyTxIds(e) => format!("ReplyTxIds({} ids)", e.len()),
        TxSubmission2Message::RequestTxs(ids) => format!("RequestTxs({} ids)", ids.len()),
        TxSubmission2Message::ReplyTxs(txs) => {
            let bytes: usize = txs.iter().map(|t| t.len()).sum();
            format!("ReplyTxs({} txs, {bytes} bytes)", txs.len())
        }
        TxSubmission2Message::Done => "Done".into(),
    }
}

#[derive(Default)]
struct Captured {
    init: u32,
    reply_txids: u32,
    reply_txids_nonempty: u32,
    reply_txs: u32,
    reply_txs_nonempty: u32,
}

impl Captured {
    /// Enough to prove the codec is on the node's wire grammar for the rich
    /// messages: a non-empty ReplyTxIds (real tx ids) and at least one
    /// ReplyTxs message.
    fn essentials_met(&self) -> bool {
        self.reply_txids_nonempty >= 1 && self.reply_txs >= 1
    }

    /// Best case: also captured the node's MsgInit and a non-empty ReplyTxs
    /// carrying real tx bodies.
    fn complete(&self) -> bool {
        self.init >= 1 && self.reply_txids_nonempty >= 1 && self.reply_txs_nonempty >= 1
    }
}

struct Config {
    out_dir: PathBuf,
    scenario: String,
    network_magic: u32,
    write_mode: MuxMode,
    run_secs: u64,
    idle_secs: u64,
    /// Block number Ade claims for its chain-sync tip, so it looks like a
    /// caught-up peer the node keeps hot (rather than a genesis peer it
    /// demotes). Set near the node's real tip height.
    tip_block_no: u64,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let listen = arg_value(&args, "--listen").unwrap_or_else(|| "0.0.0.0:3101".into());
    let cfg = Config {
        out_dir: arg_value(&args, "--out")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("corpus/network/n2n/tx_submission2")),
        scenario: arg_value(&args, "--scenario").unwrap_or_else(|| "preprod_server".into()),
        network_magic: arg_value(&args, "--magic")
            .as_deref()
            .map(|s| match s {
                "mainnet" => MAINNET_MAGIC,
                "preprod" => PREPROD_MAGIC,
                "preview" => PREVIEW_MAGIC,
                other => other.parse::<u32>().unwrap_or(PREPROD_MAGIC),
            })
            .unwrap_or(PREPROD_MAGIC),
        write_mode: match arg_value(&args, "--write-mode").as_deref() {
            Some("initiator") => MuxMode::Initiator,
            _ => MuxMode::Responder,
        },
        run_secs: arg_value(&args, "--run-timeout")
            .and_then(|s| s.parse().ok())
            .unwrap_or(150),
        idle_secs: arg_value(&args, "--idle-timeout")
            .and_then(|s| s.parse().ok())
            .unwrap_or(90),
        tip_block_no: arg_value(&args, "--tip-block-no")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
    };
    let accept_secs: u64 = arg_value(&args, "--accept-timeout")
        .and_then(|s| s.parse().ok())
        .unwrap_or(600);

    fs::create_dir_all(&cfg.out_dir)?;

    let listener = TcpListener::bind(&listen).await?;
    eprintln!("[txsub2-srv] listening on {listen} (multi-shot; up to {accept_secs}s between dials)");
    eprintln!("[txsub2-srv] Ade plays the tx-submission SERVER/consumer; add this host to the node's localRoots, then restart it");

    let mut captured = Captured::default();
    let mut seq_by_kind: HashMap<&'static str, u32> = HashMap::new();
    let mut connections: u32 = 0;

    while !captured.essentials_met() {
        let (mut stream, peer) =
            match timeout(Duration::from_secs(accept_secs), listener.accept()).await {
                Ok(Ok(x)) => x,
                Ok(Err(e)) => {
                    eprintln!("[txsub2-srv] accept error: {e}");
                    continue;
                }
                Err(_) => {
                    eprintln!("[txsub2-srv] no inbound dial within {accept_secs}s; stopping");
                    break;
                }
            };
        stream.set_nodelay(true).ok();
        connections += 1;
        eprintln!("[txsub2-srv] [conn {connections}] accepted inbound connection from {peer}");

        match handle_connection(&mut stream, &cfg, &mut captured, &mut seq_by_kind).await {
            Ok(()) => {}
            Err(e) => eprintln!("[txsub2-srv] [conn {connections}] ended: {e}"),
        }
    }

    eprintln!(
        "[txsub2-srv] CAPTURE SUMMARY over {connections} connection(s): Init={} ReplyTxIds={} (non-empty {}) ReplyTxs={} (non-empty {})",
        captured.init,
        captured.reply_txids,
        captured.reply_txids_nonempty,
        captured.reply_txs,
        captured.reply_txs_nonempty
    );

    let meta = format!(
        r#"# Captured tx-submission2 SERVER-SIDE corpus (option B: node dials Ade).
listen = "{listen}"
network_magic = {magic}
protocol = "TxSubmission2"
mini_protocol_id = {TX_SUBMISSION2_PROTOCOL_ID}
direction = "node->ade (node is the tx-submission CLIENT/provider; Ade is the SERVER/consumer)"
connections = {connections}
captured_init = {init}
captured_reply_txids = {rtid}
captured_reply_txids_nonempty = {rtidne}
captured_reply_txs = {rtx}
captured_reply_txs_nonempty = {rtxne}
"#,
        magic = cfg.network_magic,
        init = captured.init,
        rtid = captured.reply_txids,
        rtidne = captured.reply_txids_nonempty,
        rtx = captured.reply_txs,
        rtxne = captured.reply_txs_nonempty,
    );
    fs::write(cfg.out_dir.join(format!("{}_server_meta.toml", cfg.scenario)), meta)?;

    if captured.essentials_met() {
        eprintln!("[txsub2-srv] OK — essentials captured");
        Ok(())
    } else {
        Err(io::Error::other(
            "did not capture ReplyTxIds + ReplyTxs — see node logs / promotion state",
        ))
    }
}

/// Handle one accepted connection: handshake responder, then drive the
/// tx-submission SERVER (consumer) side, recording the node's CLIENT
/// (provider) replies. Returns when the per-connection budget elapses, the
/// peer closes, or this connection has captured the rich set.
async fn handle_connection(
    stream: &mut TcpStream,
    cfg: &Config,
    captured: &mut Captured,
    seq_by_kind: &mut HashMap<&'static str, u32>,
) -> io::Result<()> {
    // ---- HANDSHAKE (responder side) ----
    let hs_frame = timeout(Duration::from_secs(cfg.idle_secs), read_one_frame(stream))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "handshake propose timeout"))??;
    if hs_frame.mini_id != HANDSHAKE_PROTOCOL_ID {
        return Err(io::Error::other(format!(
            "expected handshake (proto 0) first, got proto {} mode {:?}",
            hs_frame.mini_id, hs_frame.mode
        )));
    }
    fs::write(
        cfg.out_dir.join(format!("{}_handshake_propose.cbor", cfg.scenario)),
        hs_frame.to_bytes(),
    )?;
    let proposed = match decode_handshake_message(&hs_frame.payload)
        .map_err(|e| io::Error::other(format!("handshake decode: {e:?}")))?
    {
        HandshakeMessage::ProposeVersions(table) => table,
        other => return Err(io::Error::other(format!("expected ProposeVersions, got {other:?}"))),
    };
    eprintln!(
        "[txsub2-srv]    node proposed versions: {:?}",
        proposed.0.iter().map(|(v, _)| v.get()).collect::<Vec<_>>()
    );
    let chosen = proposed
        .0
        .iter()
        .filter(|(v, _)| v.get() <= MAX_SUPPORTED_N2N_VERSION)
        .max_by_key(|(v, _)| v.get())
        .ok_or_else(|| io::Error::other("no mutually supported N2N version"))?;
    let accept =
        encode_handshake_message(&HandshakeMessage::AcceptVersion(chosen.0, chosen.1.clone()));
    stream
        .write_all(&wrap(accept, HANDSHAKE_PROTOCOL_ID, cfg.write_mode))
        .await?;
    eprintln!("[txsub2-srv]    accepted version V{}", chosen.0.get());

    // ---- TX-SUBMISSION2 (server/consumer side) ----
    // The node (CLIENT/provider) opens the protocol with MsgInit. We do NOT
    // send MsgInit (that is the client's job). We wait for the node's MsgInit,
    // then request tx ids / tx bodies and record the node's replies.
    eprintln!("[txsub2-srv]    waiting for the node's MsgInit (node is the tx provider)...");

    let mut pending_ack: u16 = 0; // tx ids received but not yet acknowledged
    let run_deadline = Instant::now() + Duration::from_secs(cfg.run_secs);
    let mut soft_deadline: Option<Instant> = None;

    loop {
        if Instant::now() >= run_deadline {
            eprintln!("[txsub2-srv]    per-connection run-timeout reached");
            return Ok(());
        }
        if let Some(sd) = soft_deadline {
            if Instant::now() >= sd {
                eprintln!("[txsub2-srv]    essentials captured; closing this connection");
                return Ok(());
            }
        }

        let frame = match timeout(Duration::from_secs(cfg.idle_secs), read_one_frame(stream)).await {
            Ok(Ok(f)) => f,
            Ok(Err(e)) => return Err(io::Error::other(format!("read: {e}"))),
            Err(_) => {
                eprintln!("[txsub2-srv]    idle timeout ({}s) with no frame", cfg.idle_secs);
                return Ok(());
            }
        };

        match frame.mini_id {
            TX_SUBMISSION2_PROTOCOL_ID => {
                let decoded = match decode_tx_submission_message(&frame.payload) {
                    Ok(m) => m,
                    Err(e) => {
                        // Real-interop finding: preserve the raw bytes of any
                        // node tx-sub frame our codec cannot decode (e.g.
                        // cardano-node's indefinite-length array in ReplyTxIds),
                        // so the exact wire shape is recorded for the codec fix
                        // and the regression corpus.
                        eprintln!("[txsub2-srv]    proto4 UNDECODABLE ({e:?}); saving raw bytes");
                        save_frame(&cfg.out_dir, &cfg.scenario, "rawundecodable", seq_by_kind, &frame)?;
                        continue;
                    }
                };
                eprintln!("[txsub2-srv] <- node tx-sub: {}", describe(&decoded));
                match decoded {
                    TxSubmission2Message::Init => {
                        captured.init += 1;
                        save_frame(&cfg.out_dir, &cfg.scenario, "init", seq_by_kind, &frame)?;
                        // We now have agency: request the first batch of ids.
                        send_msg(
                            stream,
                            cfg,
                            &TxSubmission2Message::RequestTxIds {
                                blocking: true,
                                ack: 0,
                                req: REQ_TXIDS,
                            },
                        )
                        .await?;
                    }
                    TxSubmission2Message::ReplyTxIds(entries) => {
                        captured.reply_txids += 1;
                        if !entries.is_empty() {
                            captured.reply_txids_nonempty += 1;
                        }
                        save_frame(&cfg.out_dir, &cfg.scenario, "reply_txids", seq_by_kind, &frame)?;
                        pending_ack = entries.len() as u16;
                        if let Some(first) = entries.first() {
                            // Fetch one real tx body.
                            send_msg(
                                stream,
                                cfg,
                                &TxSubmission2Message::RequestTxs(vec![first.tx_id.clone()]),
                            )
                            .await?;
                        } else {
                            // Empty (non-blocking) reply: acknowledge nothing
                            // new is outstanding and block for more.
                            send_msg(
                                stream,
                                cfg,
                                &TxSubmission2Message::RequestTxIds {
                                    blocking: true,
                                    ack: pending_ack,
                                    req: REQ_TXIDS,
                                },
                            )
                            .await?;
                            pending_ack = 0;
                        }
                    }
                    TxSubmission2Message::ReplyTxs(txs) => {
                        captured.reply_txs += 1;
                        if !txs.is_empty() {
                            captured.reply_txs_nonempty += 1;
                        }
                        save_frame(&cfg.out_dir, &cfg.scenario, "reply_txs", seq_by_kind, &frame)?;
                        // Acknowledge the batch and block for the next.
                        send_msg(
                            stream,
                            cfg,
                            &TxSubmission2Message::RequestTxIds {
                                blocking: true,
                                ack: pending_ack,
                                req: REQ_TXIDS,
                            },
                        )
                        .await?;
                        pending_ack = 0;
                    }
                    TxSubmission2Message::Done => {
                        eprintln!("[txsub2-srv]    node sent Done");
                        return Ok(());
                    }
                    other => {
                        eprintln!(
                            "[txsub2-srv]    unexpected server-role message from node: {}",
                            describe(&other)
                        );
                    }
                }

                if captured.complete() {
                    eprintln!("[txsub2-srv]    captured Init + non-empty ReplyTxIds + non-empty ReplyTxs");
                    return Ok(());
                }
                if captured.essentials_met() && soft_deadline.is_none() {
                    soft_deadline = Some(Instant::now() + Duration::from_secs(15));
                }
            }
            KEEP_ALIVE_PROTOCOL_ID => {
                if let Ok(KeepAliveMessage::KeepAlive(cookie)) =
                    decode_keep_alive_message(&frame.payload)
                {
                    stream
                        .write_all(&wrap(
                            encode_keep_alive_message(&KeepAliveMessage::ResponseKeepAlive(cookie)),
                            KEEP_ALIVE_PROTOCOL_ID,
                            cfg.write_mode,
                        ))
                        .await?;
                }
            }
            CHAIN_SYNC_PROTOCOL_ID => {
                // The node opens chain-sync as the client (it syncs headers
                // FROM Ade). We serve no chain, but we must answer or the
                // node's ~10s intersect timeout tears the whole connection
                // down before tx-submission completes. Report no intersection
                // (genesis tip), then park RequestNext in AwaitReply — which
                // moves the node into its long (~135s+) MustReply window.
                // The node's chain-sync client may PIPELINE several messages
                // into one SDU. Decode each in turn and answer it, so we never
                // leave a RequestNext unanswered (an unanswered one trips the
                // node's ~10s CanAwait timeout and tears the connection down).
                let mut off = 0usize;
                while off < frame.payload.len() {
                    let start = off;
                    if ade_codec::cbor_primitives::skip_item(&frame.payload, &mut off).is_err() {
                        break;
                    }
                    let cs = match decode_chain_sync_message(&frame.payload[start..off]) {
                        Ok(m) => m,
                        Err(e) => {
                            eprintln!("[txsub2-srv]    (chain-sync decode skipped: {e:?})");
                            break;
                        }
                    };
                    let reply = match cs {
                        // Claim Ade is at the node's own tip (the first/newest
                        // point it offered) so the node treats Ade as a healthy
                        // caught-up peer and keeps it hot. Reporting no
                        // intersection (genesis) makes the node conclude Ade is
                        // useless and demote it within ms.
                        ChainSyncMessage::FindIntersect { points } => Some(match points.first() {
                            Some(p) => ChainSyncMessage::IntersectFound {
                                point: p.clone(),
                                tip: Tip { point: p.clone(), block_no: cfg.tip_block_no },
                            },
                            None => ChainSyncMessage::IntersectNotFound {
                                tip: Tip { point: Point::Origin, block_no: 0 },
                            },
                        }),
                        ChainSyncMessage::RequestNext => Some(ChainSyncMessage::AwaitReply),
                        _ => None,
                    };
                    if let Some(msg) = reply {
                        stream
                            .write_all(&wrap(
                                encode_chain_sync_message(&msg),
                                CHAIN_SYNC_PROTOCOL_ID,
                                cfg.write_mode,
                            ))
                            .await?;
                    }
                }
            }
            other => {
                eprintln!(
                    "[txsub2-srv]    (drained proto {other} mode {:?} len {})",
                    frame.mode,
                    frame.payload.len()
                );
            }
        }
    }
}

async fn send_msg(
    stream: &mut TcpStream,
    cfg: &Config,
    msg: &TxSubmission2Message,
) -> io::Result<()> {
    eprintln!("[txsub2-srv] -> ade   tx-sub: {}", describe(msg));
    stream
        .write_all(&wrap(
            encode_tx_submission_message(msg),
            TX_SUBMISSION2_PROTOCOL_ID,
            cfg.write_mode,
        ))
        .await
}

fn save_frame(
    out_dir: &Path,
    scenario: &str,
    kind: &'static str,
    seq_by_kind: &mut HashMap<&'static str, u32>,
    frame: &WireFrame,
) -> io::Result<()> {
    let seq = seq_by_kind.entry(kind).or_insert(0);
    let path = out_dir.join(format!("{scenario}_txsub_{kind}_{:02}_recv.cbor", *seq));
    *seq += 1;
    fs::write(&path, frame.to_bytes())?;
    eprintln!("[txsub2-srv]    saved {}", path.display());
    Ok(())
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
