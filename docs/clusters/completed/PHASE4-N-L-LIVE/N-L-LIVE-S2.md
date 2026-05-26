# Invariant Slice — PHASE4-N-L-LIVE S2

**Slice Name:** RED `ade_node::wire_only` + `--mode` CLI flag + main.rs wiring.
**Cluster:** PHASE4-N-L-LIVE
**Status:** In Progress
**CEs addressed:** CE-N-L-LIVE-1 (RO-LIVE-04 mechanical), CE-N-L-LIVE-3 (¬P-2 enforcement), CE-N-L-LIVE-5 (signal-shutdown).
**Dependencies:** S1.

## Intent

Replace the print-and-exit main.rs stub. Add `--mode` parsing
(default `wire_only`). In wire-only mode: spawn one task per
`--peer`, each dials TCP, completes handshake, issues one
chain-sync `FindIntersect(Origin)`, reads the peer tip, emits
the matching JSONL events, exits. Wire-only mode MUST NOT call
`bootstrap_initial_state`.

## Scope

- `crates/ade_node/src/cli.rs` (extended) — add `--mode
  wire_only|admission`, `--log PATH`, `--tip-read-timeout-secs N`.
- `crates/ade_node/src/wire_only.rs` (new RED) —
  `run_wire_only(cli, writer) -> ExitCode` async fn.
- `crates/ade_node/src/main.rs` (rewritten) — `#[tokio::main]`,
  route on `cli.mode`, install signal handler.
- `crates/ade_node/src/lib.rs` — `pub mod wire_only`.

## Design

```rust
// cli.rs additions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode { WireOnly, Admission }
impl Cli { pub mode: Mode, pub log_path: PathBuf, pub tip_read_timeout_secs: u32, ... }

// wire_only.rs
pub async fn run_wire_only<W: Write + Send + 'static>(
    cli: &Cli,
    mut writer: LiveLogWriter<W>,
    shutdown: tokio::sync::watch::Receiver<bool>,
) -> ExitCode {
    writer.emit(&LiveLogEvent::NodeStarted {
        mode: ModeTag::WireOnly,
        peer_count: cli.peer_addrs.len() as u32,
    }).expect("log");

    let mut per_peer = Vec::new();
    for peer in &cli.peer_addrs {
        let peer_str = peer.clone();
        let timeout = Duration::from_secs(cli.tip_read_timeout_secs as u64);
        let log_tx = writer_channel.clone();
        let shutdown = shutdown.clone();
        per_peer.push(tokio::spawn(async move {
            wire_only_peer_session(peer_str, timeout, log_tx, shutdown).await
        }));
    }

    let mut ok = 0u32;
    let mut failed = 0u32;
    for h in per_peer {
        match h.await.unwrap_or(PeerOutcome::Failed) {
            PeerOutcome::Succeeded => ok += 1,
            PeerOutcome::Failed => failed += 1,
        }
    }

    writer.emit(&LiveLogEvent::WireSmokeComplete {
        admission_enabled: false,
        peer_count_ok: ok,
        peer_count_failed: failed,
    }).expect("log");

    let reason = if shutdown.borrow().clone() {
        WireOnlyShutdownReason::SignalReceived
    } else if failed > 0 {
        WireOnlyShutdownReason::PeerDialFailure
    } else {
        WireOnlyShutdownReason::TipReadComplete
    };
    writer.emit(&LiveLogEvent::NodeShutdown { reason }).expect("log");

    if failed == 0 { ExitCode::SUCCESS }
    else { ExitCode::from(EXIT_LIVE_PASS_PEER_FAILURE as u8) }
}

async fn wire_only_peer_session(
    peer: String,
    timeout: Duration,
    log_tx: mpsc::Sender<LiveLogEvent>,
    mut shutdown: watch::Receiver<bool>,
) -> PeerOutcome {
    log_tx.send(LiveLogEvent::PeerDialStarted { peer: peer.clone() }).await.ok();

    // 1. TCP connect.
    let stream = match TcpStream::connect(&peer).await {
        Ok(s) => s,
        Err(e) => {
            log_tx.send(LiveLogEvent::PeerDialFailed {
                peer: peer.clone(),
                kind: PeerDialFailureKind::TcpConnectFailed,
                detail: format!("{e:?}"),
            }).await.ok();
            return PeerOutcome::Failed;
        }
    };

    // 2. Spawn duplex transport.
    let handle = ade_network::mux::transport::spawn_duplex(stream, DuplexCapacity::DEFAULT);

    // 3. Handshake initiator (sync inside spawn_blocking).
    let (inbound, outbound, neg) = handshake_via_spawn_blocking(handle, peer.clone()).await;

    let neg = match neg {
        Ok(n) => n,
        Err(kind) => {
            log_tx.send(LiveLogEvent::PeerDialFailed {
                peer, kind, detail: String::new(),
            }).await.ok();
            return PeerOutcome::Failed;
        }
    };
    log_tx.send(LiveLogEvent::HandshakeOk {
        peer: peer.clone(),
        negotiated_version: neg.version,
    }).await.ok();

    // 4. Send FindIntersect(Origin) on chain-sync.
    let cs_frame = build_chain_sync_find_intersect_origin_frame();
    if outbound.send(cs_frame).await.is_err() { ... }

    // 5. Read tip from IntersectFound/NotFound reply (with timeout).
    let tip = match tokio::time::timeout(timeout, read_chain_sync_tip(inbound)).await {
        Ok(Ok(t)) => t,
        Ok(Err(kind)) => { log_tx.send(..PeerDialFailed { kind: TipReadProtocolError, .. }); return Failed; }
        Err(_) => { log_tx.send(..PeerDialFailed { kind: TipReadTimeout, .. }); return Failed; }
    };
    log_tx.send(LiveLogEvent::PeerTipRead {
        peer,
        slot: tip.slot.0,
        hash_hex: hex(tip.hash),
        block_no: tip.block_no,
    }).await.ok();

    // 6. Send Done + close.
    let _ = outbound.send(build_chain_sync_done_frame()).await;

    PeerOutcome::Succeeded
}
```

main.rs:

```rust
#[tokio::main]
async fn main() -> ExitCode {
    let cli = match Cli::parse_from(std::env::args()) { Ok(c) => c, Err(e) => { print + exit 1 } };

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        let mut sigint = signal(SignalKind::interrupt()).expect("sigint");
        let mut sigterm = signal(SignalKind::terminate()).expect("sigterm");
        tokio::select! {
            _ = sigint.recv() => {}
            _ = sigterm.recv() => {}
        }
        let _ = shutdown_tx.send(true);
    });

    let writer = LiveLogWriter::new(File::create(&cli.log_path).expect("log file"));
    match cli.mode {
        Mode::WireOnly => run_wire_only(&cli, writer, shutdown_rx).await,
        Mode::Admission => {
            // ¬P-3 fail-closed: no ledger seed available in this cluster.
            let mut w = writer;
            w.emit(&LiveLogEvent::NodeStarted { mode: ModeTag::WireOnly, peer_count: 0 }).ok();
            w.emit(&LiveLogEvent::NodeShutdown {
                reason: WireOnlyShutdownReason::LedgerSeedUnavailable,
            }).ok();
            ExitCode::from(EXIT_GENERIC_STARTUP as u8)
        }
    }
}
```

## §12 Mechanical Acceptance Criteria

- [ ] `main_wire_only_exits_zero_after_tip_read` (loopback
  responder) — binary dials loopback, completes handshake,
  reads tip, exits 0. JSONL log contains the 5 expected
  events in order.
- [ ] `main_wire_only_emits_handshake_ok` — separate dynamic
  assertion.
- [ ] `main_wire_only_emits_peer_tip_read` — separate dynamic
  assertion.
- [ ] `main_wire_only_never_emits_agreement_verdict` —
  inspect every emitted line for the forbidden strings.
- [ ] `main_without_genesis_does_not_attempt_admission` —
  empty stores + wire_only mode → no GenesisRequiredButAbsent.
- [ ] `main_signal_shutdown_flushes_jsonl` — send a signal
  mid-tip-read; assert the final line is a complete
  `node_shutdown` event with reason `SignalReceived`.
- [ ] `peer_dial_failure_exits_nonzero_with_error_event` —
  dial a closed port → emit `peer_dial_failed`, exit code
  `EXIT_LIVE_PASS_PEER_FAILURE = 20`.
- [ ] `ci/ci_check_wire_only_no_bootstrap_in_wire_only_path.sh`
  — grep `wire_only.rs` + `main.rs` wire-only branch for
  `bootstrap_initial_state`; assert no call.

## §14 Hard Prohibitions

- No `bootstrap_initial_state` call from wire-only code.
- No call to `n2n_dialer::N2nDialer::dial` from wire-only —
  the dialer wraps a full MuxPump spawn which is admission
  scope. Wire-only uses `spawn_duplex` directly + a much
  thinner per-peer task that sends ChainSync Done and exits.
- No retry loops.
- No `mpsc::unbounded_channel`.

## §15 Non-Goals

- No admission. No agreement_verdict. No ledger seed.
- No long-poll chain-sync. One tip-read, exit.
- No mempool / peer-sharing / tx-submission.
