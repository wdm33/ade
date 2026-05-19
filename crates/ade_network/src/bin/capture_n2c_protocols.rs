#![allow(clippy::disallowed_types)]
// RED — Imperative Shell capture binary for the 4 N2C mini-protocols
// (LocalChainSync, LocalTxSubmission, LocalStateQuery, LocalTxMonitor).
//
// One Unix-socket connection, one handshake, then run each protocol
// in turn. Each message's mux-reassembled payload is written to
// corpus/network/n2c/<protocol>/local_preprod_<scenario>_msg_NN_<dir>_<kind>.cbor.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::{timeout, Duration};

use ade_network::codec::local_chain_sync::{
    decode_local_chain_sync_message, encode_local_chain_sync_message, LocalChainSyncMessage,
    Point as LcsPoint,
};
use ade_network::codec::local_state_query::{
    decode_local_state_query_message, encode_local_state_query_message, LocalStateQueryMessage,
};
use ade_network::codec::local_tx_monitor::{
    decode_local_tx_monitor_message, encode_local_tx_monitor_message, LocalTxMonitorMessage,
};
use ade_network::codec::local_tx_submission::{
    decode_local_tx_submission_message, encode_local_tx_submission_message,
    LocalTxSubmissionMessage,
};
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
const LOCAL_CHAIN_SYNC_PROTOCOL_ID: u16 = 5;
const LOCAL_TX_SUBMISSION_PROTOCOL_ID: u16 = 6;
const LOCAL_STATE_QUERY_PROTOCOL_ID: u16 = 7;
const LOCAL_TX_MONITOR_PROTOCOL_ID: u16 = 9;

fn version_params_for_n2c(magic: u32) -> N2cVersionParams {
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
        let (proto, _mode, payload) = timeout(Duration::from_secs(120), read_one_frame(stream))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "frame read timeout"))??;
        buffers.entry(proto).or_default().extend_from_slice(&payload);
    }
}

fn save(out: &PathBuf, name: String, bytes: &[u8]) -> io::Result<()> {
    fs::create_dir_all(out)?;
    let path = out.join(name);
    fs::write(&path, bytes)?;
    eprintln!("    saved {} ({} bytes)", path.display(), bytes.len());
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let socket_path = arg_value(&args, "--socket")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/cardano/ipc/node.socket"));
    let out_root = arg_value(&args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("corpus/network/n2c"));
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

    eprintln!("[n2c] connecting to {}", socket_path.display());
    let start = Instant::now();
    let mut stream = timeout(Duration::from_secs(10), UnixStream::connect(&socket_path))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "uds connect"))??;
    eprintln!("[n2c] connected in {:.2}s", start.elapsed().as_secs_f64());

    let mut buffers: HashMap<u16, Vec<u8>> = HashMap::new();

    // ---- HANDSHAKE ----
    let proposed: Vec<u16> = vec![16, 17, 18, 19, 20, 21, 22, 23];
    let mut entries = Vec::with_capacity(proposed.len());
    for v in &proposed {
        entries.push((N2CVersion::new(*v), version_params_for_n2c(network_magic)));
    }
    let table = N2cVersionTable(entries);
    let propose_payload =
        encode_n2c_handshake_message(&N2cHandshakeMessage::ProposeVersions(table));
    stream
        .write_all(&wrap(propose_payload, HANDSHAKE_PROTOCOL_ID))
        .await?;
    let hs_bytes = read_for_protocol(&mut stream, &mut buffers, HANDSHAKE_PROTOCOL_ID, |buf| {
        match decode_n2c_handshake_message(buf) {
            Ok(_) => Ok(buf.len()),
            Err(_) => Err(true),
        }
    })
    .await?;
    let neg_v = match decode_n2c_handshake_message(&hs_bytes)
        .map_err(|e| io::Error::other(format!("hs decode: {e:?}")))?
    {
        N2cHandshakeMessage::AcceptVersion(v, _) => v.get(),
        other => return Err(io::Error::other(format!("hs not accepted: {other:?}"))),
    };
    eprintln!("[n2c] handshake → V{neg_v}");

    // ---- LSQ: Acquire(immutable tip) → Acquired → Release → Done ----
    eprintln!("[n2c] LSQ flow");
    let lsq_dir = out_root.join("local_state_query");
    let send_acq = encode_local_state_query_message(&LocalStateQueryMessage::AcquireNoPoint);
    save(
        &lsq_dir,
        format!("{scenario}_msg_00_send_acquire_no_point.cbor"),
        &send_acq,
    )?;
    stream
        .write_all(&wrap(send_acq, LOCAL_STATE_QUERY_PROTOCOL_ID))
        .await?;
    let acq_reply = read_for_protocol(
        &mut stream,
        &mut buffers,
        LOCAL_STATE_QUERY_PROTOCOL_ID,
        |b| match decode_local_state_query_message(b) {
            Ok(_) => Ok(b.len()),
            Err(_) => Err(true),
        },
    )
    .await?;
    save(
        &lsq_dir,
        format!("{scenario}_msg_01_recv_acquired_or_failure.cbor"),
        &acq_reply,
    )?;
    let acq_decoded = decode_local_state_query_message(&acq_reply)
        .map_err(|e| io::Error::other(format!("lsq decode: {e:?}")))?;
    eprintln!("    LSQ recv: {:?}", acq_decoded);
    // Release + Done (best-effort polite close — proceeds even if peer
    // sent Failure)
    if matches!(acq_decoded, LocalStateQueryMessage::Acquired) {
        let rel = encode_local_state_query_message(&LocalStateQueryMessage::Release);
        save(
            &lsq_dir,
            format!("{scenario}_msg_02_send_release.cbor"),
            &rel,
        )?;
        stream
            .write_all(&wrap(rel, LOCAL_STATE_QUERY_PROTOCOL_ID))
            .await?;
    }
    let done = encode_local_state_query_message(&LocalStateQueryMessage::Done);
    save(&lsq_dir, format!("{scenario}_msg_03_send_done.cbor"), &done)?;
    stream
        .write_all(&wrap(done, LOCAL_STATE_QUERY_PROTOCOL_ID))
        .await?;

    // ---- LocalChainSync: FindIntersect[Origin] → IntersectFound ----
    eprintln!("[n2c] LocalChainSync flow");
    let lcs_dir = out_root.join("local_chain_sync");
    let find = encode_local_chain_sync_message(&LocalChainSyncMessage::FindIntersect {
        points: vec![LcsPoint::Origin],
    });
    save(
        &lcs_dir,
        format!("{scenario}_msg_00_send_find_intersect.cbor"),
        &find,
    )?;
    stream
        .write_all(&wrap(find, LOCAL_CHAIN_SYNC_PROTOCOL_ID))
        .await?;
    let intersect_reply =
        read_for_protocol(&mut stream, &mut buffers, LOCAL_CHAIN_SYNC_PROTOCOL_ID, |b| {
            match decode_local_chain_sync_message(b) {
                Ok(_) => Ok(b.len()),
                Err(_) => Err(true),
            }
        })
        .await?;
    save(
        &lcs_dir,
        format!("{scenario}_msg_01_recv_intersect_reply.cbor"),
        &intersect_reply,
    )?;
    let intersect_decoded = decode_local_chain_sync_message(&intersect_reply)
        .map_err(|e| io::Error::other(format!("lcs decode: {e:?}")))?;
    eprintln!("    LocalChainSync recv: {:?}",
        match &intersect_decoded {
            LocalChainSyncMessage::IntersectFound { point, tip } =>
                format!("IntersectFound(point={:?}, tip_block={})", point, tip.block_no),
            other => format!("{other:?}"),
        });
    let done_lcs = encode_local_chain_sync_message(&LocalChainSyncMessage::Done);
    save(
        &lcs_dir,
        format!("{scenario}_msg_02_send_done.cbor"),
        &done_lcs,
    )?;
    stream
        .write_all(&wrap(done_lcs, LOCAL_CHAIN_SYNC_PROTOCOL_ID))
        .await?;

    // ---- LocalTxMonitor: Acquire → GetSizes → Release → Done ----
    // Done BEFORE LocalTxSubmission because a garbage-tx submit may
    // trigger a fatal decoder error in cardano-node that closes the
    // entire mux bearer.
    eprintln!("[n2c] LocalTxMonitor flow (before LTS to avoid mux death)");
    let ltm_dir_first = out_root.join("local_tx_monitor");
    let acq_ltm = encode_local_tx_monitor_message(&LocalTxMonitorMessage::Acquire);
    save(
        &ltm_dir_first,
        format!("{scenario}_msg_00_send_acquire.cbor"),
        &acq_ltm,
    )?;
    stream
        .write_all(&wrap(acq_ltm, LOCAL_TX_MONITOR_PROTOCOL_ID))
        .await?;
    let acquired_reply = read_for_protocol(
        &mut stream,
        &mut buffers,
        LOCAL_TX_MONITOR_PROTOCOL_ID,
        |b| match decode_local_tx_monitor_message(b) {
            Ok(_) => Ok(b.len()),
            Err(_) => Err(true),
        },
    )
    .await?;
    save(
        &ltm_dir_first,
        format!("{scenario}_msg_01_recv_acquired.cbor"),
        &acquired_reply,
    )?;
    eprintln!(
        "    LocalTxMonitor recv: {:?}",
        decode_local_tx_monitor_message(&acquired_reply)
            .map_err(|e| io::Error::other(format!("ltm decode: {e:?}")))?
    );
    let get_sizes = encode_local_tx_monitor_message(&LocalTxMonitorMessage::GetSizes);
    save(
        &ltm_dir_first,
        format!("{scenario}_msg_02_send_get_sizes.cbor"),
        &get_sizes,
    )?;
    stream
        .write_all(&wrap(get_sizes, LOCAL_TX_MONITOR_PROTOCOL_ID))
        .await?;
    let sizes_reply = read_for_protocol(
        &mut stream,
        &mut buffers,
        LOCAL_TX_MONITOR_PROTOCOL_ID,
        |b| match decode_local_tx_monitor_message(b) {
            Ok(_) => Ok(b.len()),
            Err(_) => Err(true),
        },
    )
    .await?;
    save(
        &ltm_dir_first,
        format!("{scenario}_msg_03_recv_reply_get_sizes.cbor"),
        &sizes_reply,
    )?;
    eprintln!(
        "    LocalTxMonitor recv: {:?}",
        decode_local_tx_monitor_message(&sizes_reply)
            .map_err(|e| io::Error::other(format!("ltm decode: {e:?}")))?
    );
    let release_ltm = encode_local_tx_monitor_message(&LocalTxMonitorMessage::Release);
    save(
        &ltm_dir_first,
        format!("{scenario}_msg_04_send_release.cbor"),
        &release_ltm,
    )?;
    stream
        .write_all(&wrap(release_ltm, LOCAL_TX_MONITOR_PROTOCOL_ID))
        .await?;
    let done_ltm_early = encode_local_tx_monitor_message(&LocalTxMonitorMessage::Done);
    save(
        &ltm_dir_first,
        format!("{scenario}_msg_05_send_done.cbor"),
        &done_ltm_early,
    )?;
    stream
        .write_all(&wrap(done_ltm_early, LOCAL_TX_MONITOR_PROTOCOL_ID))
        .await?;

    // ---- LocalTxSubmission: submit a properly-era-wrapped but
    // semantically-invalid tx; expect RejectTx with a ledger error.
    eprintln!("[n2c] LocalTxSubmission flow (expecting Reject for garbage tx)");
    let lts_dir = out_root.join("local_tx_submission");
    // HFC GenTx wire form: [era_idx, tag24(bytes(inner_cbor))]
    // Inner is `80` (CBOR array(0)) so the CBOR parses cleanly but the
    // ledger tx decoder rejects it (Conway tx expects 4+ elements).
    // [array(2), uint(6=Conway), tag(24), bytes(1), array(0)]
    let wrapped_garbage: Vec<u8> = vec![0x82, 0x06, 0xd8, 0x18, 0x41, 0x80];
    let submit = encode_local_tx_submission_message(&LocalTxSubmissionMessage::SubmitTx {
        tx_bytes: wrapped_garbage,
    });
    save(
        &lts_dir,
        format!("{scenario}_msg_00_send_submit_empty.cbor"),
        &submit,
    )?;
    if stream.write_all(&wrap(submit, LOCAL_TX_SUBMISSION_PROTOCOL_ID)).await.is_err() {
        eprintln!("    LTS submit write failed; mux likely closed");
    }
    // Best-effort: read whatever the peer sends back. If the inner-tx
    // CBOR triggers a fatal ledger decoder failure on the node side
    // (which is the typical outcome for a garbage tx), the mux closes
    // and our read returns EOF. We still captured the send-side
    // SubmitTx wire bytes which is what S-A9 §11 calls for.
    let lts_reply = read_for_protocol(
        &mut stream,
        &mut buffers,
        LOCAL_TX_SUBMISSION_PROTOCOL_ID,
        |b| match decode_local_tx_submission_message(b) {
            Ok(_) => Ok(b.len()),
            Err(_) => Err(true),
        },
    )
    .await;
    match lts_reply {
        Ok(bytes) => {
            save(
                &lts_dir,
                format!("{scenario}_msg_01_recv_reject_or_accept.cbor"),
                &bytes,
            )?;
            match decode_local_tx_submission_message(&bytes) {
                Ok(LocalTxSubmissionMessage::AcceptTx(_)) => {
                    eprintln!("    LocalTxSubmission recv: AcceptTx")
                }
                Ok(LocalTxSubmissionMessage::RejectTx(r)) => {
                    eprintln!("    LocalTxSubmission recv: RejectTx({} bytes)", r.0.len())
                }
                Ok(other) => eprintln!("    LocalTxSubmission recv: {other:?}"),
                Err(e) => eprintln!("    LocalTxSubmission decode error: {e:?}"),
            }
        }
        Err(e) => {
            eprintln!("    LTS no reply (expected for garbage tx): {e}");
        }
    }

    eprintln!("[n2c] N2C capture complete against V{neg_v}");
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
