// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration tests — PHASE4-N-L-LIVE S3 (RO-LIVE-04).
//!
//! Hermetic loopback tests: a small in-process responder runs
//! on a `TcpListener`, accepts the wire-only dialer's connection,
//! completes the N2N handshake, replies to one chain-sync
//! `FindIntersect(Origin)` with a synthetic `IntersectFound`
//! carrying a tip, accepts the dialer's `Done`, and closes.
//!
//! Assertions: the dialer emits the correct JSONL events in the
//! correct order, exits with the correct code, and NEVER emits
//! any of the four forbidden event-name literals.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::time::Duration;

use ade_network::codec::chain_sync::{
    decode_chain_sync_message, encode_chain_sync_message, ChainSyncMessage,
    Point as CsPoint, Tip as CsTip,
};
use ade_network::codec::version::N2NVersion;
use ade_network::handshake::version_table::N2N_SUPPORTED;
use ade_network::mux::frame::{
    decode_frame, encode_frame, MiniProtocolId, MuxError, MuxFrame, MuxHeader, MuxMode,
};
use ade_network::mux::transport::{
    spawn_duplex, DuplexCapacity, MuxTransportHandle,
};
use ade_network::session::{
    run_n2n_handshake_responder, Transport, TransportError as SessionTransportError,
};
use ade_node::{
    run_wire_only, Cli, LiveLogWriter, Mode, EXIT_LIVE_PASS_PEER_FAILURE,
    EXIT_GENERIC_STARTUP,
};
use ade_types::{Hash32, SlotNo};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch};

const RESPONDER_TIP_SLOT: u64 = 12345;
const RESPONDER_TIP_BLOCK_NO: u64 = 100;

fn responder_tip_hash() -> Hash32 {
    Hash32([0xAB; 32])
}

struct BlockingTransport {
    inbound: mpsc::Receiver<Vec<u8>>,
    outbound: mpsc::Sender<Vec<u8>>,
    inbound_buffer: Vec<u8>,
}

impl BlockingTransport {
    fn new(inbound: mpsc::Receiver<Vec<u8>>, outbound: mpsc::Sender<Vec<u8>>) -> Self {
        Self {
            inbound,
            outbound,
            inbound_buffer: Vec::new(),
        }
    }
    fn into_halves(self) -> (mpsc::Receiver<Vec<u8>>, mpsc::Sender<Vec<u8>>) {
        (self.inbound, self.outbound)
    }
}

impl Transport for BlockingTransport {
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), SessionTransportError> {
        while self.inbound_buffer.len() < buf.len() {
            match self.inbound.blocking_recv() {
                Some(c) => self.inbound_buffer.extend_from_slice(&c),
                None => return Err(SessionTransportError::Eof),
            }
        }
        let drained: Vec<u8> = self.inbound_buffer.drain(..buf.len()).collect();
        buf.copy_from_slice(&drained);
        Ok(())
    }
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), SessionTransportError> {
        self.outbound
            .blocking_send(bytes.to_vec())
            .map_err(|_| SessionTransportError::Io)
    }
}

/// In-process responder: accept one connection, run the N2N
/// handshake to completion, reply to one chain-sync
/// FindIntersect(Origin) with IntersectFound carrying the
/// canned tip, await Done, close cleanly.
async fn spawn_loopback_responder() -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let MuxTransportHandle {
            inbound,
            outbound,
            reader_handle,
            writer_handle,
        } = spawn_duplex(stream, DuplexCapacity::DEFAULT);
        // 1. Drive responder handshake (sync, inside spawn_blocking).
        let (inbound, outbound, _hs_result) = tokio::task::spawn_blocking(move || {
            let mut bt = BlockingTransport::new(inbound, outbound);
            let r = run_n2n_handshake_responder(&mut bt, N2N_SUPPORTED);
            let (i, o) = bt.into_halves();
            (i, o, r)
        })
        .await
        .expect("hs spawn_blocking");
        // 2. Read one chain-sync frame (must be FindIntersect).
        let mut inbound = inbound;
        let mut buffer: Vec<u8> = Vec::new();
        loop {
            if let Ok((frame, _)) = try_decode_one(&buffer) {
                if frame.header.mini_protocol_id.get() == 2 {
                    let msg = decode_chain_sync_message(&frame.payload).expect("cs decode");
                    match msg {
                        ChainSyncMessage::FindIntersect { .. } => {
                            // 3. Reply with IntersectFound carrying our tip.
                            let reply = encode_chain_sync_message(
                                &ChainSyncMessage::IntersectFound {
                                    point: CsPoint::Block {
                                        slot: SlotNo(RESPONDER_TIP_SLOT),
                                        hash: responder_tip_hash(),
                                    },
                                    tip: CsTip {
                                        point: CsPoint::Block {
                                            slot: SlotNo(RESPONDER_TIP_SLOT),
                                            hash: responder_tip_hash(),
                                        },
                                        block_no: RESPONDER_TIP_BLOCK_NO,
                                    },
                                },
                            );
                            let frame_bytes = encode_chain_sync_mux_frame(
                                reply,
                                MuxMode::Responder,
                            )
                            .expect("encode");
                            let _ = outbound.send(frame_bytes).await;
                        }
                        ChainSyncMessage::Done => {
                            break;
                        }
                        _ => {
                            break;
                        }
                    }
                    // Consume the bytes.
                    let consumed = consumed_bytes(&buffer);
                    buffer.drain(..consumed);
                    continue;
                }
                // Non-chain-sync frame; drop it.
                let consumed = consumed_bytes(&buffer);
                buffer.drain(..consumed);
                continue;
            }
            match inbound.recv().await {
                Some(chunk) => buffer.extend_from_slice(&chunk),
                None => break,
            }
        }
        reader_handle.abort();
        writer_handle.abort();
    });
    addr
}

fn try_decode_one(buf: &[u8]) -> Result<(MuxFrame, ()), MuxError> {
    let (f, _) = decode_frame(buf)?;
    Ok((f, ()))
}

fn consumed_bytes(buf: &[u8]) -> usize {
    let (f, _) = decode_frame(buf).expect("decode");
    8 + f.payload.len()
}

fn encode_chain_sync_mux_frame(
    payload: Vec<u8>,
    mode: MuxMode,
) -> Result<Vec<u8>, MuxError> {
    let frame = MuxFrame {
        header: MuxHeader {
            timestamp: 0,
            mode,
            mini_protocol_id: MiniProtocolId::new(2).expect("2"),
            length: payload.len() as u16,
        },
        payload,
    };
    encode_frame(&frame)
}

fn make_cli(peer_addr: std::net::SocketAddr, log_path: PathBuf, mode: Mode) -> Cli {
    Cli {
        genesis_path: PathBuf::from("/dev/null"),
        network: "mainnet".to_string(),
        chain_db_path: None,
        snapshot_store_path: None,
        listen_addr: None,
        peer_addrs: vec![peer_addr.to_string()],
        mode,
        log_path,
        tip_read_timeout_secs: 5,
        json_seed_path: None,
        seed_point_slot: None,
        seed_block_hash_hex: None,
        wal_dir: None,
        snapshot_dir: None,
        network_magic: None,
        genesis_hash_hex: None,
        consensus_inputs_path: None,
        out_file: None,
        period_idx: None,
        seed_file: None,
        cold_skey: None,
        kes_skey: None,
        vrf_skey: None,
        opcert: None,
        genesis_file: None,
        evidence_log: None,
        max_slots: None,
    }
}

fn read_jsonl_lines(path: &PathBuf) -> Vec<String> {
    let bytes = std::fs::read(path).expect("read log");
    let s = String::from_utf8(bytes).expect("utf8");
    s.lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn main_wire_only_exits_zero_after_tip_read() {
    let addr = spawn_loopback_responder().await;
    let tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    let log_path = tmp.path().to_path_buf();
    let cli = make_cli(addr, log_path.clone(), Mode::WireOnly);
    let writer = LiveLogWriter::new(std::fs::File::create(&log_path).expect("create"));
    let (_tx, rx) = watch::channel(false);
    let exit = run_wire_only(&cli, writer, rx).await;
    assert_eq!(format!("{exit:?}"), format!("{:?}", std::process::ExitCode::SUCCESS));
    let lines = read_jsonl_lines(&log_path);
    let kinds: Vec<&str> = lines
        .iter()
        .filter_map(|l| extract_event(l))
        .collect();
    assert_eq!(
        kinds,
        vec![
            "node_started",
            "peer_dial_started",
            "handshake_ok",
            "peer_tip_read",
            "wire_smoke_complete",
            "node_shutdown",
        ],
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn main_wire_only_emits_peer_tip_read_with_responder_tip() {
    let addr = spawn_loopback_responder().await;
    let tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    let log_path = tmp.path().to_path_buf();
    let cli = make_cli(addr, log_path.clone(), Mode::WireOnly);
    let writer = LiveLogWriter::new(std::fs::File::create(&log_path).expect("create"));
    let (_tx, rx) = watch::channel(false);
    let _ = run_wire_only(&cli, writer, rx).await;
    let lines = read_jsonl_lines(&log_path);
    let tip_line = lines
        .iter()
        .find(|l| l.contains("\"event\":\"peer_tip_read\""))
        .expect("peer_tip_read emitted");
    assert!(tip_line.contains(&format!("\"slot\":{}", RESPONDER_TIP_SLOT)));
    assert!(tip_line.contains(&format!("\"block_no\":{}", RESPONDER_TIP_BLOCK_NO)));
    assert!(tip_line.contains("\"hash_hex\":\"abababababababab"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn main_wire_only_never_emits_agreement_verdict() {
    let addr = spawn_loopback_responder().await;
    let tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    let log_path = tmp.path().to_path_buf();
    let cli = make_cli(addr, log_path.clone(), Mode::WireOnly);
    let writer = LiveLogWriter::new(std::fs::File::create(&log_path).expect("create"));
    let (_tx, rx) = watch::channel(false);
    let _ = run_wire_only(&cli, writer, rx).await;
    let lines = read_jsonl_lines(&log_path);
    for forbidden in &[
        "agreement_verdict",
        "admitted_block",
        "ledger_applied",
        "projection_updated",
    ] {
        for line in &lines {
            assert!(
                !line.contains(forbidden),
                "wire-only must not emit `{}`: line={}",
                forbidden,
                line
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn main_without_genesis_does_not_attempt_admission() {
    // Wire-only mode against an empty `chain_db_path` (None) must
    // not even try to bootstrap. We assert that no log line
    // mentions `genesis_required_but_absent` or any admission event.
    let addr = spawn_loopback_responder().await;
    let tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    let log_path = tmp.path().to_path_buf();
    let cli = make_cli(addr, log_path.clone(), Mode::WireOnly);
    let writer = LiveLogWriter::new(std::fs::File::create(&log_path).expect("create"));
    let (_tx, rx) = watch::channel(false);
    let exit = run_wire_only(&cli, writer, rx).await;
    assert_eq!(format!("{exit:?}"), format!("{:?}", std::process::ExitCode::SUCCESS));
    let lines = read_jsonl_lines(&log_path);
    for line in &lines {
        assert!(
            !line.contains("GenesisRequiredButAbsent"),
            "wire-only must not surface bootstrap errors: {}",
            line
        );
        assert!(
            !line.contains("ledger_seed_unavailable"),
            "wire-only must not claim admission shutdown reason: {}",
            line
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn peer_dial_failure_exits_nonzero_with_error_event() {
    // Bind a listener, then close it — connect to the now-dead port.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    drop(listener);
    // Briefly wait for the kernel to release the port from any
    // half-state, then dial it.
    tokio::time::sleep(Duration::from_millis(50)).await;
    let tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    let log_path = tmp.path().to_path_buf();
    let cli = make_cli(addr, log_path.clone(), Mode::WireOnly);
    let writer = LiveLogWriter::new(std::fs::File::create(&log_path).expect("create"));
    let (_tx, rx) = watch::channel(false);
    let exit = run_wire_only(&cli, writer, rx).await;
    assert_eq!(
        format!("{exit:?}"),
        format!(
            "{:?}",
            std::process::ExitCode::from(EXIT_LIVE_PASS_PEER_FAILURE as u8)
        )
    );
    let lines = read_jsonl_lines(&log_path);
    let dial_failed_line = lines
        .iter()
        .find(|l| l.contains("\"event\":\"peer_dial_failed\""))
        .expect("peer_dial_failed emitted");
    assert!(dial_failed_line.contains("\"kind\":\"tcp_connect_failed\""));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn admission_mode_fails_closed_with_ledger_seed_unavailable() {
    let tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    let log_path = tmp.path().to_path_buf();
    let writer = LiveLogWriter::new(std::fs::File::create(&log_path).expect("create"));
    let exit = ade_node::wire_only::run_admission_unavailable(writer).await;
    assert_eq!(
        format!("{exit:?}"),
        format!(
            "{:?}",
            std::process::ExitCode::from(EXIT_GENERIC_STARTUP as u8)
        )
    );
    let lines = read_jsonl_lines(&log_path);
    let kinds: Vec<&str> = lines
        .iter()
        .filter_map(|l| extract_event(l))
        .collect();
    assert_eq!(kinds, vec!["node_started", "node_shutdown"]);
    assert!(
        lines
            .iter()
            .any(|l| l.contains("\"reason\":\"ledger_seed_unavailable\"")),
        "admission must shut down with ledger_seed_unavailable",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn jsonl_events_are_valid_one_object_per_line() {
    let addr = spawn_loopback_responder().await;
    let tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    let log_path = tmp.path().to_path_buf();
    let cli = make_cli(addr, log_path.clone(), Mode::WireOnly);
    let writer = LiveLogWriter::new(std::fs::File::create(&log_path).expect("create"));
    let (_tx, rx) = watch::channel(false);
    let _ = run_wire_only(&cli, writer, rx).await;
    let lines = read_jsonl_lines(&log_path);
    assert!(!lines.is_empty());
    for line in &lines {
        assert!(line.starts_with('{'), "line must start with {{: {line}");
        assert!(line.ends_with('}'), "line must end with }}: {line}");
        assert!(
            line.contains("\"event\":\""),
            "line must carry event discriminator: {line}"
        );
        // Brace-balance check (lightweight valid-JSON probe).
        let mut depth = 0i32;
        for c in line.chars() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
        }
        assert_eq!(depth, 0, "unbalanced braces in line: {line}");
    }
}

fn extract_event(line: &str) -> Option<&str> {
    let key = "\"event\":\"";
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

// Helper used by the responder-tip test to silence the unused-
// import warning on N2NVersion.
#[allow(dead_code)]
fn _touch_n2n() {
    let _ = N2NVersion::new(14);
}
