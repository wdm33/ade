#![allow(clippy::disallowed_types)]
// RED — Imperative Shell capture binary for N2N PeerSharing
// (protocol 10). Requires V13+ and peerSharing=PeerSharingEnabled(1)
// in the version params.
//
// Flow:
//   1. Handshake (protocol 0) advertising peerSharing=1.
//   2. Send MsgShareRequest{ amount=N } on protocol 10.
//   3. Read MsgSharePeers{ peers: [...] } (may span multiple mux frames).
//   4. Send MsgDone for polite close.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

use ade_network::codec::handshake::{
    decode_handshake_message, encode_handshake_message, HandshakeMessage, VersionParams,
    VersionTable,
};
use ade_network::codec::peer_sharing::{
    decode_peer_sharing_message, encode_peer_sharing_message, PeerSharingMessage,
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
const PEER_SHARING_PROTOCOL_ID: u16 = 10;

fn version_params_for_n2n(version: u16, magic: u32) -> VersionParams {
    // peerSharing=1 (Enabled) — V13+ supports this.
    let mut buf = Vec::new();
    let field_count: u64 = if version >= 16 { 5 } else { 4 };
    encode_array_header(&mut buf, field_count);
    encode_u64(&mut buf, magic as u64);
    encode_bool(&mut buf, false); // initiator_only_diffusion_mode
    encode_u64(&mut buf, 1); // peerSharing = Enabled
    encode_bool(&mut buf, false); // query
    if version >= 16 {
        encode_bool(&mut buf, false); // peras_support
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

async fn read_for_protocol<F>(
    stream: &mut TcpStream,
    buffers: &mut HashMap<u16, Vec<u8>>,
    protocol_id: u16,
    try_decode: F,
) -> io::Result<Vec<u8>>
where
    F: Fn(&[u8]) -> Result<usize, bool>,
{
    loop {
        let buf = buffers.entry(protocol_id).or_default();
        if !buf.is_empty() {
            match try_decode(buf) {
                Ok(consumed) => {
                    let msg_bytes = buf[..consumed].to_vec();
                    buf.drain(..consumed);
                    return Ok(msg_bytes);
                }
                Err(true) => {}
                Err(false) => {
                    return Err(io::Error::other(format!(
                        "malformed payload for protocol {protocol_id}"
                    )))
                }
            }
        }
        let (proto, _mode, payload) = timeout(Duration::from_secs(60), read_one_frame(stream))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "frame read timeout"))??;
        buffers.entry(proto).or_default().extend_from_slice(&payload);
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let peer = arg_value(&args, "--peer")
        .unwrap_or_else(|| "127.0.0.1:3001".into());
    let out_dir = arg_value(&args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("corpus/network/n2n/peer_sharing"));
    let scenario =
        arg_value(&args, "--scenario").unwrap_or_else(|| "local_preprod_share".into());
    let amount: u8 = arg_value(&args, "--amount")
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
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

    eprintln!("[ps] connecting to {peer} (magic={network_magic})");
    let start = Instant::now();
    let mut stream = timeout(Duration::from_secs(10), TcpStream::connect(&peer))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "tcp connect"))??;
    eprintln!("[ps] connected in {:.2}s", start.elapsed().as_secs_f64());

    let mut buffers: HashMap<u16, Vec<u8>> = HashMap::new();

    // ---- HANDSHAKE with peerSharing=1 ----
    let proposed: Vec<u16> = vec![13, 14, 15, 16];
    let mut entries = Vec::with_capacity(proposed.len());
    for v in &proposed {
        entries.push((N2NVersion::new(*v), version_params_for_n2n(*v, network_magic)));
    }
    let table = VersionTable(entries);
    let propose_payload = encode_handshake_message(&HandshakeMessage::ProposeVersions(table));
    stream
        .write_all(&wrap_in_mux_frame(propose_payload, HANDSHAKE_PROTOCOL_ID, MuxMode::Initiator))
        .await?;
    eprintln!("[ps] sent ProposeVersions (peerSharing=1)");

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
    eprintln!("[ps] handshake accepted at v{negotiated_version}");

    // ---- PEER-SHARING request ----
    let request = PeerSharingMessage::ShareRequest { amount };
    let send_bytes = encode_peer_sharing_message(&request);
    let send_path = out_dir.join(format!("{scenario}_msg_00_send_share_request.cbor"));
    fs::write(&send_path, &send_bytes)?;
    eprintln!(
        "[ps] sent ShareRequest amount={amount} ({} bytes -> {})",
        send_bytes.len(),
        send_path.display()
    );
    stream
        .write_all(&wrap_in_mux_frame(send_bytes, PEER_SHARING_PROTOCOL_ID, MuxMode::Initiator))
        .await?;

    let recv_bytes =
        read_for_protocol(&mut stream, &mut buffers, PEER_SHARING_PROTOCOL_ID, |buf| {
            match decode_peer_sharing_message(buf) {
                Ok(_) => Ok(buf.len()),
                Err(_) => Err(true),
            }
        })
        .await?;
    let reply = decode_peer_sharing_message(&recv_bytes)
        .map_err(|e| io::Error::other(format!("peer-sharing decode: {e:?}")))?;
    let recv_path = out_dir.join(format!("{scenario}_msg_01_recv_share_peers.cbor"));
    fs::write(&recv_path, &recv_bytes)?;
    eprintln!(
        "[ps] recv reply ({} bytes -> {})",
        recv_bytes.len(),
        recv_path.display()
    );
    let returned = match &reply {
        PeerSharingMessage::SharePeers { peers } => peers.len(),
        other => {
            return Err(io::Error::other(format!(
                "expected SharePeers, got {other:?}"
            )))
        }
    };
    eprintln!("[ps] returned {returned} peer address(es)");

    // Polite close.
    stream
        .write_all(&wrap_in_mux_frame(
            encode_peer_sharing_message(&PeerSharingMessage::Done),
            PEER_SHARING_PROTOCOL_ID,
            MuxMode::Initiator,
        ))
        .await?;
    eprintln!("[ps] sent Done");

    let meta_path = out_dir.join(format!("{scenario}_meta.toml"));
    let meta = format!(
        r#"# Captured peer-sharing corpus (S-A9 real-capture portion for CE-N-A-6).
peer = "{peer}"
network_magic = {network_magic}
negotiated_n2n_version = {negotiated_version}
amount_requested = {amount}
peers_returned = {returned}
captured_at_utc = "{utc}"
protocol = "PeerSharing"
mini_protocol_id = {protocol_id}
"#,
        peer = peer,
        network_magic = network_magic,
        negotiated_version = negotiated_version,
        amount = amount,
        returned = returned,
        utc = chrono_like_utc(),
        protocol_id = PEER_SHARING_PROTOCOL_ID,
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
        31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mut month = 0usize;
    while month < 12 && d >= mdays[month] {
        d -= mdays[month];
        month += 1;
    }
    format!(
        "{year:04}-{mm:02}-{dd:02}T{h:02}:{m:02}:{s:02}Z",
        year = year, mm = month + 1, dd = d + 1, h = h, m = m, s = s
    )
}
