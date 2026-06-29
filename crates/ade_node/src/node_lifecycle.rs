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
use ade_ledger::seed_consensus_inputs::{
    decode_seed_epoch_consensus_inputs, SeedConsensusInputsError, SeedEpochConsensusInputs,
};
use ade_ledger::wal::{replay_from_anchor, RollbackPoint, RollbackReason, WalEntry, WalStore};
use ade_runtime::bootstrap::{
    bootstrap_initial_state, BootstrapInputs, BootstrapState, SeedEpochConsensusSource,
};
use ade_runtime::recovered_anchor::load_recovered_anchor_point;
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
use ade_runtime::forward_sync::{
    pump_block, ForwardSyncState, NoCheckpointSink, PumpError, PumpTip, SnapshotSink,
};
use ade_runtime::producer::coordinator::{
    coordinator_init, CoordinatorConfig, CoordinatorEvent, CoordinatorState, LedgerSnapshotRef,
};
use ade_runtime::producer::producer_shell::ProducerShell;
use ade_runtime::rollback::{ChainDbBlockSource, PersistentSnapshotCache, SnapshotCadence};
use ade_ledger::rollback::{
    commit_rollback, materialize_rolled_back_state, CommitRollbackError, MaterializeError,
    TargetPoint,
};
use ade_core::consensus::events::{BlockDistance, ChainEvent, Point, SecurityParam};
use ade_core::consensus::candidate::{CandidateFragment, ChainSelectorState};
use ade_core::consensus::fork_choice::{select_best_chain, ForkChoiceError};
use ade_ledger::block_validity::{decode_block, DecodedBlock};
use ade_runtime::receive::ChainDbWriter;

use ade_network::codec::chain_sync::Point as WirePoint;

use crate::candidate_aggregator::{assemble_candidate_set, build_candidate_fragment};
use crate::fair_merge::{fair_merge, PER_PEER_LANE_CAP};
use crate::lca_walk::{walk_to_durable_lca, CachedHeader};
use crate::fork_switch::{
    fork_switch_fence_resolved, map_lca_error, prevalidate_branch, range_refetch_should_retry,
    BranchBodySource, BranchProofError, ForkSwitchOutcome, MissingBridgeReason,
    NullBranchBodySource, PostSwitchFollow, PrefetchedBranchBodies, ProvenBranch, RangeRefetch,
    RangeRefetchOutcome,
};
use crate::selector_state::{project_tiebreaker, ForkAnchor, PendingForkSwitch};

use crate::admission::bootstrap::build_n2n_version_table;
use crate::cli::Cli;
use crate::forge_intent::{classify_forge_intent, ForgeIntent};
use crate::admission_log::{ForkChoiceEvidenceFailure, ForkChoiceResult};
use crate::convergence_evidence::{fork_switch_id, ConvergenceEvidence, ConvergenceEvidenceSink};
use crate::node_sync::{
    admit_forged_block_durably, classify_receive, durable_tip_matches,
    forge_followed_tip_admission, forge_mode_after_admit, forge_mode_on_caughtup,
    forge_one_from_recovered, participant_forge_decision, participant_forge_mode_after_admit,
    participant_forge_mode_on_caughtup, participant_sign_time_base_consistent,
    pending_reselection_forge_refusal, resolve_disposition,
    run_node_sync, single_producer_forge_decision, venue_policy, CandidateSummary,
    ForgeFollowedTipAdmission, ForgeMode, ForgeRefused, NodeBlockSource, NodeForgeOutcome,
    NodeSyncError, NodeSyncItem, ParticipantForgeDecision, ReceiveDisposition,
    SingleProducerForgeDecision, VenueRole,
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
    /// DURABLE-ADMISSION-BYTES: the WAL holds `AdmitBlock(block_hash)` but
    /// `ChainDb::get_block_by_hash` returned `None` — the durable block bytes the
    /// WAL admission authority requires are absent. Corrupted durable state, NOT
    /// block absence; fail closed (never a silent skip).
    DurableBlockBytesMissing {
        block_hash: Hash32,
        entry_index: usize,
        source: &'static str,
    },
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
    /// ECA-2-pre (DC-CINPUT-06): the warm-start sidecar is an OLD schema version
    /// (pre-v4 — missing the consensus-profile hashes / eta0 / venue geometry). A
    /// TYPED upgrade/reimport requirement, DISTINCT from a corrupt/malformed sidecar
    /// (`WarmStartBootstrap`): the store is well-formed but predates this node's
    /// required schema. Fail closed (no defaulting / no CLI re-supply); re-import to
    /// upgrade. Recoverable + auditable — the SAME typed error the bootstrap
    /// authority raises, on the live warm-start path (which decodes the sidecar first).
    ConsensusInputsSchemaUnsupported {
        found_version: u32,
        required_version: u32,
    },
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
    /// A restart-supplied shelley genesis `epochLength` disagrees with the durable
    /// seed-epoch sidecar's persisted `epoch_length_slots`. The sidecar is the
    /// epoch-geometry AUTHORITY (WARMSTART-ERA-SCHEDULE-VENUE / DC-CINPUT-05); a
    /// store must not be "repaired" by passing a different venue's genesis at
    /// restart. Fail closed.
    RestartGenesisGeometryMismatch {
        sidecar_epoch_length: u32,
        genesis_epoch_length: u64,
    },
    /// MITHRIL-VERIFIED-ANCHOR-INTEGRATION S1d: a FORBIDDEN flag
    /// (`--json-seed-path` / `--consensus-inputs-path`, the cardano-cli / JSON
    /// seed) was supplied ALONGSIDE the native Mithril FirstRun inputs
    /// (`--mithril-state-path` + `--mithril-tables-path`). The native route is
    /// the snapshot-authoritative path; mixing it with an operator seed is a
    /// structured terminal error (no ambiguous, half-authoritative bootstrap;
    /// no fallback, no silent ignore). Fail closed BEFORE any decode.
    NativeRouteForbiddenFlag(&'static str),
    /// MITHRIL-VERIFIED-ANCHOR-INTEGRATION S1d: the NATIVE FirstRun route
    /// fail-closed — a missing / mixed snapshot component, a manifest / point /
    /// network / era mismatch, or a decode / materialize / assemble / persist
    /// failure. Carries the closed `NativeFirstRunError` debug. Fail closed —
    /// TERMINAL before the WAL commit-point (authority visibility); NO bootable
    /// partial state, NO fallback to the cardano-cli / JSON seed.
    NativeFirstRun(String),
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
        | NodeLifecycleError::NativeRouteForbiddenFlag(_)
        | NodeLifecycleError::ServeStart(_) => EXIT_GENERIC_STARTUP,
        NodeLifecycleError::ManifestImport(_)
        | NodeLifecycleError::EpochMismatch { .. }
        | NodeLifecycleError::NativeFirstRun(_)
        | NodeLifecycleError::MithrilBootstrap(_) => EXIT_NODE_MITHRIL_BOOTSTRAP_FAILED,
        NodeLifecycleError::WarmStartNoAnchorLineage
        | NodeLifecycleError::WarmStartMultipleAnchorLineages { .. }
        | NodeLifecycleError::WarmStartWalReplay(_)
        | NodeLifecycleError::WarmStartNoProvenance
        | NodeLifecycleError::DurableBlockBytesMissing { .. }
        | NodeLifecycleError::WarmStartForwardReplayUnsupported { .. }
        | NodeLifecycleError::RestartGenesisGeometryMismatch { .. }
        | NodeLifecycleError::WarmStartBootstrap(_)
        | NodeLifecycleError::ConsensusInputsSchemaUnsupported { .. } => {
            EXIT_NODE_WARM_START_RECOVERY_FAILED
        }
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
    //    path fails closed. On the --bootstrap-mithril route the STORE is --data-dir
    //    (--snapshot-dir there is the read-only Mithril snapshot); see resolve_store_dir.
    let snapshot_dir = resolve_store_dir(cli)?;
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

    // S3f-4d-mat-2c (DC-EPOCH-11): open the live reduced checkpoint IFF the EVIEW activation
    // is configured -- the admission bootstrap built it at snapshot_dir/reduced-checkpoint.redb
    // (gated on the EVIEW cert-state package). Absent -> None (a non-EVIEW run; the relay
    // loop's follow/forge path is byte-identical). When present, the loop advances it to the
    // durable ChainDB tip after each admit.
    let reduced_checkpoint_path = snapshot_dir.join("reduced-checkpoint.redb");
    let mut reduced_checkpoint = if reduced_checkpoint_path.exists() {
        Some(
            ade_runtime::chaindb::ReducedUtxoCheckpoint::open(&reduced_checkpoint_path)
                .map_err(|e| NodeLifecycleError::ChainDbOpen(format!("reduced checkpoint: {e:?}")))?,
        )
    } else {
        None
    };

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

    // ECA-5: on a true FirstRun the line-461 binding ran against an empty store dir (the bootstrap had
    // not run yet) -> None. first_run_mithril_bootstrap (above) has now built the live reduced checkpoint
    // at store_dir/reduced-checkpoint.redb. Re-open it so the EVIEW authority-preparation seam is armed --
    // without this, a FirstRun that catches up across an epoch boundary sees a None reduced_checkpoint in
    // the seam's (eview, reduced_checkpoint, authority) gate, no-ops, and fails OutsideForecastRange.
    if reduced_checkpoint.is_none() && reduced_checkpoint_path.exists() {
        reduced_checkpoint = Some(
            ade_runtime::chaindb::ReducedUtxoCheckpoint::open(&reduced_checkpoint_path).map_err(|e| {
                NodeLifecycleError::ChainDbOpen(format!("reduced checkpoint (post-bootstrap): {e:?}"))
            })?,
        );
    }

    // LIVE-LEDGER-EPOCH-TRANSITION S2 (DC-EPOCH-20): open the durable non-UTxO accumulator beside the
    // reduced checkpoint. By here the FirstRun bootstrap has run (the reduced-checkpoint reopen above
    // proves it), so a native-bootstrapped node finds the sealed store; a warm start finds its prior
    // store; a non-native start finds none. OBSERVE-ONLY in S2 (S4 makes it the leadership authority),
    // so an open failure is NON-FATAL -- logged, `None`, and the follow continues without it (the live
    // advance is gated on `Some`). It NEVER blocks the proven follow.
    let accumulator_path = snapshot_dir.join("epoch-accumulator.redb");
    let epoch_accumulator = if accumulator_path.exists() {
        match ade_runtime::chaindb::EpochAccumulatorStore::open(&accumulator_path) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!(
                    "ade_node --mode node: epoch-accumulator open skipped (non-fatal): {e:?}"
                );
                None
            }
        }
    } else {
        None
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

    // WARMSTART-ERA-SCHEDULE-VENUE (DC-CINPUT-05): the durable sidecar is the
    // epoch-geometry authority; a restart-supplied --genesis-file is ONLY a
    // consistency check. Fail closed on a mismatch -- never silently honor the
    // persisted geometry while the operator supplies a different venue's genesis.
    if let Some(sidecar) = state.seed_epoch_consensus_inputs.as_ref() {
        assert_restart_genesis_matches_sidecar(cli.genesis_file.as_deref(), sidecar)?;
    }

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
            let era_schedule = recovered_node_schedule(&state, !cli.peer_addrs.is_empty(), rsw_for_cli(cli))?;
            // CONTINUITY: a relay-only follow validates incoming headers against the recovered
            // leadership view -- the SAME view the forge-ON path uses, from the seed-epoch sidecar.
            // Empty placeholder only when there is neither a live feed nor recovered inputs.
            let ledger_view = match state.seed_epoch_consensus_inputs.as_ref() {
                Some(record) => PoolDistrView::from_seed_epoch_consensus_inputs(record),
                None if !cli.peer_addrs.is_empty() => {
                    return Err(NodeLifecycleError::FeedMissingRecoveredConsensusInputs)
                }
                None => PoolDistrView::new(
                    EpochNo(epoch.unwrap_or(0)),
                    0,
                    ActiveSlotsCoeff { numer: 0, denom: 1 },
                    BTreeMap::new(),
                ),
            };
            // ECA-5 step 1: wire the cross-epoch EVIEW activation into the relay-only (forge-OFF) path
            // so a no-keys node can cross the epoch boundary -- the SAME construction as the forge-ON
            // branch, built BEFORE state.ledger moves into the spine. The replay-scratch lives under the
            // durable store dir (--data-dir), never the snapshot dir (which may be deleted post-bootstrap).
            let eview_inputs: Option<crate::epoch_wire::EviewActivationInputs> = match (
                reduced_checkpoint.as_ref(),
                state.seed_epoch_consensus_inputs.as_ref(),
                state.tip.as_ref(),
            ) {
                (Some(_live), Some(sidecar), Some(_tip)) => {
                    let network_magic = resolve_network_magic(cli)?;
                    Some(crate::epoch_wire::EviewActivationInputs {
                        seed_bootstrap_state: state.ledger.clone(),
                        // Warm-start LAYER-4 fix (mirror of the other match arm): anchor the recovery's
                        // seed->seed+2 window on the ORIGINAL seed bootstrap point persisted in the v5
                        // sidecar, NOT `tip` (the recovered durable tip -- on a restart it is EPOCHS ahead
                        // of the seed epoch, so compute_first_window_bounds returns None ->
                        // EpochViewPostPromotionMismatch). At FirstRun the sidecar's seed point IS the
                        // bootstrap tip, so this stays byte-identical there.
                        seed_point_slot: sidecar.seed_point_slot,
                        seed_point_hash: sidecar.seed_point_hash.clone(),
                        seed_epoch: sidecar.epoch_no,
                        network_magic,
                        nonce: sidecar.epoch_nonce.0.clone(),
                        genesis_hash: sidecar.genesis_hash.clone(),
                        protocol_params_hash: sidecar.protocol_params_hash.clone(),
                        asc: sidecar.active_slots_coeff,
                        replay_scratch_path: resolve_store_dir(cli)?
                            .join("eview-replay-scratch.redb"),
                        next_epoch_bridge: chaindb
                            .get_bootstrap_next_epoch_authority(&sidecar.anchor_fp)
                            .ok()
                            .flatten()
                            .and_then(|b| {
                                ade_ledger::bootstrap_bridge::decode_bootstrap_next_epoch_authority(
                                    &b,
                                )
                                .ok()
                            }),
                        // M1 (B3c): `.ok()` downgrades a missing/unreadable/undecodable rupd to None --
                        // a deliberate mirror of the `next_epoch_bridge` recovery above. A None is NOT
                        // silently accepted: the seed+2 derivation (derive_candidate) FAILS CLOSED on an
                        // absent rupd, so a corrupt sidecar surfaces as a terminal
                        // BootstrapRewardUpdateAbsent at the authority derivation, never a silent zero.
                        bootstrap_reward_delta: chaindb
                            .get_bootstrap_reward_update(&sidecar.anchor_fp)
                            .ok()
                            .flatten()
                            .and_then(|b| {
                                ade_ledger::bootstrap_reward_update::decode_bootstrap_reward_update(
                                    &b,
                                )
                                .ok()
                            }),
                    })
                }
                _ => None,
            };
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
            // CONTINUITY: thread the recovered anchor point + seed-epoch eta0 into the forward-sync
            // state (the SAME values the forge-ON follow uses), so run_node_sync recognises the
            // post-intersection RollBackward(anchor) as an idempotent boundary rewind and validates
            // the header VRF against the recovered nonce, not the snapshot Nonce::ZERO placeholder.
            fwd.recovered_anchor = state.tip.clone();
            fwd.recovered_eta0 = state
                .seed_epoch_consensus_inputs
                .as_ref()
                .map(|s| s.epoch_nonce.clone());
            // CONTINUITY / RO-LIVE-01: a relay-only (forge-OFF) node FOLLOWS the chain when an
            // upstream peer is configured (--peer) -- wire the same LIVE WirePump feed the forge-ON
            // branch uses. Empty --peer keeps the empty source (halts clean). Network magic comes
            // from --network-magic or the committed --network profile.
            let mut source = if !cli.peer_addrs.is_empty() {
                let network_magic = resolve_network_magic(cli)?;
                spawn_live_wire_pump_source(&cli.peer_addrs, network_magic, state.tip.as_ref())
            } else {
                NodeBlockSource::in_memory(Vec::new())
            };
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
                None,
                reduced_checkpoint.as_ref(),
                eview_inputs.as_ref(), // ECA-5: cross-epoch EVIEW activation wired into the relay-only path
                epoch_accumulator.as_ref(),
            )
            .await?;
            eprintln!(
                "ade_node --mode node: relay run loop exited \
                 (recovered/bootstrapped epoch={epoch:?}, tip slot={tip_slot:?}; \
                 forge OFF — no operator keys supplied; {}). NO block produced.",
                if cli.peer_addrs.is_empty() {
                    "NO live peer source wired — halts clean"
                } else {
                    "followed the live peer until shutdown / feed-end"
                }
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
            } = operator_forge::build_operator_forge_material(
                &paths,
                // OP-OPS-04: the recovered durable tip slot anchors the operator
                // KES period (no wall-clock in the deterministic shell; the
                // per-block forge advances the key per forged slot).
                state.tip.as_ref().map(|t| t.slot).unwrap_or(SlotNo(0)),
            )
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
            let era_schedule = recovered_node_schedule(&state, !cli.peer_addrs.is_empty(), rsw_for_cli(cli))?;
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
            // PHASE4-N-AK AK-S2 (DC-NODE-32): thread the already-loaded recovered
            // anchor point (AK-S1 / BootstrapState.tip) into the forward-sync state
            // — the SINGLE anchor authority — so run_node_sync recognises the relay's
            // post-intersection RollBackward(anchor) as an idempotent boundary rewind
            // (a bare anchor is a recovery snapshot, not a stored block). This is the
            // SAME value the wire pump FindIntersects at below; never re-read from the
            // store inside the loop.
            fwd.recovered_anchor = state.tip.clone();
            // PHASE4-N-AN (T-REC-06): thread the recovered seed-epoch eta0 into the
            // forward-sync state (set once, alongside the anchor). On a peer
            // RollBackward the rollback-follow (`apply_chain_event`) overlays it
            // onto the materialize replay chain_dep so rollback replay validates the
            // header VRF against eta0 — the SAME nonce live admit used — instead of
            // the snapshot `Nonce::ZERO` placeholder (replay-equivalence). Sourced
            // from the recovered sidecar, never peer/CLI/wall-clock.
            fwd.recovered_eta0 = state
                .seed_epoch_consensus_inputs
                .as_ref()
                .map(|s| s.epoch_nonce.clone());
            // EPOCH-CONTINUITY-ACTIVATION ECA-2 (DC-EPOCH-14): construct the SEED-derived activation
            // inputs DETERMINISTICALLY from canonical durable state -- never a flag, never a restart
            // CLI/genesis. EVIEW is "configured" IFF the live reduced checkpoint + the v4 cert-state
            // sidecar + a recovered tip are ALL present (the bootstrap built them together);
            // otherwise `None` keeps the path inert (byte-identical). Every field is recovered from
            // the STORE: the seed ledger (cert state), the seed point (the recovered tip), the seed
            // epoch + eta0 + ASC + the consensus-profile hashes (the v4 sidecar, DC-CINPUT-06), the
            // resolved network magic, and a deterministic scratch path (a sibling of the live
            // checkpoint). No wall clock, no peer, no genesis re-read.
            let eview_inputs: Option<crate::epoch_wire::EviewActivationInputs> = match (
                reduced_checkpoint.as_ref(),
                state.seed_epoch_consensus_inputs.as_ref(),
                state.tip.as_ref(),
            ) {
                (Some(_live), Some(sidecar), Some(_tip)) => {
                    let network_magic = resolve_network_magic(cli)?;
                    Some(crate::epoch_wire::EviewActivationInputs {
                        seed_bootstrap_state: state.ledger.clone(),
                        // Warm-start LAYER-4 fix: anchor the recovery's seed→seed+2 window on the
                        // ORIGINAL seed bootstrap point (persisted in the v5 sidecar), NOT `tip` (the
                        // recovered durable tip — on a restart it is EPOCHS AHEAD of the seed epoch,
                        // so compute_first_window_bounds returns None -> EpochViewPostPromotionMismatch).
                        // At FirstRun (node_lifecycle.rs:578) `tip` IS this point; on warm-start it is not.
                        seed_point_slot: sidecar.seed_point_slot,
                        seed_point_hash: sidecar.seed_point_hash.clone(),
                        seed_epoch: sidecar.epoch_no,
                        network_magic,
                        nonce: sidecar.epoch_nonce.0.clone(),
                        genesis_hash: sidecar.genesis_hash.clone(),
                        protocol_params_hash: sidecar.protocol_params_hash.clone(),
                        asc: sidecar.active_slots_coeff,
                        replay_scratch_path: snapshot_dir.join("eview-replay-scratch.redb"),
                        next_epoch_bridge: chaindb
                            .get_bootstrap_next_epoch_authority(&sidecar.anchor_fp)
                            .ok()
                            .flatten()
                            .and_then(|b| {
                                ade_ledger::bootstrap_bridge::decode_bootstrap_next_epoch_authority(
                                    &b,
                                )
                                .ok()
                            }),
                        // M1 (B3c): `.ok()` downgrades a missing/unreadable/undecodable rupd to None --
                        // a deliberate mirror of the `next_epoch_bridge` recovery above. A None is NOT
                        // silently accepted: the seed+2 derivation (derive_candidate) FAILS CLOSED on an
                        // absent rupd, so a corrupt sidecar surfaces as a terminal
                        // BootstrapRewardUpdateAbsent at the authority derivation, never a silent zero.
                        bootstrap_reward_delta: chaindb
                            .get_bootstrap_reward_update(&sidecar.anchor_fp)
                            .ok()
                            .flatten()
                            .and_then(|b| {
                                ade_ledger::bootstrap_reward_update::decode_bootstrap_reward_update(
                                    &b,
                                )
                                .ok()
                            }),
                    })
                }
                _ => None,
            };
            let eview_activation: Option<&crate::epoch_wire::EviewActivationInputs> =
                eview_inputs.as_ref();
            // PHASE4-N-F-G-C S1: wire a LIVE WirePump feed when an upstream peer
            // is configured (`--peer`). Empty `--peer` keeps the prior empty
            // source (forge-CAPABLE, halts clean — the `On` arm is observable
            // only once a live feed is wired, RO-LIVE-01). The live source is a
            // *fill* of the closed `NodeBlockSource::WirePump` arm — no new
            // variant, no second tip-advance, no verdict; dial / parse failures
            // are logged-and-dropped (admission honest-scope C3), never fatal.
            let mut source = if live_feed_wired {
                let network_magic = resolve_network_magic(cli)?;
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
                    let serve_magic = resolve_network_magic(cli)?;
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
            // PHASE4-N-AI AI-S4b-i (OQ-5): declare an explicitly participant
            // venue. INERT here -- only sets venue_role; AI-S4b-ii wires the
            // live fork-choice routing + forge gate that consume it.
            if cli.participant_venue {
                activation.declare_participant_venue();
            }
            // PHASE4-N-AO S6 (CE-AO-6): the magic to live-BlockFetch a winning
            // branch from the winning peer (prefetch_branch_bodies). Absent it,
            // a fork-choice win is held by NullBranchBodySource (the fence).
            activation.network_magic = cli.network_magic;
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
            // PHASE4-N-AJ AJ-S2 (DC-NODE-30): build the convergence-evidence
            // context. Disabled sink when --convergence-evidence-path is absent
            // (no file; consensus + existing logs unchanged). Oracle binding = the
            // imported bundle fingerprint (canonical.fingerprint, DC-ADMIT-10
            // parity) when the convergence pass supplies --consensus-inputs-path,
            // else the recovered-oracle ledger fingerprint.
            let mut convergence = {
                let sink = ConvergenceEvidenceSink::open(cli.convergence_evidence_path.as_deref())
                    .map_err(|e| {
                        NodeLifecycleError::ChainDbOpen(format!(
                            "convergence-evidence: {:?}",
                            e.kind()
                        ))
                    })?;
                let fp: Hash32 = cli
                    .convergence_evidence_path
                    .as_ref()
                    .and(cli.consensus_inputs_path.as_ref())
                    .and_then(|p| import_live_consensus_inputs(p).ok())
                    .map(|c| c.fingerprint)
                    .unwrap_or_else(|| fingerprint(&fwd.receive.ledger).combined);
                ConvergenceEvidence::new(sink, &fp)
            };
            // MEM-MEASURE-A2 (OP-MEM-01): idle recovered-tip + post-recovery memory
            // samples, before the relay loop consumes any peer block. Observe-only --
            // RSS never feeds authority; the sample is skipped off-Linux.
            {
                let tip_slot = fwd.recovered_anchor.as_ref().map(|t| t.slot.0).unwrap_or(0);
                let ledger_fp = fingerprint(&fwd.receive.ledger).combined;
                convergence.emit_memory_measure(
                    "wal_checkpoint_recovery",
                    tip_slot,
                    tip_slot,
                    &ledger_fp,
                );
                convergence.emit_memory_measure("idle_recovered_tip", tip_slot, tip_slot, &ledger_fp);
            }
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
                Some(&mut convergence),
                reduced_checkpoint.as_ref(),
                eview_activation,
                epoch_accumulator.as_ref(),
            )
            .await?;
            // MEM-MEASURE-A2 (OP-MEM-01): final sustained sample + run-level memory
            // summary. The loop returned Ok, so the run completed with no fatal Diverged
            // halt -> the durable chain is replay-equivalent by the enforced DC-WAL-03
            // (replay verdict `agreed`). Observe-only.
            {
                let tip_slot = chaindb.tip().ok().flatten().map(|t| t.slot.0).unwrap_or(0);
                let ledger_fp = fingerprint(&fwd.receive.ledger).combined;
                convergence.emit_memory_measure("sustained", tip_slot, tip_slot, &ledger_fp);
                convergence.emit_memory_summary("agreed");
            }
            // PHASE4-N-AJ AJ-S2 (DC-NODE-30 / G1): a sink write failure poisons the
            // transcript -- non-fatal to authority, but the operator must NOT commit
            // an incomplete transcript for CE-AI-6.
            if convergence.is_incomplete() {
                eprintln!(
                    "ade_node --mode node: convergence-evidence transcript INCOMPLETE \
                     (a sink write failed) -- do NOT commit it for CE-AI-6."
                );
            }
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
/// PHASE4-N-AK AK-S1 (DC-NODE-31): the FindIntersect start point for the live
/// wire pump. A recovered live-follow tip — the `BootstrapState.tip` that
/// `resolve_live_follow_start` produced (a servable ChainDb tip OR the persisted
/// recovered anchor) — becomes a `Point::Block`; only a truly Origin /
/// cold-start (`None`) starts from `Point::Origin`. Behavior-IDENTICAL to the
/// prior inline match, extracted so the start-point choice is a single testable
/// authority (CE-AK-2): a bare-anchor recovery now passes `Some(anchor)` here,
/// so the pump FindIntersects at the anchor, not Origin. The wire pump's
/// dial / transmit / receive behavior is otherwise UNCHANGED.
fn wire_pump_start_point(recovered_tip: Option<&ChainTip>) -> ade_network::codec::chain_sync::Point {
    match recovered_tip {
        Some(t) => ade_network::codec::chain_sync::Point::Block {
            slot: t.slot,
            hash: t.hash.clone(),
        },
        None => ade_network::codec::chain_sync::Point::Origin,
    }
}

fn spawn_live_wire_pump_source(
    peer_addrs: &[String],
    network_magic: u32,
    recovered_tip: Option<&ChainTip>,
) -> NodeBlockSource {
    let our_versions = build_n2n_version_table(network_magic);
    let start_point = wire_pump_start_point(recovered_tip);
    // PHASE4-N-AO S8 (DC-PUMP-04): the merged feed the `WirePump` consumer reads is
    // UNCHANGED in shape (one peer-attributed event sequence). Below it, each peer
    // now gets its OWN bounded lane drained by a fair round-robin merge — a hot peer
    // fills only its own lane (self-backpressure) and can no longer starve the
    // others off the participant path (the gap the S7 live retry surfaced).
    let (merged_tx, merged_rx) = mpsc::channel::<AdmissionPeerEvent>(LIVE_WIRE_PUMP_CHANNEL_CAP);
    // Per-peer lanes in a DETERMINISTIC order derived from the configured `--peer`
    // list (an explicit `Vec` — never HashMap/HashSet iteration, never scheduler
    // timing). The lane order is RED scheduling OPPORTUNITY only; it never decides
    // fork-choice (select_best_chain stays arrival-order independent, CN-CONS-01).
    let mut lanes: Vec<Option<mpsc::Receiver<AdmissionPeerEvent>>> = Vec::new();
    for raw_addr in peer_addrs {
        let addr: std::net::SocketAddr = match raw_addr.parse() {
            Ok(a) => a,
            Err(_) => {
                eprintln!("ade_node --mode node: skipping unparseable --peer addr {raw_addr}");
                continue;
            }
        };
        let (lane_tx, lane_rx) = mpsc::channel::<AdmissionPeerEvent>(PER_PEER_LANE_CAP);
        lanes.push(Some(lane_rx));
        let pump_versions = our_versions.clone();
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
                run_admission_wire_pump(transport, label, start, version, network_magic, lane_tx)
                    .await;
        });
    }
    // RED fair-merge: round-robin the per-peer lanes into the single merged feed.
    // No peer parsed → empty lanes → the merge ends immediately → the feed ends →
    // the relay loop halts clean (the same outcome as the prior empty source).
    tokio::spawn(fair_merge(lanes, merged_tx));
    NodeBlockSource::from_wire_pump(merged_rx)
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
    /// PHASE4-N-AI AI-S4b-ii (DC-NODE-28): a fork-choice re-selection (rollback
    /// apply) is in flight. Set before `apply_chain_event`, cleared only after it
    /// returns. The ForgeTick gate refuses while set — no forge on a stale
    /// pre-resolution tip (the producer race).
    pub pending_reselection: bool,
    /// PHASE4-N-AO S3 (DC-NODE-36): the block-count rollback security parameter k
    /// the live `select_best_chain` dispatch uses for the `rollback_depth <= k`
    /// eligibility bound. Cardano k (preprod/mainnet 2160) by default — matching
    /// the hardcoded `make_node_schedule` window; an explicit venue (e.g. the
    /// CE-AO-6 two-producer venue) overrides it post-construction. Durable/config
    /// authority, NEVER peer-supplied; S4's `materialize` keeps the final,
    /// independent `RollbackTooDeep` authority.
    pub security_param: SecurityParam,
    /// PHASE4-N-AO S3 (DC-NODE-36): the PROVISIONAL fork-choice decision the live
    /// participant dispatch emits on a `select_best_chain` win — consumed by S4
    /// (latent until then). S3 sets this + `pending_reselection` but applies
    /// nothing (no rollback-commit, no body-fetch). `None` => no pending switch.
    pub pending_fork_switch: Option<PendingForkSwitch>,
    /// PHASE4-N-AO S11 (DC-NODE-39): a post-`ForkChoiceWin` competing descendant
    /// could not be bridged to the durable adopted tip / a durable stored ancestor
    /// within k. Set (with the closed reason) by the dispatch on the walk-fail /
    /// materialize-fail paths that pre-S11 SILENTLY no-op'd; HOLDS the forge fence
    /// (`fork_switch_fence_resolved` refuses while it is `Some`); cleared on forward
    /// progress (a successful `LinearExtend` admit or a proven fork-switch adoption)
    /// so it is a HOLD-until-progress, not a permanent halt. NEVER an adoption path,
    /// a rollback target, or a reason to admit the un-bridgeable block. In-memory,
    /// not persisted, not replay-visible.
    pub pending_missing_bridge: Option<MissingBridgeReason>,
    /// PHASE4-N-AO S13 (DC-NODE-40): walk-visible EVIDENCE of the blocks Ade itself
    /// rolled back during a `ForkChoiceWin` adoption (admitted `LinearExtend`, so
    /// never in the competing-only S7 branch cache). Populated by `apply_fork_switch`
    /// BEFORE the rollback; consulted by `walk_to_durable_lca` on a per-peer-cache
    /// miss so a competing branch that descends through Ade's own rolled-back chain
    /// stays EVALUABLE (fork-choice resolves it) instead of a false `BranchGap` ->
    /// `MissingBridge` over-fire. Cross-iteration (lives in the fork-switch lifecycle
    /// state, beside `pending_*`). EVIDENCE, not authority: k-bounded (block depth),
    /// hash-keyed `BTreeMap` (self-binding, never HashMap-iterated for ordering);
    /// NEVER durable, the LCA anchor, a rollback target, or a bypass of S2/S4.
    pub rollback_retention: BTreeMap<Hash32, CachedHeader>,
    /// PHASE4-N-AO S4 (DC-NODE-37): the last fork-switch proof failure (a structured
    /// LOCAL observation surface — in-memory, not persisted, not evidence — so a
    /// failed/lying/incomplete replacement branch is never a silent drop). Set when
    /// `apply_fork_switch` could not prove the branch; cleared on a proven adoption.
    pub last_fork_switch_failure: Option<BranchProofError>,
    /// PHASE4-N-AO S6 (CE-AO-6): the network magic used to dial the winning peer for
    /// the live `BlockFetch` of a winning branch (`prefetch_branch_bodies`). `None`
    /// (test / forge-off / no `--network-magic`) => no live fetch; a win is held by
    /// `NullBranchBodySource` (the fence stays set). The fetch is byte-only; S4
    /// prevalidates regardless.
    pub network_magic: Option<u32>,
    /// PHASE4-N-AO S14 (DC-NODE-41): the post-`ForkChoiceWin` follow target -- the
    /// winning peer + adopted tip + fork_switch_id, recorded on a proven adoption.
    /// CONSULTED (read-only) by the dispatch to decide whether a `MissingBridge` for
    /// a winning-peer descendant is ELIGIBLE for active range re-fetch. RECOVERY
    /// state, NEVER selection authority (S3 already decided the winner). In-memory,
    /// not persisted, not replay-visible.
    pub post_switch_follow: Option<PostSwitchFollow>,
    /// PHASE4-N-AO S14 (DC-NODE-41): a pending active range re-fetch the dispatch set
    /// on an ELIGIBLE winning-peer descendant `MissingBridge` (the DC-NODE-39 floor
    /// hold remains set ALONGSIDE it). Consumed by the relay loop: bounded-retry
    /// `prefetch_branch_bodies` -> `recover_missing_range` (byte-only fetch, BLUE
    /// `pump_block` is the sole admit), clearing the missing-bridge hold ONLY on real
    /// admitted progress. A short / lying / unservable range leaves the floor hold.
    /// In-memory, not persisted, not replay-visible.
    pub pending_range_refetch: Option<RangeRefetch>,
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
            pending_reselection: false,
            // Cardano k (preprod/mainnet). An explicit two-producer venue overrides
            // this post-construction; never peer-supplied (DC-NODE-36).
            security_param: SecurityParam(2160),
            pending_fork_switch: None,
            pending_missing_bridge: None,
            rollback_retention: BTreeMap::new(),
            last_fork_switch_failure: None,
            network_magic: None,
            post_switch_follow: None,
            pending_range_refetch: None,
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

    /// PHASE4-N-AI AI-S4b-i (OQ-5): declare an explicitly participant venue.
    /// INERT until AI-S4b-ii wires the live fork-choice routing -- it only sets
    /// the role; no existing live consumer branches on `Participant` yet, so the
    /// loop reaches the same fallback as `Unknown` until then. `Participant` is a
    /// distinct declared venue, NOT semantically `Unknown`.
    pub fn declare_participant_venue(&mut self) {
        self.venue_role = VenueRole::Participant;
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
    seed_view: &PoolDistrView,
    shutdown: &mut watch::Receiver<bool>,
    forge: Option<&mut ForgeActivation<'_>>,
) -> Result<(), NodeLifecycleError> {
    run_relay_loop_with_sched(
        state, source, chaindb, wal, era_schedule, seed_view, shutdown, forge, None, None, None,
        None, None,
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
        ForgeMode::ParticipantExtendOnSelectedHead { .. } => {
            ForgeModeKind::ParticipantExtendOnSelectedHead
        }
    }
}

/// EPOCH-CONSENSUS-VIEW S3f-4d-mat-2c / LIVE-LEDGER-EPOCH-TRANSITION S3 (DC-EPOCH-11 / DC-EPOCH-22):
/// advance the live reduced checkpoint FORWARD to `target_slot` over the durable ChainDB. No reorg reset
/// here -- the co-advancer hoists the reset so every segment is purely forward, and idempotent-resume
/// makes folding seed->s_prev->tip in segments byte-identical to seed->tip in one shot. No-op when EVIEW
/// is not configured (`None`). FAIL-CLOSED: a malformed (unsealed) checkpoint or an advance fault leaves
/// the checkpoint at its last good slot and propagates, so EpochConsensusView never produces from a
/// stale/partial checkpoint.
fn advance_reduced_checkpoint_forward_to(
    reduced_checkpoint: Option<&ade_runtime::chaindb::ReducedUtxoCheckpoint>,
    chaindb: &dyn ChainDb,
    target_slot: SlotNo,
) -> Result<(), NodeLifecycleError> {
    let Some(cp) = reduced_checkpoint else {
        return Ok(());
    };
    // A LIVE checkpoint MUST carry its sealed seed slot (the build calls seal_bootstrap). A
    // present-but-unsealed checkpoint is malformed -- advancing it from slot 0 would re-apply
    // blocks already folded into the seed UTxO. FAIL-CLOSED rather than corrupt.
    let seed_slot = cp
        .seed_slot()
        .map_err(|e| NodeLifecycleError::RelaySync(format!("reduced-checkpoint seed slot: {e:?}")))?
        .ok_or_else(|| {
            NodeLifecycleError::RelaySync(
                "reduced checkpoint has no sealed bootstrap baseline (malformed)".to_string(),
            )
        })?;
    ade_runtime::chaindb::advance_reduced_checkpoint_over_chaindb(
        cp,
        chaindb,
        seed_slot,
        target_slot,
        ade_types::CardanoEra::Conway,
    )
    .map_err(|e| NodeLifecycleError::RelaySync(format!("reduced-checkpoint advance: {e:?}")))
}

/// S3f-4d-mat-3 (DC-EPOCH-11): reorg reset for the reduced checkpoint. If the checkpoint advanced PAST
/// the current durable tip, a rollback shortened the chain -- re-materialize to the sealed seed baseline
/// (the reduced delta is not invertible); the forward advance then replays from seed+1. Fail-closed
/// (malformed seed / reset fault). Hoisted out of the forward advance so the co-advancer's segmented walk
/// is purely forward.
fn reduced_checkpoint_reset_if_ahead(
    reduced_checkpoint: Option<&ade_runtime::chaindb::ReducedUtxoCheckpoint>,
    tip: &ChainTip,
) -> Result<(), NodeLifecycleError> {
    let Some(cp) = reduced_checkpoint else {
        return Ok(());
    };
    let seed_slot = cp
        .seed_slot()
        .map_err(|e| NodeLifecycleError::RelaySync(format!("reduced-checkpoint seed slot: {e:?}")))?
        .ok_or_else(|| {
            NodeLifecycleError::RelaySync(
                "reduced checkpoint has no sealed bootstrap baseline (malformed)".to_string(),
            )
        })?;
    let advanced = cp
        .last_advanced_slot()
        .map_err(|e| NodeLifecycleError::RelaySync(format!("reduced-checkpoint slot: {e:?}")))?
        .unwrap_or(seed_slot);
    if advanced.0 > tip.slot.0 {
        cp.reset_to_bootstrap().map_err(|e| {
            NodeLifecycleError::RelaySync(format!("reduced-checkpoint re-materialize: {e:?}"))
        })?;
    }
    Ok(())
}

/// LIVE-LEDGER-EPOCH-TRANSITION S2/S3 (DC-EPOCH-20 / PO-4): reorg reset for the durable EpochAccumulator,
/// OBSERVE-ONLY. If the accumulator advanced PAST the durable tip -> a rollback shortened the chain;
/// rematerialize to the sealed seed baseline (the within-epoch delta is not invertible), and the forward
/// fold replays from seed+1. A reset fault is swallowed (the accumulator is non-authoritative in S3) -- a
/// still-ahead accumulator simply folds nothing on the forward walk. Skipped if unsealed (a present-but-
/// unsealed store is malformed; the fold's skip-if-unsealed handles it too). Hoisted to the co-advancer's
/// top so the segmented walk is purely forward.
///
/// S4 OBLIGATION (S2 IDD review, MEDIUM-2): this HEIGHT check (advanced > tip) suffices ONLY because the
/// sole driver is the fail-closed, forward-only run_node_sync path (every non-anchor rollback fail-closes
/// before this is reached). A later reorging fork-choice / participant driver MUST replace it with a
/// LINEAGE check (the durable hash at last_advanced) -- a longer chain diverging BELOW last_advanced is
/// height-invisible and would fold a new suffix onto a stale prefix (a split-lineage accumulator).
fn accumulator_reset_if_ahead(
    epoch_accumulator: Option<&ade_runtime::chaindb::EpochAccumulatorStore>,
    tip: &ChainTip,
) {
    let Some(store) = epoch_accumulator else {
        return;
    };
    let Ok(Some(seed_slot)) = store.seed_slot() else {
        return;
    };
    let advanced = store
        .last_advanced_slot()
        .ok()
        .flatten()
        .unwrap_or(seed_slot);
    if advanced.0 > tip.slot.0 {
        let _ = store.reset_to_bootstrap();
    }
}

/// LIVE-LEDGER-EPOCH-TRANSITION S3 (DC-EPOCH-22, BOUNDARY-ALIGNED-MARK-CAPTURE): the co-advancer called
/// after each durable admit. It reconciles BOTH derived stores -- the EVIEW reduced checkpoint and the
/// durable EpochAccumulator -- to the durable ChainDB tip in ONE pass that SEGMENTS at each epoch boundary.
///
/// The accumulator's within-epoch fold STALLS at a boundary block `s_bb` with its cursor left at `s_prev`
/// (the last within-epoch block of the closing epoch). To cross, it needs the SNAP stake mark captured at
/// the EXACT boundary point `s_prev` -- never the post-pass tip (byte-wrong: catch-up is already past the
/// boundary; even steady-state's tip is the FIRST block of the new epoch, whose UTxO delta must NOT be in
/// the mark). So at each stall this advances the reduced checkpoint EXACTLY to `s_prev`, captures
/// `sum_base_credential_stake()` there, durably binds the BoundaryMark witness (point + lineage) BEFORE
/// the cross, then crosses the accumulator over `s_bb` with that mark; the loop resumes folding the new
/// epoch (so multi-boundary catch-up crosses every boundary in one call).
///
/// TWO fault classes: the reduced-checkpoint advances are FAIL-CLOSED (`?` -- a checkpoint I/O fault is a
/// real EVIEW problem that halts the follow); every ACCUMULATOR operation (fold / capture / bind / cross)
/// is OBSERVE-ONLY (log + stop, never halt) -- S3 keeps the accumulator non-authoritative (S4 flips it).
/// Regardless of the accumulator outcome the checkpoint is GUARANTEED to reach the durable tip (EVIEW
/// currency: `maybe_activate_epoch_boundary` reads it there). With `epoch_accumulator = None` this reduces
/// to the pre-S3 reduced-checkpoint-reset-then-advance-to-tip (byte-identical).
fn advance_ledger_state_to_durable_tip(
    reduced_checkpoint: Option<&ade_runtime::chaindb::ReducedUtxoCheckpoint>,
    epoch_accumulator: Option<&ade_runtime::chaindb::EpochAccumulatorStore>,
    chaindb: &dyn ChainDb,
    era_schedule: &EraSchedule,
) -> Result<(), NodeLifecycleError> {
    use ade_runtime::chaindb::{
        advance_accumulator_over_chaindb, cross_accumulator_over_boundary_block,
        AccumulatorBoundaryOutcome, AccumulatorChaindbOutcome,
    };

    let Some(tip) = chaindb
        .tip()
        .map_err(|e| NodeLifecycleError::RelaySync(format!("ledger-advance tip read: {e:?}")))?
    else {
        return Ok(());
    };

    // Hoist the reorg reset for BOTH stores so the segmented walk below is purely forward.
    reduced_checkpoint_reset_if_ahead(reduced_checkpoint, &tip)?;
    accumulator_reset_if_ahead(epoch_accumulator, &tip);

    // The boundary-segmented accumulator cross loop (observe-only). Skipped when no accumulator is
    // configured -> the EVIEW-only advance below is byte-identical to the pre-S3 path.
    if let Some(store) = epoch_accumulator {
        // Skip-if-unsealed: a present-but-unsealed store is malformed (never fold from slot 0 over a seed
        // that already absorbed those blocks). The checkpoint still reaches tip below.
        if let Ok(Some(seed_slot)) = store.seed_slot() {
            loop {
                match advance_accumulator_over_chaindb(
                    store,
                    chaindb,
                    era_schedule,
                    seed_slot,
                    tip.slot,
                ) {
                    Ok(AccumulatorChaindbOutcome::ReachedTip { .. }) => break,
                    Ok(AccumulatorChaindbOutcome::StalledAt { slot: s_bb, reason }) => {
                        // s_prev: the accumulator's cursor after the within-epoch fold -- the boundary point
                        // (the last within-epoch block of the closing epoch).
                        let s_prev = match store.last_advanced_slot() {
                            Ok(Some(s)) => s,
                            _ => {
                                crate::node_log!(
                                    "epoch-accumulator: boundary at {} but no durable cursor (observe-only stall): {}",
                                    s_bb.0,
                                    reason
                                );
                                break;
                            }
                        };
                        let Some(cp) = reduced_checkpoint else {
                            crate::node_log!(
                                "epoch-accumulator: boundary at {} but no reduced checkpoint -> observe-only stall: {}",
                                s_bb.0,
                                reason
                            );
                            break;
                        };
                        // FAIL-CLOSED (EVIEW): bring the checkpoint EXACTLY to the boundary point so the mark
                        // is the end-of-epoch stake, before the new epoch's first block.
                        advance_reduced_checkpoint_forward_to(Some(cp), chaindb, s_prev)?;
                        // Capture the per-credential SNAP mark at s_prev (observe-only on a sum fault).
                        let mark = match cp.sum_base_credential_stake() {
                            Ok(m) => m,
                            Err(e) => {
                                crate::node_log!(
                                    "epoch-accumulator: boundary mark capture at {} failed (observe-only): {:?}",
                                    s_prev.0,
                                    e
                                );
                                break;
                            }
                        };
                        // The boundary point's canonical lineage hash (observe-only on a missing/failed read).
                        let boundary_hash = match chaindb.get_block_by_slot(s_prev) {
                            Ok(Some(b)) => b.hash,
                            Ok(None) => {
                                crate::node_log!(
                                    "epoch-accumulator: boundary point {} has no durable block (observe-only stall)",
                                    s_prev.0
                                );
                                break;
                            }
                            Err(e) => {
                                crate::node_log!(
                                    "epoch-accumulator: boundary point {} hash read failed (observe-only): {:?}",
                                    s_prev.0,
                                    e
                                );
                                break;
                            }
                        };
                        // DURABLE: bind the witness (point + lineage) BEFORE the cross -- a crash here recovers
                        // the binding and the cross re-derives + crosses (DC-EPOCH-22).
                        if let Err(e) = store.bind_boundary_mark(s_prev, &boundary_hash) {
                            crate::node_log!(
                                "epoch-accumulator: boundary mark bind at {} failed (observe-only): {:?}",
                                s_prev.0,
                                e
                            );
                            break;
                        }
                        match cross_accumulator_over_boundary_block(
                            store,
                            chaindb,
                            era_schedule,
                            s_bb,
                            &mark,
                        ) {
                            Ok(AccumulatorBoundaryOutcome::Crossed {
                                from_epoch,
                                to_epoch,
                                slot,
                            }) => {
                                let _ = store.clear_boundary_mark();
                                // Observable proof of self-derived ledger continuity across a boundary
                                // (CE-3c): the mark was captured at the boundary point s_prev, not the tip.
                                crate::node_log!(
                                    "epoch-accumulator: CROSSED boundary {} -> {} at slot {} (mark from s_prev {})",
                                    from_epoch.0,
                                    to_epoch.0,
                                    slot.0,
                                    s_prev.0
                                );
                                // Loop: resume the within-epoch fold in the new epoch (s_bb+1 onward).
                            }
                            Ok(AccumulatorBoundaryOutcome::AlreadyCrossed { .. }) => {
                                // Idempotent re-entry (already crossed) -- silent; loop to resume folding.
                                let _ = store.clear_boundary_mark();
                            }
                            Ok(AccumulatorBoundaryOutcome::Stalled { slot, reason }) => {
                                crate::node_log!(
                                    "epoch-accumulator: boundary cross stalled at {} (observe-only): {}",
                                    slot.0,
                                    reason
                                );
                                break;
                            }
                            Err(e) => {
                                crate::node_log!(
                                    "epoch-accumulator: boundary cross fault (observe-only): {:?}",
                                    e
                                );
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        // S4 OBLIGATION (S2 IDD review, MEDIUM-1): swallowing a REAL fault is IDD-§8-compliant
                        // ONLY while the accumulator is non-authoritative + readiness-gated. At the S4
                        // authority flip this Err arm MUST halt (swallow only stalls).
                        crate::node_log!(
                            "epoch-accumulator: within-epoch reconcile fault (observe-only): {:?}",
                            e
                        );
                        break;
                    }
                }
            }
        }
    }

    // GUARANTEE the EVIEW checkpoint reaches the durable tip (fail-closed), regardless of the accumulator
    // outcome. Forward-only -- the reorg reset was hoisted above.
    advance_reduced_checkpoint_forward_to(reduced_checkpoint, chaindb, tip.slot)?;
    Ok(())
}

/// EPOCH-CONTINUITY-ACTIVATION ECA-1 (DC-EPOCH-13): the first-boundary epoch-view activation,
/// called after each durable admit. AUTOMATIC -- no arming flag. A strict NO-OP (byte-identical)
/// unless EVIEW is configured (`eview_activation` + `reduced_checkpoint` both `Some` = canonical
/// durable state present) AND the seed epoch has completed. The SOLE authoritative derive is the
/// durable window replay; the live checkpoint is the readiness witness, never the derive source.
/// A terminal `ActivationError` halts the loop.
fn maybe_activate_epoch_boundary(
    eview_activation: Option<&crate::epoch_wire::EviewActivationInputs>,
    reduced_checkpoint: Option<&ade_runtime::chaindb::ReducedUtxoCheckpoint>,
    chaindb: &PersistentChainDb,
    era_schedule: &mut EraSchedule,
    wal: &mut FileWalStore,
    authority: &mut crate::epoch_activation::ActiveEpochAuthority,
) -> Result<(), NodeLifecycleError> {
    let (Some(inputs), Some(live)) = (eview_activation, reduced_checkpoint) else {
        return Ok(());
    };
    let Some(tip) = chaindb
        .tip()
        .map_err(|e| NodeLifecycleError::RelaySync(format!("eview activation tip: {e:?}")))?
    else {
        return Ok(());
    };
    let selected_point = ade_core::consensus::events::Point {
        slot: tip.slot,
        hash: tip.hash.clone(),
    };
    let scratch = inputs.replay_scratch_path.clone();
    let outcome = inputs
        .maybe_activate(
            era_schedule,
            tip.slot,
            live,
            chaindb,
            &selected_point,
            authority,
            &scratch,
            |entry| wal.append(entry.clone()).is_ok(),
        )
        .map_err(|e| NodeLifecycleError::RelaySync(format!("eview activation: {e:?}")))?;
    // ECA-3 (DC-EPOCH-14): the authority is promoted IN PLACE (the atomic Seed->Promoted swap) — both
    // header validation AND leadership now resolve the promoted N+1 view from this ONE holder. The
    // outcome (Promoted / NotYet) is evidence only; the mutation of `authority` is the effect.
    // ECA-5 (DC-EPOCH-15): same transition -- the authority just promoted in place; atomically extend the
    // owned forecast schedule to cover its epoch so downstream header validation admits the post-boundary slot.
    extend_schedule_to_epoch(era_schedule, authority.epoch());
    let _ = outcome;
    Ok(())
}

/// ECA-5 (DC-EPOCH-15): extend the forecast horizon to match the promoted authority's epoch. DERIVED
/// state -- each appended EraSummary for epoch e is a pure function of the seed-epoch geometry (the
/// schedule's FIRST summary): start_slot = seed.start_slot + (e - seed.start_epoch) * epoch_length, with
/// the same era/slot_length/epoch_length/safe_zone. Idempotent (a no-op unless the authority's epoch
/// exceeds the schedule's last summary) and gap-filling (appends every intermediate epoch), so a live
/// per-boundary append and a warm-start single reconstruction yield byte-identical summaries. No
/// flag/clock/peer input -- the horizon extends ONLY after (and to match) a durable authority promotion.
pub(crate) fn extend_schedule_to_epoch(era_schedule: &mut EraSchedule, target: EpochNo) {
    // Delegates to the single shared definition on EraSchedule (ade_core): the live follow and the
    // warm-start replay path MUST extend the forecast horizon identically, so there is exactly ONE
    // copy of this logic (no second convention that can drift -- the credential-decoder lesson).
    era_schedule.extend_to_epoch(target);
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
    // EPOCH-CONTINUITY-ACTIVATION ECA-3 (DC-EPOCH-14): the recovered SEED PoolDistrView. The loop
    // owns an `ActiveEpochAuthority` over it — the SOLE view source for BOTH header validation AND
    // leadership; at the boundary it atomically swaps Seed->Promoted. (Was `ledger_view: &dyn
    // LedgerView`; the borrowed seed view is unchanged until a promotion, so this is byte-identical
    // until the swap is wired.)
    seed_view: &PoolDistrView,
    shutdown: &mut watch::Receiver<bool>,
    mut forge: Option<&mut ForgeActivation<'_>>,
    mut sched: Option<&mut dyn crate::live_log::NodeSchedSink>,
    // PHASE4-N-AJ AJ-S2 (DC-NODE-30): emit-only convergence evidence, threaded to
    // the Participant receive path. `None` on the forge-off / wrapper / test
    // callers. Evidence observes authority; it never becomes authority.
    mut convergence: Option<&mut ConvergenceEvidence>,
    // EPOCH-CONSENSUS-VIEW S3f-4d-mat-2c (DC-EPOCH-11): the live reduced-UTxO checkpoint,
    // `Some` ONLY when the EVIEW activation is configured (the bootstrap built it). After each
    // durable admit the loop advances it to the ChainDB tip (replay-equivalent, fail-closed).
    // `None` on non-EVIEW / wrapper / test callers -> the follow/forge path is byte-identical.
    reduced_checkpoint: Option<&ade_runtime::chaindb::ReducedUtxoCheckpoint>,
    // EPOCH-CONTINUITY-ACTIVATION ECA-1 (DC-EPOCH-13): the SEED-derived activation inputs, `Some`
    // ONLY when EVIEW is configured (canonical durable state). The loop runs the AUTOMATIC
    // first-boundary activation (no arming flag; the only gate is the deterministic predicate over
    // durable state) after each admit. `None` on non-EVIEW / wrapper / test callers -> inert
    // (byte-identical).
    eview_activation: Option<&crate::epoch_wire::EviewActivationInputs>,
    // LIVE-LEDGER-EPOCH-TRANSITION S2 (DC-EPOCH-20): the durable non-UTxO accumulator, `Some` when a
    // native bootstrap sealed it (or a warm start reopened it). After each durable admit the loop
    // advances it OBSERVE-ONLY (the accumulator is not yet authoritative; S4 flips it), so a stall /
    // fault never affects the follow. `None` on non-native / wrapper / test callers -> inert.
    epoch_accumulator: Option<&ade_runtime::chaindb::EpochAccumulatorStore>,
) -> Result<(), NodeLifecycleError> {
    // ECA-3 (DC-EPOCH-14): the ONE owned epoch-authority the loop holds — the SOLE view source for
    // BOTH header validation and leadership. Resolved FRESH at each authoritative decision via
    // `authority.ledger_view()` / `authority.pool_distr_view()` (never retained across the swap);
    // promoted IN PLACE at the boundary by `maybe_activate_epoch_boundary` (the atomic Seed->Promoted).
    // Its CANONICAL mode is established from durable state -- NOT an ambient flag: EVIEW configured
    // (the activation inputs are present = the reduced checkpoint + the v4 consensus-profile sidecar
    // are bound durably) => ContinuityRequired (a missing N+1 promotion is terminal); otherwise
    // SeedOnly (a limited producer that no-forges past its seed epoch but KEEPS FOLLOWING). The mode
    // is the SAME on warm-start (the inputs are recovered from the store, never CLI/genesis).
    // ECA-5 (DC-EPOCH-15): own the forecast schedule so it can be extended in place at a boundary and
    // atomically replace the loop's owned copy -- no shared mutable reference can leave validation on
    // the old horizon after promotion. The caller passes the seed-epoch schedule by ref; the loop holds
    // the authoritative owned copy, extended ONLY when the authority promotes (+ on warm-start recovery).
    let mut era_schedule = era_schedule.clone();
    let mut authority = match eview_activation {
        Some(inputs) => crate::epoch_activation::ActiveEpochAuthority::continuity(
            seed_view,
            ade_core::consensus::events::Point {
                slot: inputs.seed_point_slot,
                hash: inputs.seed_point_hash.clone(),
            },
            ade_ledger::reduced_epoch_view::consensus_profile_commitment(
                &inputs.genesis_hash,
                &inputs.protocol_params_hash,
                inputs.asc,
            ),
            crate::epoch_activation::TargetEpochPolicy::SetSnapshotLag {
                lag_epochs: crate::epoch_source_window::LEADERSHIP_SNAPSHOT_LAG_EPOCHS as u32,
            },
        ),
        None => crate::epoch_activation::ActiveEpochAuthority::seed(seed_view),
    };
    // Phase 4 (ECA-4, DC-EPOCH-06 recovery exactness): BEFORE the loop, if a durable activation record
    // exists, recover the promoted authority from the VERIFIED record (re-derive via the SAME window
    // replay + reject-non-recomputable) — so a restart AFTER a promotion starts from the recorded N+1
    // view (criteria 4/5), never a stale seed. The live first-boundary re-fire is then idempotent. A
    // None record (crash before the WAL) keeps Seed; a record whose candidate cannot be RECOMPUTED
    // identically is a TERMINAL halt (never trust a parsed record alone, never fall back to the seed).
    if let (Some(inputs), Some(live)) = (eview_activation, reduced_checkpoint) {
        let entries = wal
            .read_all()
            .map_err(|e| NodeLifecycleError::RelaySync(format!("eview recovery WAL read: {e:?}")))?;
        let resolved = crate::epoch_activation::resolve_activation_record(&entries)
            .map_err(|e| NodeLifecycleError::RelaySync(format!("eview recovery resolve: {e:?}")))?;
        crate::epoch_wire::maybe_recover_promoted_authority(
            resolved.as_ref(),
            &era_schedule,
            inputs.seed_epoch,
            inputs.seed_point_slot,
            inputs.seed_point_hash.clone(),
            live,
            chaindb,
            &inputs.seed_bootstrap_state,
            inputs.network_magic,
            // Layer 4 (DC-EPOCH-06): the recovery re-derives the LATEST record's epoch view, whose
            // eta0 is the durable tip's epoch nonce -- NOT the seed sidecar nonce (`inputs.nonce`).
            // The replay-forward reconstructed that nonce into the recovered chain_dep (replay-derived
            // from the durable blocks, independent of the record), so feeding it and then comparing the
            // re-derived candidate against the record stays reject-non-recomputable.
            state.receive.chain_dep.epoch_nonce.0.clone(),
            inputs.genesis_hash.clone(),
            inputs.protocol_params_hash.clone(),
            inputs.asc,
            inputs.bootstrap_reward_delta.as_ref(),
            inputs.next_epoch_bridge.as_ref(),
            &mut authority,
            &inputs.replay_scratch_path,
        )
        .map_err(|e| NodeLifecycleError::RelaySync(format!("eview recovery: {e:?}")))?;
    }
    // ECA-5 (DC-EPOCH-15): warm-start forecast reconstruction. If the recovery promoted the authority
    // to N+1 (or beyond), extend the owned schedule to match -- deriving the SAME summaries the live
    // per-boundary append produced (byte-identical). A no-op when the recovery kept the seed.
    extend_schedule_to_epoch(&mut era_schedule, authority.epoch());
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
                // PHASE4-N-AI AI-S4b-ii: an explicitly-declared Participant venue
                // routes the live receive path through the fork-choice follow
                // (detector + rollback-apply); every other venue keeps the
                // verbatim extend-only run_node_sync path.
                let is_participant = forge
                    .as_deref()
                    .map(|a| a.venue_role == VenueRole::Participant)
                    .unwrap_or(false);
                if is_participant {
                    // AI-S6 (Sec W-3): fail closed with a typed error rather than
                    // panic if a Participant venue lacks a forge activation
                    // (defensive -- is_participant already implies forge.is_some()).
                    let act = match forge.as_deref_mut() {
                        Some(a) => a,
                        None => {
                            return Err(NodeLifecycleError::MissingFlag(
                                "participant-venue requires a forge activation (operator keys)",
                            ))
                        }
                    };
                    run_participant_sync(
                        source,
                        state,
                        chaindb,
                        wal,
                        &era_schedule,
                        authority.ledger_view(),
                        &mut act.pending_reselection,
                        act.security_param,
                        &mut act.pending_fork_switch,
                        &mut act.pending_missing_bridge,
                        &act.rollback_retention,
                        act.post_switch_follow.as_ref(),
                        &mut act.pending_range_refetch,
                        convergence.as_deref_mut(),
                    )
                    .await
                    .map_err(|e| NodeLifecycleError::RelaySync(format!("{e:?}")))?;
                    // PHASE4-N-AO S4+S6 (DC-NODE-37 / CE-AO-6): consume the provisional
                    // decision S3 may have set. When a network magic is configured,
                    // LIVE-BlockFetch the winning branch from the winning peer
                    // (prefetch_branch_bodies, anchor->winner_tip) and feed those bytes
                    // to apply_fork_switch; absent a magic (test / forge-off) a win is
                    // held by NullBranchBodySource (fence stays set). apply_fork_switch
                    // PROVES the bytes before any commit either way -- the live fetch is
                    // byte-only, never adoption authority.
                    let magic = act.network_magic;
                    if let Some(switch) = act.pending_fork_switch.clone() {
                        // PHASE4-N-AO S9 (DC-EVIDENCE-04): the fork_switch_id ties this
                        // apply cycle to the S3 fork_choice_selected{win} -- the SAME
                        // canonical tuple (winning_peer + fork_anchor + winner_tip).
                        let fsid = fork_switch_id(
                            &switch.winning_peer,
                            switch.fork_anchor.slot.0,
                            &switch.fork_anchor.hash,
                            switch.winner_tip.slot.0,
                            &switch.winner_tip.hash,
                        );
                        let (body_source, fetched_count): (Box<dyn BranchBodySource>, u64) =
                            match magic {
                                Some(m) => {
                                    if let Some(ev) = convergence.as_deref_mut() {
                                        ev.emit_branch_fetch_started(
                                            &fsid,
                                            &switch.winning_peer,
                                            switch.fork_anchor.slot.0,
                                            switch.winner_tip.slot.0,
                                        );
                                    }
                                    let prefetched = prefetch_branch_bodies(
                                        &switch.winning_peer,
                                        &switch.fork_anchor,
                                        &switch.winner_tip,
                                        m,
                                    )
                                    .await;
                                    let n = prefetched.len() as u64;
                                    if let Some(ev) = convergence.as_deref_mut() {
                                        ev.emit_branch_fetch_completed(&fsid, &switch.winning_peer, n);
                                    }
                                    (Box::new(prefetched) as Box<dyn BranchBodySource>, n)
                                }
                                None => (Box::new(NullBranchBodySource), 0),
                            };
                        let outcome = apply_fork_switch(
                            state,
                            chaindb,
                            wal,
                            &switch,
                            &mut act.pending_fork_switch,
                            &mut act.pending_reselection,
                            &mut act.last_fork_switch_failure,
                            body_source.as_ref(),
                            &era_schedule,
                            authority.ledger_view(),
                            act.security_param,
                            &mut act.rollback_retention,
                        )
                        .map_err(|e| NodeLifecycleError::RelaySync(format!("{e:?}")))?;
                        // PHASE4-N-AO S9 (DC-EVIDENCE-04): EXACTLY ONE terminal event for
                        // this fork_switch_id -- applied (proven adoption) OR failed
                        // (structured closed code). Observe-only; never feeds back. On a
                        // proven adoption the existing GREEN S6 reducer (block_admitted +
                        // agreement_verdict, DC-NODE-30) follows for the adopted winner.
                        match &outcome {
                            ForkSwitchOutcome::Adopted {
                                new_tip,
                                new_tip_prev,
                            } => {
                                // PHASE4-N-AO S11 (DC-NODE-39): a proven fork-switch
                                // adoption is forward progress -- clear any
                                // missing-bridge hold (the winning branch was durably
                                // adopted, so the prior stranded tip is superseded).
                                act.pending_missing_bridge = None;
                                // PHASE4-N-AO S14 (DC-NODE-41): record the post-switch
                                // follow target so a later MissingBridge for THIS
                                // winning peer's descendant is eligible for active range
                                // re-fetch (winning-peer-only). RECOVERY state, NEVER
                                // selection authority -- a re-fetched body is still
                                // proven by pump_block before any tip advance. A new
                                // adoption overwrites it (self-correcting).
                                act.post_switch_follow = Some(PostSwitchFollow {
                                    winning_peer: switch.winning_peer.clone(),
                                    adopted_tip: new_tip.clone(),
                                    fork_switch_id: fsid.clone(),
                                });
                                if let Some(ev) = convergence.as_deref_mut() {
                                    ev.emit_branch_prevalidated(
                                        &fsid,
                                        &switch.winning_peer,
                                        fetched_count,
                                    );
                                    ev.emit_fork_switch_applied(
                                        &fsid,
                                        &switch.winning_peer,
                                        new_tip.slot.0,
                                        &new_tip.hash,
                                    );
                                    // DC-MEM-11: the rare fork-switch-applied evidence path
                                    // intentionally FULL-recomputes (always exact) -- it is not the
                                    // per-block catch-up hot path, so it is not routed through the
                                    // utxo_fp_cache (emit_participant_admit, the hot path, reuses prior_fp).
                                    let post_fp = fingerprint(&state.receive.ledger).combined;
                                    let peer_tip = source.followed_peer_tip_signal().tip();
                                    ev.emit_admit_and_verdict(
                                        new_tip.slot.0,
                                        &new_tip.hash,
                                        new_tip_prev,
                                        &post_fp,
                                        peer_tip,
                                    );
                                }
                            }
                            ForkSwitchOutcome::ProofFailed { error } => {
                                if let Some(ev) = convergence.as_deref_mut() {
                                    ev.emit_fork_switch_failed(
                                        &fsid,
                                        &switch.winning_peer,
                                        map_branch_proof_failure(error),
                                    );
                                }
                            }
                        }
                    }
                    // PHASE4-N-AO S14 (DC-NODE-41): consume an eligible range re-fetch
                    // the dispatch set for a post-ForkChoiceWin winning-peer descendant
                    // whose bridge ChainSync streamed past (Fault 2 -- ChainSync sends
                    // each block once, so the passive DC-NODE-39 floor cannot recover
                    // it). ACTIVE recovery layered ON the floor: byte-only BlockFetch of
                    // durable_tip+1..descendant from the winning peer, admitted in
                    // parent-link order via pump_block (the SOLE admit), clearing the
                    // missing-bridge hold ONLY on real admitted progress. A short /
                    // lying / unservable range leaves the structured hold (the floor
                    // fallback). Bounded retry; winning-peer-only.
                    if let Some(req) = act.pending_range_refetch.take() {
                        // Staleness guard: drive ONLY for the CURRENT post-switch follow
                        // context (a newer adoption supersedes a stale request) and only
                        // with a magic configured for the live fetch. Otherwise drop it
                        // -- the floor hold (if still set) keeps the fence held; never a
                        // silent stall, never a spin (take() already consumed it).
                        let current = act
                            .post_switch_follow
                            .as_ref()
                            .map(|p| {
                                p.fork_switch_id == req.fork_switch_id && p.winning_peer == req.peer
                            })
                            .unwrap_or(false);
                        if let (true, Some(m)) = (current, magic) {
                            // The fetch start point -- `prefetch_branch_bodies` uses
                            // only (slot, hash) for the wire FindIntersect; block_no is
                            // not a fetch input (the served bytes are proven by
                            // pump_block regardless).
                            let from_anchor = ForkAnchor {
                                slot: req.from_tip.slot,
                                hash: req.from_tip.hash.clone(),
                                block_no: BlockNo(0),
                            };
                            if let Some(ev) = convergence.as_deref_mut() {
                                ev.emit_range_refetch_started(
                                    &req.fork_switch_id,
                                    &req.peer,
                                    req.from_tip.slot.0,
                                    req.to_descendant.slot.0,
                                    req.reason.as_str(),
                                );
                            }
                            // Bounded retry (RED policy): re-attempt the byte-only fetch
                            // up to MAX_RANGE_REFETCH_ATTEMPTS; only Admitted is forward
                            // progress. Each attempt re-proves via pump_block -- the
                            // fetched bytes are never authority.
                            let mut attempts = 0u32;
                            let mut outcome = RangeRefetchOutcome::Unavailable;
                            while range_refetch_should_retry(attempts) {
                                attempts += 1;
                                let prefetched = prefetch_branch_bodies(
                                    &req.peer,
                                    &from_anchor,
                                    &req.to_descendant,
                                    m,
                                )
                                .await;
                                outcome = recover_missing_range(
                                    state,
                                    chaindb,
                                    wal,
                                    &prefetched,
                                    &req,
                                    &era_schedule,
                                    authority.ledger_view(),
                                    source,
                                    convergence.as_deref_mut(),
                                );
                                if outcome.is_admitted() {
                                    break;
                                }
                            }
                            if let Some(ev) = convergence.as_deref_mut() {
                                ev.emit_range_refetch_completed(
                                    &req.fork_switch_id,
                                    &req.peer,
                                    outcome.as_str(),
                                );
                            }
                            // Clear the missing-bridge hold ONLY on real admitted
                            // progress (the same DC-NODE-39 clear rule). A non-admitted
                            // outcome LEAVES the floor hold -> the fence stays held
                            // (fail-closed); the request is consumed (no spin).
                            if outcome.is_admitted() {
                                act.pending_missing_bridge = None;
                            }
                        }
                    }
                    // PHASE4-N-AO S5 (DC-NODE-28 resolution): the forge fence clears
                    // ONLY on a RESOLVED state -- no pending decision AND caught up to
                    // the followed peer (the DC-NODE-15 signal). A proof failure left
                    // the fence HELD (S4); it is never cleared as a failure side
                    // effect. Runs unconditionally so a held fence resolves once the
                    // participant loop catches up.
                    let durable_servable_tip: Option<TipPoint> = ChainDbServedSource::new(chaindb)
                        .tip()
                        .map(|(slot, hash, block_no)| TipPoint {
                            slot,
                            hash,
                            block_no,
                        });
                    let caught_up = matches!(
                        forge_followed_tip_admission(
                            durable_servable_tip,
                            source.followed_peer_tip_signal().tip(),
                        ),
                        ForgeFollowedTipAdmission::CaughtUp
                    );
                    if fork_switch_fence_resolved(
                        &act.pending_fork_switch,
                        &act.pending_missing_bridge,
                        caught_up,
                    ) {
                        act.pending_reselection = false;
                    }
                } else {
                    let sync_outcome = run_node_sync(
                        source,
                        state,
                        chaindb,
                        wal,
                        &mut era_schedule,
                        None,
                        Some(&mut authority),
                        eview_activation,
                        reduced_checkpoint,
                    )
                        .await
                        .map_err(|e| NodeLifecycleError::RelaySync(format!("{e:?}")))?;
                    // B3b yield-at-boundary (DC-EPOCH-17): if the pass YIELDED on a durable boundary
                    // promotion, surface the structured crossing (never a bare bool). Whether it yielded
                    // or the feed ended, the next steps are identical AND deliberate -- advance the
                    // reduced checkpoint to the durable tip (so the NEXT boundary's window-replay reads a
                    // CURRENT stake view; this is precisely why the yield exists), then run the idempotent
                    // first-boundary fallback. The authority is NOT re-created here (it persists across
                    // iterations) -- a boundary is a clean in-process re-entry, never a reconnect.
                    if let crate::node_sync::SyncOutcome::BoundaryPromoted {
                        from_epoch,
                        to_epoch,
                        promotion_commitment,
                        ..
                    } = &sync_outcome
                    {
                        crate::node_log!(
                            "epoch-boundary yield: {} -> {} (eta0 {:?})",
                            from_epoch.0, to_epoch.0, promotion_commitment
                        );
                    }
                    // LIVE-LEDGER-EPOCH-TRANSITION S3 (DC-EPOCH-22): after the durable admit, the
                    // co-advancer reconciles BOTH derived stores -- the EVIEW reduced checkpoint and
                    // the durable EpochAccumulator -- to the ChainDB tip in ONE pass that SEGMENTS at
                    // each epoch boundary, capturing the SNAP mark at the exact boundary point so the
                    // accumulator CROSSES instead of stalling. The checkpoint advances are fail-closed
                    // (EVIEW currency); the accumulator is observe-only (a stall/fault never halts the
                    // follow). None/None -> byte-identical no-op.
                    advance_ledger_state_to_durable_tip(
                        reduced_checkpoint,
                        epoch_accumulator,
                        chaindb,
                        &era_schedule,
                    )?;
                    // EPOCH-CONTINUITY-ACTIVATION ECA-1 (DC-EPOCH-13): the AUTOMATIC first-boundary
                    // activation (no arming flag). A strict no-op (byte-identical) until EVIEW is
                    // configured + the seed epoch completes; then it derives the bound view
                    // (durable window replay) + atomically promotes the ONE authority
                    // (Seed->Promoted; both consumers then read the N+1 view). Terminal halt on failure.
                    maybe_activate_epoch_boundary(
                        eview_activation,
                        reduced_checkpoint,
                        chaindb,
                        &mut era_schedule,
                        wal,
                        &mut authority,
                    )?;
                }
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
                if let Some(refusal) = pending_reselection_forge_refusal(act.pending_reselection) {
                    // DC-NODE-28: a fork-choice re-selection is unresolved -- refuse
                    // the forge (typed), never forge on the stale pre-resolution tip.
                    act.last_forge_refused = Some(refusal);
                } else if let Some(kes_period) = act.coordinator_state.kes_period_for_slot(slot.0) {
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
                    // CN-FOLLOW-01: the Participant extend derives its forge base from the
                    // CURRENT durable servable tip; capture it so the sign-time re-check can
                    // confirm the durable head did not race ahead between decision and sign.
                    let mut participant_forge_base: Option<TipPoint> = None;
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
                    } else if act.venue_role == VenueRole::Participant {
                        // CN-FOLLOW-01 (DC-FOLLOW-FORGE-01): a keyed Participant venue
                        // FOLLOWS the AO-selected chain (run_participant_sync) and must
                        // also PRODUCE on it. Mirror the single-producer two-state mode:
                        // the DC-NODE-15 gate until the first caught-up instant latches
                        // the extend mode, then forge on the AO-selected durable head
                        // (ChainDb::tip) fenced by DC-NODE-28 (pending fork-choice /
                        // reselection / missing-bridge), NOT the single-producer
                        // observed-feed fence and NOT the per-tick DC-NODE-15 exact-
                        // equality re-check the racing frontier makes unsatisfiable.
                        match participant_forge_decision(
                            &act.forge_mode,
                            durable_servable_tip.clone(),
                            followed_peer_tip.clone(),
                            act.venue_role,
                            act.pending_reselection,
                            act.pending_fork_switch.is_some(),
                            act.pending_missing_bridge.is_some(),
                        ) {
                            // ExtendOnSelectedHead forges on the AO-selected durable head
                            // read at this decision boundary (`forge_base` == the durable
                            // ChainDb::tip). Capture it for the sign-time base-consistency
                            // re-check: the forge below builds on `selected_tip`
                            // (ChainDb::tip read in the same tick), and a participant admit /
                            // fork-selection could advance the durable head before the sign,
                            // so the re-check refuses rather than sign a stale block
                            // (DC-CONS-24).
                            ParticipantForgeDecision::ExtendOnSelectedHead { forge_base } => {
                                participant_forge_base = Some(forge_base);
                                true
                            }
                            ParticipantForgeDecision::Refuse(refused) => {
                                act.last_forge_refused = Some(refused);
                                false
                            }
                            ParticipantForgeDecision::UseInitialCatchupGate => {
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
                                        // First caught-up instant: latch the Participant
                                        // extend mode on the durable servable head (the
                                        // AO-selected tip the gate just matched). The
                                        // extend head is the durable tip, NOT the followed
                                        // peer tip (they byte-equal here by DC-NODE-15).
                                        if let Some(head) = durable_servable_tip.clone() {
                                            act.forge_mode = participant_forge_mode_on_caughtup(
                                                &act.forge_mode,
                                                head,
                                            );
                                        }
                                        true
                                    }
                                }
                            }
                        }
                    } else {
                        // Default (Unknown) venue — pure DC-NODE-15, unchanged.
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
                    // CN-FOLLOW-01 (DC-FOLLOW-FORGE-01) sign-time base-consistency
                    // re-check. The Participant decision derived the forge base from the
                    // durable servable tip read at the decision boundary; re-read it now,
                    // immediately before signing/admit, and refuse deterministically if a
                    // participant admit / fork-selection advanced the durable head in
                    // between — so a stale block is never signed on the superseded base.
                    // The next ForgeTick re-evaluates from the new durable tip. A no-op for
                    // SingleProducer / cold-start (participant_forge_base is None there), so
                    // the single-producer path is byte-for-byte unchanged.
                    let sign_time_ok: bool = match &participant_forge_base {
                        Some(decision_base) => {
                            let sign_time_tip: Option<TipPoint> =
                                ChainDbServedSource::new(chaindb).tip().map(
                                    |(slot, hash, block_no)| TipPoint {
                                        slot,
                                        hash,
                                        block_no,
                                    },
                                );
                            match participant_sign_time_base_consistent(
                                decision_base,
                                sign_time_tip.as_ref(),
                            ) {
                                Some(refused) => {
                                    act.last_forge_refused = Some(refused);
                                    false
                                }
                                None => true,
                            }
                        }
                        None => true,
                    };
                    if proceed_to_forge
                        && sign_time_ok
                        && (cold_start_permitted || selected_tip.is_some())
                    {
                        // DC-NODE-20 / CN-FOLLOW-01 forge-base evidence (RED, emit-only):
                        // in a single-producer OR Participant venue the forge base is the
                        // local selected durable tip (`selected_tip` == ChainDb::tip, the
                        // AO-selected head for Participant) — NOT the followed peer tip and
                        // NOT a cert. Serializes the decision already made.
                        if matches!(
                            act.venue_role,
                            VenueRole::SingleProducer | VenueRole::Participant
                        ) {
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
                            &era_schedule,
                            slot.0,
                            kes_period,
                            act.protocol_version,
                            &authority,
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
                                        &era_schedule,
                                        authority.ledger_view(),
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
                                } else if act.venue_role == VenueRole::Participant {
                                    // CN-FOLLOW-01: advance the Participant extend head to
                                    // the durable spine head just admitted (the forge's own
                                    // successor) ONLY on an actual forge+admit — a no-op on
                                    // a not_leader tick. The next ForgeTick extends N+1.
                                    let own_tip = ChainDbServedSource::new(chaindb).tip().map(
                                        |(slot, hash, block_no)| TipPoint {
                                            slot,
                                            hash,
                                            block_no,
                                        },
                                    );
                                    act.forge_mode = participant_forge_mode_after_admit(
                                        &act.forge_mode,
                                        admitted,
                                        own_tip,
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
    for (entry_index, entry) in entries.iter().enumerate() {
        // Only `AdmitBlock` entries reference preserved block bytes;
        // `SeedEpochConsensusInputsImported` (A3a) entries carry no block
        // hash and are skipped.
        if let ade_ledger::wal::WalEntry::AdmitBlock { block_hash, .. } = entry {
            // DURABLE-ADMISSION-BYTES: a WAL `AdmitBlock` whose bytes are absent
            // from the ChainDb is corrupted durable state, NOT block absence.
            // Fail closed — never the prior silent skip (which masked the
            // admission-runner persistence gap behind an empty replay map).
            match ChainDb::get_block_by_hash(chaindb, block_hash)
                .map_err(|e| NodeLifecycleError::OnDiskRead(format!("{e:?}")))?
            {
                Some(stored) => {
                    block_bytes.insert(block_hash.clone(), stored.bytes);
                }
                None => {
                    return Err(NodeLifecycleError::DurableBlockBytesMissing {
                        block_hash: block_hash.clone(),
                        entry_index,
                        source: "ChainDb::get_block_by_hash",
                    });
                }
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
    // ECA-2-pre (DC-CINPUT-06): a schema-VERSION mismatch (a pre-v4 sidecar) is a
    // TYPED upgrade/reimport requirement on the LIVE warm-start path too -- this is
    // the FIRST decode of the sidecar (for geometry), so it must surface the same
    // typed error the bootstrap authority does, never a generic decode string, so an
    // operator can tell "reimport the store" from "the store is corrupt".
    let sidecar = decode_seed_epoch_consensus_inputs(&sidecar_bytes).map_err(|e| match e {
        SeedConsensusInputsError::UnknownVersion { expected, found } => {
            NodeLifecycleError::ConsensusInputsSchemaUnsupported {
                found_version: found,
                required_version: expected,
            }
        }
        other => NodeLifecycleError::WarmStartBootstrap(format!("sidecar decode: {other:?}")),
    })?;
    let ledger_view = PoolDistrView::from_seed_epoch_consensus_inputs(&sidecar);
    // WARMSTART-ERA-SCHEDULE-VENUE (DC-CINPUT-05): rebuild the recovery
    // era-schedule from the DURABLE sidecar geometry persisted at import -- the
    // venue's real epoch_start_slot + epoch_length (preview 86400, preprod
    // 432000, ...), NOT re-derived as epoch_no * a hardcoded length. This is the
    // SAME geometry the import used, so forward-replay is venue-correct and
    // replay-equivalent (the recovered store, not a restart CLI, is authority).
    // RSW None: the durable sidecar carries no k (DC-CINPUT-05 sidecar authority),
    // so the warm-start candidate freeze is inert until B4 persists it. The live
    // forge-OFF relay (recovered_node_schedule) supplies it from --network for the
    // within-run gate.
    let era_schedule = make_node_schedule(
        sidecar.epoch_start_slot,
        sidecar.epoch_no,
        sidecar.epoch_length_slots,
        None,
    );

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
            // PHASE4-N-AI AI-S6: a RollBack is not an AdmitBlock and does not
            // define the WAL-tail slot. AI-S3/S4b-ii DO produce RollBack entries
            // (the live Participant reorg-follow); skipping them in this reverse
            // scan is safe because the load-bearing recovery floor is the durable
            // ChainDb trim (commit_rollback trims at apply time) + the
            // rollback-aware T-REC-05 fingerprint check in replay_from_anchor --
            // NOT this scan.
            ade_ledger::wal::WalEntry::RollBack { .. } => None,
            // EPOCH-CONSENSUS-VIEW S3f-4a: an activation record is not an AdmitBlock and
            // does not define the WAL-tail slot.
            ade_ledger::wal::WalEntry::EpochConsensusViewActivated { .. } => None,
        })
        .unwrap_or(SlotNo(0));
    chaindb
        .rollback_to_slot(wal_tail_slot)
        .map_err(|e| NodeLifecycleError::OnDiskRead(format!("rollback_to_slot: {e:?}")))?;

    // 5. PHASE4-N-AK AK-S1 (DC-NODE-31): load + fail-closed verify the
    //    persisted recovered anchor point for THIS (non-Origin) recovered
    //    lineage. `warm_start_recovery` is only reached once a seed-epoch anchor
    //    lineage was discovered (step 1), so the store is definitively
    //    non-Origin — a missing / malformed / fingerprint-mismatched anchor-point
    //    record halts here, never a silent Origin fallback. The loaded
    //    `(slot, hash)` is the canonical live-follow start input: it makes a
    //    bare-anchor recovery FindIntersect at the anchor, not Origin (which the
    //    relay answers with RollBackward(Origin), tripping the AI-S4a Origin
    //    fail-close). Store-derived, never CLI re-supply.
    let recovered_anchor = load_recovered_anchor_point(chaindb, &anchor_fp)
        .map_err(|e| NodeLifecycleError::WarmStartBootstrap(format!("anchor-point load: {e:?}")))?;

    // 6. The single authority. RequiredFromRecoveredProvenance runs the
    //    fail-closed sidecar verify chain; its warm-start branch forward-replays
    //    from the nearest snapshot ≤ the (reconciled) tip over the preserved
    //    bytes (the SOLE consumer of era_schedule / ledger_view).
    //    `resolve_live_follow_start(chaindb.tip(), recovered_anchor)` then sets
    //    `BootstrapState.tip`: a servable ChainDb tip still wins (a recovered
    //    local continuation spine); a bare anchor surfaces `recovered_anchor`.
    let mut recovered = bootstrap_initial_state(BootstrapInputs {
        chaindb,
        snapshot_store: chaindb,
        era_schedule: &era_schedule,
        ledger_view: &ledger_view,
        genesis_initial: None,
        seed_epoch_consensus_source: SeedEpochConsensusSource::RequiredFromRecoveredProvenance(
            provenance,
        ),
        recovered_anchor: Some(recovered_anchor),
    })
    .map_err(|e| NodeLifecycleError::WarmStartBootstrap(format!("{e:?}")))?;

    // 7. PHASE4-N-U S2 (T-REC-05): the recovered ledger fingerprint MUST equal
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
/// Resolve Ade's durable STORE directory (chain.db, WAL, reduced-checkpoint.redb). On the
/// `--bootstrap-mithril` route `--snapshot-dir` is the READ-ONLY Mithril snapshot, so the store is
/// `--data-dir` (required). On the legacy routes the store is `--snapshot-dir`; `--data-dir` takes
/// precedence when given. The two never overlap, so a judge cannot put Ade storage in the snapshot dir.
fn resolve_store_dir(cli: &Cli) -> Result<&std::path::Path, NodeLifecycleError> {
    if cli.bootstrap_mithril.is_some() {
        cli.data_dir.as_deref().ok_or(NodeLifecycleError::MissingFlag(
            "--data-dir (Ade's durable store, required with --bootstrap-mithril; --snapshot-dir is the Mithril snapshot)",
        ))
    } else {
        cli.data_dir
            .as_deref()
            .or(cli.snapshot_dir.as_deref())
            .ok_or(NodeLifecycleError::MissingFlag("--snapshot-dir"))
    }
}

/// Resolve the N2N network magic for the live wire pump: the explicit --network-magic, else the
/// committed --network profile's magic (so `node run --network preview` needs no --network-magic).
fn resolve_network_magic(cli: &Cli) -> Result<u32, NodeLifecycleError> {
    if let Some(m) = cli.network_magic {
        return Ok(m);
    }
    crate::bootstrap_export::resolve_network_profile(&cli.network)
        .map(|p| p.network_magic)
        .map_err(|_| {
            NodeLifecycleError::MissingFlag("--network-magic (or a known --network: preview|preprod)")
        })
}

fn first_run_mithril_bootstrap(
    cli: &Cli,
    chaindb: &PersistentChainDb,
    wal: &mut FileWalStore,
) -> Result<BootstrapState, NodeLifecycleError> {
    // MITHRIL-VERIFIED-ANCHOR-INTEGRATION S1d: the NATIVE route. When the V2
    // LedgerDB `state` + the Stage-2 `tables` are BOTH supplied, the FirstRun
    // arm routes the verified snapshot through the unchanged S1a/S1b/S1c chain
    // (the snapshot IS the source) and the cardano-cli / JSON seed is
    // FORBIDDEN. This supersedes the CLI-seed body below; the two are NEVER a
    // fallback for one another.
    if cli.bootstrap_mithril.is_some()
        || (cli.mithril_state_path.is_some() && cli.mithril_tables_path.is_some())
    {
        return first_run_native_mithril_bootstrap(cli, chaindb, wal);
    }

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
    // WARMSTART-ERA-SCHEDULE-VENUE (DC-CINPUT-05): the import-window schedule uses
    // the canonical bundle's venue geometry (epoch_length = end - start + 1), the
    // SAME values merge_seed_epoch_consensus_inputs persists into the sidecar for
    // warm-start recovery.
    let canonical_epoch_length = canonical.epoch_length_slots().ok_or_else(|| {
        NodeLifecycleError::ExtractionRead(
            "canonical consensus_inputs: epoch window is not a valid u32 slot length".to_string(),
        )
    })?;
    let era_schedule = make_node_schedule(
        canonical.epoch_start_slot,
        canonical.epoch_no,
        canonical_epoch_length,
        None,
    );

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

    // Honest success record. The dispatcher converges into the relay run loop; the bootstrapped
    // BootstrapState is returned for it. CONTINUITY: the seed-epoch consensus inputs are persisted
    // (sidecar + WAL provenance) AND threaded in-memory via `MithrilBootstrapOutput`, so the relay
    // loop projects the leadership view immediately on this path too -- not deferred to a restart.
    eprintln!(
        "ade_node --mode node: first-run Mithril bootstrap complete \
         (anchor initial_ledger_fingerprint={:?}, epoch={}).",
        out.anchor.initial_ledger_fingerprint, canonical.epoch_no.0
    );
    Ok(BootstrapState {
        ledger: out.ledger,
        chain_dep: out.chain_dep,
        tip: out.tip.or_else(|| {
            // FirstRun cold-start has no chaindb tip, but the certified anchor IS the live-follow
            // start. Seed it so the relay loop's recovered_anchor + the pump FindIntersect both
            // anchor at the certified point (WarmStart resolves the same via resolve_live_follow_start).
            Some(ChainTip {
                hash: out.anchor.seed_point.block_hash.clone(),
                slot: out.anchor.seed_point.slot,
            })
        }),
        // CONTINUITY (immediate follow): the SAME anchor-bound seed-epoch consensus inputs the
        // bootstrap bound + persisted, threaded in-memory so FirstRun ChainSync can project the
        // header-validation view without a restart (no sidecar read-back).
        seed_epoch_consensus_inputs: Some(out.seed_epoch_consensus_inputs),
        replayed_anchor_block_no: None,
    })
}

/// MITHRIL-VERIFIED-ANCHOR-INTEGRATION S1d: the NATIVE FirstRun route. Routes
/// the verified Mithril manifest + the V2 LedgerDB `state` + the Stage-2
/// `tables` + the Cardano Shelley genesis through the unchanged S1a/S1b/S1c
/// native chain (`native_firstrun::native_first_run_bootstrap`) and persists
/// the durable artifacts ATOMICALLY through the single closed Mithril
/// composition. The cardano-cli / JSON seed is FORBIDDEN; the snapshot IS the
/// source.
///
/// Failure semantics (TERMINAL before authority visibility): a forbidden flag,
/// a missing / mixed component, a manifest / point / network / era mismatch, or
/// a decode / materialize / assemble / persist failure all halt before the WAL
/// commit-point, leaving NO bootable partial state and NO fallback.
fn first_run_native_mithril_bootstrap(
    cli: &Cli,
    chaindb: &PersistentChainDb,
    wal: &mut FileWalStore,
) -> Result<BootstrapState, NodeLifecycleError> {
    // FORBID the cardano-cli / JSON seed alongside the native inputs (no
    // ambiguous / half-authoritative path). Terminal BEFORE any decode.
    if cli.json_seed_path.is_some() {
        return Err(NodeLifecycleError::NativeRouteForbiddenFlag("--json-seed-path"));
    }
    if cli.consensus_inputs_path.is_some() {
        return Err(NodeLifecycleError::NativeRouteForbiddenFlag(
            "--consensus-inputs-path",
        ));
    }

    // Require all native components (manifest + state + tables + shelley
    // genesis). A missing one is terminal before any decode (mixed-component).
    // Resolve the native inputs. STANDARD (--bootstrap-mithril): the manifest is the flag value and
    // state/tables are read from --snapshot-dir (the Mithril snapshot dir). LEGACY: explicit
    // --mithril-manifest/state/tables paths. (--snapshot-dir on the bootstrap route is the snapshot,
    // never Ade storage -- that is --data-dir.)
    let (manifest_path, state_path, tables_path): (
        std::path::PathBuf,
        std::path::PathBuf,
        std::path::PathBuf,
    ) = if let Some(manifest) = cli.bootstrap_mithril.as_ref() {
        let snap = cli.snapshot_dir.as_ref().ok_or(NodeLifecycleError::MissingFlag(
            "--snapshot-dir (the Mithril snapshot dir, required with --bootstrap-mithril)",
        ))?;
        (manifest.clone(), snap.join("state"), snap.join("tables"))
    } else {
        (
            cli.mithril_manifest_path
                .clone()
                .ok_or(NodeLifecycleError::MissingFlag("--mithril-manifest-path"))?,
            cli.mithril_state_path
                .clone()
                .ok_or(NodeLifecycleError::MissingFlag("--mithril-state-path"))?,
            cli.mithril_tables_path
                .clone()
                .ok_or(NodeLifecycleError::MissingFlag("--mithril-tables-path"))?,
        )
    };
    // Read the manifest + state + tables native components (terminal on a read failure — no path
    // bytes in the error). The Shelley genesis is resolved below from --network (committed
    // profile) or --shelley-genesis-path (advanced override), not as a required fourth file.
    let manifest_bytes = std::fs::read(manifest_path)
        .map_err(|e| NodeLifecycleError::ExtractionRead(format!("manifest: {:?}", e.kind())))?;
    let state_cbor = std::fs::read(state_path)
        .map_err(|e| NodeLifecycleError::ExtractionRead(format!("mithril state: {:?}", e.kind())))?;
    let tables_bytes = std::fs::read(tables_path).map_err(|e| {
        NodeLifecycleError::ExtractionRead(format!("mithril tables: {:?}", e.kind()))
    })?;
    // Resolve the genesis facts + the expected-network binding. STANDARD path: the committed
    // NetworkProfile for --network (no genesis file). ADVANCED override: --shelley-genesis-path
    // (a custom network). Network selection picks immutable constants + an expected genesis hash;
    // the native chain then proves the manifest binds to that profile.
    let profile = crate::bootstrap_export::resolve_network_profile(&cli.network).ok();
    let genesis_facts = match (cli.shelley_genesis_path.as_ref(), profile.as_ref()) {
        (Some(path), _) => {
            let bytes = std::fs::read(path).map_err(|e| {
                NodeLifecycleError::ExtractionRead(format!("shelley genesis: {:?}", e.kind()))
            })?;
            crate::native_firstrun::parse_native_shelley_genesis(&bytes)
                .map_err(|e| NodeLifecycleError::NativeFirstRun(format!("{e:?}")))?
        }
        (None, Some(p)) => crate::native_firstrun::NativeGenesisFacts {
            constants: ade_runtime::mithril_native_assembly::NativeGenesisConstants {
                max_lovelace_supply: p.max_lovelace_supply,
                active_slots_coeff: ade_core::consensus::vrf_cert::ActiveSlotsCoeff {
                    numer: p.active_slots_coeff.0,
                    denom: p.active_slots_coeff.1,
                },
            },
            epoch_length_slots: p.epoch_length as u32,
            security_param: p.security_param,
        },
        (None, None) => {
            return Err(NodeLifecycleError::MissingFlag(
                "--shelley-genesis-path (or a known --network: preview|preprod)",
            ))
        }
    };
    let expected_network = profile.as_ref().map(|p| (p.network_magic, p.genesis_hash.clone()));

    // Route through the unchanged native chain. The persistent ChainDb / WAL + the reduced
    // checkpoint live in the STORE (--data-dir on the --bootstrap-mithril route, NOT the snapshot).
    let snapshot_dir = resolve_store_dir(cli)?;
    let out = crate::native_firstrun::native_first_run_bootstrap(
        &manifest_bytes,
        &state_cbor,
        &tables_bytes,
        genesis_facts,
        expected_network,
        snapshot_dir,
        chaindb,
        chaindb,
        wal,
        |canonical| Box::new(pool_distr_view_from_canonical(canonical)),
    )
    .map_err(|e| NodeLifecycleError::NativeFirstRun(format!("{e:?}")))?;

    // The canonical bootstrap RECEIPT — authority-relevant facts only, printed BEFORE ChainSync.
    let reduced_cp = snapshot_dir.join("reduced-checkpoint.redb");
    eprintln!(
        "\n=== Ade native Mithril bootstrap receipt ===\n\
         network / profile      : {} (magic {})\n\
         shelley genesis hash   : {:?}\n\
         certified anchor point : slot {} / block {:?}\n\
         seed artifact commit   : {:?}\n\
         UTxO commitment        : {:?}\n\
         durable ledger lineage : {:?}\n\
         reduced checkpoint     : {} ({})\n\
         ChainSync              : {}\n\
         ============================================",
        cli.network,
        out.anchor.network_magic,
        out.anchor.genesis_hash,
        out.anchor.seed_point.slot.0,
        out.anchor.seed_point.block_hash,
        out.anchor.seed_artifact_hash,
        out.anchor.imported_utxo_fingerprint,
        out.anchor.initial_ledger_fingerprint,
        if reduced_cp.exists() {
            "built"
        } else {
            "absent (no EVIEW package)"
        },
        reduced_cp.display(),
        if cli.peer_addrs.is_empty() {
            "no --peer configured (forge-capable, halts clean)".to_string()
        } else {
            format!("starting against {} peer(s)", cli.peer_addrs.len())
        },
    );
    Ok(BootstrapState {
        ledger: out.ledger,
        chain_dep: out.chain_dep,
        tip: out.tip.or_else(|| {
            // FirstRun cold-start has no chaindb tip, but the certified anchor IS the live-follow
            // start. Seed it so the relay loop's recovered_anchor + the pump FindIntersect both
            // anchor at the certified point (WarmStart resolves the same via resolve_live_follow_start).
            Some(ChainTip {
                hash: out.anchor.seed_point.block_hash.clone(),
                slot: out.anchor.seed_point.slot,
            })
        }),
        // CONTINUITY (immediate follow): the SAME anchor-bound seed-epoch consensus inputs the
        // bootstrap bound + persisted, threaded in-memory so FirstRun ChainSync can project the
        // header-validation view without a restart (no sidecar read-back).
        seed_epoch_consensus_inputs: Some(out.seed_epoch_consensus_inputs),
        replayed_anchor_block_no: None,
    })
}

/// Conway-only single-era schedule consistent with the imported epoch
/// window (mirrors the established `make_schedule_for_imported_window`
/// pattern in `produce_mode` / `admission`). `locate` resolves slots in
/// the window to `epoch_no`.
/// WARMSTART-ERA-SCHEDULE-VENUE (DC-CINPUT-05): the epoch geometry
/// (`epoch_start_slot`, `epoch_length_slots`) is supplied by the caller from
/// DURABLE/venue authority -- the recovered seed-epoch sidecar or the canonical
/// import bundle -- NEVER hardcoded and NEVER switched on a venue name. `safe_zone`
/// tracks the epoch length (preserving the prior `epoch_length == safe_zone`
/// relationship). A zero `epoch_length_slots` is a caller bug, not a venue value.
fn make_node_schedule(
    epoch_start_slot: SlotNo,
    epoch_no: EpochNo,
    epoch_length_slots: u32,
    rsw: Option<u32>,
) -> EraSchedule {
    EraSchedule::new(
        BootstrapAnchorHash(Hash32([0u8; 32])),
        epoch_start_slot.0,
        vec![EraSummary {
            randomness_stabilisation_window_slots: rsw,
            era: CardanoEra::Conway,
            start_slot: epoch_start_slot,
            start_epoch: epoch_no,
            slot_length_ms: 1_000,
            epoch_length_slots,
            safe_zone_slots: epoch_length_slots,
        }],
    )
    .unwrap_or_else(|_| {
        // EraSchedule::new only fails on a zero epoch length -- a caller bug
        // (the venue geometry is never zero). Reconstruct the same single
        // summary so the owner has no panic path. (Unreachable with non-zero
        // venue geometry.)
        EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            epoch_start_slot.0,
            vec![EraSummary {
                randomness_stabilisation_window_slots: rsw,
                era: CardanoEra::Conway,
                start_slot: epoch_start_slot,
                start_epoch: epoch_no,
                slot_length_ms: 1_000,
                epoch_length_slots,
                safe_zone_slots: epoch_length_slots,
            }],
        )
        .expect("non-zero venue epoch length")
    })
}

/// The Praos randomness-stabilisation window `RSW = ceil(4k/f)` in slots for the
/// relay loop's venue, resolved from the committed `--network` profile
/// (`k = securityParam`, `f = active_slots_coeff`). `None` when the network is
/// unknown (e.g. a bare `--shelley-genesis-path` start with no `--network`): the
/// candidate freeze then stays INERT and the boundary tick fails closed until B4
/// persists `k` in the sidecar (DC-EPOCH-16).
fn rsw_for_cli(cli: &Cli) -> Option<u32> {
    let p = crate::bootstrap_export::resolve_network_profile(&cli.network).ok()?;
    let (numer, denom) = p.active_slots_coeff;
    // RSW from the single BLUE source of truth (shared with the genesis parser).
    ade_core::consensus::era_schedule::praos_rsw_slots(
        p.security_param,
        u64::from(numer),
        u64::from(denom),
    )
}

/// WARMSTART-ERA-SCHEDULE-VENUE (DC-CINPUT-05): build the live-follow / forge
/// era-schedule from the DURABLE recovered sidecar geometry -- never re-derived
/// from the restart CLI/genesis. Mirrors the recovered `ledger_view` fail-closed
/// posture: with a live feed wired, an absent sidecar fails closed (you cannot
/// validate followed/forged blocks without the venue schedule); with NO feed the
/// schedule is a provably-unconsumed inert placeholder (an explicit 1-slot
/// genesis marker -- NOT a venue value, NO hidden 432000).
fn recovered_node_schedule(
    state: &BootstrapState,
    live_feed_wired: bool,
    rsw: Option<u32>,
) -> Result<EraSchedule, NodeLifecycleError> {
    match state.seed_epoch_consensus_inputs.as_ref() {
        Some(s) => Ok(make_node_schedule(
            s.epoch_start_slot,
            s.epoch_no,
            s.epoch_length_slots,
            rsw,
        )),
        None if live_feed_wired => Err(NodeLifecycleError::FeedMissingRecoveredConsensusInputs),
        None => Ok(make_node_schedule(SlotNo(0), EpochNo(0), 1, None)),
    }
}

/// WARMSTART-ERA-SCHEDULE-VENUE (DC-CINPUT-05): assert a restart-supplied shelley
/// genesis agrees with the durable sidecar's epoch geometry. The sidecar is the
/// AUTHORITY; the genesis is ONLY a consistency check. No genesis supplied (or a
/// genesis carrying no `epochLength`) -> no check: the sidecar stands alone and
/// the geometry it persisted at import is used regardless of the restart CLI. A
/// present-but-MISMATCHED `epochLength` fails closed -- an operator must not
/// "repair" a store by passing a different venue's genesis at restart.
fn assert_restart_genesis_matches_sidecar(
    genesis_file: Option<&std::path::Path>,
    sidecar: &SeedEpochConsensusInputs,
) -> Result<(), NodeLifecycleError> {
    let Some(path) = genesis_file else {
        return Ok(());
    };
    // A genesis that cannot be read/parsed is a forge-key / clock-ingress concern
    // surfaced on the forge path; the geometry authority is the sidecar, so this
    // check stays non-authoritative on read/parse failure (does not duplicate it).
    let Ok(bytes) = std::fs::read(path) else {
        return Ok(());
    };
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Ok(());
    };
    let Some(genesis_epoch_length) = json.get("epochLength").and_then(|v| v.as_u64()) else {
        return Ok(());
    };
    if genesis_epoch_length != sidecar.epoch_length_slots as u64 {
        return Err(NodeLifecycleError::RestartGenesisGeometryMismatch {
            sidecar_epoch_length: sidecar.epoch_length_slots,
            genesis_epoch_length,
        });
    }
    Ok(())
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
        NodeLifecycleError::RestartGenesisGeometryMismatch {
            sidecar_epoch_length,
            genesis_epoch_length,
        } => {
            eprintln!(
                "ade_node --mode node: FAIL-CLOSED -- restart --genesis-file epochLength \
                 {genesis_epoch_length} disagrees with the durable seed-epoch sidecar's \
                 persisted epoch_length_slots {sidecar_epoch_length}. The recovered store's \
                 epoch geometry is authoritative (WARMSTART-ERA-SCHEDULE-VENUE); a store must \
                 NOT be repaired by supplying a different venue's genesis at restart."
            );
        }
        NodeLifecycleError::DurableBlockBytesMissing {
            block_hash,
            entry_index,
            source,
        } => {
            eprintln!(
                "ade_node --mode node: warm-start FAIL-CLOSED -- WAL AdmitBlock #{entry_index} \
                 references block {block_hash:?} whose preserved bytes are absent from the ChainDb \
                 (via {source}); corrupted durable state, NOT block absence (DURABLE-ADMISSION-BYTES)."
            );
        }
        NodeLifecycleError::ConsensusInputsSchemaUnsupported {
            found_version,
            required_version,
        } => {
            eprintln!(
                "ade_node --mode node: warm-start FAIL-CLOSED -- the durable seed-epoch \
                 consensus-inputs sidecar is schema v{found_version}, but this node requires \
                 v{required_version} (ECA-2-pre / DC-CINPUT-06: the durable consensus profile now \
                 carries genesis_hash + protocol_params_hash). This is a SCHEMA-UPGRADE / REIMPORT \
                 requirement, NOT corruption -- re-import the seed consensus inputs to rewrite the \
                 sidecar at v{required_version}."
            );
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
        NodeLifecycleError::NativeRouteForbiddenFlag(flag) => {
            eprintln!(
                "ade_node --mode node: NATIVE Mithril FirstRun route FAIL-CLOSED -- the \
                 forbidden flag {flag} (the cardano-cli / JSON seed) was supplied \
                 alongside --mithril-state-path + --mithril-tables-path. The native route \
                 is snapshot-authoritative; mixing it with an operator seed is rejected \
                 (no ambiguous / half-authoritative bootstrap, no fallback)."
            );
        }
        NodeLifecycleError::NativeFirstRun(d) => {
            eprintln!(
                "ade_node --mode node: NATIVE Mithril FirstRun bootstrap failed ({d}); \
                 failing closed. The manifest + state + tables + Shelley genesis must \
                 cohere (point / network / era) and decode/materialize/assemble/persist \
                 cleanly; TERMINAL before the WAL commit-point, no bootable partial state, \
                 no fallback to the cardano-cli / JSON seed."
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
                fwd.recovered_eta0.as_ref(),
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
            // DC-MEM-11: the ledger was REPLACED wholesale by commit_rollback, so
            // drop the per-loop UTxO-fp cache (keyed on OverlayUtxo generation) --
            // the next admit rebuilds it from the rolled-back state. Structural
            // guard against cross-fork generation reuse under a future
            // track_utxo=true; a byte-identical no-op recompute under track_utxo=false.
            fwd.invalidate_utxo_fp_cache();
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

/// PHASE4-N-AI AI-S4b-ii: the live Participant receive routing. Drains the
/// source's ordered items and routes each, gated on `VenueRole::Participant`:
///   - `Block`: decode → `CandidateSummary` + `in_spine` (ChainDb) →
///     `classify_receive` → `resolve_disposition(Participant)` → `AlreadyHave`
///     drop / `LinearExtend` `pump_block` / `Competing` fail-closed (a bare
///     competing block has no safe fork point — single-best-peer).
///   - `RollBack(point)`: verify `point` is in the durable ChainDb (fail-closed
///     if absent / Origin) → construct `ChainEvent::RolledBack` → set
///     `pending_reselection` → `apply_chain_event` → clear pending ONLY after
///     the apply returns (reconcile/failure handling complete; DC-NODE-28).
///
/// `pump_block` stays the sole roll-forward admit; the loop never calls
/// `select_best_chain` / `process_stream_input` (DC-CONS-03 honored). The
/// rollback's within-k bound is enforced by `apply_chain_event`'s materialize.
#[allow(clippy::too_many_arguments)]
/// PHASE4-N-AJ AJ-S2 (DC-NODE-30): on a successful `pump_block` admit, emit the
/// `block_admitted` + `agreement_verdict` convergence evidence as a GREEN
/// side-output. No-op when the sink is absent or the pump was an idempotent
/// no-op (`None`). `post_fp` is the post-admit recovered ledger fingerprint;
/// the peer tip is the observed followed-peer tip (`None` => `Origin`). The
/// verdict is emit-only -- it is NEVER read back into any authority path.
fn emit_participant_admit(
    evidence: Option<&mut ConvergenceEvidence>,
    state: &ForwardSyncState,
    source: &NodeBlockSource,
    pumped: Option<PumpTip>,
) {
    if let (Some(ev), Some(tip)) = (evidence, pumped) {
        // LIVE-FOLLOW-THROUGHPUT: reuse the running post-admit fingerprint the
        // reducer just computed on THIS admit -- `forward_sync_step` set
        // `state.prior_fp` to the post-admit ledger fingerprint, and nothing
        // mutates the ledger between the `pump_block` and here. Recomputing the
        // full `fingerprint()` would re-run the O(n) Ristretto255 UTxO scan a
        // SECOND time per block (doubling the catch-up cost). Byte-identical
        // value; observe-only evidence -- never read back into any authority path.
        let post_fp = state.prior_fp.clone();
        let peer_tip = source.followed_peer_tip_signal().tip();
        ev.emit_admit_and_verdict(tip.slot.0, &tip.hash, &tip.prev_hash, &post_fp, peer_tip);
        // MEM-MEASURE-A2 (OP-MEM-01): per-admit RSS sample paired with the durable tip
        // ledger fingerprint (`post_fp`). Observe-only; RSS never feeds authority.
        ev.emit_memory_measure("chain_sync_follow", tip.slot.0, tip.slot.0, &post_fp);
    }
}

/// PHASE4-N-AO S3 (DC-NODE-36): the provisional outcome of a live fork-choice
/// dispatch. `Switch` is a DECISION ONLY — S4 applies it; S3 never does.
enum ForkSwitchDecision {
    /// Keep the current durable chain (a tiebreaker loss, an ineligible reject —
    /// incl. `ExceededRollback` for depth > k — or no eligible candidate). No
    /// `PendingForkSwitch`, S4 not invoked, nothing applied.
    KeepCurrent,
    /// A strictly-preferred eligible candidate won — a PROVISIONAL switch for S4.
    Switch(PendingForkSwitch),
}

/// PHASE4-N-AO S3 (DC-NODE-36): run the SOLE selector over the per-peer candidate
/// set and map its verdict to a provisional decision. Pure over its inputs — no
/// I/O, no store, no mutation; the BLUE `select_best_chain` is the only selector.
/// On a `ChainSelected` win the winning fragment is located by MATCHING the
/// selector's returned tip identity (slot + tip `body_hash`) against the candidate
/// set — a lookup of *which* candidate BLUE chose, never a second selection.
fn decide_fork_switch(
    selector_state: &ChainSelectorState,
    competing: &BTreeMap<String, (CandidateFragment, Point)>,
) -> Result<ForkSwitchDecision, ForkChoiceError> {
    let candidates = assemble_candidate_set(competing.values().map(|(f, _)| f.clone()).collect());
    let (_new_state, event) = select_best_chain(selector_state, &candidates)?;
    match event {
        ChainEvent::ChainSelected { new_tip, .. } => {
            let winner = competing.iter().find(|(_peer, (c, _tip))| {
                c.headers
                    .last()
                    .map(|h| h.slot == new_tip.slot && h.body_hash == new_tip.hash)
                    .unwrap_or(false)
            });
            match winner {
                // `cand_tip` is the competing block's stored `(slot, block hash)` --
                // the S6 BlockFetch endpoint, retained but NOT adoption authority.
                Some((peer, (frag, cand_tip))) => {
                    Ok(ForkSwitchDecision::Switch(PendingForkSwitch {
                        fork_anchor: ForkAnchor {
                            slot: frag.anchor.slot,
                            hash: frag.anchor.hash.clone(),
                            block_no: frag.anchor_block_no,
                        },
                        winning_peer: peer.clone(),
                        winning_candidate: frag.clone(),
                        winner_tip: cand_tip.clone(),
                    }))
                }
                // Unreachable: ChainSelected.new_tip is one of the candidates' tips.
                // Fail SAFE (keep current) rather than fabricate a switch.
                None => Ok(ForkSwitchDecision::KeepCurrent),
            }
        }
        // Rejected (TiebreakerLossKeepCurrent / ExceededRollback /
        // ForkBeforeImmutableTip) or any non-selection event => keep current.
        _ => Ok(ForkSwitchDecision::KeepCurrent),
    }
}

/// PHASE4-N-AO S3 (DC-NODE-36): the live `NeedsForkChoice` dispatch driver (RED).
/// DECIDE-ONLY — it sets a provisional `PendingForkSwitch` + the DC-NODE-28 forge
/// fence on a fork-choice win and APPLIES NOTHING (no `commit_rollback`, no
/// `pump_block` of a winner, no `WalEntry::RollBack`, no body-fetch — that is S4).
///
/// Proof center: the fork anchor is bound to Ade's DURABLE STORED `(slot, hash)`
/// via `get_block_by_hash(prev_hash)` — never peer-supplied; an unknown / genesis
/// `prev_hash` fails closed (`UnexpectedRollback`). `anchor_chain_dep` comes from a
/// READ-ONLY `materialize_rolled_back_state` at that durable anchor (no commit;
/// passes the recovered eta0, T-REC-06). The current selector tiebreaker is a
/// projection from Ade's OWN already-admitted durable tip block bytes (local
/// durable authority). The conservative immutable FLOOR (the recovered anchor /
/// genesis) is selector-state input only — it NEVER permits a rollback; the
/// authoritative depth bound is `rollback_depth <= k` (and S4's independent
/// `materialize` `RollbackTooDeep`).
#[allow(clippy::too_many_arguments)]
fn dispatch_competing_fork_choice<D>(
    state: &ForwardSyncState,
    chaindb: &D,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    security_param: SecurityParam,
    durable_tip: &TipPoint,
    peer: &str,
    decoded: &DecodedBlock,
    competing: &mut BTreeMap<String, (CandidateFragment, Point)>,
    branch_caches: &mut BTreeMap<String, BTreeMap<Hash32, CachedHeader>>,
    pending_fork_switch: &mut Option<PendingForkSwitch>,
    pending_reselection: &mut bool,
    // PHASE4-N-AO S11 (DC-NODE-39): the missing-bridge hold. Set (with the closed
    // reason) on the walk-fail / materialize-fail paths -- a STRUCTURED fail-closed
    // outcome that holds the forge fence, NEVER a silent no-op and NEVER an admit of
    // the un-bridgeable block.
    pending_missing_bridge: &mut Option<MissingBridgeReason>,
    // PHASE4-N-AO S13 (DC-NODE-40): walk-visible EVIDENCE of Ade's own rolled-back
    // blocks, consulted by `walk_to_durable_lca` on a per-peer-cache miss. Read-only
    // here; populated by `apply_fork_switch`. The LCA anchor stays ChainDb-durable only.
    rollback_retention: &BTreeMap<Hash32, CachedHeader>,
    // PHASE4-N-AO S14 (DC-NODE-41): the post-`ForkChoiceWin` follow target (read-only)
    // -- consulted to decide whether a `MissingBridge` for THIS winning peer's
    // descendant is ELIGIBLE for active range re-fetch. Never selection authority.
    post_switch_follow: Option<&PostSwitchFollow>,
    // PHASE4-N-AO S14 (DC-NODE-41): the active range re-fetch sink. SET (alongside the
    // DC-NODE-39 floor hold) when an un-bridgeable competing block is a winning-peer
    // descendant ahead of the durable tip; the relay loop consumes + drives it.
    pending_range_refetch: &mut Option<RangeRefetch>,
    mut evidence: Option<&mut ConvergenceEvidence>,
) -> Result<(), NodeSyncError>
where
    D: ChainDb + SnapshotStore,
{
    // PHASE4-N-AO S9 (DC-EVIDENCE-04): observe-only decide-half taps. needs ->
    // lca -> candidate -> selected. NONE feeds back into selection/apply/fence.
    if let Some(ev) = evidence.as_deref_mut() {
        ev.emit_needs_fork_choice(peer, decoded.header_input.slot.0, &decoded.block_hash);
    }
    // PHASE4-N-AO S7 (DC-NODE-38): the fork anchor is the durable LAST COMMON
    // ANCESTOR, reached by walking the competing branch's preserved parent links --
    // NOT the competing block's immediate parent (durable only for a 1-deep fork;
    // the live-geometry gap CE-AO-6 surfaced). Cache this competing block (an
    // indexed memory of received preserved headers, self-bound by re-derived hash --
    // NOT authority), then walk back to the durable stored LCA.
    branch_caches.entry(peer.to_string()).or_default().insert(
        decoded.block_hash.clone(),
        CachedHeader {
            header: decoded.header_input.clone(),
            prev_hash: decoded.prev_hash.clone(),
            block_hash: decoded.block_hash.clone(),
        },
    );
    let branch_cache = match branch_caches.get(peer) {
        Some(c) => c,
        None => return Ok(()),
    };
    // The walk is k-bounded by BLOCK DEPTH (security_param.0; never slot distance).
    // Any LcaError -- no durable LCA within k, a branch gap, over-k, a cache
    // self-binding violation, a lying parent link -- keeps the current validated
    // chain (a selector fail-closed, no durable mutation) but, per S11 (DC-NODE-39),
    // is a STRUCTURED MissingBridge that HOLDS the forge fence (no longer the pre-S11
    // silent fence-untouched no-op). The cache is evidence; the durable LCA
    // (slot+hash, DC-NODE-29) + S2 validation + S4 body proof are authority.
    let lca = match walk_to_durable_lca(
        branch_cache,
        rollback_retention,
        &decoded.block_hash,
        chaindb,
        security_param.0,
    ) {
        Ok(r) => r,
        // PHASE4-N-AO S11 (DC-NODE-39): the competing branch cannot connect to a
        // durable stored ancestor within k (branch gap / over-k / no durable
        // ancestor / cache self-binding violation). NOT a silent no-op: emit the
        // structured closed `MissingBridge` evidence and HOLD the forge fence
        // (`pending_missing_bridge`). The durable chain is byte-unchanged, the block
        // is NOT admitted -- MissingBridge is a fail-closed outcome only, never an
        // adoption path or a reason to trust the later block.
        Err(e) => {
            let reason = map_lca_error(&e);
            if let Some(ev) = evidence.as_deref_mut() {
                ev.emit_missing_bridge(peer, &decoded.block_hash, reason.as_str());
            }
            // PHASE4-N-AO S14 (DC-NODE-41): if this un-bridgeable competing block is a
            // post-`ForkChoiceWin` WINNING-PEER descendant AHEAD of our durable tip,
            // set an ELIGIBLE active range re-fetch (durable_tip+1 .. Z) -- the floor
            // HOLD set below remains the fail-closed fallback. WINNING-PEER-ONLY: a
            // loser / unknown-peer / pre-switch gap (no matching post_switch_follow, or
            // not ahead of the tip) takes the unchanged passive floor (no fetch spam).
            // This is a fetch TRIGGER, never selection: the recovered bytes are still
            // proven by `pump_block` (the sole admit) before any tip advance.
            if let Some(psf) = post_switch_follow {
                if psf.winning_peer == peer
                    && decoded.header_input.slot.0 > durable_tip.slot.0
                {
                    *pending_range_refetch = Some(RangeRefetch {
                        peer: peer.to_string(),
                        from_tip: Point {
                            slot: durable_tip.slot,
                            hash: durable_tip.hash.clone(),
                        },
                        to_descendant: Point {
                            slot: decoded.header_input.slot,
                            hash: decoded.block_hash.clone(),
                        },
                        fork_switch_id: psf.fork_switch_id.clone(),
                        reason: reason.clone(),
                    });
                }
            }
            *pending_missing_bridge = Some(reason);
            return Ok(());
        }
    };
    if let Some(ev) = evidence.as_deref_mut() {
        ev.emit_lca_discovered(
            peer,
            lca.anchor_slot.0,
            &lca.anchor_hash,
            lca.headers.len() as u64,
        );
    }
    // The anchor binds the STORED slot + the resolved LCA hash (DC-NODE-29).
    let anchor = Point {
        slot: lca.anchor_slot,
        hash: lca.anchor_hash.clone(),
    };
    // (proof center) anchor_chain_dep via a READ-ONLY materialize at the durable
    // LCA — no commit, no WAL, no durable mutation; passes the recovered eta0.
    let reader = PersistentSnapshotCache::new(chaindb);
    let source = ChainDbBlockSource::new(chaindb);
    let (_anchor_ledger, anchor_chain_dep) = match materialize_rolled_back_state(
        TargetPoint {
            slot: anchor.slot,
            hash: lca.anchor_hash.clone(),
        },
        &reader,
        &source,
        era_schedule,
        ledger_view,
        state.recovered_eta0.as_ref(),
    ) {
        Ok(v) => v,
        // PHASE4-N-AO S11 (DC-NODE-39): the durable LCA is unreachable for a
        // read-only materialize (beyond retention) -- the branch cannot be
        // reconstructed to prove it. NOT a silent no-op: emit the structured closed
        // `MissingBridge{lca_unreachable}` and HOLD the forge fence. The durable
        // chain is byte-unchanged; never adopt an unreconstructable branch.
        Err(_) => {
            if let Some(ev) = evidence.as_deref_mut() {
                ev.emit_missing_bridge(
                    peer,
                    &decoded.block_hash,
                    MissingBridgeReason::LcaUnreachable.as_str(),
                );
            }
            *pending_missing_bridge = Some(MissingBridgeReason::LcaUnreachable);
            return Ok(());
        }
    };
    let anchor_block_no = anchor_chain_dep.last_block_no.unwrap_or(BlockNo(0));
    // S2 pure construction over the COMPLETE competing branch LCA+1..=tip (multi-
    // header — build_candidate_fragment already takes a slice). Each header is
    // validated via the BLUE authority (never minted); an invalid / incomplete
    // branch is dropped (fail closed) — the current chain is untouched. The
    // rollback_depth = durable_tip - lca_block_no is the second BLOCK-DEPTH k bound,
    // enforced downstream by select_best_chain.
    let frag = match build_candidate_fragment(
        anchor.clone(),
        anchor_block_no,
        BlockNo(durable_tip.block_no),
        &anchor_chain_dep,
        &lca.headers,
        ledger_view,
        era_schedule,
    ) {
        Ok(f) => f,
        Err(_) => return Ok(()),
    };
    if let Some(ev) = evidence.as_deref_mut() {
        ev.emit_candidate_fragment_built(peer, anchor.slot.0, frag.headers.len() as u64);
    }
    // PHASE4-N-AO S6 (CE-AO-6): retain the competing block's tip `(slot, block
    // hash)` alongside the fragment -- the live BlockFetch endpoint (NOT adoption
    // authority; S4 still binds + prevalidates the fetched bytes).
    let cand_tip = Point {
        slot: decoded.header_input.slot,
        hash: decoded.block_hash.clone(),
    };
    competing.insert(peer.to_string(), (frag, cand_tip));

    // Derive the live ChainSelectorState from DURABLE authority (Option A):
    //   current_tiebreaker = a projection from Ade's OWN durable tip block bytes,
    //   immutable_tip      = a conservative FLOOR (recovered anchor / genesis) — a
    //                        lower-bound guard, NOT an immutable tip; it never
    //                        permits a rollback (selector-state input only),
    //   security_param     = k (durable/config authority; the depth bound).
    let tip_stored = match chaindb
        .get_block_by_hash(&durable_tip.hash)
        .map_err(|e| NodeSyncError::Pump(format!("{e:?}")))?
    {
        Some(s) => s,
        // The durable tip is not a stored servable block (a bare recovery anchor) —
        // its tiebreaker cannot be projected; keep current (conservative no-op).
        None => return Ok(()),
    };
    let tip_decoded = decode_block(&tip_stored.bytes)
        .map_err(|e| NodeSyncError::Pump(format!("decode tip: {e:?}")))?;
    let current_tiebreaker = match project_tiebreaker(&tip_decoded.header_input) {
        Ok(tb) => tb,
        // A legacy / unsupported durable tip — keep current (conservative no-op).
        Err(_) => return Ok(()),
    };
    let (floor_point, floor_block_no) = match &state.recovered_anchor {
        Some(a) => (
            Point {
                slot: a.slot,
                hash: a.hash.clone(),
            },
            // Metadata only — select_best_chain gates eligibility on the floor SLOT
            // (+ rollback_depth <= k), never the floor block number.
            BlockNo(0),
        ),
        None => (
            Point {
                slot: SlotNo(0),
                hash: Hash32([0u8; 32]),
            },
            BlockNo(0),
        ),
    };
    let selector_state = ChainSelectorState {
        current_tip: Point {
            slot: durable_tip.slot,
            hash: durable_tip.hash.clone(),
        },
        current_tip_block_no: BlockNo(durable_tip.block_no),
        current_tiebreaker,
        immutable_tip: floor_point,
        immutable_tip_block_no: floor_block_no,
        security_param,
    };

    // The SOLE selector. A win is PROVISIONAL: set the decision + the DC-NODE-28
    // forge fence and APPLY NOTHING. A loss / ineligible reject keeps the current
    // chain (no decision, S4 not invoked).
    match decide_fork_switch(&selector_state, competing) {
        Ok(ForkSwitchDecision::Switch(switch)) => {
            if let Some(ev) = evidence.as_deref_mut() {
                let fsid = fork_switch_id(
                    &switch.winning_peer,
                    switch.fork_anchor.slot.0,
                    &switch.fork_anchor.hash,
                    switch.winner_tip.slot.0,
                    &switch.winner_tip.hash,
                );
                ev.emit_fork_choice_selected(
                    &fsid,
                    &switch.winning_peer,
                    ForkChoiceResult::Win,
                    Some(switch.winner_tip.slot.0),
                    Some(&switch.winner_tip.hash),
                );
            }
            // PHASE4-N-AO S9 (DC-EVIDENCE-04): a prior provisional win being
            // overwritten by this newer win on the same fork is SUPERSEDED -- emit
            // its terminal so EVERY win resolves to applied | failed | superseded
            // (the relay loop only applies the FINAL pending). Observe-only.
            if let Some(old) = pending_fork_switch.as_ref() {
                let old_fsid = fork_switch_id(
                    &old.winning_peer,
                    old.fork_anchor.slot.0,
                    &old.fork_anchor.hash,
                    old.winner_tip.slot.0,
                    &old.winner_tip.hash,
                );
                let old_peer = old.winning_peer.clone();
                if let Some(ev) = evidence.as_deref_mut() {
                    ev.emit_fork_switch_superseded(&old_fsid, &old_peer);
                }
            }
            // Fence FIRST: no forge may slip onto the stale pre-switch tip while a
            // reselection is pending (DC-NODE-28). S4 clears it after it applies.
            *pending_reselection = true;
            *pending_fork_switch = Some(switch);
            Ok(())
        }
        // Keep current (loss / ineligible) or an empty set — nothing applied.
        Ok(ForkSwitchDecision::KeepCurrent) | Err(ForkChoiceError::NoCandidates) => {
            if let Some(ev) = evidence.as_deref_mut() {
                let fsid = fork_switch_id(
                    peer,
                    lca.anchor_slot.0,
                    &lca.anchor_hash,
                    decoded.header_input.slot.0,
                    &decoded.block_hash,
                );
                ev.emit_fork_choice_selected(
                    &fsid,
                    peer,
                    ForkChoiceResult::Loss,
                    None,
                    None,
                );
            }
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn run_participant_sync<D>(
    source: &mut NodeBlockSource,
    state: &mut ForwardSyncState,
    chaindb: &D,
    wal: &mut dyn WalStore,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    pending_reselection: &mut bool,
    // PHASE4-N-AO S3 (DC-NODE-36): k for the live `select_best_chain` eligibility
    // bound (durable/config authority, never peer-supplied).
    security_param: SecurityParam,
    // PHASE4-N-AO S3 (DC-NODE-36): the provisional fork-choice decision sink. Set
    // on a win (S4 applies); S3 applies nothing.
    pending_fork_switch: &mut Option<PendingForkSwitch>,
    // PHASE4-N-AO S11 (DC-NODE-39): the missing-bridge hold. Set by the dispatch when
    // a post-switch competing descendant cannot connect to a durable ancestor within
    // k (a STRUCTURED fail-closed outcome holding the forge fence); CLEARED here on a
    // successful `LinearExtend` admit (forward progress -- the bridge arrived).
    pending_missing_bridge: &mut Option<MissingBridgeReason>,
    // PHASE4-N-AO S13 (DC-NODE-40): walk-visible EVIDENCE of Ade's own rolled-back
    // blocks (read-only here), threaded into the competing-fork-choice dispatch so the
    // LCA walk can bridge a competing branch that descends through Ade's rolled-back
    // chain. Owned cross-iteration in `ForgeActivation`; populated by `apply_fork_switch`.
    rollback_retention: &BTreeMap<Hash32, CachedHeader>,
    // PHASE4-N-AO S14 (DC-NODE-41): the post-`ForkChoiceWin` follow target (read-only)
    // + the active range re-fetch sink. On a winning-peer descendant `MissingBridge`,
    // the dispatch sets `pending_range_refetch` (alongside the floor hold); the relay
    // loop drives it. Threaded from `ForgeActivation`.
    post_switch_follow: Option<&PostSwitchFollow>,
    pending_range_refetch: &mut Option<RangeRefetch>,
    // PHASE4-N-AJ AJ-S2 (DC-NODE-30): emit-only convergence evidence. `None` =>
    // no emission. Evidence observes authority; it never becomes authority.
    mut evidence: Option<&mut ConvergenceEvidence>,
) -> Result<(), NodeSyncError>
where
    D: ChainDb + SnapshotStore,
{
    // PHASE4-N-AO S3 (DC-NODE-36): per-peer competing-candidate tracker, keyed by
    // peer (S1 identity). Deterministic (`BTreeMap`); each entry is that peer's
    // latest validated competing candidate. Accumulates across the drain so the
    // selector compares the full competing set (arrival-order independent).
    let mut competing: BTreeMap<String, (CandidateFragment, Point)> = BTreeMap::new();
    // PHASE4-N-AO S7 (DC-NODE-38): per-peer competing-branch header cache — an
    // indexed memory of received preserved headers (NOT authority), enabling the
    // last-common-ancestor walk for live multi-block branches. Accumulates across
    // the drain so a later, deeper competing block can walk back through the
    // intermediate headers Ade already saw. Transient (in-memory; no durable state).
    let mut branch_caches: BTreeMap<String, BTreeMap<Hash32, CachedHeader>> = BTreeMap::new();
    while let Some(item) = source.next_item().await {
        match item {
            NodeSyncItem::Block { peer, bytes } => {
                // AJ-S2 (DC-NODE-30): decode first so the convergence evidence can
                // record EVERY considered peer block (peer input) BEFORE the route
                // decides drop/admit/refuse. `block_received` is evidence of peer
                // input, not of local admission.
                let decoded =
                    decode_block(&bytes).map_err(|e| NodeSyncError::Pump(format!("decode: {e:?}")))?;
                let cand_slot = decoded.header_input.slot;
                let cand_hash = decoded.block_hash.clone();
                if let Some(ev) = evidence.as_deref_mut() {
                    ev.emit_block_received(&peer, cand_slot.0, &cand_hash);
                }
                // Durable tip (the detector's reference). With no durable tip yet
                // the cold-start path is out of scope for this slice -- extend via
                // the sole admit authority (pump_block), the existing behavior.
                let durable = ChainDb::tip(chaindb).map_err(|e| NodeSyncError::Pump(format!("{e:?}")))?;
                let durable_tip = match durable {
                    Some(t) => TipPoint {
                        slot: t.slot,
                        hash: t.hash,
                        block_no: state.receive.chain_dep.last_block_no.map(|b| b.0).unwrap_or(0),
                    },
                    None => {
                        let pumped = pump_block(state, chaindb, wal, &NoCheckpointSink, &bytes, era_schedule, ledger_view)
                            .map_err(|e| NodeSyncError::Pump(format!("{e:?}")))?;
                        emit_participant_admit(evidence.as_deref_mut(), state, source, pumped);
                        continue;
                    }
                };
                let candidate = CandidateSummary {
                    slot: cand_slot,
                    block_no: decoded.header_input.block_no,
                    hash: cand_hash,
                    prev_hash: decoded.prev_hash.clone(),
                };
                let in_spine = chaindb
                    .get_block_by_hash(&candidate.hash)
                    .map_err(|e| NodeSyncError::Pump(format!("{e:?}")))?
                    .is_some();
                let class = classify_receive(durable_tip.clone(), &candidate, in_spine);
                match resolve_disposition(class, VenueRole::Participant) {
                    // Known echo -- drop; `block_received` already recorded, no admit,
                    // no verdict (block_received does not imply admission).
                    ReceiveDisposition::AlreadyHave => {}
                    ReceiveDisposition::LinearExtend => {
                        // pump_block is the SOLE roll-forward admit (unchanged). Only
                        // a successful admit emits block_admitted + agreement_verdict
                        // (the verdict is emit-only -- it never influences routing).
                        let pumped = pump_block(state, chaindb, wal, &NoCheckpointSink, &bytes, era_schedule, ledger_view)
                            .map_err(|e| NodeSyncError::Pump(format!("{e:?}")))?;
                        // PHASE4-N-AO S11 (DC-NODE-39): forward progress clears a
                        // missing-bridge hold -- a real `LinearExtend` admit
                        // (`Some(tip)`, not an idempotent no-op) means the bridge
                        // arrived and Ade advanced, so the held forge fence may
                        // resolve. An echo / no-op (`None`) does NOT clear the hold.
                        if pumped.is_some() {
                            *pending_missing_bridge = None;
                        }
                        emit_participant_admit(evidence.as_deref_mut(), state, source, pumped);
                    }
                    // PHASE4-N-AO S3 (DC-NODE-36): a competing block on the Participant
                    // venue is routed to the SOLE BLUE selector. DECIDE-ONLY — a
                    // fork-choice win is held as a provisional `PendingForkSwitch` (+
                    // the DC-NODE-28 forge fence); S4 applies it. The fork anchor binds
                    // Ade's durable stored point (never peer data); an un-anchorable
                    // competing block fails closed inside the dispatch. `block_received`
                    // already recorded; no block_admitted (S3 admits nothing).
                    ReceiveDisposition::NeedsForkChoice => {
                        dispatch_competing_fork_choice(
                            state,
                            chaindb,
                            era_schedule,
                            ledger_view,
                            security_param,
                            &durable_tip,
                            &peer,
                            &decoded,
                            &mut competing,
                            &mut branch_caches,
                            pending_fork_switch,
                            pending_reselection,
                            pending_missing_bridge,
                            rollback_retention,
                            post_switch_follow,
                            pending_range_refetch,
                            evidence.as_deref_mut(),
                        )?;
                    }
                    // A single-producer venue still REFUSES a competing block (fail
                    // closed) -- multi-candidate selection is the Participant path only.
                    ReceiveDisposition::RefuseSingleProducer => {
                        return Err(NodeSyncError::UnexpectedRollback);
                    }
                }
            }
            NodeSyncItem::RollBack { point: wire_point, .. } => {
                // Verify the rollback point is in the durable chain -- no fabricated
                // block_no, no Origin (AI-S4a already fails Origin at the wire).
                let (slot, hash) = match wire_point {
                    ade_network::codec::chain_sync::Point::Block { slot, hash } => (slot, hash),
                    ade_network::codec::chain_sync::Point::Origin => {
                        return Err(NodeSyncError::UnexpectedRollback);
                    }
                };
                // DC-NODE-33 (PHASE4-N-AL) -- the participant mirror of DC-NODE-32.
                // The recovered bootstrap anchor (DC-NODE-31 / state.recovered_anchor,
                // set in the forge-ON arm at :563) is an authoritative local boundary
                // point: the relay's standard post-IntersectFound RollBackward(anchor)
                // (exact slot AND hash) is an idempotent no-op -- the node is already at
                // the anchor, a recovery snapshot boundary that is NOT a stored servable
                // block, so the DC-NODE-29 resolution below would otherwise fail closed
                // (get_block_by_hash(anchor) -> None) before the first forward admit.
                // Evaluated BEFORE get_block_by_hash; every non-anchor rollback still
                // falls through to the unchanged DC-NODE-29 stored-block authority, and
                // Origin already failed closed above (AI-S4a). No durable mutation, no
                // pending_reselection; never re-read from the store.
                if let Some(anchor) = &state.recovered_anchor {
                    if slot == anchor.slot && hash == anchor.hash {
                        continue;
                    }
                }
                // AI-S6 (DC-NODE-29): resolve the wire hash against the durable
                // ChainDb and use the STORED chain point as the sole authority. The
                // peer-supplied slot MUST equal the stored slot for the hash; an
                // unknown hash or a slot mismatch fails closed HERE -- before
                // apply_chain_event, i.e. before commit_rollback / WalEntry::RollBack
                // / any durable mutation. The peer slot never constructs `to_point`
                // (a target built from peer slot + local hash is mixed authority).
                let stored = match chaindb
                    .get_block_by_hash(&hash)
                    .map_err(|e| NodeSyncError::Pump(format!("{e:?}")))?
                {
                    Some(s) => s,
                    None => return Err(NodeSyncError::UnexpectedRollback),
                };
                if slot != stored.slot {
                    return Err(NodeSyncError::RollbackPointSlotMismatch {
                        peer_slot: slot,
                        stored_slot: stored.slot,
                        hash,
                    });
                }
                let event = ChainEvent::RolledBack {
                    to_point: Point {
                        slot: stored.slot,
                        hash,
                    },
                    depth: BlockDistance(0),
                };
                // DC-NODE-28: set pending BEFORE apply; clear ONLY after apply
                // returns (reconcile/failure handling complete) -- no forge may
                // slip through between rollback start and durable settlement.
                *pending_reselection = true;
                let applied = apply_chain_event(
                    state,
                    chaindb,
                    wal,
                    &NoCheckpointSink,
                    &event,
                    RollbackReason::PeerRollBackward,
                    None,
                    era_schedule,
                    ledger_view,
                );
                *pending_reselection = false;
                applied.map_err(|e| NodeSyncError::Pump(format!("apply_chain_event: {e:?}")))?;
            }
        }
    }
    Ok(())
}

/// PHASE4-N-AO S4 (DC-NODE-37): PROVE the selected replacement branch — fetch the
/// bodies (RED, from the winning peer) + read-only materialize the durable fork
/// anchor (`CN-STORE-07`) + prove the complete branch (`prevalidate_branch`, GREEN).
/// **Performs NO durable mutation** — no `commit_rollback`, no `pump_block`, no WAL.
/// Returns the `ProvenBranch` or a structured `BranchProofError`; the caller commits
/// ONLY on `Ok`.
fn prove_fork_switch<D>(
    state: &ForwardSyncState,
    chaindb: &D,
    switch: &PendingForkSwitch,
    body_source: &dyn BranchBodySource,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<ProvenBranch, BranchProofError>
where
    D: ChainDb + SnapshotStore,
{
    // (RED) Fetch every body of the winning branch (anchor->tip) from the winning
    // peer. A missing body is a proof failure -- the branch is not proven.
    let mut bodies: Vec<Vec<u8>> = Vec::with_capacity(switch.winning_candidate.headers.len());
    for header in &switch.winning_candidate.headers {
        let body = body_source
            .fetch_body(&switch.winning_peer, header.slot)
            .map_err(|_| BranchProofError::BodyUnavailable { slot: header.slot })?;
        bodies.push(body);
    }
    // (RED) Read-only materialize at the durable fork anchor (DC-NODE-29 point).
    // An unreachable anchor (beyond k / retention) fails closed HERE, before any
    // commit -- the independent depth guard (DC-CONS-05).
    let reader = PersistentSnapshotCache::new(chaindb);
    let source = ChainDbBlockSource::new(chaindb);
    let (anchor_ledger, anchor_chain_dep) = materialize_rolled_back_state(
        TargetPoint {
            slot: switch.fork_anchor.slot,
            hash: switch.fork_anchor.hash.clone(),
        },
        &reader,
        &source,
        era_schedule,
        ledger_view,
        state.recovered_eta0.as_ref(),
    )
    .map_err(|_| BranchProofError::AnchorUnreachable)?;
    // (GREEN) Prove the COMPLETE branch (bind + link + block_validity fold).
    prevalidate_branch(
        &switch.fork_anchor,
        &switch.winning_candidate,
        &bodies,
        &anchor_ledger,
        &anchor_chain_dep,
        era_schedule,
        ledger_view,
    )
}

/// PHASE4-N-AO S4 (DC-NODE-37): the fork-switch apply driver (RED). Turns S3's
/// provisional `PendingForkSwitch` into a durable adoption ONLY after
/// `prove_fork_switch` proves the complete replacement branch — the proof STRICTLY
/// precedes the irreversible `commit_rollback`.
///
/// **A `PendingForkSwitch` is not authority to roll back; it is only authority to
/// attempt proof of the selected replacement branch.**
///
/// On a proof failure: NO durable mutation; the decision is retired as a structured
/// `ProofFailed`; the `pending_reselection` forge fence is **HELD** (never cleared
/// as a side effect of an unproven branch — no silent "failed winner, resume
/// forging"). On a proven branch: adopt via the existing `apply_chain_event`
/// authorities (`DC-NODE-25`) — `RolledBack(fork_anchor)` + `ChainSelected(body)×N`,
/// recorded as `WalEntry::RollBack{ForkChoiceWin}` — then clear the fence LAST.
/// PHASE4-N-AO S9 (DC-EVIDENCE-04): map the structured `BranchProofError` to the
/// CLOSED `ForkChoiceEvidenceFailure` code -- the evidence vocabulary carries no
/// free-form error string. Observe-only (the mapping never affects authority).
fn map_branch_proof_failure(e: &BranchProofError) -> ForkChoiceEvidenceFailure {
    match e {
        BranchProofError::EmptyBranch => ForkChoiceEvidenceFailure::EmptyBranch,
        BranchProofError::BodyUnavailable { .. } => ForkChoiceEvidenceFailure::BodyUnavailable,
        BranchProofError::BodyHeaderMismatch { .. } => ForkChoiceEvidenceFailure::BodyHeaderMismatch,
        BranchProofError::BrokenParentLink { .. } => ForkChoiceEvidenceFailure::BrokenParentLink,
        BranchProofError::BodyInvalid { .. } => ForkChoiceEvidenceFailure::BodyInvalid,
        BranchProofError::AnchorUnreachable => ForkChoiceEvidenceFailure::AnchorUnreachable,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn apply_fork_switch<D>(
    state: &mut ForwardSyncState,
    chaindb: &D,
    wal: &mut dyn WalStore,
    switch: &PendingForkSwitch,
    pending_fork_switch: &mut Option<PendingForkSwitch>,
    pending_reselection: &mut bool,
    last_fork_switch_failure: &mut Option<BranchProofError>,
    body_source: &dyn BranchBodySource,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    // PHASE4-N-AO S13 (DC-NODE-40): block-depth k for the rollback-retention bound.
    security_param: SecurityParam,
    // PHASE4-N-AO S13 (DC-NODE-40): the walk-visible rollback-retention EVIDENCE. This
    // is the ONLY writer -- it captures the blocks about to be rolled back (Ade's own
    // durable chain fork_anchor+1..=old_tip) as self-bound, k-bounded evidence BEFORE
    // the rollback removes them, so a later competing branch descending through them
    // stays evaluable. NEVER durable / anchor / rollback-target / S2-S4 bypass.
    rollback_retention: &mut BTreeMap<Hash32, CachedHeader>,
) -> Result<ForkSwitchOutcome, NodeSyncError>
where
    D: ChainDb + SnapshotStore,
{
    // PROVE FIRST. prove_fork_switch performs no durable mutation; on failure the
    // current chain is byte-unchanged.
    let proven = match prove_fork_switch(
        state,
        chaindb,
        switch,
        body_source,
        era_schedule,
        ledger_view,
    ) {
        Ok(p) => p,
        Err(error) => {
            // Retire the decision as a STRUCTURED failure; HOLD the forge fence.
            *last_fork_switch_failure = Some(error.clone());
            *pending_fork_switch = None;
            return Ok(ForkSwitchOutcome::ProofFailed { error });
        }
    };
    // A proven branch is non-empty by construction; guard BEFORE the irreversible
    // step so an empty branch can never half-switch.
    let final_tip = match proven.blocks.last() {
        Some(b) => b.tip.clone(),
        None => {
            *last_fork_switch_failure = Some(BranchProofError::EmptyBranch);
            *pending_fork_switch = None;
            return Ok(ForkSwitchOutcome::ProofFailed {
                error: BranchProofError::EmptyBranch,
            });
        }
    };
    // The adopted tip's validated parent (S10 / DC-EVIDENCE-05): the prior block
    // in the proven branch, or the fork anchor for a single-block branch. A
    // local, validated fact — never peer-claimed.
    let new_tip_prev = match proven.blocks.len() {
        1 => switch.fork_anchor.hash.clone(),
        n => proven.blocks[n - 2].tip.hash.clone(),
    };

    // PHASE4-N-AO S13 (DC-NODE-40): retain the about-to-be-rolled-back blocks as
    // walk-visible EVIDENCE before the rollback removes them from durable. Capture
    // Ade's OWN durable chain old_tip -> fork_anchor+1 (EXCLUSIVE of the anchor, which
    // stays durable) as SELF-BOUND CachedHeaders (key == re-derived block_hash, never
    // a peer claim) so a later competing branch descending through Ade's rolled-back
    // chain can reach a durable ancestor instead of a false BranchGap -> MissingBridge
    // over-fire. EVIDENCE ONLY: never durable, never the LCA anchor (the walk's anchor
    // check is ChainDb-only), never a rollback target, never an S2/S4 bypass.
    if let Ok(Some(old_tip)) = chaindb.tip() {
        let anchor_hash = switch.fork_anchor.hash.clone();
        let mut cur = old_tip.hash;
        let mut steps = 0u64;
        // The rollback is <= k by S3 eligibility; cap the walk at k block depth.
        while cur != anchor_hash && steps <= security_param.0 {
            let stored = match chaindb.get_block_by_hash(&cur) {
                Ok(Some(s)) => s,
                _ => break,
            };
            let d = match decode_block(&stored.bytes) {
                Ok(d) => d,
                Err(_) => break,
            };
            let next = match d.prev_hash.block_hash() {
                Some(h) => h.clone(),
                None => break, // genesis -- no further parent
            };
            // Self-binding: only retain a stored block that re-derives to its own
            // lookup hash; the map key IS the re-derived block_hash (never peer-claimed).
            if d.block_hash == cur {
                rollback_retention.insert(
                    d.block_hash.clone(),
                    CachedHeader {
                        header: d.header_input.clone(),
                        prev_hash: d.prev_hash.clone(),
                        block_hash: d.block_hash.clone(),
                    },
                );
            }
            cur = next;
            steps += 1;
        }
        // k-BOUND eviction (no unbounded growth): keep only entries within k block
        // depth of the highest retained block (~ the latest rollback boundary).
        if let Some(max_bno) = rollback_retention
            .values()
            .map(|c| c.header.block_no.0)
            .max()
        {
            let cutoff = max_bno.saturating_sub(security_param.0);
            rollback_retention.retain(|_, c| c.header.block_no.0 >= cutoff);
        }
    }

    // ONLY NOW adopt via the existing apply authorities (DC-NODE-25). The
    // prevalidation guarantees each pump_block below succeeds (except crash -> WAL
    // replay). commit_rollback (irreversible) happens HERE, after proof.
    apply_chain_event(
        state,
        chaindb,
        wal,
        &NoCheckpointSink,
        &ChainEvent::RolledBack {
            to_point: Point {
                slot: switch.fork_anchor.slot,
                hash: switch.fork_anchor.hash.clone(),
            },
            depth: BlockDistance(0),
        },
        RollbackReason::ForkChoiceWin,
        None,
        era_schedule,
        ledger_view,
    )
    .map_err(|e| NodeSyncError::Pump(format!("fork-switch rollback: {e:?}")))?;
    for block in &proven.blocks {
        apply_chain_event(
            state,
            chaindb,
            wal,
            &NoCheckpointSink,
            &ChainEvent::ChainSelected {
                new_tip: block.tip.clone(),
                replaced_tip: None,
            },
            RollbackReason::ForkChoiceWin,
            Some(&block.bytes),
            era_schedule,
            ledger_view,
        )
        .map_err(|e| NodeSyncError::Pump(format!("fork-switch roll-forward: {e:?}")))?;
    }

    // Reconcile is enforced inside apply_chain_event (DC-NODE-26). Clear the
    // decision + the forge fence LAST -- now resolved (ON the winner).
    *pending_fork_switch = None;
    *pending_reselection = false;
    *last_fork_switch_failure = None;
    Ok(ForkSwitchOutcome::Adopted {
        new_tip: final_tip,
        new_tip_prev,
    })
}

/// PHASE4-N-AO S14 (DC-NODE-41): admit a re-fetched missing range in PARENT-LINK ORDER.
/// GREEN sequencing over the RED-fetched bytes; BLUE `pump_block` is the SOLE admit --
/// each body's parent-link + body-hash + ledger validity is enforced by the chokepoint,
/// so a lying / out-of-order / short range is REJECTED, never admitted. Returns the
/// closed [`RangeRefetchOutcome`]; only `Admitted` (the target descendant reached) is
/// forward progress that clears the missing-bridge hold. Pumps the bodies the winning
/// peer served (ascending slot order); each must linear-extend the prior admitted tip.
///
/// `source` + `evidence` carry the per-admitted-block convergence evidence: each
/// recovered descendant emits `block_admitted` + `agreement_verdict` IDENTICALLY to
/// a normal `LinearExtend` admit (so the post-switch branch-continuity gate, S10
/// DC-EVIDENCE-05, sees the recovered descendants as followed blocks). `evidence` =
/// `None` (the part-1 hermetic tests) emits nothing.
pub fn recover_missing_range<D>(
    state: &mut ForwardSyncState,
    chaindb: &D,
    wal: &mut dyn WalStore,
    prefetched: &PrefetchedBranchBodies,
    req: &RangeRefetch,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    source: &NodeBlockSource,
    mut evidence: Option<&mut ConvergenceEvidence>,
) -> RangeRefetchOutcome
where
    D: ChainDb + SnapshotStore,
{
    let bodies = prefetched.ordered_for_peer(&req.peer);
    if bodies.is_empty() {
        // The winning peer served no range -- the hold remains (no admit, no mutation).
        return RangeRefetchOutcome::Unavailable;
    }
    let mut reached = false;
    for bytes in bodies {
        match pump_block(
            state,
            chaindb,
            wal,
            &NoCheckpointSink,
            &bytes,
            era_schedule,
            ledger_view,
        ) {
            // Admitted as a LinearExtend of the prior tip. If it is the target
            // descendant, the range is fully recovered. Emit block_admitted +
            // agreement_verdict for the recovered descendant (same as a normal
            // LinearExtend admit -- S10 continuity counts it as a followed block).
            Ok(Some(tip)) => {
                if tip.hash == req.to_descendant.hash {
                    reached = true;
                }
                emit_participant_admit(evidence.as_deref_mut(), state, source, Some(tip));
            }
            // Idempotent no-op (already durable): if the descendant is already in the
            // store, the range is satisfied; otherwise keep walking the served range.
            Ok(None) => {
                if matches!(
                    chaindb.get_block_by_hash(&req.to_descendant.hash),
                    Ok(Some(_))
                ) {
                    reached = true;
                }
            }
            // The BLUE chokepoint REJECTED a fetched body (parent-link / body-hash /
            // ledger). NOT admitted; the structured MissingBridge hold remains. A
            // non-extending body is a parent-link mismatch; a decoded-but-invalid body
            // is a validation failure (BlockFetch bytes are never authority).
            Err(e) => {
                return match e {
                    PumpError::Receive(_) => RangeRefetchOutcome::ParentLinkMismatch,
                    _ => RangeRefetchOutcome::ValidationFailed,
                };
            }
        }
    }
    if reached {
        RangeRefetchOutcome::Admitted
    } else {
        // Served some blocks but never reached the target descendant -- short range.
        RangeRefetchOutcome::ShortRange
    }
}

/// PHASE4-N-AO S6 (CE-AO-6): live BlockFetch of the winning branch's bodies (RED).
/// The winning peer is ON the winning chain, so FOLLOWING it from the durable fork
/// anchor yields the winning branch anchor→`winner_tip`. Reuses the existing
/// consume client (`dial_for_admission` + `run_admission_wire_pump`) — NO new
/// block-fetch client, NO new venue.
///
/// **Returns BYTES only** — a best-effort `PrefetchedBranchBodies`. It NEVER
/// certifies selection or validity and NEVER clears the fence: a failed / partial
/// / truncated / lying fetch is rejected by S4 `prevalidate_branch` before any
/// `commit_rollback` (the byte-only boundary; `DC-NODE-35/37`). Bounded by a
/// timeout so a stalled / Byzantine peer cannot hang the relay loop.
pub async fn prefetch_branch_bodies(
    peer_addr: &str,
    fork_anchor: &ForkAnchor,
    winner_tip: &Point,
    network_magic: u32,
) -> PrefetchedBranchBodies {
    let mut prefetched = PrefetchedBranchBodies::new();
    let sock: std::net::SocketAddr = match peer_addr.parse() {
        Ok(s) => s,
        // Unparseable / unreachable peer label -> empty (S4 holds the fence).
        Err(_) => return prefetched,
    };
    let (transport, version) =
        match dial_for_admission(sock, build_n2n_version_table(network_magic)).await {
            Ok(v) => v,
            // Dial / N2N handshake failed -> empty (no bytes, fence held).
            Err(_) => return prefetched,
        };
    // Follow FROM the fork anchor; the peer's chain anchor->tip IS the winning
    // branch. The pump block-fetches each forwarded block and emits it.
    let start = WirePoint::Block {
        slot: fork_anchor.slot,
        hash: fork_anchor.hash.clone(),
    };
    let (ev_tx, mut ev_rx) = mpsc::channel::<AdmissionPeerEvent>(64);
    let pump = tokio::spawn(run_admission_wire_pump(
        transport,
        sock.to_string(),
        start,
        version,
        network_magic,
        ev_tx,
    ));
    let _ = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        while let Some(ev) = ev_rx.recv().await {
            if let AdmissionPeerEvent::Block { block_bytes, .. } = ev {
                if let Ok(decoded) = decode_block(&block_bytes) {
                    let reached_tip = decoded.block_hash == winner_tip.hash;
                    prefetched.insert(peer_addr, decoded.header_input.slot, block_bytes);
                    if reached_tip {
                        break; // collected up to the selected winner tip
                    }
                }
            }
        }
    })
    .await;
    pump.abort();
    prefetched
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    // ECA-5 (DC-EPOCH-15): forecast-horizon extension coupled to authority promotion.
    mod eca5_forecast_crossing {
        use super::super::*;

        const L: u32 = 86_400;
        const N: u64 = 1338;
        fn seed_sched() -> EraSchedule {
            make_node_schedule(SlotNo(N * u64::from(L)), EpochNo(N), L, None)
        }
        fn slot_in(epoch: u64) -> SlotNo {
            SlotNo(epoch * u64::from(L) + 30)
        }

        #[test]
        fn forecast_extends_only_on_promotion() {
            let mut sched = seed_sched();
            // Before promotion: an N+1 slot is OUTSIDE the forecast horizon (= the boundary).
            assert!(sched.check_forecast_horizon(slot_in(N + 1)).is_err());
            // Idempotent no-op for the seed epoch itself.
            extend_schedule_to_epoch(&mut sched, EpochNo(N));
            assert_eq!(sched.eras().len(), 1);
            assert!(sched.check_forecast_horizon(slot_in(N + 1)).is_err());
            // After promotion to N+1: the horizon extends; the N+1 slot validates + locates to N+1.
            extend_schedule_to_epoch(&mut sched, EpochNo(N + 1));
            assert_eq!(sched.eras().len(), 2);
            assert!(sched.check_forecast_horizon(slot_in(N + 1)).is_ok());
            assert_eq!(sched.locate(slot_in(N + 1)).unwrap().epoch, EpochNo(N + 1));
            assert_eq!(sched.locate(slot_in(N)).unwrap().epoch, EpochNo(N));
            // N+2 is still out -- the horizon never reaches an unpromoted epoch.
            assert!(sched.check_forecast_horizon(slot_in(N + 2)).is_err());
        }

        #[test]
        fn warmstart_reconstruction_is_byte_identical_to_live_append() {
            // Live: append per boundary (N -> N+1 -> N+2).
            let mut live = seed_sched();
            extend_schedule_to_epoch(&mut live, EpochNo(N + 1));
            extend_schedule_to_epoch(&mut live, EpochNo(N + 2));
            // Warm-start: reconstruct to N+2 in one shot from the seed.
            let mut warm = seed_sched();
            extend_schedule_to_epoch(&mut warm, EpochNo(N + 2));
            assert_eq!(live.eras(), warm.eras());
            assert_eq!(live, warm);
            // Deterministic across rebuilds.
            let mut warm2 = seed_sched();
            extend_schedule_to_epoch(&mut warm2, EpochNo(N + 2));
            assert_eq!(warm, warm2);
        }

        #[test]
        fn eraschedule_supports_adjacent_same_era_summaries() {
            // Proof obligation 1: EraSchedule::new/locate handle adjacent same-era consecutive epochs.
            let mut sched = seed_sched();
            extend_schedule_to_epoch(&mut sched, EpochNo(N + 2));
            assert_eq!(sched.eras().len(), 3);
            for off in 0..=2u64 {
                let loc = sched.locate(slot_in(N + off)).unwrap();
                assert_eq!(loc.epoch, EpochNo(N + off));
                assert!(matches!(loc.era, CardanoEra::Conway));
            }
        }
    }

    // ===== LIVE-LEDGER-EPOCH-TRANSITION S3 (DC-EPOCH-22): the boundary-aligned co-advancer =====
    // The node_lifecycle co-advancer that SEGMENTS the reduced-checkpoint + accumulator advance at each
    // epoch boundary: at a boundary stall it brings the checkpoint to the boundary point `s_prev`, captures
    // the SNAP mark there, durably binds the BoundaryMark witness, and crosses the accumulator. Hermetic
    // (InMemoryChainDb + real redb stores via tempfile). The mark VALUE is CE-3c's job -- these prove the
    // ORCHESTRATION (cross / multi-boundary catch-up / EVIEW currency / observe-only).
    mod co_advance_ledger_state {
        use super::super::*;
        use ade_ledger::epoch_accumulator::EpochAccumulator;
        use ade_ledger::reduced_utxo::ReducedStakeRef;
        use ade_runtime::chaindb::{
            EpochAccumulatorStore, InMemoryChainDb, ReducedUtxoCheckpoint, StoredBlock,
        };
        use ade_types::shelley::cert::StakeCredential;
        use ade_types::tx::{Coin, TxIn};
        use std::collections::BTreeMap;
        use tempfile::TempDir;

        const RAW_CONWAY_BLOCK: &[u8] =
            include_bytes!("../tests/fixtures/raw_era_block_conway.cbor");

        /// A from-genesis Conway schedule with 86_000-slot epochs: `locate(86_000 * E).epoch == E`, so slot
        /// 43_000_000 is epoch 500 (within-epoch vs the sealed store), 43_086_000 epoch 501 (a boundary),
        /// 43_172_000 epoch 502 (the next boundary). Reuses the node's own `make_node_schedule` builder.
        fn schedule_86k() -> EraSchedule {
            make_node_schedule(SlotNo(0), EpochNo(0), 86_000, None)
        }

        fn cred(b: u8) -> StakeCredential {
            StakeCredential::KeyHash(Hash28([b; 28]))
        }

        /// A sealed EpochAccumulator at epoch 500 with reserves -- the accumulator the real Conway block
        /// applies cleanly to (mirrors the ade_runtime advance tests' `sealed_store_at_epoch_500`).
        fn sealed_store_at_epoch_500(tmp: &TempDir, seed_slot: SlotNo) -> EpochAccumulatorStore {
            let mut acc = EpochAccumulator::new(CardanoEra::Conway);
            acc.epoch_state.epoch = EpochNo(500);
            acc.epoch_state.reserves = Coin(1_000_000_000_000_000);
            let s = EpochAccumulatorStore::open(&tmp.path().join("acc.redb")).unwrap();
            s.seal_bootstrap(&acc, seed_slot).unwrap();
            s
        }

        /// A sealed reduced checkpoint with two delegated base creds, so the captured mark is non-empty
        /// (mirroring the #2b-i proven mark). The advancer folds the real Conway block cleanly over it.
        fn sealed_checkpoint(tmp: &TempDir, seed_slot: SlotNo) -> ReducedUtxoCheckpoint {
            let cp = ReducedUtxoCheckpoint::open(&tmp.path().join("cp.redb")).unwrap();
            let mut reduced: BTreeMap<TxIn, (Coin, ReducedStakeRef)> = BTreeMap::new();
            reduced.insert(
                TxIn {
                    tx_hash: Hash32([1; 32]),
                    index: 0,
                },
                (Coin(5_000_000), ReducedStakeRef::Base(cred(0x11))),
            );
            reduced.insert(
                TxIn {
                    tx_hash: Hash32([2; 32]),
                    index: 0,
                },
                (Coin(7_000_000), ReducedStakeRef::Base(cred(0x22))),
            );
            cp.build_from(&reduced).unwrap();
            cp.seal_bootstrap(seed_slot).unwrap();
            cp
        }

        fn put_raw(db: &InMemoryChainDb, slot: u64) {
            db.put_block(&StoredBlock {
                hash: Hash32([(slot & 0xff) as u8; 32]),
                slot: SlotNo(slot),
                bytes: RAW_CONWAY_BLOCK.to_vec(),
            })
            .unwrap();
        }

        /// CE-3c hermetic prerequisite: the co-advancer crosses ONE epoch boundary -- it captures the mark
        /// at the boundary point `s_prev`, binds the witness, crosses the accumulator into the new epoch,
        /// and leaves the reduced checkpoint at the durable tip with the binding consumed + cleared.
        #[test]
        fn co_advance_crosses_a_boundary() {
            let tmp = TempDir::new().unwrap();
            let cp = sealed_checkpoint(&tmp, SlotNo(42_000_000));
            let store = sealed_store_at_epoch_500(&tmp, SlotNo(42_000_000));
            let db = InMemoryChainDb::new();
            put_raw(&db, 43_000_000); // epoch 500, within-epoch -> s_prev
            put_raw(&db, 43_086_000); // epoch 501, the boundary block -> s_bb
            let sched = schedule_86k();

            advance_ledger_state_to_durable_tip(Some(&cp), Some(&store), &db, &sched).unwrap();

            // The accumulator CROSSED into epoch 501 at the boundary slot.
            let (slot, acc) = store.load_current().unwrap().unwrap();
            assert_eq!(
                acc.epoch_state.epoch,
                EpochNo(501),
                "the accumulator crossed the boundary"
            );
            assert_eq!(slot, SlotNo(43_086_000), "advanced to the boundary block slot");
            // EVIEW currency: the reduced checkpoint reached the durable tip.
            assert_eq!(cp.last_advanced_slot().unwrap(), Some(SlotNo(43_086_000)));
            // The boundary-mark binding was consumed + cleared by the cross.
            assert_eq!(store.boundary_mark_binding().unwrap(), None);
        }

        /// EVIEW-preservation: with NO accumulator the co-advancer reduces to the pre-S3 reduced-checkpoint
        /// advance -- it brings the checkpoint to the durable tip and nothing else.
        #[test]
        fn co_advance_checkpoint_only_when_no_accumulator() {
            let tmp = TempDir::new().unwrap();
            let cp = sealed_checkpoint(&tmp, SlotNo(42_000_000));
            let db = InMemoryChainDb::new();
            put_raw(&db, 43_000_000);
            put_raw(&db, 43_086_000);
            let sched = schedule_86k();

            advance_ledger_state_to_durable_tip(Some(&cp), None, &db, &sched).unwrap();

            assert_eq!(cp.last_advanced_slot().unwrap(), Some(SlotNo(43_086_000)));
        }

        /// Multi-boundary catch-up: TWO boundaries (501 then 502) in `(seed, tip]` -> ONE call crosses BOTH.
        #[test]
        fn co_advance_multi_boundary_catch_up() {
            let tmp = TempDir::new().unwrap();
            let cp = sealed_checkpoint(&tmp, SlotNo(42_000_000));
            let store = sealed_store_at_epoch_500(&tmp, SlotNo(42_000_000));
            let db = InMemoryChainDb::new();
            put_raw(&db, 43_000_000); // epoch 500, within-epoch
            put_raw(&db, 43_086_000); // epoch 501, boundary #1
            put_raw(&db, 43_100_000); // epoch 501, within-epoch
            put_raw(&db, 43_172_000); // epoch 502, boundary #2
            let sched = schedule_86k();

            advance_ledger_state_to_durable_tip(Some(&cp), Some(&store), &db, &sched).unwrap();

            let (slot, acc) = store.load_current().unwrap().unwrap();
            assert_eq!(
                acc.epoch_state.epoch,
                EpochNo(502),
                "ONE call crossed BOTH boundaries"
            );
            assert_eq!(slot, SlotNo(43_172_000));
            assert_eq!(cp.last_advanced_slot().unwrap(), Some(SlotNo(43_172_000)));
            assert_eq!(store.boundary_mark_binding().unwrap(), None);
        }

        /// Observe-only: an accumulator but NO checkpoint (no mark source) -> the boundary STALLS; the call
        /// returns Ok (never halts the follow) and the accumulator does NOT cross.
        #[test]
        fn co_advance_observe_only_when_no_checkpoint() {
            let tmp = TempDir::new().unwrap();
            let store = sealed_store_at_epoch_500(&tmp, SlotNo(42_000_000));
            let db = InMemoryChainDb::new();
            put_raw(&db, 43_000_000); // epoch 500, within-epoch
            put_raw(&db, 43_086_000); // epoch 501, boundary
            let sched = schedule_86k();

            advance_ledger_state_to_durable_tip(None, Some(&store), &db, &sched).unwrap();

            let (slot, acc) = store.load_current().unwrap().unwrap();
            assert_eq!(acc.epoch_state.epoch, EpochNo(500), "no mark source -> no cross");
            assert_eq!(
                slot,
                SlotNo(43_000_000),
                "folded within-epoch up to s_prev, then stalled observe-only"
            );
        }
    }

    // ===== PHASE4-N-AO S3 (DC-NODE-36): live selector dispatch decision =====
    // Unit tests of `decide_fork_switch` — the SOLE-selector verdict-to-decision
    // mapping — over SYNTHETIC candidates (no I/O, no corpus). The integration
    // wiring (durable anchor binding + read-only materialize + no mutation) is in
    // `tests/live_fork_choice_ai_s4bii.rs`.
    mod s3_select_dispatch {
        use super::super::*;
        use ade_core::consensus::candidate::TiebreakerView;
        use ade_core::consensus::header_summary::ValidatedHeaderSummary;
        use ade_crypto::vrf::VrfOutput;

        fn tv(slot: u64, vrf_first: u8) -> TiebreakerView {
            TiebreakerView {
                slot: SlotNo(slot),
                issuer_hash: Hash28([0xAA; 28]),
                op_cert_counter: 1,
                leader_vrf_output_first_8: [vrf_first; 8],
            }
        }

        fn summary(slot: u64, block_no: u64, body: u8, vrf_first: u8) -> ValidatedHeaderSummary {
            let mut out = [0u8; 64];
            out[0..8].copy_from_slice(&[vrf_first; 8]);
            ValidatedHeaderSummary {
                slot: SlotNo(slot),
                block_no: BlockNo(block_no),
                body_hash: Hash32([body; 32]),
                issuer_pool: Hash28([0xAA; 28]),
                op_cert_counter: 1,
                vrf_leader_output: VrfOutput(out),
            }
        }

        // A one-header fragment: tip block_no = anchor_block_no + 1;
        // rollback_depth = current_block_no - anchor_block_no.
        fn fragment(
            anchor_slot: u64,
            anchor_block_no: u64,
            current_block_no: u64,
            tip_slot: u64,
            tip_body: u8,
            tip_vrf_first: u8,
        ) -> CandidateFragment {
            CandidateFragment {
                anchor: Point {
                    slot: SlotNo(anchor_slot),
                    hash: Hash32([0x99; 32]),
                },
                anchor_block_no: BlockNo(anchor_block_no),
                select_view: tv(tip_slot, tip_vrf_first),
                rollback_depth: BlockDistance(current_block_no.saturating_sub(anchor_block_no)),
                headers: vec![summary(tip_slot, anchor_block_no + 1, tip_body, tip_vrf_first)],
            }
        }

        // A competing entry: the fragment + its tip `(slot, block hash)`. The block
        // hash is a synthetic test value (distinct from body_hash); winner_tip is
        // fetch-endpoint metadata, not asserted by these selection tests.
        fn candidate(
            anchor_slot: u64,
            anchor_block_no: u64,
            current_block_no: u64,
            tip_slot: u64,
            tip_body: u8,
            tip_vrf_first: u8,
        ) -> (CandidateFragment, Point) {
            (
                fragment(
                    anchor_slot,
                    anchor_block_no,
                    current_block_no,
                    tip_slot,
                    tip_body,
                    tip_vrf_first,
                ),
                Point {
                    slot: SlotNo(tip_slot),
                    hash: Hash32([tip_vrf_first; 32]),
                },
            )
        }

        fn state(current_block_no: u64, current_slot: u64, current_vrf_first: u8, k: u64) -> ChainSelectorState {
            ChainSelectorState {
                current_tip: Point {
                    slot: SlotNo(current_slot),
                    hash: Hash32([0x11; 32]),
                },
                current_tip_block_no: BlockNo(current_block_no),
                current_tiebreaker: tv(current_slot, current_vrf_first),
                // Conservative floor at genesis (slot 0) — every anchor above it.
                immutable_tip: Point {
                    slot: SlotNo(0),
                    hash: Hash32([0u8; 32]),
                },
                immutable_tip_block_no: BlockNo(0),
                security_param: SecurityParam(k),
            }
        }

        #[test]
        fn win_emits_switch_to_winning_peer_and_durable_anchor() {
            // Candidate tip block 101 > current 100 => ChainSelected (block-no win).
            let mut competing = BTreeMap::new();
            competing.insert("peer-A".to_string(), candidate(50, 100, 100, 60, 0x22, 0x01));
            match decide_fork_switch(&state(100, 70, 0x05, 2160), &competing).expect("decides") {
                ForkSwitchDecision::Switch(s) => {
                    assert_eq!(s.winning_peer, "peer-A");
                    assert_eq!(s.fork_anchor.block_no, BlockNo(100));
                    assert_eq!(s.fork_anchor.slot, SlotNo(50));
                    assert_eq!(
                        s.winning_candidate.headers.last().unwrap().block_no,
                        BlockNo(101)
                    );
                }
                ForkSwitchDecision::KeepCurrent => panic!("a longer candidate must win"),
            }
        }

        #[test]
        fn tiebreaker_loss_keeps_current() {
            // Candidate tip block 100 == current 100; candidate slot 60 > current
            // slot 50 => current preferred (lower slot wins) => KeepCurrent.
            let mut competing = BTreeMap::new();
            competing.insert("peer-A".to_string(), candidate(49, 99, 100, 60, 0x22, 0x01));
            assert!(matches!(
                decide_fork_switch(&state(100, 50, 0x01, 2160), &competing).unwrap(),
                ForkSwitchDecision::KeepCurrent
            ));
        }

        #[test]
        fn exceeded_rollback_keeps_current() {
            // rollback_depth = current(100) - anchor(90) = 10 > k(5) =>
            // ExceededRollback (ineligible) => KeepCurrent, though the chain is
            // longer. (S4 keeps the independent materialize RollbackTooDeep guard.)
            let mut competing = BTreeMap::new();
            competing.insert("peer-A".to_string(), candidate(40, 90, 100, 60, 0x22, 0x01));
            assert!(matches!(
                decide_fork_switch(&state(100, 70, 0x05, 5), &competing).unwrap(),
                ForkSwitchDecision::KeepCurrent
            ));
        }

        #[test]
        fn best_of_two_peers_wins_and_is_identified() {
            // Two competing peers: B's tip (block 102) beats A's (block 101) => B
            // wins, and the winner is identified by the selector's returned tip.
            let mut competing = BTreeMap::new();
            competing.insert("peer-A".to_string(), candidate(50, 100, 100, 60, 0x2A, 0x01));
            competing.insert("peer-B".to_string(), candidate(50, 101, 100, 61, 0x2B, 0x02));
            match decide_fork_switch(&state(100, 70, 0x05, 2160), &competing).unwrap() {
                ForkSwitchDecision::Switch(s) => {
                    assert_eq!(s.winning_peer, "peer-B");
                    assert_eq!(s.winning_candidate.headers.last().unwrap().block_no, BlockNo(102));
                }
                ForkSwitchDecision::KeepCurrent => panic!("the longer of two candidates must win"),
            }
        }
    }

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
            empty.next_item().await.is_none(),
            "empty --peer must yield an ended feed (no block, no graft)"
        );
        // Unparseable addr: logged-and-skipped (C3), no pump task → ended feed.
        let mut bad = spawn_live_wire_pump_source(
            &["definitely-not-a-socket-addr".to_string()],
            1,
            None,
        );
        assert!(
            bad.next_item().await.is_none(),
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
            bootstrap_mithril: None,
            data_dir: None,
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
            mithril_state_path: None,
            mithril_tables_path: None,
            shelley_genesis_path: None,
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
            participant_venue: false,
            convergence_evidence_path: None,
            output_base: None,
            keep_raw_capture: false,
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

    // ===== MITHRIL-VERIFIED-ANCHOR-INTEGRATION S1d: NATIVE FirstRun route =====
    //
    // These dispatch-level tests exercise the NATIVE route gate (state + tables
    // present) and its fail-closed guards (forbidden flag, missing component)
    // through the real run_node_lifecycle_inner. They halt BEFORE any decode,
    // so the state/tables file CONTENTS are irrelevant (the files need not even
    // exist for the forbidden-flag / missing-flag guards). The positive native
    // bootstrap (real snapshot -> MithrilBootstrapOutput + anchor recoverable +
    // equals function-level S1b) is proven in crates/ade_node/tests/
    // native_firstrun_live.rs against the real preprod snapshot.

    /// A Node-mode Cli over a fresh tempdir carrying the NATIVE FirstRun inputs
    /// (manifest + state + tables + shelley genesis as paths). `forbidden`
    /// optionally adds a `--json-seed-path` to exercise the forbidden-flag
    /// terminal. Any path may be absent (`None`) to exercise a missing
    /// component.
    fn native_fixture(
        manifest: Option<&str>,
        state_present: bool,
        tables_present: bool,
        shelley_genesis: Option<&str>,
        forbidden_json_seed: bool,
    ) -> Fixture {
        let dir = TempDir::new().unwrap();
        let base = dir.path();
        let snap = base.join("snap");
        let wal = base.join("wal");
        let manifest_path = manifest.map(|m| write_file(base, "manifest.json", m));
        let state_path = if state_present {
            Some(write_file(base, "state", "synthetic-state-bytes"))
        } else {
            None
        };
        let tables_path = if tables_present {
            Some(write_file(base, "tables", "synthetic-tables-bytes"))
        } else {
            None
        };
        let shelley_path = shelley_genesis.map(|g| write_file(base, "shelley-genesis.json", g));
        let json_seed_path = if forbidden_json_seed {
            Some(write_file(base, "utxo.json", UTXO_JSON))
        } else {
            None
        };

        let cli = Cli {
            genesis_path: base.join("genesis.json"),
            network: "preprod".to_string(),
            chain_db_path: None,
            bootstrap_mithril: None,
            data_dir: None,
            snapshot_store_path: None,
            listen_addr: None,
            peer_addrs: vec![],
            mode: crate::cli::Mode::Node,
            log_path: base.join("node.jsonl"),
            tip_read_timeout_secs: 5,
            json_seed_path,
            seed_point_slot: None,
            seed_block_hash_hex: None,
            wal_dir: Some(wal),
            snapshot_dir: Some(snap),
            network_magic: None,
            genesis_hash_hex: None,
            consensus_inputs_path: None,
            mithril_manifest_path: manifest_path,
            mithril_state_path: state_path,
            mithril_tables_path: tables_path,
            shelley_genesis_path: shelley_path,
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
            participant_venue: false,
            convergence_evidence_path: None,
            output_base: None,
            keep_raw_capture: false,
        };
        Fixture { _dir: dir, cli }
    }

    const SHELLEY_GENESIS_JSON: &str = r#"{
        "maxLovelaceSupply": 45000000000000000,
        "activeSlotsCoeff": 0.05,
        "epochLength": 432000,
        "slotLength": 1,
        "systemStart": "2022-06-01T00:00:00Z"
    }"#;

    #[tokio::test]
    async fn native_first_run_bootstrap_mithril_requires_data_dir() {
        // ROUTE DISTINCTION (the contract's safety): on the --bootstrap-mithril route --snapshot-dir
        // is the READ-ONLY Mithril snapshot and --data-dir is Ade's store. Missing --data-dir is
        // terminal — a judge cannot accidentally put Ade storage into the Mithril snapshot dir.
        let mut f = native_fixture(
            Some(&manifest_json(CERTIFIED_SLOT, NETWORK_MAGIC, GENESIS_HASH_HEX)),
            true,
            true,
            Some(SHELLEY_GENESIS_JSON),
            false,
        );
        f.cli.bootstrap_mithril = f.cli.mithril_manifest_path.clone();
        f.cli.data_dir = None;
        let (_sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
        let r = run_node_lifecycle_inner(&f.cli, &mut sd_rx).await;
        assert!(
            matches!(r, Err(NodeLifecycleError::MissingFlag(m)) if m.contains("--data-dir")),
            "--bootstrap-mithril without --data-dir must be terminal, got {r:?}"
        );
    }

    #[tokio::test]
    async fn native_first_run_forbidden_json_seed_is_terminal() {
        // --json-seed-path supplied ALONGSIDE the native inputs => a structured
        // terminal error before any decode (no fallback, no silent ignore).
        let f = native_fixture(
            Some(&manifest_json(CERTIFIED_SLOT, NETWORK_MAGIC, GENESIS_HASH_HEX)),
            true,
            true,
            Some(SHELLEY_GENESIS_JSON),
            true, // the forbidden --json-seed-path
        );
        let (_sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
        let r = run_node_lifecycle_inner(&f.cli, &mut sd_rx).await;
        assert_eq!(
            r,
            Err(NodeLifecycleError::NativeRouteForbiddenFlag("--json-seed-path")),
            "a forbidden flag with the native inputs must be terminal, got {r:?}"
        );
        // Nothing persisted (terminal before any decode/admit).
        let snapshot_dir = f.cli.snapshot_dir.as_ref().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(snapshot_dir.join("chain.db")))
                .unwrap();
        assert!(SnapshotStore::list_snapshot_slots(&chaindb)
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn native_first_run_forbidden_consensus_inputs_is_terminal() {
        // --consensus-inputs-path supplied alongside the native inputs =>
        // terminal (the second forbidden flag).
        let mut f = native_fixture(
            Some(&manifest_json(CERTIFIED_SLOT, NETWORK_MAGIC, GENESIS_HASH_HEX)),
            true,
            true,
            Some(SHELLEY_GENESIS_JSON),
            false,
        );
        // Attach the forbidden --consensus-inputs-path directly.
        let cpath = write_file(f._dir.path(), "cinputs.json", "{}");
        f.cli.consensus_inputs_path = Some(cpath);
        let (_sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
        let r = run_node_lifecycle_inner(&f.cli, &mut sd_rx).await;
        assert_eq!(
            r,
            Err(NodeLifecycleError::NativeRouteForbiddenFlag(
                "--consensus-inputs-path"
            )),
            "a forbidden --consensus-inputs-path with the native inputs must be terminal, got {r:?}"
        );
    }

    #[tokio::test]
    async fn native_first_run_missing_manifest_is_terminal() {
        // state + tables present (native route taken) but the manifest absent
        // => a missing-component terminal before any decode.
        let f = native_fixture(None, true, true, Some(SHELLEY_GENESIS_JSON), false);
        let (_sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
        let r = run_node_lifecycle_inner(&f.cli, &mut sd_rx).await;
        assert_eq!(
            r,
            Err(NodeLifecycleError::MissingFlag("--mithril-manifest-path")),
            "native route with no manifest must be terminal, got {r:?}"
        );
    }

    #[tokio::test]
    async fn native_first_run_missing_genesis_and_unknown_network_is_terminal() {
        // The Shelley genesis is resolved from --network (a committed profile) OR
        // --shelley-genesis-path. With NEITHER a known --network NOR the genesis file there is no
        // genesis source => terminal. (A known --network supplies it; an unknown one cannot.)
        let mut f = native_fixture(
            Some(&manifest_json(CERTIFIED_SLOT, NETWORK_MAGIC, GENESIS_HASH_HEX)),
            true,
            true,
            None, // no --shelley-genesis-path
            false,
        );
        f.cli.network = "an-unsupported-network".to_string();
        let (_sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
        let r = run_node_lifecycle_inner(&f.cli, &mut sd_rx).await;
        assert_eq!(
            r,
            Err(NodeLifecycleError::MissingFlag(
                "--shelley-genesis-path (or a known --network: preview|preprod)"
            )),
            "no genesis file + an unknown --network must be terminal, got {r:?}"
        );
    }

    #[tokio::test]
    async fn native_first_run_malformed_manifest_is_terminal() {
        // A malformed manifest is fail-closed inside import_mithril_manifest
        // (terminal before any state decode).
        let f = native_fixture(
            Some("{ not valid manifest json"),
            true,
            true,
            Some(SHELLEY_GENESIS_JSON),
            false,
        );
        let (_sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
        let r = run_node_lifecycle_inner(&f.cli, &mut sd_rx).await;
        assert!(
            matches!(r, Err(NodeLifecycleError::NativeFirstRun(_))),
            "a malformed manifest on the native route must be terminal, got {r:?}"
        );
        // Nothing persisted.
        let snapshot_dir = f.cli.snapshot_dir.as_ref().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(snapshot_dir.join("chain.db")))
                .unwrap();
        assert!(SnapshotStore::list_snapshot_slots(&chaindb)
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn native_first_run_malformed_shelley_genesis_is_terminal() {
        // A shelley genesis missing maxLovelaceSupply => GenesisParse terminal.
        let f = native_fixture(
            Some(&manifest_json(CERTIFIED_SLOT, NETWORK_MAGIC, GENESIS_HASH_HEX)),
            true,
            true,
            Some(r#"{ "activeSlotsCoeff": 0.05, "epochLength": 432000 }"#),
            false,
        );
        let (_sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
        let r = run_node_lifecycle_inner(&f.cli, &mut sd_rx).await;
        assert!(
            matches!(r, Err(NodeLifecycleError::NativeFirstRun(_))),
            "a malformed shelley genesis on the native route must be terminal, got {r:?}"
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
    use ade_ledger::recovered_anchor_point::{encode_recovered_anchor_point, RecoveredAnchorPoint};
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
    // PHASE4-N-AK AK-S1: the recovered anchor POINT (below the tip; a real,
    // non-Origin block hash). At seed/recover the shared persist authority
    // writes this record alongside the seed-epoch sidecar (DC-NODE-31); the
    // warm-start anchor-point load fails closed without it, so every recovered
    // store the harness builds must carry it.
    const WARM_ANCHOR_SLOT: u64 = 23_013_600;
    const WARM_ANCHOR_HASH: Hash32 = Hash32([0x2e; 32]);

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
            epoch_start_slot: SlotNo(epoch.0 * 432_000),
            epoch_length_slots: 432_000,
            epoch_nonce: Nonce(Hash32([0x99; 32])),
            genesis_hash: Hash32([0x9a; 32]),
            protocol_params_hash: Hash32([0x9b; 32]),
            seed_point_slot: SlotNo(epoch.0 * 432_000 + 100),
            seed_point_hash: Hash32([0x6c; 32]),
            active_slots_coeff: ActiveSlotsCoeff {
                numer: 5,
                denom: 100,
            },
            total_active_stake: 1_000,
            pool_distribution: pools,
        }
    }

    /// PHASE4-N-AK AK-S1 (DC-NODE-31): persist the recovered anchor-point
    /// record bound to `WARM_ANCHOR_FP`, mirroring what
    /// `seed_epoch_lineage::persist_seed_epoch_consensus_inputs` writes at
    /// seed/recover. A recovered store the warm-start can recover from MUST
    /// carry this record (the warm-start anchor-point load fails closed
    /// otherwise); the durable-tip builders below write it so every existing
    /// warm-start test keeps a valid post-AK store.
    fn put_warm_anchor_point(chaindb: &PersistentChainDb) {
        let ap = RecoveredAnchorPoint {
            anchor_fp: WARM_ANCHOR_FP,
            slot: SlotNo(WARM_ANCHOR_SLOT),
            block_hash: WARM_ANCHOR_HASH,
        };
        chaindb
            .put_recovered_anchor_point(&WARM_ANCHOR_FP, &encode_recovered_anchor_point(&ap))
            .unwrap();
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
        // AK-S1: a recovered store carries the anchor-point record. With a
        // servable tip present, `resolve_live_follow_start` still returns that
        // tip (the anchor is below it) — these tests' tip assertions are
        // unchanged; the record only lets the warm-start load succeed.
        put_warm_anchor_point(chaindb);
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
        // AK-S1: as in `put_tip_and_snapshot`, a recovered store carries the
        // anchor-point record; with a servable tip the resolver still prefers
        // it, so the tip assertions are unchanged.
        put_warm_anchor_point(chaindb);
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

    #[test]
    fn warm_start_pre_v4_sidecar_is_typed_schema_upgrade_not_corruption() {
        // ECA-2-pre (DC-CINPUT-06): on the LIVE warm-start path, a well-formed
        // pre-v4 sidecar fails closed with the TYPED ConsensusInputsSchemaUnsupported
        // (a reimport requirement), DISTINCT from the generic WarmStartBootstrap
        // (corruption) -- so the live-path diagnostics match the bootstrap authority.
        let d = fresh_warm_dirs();
        // A valid current-schema (v5) sidecar, with the version uint (index 1; index
        // 0 is the array(13) header) rewritten 0x05 -> 0x03 so it decodes as an old schema.
        let mut bytes =
            encode_seed_epoch_consensus_inputs(&warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH));
        bytes[1] = 0x03;
        {
            let (chaindb, mut wal) = open_warm_stores(&d);
            chaindb
                .put_seed_epoch_consensus_inputs(&WARM_ANCHOR_FP, &bytes)
                .unwrap();
            append_seed_epoch_provenance(&mut wal, &WARM_ANCHOR_FP, WARM_EPOCH, &bytes).unwrap();
            put_durable_tip(&chaindb, &mut wal, WARM_TIP_SLOT);
        }

        let (chaindb, wal) = open_warm_stores(&d);
        let err = warm_start_recovery(&chaindb, &wal)
            .expect_err("a pre-v4 sidecar must fail closed on the warm-start path");
        assert!(
            matches!(
                err,
                NodeLifecycleError::ConsensusInputsSchemaUnsupported {
                    found_version: 3,
                    required_version: 5
                }
            ),
            "the live warm-start path must surface the TYPED schema-upgrade error, not generic corruption; got {err:?}"
        );
    }

    /// Capture a bare-Conway snapshot AT `slot` with NO stored block — a BARE
    /// anchor: `chaindb.tip()` stays `None` (no servable post-anchor block),
    /// the exact pre-AK regression precondition.
    fn put_bare_anchor_snapshot(chaindb: &PersistentChainDb, slot: u64) {
        let ledger = LedgerState::new(CardanoEra::Conway);
        let chain_dep = PraosChainDepState::genesis(Nonce(Hash32([0xCD; 32])));
        PersistentSnapshotCache::new(chaindb)
            .capture(SlotNo(slot), &ledger, &chain_dep)
            .unwrap();
    }

    #[test]
    fn recovered_bare_anchor_findintersect_starts_at_anchor_not_origin() {
        // CE-AK-2 (DC-NODE-31): a BARE-anchor warm-start (snapshot at the
        // anchor slot, NO servable post-anchor block, so `chaindb.tip()` is
        // None) resolves the live-follow start tip to the persisted anchor
        // POINT — so the wire pump FindIntersects at the anchor `Block` point,
        // NOT `Origin`. The pre-AK regression returned tip=None here -> Origin
        // -> the relay's RollBackward(Origin) tripped the AI-S4a fail-close.
        let d = fresh_warm_dirs();
        let record = warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH);
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        {
            let (chaindb, mut wal) = open_warm_stores(&d);
            chaindb
                .put_seed_epoch_consensus_inputs(&WARM_ANCHOR_FP, &bytes)
                .unwrap();
            append_seed_epoch_provenance(&mut wal, &WARM_ANCHOR_FP, WARM_EPOCH, &bytes).unwrap();
            // The recovered anchor POINT (real, non-Origin hash) — persisted at
            // seed/recover, loaded + verified at warm-start.
            put_warm_anchor_point(&chaindb);
            // A BARE anchor: a snapshot AT the anchor slot, NO servable block
            // above it. No AdmitBlock entries either (admit_count == 0).
            put_bare_anchor_snapshot(&chaindb, WARM_ANCHOR_SLOT);
            // stores dropped here -> restart boundary.
        }

        let (chaindb, wal) = open_warm_stores(&d);
        let state = warm_start_recovery(&chaindb, &wal).expect("bare-anchor warm-start recovers");

        // The live-follow start tip is the persisted anchor (slot + REAL hash),
        // NOT None — the durable restart authority is the store, not the CLI.
        let expected = ChainTip {
            slot: SlotNo(WARM_ANCHOR_SLOT),
            hash: WARM_ANCHOR_HASH,
        };
        assert_eq!(
            state.tip.as_ref(),
            Some(&expected),
            "bare-anchor recovery surfaces the persisted anchor as the live-follow tip"
        );

        // And the wire pump's FindIntersect start point is that anchor `Block`,
        // NOT `Origin` (so the AI-S4a Origin fail-close is never reached).
        let start = wire_pump_start_point(state.tip.as_ref());
        assert_eq!(
            start,
            ade_network::codec::chain_sync::Point::Block {
                slot: SlotNo(WARM_ANCHOR_SLOT),
                hash: WARM_ANCHOR_HASH,
            },
            "FindIntersect must start at the anchor Block point, not Origin"
        );
        assert_ne!(
            start,
            ade_network::codec::chain_sync::Point::Origin,
            "a bare-anchor recovery must NOT FindIntersect from Origin"
        );
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
    fn warmstart_from_real_admission_store_uses_persisted_bytes_no_mock() {
        // DURABLE-ADMISSION-BYTES (load-bearing positive): a store written with
        // the durable-admit contract `chaindb.put_block(bytes)` THEN
        // `wal.append(AdmitBlock{hash})` — the EXACT ordering the admission
        // runner now performs (admission/runner.rs), reproduced here by
        // `put_durable_tip` — recovers across a fresh open. warm_start_recovery
        // takes NO injected byte map: it reads the preserved bytes back out of
        // the persistent ChainDb. Pairs with the negative below (remove the
        // bytes -> fail closed), which together prove the recovery consumes the
        // REAL persistent store, not a harness-supplied map.
        let d = fresh_warm_dirs();
        let record = warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH);
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        {
            let (chaindb, mut wal) = open_warm_stores(&d);
            chaindb
                .put_seed_epoch_consensus_inputs(&WARM_ANCHOR_FP, &bytes)
                .unwrap();
            append_seed_epoch_provenance(&mut wal, &WARM_ANCHOR_FP, WARM_EPOCH, &bytes).unwrap();
            // put_block(hash=0xBB, bytes=0xAB;8) THEN wal.append(AdmitBlock{0xBB}).
            put_durable_tip(&chaindb, &mut wal, WARM_TIP_SLOT);
            // stores dropped here -> fresh-open / restart boundary.
        }

        let (chaindb, wal) = open_warm_stores(&d);
        let state = warm_start_recovery(&chaindb, &wal)
            .expect("warm-start recovers from the persisted admission store (no mock)");
        assert_eq!(
            state.tip.map(|t| t.slot.0),
            Some(WARM_TIP_SLOT),
            "recovered live-follow tip is the durably-admitted block"
        );
        // The preserved bytes are the REAL ones the contract wrote, retrievable
        // by hash from the same persistent store the recovery read.
        let back = ChainDb::get_block_by_hash(&chaindb, &Hash32([0xBB; 32]))
            .unwrap()
            .expect("the admitted block's bytes are durable in the ChainDb");
        assert_eq!(
            back.bytes,
            vec![0xAB; 8],
            "byte-identical preserved admission block"
        );
    }

    #[test]
    fn warmstart_fails_closed_when_wal_admitblock_missing_bytes() {
        // DURABLE-ADMISSION-BYTES (load-bearing negative): a WAL AdmitBlock
        // whose preserved bytes are ABSENT from the ChainDb is corrupted durable
        // state, NOT block absence. warm_start_recovery must fail closed with
        // DurableBlockBytesMissing — never the prior silent skip that masked the
        // admission-runner persistence gap behind an empty replay map. This is
        // the positive above MINUS the chaindb.put_block (the exact pre-fix gap).
        let d = fresh_warm_dirs();
        let record = warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH);
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        let admitted_hash = Hash32([0xBB; 32]);
        {
            let (chaindb, mut wal) = open_warm_stores(&d);
            chaindb
                .put_seed_epoch_consensus_inputs(&WARM_ANCHOR_FP, &bytes)
                .unwrap();
            // The WAL records an admitted block, but its bytes were NEVER
            // persisted to the ChainDb (no put_block) — the pre-fix gap.
            let ledger = LedgerState::new(CardanoEra::Conway);
            wal.append(ade_ledger::wal::WalEntry::AdmitBlock {
                prior_fp: WARM_ANCHOR_FP,
                block_hash: admitted_hash.clone(),
                slot: SlotNo(WARM_TIP_SLOT),
                verdict: ade_ledger::wal::BlockVerdictTag::Valid,
                post_fp: fingerprint(&ledger).combined,
            })
            .unwrap();
        }
        let (chaindb, wal) = open_warm_stores(&d);
        let r = warm_start_recovery(&chaindb, &wal);
        match r {
            Err(NodeLifecycleError::DurableBlockBytesMissing {
                block_hash,
                entry_index,
                source,
            }) => {
                assert_eq!(block_hash, admitted_hash, "names the block whose bytes are absent");
                assert_eq!(source, "ChainDb::get_block_by_hash", "names the failed lookup");
                assert_eq!(entry_index, 0, "the sole WAL entry (the AdmitBlock) index");
            }
            other => panic!(
                "absent admit-block bytes must fail closed with DurableBlockBytesMissing, got {other:?}"
            ),
        }
    }

    #[test]
    fn warm_start_schedule_locates_block_by_venue_geometry_not_hardcoded_432000() {
        // WARMSTART-ERA-SCHEDULE-VENUE (DC-CINPUT-05) regression for the live
        // C2-PREVIEW forge failure. The warm-start/forge schedule must use the
        // VENUE epoch length, never the hardcoded preprod 432000. The prior
        // warm-start tests all used snapshot-at-tip (DEGENERATE forward-replay),
        // so they never called EraSchedule::locate -- this exercises it directly,
        // the exact HFC slot->epoch step that failed live.
        //
        // PREVIEW (epoch_length 86400): epoch 1331 starts at 114_998_400. A
        // followed block at slot 115_030_409 (~77 slots past the seed) is WITHIN
        // epoch 1331, so the venue schedule LOCATES it (no SlotBeforeSystemStart).
        let preview = make_node_schedule(SlotNo(114_998_400), EpochNo(1331), 86_400, None);
        assert!(
            preview.locate(SlotNo(115_030_409)).is_ok(),
            "preview venue geometry must locate the followed block, got {:?}",
            preview.locate(SlotNo(115_030_409))
        );

        // The PRE-FIX hardcoded behavior placed the era start at epoch_no*432000 =
        // 574_992_000 -- AFTER the block. locate() then fails SlotBeforeSystemStart,
        // the EXACT live failure: wrong geometry rejects deterministically.
        let wrong = make_node_schedule(SlotNo(1331 * 432_000), EpochNo(1331), 432_000, None);
        let err = wrong
            .locate(SlotNo(115_030_409))
            .expect_err("wrong (preprod-length) geometry must reject the preview block");
        let shown = format!("{err:?}");
        assert!(
            shown.contains("SlotBeforeSystemStart") && shown.contains("574992000"),
            "wrong geometry must reject deterministically as SlotBeforeSystemStart@574992000, got {shown}"
        );

        // PREPROD (epoch_length 432000): the SAME code path is venue-correct for
        // preprod -- epoch 580 starts at 250_560_000; a block 500 slots in locates.
        let preprod = make_node_schedule(SlotNo(580 * 432_000), EpochNo(580), 432_000, None);
        assert!(
            preprod.locate(SlotNo(580 * 432_000 + 500)).is_ok(),
            "preprod venue geometry must locate its block"
        );
    }

    #[test]
    fn restart_genesis_epoch_length_mismatch_fails_closed() {
        // WARMSTART-ERA-SCHEDULE-VENUE (DC-CINPUT-05): the durable sidecar geometry
        // is authority; a restart --genesis-file is ONLY a consistency check. The
        // sidecar here persists epoch_length_slots = 432_000 (preprod).
        let dir = tempfile::tempdir().expect("tmpdir");
        let sidecar = warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH);
        assert_eq!(sidecar.epoch_length_slots, 432_000);

        // Matching epochLength -> Ok.
        let matching = dir.path().join("match.json");
        std::fs::write(&matching, br#"{"epochLength": 432000}"#).unwrap();
        assert!(assert_restart_genesis_matches_sidecar(Some(&matching), &sidecar).is_ok());

        // A DIFFERENT venue's epochLength (86400 preview) -> fail closed.
        let mismatch = dir.path().join("mismatch.json");
        std::fs::write(&mismatch, br#"{"epochLength": 86400}"#).unwrap();
        match assert_restart_genesis_matches_sidecar(Some(&mismatch), &sidecar) {
            Err(NodeLifecycleError::RestartGenesisGeometryMismatch {
                sidecar_epoch_length,
                genesis_epoch_length,
            }) => {
                assert_eq!(sidecar_epoch_length, 432_000);
                assert_eq!(genesis_epoch_length, 86_400);
            }
            other => panic!("mismatched genesis epochLength must fail closed, got {other:?}"),
        }

        // No genesis supplied -> sidecar stands alone, no check.
        assert!(assert_restart_genesis_matches_sidecar(None, &sidecar).is_ok());

        // A genesis without an epochLength field -> non-authoritative, no check.
        let no_field = dir.path().join("nofield.json");
        std::fs::write(&no_field, br#"{"systemStart": "2022-01-01T00:00:00Z"}"#).unwrap();
        assert!(assert_restart_genesis_matches_sidecar(Some(&no_field), &sidecar).is_ok());
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
            bootstrap_mithril: None,
            data_dir: None,
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
            mithril_state_path: None,
            mithril_tables_path: None,
            shelley_genesis_path: None,
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
            participant_venue: false,
            convergence_evidence_path: None,
            output_base: None,
            keep_raw_capture: false,
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
        // OP-OPS-04: the opcert covers the recovered tip's ABSOLUTE KES period
        // (WARM_TIP_SLOT / slotsPerKESPeriod = 23_013_663 / 129_600 = 177), so the
        // injected current period 177 lands at the opcert start (delta 0). CBOR
        // uint 177 = 0x18 0xB1.
        ocbor.extend_from_slice(&[0x18, 177]); // kes_period 177
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
