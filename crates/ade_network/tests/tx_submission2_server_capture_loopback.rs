// Hermetic loopback smoke test for the SERVER-SIDE tx-submission2 capture
// harness (`ade_tx_submission2_server_capture`).
//
// This test plays the cardano-node's role against the real harness binary:
// it dials the harness, sends ProposeVersions (handshake client), then acts
// as the tx-submission2 CLIENT (provider) — sending MsgInit, then offering
// tx ids (MsgReplyTxIds) and a tx body (MsgReplyTxs) in response to the
// harness's RequestTxIds / RequestTxs. It then asserts the harness exited 0
// and wrote round-trippable corpus frames for the rich node-originated
// messages.
//
// Ade-vs-Ade: this proves the harness's wiring (handshake responder, SERVER
// role, request decisions, frame capture) is internally consistent. It does
// NOT prove conformance to the cardano-node Haskell wire grammar — that comes
// from the live capture consumed by tx_submission2_real_capture_corpus.rs.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::disallowed_types)]

use std::process::{Command, Stdio};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

use ade_network::codec::handshake::{
    decode_handshake_message, encode_handshake_message, HandshakeMessage, VersionParams,
    VersionTable,
};
use ade_network::codec::primitives::{encode_array_header, encode_bool, encode_u64};
use ade_network::codec::tx_submission::{
    decode_tx_submission_message, encode_tx_submission_message, TxIdAndSize, TxSubmission2Message,
    TxSubmissionTxId,
};
use ade_network::codec::version::N2NVersion;
use ade_network::mux::frame::{
    encode_frame, MiniProtocolId, MuxFrame, MuxHeader, MuxMode, HEADER_LEN,
};
use ade_types::{Hash32, TxId};

const HANDSHAKE: u16 = 0;
const TX_SUBMISSION2: u16 = 4;

fn free_port() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let p = l.local_addr().unwrap().port();
    drop(l);
    p
}

fn n2n_params(magic: u64) -> VersionParams {
    let mut buf = Vec::new();
    encode_array_header(&mut buf, 4);
    encode_u64(&mut buf, magic);
    encode_bool(&mut buf, false);
    encode_u64(&mut buf, 0);
    encode_bool(&mut buf, false);
    VersionParams(buf)
}

/// Wrap a payload as the connection initiator (the node's role).
fn wrap_initiator(payload: Vec<u8>, proto: u16) -> Vec<u8> {
    let frame = MuxFrame {
        header: MuxHeader {
            timestamp: 0,
            mode: MuxMode::Initiator,
            mini_protocol_id: MiniProtocolId::new(proto).unwrap(),
            length: payload.len() as u16,
        },
        payload,
    };
    encode_frame(&frame).unwrap()
}

async fn read_frame(stream: &mut TcpStream) -> (u16, Vec<u8>) {
    let mut header = [0u8; HEADER_LEN];
    timeout(Duration::from_secs(10), stream.read_exact(&mut header))
        .await
        .expect("frame header timeout")
        .expect("read header");
    let proto = u16::from_be_bytes([header[4], header[5]]) & 0x7FFF;
    let len = u16::from_be_bytes([header[6], header[7]]) as usize;
    let mut payload = vec![0u8; len];
    if len > 0 {
        timeout(Duration::from_secs(10), stream.read_exact(&mut payload))
            .await
            .expect("frame payload timeout")
            .expect("read payload");
    }
    (proto, payload)
}

/// Read frames until we get one on `want_proto`, decoding it as a tx-sub
/// message (skips e.g. keep-alive).
async fn read_tx_sub(stream: &mut TcpStream) -> TxSubmission2Message {
    loop {
        let (proto, payload) = read_frame(stream).await;
        if proto == TX_SUBMISSION2 {
            return decode_tx_submission_message(&payload).expect("decode tx-sub");
        }
    }
}

async fn connect_retry(addr: &str) -> TcpStream {
    for _ in 0..50 {
        if let Ok(s) = TcpStream::connect(addr).await {
            return s;
        }
        sleep(Duration::from_millis(100)).await;
    }
    panic!("could not connect to harness at {addr}");
}

#[test]
fn harness_completes_exchange_and_writes_roundtrippable_corpus() {
    let port = free_port();
    let listen = format!("127.0.0.1:{port}");
    let tmp =
        std::env::temp_dir().join(format!("ade_txsub2_srv_loop_{}_{}", std::process::id(), port));
    let _ = std::fs::remove_dir_all(&tmp);

    let bin = env!("CARGO_BIN_EXE_ade_tx_submission2_server_capture");
    let mut child = Command::new(bin)
        .args([
            "--listen",
            &listen,
            "--out",
            tmp.to_str().unwrap(),
            "--scenario",
            "looptest",
            "--magic",
            "preprod",
            "--accept-timeout",
            "20",
            "--run-timeout",
            "20",
            "--idle-timeout",
            "10",
        ])
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn harness binary");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let mut s = connect_retry(&listen).await;

        // Handshake: node proposes, harness accepts.
        let table = VersionTable(vec![
            (N2NVersion::new(13), n2n_params(1)),
            (N2NVersion::new(14), n2n_params(1)),
        ]);
        s.write_all(&wrap_initiator(
            encode_handshake_message(&HandshakeMessage::ProposeVersions(table)),
            HANDSHAKE,
        ))
        .await
        .unwrap();

        let (proto, payload) = read_frame(&mut s).await;
        assert_eq!(proto, HANDSHAKE, "expected handshake accept");
        match decode_handshake_message(&payload).unwrap() {
            HandshakeMessage::AcceptVersion(v, _) => assert_eq!(v.get(), 14, "accept highest"),
            other => panic!("expected AcceptVersion, got {other:?}"),
        }

        // The node (client/provider) opens tx-submission with MsgInit.
        s.write_all(&wrap_initiator(
            encode_tx_submission_message(&TxSubmission2Message::Init),
            TX_SUBMISSION2,
        ))
        .await
        .unwrap();

        // Harness (server/consumer) requests tx ids (blocking).
        match read_tx_sub(&mut s).await {
            TxSubmission2Message::RequestTxIds { blocking, ack, .. } => {
                assert!(blocking, "first request should be blocking");
                assert_eq!(ack, 0, "first request acknowledges nothing");
            }
            other => panic!("expected RequestTxIds, got {other:?}"),
        }

        // Provider offers two era-tagged tx ids (Conway = era 6).
        let id0 = TxSubmissionTxId { era: 6, id: TxId(Hash32([0xAB; 32])) };
        let id1 = TxSubmissionTxId { era: 6, id: TxId(Hash32([0xCD; 32])) };
        s.write_all(&wrap_initiator(
            encode_tx_submission_message(&TxSubmission2Message::ReplyTxIds(vec![
                TxIdAndSize { tx_id: id0.clone(), size: 211 },
                TxIdAndSize { tx_id: id1, size: 198 },
            ])),
            TX_SUBMISSION2,
        ))
        .await
        .unwrap();

        // Harness requests the first body.
        match read_tx_sub(&mut s).await {
            TxSubmission2Message::RequestTxs(ids) => {
                assert_eq!(ids, vec![id0], "harness should request the first advertised id");
            }
            other => panic!("expected RequestTxs, got {other:?}"),
        }

        // Provider delivers one (synthetic, era-wrapped) tx body.
        s.write_all(&wrap_initiator(
            encode_tx_submission_message(&TxSubmission2Message::ReplyTxs(vec![vec![
                0x82, 0x06, 0xd8, 0x18, 0x42, 0xAA, 0xBB,
            ]])),
            TX_SUBMISSION2,
        ))
        .await
        .unwrap();

        // Harness acknowledges the batch with another (blocking) RequestTxIds.
        match read_tx_sub(&mut s).await {
            TxSubmission2Message::RequestTxIds { ack, .. } => {
                assert_eq!(ack, 2, "harness should acknowledge the 2 offered ids");
            }
            other => panic!("expected RequestTxIds (ack), got {other:?}"),
        }
    });

    let status = child.wait().expect("wait harness");
    assert!(status.success(), "harness should exit 0 after capturing essentials");

    let names: Vec<String> = std::fs::read_dir(&tmp)
        .expect("read tmp corpus dir")
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    for needle in ["_txsub_init_", "_txsub_reply_txids_", "_txsub_reply_txs_"] {
        assert!(
            names.iter().any(|n| n.contains(needle) && n.ends_with("_recv.cbor")),
            "expected a {needle} capture; got {names:?}"
        );
    }

    for n in names.iter().filter(|n| n.contains("_txsub_") && n.ends_with("_recv.cbor")) {
        let bytes = std::fs::read(tmp.join(n)).unwrap();
        assert!(bytes.len() > HEADER_LEN, "{n}: shorter than mux header");
        let payload = &bytes[HEADER_LEN..];
        let msg = decode_tx_submission_message(payload)
            .unwrap_or_else(|e| panic!("{n}: decode failed {e:?}"));
        assert_eq!(
            encode_tx_submission_message(&msg),
            payload,
            "{n}: re-encode not byte-identical"
        );
    }

    let _ = std::fs::remove_dir_all(&tmp);
}
