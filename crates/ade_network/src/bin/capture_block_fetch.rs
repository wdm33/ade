#![allow(clippy::disallowed_types)]
// RED — Imperative Shell capture binary for N2N BlockFetch.
//
// Flow:
//   1. Handshake (protocol 0).
//   2. Open chain-sync (protocol 2): FindIntersect[Origin] → IntersectFound;
//      extract the peer's tip Point.
//   3. Send a keep-alive ping (protocol 8) so the peer sees a live client.
//   4. Open block-fetch (protocol 3): MsgRequestRange(tip, tip).
//   5. Accumulate frames until we observe MsgStartBatch / MsgBlock / MsgBatchDone
//      (each can span multiple mux frames; reassemble per-protocol buffer).
//   6. Send block-fetch MsgClientDone for polite close.
//
// Captured corpus: each decoded block-fetch message saved as a separate
// <scenario>_msg_NN_<kind>.cbor file containing the FULL CBOR payload
// (reassembled across mux frames). Plus a meta TOML.
//
// EMPIRICAL NOTE (2026-05): IOG-operated public relays
// (preprod-node.play.dev.cardano.org, backbone.cardano.iog.io) accept
// handshake + chain-sync + keep-alive from random clients but reset the
// TCP connection on MsgRequestRange — block-fetch is gated to
// topology-known peers, regardless of `initiator_only_diffusion_mode`.
// The same RequestRange bytes our codec emits are structurally identical
// to what cardano-node produces (verified by raw-hex inspection); the
// reset is a policy decision, not a codec error. Use against a local
// cardano-node or a permissive community relay to obtain real
// block-fetch corpus.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

use ade_network::codec::block_fetch::{
    decode_block_fetch_message, encode_block_fetch_message, BlockFetchMessage,
    Point as BfPoint, Range as BfRange,
};
use ade_network::codec::chain_sync::{
    decode_chain_sync_message, encode_chain_sync_message, ChainSyncMessage,
    Point as CsPoint,
};
use ade_network::codec::handshake::{
    decode_handshake_message, encode_handshake_message, HandshakeMessage, VersionParams,
    VersionTable,
};
use ade_network::codec::keep_alive::{
    decode_keep_alive_message, encode_keep_alive_message, KeepAliveCookie, KeepAliveMessage,
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
const BLOCK_FETCH_PROTOCOL_ID: u16 = 3;
const KEEP_ALIVE_PROTOCOL_ID: u16 = 8;

fn version_params_for_n2n(version: u16, magic: u32) -> VersionParams {
    // V11..V15 wire shape: [networkMagic, initiatorOnlyDiffusionMode, peerSharing, query]
    // V16+ adds peras_support flag.
    // We advertise initiator_only=false so the relay opens both directions and
    // we can run block-fetch (some relays gate cold/initiator-only peers).
    let mut buf = Vec::new();
    let field_count: u64 = if version >= 16 { 5 } else { 4 };
    encode_array_header(&mut buf, field_count);
    encode_u64(&mut buf, magic as u64);
    encode_bool(&mut buf, false);
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

async fn read_one_frame(stream: &mut TcpStream) -> io::Result<(u16, MuxMode, Vec<u8>)> {
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
    Ok((mini_id, mode, payload))
}

/// Read mux frames until we have a complete CBOR payload buffered for
/// `protocol_id`. Other protocols' bytes are accumulated into their
/// own buffers so multiplexed traffic doesn't get dropped.
async fn read_for_protocol<F>(
    stream: &mut TcpStream,
    buffers: &mut HashMap<u16, Vec<u8>>,
    protocol_id: u16,
    try_decode: F,
) -> io::Result<Vec<u8>>
where
    F: Fn(&[u8]) -> Result<usize, bool>, // Ok(consumed) on success, Err(true) = truncated, Err(false) = malformed
{
    loop {
        // Try decoding what's already buffered.
        let buf = buffers.entry(protocol_id).or_default();
        if !buf.is_empty() {
            match try_decode(buf) {
                Ok(consumed) => {
                    let msg_bytes = buf[..consumed].to_vec();
                    buf.drain(..consumed);
                    return Ok(msg_bytes);
                }
                Err(true) => { /* need more */ }
                Err(false) => {
                    return Err(io::Error::other(format!(
                        "malformed payload for protocol {protocol_id}"
                    )));
                }
            }
        }

        let (proto, _mode, payload) =
            timeout(Duration::from_secs(15), read_one_frame(stream))
                .await
                .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "frame read timeout"))??;
        buffers.entry(proto).or_default().extend_from_slice(&payload);
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let peer = arg_value(&args, "--peer")
        .unwrap_or_else(|| "preprod-node.play.dev.cardano.org:3001".into());
    let out_dir = arg_value(&args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("corpus/network/n2n/block_fetch"));
    let scenario =
        arg_value(&args, "--scenario").unwrap_or_else(|| "preprod_tip_one_block".into());
    let network_magic: u32 = arg_value(&args, "--magic")
        .as_deref()
        .map(|s| match s {
            "mainnet" => MAINNET_MAGIC,
            "preprod" => PREPROD_MAGIC,
            "preview" => PREVIEW_MAGIC,
            other => other.parse::<u32>().unwrap_or(PREPROD_MAGIC),
        })
        .unwrap_or(PREPROD_MAGIC);

    fs::create_dir_all(&out_dir)?;

    eprintln!("[bf] connecting to {peer} (magic={network_magic})");
    let start = Instant::now();
    let mut stream = timeout(Duration::from_secs(10), TcpStream::connect(&peer))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "tcp connect"))??;
    eprintln!("[bf] connected in {:.2}s", start.elapsed().as_secs_f64());

    let mut buffers: HashMap<u16, Vec<u8>> = HashMap::new();

    // ---- HANDSHAKE ----
    let proposed: Vec<u16> = vec![11, 12, 13, 14, 15, 16];
    let mut entries = Vec::with_capacity(proposed.len());
    for v in &proposed {
        entries.push((N2NVersion::new(*v), version_params_for_n2n(*v, network_magic)));
    }
    let table = VersionTable(entries);
    let propose_payload = encode_handshake_message(&HandshakeMessage::ProposeVersions(table));
    stream
        .write_all(&wrap_in_mux_frame(
            propose_payload,
            HANDSHAKE_PROTOCOL_ID,
            MuxMode::Initiator,
        ))
        .await?;
    eprintln!("[bf] sent ProposeVersions");

    let hs_bytes = read_for_protocol(&mut stream, &mut buffers, HANDSHAKE_PROTOCOL_ID, |buf| {
        match decode_handshake_message(buf) {
            Ok(_) => Ok(buf.len()),
            Err(_) => Err(true),
        }
    })
    .await?;
    let hs_reply = decode_handshake_message(&hs_bytes)
        .map_err(|e| io::Error::other(format!("handshake decode: {e:?}")))?;
    let negotiated_version = match hs_reply {
        HandshakeMessage::AcceptVersion(v, _) => v.get(),
        other => return Err(io::Error::other(format!("handshake not accepted: {other:?}"))),
    };
    eprintln!("[bf] handshake accepted at v{negotiated_version}");

    // ---- CHAIN-SYNC: learn the peer's tip ----
    let find = ChainSyncMessage::FindIntersect {
        points: vec![CsPoint::Origin],
    };
    stream
        .write_all(&wrap_in_mux_frame(
            encode_chain_sync_message(&find),
            CHAIN_SYNC_PROTOCOL_ID,
            MuxMode::Initiator,
        ))
        .await?;
    eprintln!("[bf] sent chain-sync FindIntersect[Origin]");

    let cs_bytes = read_for_protocol(&mut stream, &mut buffers, CHAIN_SYNC_PROTOCOL_ID, |buf| {
        match decode_chain_sync_message(buf) {
            Ok(_) => Ok(buf.len()),
            Err(_) => Err(true),
        }
    })
    .await?;
    let cs_reply = decode_chain_sync_message(&cs_bytes)
        .map_err(|e| io::Error::other(format!("chain-sync decode: {e:?}")))?;
    let tip_point: CsPoint = match cs_reply {
        ChainSyncMessage::IntersectFound { tip, .. } => {
            eprintln!("[bf] chain-sync tip block_no={}", tip.block_no);
            tip.point
        }
        other => return Err(io::Error::other(format!("expected IntersectFound: {other:?}"))),
    };

    // Translate chain_sync::Point → block_fetch::Point (same shape, different
    // type because each protocol owns its own opaque-grammar surface).
    let bf_tip_point = match &tip_point {
        CsPoint::Origin => BfPoint::Origin,
        CsPoint::Block { slot, hash } => BfPoint::Block {
            slot: *slot,
            hash: hash.clone(),
        },
    };
    if matches!(bf_tip_point, BfPoint::Origin) {
        return Err(io::Error::other(
            "preprod tip is Origin (no blocks yet) — cannot request a range",
        ));
    }

    // Leave chain-sync open in Idle state — sending Done causes some
    // relays to interpret it as a disconnect signal and reset the
    // mux. The mux supports multiple simultaneously-active
    // mini-protocols; block-fetch on a separate channel is fine.

    // ---- KEEP-ALIVE warm-up: some relays gate block-fetch on
    // observable keep-alive activity (cold/warm/hot peer transition).
    // Send one round-trip ping so the peer sees us as a live client.
    let ka_cookie = KeepAliveCookie(0x4ADE);
    stream
        .write_all(&wrap_in_mux_frame(
            encode_keep_alive_message(&KeepAliveMessage::KeepAlive(ka_cookie)),
            KEEP_ALIVE_PROTOCOL_ID,
            MuxMode::Initiator,
        ))
        .await?;
    eprintln!("[bf] sent keep-alive ping (cookie=0x{:04x})", ka_cookie.0);
    let ka_bytes = read_for_protocol(&mut stream, &mut buffers, KEEP_ALIVE_PROTOCOL_ID, |buf| {
        match decode_keep_alive_message(buf) {
            Ok(_) => Ok(buf.len()),
            Err(_) => Err(true),
        }
    })
    .await?;
    let ka_reply = decode_keep_alive_message(&ka_bytes)
        .map_err(|e| io::Error::other(format!("keep-alive decode: {e:?}")))?;
    eprintln!("[bf] keep-alive reply: {ka_reply:?}");

    // ---- BLOCK-FETCH: request (tip, tip) ----
    let request = BlockFetchMessage::RequestRange(BfRange {
        from: bf_tip_point.clone(),
        to: bf_tip_point.clone(),
    });
    let req_payload = encode_block_fetch_message(&request);
    eprintln!(
        "[bf] block-fetch RequestRange tip point = {:?}",
        bf_tip_point
    );
    eprintln!(
        "[bf] block-fetch request hex ({} bytes): {}",
        req_payload.len(),
        req_payload
            .iter()
            .take(80)
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let req_frame = wrap_in_mux_frame(req_payload, BLOCK_FETCH_PROTOCOL_ID, MuxMode::Initiator);
    eprintln!(
        "[bf] full mux frame hex (first 16 bytes): {}",
        req_frame
            .iter()
            .take(16)
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    stream.write_all(&req_frame).await?;
    eprintln!("[bf] sent block-fetch RequestRange(tip, tip)");

    // Read messages until BatchDone or NoBlocks.
    let mut messages_captured = 0;
    let mut msg_idx = 0;
    loop {
        let bytes = read_for_protocol(
            &mut stream,
            &mut buffers,
            BLOCK_FETCH_PROTOCOL_ID,
            |buf| match decode_block_fetch_message(buf) {
                Ok(_) => Ok(buf.len()),
                Err(_) => Err(true),
            },
        )
        .await?;
        let msg = decode_block_fetch_message(&bytes)
            .map_err(|e| io::Error::other(format!("block-fetch decode: {e:?}")))?;
        let kind = match &msg {
            BlockFetchMessage::StartBatch => "start_batch",
            BlockFetchMessage::Block { bytes } => {
                eprintln!("[bf] msg {msg_idx}: Block ({} bytes payload)", bytes.len());
                "block"
            }
            BlockFetchMessage::BatchDone => "batch_done",
            BlockFetchMessage::NoBlocks => "no_blocks",
            other => {
                return Err(io::Error::other(format!(
                    "unexpected block-fetch reply: {other:?}"
                )))
            }
        };
        let path = out_dir.join(format!("{scenario}_msg_{msg_idx:02}_{kind}.cbor"));
        fs::write(&path, &bytes)?;
        eprintln!(
            "[bf] msg {msg_idx} = {kind} ({} bytes) saved to {}",
            bytes.len(),
            path.display()
        );
        messages_captured += 1;
        msg_idx += 1;
        if matches!(msg, BlockFetchMessage::BatchDone | BlockFetchMessage::NoBlocks) {
            break;
        }
        if msg_idx > 20 {
            eprintln!("[bf] safety break after 20 messages");
            break;
        }
    }

    // Polite close.
    stream
        .write_all(&wrap_in_mux_frame(
            encode_block_fetch_message(&BlockFetchMessage::ClientDone),
            BLOCK_FETCH_PROTOCOL_ID,
            MuxMode::Initiator,
        ))
        .await?;
    eprintln!("[bf] sent ClientDone");

    let meta_path = out_dir.join(format!("{scenario}_meta.toml"));
    let meta = format!(
        r#"# Captured block-fetch corpus (S-A9 real-capture portion for CE-N-A-3).
peer = "{peer}"
network_magic = {network_magic}
negotiated_n2n_version = {negotiated_version}
messages_captured = {messages_captured}
captured_at_utc = "{utc}"
protocol = "BlockFetch"
mini_protocol_id = {protocol_id}
# Each *_msg_NN_<kind>.cbor contains the FULL chain of mux frame
# payload bytes (reassembled across mux fragments) that constitute one
# BlockFetch message.
"#,
        peer = peer,
        network_magic = network_magic,
        negotiated_version = negotiated_version,
        messages_captured = messages_captured,
        utc = chrono_like_utc(),
        protocol_id = BLOCK_FETCH_PROTOCOL_ID,
    );
    fs::write(&meta_path, meta)?;

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
