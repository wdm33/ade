#![allow(clippy::disallowed_types)]
// RED — Imperative Shell capture binary.
//
// Connects to a public mainnet relay via TCP, performs the N2N handshake
// using our own BLUE codec + state machine, captures the cardano-node
// reply bytes verbatim, and writes both directions to disk for use as
// real-capture corpus fixtures (S-A9 obligation).
//
// This binary is NOT part of the BLUE authority surface. It uses tokio,
// wall-clock, and stdout — all permitted in RED. The corpus it produces
// is the canonical reference: any future codec change must round-trip
// these captured bytes byte-identically.
//
// Usage:
//   cargo run -p ade_network --bin ade_handshake_capture -- \
//       --peer backbone.cardano.iog.io:3001 \
//       --out corpus/network/n2n/handshake/
//
// Output (per scenario):
//   <out>/<scenario>_sent.cbor    raw bytes we sent (MsgProposeVersions)
//   <out>/<scenario>_recv.cbor    raw bytes the peer sent us (MsgAcceptVersion or MsgRefuse)
//   <out>/<scenario>_meta.toml    metadata: peer addr, network magic, versions proposed, frame timestamp

use std::env;
use std::fs;
use std::io::{self};
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

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
const HANDSHAKE_PROTOCOL_ID: u16 = 0;

fn version_params_for_n2n(version: u16) -> VersionParams {
    let mut buf = Vec::new();
    let field_count: u64 = if version >= 16 { 5 } else { 4 };
    encode_array_header(&mut buf, field_count);
    encode_u64(&mut buf, MAINNET_MAGIC as u64);
    // initiatorOnlyDiffusionMode = true: we are a client, never accept inbound.
    encode_bool(&mut buf, true);
    // peerSharing = NoPeerSharing (0).
    encode_u64(&mut buf, 0);
    // query = false: not a diagnostic handshake.
    encode_bool(&mut buf, false);
    if version >= 16 {
        // perasSupport = false (introduced at NodeToNodeV_16).
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let peer = arg_value(&args, "--peer").unwrap_or_else(|| "backbone.cardano.iog.io:3001".into());
    let out_dir = arg_value(&args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("corpus/network/n2n/handshake"));
    let scenario =
        arg_value(&args, "--scenario").unwrap_or_else(|| "mainnet_v11_v16_propose".into());

    fs::create_dir_all(&out_dir)?;

    eprintln!("[capture] connecting to {peer}");
    let start = Instant::now();
    let mut stream = timeout(Duration::from_secs(10), TcpStream::connect(&peer))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "tcp connect timed out"))??;
    eprintln!(
        "[capture] connected in {:.2}s",
        start.elapsed().as_secs_f64()
    );

    // Versions to propose. Defaults to V11..V16 (full cardano-node
    // 10.6.2..11.0.1 range); --versions overrides.
    let proposed: Vec<u16> = arg_value(&args, "--versions")
        .map(|s| {
            s.split(',')
                .filter_map(|x| x.trim().parse::<u16>().ok())
                .collect()
        })
        .unwrap_or_else(|| vec![11, 12, 13, 14, 15, 16]);
    let mut entries = Vec::with_capacity(proposed.len());
    for v in &proposed {
        entries.push((N2NVersion::new(*v), version_params_for_n2n(*v)));
    }
    let table = VersionTable(entries);
    let payload = encode_handshake_message(&HandshakeMessage::ProposeVersions(table));
    eprintln!("[capture] handshake CBOR length: {} bytes", payload.len());

    let frame = MuxFrame {
        header: MuxHeader {
            timestamp: now_micros_mod_u32(),
            mode: MuxMode::Initiator,
            mini_protocol_id: MiniProtocolId::new(HANDSHAKE_PROTOCOL_ID)
                .expect("handshake id is 0"),
            length: payload.len() as u16,
        },
        payload,
    };
    let sent_bytes = encode_frame(&frame).expect("encode initiator frame");
    eprintln!("[capture] mux frame length: {} bytes", sent_bytes.len());

    // Persist sent bytes BEFORE writing to socket so we have them even
    // if the peer hangs up.
    let sent_path = out_dir.join(format!("{scenario}_sent.cbor"));
    fs::write(&sent_path, &sent_bytes)?;
    eprintln!("[capture] wrote sent bytes to {}", sent_path.display());

    stream.write_all(&sent_bytes).await?;
    eprintln!("[capture] sent ProposeVersions; waiting for reply");

    // Read response frame(s). Mux header is 8 bytes; we read header
    // first to know payload length, then the payload. Repeat until we
    // observe a complete handshake reply (MsgAcceptVersion / MsgRefuse
    // / MsgQueryReply).
    let mut recv_bytes = Vec::new();
    let recv_start = Instant::now();
    loop {
        let mut header_buf = [0u8; HEADER_LEN];
        let read_result = timeout(Duration::from_secs(10), stream.read_exact(&mut header_buf)).await;
        match read_result {
            Ok(Ok(_)) => {}
            Ok(Err(e)) if e.kind() == io::ErrorKind::UnexpectedEof => {
                eprintln!("[capture] peer closed connection (EOF)");
                break;
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                eprintln!("[capture] read timed out after 10s");
                break;
            }
        }
        recv_bytes.extend_from_slice(&header_buf);

        // Parse header in-place (decode_frame would require the full
        // payload to be present, but we read payload below).
        let id_word = u16::from_be_bytes([header_buf[4], header_buf[5]]);
        let mode_bit_set = id_word & 0x8000 != 0;
        let mini_id = id_word & 0x7FFF;
        let mode = if mode_bit_set {
            MuxMode::Responder
        } else {
            MuxMode::Initiator
        };
        let payload_len = u16::from_be_bytes([header_buf[6], header_buf[7]]) as usize;

        // Read the announced payload.
        let mut payload_buf = vec![0u8; payload_len];
        if payload_len > 0 {
            stream.read_exact(&mut payload_buf).await?;
            recv_bytes.extend_from_slice(&payload_buf);
        }

        eprintln!(
            "[capture] recv frame: protocol_id={} mode={:?} payload_len={}",
            mini_id, mode, payload_len
        );

        // Decode handshake message from payload. If decode succeeds, we
        // can stop reading.
        if mini_id == HANDSHAKE_PROTOCOL_ID {
            match decode_handshake_message(&payload_buf) {
                Ok(msg) => {
                    let kind = match &msg {
                        HandshakeMessage::AcceptVersion(v, _) => {
                            format!("AcceptVersion(v={})", v.get())
                        }
                        HandshakeMessage::Refuse(reason) => format!("Refuse({reason:?})"),
                        HandshakeMessage::QueryReply(_) => "QueryReply".into(),
                        HandshakeMessage::ProposeVersions(_) => "ProposeVersions".into(),
                    };
                    eprintln!("[capture] decoded handshake reply: {kind}");
                    break;
                }
                Err(e) => {
                    eprintln!(
                        "[capture] handshake decode failed (will keep reading): {e:?}"
                    );
                }
            }
        }

        if recv_start.elapsed() > Duration::from_secs(15) {
            eprintln!("[capture] giving up — no decoded handshake reply within 15s");
            break;
        }
    }

    let recv_path = out_dir.join(format!("{scenario}_recv.cbor"));
    fs::write(&recv_path, &recv_bytes)?;
    eprintln!(
        "[capture] wrote {} received bytes to {}",
        recv_bytes.len(),
        recv_path.display()
    );

    let meta_path = out_dir.join(format!("{scenario}_meta.toml"));
    let meta = format!(
        r#"# Captured handshake fixture (S-A9 corpus).
peer = "{peer}"
captured_at_utc = "{utc}"
network_magic = {magic}
versions_proposed = {versions:?}
expected_negotiated = "v16 (cardano-node 11.0.1 max common)"
sent_bytes_len = {sent_len}
recv_bytes_len = {recv_len}
protocol = "Handshake"
direction_sent = "Initiator"
direction_recv = "Responder"
mini_protocol_id = {protocol_id}
"#,
        peer = peer,
        utc = chrono_like_utc(),
        magic = MAINNET_MAGIC,
        versions = proposed,
        sent_len = sent_bytes.len(),
        recv_len = recv_bytes.len(),
        protocol_id = HANDSHAKE_PROTOCOL_ID,
    );
    fs::write(&meta_path, meta)?;
    eprintln!("[capture] wrote metadata to {}", meta_path.display());

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
    // Minimal ISO-8601-ish UTC timestamp without pulling chrono in.
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    let secs = dur.as_secs();
    // Days since 1970-01-01 (ignoring leap seconds).
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let h = rem / 3600;
    let m = (rem % 3600) / 60;
    let s = rem % 60;
    // Crude date arithmetic — good enough for a metadata stamp.
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
