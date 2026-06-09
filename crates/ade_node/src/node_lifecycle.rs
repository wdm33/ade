// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED `--mode node` Ade node lifecycle owner (PHASE4-N-F-C).
//!
//! `PHASE4-N-F-C-LIFECYCLE-OWNER`: this module is THE single production
//! recovered-state lifecycle owner for PHASE4-N-F-C — see
//! `docs/clusters/PHASE4-N-F-C/cluster.md`, the L1 slice doc
//! `docs/clusters/PHASE4-N-F-C/C1-production-lifecycle-owner.md`, and the
//! L2 slice doc `docs/clusters/PHASE4-N-F-C/L2-mithril-first-run-bootstrap.md`.
//!
//! Shape:
//!   1. open a persistent `ChainDb` + `FileWalStore`,
//!   2. classify first-run (empty store) vs warm-start (non-empty) as a
//!      PURE function of on-disk state (`classify_start`), then
//!   3. FirstRun → **Mithril-only first-run bootstrap (L2)**: assemble the
//!      seed from documented-extraction inputs bound to a Mithril manifest,
//!      run `bootstrap_from_mithril_snapshot` (its first non-test caller),
//!      which fail-closes on `verify_mithril_binding` BEFORE any state is
//!      admitted and persists the seed-epoch sidecar + WAL provenance under
//!      one `BootstrapAnchor` lineage.
//!      WarmStart → production warm-start recovery (L3) — still a typed
//!      FAIL-CLOSED stub here (L3 builds it).
//!
//! Mithril-only, fail-closed (cluster rule): the FirstRun arm has NO
//! genesis branch, NO `--consensus-inputs-path`-as-forge-input, NO
//! peer-extracted-without-cert path, NO tip-bundle, NO cold-`produce_mode`
//! fallback, and NO native Mithril UTXO-HD/LedgerDB decode. The
//! `--json-seed-path` + `--consensus-inputs-path` files are **first-run
//! bootstrap extraction inputs** (documented cardano-cli extraction from the
//! Mithril-certified/restored state), Mithril-bound by the manifest +
//! `verify_mithril_binding` — never forge inputs. Initial state flows ONLY
//! through the single `bootstrap_initial_state` authority (which
//! `bootstrap_from_mithril_snapshot` calls); the owner never calls a second
//! bootstrap authority. `produce_mode` and `admission` remain unchanged
//! diagnostic modes.
//!
//! Not yet wired (later slices): L3 warm-start recovery; L4 peer BlockFetch
//! → durable `pump_block` apply; L5 produce from the recovered selected tip
//! + recovered inputs; L6 BA-02 peer-acceptance evidence.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

use ade_core::consensus::era_schedule::{EraSchedule, EraSummary};
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_core::consensus::BootstrapAnchorHash;
use ade_ledger::consensus_view::PoolDistrView;
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::state::LedgerState;
use ade_ledger::seed_consensus_inputs::decode_seed_epoch_consensus_inputs;
use ade_ledger::wal::{replay_from_anchor, RollbackPoint, RollbackReason, WalEntry, WalStore};
use ade_runtime::bootstrap::{
    bootstrap_initial_state, BootstrapInputs, BootstrapState, SeedEpochConsensusSource,
};
use ade_runtime::admission::{dial_for_admission, run_admission_wire_pump, AdmissionPeerEvent};
use ade_runtime::chaindb::{
    ChainDb, ChainTip, PersistentChainDb, PersistentChainDbOptions, SnapshotStore,
};
use ade_runtime::consensus_inputs::{import_live_consensus_inputs, LiveConsensusInputsCanonical};
use ade_runtime::mithril_bootstrap::{bootstrap_from_mithril_snapshot, MithrilSeedPointInputs};
use ade_runtime::mithril_import::import_mithril_manifest_from_bytes;
use ade_runtime::seed_import::import_cardano_cli_json_utxo;
use ade_runtime::wal::FileWalStore;
use ade_types::shelley::block::ProtocolVersion;
use ade_types::{BlockNo, CardanoEra, EpochNo, Hash28, Hash32, SlotNo};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch};

use ade_core::consensus::ledger_view::LedgerView;
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::receive::ReceiveState;
use ade_runtime::clock::{checked_millis_to_slot, Clock, SlotAlignmentError, SystemClock};
use ade_runtime::forward_sync::{pump_block, ForwardSyncState, PumpError, SnapshotSink};
use ade_runtime::producer::coordinator::{
    coordinator_init, CoordinatorConfig, CoordinatorEvent, CoordinatorState, LedgerSnapshotRef,
};
use ade_runtime::producer::producer_shell::ProducerShell;
use ade_runtime::rollback::{ChainDbBlockSource, PersistentSnapshotCache, SnapshotCadence};
use ade_ledger::rollback::{
    commit_rollback, materialize_rolled_back_state, CommitRollbackError, MaterializeError,
    TargetPoint,
};
use ade_core::consensus::events::ChainEvent;
use ade_runtime::receive::ChainDbWriter;

use crate::admission::bootstrap::build_n2n_version_table;
use crate::cli::Cli;
use crate::forge_intent::{classify_forge_intent, ForgeIntent};
use crate::node_sync::{
    admit_forged_block_durably, durable_tip_matches, forge_followed_tip_admission,
    forge_mode_after_admit, forge_mode_on_caughtup, forge_one_from_recovered, run_node_sync,
    single_producer_forge_decision, venue_policy, ForgeFollowedTipAdmission, ForgeMode,
    ForgeRefused, NodeBlockSource, NodeForgeOutcome, SingleProducerForgeDecision,
    VenueRole,
};
use crate::operator_forge;
use crate::run_loop_planner::{
    forge_slot_status, plan_loop_step, ForgeSlotStatus, LoopState, LoopStep, ShutdownStatus,
    SyncStatus, VenuePolicy,
};
use crate::EXIT_GENERIC_STARTUP;

// PHASE4-N-F-G-H S2: node-spine serve-to-peer sibling imports. The serve
// reuses the per-peer N2N session machinery (`run_per_peer_session`) + the
// single shared serve-dispatch core (S1, `ade_runtime::network::serve_dispatch`)
// over the G-B `ServedChainView`. The serve listener advertises the N2N
// responder table built per the configured network magic (S2b,
// `n2n_supported_for_magic`) — NOT the static mainnet `N2N_SUPPORTED`.
use ade_ledger::receive::events::TipPoint;
use ade_network::chain_sync::server::ServedHeaderLookup;
use ade_network::handshake::version_table::n2n_supported_for_magic;
use ade_runtime::network::n2n_listener::{run_per_peer_session, PerPeerSessionConfig};
use ade_runtime::network::outbound_command::new_per_peer_outbound;
use ade_runtime::network::serve_dispatch::{
    dispatch_server_frame_event_to_outbound, install_server_peer_state, remove_server_peer_state,
    ServedChainSource, ServerPeerStates,
};
use ade_runtime::network::ChainDbServedSource;
use ade_runtime::orchestrator::event::{OrchestratorEvent, PeerRole};
use ade_runtime::orchestrator::n2n_server_pump::PeerIdGenerator;
use ade_runtime::producer::producer_log::PeerId as ServerPeerId;

/// Clean-exit code (mirrors the local constant in `wire_only`; the
/// crate root does not re-export a single `EXIT_OK`).
const EXIT_OK: i32 = 0;

/// Exit code emitted when the node lifecycle owner reaches an arm whose
/// production wiring has not landed yet (currently L3 warm-start).
/// Distinct from a generic startup error so an operator can tell a
/// "not-yet-wired, fail-closed" exit from a bad-CLI exit.
pub const EXIT_NODE_LIFECYCLE_UNWIRED: i32 = 40;

/// Exit code for a fail-closed first-run Mithril bootstrap (missing
/// manifest / binding mismatch / epoch mismatch / extraction failure /
/// bootstrap failure). Distinct so an operator can tell a Mithril
/// provenance failure from a bad-CLI or not-yet-wired exit.
pub const EXIT_NODE_MITHRIL_BOOTSTRAP_FAILED: i32 = 41;

/// Exit code for a fail-closed production warm-start recovery (L3): no
/// persisted anchor lineage, more than one lineage, missing WAL
/// provenance, a WAL replay defect (chain break / missing block bytes /
/// duplicate provenance / anchor mismatch), a snapshot below the tip that
/// would require forward replay (L4 territory), or the
/// `bootstrap_initial_state` sidecar verify chain failing. Distinct so an
/// operator can tell a recovery failure from a first-run / bad-CLI exit.
pub const EXIT_NODE_WARM_START_RECOVERY_FAILED: i32 = 42;

/// Exit code for a fail-closed relay run-loop sync step (PHASE4-N-F-D): the
/// `run_node_sync` → `pump_block` seam rejected a block (undecodable /
/// unvalidatable / cross-epoch / durability fault). Distinct so an operator
/// can tell a sync failure from a bootstrap / recovery / bad-CLI exit.
pub const EXIT_NODE_RELAY_SYNC_FAILED: i32 = 43;

/// Exit code for a fail-closed operator-key ingress (PHASE4-N-F-F): a partial
/// operator key set, an operator-material load failure, or a genesis-anchor
/// parse failure on the forge-on path. Distinct so an operator can tell a
/// key-ingress failure from a bootstrap / recovery / sync / bad-CLI exit.
pub const EXIT_NODE_FORGE_KEY_INGRESS_FAILED: i32 = 44;

/// The first-run-vs-warm-start classification — a closed sum derived
/// purely from what is persisted on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStart {
    /// Nothing persisted: no ChainDb tip AND no snapshots. The Mithril
    /// first-run bootstrap (L2) owns this arm.
    FirstRun,
    /// Something persisted: a ChainDb tip and/or at least one snapshot.
    /// The production warm-start recovery (L3) owns this arm.
    WarmStart,
}

/// Closed owner-error surface. Every variant is a deterministic
/// fail-closed halt — none performs a genesis / bundle / cold-start /
/// tip-bundle fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeLifecycleError {
    /// A required path/flag was not supplied.
    MissingFlag(&'static str),
    /// Opening the persistent `ChainDb` failed.
    ChainDbOpen(String),
    /// Opening the `FileWalStore` failed.
    WalOpen(String),
    /// Reading on-disk state (tip / snapshot slots) failed.
    OnDiskRead(String),
    /// A first-run input file (manifest / UTxO / consensus inputs) could
    /// not be read or parsed.
    ExtractionRead(String),
    /// Parsing a fixed-size hash CLI value (`--genesis-hash` /
    /// `--seed-block-hash`) failed.
    BadHashHex(&'static str),
    /// The Mithril manifest could not be imported (malformed manifest /
    /// unsupported artifact type). Fail closed.
    ManifestImport(String),
    /// The manifest's certified point falls outside the consensus
    /// inputs' declared epoch window (epoch-consistency check, L2 §9.4).
    /// The documented extraction was taken for a different epoch than the
    /// Mithril certificate attests. Fail closed.
    EpochMismatch {
        consensus_epoch: u64,
        certified_slot: u64,
        consensus_window: (u64, u64),
    },
    /// `bootstrap_from_mithril_snapshot` failed: binding mismatch, the
    /// single bootstrap authority, the seed-epoch merge, the sidecar
    /// persist, or the WAL-provenance append. Carries the closed
    /// `MithrilBootstrapError` debug. Fail closed — NO fallback.
    MithrilBootstrap(String),
    /// Warm-start: the sidecar table holds no persisted anchor lineage,
    /// so there is nothing to recover. Fail closed — NO bundle fallback.
    WarmStartNoAnchorLineage,
    /// Warm-start: the sidecar table holds more than one anchor lineage.
    /// Exactly one is expected (single-epoch, single-shot; CN-ANCHOR-01).
    /// Fail closed rather than guess which lineage to recover.
    WarmStartMultipleAnchorLineages { count: usize },
    /// Warm-start: reading or replaying the WAL fail-closed — a
    /// `ChainBreak`, `BlockBytesMissing`, `DuplicateProvenance`, or
    /// `ProvenanceAnchorMismatch` (the WAL provenance entry's `anchor_fp`
    /// disagreed with the independent sidecar-key anchor_fp). Carries the
    /// closed `WalError` debug. Fail closed.
    WarmStartWalReplay(String),
    /// Warm-start: the WAL replay surfaced no `RecoveredBootstrapProvenance`
    /// (no `SeedEpochConsensusInputsImported` entry). The sidecar exists but
    /// its commit-point provenance is absent — treat as "not imported".
    /// Fail closed.
    WarmStartNoProvenance,
    /// Warm-start: the persisted snapshot is below the chain tip, so
    /// recovery would require forward block replay. That is L4 durable-apply
    /// territory (and L4c's crash-window proof); L3 recovers only a
    /// snapshot-at-tip precondition. Fail closed rather than replay with a
    /// non-recovered leadership view.
    WarmStartForwardReplayUnsupported { tip_slot: u64 },
    /// Warm-start: the single `bootstrap_initial_state` authority's
    /// `RequiredFromRecoveredProvenance` verify chain fail-closed — sidecar
    /// missing for the anchor, `sidecar_hash` mismatch, anchor/epoch binding
    /// mismatch, byte-identity mismatch, or a malformed sidecar. Carries the
    /// closed `BootstrapError` debug. Fail closed — NO bundle fallback.
    WarmStartBootstrap(String),
    /// The relay run loop's sync step (`run_node_sync` → `pump_block`)
    /// fail-closed on a block (undecodable, unvalidatable, a cross-epoch
    /// header beyond the recovered single-epoch view, or a durability
    /// fault). Carries the closed `NodeSyncError` debug. Fail closed — the
    /// loop never skips past a rejected block (PHASE4-N-F-D S2).
    RelaySync(String),
    /// PHASE4-N-F-F: operator-key ingress for `--mode node` fail-closed — a
    /// partial operator key set (some-but-not-all key flags), an
    /// operator-material load failure, or a genesis-anchor parse failure on the
    /// forge-on path. Carries a structured, secret-free message (no path bytes,
    /// no key bytes). Fail closed — NO forge with a partial set, NO silent
    /// relay-only fallback. Does NOT touch the bootstrap/recovery layer.
    ForgeKeyIngress(String),
    /// PHASE4-N-F-G-H S2: node-spine serve-to-peer start fail-closed — the
    /// `--listen` value did not parse, or binding the serve listener failed.
    /// Surfaced explicitly (fail-fast): the node never proceeds claiming live
    /// serve capability while serving is disabled (no silent live-serve claim).
    /// Carries a structured, secret-free message.
    ServeStart(String),
    /// PHASE4-N-F-G-P (DC-CINPUT-04): a live feed is wired (`--peer`) but the
    /// recovered state carries no `SeedEpochConsensusInputs`, so the feed
    /// header-validation view (Step 5 VRF-keyhash + Step 7 leader threshold)
    /// cannot be projected from the recovered consensus surface. Fail closed —
    /// never validate a peer's block against an empty stake view, never
    /// accept-if-missing.
    FeedMissingRecoveredConsensusInputs,
}

/// Pure first-run-vs-warm-start classifier. A function of on-disk state
/// ONLY (no wall-clock, no env): first-run iff the store is completely
/// empty (no tip and no snapshots); otherwise warm-start. Mirrors the
/// branch `bootstrap_initial_state` itself takes, so the owner and the
/// single authority agree on what "empty" means.
pub fn classify_start(has_tip: bool, has_snapshots: bool) -> NodeStart {
    if !has_tip && !has_snapshots {
        NodeStart::FirstRun
    } else {
        NodeStart::WarmStart
    }
}

// PHASE4-N-U S3 (DC-NODE-13): the PHASE4-N-F-G-R monotone served-chain gate
// (`serve_gate_admits`) is RETIRED. It gated an in-memory accumulator so the
// served view held exactly one block 0 despite the hermetic forge's re-mints.
// With own-forged durable admit (S1), the durable chain is extend-only
// (DC-CONS-23) — a re-mint block 0 fails closed at admit, so the durable chain
// holds exactly one block 0 by construction. The serve task now projects that
// durable chain (`run_node_serve_task` over `ChainDbServedSource`), so the
// stability the gate provided is a property of the durable chain itself — no
// gate needed. DC-NODE-11's invariant is preserved (and strengthened) by
// serve-as-projection.

/// The `--mode node` owner entry. Returns a process exit code.
///
/// `shutdown` is the SIGINT/SIGTERM watch flag; it is now load-bearing —
/// both lifecycle arms converge into the relay run loop (PHASE4-N-F-D S2),
/// which halts cleanly when `shutdown` flips.
pub async fn run_node_lifecycle(cli: Cli, mut shutdown: watch::Receiver<bool>) -> ExitCode {
    match run_node_lifecycle_inner(&cli, &mut shutdown).await {
        Ok(()) => ExitCode::from(EXIT_OK as u8),
        Err(e) => {
            report(&e);
            ExitCode::from(exit_code_for(&e) as u8)
        }
    }
}

fn exit_code_for(e: &NodeLifecycleError) -> i32 {
    match e {
        NodeLifecycleError::MissingFlag(_)
        | NodeLifecycleError::ChainDbOpen(_)
        | NodeLifecycleError::WalOpen(_)
        | NodeLifecycleError::OnDiskRead(_)
        | NodeLifecycleError::BadHashHex(_)
        | NodeLifecycleError::ExtractionRead(_)
        | NodeLifecycleError::ServeStart(_) => EXIT_GENERIC_STARTUP,
        NodeLifecycleError::ManifestImport(_)
        | NodeLifecycleError::EpochMismatch { .. }
        | NodeLifecycleError::MithrilBootstrap(_) => EXIT_NODE_MITHRIL_BOOTSTRAP_FAILED,
        NodeLifecycleError::WarmStartNoAnchorLineage
        | NodeLifecycleError::WarmStartMultipleAnchorLineages { .. }
        | NodeLifecycleError::WarmStartWalReplay(_)
        | NodeLifecycleError::WarmStartNoProvenance
        | NodeLifecycleError::WarmStartForwardReplayUnsupported { .. }
        | NodeLifecycleError::WarmStartBootstrap(_) => EXIT_NODE_WARM_START_RECOVERY_FAILED,
        NodeLifecycleError::RelaySync(_)
        | NodeLifecycleError::FeedMissingRecoveredConsensusInputs => EXIT_NODE_RELAY_SYNC_FAILED,
        NodeLifecycleError::ForgeKeyIngress(_) => EXIT_NODE_FORGE_KEY_INGRESS_FAILED,
    }
}

async fn run_node_lifecycle_inner(
    cli: &Cli,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<(), NodeLifecycleError> {
    // 1. Required persistence paths. `--snapshot-dir` holds the
    //    persistent ChainDb (which is also the SnapshotStore);
    //    `--wal-dir` holds the FileWalStore. No defaults: a missing
    //    path fails closed.
    let snapshot_dir = cli
        .snapshot_dir
        .as_ref()
        .ok_or(NodeLifecycleError::MissingFlag("--snapshot-dir"))?;
    let wal_dir = cli
        .wal_dir
        .as_ref()
        .ok_or(NodeLifecycleError::MissingFlag("--wal-dir"))?;

    // 2. Ensure the persistence directories exist (mirrors
    //    admission/bootstrap.rs). On a true first run the dirs are
    //    absent; creating them lets the first-run arm be REACHED.
    //    Creating an empty dir persists no chain facts.
    std::fs::create_dir_all(snapshot_dir)
        .map_err(|e| NodeLifecycleError::ChainDbOpen(format!("snapshot-dir: {:?}", e.kind())))?;
    std::fs::create_dir_all(wal_dir)
        .map_err(|e| NodeLifecycleError::WalOpen(format!("wal-dir: {:?}", e.kind())))?;

    // 3. Open the persistent stores. The ChainDb doubles as the
    //    SnapshotStore (PHASE4-N-T/N-Y); the WAL is the on-disk append
    //    log. Opening is non-mutating w.r.t. chain facts.
    let chaindb_path = snapshot_dir.join("chain.db");
    // PHASE4-N-U S3 (DC-NODE-13): shared (Arc) so the spawned serve task can
    // READ the durable ChainDb projection concurrently with the relay loop's
    // writes — redb reads are MVCC, so concurrent read-during-write is safe.
    // The setup + relay loop borrow `&chaindb` (deref-coerces to
    // `&PersistentChainDb`); the serve task gets an owned `Arc::clone`.
    let chaindb = Arc::new(
        PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path))
            .map_err(|e| NodeLifecycleError::ChainDbOpen(format!("{e:?}")))?,
    );
    let mut wal =
        FileWalStore::open(wal_dir).map_err(|e| NodeLifecycleError::WalOpen(format!("{e:?}")))?;

    // 4. Classify first-run vs warm-start as a pure function of on-disk
    //    state. (The same `(tip, snapshots)` axes `bootstrap_initial_state`
    //    branches on.)
    let tip = ChainDb::tip(chaindb.as_ref())
        .map_err(|e| NodeLifecycleError::OnDiskRead(format!("{e:?}")))?;
    let snapshot_slots = SnapshotStore::list_snapshot_slots(chaindb.as_ref())
        .map_err(|e| NodeLifecycleError::OnDiskRead(format!("{e:?}")))?;
    let start = classify_start(tip.is_some(), !snapshot_slots.is_empty());

    // 5. Obtain the verified initial state through the single bootstrap
    //    authority (FirstRun via bootstrap_from_mithril_snapshot; WarmStart
    //    via the warm-start verify chain). Fail closed; NO genesis / bundle /
    //    cold / tip fallback on either arm.
    let state = match start {
        NodeStart::FirstRun => first_run_mithril_bootstrap(cli, &chaindb, &mut wal)?,
        NodeStart::WarmStart => warm_start_recovery(&chaindb, &wal)?,
    };

    // 6. Both arms CONVERGE here into the one relay run loop (CN-NODE-02): no
    //    arm prints-and-exits any more.
    //
    //    N-F-D wires NO live peer (the live WirePump source is the RO-LIVE-01
    //    follow-on), so the binary's source is EMPTY: the loop is genuinely
    //    ENTERED and the GREEN planner drives it to a clean halt on the first
    //    tick (Ending + NoWorkReady => HaltCleanly) WITHOUT any SyncOnce
    //    consuming era_schedule / ledger_view. Those are deterministic
    //    placeholders here, PROVABLY UNCONSUMED on this binary path (empty
    //    source) — the same justification as the warm-start placeholder
    //    schedule/view. The populated-source behavior (durable sync, idle,
    //    shutdown, cross-epoch fail-closed) is proven HERMETICALLY by the
    //    run_relay_loop tests, NOT on this binary path. This is a hermetic
    //    cluster; it makes NO live-peer claim.
    let epoch = state
        .seed_epoch_consensus_inputs
        .as_ref()
        .map(|s| s.epoch_no.0);
    let tip_slot = state.tip.as_ref().map(|t| t.slot.0);

    // PHASE4-N-F-F: classify forge intent from operator-key flag PRESENCE.
    // Complete set => forge on; none => relay-only; partial => fail closed.
    // This does NOT bootstrap and does NOT call Mithril — the forge base is the
    // SINGLE recovered `state` produced above (FirstRun Mithril / WarmStart WAL).
    let intent = classify_forge_intent(
        cli.cold_skey.as_deref(),
        cli.kes_skey.as_deref(),
        cli.vrf_skey.as_deref(),
        cli.opcert.as_deref(),
        cli.genesis_file.as_deref(),
    )
    .map_err(|e| NodeLifecycleError::ForgeKeyIngress(format!("{e}")))?;

    match intent {
        ForgeIntent::Off => {
            // Exact N-F-D/N-F-E relay: forge OFF. Move the recovered ledger +
            // chain_dep into the spine (no clone); `None` reduces the planner to
            // the exact N-F-D relay behavior. Placeholders are PROVABLY UNCONSUMED
            // on the empty source (a feed-end halts the loop on iteration 1).
            let era_schedule = make_node_schedule(SlotNo(0), EpochNo(0));
            let ledger_view = PoolDistrView::new(
                EpochNo(0),
                0,
                ActiveSlotsCoeff { numer: 0, denom: 1 },
                BTreeMap::new(),
            );
            // PHASE4-N-AE.C (DC-WAL-02): the first followed AdmitBlock must chain
            // from the fingerprint of the ledger state the follow extends (the
            // recovered ledger tip = the WAL-tail post_fp), not from zero. Read it
            // before `state.ledger` is moved into the receive sub-state.
            let anchor_fp = fingerprint(&state.ledger).combined;
            let mut fwd = ForwardSyncState::new(
                ReceiveState::new(state.ledger, state.chain_dep),
                anchor_fp,
                SnapshotCadence::DEFAULT,
            );
            let mut source = NodeBlockSource::in_memory(Vec::new());
            // PHASE4-N-AH S4a (CN-NODE-04 / DC-NODE-20): emit the closed feed/forge
            // sched transcript to the --log JSONL file (node-run.jsonl) — the canonical
            // evidence artifact (stderr fallback); emit-only, never alters scheduling.
            let sched_sink: Box<dyn std::io::Write> = match std::fs::File::create(&cli.log_path)
            {
                Ok(f) => Box::new(f),
                Err(_) => Box::new(std::io::stderr()),
            };
            let mut sched_log = crate::live_log::NodeSchedLogWriter::new(sched_sink);
            run_relay_loop_with_sched(
                &mut fwd,
                &mut source,
                &chaindb,
                &mut wal,
                &era_schedule,
                &ledger_view,
                shutdown,
                None,
                Some(&mut sched_log),
            )
            .await?;
            eprintln!(
                "ade_node --mode node: relay run loop entered and halted cleanly \
                 (recovered/bootstrapped epoch={epoch:?}, tip slot={tip_slot:?}; \
                 forge OFF — no operator keys supplied; NO live peer source wired \
                 — sync / idle / shutdown proven hermetically). NO block produced."
            );
        }
        ForgeIntent::On(paths) => {
            // PHASE4-N-F-F: operator-material-backed forge activation. Loads the
            // operator signing material ONLY — it does NOT bootstrap, does NOT
            // call Mithril, and reuses the SINGLE recovered `state` above as the
            // forge base (CN-NODE-01: no second bootstrap path).
            let operator_forge::OperatorForgeMaterial {
                mut shell,
                genesis,
                pool_id,
                anchor_millis,
                start_slot,
                slot_length_ms,
            } = operator_forge::build_operator_forge_material(&paths)
                .map_err(|e| NodeLifecycleError::ForgeKeyIngress(format!("{e}")))?;
            // Coordinator: the genesis-anchor host for the REUSED
            // `kes_period_for_slot` (no slot→KES reimplementation). Holds no
            // secrets (CN-PROD-02).
            let (coord_state, _init_effects) = coordinator_init(CoordinatorConfig {
                genesis_anchor: genesis,
                opcert_meta: shell.public_metadata(),
                initial_chain_tip: None,
                initial_ledger_snapshot_ref: LedgerSnapshotRef(0),
                broadcast_queue_limit: 32,
                peer_limit: 16,
            });
            // Real era schedule from the recovered epoch (consumed only when a
            // live feed lands; unconsumed on the empty source this cluster).
            let era_schedule =
                make_node_schedule(SlotNo(tip_slot.unwrap_or(0)), EpochNo(epoch.unwrap_or(0)));
            // DC-CINPUT-04 (PHASE4-N-F-G-P): the feed header-validation view MUST be
            // the recovered consensus surface — the SAME projection the forge uses
            // (`forge_one_from_recovered` / DC-CINPUT-02b) — so Step 5 (VRF-keyhash
            // binding) + Step 7 (leader threshold) see the real recovered ASC + total
            // + pool stake + pool VRF keyhash. An empty placeholder makes the live
            // feed reject EVERY block (`pool_active_stake == None` ⇒ a structural
            // `VrfCert(VerificationFailed)`). Fail closed when a live feed is wired
            // (`--peer`) but the recovered record is absent — never an empty view,
            // never accept-if-missing. With NO feed wired the loop halts before
            // consuming the view, so an absent record degrades to a
            // provably-unconsumed placeholder rather than a hard stop.
            let live_feed_wired = !cli.peer_addrs.is_empty();
            let ledger_view = match state.seed_epoch_consensus_inputs.as_ref() {
                Some(record) => PoolDistrView::from_seed_epoch_consensus_inputs(record),
                None if live_feed_wired => {
                    return Err(NodeLifecycleError::FeedMissingRecoveredConsensusInputs)
                }
                None => PoolDistrView::new(
                    EpochNo(epoch.unwrap_or(0)),
                    0,
                    ActiveSlotsCoeff { numer: 0, denom: 1 },
                    BTreeMap::new(),
                ),
            };
            // Recovered-state lifetime: clone ledger + chain_dep into the relay
            // spine (the spine evolves ITS copy forward), keep `state` owned as
            // the recovered baseline the forge reads. One recovered state; the
            // forge base IS the spine base.
            // PHASE4-N-AE.C (DC-WAL-02): first followed AdmitBlock chains from the
            // fingerprint of the recovered ledger tip the follow extends (the
            // WAL-tail post_fp), not from zero — so a recover→followed store
            // warm-starts replay-equivalently (T-REC-05).
            let mut fwd = ForwardSyncState::new(
                ReceiveState::new(state.ledger.clone(), state.chain_dep.clone()),
                fingerprint(&state.ledger).combined,
                SnapshotCadence::DEFAULT,
            );
            // PHASE4-N-F-G-C S1: wire a LIVE WirePump feed when an upstream peer
            // is configured (`--peer`). Empty `--peer` keeps the prior empty
            // source (forge-CAPABLE, halts clean — the `On` arm is observable
            // only once a live feed is wired, RO-LIVE-01). The live source is a
            // *fill* of the closed `NodeBlockSource::WirePump` arm — no new
            // variant, no second tip-advance, no verdict; dial / parse failures
            // are logged-and-dropped (admission honest-scope C3), never fatal.
            let mut source = if live_feed_wired {
                let network_magic = cli
                    .network_magic
                    .ok_or(NodeLifecycleError::MissingFlag("--network-magic"))?;
                spawn_live_wire_pump_source(&cli.peer_addrs, network_magic, state.tip.as_ref())
            } else {
                NodeBlockSource::in_memory(Vec::new())
            };
            // The injected clock is the SOLE wall-clock observation (DC-NODE-03).
            let mut clock = SystemClock::new(slot_length_ms);
            // S2: protocol_version + pparams come from the recovered ledger's
            // current protocol_params (installed by S2a) — the single truthful
            // source, consumed here, never fabricated or re-derived.
            let (current_pparams, current_protocol_version) =
                forge_constants_from_pparams(&state.ledger.protocol_params);
            // PHASE4-N-U S3 (DC-NODE-13): node-spine serve-to-peer task reading
            // the DURABLE ChainDb projection. When `--listen` is set, bind the
            // serve listener (fail-fast on bind failure — no silent live-serve
            // claim) and spawn `run_node_serve_task` OUTSIDE `run_relay_loop`,
            // reading an Arc::clone of the durable ChainDb (serve-as-projection;
            // the G-R push sibling + accumulator are retired — own-forged blocks
            // are durably admitted via admit_forged_block_durably -> pump_block in
            // the ForgeTick arm, S1). Request-driven serve only (no `advance_tip`).
            // The serve task lifetime is owned by the node lifecycle owner (the
            // operator `shutdown` watch), NOT the feed loop (DC-NODE-09): a clean
            // feed-end halt must not tear down serving.
            let node_serve_handle = match cli.listen_addr.as_deref() {
                Some(listen) => {
                    // Serving a peer requires the network's magic (the serve
                    // listener advertises it via n2n_supported_for_magic, S2b);
                    // fail-fast if absent (no silent live-serve claim).
                    let serve_magic = cli
                        .network_magic
                        .ok_or(NodeLifecycleError::MissingFlag("--network-magic"))?;
                    let listener = bind_serve_listener(listen)
                        .await
                        .map_err(|e| NodeLifecycleError::ServeStart(format!("{e:?}")))?;
                    // DC-NODE-09: gate the serve task on the operator `shutdown` watch
                    // (a clone), never a feed-end-triggered stop. The serve listener
                    // stays available until explicit node shutdown, a fatal serve
                    // error, or lifecycle cancellation — so a peer that retries after
                    // the upstream feed ended can still BlockFetch a durable block.
                    // The serve task is READ-ONLY over the durable ChainDb (an
                    // Arc::clone); this grants availability, not authority.
                    let serve_chaindb: Arc<dyn ChainDb> = chaindb.clone();
                    let task = tokio::spawn(run_node_serve_task(
                        listener,
                        serve_chaindb,
                        serve_magic,
                        shutdown.clone(),
                    ));
                    Some(task)
                }
                None => None,
            };
            let mut activation = ForgeActivation::new(
                &mut clock,
                &coord_state,
                &state,
                &mut shell,
                pool_id,
                current_pparams,
                current_protocol_version,
                anchor_millis,
                start_slot,
                slot_length_ms,
            );
            // PHASE4-N-AF (DC-NODE-18): when the operator declares an explicitly
            // single-producer venue, enable extend-own-spine behind the fence.
            // Absent the flag, `venue_role` stays Unknown ⇒ pure DC-NODE-15.
            if cli.single_producer_venue {
                activation.declare_single_producer_venue();
                // PHASE4-N-AH S4b (DC-NODE-22): re-enter the extend state directly when
                // warm-start recovered a local durable continuation spine ABOVE the
                // replay anchor (the warm-start analog of DC-NODE-20) — so a restarted
                // single-producer node resumes forging on ChainDb::tip without a fresh
                // follow-link catch-up. Else (bare anchor / first-run / no summary) the
                // forge mode stays InitialCatchupRequired. Fail-closed; the per-tick
                // DC-NODE-20 fence + pump_block-sole-admit still gate every forge.
                let recovered_tip = ChainDbServedSource::new(&*chaindb)
                    .tip()
                    .map(|(slot, hash, block_no)| TipPoint {
                        slot,
                        hash,
                        block_no,
                    });
                activation.forge_mode = crate::node_sync::warm_start_forge_mode(
                    activation.venue_role,
                    recovered_tip.as_ref(),
                    state.replayed_anchor_block_no,
                );
            }
            // PHASE4-N-F-G-J S1 (CN-NODE-04): emit the closed feed/forge
            // scheduling diagnostics to stderr (emit-only; never alters
            // scheduling). The forge-on path the C1 rerun exercises —
            // forge_tick_skipped{reason} reveals the empty-feed halt.
            let sched_sink: Box<dyn std::io::Write> = match std::fs::File::create(&cli.log_path)
            {
                Ok(f) => Box::new(f),
                Err(_) => Box::new(std::io::stderr()),
            };
            let mut sched_log = crate::live_log::NodeSchedLogWriter::new(sched_sink);
            run_relay_loop_with_sched(
                &mut fwd,
                &mut source,
                &chaindb,
                &mut wal,
                &era_schedule,
                &ledger_view,
                shutdown,
                Some(&mut activation),
                Some(&mut sched_log),
            )
            .await?;
            // PHASE4-N-U S3: no handoff channel / push sibling to drain — the
            // serve task reads the durable ChainDb directly. Drop the forge
            // activation (releases its &mut borrows on clock/shell), then await
            // the serve task.
            drop(activation);
            // DC-NODE-09: do NOT stop the serve task at feed-end. Await it — it ends
            // ONLY when the operator `shutdown` watch flips (which `run_relay_loop`
            // also observed) or on a fatal serve error. On a clean feed-end halt with
            // `shutdown` still false, this keeps Ade reachable so a late peer can
            // BlockFetch a durable block from the served projection. The process
            // still always terminates: operator shutdown ends BOTH the relay loop
            // and the serve task.
            if let Some(handle) = node_serve_handle {
                let _ = handle.await;
            }
            // Honest record. PHASE4-N-F-G-C S1: with a LIVE feed wired (`--peer`)
            // the forge is observable when the feed is Continuing and a due
            // leader slot is reached; peer ACCEPT is NOT claimed here — it is
            // operator-gated (RO-LIVE-01/06), proven only by the peer's
            // validation log. With NO `--peer` the empty source halts before any
            // ForgeTick (forge-CAPABLE, not observable — RO-LIVE-01 follow-on).
            // Either way: NO peer-acceptance / BA-02 claim.
            if live_feed_wired {
                eprintln!(
                    "ade_node --mode node: relay run loop exited \
                     (recovered/bootstrapped epoch={epoch:?}, tip slot={tip_slot:?}; \
                     forge CAPABLE — operator keys loaded — LIVE WirePump feed wired \
                     to {peers:?}: forge is observable when the feed is Continuing and \
                     a due leader slot is reached. Peer ACCEPT is NOT claimed — it is \
                     operator-gated (RO-LIVE-01/06), proven only by the peer's \
                     validation log. NO peer-acceptance / BA-02 claim.",
                    peers = cli.peer_addrs
                );
            } else {
                eprintln!(
                    "ade_node --mode node: relay run loop entered and halted cleanly \
                     (recovered/bootstrapped epoch={epoch:?}, tip slot={tip_slot:?}; \
                     forge CAPABLE — operator keys loaded — but NOT observable: no \
                     --peer supplied, the empty source halts before any ForgeTick \
                     (RO-LIVE-01 follow-on). NO block served / admitted / gossiped; \
                     NO durable tip advanced."
                );
            }
        }
    }
    Ok(())
}

/// PHASE4-N-F-G-C S1: capacity of the live WirePump feed channel (bounded;
/// mirrors the admission-bootstrap precedent). The `WirePump` lookahead drains
/// it via `next_block`; back-pressure is bounded.
const LIVE_WIRE_PUMP_CHANNEL_CAP: usize = 64;

/// PHASE4-N-F-G-C S1: build a LIVE [`NodeBlockSource::WirePump`] from the
/// operator-supplied upstream peer(s). This is **RED wiring only** — it reuses
/// the closed admission dial + pump (`dial_for_admission` +
/// `run_admission_wire_pump`) VERBATIM (no reimplementation, no new wire
/// authority) and feeds their `ade_runtime::admission::AdmissionPeerEvent`
/// output DIRECTLY into the `WirePump` arm (the node spine consumes the runtime
/// event type — no bridge). The live source is a *fill* of the closed 2-variant
/// [`NodeBlockSource`] (no new variant), adds no second tip-advance path, and
/// carries no verdict.
///
/// Honest-scope (C3, mirrors `admission::bootstrap::spawn_wire_pumps_for_admission`):
/// an unparseable `--peer` addr or a `dial_for_admission` failure is
/// logged-and-dropped — never fatal, never a fabricated address, never a silent
/// tip graft. If no peer yields a live pump, the feed ends and the relay loop
/// halts clean (the same outcome as the empty source).
fn spawn_live_wire_pump_source(
    peer_addrs: &[String],
    network_magic: u32,
    recovered_tip: Option<&ChainTip>,
) -> NodeBlockSource {
    let our_versions = build_n2n_version_table(network_magic);
    let start_point = match recovered_tip {
        Some(t) => ade_network::codec::chain_sync::Point::Block {
            slot: t.slot,
            hash: t.hash.clone(),
        },
        None => ade_network::codec::chain_sync::Point::Origin,
    };
    let (events_tx, events_rx) = mpsc::channel::<AdmissionPeerEvent>(LIVE_WIRE_PUMP_CHANNEL_CAP);
    for raw_addr in peer_addrs {
        let addr: std::net::SocketAddr = match raw_addr.parse() {
            Ok(a) => a,
            Err(_) => {
                eprintln!("ade_node --mode node: skipping unparseable --peer addr {raw_addr}");
                continue;
            }
        };
        let pump_versions = our_versions.clone();
        let pump_tx = events_tx.clone();
        let start = start_point.clone();
        let label = raw_addr.clone();
        tokio::spawn(async move {
            let (transport, version) = match dial_for_admission(addr, pump_versions).await {
                Ok(pair) => pair,
                Err(e) => {
                    eprintln!("ade_node --mode node: dial-for-admission failed for {label}: {e:?}");
                    return;
                }
            };
            let _ =
                run_admission_wire_pump(transport, label, start, version, network_magic, pump_tx)
                    .await;
        });
    }
    // Drop the builder's own sender: the spawned tasks hold their own clones, so
    // the channel stays open while any pump runs and closes once they all end
    // (or immediately if no `--peer` parsed → feed ends → loop halts clean).
    drop(events_tx);
    NodeBlockSource::from_wire_pump(events_rx)
}

/// PHASE4-N-F-G-H S2: capacity of the node-spine serve event channel (inbound
/// `OrchestratorEvent`s from the per-peer sessions). Bounded back-pressure.
const NODE_SERVE_EVENT_CHANNEL_CAP: usize = 64;

/// PHASE4-N-F-G-H S2: closed serve-start failure surface. A bind failure under
/// `--listen` MUST be surfaced (no silent live-serve claim).
#[derive(Debug)]
pub enum ServeStartError {
    /// The `--listen` value did not parse as a socket address.
    InvalidAddr(String),
    /// Binding the serve listener failed (e.g. address already in use).
    Bind(std::io::ErrorKind),
}

/// PHASE4-N-F-G-H S2: bind the node-spine serve listener, surfacing a bind
/// failure explicitly. The On-arm fail-fasts on `Err` — the node never proceeds
/// claiming live-serve capability while serving is disabled. Returns the BOUND
/// listener so the caller knows the actual local address (an ephemeral `:0`
/// resolves to a real port) and the serve task binds exactly ONCE.
pub async fn bind_serve_listener(listen_addr: &str) -> Result<TcpListener, ServeStartError> {
    let addr: SocketAddr = listen_addr
        .parse()
        .map_err(|_| ServeStartError::InvalidAddr(listen_addr.to_string()))?;
    TcpListener::bind(addr)
        .await
        .map_err(|e| ServeStartError::Bind(e.kind()))
}

/// PHASE4-N-U S3 (DC-NODE-13): the node-spine serve task. REQUEST-DRIVEN serve of
/// the DURABLE adopted chain (a read-only projection of the durable ChainDb) to
/// real peers, run OUTSIDE `run_relay_loop` (a sibling). It accepts inbound peers
/// on the pre-bound `listener` — reusing the per-peer N2N session machinery
/// `run_per_peer_session` (handshake + mux + session) verbatim — and routes each
/// orchestrator event to the SINGLE shared serve-dispatch core (S1):
/// `PeerConnected { role: DownstreamServer }` -> `install_server_peer_state`;
/// `PeerDisconnected` -> `remove_server_peer_state`; server frames ->
/// `dispatch_server_frame_event_to_outbound` over `ServedChainSource::DurableChainDb`.
///
/// COORDINATOR-FREE: no `CoordinatorState`, no `coordinator_step`, no producer
/// evidence writer (those stay in `produce_mode`). REQUEST-DRIVEN ONLY: there is
/// NO proactive `producer_chain_sync_advance_tip` reactor — a follower's
/// `RequestNext` is answered with `RollForward` iff the block is already durable
/// at request time. Stops when `shutdown_rx` flips. The serve is READ-ONLY over
/// the durable ChainDb (it advances no tip, admits nothing); every byte served
/// traces to the validated durable admit (CN-CONS-07 serve clause). Supersedes
/// the G-R monotone-gated accumulator: the durable chain is extend-only, so it
/// is coherent and holds exactly one block 0 by construction, and serving
/// survives restart (the accumulator did not).
pub async fn run_node_serve_task(
    listener: TcpListener,
    serve_chaindb: Arc<dyn ChainDb>,
    network_magic: u32,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let (events_tx, mut events_rx) =
        mpsc::channel::<OrchestratorEvent>(NODE_SERVE_EVENT_CHANNEL_CAP);
    let peer_outbound = new_per_peer_outbound();
    let peer_id_generator = Arc::new(PeerIdGenerator::new());
    let mut peers_state: ServerPeerStates = BTreeMap::new();

    loop {
        tokio::select! {
            biased;
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    break;
                }
            }
            accept = listener.accept() => {
                let (stream, _addr) = match accept {
                    Ok(pair) => pair,
                    // A fatal accept error ends the serve sibling; the relay/sync
                    // spine is independent. (Bindability was already surfaced by
                    // `bind_serve_listener`; this is a post-bind accept fault.)
                    Err(_) => break,
                };
                let session_cfg = PerPeerSessionConfig {
                    stream,
                    our_supported: n2n_supported_for_magic(network_magic).into(),
                    peer_id_generator: peer_id_generator.clone(),
                    events_out: events_tx.clone(),
                    peer_outbound: Some(peer_outbound.clone()),
                };
                tokio::spawn(run_per_peer_session(session_cfg));
            }
            evt = events_rx.recv() => {
                let evt = match evt {
                    Some(e) => e,
                    None => break,
                };
                match &evt {
                    OrchestratorEvent::PeerConnected {
                        peer_id,
                        chain_sync_version,
                        block_fetch_version,
                        role: PeerRole::DownstreamServer,
                    } => {
                        install_server_peer_state(
                            &mut peers_state,
                            ServerPeerId(peer_id.0),
                            *chain_sync_version,
                            *block_fetch_version,
                        );
                    }
                    OrchestratorEvent::PeerDisconnected { peer_id, .. } => {
                        remove_server_peer_state(
                            &mut peers_state,
                            &peer_outbound,
                            ServerPeerId(peer_id.0),
                        )
                        .await;
                    }
                    OrchestratorEvent::PeerN2nServerChainSyncFrame { .. }
                    | OrchestratorEvent::PeerN2nServerBlockFetchFrame { .. } => {
                        // Request-driven serve over the SINGLE shared dispatch
                        // core, reading the durable ChainDb projection
                        // (DC-NODE-13). Dispatch errors drop the peer; never
                        // panic, never mutate authoritative state.
                        let _ = dispatch_server_frame_event_to_outbound(
                            &evt,
                            &mut peers_state,
                            ServedChainSource::DurableChainDb(serve_chaindb.as_ref()),
                            &peer_outbound,
                        )
                        .await;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// The RED relay run loop (PHASE4-N-F-D S2). Both `--mode node` lifecycle
/// arms converge here. Each iteration reads the three closed lifecycle inputs
/// (operator shutdown intent, momentary source readiness, structural feed
/// liveness), asks the GREEN [`plan_loop_step`] planner for the next step,
/// and performs exactly that step:
///
///   - `SyncOnce`  → one `run_node_sync` (the SOLE block-consumption path):
///     drains the currently-available batch through the single
///     `run_node_sync` → `pump_block` seam, durable-before-tip, capturing its
///     E4 checkpoint. The durable tip advances ONLY here (DC-SYNC-02). A
///     reject fails closed via [`NodeLifecycleError::RelaySync`] — never a
///     skip-past, never a fallback.
///   - `Idle`      → the SOLE inter-iteration await: wait for the next block
///     to become available OR a shutdown signal. Cancellation-safe — no
///     durable apply is in flight here.
///   - `HaltCleanly` → break at this boundary, on-disk state recoverable.
///
/// The loop owns NO authority (CN-NODE-02): it forges nothing, admits
/// nothing through a second path, derives no verdict, follows no peer, and
/// never advances the tip except through `run_node_sync`. `run_node_sync` is
/// **awaited to completion** inside `SyncOnce` and is NEVER placed inside the
/// shutdown `select!`, so it can never be cancelled between a durable apply
/// and its checkpoint.
/// Opt-in forge activation for the relay run loop (PHASE4-N-F-E S2).
///
/// When `run_relay_loop` is passed `Some(ForgeActivation)`, it attempts a
/// **self-accept-only** forge at each due, leader-eligible slot — advancing no
/// durable tip and serving / admitting / gossiping nothing. When passed `None`,
/// the loop is the exact N-F-D relay (forge off; `ForgeSlotStatus::NotDue`).
///
/// Constructed only by hermetic callers — `--mode node` performs NO operator-key
/// file/config ingestion (that is a separate RED key-ingress cluster). Every
/// field is an existing recovered / bootstrap / producer-shell input; nothing
/// here is a new semantic source.
pub struct ForgeActivation<'a> {
    /// Injected clock — the sole wall-clock observation. RED `now_millis` /
    /// `next_tick` is converted to a `SlotNo` via `millis_to_slot`; only the
    /// `SlotNo` crosses into the planner / forge call (clock seam, DC-NODE-03).
    pub clock: &'a mut dyn Clock,
    /// Genesis-anchor host for the REUSED `kes_period_for_slot` — no new GREEN
    /// helper, no slot->KES reimplementation.
    pub coordinator_state: &'a CoordinatorState,
    /// Recovered forge base — the SOLE leadership source, projected only inside
    /// the fenced `forge_one_from_recovered` (DC-CINPUT-02b / CN-CINPUT-03).
    pub recovered: &'a BootstrapState,
    /// Operator key custody (hermetic/fenced material only).
    pub shell: &'a mut ProducerShell,
    pub pool_id: Hash28,
    pub pparams: ProtocolParameters,
    pub protocol_version: ProtocolVersion,
    /// `millis_to_slot` anchor (SystemStart + era slot length).
    pub anchor_millis: u64,
    pub start_slot: SlotNo,
    pub slot_length_ms: u32,
    /// Monotonic forge-slot guard state — updated ONLY after an actual
    /// `forge_one_from_recovered` attempt (never on skip / forge-off).
    last_forged_slot: Option<SlotNo>,
    /// Slot derived this iteration; consumed by the `ForgeTick` arm and reset to
    /// `None` at the top of every iteration so a skipped / failed path can never
    /// forge for a stale slot.
    pending_slot: Option<SlotNo>,
    /// In-memory hermetic test observation ONLY. Not persisted, not logged, not
    /// replay authority, not BA-02 / RO-LIVE evidence.
    pub hermetic_forge_outcomes: Vec<CoordinatorEvent>,
    /// S3: the last clock→slot alignment fail-closed (set when the wall-clock is
    /// before the genesis anchor, cleared on a successful alignment). A structured
    /// LOCAL node-forge observation surface — in-memory, not persisted, not logged,
    /// not evidence — that makes the fail-closed visible (never a silent `NotDue`).
    pub last_slot_alignment_fail: Option<SlotAlignmentError>,
    /// PHASE4-N-AE.A (DC-NODE-15): the last forge-on-followed-tip refusal
    /// (`ForgeRefused::NotCaughtUp`), set when the admissibility gate prevented
    /// a forge (durable servable tip != followed peer tip) and cleared when a
    /// forge is admitted. A structured LOCAL observation surface carrying the
    /// observed tips + reason — in-memory, not persisted, not evidence — that
    /// makes the typed refusal visible (never a silent skip, never log-only).
    pub last_forge_refused: Option<ForgeRefused>,
    /// DC-NODE-18 (PHASE4-N-AF): the single-producer forge mode (RED scheduling
    /// state; NOT persisted, NOT replay-visible). Default `InitialCatchupRequired`.
    pub forge_mode: ForgeMode,
    /// DC-NODE-18: the declared venue role. Default `Unknown` — the extend gate
    /// fails closed, so a node that does NOT explicitly declare a single-producer
    /// venue forges EXACTLY as the prior DC-NODE-15-only path (no behavior change).
    pub venue_role: VenueRole,
}

impl<'a> ForgeActivation<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        clock: &'a mut dyn Clock,
        coordinator_state: &'a CoordinatorState,
        recovered: &'a BootstrapState,
        shell: &'a mut ProducerShell,
        pool_id: Hash28,
        pparams: ProtocolParameters,
        protocol_version: ProtocolVersion,
        anchor_millis: u64,
        start_slot: SlotNo,
        slot_length_ms: u32,
    ) -> Self {
        Self {
            clock,
            coordinator_state,
            recovered,
            shell,
            pool_id,
            pparams,
            protocol_version,
            anchor_millis,
            start_slot,
            slot_length_ms,
            last_forged_slot: None,
            pending_slot: None,
            hermetic_forge_outcomes: Vec::new(),
            last_slot_alignment_fail: None,
            last_forge_refused: None,
            forge_mode: ForgeMode::InitialCatchupRequired,
            venue_role: VenueRole::Unknown,
        }
    }

    /// DC-NODE-18: declare this an explicitly single-producer venue (relay
    /// non-producing, Ade sole producer), enabling extend-own-spine behind the
    /// fail-closed fence. If un-called, `venue_role` stays `Unknown` ⇒ the extend
    /// path never activates and the forge stays pure DC-NODE-15. (DC-NODE-21: the
    /// adoption certificate is NOT a forge input — the harness owns it as evidence.)
    pub fn declare_single_producer_venue(&mut self) {
        self.venue_role = VenueRole::SingleProducer;
    }
}

// DC-NODE-21 (PHASE4-N-AH S2): the adoption-certificate parser is REMOVED from
// ade_node entirely — the operator harness owns cert/evidence parsing outside the
// forge loop. The cert is never a forge input (DC-NODE-20: the forge base is
// ChainDb::tip).

/// DC-NODE-15 forge-on-followed-tip refusal, factored so the DC-NODE-18
/// `UseInitialCatchupGate` path and the default (non-single-producer) path share ONE
/// gate. `None` ⇒ admissible (caught up, or cold-start).
fn dc_node_15_refusal(
    is_cold_start: bool,
    durable_servable_tip: &Option<TipPoint>,
    followed_peer_tip: &Option<TipPoint>,
) -> Option<ForgeRefused> {
    if is_cold_start {
        return None;
    }
    match forge_followed_tip_admission(durable_servable_tip.clone(), followed_peer_tip.clone()) {
        ForgeFollowedTipAdmission::CaughtUp => None,
        ForgeFollowedTipAdmission::NotCaughtUp { reason } => Some(ForgeRefused::NotCaughtUp {
            local_servable_tip: durable_servable_tip.clone(),
            followed_peer_tip: followed_peer_tip.clone(),
            reason,
        }),
    }
}

/// S2: derive the forge's current `protocol_version` + `pparams` from the
/// recovered ledger's `protocol_params` (installed by S2a) — the single truthful
/// source, consumed here, never a fabricated default / genesis-initial value.
pub(crate) fn forge_constants_from_pparams(
    pp: &ProtocolParameters,
) -> (ProtocolParameters, ProtocolVersion) {
    (
        pp.clone(),
        ProtocolVersion {
            major: pp.protocol_major as u64,
            minor: pp.protocol_minor as u64,
        },
    )
}

#[allow(clippy::too_many_arguments)]
/// Relay loop with NO diagnostic sink (the existing 8-arg API — hermetic tests
/// and any caller that does not emit CN-NODE-04 events). Delegates to
/// [`run_relay_loop_with_sched`] with `sched = None`; the scheduling is
/// identical (the sink is emit-only and never alters control flow).
pub async fn run_relay_loop(
    state: &mut ForwardSyncState,
    source: &mut NodeBlockSource,
    chaindb: &PersistentChainDb,
    wal: &mut FileWalStore,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    shutdown: &mut watch::Receiver<bool>,
    forge: Option<&mut ForgeActivation<'_>>,
) -> Result<(), NodeLifecycleError> {
    run_relay_loop_with_sched(
        state, source, chaindb, wal, era_schedule, ledger_view, shutdown, forge, None,
    )
    .await
}

/// Map the GREEN `node_sync::ForgeMode` state to the closed diagnostic
/// `live_log::ForgeModeKind` for the RED sched transcript (CN-NODE-04 / DC-NODE-20
/// evidence). Emit-only projection; never read back into any authority path.
fn forge_mode_kind(m: &ForgeMode) -> crate::live_log::ForgeModeKind {
    use crate::live_log::ForgeModeKind;
    match m {
        ForgeMode::InitialCatchupRequired => ForgeModeKind::InitialCatchupRequired,
        ForgeMode::CaughtUpToPeerTip { .. } => ForgeModeKind::CaughtUpToPeerTip,
        ForgeMode::SingleProducerExtendOwnDurableSpine { .. } => {
            ForgeModeKind::SingleProducerExtendOwnDurableSpine
        }
    }
}

/// Relay loop with an optional emit-only CN-NODE-04 diagnostic sink
/// (PHASE4-N-F-G-J S1). The binary `--mode node` path passes a real sink; the
/// sink is best-effort and NEVER alters the loop's scheduling / control flow,
/// and the GREEN planner never reads an event (emit-only).
pub async fn run_relay_loop_with_sched(
    state: &mut ForwardSyncState,
    source: &mut NodeBlockSource,
    chaindb: &PersistentChainDb,
    wal: &mut FileWalStore,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    shutdown: &mut watch::Receiver<bool>,
    mut forge: Option<&mut ForgeActivation<'_>>,
    mut sched: Option<&mut dyn crate::live_log::NodeSchedSink>,
) -> Result<(), NodeLifecycleError> {
    loop {
        let shutdown_status = if *shutdown.borrow() {
            ShutdownStatus::ShutdownRequested
        } else {
            ShutdownStatus::Running
        };
        let sync_status = if source.has_work_ready() {
            SyncStatus::WorkAvailable
        } else {
            SyncStatus::NoWorkReady
        };
        let loop_state = if source.is_ended() {
            LoopState::Ending
        } else {
            LoopState::Continuing
        };
        // PHASE4-N-F-E S2: forge-slot scheduling. RED observes the injected
        // clock; only the derived `SlotNo` crosses into the GREEN monotonic
        // guard + planner. Forge OFF (`None`) => always `NotDue` => the planner
        // reduces to the exact N-F-D relay mapping (no `ForgeTick`).
        let forge_slot = match forge.as_deref_mut() {
            None => ForgeSlotStatus::NotDue,
            Some(act) => {
                act.pending_slot = None; // reset so a stale slot can never forge
                match act.clock.next_tick() {
                    Some(now_ms) => match checked_millis_to_slot(
                        now_ms,
                        act.anchor_millis,
                        act.start_slot,
                        act.slot_length_ms,
                    ) {
                        Ok(slot) => {
                            act.last_slot_alignment_fail = None;
                            act.pending_slot = Some(slot);
                            forge_slot_status(act.last_forged_slot, slot)
                        }
                        // S3 (CE-G-A-3): an implausible clock→slot alignment (the
                        // wall-clock is before the genesis anchor) FAILS CLOSED at
                        // the RED clock seam — no forge, no `last_forged_slot`
                        // advance, `pending_slot` stays None; surfaced as a
                        // structured local outcome (`last_slot_alignment_fail`).
                        // NotDue to the planner; the relay loop keeps syncing
                        // (forge stays subordinate to the sync spine, DC-NODE-05).
                        Err(e) => {
                            act.last_slot_alignment_fail = Some(e);
                            ForgeSlotStatus::NotDue
                        }
                    },
                    // Clock exhausted — no more forge slots scheduled.
                    None => ForgeSlotStatus::NotDue,
                }
            }
        };
        // PHASE4-N-F-G-J S1: was a forge slot due THIS iteration? Captured before
        // the (unchanged) planner call so the HaltCleanly arm can emit the
        // forge_tick_skipped diagnostic without consulting the planner output.
        let forge_was_due = matches!(forge_slot, ForgeSlotStatus::Due);
        // PHASE4-N-AG S2 (DC-NODE-19): a certified single-producer venue in the
        // extend state continues forging past a structural feed EOF; every other
        // venue (incl. forge-off / relay-only) keeps the verbatim HaltOnFeedEnd
        // behavior. `policy` is a content-blind projection of (venue_role,
        // forge_mode) — the planner never sees the venue/mode details.
        let policy = match forge.as_deref() {
            Some(act) => venue_policy(act.venue_role, &act.forge_mode),
            None => VenuePolicy::HaltOnFeedEnd,
        };
        match plan_loop_step(loop_state, sync_status, forge_slot, shutdown_status, policy) {
            LoopStep::SyncOnce => {
                run_node_sync(source, state, chaindb, wal, era_schedule, ledger_view)
                    .await
                    .map_err(|e| NodeLifecycleError::RelaySync(format!("{e:?}")))?;
            }
            LoopStep::ForgeTick => {
                if let Some(s) = sched.as_deref_mut() {
                    s.record(&crate::live_log::NodeSchedEvent::ForgeTickConsidered);
                }
                // ForgeTick is reachable only with forge active (the planner can
                // never return it for `NotDue`). Exactly ONE fenced forge attempt;
                // advances NO durable tip, serves / admits / gossips nothing.
                let act = forge
                    .as_deref_mut()
                    .expect("ForgeTick implies forge activation present");
                let slot = act
                    .pending_slot
                    .expect("ForgeTick implies a derived forge slot");
                // KES period via the REUSED CoordinatorState method (no
                // reimplementation). Out of range => skip: no forge, no
                // `last_forged_slot` update (S3b proves the fail-closed path).
                let mut forged = false;
                if let Some(kes_period) = act.coordinator_state.kes_period_for_slot(slot.0) {
                    // PHASE4-N-AE.A (DC-NODE-15): the forge base is the DURABLE
                    // servable tip — `ChainDb::tip()`. The recovered snapshot
                    // anchor is NEVER a forge base (the `recovered.tip` fallback
                    // is removed): a forge must build only on a StoredBlock a peer
                    // can FindIntersect. Read-only — the forge never writes it.
                    let selected_tip = ChainDb::tip(chaindb)
                        .map_err(|e| NodeLifecycleError::RelaySync(format!("{e:?}")))?;
                    // DC-NODE-15 admissibility inputs: the durable servable tip a
                    // peer would see (the serve PROJECTION's tip — slot, hash, AND
                    // block_no), and the followed peer tip the wire stream observed
                    // (a separate structured admissibility input, NOT a sync tip
                    // authority). The peer-tip signal may only PREVENT a forge.
                    let durable_servable_tip: Option<TipPoint> = ChainDbServedSource::new(chaindb)
                        .tip()
                        .map(|(slot, hash, block_no)| TipPoint {
                            slot,
                            hash,
                            block_no,
                        });
                    let followed_peer_tip = source.followed_peer_tip_signal().tip();
                    // S4 (DC-NODE-08): the from-genesis cold-start (block 0 +
                    // PrevHash::Genesis) is a distinct path, UPSTREAM of the
                    // followed-tip gate. It applies ONLY when there is no durable
                    // tip AND the node did NOT recover at a non-Origin anchor
                    // (`recovered.tip` is None ⇒ genesis), the recovered
                    // seed-epoch lineage is present, and the feed is forge-eligible
                    // (CN-NODE-04: no_block_available | clean_empty). A node that
                    // recovered at a non-Origin anchor is NEVER cold-started — it
                    // takes the DC-NODE-15 gate and waits to be caught up.
                    let is_from_genesis_cold_start =
                        selected_tip.is_none() && act.recovered.tip.is_none();
                    let cold_start_permitted = is_from_genesis_cold_start
                        && may_cold_start_forge(
                            false,
                            act.recovered.seed_epoch_consensus_inputs.is_some(),
                            source.feed_reason().is_forge_eligible(),
                        );
                    // PHASE4-N-AE.A (DC-NODE-15): on the recovered/following path,
                    // the forge is admissible ONLY when `durable_servable_tip ==
                    // followed_peer_tip` (hash AND block_no); otherwise it fails
                    // closed to a typed `ForgeRefused::NotCaughtUp`. The cold-start
                    // path is ungated (its parent is Genesis, intersectable via
                    // Origin). A `Refused` is NO forge, NO state transition, tip
                    // unchanged — the typed refusal is recorded (never log-only).
                    // DC-NODE-18 (PHASE4-N-AF) mode-aware forge gate. The DEFAULT
                    // venue (`Unknown`) takes the pure DC-NODE-15 path — EXACTLY the
                    // prior behavior (no change). An explicitly declared
                    // single-producer venue walks the `ForgeMode` state machine:
                    // initial catch-up via DC-NODE-15, then — once the relay has
                    // adopted the first successor (proved by an explicit RED
                    // certificate, NEVER inferred from self-admit) — it extends its
                    // OWN durable spine. A refuse/await is NO forge, NO state
                    // transition, tip unchanged; the typed refusal is recorded (never
                    // log-only). `proceed_to_forge` is a per-tick control flag, NOT
                    // the mode (the mode is the `ForgeMode` enum on `act`).
                    // DC-NODE-19 (S2) — certified-run fence, condition 7: on a
                    // CONTINUATION tick (the follow-link feed has EOF'd ⇒ loop_state
                    // == Ending) the extend forge requires the venue-adoption
                    // certificate to remain present + well-formed; absent/malformed ⇒
                    // fail closed (no continuation), recorded as a typed fence
                    // violation. The pre-EOF (Continuing) path is unchanged.
                    // DC-NODE-20: the forge base is Ade's own local durable spine head
                    // (ChainDb::tip). The adoption certificate is NOT read into the forge
                    // path -- it is evidence-only (DC-NODE-21). A feed-EOF continuation in
                    // the extend state no longer requires a cert (DC-NODE-19 continue-past-
                    // EOF core preserved; its cert-fence clause superseded by DC-NODE-20).
                    let proceed_to_forge: bool = if act.venue_role == VenueRole::SingleProducer {
                        match single_producer_forge_decision(
                            &act.forge_mode,
                            durable_servable_tip.clone(),
                            followed_peer_tip.clone(),
                            followed_peer_tip.clone(),
                            act.venue_role,
                            false,
                            false,
                        ) {
                            // ExtendOwnSpine forges on the durable spine head. The
                            // GREEN fence already required durable_servable_tip ==
                            // current_tip (the forge_base it returns), and the forge
                            // below builds on `selected_tip` (ChainDb::tip) — the SAME
                            // durable head — so the forge base stays BLUE-sourced and
                            // byte-equals forge_base (DC-CONS-24); the payload is not
                            // re-threaded because the base is read fresh from the tip.
                            SingleProducerForgeDecision::ExtendOwnSpine { .. } => true,
                            SingleProducerForgeDecision::Refuse(refused) => {
                                act.last_forge_refused = Some(refused);
                                false
                            }
                            SingleProducerForgeDecision::UseInitialCatchupGate => {
                                match dc_node_15_refusal(
                                    is_from_genesis_cold_start,
                                    &durable_servable_tip,
                                    &followed_peer_tip,
                                ) {
                                    Some(refused) => {
                                        act.last_forge_refused = Some(refused);
                                        false
                                    }
                                    None => {
                                        // Caught up (or cold-start): advance the mode
                                        // to CaughtUpToPeerTip when a real peer tip is
                                        // present (the on-caughtup transition).
                                        if let Some(pt) = followed_peer_tip.clone() {
                                            act.forge_mode =
                                                forge_mode_on_caughtup(&act.forge_mode, pt);
                                        }
                                        true
                                    }
                                }
                            }
                        }
                    } else {
                        // Default (non-single-producer) venue — pure DC-NODE-15.
                        match dc_node_15_refusal(
                            is_from_genesis_cold_start,
                            &durable_servable_tip,
                            &followed_peer_tip,
                        ) {
                            Some(refused) => {
                                act.last_forge_refused = Some(refused);
                                false
                            }
                            None => true,
                        }
                    };
                    if proceed_to_forge && (cold_start_permitted || selected_tip.is_some()) {
                        // DC-NODE-20 forge-base evidence (RED, emit-only): in a
                        // single-producer venue the forge base is the local selected
                        // durable tip (`selected_tip` == ChainDb::tip) — NOT the followed
                        // peer tip and NOT a cert. Serializes the decision already made.
                        if act.venue_role == VenueRole::SingleProducer {
                            // The forge base == the local durable ChainDb tip (block_no
                            // carried by ChainDbServedSource; `selected_tip`/ChainTip has
                            // only slot+hash). Same tip, just enriched for the transcript.
                            if let Some((_, base_hash, base_block_no)) =
                                ChainDbServedSource::new(chaindb).tip()
                            {
                                if let Some(s) = sched.as_deref_mut() {
                                    s.record(&crate::live_log::NodeSchedEvent::ForgeBaseSelected {
                                        forge_mode: forge_mode_kind(&act.forge_mode),
                                        forge_base_source:
                                            crate::live_log::ForgeBaseSource::LocalChaindbTip,
                                        forge_base_hash: base_hash,
                                        forge_base_block_no: base_block_no,
                                        followed_peer_tip_block_no: followed_peer_tip
                                            .as_ref()
                                            .map(|t| t.block_no),
                                        followed_peer_tip_hash: followed_peer_tip
                                            .as_ref()
                                            .map(|t| t.hash.clone()),
                                        cert_path_present: false,
                                    });
                                }
                            }
                        }
                        if let Some(s) = sched.as_deref_mut() {
                            s.record(&crate::live_log::NodeSchedEvent::ForgeAttempted);
                        }
                        // The single fenced forge attempt, mapped to the closed
                        // NodeForgeOutcome. CaughtUp ⇒ forge on the durable
                        // servable tip (`selected_tip`, which byte-equals the
                        // followed peer tip — DC-CONS-24); cold-start ⇒ the
                        // genesis-successor (selected_tip None ⇒ block 0 +
                        // PrevHash::Genesis, assembled inside the forge call). The
                        // forge call only ever produces Forged / Failed — the
                        // Refused state is the gate's exclusive output (handled
                        // above), so it cannot arise here.
                        let outcome = match forge_one_from_recovered(
                            act.recovered,
                            &state.receive.chain_dep,
                            &state.receive.ledger,
                            selected_tip.as_ref(),
                            act.shell,
                            &act.pool_id,
                            &act.pparams,
                            era_schedule,
                            slot.0,
                            kes_period,
                            act.protocol_version,
                        ) {
                            Ok((event, handoff)) => NodeForgeOutcome::Forged(event, handoff),
                            Err(e) => NodeForgeOutcome::Failed(e),
                        };
                        match outcome {
                            // Failed = the forge path was attempted and failed;
                            // propagate fail-fast (a real invariant/IO failure in
                            // this single-threaded loop). Mechanically DISTINCT
                            // from Refused (gate-prevented, no transition).
                            NodeForgeOutcome::Failed(e) => {
                                return Err(NodeLifecycleError::RelaySync(format!("{e:?}")));
                            }
                            // Refused never originates from the forge call.
                            NodeForgeOutcome::Refused(refused) => {
                                act.last_forge_refused = Some(refused);
                            }
                            NodeForgeOutcome::Forged(event, handoff) => {
                                // PHASE4-N-U S1 (DC-NODE-12): a self-accepted forged
                                // block becomes durable ONLY by submission to the
                                // SAME pump_block chokepoint received blocks use
                                // (durable-before-tip; the forge advances no tip
                                // directly), so the durable tip advances and the
                                // next ForgeTick builds N+1 (state.receive + the
                                // durable ChainDb advance together via pump_block). A
                                // stale-tip forge fails closed inside pump_block
                                // (extend-only block_validity / prior_fp —
                                // DC-CONS-23); in this single-threaded loop the forge
                                // always builds on the tip it just read, so a reject
                                // is a real invariant/IO failure and is propagated
                                // (fail-fast). PHASE4-N-U S3: there is no separate
                                // serve handoff — the durable block this admits IS
                                // what the serve task projects (serve-as-projection,
                                // DC-NODE-13); the G-R push sibling is retired.
                                // DC-NODE-18: capture whether an ACTUAL block was
                                // admitted (handoff present). A not_leader / no-op
                                // tick sets `forged = true` but admits nothing, and
                                // MUST NOT advance the single-producer mode.
                                let admitted = handoff.is_some();
                                if let Some(h) = handoff {
                                    admit_forged_block_durably(
                                        &h,
                                        state,
                                        chaindb,
                                        wal,
                                        era_schedule,
                                        ledger_view,
                                    )
                                    .map_err(|e| NodeLifecycleError::RelaySync(format!("{e:?}")))?;
                                }
                                // Closed diagnostic projection of the reused forge
                                // outcome, read before the move-push. Operational
                                // tier — never an acceptance / BA-02 signal.
                                let forge_outcome = forge_outcome_of(&event);
                                // A forge was admitted: clear any stale refusal.
                                act.last_forge_refused = None;
                                // Local hermetic observation only — never persisted
                                // / served / admitted / applied; the durable tip is
                                // untouched by this arm. `last_forged_slot` advances
                                // ONLY here, after an actual attempt.
                                act.hermetic_forge_outcomes.push(event);
                                act.last_forged_slot = Some(slot);
                                forged = true;
                                // DC-NODE-18: advance the single-producer forge mode
                                // ONLY after an actual forge+admit (`admitted`) --
                                // admissibility SCHEDULING only (the durable surface
                                // above is untouched; a no-op in a non-single-producer
                                // venue and on a not_leader tick). `own_tip` is the
                                // durable spine head just admitted.
                                if act.venue_role == VenueRole::SingleProducer {
                                    let own_tip = ChainDbServedSource::new(chaindb).tip().map(
                                        |(slot, hash, block_no)| TipPoint {
                                            slot,
                                            hash,
                                            block_no,
                                        },
                                    );
                                    act.forge_mode = forge_mode_after_admit(
                                        &act.forge_mode,
                                        admitted,
                                        own_tip,
                                        followed_peer_tip.clone(),
                                    );
                                }
                                if let Some(s) = sched.as_deref_mut() {
                                    s.record(&crate::live_log::NodeSchedEvent::ForgeResult {
                                        outcome: forge_outcome,
                                        self_admit_via_pump_block: admitted,
                                        entered_forge_mode: forge_mode_kind(&act.forge_mode),
                                    });
                                }
                            }
                        }
                    }
                }
                if !forged {
                    // Considered, but no forge ran (KES period out of range or no
                    // selected tip) — the closed off-tip skip outcome.
                    if let Some(s) = sched.as_deref_mut() {
                        s.record(&crate::live_log::NodeSchedEvent::ForgeResult {
                            outcome: crate::live_log::ForgeOutcome::NoTipAvailable,
                            self_admit_via_pump_block: false,
                            entered_forge_mode: forge_mode_kind(&act.forge_mode),
                        });
                    }
                }
            }
            LoopStep::Idle => {
                if let Some(s) = sched.as_deref_mut() {
                    s.record(&crate::live_log::NodeSchedEvent::FeedUnavailable {
                        reason: source.feed_reason(),
                    });
                }
                // DC-NODE-19 (S2): in continue-mode the feed has EOF'd —
                // `LoopState::Ending` is only reachable here under
                // `ContinueInSingleProducerExtend` (HaltOnFeedEnd + Ending =>
                // HaltCleanly, never Idle). The dead feed's `wait_ready` would park
                // forever and starve the forge cadence, so wake on the slot-cadence
                // timer or shutdown instead. A live (Continuing) feed keeps the
                // feed-driven wait. Outputs stay deterministic under the injected
                // clock schedule (the sleep paces; the clock decides slots).
                match loop_state {
                    LoopState::Ending => {
                        let poll = std::time::Duration::from_millis(
                            forge
                                .as_deref()
                                .map(|a| u64::from(a.slot_length_ms))
                                .unwrap_or(1_000),
                        );
                        tokio::select! {
                            _ = tokio::time::sleep(poll) => {}
                            _ = shutdown.changed() => {}
                        }
                    }
                    LoopState::Continuing => {
                        tokio::select! {
                            _ = source.wait_ready() => {}
                            _ = shutdown.changed() => {}
                        }
                    }
                }
            }
            LoopStep::HaltCleanly => {
                // PHASE4-N-F-G-J S1: the diagnostic that reveals the C1 skip — a
                // forge slot was due but the (terminal) feed-end made the planner
                // halt. `forge_tick_skipped{reason}` carries the closed feed-state
                // classification (fail-closed `unknown_disconnected` for a
                // reason-less WirePump end); otherwise the plain feed_unavailable.
                if let Some(s) = sched.as_deref_mut() {
                    let reason = source.feed_reason();
                    if forge_was_due {
                        s.record(&crate::live_log::NodeSchedEvent::ForgeTickSkipped { reason });
                    } else {
                        s.record(&crate::live_log::NodeSchedEvent::FeedUnavailable { reason });
                    }
                }
                break;
            }
        }
    }
    Ok(())
}

/// Closed diagnostic projection of the reused forge `CoordinatorEvent` outcome
/// (PHASE4-N-F-G-J S1, CN-NODE-04). Operational tier — never an acceptance /
/// BA-02 signal. An unexpected non-forge variant from the forge path maps to
/// `Failed` (defensive).
fn forge_outcome_of(ev: &CoordinatorEvent) -> crate::live_log::ForgeOutcome {
    use crate::live_log::ForgeOutcome;
    match ev {
        CoordinatorEvent::ForgeSucceeded { .. } => ForgeOutcome::Succeeded,
        CoordinatorEvent::ForgeNotLeader { .. } => ForgeOutcome::NotLeader,
        _ => ForgeOutcome::Failed,
    }
}

/// WarmStart arm — production warm-start recovery (L3).
///
/// Reconstructs the verified recovered `BootstrapState` (including the
/// recovered `SeedEpochConsensusInputs`) from on-disk state alone:
///
///   1. **W2 discovery (independent of the WAL):** enumerate the anchor
///      fingerprints persisted in the sidecar table
///      (`list_seed_epoch_consensus_anchor_fps`). The sidecar table key is a
///      source structurally independent of the WAL provenance entry — so
///      using it as the replay anchor keeps the anchor-mismatch check
///      non-circular. Require exactly one lineage; zero or many ⇒ fail closed.
///   2. **WAL replay:** `read_all` → `replay_from_anchor(anchor_fp, …)`. The
///      replay validates that the WAL `SeedEpochConsensusInputsImported`
///      entry's own `anchor_fp` equals the independent `anchor_fp` from (1).
///      No provenance recovered ⇒ fail closed.
///   3. **Single authority:** `bootstrap_initial_state` with
///      `RequiredFromRecoveredProvenance` runs the fail-closed verify chain
///      (sidecar present → `blake2b_256` hash == provenance → A1 decode →
///      anchor/epoch binding → byte-identity re-encode). NO bundle fallback.
///
/// PHASE4-N-U S2: forward-replay IS supported. `bootstrap_initial_state`'s
/// warm-start branch forward-replays from the nearest snapshot ≤ the
/// (reconciled) tip over the preserved bytes — so a forged tip (which carries
/// no snapshot-at-tip; S1 captures none, recovery is via WAL replay) recovers.
/// The `era_schedule` / `ledger_view` the fold consumes are reconstructed from
/// the recovered seed-epoch sidecar (NOT placeholders). Before warm-start the
/// chaindb is reconciled to the WAL tail (DC-WAL-04 no-orphan), and after, the
/// recovered fingerprint is checked against the WAL-tail post_fp (T-REC-05,
/// fail-fast on divergence). From-genesis single-Conway-era era_schedule
/// reconstruction (the genesis seed epoch ⇒ (0,0)); non-genesis multi-era is a
/// separate concern (S2 §15 non-goal).
///
/// `wal` is read-only here (`read_all` takes `&self`); recovery appends
/// nothing. `pub(crate)` so the L4 sync driver's kill→recover proof
/// (`node_sync` tests) can round-trip a synced tip through the real
/// recovery path; not exported outside the crate.
pub(crate) fn warm_start_recovery(
    chaindb: &PersistentChainDb,
    wal: &FileWalStore,
) -> Result<BootstrapState, NodeLifecycleError> {
    // 1. W2 discovery: the independent anchor lineage(s) from the sidecar
    //    table key. Discovery ONLY — the verify chain below is the authority.
    let anchor_fps = SnapshotStore::list_seed_epoch_consensus_anchor_fps(chaindb)
        .map_err(|e| NodeLifecycleError::OnDiskRead(format!("{e:?}")))?;
    let anchor_fp = match anchor_fps.as_slice() {
        [single] => single.clone(),
        [] => return Err(NodeLifecycleError::WarmStartNoAnchorLineage),
        _ => {
            return Err(NodeLifecycleError::WarmStartMultipleAnchorLineages {
                count: anchor_fps.len(),
            })
        }
    };

    // 2. Replay the WAL from the INDEPENDENT anchor_fp. Once L4b's durable
    //    apply has appended `AdmitBlock` entries, `replay_from_anchor`
    //    requires the preserved block bytes for each one (it fails closed
    //    with `BlockBytesMissing` otherwise). Build that map from the
    //    persistent ChainDb, exactly as the test/capability
    //    `recover_node_state` does (RED driver supplying preserved bytes;
    //    no BLUE replay change). A seed-epoch-only store (L2 first run,
    //    pre-sync) has zero `AdmitBlock` entries, so the map is empty and
    //    replay still passes.
    let entries = wal
        .read_all()
        .map_err(|e| NodeLifecycleError::WarmStartWalReplay(format!("{e:?}")))?;
    let mut block_bytes: BTreeMap<Hash32, Vec<u8>> = BTreeMap::new();
    for entry in &entries {
        // Only `AdmitBlock` entries reference preserved block bytes;
        // `SeedEpochConsensusInputsImported` (A3a) entries carry no block
        // hash and are skipped.
        if let ade_ledger::wal::WalEntry::AdmitBlock { block_hash, .. } = entry {
            if let Some(stored) = ChainDb::get_block_by_hash(chaindb, block_hash)
                .map_err(|e| NodeLifecycleError::OnDiskRead(format!("{e:?}")))?
            {
                block_bytes.insert(block_hash.clone(), stored.bytes);
            }
        }
    }
    let replay = replay_from_anchor(&anchor_fp, &entries, &block_bytes)
        .map_err(|e| NodeLifecycleError::WarmStartWalReplay(format!("{e:?}")))?;
    let provenance = replay
        .provenance
        .ok_or(NodeLifecycleError::WarmStartNoProvenance)?;
    let wal_tail_fp = replay.tail_fp.clone();
    let admit_count = replay.admit_count;

    // 3. PHASE4-N-U S2: reconstruct the recovery era_schedule + ledger_view from
    //    the recovered seed-epoch sidecar (replacing the L3 snapshot-at-tip-only
    //    placeholders), so bootstrap_initial_state's warm-start branch can
    //    FORWARD-REPLAY from a snapshot strictly below the tip. A forged tip (S1)
    //    carries NO snapshot-at-tip; it is recovered by WAL replay over the
    //    durable blocks. The sidecar is durable in the anchor-fp-keyed table.
    let sidecar_bytes = SnapshotStore::get_seed_epoch_consensus_inputs(chaindb, &anchor_fp)
        .map_err(|e| NodeLifecycleError::OnDiskRead(format!("{e:?}")))?
        .ok_or(NodeLifecycleError::WarmStartNoProvenance)?;
    let sidecar = decode_seed_epoch_consensus_inputs(&sidecar_bytes)
        .map_err(|e| NodeLifecycleError::WarmStartBootstrap(format!("sidecar decode: {e:?}")))?;
    let ledger_view = PoolDistrView::from_seed_epoch_consensus_inputs(&sidecar);
    // From-genesis single-Conway-era: the seed epoch starts at
    // epoch_no * epoch_length_slots (the genesis seed epoch ⇒ (0, 0), matching
    // the live WarmStart arm's make_node_schedule). Non-genesis multi-era
    // reconstruction is a separate concern (S2 §15 non-goal).
    let era_schedule =
        make_node_schedule(SlotNo(sidecar.epoch_no.0 * 432_000), sidecar.epoch_no);

    // 4. PHASE4-N-U S2 (DC-WAL-04 no-orphan): reconcile the chaindb to the WAL
    //    tail BEFORE warm-start. The WAL — not chaindb.tip() — is the admission
    //    authority; a torn StoreBlockBytes-before-AppendWal crash leaves an
    //    orphan block durable in the chaindb but absent from the WAL. Drop every
    //    block above the WAL-tail slot (deterministic, idempotent; empty WAL ⇒
    //    slot 0). Mirrors recover_node_state.
    let wal_tail_slot = entries
        .iter()
        .rev()
        .find_map(|entry| match entry {
            ade_ledger::wal::WalEntry::AdmitBlock { slot, .. } => Some(*slot),
            ade_ledger::wal::WalEntry::SeedEpochConsensusInputsImported { .. } => None,
            // PHASE4-N-AI AI-S1: a RollBack is not an AdmitBlock and
            // does not define the WAL-tail slot. No RollBack entries are
            // produced until AI-S3 makes recovery rollback-aware.
            ade_ledger::wal::WalEntry::RollBack { .. } => None,
        })
        .unwrap_or(SlotNo(0));
    chaindb
        .rollback_to_slot(wal_tail_slot)
        .map_err(|e| NodeLifecycleError::OnDiskRead(format!("rollback_to_slot: {e:?}")))?;

    // 5. The single authority. RequiredFromRecoveredProvenance runs the
    //    fail-closed sidecar verify chain; its warm-start branch forward-replays
    //    from the nearest snapshot ≤ the (reconciled) tip over the preserved
    //    bytes (the SOLE consumer of era_schedule / ledger_view).
    let mut recovered = bootstrap_initial_state(BootstrapInputs {
        chaindb,
        snapshot_store: chaindb,
        era_schedule: &era_schedule,
        ledger_view: &ledger_view,
        genesis_initial: None,
        seed_epoch_consensus_source: SeedEpochConsensusSource::RequiredFromRecoveredProvenance(
            provenance,
        ),
    })
    .map_err(|e| NodeLifecycleError::WarmStartBootstrap(format!("{e:?}")))?;

    // 6. PHASE4-N-U S2 (T-REC-05): the recovered ledger fingerprint MUST equal
    //    the WAL-tail post_fp (when ≥1 AdmitBlock) — a deterministic fail-fast,
    //    never a silent recovery divergence (the WAL is the admission authority).
    if admit_count > 0 {
        let recovered_fp = fingerprint(&recovered.ledger).combined;
        if recovered_fp != wal_tail_fp {
            return Err(NodeLifecycleError::WarmStartBootstrap(format!(
                "wal-tail fingerprint mismatch: expected {wal_tail_fp:?}, recovered {recovered_fp:?}"
            )));
        }
    }

    // PHASE4-N-AH S4b (DC-NODE-22): the derived replay-anchor summary. The recovered
    // tip is `admit_count` AdmitBlocks above the replay anchor, so the anchor's block
    // number = recovered_tip.block_no - admit_count. This is a DERIVED recovery summary
    // (not an independently persisted chain point), using recovery's authoritative
    // admit_count (the same count that backs the T-REC-05 fingerprint check above) --
    // NOT the snapshot-fragile raw WAL entry count. It lets the warm-start arm
    // distinguish bare-anchor recovery (admit_count 0) from recovery with a replayed
    // local continuation spine (admit_count > 0).
    let recovered_tip_block_no = ChainDbServedSource::new(chaindb).tip().map(|(_, _, bn)| bn);
    recovered.replayed_anchor_block_no =
        recovered_tip_block_no.map(|tip_bn| tip_bn.saturating_sub(admit_count as u64));
    Ok(recovered)
}

/// FirstRun arm — the Mithril-only first-run bootstrap (L2).
///
/// Assembles the seed from the documented-extraction inputs
/// (`--json-seed-path`, `--consensus-inputs-path`) bound to the Mithril
/// `--mithril-manifest-path`, runs the epoch-consistency check, then calls
/// `bootstrap_from_mithril_snapshot` (first non-test caller) which:
///   - imports the manifest provenance,
///   - mints one anchor from the operator-independent seed point,
///   - `verify_mithril_binding` fail-closed BEFORE any state is admitted,
///   - `bootstrap_initial_state` (the single authority) over the PERSISTENT
///     stores, then persists the seed-epoch sidecar + WAL provenance.
///
/// On success: state is durably persisted. L2 does not sync (L4) or produce
/// (L5), so the owner reports success honestly and exits 0 — no block is
/// produced.
fn first_run_mithril_bootstrap(
    cli: &Cli,
    chaindb: &PersistentChainDb,
    wal: &mut FileWalStore,
) -> Result<BootstrapState, NodeLifecycleError> {
    // --- First-run inputs (documented extraction, Mithril-bound). ---
    let manifest_path = cli
        .mithril_manifest_path
        .as_ref()
        .ok_or(NodeLifecycleError::MissingFlag("--mithril-manifest-path"))?;
    let json_seed_path = cli
        .json_seed_path
        .as_ref()
        .ok_or(NodeLifecycleError::MissingFlag("--json-seed-path"))?;
    let consensus_inputs_path = cli
        .consensus_inputs_path
        .as_ref()
        .ok_or(NodeLifecycleError::MissingFlag("--consensus-inputs-path"))?;
    let network_magic = cli
        .network_magic
        .ok_or(NodeLifecycleError::MissingFlag("--network-magic"))?;
    let genesis_hash_hex = cli
        .genesis_hash_hex
        .as_ref()
        .ok_or(NodeLifecycleError::MissingFlag("--genesis-hash"))?;
    let seed_point_slot = cli
        .seed_point_slot
        .ok_or(NodeLifecycleError::MissingFlag("--seed-point-slot"))?;
    let seed_block_hash_hex = cli
        .seed_block_hash_hex
        .as_ref()
        .ok_or(NodeLifecycleError::MissingFlag("--seed-block-hash"))?;

    let genesis_hash =
        parse_hash32(genesis_hash_hex).ok_or(NodeLifecycleError::BadHashHex("--genesis-hash"))?;
    let seed_block_hash = parse_hash32(seed_block_hash_hex)
        .ok_or(NodeLifecycleError::BadHashHex("--seed-block-hash"))?;

    // Read the Mithril manifest bytes (provenance carrier).
    let manifest_bytes = std::fs::read(manifest_path)
        .map_err(|e| NodeLifecycleError::ExtractionRead(format!("manifest: {:?}", e.kind())))?;

    // Documented extraction → seed ledger.
    let (utxo, utxo_fp) = import_cardano_cli_json_utxo(json_seed_path)
        .map_err(|e| NodeLifecycleError::ExtractionRead(format!("json_seed: {e:?}")))?;
    let mut seed_ledger = LedgerState::new(CardanoEra::Conway);
    seed_ledger.utxo_state = utxo;
    let initial_ledger_fingerprint = fingerprint(&seed_ledger).combined;

    // Documented extraction → consensus inputs (eta0 / stake / ASC / epoch).
    let canonical = import_live_consensus_inputs(consensus_inputs_path)
        .map_err(|e| NodeLifecycleError::ExtractionRead(format!("consensus_inputs: {e:?}")))?;
    let seed_chain_dep = PraosChainDepState::genesis(canonical.epoch_nonce.clone());

    // Era schedule for the imported epoch window (used to derive the
    // certified epoch + by the composer's authority on warm-start; the
    // cold-start branch this first run takes does not consume it).
    let era_schedule = make_node_schedule(canonical.epoch_start_slot, canonical.epoch_no);

    // --- Epoch-consistency check (L2 §9.4), BEFORE the composer. ---
    // Parse the manifest provenance to obtain its attested certified
    // point, then require that point to fall WITHIN the consensus inputs'
    // own declared epoch window [epoch_start_slot, epoch_end_slot]. This
    // binds the documented consensus extraction to the same epoch the
    // Mithril certificate attests — a certified slot outside the window
    // means the inputs are from a different epoch. Fail closed.
    let import = import_mithril_manifest_from_bytes(&manifest_bytes)
        .map_err(|e| NodeLifecycleError::ManifestImport(format!("{e:?}")))?;
    let certified_slot = import.report.certified_point.slot;
    let in_window = certified_slot.0 >= canonical.epoch_start_slot.0
        && certified_slot.0 <= canonical.epoch_end_slot.0;
    if !in_window {
        return Err(NodeLifecycleError::EpochMismatch {
            consensus_epoch: canonical.epoch_no.0,
            certified_slot: certified_slot.0,
            consensus_window: (canonical.epoch_start_slot.0, canonical.epoch_end_slot.0),
        });
    }

    // Leadership view (real zip of the canonical inputs; unused on the
    // first-run cold-start branch — bootstrap_initial_state consumes
    // ledger_view only on warm-start — but built faithfully, no placeholder).
    let ledger_view = pool_distr_view_from_canonical(&canonical);

    // --- Operator-independent seed point (DC-MITHRIL-02). ---
    let seed_point_inputs = MithrilSeedPointInputs {
        seed_slot: SlotNo(seed_point_slot),
        seed_block_hash,
        network_magic,
        genesis_hash,
        seed_artifact_hash: blake2b_256_of_file(json_seed_path).ok_or(
            NodeLifecycleError::ExtractionRead("json_seed: re-read for artifact hash".into()),
        )?,
        imported_utxo_fingerprint: utxo_fp,
        initial_ledger_fingerprint,
    };

    // --- The single composition: verify-before-admit, persist sidecar +
    //     WAL provenance. First non-test caller. NO fallback on error. ---
    let out = bootstrap_from_mithril_snapshot(
        &seed_point_inputs,
        seed_ledger,
        seed_chain_dep,
        &manifest_bytes,
        &canonical,
        chaindb,
        chaindb,
        wal,
        &era_schedule,
        &ledger_view,
    )
    .map_err(|e| NodeLifecycleError::MithrilBootstrap(format!("{e:?}")))?;

    // Honest success record. The dispatcher converges into the relay run
    // loop; the bootstrapped BootstrapState is returned for it. The recovered
    // seed-epoch consensus inputs are persisted (sidecar + WAL provenance) but
    // not held in `MithrilBootstrapOutput`; on this binary path the empty
    // source halts the loop before any sync consumes a leadership view, so
    // `seed_epoch_consensus_inputs: None` here is provably unobserved.
    eprintln!(
        "ade_node --mode node: first-run Mithril bootstrap complete \
         (anchor initial_ledger_fingerprint={:?}, epoch={}).",
        out.anchor.initial_ledger_fingerprint, canonical.epoch_no.0
    );
    Ok(BootstrapState {
        ledger: out.ledger,
        chain_dep: out.chain_dep,
        tip: out.tip,
        seed_epoch_consensus_inputs: None,
        replayed_anchor_block_no: None,
    })
}

/// Conway-only single-era schedule consistent with the imported epoch
/// window (mirrors the established `make_schedule_for_imported_window`
/// pattern in `produce_mode` / `admission`). `locate` resolves slots in
/// the window to `epoch_no`.
fn make_node_schedule(epoch_start_slot: SlotNo, epoch_no: EpochNo) -> EraSchedule {
    EraSchedule::new(
        BootstrapAnchorHash(Hash32([0u8; 32])),
        epoch_start_slot.0,
        vec![EraSummary {
            era: CardanoEra::Conway,
            start_slot: epoch_start_slot,
            start_epoch: epoch_no,
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 432_000,
        }],
    )
    .unwrap_or_else(|_| {
        // EraSchedule::new only fails on a zero epoch length, which is a
        // constant above. Construct the same single summary again so the
        // owner has no panic path. (Unreachable in practice.)
        EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            epoch_start_slot.0,
            vec![EraSummary {
                era: CardanoEra::Conway,
                start_slot: epoch_start_slot,
                start_epoch: epoch_no,
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 432_000,
            }],
        )
        .expect("constant 432_000 epoch length is non-zero")
    })
}

/// Zip the canonical consensus inputs into the leadership `PoolDistrView`
/// (mirrors `produce_mode::pool_distr_view_from_consensus_inputs`). The
/// canonical bundle keeps per-pool stake (`pool_distribution`) and VRF
/// keyhashes (`pool_vrf_keyhashes`) in two separate maps; this zips them.
/// A pool absent from the keyhash map cannot be a forge leader anyway, so
/// it takes a zero-hash keyhash that keeps the stake total intact (same
/// rule as the produce-mode projection).
fn pool_distr_view_from_canonical(canonical: &LiveConsensusInputsCanonical) -> PoolDistrView {
    let asc = canonical.active_slots_coeff;
    let mut pools: BTreeMap<ade_types::Hash28, ade_ledger::consensus_view::PoolEntry> =
        BTreeMap::new();
    let mut total: u64 = 0;
    for (pool_id, entry) in &canonical.pool_distribution {
        total = total.saturating_add(entry.active_stake);
        let vrf_keyhash = canonical
            .pool_vrf_keyhashes
            .get(pool_id)
            .cloned()
            .unwrap_or(Hash32([0u8; 32]));
        pools.insert(
            pool_id.clone(),
            ade_ledger::consensus_view::PoolEntry {
                active_stake: entry.active_stake,
                vrf_keyhash,
            },
        );
    }
    PoolDistrView::new(canonical.epoch_no, total, asc, pools)
}

/// Parse a 64-hex-char string into a 32-byte hash. Mirrors the
/// `parse_hash32` helpers in `admission`. Returns `None` on wrong length
/// or non-hex.
fn parse_hash32(hex: &str) -> Option<Hash32> {
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        let pair = hex.get(i * 2..i * 2 + 2)?;
        out[i] = u8::from_str_radix(pair, 16).ok()?;
    }
    Some(Hash32(out))
}

fn blake2b_256_of_file(path: &Path) -> Option<Hash32> {
    let bytes = std::fs::read(path).ok()?;
    Some(ade_crypto::blake2b::blake2b_256(&bytes))
}

fn report(e: &NodeLifecycleError) {
    match e {
        NodeLifecycleError::MissingFlag(flag) => {
            eprintln!("ade_node --mode node: {flag} is required");
        }
        NodeLifecycleError::ChainDbOpen(d) => {
            eprintln!("ade_node --mode node: cannot open persistent ChainDb: {d}");
        }
        NodeLifecycleError::WalOpen(d) => {
            eprintln!("ade_node --mode node: cannot open FileWalStore: {d}");
        }
        NodeLifecycleError::OnDiskRead(d) => {
            eprintln!("ade_node --mode node: cannot read on-disk state: {d}");
        }
        NodeLifecycleError::ExtractionRead(d) => {
            eprintln!(
                "ade_node --mode node: first-run extraction input read/parse failed ({d}); \
                 failing closed."
            );
        }
        NodeLifecycleError::BadHashHex(flag) => {
            eprintln!("ade_node --mode node: {flag} is not a 64-char hex hash");
        }
        NodeLifecycleError::ManifestImport(d) => {
            eprintln!(
                "ade_node --mode node: Mithril manifest import failed ({d}); failing closed. \
                 No genesis / bundle / cold-start fallback is permitted."
            );
        }
        NodeLifecycleError::EpochMismatch {
            consensus_epoch,
            certified_slot,
            consensus_window,
        } => {
            eprintln!(
                "ade_node --mode node: epoch-consistency check failed — the Mithril certificate's \
                 certified slot {certified_slot} falls outside the consensus inputs' epoch \
                 {consensus_epoch} window [{}, {}]; failing closed.",
                consensus_window.0, consensus_window.1
            );
        }
        NodeLifecycleError::MithrilBootstrap(d) => {
            eprintln!(
                "ade_node --mode node: Mithril first-run bootstrap failed ({d}); failing closed. \
                 verify_mithril_binding must pass before any state is admitted; no fallback."
            );
        }
        NodeLifecycleError::WarmStartNoAnchorLineage => {
            eprintln!(
                "ade_node --mode node: warm start detected (non-empty store) but no persisted \
                 seed-epoch anchor lineage to recover; failing closed. No bundle fallback."
            );
        }
        NodeLifecycleError::WarmStartMultipleAnchorLineages { count } => {
            eprintln!(
                "ade_node --mode node: warm start found {count} persisted anchor lineages; \
                 exactly one is expected (single-epoch, single-shot). Failing closed."
            );
        }
        NodeLifecycleError::WarmStartWalReplay(d) => {
            eprintln!(
                "ade_node --mode node: warm-start WAL replay failed ({d}); failing closed. \
                 No bundle fallback is permitted."
            );
        }
        NodeLifecycleError::WarmStartNoProvenance => {
            eprintln!(
                "ade_node --mode node: warm-start WAL has no seed-epoch provenance entry \
                 (sidecar present but not committed); treating as not-imported. Failing closed."
            );
        }
        NodeLifecycleError::WarmStartForwardReplayUnsupported { tip_slot } => {
            eprintln!(
                "ade_node --mode node: warm-start needs forward block replay (no snapshot at \
                 tip slot {tip_slot}); that is L4 durable-apply territory. Failing closed."
            );
        }
        NodeLifecycleError::WarmStartBootstrap(d) => {
            eprintln!(
                "ade_node --mode node: warm-start recovery failed in the bootstrap authority \
                 ({d}); failing closed. The recovered sidecar did not verify; no bundle fallback."
            );
        }
        NodeLifecycleError::RelaySync(d) => {
            eprintln!(
                "ade_node --mode node: relay run-loop sync step failed ({d}); \
                 failing closed (no skip-past, no fallback)."
            );
        }
        NodeLifecycleError::FeedMissingRecoveredConsensusInputs => {
            eprintln!(
                "ade_node --mode node: a live feed is wired (--peer) but the recovered \
                 state carries no seed-epoch consensus inputs, so the feed \
                 header-validation view (leader threshold + VRF-keyhash) cannot be \
                 projected from the recovered consensus surface; failing closed \
                 (no empty-stake view, no accept-if-missing)."
            );
        }
        NodeLifecycleError::ForgeKeyIngress(d) => {
            eprintln!(
                "ade_node --mode node: operator-key ingress failed ({d}); failing \
                 closed. Supply the COMPLETE operator key set \
                 (--cold-skey --kes-skey --vrf-skey --opcert --genesis-file) to \
                 forge, or none of them to run relay-only."
            );
        }
        NodeLifecycleError::ServeStart(d) => {
            eprintln!(
                "ade_node --mode node: serve-to-peer start failed ({d}); failing \
                 closed. The --listen address must parse and be bindable; the node \
                 does not proceed claiming live-serve capability while serving is \
                 disabled."
            );
        }
    }
}

/// GREEN cold-start forge permission (DC-NODE-08): the genesis-successor may be
/// forged only when there is NO selected tip (a from-genesis cold start) AND the
/// recovered seed-epoch lineage is present AND the feed is forge-eligible
/// (CN-NODE-04: no_block_available | clean_empty). ForgeIntent::On is a
/// precondition of reaching this decision (the forge activation is present); a
/// present tip takes the existing WITH-tip path, never this gate. Pure: proposes
/// the permission; the BLUE forge / self_accept disposes.
fn may_cold_start_forge(
    selected_tip_present: bool,
    has_recovered_lineage: bool,
    feed_eligible: bool,
) -> bool {
    !selected_tip_present && has_recovered_lineage && feed_eligible
}

// =====================================================================
// PHASE4-N-AI AI-S3 — live fork-choice apply driver (DC-NODE-25 + DC-NODE-26;
// CE-AI-1 production half). RED composition over the EXISTING enforced
// authorities — owns no decision (the chain_selector orchestrator owns
// select_best_chain) and never calls a chain selector. Latent until AI-S4
// wires it into the receive loop.
// =====================================================================

/// The durable tip after an applied `ChainEvent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedTip {
    pub slot: SlotNo,
    pub hash: Hash32,
}

/// Closed apply-driver failure surface. Every variant halts the apply
/// deterministically; none silently diverges.
#[derive(Debug)]
pub enum ApplyError {
    /// `materialize_rolled_back_state` failed (e.g. RollbackTooDeep — the
    /// fork point is beyond retention / k; DC-CONS-05/06 fail-closed).
    Materialize(MaterializeError),
    /// `commit_rollback` failed — its irreversible-step-first shape leaves
    /// `fwd.receive` + ChainDb unchanged, and NO WAL record is appended.
    CommitRollback(CommitRollbackError),
    /// The durable rollback record could not be appended AFTER a successful
    /// `commit_rollback`. Fail-fast (recovery hardening is AI-S4/S5).
    Wal(ade_ledger::wal::WalError),
    /// A `ChainSelected` roll-forward through `pump_block` failed (e.g. an
    /// invalid body — no tip advance).
    Pump(PumpError),
    /// A `ChainSelected` was applied without the roll-forward block bytes.
    MissingRollForwardBlock,
    /// DC-NODE-26: after apply, the durable ChainDb tip != the event's target.
    ReconciliationMismatch {
        expected_slot: SlotNo,
        expected_hash: Hash32,
        actual: Option<ChainTip>,
    },
}

/// PHASE4-N-AI AI-S3 (DC-NODE-25 + DC-NODE-26; CE-AI-1 production half): apply
/// ONE fork-choice `ChainEvent` to the live durable spine (`fwd`) using ONLY
/// the existing enforced authorities. RED composition — owns no decision and
/// never calls `select_best_chain` / `fork_choice` / a chain selector.
///
/// Per event:
///   - `RolledBack { to_point, .. }`: (1) `materialize_rolled_back_state`
///     (CN-STORE-07) → (2) `commit_rollback` over the live `fwd.receive`
///     (DC-CONS-20 lockstep over ChainDb + ledger + chain_dep) → (3) re-anchor
///     `fwd.prior_fp` to the rolled-back ledger fp → (4) append
///     `WalEntry::RollBack` (AI-S1) **only after** the commit succeeds → (5)
///     reconcile (DC-NODE-26).
///   - `ChainSelected { new_tip, .. }`: roll FORWARD via `pump_block`
///     (DC-NODE-05/12 — the sole durable admit; header→body coherent) →
///     reconcile.
///   - `Rejected` (and the non-orchestrator `ChainExtended` / `RolledForward`,
///     which `process_stream_input` never emits): no durable change.
///
/// `Ok(None)` = no durable change; `Ok(Some(tip))` = the new durable tip.
#[allow(clippy::too_many_arguments)]
pub fn apply_chain_event<D, S>(
    fwd: &mut ForwardSyncState,
    chaindb: &D,
    wal: &mut dyn WalStore,
    snapshots: &S,
    event: &ChainEvent,
    reason: RollbackReason,
    roll_forward_block: Option<&[u8]>,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<Option<AppliedTip>, ApplyError>
where
    D: ChainDb + SnapshotStore,
    S: SnapshotSink,
{
    match event {
        ChainEvent::RolledBack { to_point, .. } => {
            let target = TargetPoint {
                slot: to_point.slot,
                hash: to_point.hash.clone(),
            };
            // (1) Materialize the rolled-back state via the SOLE authority.
            let reader = PersistentSnapshotCache::new(chaindb);
            let source = ChainDbBlockSource::new(chaindb);
            let (new_ledger, new_chain_dep) = materialize_rolled_back_state(
                target.clone(),
                &reader,
                &source,
                era_schedule,
                ledger_view,
            )
            .map_err(ApplyError::Materialize)?;
            // Capture the abandoned (pre-rollback) tip + the rolled-back
            // block_no for the audit record BEFORE the commit mutates state.
            let prior_block_no = fwd.receive.chain_dep.last_block_no.map(|b| b.0).unwrap_or(0);
            let prior_slot = fwd.receive.chain_dep.last_slot.map(|s| s.0).unwrap_or(0);
            let prior_hash = chaindb
                .tip()
                .ok()
                .flatten()
                .map(|t| t.hash)
                .unwrap_or(Hash32([0u8; 32]));
            let to_block_no = new_chain_dep.last_block_no.map(|b| b.0).unwrap_or(0);
            // (2) Commit the rollback (DC-CONS-20 lockstep over the live
            //     ReceiveState + ChainDb). Irreversible-step-first: on failure
            //     state is unchanged and NO WAL record is written below.
            {
                let mut writer = ChainDbWriter::new(chaindb);
                commit_rollback(
                    &mut fwd.receive,
                    target,
                    new_ledger,
                    new_chain_dep,
                    &mut writer,
                )
                .map_err(ApplyError::CommitRollback)?;
            }
            // (3) Re-anchor the WAL running fingerprint to the rolled-back fp.
            let rolled_back_fp = fingerprint(&fwd.receive.ledger).combined;
            fwd.prior_fp = rolled_back_fp;
            // (4) Append the durable rollback record — ONLY after commit.
            let rb_point = RollbackPoint {
                slot: to_point.slot,
                hash: to_point.hash.clone(),
                block_no: BlockNo(to_block_no),
            };
            wal.append(WalEntry::RollBack {
                to_point: rb_point.clone(),
                reason,
                prior_tip: RollbackPoint {
                    slot: SlotNo(prior_slot),
                    hash: prior_hash,
                    block_no: BlockNo(prior_block_no),
                },
                // selected_tip is audit-only (AI-S1): at rollback time the new
                // chain's root is the rollback target (extended by subsequent
                // ChainSelected events). Replay never sets the durable tip from it.
                selected_tip: rb_point,
            })
            .map_err(ApplyError::Wal)?;
            // (5) Reconcile (DC-NODE-26): the durable tip must be the target.
            let tip = chaindb.tip().ok().flatten();
            if !durable_tip_matches(tip.as_ref(), to_point.slot, &to_point.hash) {
                return Err(ApplyError::ReconciliationMismatch {
                    expected_slot: to_point.slot,
                    expected_hash: to_point.hash.clone(),
                    actual: tip,
                });
            }
            Ok(Some(AppliedTip {
                slot: to_point.slot,
                hash: to_point.hash.clone(),
            }))
        }
        ChainEvent::ChainSelected { new_tip, .. } => {
            let bytes = roll_forward_block.ok_or(ApplyError::MissingRollForwardBlock)?;
            // Roll forward through the SOLE durable admit authority
            // (DC-NODE-05/12); pump_block validates the body (header→body
            // coherent — no tip advance without a validated body).
            pump_block(fwd, chaindb, wal, snapshots, bytes, era_schedule, ledger_view)
                .map_err(ApplyError::Pump)?;
            let tip = chaindb.tip().ok().flatten();
            if !durable_tip_matches(tip.as_ref(), new_tip.slot, &new_tip.hash) {
                return Err(ApplyError::ReconciliationMismatch {
                    expected_slot: new_tip.slot,
                    expected_hash: new_tip.hash.clone(),
                    actual: tip,
                });
            }
            Ok(Some(AppliedTip {
                slot: new_tip.slot,
                hash: new_tip.hash.clone(),
            }))
        }
        ChainEvent::Rejected { .. }
        | ChainEvent::ChainExtended { .. }
        | ChainEvent::RolledForward { .. } => Ok(None),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    // PHASE4-N-U S3 (DC-NODE-13): the serve_gate_admits monotone-block_no test
    // (serve_gate_admits_first_block_zero_then_skips_reforged_block_zero) is
    // RETIRED with the gate. Serve-as-projection of the extend-only durable
    // chain provides the same (stronger) stability — the durable chain holds
    // exactly one block 0 by construction (DC-CONS-23) — proven by the
    // tests/ade_node served-chain-projection tests + ci_check_served_chain_projection.sh.

    #[test]
    fn node_forge_protocol_version_and_pparams_from_recovered_current_view() {
        // S2: the forge sources protocol_version + pparams from the recovered
        // ledger's current protocol_params (installed by S2a), NOT the stale
        // default protocol_major 2 — the PO-1 anti-regression.
        let mut pp = ProtocolParameters::default();
        pp.protocol_major = 9;
        pp.protocol_minor = 1;
        let (out_pp, out_pv) = forge_constants_from_pparams(&pp);
        assert_eq!(out_pv, ProtocolVersion { major: 9, minor: 1 });
        assert_eq!(out_pp.protocol_major, 9);
        assert_ne!(out_pv.major, 2, "must not be the stale default protocol_major");
    }

    // ===== PHASE4-N-F-G-J S4: cold-start forge permission gate =====

    #[test]
    fn cold_start_gate_allows_genesis_when_eligible_and_recovered() {
        // no tip + recovered lineage + eligible feed ⇒ may cold-start forge.
        assert!(may_cold_start_forge(false, true, true));
    }

    #[test]
    fn node_spine_cold_start_ineligible_feed_does_not_forge() {
        // UnknownDisconnected (ineligible feed) ⇒ no genesis forge; fail closed.
        assert!(!may_cold_start_forge(false, true, false));
    }

    #[test]
    fn cold_start_gate_blocks_without_recovered_lineage() {
        // No recovered seed-epoch lineage ⇒ no forge from raw/unanchored genesis.
        assert!(!may_cold_start_forge(false, false, true));
    }

    #[test]
    fn cold_start_gate_inactive_when_tip_present() {
        // A present tip takes the existing WITH-tip path, never the cold-start
        // gate — so the genesis forge never double-fires once a tip exists.
        assert!(!may_cold_start_forge(true, true, true));
    }

    // ===== PHASE4-N-F-G-C S1: live WirePump feed helper (CE-G-C-1) =========

    /// PHASE4-N-F-G-C S1: the live-wire helper is fail-soft (C3 honest-scope):
    /// with NO usable peer (empty `--peer`, or an unparseable addr) it builds a
    /// `NodeBlockSource::WirePump` whose channel is already closed — so the feed
    /// ends and the relay loop halts clean (the same outcome as the empty
    /// source). NEVER fatal, NEVER a fabricated address, NEVER a silent tip
    /// graft. (This is why empty `--peer` preserves the prior forge-CAPABLE,
    /// halts-clean contract; the live feed is opt-in via `--peer`.)
    #[tokio::test]
    async fn spawn_live_wire_pump_source_with_no_usable_peer_yields_ended_feed() {
        // Empty peer set: no pump task spawned, the builder's sender is dropped
        // immediately → the feed is closed → next_block yields None.
        let mut empty = spawn_live_wire_pump_source(&[], 1, None);
        assert!(
            empty.next_block().await.is_none(),
            "empty --peer must yield an ended feed (no block, no graft)"
        );
        // Unparseable addr: logged-and-skipped (C3), no pump task → ended feed.
        let mut bad = spawn_live_wire_pump_source(
            &["definitely-not-a-socket-addr".to_string()],
            1,
            None,
        );
        assert!(
            bad.next_block().await.is_none(),
            "an unparseable --peer must be skipped, yielding an ended feed (never fatal)"
        );
    }

    // ===== L1: pure classifier =====

    #[test]
    fn classify_empty_store_is_first_run() {
        assert_eq!(classify_start(false, false), NodeStart::FirstRun);
    }

    #[test]
    fn classify_any_persisted_state_is_warm_start() {
        assert_eq!(classify_start(true, false), NodeStart::WarmStart);
        assert_eq!(classify_start(false, true), NodeStart::WarmStart);
        assert_eq!(classify_start(true, true), NodeStart::WarmStart);
    }

    #[test]
    fn classify_is_pure_two_calls_identical() {
        for &has_tip in &[false, true] {
            for &has_snap in &[false, true] {
                assert_eq!(
                    classify_start(has_tip, has_snap),
                    classify_start(has_tip, has_snap),
                );
            }
        }
    }

    // ===== L2: Mithril first-run bootstrap (hermetic) =====
    //
    // THROWAWAY SYNTHETIC FIXTURE. The manifest cert hash / genesis hash /
    // certified point below are fabricated to make verify_mithril_binding
    // PASS for a tiny synthetic seed. This proves Ade's COMPOSITION +
    // FAIL-CLOSED MECHANICS ONLY — it proves NOTHING about a real Mithril
    // certificate or real preprod state. The real preprod/Mithril claim is
    // the operational-prerequisite live leg (L2 doc §9.4), not this test.

    use std::io::Write;
    use tempfile::TempDir;

    // Epoch window chosen so the manifest's certified slot falls inside it.
    const EPOCH_NO: u64 = 576;
    const EPOCH_START_SLOT: u64 = 23_000_000;
    const CERTIFIED_SLOT: u64 = 23_013_663; // within [EPOCH_START_SLOT, +432_000)
    const GENESIS_HASH_HEX: &str =
        "1111111111111111111111111111111111111111111111111111111111111111";
    const BLOCK_HASH_HEX: &str = "2222222222222222222222222222222222222222222222222222222222222222";
    const CERT_HASH_HEX: &str = "6666666666666666666666666666666666666666666666666666666666666666";
    const NETWORK_MAGIC: u32 = 1;

    fn manifest_json(certified_slot: u64, network_magic: u32, genesis_hex: &str) -> String {
        format!(
            r#"{{
                "artifact_type": "cardano-database-snapshot",
                "certificate_hash_hex": "{CERT_HASH_HEX}",
                "network_magic": {network_magic},
                "genesis_hash_hex": "{genesis_hex}",
                "certified_point": {{
                    "slot": {certified_slot},
                    "block_hash_hex": "{BLOCK_HASH_HEX}"
                }},
                "immutable_range": {{ "lo": 0, "hi": 4242 }},
                "source_mithril_client_version": "throwaway-synthetic-fixture",
                "source_command": "throwaway-synthetic-fixture (NOT a real Mithril artifact)"
            }}"#
        )
    }

    // Minimal cardano-cli `query utxo` JSON: an empty UTxO set is a valid
    // (if trivial) seed for the composition-mechanics test.
    const UTXO_JSON: &str = "{}";

    fn consensus_inputs_json(epoch_no: u64, epoch_start_slot: u64) -> String {
        // Mirrors the RawConsensusInputs shape consumed by
        // import_live_consensus_inputs. Epoch window must contain
        // CERTIFIED_SLOT for the positive case.
        format!(
            r#"{{
                "network_magic": {NETWORK_MAGIC},
                "genesis_hash_hex": "{GENESIS_HASH_HEX}",
                "era": "conway",
                "epoch_no": {epoch_no},
                "epoch_start_slot": {epoch_start_slot},
                "epoch_end_slot": {},
                "active_slots_coeff": {{ "numer": 5, "denom": 100 }},
                "epoch_nonce_hex": "{BLOCK_HASH_HEX}",
                "pool_distribution": {{}},
                "pool_vrf_keyhashes": {{}},
                "protocol_params_hash_hex": "{GENESIS_HASH_HEX}",
                "source_cardano_node_version": "throwaway-synthetic-fixture",
                "source_query_command": "throwaway-synthetic-fixture",
                "source_tip_hash_hex": "{BLOCK_HASH_HEX}",
                "source_tip_slot": {epoch_start_slot}
            }}"#,
            epoch_start_slot + 432_000 - 1
        )
    }

    struct Fixture {
        _dir: TempDir,
        cli: Cli,
    }

    fn write_file(dir: &Path, name: &str, contents: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        p
    }

    /// Build a node-mode Cli over a fresh tempdir with the given fixture
    /// file contents. `mithril_manifest` / `consensus_inputs` overridable
    /// for the negative cases.
    fn fixture(
        manifest: Option<&str>,
        utxo: &str,
        consensus: &str,
        genesis_hash_hex: &str,
        seed_slot: u64,
        network_magic: u32,
    ) -> Fixture {
        let dir = TempDir::new().unwrap();
        let base = dir.path();
        let snap = base.join("snap");
        let wal = base.join("wal");
        let manifest_path = manifest.map(|m| write_file(base, "manifest.json", m));
        let utxo_path = write_file(base, "utxo.json", utxo);
        let cinputs_path = write_file(base, "consensus_inputs.json", consensus);

        let cli = Cli {
            genesis_path: base.join("genesis.json"),
            network: "preprod".to_string(),
            chain_db_path: None,
            snapshot_store_path: None,
            listen_addr: None,
            peer_addrs: vec![],
            mode: crate::cli::Mode::Node,
            log_path: base.join("node.jsonl"),
            tip_read_timeout_secs: 5,
            json_seed_path: Some(utxo_path),
            seed_point_slot: Some(seed_slot),
            seed_block_hash_hex: Some(BLOCK_HASH_HEX.to_string()),
            wal_dir: Some(wal),
            snapshot_dir: Some(snap),
            network_magic: Some(network_magic),
            genesis_hash_hex: Some(genesis_hash_hex.to_string()),
            consensus_inputs_path: Some(cinputs_path),
            mithril_manifest_path: manifest_path,
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
            single_producer_venue: false,
        };
        Fixture { _dir: dir, cli }
    }

    #[tokio::test]
    async fn first_run_mithril_positive_persists_and_succeeds() {
        let f = fixture(
            Some(&manifest_json(
                CERTIFIED_SLOT,
                NETWORK_MAGIC,
                GENESIS_HASH_HEX,
            )),
            UTXO_JSON,
            &consensus_inputs_json(EPOCH_NO, EPOCH_START_SLOT),
            GENESIS_HASH_HEX,
            CERTIFIED_SLOT, // operator seed point == manifest certified point => binding ok
            NETWORK_MAGIC,
        );
        let (_sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
        let r = run_node_lifecycle_inner(&f.cli, &mut sd_rx).await;
        assert!(r.is_ok(), "positive first-run should succeed, got {r:?}");

        // What the Mithril bootstrap persists on a cold store is the
        // anchor-fp-keyed seed-epoch SIDECAR (+ its WAL provenance) — NOT
        // a slot-snapshot (bootstrap_initial_state cold-start writes no
        // block/snapshot). So assert the sidecar is present, keyed by the
        // anchor_fp the owner derived = fingerprint(seed_ledger).combined.
        // Reconstruct that fingerprint exactly as the owner does.
        let (utxo, _) =
            import_cardano_cli_json_utxo(f.cli.json_seed_path.as_ref().unwrap()).unwrap();
        let mut seed_ledger = LedgerState::new(CardanoEra::Conway);
        seed_ledger.utxo_state = utxo;
        let anchor_fp = fingerprint(&seed_ledger).combined;

        let snapshot_dir = f.cli.snapshot_dir.as_ref().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(snapshot_dir.join("chain.db")))
                .unwrap();
        let sidecar = SnapshotStore::get_seed_epoch_consensus_inputs(&chaindb, &anchor_fp).unwrap();
        assert!(
            sidecar.is_some(),
            "first-run Mithril bootstrap must persist the anchor-fp-keyed seed-epoch sidecar"
        );
    }

    #[tokio::test]
    async fn first_run_fails_closed_on_missing_manifest() {
        let f = fixture(
            None, // no --mithril-manifest-path
            UTXO_JSON,
            &consensus_inputs_json(EPOCH_NO, EPOCH_START_SLOT),
            GENESIS_HASH_HEX,
            CERTIFIED_SLOT,
            NETWORK_MAGIC,
        );
        let (_sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
        let r = run_node_lifecycle_inner(&f.cli, &mut sd_rx).await;
        assert_eq!(
            r,
            Err(NodeLifecycleError::MissingFlag("--mithril-manifest-path"))
        );
    }

    #[tokio::test]
    async fn first_run_fails_closed_on_binding_mismatch() {
        // Operator seed point (seed_slot) ≠ manifest certified point =>
        // verify_mithril_binding CertifiedPointMismatch, before any admit.
        let f = fixture(
            Some(&manifest_json(
                CERTIFIED_SLOT,
                NETWORK_MAGIC,
                GENESIS_HASH_HEX,
            )),
            UTXO_JSON,
            &consensus_inputs_json(EPOCH_NO, EPOCH_START_SLOT),
            GENESIS_HASH_HEX,
            CERTIFIED_SLOT + 1, // genuinely different point
            NETWORK_MAGIC,
        );
        let (_sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
        let r = run_node_lifecycle_inner(&f.cli, &mut sd_rx).await;
        assert!(
            matches!(r, Err(NodeLifecycleError::MithrilBootstrap(_))),
            "binding mismatch must fail closed, got {r:?}"
        );
        // And nothing persisted.
        let snapshot_dir = f.cli.snapshot_dir.as_ref().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(snapshot_dir.join("chain.db")))
                .unwrap();
        assert!(
            SnapshotStore::list_snapshot_slots(&chaindb)
                .unwrap()
                .is_empty(),
            "no state may be admitted when the binding fails"
        );
    }

    #[tokio::test]
    async fn first_run_fails_closed_on_epoch_mismatch() {
        // Consensus inputs for an epoch whose window does NOT contain the
        // manifest certified slot => EpochMismatch, before the composer.
        // Use an epoch window far from CERTIFIED_SLOT.
        let other_start = EPOCH_START_SLOT + 432_000; // next epoch window
        let f = fixture(
            Some(&manifest_json(
                CERTIFIED_SLOT,
                NETWORK_MAGIC,
                GENESIS_HASH_HEX,
            )),
            UTXO_JSON,
            &consensus_inputs_json(EPOCH_NO + 1, other_start),
            GENESIS_HASH_HEX,
            CERTIFIED_SLOT,
            NETWORK_MAGIC,
        );
        let (_sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
        let r = run_node_lifecycle_inner(&f.cli, &mut sd_rx).await;
        assert!(
            matches!(r, Err(NodeLifecycleError::EpochMismatch { .. })),
            "epoch mismatch must fail closed, got {r:?}"
        );
    }

    #[tokio::test]
    async fn first_run_fails_closed_on_malformed_extraction() {
        let f = fixture(
            Some(&manifest_json(
                CERTIFIED_SLOT,
                NETWORK_MAGIC,
                GENESIS_HASH_HEX,
            )),
            "{ not valid utxo json",
            &consensus_inputs_json(EPOCH_NO, EPOCH_START_SLOT),
            GENESIS_HASH_HEX,
            CERTIFIED_SLOT,
            NETWORK_MAGIC,
        );
        let (_sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
        let r = run_node_lifecycle_inner(&f.cli, &mut sd_rx).await;
        assert!(
            matches!(r, Err(NodeLifecycleError::ExtractionRead(_))),
            "malformed extraction must fail closed, got {r:?}"
        );
    }

    // ===== L3: production warm-start recovery (hermetic) =====
    //
    // CONSTRUCTED WARM-START PRECONDITION FIXTURE (a valid persisted
    // precondition, NOT fabricated evidence): an anchor-fp-keyed seed-epoch
    // sidecar + its WAL provenance entry + a snapshot at the recovered tip,
    // written to a real PersistentChainDb + FileWalStore, then dropped and
    // reopened (the persist -> drop -> reopen -> recover restart proof). L3
    // proves the warm-start recovery transition over this precondition; L4c
    // later proves that normal peer fetch + durable apply creates this
    // precondition naturally. The fixture IS the valid persisted warm-start
    // precondition — it is the legitimate proof input for the recovery
    // transition, not a stand-in for live evidence.

    use ade_core::consensus::praos_state::Nonce;
    use ade_ledger::consensus_view::PoolEntry;
    use ade_ledger::seed_consensus_inputs::{
        encode_seed_epoch_consensus_inputs, SeedEpochConsensusInputs,
    };
    use ade_ledger::wal::WalEntry;
    use ade_runtime::chaindb::StoredBlock;
    use ade_runtime::rollback::PersistentSnapshotCache;
    use ade_runtime::seed_consensus_provenance::append_seed_epoch_provenance;
    use ade_types::Hash28;

    const WARM_ANCHOR_FP: Hash32 = Hash32([0x5A; 32]);
    const WARM_EPOCH: EpochNo = EpochNo(576);
    const WARM_TIP_SLOT: u64 = 23_013_663;

    struct WarmDirs {
        _dir: TempDir,
        snap: std::path::PathBuf,
        wal: std::path::PathBuf,
    }

    fn fresh_warm_dirs() -> WarmDirs {
        let dir = TempDir::new().unwrap();
        let snap = dir.path().join("snap");
        let wal = dir.path().join("wal");
        std::fs::create_dir_all(&snap).unwrap();
        std::fs::create_dir_all(&wal).unwrap();
        WarmDirs {
            _dir: dir,
            snap,
            wal,
        }
    }

    fn open_warm_stores(d: &WarmDirs) -> (PersistentChainDb, FileWalStore) {
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(d.snap.join("chain.db"))).unwrap();
        let wal = FileWalStore::open(&d.wal).unwrap();
        (chaindb, wal)
    }

    fn warm_sample_record(anchor_fp: Hash32, epoch: EpochNo) -> SeedEpochConsensusInputs {
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        pools.insert(
            Hash28([0x01; 28]),
            PoolEntry {
                active_stake: 1_000,
                vrf_keyhash: Hash32([0x07; 32]),
            },
        );
        SeedEpochConsensusInputs {
            anchor_fp,
            epoch_no: epoch,
            epoch_nonce: Nonce(Hash32([0x99; 32])),
            active_slots_coeff: ActiveSlotsCoeff {
                numer: 5,
                denom: 100,
            },
            total_active_stake: 1_000,
            pool_distribution: pools,
        }
    }

    /// Put a block at `slot` and capture a bare-Conway snapshot AT that
    /// same slot. With the snapshot exactly at the tip, the warm-start's
    /// `materialize_rolled_back_state` takes its degenerate branch and never
    /// folds a block forward — the sole consumer of era_schedule/ledger_view
    /// — so those placeholders are provably unconsumed.
    fn put_tip_and_snapshot(chaindb: &PersistentChainDb, slot: u64) {
        chaindb
            .put_block(&StoredBlock {
                hash: Hash32([0xBB; 32]),
                slot: SlotNo(slot),
                bytes: vec![0xAB; 8],
            })
            .unwrap();
        let ledger = LedgerState::new(CardanoEra::Conway);
        let chain_dep = PraosChainDepState::genesis(Nonce(Hash32([0xCD; 32])));
        PersistentSnapshotCache::new(chaindb)
            .capture(SlotNo(slot), &ledger, &chain_dep)
            .unwrap();
    }

    /// PHASE4-N-U S2: a REALISTIC durable tip — a block, its WAL `AdmitBlock`,
    /// and a snapshot AT the tip slot (mirrors the pump's StoreBlockBytes +
    /// AppendWal + checkpoint). The `AdmitBlock` chains from the anchor
    /// (`prior_fp == WARM_ANCHOR_FP`) and its `post_fp` is the snapshot ledger's
    /// fingerprint, so warm_start_recovery's WAL-tail reconciliation KEEPS the
    /// block and the T-REC-05 fingerprint guard passes (snapshot-at-tip ⇒
    /// degenerate forward-replay).
    fn put_durable_tip(chaindb: &PersistentChainDb, wal: &mut FileWalStore, slot: u64) {
        let ledger = LedgerState::new(CardanoEra::Conway);
        let chain_dep = PraosChainDepState::genesis(Nonce(Hash32([0xCD; 32])));
        chaindb
            .put_block(&StoredBlock {
                hash: Hash32([0xBB; 32]),
                slot: SlotNo(slot),
                bytes: vec![0xAB; 8],
            })
            .unwrap();
        wal.append(ade_ledger::wal::WalEntry::AdmitBlock {
            prior_fp: WARM_ANCHOR_FP,
            block_hash: Hash32([0xBB; 32]),
            slot: SlotNo(slot),
            verdict: ade_ledger::wal::BlockVerdictTag::Valid,
            post_fp: fingerprint(&ledger).combined,
        })
        .unwrap();
        PersistentSnapshotCache::new(chaindb)
            .capture(SlotNo(slot), &ledger, &chain_dep)
            .unwrap();
    }

    #[test]
    fn warm_start_recovers_seed_epoch_consensus_inputs_byte_identical() {
        // The CE-L-3 positive: a valid persisted precondition recovers the
        // byte-identical seed-epoch sidecar through the single
        // bootstrap_initial_state authority, across a drop+reopen boundary.
        let d = fresh_warm_dirs();
        let record = warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH);
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        {
            let (chaindb, mut wal) = open_warm_stores(&d);
            chaindb
                .put_seed_epoch_consensus_inputs(&WARM_ANCHOR_FP, &bytes)
                .unwrap();
            append_seed_epoch_provenance(&mut wal, &WARM_ANCHOR_FP, WARM_EPOCH, &bytes).unwrap();
            put_durable_tip(&chaindb, &mut wal, WARM_TIP_SLOT);
            // stores dropped here -> restart boundary.
        }

        let (chaindb, wal) = open_warm_stores(&d);
        let state = warm_start_recovery(&chaindb, &wal).expect("warm-start recovers");

        let recovered = state
            .seed_epoch_consensus_inputs
            .expect("warm-start recovers the sidecar");
        assert_eq!(recovered, record);
        // Byte-identity: re-encoding the recovered record reproduces exactly
        // the persisted sidecar bytes.
        assert_eq!(encode_seed_epoch_consensus_inputs(&recovered), bytes);
        // Recovered tip matches the persisted tip.
        assert_eq!(state.tip.map(|t| t.slot.0), Some(WARM_TIP_SLOT));
    }

    #[tokio::test]
    async fn warm_start_dispatch_succeeds_end_to_end() {
        // The whole owner path: classify_start -> WarmStart arm ->
        // warm_start_recovery -> Ok, over the same constructed precondition.
        let d = fresh_warm_dirs();
        let record = warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH);
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        {
            let (chaindb, mut wal) = open_warm_stores(&d);
            chaindb
                .put_seed_epoch_consensus_inputs(&WARM_ANCHOR_FP, &bytes)
                .unwrap();
            append_seed_epoch_provenance(&mut wal, &WARM_ANCHOR_FP, WARM_EPOCH, &bytes).unwrap();
            put_tip_and_snapshot(&chaindb, WARM_TIP_SLOT);
        }
        let cli = warm_cli(&d);
        let (_sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
        let r = run_node_lifecycle_inner(&cli, &mut sd_rx).await;
        assert!(r.is_ok(), "warm-start dispatch should succeed, got {r:?}");
    }

    #[test]
    fn warm_start_fails_closed_on_missing_sidecar() {
        // No sidecar persisted. With W2 discovery sourced from the sidecar
        // table key, an absent sidecar surfaces as "no anchor lineage" — the
        // fail-closed "nothing to recover", with NO bundle fallback. (This
        // is the reachable form of the doc's missing-sidecar case: the
        // discovery step guarantees the sidecar key exists before the
        // bootstrap authority's own SidecarMissing check can run.)
        let d = fresh_warm_dirs();
        {
            let (chaindb, _wal) = open_warm_stores(&d);
            put_tip_and_snapshot(&chaindb, WARM_TIP_SLOT);
        }
        let (chaindb, wal) = open_warm_stores(&d);
        let r = warm_start_recovery(&chaindb, &wal);
        assert!(
            matches!(r, Err(NodeLifecycleError::WarmStartNoAnchorLineage)),
            "missing sidecar must fail closed, got {r:?}"
        );
    }

    #[test]
    fn warm_start_fails_closed_on_missing_wal_provenance() {
        // Sidecar present, but no WAL provenance entry committed: replay
        // recovers no provenance -> fail closed (treat as not-imported).
        let d = fresh_warm_dirs();
        let record = warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH);
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        {
            let (chaindb, _wal) = open_warm_stores(&d);
            chaindb
                .put_seed_epoch_consensus_inputs(&WARM_ANCHOR_FP, &bytes)
                .unwrap();
            put_tip_and_snapshot(&chaindb, WARM_TIP_SLOT);
            // No append_seed_epoch_provenance.
        }
        let (chaindb, wal) = open_warm_stores(&d);
        let r = warm_start_recovery(&chaindb, &wal);
        assert!(
            matches!(r, Err(NodeLifecycleError::WarmStartNoProvenance)),
            "missing WAL provenance must fail closed, got {r:?}"
        );
    }

    #[test]
    fn warm_start_fails_closed_on_sidecar_hash_mismatch() {
        // Sidecar present + WAL provenance present, but the provenance
        // sidecar_hash does not bind the persisted bytes -> the bootstrap
        // authority's verify chain fails closed (SeedConsensusHashMismatch).
        let d = fresh_warm_dirs();
        let record = warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH);
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        {
            let (chaindb, mut wal) = open_warm_stores(&d);
            chaindb
                .put_seed_epoch_consensus_inputs(&WARM_ANCHOR_FP, &bytes)
                .unwrap();
            // Raw WAL entry with a deliberately wrong sidecar_hash.
            wal.append(WalEntry::SeedEpochConsensusInputsImported {
                anchor_fp: WARM_ANCHOR_FP,
                sidecar_hash: Hash32([0xAA; 32]),
                epoch_no: WARM_EPOCH,
            })
            .unwrap();
            put_tip_and_snapshot(&chaindb, WARM_TIP_SLOT);
        }
        let (chaindb, wal) = open_warm_stores(&d);
        let r = warm_start_recovery(&chaindb, &wal);
        match r {
            Err(NodeLifecycleError::WarmStartBootstrap(d)) => {
                assert!(
                    d.contains("SeedConsensusHashMismatch"),
                    "expected SeedConsensusHashMismatch, got {d}"
                );
            }
            other => panic!("hash mismatch must fail closed in bootstrap, got {other:?}"),
        }
    }

    #[test]
    fn warm_start_fails_closed_on_anchor_mismatch() {
        // Sidecar stored under anchor X (the discovery source); the WAL
        // provenance entry names a DIFFERENT anchor Y. Replaying from the
        // independent X catches the mismatch -> fail closed. This is the
        // non-circular check: the sidecar-key anchor must equal the WAL
        // entry's anchor.
        let d = fresh_warm_dirs();
        let record = warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH);
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        {
            let (chaindb, mut wal) = open_warm_stores(&d);
            chaindb
                .put_seed_epoch_consensus_inputs(&WARM_ANCHOR_FP, &bytes)
                .unwrap();
            // WAL provenance for a different anchor (0x99 != 0x5A).
            append_seed_epoch_provenance(&mut wal, &Hash32([0x99; 32]), WARM_EPOCH, &bytes)
                .unwrap();
            put_tip_and_snapshot(&chaindb, WARM_TIP_SLOT);
        }
        let (chaindb, wal) = open_warm_stores(&d);
        let r = warm_start_recovery(&chaindb, &wal);
        match r {
            Err(NodeLifecycleError::WarmStartWalReplay(d)) => {
                assert!(
                    d.contains("ProvenanceAnchorMismatch"),
                    "expected ProvenanceAnchorMismatch, got {d}"
                );
            }
            other => panic!("anchor mismatch must fail closed in WAL replay, got {other:?}"),
        }
    }

    #[test]
    fn warm_start_fails_closed_on_duplicate_provenance() {
        // Two WAL provenance entries for the same anchor -> replay fails
        // closed (exactly one provenance entry is allowed per anchor).
        let d = fresh_warm_dirs();
        let record = warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH);
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        {
            let (chaindb, mut wal) = open_warm_stores(&d);
            chaindb
                .put_seed_epoch_consensus_inputs(&WARM_ANCHOR_FP, &bytes)
                .unwrap();
            append_seed_epoch_provenance(&mut wal, &WARM_ANCHOR_FP, WARM_EPOCH, &bytes).unwrap();
            append_seed_epoch_provenance(&mut wal, &WARM_ANCHOR_FP, WARM_EPOCH, &bytes).unwrap();
            put_tip_and_snapshot(&chaindb, WARM_TIP_SLOT);
        }
        let (chaindb, wal) = open_warm_stores(&d);
        let r = warm_start_recovery(&chaindb, &wal);
        match r {
            Err(NodeLifecycleError::WarmStartWalReplay(d)) => {
                assert!(
                    d.contains("DuplicateProvenance"),
                    "expected DuplicateProvenance, got {d}"
                );
            }
            other => panic!("duplicate provenance must fail closed, got {other:?}"),
        }
    }

    #[test]
    fn warm_start_fails_closed_on_multiple_anchor_lineages() {
        // Two distinct anchor lineages persisted -> exactly-one is required;
        // fail closed rather than guess which to recover (CN-ANCHOR-01).
        let d = fresh_warm_dirs();
        let rec_a = warm_sample_record(Hash32([0x5A; 32]), WARM_EPOCH);
        let rec_b = warm_sample_record(Hash32([0x5B; 32]), WARM_EPOCH);
        {
            let (chaindb, _wal) = open_warm_stores(&d);
            chaindb
                .put_seed_epoch_consensus_inputs(
                    &Hash32([0x5A; 32]),
                    &encode_seed_epoch_consensus_inputs(&rec_a),
                )
                .unwrap();
            chaindb
                .put_seed_epoch_consensus_inputs(
                    &Hash32([0x5B; 32]),
                    &encode_seed_epoch_consensus_inputs(&rec_b),
                )
                .unwrap();
            put_tip_and_snapshot(&chaindb, WARM_TIP_SLOT);
        }
        let (chaindb, wal) = open_warm_stores(&d);
        let r = warm_start_recovery(&chaindb, &wal);
        assert!(
            matches!(
                r,
                Err(NodeLifecycleError::WarmStartMultipleAnchorLineages { count: 2 })
            ),
            "multiple lineages must fail closed, got {r:?}"
        );
    }

    #[test]
    fn warm_start_drops_orphan_block_above_wal_tail() {
        // PHASE4-N-U S2 (DC-WAL-04 no-orphan): a torn StoreBlockBytes-before-
        // AppendWal crash leaves a block durable in the ChainDb but ABSENT from
        // the WAL — an orphan ABOVE the WAL tail. Warm-start reconciles the
        // ChainDb to the WAL tail (rollback_to_slot) and drops the orphan; the
        // recovered tip is the WAL-tail tip, never the un-WAL'd orphan.
        // (This replaces the obsolete snapshot-at-tip-only guard test: forward
        // replay from a sub-tip snapshot IS now supported — S2.)
        let d = fresh_warm_dirs();
        let record = warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH);
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        {
            let (chaindb, mut wal) = open_warm_stores(&d);
            chaindb
                .put_seed_epoch_consensus_inputs(&WARM_ANCHOR_FP, &bytes)
                .unwrap();
            append_seed_epoch_provenance(&mut wal, &WARM_ANCHOR_FP, WARM_EPOCH, &bytes).unwrap();
            // The legit durable tip: block + WAL AdmitBlock + snapshot.
            put_durable_tip(&chaindb, &mut wal, WARM_TIP_SLOT);
            // A torn-write ORPHAN one slot above: a ChainDb block with NO WAL
            // AdmitBlock (StoreBlockBytes done, AppendWal not).
            chaindb
                .put_block(&StoredBlock {
                    hash: Hash32([0xCC; 32]),
                    slot: SlotNo(WARM_TIP_SLOT + 1),
                    bytes: vec![0xCD; 8],
                })
                .unwrap();
        }
        let (chaindb, wal) = open_warm_stores(&d);
        let state = warm_start_recovery(&chaindb, &wal)
            .expect("warm-start recovers, reconciling the orphan away");
        // The recovered tip is the WAL-tail tip, NOT the un-WAL'd orphan above it.
        assert_eq!(
            state.tip.map(|t| t.slot.0),
            Some(WARM_TIP_SLOT),
            "the orphan block above the WAL tail must be dropped (DC-WAL-04 no-orphan)"
        );
        // The orphan is gone from the durable ChainDb.
        assert!(
            ChainDb::get_block_by_hash(&chaindb, &Hash32([0xCC; 32]))
                .unwrap()
                .is_none(),
            "the reconciliation must drop the orphan block from the ChainDb"
        );
    }

    /// Minimal node-mode Cli for the end-to-end warm-start dispatch test:
    /// only the two persistence dirs are set; the FirstRun-only inputs are
    /// all `None` (the WarmStart arm never reads them).
    fn warm_cli(d: &WarmDirs) -> Cli {
        Cli {
            genesis_path: d._dir.path().join("genesis.json"),
            network: "preprod".to_string(),
            chain_db_path: None,
            snapshot_store_path: None,
            listen_addr: None,
            peer_addrs: vec![],
            mode: crate::cli::Mode::Node,
            log_path: d._dir.path().join("node.jsonl"),
            tip_read_timeout_secs: 5,
            json_seed_path: None,
            seed_point_slot: None,
            seed_block_hash_hex: None,
            wal_dir: Some(d.wal.clone()),
            snapshot_dir: Some(d.snap.clone()),
            network_magic: None,
            genesis_hash_hex: None,
            consensus_inputs_path: None,
            mithril_manifest_path: None,
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
            single_producer_venue: false,
        }
    }

    // ---- PHASE4-N-F-F S3: --mode node operator-key ingress (On path) -----

    /// Write a complete real-format operator key set + genesis into `dir`
    /// (ade-native KES envelope, cardano-cli VRF/cold text-envelopes, opcert
    /// JSON whose hot_vkey is the KES vkey from the same seed). Returns
    /// (cold, kes, vrf, opcert, genesis). Mirrors the operator_forge fixture
    /// idiom; writes no key bytes to any log/snapshot.
    fn write_node_operator_material(
        dir: &std::path::Path,
    ) -> (
        std::path::PathBuf,
        std::path::PathBuf,
        std::path::PathBuf,
        std::path::PathBuf,
        std::path::PathBuf,
    ) {
        use std::io::Write as _;
        fn hexe(bytes: &[u8]) -> String {
            let mut s = String::with_capacity(bytes.len() * 2);
            for b in bytes {
                s.push_str(&format!("{b:02x}"));
            }
            s
        }
        fn cli_envelope(path: &std::path::Path, ty: &str, payload: &[u8]) {
            let cbor_hex = format!("58{:02x}{}", payload.len(), hexe(payload));
            let json = format!(
                "{{\"type\":\"{ty}\",\"description\":\"N-F-F S3 fixture\",\"cborHex\":\"{cbor_hex}\"}}"
            );
            let mut f = std::fs::File::create(path).unwrap();
            f.write_all(json.as_bytes()).unwrap();
        }
        let kes_seed = [0x42u8; 32];
        let kes = dir.join("kes.ade.skey");
        ade_runtime::producer::keys::write_ade_kes_envelope(&kes, &kes_seed, 0).unwrap();
        let (vrf_sk, _) = cardano_crypto::vrf::VrfDraft03::keypair_from_seed(&[0x07u8; 32]);
        let vrf = dir.join("vrf.skey");
        cli_envelope(&vrf, "VrfSigningKey_PraosVRF", &vrf_sk);
        let cold = dir.join("cold.skey");
        cli_envelope(&cold, "StakePoolSigningKey_ed25519", &[0x33u8; 32]);
        use ade_crypto::kes_sum::KesAlgorithm;
        let kes_raw = ade_crypto::kes_sum::Sum6Kes::gen_key_kes_from_seed_bytes(&kes_seed).unwrap();
        let kes_vk = ade_crypto::kes_sum::Sum6Kes::derive_verification_key(&kes_raw);
        // REAL NodeOperationalCertificate envelope (S2): array(2)[array(4)[...], cold_vk].
        let mut ocbor = vec![0x82u8, 0x84, 0x58, 0x20];
        ocbor.extend_from_slice(&kes_vk);
        ocbor.push(0x00); // sequence_number 0
        ocbor.push(0x00); // kes_period 0
        ocbor.extend_from_slice(&[0x58, 0x40]);
        ocbor.extend_from_slice(&[0u8; 64]); // sigma
        ocbor.extend_from_slice(&[0x58, 0x20]);
        ocbor.extend_from_slice(&[0u8; 32]); // cold_vk
        let opcert = dir.join("opcert.json");
        std::fs::write(
            &opcert,
            format!(
                "{{\"type\":\"NodeOperationalCertificate\",\"description\":\"\",\"cborHex\":\"{}\"}}",
                hexe(&ocbor)
            ),
        )
        .unwrap();
        // REAL shelley-genesis.json (clock/KES/network constants only; S2).
        let genesis = dir.join("op-genesis.json");
        std::fs::write(
            &genesis,
            br#"{"networkMagic":1,"systemStart":"2022-06-01T00:00:00Z","slotLength":1,"slotsPerKESPeriod":129600,"maxKESEvolutions":63}"#,
        )
        .unwrap();
        (cold, kes, vrf, opcert, genesis)
    }

    fn warm_fixture(d: &WarmDirs) {
        let record = warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH);
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        let (chaindb, mut wal) = open_warm_stores(d);
        chaindb
            .put_seed_epoch_consensus_inputs(&WARM_ANCHOR_FP, &bytes)
            .unwrap();
        append_seed_epoch_provenance(&mut wal, &WARM_ANCHOR_FP, WARM_EPOCH, &bytes).unwrap();
        put_tip_and_snapshot(&chaindb, WARM_TIP_SLOT);
    }

    #[tokio::test]
    async fn node_mode_with_operator_keys_warm_start_forge_capable_halts_clean() {
        // On path end-to-end (CE-F-3 + CE-F-4): warm-start recovers the SINGLE
        // BootstrapState, classify_forge_intent => On, build the
        // operator-material-backed activation on that recovered state, enter
        // run_relay_loop with Some(..) — and halt cleanly on the empty source
        // (forge CAPABLE, not observable; no second bootstrap, no Mithril call).
        let d = fresh_warm_dirs();
        warm_fixture(&d);
        let (cold, kes, vrf, opcert, genesis) = write_node_operator_material(d._dir.path());
        let mut cli = warm_cli(&d);
        cli.cold_skey = Some(cold);
        cli.kes_skey = Some(kes);
        cli.vrf_skey = Some(vrf);
        cli.opcert = Some(opcert);
        cli.genesis_file = Some(genesis);
        let (_sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
        let r = run_node_lifecycle_inner(&cli, &mut sd_rx).await;
        assert!(
            r.is_ok(),
            "forge-on warm-start should halt cleanly, got {r:?}"
        );
    }

    #[tokio::test]
    async fn node_mode_partial_operator_keys_fail_closed() {
        // A partial operator key set must fail closed — never a silent relay
        // fallback, never a forge (CE-F-1 wired into the binary arm).
        let d = fresh_warm_dirs();
        warm_fixture(&d);
        let (cold, kes, _vrf, _opcert, _genesis) = write_node_operator_material(d._dir.path());
        let mut cli = warm_cli(&d);
        // Only cold + kes present — VRF / opcert / genesis missing.
        cli.cold_skey = Some(cold);
        cli.kes_skey = Some(kes);
        let (_sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
        let r = run_node_lifecycle_inner(&cli, &mut sd_rx).await;
        assert!(
            matches!(r, Err(NodeLifecycleError::ForgeKeyIngress(_))),
            "partial operator keys must fail closed, got {r:?}"
        );
    }
}
