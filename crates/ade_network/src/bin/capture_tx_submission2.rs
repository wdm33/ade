#![allow(clippy::disallowed_types)]
// RED — Imperative Shell capture binary for N2N TxSubmission2
// (protocol 4). Server-driven: after we send MsgInit, the server
// sends MsgRequestTxIds; we reply with an empty MsgReplyTxIds (or a
// pre-loaded set). The corpus capture is the server's RequestTxIds
// + our ReplyTxIds, byte-identical round-trip on both sides.

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
use ade_network::codec::primitives::{encode_array_header, encode_bool, encode_u64};
use ade_network::codec::tx_submission::{
    decode_tx_submission_message, encode_tx_submission_message, TxSubmission2Message,
};
use ade_network::codec::version::N2NVersion;
use ade_network::mux::frame::{
    encode_frame, MiniProtocolId, MuxFrame, MuxHeader, MuxMode, HEADER_LEN,
};

const MAINNET_MAGIC: u32 = 764_824_073;
const PREPROD_MAGIC: u32 = 1;
const PREVIEW_MAGIC: u32 = 2;
const HANDSHAKE_PROTOCOL_ID: u16 = 0;
const TX_SUBMISSION2_PROTOCOL_ID: u16 = 4;

fn version_params_for_n2n(version: u16, magic: u32) -> VersionParams {
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

fn wrap(payload: Vec<u8>, protocol_id: u16) -> Vec<u8> {
    let frame = MuxFrame {
        header: MuxHeader {
            timestamp: now_micros_mod_u32(),
            mode: MuxMode::Initiator,
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
    let peer = arg_value(&args, "--peer").unwrap_or_else(|| "127.0.0.1:3001".into());
    let out_dir = arg_value(&args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("corpus/network/n2n/tx_submission2"));
    let scenario = arg_value(&args, "--scenario").unwrap_or_else(|| "local_preprod".into());
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
    eprintln!("[txsub2] connecting to {peer}");
    let start = Instant::now();
    let mut stream = timeout(Duration::from_secs(10), TcpStream::connect(&peer))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "tcp connect"))??;
    eprintln!("[txsub2] connected in {:.2}s", start.elapsed().as_secs_f64());

    let mut buffers: HashMap<u16, Vec<u8>> = HashMap::new();

    // ---- HANDSHAKE ----
    let proposed: Vec<u16> = vec![11, 12, 13, 14, 15, 16];
    let mut entries = Vec::with_capacity(proposed.len());
    for v in &proposed {
        entries.push((N2NVersion::new(*v), version_params_for_n2n(*v, network_magic)));
    }
    let table = VersionTable(entries);
    let propose = encode_handshake_message(&HandshakeMessage::ProposeVersions(table));
    stream.write_all(&wrap(propose, HANDSHAKE_PROTOCOL_ID)).await?;
    let hs_bytes = read_for_protocol(&mut stream, &mut buffers, HANDSHAKE_PROTOCOL_ID, |b| {
        match decode_handshake_message(b) {
            Ok(_) => Ok(b.len()),
            Err(_) => Err(true),
        }
    })
    .await?;
    let neg = match decode_handshake_message(&hs_bytes)
        .map_err(|e| io::Error::other(format!("hs decode: {e:?}")))?
    {
        HandshakeMessage::AcceptVersion(v, _) => v.get(),
        other => return Err(io::Error::other(format!("hs not accepted: {other:?}"))),
    };
    eprintln!("[txsub2] handshake → V{neg}");

    // ---- TX-SUBMISSION2 ----
    // Send MsgInit (client agency in initial state).
    let init = encode_tx_submission_message(&TxSubmission2Message::Init);
    let init_path = out_dir.join(format!("{scenario}_msg_00_send_init.cbor"));
    fs::write(&init_path, &init)?;
    eprintln!("[txsub2] sent Init ({} bytes)", init.len());
    stream.write_all(&wrap(init, TX_SUBMISSION2_PROTOCOL_ID)).await?;

    // Server will respond with MsgRequestTxIds.
    let req = read_for_protocol(&mut stream, &mut buffers, TX_SUBMISSION2_PROTOCOL_ID, |b| {
        match decode_tx_submission_message(b) {
            Ok(_) => Ok(b.len()),
            Err(_) => Err(true),
        }
    })
    .await?;
    let req_path = out_dir.join(format!("{scenario}_msg_01_recv_request_txids.cbor"));
    fs::write(&req_path, &req)?;
    let req_decoded = decode_tx_submission_message(&req)
        .map_err(|e| io::Error::other(format!("txsub2 decode: {e:?}")))?;
    eprintln!("[txsub2] recv: {:?}", req_decoded);
    let (blocking, ack, req_count) = match req_decoded {
        TxSubmission2Message::RequestTxIds { blocking, ack, req } => (blocking, ack, req),
        other => return Err(io::Error::other(format!("expected RequestTxIds: {other:?}"))),
    };

    // Reply with empty MsgReplyTxIds (we have no txs to offer).
    // If the server requested blocking mode, an empty reply may close
    // the protocol — that's still byte-identical round-trip on our
    // side.
    let reply = encode_tx_submission_message(&TxSubmission2Message::ReplyTxIds(Vec::new()));
    let reply_path = out_dir.join(format!("{scenario}_msg_02_send_reply_txids_empty.cbor"));
    fs::write(&reply_path, &reply)?;
    eprintln!(
        "[txsub2] sent ReplyTxIds (empty) ({} bytes) — server asked blocking={blocking}, ack={ack}, req={req_count}",
        reply.len()
    );
    stream.write_all(&wrap(reply, TX_SUBMISSION2_PROTOCOL_ID)).await?;

    // Best-effort: read whatever comes next (could be another
    // RequestTxIds, or a Done, or the server may simply close).
    let next = read_for_protocol(&mut stream, &mut buffers, TX_SUBMISSION2_PROTOCOL_ID, |b| {
        match decode_tx_submission_message(b) {
            Ok(_) => Ok(b.len()),
            Err(_) => Err(true),
        }
    })
    .await;
    match next {
        Ok(bytes) => {
            let path = out_dir.join(format!("{scenario}_msg_03_recv_next.cbor"));
            fs::write(&path, &bytes)?;
            let decoded = decode_tx_submission_message(&bytes)
                .map_err(|e| io::Error::other(format!("decode: {e:?}")))?;
            eprintln!("[txsub2] recv next: {decoded:?}");
        }
        Err(e) => eprintln!("[txsub2] no further reply: {e}"),
    }

    let meta = format!(
        r#"# Captured tx-submission2 corpus (S-A9 real-capture for CE-N-A-4).
peer = "{peer}"
network_magic = {network_magic}
negotiated_n2n_version = {neg}
protocol = "TxSubmission2"
mini_protocol_id = {protocol_id}
"#,
        peer = peer,
        network_magic = network_magic,
        neg = neg,
        protocol_id = TX_SUBMISSION2_PROTOCOL_ID,
    );
    fs::write(out_dir.join(format!("{scenario}_meta.toml")), meta)?;
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
