#![allow(clippy::disallowed_types)]
// RED — Imperative Shell capture binary for N2C Handshake (protocol 0
// on the local Unix-socket bearer).
//
// Flow:
//   1. Connect to /opt/cardano/ipc/node.socket (or --socket override).
//   2. Send N2C handshake ProposeVersions for V16..V23.
//   3. Read AcceptVersion (or Refuse).
//   4. Send a polite mux-level disconnect (close the stream).

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::{timeout, Duration};

use ade_network::codec::n2c_handshake::{
    decode_n2c_handshake_message, encode_n2c_handshake_message, N2cHandshakeMessage,
    N2cVersionParams, N2cVersionTable,
};
use ade_network::codec::primitives::{encode_array_header, encode_bool, encode_u64};
use ade_network::codec::version::N2CVersion;
use ade_network::mux::frame::{
    encode_frame, MiniProtocolId, MuxFrame, MuxHeader, MuxMode, HEADER_LEN,
};

const MAINNET_MAGIC: u32 = 764_824_073;
const PREPROD_MAGIC: u32 = 1;
const PREVIEW_MAGIC: u32 = 2;
const HANDSHAKE_PROTOCOL_ID: u16 = 0;

fn version_params_for_n2c(magic: u32) -> N2cVersionParams {
    // N2C V16..V23 params: [networkMagic, query]
    let mut buf = Vec::new();
    encode_array_header(&mut buf, 2);
    encode_u64(&mut buf, magic as u64);
    encode_bool(&mut buf, false); // query=false
    N2cVersionParams(buf)
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

async fn read_one_frame(stream: &mut UnixStream) -> io::Result<(u16, MuxMode, Vec<u8>)> {
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
    stream: &mut UnixStream,
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
        let (proto, _mode, payload) = timeout(Duration::from_secs(30), read_one_frame(stream))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "frame read timeout"))??;
        buffers.entry(proto).or_default().extend_from_slice(&payload);
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let socket_path = arg_value(&args, "--socket")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/cardano/ipc/node.socket"));
    let out_dir = arg_value(&args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("corpus/network/n2c/handshake"));
    let scenario =
        arg_value(&args, "--scenario").unwrap_or_else(|| "local_preprod_handshake".into());
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

    eprintln!("[n2c] connecting to {} (magic={network_magic})", socket_path.display());
    let start = Instant::now();
    let mut stream = timeout(Duration::from_secs(10), UnixStream::connect(&socket_path))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "uds connect"))??;
    eprintln!("[n2c] connected in {:.2}s", start.elapsed().as_secs_f64());

    let mut buffers: HashMap<u16, Vec<u8>> = HashMap::new();

    // ---- N2C HANDSHAKE ----
    let proposed: Vec<u16> = vec![16, 17, 18, 19, 20, 21, 22, 23];
    let mut entries = Vec::with_capacity(proposed.len());
    for v in &proposed {
        entries.push((N2CVersion::new(*v), version_params_for_n2c(network_magic)));
    }
    let table = N2cVersionTable(entries);
    let propose_msg = N2cHandshakeMessage::ProposeVersions(table);
    let propose_payload = encode_n2c_handshake_message(&propose_msg);
    let send_path = out_dir.join(format!("{scenario}_msg_00_send_propose.cbor"));
    fs::write(&send_path, &propose_payload)?;
    eprintln!(
        "[n2c] sent ProposeVersions for V16..V23 ({} bytes -> {})",
        propose_payload.len(),
        send_path.display()
    );
    stream
        .write_all(&wrap_in_mux_frame(propose_payload, HANDSHAKE_PROTOCOL_ID, MuxMode::Initiator))
        .await?;

    let hs_bytes = read_for_protocol(&mut stream, &mut buffers, HANDSHAKE_PROTOCOL_ID, |buf| {
        match decode_n2c_handshake_message(buf) {
            Ok(_) => Ok(buf.len()),
            Err(_) => Err(true),
        }
    })
    .await?;
    let recv_path = out_dir.join(format!("{scenario}_msg_01_recv_reply.cbor"));
    fs::write(&recv_path, &hs_bytes)?;
    eprintln!(
        "[n2c] recv reply ({} bytes -> {})",
        hs_bytes.len(),
        recv_path.display()
    );
    let hs_reply = decode_n2c_handshake_message(&hs_bytes)
        .map_err(|e| io::Error::other(format!("n2c handshake decode: {e:?}")))?;
    let negotiated_version = match &hs_reply {
        N2cHandshakeMessage::AcceptVersion(v, _) => v.get(),
        N2cHandshakeMessage::Refuse(reason) => {
            return Err(io::Error::other(format!("n2c handshake refused: {reason:?}")))
        }
        other => return Err(io::Error::other(format!("unexpected n2c handshake: {other:?}"))),
    };
    eprintln!("[n2c] handshake accepted at V{negotiated_version}");

    let meta_path = out_dir.join(format!("{scenario}_meta.toml"));
    let meta = format!(
        r#"# Captured N2C handshake corpus (S-A9 real-capture for N2C handshake).
socket = "{}"
network_magic = {network_magic}
negotiated_n2c_version = {negotiated_version}
captured_at_utc = "{utc}"
protocol = "N2cHandshake"
mini_protocol_id = {protocol_id}
"#,
        socket_path.display(),
        network_magic = network_magic,
        negotiated_version = negotiated_version,
        utc = chrono_like_utc(),
        protocol_id = HANDSHAKE_PROTOCOL_ID,
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
