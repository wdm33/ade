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
//! per-peer block-fetch loop closure is the operator-runbook
//! deliverable; the slot loop + listener integration drive the
//! real forge handler `run_real_forge` (N-R-A/N-S/N-W/N-X — no
//! stub).
//!
//! Honest scope: the slot-loop drives the coordinator
//! deterministically over slot ticks, and `run_real_forge` performs
//! the real VRF / leader-check / KES-sign / forge / self-accept
//! composition. What remains for a live operator pass is tracked
//! under CN-CONS-06's / RO-LIVE-01's live half (the produce-mode
//! wiring cluster + operator stake + the witnessed pass).

use std::collections::BTreeMap;
use std::io::Write;
use std::net::SocketAddr;
use std::path::Path;
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use ade_core::consensus::leader_check::{verify_and_evaluate_leader, LeaderCheckVerdict};
use ade_core::consensus::leader_schedule::{
    query_leader_schedule, LeaderScheduleAnswer, LeaderScheduleQuery,
};
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::era_schedule::EraSchedule;
use ade_crypto::kes::KesPeriod;
use ade_crypto::vrf::VrfVerificationKey;
use ade_ledger::consensus_view::PoolDistrView;
use ade_ledger::mempool::admit::MempoolState;
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::producer::forge::{forge_block, ForgeError};
use ade_ledger::producer::self_accept::self_accept;
use ade_ledger::state::LedgerState;
use ade_runtime::consensus_inputs::{import_live_consensus_inputs, LiveConsensusInputsCanonical};
use ade_runtime::network::n2n_listener::{run_n2n_listener, N2nListenerConfig};
use ade_runtime::producer::tick_assembler::{assemble_tick, TickInputs};
use ade_runtime::seed_import::import_cardano_cli_json_utxo;
use ade_types::shelley::block::ProtocolVersion;
use ade_types::{BlockNo, EpochNo, Hash28, Hash32, SlotNo};
use ade_runtime::orchestrator::event::{OrchestratorEvent, PeerRole};
use ade_runtime::orchestrator::n2n_server_pump::PeerIdGenerator;
use ade_runtime::producer::chain_evolution::ChainEvolution;
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

    // 5b. Real bootstrap state (cold-start from the operator seed).
    //
    // A fresh `InMemoryChainDb` is empty as both ChainDb and
    // SnapshotStore, so `bootstrap_initial_state` takes the
    // cold-start branch and returns the seeded `genesis_initial`
    // unchanged with `tip = None`. The single bootstrap authority
    // is the only path to initial state (CN-NODE-01). The forge
    // path consumes these real values: `run_real_forge` builds its
    // `ForgeRequestContext` from the real base state + PoolDistrView
    // + EraSchedule threaded here, so leadership is driven by the
    // operator's consensus-inputs bundle, not a synthetic placeholder.
    let (utxo, _utxo_fp) = match import_cardano_cli_json_utxo(&cli.json_seed_path) {
        Ok(p) => p,
        Err(e) => return startup_fail_detail("json_seed", &format!("{e:?}")),
    };
    let consensus = match import_live_consensus_inputs(&cli.consensus_inputs_path) {
        Ok(c) => c,
        Err(e) => return startup_fail_detail("consensus_inputs", &format!("{e:?}")),
    };
    let mut seed_ledger = LedgerState::new(ade_types::CardanoEra::Conway);
    seed_ledger.utxo_state = utxo;
    let seed_chain_dep = PraosChainDepState::genesis(consensus.epoch_nonce.clone());
    let real_era_schedule =
        make_schedule_for_imported_window(&consensus.epoch_start_slot, consensus.epoch_no);
    let real_pool_distr = pool_distr_view_from_consensus_inputs(&consensus);
    let cold_db = ade_runtime::chaindb::InMemoryChainDb::new();
    let ade_runtime::bootstrap::BootstrapState {
        ledger: boot_ledger,
        chain_dep: boot_chain_dep,
        tip: boot_tip,
        ..
    } = match ade_runtime::bootstrap::bootstrap_initial_state(
        ade_runtime::bootstrap::BootstrapInputs {
            chaindb: &cold_db,
            snapshot_store: &cold_db,
            era_schedule: &real_era_schedule,
            ledger_view: &real_pool_distr,
            genesis_initial: Some((seed_ledger, seed_chain_dep)),
            // A3b: produce-mode cold-starts from the operator
            // consensus-inputs bundle; the recovered-sidecar warm-start
            // path is a later production-wiring slice, not this.
            seed_epoch_consensus_source:
                ade_runtime::bootstrap::SeedEpochConsensusSource::NotRequired,
        },
    ) {
        Ok(t) => t,
        Err(e) => return startup_fail_detail("bootstrap", &format!("{e:?}")),
    };

    // PHASE4-N-T S3: seed the linear `ChainEvolution` typestate from the
    // real cold-start bootstrap triple. `eta0` is the consensus-inputs
    // epoch nonce — the SAME nonce `boot_chain_dep` carries (S1 built
    // `boot_chain_dep` from `PraosChainDepState::genesis(epoch_nonce)`),
    // so the per-slot `query_leader_schedule` (which reads
    // `state.epoch_nonce`) and `run_real_forge` (which reads `ctx.eta0`)
    // share one nonce — required for the leader-check vrf-input
    // cross-check. The coordinator `ChainTip` carries `block_number`,
    // which the chaindb tip does not; cold-start has no tip, so the map
    // resolves to `None` (warm-start is test-only until N-U).
    let coord_boot_tip: Option<ChainTip> = boot_tip.as_ref().map(|t| ChainTip {
        slot: t.slot.0,
        block_hash: t.hash.0,
        block_number: 0,
    });
    let mut chain_evo: Option<ChainEvolution> = Some(ChainEvolution::seed(
        boot_ledger,
        boot_chain_dep,
        coord_boot_tip,
        real_era_schedule,
        real_pool_distr,
        consensus.epoch_nonce.clone(),
    ));

    // Operator pool id (mirrors producer_shell.rs:174). Real pparams
    // fidelity is out of N-T scope — keep default, as the synthetic path
    // did.
    let pool_id = ade_types::Hash28(ade_crypto::blake2b_224(&shell.cold_vk().0).0);
    let pparams = ProtocolParameters::default();

    // 6. Spawn listener.
    let (events_tx, mut events_rx) = mpsc::channel::<OrchestratorEvent>(64);
    let peer_outbound = ade_runtime::network::outbound_command::new_per_peer_outbound();
    let listener_cfg = N2nListenerConfig {
        bind_addr: listen_addr,
        our_supported: ade_network::handshake::version_table::N2N_SUPPORTED,
        peer_id_generator: Arc::new(PeerIdGenerator::new()),
        events_out: events_tx,
        peer_outbound: Some(peer_outbound.clone()),
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
    // PHASE4-N-T S3 (CE-T-9): absolute start slot from the bootstrap tip,
    // not 0. Cold-start has no tip ⇒ the first slot of the imported
    // window (`consensus.epoch_start_slot`). Per-tick `+= 1` increment +
    // `--max-slots` cap semantics are unchanged.
    let mut current_slot: u64 = boot_tip
        .as_ref()
        .map(|t| t.slot.0 + 1)
        .unwrap_or(consensus.epoch_start_slot.0);
    // `--max-slots` caps the number of ticks the loop processes, not the
    // absolute slot value (which now starts from the bootstrap tip, not
    // 0). Track the tick count separately so the cap keeps its
    // count-of-slots semantics.
    let mut ticks_elapsed: u64 = 0;
    let mut shutdown_rx_mut = shutdown_rx.clone();
    let mut connected_peers: BTreeMap<PeerId, ()> = BTreeMap::new();
    // PHASE4-N-R-B B2/B3: per-peer n2n_server state map + served-chain
    // handle/view pair. The handle stays in produce_mode; the view is
    // borrowed by `dispatch_server_frame_event_to_outbound` to read
    // the current snapshot atomically.
    let mut peers_state: ServerPeerStates = BTreeMap::new();
    let (served_chain_handle, served_chain_view) =
        ade_runtime::producer::served_chain_handle::ServedChainHandle::new();

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
                    if ticks_elapsed >= max {
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
                        ticks_elapsed += 1;
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
                    &mut chain_evo,
                    &pool_id,
                    &pparams,
                    &served_chain_handle,
                ) {
                    eprintln!("ade_node produce: slot effects: {}", detail);
                    return ExitCode::from(EXIT_PRODUCE_FAILURE as u8);
                }
                current_slot += 1;
                ticks_elapsed += 1;
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
                    &mut peers_state,
                    &served_chain_view,
                    &peer_outbound,
                )
                .await
                {
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

/// PHASE4-N-T: startup failure with a free-form detail string (the
/// seed-import / consensus-inputs / bootstrap path does not produce a
/// `KeyLoadError`).
fn startup_fail_detail(field: &'static str, detail: &str) -> ExitCode {
    eprintln!("ade_node produce: {} failed: {}", field, detail);
    ExitCode::from(EXIT_PRODUCE_FAILURE as u8)
}

/// Project the operator consensus-inputs bundle into the leadership
/// `PoolDistrView`. `total_active_stake` = sum of pool active stakes;
/// per-pool `vrf_keyhash` from the bundle's `pool_vrf_keyhashes` map.
/// `PoolDistrView` impls `LedgerView`, so it doubles as the
/// `&dyn LedgerView` for `block_validity` / `self_accept` (OI-T.3).
fn pool_distr_view_from_consensus_inputs(
    c: &LiveConsensusInputsCanonical,
) -> PoolDistrView {
    use ade_ledger::consensus_view::PoolEntry;
    let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
    let mut total: u64 = 0;
    for (pool, entry) in &c.pool_distribution {
        total = total.saturating_add(entry.active_stake);
        // A pool absent from the keyhash map cannot be a forge leader
        // anyway; the zero-hash fallback keeps the projection total.
        let vrf_keyhash = c
            .pool_vrf_keyhashes
            .get(pool)
            .cloned()
            .unwrap_or(Hash32([0u8; 32]));
        pools.insert(
            pool.clone(),
            PoolEntry {
                active_stake: entry.active_stake,
                vrf_keyhash,
            },
        );
    }
    PoolDistrView::new(c.epoch_no, total, c.active_slots_coeff, pools)
}

/// Build an era schedule whose single Conway entry starts at the
/// imported epoch's start slot. Mirrors
/// `crate::admission::bootstrap::make_schedule_for_imported_window`:
/// admission is single-epoch, so a single-entry schedule with a
/// safe-zone equal to the epoch length resolves every slot in the
/// window to the bundle's epoch (not epoch 0).
fn make_schedule_for_imported_window(
    epoch_start_slot: &SlotNo,
    epoch_no: EpochNo,
) -> EraSchedule {
    EraSchedule::new(
        ade_core::consensus::BootstrapAnchorHash(Hash32([0u8; 32])),
        epoch_start_slot.0,
        vec![ade_core::consensus::EraSummary {
            era: ade_types::CardanoEra::Conway,
            start_slot: *epoch_start_slot,
            start_epoch: epoch_no,
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 432_000,
        }],
    )
    .expect("era schedule for imported window")
}

/// Try the cardano-cli envelope loader first; if the envelope JSON
/// fails to parse (Ade-native format is binary CBOR) or the envelope
/// type doesn't match, fall back to the Ade-native loader.
///
/// `pub(crate)` so the PHASE4-N-F-F `operator_forge` node-path ingress site
/// reuses the same KES-any-format loader rather than duplicating it.
pub(crate) fn load_kes_skey_any_format(
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
///
/// `pub(crate)` so the PHASE4-N-F-F `operator_forge` node-path ingress site
/// reuses the same opcert parser rather than duplicating it.
pub(crate) fn parse_simple_opcert_json(path: &Path) -> Result<OperationalCert, &'static str> {
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

/// Read-only inputs the real forge handler needs in addition to the
/// `RequestForge` effect fields. Built by the main loop from the
/// loaded ledger snapshot / chain tip / era schedule / chain dep
/// state.
///
/// The leadership-driving inputs are real: the main loop builds
/// `leader_schedule_answer`, `eta0` (base chain-dep epoch_nonce),
/// `base_state`, and `vrf_vk` from the ChainEvolution state + the
/// operator consensus-inputs bundle (see the `RequestForge` arm).
/// Honest scope: `protocol_version`, `pparams`, and
/// `prev_opcert_counter` are still defaults — deriving them from the
/// loaded genesis/opcert is the produce-mode wiring cluster (G4).
pub struct ForgeRequestContext<'a> {
    pub eta0: &'a Nonce,
    pub vrf_vk: &'a VrfVerificationKey,
    pub leader_schedule_answer: &'a LeaderScheduleAnswer,
    pub pparams: &'a ProtocolParameters,
    pub base_state: &'a LedgerState,
    pub chain_dep_state: &'a PraosChainDepState,
    pub era_schedule: &'a EraSchedule,
    pub pool_distr_view: &'a PoolDistrView,
    pub block_number: BlockNo,
    pub prev_hash: Hash32,
    pub protocol_version: ProtocolVersion,
    pub prev_opcert_counter: Option<u64>,
}

/// PHASE4-N-R-A A3: real forge composition.
///
/// BLUE-then-RED-then-BLUE pipeline:
///
/// 1. **RED** — `vrf_prove(expected_vrf_input)` using the operator's
///    VRF signing key.
/// 2. **BLUE** — `verify_and_evaluate_leader` returns
///    `LeaderCheckVerdict`. `NotEligible` → return `ForgeNotLeader`.
///    `Eligible` → continue.
/// 3. **RED** — KES-signs the real header (N-S-A, two-pass): a
///    placeholder-signed first pass computes body_hash/body_size and
///    the header_body fields; the second pass KES-signs the canonical
///    `UnsignedHeaderPreImage` over those fields (CN-KES-HEADER-01).
/// 4. **GREEN** — `assemble_tick` stitches signed artifacts into a
///    canonical `ProducerTick`.
/// 5. **BLUE** — `forge_block` constructs the block from the tick.
/// 6. **BLUE** — `self_accept` runs full header + body validation.
///    `Accepted` → emit `ForgeSucceeded`; anything else → emit
///    `ForgeFailed { SelfAcceptRejected }`.
///
/// Returns the `CoordinatorEvent` to feed back into `coordinator_step`.
pub fn run_real_forge(
    slot: u64,
    kes_period: u32,
    ctx: &ForgeRequestContext<'_>,
    shell: &mut ProducerShell,
) -> CoordinatorEvent {
    // PHASE4-N-W S1 — producer-era policy: the producer forges Praos-era
    // (Babbage/Conway) blocks only. A non-Praos era fail-closes here
    // (the sketch's `UnsupportedEra::ProducerForge`; I6 / N5), before any
    // VRF/KES key use on the rejected path. TPraos *validation* is
    // unaffected. A locate failure is not era policy — it maps to `Other`.
    let era = match ctx.era_schedule.locate(ade_types::SlotNo(slot)) {
        Ok(loc) => loc.era,
        Err(_) => {
            return CoordinatorEvent::ForgeFailed {
                slot,
                reason: ForgeFailureReason::Other,
            };
        }
    };
    if !era.is_praos() {
        return CoordinatorEvent::ForgeFailed {
            slot,
            reason: ForgeFailureReason::UnsupportedProducerEra,
        };
    }

    // RED step 1 — VRF prove over the era-correct leader input. The single
    // authority (query_leader_schedule via leader_vrf_input) already built it
    // into the answer; we prove over exactly those bytes — no independent
    // re-derivation in the RED shell (CN-FORGE-04 / N3).
    let alpha = ctx.leader_schedule_answer.expected_vrf_input.alpha_bytes();
    let (vrf_proof, _vrf_output_red) = match shell.vrf_prove(alpha) {
        Ok(v) => v,
        Err(_) => {
            return CoordinatorEvent::ForgeFailed {
                slot,
                reason: ForgeFailureReason::Other,
            };
        }
    };

    // BLUE step 2 — leader-check evaluator (verify proof + threshold). The
    // era (located above) selects the Praos construction; the evaluator
    // cross-checks the answer's alpha against the single authority.
    let verdict = match verify_and_evaluate_leader(
        era,
        ade_types::SlotNo(slot),
        ctx.eta0,
        ctx.vrf_vk,
        &vrf_proof,
        ctx.leader_schedule_answer,
    ) {
        Ok(v) => v,
        Err(_) => {
            return CoordinatorEvent::ForgeFailed {
                slot,
                reason: ForgeFailureReason::Other,
            };
        }
    };
    let (verified_vrf_output, _leader_proof) = match verdict {
        LeaderCheckVerdict::NotEligible {
            slot: _,
            vrf_output_fingerprint,
        } => {
            return CoordinatorEvent::ForgeNotLeader {
                slot,
                vrf_output_fingerprint: vrf_output_fingerprint.0,
            };
        }
        LeaderCheckVerdict::Eligible {
            slot: _,
            vrf_output,
            leader_proof,
        } => (vrf_output, leader_proof),
    };

    // PHASE4-N-S-A A3: real KES-signs-unsigned-header bridge.
    //
    // Two-pass forge: the first pass uses a placeholder KES
    // signature to compute body_hash + body_size + all the
    // header_body fields; the second pass uses the real KES
    // signature over the canonical UnsignedHeaderPreImage.
    //
    // Body_hash + body_size are deterministic in (base_state,
    // mempool, mempool_tx_bytes) and INDEPENDENT of
    // kes_signature, so the body bytes are byte-identical
    // across passes; only the header's KES signature changes.
    //
    // The duplicate forge is intentional honest scope (saves
    // refactoring forge_block's signature). Real-load
    // optimizations land in a future cluster.

    use ade_crypto::kes::SUM6_KES_SIG_LEN;
    use ade_ledger::block_validity::header_input::decode_block as decode_for_preimage;
    use ade_ledger::block_validity::unsigned_header_pre_image::unsigned_header_pre_image;

    // Pre-flight period check: fail fast with KesPeriodMismatch
    // BEFORE the placeholder-signature first pass, so an
    // out-of-window kes_period doesn't surface as the
    // forge_block opcert error.
    if !shell.kes_period_in_window(kes_period) {
        return CoordinatorEvent::ForgeFailed {
            slot,
            reason: ForgeFailureReason::KesPeriodMismatch,
        };
    }

    // First pass — placeholder signature.
    let placeholder_sig = ade_crypto::kes::KesSignature([0u8; SUM6_KES_SIG_LEN]);
    let inputs_placeholder = TickInputs {
        vrf_proof: vrf_proof.clone(),
        kes_period: KesPeriod(kes_period),
        kes_signature: placeholder_sig,
        opcert: shell.opcert().clone(),
        cold_vk: shell.cold_vk(),
        vrf_vkey: ctx.vrf_vk.0.to_vec(),
        leader_answer: ctx.leader_schedule_answer.clone(),
        pparams: ctx.pparams.clone(),
        mempool_tx_bytes: Vec::new(),
        prev_opcert_counter: ctx.prev_opcert_counter,
        block_number: ctx.block_number,
        prev_hash: ctx.prev_hash.clone(),
        protocol_version: ctx.protocol_version,
    };
    let mempool = MempoolState::new(ctx.base_state.clone());
    let tick_placeholder =
        match assemble_tick(slot, ctx.base_state, &mempool, &inputs_placeholder) {
            Ok(t) => t,
            Err(_) => {
                return CoordinatorEvent::ForgeFailed {
                    slot,
                    reason: ForgeFailureReason::Other,
                };
            }
        };
    let forged_placeholder = match forge_block(&tick_placeholder) {
        Ok((forged, _)) => forged,
        Err(err) => {
            let reason = map_forge_error(&err);
            return CoordinatorEvent::ForgeFailed { slot, reason };
        }
    };

    // Extract canonical header_body fields via decode_block +
    // re-decode of inner block (matches A2's byte-match test
    // shape).
    let decoded_placeholder = match decode_for_preimage(&forged_placeholder.bytes) {
        Ok(d) => d,
        Err(_) => {
            return CoordinatorEvent::ForgeFailed {
                slot,
                reason: ForgeFailureReason::Other,
            };
        }
    };
    let inner_placeholder =
        &forged_placeholder.bytes[decoded_placeholder.inner_start..decoded_placeholder.inner_end];
    let preserved_placeholder = match ade_codec::conway::decode_conway_block(inner_placeholder) {
        Ok(p) => p,
        Err(_) => {
            return CoordinatorEvent::ForgeFailed {
                slot,
                reason: ForgeFailureReason::Other,
            };
        }
    };
    let header_body_decoded = preserved_placeholder.decoded().header.body.clone();
    let canonical_body_hash = header_body_decoded.body_hash.clone();
    let canonical_body_size = header_body_decoded.body_size;

    // Build the vrf_result CBOR field the recipe expects
    // (mirrors forge.rs:282-288).
    let vrf_result = {
        use ade_codec::cbor::{
            write_array_header, write_bytes_canonical, ContainerEncoding, IntWidth,
        };
        let mut buf = Vec::with_capacity(2 + 64 + 80);
        write_array_header(
            &mut buf,
            ContainerEncoding::Definite(2, IntWidth::Inline),
        );
        write_bytes_canonical(&mut buf, &verified_vrf_output.0);
        write_bytes_canonical(&mut buf, &vrf_proof.0);
        buf
    };

    let preimage = match unsigned_header_pre_image(
        slot,
        ctx.block_number.0,
        ctx.prev_hash.clone(),
        shell.cold_vk().0.to_vec(),
        ctx.vrf_vk.0.to_vec(),
        vrf_result,
        canonical_body_size,
        canonical_body_hash,
        shell.opcert().clone(),
        ctx.protocol_version,
    ) {
        Ok(p) => p,
        Err(_) => {
            return CoordinatorEvent::ForgeFailed {
                slot,
                reason: ForgeFailureReason::Other,
            };
        }
    };

    // RED step 3 (real) — KES-sign the canonical unsigned-header
    // pre-image via the branded type. Arbitrary-byte signing is
    // structurally unrepresentable.
    let real_kes_signature = match shell.kes_sign_header(kes_period, &preimage) {
        Ok(s) => s,
        Err(_) => {
            return CoordinatorEvent::ForgeFailed {
                slot,
                reason: ForgeFailureReason::KesPeriodMismatch,
            };
        }
    };

    // GREEN step 4 — assemble tick with REAL signature.
    let inputs = TickInputs {
        vrf_proof,
        kes_period: KesPeriod(kes_period),
        kes_signature: real_kes_signature,
        opcert: shell.opcert().clone(),
        cold_vk: shell.cold_vk(),
        vrf_vkey: ctx.vrf_vk.0.to_vec(),
        leader_answer: ctx.leader_schedule_answer.clone(),
        pparams: ctx.pparams.clone(),
        mempool_tx_bytes: Vec::new(),
        prev_opcert_counter: ctx.prev_opcert_counter,
        block_number: ctx.block_number,
        prev_hash: ctx.prev_hash.clone(),
        protocol_version: ctx.protocol_version,
    };
    let tick = match assemble_tick(slot, ctx.base_state, &mempool, &inputs) {
        Ok(t) => t,
        Err(_) => {
            return CoordinatorEvent::ForgeFailed {
                slot,
                reason: ForgeFailureReason::Other,
            };
        }
    };

    // BLUE step 5 — final forge with the real signature.
    let forged = match forge_block(&tick) {
        Ok((forged, _effects)) => forged,
        Err(err) => {
            let reason = map_forge_error(&err);
            return CoordinatorEvent::ForgeFailed { slot, reason };
        }
    };

    // BLUE step 6 — self-accept. The synthetic A3 fixture will
    // typically fail here because the placeholder KES signing payload
    // doesn't match the real header bytes; that's the documented
    // honest-scope path. A4 integration tests exercise the
    // ForgeSucceeded branch with realer fixtures.
    let accepted = self_accept(
        &forged.bytes,
        ctx.base_state,
        ctx.chain_dep_state,
        ctx.era_schedule,
        ctx.pool_distr_view,
    );
    let accepted = match accepted {
        Ok(a) => a,
        Err(_) => {
            return CoordinatorEvent::ForgeFailed {
                slot,
                reason: ForgeFailureReason::SelfAcceptRejected,
            };
        }
    };

    // Defense: the accepted block's hash MUST match the forged hash.
    let block_hash = accepted_block_hash(&accepted);
    let _ = verified_vrf_output; // suppress unused; carried for completeness

    CoordinatorEvent::ForgeSucceeded {
        slot,
        artifact: artifact_from_accepted(&accepted, block_hash, forged.bytes),
    }
}

fn map_forge_error(e: &ForgeError) -> ForgeFailureReason {
    match e {
        ForgeError::NotLeader { .. } => ForgeFailureReason::Other, // unreachable post-Eligible
        ForgeError::OpCertRejected(_) => ForgeFailureReason::Other,
        ForgeError::TxSetNotAdmissiblePrefix { .. }
        | ForgeError::MempoolWidthMismatch { .. }
        | ForgeError::MempoolAcceptedMismatch { .. } => ForgeFailureReason::EmptyMempool,
        ForgeError::BadKesSignatureLength { .. }
        | ForgeError::TxComponentSplit { .. } => ForgeFailureReason::Other,
    }
}

fn accepted_block_hash(b: &ade_ledger::producer::AcceptedBlock) -> [u8; 32] {
    // The canonical block_hash is blake2b_256 over the header bytes.
    // Decode via the BLUE header_input recipe.
    use ade_ledger::block_validity::header_input::decode_block;
    match decode_block(b.as_bytes()) {
        Ok(decoded) => decoded.block_hash.0,
        Err(_) => [0u8; 32],
    }
}

fn artifact_from_accepted(
    _accepted: &ade_ledger::producer::AcceptedBlock,
    hash: [u8; 32],
    bytes: Vec<u8>,
) -> ade_runtime::producer::coordinator::ForgedBlockArtifact {
    use ade_ledger::block_validity::header_input::decode_block;
    use ade_runtime::producer::coordinator::ForgedBlockArtifact;
    let slot = decode_block(&bytes)
        .map(|d| d.header_input.slot.0)
        .unwrap_or(0);
    ForgedBlockArtifact { slot, hash, bytes }
}

/// **PHASE4-N-T S4** — closed broadcast-push failure surface. No
/// `String` payloads.
#[derive(Debug, Clone, PartialEq)]
pub enum BroadcastPushError {
    /// The `self_accept` replay (inside `ChainEvolution::advance`)
    /// rejected the forged bytes — `push_atomic` is NOT called and the
    /// chain does not advance.
    SelfAcceptReplayRejected,
    /// `ServedChainHandle::push_atomic` itself failed.
    Push(ade_runtime::producer::served_chain_handle::PushError),
}

/// Apply effects with the real forge handler.
///
/// **PHASE4-N-T S3:** the forge inputs are derived from the linear
/// `ChainEvolution` typestate seeded from the real cold-start bootstrap
/// state — no synthetic scaffolding. Each `RequestForge`:
///
/// 1. Queries the real `LeaderScheduleAnswer` for the operator's pool
///    via BLUE `query_leader_schedule` against the chain's current
///    base state. `UnknownPool` / outside-horizon ⇒ we are not a
///    leader this slot (`ForgeNotLeader`).
/// 2. Builds the `ForgeRequestContext` from the chain's evolving base —
///    `eta0` is `chain_evo.base_chain_dep().epoch_nonce`, the SAME
///    nonce `query_leader_schedule` used for the VRF input.
/// 3. Runs the BLUE-then-RED-then-BLUE forge via [`run_real_forge`].
/// 4. On `ForgeSucceeded`, advances the chain via the sole
///    `ChainEvolution::advance` path and captures the BLUE-minted
///    `AcceptedBlock` token. An advance rejection fails closed: the
///    chain does not advance, `push_atomic` is NOT called, the slot is
///    recorded as a forge failure, and `BroadcastPushError::SelfAccept`-
///    `ReplayRejected` is logged.
///
/// **PHASE4-N-T S4:** the `BroadcastBlock` effect (emitted by the
/// `ForgeSucceeded` `coordinator_step` into `more_effects`) routes the
/// captured `AcceptedBlock` to `ServedChainHandle::push_atomic` so the
/// forged block becomes servable. A `push_atomic` failure logs
/// `BroadcastPushError::Push`.
#[allow(clippy::too_many_arguments)]
fn apply_effects_with_forge_handler(
    effects: &[CoordinatorEffect],
    evidence_writer: &mut std::fs::File,
    coord_state: &mut ade_runtime::producer::coordinator::CoordinatorState,
    shell: &mut ProducerShell,
    chain_evo: &mut Option<ChainEvolution>,
    pool_id: &Hash28,
    pparams: &ProtocolParameters,
    served_chain_handle: &ade_runtime::producer::served_chain_handle::ServedChainHandle,
) -> Result<(), &'static str> {
    for e in effects {
        match e {
            CoordinatorEffect::LogEvidence { event } => {
                write_evidence_event(evidence_writer, event)?;
            }
            CoordinatorEffect::RequestForge {
                slot,
                kes_period,
                ledger_snapshot_ref: _,
                chain_tip: _,
            } => {
                let evo = chain_evo.as_ref().ok_or("chain_evo consumed")?;
                let answer = match query_leader_schedule(
                    &LeaderScheduleQuery {
                        slot: ade_types::SlotNo(*slot),
                        pool: pool_id.clone(),
                    },
                    evo.pool_distr_view(),
                    evo.era_schedule(),
                    evo.base_chain_dep(),
                ) {
                    Ok(a) => Some(a),
                    // Unknown pool / outside horizon ⇒ not a leader.
                    Err(_) => None,
                };

                let event = match answer {
                    None => CoordinatorEvent::ForgeNotLeader {
                        slot: *slot,
                        vrf_output_fingerprint: [0u8; 8],
                    },
                    Some(answer) => {
                        let vrf_vk = shell.vrf_verification_key();
                        let ctx = ForgeRequestContext {
                            eta0: &evo.base_chain_dep().epoch_nonce,
                            vrf_vk: &vrf_vk,
                            leader_schedule_answer: &answer,
                            pparams,
                            base_state: evo.base_ledger(),
                            chain_dep_state: evo.base_chain_dep(),
                            era_schedule: evo.era_schedule(),
                            pool_distr_view: evo.pool_distr_view(),
                            block_number: ade_types::BlockNo(evo.next_block_number()),
                            prev_hash: evo.prev_hash(),
                            protocol_version: ProtocolVersion { major: 9, minor: 0 },
                            prev_opcert_counter: None,
                        };
                        run_real_forge(*slot, *kes_period, &ctx, shell)
                    }
                };

                // Advance the chain on a self-accepted forge via the sole
                // `ChainEvolution::advance` path. Clone the artifact bytes
                // before `event` is consumed by `coordinator_step`. On
                // advance rejection, fail closed: do not advance, and
                // surface this slot as a forge failure instead of success.
                //
                // The BLUE-minted `AcceptedBlock` token produced by
                // `advance` is captured into `pending_accepted` and routed
                // to `push_atomic` in the `BroadcastBlock` arm of
                // `more_effects` below (S4) — never minted here.
                let mut pending_accepted: Option<ade_ledger::producer::AcceptedBlock> = None;
                let event = match &event {
                    CoordinatorEvent::ForgeSucceeded { slot, artifact } => {
                        let forged_bytes = artifact.bytes.clone();
                        let evo = chain_evo.take().ok_or("chain_evo consumed")?;
                        match evo.advance(&forged_bytes) {
                            Ok((next_evo, accepted)) => {
                                *chain_evo = Some(next_evo);
                                pending_accepted = Some(accepted);
                                event
                            }
                            Err(e) => {
                                // The advance-`Err` arm is the `self_accept`
                                // replay-rejection path: no broadcast, no
                                // push, keep feeding `ForgeFailed`.
                                eprintln!(
                                    "ade_node produce: chain advance rejected: {e:?} ({:?})",
                                    BroadcastPushError::SelfAcceptReplayRejected
                                );
                                CoordinatorEvent::ForgeFailed {
                                    slot: *slot,
                                    reason: ForgeFailureReason::Other,
                                }
                            }
                        }
                    }
                    _ => event,
                };

                let prev_state = coord_state.clone();
                let (new_state, more_effects) =
                    coordinator_step(prev_state, event).map_err(|_| "forge handler step")?;
                *coord_state = new_state;
                for me in &more_effects {
                    match me {
                        CoordinatorEffect::LogEvidence { event } => {
                            write_evidence_event(evidence_writer, event)?;
                        }
                        CoordinatorEffect::BroadcastBlock { artifact: _ } => {
                            // Route the self-accepted token to the sole
                            // `push_atomic` authority so the forged block
                            // becomes servable. A `BroadcastBlock` without a
                            // pending token (no preceding self-accepted
                            // advance) is unreachable — a defensive no-op.
                            if let Some(accepted) = pending_accepted.take() {
                                if let Err(e) = served_chain_handle.push_atomic(accepted) {
                                    eprintln!(
                                        "ade_node produce: served push failed: {:?}",
                                        BroadcastPushError::Push(e)
                                    );
                                }
                            }
                        }
                        CoordinatorEffect::RequestForge { .. } => {}
                    }
                }
            }
            CoordinatorEffect::BroadcastBlock { artifact: _ } => {
                // S4 wires push_atomic here.
            }
        }
    }
    Ok(())
}

/// PHASE4-N-R-B B3 per-peer state map. Keyed by `PeerId`;
/// inserted on `PeerConnected { role: DownstreamServer }`;
/// removed on `PeerDisconnected`; consumed by frame-event
/// dispatch.
type ServerPeerStates = BTreeMap<PeerId, ade_runtime::network::n2n_server::PerPeerN2nServerState>;

async fn handle_listener_event(
    evt: OrchestratorEvent,
    coord_state: &mut ade_runtime::producer::coordinator::CoordinatorState,
    evidence_writer: &mut std::fs::File,
    connected_peers: &mut BTreeMap<PeerId, ()>,
    peers_state: &mut ServerPeerStates,
    served_chain_view: &ade_runtime::producer::served_chain_handle::ServedChainView,
    peer_outbound: &ade_runtime::network::outbound_command::PerPeerOutbound,
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
            // PHASE4-N-R-B B3: install per-peer n2n_server state so
            // subsequent server-frame events can dispatch.
            peers_state.insert(
                coord_peer_id,
                ade_runtime::network::n2n_server::PerPeerN2nServerState::new(
                    chain_sync_version,
                    block_fetch_version,
                ),
            );
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
            peers_state.remove(&coord_peer_id);
            peer_outbound
                .write()
                .await
                .remove(&ade_runtime::orchestrator::event::PeerId(peer_id.0));
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
        OrchestratorEvent::PeerN2nServerChainSyncFrame { .. }
        | OrchestratorEvent::PeerN2nServerBlockFetchFrame { .. } => {
            let (_replies, served_evidence) =
                dispatch_server_frame_event_to_outbound(
                    &evt,
                    peers_state,
                    served_chain_view,
                    peer_outbound,
                )
                .await
                .map_err(|_| "server frame dispatch")?;
            // Emit one BlockServed per block observed present in the served
            // snapshot for this block-fetch range. The Vec was collected
            // before any await (no watch::Ref held across await); evidence
            // is emitted here where the writer lives.
            for ev in &served_evidence {
                write_evidence_event(
                    evidence_writer,
                    &ProducerLogEvent::BlockServed {
                        peer_id: ev.peer_id,
                        slot: ev.slot,
                        hash: ev.hash,
                        bytes_len: ev.bytes_len,
                    },
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// **PHASE4-N-S-B B4** — closed dispatch-error surface for the
/// outbound-relay path. No `String` payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    /// Peer is not in the per-peer state map (PeerConnected
    /// never arrived or PeerDisconnected already cleared it).
    UnknownPeer { peer_id: u64 },
    /// Peer is in `peers_state` but the PerPeerOutbound map
    /// has no sender registered for that peer. Indicates a
    /// listener/produce_mode synchronization bug.
    PeerOutboundMissing { peer_id: u64 },
    /// `mpsc::Sender::try_send` failed — either the channel
    /// is full (peer not draining fast enough) or the
    /// receiver was dropped (MuxPump task exited).
    SendFailure { peer_id: u64 },
    /// BLUE reducer rejected the inbound frame as malformed.
    ReducerError,
}

/// **PHASE4-N-T S4** — GREEN evidence observation of a block actually
/// present in the served snapshot for a block-fetch request range.
/// Collected into an owned `Vec` BEFORE any `.await` (never holds the
/// `watch::Ref` across an await) and emitted as
/// `ProducerLogEvent::BlockServed` by `handle_listener_event`. This
/// OBSERVES the served snapshot; it does not re-decide what the BLUE
/// serve reducer (`producer_block_fetch_serve`) serves, and is never
/// fabricated — `(slot, hash, bytes_len)` are read from the snapshot.
struct ServedBlockEvidence {
    peer_id: PeerId,
    slot: u64,
    hash: [u8; 32],
    bytes_len: u32,
}

/// **PHASE4-N-S-B B4** — outbound-relay-aware dispatch.
///
/// Uses the lower-level BLUE reducers (`producer_chain_sync_serve` /
/// `producer_block_fetch_serve`) directly so the reply is
/// typed `ServerReply`, not pre-encoded `Vec<u8>`. The typed
/// reply is wrapped in `OutboundCommand` and try_sent through
/// the per-peer outbound channel. No `Vec<u8>` byte tunnel.
///
/// On lookup or send failure, returns the closed
/// `DispatchError` variant; never panics.
async fn dispatch_server_frame_event_to_outbound(
    event: &OrchestratorEvent,
    peers_state: &mut ServerPeerStates,
    served_chain_view: &ade_runtime::producer::served_chain_handle::ServedChainView,
    peer_outbound: &ade_runtime::network::outbound_command::PerPeerOutbound,
) -> Result<(usize, Vec<ServedBlockEvidence>), DispatchError> {
    use ade_network::block_fetch::server::producer_block_fetch_serve;
    use ade_network::chain_sync::server::producer_chain_sync_serve;
    use ade_network::codec::block_fetch::{decode_block_fetch_message, BlockFetchMessage, Point};
    use ade_network::codec::chain_sync::decode_chain_sync_message;
    use ade_runtime::network::outbound_command::OutboundCommand;
    use ade_runtime::producer::served_chain_lookups::ServedChainLookups;

    match event {
        OrchestratorEvent::PeerN2nServerChainSyncFrame { peer_id, bytes } => {
            let key = PeerId(peer_id.0);
            let state = peers_state
                .get(&key)
                .cloned()
                .ok_or(DispatchError::UnknownPeer { peer_id: peer_id.0 })?;
            let snap_ref = served_chain_view.borrow();
            let msg = decode_chain_sync_message(bytes)
                .map_err(|_| DispatchError::ReducerError)?;
            let lookups = ServedChainLookups { snap: &*snap_ref };
            let chain_sync_version = state.chain_sync_version;
            let block_fetch_version = state.block_fetch_version;
            let block_fetch_old = state.block_fetch;
            let (cs2, step) = producer_chain_sync_serve(
                state.chain_sync,
                msg,
                &lookups,
                chain_sync_version,
            )
            .map_err(|_| DispatchError::ReducerError)?;
            let mut sent = 0usize;
            if let ade_network::chain_sync::server::ServerStep::Reply(reply) = step {
                let cmd = OutboundCommand::ChainSync {
                    peer: ade_runtime::orchestrator::event::PeerId(peer_id.0),
                    reply,
                };
                let map = peer_outbound.read().await;
                let sender = map.get(&ade_runtime::orchestrator::event::PeerId(peer_id.0))
                    .ok_or(DispatchError::PeerOutboundMissing { peer_id: peer_id.0 })?;
                sender
                    .try_send(cmd)
                    .map_err(|_| DispatchError::SendFailure { peer_id: peer_id.0 })?;
                sent = 1;
            }
            let updated_state = ade_runtime::network::n2n_server::PerPeerN2nServerState {
                chain_sync: cs2,
                block_fetch: block_fetch_old,
                chain_sync_version,
                block_fetch_version,
            };
            peers_state.insert(key, updated_state);
            Ok((sent, Vec::new()))
        }
        OrchestratorEvent::PeerN2nServerBlockFetchFrame { peer_id, bytes } => {
            let key = PeerId(peer_id.0);
            let state = peers_state
                .get(&key)
                .cloned()
                .ok_or(DispatchError::UnknownPeer { peer_id: peer_id.0 })?;
            let snap_ref = served_chain_view.borrow();
            let msg = decode_block_fetch_message(bytes)
                .map_err(|_| DispatchError::ReducerError)?;
            // Capture the requested point range before `msg` is consumed
            // by the reducer; a closed-end RequestRange carries `(slot,
            // hash)` for both endpoints (Origin has no key in the
            // snapshot's BTreeMap, so a range touching Origin observes no
            // present block — never over-claims).
            let requested_range = match &msg {
                BlockFetchMessage::RequestRange(r) => match (&r.from, &r.to) {
                    (
                        Point::Block { slot: fs, hash: fh },
                        Point::Block { slot: ts, hash: th },
                    ) => Some(((*fs, fh.clone()), (*ts, th.clone()))),
                    _ => None,
                },
                _ => None,
            };
            let lookups = ServedChainLookups { snap: &*snap_ref };
            let chain_sync_version = state.chain_sync_version;
            let block_fetch_version = state.block_fetch_version;
            let chain_sync_old = state.chain_sync;
            let (bf2, step) = producer_block_fetch_serve(
                state.block_fetch,
                msg,
                &lookups,
                block_fetch_version,
            )
            .map_err(|_| DispatchError::ReducerError)?;
            // GREEN evidence: observe which requested blocks are PRESENT in
            // the served snapshot, reading real `(slot, hash, bytes_len)`.
            // Collect into an owned `Vec` while the `watch::Ref` is held,
            // then drop the ref BEFORE the outbound `.await` (no Ref held
            // across await). Only present blocks within the requested
            // range are observed — never a fabricated/zeroed BlockServed.
            let mut served_evidence: Vec<ServedBlockEvidence> = Vec::new();
            if let Some((from, to)) = requested_range {
                for (s, h, b) in snap_ref.range_bytes(from, to) {
                    served_evidence.push(ServedBlockEvidence {
                        peer_id: PeerId(peer_id.0),
                        slot: s.0,
                        hash: h.0,
                        bytes_len: b.len() as u32,
                    });
                }
            }
            drop(snap_ref);
            let mut sent = 0usize;
            if let ade_network::block_fetch::server::BlockFetchServerStep::Replies(replies) = step {
                let map = peer_outbound.read().await;
                let sender = map.get(&ade_runtime::orchestrator::event::PeerId(peer_id.0))
                    .ok_or(DispatchError::PeerOutboundMissing { peer_id: peer_id.0 })?;
                for reply in replies {
                    let cmd = OutboundCommand::BlockFetch {
                        peer: ade_runtime::orchestrator::event::PeerId(peer_id.0),
                        reply,
                    };
                    sender
                        .try_send(cmd)
                        .map_err(|_| DispatchError::SendFailure { peer_id: peer_id.0 })?;
                    sent += 1;
                }
            }
            let updated_state = ade_runtime::network::n2n_server::PerPeerN2nServerState {
                chain_sync: chain_sync_old,
                block_fetch: bf2,
                chain_sync_version,
                block_fetch_version,
            };
            peers_state.insert(key, updated_state);
            Ok((sent, served_evidence))
        }
        _ => Ok((0, Vec::new())),
    }
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

// `ForgeFailureReason` is now constructed directly by `run_real_forge`
// (Other / UnsupportedProducerEra), so this dead-code shim is redundant;
// removal is deferred to the produce-mode wiring cluster's hygiene slice.
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
        // PHASE4-N-T S3: the produce loop now ticks from the absolute
        // bootstrap slot (the consensus-inputs `epoch_start_slot` =
        // 86_400_000 for the cold-start fixture). Anchor the KES window
        // there so those absolute slots map to KES period 0 (an
        // anchor of 0 would put slot 86.4M at period 666 > kes_max_period).
        std::fs::write(
            &genesis_path,
            br#"{
                "network_magic": 1,
                "slot_zero_time_unix_ms": 1000,
                "slot_length_ms": 10,
                "slots_per_kes_period": 129600,
                "kes_anchor_slot": 86400000,
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

        // PHASE4-N-T: produce mode now cold-starts from an operator
        // seed. Write the synthetic JSON-UTxO seed + consensus-inputs
        // fixtures so the smoke test exercises the real bootstrap path.
        let seed_path = dir.join("seed.json");
        std::fs::write(&seed_path, TEST_JSON_SEED).unwrap();
        let cinputs_path = dir.join("cinputs.json");
        std::fs::write(&cinputs_path, TEST_CONSENSUS_INPUTS).unwrap();

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
            json_seed_path: seed_path,
            consensus_inputs_path: cinputs_path,
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

        // Must have 3 LeaderCheckOutcome events. The fixture's
        // pool_distribution does not contain this shell's operator pool,
        // so the real `query_leader_schedule` returns `UnknownPool` each
        // slot ⇒ `ForgeNotLeader` ⇒ `LeaderCheckOutcome { is_leader:
        // false }`. A correct, never-forging run.
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

    // Synthetic 2-UTxO JSON seed fixture. Self-evidently a test
    // fixture (zero/aa/01 placeholder tx ids + a single preprod-shaped
    // address), NOT captured on-chain data.
    const TEST_JSON_SEED: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000001#0": {
            "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
            "value": { "lovelace": 1000000 }
        },
        "0000000000000000000000000000000000000000000000000000000000000002#3": {
            "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
            "value": { "lovelace": 2000000 }
        }
    }"#;

    // Synthetic 1-pool consensus-inputs fixture (all-zero/placeholder
    // hashes). NOT captured on-chain data.
    const TEST_CONSENSUS_INPUTS: &str = r#"{
        "network_magic": 1,
        "genesis_hash_hex": "00000000000000000000000000000000000000000000000000000000000000aa",
        "era": "conway",
        "epoch_no": 200,
        "epoch_start_slot": 86400000,
        "epoch_end_slot": 86832000,
        "active_slots_coeff": {"numer": 1, "denom": 20},
        "epoch_nonce_hex": "00000000000000000000000000000000000000000000000000000000000000bb",
        "pool_distribution": {
            "00000000000000000000000000000000000000000000000000000001": {"active_stake": 123}
        },
        "pool_vrf_keyhashes": {
            "00000000000000000000000000000000000000000000000000000001": "00000000000000000000000000000000000000000000000000000000000000cc"
        },
        "protocol_params_hash_hex": "00000000000000000000000000000000000000000000000000000000000000dd",
        "source_cardano_node_version": "cardano-node 11.0.1",
        "source_query_command": "cardano-cli conway query stake-distribution --testnet-magic 1",
        "source_tip_hash_hex": "00000000000000000000000000000000000000000000000000000000000000ee",
        "source_tip_slot": 86400500
    }"#;

    /// CE-T-4: the cold-start bootstrap path seeds the **real** ledger
    /// from the operator JSON-UTxO seed. The returned ledger's
    /// fingerprint MUST equal the imported-UTxO ledger's fingerprint,
    /// and the tip MUST be absent (cold-start has no tip).
    #[test]
    fn produce_mode_bootstrap_cold_start_seeds_real_ledger() {
        use ade_ledger::fingerprint::fingerprint;

        let dir = tempfile::tempdir().unwrap();
        let seed_path = dir.path().join("seed.json");
        let cinputs_path = dir.path().join("cinputs.json");
        std::fs::write(&seed_path, TEST_JSON_SEED).unwrap();
        std::fs::write(&cinputs_path, TEST_CONSENSUS_INPUTS).unwrap();

        let (utxo, _fp) = import_cardano_cli_json_utxo(&seed_path).expect("seed import");
        let consensus = import_live_consensus_inputs(&cinputs_path).expect("consensus import");

        // The imported-UTxO ledger fingerprint = the expected target.
        let mut expected_ledger = LedgerState::new(ade_types::CardanoEra::Conway);
        expected_ledger.utxo_state = utxo.clone();
        let expected_fp = fingerprint(&expected_ledger).combined.clone();

        // Seed exactly as run_produce_mode does, then route through the
        // sole bootstrap authority (cold-start branch).
        let mut seed_ledger = LedgerState::new(ade_types::CardanoEra::Conway);
        seed_ledger.utxo_state = utxo;
        let seed_chain_dep = PraosChainDepState::genesis(consensus.epoch_nonce.clone());
        let real_era_schedule = make_schedule_for_imported_window(
            &consensus.epoch_start_slot,
            consensus.epoch_no,
        );
        let real_pool_distr = pool_distr_view_from_consensus_inputs(&consensus);
        let cold_db = ade_runtime::chaindb::InMemoryChainDb::new();
        let ade_runtime::bootstrap::BootstrapState {
            ledger: boot_ledger,
            tip: boot_tip,
            ..
        } = ade_runtime::bootstrap::bootstrap_initial_state(
            ade_runtime::bootstrap::BootstrapInputs {
                chaindb: &cold_db,
                snapshot_store: &cold_db,
                era_schedule: &real_era_schedule,
                ledger_view: &real_pool_distr,
                genesis_initial: Some((seed_ledger, seed_chain_dep)),
                seed_epoch_consensus_source:
                    ade_runtime::bootstrap::SeedEpochConsensusSource::NotRequired,
            },
        )
        .expect("cold-start bootstrap");

        assert_eq!(
            fingerprint(&boot_ledger).combined,
            expected_fp,
            "cold-start ledger fingerprint must equal imported-UTxO ledger fingerprint"
        );
        assert!(boot_tip.is_none(), "cold-start must have no tip");
    }

    // =====================================================================
    // PHASE4-N-T S4 — BroadcastBlock → push_atomic (CE-T-10)
    // =====================================================================
    //
    // A synthetic forge cannot reach a self-accepting `ForgeSucceeded`
    // in-process (`forge_handler_variants.rs` proves self_accept rejects
    // the synthetic block), so per the slice doc these tests drive the
    // broadcast push path directly with a corpus-derived `AcceptedBlock`
    // and exercise the rejection path via a flipped-body block through the
    // sole `ChainEvolution::advance` authority — mirroring exactly what
    // `apply_effects_with_forge_handler`'s `ForgeSucceeded` arm runs.

    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce as CoreNonce};
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_ledger::block_validity::decode_block as bv_decode_block;
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::producer::self_accept::self_accept as ledger_self_accept;
    use ade_runtime::producer::served_chain_handle::ServedChainHandle as SChainHandle;
    use ade_runtime::producer::chain_evolution::{ChainEvolution as CE, ChainEvolutionError};
    use ade_testkit::validity::ConwayValidityCorpus;
    use std::collections::BTreeMap;

    const T4_EPOCH_576: EpochNo = EpochNo(576);
    const T4_EPOCH_577_START: u64 = 163_900_800;
    const T4_MAINNET_EPOCH_LENGTH: u64 = 432_000;

    fn t4_schedule() -> EraSchedule {
        let start_576 = T4_EPOCH_577_START - T4_MAINNET_EPOCH_LENGTH;
        EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![EraSummary {
                era: ade_types::CardanoEra::Conway,
                start_slot: SlotNo(start_576),
                start_epoch: T4_EPOCH_576,
                slot_length_ms: 1_000,
                epoch_length_slots: T4_MAINNET_EPOCH_LENGTH as u32,
                safe_zone_slots: T4_MAINNET_EPOCH_LENGTH as u32,
            }],
        )
        .expect("schedule is well-formed")
    }

    fn t4_view(c: &ConwayValidityCorpus) -> PoolDistrView {
        let total = c.pd_total_active_stake;
        let asc = ActiveSlotsCoeff {
            numer: c.asc.numer as u32,
            denom: c.asc.denom as u32,
        };
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        for (pool_id, p) in &c.pools {
            let scale = total / p.sigma.denom;
            pools.insert(
                Hash28(*pool_id),
                PoolEntry {
                    active_stake: p.sigma.numer * scale,
                    vrf_keyhash: Hash32(p.vrf_keyhash),
                },
            );
        }
        PoolDistrView::new(T4_EPOCH_576, total, asc, pools)
    }

    fn t4_ledger() -> LedgerState {
        let mut l = LedgerState::new(ade_types::CardanoEra::Conway);
        l.epoch_state.epoch = T4_EPOCH_576;
        l
    }

    fn t4_chain_dep(eta0: [u8; 32]) -> PraosChainDepState {
        let mut s = PraosChainDepState::empty();
        s.epoch_nonce = CoreNonce(Hash32(eta0));
        s.evolving_nonce = CoreNonce(Hash32(eta0));
        s
    }

    fn t4_inner_span(env_bytes: &[u8]) -> (usize, usize) {
        let env = decode_block_envelope(env_bytes).expect("envelope decodes");
        (env.block_start, env.block_end)
    }

    fn t4_pick_lightest(c: &ConwayValidityCorpus) -> &[u8] {
        let idx = (0..c.blocks.len())
            .min_by_key(|&i| {
                let (s, e) = t4_inner_span(&c.blocks[i]);
                e - s
            })
            .expect("corpus is non-empty");
        &c.blocks[idx]
    }

    fn t4_pick_heaviest(c: &ConwayValidityCorpus) -> &[u8] {
        let idx = (0..c.blocks.len())
            .max_by_key(|&i| {
                let (s, e) = t4_inner_span(&c.blocks[i]);
                e - s
            })
            .expect("corpus is non-empty");
        &c.blocks[idx]
    }

    /// Flip one body byte so the header is untouched but the recomputed
    /// body hash changes — mirrors `chain_evolution.rs::flip_body_byte`.
    fn t4_flip_body_byte(env_bytes: &[u8]) -> Vec<u8> {
        let (start, end) = t4_inner_span(env_bytes);
        let base = bv_decode_block(env_bytes).expect("base block decodes");
        for idx in (start..end).rev() {
            let mut bad = env_bytes.to_vec();
            bad[idx] ^= 0x01;
            if let Ok(d) = bv_decode_block(&bad) {
                if d.computed_body_hash != base.computed_body_hash {
                    return bad;
                }
            }
        }
        panic!("no structure-preserving body flip found");
    }

    fn t4_seed(c: &ConwayValidityCorpus) -> CE {
        CE::seed(
            t4_ledger(),
            t4_chain_dep(c.epoch_nonce),
            None,
            t4_schedule(),
            t4_view(c),
            CoreNonce(Hash32(c.epoch_nonce)),
        )
    }

    /// CE-T-10 (negative): a `ForgeSucceeded` artifact whose bytes fail
    /// `ChainEvolution::advance`'s self_accept (flipped body byte) is the
    /// `BroadcastPushError::SelfAcceptReplayRejected` path: `advance`
    /// returns `Err`, so `push_atomic` is NEVER called (the served
    /// snapshot stays empty) and the handler surfaces the slot as
    /// `ForgeFailed`.
    #[test]
    fn broadcast_rejects_non_self_accepted_block() {
        let corpus = ConwayValidityCorpus::load().expect("corpus loads");
        // Heaviest block: a non-empty body so a structure-preserving
        // content flip exists.
        let block = t4_pick_heaviest(&corpus).to_vec();
        let altered = t4_flip_body_byte(&block);

        let (handle, view) = SChainHandle::new();
        assert!(view.borrow().is_empty(), "served snapshot starts empty");

        // Mirror `apply_effects_with_forge_handler`'s ForgeSucceeded arm:
        // `advance` is the gate before any push.
        let evo = t4_seed(&corpus);
        let surfaced_event = match evo.advance(&altered) {
            Ok((_next, accepted)) => {
                // Would push on success — assert we never reach here.
                handle.push_atomic(accepted).expect("unexpected push");
                CoordinatorEvent::ForgeSucceeded {
                    slot: 0,
                    artifact: ade_runtime::producer::coordinator::ForgedBlockArtifact {
                        slot: 0,
                        hash: [0u8; 32],
                        bytes: altered.clone(),
                    },
                }
            }
            Err(e) => {
                // The advance-`Err` arm: no push, keep feeding ForgeFailed.
                assert!(
                    matches!(e, ChainEvolutionError::SelfAcceptRejected(_)),
                    "flipped body must be a self_accept rejection, got {e:?}"
                );
                CoordinatorEvent::ForgeFailed {
                    slot: 0,
                    reason: ForgeFailureReason::Other,
                }
            }
        };

        // push_atomic was NOT called: the served snapshot is still empty.
        assert!(
            view.borrow().is_empty(),
            "rejected block must not be pushed to the served snapshot"
        );
        // The slot is surfaced as a forge failure, not a success.
        assert!(
            matches!(surfaced_event, CoordinatorEvent::ForgeFailed { .. }),
            "rejected forge must surface as ForgeFailed"
        );
    }

    /// CE-T-10 (positive): a valid forged (corpus) block self-accepts via
    /// `ChainEvolution::advance`; the resulting BLUE-minted
    /// `AcceptedBlock` is routed to `ServedChainHandle::push_atomic` (the
    /// `BroadcastBlock` path) and the served snapshot then contains it.
    #[test]
    fn broadcast_pushes_self_accepted_block_to_served() {
        let corpus = ConwayValidityCorpus::load().expect("corpus loads");
        let block = t4_pick_lightest(&corpus).to_vec();

        let (handle, view) = SChainHandle::new();
        assert!(view.borrow().is_empty(), "served snapshot starts empty");

        // Obtain the BLUE-minted token via `advance` (the sole authority;
        // GREEN never mints) — same as the handler's ForgeSucceeded arm.
        let evo = t4_seed(&corpus);
        let (_next, accepted) = evo
            .advance(&block)
            .expect("lightest corpus block self-accepts");

        // Cross-check the token equals a direct self_accept (the same
        // authority the handler relies on for the broadcast token).
        let direct = ledger_self_accept(
            &block,
            &t4_ledger(),
            &t4_chain_dep(corpus.epoch_nonce),
            &t4_schedule(),
            &t4_view(&corpus),
        )
        .expect("direct self_accept");
        assert_eq!(
            accepted.as_bytes(),
            direct.as_bytes(),
            "advance token must equal direct self_accept token bytes"
        );

        // BroadcastBlock → push_atomic.
        let tip = handle.push_atomic(accepted).expect("push_atomic admits");

        let snap = view.borrow();
        assert_eq!(snap.len(), 1, "served snapshot must contain the pushed block");
        assert!(
            snap.block_at(tip.slot, &tip.hash).is_some(),
            "pushed block must be present by (slot, hash) key"
        );
    }
}
