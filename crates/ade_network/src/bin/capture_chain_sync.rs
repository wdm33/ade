#![allow(clippy::disallowed_types)]
// RED — Imperative Shell capture binary for N2N chain-sync.
//
// Performs the N2N handshake against a Cardano relay, then drives the
// chain-sync mini-protocol (FindIntersect → RequestNext loop) and
// captures each frame the peer sends us. Defaults target preprod
// testnet (network magic 1) to avoid mainnet relay load.
//
// Each captured frame is written to its own pair of files:
//   <out>/<scenario>_frame_NN_recv.cbor     mux frame bytes (header + payload)
//   <out>/<scenario>_frame_NN_recv_payload.cbor  just the chain-sync CBOR
// plus one metadata file:
//   <out>/<scenario>_meta.toml
//
// We do NOT decode block header bodies (those are era-specific and
// opaque at this layer per DC-PROTO-06).

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

use ade_network::codec::chain_sync::{
    decode_chain_sync_message, encode_chain_sync_message, ChainSyncMessage, Point,
};
use ade_network::codec::handshake::{
    decode_handshake_message, encode_handshake_message, HandshakeMessage, VersionParams,
    VersionTable,
};
use ade_network::codec::primitives::{encode_array_header, encode_bool, encode_u64};
use ade_network::codec::version::N2NVersion;
use ade_network::mux::frame::{
    encode_frame, MiniProtocolId, MuxFrame, MuxHeader, MuxMode, HEADER_LEN,
};

const MAINNET_MAGIC: u32 = 764_824_073;
const PREPROD_MAGIC: u32 = 1;
const PREVIEW_MAGIC: u32 = 2;
const HANDSHAKE_PROTOCOL_ID: u16 = 0;
const CHAIN_SYNC_PROTOCOL_ID: u16 = 2;

fn version_params_for_n2n(version: u16, magic: u32) -> VersionParams {
    let mut buf = Vec::new();
    let field_count: u64 = if version >= 16 { 5 } else { 4 };
    encode_array_header(&mut buf, field_count);
    encode_u64(&mut buf, magic as u64);
    encode_bool(&mut buf, true);  // initiatorOnlyDiffusionMode
    encode_u64(&mut buf, 0);       // peerSharing = NoPeerSharing
    encode_bool(&mut buf, false);  // query
    if version >= 16 {
        encode_bool(&mut buf, false);  // perasSupport
    }
    VersionParams(buf)
}

fn now_micros_mod_u32() -> u32 {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    (dur.as_micros() as u64 & 0xFFFF_FFFF) as u32
}

fn wrap_in_mux_frame(payload: Vec<u8>, protocol_id: u16, mode: MuxMode) -> Vec<u8> {
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

async fn read_one_frame(stream: &mut TcpStream) -> io::Result<(u16, MuxMode, Vec<u8>, Vec<u8>)> {
    let mut header_buf = [0u8; HEADER_LEN];
    stream.read_exact(&mut header_buf).await?;
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
        stream.read_exact(&mut payload).await?;
    }
    let mut full = header_buf.to_vec();
    full.extend_from_slice(&payload);
    Ok((mini_id, mode, payload, full))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let peer = arg_value(&args, "--peer")
        .unwrap_or_else(|| "preprod-node.play.dev.cardano.org:3001".into());
    let out_dir = arg_value(&args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("corpus/network/n2n/chain_sync"));
    let scenario =
        arg_value(&args, "--scenario").unwrap_or_else(|| "preprod_origin_5_frames".into());
    let network_magic: u32 = arg_value(&args, "--magic")
        .as_deref()
        .map(|s| match s {
            "mainnet" => MAINNET_MAGIC,
            "preprod" => PREPROD_MAGIC,
            "preview" => PREVIEW_MAGIC,
            other => other.parse::<u32>().unwrap_or(PREPROD_MAGIC),
        })
        .unwrap_or(PREPROD_MAGIC);
    let frame_count: usize = arg_value(&args, "--frames")
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    // Optional intersect point: when both are supplied we FindIntersect
    // at this concrete (slot, hash) instead of Origin, so the captured
    // RollForward frames carry the era at that chain position (e.g. a
    // recent Conway point) rather than the Byron blocks at Origin.
    let intersect_point: Option<(u64, [u8; 32])> = match (
        arg_value(&args, "--intersect-slot").as_deref().and_then(|s| s.parse::<u64>().ok()),
        arg_value(&args, "--intersect-hash").as_deref().and_then(parse_hash32),
    ) {
        (Some(slot), Some(hash)) => Some((slot, hash)),
        _ => None,
    };

    fs::create_dir_all(&out_dir)?;

    eprintln!("[cs] connecting to {peer} (magic={network_magic})");
    let start = Instant::now();
    let mut stream = timeout(Duration::from_secs(10), TcpStream::connect(&peer))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "tcp connect timed out"))??;
    eprintln!("[cs] connected in {:.2}s", start.elapsed().as_secs_f64());

    // ---- HANDSHAKE ----
    let proposed: Vec<u16> = vec![11, 12, 13, 14, 15, 16];
    let mut entries = Vec::with_capacity(proposed.len());
    for v in &proposed {
        entries.push((N2NVersion::new(*v), version_params_for_n2n(*v, network_magic)));
    }
    let table = VersionTable(entries);
    let propose_payload = encode_handshake_message(&HandshakeMessage::ProposeVersions(table));
    let propose_frame = wrap_in_mux_frame(propose_payload, HANDSHAKE_PROTOCOL_ID, MuxMode::Initiator);
    stream.write_all(&propose_frame).await?;
    eprintln!("[cs] sent ProposeVersions");

    // Read handshake reply.
    let (proto, mode, payload, _full) = timeout(Duration::from_secs(10), read_one_frame(&mut stream))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "handshake reply timed out"))??;
    assert_eq!(proto, HANDSHAKE_PROTOCOL_ID, "expected handshake reply");
    assert_eq!(mode, MuxMode::Responder, "expected Responder mode");
    let hs_reply = decode_handshake_message(&payload)
        .map_err(|e| io::Error::other(format!("handshake decode: {e:?}")))?;
    let negotiated_version = match hs_reply {
        HandshakeMessage::AcceptVersion(v, _) => v.get(),
        other => {
            return Err(io::Error::other(format!(
                "handshake did not accept: {other:?}"
            )));
        }
    };
    eprintln!("[cs] handshake accepted at v{negotiated_version}");

    // ---- CHAIN-SYNC: FindIntersect ----
    // Default: Origin (the pseudo-point before any block; every chain
    // has it, so IntersectFound is guaranteed → Byron blocks). When an
    // explicit --intersect-slot/--intersect-hash is supplied, intersect
    // there so RequestNext rolls forward from that chain position.
    let intersect_pt = match intersect_point {
        Some((slot, hash)) => {
            eprintln!("[cs] intersecting at slot {slot}");
            Point::Block {
                slot: ade_types::SlotNo(slot),
                hash: ade_types::Hash32(hash),
            }
        }
        None => Point::Origin,
    };
    let find_intersect = ChainSyncMessage::FindIntersect {
        points: vec![intersect_pt],
    };
    let payload = encode_chain_sync_message(&find_intersect);
    let frame = wrap_in_mux_frame(payload, CHAIN_SYNC_PROTOCOL_ID, MuxMode::Initiator);
    stream.write_all(&frame).await?;
    eprintln!("[cs] sent FindIntersect[Origin]");

    // Read IntersectFound.
    let (proto, _mode, payload, full) =
        timeout(Duration::from_secs(10), read_one_frame(&mut stream))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "intersect reply"))??;
    assert_eq!(proto, CHAIN_SYNC_PROTOCOL_ID);
    let intersect_msg = decode_chain_sync_message(&payload)
        .map_err(|e| io::Error::other(format!("intersect decode: {e:?}")))?;
    match &intersect_msg {
        ChainSyncMessage::IntersectFound { point, tip } => {
            eprintln!(
                "[cs] IntersectFound at {point:?}, peer tip block_no={}",
                tip.block_no
            );
        }
        other => {
            return Err(io::Error::other(format!(
                "expected IntersectFound, got {other:?}"
            )));
        }
    }
    let intersect_path = out_dir.join(format!("{scenario}_intersect_recv.cbor"));
    fs::write(&intersect_path, &full)?;

    // ---- RequestNext loop ----
    //
    // Be polite: send one RequestNext per frame budget. Cap total
    // iterations so we don't accidentally hammer the relay if AwaitReply
    // dominates (we don't expect that at-tip; but defensive).
    let mut frames_captured = 0;
    let mut frame_idx = 0;
    let max_iterations = frame_count * 2 + 5;
    let mut decode_failures = 0;
    while frames_captured < frame_count && frame_idx < max_iterations {
        let request_next = ChainSyncMessage::RequestNext;
        let payload = encode_chain_sync_message(&request_next);
        let frame = wrap_in_mux_frame(payload, CHAIN_SYNC_PROTOCOL_ID, MuxMode::Initiator);
        stream.write_all(&frame).await?;

        let (proto, _mode, payload, full) =
            timeout(Duration::from_secs(15), read_one_frame(&mut stream))
                .await
                .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "request next"))??;
        assert_eq!(proto, CHAIN_SYNC_PROTOCOL_ID);

        let msg = match decode_chain_sync_message(&payload) {
            Ok(m) => m,
            Err(e) => {
                decode_failures += 1;
                eprintln!("[cs] frame {frame_idx}: decode FAILED ({e:?})");
                eprintln!(
                    "[cs] first 32 payload bytes (hex): {}",
                    payload
                        .iter()
                        .take(32)
                        .map(|b| format!("{b:02x}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                if decode_failures >= 2 {
                    eprintln!("[cs] stopping after repeated decode failures");
                    break;
                }
                frame_idx += 1;
                continue;
            }
        };

        // Decode succeeded — save raw frame bytes for the corpus.
        let raw_path = out_dir.join(format!("{scenario}_frame_{frame_idx:02}_recv.cbor"));
        fs::write(&raw_path, &full)?;

        match &msg {
            ChainSyncMessage::RollForward { header, tip } => {
                eprintln!(
                    "[cs] frame {}: RollForward header={} bytes, tip block_no={}",
                    frame_idx,
                    header.len(),
                    tip.block_no
                );
                frames_captured += 1;
            }
            ChainSyncMessage::RollBackward { point, tip } => {
                eprintln!(
                    "[cs] frame {}: RollBackward point={:?} tip block_no={}",
                    frame_idx, point, tip.block_no
                );
                frames_captured += 1;
            }
            ChainSyncMessage::AwaitReply => {
                eprintln!("[cs] frame {frame_idx}: AwaitReply (server has no data yet)");
            }
            other => {
                return Err(io::Error::other(format!(
                    "unexpected chain-sync reply: {other:?}"
                )));
            }
        }
        frame_idx += 1;
        // Polite pace between RequestNext rounds.
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Polite close.
    let done = ChainSyncMessage::Done;
    let payload = encode_chain_sync_message(&done);
    let frame = wrap_in_mux_frame(payload, CHAIN_SYNC_PROTOCOL_ID, MuxMode::Initiator);
    let _ = stream.write_all(&frame).await;
    eprintln!("[cs] sent Done");

    // Metadata.
    let meta_path = out_dir.join(format!("{scenario}_meta.toml"));
    let meta = format!(
        r#"# Captured chain-sync corpus (S-A9 real-capture closure for CE-N-A-2).
peer = "{peer}"
network_magic = {network_magic}
negotiated_n2n_version = {negotiated_version}
frames_captured = {frames_captured}
captured_at_utc = "{utc}"
protocol = "ChainSync"
mini_protocol_id = {protocol_id}
# Each *_frame_NN_recv.cbor contains the full mux frame (8-byte header
# + ChainSync CBOR payload). Strip the first 8 bytes to feed the
# payload through decode_chain_sync_message.
"#,
        peer = peer,
        network_magic = network_magic,
        negotiated_version = negotiated_version,
        frames_captured = frames_captured,
        utc = chrono_like_utc(),
        protocol_id = CHAIN_SYNC_PROTOCOL_ID,
    );
    fs::write(&meta_path, meta)?;
    eprintln!(
        "[cs] captured {frames_captured} chain-sync frames in {out_dir:?}",
        frames_captured = frames_captured,
        out_dir = out_dir
    );

    Ok(())
}

fn parse_hash32(s: &str) -> Option<[u8; 32]> {
    let s = s.trim();
    // ASCII guard: keeps the `s[i*2..i*2+2]` slicing on char boundaries
    // (a 64-byte non-ASCII string would otherwise panic).
    if s.len() != 64 || !s.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
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

fn chrono_like_utc() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    let secs = dur.as_secs();
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let h = rem / 3600;
    let m = (rem % 3600) / 60;
    let s = rem % 60;
    let mut year: u64 = 1970;
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
    let mdays: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0usize;
    while month < 12 && d >= mdays[month] {
        d -= mdays[month];
        month += 1;
    }
    format!(
        "{year:04}-{mm:02}-{dd:02}T{h:02}:{m:02}:{s:02}Z",
        year = year,
        mm = month + 1,
        dd = d + 1,
        h = h,
        m = m,
        s = s
    )
}
