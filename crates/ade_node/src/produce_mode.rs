// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED `ade_node --mode produce` driver (PHASE4-N-Q S5).
//!
//! Composes the N-Q surfaces shipped in S2..S4:
//!
//! - `ade_runtime::producer::coordinator` (GREEN; no secret material).
//! - `ade_runtime::producer::producer_shell` (RED; key custody).
//! - `ade_runtime::producer::producer_log` (GREEN; closed
//!   `ProducerLogEvent` JSONL vocabulary).
//! - `ade_runtime::network::n2n_listener` (RED; tokio TCP listener).
//!
//! Produces a JSONL evidence log at `--evidence-log PATH` whose
//! every line is a serialized `ProducerLogEvent` (closed
//! vocabulary; no socket addresses; no key material). The full
//! per-peer block-fetch loop closure is the S6 operator-runbook
//! deliverable; S5 ships the composition + slot loop + listener
//! integration with a stub forge handler (returns ForgeNotLeader).
//!
//! Honest scope (S5): the slot-loop drives the coordinator
//! deterministically over slot ticks; forge integration is
//! stubbed for the smoke test (real forge requires Conway-era
//! ledger state + mempool + era schedule — operator-action
//! work tracked under CN-CONS-06's live half).

use std::collections::BTreeMap;
use std::io::Write;
use std::net::SocketAddr;
use std::path::Path;
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use ade_runtime::network::n2n_listener::{run_n2n_listener, N2nListenerConfig};
use ade_runtime::orchestrator::event::{OrchestratorEvent, PeerRole};
use ade_runtime::orchestrator::n2n_server_pump::PeerIdGenerator;
use ade_runtime::producer::coordinator::{
    coordinator_init, coordinator_step, ChainTip, CoordinatorConfig, CoordinatorEffect,
    CoordinatorError, CoordinatorEvent, GenesisAnchor, LedgerSnapshotRef,
};
use ade_runtime::producer::keys::{
    load_ade_kes_signing_key, load_cold_signing_key_skey, load_kes_signing_key_skey,
    load_vrf_signing_key_skey, KeyLoadError,
};
use ade_runtime::producer::producer_log::{
    ForgeFailureReason, PeerDisconnectReason, PeerId, ProducerLogEvent, ShutdownReason,
};
use ade_runtime::producer::producer_shell::ProducerShell;
use ade_types::shelley::block::OperationalCert;

use tokio::sync::{mpsc, watch};
use tokio::time::interval;

use crate::cli::ProduceCli;

// =========================================================================
// Run entry point
// =========================================================================

pub const EXIT_PRODUCE_FAILURE: i32 = crate::node::EXIT_GENERIC_STARTUP;

/// Drive ade_node in produce mode. Returns when:
/// - `shutdown_rx` flips to `true` (SIGINT/SIGTERM), OR
/// - `cli.max_slots` is reached (smoke-test mode).
pub async fn run_produce_mode(cli: ProduceCli, shutdown_rx: watch::Receiver<bool>) -> ExitCode {
    let listen_addr = match SocketAddr::from_str(&cli.listen_addr) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("ade_node produce: invalid --listen address");
            return ExitCode::from(EXIT_PRODUCE_FAILURE as u8);
        }
    };

    // 1. Load keys + opcert + genesis.
    let cold = match load_cold_signing_key_skey(&cli.cold_skey) {
        Ok(k) => k,
        Err(e) => return startup_fail("cold_skey", &e),
    };
    let vrf = match load_vrf_signing_key_skey(&cli.vrf_skey) {
        Ok(k) => k,
        Err(e) => return startup_fail("vrf_skey", &e),
    };
    let kes = match load_kes_skey_any_format(&cli.kes_skey) {
        Ok(k) => k,
        Err(e) => return startup_fail("kes_skey", &e),
    };
    let opcert = match parse_simple_opcert_json(&cli.opcert) {
        Ok(c) => c,
        Err(detail) => {
            eprintln!("ade_node produce: opcert: {}", detail);
            return ExitCode::from(EXIT_PRODUCE_FAILURE as u8);
        }
    };
    let genesis = match parse_simple_genesis_json(&cli.genesis_file) {
        Ok(g) => g,
        Err(detail) => {
            eprintln!("ade_node produce: genesis: {}", detail);
            return ExitCode::from(EXIT_PRODUCE_FAILURE as u8);
        }
    };

    // 2. Initialize producer shell.
    let mut shell = match ProducerShell::init(kes, vrf, cold, opcert) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ade_node produce: shell init: {:?}", e);
            return ExitCode::from(EXIT_PRODUCE_FAILURE as u8);
        }
    };

    // 3. Initialize coordinator.
    let coord_cfg = CoordinatorConfig {
        genesis_anchor: genesis,
        opcert_meta: shell.public_metadata(),
        initial_chain_tip: None,
        initial_ledger_snapshot_ref: LedgerSnapshotRef(0),
        broadcast_queue_limit: 32,
        peer_limit: 16,
    };
    let (mut coord_state, init_effects) = coordinator_init(coord_cfg);

    // 4. Open evidence-log file.
    let mut evidence_writer = match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&cli.evidence_log)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "ade_node produce: cannot open evidence log: {}",
                e.kind() as i32
            );
            return ExitCode::from(EXIT_PRODUCE_FAILURE as u8);
        }
    };

    // 5. Apply init effects (writes CoordinatorStarted to the log).
    if let Err(detail) = apply_effects(&init_effects, &mut evidence_writer) {
        eprintln!("ade_node produce: init effects: {}", detail);
        return ExitCode::from(EXIT_PRODUCE_FAILURE as u8);
    }

    // 6. Spawn listener.
    let (events_tx, mut events_rx) = mpsc::channel::<OrchestratorEvent>(64);
    let listener_cfg = N2nListenerConfig {
        bind_addr: listen_addr,
        our_supported: ade_network::handshake::version_table::N2N_SUPPORTED,
        peer_id_generator: Arc::new(PeerIdGenerator::new()),
        events_out: events_tx,
    };
    let listener_shutdown_rx = shutdown_rx.clone();
    let _listener_handle =
        tokio::spawn(async move { run_n2n_listener(listener_cfg, listener_shutdown_rx).await });

    // 7. Run the main slot loop.
    //
    // Slot ticker fires every `genesis.slot_length_ms` ms. For S5
    // smoke testing the operator sets `--max-slots N` so the loop
    // terminates after N ticks; production omits the cap and relies
    // on SIGINT.
    let slot_interval = Duration::from_millis(genesis.slot_length_ms.max(1));
    let mut ticker = interval(slot_interval);
    let mut current_slot: u64 = 0;
    let mut shutdown_rx_mut = shutdown_rx.clone();
    let mut connected_peers: BTreeMap<PeerId, ()> = BTreeMap::new();

    loop {
        tokio::select! {
            biased;
            _ = shutdown_rx_mut.changed() => {
                if *shutdown_rx_mut.borrow() {
                    let (_new_state, effects) = match coordinator_step(
                        coord_state.clone(),
                        CoordinatorEvent::Shutdown {
                            reason: ShutdownReason::SignalReceived,
                        },
                    ) {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!("ade_node produce: shutdown step: {:?}", e);
                            return ExitCode::from(EXIT_PRODUCE_FAILURE as u8);
                        }
                    };
                    let _ = apply_effects(&effects, &mut evidence_writer);
                    return ExitCode::SUCCESS;
                }
            }
            _ = ticker.tick() => {
                if let Some(max) = cli.max_slots {
                    if current_slot >= max {
                        let (_new_state, effects) = coordinator_step(
                            coord_state.clone(),
                            CoordinatorEvent::Shutdown {
                                reason: ShutdownReason::ScheduleEnded,
                            },
                        )
                        .unwrap_or((coord_state.clone(), Vec::new()));
                        let _ = apply_effects(&effects, &mut evidence_writer);
                        return ExitCode::SUCCESS;
                    }
                }

                let (new_state, effects) = match coordinator_step(
                    coord_state.clone(),
                    CoordinatorEvent::SlotTick { slot: current_slot },
                ) {
                    Ok(p) => p,
                    Err(CoordinatorError::SlotDrift { .. }) => {
                        current_slot += 1;
                        continue;
                    }
                    Err(e) => {
                        eprintln!("ade_node produce: slot step: {:?}", e);
                        return ExitCode::from(EXIT_PRODUCE_FAILURE as u8);
                    }
                };
                coord_state = new_state;
                if let Err(detail) = apply_effects_with_forge_handler(
                    &effects,
                    &mut evidence_writer,
                    &mut coord_state,
                    &mut shell,
                ) {
                    eprintln!("ade_node produce: slot effects: {}", detail);
                    return ExitCode::from(EXIT_PRODUCE_FAILURE as u8);
                }
                current_slot += 1;
            }
            evt = events_rx.recv() => {
                let evt = match evt {
                    Some(e) => e,
                    None => continue,
                };
                if let Err(detail) = handle_listener_event(
                    evt,
                    &mut coord_state,
                    &mut evidence_writer,
                    &mut connected_peers,
                ) {
                    eprintln!("ade_node produce: listener event: {}", detail);
                }
            }
        }
    }
}

// =========================================================================
// Helpers
// =========================================================================

fn startup_fail(label: &'static str, err: &KeyLoadError) -> ExitCode {
    eprintln!("ade_node produce: load {} failed: {:?}", label, err);
    ExitCode::from(EXIT_PRODUCE_FAILURE as u8)
}

/// Try the cardano-cli envelope loader first; if the envelope JSON
/// fails to parse (Ade-native format is binary CBOR) or the envelope
/// type doesn't match, fall back to the Ade-native loader.
fn load_kes_skey_any_format(
    path: &Path,
) -> Result<ade_runtime::producer::signing::KesSecret, KeyLoadError> {
    match load_kes_signing_key_skey(path) {
        Ok(k) => Ok(k),
        Err(KeyLoadError::UnexpectedType { .. })
        | Err(KeyLoadError::MalformedEnvelope { .. }) => load_ade_kes_signing_key(path),
        Err(e) => Err(e),
    }
}

/// Minimal opcert JSON format (S5 honest scope):
/// `{"hot_vkey_hex": "<64 hex>", "sequence_number": <u64>,
///   "kes_period": <u64>, "sigma_hex": "<128 hex>"}`.
///
/// S6's operator runbook will document conversion from cardano-cli
/// `node.opcert` (text-envelope cborHex) to this format. A full
/// cardano-cli envelope parser is deferred to S6.
fn parse_simple_opcert_json(path: &Path) -> Result<OperationalCert, &'static str> {
    let bytes = std::fs::read(path).map_err(|_| "cannot read opcert file")?;
    #[derive(serde::Deserialize)]
    struct OpCertJson {
        hot_vkey_hex: String,
        sequence_number: u64,
        kes_period: u64,
        sigma_hex: String,
    }
    let parsed: OpCertJson =
        serde_json::from_slice(&bytes).map_err(|_| "opcert JSON parse failure")?;
    let hot_vkey = hex_decode(&parsed.hot_vkey_hex).map_err(|_| "hot_vkey_hex decode")?;
    if hot_vkey.len() != 32 {
        return Err("hot_vkey must be 32 bytes");
    }
    let sigma = hex_decode(&parsed.sigma_hex).map_err(|_| "sigma_hex decode")?;
    if sigma.len() != 64 {
        return Err("sigma must be 64 bytes");
    }
    Ok(OperationalCert {
        hot_vkey,
        sequence_number: parsed.sequence_number,
        kes_period: parsed.kes_period,
        sigma,
    })
}

/// Minimal Conway genesis subset (S5 honest scope):
/// `{"network_magic": <u32>, "slot_zero_time_unix_ms": <u64>,
///   "slot_length_ms": <u64>, "slots_per_kes_period": <u64>,
///   "kes_anchor_slot": <u64>, "kes_max_period": <u32>}`.
///
/// Real cardano-cli `conway-genesis.json` parsing lands in S6.
fn parse_simple_genesis_json(path: &Path) -> Result<GenesisAnchor, &'static str> {
    let bytes = std::fs::read(path).map_err(|_| "cannot read genesis file")?;
    #[derive(serde::Deserialize)]
    struct GenesisJson {
        network_magic: u32,
        slot_zero_time_unix_ms: u64,
        slot_length_ms: u64,
        slots_per_kes_period: u64,
        kes_anchor_slot: u64,
        kes_max_period: u32,
    }
    let parsed: GenesisJson =
        serde_json::from_slice(&bytes).map_err(|_| "genesis JSON parse failure")?;
    Ok(GenesisAnchor {
        network_magic: parsed.network_magic,
        slot_zero_time_unix_ms: parsed.slot_zero_time_unix_ms,
        slot_length_ms: parsed.slot_length_ms,
        slots_per_kes_period: parsed.slots_per_kes_period,
        kes_anchor_slot: parsed.kes_anchor_slot,
        kes_max_period: parsed.kes_max_period,
    })
}

fn hex_decode(s: &str) -> Result<Vec<u8>, &'static str> {
    if s.len() % 2 != 0 {
        return Err("odd-length hex");
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_nibble(bytes[i])?;
        let lo = hex_nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn hex_nibble(c: u8) -> Result<u8, &'static str> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err("non-hex character"),
    }
}

/// Apply effects emitted by `coordinator_init` (no forge requests
/// expected at init time).
fn apply_effects(
    effects: &[CoordinatorEffect],
    evidence_writer: &mut std::fs::File,
) -> Result<(), &'static str> {
    for e in effects {
        match e {
            CoordinatorEffect::LogEvidence { event } => {
                write_evidence_event(evidence_writer, event)?;
            }
            CoordinatorEffect::RequestForge { .. } => {
                return Err("unexpected RequestForge effect at init");
            }
            CoordinatorEffect::BroadcastBlock { .. } => {
                return Err("unexpected BroadcastBlock effect at init");
            }
        }
    }
    Ok(())
}

/// Apply effects with the forge handler — feeds RequestForge back
/// into the coordinator as a stub ForgeNotLeader event (S5 honest
/// scope; real forge integration lands in S6 operator-runbook
/// work).
fn apply_effects_with_forge_handler(
    effects: &[CoordinatorEffect],
    evidence_writer: &mut std::fs::File,
    coord_state: &mut ade_runtime::producer::coordinator::CoordinatorState,
    _shell: &mut ProducerShell,
) -> Result<(), &'static str> {
    for e in effects {
        match e {
            CoordinatorEffect::LogEvidence { event } => {
                write_evidence_event(evidence_writer, event)?;
            }
            CoordinatorEffect::RequestForge {
                slot,
                kes_period: _,
                ledger_snapshot_ref: _,
                chain_tip: _,
            } => {
                // S5 stub: emit ForgeNotLeader. Real forge integration
                // (composes shell.vrf_prove + shell.kes_sign_at +
                // scheduler_step + self_accept) is the S6 operator-
                // runbook deliverable.
                let stub_event = CoordinatorEvent::ForgeNotLeader {
                    slot: *slot,
                    vrf_output_fingerprint: [0u8; 8],
                };
                let prev_state = coord_state.clone();
                let (new_state, more_effects) =
                    coordinator_step(prev_state, stub_event).map_err(|_| "forge stub step")?;
                *coord_state = new_state;
                for me in &more_effects {
                    if let CoordinatorEffect::LogEvidence { event } = me {
                        write_evidence_event(evidence_writer, event)?;
                    }
                }
            }
            CoordinatorEffect::BroadcastBlock { artifact: _ } => {
                // S5 stub: log a placeholder. The served-snapshot
                // wiring (push the artifact to ServedChainSnapshot
                // for n2n_server reducers to serve) lands in S6.
            }
        }
    }
    Ok(())
}

fn handle_listener_event(
    evt: OrchestratorEvent,
    coord_state: &mut ade_runtime::producer::coordinator::CoordinatorState,
    evidence_writer: &mut std::fs::File,
    connected_peers: &mut BTreeMap<PeerId, ()>,
) -> Result<(), &'static str> {
    match evt {
        OrchestratorEvent::PeerConnected {
            peer_id,
            chain_sync_version,
            block_fetch_version,
            role: PeerRole::DownstreamServer,
        } => {
            let coord_peer_id = PeerId(peer_id.0);
            connected_peers.insert(coord_peer_id, ());
            let prev_state = coord_state.clone();
            let (new_state, effects) = coordinator_step(
                prev_state,
                CoordinatorEvent::PeerConnected {
                    peer_id: coord_peer_id,
                    chain_sync_version: chain_sync_version.get() as u32,
                    block_fetch_version: block_fetch_version.get() as u32,
                },
            )
            .map_err(|_| "peer connected step")?;
            *coord_state = new_state;
            for e in &effects {
                if let CoordinatorEffect::LogEvidence { event } = e {
                    write_evidence_event(evidence_writer, event)?;
                }
            }
        }
        OrchestratorEvent::PeerDisconnected { peer_id, .. } => {
            let coord_peer_id = PeerId(peer_id.0);
            if connected_peers.remove(&coord_peer_id).is_some() {
                let prev_state = coord_state.clone();
                let (new_state, effects) = coordinator_step(
                    prev_state,
                    CoordinatorEvent::PeerDisconnected {
                        peer_id: coord_peer_id,
                        reason: PeerDisconnectReason::Graceful,
                    },
                )
                .map_err(|_| "peer disconnected step")?;
                *coord_state = new_state;
                for e in &effects {
                    if let CoordinatorEffect::LogEvidence { event } = e {
                        write_evidence_event(evidence_writer, event)?;
                    }
                }
            }
        }
        // S5 honest scope: per-peer mini-protocol frame events
        // (PeerN2nServerChainSyncFrame, PeerN2nServerBlockFetchFrame)
        // are not yet dispatched into n2n_server reducers; S6's
        // operator-runbook work composes the per-peer pump that
        // serves blocks via the ServedChainSnapshot.
        _ => {}
    }
    Ok(())
}

fn write_evidence_event(
    writer: &mut std::fs::File,
    event: &ProducerLogEvent,
) -> Result<(), &'static str> {
    let line = serde_json::to_string(event).map_err(|_| "event serialize failure")?;
    writer
        .write_all(line.as_bytes())
        .map_err(|_| "evidence write failure")?;
    writer
        .write_all(b"\n")
        .map_err(|_| "evidence write failure")?;
    writer.flush().map_err(|_| "evidence flush failure")?;
    Ok(())
}

// Suppress unused warning: ForgeFailureReason is part of the closed
// surface but the S5 stub forge handler emits ForgeNotLeader only.
// S6's real forge handler will exercise the failure path.
#[allow(dead_code)]
fn _force_use_forge_failure_reason() -> ForgeFailureReason {
    ForgeFailureReason::Other
}

#[allow(dead_code)]
fn _force_use_chain_tip(t: ChainTip) -> u64 {
    t.slot
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn write_test_files(dir: &Path) -> (ProduceCli, std::path::PathBuf) {
        // Genesis fixture
        let genesis_path = dir.join("genesis.json");
        std::fs::write(
            &genesis_path,
            br#"{
                "network_magic": 1,
                "slot_zero_time_unix_ms": 1000,
                "slot_length_ms": 10,
                "slots_per_kes_period": 129600,
                "kes_anchor_slot": 0,
                "kes_max_period": 63
            }"#,
        )
        .unwrap();

        // Synthetic keys via the loaders' shapes. For the smoke test
        // we generate via Ade-native key-gen (writes a real
        // `ade.kes.seed.v1` envelope).
        let kes_path = dir.join("kes.ade.skey");
        let kes_seed = [0x42u8; 32];
        ade_runtime::producer::keys::write_ade_kes_envelope(&kes_path, &kes_seed, 0).unwrap();

        let vrf_seed = [0x07u8; 32];
        let (vrf_sk_bytes, _) =
            cardano_crypto::vrf::VrfDraft03::keypair_from_seed(&vrf_seed);
        let vrf_path = dir.join("vrf.skey");
        write_cardano_cli_envelope(
            &vrf_path,
            "VrfSigningKey_PraosVRF",
            &vrf_sk_bytes,
        );

        let cold_seed = [0x33u8; 32];
        let cold_path = dir.join("cold.skey");
        write_cardano_cli_envelope(
            &cold_path,
            "StakePoolSigningKey_ed25519",
            &cold_seed,
        );

        // Synthetic opcert: hot_vkey = the KES vkey from the same
        // seed; sequence_number = 0; kes_period = 0; sigma = 64
        // zero bytes (signature verification is deferred to BLUE
        // forge pipeline — not exercised by S5 smoke).
        use ade_crypto::kes_sum::KesAlgorithm;
        let kes_sk_raw =
            ade_crypto::kes_sum::Sum6Kes::gen_key_kes_from_seed_bytes(&kes_seed).unwrap();
        let kes_vkey = ade_crypto::kes_sum::Sum6Kes::derive_verification_key(&kes_sk_raw);
        let opcert_path = dir.join("opcert.json");
        let opcert_json = format!(
            r#"{{"hot_vkey_hex": "{}", "sequence_number": 0, "kes_period": 0, "sigma_hex": "{}"}}"#,
            hex_encode(&kes_vkey),
            "0".repeat(128),
        );
        std::fs::write(&opcert_path, opcert_json).unwrap();

        let evidence_log = dir.join("evidence.jsonl");
        let cli = ProduceCli {
            listen_addr: "127.0.0.1:0".to_string(),
            cold_skey: cold_path,
            kes_skey: kes_path,
            vrf_skey: vrf_path,
            opcert: opcert_path,
            genesis_file: genesis_path,
            evidence_log: evidence_log.clone(),
            max_slots: Some(3),
        };
        (cli, evidence_log)
    }

    fn write_cardano_cli_envelope(path: &Path, ty: &str, payload: &[u8]) {
        let cbor_hex = format!("{:02x}{}", 0x58, format!("{:02x}", payload.len()));
        let cbor_hex = format!("{}{}", cbor_hex, hex_encode(payload));
        let json = format!(
            "{{\"type\":\"{}\",\"description\":\"S5 test fixture\",\"cborHex\":\"{}\"}}",
            ty, cbor_hex
        );
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(json.as_bytes()).unwrap();
    }

    fn hex_encode(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }

    /// Listener-binding can race in this test if `127.0.0.1:0`
    /// is interpreted before the OS assigns a port; the test
    /// uses `max_slots` to bound the run.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn produce_mode_starts_runs_three_slots_and_exits_via_max_slots() {
        let dir = tempfile::tempdir().unwrap();
        let (cli, evidence_log) = write_test_files(dir.path());

        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        let exit = run_produce_mode(cli, shutdown_rx).await;
        assert_eq!(exit, ExitCode::SUCCESS);

        // Verify evidence log structure.
        let content = std::fs::read_to_string(&evidence_log).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert!(!lines.is_empty(), "evidence log is empty");

        // First line must be CoordinatorStarted.
        assert!(
            lines[0].contains("\"kind\":\"CoordinatorStarted\""),
            "first line not CoordinatorStarted: {}",
            lines[0]
        );

        // Must have ≥ 3 SlotTick events.
        let slot_ticks = lines
            .iter()
            .filter(|l| l.contains("\"kind\":\"SlotTick\""))
            .count();
        assert_eq!(slot_ticks, 3, "expected 3 SlotTick events, got {}", slot_ticks);

        // Must have ≥ 3 LeaderCheckOutcome events (stub emits
        // is_leader: false for each slot).
        let leader_checks = lines
            .iter()
            .filter(|l| l.contains("\"kind\":\"LeaderCheckOutcome\""))
            .count();
        assert_eq!(
            leader_checks, 3,
            "expected 3 LeaderCheckOutcome events, got {}",
            leader_checks
        );

        // Final event must be CoordinatorShutdown with
        // ScheduleEnded reason.
        let last = lines.last().unwrap();
        assert!(
            last.contains("\"kind\":\"CoordinatorShutdown\""),
            "last line not CoordinatorShutdown: {}",
            last
        );
        assert!(
            last.contains("\"reason\":\"ScheduleEnded\""),
            "shutdown reason not ScheduleEnded: {}",
            last
        );

        // No socket addresses leaked into the log (DC-PROD-01 / N15).
        assert!(!content.contains("127.0.0.1"), "socket addr leaked");
        // No seed bytes (synthesized seeds are 0x42, 0x07, 0x33).
        assert!(!content.contains("42424242"), "seed leak: kes");
        assert!(!content.contains("07070707"), "seed leak: vrf");
        assert!(!content.contains("33333333"), "seed leak: cold");
    }
}
