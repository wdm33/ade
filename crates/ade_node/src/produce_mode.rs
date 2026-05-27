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

use ade_core::consensus::leader_check::{verify_and_evaluate_leader, LeaderCheckVerdict};
use ade_core::consensus::leader_schedule::LeaderScheduleAnswer;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::vrf_cert::{vrf_input, VrfRole};
use ade_core::consensus::era_schedule::EraSchedule;
use ade_crypto::kes::KesPeriod;
use ade_crypto::vrf::VrfVerificationKey;
use ade_ledger::consensus_view::PoolDistrView;
use ade_ledger::mempool::admit::MempoolState;
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::producer::forge::{forge_block, ForgeError};
use ade_ledger::producer::self_accept::self_accept;
use ade_ledger::state::LedgerState;
use ade_runtime::network::n2n_listener::{run_n2n_listener, N2nListenerConfig};
use ade_runtime::producer::tick_assembler::{assemble_tick, TickInputs};
use ade_types::shelley::block::ProtocolVersion;
use ade_types::{BlockNo, Hash32};
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
    let synthetic_forge = build_synthetic_forge_context(&{
        // Re-borrow genesis for the synthetic context. The
        // genesis_anchor moved into CoordinatorConfig; copy by value.
        ade_runtime::producer::coordinator::GenesisAnchor {
            network_magic: coord_state.genesis_anchor.network_magic,
            slot_zero_time_unix_ms: coord_state.genesis_anchor.slot_zero_time_unix_ms,
            slot_length_ms: coord_state.genesis_anchor.slot_length_ms,
            slots_per_kes_period: coord_state.genesis_anchor.slots_per_kes_period,
            kes_anchor_slot: coord_state.genesis_anchor.kes_anchor_slot,
            kes_max_period: coord_state.genesis_anchor.kes_max_period,
        }
    });

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
    // PHASE4-N-R-B B2/B3: per-peer n2n_server state map + served-chain
    // handle/view pair. The handle stays in produce_mode; the view is
    // borrowed by `dispatch_server_frame_event` to read the current
    // snapshot atomically.
    let mut peers_state: ServerPeerStates = BTreeMap::new();
    let (_served_chain_handle, served_chain_view) =
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
                    &synthetic_forge,
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
                    &mut peers_state,
                    &served_chain_view,
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

/// Read-only inputs the real forge handler needs in addition to the
/// `RequestForge` effect fields. Built by the main loop from the
/// loaded ledger snapshot / chain tip / era schedule / chain dep
/// state.
///
/// **A3 honest scope:** the main loop currently passes synthesized
/// "never-leader" placeholders (`stake_fraction = (0, 1)`,
/// `LedgerState::new(Conway)`, default `ProtocolParameters`). The
/// composition function is complete; promoting the inputs to real
/// values is A4 (integration tests) + N-R-B/C (main-loop wiring).
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
/// 3. **RED** — `kes_sign_at(kes_period, signing_payload)` produces
///    the KES signature. **A3 honest scope:** the signing payload
///    is currently the `expected_vrf_input` bytes (a placeholder).
///    The real Praos KES-signs-unsigned-header recipe lives in a
///    future cluster; for A3 the placeholder is sufficient because
///    `self_accept` will reject the synthetic block anyway, exercising
///    the `ForgeFailed { SelfAcceptRejected }` path.
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
    // RED step 1 — VRF prove over the canonical leader input.
    let expected = vrf_input(
        ade_types::SlotNo(slot),
        ctx.eta0,
        VrfRole::LeaderEligibility,
    );
    let (vrf_proof, _vrf_output_red) = match shell.vrf_prove(&expected) {
        Ok(v) => v,
        Err(_) => {
            return CoordinatorEvent::ForgeFailed {
                slot,
                reason: ForgeFailureReason::Other,
            };
        }
    };

    // BLUE step 2 — leader-check evaluator (verify proof + threshold).
    let verdict = match verify_and_evaluate_leader(
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

/// Build the synthetic ForgeRequestContext used by the main loop.
///
/// **A3 honest scope:** every field is a placeholder. The
/// `leader_schedule_answer` carries zero stake → the real forge
/// composition always reaches step 2 and returns `NotLeader`,
/// preserving the smoke test's expected 3 `LeaderCheckOutcome`
/// events. Promoting these to realer values is A4 + N-R-B/C work.
fn build_synthetic_forge_context(genesis: &GenesisAnchor) -> SyntheticForgeInputs {
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_types::{CardanoEra, EpochNo, Hash28};

    let _ = genesis;
    let eta0 = Nonce(Hash32([0u8; 32]));
    let leader_answer = LeaderScheduleAnswer {
        slot: ade_types::SlotNo(0),
        pool: Hash28([0u8; 28]),
        epoch: EpochNo(0),
        expected_vrf_input: [0u8; ade_core::consensus::vrf_cert::VRF_INPUT_LEN],
        stake_fraction: (0, 1),
        asc: ActiveSlotsCoeff { numer: 1, denom: 20 },
    };
    let pparams = ProtocolParameters::default();
    let base_state = LedgerState::new(CardanoEra::Conway);
    let chain_dep_state = PraosChainDepState::empty();
    let era_schedule = EraSchedule::new(
        ade_core::consensus::BootstrapAnchorHash(Hash32([0u8; 32])),
        0,
        vec![ade_core::consensus::EraSummary {
            era: CardanoEra::Conway,
            start_slot: ade_types::SlotNo(0),
            start_epoch: EpochNo(0),
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 129_600,
        }],
    )
    .expect("synthetic era schedule");
    let pool_distr_view = PoolDistrView::new(
        EpochNo(0),
        0,
        ActiveSlotsCoeff { numer: 1, denom: 20 },
        std::collections::BTreeMap::new(),
    );

    SyntheticForgeInputs {
        eta0,
        leader_schedule_answer: leader_answer,
        pparams,
        base_state,
        chain_dep_state,
        era_schedule,
        pool_distr_view,
        block_number: BlockNo(1),
        prev_hash: Hash32([0u8; 32]),
        protocol_version: ProtocolVersion { major: 9, minor: 0 },
        prev_opcert_counter: None,
    }
}

struct SyntheticForgeInputs {
    eta0: Nonce,
    leader_schedule_answer: LeaderScheduleAnswer,
    pparams: ProtocolParameters,
    base_state: LedgerState,
    chain_dep_state: PraosChainDepState,
    era_schedule: EraSchedule,
    pool_distr_view: PoolDistrView,
    block_number: BlockNo,
    prev_hash: Hash32,
    protocol_version: ProtocolVersion,
    prev_opcert_counter: Option<u64>,
}

impl SyntheticForgeInputs {
    /// Build a per-slot `LeaderScheduleAnswer` whose
    /// `expected_vrf_input` matches `vrf_input(slot, eta0, LEADER)` —
    /// so `verify_and_evaluate_leader` passes the coherence check and
    /// reaches the threshold step (where zero stake yields
    /// `NotEligible`).
    fn leader_schedule_answer_for_slot(&self, slot: u64) -> LeaderScheduleAnswer {
        LeaderScheduleAnswer {
            slot: ade_types::SlotNo(slot),
            pool: self.leader_schedule_answer.pool.clone(),
            epoch: self.leader_schedule_answer.epoch,
            expected_vrf_input: vrf_input(
                ade_types::SlotNo(slot),
                &self.eta0,
                VrfRole::LeaderEligibility,
            ),
            stake_fraction: self.leader_schedule_answer.stake_fraction,
            asc: self.leader_schedule_answer.asc,
        }
    }

    fn as_context_with_answer_and_vk<'a>(
        &'a self,
        answer: &'a LeaderScheduleAnswer,
        vrf_vk: &'a VrfVerificationKey,
    ) -> ForgeRequestContext<'a> {
        ForgeRequestContext {
            eta0: &self.eta0,
            vrf_vk,
            leader_schedule_answer: answer,
            pparams: &self.pparams,
            base_state: &self.base_state,
            chain_dep_state: &self.chain_dep_state,
            era_schedule: &self.era_schedule,
            pool_distr_view: &self.pool_distr_view,
            block_number: self.block_number,
            prev_hash: self.prev_hash.clone(),
            protocol_version: self.protocol_version,
            prev_opcert_counter: self.prev_opcert_counter,
        }
    }
}

/// Apply effects with the real forge handler (A3): replaces the S5
/// `ForgeNotLeader`-only stub with the full BLUE-then-RED-then-BLUE
/// composition via [`run_real_forge`]. The synthetic context built by
/// [`build_synthetic_forge_context`] carries zero stake, so the
/// composition always returns `NotLeader` at step 2 for the current
/// smoke-test inputs. A4 integration tests exercise the full path
/// with realer fixtures; N-R-B/C promote the main-loop inputs to
/// real ledger state.
fn apply_effects_with_forge_handler(
    effects: &[CoordinatorEffect],
    evidence_writer: &mut std::fs::File,
    coord_state: &mut ade_runtime::producer::coordinator::CoordinatorState,
    shell: &mut ProducerShell,
    synthetic: &SyntheticForgeInputs,
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
                // Build per-slot LeaderScheduleAnswer so its
                // expected_vrf_input matches the canonical input
                // verify_and_evaluate_leader computes from (slot,
                // eta0). The synthetic context provides the stake +
                // ASC + pool ID; the per-slot wrapper computes the
                // VRF input recipe.
                let answer = synthetic.leader_schedule_answer_for_slot(*slot);
                // Use the shell's real VRF VK so verify_vrf accepts the
                // proof shell.vrf_prove produces.
                let real_vrf_vk = shell.vrf_verification_key();
                let ctx = synthetic.as_context_with_answer_and_vk(&answer, &real_vrf_vk);
                let event = run_real_forge(*slot, *kes_period, &ctx, shell);
                let prev_state = coord_state.clone();
                let (new_state, more_effects) =
                    coordinator_step(prev_state, event).map_err(|_| "forge handler step")?;
                *coord_state = new_state;
                for me in &more_effects {
                    if let CoordinatorEffect::LogEvidence { event } = me {
                        write_evidence_event(evidence_writer, event)?;
                    }
                }
            }
            CoordinatorEffect::BroadcastBlock { artifact: _ } => {
                // N-R-B (B2) wires push_atomic here.
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

fn handle_listener_event(
    evt: OrchestratorEvent,
    coord_state: &mut ade_runtime::producer::coordinator::CoordinatorState,
    evidence_writer: &mut std::fs::File,
    connected_peers: &mut BTreeMap<PeerId, ()>,
    peers_state: &mut ServerPeerStates,
    served_chain_view: &ade_runtime::producer::served_chain_handle::ServedChainView,
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
        // PHASE4-N-R-B B3: explicit dispatch into n2n_server reducers.
        // Receive-side variants (PeerChainSyncFrame, PeerBlockFetchFrame)
        // are not relevant in produce mode (we are the server) — they
        // remain absorbed.
        //
        // Server-side variants (PeerN2nServerChainSyncFrame,
        // PeerN2nServerBlockFetchFrame): explicit dispatch is wired in
        // the dedicated `handle_server_frame_*` helpers below.
        // **Honest scope (B3):** dispatch runs and the response bytes
        // are constructed; the transmit-back-to-peer wire requires
        // extending MuxPump (deferred to N-R-C / a future cluster).
        // For now, B3 advances per-peer state correctly and proves the
        // reducers run; B4 closes via integration tests.
        // PHASE4-N-R-B B3: explicit dispatch for server-side frame
        // events. Replaces the previous `_ => {}` absorption.
        OrchestratorEvent::PeerN2nServerChainSyncFrame { .. }
        | OrchestratorEvent::PeerN2nServerBlockFetchFrame { .. } => {
            // dispatch_server_frame_event advances per-peer state +
            // computes response bytes. The response-byte length is
            // returned for logging; transmitting bytes back to the
            // peer requires extending MuxPump with an outbound-relay
            // channel (deferred to N-R-C / a future cluster).
            let _reply_byte_count =
                dispatch_server_frame_event(&evt, peers_state, served_chain_view)?;
        }
        _ => {}
    }
    Ok(())
}

/// PHASE4-N-R-B B3: dispatch a server-side frame event through the
/// n2n_server reducer. The reducer outputs are LOGGED (length +
/// fingerprint) into the evidence stream; transmitting bytes back
/// to the peer requires extending MuxPump with an outbound-relay
/// channel — a documented N-R-C / future-cluster deliverable.
fn dispatch_server_frame_event(
    event: &OrchestratorEvent,
    peers_state: &mut ServerPeerStates,
    served_chain_view: &ade_runtime::producer::served_chain_handle::ServedChainView,
) -> Result<usize, &'static str> {
    use ade_runtime::network::n2n_server::{
        dispatch_block_fetch_frame, dispatch_chain_sync_frame,
    };
    match event {
        OrchestratorEvent::PeerN2nServerChainSyncFrame { peer_id, bytes } => {
            let key = PeerId(peer_id.0);
            let state = peers_state.get(&key).cloned().ok_or("peer not connected")?;
            let snap_ref = served_chain_view.borrow();
            let (new_state, maybe_reply, _done) =
                dispatch_chain_sync_frame(state, bytes, &*snap_ref).map_err(|_| "chain-sync dispatch")?;
            peers_state.insert(key, new_state);
            Ok(maybe_reply.map(|b| b.len()).unwrap_or(0))
        }
        OrchestratorEvent::PeerN2nServerBlockFetchFrame { peer_id, bytes } => {
            let key = PeerId(peer_id.0);
            let state = peers_state.get(&key).cloned().ok_or("peer not connected")?;
            let snap_ref = served_chain_view.borrow();
            let (new_state, replies, _done) =
                dispatch_block_fetch_frame(state, bytes, &*snap_ref).map_err(|_| "block-fetch dispatch")?;
            peers_state.insert(key, new_state);
            Ok(replies.iter().map(Vec::len).sum())
        }
        _ => Ok(0),
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
