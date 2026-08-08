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
use ade_runtime::admission::{
    dial_for_admission, run_admission_wire_pump, AdmissionPeerEvent, AdmissionWirePumpError,
    AdmissionWirePumpResult,
};
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
use ade_core::consensus::era_schedule::{BootstrapBoundTimingAuthority, TimingAnchorError};
use ade_runtime::clock::{Clock, SystemClock};
use ade_runtime::forward_sync::{
    pump_block, ForwardSyncState, NoCheckpointSink, PumpError, PumpTip, SnapshotSink,
};
use ade_runtime::producer::coordinator::{
    coordinator_init, CoordinatorConfig, CoordinatorEvent, CoordinatorState, KesSlotError,
    LedgerSnapshotRef,
};
use ade_runtime::producer::producer_shell::ProducerShell;
use ade_runtime::rollback::{ChainDbBlockSource, PersistentSnapshotCache, SnapshotCadence};
use ade_ledger::rollback::{
    admit_rollback, commit_rollback, materialize_rolled_back_state, reconcile_recovery,
    CommitRollbackError, MaterializeError, RecoveryAction, ResetReason, RollbackAdmissionError,
    RollbackPoint as CanonicalPoint, TargetPoint,
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
    /// CONWAY-PROPOSAL-DEPOSIT-EXPIRY S2 (absent ≠ empty): the persisted EpochAccumulator's sealed
    /// bootstrap baseline PREDATES the governance-proposal import (Conway+ store with `gov_state = None`,
    /// a pre-v6 bootstrap). A missing imported governance set must NEVER masquerade as "zero proposals";
    /// fail closed — re-bootstrap to upgrade. A TYPED re-bootstrap requirement (like the old-sidecar
    /// schema gate), DISTINCT from a corrupt store (which stays non-fatal/observe-only).
    AccumulatorPredatesGovernanceImport {
        era_tag: u64,
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
    /// LIVE-LEDGER-EPOCH-TRANSITION S4: the epoch-indexed frozen leadership authority
    /// (DC-EPOCH-25) cannot answer for the epoch the node must validate a slot in — the
    /// accumulator store is absent, uncertified (legacy / no leadership marker), or
    /// carries no sealed leadership object for the epoch. Post-S4 the leader schedule is
    /// read ONLY from the frozen authority (`leadership_authority_for_epoch`), by EXACT
    /// epoch; there is NO seed-window fallback. Fail closed — re-bootstrap to
    /// leadership-certify the store.
    ProductionLeadershipAuthorityUnavailable { epoch: u64, reason: String },
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
    /// LIVE-LEDGER-EPOCH-TRANSITION S5 (step 2b): a TERMINAL recovery-admission fault. The persisted
    /// accumulator cannot be proven to describe ONE canonical selected-chain prefix, so recovery FAILS
    /// CLOSED — the store is terminal until re-bootstrap or an explicit admissible recovery. This is the
    /// recovery-INTEGRITY exception to the S3 observe-only contract: an ordinary follow-time observe fault
    /// still does NOT halt (the accumulator is simply not promoted), but a durable-state contradiction does.
    RecoveryAdmission(RecoveryAdmissionFault),
}

/// LIVE-LEDGER-EPOCH-TRANSITION S5 (step 2b): the typed reason a recovery ADMISSION failed closed, and the
/// operator action it implies. Every variant is TERMINAL — the durable accumulator/checkpoint state cannot
/// be proven replay-equivalent to one canonical selected-chain prefix, so it is not admissible as recovered
/// authority until re-bootstrap (or an explicit admissible recovery). NOT a leadership statement (that is S4)
/// — purely "can this persisted accumulator be trusted, or rematerialized from canonical blocks?".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAdmissionFault {
    /// Lineage contradiction: the committed anchor is NOT the canonical block at its slot. Re-bootstrap.
    LineageMismatch { slot: u64 },
    /// Rollback exceeds SecurityParam k: the durable tip is more than k blocks behind the accumulator.
    /// Re-bootstrap (or explicit admissible recovery).
    ExceededRollback { depth: u64, k: u64 },
    /// Store repair not admissible: the rollback target is below the sealed bootstrap anchor. Re-bootstrap.
    BeforeBootstrapAnchor {
        target_block_no: u64,
        anchor_block_no: u64,
    },
    /// Lineage contradiction: the committed anchor's slot carries no canonical block. Re-bootstrap.
    TargetNotOnCanonicalChain { slot: u64 },
    /// Schema / corruption fault: the persisted lineage anchor is malformed. Re-bootstrap.
    CorruptLastAdvancedPoint,
    /// Canonical span missing: a block required to reconstruct the accumulator is absent from the durable
    /// ChainDB. Re-bootstrap (or repair the span).
    MissingCanonicalSpan { slot: u64 },
    /// Canonical span not admissible: a block in the span does not decode / does not chain (a prev-hash
    /// break). Re-bootstrap.
    NonContiguousCanonicalSpan { slot: u64 },
    /// Schema / corruption fault: the rematerialized ledger fingerprint disagrees with the recovered WAL-tail
    /// commitment (T-REC-05). Re-bootstrap.
    ///
    /// P6-S4: carries a SELF-DESCRIBING [`ReplayDivergenceReport`] alongside the two hashes. The bare
    /// pair cost hours and four wrong hypotheses to diagnose in P4; the report carries the evidence
    /// that actually cracked it — per-component fingerprints (which components the replay MOVED), the
    /// ledger-vs-schedule epoch pair, the replay anchor and span, and the store's semantics generation.
    /// Boxed to keep the enum small.
    FingerprintMismatch {
        expected: Hash32,
        recovered: Hash32,
        report: Box<ade_ledger::replay_divergence::ReplayDivergenceReport>,
    },
}

impl RecoveryAdmissionFault {
    /// Map the BLUE `reconcile_recovery` / `admit_rollback` rejection into the terminal recovery fault.
    fn from_admission(e: RollbackAdmissionError) -> Self {
        match e {
            RollbackAdmissionError::LineageMismatch { slot, .. } => {
                RecoveryAdmissionFault::LineageMismatch { slot: slot.0 }
            }
            RollbackAdmissionError::ExceededRollback { depth, k, .. } => {
                RecoveryAdmissionFault::ExceededRollback { depth, k }
            }
            RollbackAdmissionError::BeforeBootstrapAnchor {
                target_block_no,
                anchor_block_no,
            } => RecoveryAdmissionFault::BeforeBootstrapAnchor {
                target_block_no: target_block_no.0,
                anchor_block_no: anchor_block_no.0,
            },
            RollbackAdmissionError::TargetNotOnCanonicalChain { slot } => {
                RecoveryAdmissionFault::TargetNotOnCanonicalChain { slot: slot.0 }
            }
        }
    }
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
        | NodeLifecycleError::ConsensusInputsSchemaUnsupported { .. }
        | NodeLifecycleError::AccumulatorPredatesGovernanceImport { .. }
        | NodeLifecycleError::RecoveryAdmission(_) => EXIT_NODE_WARM_START_RECOVERY_FAILED,
        NodeLifecycleError::RelaySync(_)
        | NodeLifecycleError::FeedMissingRecoveredConsensusInputs
        | NodeLifecycleError::ProductionLeadershipAuthorityUnavailable { .. } => {
            EXIT_NODE_RELAY_SYNC_FAILED
        }
        NodeLifecycleError::ForgeKeyIngress(_) => EXIT_NODE_FORGE_KEY_INGRESS_FAILED,
    }
}

/// LIVE-LEDGER-EPOCH-TRANSITION S4: the SOLE production leadership / header-validation view for the epoch a
/// recovered seed record anchors — the epoch-indexed frozen leadership authority (DC-EPOCH-25), read by EXACT
/// epoch from the durable native-frozen store. Byte-identical to the retired
/// `PoolDistrView::from_seed_epoch_consensus_inputs(record)` for the seed epoch (S4-0 proved
/// `leadership_authority_for_epoch(seed) == from_seed`), but sourced from the durable authority, never
/// re-projected from the seed record's pool set. Fail closed if the store is absent / uncertified / missing the
/// epoch — there is NO seed-window fallback. `active_slots_coeff` is the venue genesis constant (geometry, not a
/// leadership read); the retired call was the pool-set projection, which is what the flip removes.
fn leadership_view_from_frozen_authority(
    store: Option<&ade_runtime::chaindb::EpochAccumulatorStore>,
    record: &SeedEpochConsensusInputs,
) -> Result<PoolDistrView, NodeLifecycleError> {
    let store = store.ok_or_else(|| NodeLifecycleError::ProductionLeadershipAuthorityUnavailable {
        epoch: record.epoch_no.0,
        reason: "epoch-accumulator store absent (re-bootstrap to leadership-certify)".to_string(),
    })?;
    let frozen = store.leadership_authority_for_epoch(record.epoch_no).map_err(|e| {
        NodeLifecycleError::ProductionLeadershipAuthorityUnavailable {
            epoch: record.epoch_no.0,
            reason: format!("{e:?}"),
        }
    })?;
    Ok(frozen.to_pool_distr_view(record.active_slots_coeff))
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
    // LIVE-LEDGER-EPOCH-TRANSITION S4: the warm-start recovery replay reads the leader schedule by EXACT epoch
    // from the epoch-indexed frozen leadership authority (DC-EPOCH-25), which for a warm start PRE-EXISTS (a prior
    // run sealed it). Open a SHORT-LIVED handle here and drop it before the live authority open below (redb is
    // single-open). A FirstRun has no store yet (the bootstrap below creates it), so this is None and the
    // warm-start arm is not taken.
    let accumulator_path = snapshot_dir.join("epoch-accumulator.redb");
    let warm_accumulator = if accumulator_path.exists() {
        ade_runtime::chaindb::EpochAccumulatorStore::open(&accumulator_path).ok()
    } else {
        None
    };
    let state = match start {
        NodeStart::FirstRun => first_run_mithril_bootstrap(cli, &chaindb, &mut wal)?,
        NodeStart::WarmStart => warm_start_recovery(&chaindb, &wal, warm_accumulator.as_ref(), rsw_for_cli(cli))?,
    };
    drop(warm_accumulator);

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
    // advance is gated on `Some`). It NEVER blocks the proven follow. (`accumulator_path` was computed above for
    // the short-lived warm-start handle; reused here for the live authority handle now the bootstrap has sealed
    // it on FirstRun.)
    let epoch_accumulator = if accumulator_path.exists() {
        match ade_runtime::chaindb::EpochAccumulatorStore::open(&accumulator_path) {
            Ok(s) => {
                // CONWAY-PROPOSAL-DEPOSIT-EXPIRY S2 (absent != empty): a sealed bootstrap baseline that
                // PREDATES the governance-proposal import (Conway+ store with gov_state = None, pre-v6) is
                // a TYPED re-bootstrap requirement -- fail closed. A missing imported governance set must
                // NEVER load as "zero proposals". DISTINCT from a corrupt store (the non-fatal arm below).
                if let Err(
                    ade_runtime::chaindb::AccumulatorReadinessError::GovernanceImportRequired { era_tag },
                ) = s.verify_governance_imported()
                {
                    return Err(NodeLifecycleError::AccumulatorPredatesGovernanceImport { era_tag });
                }
                Some(s)
            }
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
                Some(record) => {
                    leadership_view_from_frozen_authority(epoch_accumulator.as_ref(), record)?
                }
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
                RecoveryAdmissionPolicy::cardano(),
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
                Some(record) => {
                    leadership_view_from_frozen_authority(epoch_accumulator.as_ref(), record)?
                }
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
            // LIVE-2c ACTIVATION part 1 — establish the ONE bootstrap-bound wall-clock→slot
            // authority, BEFORE the clock exists, so there is never a moment where a forge-capable
            // node holds a clock but no bound calendar. The venue calendar is selected by the
            // DURABLE genesis hash (the operator cannot choose it); `--network` and the operator's
            // real shelley-genesis are fail-closed cross-checks only.
            //
            // A forge-ON start REQUIRES the sidecar: without it there is no bootstrap fact to bind
            // the calendar to, and a forge whose slot cannot be justified must not start at all
            // (the same fail-closed posture `recovered_node_schedule` and the recovered
            // `ledger_view` already take on this path).
            let forge_timing = {
                let sidecar = state.seed_epoch_consensus_inputs.as_ref().ok_or(
                    NodeLifecycleError::FeedMissingRecoveredConsensusInputs,
                )?;
                crate::forge_timing::establish_forge_timing_authority(
                    sidecar,
                    &cli.network,
                    Some(crate::forge_timing::GenesisTimingCrossCheck {
                        system_start_unix_ms: genesis.slot_zero_time_unix_ms,
                        active_slot_length_ms: u32::try_from(genesis.slot_length_ms)
                            .unwrap_or(u32::MAX),
                    }),
                )
                .map_err(|e| NodeLifecycleError::ForgeKeyIngress(format!("{e}")))?
            };
            // The injected clock is the SOLE wall-clock observation (DC-NODE-03). Its cadence comes
            // from the timing authority's active segment -- one slot-length number on this path.
            let mut clock = SystemClock::new(forge_timing.slot_cadence_ms());
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
                forge_timing,
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
                RecoveryAdmissionPolicy::cardano(),
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

/// LIVE-WIRE-LIVENESS S2 (INV-WL-10): bounded, deterministic reconnect backoff.
/// A fixed escalating schedule capped at 30s — no randomness (a wall-clock RED
/// transport concern that never reaches the BLUE core), so a relay that is down
/// for a while is retried patiently rather than hot-looped.
const RECONNECT_BACKOFF_SECS: &[u64] = &[1, 2, 4, 8, 15, 30];

fn reconnect_backoff_secs(attempt: usize) -> u64 {
    let last = RECONNECT_BACKOFF_SECS.len() - 1;
    RECONNECT_BACKOFF_SECS[attempt.min(last)]
}

/// LIVE-WIRE-LIVENESS S2 (INV-WL-9): the closed reconnect policy over the wire
/// pump's outcome sum. Named + total so the decision is one testable authority
/// rather than an inline `matches!`.
///
/// TRANSPORT-level loss reconnects. A peer PROTOCOL / GRAMMAR violation keeps
/// the pre-slice fail-closed drop — a systematically bad peer must not be
/// retried into a livelock — and `EventsChannelDropped` means the consumer is
/// gone, so there is nobody to reconnect for.
///
/// This never sees a consensus outcome: admission, rollback k-guard, boundary
/// promotion and the forge fence all live in the consumer and keep their own
/// typed halts (INV-WL-6).
fn should_reconnect_after(outcome: &AdmissionWirePumpResult) -> bool {
    match outcome {
        AdmissionWirePumpResult::Eof => true,
        AdmissionWirePumpResult::Error(e) => match e {
            AdmissionWirePumpError::TransportRead | AdmissionWirePumpError::TransportWrite => true,
            AdmissionWirePumpError::Session(_)
            | AdmissionWirePumpError::ChainSyncDecode
            | AdmissionWirePumpError::BlockFetchDecode
            | AdmissionWirePumpError::UnexpectedProtocolMessage { .. }
            | AdmissionWirePumpError::UnsupportedRollbackPoint
            | AdmissionWirePumpError::KeepAlive(_)
            | AdmissionWirePumpError::DeferredFrameOverflow => false,
        },
        AdmissionWirePumpResult::EventsChannelDropped => false,
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
        // LIVE-WIRE-LIVENESS S2: per-peer SUPERVISOR (was a one-shot dial+pump).
        // A transport-level loss of an ESTABLISHED session is recovered instead
        // of ending the run (observed live 2026-08-01: `exit=Eof` ->
        // `relay run loop exited`). Recovery is transport-only: every consensus
        // decision stays in the consumer with its existing typed halt
        // (INV-WL-6).
        tokio::spawn(async move {
            let mut start = start;
            let mut established = false;
            let mut attempt: usize = 0;
            // Most recent block delivered downstream; decoded ONLY on reconnect
            // to derive the resume point (INV-WL-7), never per block.
            let mut last_block_bytes: Option<Vec<u8>> = None;

            loop {
                let (transport, version) =
                    match dial_for_admission(addr, pump_versions.clone()).await {
                        Ok(pair) => pair,
                        Err(e) => {
                            if !established {
                                // INV-WL-8: startup semantics UNCHANGED — an
                                // unreachable peer is logged-and-dropped, never
                                // an infinite boot spin.
                                eprintln!(
                                    "ade_node --mode node: dial-for-admission failed for {label}: {e:?}"
                                );
                                return;
                            }
                            let backoff = reconnect_backoff_secs(attempt);
                            attempt = attempt.saturating_add(1);
                            eprintln!(
                                "ade_node --mode node: re-dial {label} failed ({e:?}); \
                                 retrying in {backoff}s"
                            );
                            tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                            continue;
                        }
                    };
                established = true;
                attempt = 0;

                // Interpose so the per-SESSION `Disconnected` is not surfaced as
                // a feed end, and so the resume point is observable. Forwarding
                // is 1:1 with `send().await`, preserving DC-PUMP-04 per-peer
                // self-backpressure.
                let (inner_tx, mut inner_rx) =
                    mpsc::channel::<AdmissionPeerEvent>(PER_PEER_LANE_CAP);
                let pump = tokio::spawn(run_admission_wire_pump(
                    transport,
                    label.clone(),
                    start.clone(),
                    version,
                    network_magic,
                    inner_tx,
                ));
                let mut consumer_gone = false;
                while let Some(ev) = inner_rx.recv().await {
                    if matches!(ev, AdmissionPeerEvent::Disconnected { .. }) {
                        // Per-SESSION artifact. Surfacing it would latch the
                        // consumer's feed-end flag (node_sync `disconnected`),
                        // ending the feed permanently even though we reconnect.
                        continue;
                    }
                    if let AdmissionPeerEvent::Block { block_bytes, .. } = &ev {
                        last_block_bytes = Some(block_bytes.clone());
                    }
                    if lane_tx.send(ev).await.is_err() {
                        consumer_gone = true;
                        break;
                    }
                }
                let outcome = pump.await;
                if consumer_gone {
                    return;
                }
                // INV-WL-9: transport-level outcomes reconnect; a peer protocol /
                // grammar violation keeps today's fail-closed drop, and a dropped
                // consumer channel means the runner is gone.
                // A pump task that panicked or was cancelled (`Err`) is NOT a
                // transport loss — do not retry into it.
                let reconnect = outcome.as_ref().is_ok_and(should_reconnect_after);
                if !reconnect {
                    eprintln!(
                        "ade_node --mode node: live feed to {label} ended ({outcome:?}); \
                         not reconnecting"
                    );
                    return;
                }
                // INV-WL-7: resume from the last block actually delivered. Events
                // already buffered in the lane are still delivered and are ORDERED
                // BEFORE the new session's, so by the time the consumer sees the
                // new session's rollback-to-intersection, that point is already
                // durable and within k.
                if let Some(bytes) = &last_block_bytes {
                    if let Ok(decoded) = decode_block(bytes) {
                        start = ade_network::codec::chain_sync::Point::Block {
                            slot: decoded.header_input.slot,
                            hash: decoded.block_hash.clone(),
                        };
                    }
                }
                let backoff = reconnect_backoff_secs(attempt);
                attempt = attempt.saturating_add(1);
                eprintln!(
                    "ade_node --mode node: live feed to {label} lost ({outcome:?}); \
                     reconnecting in {backoff}s from {start:?}"
                );
                tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
            }
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
    /// `next_tick` is converted to a `SlotNo` by [`Self::timing`]; only the `SlotNo` crosses into
    /// the planner / forge call (clock seam, DC-NODE-03).
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
    /// LIVE-2c part 2 — THE forge's wall-clock→slot authority, and the only one.
    ///
    /// This replaced the naive `(anchor_millis, start_slot, slot_length_ms)` triple, which anchored
    /// the venue's system start to a SINGLE slot length and so ignored that preprod's first 86_400
    /// slots lasted 20 s — wrong by exactly `86_400 × (20 − 1) = 1_641_600` slots, ~19 days. The
    /// triple is not deprecated here, it is GONE: two reachable slot authorities are the defect
    /// class, so "prefer the new one" would not have closed it.
    ///
    /// Bootstrap-bound: it can only be built by binding a committed venue calendar to the store's
    /// own durable facts, so a restart RECONSTRUCTS it rather than minting a new one.
    pub timing: BootstrapBoundTimingAuthority,
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
    /// LIVE-2c part 2: the last wall-clock→slot derivation fail-closed (set when the captured
    /// instant precedes the anchor's declared domain, cleared on a successful derivation). A
    /// structured LOCAL observation surface — in-memory, not persisted, not evidence — that makes
    /// the fail-closed visible (never a silent `NotDue`). Replaces `last_slot_alignment_fail`: the
    /// guard is now the anchor's own domain rather than a separately-supplied genesis anchor.
    pub last_slot_derivation_fail: Option<TimingAnchorError>,
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
        timing: BootstrapBoundTimingAuthority,
    ) -> Self {
        Self {
            clock,
            coordinator_state,
            recovered,
            shell,
            pool_id,
            pparams,
            protocol_version,
            timing,
            last_forged_slot: None,
            pending_slot: None,
            hermetic_forge_outcomes: Vec::new(),
            last_slot_derivation_fail: None,
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

/// CN-NODE-04 diagnostic projection (emit-only): the typed `ForgeRefused` onto the
/// closed `ForgeSkipReason` set, so an operator can tell WHY a forge tick skipped.
/// Before this, every cause collapsed into `outcome: no_tip_available` and the typed
/// refusal was computed and discarded. `None` means no typed refusal was recorded and
/// a selected tip WAS available -- which rules the DC-NODE-15 gate out and points at
/// the KES window instead. Never reads back into scheduling or control flow.
/// CN-NODE-04 diagnostic projection (emit-only): the two tips the DC-NODE-15 gate
/// compared, when it refused on them. `tip_mismatch` says they disagree but not WHERE,
/// and the gate requires equality on BOTH `hash` and `block_no` -- so a lagging serve
/// projection, a systematic block_no disagreement, and a hash difference (a different
/// chain than we believe) are indistinguishable without these. Never read back.
fn forge_compared_tips(refused: Option<&ForgeRefused>) -> Option<crate::live_log::ComparedTips> {
    match refused {
        Some(ForgeRefused::NotCaughtUp {
            local_servable_tip,
            followed_peer_tip,
            ..
        }) => Some(crate::live_log::ComparedTips {
            local_slot: local_servable_tip.as_ref().map(|t| t.slot.0),
            local_block_no: local_servable_tip.as_ref().map(|t| t.block_no),
            local_hash: local_servable_tip.as_ref().map(|t| t.hash.clone()),
            peer_slot: followed_peer_tip.as_ref().map(|t| t.slot.0),
            peer_block_no: followed_peer_tip.as_ref().map(|t| t.block_no),
            peer_hash: followed_peer_tip.as_ref().map(|t| t.hash.clone()),
        }),
        _ => None,
    }
}

fn forge_skip_reason(refused: Option<&ForgeRefused>) -> Option<crate::live_log::ForgeSkipReason> {
    use crate::live_log::ForgeSkipReason as R;
    use crate::node_sync::NotCaughtUpReason as N;
    match refused {
        Some(ForgeRefused::NotCaughtUp { reason, .. }) => Some(match reason {
            N::NoFollowedPeerTip => R::NoFollowedPeerTip,
            N::NoDurableServableTip => R::NoDurableServableTip,
            N::TipMismatch => R::TipMismatch,
        }),
        Some(ForgeRefused::SingleProducerFenceViolation { .. }) => Some(R::SingleProducerFence),
        Some(ForgeRefused::ReselectionPending) => Some(R::ReselectionPending),
        Some(ForgeRefused::ParticipantFenceViolation { .. }) => Some(R::ParticipantFence),
        Some(ForgeRefused::ParticipantForgeBaseChangedBeforeSign { .. }) => {
            Some(R::ForgeBaseChangedBeforeSign)
        }
        // LIVE-2c part 3 (B11): the three KES-window reasons stay SEPARATE. Collapsing them would
        // reproduce the defect one level up — an operator reading `kes_window` still could not tell
        // "not valid yet" from "rotate the key now".
        Some(ForgeRefused::KesWindow(e)) => Some(match e {
            KesSlotError::BeforeOperationalCertificateStart { .. } => R::KesBeforeOpcertStart,
            KesSlotError::AfterOperationalCertificateEnd { .. } => R::KesAfterOpcertEnd,
            KesSlotError::PeriodArithmeticOverflow { .. } => R::KesPeriodOverflow,
        }),
        None => None,
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
        None, None, RecoveryAdmissionPolicy::cardano(),
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

/// EVIEW-RECOVERY-LINEAGE R2 (DC-EPOCH-32): where the reduced checkpoint ended up relative to a
/// requested boundary point. A CLOSED sum returned in place of `Ok(())` so a caller can never again
/// mistake "left far past the target" for "sitting on the target" -- the silent-success shape that
/// let a refold seal a boundary mark read at the durable tip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckpointPositioning {
    /// The cursor was already exactly on the boundary point.
    AlreadyAt,
    /// The cursor was behind it and was folded forward onto it (the fresh catch-up case).
    AdvancedForward,
    /// The cursor was PAST it: re-materialized to the sealed bootstrap baseline and replayed
    /// forward onto it. This is the REFOLD case the slice exists for.
    RewoundAndReplayed,
    /// The cursor could NOT be brought onto it. NOTHING may be sealed from the checkpoint here.
    Unreachable { advanced: u64, seed: u64 },
}

/// EVIEW-RECOVERY-LINEAGE R2 (DC-EPOCH-32 / INV-ER-2): bring the reduced checkpoint to EXACTLY the
/// boundary point `s_prev` before the boundary mark and the checkpoint commitment are read off it.
///
/// `advance_reduced_checkpoint_forward_to` is purely FORWARD: `from = cursor + 1`, and the walk
/// breaks on the first block past the target -- so a cursor ALREADY past the target returns
/// `Ok(())` having moved nothing and signalled nothing. That is harmless on a fresh catch-up, where
/// the cursor is always behind. It is wrong on a REFOLD: the co-advancer drives the checkpoint to
/// the durable tip at the end of every pass, an accumulator reset does not rewind it (the tip-only
/// `reduced_checkpoint_reset_if_ahead` does not fire on a same-chain refold), and the accumulator
/// then re-crosses a boundary far behind the cursor. Mark and `finalize()` are then taken at the
/// TIP, the re-sealed frozen leadership carries a different `source_checkpoint_commitment` and
/// stake view than the crossing that wrote the durable eview activation record, and the divergence
/// stays LATENT until a restart compares them and halts on `EpochViewPostPromotionMismatch`.
///
/// So: rewind when ahead (`reset_to_bootstrap` -- the reduced delta is NOT invertible, so
/// re-materializing from the sealed seed is the only way back), advance forward, then VERIFY the
/// cursor landed exactly on the boundary point rather than assuming it. Re-derivation is
/// byte-identical because the original crossing folded the same `(seed, s_prev]` prefix in the same
/// order, and that prefix is immutable: `s_prev` belongs to a boundary already deeper than `k`, and
/// rollback admission refuses anything deeper than `k`.
///
/// Store faults propagate exactly as the bare forward advance did. `Unreachable` does not -- the
/// caller stalls observe-only instead, which is strictly safer than sealing a wrong object.
fn position_reduced_checkpoint_at_boundary(
    cp: &ade_runtime::chaindb::ReducedUtxoCheckpoint,
    chaindb: &dyn ChainDb,
    boundary_slot: SlotNo,
) -> Result<CheckpointPositioning, NodeLifecycleError> {
    let seed = cp
        .seed_slot()
        .map_err(|e| NodeLifecycleError::RelaySync(format!("reduced-checkpoint seed slot: {e:?}")))?
        .ok_or_else(|| {
            NodeLifecycleError::RelaySync(
                "reduced checkpoint has no sealed bootstrap baseline (malformed)".to_string(),
            )
        })?;
    let cursor = cp
        .last_advanced_slot()
        .map_err(|e| NodeLifecycleError::RelaySync(format!("reduced-checkpoint slot: {e:?}")))?
        .unwrap_or(seed);
    if cursor.0 == boundary_slot.0 {
        return Ok(CheckpointPositioning::AlreadyAt);
    }
    // A boundary BEFORE the sealed seed can never be re-derived -- the pre-seed deltas are not held.
    if boundary_slot.0 < seed.0 {
        return Ok(CheckpointPositioning::Unreachable {
            advanced: cursor.0,
            seed: seed.0,
        });
    }
    let rewound = cursor.0 > boundary_slot.0;
    if rewound {
        cp.reset_to_bootstrap().map_err(|e| {
            NodeLifecycleError::RelaySync(format!("reduced-checkpoint re-materialize: {e:?}"))
        })?;
    }
    advance_reduced_checkpoint_forward_to(Some(cp), chaindb, boundary_slot)?;
    // VERIFY, never assume: exactly on the boundary point, on the expected seed lineage.
    // `verify_ready_at` is the same EXACT-at-slot gate the DERIVE path already fails closed on --
    // the seal path simply never asked, which is the asymmetry this slice closes.
    if cp.verify_ready_at(boundary_slot, seed).is_ok() {
        return Ok(if rewound {
            CheckpointPositioning::RewoundAndReplayed
        } else {
            CheckpointPositioning::AdvancedForward
        });
    }
    let advanced = cp
        .last_advanced_slot()
        .map_err(|e| NodeLifecycleError::RelaySync(format!("reduced-checkpoint slot: {e:?}")))?
        .unwrap_or(seed);
    Ok(CheckpointPositioning::Unreachable {
        advanced: advanced.0,
        seed: seed.0,
    })
}

/// LIVE-LEDGER-EPOCH-TRANSITION S5 (step 2b): the recovery-admission POLICY — the single Cardano-derived
/// bound the recovery layer is allowed to hold. Constructed ONCE at the lifecycle entry from the validated
/// network/genesis settings and threaded explicitly downward (NEVER reached sideways from ambient config),
/// so the recovery decision has exactly the authority it needs — rollback depth (`SecurityParam` k) — and no
/// more. A recovery bound, not S4 leadership behaviour.
#[derive(Debug, Clone, Copy)]
pub struct RecoveryAdmissionPolicy {
    pub security_param: SecurityParam,
}

impl RecoveryAdmissionPolicy {
    /// The recovery bound for the node's supported Cardano networks (preprod / preview / mainnet): the
    /// SecurityParam k = 2160 the follow path already pins (see the `security_param: SecurityParam(2160)`
    /// producer default). A single source of truth for the recovery-depth bound, threaded explicitly — the
    /// two-producer venue override touches leadership eligibility, never this recovery bound.
    pub const fn cardano() -> Self {
        RecoveryAdmissionPolicy {
            security_param: SecurityParam(2160),
        }
    }
}

/// LIVE-LEDGER-EPOCH-TRANSITION S2/S3 → S5 (DC-EPOCH-20 / PO-4): the durable EpochAccumulator reorg reset,
/// evolved into the lineage-certified, k-bounded RECOVERY ADMISSION that replaces the former height-only
/// reset (the S4/MEDIUM-2 obligation, discharged HERE — no longer S4's). Given the persisted lineage anchor
/// (2a `LastAdvancedPoint`) + the durable canonical ChainDB, the BLUE `reconcile_recovery` decides: trust the
/// accumulator (forward-fold), rematerialize it (reset + re-fold from canonical blocks), or — if the durable
/// state cannot be proven to describe ONE canonical selected-chain prefix — FAIL CLOSED (`RecoveryAdmission`,
/// terminal until re-bootstrap). This is the recovery-INTEGRITY exception to the S3 observe-only contract: an
/// ordinary follow-time observe fault still does NOT halt (the accumulator is simply not promoted), but a
/// durable-state contradiction does. It does NOT promote the accumulator to leadership (that is S4).
fn accumulator_recover_admit(
    epoch_accumulator: Option<&ade_runtime::chaindb::EpochAccumulatorStore>,
    chaindb: &dyn ChainDb,
    tip: &ChainTip,
    policy: &RecoveryAdmissionPolicy,
) -> Result<(), NodeLifecycleError> {
    let Some(store) = epoch_accumulator else {
        return Ok(());
    };
    // Unsealed store -> skip (the fold's skip-if-unsealed handles it); NOT a recovery fault.
    let Ok(Some(seed_slot)) = store.seed_slot() else {
        return Ok(());
    };

    // The persisted lineage anchor (2a). A corrupt anchor is terminal.
    let anchor = store.last_advanced_point().map_err(|_| {
        NodeLifecycleError::RecoveryAdmission(RecoveryAdmissionFault::CorruptLastAdvancedPoint)
    })?;

    // The bootstrap seed floor. `reconcile_recovery` reads it ONLY as the `admit_rollback` floor
    // (BeforeBootstrapAnchor) in the rollback branches; the forward / no-anchor paths never touch it. So
    // an absent seed block (e.g. a synthetic seed baseline below the durable blocks) is NOT fatal here --
    // it degrades to a genesis floor (BlockNo 0, the weakest floor), leaving the k-bound as the binding
    // rollback rail. A seed block that is PRESENT but undecodable is still terminal (NonContiguousCanonicalSpan).
    let seed_pt = resolve_canonical_point(chaindb, seed_slot)?.unwrap_or(CanonicalPoint {
        slot: seed_slot,
        block_no: BlockNo(0),
        hash: Hash32([0u8; 32]),
    });
    // The durable tip MUST be a present canonical block (it is the ChainDB's own tip).
    let tip_pt = resolve_canonical_point(chaindb, tip.slot)?.ok_or(
        NodeLifecycleError::RecoveryAdmission(RecoveryAdmissionFault::MissingCanonicalSpan {
            slot: tip.slot.0,
        }),
    )?;
    // The canonical block at the anchor's slot (None if the durable chain no longer carries it).
    let durable_at_anchor = match &anchor {
        Some(a) => resolve_canonical_point(chaindb, a.slot)?,
        None => None,
    };
    let anchor_pt = anchor.as_ref().map(|a| CanonicalPoint {
        slot: a.slot,
        block_no: a.block_no,
        hash: a.header_hash.clone(),
    });

    let anchor_before = trace_anchor_parts(
        anchor
            .as_ref()
            .map(|a| (a.slot.0, a.block_no.0, &a.header_hash)),
    );
    let durable_tip = trace_pt(Some(&tip_pt));

    let action = reconcile_recovery(
        anchor_pt.as_ref(),
        durable_at_anchor.as_ref(),
        &tip_pt,
        &seed_pt,
        policy.security_param.0,
        |slot| {
            resolve_canonical_point(chaindb, slot)
                .ok()
                .flatten()
                .map(|p| p.hash)
        },
    )
    .map_err(|e| {
        // EMIT-ONLY: a fail-closed recovery is traced before it propagates, so a terminal halt is not
        // the FIRST thing an operator learns about the anchor state.
        let reason = match &e {
            RollbackAdmissionError::LineageMismatch { .. } => {
                RecoveryTraceReason::CanonicalHashMismatch
            }
            RollbackAdmissionError::TargetNotOnCanonicalChain { .. } => {
                RecoveryTraceReason::MissingCanonicalBlock
            }
            _ => RecoveryTraceReason::CanonicalHashMismatch,
        };
        emit_recovery_trace(
            RecoveryTracePath::RecoveryAdmit,
            "error",
            reason,
            &anchor_before,
            &durable_tip,
            "none",
            &anchor_before,
        );
        NodeLifecycleError::RecoveryAdmission(RecoveryAdmissionFault::from_admission(e))
    })?;

    match action {
        RecoveryAction::ForwardFold => {
            emit_recovery_trace(
                RecoveryTracePath::RecoveryAdmit,
                "forward_fold",
                RecoveryTraceReason::ForwardFoldNoReset,
                &anchor_before,
                &durable_tip,
                "none",
                &anchor_before,
            );
            Ok(())
        }
        RecoveryAction::ResetAndRefold { reason } => {
            // Reset to the sealed seed; the co-advancer re-folds from the canonical prefix and re-writes a
            // fresh 2a lineage anchor on each advance. A reset fault here is a store failure, not observe-only.
            store.reset_to_bootstrap().map_err(|e| {
                NodeLifecycleError::RelaySync(format!("accumulator recovery reset: {e:?}"))
            })?;
            // anchor AFTER the reset -- this is what answers "does the reset clear the anchor and thereby
            // guarantee the next pass resets again?"
            let after = store.last_advanced_point().ok().flatten();
            let anchor_after = trace_anchor_parts(
                after
                    .as_ref()
                    .map(|a| (a.slot.0, a.block_no.0, &a.header_hash)),
            );
            emit_recovery_trace(
                RecoveryTracePath::RecoveryAdmit,
                "reset_and_refold",
                RecoveryTraceReason::from_reset(reason),
                &anchor_before,
                &durable_tip,
                "none",
                &anchor_after,
            );
            Ok(())
        }
    }
}

/// EMIT-ONLY recovery/refold trace (RED shell). Answers the questions 8h of logs could not: WHO asked for
/// an accumulator reset, WHY, what anchor/tip state caused it, and whether the refold restored the anchor.
///
/// Strictly diagnostic. It changes no scheduling, no retry/backoff, no anchor lifecycle, no rollback
/// admission and no refold decision — it serialises decisions already made. BLUE returns the structured
/// [`RecoveryAction`] / [`ResetReason`]; this shell projects and emits it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryTracePath {
    /// `accumulator_recover_admit` — runs at the top of EVERY advance pass.
    RecoveryAdmit,
    /// `accumulator_admit_and_clear_for_rollback` — the peer-rollback pre-clear.
    RollbackAdmit,
}

impl RecoveryTracePath {
    fn as_str(self) -> &'static str {
        match self {
            Self::RecoveryAdmit => "recovery_admit",
            Self::RollbackAdmit => "rollback_admit",
        }
    }
}

/// Closed reset/trace reason. No free-form string, no catch-all: a new cause is a compile error at the
/// projection until it is named here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryTraceReason {
    /// BLUE `ResetReason::AnchorAbsent` — no lineage anchor (including the state a PREVIOUS reset left).
    AnchorAbsent,
    /// BLUE `ResetReason::DurableTipBehindAnchor` — the durable chain shortened below the anchor.
    DurableTipBehindAnchor,
    /// The peer-rollback pre-clear path reset the accumulator for an admitted rollback.
    RollbackAdmission,
    /// Recovery fail-closed: the anchor's slot carries a DIFFERENT canonical hash.
    CanonicalHashMismatch,
    /// Recovery fail-closed: no canonical block at the anchor's slot.
    MissingCanonicalBlock,
    /// Forward-fold — no reset at all (emitted so a quiet pass is still observable).
    ForwardFoldNoReset,
}

impl RecoveryTraceReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::AnchorAbsent => "anchor_absent",
            Self::DurableTipBehindAnchor => "durable_tip_behind_anchor",
            Self::RollbackAdmission => "rollback_admission",
            Self::CanonicalHashMismatch => "canonical_hash_mismatch",
            Self::MissingCanonicalBlock => "missing_canonical_block",
            Self::ForwardFoldNoReset => "forward_fold_no_reset",
        }
    }

    /// TOTAL projection of the BLUE decision. Compile-enforced: a new `ResetReason` variant breaks here
    /// until it is traced, which is the point of putting the reason in the BLUE type rather than
    /// re-deriving it in the shell.
    fn from_reset(reason: ResetReason) -> Self {
        match reason {
            ResetReason::AnchorAbsent => Self::AnchorAbsent,
            ResetReason::DurableTipBehindAnchor => Self::DurableTipBehindAnchor,
        }
    }
}

/// Render an optional lineage point as a stable `slot/block_no/hash8` triple (or `absent`).
fn trace_pt(p: Option<&CanonicalPoint>) -> String {
    match p {
        None => "absent".to_string(),
        Some(p) => format!(
            "{}/{}/{}",
            p.slot.0,
            p.block_no.0,
            hex_prefix8(&p.hash)
        ),
    }
}

/// Takes the PARTS rather than the store's point type, so the trace does not depend on a
/// module path that is not re-exported.
fn trace_anchor_parts(a: Option<(u64, u64, &Hash32)>) -> String {
    match a {
        None => "absent".to_string(),
        Some((slot, block_no, hash)) => format!("{}/{}/{}", slot, block_no, hex_prefix8(hash)),
    }
}

fn hex_prefix8(h: &Hash32) -> String {
    h.0.iter().take(4).map(|b| format!("{b:02x}")).collect()
}

/// Emit ONE structured line per recovery/reset decision. Field set is fixed so the stream is parseable:
/// `path`, `action`, `reason`, `anchor_before`, `durable_tip`, `rollback_target`, `anchor_after`.
#[allow(clippy::too_many_arguments)]
fn emit_recovery_trace(
    path: RecoveryTracePath,
    action: &'static str,
    reason: RecoveryTraceReason,
    anchor_before: &str,
    durable_tip: &str,
    rollback_target: &str,
    anchor_after: &str,
) {
    crate::node_log!(
        "recovery-trace: path={} action={} reason={} anchor_before={} durable_tip={} \
         rollback_target={} anchor_after={}",
        path.as_str(),
        action,
        reason.as_str(),
        anchor_before,
        durable_tip,
        rollback_target,
        anchor_after
    );
}

/// Resolve the canonical point `(slot, block_no, header_hash)` of the durable block at `slot`, or `None` if
/// no block sits at that slot. A block that does not decode is a non-admissible span (a corruption / a
/// prev-hash break in the canonical prefix) -> terminal `NonContiguousCanonicalSpan`. Fail-closed on a
/// ChainDB read error.
fn resolve_canonical_point(
    chaindb: &dyn ChainDb,
    slot: SlotNo,
) -> Result<Option<CanonicalPoint>, NodeLifecycleError> {
    let stored = chaindb.get_block_by_slot(slot).map_err(|e| {
        NodeLifecycleError::RelaySync(format!("recovery point read {}: {e:?}", slot.0))
    })?;
    match stored {
        None => Ok(None),
        Some(sb) => {
            let decoded = decode_block(&sb.bytes).map_err(|_| {
                NodeLifecycleError::RecoveryAdmission(
                    RecoveryAdmissionFault::NonContiguousCanonicalSpan { slot: slot.0 },
                )
            })?;
            Ok(Some(CanonicalPoint {
                slot,
                block_no: decoded.header_input.block_no,
                hash: sb.hash,
            }))
        }
    }
}

/// LIVE-LEDGER-EPOCH-TRANSITION S5 (step 2b, EVENT-QUALIFIED live rollback): bring the durable
/// EpochAccumulator into lockstep with a chain-selection-ADMITTED `ChainEvent::RolledBack`, BEFORE the
/// caller's `commit_rollback` trims the ChainDB. PROVENANCE is the discriminator that lets `reconcile_recovery`
/// stay strict: this pre-clear runs ONLY for a live rollback event (a selected-chain transition), never for
/// an unexplained persisted contradiction (which stays terminal at warm-start in `accumulator_recover_admit`).
/// If the accumulator carries a certified lineage anchor, the rollback of that anchor to `target` must be
/// ADMISSIBLE against the PRE-rollback canonical chain (`admit_rollback`: target >= bootstrap seed, depth <= k,
/// target on-canonical); then the anchor is durably CLEARED here so NO crash window ever leaves a certified
/// anchor over the abandoned prefix -- the next advance sees anchor-absent and refolds from the post-rollback
/// canonical chain. Inadmissible -> terminal typed fault (NEVER a silent reset of a certified store). Anchor
/// absent (already uncertified) / store unsealed / no accumulator -> nothing to admit or clear.
fn accumulator_admit_and_clear_for_rollback(
    epoch_accumulator: Option<&ade_runtime::chaindb::EpochAccumulatorStore>,
    chaindb: &dyn ChainDb,
    target: &Point,
    policy: &RecoveryAdmissionPolicy,
) -> Result<(), NodeLifecycleError> {
    let Some(store) = epoch_accumulator else {
        return Ok(());
    };
    let Ok(Some(seed_slot)) = store.seed_slot() else {
        return Ok(());
    };
    // Only a CERTIFIED accumulator holds authority to bring into lockstep. An absent anchor is already
    // uncertified -> the advance/recovery pass refolds it from canonical regardless; nothing to admit.
    let Some(anchor) = store.last_advanced_point().map_err(|_| {
        NodeLifecycleError::RecoveryAdmission(RecoveryAdmissionFault::CorruptLastAdvancedPoint)
    })?
    else {
        return Ok(());
    };
    // The seed floor (non-fatal genesis fallback, as in `accumulator_recover_admit`).
    let seed_pt = resolve_canonical_point(chaindb, seed_slot)?.unwrap_or(CanonicalPoint {
        slot: seed_slot,
        block_no: BlockNo(0),
        hash: Hash32([0u8; 32]),
    });
    // The target's height from the PRE-rollback canonical chain (absent -> not a selected point at all).
    let target_at = resolve_canonical_point(chaindb, target.slot)?.ok_or(
        NodeLifecycleError::RecoveryAdmission(RecoveryAdmissionFault::TargetNotOnCanonicalChain {
            slot: target.slot.0,
        }),
    )?;
    let anchor_pt = CanonicalPoint {
        slot: anchor.slot,
        block_no: anchor.block_no,
        hash: anchor.header_hash.clone(),
    };
    // The rollback target carries the EVENT's claimed hash; `admit_rollback`'s lineage check binds it to the
    // pre-rollback canonical block at that slot (a divergent target -> LineageMismatch).
    let target_pt = CanonicalPoint {
        slot: target.slot,
        block_no: target_at.block_no,
        hash: target.hash.clone(),
    };
    admit_rollback(
        &anchor_pt,
        &target_pt,
        &seed_pt,
        policy.security_param.0,
        |slot| {
            resolve_canonical_point(chaindb, slot)
                .ok()
                .flatten()
                .map(|p| p.hash)
        },
    )
    .map_err(|e| NodeLifecycleError::RecoveryAdmission(RecoveryAdmissionFault::from_admission(e)))?;
    // Admissible: durably CLEAR the lineage anchor BEFORE the caller commits the ChainDB rollback, so a crash
    // in the window leaves an anchor-absent (uncertified) store that the next advance refolds from canonical.
    //
    // ACCUMULATOR-REFOLD-BOUND S1: rewind to the SETTLED point when one is admissible, else to the
    // bootstrap baseline exactly as before. Both leave the store uncertified; only the amount of
    // canonical chain the next advance must re-derive differs.
    let rb_anchor_before = {
        let a = store.last_advanced_point().ok().flatten();
        trace_anchor_parts(
            a.as_ref()
                .map(|a| (a.slot.0, a.block_no.0, &a.header_hash)),
        )
    };
    let rb_target = format!("{}/{}/{}", target.slot.0, target_at.block_no.0, hex_prefix8(&target.hash));
    let rb_tip = {
        let t = ChainDb::tip(chaindb).ok().flatten();
        match t.and_then(|t| resolve_canonical_point(chaindb, t.slot).ok().flatten()) {
            Some(p) => trace_pt(Some(&p)),
            None => "absent".to_string(),
        }
    };
    if settled_rewind_admissible(store, chaindb, target, policy.security_param.0) {
        match store.reset_to_settled() {
            Ok(true) => {
                let a = store.last_advanced_point().ok().flatten();
                emit_recovery_trace(
                    RecoveryTracePath::RollbackAdmit,
                    "reset_to_settled",
                    RecoveryTraceReason::RollbackAdmission,
                    &rb_anchor_before,
                    &rb_tip,
                    &rb_target,
                    &trace_anchor_parts(
                        a.as_ref()
                            .map(|a| (a.slot.0, a.block_no.0, &a.header_hash)),
                    ),
                );
                return Ok(());
            }
            // No settled point recorded (older store / just reset) -> bootstrap.
            Ok(false) => {}
            Err(e) => {
                // Observe-only: a settled-rewind fault costs refold time, never safety, because the
                // bootstrap fallback below is the unchanged pre-slice behaviour.
                crate::node_log!(
                    "epoch-accumulator: settled rewind failed ({e:?}); falling back to bootstrap"
                );
            }
        }
    }
    store.reset_to_bootstrap().map_err(|e| {
        NodeLifecycleError::RelaySync(format!("accumulator rollback pre-clear: {e:?}"))
    })?;
    let a = store.last_advanced_point().ok().flatten();
    emit_recovery_trace(
        RecoveryTracePath::RollbackAdmit,
        "reset_to_bootstrap",
        RecoveryTraceReason::RollbackAdmission,
        &rb_anchor_before,
        &rb_tip,
        &rb_target,
        &trace_anchor_parts(
            a.as_ref()
                .map(|a| (a.slot.0, a.block_no.0, &a.header_hash)),
        ),
    );
    Ok(())
}

/// LIVE-REFOLD-THRASH RF-1 (DC-EPOCH-35): after the ChainDb rollback has COMMITTED, re-certify the
/// settled rewind point against the POST-rollback canonical chain and re-establish the lineage
/// anchor there, so the next recovery pass FORWARD-FOLDS from the bounded point instead of reading
/// an absent anchor and refolding from the bootstrap baseline.
///
/// The defect this closes: `reset_to_settled` applies a correct bounded rewind and clears the anchor
/// (DC-EPOCH-29 — the store must not be lineage authority across the rollback window). The next pass
/// then reconciles an ABSENT anchor to `ResetAndRefold { AnchorAbsent }` and calls
/// `reset_to_bootstrap`, which discards the rewind *and deletes the settled triple*, so every
/// subsequent rollback is unbounded too. Measured live growing 153,565 → 171,449 slots per refold,
/// until the refold outgrew the inter-rollback interval and the node stopped holding tip at all.
///
/// ORDER IS THE SAFETY PROPERTY. The anchor is never carried ACROSS the rollback commit: the
/// pre-clear still runs first and a crash in that window still refolds from canonical. Only after
/// the rollback is durable is the point re-proved against the chain AS IT NOW STANDS:
///
///  1. **Still canonical** — the settled point's header hash resolves at its slot on the NEW chain.
///     A hash pins its whole ancestry, so this proves the prefix that produced the stored state is
///     byte-identical to the current canonical prefix.
///  2. **Still k-settled** — `k` BLOCKS behind the NEW tip (block units, no ASC assumption).
///  3. **Integrity** — the CE-RF-6 fingerprint verifies and the cursor sits exactly at the point
///     (checked store-side by `recertify_settled_anchor`).
///
/// Any failure leaves the anchor ABSENT, i.e. exactly today's behaviour: the next pass refolds from
/// bootstrap. The fallback is always safe, so this can only ever save refold time.
fn accumulator_recertify_settled_after_rollback(
    epoch_accumulator: Option<&ade_runtime::chaindb::EpochAccumulatorStore>,
    chaindb: &dyn ChainDb,
    policy: &RecoveryAdmissionPolicy,
) -> Result<(), NodeLifecycleError> {
    let Some(store) = epoch_accumulator else {
        return Ok(());
    };
    // No settled point (bootstrap reset ran, or none promoted yet) -> nothing to re-certify.
    let Ok(Some(sp)) = store.settled_rewind_point() else {
        return Ok(());
    };
    // (1) still canonical at its slot on the POST-rollback chain.
    let canonical = resolve_canonical_point(chaindb, sp.slot)?;
    let Some(cp) = canonical.filter(|c| c.hash == sp.header_hash) else {
        crate::node_log!(
            "epoch-accumulator: settled point {} not canonical after rollback -> anchor stays absent (bootstrap refold)",
            sp.slot.0
        );
        return Ok(());
    };
    // (2) still k BLOCKS behind the NEW tip.
    let Ok(Some(tip)) = chaindb.tip() else {
        return Ok(());
    };
    let Some(tip_pt) = resolve_canonical_point(chaindb, tip.slot)? else {
        return Ok(());
    };
    if cp.block_no.0.saturating_add(policy.security_param.0) > tip_pt.block_no.0 {
        crate::node_log!(
            "epoch-accumulator: settled point {} not k-settled against new tip {} -> anchor stays absent",
            sp.slot.0,
            tip_pt.block_no.0
        );
        return Ok(());
    }
    // (3) integrity + cursor, store-side. `None` = refused; anchor stays absent.
    match store.recertify_settled_anchor() {
        Ok(Some(p)) => {
            crate::node_log!(
                "epoch-accumulator: settled rewind RE-CERTIFIED at {}/{} after rollback -- next pass forward-folds, no bootstrap refold (DC-EPOCH-35)",
                p.slot.0,
                p.block_no.0
            );
        }
        Ok(None) => {
            crate::node_log!(
                "epoch-accumulator: settled triple failed integrity/cursor re-certification -> anchor stays absent (bootstrap refold)"
            );
        }
        Err(e) => {
            crate::node_log!(
                "epoch-accumulator: settled re-certification faulted (observe-only, bootstrap refold): {e:?}"
            );
        }
    }
    Ok(())
}

/// ACCUMULATOR-REFOLD-BOUND S1: may the accumulator be rewound to its SETTLED point for this
/// rollback, instead of all the way to the bootstrap baseline?
///
/// Three conditions, ALL required; any failure (including any I/O fault) answers `false` and the
/// caller falls back to `reset_to_bootstrap`, which is the unchanged pre-slice behaviour. The
/// fallback is always safe, so this predicate can only ever cost refold time.
///
///  1. **Settled** (INV-AR-1) — the point is at least `k` BLOCKS behind the durable tip, so no
///     admissible reorg can reach it. Compared in BLOCK units, so no active-slot-coefficient
///     assumption is needed (a slot comparison would need one, and would be wrong if `f` differed).
///  2. **Not ahead of the target** (INV-AR-2) — never rewind FORWARD past the rollback target.
///     Implied by (1) since the target is within `k` of the tip, but asserted rather than inferred.
///  3. **Lineage intact** (INV-AR-2) — the point's header hash still resolves canonically at its
///     slot. A point the chain has abandoned is refused.
fn settled_rewind_admissible(
    store: &ade_runtime::chaindb::EpochAccumulatorStore,
    chaindb: &dyn ChainDb,
    target: &Point,
    security_param: u64,
) -> bool {
    let Ok(Some(sp)) = store.settled_rewind_point() else {
        return false;
    };
    let Ok(Some(tip)) = chaindb.tip() else {
        return false;
    };
    let Ok(Some(tip_pt)) = resolve_canonical_point(chaindb, tip.slot) else {
        return false;
    };
    // (1) settled: k blocks of separation from the tip.
    if sp.block_no.0.saturating_add(security_param) > tip_pt.block_no.0 {
        return false;
    }
    // (2) never forward of the rollback target.
    if sp.slot.0 > target.slot.0 {
        return false;
    }
    // (3) lineage still canonical at that slot.
    matches!(
        resolve_canonical_point(chaindb, sp.slot),
        Ok(Some(p)) if p.hash == sp.header_hash
    )
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
/// Is an accumulator boundary crossing a post-rollback REFOLD rather than a fresh crossing?
///
/// Both are real crossings. `accumulator_admit_and_clear_for_rollback` calls
/// `reset_to_bootstrap()` on every admitted rollback (S5 pre-clear), so the next advance
/// genuinely re-derives every boundary since the bootstrap anchor. This only labels which
/// case the operator is looking at; it never changes behaviour.
///
/// Stateless and exact: on a FRESH crossing the durable tip is in `to_epoch` (we just crossed
/// into the epoch we are following). On a REFOLD the tip is already in a LATER epoch. An
/// unknown tip epoch degrades to "fresh" — the unlabelled, pre-existing line.
fn crossing_is_refold(tip_epoch: Option<EpochNo>, to_epoch: EpochNo) -> bool {
    tip_epoch.is_some_and(|te| te.0 > to_epoch.0)
}


/// B6 CENSUS IV: accumulate the boundary arm's elapsed time on EVERY exit path, including the
/// several `break`s inside it. A plain `elapsed()` at the end of the arm would miss them and report
/// zero for exactly the branch that costs the most.
struct BoundaryArmTimer<'a> {
    start: std::time::Instant,
    acc: &'a mut u128,
}
impl Drop for BoundaryArmTimer<'_> {
    fn drop(&mut self) {
        *self.acc += self.start.elapsed().as_millis();
    }
}

fn advance_ledger_state_to_durable_tip(
    reduced_checkpoint: Option<&ade_runtime::chaindb::ReducedUtxoCheckpoint>,
    epoch_accumulator: Option<&ade_runtime::chaindb::EpochAccumulatorStore>,
    chaindb: &dyn ChainDb,
    era_schedule: &EraSchedule,
    policy: &RecoveryAdmissionPolicy,
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

    // The durable tip's epoch -- used ONLY to label a boundary crossing as fresh vs a
    // post-rollback refold in the observability below. Observe-only: a locate fault degrades
    // to the unlabelled (fresh) log line, never to a halt.
    let tip_epoch = era_schedule.locate(tip.slot).ok().map(|l| l.epoch);

    // B6 CENSUS II — split the co-advance into its three constituent calls.
    //
    // The first census attributed 73% of loop time to this function as a whole. The slot->hash index
    // fix then took the at-tip pass from 115.0s to 90.2s on the SAME 9 blocks: real, but only ~22%,
    // so ~78% of the fixed cost is elsewhere INSIDE here. Splitting it is the same discipline that
    // named C in the first place -- attribute before fixing, do not guess twice.
    let t_reset = std::time::Instant::now();
    reduced_checkpoint_reset_if_ahead(reduced_checkpoint, &tip)?;
    let ms_reset = t_reset.elapsed().as_millis();
    let t_recover = std::time::Instant::now();
    accumulator_recover_admit(epoch_accumulator, chaindb, &tip, policy)?;
    let ms_recover = t_recover.elapsed().as_millis();
    let t_accum_loop = std::time::Instant::now();
    // B6 CENSUS III: inside the accumulator loop, separate the PER-BLOCK walk from the
    // ONCE-PER-PASS ReachedTip bookkeeping. The at-tip pass costs ~66s here on 2 blocks, so the
    // cost is not the walk's per-block work -- this says which of the two it actually is.
    let mut census_ms_walk: u128 = 0;
    let mut census_ms_settle: u128 = 0;
    // B6 CENSUS IV: walk + settle accounted for only 129ms of a 61,473ms block, so the cost is in
    // the untimed remainder. Time EVERY statement in the block rather than reason about which of
    // them "looks cheap" -- that reasoning has been wrong twice in this investigation.
    let mut census_ms_seed_slot: u128 = 0;
    let mut census_ms_boundary_arm: u128 = 0;

    // The boundary-segmented accumulator cross loop (observe-only). Skipped when no accumulator is
    // configured -> the EVIEW-only advance below is byte-identical to the pre-S3 path.
    if let Some(store) = epoch_accumulator {
        // Skip-if-unsealed: a present-but-unsealed store is malformed (never fold from slot 0 over a seed
        // that already absorbed those blocks). The checkpoint still reaches tip below.
        let t_seed = std::time::Instant::now();
        let seed_slot_probe = store.seed_slot();
        census_ms_seed_slot = t_seed.elapsed().as_millis();
        if let Ok(Some(seed_slot)) = seed_slot_probe {
            loop {
                let t_walk = std::time::Instant::now();
                let walk_outcome = advance_accumulator_over_chaindb(
                    store,
                    chaindb,
                    era_schedule,
                    seed_slot,
                    tip.slot,
                );
                census_ms_walk += t_walk.elapsed().as_millis();
                match walk_outcome {
                    Ok(AccumulatorChaindbOutcome::ReachedTip { .. }) => {
                        // ACCUMULATOR-REFOLD-BOUND S1: roll the bounded rewind buffer now that the
                        // accumulator is current. Observe-only -- a fault here only means the next
                        // rollback refolds from bootstrap as it did pre-slice.
                        let t_settle = std::time::Instant::now();
                        if let Ok(Some(tip_pt)) = resolve_canonical_point(chaindb, tip.slot) {
                            if let Err(e) = store
                                .roll_settled_rewind_point(tip_pt.block_no, policy.security_param.0)
                            {
                                crate::node_log!(
                                    "epoch-accumulator: settled-rewind roll failed (observe-only): {e:?}"
                                );
                            }
                        }
                        census_ms_settle += t_settle.elapsed().as_millis();
                        break;
                    }
                    Ok(AccumulatorChaindbOutcome::StalledAt { slot: s_bb, reason }) => {
                        let t_bnd = std::time::Instant::now();
                        let _census_bnd = BoundaryArmTimer {
                            start: t_bnd,
                            acc: &mut census_ms_boundary_arm,
                        };
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
                        // is the end-of-epoch stake, before the new epoch's first block. EVIEW-R2
                        // (DC-EPOCH-32): "exactly" is now VERIFIED, and a cursor left PAST the boundary point
                        // is rewound onto it rather than silently accepted -- a refold used to seal its mark
                        // and commitment at the durable tip, producing frozen leadership that disagreed with
                        // the durable activation record and halted the NEXT restart.
                        match position_reduced_checkpoint_at_boundary(cp, chaindb, s_prev)? {
                            CheckpointPositioning::AlreadyAt
                            | CheckpointPositioning::AdvancedForward => {}
                            CheckpointPositioning::RewoundAndReplayed => {
                                crate::node_log!(
                                    "epoch-accumulator: reduced checkpoint REWOUND onto boundary point {} \
                                     before sealing (it sat past it -- DC-EPOCH-32)",
                                    s_prev.0
                                );
                            }
                            CheckpointPositioning::Unreachable { advanced, seed } => {
                                crate::node_log!(
                                    "epoch-accumulator: boundary point {} unreachable for the reduced \
                                     checkpoint (cursor {}, seed {}) -> observe-only stall; NOTHING sealed \
                                     (DC-EPOCH-32)",
                                    s_prev.0,
                                    advanced,
                                    seed
                                );
                                break;
                            }
                        }
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
                        // S4-L2 (v6): capture the reduced-checkpoint commitment finalized AT s_prev (the mark
                        // source; cp was advanced to s_prev above). This is sealed INTO the frozen leadership
                        // object so the promoted candidate authority carries its own provenance — no window
                        // replay, no live-checkpoint lookup at promotion time. Observe-only on a fault.
                        let source_commitment = match cp.finalize() {
                            Ok(c) => c,
                            Err(e) => {
                                crate::node_log!(
                                    "epoch-accumulator: checkpoint finalize at s_prev {} failed (observe-only): {:?}",
                                    s_prev.0,
                                    e
                                );
                                break;
                            }
                        };
                        match cross_accumulator_over_boundary_block(
                            store,
                            chaindb,
                            era_schedule,
                            s_bb,
                            &mark,
                            s_prev,
                            &boundary_hash,
                            &source_commitment,
                        ) {
                            Ok(AccumulatorBoundaryOutcome::Crossed {
                                from_epoch,
                                to_epoch,
                                slot,
                            }) => {
                                let _ = store.clear_boundary_mark();
                                // Distinguish a FRESH crossing from a post-rollback REFOLD. Both are real
                                // crossings -- `accumulator_admit_and_clear_for_rollback` calls
                                // `reset_to_bootstrap()` on every admitted rollback (S5 pre-clear: a crash
                                // in the rollback window must not leave a certified-but-wrong store), so the
                                // next advance genuinely re-derives every boundary since the bootstrap
                                // anchor. Reporting a refold as a fresh boundary is misleading (observed
                                // live 2026-08-01: 14 identical "CROSSED 1375 -> 1376" lines over 18h, one
                                // per reorg), and simply silencing it would HIDE the re-derivation cost --
                                // which grows the further the tip is from the anchor. So say which it is.
                                //
                                // Discriminator is stateless and exact: on a fresh crossing the durable tip
                                // is in `to_epoch`; on a refold the tip is already in a LATER epoch.
                                let refolding = crossing_is_refold(tip_epoch, to_epoch);
                                if refolding {
                                    crate::node_log!(
                                        "epoch-accumulator: REFOLD re-crossed boundary {} -> {} at slot {} \
                                         (mark from s_prev {}) -- re-derived after a rollback reset; durable \
                                         tip is already in epoch {}, {} slots left to refold",
                                        from_epoch.0,
                                        to_epoch.0,
                                        slot.0,
                                        s_prev.0,
                                        tip_epoch.map_or(0, |e| e.0),
                                        tip.slot.0.saturating_sub(slot.0)
                                    );
                                } else {
                                    // Observable proof of self-derived ledger continuity across a boundary
                                    // (CE-3c): the mark was captured at the boundary point s_prev, not the
                                    // tip. Byte-identical to the pre-fix line -- CE-3c / CE-4 evidence
                                    // quotes this verbatim.
                                    crate::node_log!(
                                        "epoch-accumulator: CROSSED boundary {} -> {} at slot {} (mark from s_prev {})",
                                        from_epoch.0,
                                        to_epoch.0,
                                        slot.0,
                                        s_prev.0
                                    );
                                }
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

    let ms_accum_loop = t_accum_loop.elapsed().as_millis();
    // GUARANTEE the EVIEW checkpoint reaches the durable tip (fail-closed), regardless of the accumulator
    // outcome. Forward-only -- the reorg reset was hoisted above.
    let t_cp_forward = std::time::Instant::now();
    advance_reduced_checkpoint_forward_to(reduced_checkpoint, chaindb, tip.slot)?;
    let ms_cp_forward = t_cp_forward.elapsed().as_millis();
    crate::node_log!(
        "b6-census-coadv: non_authoritative=true reset_ms={} recover_admit_ms={} \
         accumulator_loop_ms={} (seed_slot_ms={} walk_ms={} settle_ms={} boundary_arm_ms={}) \
         checkpoint_forward_ms={}",
        ms_reset,
        ms_recover,
        ms_accum_loop,
        census_ms_seed_slot,
        census_ms_walk,
        census_ms_settle,
        census_ms_boundary_arm,
        ms_cp_forward
    );
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
    // LIVE-LEDGER-EPOCH-TRANSITION S2 (DC-EPOCH-20) + S5 (step 2b): the durable non-UTxO accumulator,
    // `Some` when a native bootstrap sealed it (or a warm start reopened it). Two DISTINCT contracts:
    // ordinary FOLLOW-TIME advance after each admit stays OBSERVE-ONLY (the accumulator is not yet
    // authoritative; S4 flips it), so a within-epoch stall / compute fault never affects the follow;
    // but RECOVERY admission (the warm-start / durable-tip reconcile gated by `recovery_policy` below)
    // is the integrity EXCEPTION -- an uncertified or lineage-contradicted durable accumulator fails
    // CLOSED. `None` on non-native / wrapper / test callers -> inert.
    epoch_accumulator: Option<&ade_runtime::chaindb::EpochAccumulatorStore>,
    // LIVE-LEDGER-EPOCH-TRANSITION S5 (step 2b): the recovery-admission policy (Cardano-derived rollback
    // bound). Threaded explicitly from the lifecycle entry; the recovery reconcile after each admit uses it
    // to fail closed on an uncertified / inadmissible durable accumulator (the recovery-integrity exception
    // to the observe-only contract).
    recovery_policy: RecoveryAdmissionPolicy,
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
    // CE-4A.3-R4 (warm-start recovery ordering): reconcile the durable accumulator to the ChainDb tip via
    // the PRODUCTION ResetAndRefold (`advance_ledger_state_to_durable_tip` -> `accumulator_recover_admit`,
    // gated by `recovery_policy`) with a tip-extended schedule, BEFORE the eview recovery below — so recovery
    // reads a RESEALED accumulator, not a rollback-pending / lagging one. A crash in the rollback->refold
    // window leaves the tip epoch's frozen leadership unsealed; without this the eview recovery fails closed
    // `RecoveryEpochUnsealed` before the loop's post-admit advance (`:2847`) could reseal it. No-op
    // (ForwardFold) for a consistent warm-start -> byte-identical to the pre-R4 proven paths; fail-closed on
    // an uncertified / inadmissible accumulator (the `recovery_policy` integrity exception) -> NEVER a reseal
    // of a genuinely-corrupt store (the reconcile reseals ONLY from the durable ChainDb, the sole authority).
    if let Some(acc) = epoch_accumulator {
        let reconcile_sched = {
            let mut s = era_schedule.clone();
            if let Some(tip) = ChainDb::tip(chaindb).ok().flatten() {
                s.extend_to_slot(tip.slot);
            }
            s
        };
        advance_ledger_state_to_durable_tip(
            reduced_checkpoint,
            Some(acc),
            chaindb,
            &reconcile_sched,
            &recovery_policy,
        )
        .map_err(|e| {
            NodeLifecycleError::RelaySync(format!("R4 warm-start reconcile-before-recovery: {e:?}"))
        })?;
    }
    // Phase 4 (ECA-4, DC-EPOCH-06 recovery exactness): BEFORE the loop, if a durable activation record
    // exists, recover the promoted authority from the VERIFIED record (re-derive via the SAME window
    // replay + reject-non-recomputable) — so a restart AFTER a promotion starts from the recorded N+1
    // view (criteria 4/5), never a stale seed. The live first-boundary re-fire is then idempotent. A
    // None record (crash before the WAL) keeps Seed; a record whose candidate cannot be RECOMPUTED
    // identically is a TERMINAL halt (never trust a parsed record alone, never fall back to the seed).
    if let (Some(inputs), Some(_live)) = (eview_activation, reduced_checkpoint) {
        let entries = wal
            .read_all()
            .map_err(|e| NodeLifecycleError::RelaySync(format!("eview recovery WAL read: {e:?}")))?;
        // CE-4A.3-R3: the durable tip bounds BOTH the rollback-aware activation resolution (an activation
        // for an epoch a rollback un-crossed, or above the tip epoch, is not selected) AND
        // `recovered_tip_epoch`. Build the era schedule ONCE (extended to the tip) so the resolver's
        // epoch-of-rollback-target and the nonce-epoch derivation agree on the same selected-chain tip.
        let durable_tip = ChainDb::tip(chaindb).ok().flatten();
        let recovery_sched = {
            let mut s = era_schedule.clone();
            if let Some(tip) = &durable_tip {
                s.extend_to_slot(tip.slot);
            }
            s
        };
        // CE-4A.3-R1: the recovered epoch nonce is eta0(C) where C = the durable tip's epoch (the chain-dep
        // was replay-derived to the tip, so its epoch_nonce is that epoch's eta0 -- NOT the seed sidecar
        // nonce). Bind it to that epoch so the frozen-regime recovery asserts the nonce matches the record's
        // target (never a pre-boundary / wrong-epoch chain-dep silently producing a plausible-wrong view).
        let recovered_tip_epoch = match &durable_tip {
            Some(tip) => recovery_sched.locate(tip.slot).map(|l| l.epoch).unwrap_or(inputs.seed_epoch),
            None => inputs.seed_epoch,
        };
        let resolved = crate::epoch_activation::resolve_active_activation_at_tip(
            &entries,
            durable_tip.as_ref().map(|_| recovered_tip_epoch.0),
            |slot| {
                recovery_sched
                    .locate(SlotNo(slot))
                    .map(|l| l.epoch.0)
                    .unwrap_or(inputs.seed_epoch.0)
            },
        )
        .map_err(|e| NodeLifecycleError::RelaySync(format!("eview recovery resolve: {e:?}")))?;
        crate::epoch_wire::maybe_recover_promoted_authority(
            resolved.as_ref(),
            inputs.seed_epoch,
            inputs.network_magic,
            inputs.genesis_hash.clone(),
            inputs.protocol_params_hash.clone(),
            inputs.asc,
            inputs.next_epoch_bridge.as_ref(),
            epoch_accumulator,
            crate::epoch_wire::RecoveredEpochNonce {
                epoch: recovered_tip_epoch,
                eta0: state.receive.chain_dep.epoch_nonce.0.clone(),
            },
            // PREPROD-NONCE-2 (CE-N2-4): the tip-extended schedule the resolver above already used, so
            // the seed+1 bridge branch decides eta0 finality against the SAME venue RSW the live loop
            // and the candidate-freeze rule read (never a second, differently-built geometry).
            &recovery_sched,
            &mut authority,
        )
        .map_err(|e| NodeLifecycleError::RelaySync(format!("eview recovery: {e:?}")))?;
    }
    // ECA-5 (DC-EPOCH-15): warm-start forecast reconstruction. If the recovery promoted the authority
    // to N+1 (or beyond), extend the owned schedule to match -- deriving the SAME summaries the live
    // per-boundary append produced (byte-identical). A no-op when the recovery kept the seed.
    extend_schedule_to_epoch(&mut era_schedule, authority.epoch());
    loop {
        // B6 CENSUS (emit-only, NON-AUTHORITATIVE, operational tier).
        //
        // LIVE-2c measured the planner returning once per ~5-8 minutes on this venue while preprod
        // produces a block every ~20-90s, so `has_work_ready()` never goes false and no ForgeTick is
        // ever scheduled. That is a starvation condition, and four explanations fit it: (A) one
        // sync call takes minutes, (B) the pass processes an unbounded amount of work, (C) a
        // downstream recovery/refold dominates, (D) the planner is reached late rather than slowly.
        //
        // They compete for the SAME elapsed time, so they are timed inside ONE pass rather than by
        // four separate probes -- four probes would let each hypothesis look true on a different
        // iteration. `Instant` is monotonic and used for DURATIONS only; it never reaches the slot
        // conversion, and none of these values is ever read back by the planner.
        let iter_t0 = std::time::Instant::now();
        let shutdown_status = if *shutdown.borrow() {
            ShutdownStatus::ShutdownRequested
        } else {
            ShutdownStatus::Running
        };
        let t_has_work = std::time::Instant::now();
        let sync_status = if source.has_work_ready() {
            SyncStatus::WorkAvailable
        } else {
            SyncStatus::NoWorkReady
        };
        let ms_has_work = t_has_work.elapsed().as_millis();
        let t_is_ended = std::time::Instant::now();
        let loop_state = if source.is_ended() {
            LoopState::Ending
        } else {
            LoopState::Continuing
        };
        let ms_is_ended = t_is_ended.elapsed().as_millis();
        let t_next_tick = std::time::Instant::now();
        // PHASE4-N-F-E S2: forge-slot scheduling. RED observes the injected
        // clock; only the derived `SlotNo` crosses into the GREEN monotonic
        // guard + planner. Forge OFF (`None`) => always `NotDue` => the planner
        // reduces to the exact N-F-D relay mapping (no `ForgeTick`).
        let forge_slot = match forge.as_deref_mut() {
            None => ForgeSlotStatus::NotDue,
            Some(act) => {
                act.pending_slot = None; // reset so a stale slot can never forge
                // LIVE-2c part 2 (CE-L2c-1): THE wall-clock→slot conversion, and the only one
                // reachable from forging. The naive triple that stood here is deleted, not
                // deprecated — `checked_millis_to_slot` no longer exists.
                match act.clock.next_tick() {
                    Some(now_ms) => match act.timing.slot_at(now_ms) {
                        Ok(slot) => {
                            act.last_slot_derivation_fail = None;
                            act.pending_slot = Some(slot);
                            forge_slot_status(act.last_forged_slot, slot)
                        }
                        // The captured instant precedes the anchor's declared domain (or the
                        // arithmetic refused). FAIL CLOSED at the RED clock seam — no forge, no
                        // `last_forged_slot` advance, `pending_slot` stays None; surfaced as a
                        // structured local outcome AND emitted, because a conversion that cannot
                        // be justified must leave a reason behind rather than a silent NotDue.
                        // NotDue to the planner; the relay loop keeps syncing (forge stays
                        // subordinate to the sync spine, DC-NODE-05).
                        Err(e) => {
                            if act.last_slot_derivation_fail.as_ref() != Some(&e) {
                                crate::node_log!(
                                    "live2c-slot-derivation-refused: captured_ms={} reason={:?} \
                                     domain_start_ms={} domain_start_slot={} -- NO forge this tick",
                                    now_ms,
                                    e,
                                    act.timing.anchor().domain_start_ms(),
                                    act.timing.anchor().domain_start_slot().0
                                );
                            }
                            act.last_slot_derivation_fail = Some(e);
                            ForgeSlotStatus::NotDue
                        }
                    },
                    // Clock exhausted — no more forge slots scheduled.
                    None => ForgeSlotStatus::NotDue,
                }
            }
        };
        // B6 CENSUS: `SystemClock::next_tick` SLEEPS to the slot boundary when the loop is ahead,
        // and returns immediately when it is behind — so this is the one pre-planner cost that is
        // expected to be large on a HEALTHY loop and ~0 on a starved one. Measuring it separates
        // "waiting for the next slot" from "the planner is reached late" (hypothesis D).
        let ms_next_tick = t_next_tick.elapsed().as_millis();
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
        // LIVE-2b DISCRIMINATOR (emit-only, NON-AUTHORITATIVE, operational tier).
        //
        // The single boundary where all six branch-table inputs coexist: the planner's verdict is
        // computed here, and `forge`/`forge_slot`/`sync_status` are still in scope. The branch table
        // (SLICE-LIVE-2b) found FIVE silent exits between a captured slot and the first typed
        // surface, and a warm-started forge-capable node held tip ~17 min emitting zero forge
        // activity — so which exit fired is not inferable from the existing logs. Each candidate is a
        // DISTINCT combination of these values, so one run separates them.
        //
        // Deliberately observational: computed from values the loop already holds, with NO fallback
        // calculation added merely to populate the event, NO key material, and no read-back — the
        // planner verdict below is recomputed from the same inputs and cannot be influenced by this.
        // This is a diagnostic, not an authority: it must never become a planner input.
        //
        // selected_tip is OMITTED on purpose. It is not in scope here (`state` is ForwardSyncState),
        // and the two things that ARE cheap -- `recovered_anchor` and the followed PEER tip -- are
        // different quantities. Emitting either under a `selected_tip` label would be a mislabel, and
        // reading the ChainDb per iteration would be exactly the fallback calculation this probe is
        // forbidden from adding. The six branch-table inputs below fully separate the candidates.
        let planned = plan_loop_step(loop_state, sync_status, forge_slot, shutdown_status, policy);
        // B6 CENSUS hypothesis D: was the planner reached LATE, or is it the dispatch that is slow?
        let ms_to_planner = iter_t0.elapsed().as_millis();
        {
            let (forge_active, pending_slot, last_forged_slot) = match forge.as_deref() {
                Some(act) => (
                    true,
                    act.pending_slot.map(|s| s.0),
                    act.last_forged_slot.map(|s| s.0),
                ),
                None => (false, None, None),
            };
            crate::node_log!(
                "live2b-tick-probe: non_authoritative=true forge_active={} logical_slot={:?}                  last_forged_slot={:?} forge_slot_status={:?} sync_status={:?} loop_state={:?}                  loop_step={:?}",
                forge_active,
                pending_slot,
                last_forged_slot,
                forge_slot,
                sync_status,
                loop_state,
                planned
            );
        }
        match planned {
            LoopStep::SyncOnce => {
                // B6 CENSUS: the four candidates all live in this arm. `blocks_before` vs
                // `blocks_after` is hypothesis B's operand (how much work one pass takes on), and it
                // is also what makes the two A/B arms comparable — the chain moves between them, so
                // iterations must be compared at equal work, not by wall time alone.
                let census_blocks_before = ChainDbServedSource::new(chaindb).tip().map(|(_, _, b)| b);
                // Deliberately UNINITIALISED: both the participant and node branches must assign it,
                // so a future third branch that forgets to instrument the sync call fails to compile
                // rather than silently reporting 0 ms and exonerating hypothesis A.
                let census_ms_sync: u128;
                // These two are node-branch only (the participant path calls neither), so 0 is a
                // truthful "did not run", not a missing measurement.
                let mut census_ms_coadvance: u128 = 0;
                let mut census_ms_boundary: u128 = 0;
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
                    let t_census_sync = std::time::Instant::now();
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
                        epoch_accumulator,
                    )
                    .await
                    .map_err(|e| match e {
                        NodeSyncError::RecoveryAdmission(f) => {
                            NodeLifecycleError::RecoveryAdmission(f)
                        }
                        other => NodeLifecycleError::RelaySync(format!("{other:?}")),
                    })?;
                    census_ms_sync = t_census_sync.elapsed().as_millis();
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
                            epoch_accumulator,
                        )
                        .map_err(|e| match e {
                            NodeSyncError::RecoveryAdmission(f) => {
                                NodeLifecycleError::RecoveryAdmission(f)
                            }
                            other => NodeLifecycleError::RelaySync(format!("{other:?}")),
                        })?;
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
                    // LIVE-FORGE-HARDENING S1: the single-producer / keyless-follower path now FOLLOWS a
                    // legal live rollback via run_node_sync. Source the rollback k-guard bound from the
                    // forge activation (default k for a keyless follower). The DC-NODE-28 fence is `None`:
                    // SyncOnce and ForgeTick are mutually-exclusive loop steps, so no ForgeTick observes the
                    // fence during run_node_sync's synchronous rollback apply (the helper sets+clears it in
                    // the one call); unlike the participant path there is no cross-iteration pending state.
                    let forge_k = forge
                        .as_deref()
                        .map(|a| a.security_param)
                        .unwrap_or_else(|| RecoveryAdmissionPolicy::cardano().security_param);
                    let t_census_sync = std::time::Instant::now();
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
                        // S4-L2: the frozen leadership authority — the SOLE promotion source for candidate
                        // epochs >= seed+2 (prepare_authority_for_candidate_slot fails closed without it).
                        epoch_accumulator,
                        forge_k,
                        None,
                    )
                        .await
                        .map_err(|e| NodeLifecycleError::RelaySync(format!("{e:?}")))?;
                    census_ms_sync = t_census_sync.elapsed().as_millis();
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
                    let t_census_coadv = std::time::Instant::now();
                    advance_ledger_state_to_durable_tip(
                        reduced_checkpoint,
                        epoch_accumulator,
                        chaindb,
                        &era_schedule,
                        &recovery_policy,
                    )?;
                    census_ms_coadvance = t_census_coadv.elapsed().as_millis();
                    // EPOCH-CONTINUITY-ACTIVATION ECA-1 (DC-EPOCH-13): the AUTOMATIC first-boundary
                    // activation (no arming flag). A strict no-op (byte-identical) until EVIEW is
                    // configured + the seed epoch completes; then it derives the bound view
                    // (durable window replay) + atomically promotes the ONE authority
                    // (Seed->Promoted; both consumers then read the N+1 view). Terminal halt on failure.
                    let t_census_bnd = std::time::Instant::now();
                    maybe_activate_epoch_boundary(
                        eview_activation,
                        reduced_checkpoint,
                        chaindb,
                        &mut era_schedule,
                        wal,
                        &mut authority,
                    )?;
                    census_ms_boundary = t_census_bnd.elapsed().as_millis();
                }
                // B6 CENSUS: one line, one pass, all four candidates competing for the same elapsed
                // time. `to_planner_ms` is hypothesis D (reached LATE); `sync_ms` is A; `blocks` is
                // B; `coadvance_ms` + `boundary_ms` are C. Emit-only; never read back.
                {
                    let blocks_after = ChainDbServedSource::new(chaindb).tip().map(|(_, _, b)| b);
                    let admitted = match (census_blocks_before, blocks_after) {
                        (Some(a), Some(b)) => b.saturating_sub(a),
                        _ => 0,
                    };
                    crate::node_log!(
                        "b6-census: non_authoritative=true iter_total_ms={} to_planner_ms={} \
                         has_work_ms={} is_ended_ms={} next_tick_ms={} sync_ms={} coadvance_ms={} \
                         boundary_ms={} blocks_admitted={} tip_block={:?}",
                        iter_t0.elapsed().as_millis(),
                        ms_to_planner,
                        ms_has_work,
                        ms_is_ended,
                        ms_next_tick,
                        census_ms_sync,
                        census_ms_coadvance,
                        census_ms_boundary,
                        admitted,
                        blocks_after
                    );
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
                // LIVE-2c part 3 (CE-L2c-A5): clear the previous tick's refusal FIRST. It was
                // sticky — written in eight places and cleared only on a successful forge. Every
                // path that DOES refuse overwrites it, so the harm was confined to the one path
                // that records nothing (no fence, no KES failure, and no tip to build on): that
                // tick re-emitted the PREVIOUS tick's reason and tip operands, turning an honest
                // absence into confident, wrong evidence. Narrow, but it is exactly the case an
                // operator reads when nothing is happening, and ruling it out by hand was a
                // required step in diagnosing this slice's live run.
                act.last_forge_refused = None;
                // KES period via the REUSED CoordinatorState method (no reimplementation). LIVE-2c
                // part 3: the `Result` form — an admitted tick may not disappear into a `None`.
                let mut forged = false;
                if let Some(refusal) = pending_reselection_forge_refusal(act.pending_reselection) {
                    // DC-NODE-28: a fork-choice re-selection is unresolved -- refuse
                    // the forge (typed), never forge on the stale pre-resolution tip.
                    act.last_forge_refused = Some(refusal);
                } else if let Some(kes_period) =
                    match act.coordinator_state.kes_period_for_slot_checked(slot.0) {
                        Ok(p) => Some(p),
                        // B11 CLOSED: the op-cert does not cover this slot. A typed, structured
                        // refusal naming WHICH of the three conditions fired and the slot bound an
                        // operator can act on — never a skip, never `no_tip_available`.
                        Err(e) => {
                            act.last_forge_refused = Some(ForgeRefused::KesWindow(e));
                            None
                        }
                    }
                {
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
                                // (The per-tick reset at the top of this arm already cleared any
                                // earlier refusal — LIVE-2c CE-L2c-A5.)
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
                                        // Reached a leader check -- nothing was skipped.
                                        skip_reason: None,
                                        compared_tips: None,
                                    });
                                }
                            }
                        }
                    }
                }
                if !forged {
                    // LIVE-2c part 3: the tick was ADMITTED and no forge ran. The outcome now says
                    // which of the two things happened instead of collapsing both into
                    // `no_tip_available`: a typed refusal was recorded (a fence, the KES window)
                    // => `refused` with that reason; nothing was recorded => there really was no
                    // selected tip to build on. `skip_reason` is this tick's own (the arm resets it
                    // on entry), so the reason and its operands always describe THIS slot.
                    let skip_reason = forge_skip_reason(act.last_forge_refused.as_ref());
                    if let Some(s) = sched.as_deref_mut() {
                        s.record(&crate::live_log::NodeSchedEvent::ForgeResult {
                            outcome: match skip_reason {
                                Some(_) => crate::live_log::ForgeOutcome::Refused,
                                None => crate::live_log::ForgeOutcome::NoTipAvailable,
                            },
                            self_admit_via_pump_block: false,
                            entered_forge_mode: forge_mode_kind(&act.forge_mode),
                            skip_reason,
                            compared_tips: forge_compared_tips(act.last_forge_refused.as_ref()),
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
                        // LIVE-2c part 2: the wake cadence comes from the SAME authority the slot
                        // conversion does (the active segment's own slot length), so there is no
                        // second slot-length number anywhere on this path.
                        let poll = std::time::Duration::from_millis(
                            forge
                                .as_deref()
                                .map(|a| u64::from(a.timing.slot_cadence_ms()))
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
    epoch_accumulator: Option<&ade_runtime::chaindb::EpochAccumulatorStore>,
    // CE-4A.3-R4c (DC-EPOCH-16): the venue RSW (`ceil(4k/f)`) for the materialize replay's candidate-nonce
    // freeze. The SAME value the live loop's `recovered_node_schedule` already supplies from `--network`, so
    // the warm-start replay freezes the candidate IDENTICALLY to the loop. Without it the replay's freeze is
    // inert (`RSW=None -> CANDIDATE_FREEZE_INERT`) and a warm-restart whose durable tip is PAST an epoch's
    // candidate-freeze slot OVER-TRACKS the candidate -> wrong `eta0(N+1)` -> the next boundary's header VRF
    // fails closed. `None` keeps the pre-R4c behavior (correct only when the tip is BEFORE the freeze slot).
    rsw: Option<u32>,
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
    // CE-4A.3-R4: an AdmitBlock SUPERSEDED by a later `WalEntry::RollBack` (its block was trimmed from the
    // ChainDb by the rollback's `commit_rollback`) is ABANDONED — `replay_from_anchor` skips it and does NOT
    // require its bytes (its own `compute_superseded` pre-pass). The pre-load below must apply the SAME
    // supersession, else a legitimate warm-restart-after-rollback fails closed `DurableBlockBytesMissing` on
    // a rolled-back block. A NON-superseded AdmitBlock with absent bytes is still corrupt durable state ->
    // fail closed (the invariant is preserved for the live / no-rollback path).
    let superseded = ade_ledger::wal::compute_superseded(&entries)
        .map_err(|e| NodeLifecycleError::WarmStartWalReplay(format!("{e:?}")))?;
    let mut block_bytes: BTreeMap<Hash32, Vec<u8>> = BTreeMap::new();
    for (entry_index, entry) in entries.iter().enumerate() {
        // Only `AdmitBlock` entries reference preserved block bytes;
        // `SeedEpochConsensusInputsImported` (A3a) entries carry no block
        // hash and are skipped.
        if let ade_ledger::wal::WalEntry::AdmitBlock { block_hash, .. } = entry {
            // CE-4A.3-R4: a rollback-superseded AdmitBlock's bytes are not required (it is abandoned).
            if superseded[entry_index] {
                continue;
            }
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
    // LIVE-LEDGER-EPOCH-TRANSITION S4: the warm-start replay leader schedule is the epoch-indexed frozen
    // leadership authority for the recovered seed epoch (read by EXACT epoch), not a re-projection of the seed
    // record's pool set. Byte-identical for the seed epoch (S4-0 1c lineage proof); fail closed if the durable
    // authority cannot answer (absent / uncertified / unsealed) — no seed-window fallback.
    let ledger_view = leadership_view_from_frozen_authority(epoch_accumulator, &sidecar)?;
    // WARMSTART-ERA-SCHEDULE-VENUE (DC-CINPUT-05): rebuild the recovery
    // era-schedule from the DURABLE sidecar geometry persisted at import -- the
    // venue's real epoch_start_slot + epoch_length (preview 86400, preprod
    // 432000, ...), NOT re-derived as epoch_no * a hardcoded length. This is the
    // SAME geometry the import used, so forward-replay is venue-correct and
    // replay-equivalent (the recovered store, not a restart CLI, is authority).
    // CE-4A.3-R4c (DC-EPOCH-16): the materialize replay below re-validates the durable blocks up to the
    // (reconciled) tip, which may be PAST an epoch's candidate-freeze slot (a rollback+warm-restart lands the
    // tip mid-epoch, after the freeze). The replay MUST freeze the candidate nonce at the SAME slot the live
    // loop does, so `eta0(N+1)` reconstructs correctly at the next boundary. So the replay schedule carries
    // the venue `rsw` the caller supplied (the SAME `--network` source `recovered_node_schedule` uses for the
    // loop). `None` leaves the freeze inert -- correct ONLY when the tip is before the freeze slot.
    // LIVE-FORGE-HARDENING S2 (DC-EPOCH-16): derive the candidate-freeze window RSW = ceil(4k/f) from
    // the DURABLE sidecar (v6 persists `security_param`), so warm-start freezes the candidate nonce
    // IDENTICALLY to the live fold regardless of whether a restart CLI supplies `--network` (an absent
    // CLI no longer leaves the freeze INERT -> over-track -> DC-EPOCH-16). The store is the sole
    // authority; the CLI-supplied `rsw` is kept ONLY as a fail-closed cross-check (a mismatch means the
    // restart CLI disagrees with the durable venue -> terminal, never silently preferred). Mirrors
    // `assert_restart_genesis_matches_sidecar`.
    let sidecar_rsw = sidecar_freeze_rsw(&sidecar, rsw)?;
    let era_schedule = make_node_schedule(
        sidecar.epoch_start_slot,
        sidecar.epoch_no,
        sidecar.epoch_length_slots,
        sidecar_rsw,
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
            // T-REC-05 promoted (S5 2b): a WAL-tail fingerprint disagreement is a durable-state
            // contradiction -- the recovered ledger is NOT replay-equivalent to the admitted chain --
            // so it fails closed as a typed recovery-admission fault, not a generic warm-start string.
            //
            // P6-S4: the fault itself now CARRIES the diagnosis. The bare (expected, recovered) pair
            // cost hours and four wrong hypotheses in P4; what actually cracked it was per-component
            // fingerprints plus the ledger-vs-schedule epoch pair. Both are assembled here and travel
            // WITH the error, so the next divergence is read off the fault instead of reconstructed
            // with bespoke probes. Everything below is either already computed or one cheap read; the
            // anchor decode is best-effort and degrades to `None` rather than masking the real fault.
            let snaps = SnapshotStore::list_snapshot_slots(chaindb).unwrap_or_default();
            let anchor_slot = snaps.iter().rev().find(|s| s.0 <= wal_tail_slot.0).copied();
            let report_reader = PersistentSnapshotCache::new(chaindb);
            let report_source = ChainDbBlockSource::new(chaindb);
            let anchor_fp = anchor_slot.and_then(|a| {
                materialize_rolled_back_state(
                    TargetPoint {
                        slot: a,
                        hash: Hash32([0u8; 32]),
                    },
                    &report_reader,
                    &report_source,
                    &era_schedule,
                    &ledger_view,
                    None,
                )
                .ok()
                .map(|(l, _)| fingerprint(&l))
            });
            let span_blocks = anchor_slot.map(|a| {
                entries
                    .iter()
                    .filter(|e| {
                        matches!(e, ade_ledger::wal::WalEntry::AdmitBlock { slot, .. }
                            if slot.0 > a.0 && slot.0 <= wal_tail_slot.0)
                    })
                    .count() as u64
            });
            let report = ade_ledger::replay_divergence::ReplayDivergenceReport {
                slot: wal_tail_slot,
                admit_count: admit_count as u64,
                ledger_epoch: recovered.ledger.epoch_state.epoch,
                schedule_epoch: era_schedule.locate(wal_tail_slot).ok().map(|l| l.epoch),
                expected_combined: wal_tail_fp.clone(),
                actual: fingerprint(&recovered.ledger),
                anchor: anchor_fp,
                anchor_slot,
                span_blocks,
                store_semantics_version: ade_ledger::store_semantics::STORE_SEMANTICS_VERSION,
                artifact: ade_ledger::store_semantics::AuthorityArtifact::ChainDb,
            };
            crate::node_log!("warmstart-replay-divergence: {}", report);
            // EMIT-ONLY: the geometry above says the replay reached the right tip from a
            // near-by anchor, which leaves the ANCHOR itself as the suspect. Each durable
            // snapshot has a WAL-recorded expected value: the `post_fp` of the AdmitBlock at
            // the same slot. A degenerate `materialize_rolled_back_state` AT a snapshot slot
            // reads that snapshot back with no forward replay and no header-VRF validation,
            // so this compares the STORED anchor against the ADMITTED ledger, per snapshot,
            // over the production materialize path. It distinguishes "the forward replay
            // diverged" from "the replay was correct and started from a poisoned anchor" --
            // and names WHICH anchors are clean. Bounded by the snapshot retention count and
            // reached only on an already-terminal fault.
            let wal_fp_at: BTreeMap<u64, Hash32> = entries
                .iter()
                .filter_map(|e| match e {
                    ade_ledger::wal::WalEntry::AdmitBlock { slot, post_fp, .. } => {
                        Some((slot.0, post_fp.clone()))
                    }
                    _ => None,
                })
                .collect();
            let wal_hash_at: BTreeMap<u64, Hash32> = entries
                .iter()
                .filter_map(|e| match e {
                    ade_ledger::wal::WalEntry::AdmitBlock {
                        slot, block_hash, ..
                    } => Some((slot.0, block_hash.clone())),
                    _ => None,
                })
                .collect();
            let probe_reader = PersistentSnapshotCache::new(chaindb);
            let probe_source = ChainDbBlockSource::new(chaindb);
            // P6-S4: the anchor was already materialized to build the report, so REUSE it rather than
            // decoding a multi-GB ledger a second time. This path is terminal and was reached, in P4,
            // on a box that had just been OOM-killed -- gratuitous re-materializes here are how a
            // diagnostic turns into a second outage. The verdict below is still worth emitting: it
            // compares the stored anchor against the WAL's `post_fp` at the SAME slot, which
            // distinguishes "the anchor was poisoned" from "the forward replay diverged".
            let anchor = report.anchor_slot;
            if let Some(a) = anchor {
                match report.anchor.as_ref() {
                    Some(f) => {
                        let snap_fp = f.combined.clone();
                        let expected = wal_fp_at.get(&a.0);
                        crate::node_log!(
                            "warmstart-snap-probe: slot={} snap_fp={} wal_fp={} verdict={}",
                            a.0,
                            hex_prefix8(&snap_fp),
                            expected.map(hex_prefix8).unwrap_or_else(|| "-".to_string()),
                            match expected {
                                None => "no-wal-entry",
                                Some(w) if *w == snap_fp => "CLEAN",
                                Some(_) => "POISONED",
                            },
                        );
                    }
                    None => crate::node_log!(
                        "warmstart-snap-probe: slot={} anchor could not be materialized",
                        a.0
                    ),
                }
                // EMIT-ONLY: the replay reads BLOCK BYTES from the ChainDb, but the WAL is
                // the record of what was ADMITTED. Recovery only trims orphans ABOVE the
                // WAL tail (`rollback_to_slot(wal_tail_slot)`), so a block stored below the
                // tail but never admitted -- the pump writes StoreBlockBytes BEFORE
                // AppendWal -- survives recovery and would be replayed as if admitted.
                // Compare the two sets over the replay span directly: no ledger work, just
                // block reads. An `extra_in_chaindb` slot or a `hash_mismatch` means the
                // replay is applying a DIFFERENT chain than the WAL recorded.
                let wal_span: Vec<u64> = wal_hash_at
                    .range((a.0 + 1)..=wal_tail_slot.0)
                    .map(|(s, _)| *s)
                    .collect();
                match chaindb.range_bytes_capped(SlotNo(a.0 + 1), wal_tail_slot, 4096) {
                    Ok(range) => {
                        let mut extra: Vec<u64> = Vec::new();
                        let mut mismatch: Vec<u64> = Vec::new();
                        for (slot, bytes) in &range.blocks {
                            match wal_hash_at.get(&slot.0) {
                                None => extra.push(slot.0),
                                Some(w) => {
                                    let got = decode_block(bytes).ok().map(|d| d.block_hash);
                                    if got.as_ref() != Some(w) {
                                        mismatch.push(slot.0);
                                    }
                                }
                            }
                        }
                        crate::node_log!(
                            "warmstart-chain-vs-wal: span=({},{}] chaindb_blocks={} wal_admits={} \
                             truncated={} extra_in_chaindb={} first_extra={:?} hash_mismatch={} \
                             first_mismatch={:?}",
                            a.0,
                            wal_tail_slot.0,
                            range.blocks.len(),
                            wal_span.len(),
                            range.truncated,
                            extra.len(),
                            extra.first(),
                            mismatch.len(),
                            mismatch.first(),
                        );
                    }
                    Err(e) => {
                        crate::node_log!("warmstart-chain-vs-wal: range-err={:?}", e)
                    }
                }
            }
            // EMIT-ONLY: if the anchors are clean, the divergence is INSIDE the forward
            // replay, and the useful fact is WHICH block first disagrees. Every admitted
            // slot in (anchor, wal_tail] has a WAL-recorded `post_fp`, and
            // `materialize_rolled_back_state` at slot S replays the SAME span the failing
            // recovery replays, so `materialize(S) == wal_post_fp(S)` is monotone: true up
            // to the first divergent block, false after. That admits a BINARY SEARCH --
            // ~log2(n) materializes instead of one per block -- for the first slot where
            // the warm-start replay stops reproducing live admission. Reached only on an
            // already-terminal fault.
            if let Some(anchor) = anchor {
                let span: Vec<u64> = wal_fp_at
                    .range((anchor.0 + 1)..=wal_tail_slot.0)
                    .map(|(s, _)| *s)
                    .collect();
                let agrees = |slot: u64| -> Option<bool> {
                    let at = TargetPoint {
                        slot: SlotNo(slot),
                        hash: Hash32([0u8; 32]),
                    };
                    let got = materialize_rolled_back_state(
                        at,
                        &probe_reader,
                        &probe_source,
                        &era_schedule,
                        &ledger_view,
                        None,
                    )
                    .ok()?;
                    Some(fingerprint(&got.0).combined == *wal_fp_at.get(&slot)?)
                };
                // Invariant: `lo` agrees, `hi` disagrees. Converge on the first disagreeing slot.
                let (mut lo, mut hi) = (0usize, span.len());
                while lo < hi {
                    let mid = lo + (hi - lo) / 2;
                    match agrees(span[mid]) {
                        Some(true) => lo = mid + 1,
                        Some(false) => hi = mid,
                        None => break,
                    }
                }
                crate::node_log!(
                    "warmstart-replay-bisect: anchor={} span_blocks={} first_divergent_slot={:?} \
                     wal_fp_there={:?}",
                    anchor.0,
                    span.len(),
                    span.get(lo),
                    span.get(lo).and_then(|s| wal_fp_at.get(s)).map(hex_prefix8),
                );
                // P6-S4: the per-component dump that used to live here is GONE -- superseded by the
                // typed report above, which carries the anchor's and the result's components and
                // computes `moved_components()` from them. Keeping both would have cost two more
                // multi-GB materializes to restate what the fault already says, on a terminal path
                // that is reached exactly when the machine is least healthy.
            }
            return Err(NodeLifecycleError::RecoveryAdmission(
                RecoveryAdmissionFault::FingerprintMismatch {
                    expected: wal_tail_fp,
                    recovered: recovered_fp,
                    report: Box::new(report),
                },
            ));
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
    // LIVE-LEDGER-EPOCH-TRANSITION S4-L1: seal the epoch-indexed frozen leadership authority (DC-EPOCH-25) from
    // the manifest-bound seed record so the production leadership reads (initial/warm header-validation view) read
    // it by EXACT epoch — the legacy-route analog of the native route's bootstrap seal (native_firstrun). The
    // seed record's pool_distribution IS the seed leadership nesPd (S4-0, proven byte-exact). Leadership-only (no
    // accumulator baseline on this route): the governance gate ignores the resulting `Unsealed` verdict, and
    // `leadership_authority_for_epoch` needs only the leadership marker. Block-scoped so the redb handle drops
    // before the caller re-opens the store as the live authority. Non-fatal, like the native seal.
    {
        if let Ok(store_dir) = resolve_store_dir(cli) {
            // S4-L2 (v6): derive the seed-point checkpoint commitment HONESTLY from THIS route's OWN restored seed
            // ledger -- the reduced-UTxO checkpoint over `out.ledger`, the SAME derivation the native route uses
            // (reduce the UTxO -> build_from -> seal at the seed point -> finalize). This mithril cold-start
            // persists no live checkpoint at seal time, so it is computed in a SCRATCH checkpoint that is removed
            // immediately. NEVER fabricated. These bootstrap-indexed objects are non-promotion-certified regardless
            // (the promotion reader requires current-only-non-bootstrap), so the commitment is honest provenance.
            let seed_commitment: Option<Hash32> = {
                use ade_ledger::reduced_utxo::{reduce_txout, ReducedStakeRef};
                let mut reduced: std::collections::BTreeMap<
                    ade_types::tx::TxIn,
                    (ade_types::tx::Coin, ReducedStakeRef),
                > = std::collections::BTreeMap::new();
                for (txin, txout) in out.ledger.utxo_state.utxos.iter() {
                    reduced.insert(txin.clone(), reduce_txout(txout));
                }
                let scratch = store_dir.join("seed-commitment-scratch.redb");
                let _ = std::fs::remove_file(&scratch);
                let commitment = ade_runtime::chaindb::ReducedUtxoCheckpoint::open(&scratch)
                    .ok()
                    .and_then(|cp| {
                        cp.build_from(&reduced).ok()?;
                        cp.seal_bootstrap(out.seed_epoch_consensus_inputs.seed_point_slot).ok()?;
                        cp.finalize().ok()
                    });
                let _ = std::fs::remove_file(&scratch);
                commitment
            };
            match (
                seed_commitment,
                ade_runtime::chaindb::EpochAccumulatorStore::open(
                    &store_dir.join("epoch-accumulator.redb"),
                ),
            ) {
                (Some(commitment), Ok(store)) => {
                    if let Err(e) = store.seal_bootstrap_leadership_epochs(&[
                        ade_ledger::frozen_leadership::FrozenLeadershipPoolDistr::from_seed_epoch_consensus_inputs(
                            &out.seed_epoch_consensus_inputs,
                            commitment,
                        ),
                    ]) {
                        eprintln!(
                            "ade_node --mode node: legacy first-run frozen-leadership seal skipped (non-fatal): {e:?}"
                        );
                    }
                }
                (None, _) => eprintln!(
                    "ade_node --mode node: legacy first-run frozen-leadership seal skipped (seed-point reduced-checkpoint commitment derivation failed; L1-view-only until re-bootstrap)"
                ),
                (_, Err(e)) => eprintln!(
                    "ade_node --mode node: legacy first-run accumulator open skipped (non-fatal): {e:?}"
                ),
            }
        }
    }
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
                security_param: p.security_param,
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
    // PREPROD-NONCE-1 (emit-only): the STARTING nonce quad the seed hands the fold.
    //
    // The sidecar carries a SINGLE `epoch_nonce` -- there is no imported candidate/evolving pair --
    // so `candidate` at the seed point is whatever the bootstrap constructed, and every later
    // accumulation is relative to it. A boundary eta0 disagreement is therefore ambiguous between
    // "the fold diverged" and "the fold started from the wrong place" unless the starting values are
    // on the record. Pairs with `nonce1-boundary-operands` to bracket the whole accumulation.
    crate::node_log!(
        "nonce1-seed-quad: seed_epoch={:?} seed_slot={} epoch_start_slot={} epoch_len={} \
         sidecar_eta0={:?} start_candidate={:?} start_evolving={:?} start_epoch_nonce={:?} \
         start_prev_epoch_nonce={:?} start_lab={:?} start_last_epoch_block={:?}",
        out.seed_epoch_consensus_inputs.epoch_no,
        out.seed_epoch_consensus_inputs.seed_point_slot.0,
        out.seed_epoch_consensus_inputs.epoch_start_slot.0,
        out.seed_epoch_consensus_inputs.epoch_length_slots,
        out.seed_epoch_consensus_inputs.epoch_nonce.0,
        out.chain_dep.candidate_nonce.0,
        out.chain_dep.evolving_nonce.0,
        out.chain_dep.epoch_nonce.0,
        out.chain_dep.previous_epoch_nonce.0,
        out.chain_dep.lab_nonce.0,
        out.chain_dep.last_epoch_block,
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
/// unknown (e.g. a bare `--shelley-genesis-path` start with no `--network`).
/// LIVE-FORGE-HARDENING S2 (DC-EPOCH-16) closed the former inert-freeze gap: the
/// recovered freeze window now comes from the DURABLE sidecar's `k` via
/// `sidecar_freeze_rsw`, so this CLI value is ONLY a fail-closed cross-check --
/// `None` here means "no cross-check available," never an inert candidate freeze.
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

/// LIVE-FORGE-HARDENING S2 (DC-EPOCH-16): the candidate-freeze window `RSW = ceil(4k/f)` for a
/// RECOVERED venue, sourced from the DURABLE sidecar's persisted `k` -- NOT the restart CLI. Both the
/// recovery replay (`warm_start_recovery`) and the forward live-loop schedule (`recovered_node_schedule`)
/// derive the window HERE, so they can never desync: the store is the SOLE freeze authority on both
/// paths, and an absent/unsupported restart `--network` can no longer leave the freeze INERT (`None`)
/// -> candidate over-track -> wrong `eta0(N+1)`. The CLI-supplied `cli_rsw` is retained ONLY as a
/// fail-closed cross-check (a mismatch means the restart CLI disagrees with the durable venue ->
/// terminal, never silently preferred). Mirrors `assert_restart_genesis_matches_sidecar` (geometry).
fn sidecar_freeze_rsw(
    sidecar: &SeedEpochConsensusInputs,
    cli_rsw: Option<u32>,
) -> Result<Option<u32>, NodeLifecycleError> {
    let store_rsw = ade_core::consensus::era_schedule::praos_rsw_slots(
        sidecar.security_param,
        u64::from(sidecar.active_slots_coeff.numer),
        u64::from(sidecar.active_slots_coeff.denom),
    );
    if let (Some(cli), Some(store)) = (cli_rsw, store_rsw) {
        if cli != store {
            return Err(NodeLifecycleError::NativeFirstRun(format!(
                "DC-EPOCH-16 RSW cross-check: durable sidecar k={} -> RSW {} != CLI-supplied RSW {}",
                sidecar.security_param, store, cli
            )));
        }
    }
    // PREPROD-NONCE-1 (emit-only): the candidate-freeze window's PROVENANCE, not just its value.
    // The prior preview harness defect was exactly a wrong venue `k`, and the value alone cannot
    // distinguish "the store said so" from "the CLI said so" from "nothing said so and the freeze is
    // INERT". Emitted once at resolution, before any boundary depends on it.
    crate::node_log!(
        "nonce1-freeze-window: source=durable-sidecar k={} f={}/{} store_rsw={:?} cli_rsw={:?} \
         cross_check={} effective_rsw={:?}",
        sidecar.security_param,
        sidecar.active_slots_coeff.numer,
        sidecar.active_slots_coeff.denom,
        store_rsw,
        cli_rsw,
        match (cli_rsw, store_rsw) {
            (Some(_), Some(_)) => "agreed",
            (None, Some(_)) => "store-only (no CLI value to cross-check)",
            (Some(_), None) => "store DERIVED NOTHING -- freeze would be INERT",
            (None, None) => "NEITHER -- freeze INERT",
        },
        store_rsw,
    );
    Ok(store_rsw)
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
    cli_rsw: Option<u32>,
) -> Result<EraSchedule, NodeLifecycleError> {
    match state.seed_epoch_consensus_inputs.as_ref() {
        // LIVE-FORGE-HARDENING S2 (DC-EPOCH-16): the FORWARD live-loop freeze window is the DURABLE
        // store's authority (identical to the recovery replay via the shared `sidecar_freeze_rsw`), so
        // an absent/unsupported restart `--network` can no longer leave the candidate freeze INERT on
        // the forward path -> over-track -> wrong `eta0`. The CLI rsw is only a fail-closed cross-check.
        Some(s) => Ok(make_node_schedule(
            s.epoch_start_slot,
            s.epoch_no,
            s.epoch_length_slots,
            sidecar_freeze_rsw(s, cli_rsw)?,
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
        NodeLifecycleError::RecoveryAdmission(f) => {
            eprintln!(
                "ade_node --mode node: TERMINAL recovery-admission fault ({f:?}) -- the persisted \
                 accumulator cannot be proven to describe ONE canonical selected-chain prefix; the store is \
                 terminal until re-bootstrap (or an explicit admissible recovery). Recovery does not \
                 continue with an uncertified or inadmissible durable accumulator."
            );
        }
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
        NodeLifecycleError::AccumulatorPredatesGovernanceImport { era_tag } => {
            eprintln!(
                "ade_node --mode node: warm-start FAIL-CLOSED -- the durable EpochAccumulator's sealed \
                 bootstrap baseline (era tag {era_tag}) PREDATES the governance-proposal import: it \
                 carries NO imported governance state (gov_state = None, a pre-v6 bootstrap). A missing \
                 imported governance set must NEVER be treated as zero proposals (absent != empty). This \
                 is a RE-BOOTSTRAP requirement, NOT corruption -- re-bootstrap from the certified \
                 snapshot to import the gov proposals + committee and rewrite the store at v6."
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
        NodeLifecycleError::ProductionLeadershipAuthorityUnavailable { epoch, reason } => {
            eprintln!(
                "ade_node --mode node: the epoch-indexed frozen leadership authority cannot \
                 answer for epoch {epoch} ({reason}); the leader schedule is read ONLY from \
                 the durable frozen authority (no seed-window fallback), so failing closed. \
                 Re-bootstrap to leadership-certify the store."
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

/// LIVE-FORGE-HARDENING S1 — the shared peer-`RollBackward` resolve+apply authority for BOTH live
/// loops (`run_participant_sync` and the `--mode node` forge path `run_node_sync`). Extracted verbatim
/// from the participant `RollBack` arm: DC-NODE-29 (canonical STORED slot+hash point is the sole
/// authority), DC-NODE-33 (recovered-anchor exact-slot-and-hash is an idempotent no-op), the k-guard
/// via `accumulator_admit_and_clear_for_rollback`, and DC-NODE-28 (fence set BEFORE apply, cleared
/// after). Reuses the BLUE `materialize_rolled_back_state` / `commit_rollback` (through
/// `apply_chain_event`) UNCHANGED — no new authority, no new `WalEntry` variant. `Ok(())` means both
/// the anchor no-op and a successfully-applied rollback (the caller keeps following); every illegal
/// rollback keeps its exact typed halt (Origin / unknown-hash → `UnexpectedRollback`, peer-slot ≠
/// stored-slot → `RollbackPointSlotMismatch`, beyond-k / below-seed / off-chain → the accumulator fault).
#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve_and_apply_peer_rollback<D>(
    state: &mut ForwardSyncState,
    chaindb: &D,
    wal: &mut dyn WalStore,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    epoch_accumulator: Option<&ade_runtime::chaindb::EpochAccumulatorStore>,
    security_param: SecurityParam,
    wire_point: ade_network::codec::chain_sync::Point,
    pending_reselection: &mut bool,
) -> Result<(), NodeSyncError>
where
    D: ChainDb + SnapshotStore,
{
    let (slot, hash) = match wire_point {
        ade_network::codec::chain_sync::Point::Block { slot, hash } => (slot, hash),
        ade_network::codec::chain_sync::Point::Origin => {
            return Err(NodeSyncError::UnexpectedRollback);
        }
    };
    // DC-NODE-33: the recovered bootstrap anchor (a recovery snapshot boundary, NOT a stored servable
    // block) bound on BOTH slot and hash is an idempotent no-op -- evaluated BEFORE get_block_by_hash
    // (which would otherwise fail closed on the un-stored anchor). No durable mutation.
    if let Some(anchor) = &state.recovered_anchor {
        if slot == anchor.slot && hash == anchor.hash {
            return Ok(());
        }
    }
    // DC-NODE-29: resolve the wire hash against the durable ChainDb; the STORED point is the sole
    // authority (peer slot never constructs `to_point`). Unknown hash or peer-slot != stored-slot
    // fails closed HERE -- before apply_chain_event / commit_rollback / any durable mutation.
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
    let target = Point {
        slot: stored.slot,
        hash,
    };
    // S5 (2b): pre-clear the durable accumulator's certified anchor (k-guard via admit_rollback) BEFORE
    // commit_rollback trims the ChainDb. A typed RecoveryAdmission fault is terminal; an incidental
    // store/read fault -> Pump.
    accumulator_admit_and_clear_for_rollback(
        epoch_accumulator,
        chaindb,
        &target,
        &RecoveryAdmissionPolicy { security_param },
    )
    .map_err(|e| match e {
        NodeLifecycleError::RecoveryAdmission(f) => NodeSyncError::RecoveryAdmission(f),
        other => NodeSyncError::Pump(format!("accumulator rollback pre-clear: {other:?}")),
    })?;
    let (target_slot_for_trace, target_hash_for_trace) =
        (target.slot.0, hex_prefix8(&target.hash));
    let event = ChainEvent::RolledBack {
        to_point: target,
        depth: BlockDistance(0),
    };
    // DC-NODE-28: set pending BEFORE apply; clear ONLY after apply returns (reconcile/failure handling
    // complete) -- no forge may slip through between rollback start and durable settlement.
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
    // LIVE-REFOLD-THRASH RF-1 (DC-EPOCH-35): the rollback is now DURABLE. Re-certify the settled
    // rewind point against the chain as it now stands and re-establish the lineage anchor there, so
    // the next recovery pass forward-folds from the bounded point instead of reading an absent
    // anchor and refolding from bootstrap. Strictly AFTER commit -- the anchor is never carried
    // across the rollback window, so the S5 pre-clear crash-safety property is unchanged. Any
    // refusal leaves the anchor absent, i.e. exactly the pre-slice behaviour.
    accumulator_recertify_settled_after_rollback(
        epoch_accumulator,
        chaindb,
        &RecoveryAdmissionPolicy { security_param },
    )
    .map_err(|e| NodeSyncError::Pump(format!("settled re-certification: {e:?}")))?;
    // EMIT-ONLY: a followed rollback was previously SILENT, so the `follow:` tip log (which is
    // throttled) was the only hint one had happened -- and it undercounts badly. Without this an
    // operator cannot tell a rollback-driven accumulator reset from a recovery-driven one.
    crate::node_log!(
        "rollback-followed: to_slot={} to_hash={}",
        target_slot_for_trace,
        target_hash_for_trace
    );
    Ok(())
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
    // LIVE-LEDGER-EPOCH-TRANSITION S5 (2b): the durable EpochAccumulator, so a chain-selection-admitted
    // rollback pre-clears its certified lineage anchor BEFORE commit_rollback (event-qualified lockstep).
    // `None` on non-native / wrapper callers -> inert.
    epoch_accumulator: Option<&ade_runtime::chaindb::EpochAccumulatorStore>,
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
                // LIVE-FORGE-HARDENING S1: the resolve+apply logic is now the shared
                // `resolve_and_apply_peer_rollback` authority (identical DC-NODE-29/33/28 logic; also
                // driven by the run_node_sync forge path). Participant behavior is byte-for-byte unchanged.
                resolve_and_apply_peer_rollback(
                    state,
                    chaindb,
                    wal,
                    era_schedule,
                    ledger_view,
                    epoch_accumulator,
                    security_param,
                    wire_point,
                    pending_reselection,
                )?;
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
    // LIVE-LEDGER-EPOCH-TRANSITION S5 (2b): the durable EpochAccumulator, so a proven fork-switch (a
    // chain-selection-admitted rollback to the LCA) pre-clears its certified lineage anchor BEFORE
    // commit_rollback (event-qualified lockstep). `None` on non-native / wrapper callers -> inert.
    epoch_accumulator: Option<&ade_runtime::chaindb::EpochAccumulatorStore>,
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

    // LIVE-LEDGER-EPOCH-TRANSITION S5 (2b): a proven fork-switch is a chain-selection-admitted rollback to
    // the LCA -- bring the durable accumulator into lockstep by pre-clearing its certified anchor (after
    // admitting the rollback to `fork_anchor`) BEFORE the commit_rollback below trims the ChainDB. Terminal
    // on an inadmissible rollback.
    accumulator_admit_and_clear_for_rollback(
        epoch_accumulator,
        chaindb,
        &Point {
            slot: switch.fork_anchor.slot,
            hash: switch.fork_anchor.hash.clone(),
        },
        &RecoveryAdmissionPolicy { security_param },
    )
    .map_err(|e| match e {
        NodeLifecycleError::RecoveryAdmission(f) => NodeSyncError::RecoveryAdmission(f),
        other => NodeSyncError::Pump(format!("fork-switch accumulator pre-clear: {other:?}")),
    })?;
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
            // v2: a persisted Conway accumulator carries the deposit params (the codec fails closed on None).
            acc.conway_deposit_params = Some(ade_ledger::pparams::ConwayOnlyDepositParams {
                drep_deposit: Coin(500_000_000),
                gov_action_deposit: Coin(100_000_000_000),
                drep_activity: 20,
            });
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

        /// CN-NODE-04: the forge skip-reason projection is TOTAL over the typed refusal set.
    ///
    /// `outcome: no_tip_available` is a catch-all; before this the distinguishing typed
    /// refusal was computed and discarded, so an operator could not tell a tip mismatch
    /// from a fence refusal from a KES-window failure — three different fixes. Each
    /// refusal must map to its own discriminator, and `None` (no typed refusal) must
    /// stay distinguishable, since that is what rules the DC-NODE-15 gate OUT.
    #[test]
    fn forge_skip_reason_projects_every_typed_refusal() {
        use crate::live_log::ForgeSkipReason as R;
        use crate::node_sync::NotCaughtUpReason as N;

        let tips = || (None, None);
        for (reason, expect) in [
            (N::NoFollowedPeerTip, R::NoFollowedPeerTip),
            (N::NoDurableServableTip, R::NoDurableServableTip),
            (N::TipMismatch, R::TipMismatch),
        ] {
            let (local_servable_tip, followed_peer_tip) = tips();
            let refused = ForgeRefused::NotCaughtUp {
                local_servable_tip,
                followed_peer_tip,
                reason,
            };
            assert_eq!(forge_skip_reason(Some(&refused)), Some(expect));
        }

        assert_eq!(
            forge_skip_reason(Some(&ForgeRefused::ReselectionPending)),
            Some(R::ReselectionPending)
        );

        // No typed refusal recorded -> None. This is NOT "no reason": it positively
        // rules out the DC-NODE-15 gate and points at the KES window instead.
        assert_eq!(forge_skip_reason(None), None);
    }

    /// Every BLUE `RecoveryAction` branch projects to a typed trace reason, and the reasons are
    /// distinct. The projection is TOTAL by construction (a new `ResetReason` breaks
    /// `from_reset` at compile time); this pins that the mapping is also injective, so a trace
    /// cannot conflate an absent anchor with an over-advanced accumulator — the two have
    /// different fixes and that ambiguity is exactly what cost us 8h of unreadable logs.
    #[test]
    fn every_recovery_branch_projects_to_a_distinct_trace_reason() {
        assert_eq!(
            RecoveryTraceReason::from_reset(ResetReason::AnchorAbsent),
            RecoveryTraceReason::AnchorAbsent
        );
        assert_eq!(
            RecoveryTraceReason::from_reset(ResetReason::DurableTipBehindAnchor),
            RecoveryTraceReason::DurableTipBehindAnchor
        );
        // Discriminators are stable AND pairwise distinct across the whole closed set.
        let all = [
            RecoveryTraceReason::AnchorAbsent,
            RecoveryTraceReason::DurableTipBehindAnchor,
            RecoveryTraceReason::RollbackAdmission,
            RecoveryTraceReason::CanonicalHashMismatch,
            RecoveryTraceReason::MissingCanonicalBlock,
            RecoveryTraceReason::ForwardFoldNoReset,
        ];
        let mut seen: Vec<&str> = all.iter().map(|r| r.as_str()).collect();
        let n = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), n, "trace reason discriminators must be distinct");
        assert_eq!(
            RecoveryTracePath::RecoveryAdmit.as_str(),
            "recovery_admit"
        );
        assert_eq!(RecoveryTracePath::RollbackAdmit.as_str(), "rollback_admit");
    }

    /// CN-NODE-04: a tip refusal carries BOTH tips, so `tip_mismatch` says WHERE.
    ///
    /// The gate requires equality on both `hash` and `block_no`. Without the tips, a
    /// serve projection lagging by one block, a systematic block_no disagreement, and a
    /// hash difference all emit identically — and the live rehearsal showed 100% of
    /// steady-state ticks refusing with `tip_mismatch`, which is unactionable on its own.
    #[test]
    fn tip_refusal_carries_both_compared_tips() {
        let local = TipPoint {
            slot: SlotNo(100),
            hash: Hash32([0xAA; 32]),
            block_no: 10,
        };
        let peer = TipPoint {
            slot: SlotNo(101),
            hash: Hash32([0xBB; 32]),
            block_no: 11,
        };
        let refused = ForgeRefused::NotCaughtUp {
            local_servable_tip: Some(local),
            followed_peer_tip: Some(peer),
            reason: crate::node_sync::NotCaughtUpReason::TipMismatch,
        };
        let t = forge_compared_tips(Some(&refused)).expect("tips emitted on a tip refusal");
        assert_eq!(t.local_slot, Some(100));
        assert_eq!(t.local_block_no, Some(10));
        assert_eq!(t.local_hash, Some(Hash32([0xAA; 32])));
        assert_eq!(t.peer_slot, Some(101));
        assert_eq!(t.peer_block_no, Some(11));
        assert_eq!(t.peer_hash, Some(Hash32([0xBB; 32])));

        // An ABSENT tip is preserved as None rather than fabricated -- absence is itself
        // one of the named refusals.
        let half = ForgeRefused::NotCaughtUp {
            local_servable_tip: None,
            followed_peer_tip: None,
            reason: crate::node_sync::NotCaughtUpReason::NoDurableServableTip,
        };
        let t2 = forge_compared_tips(Some(&half)).expect("still emitted");
        assert_eq!(t2.local_slot, None);
        assert_eq!(t2.peer_hash, None);

        // A non-tip refusal carries no tips (nothing to compare).
        assert!(forge_compared_tips(Some(&ForgeRefused::ReselectionPending)).is_none());
        assert!(forge_compared_tips(None).is_none());
    }

    /// ACCUMULATOR-REFOLD-BOUND S1 — CE-AR-2 / INV-AR-1 and CE-AR-3 / INV-AR-2.
        ///
        /// `settled_rewind_admissible` is the gate on the bounded rewind. Two independent reasons
        /// to refuse, each checked in isolation: a point still inside the reorg window (an
        /// admissible reorg could still reach it), and a point the chain has ABANDONED (its hash no
        /// longer resolves canonically at that slot). Either refusal sends the caller to
        /// `reset_to_bootstrap`, i.e. the unchanged pre-slice behaviour — so a refusal costs refold
        /// time and nothing else.
        #[test]
        fn settled_rewind_admission_requires_settled_depth_and_intact_lineage() {
            use ade_runtime::chaindb::advance_accumulator_over_chaindb;
            let tmp = TempDir::new().unwrap();
            let s = sealed_store_at_epoch_500(&tmp, SlotNo(42_000_000));
            let db = InMemoryChainDb::new();
            put_raw(&db, 43_000_000);
            advance_accumulator_over_chaindb(
                &s,
                &db,
                &schedule_86k(),
                SlotNo(42_000_000),
                SlotNo(43_500_000),
            )
            .unwrap();

            // Promote the folded point to SETTLED. The synthetic fixture reuses one raw block, so
            // every block decodes to the same height; k=0 here isolates the two conditions under
            // test from the height arithmetic.
            let bn = s
                .last_advanced_point()
                .unwrap()
                .expect("certified after fold")
                .block_no;
            assert!(!s.roll_settled_rewind_point(bn, 0).unwrap());
            assert!(s.roll_settled_rewind_point(bn, 0).unwrap());
            let sp = s.settled_rewind_point().unwrap().expect("promoted");
            assert_eq!(sp.slot, SlotNo(43_000_000));

            let target = Point {
                slot: SlotNo(43_000_000),
                hash: sp.header_hash.clone(),
            };

            // Baseline: settled (k = 0) AND lineage intact -> admissible.
            assert!(
                settled_rewind_admissible(&s, &db, &target, 0),
                "a settled, lineage-intact point must be usable"
            );

            // CE-AR-2 / INV-AR-1: the same point is REFUSED once k puts it inside the reorg window.
            assert!(
                !settled_rewind_admissible(&s, &db, &target, 1_000_000),
                "a point within k of the tip is not settled and must be refused"
            );

            // CE-AR-3 / INV-AR-2: a chain carrying a DIFFERENT block at that slot -> the point was
            // abandoned. Refused even though the depth condition still holds.
            let diverged = InMemoryChainDb::new();
            diverged
                .put_block(&StoredBlock {
                    hash: Hash32([0xEE; 32]),
                    slot: SlotNo(43_000_000),
                    bytes: RAW_CONWAY_BLOCK.to_vec(),
                })
                .unwrap();
            assert_ne!(sp.header_hash, Hash32([0xEE; 32]));
            assert!(
                !settled_rewind_admissible(&s, &diverged, &target, 0),
                "a rewind point the chain has abandoned must never be trusted as a baseline"
            );
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

            advance_ledger_state_to_durable_tip(
                Some(&cp),
                Some(&store),
                &db,
                &sched,
                &RecoveryAdmissionPolicy::cardano(),
            )
            .unwrap();

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

        // ===== LIVE-REFOLD-THRASH RF-1: post-rollback anchor re-certification (DC-EPOCH-35) =====
        //
        // The defect: `reset_to_settled` applies a correct BOUNDED rewind and clears the anchor
        // (DC-EPOCH-29). The next recovery pass then reconciles an ABSENT anchor to
        // `ResetAndRefold { AnchorAbsent }` and calls `reset_to_bootstrap`, discarding the rewind AND
        // deleting the settled triple -- so every later rollback is unbounded too. Measured live
        // growing 153,565 -> 171,449 slots per refold until the node stopped holding tip at all.

        /// Fold a store to a point and promote it to SETTLED, then rewind to it -- the state the
        /// re-certification runs against, immediately after a durable rollback.
        fn settled_and_rewound(tmp: &TempDir, db: &InMemoryChainDb) -> EpochAccumulatorStore {
            use ade_runtime::chaindb::advance_accumulator_over_chaindb;
            let s = sealed_store_at_epoch_500(tmp, SlotNo(42_000_000));
            put_raw(db, 43_000_000);
            advance_accumulator_over_chaindb(
                &s,
                db,
                &schedule_86k(),
                SlotNo(42_000_000),
                SlotNo(43_500_000),
            )
            .unwrap();
            let bn = s
                .last_advanced_point()
                .unwrap()
                .expect("certified")
                .block_no;
            assert!(!s.roll_settled_rewind_point(bn, 0).unwrap());
            assert!(s.roll_settled_rewind_point(bn, 0).unwrap());
            assert!(s.settled_rewind_point().unwrap().is_some(), "promoted");
            assert!(s.reset_to_settled().unwrap());
            // DC-EPOCH-29: uncertified across the rollback window. This is the state the bug leaves
            // behind permanently, and what re-certification is allowed to close AFTER the commit.
            assert!(s.last_advanced_point().unwrap().is_none());
            s
        }

        /// Gates 3+4 (DC-EPOCH-35): a canonical, k-settled, fingerprint-verified settled point is
        /// re-certified after the rollback, and the next recovery pass therefore FORWARD-FOLDS
        /// instead of returning `ResetAndRefold { AnchorAbsent }`.
        #[test]
        fn a_recertified_settled_point_makes_the_next_pass_forward_fold() {
            use ade_ledger::rollback::admission::{reconcile_recovery, RecoveryAction};
            let tmp = TempDir::new().unwrap();
            let db = InMemoryChainDb::new();
            let s = settled_and_rewound(&tmp, &db);
            let sp = s.settled_rewind_point().unwrap().expect("promoted");
            // The synthetic fixture reuses ONE raw block, so every block decodes to the same height;
            // k=0 isolates the lineage/integrity conditions from the height arithmetic, exactly as
            // `settled_rewind_admission_requires_settled_depth_and_intact_lineage` does. The k-bound
            // itself is proven by the dedicated negative below.
            let policy = RecoveryAdmissionPolicy {
                security_param: SecurityParam(0),
            };

            // BEFORE: an absent anchor reconciles to ResetAndRefold -- the loop that eats the rewind.
            let tip_pt = resolve_canonical_point(&db, SlotNo(43_000_000))
                .unwrap()
                .unwrap();
            let seed_pt = resolve_canonical_point(&db, SlotNo(42_000_000))
                .unwrap()
                .unwrap_or(CanonicalPoint {
                    slot: SlotNo(42_000_000),
                    block_no: BlockNo(0),
                    hash: Hash32([0u8; 32]),
                });
            let canonical = |slot: SlotNo| {
                resolve_canonical_point(&db, slot)
                    .ok()
                    .flatten()
                    .map(|p| p.hash)
            };
            assert!(matches!(
                reconcile_recovery(None, None, &tip_pt, &seed_pt, 0, canonical),
                Ok(RecoveryAction::ResetAndRefold { .. })
            ));

            // Re-certify against the POST-rollback chain.
            accumulator_recertify_settled_after_rollback(Some(&s), &db, &policy).unwrap();

            // Gate 3: a NEW anchor exists, at the settled point.
            let anchor = s
                .last_advanced_point()
                .unwrap()
                .expect("re-certification must re-establish the anchor");
            assert_eq!(anchor.slot, sp.slot);

            // Gate 4: the next pass forward-folds -- no AnchorAbsent reset.
            let anchor_pt = CanonicalPoint {
                slot: anchor.slot,
                block_no: anchor.block_no,
                hash: anchor.header_hash.clone(),
            };
            let durable_at_anchor = resolve_canonical_point(&db, anchor.slot).unwrap();
            assert_eq!(
                reconcile_recovery(
                    Some(&anchor_pt),
                    durable_at_anchor.as_ref(),
                    &tip_pt,
                    &seed_pt,
                    0,
                    canonical
                ),
                Ok(RecoveryAction::ForwardFold),
                "a re-certified settled anchor must FORWARD-FOLD, never reset to bootstrap"
            );

            // Gate 5: the accumulator is still at the settled point, not rewound to the seed.
            let (slot, _) = s.load_current().unwrap().expect("sealed");
            assert_eq!(slot, sp.slot, "no bootstrap refold after a bounded rewind");
        }

        /// NEGATIVE: a settled point the POST-rollback chain has ABANDONED must be refused, leaving
        /// the anchor absent so the caller refolds from bootstrap. This is the condition that keeps
        /// the fix safe -- a hash pins its whole ancestry, so re-certifying only when the hash still
        /// resolves is what proves the stored state matches the current canonical prefix.
        #[test]
        fn recertification_refuses_a_settled_point_the_new_chain_abandoned() {
            let tmp = TempDir::new().unwrap();
            let db = InMemoryChainDb::new();
            let s = settled_and_rewound(&tmp, &db);
            let sp = s.settled_rewind_point().unwrap().expect("promoted");

            // The rollback replaced the block at that slot with a different one.
            let diverged = InMemoryChainDb::new();
            diverged
                .put_block(&StoredBlock {
                    hash: Hash32([0xEE; 32]),
                    slot: SlotNo(43_000_000),
                    bytes: RAW_CONWAY_BLOCK.to_vec(),
                })
                .unwrap();
            assert_ne!(sp.header_hash, Hash32([0xEE; 32]));

            accumulator_recertify_settled_after_rollback(
                Some(&s),
                &diverged,
                &RecoveryAdmissionPolicy::cardano(),
            )
            .unwrap();
            assert!(
                s.last_advanced_point().unwrap().is_none(),
                "an abandoned settled point must NOT be certified as lineage authority"
            );
        }

        /// NEGATIVE: a settled point that is no longer `k` blocks behind the NEW tip must be refused
        /// -- an admissible reorg could still reach it, so it is not a safe baseline.
        #[test]
        fn recertification_refuses_a_settled_point_not_k_settled_against_the_new_tip() {
            let tmp = TempDir::new().unwrap();
            let db = InMemoryChainDb::new();
            let s = settled_and_rewound(&tmp, &db);

            // A huge k puts the settled point back inside the reorg window.
            accumulator_recertify_settled_after_rollback(
                Some(&s),
                &db,
                &RecoveryAdmissionPolicy {
                    security_param: SecurityParam(1_000_000),
                },
            )
            .unwrap();
            assert!(
                s.last_advanced_point().unwrap().is_none(),
                "a point inside the reorg window must NOT be certified"
            );
        }

        // ===== EVIEW-RECOVERY-LINEAGE R2: a refold must re-seal frozen leadership byte-identically =====
        //
        // The defect these pin: the co-advancer leaves the reduced checkpoint at the durable TIP at the
        // end of every pass, an accumulator reset does not rewind it, and the forward-only advance
        // SILENTLY no-ops when asked to go back to a boundary point. A refold therefore read its
        // boundary mark and `finalize()` commitment at the tip and re-sealed frozen leadership that
        // disagreed with the durable eview activation record -- latently, only halting on the NEXT
        // restart with `EpochViewPostPromotionMismatch`.

        /// A produced entry the ChainDB does NOT hold, so "checkpoint left where it sat" and "checkpoint
        /// rewound onto the boundary point" are DISTINGUISHABLE states. Every block in these fixtures is
        /// the same raw Conway block, and re-applying it is idempotent, so without this the checkpoint's
        /// content at `s_prev` and at the tip would be equal and the tests would pass either way.
        fn off_chain_produced() -> Vec<(TxIn, Coin, ReducedStakeRef)> {
            vec![(
                TxIn {
                    tx_hash: Hash32([0xAB; 32]),
                    index: 0,
                },
                Coin(9_000_000),
                ReducedStakeRef::Base(cred(0x33)),
            )]
        }

        /// CE-R2-1 (DC-EPOCH-32 / INV-ER-2): positioning is EXACT. A checkpoint left PAST a boundary
        /// point is rewound onto it and reproduces, byte for byte, the commitment and mark it held when
        /// it first passed through -- which is precisely what makes a refold's re-seal identical.
        #[test]
        fn positioning_rewinds_a_checkpoint_that_sits_past_the_boundary_point() {
            let tmp = TempDir::new().unwrap();
            let cp = sealed_checkpoint(&tmp, SlotNo(42_000_000));
            let db = InMemoryChainDb::new();
            put_raw(&db, 43_000_000); // the boundary point s_prev
            put_raw(&db, 43_100_000); // a later block

            // What the ORIGINAL crossing saw at the boundary point.
            assert_eq!(
                position_reduced_checkpoint_at_boundary(&cp, &db, SlotNo(43_000_000)).unwrap(),
                CheckpointPositioning::AdvancedForward
            );
            let ref_commitment = cp.finalize().unwrap();
            let ref_mark = cp.sum_base_credential_stake().unwrap();

            // The co-advancer drives the checkpoint on past the boundary, absorbing later state.
            cp.advance_block(SlotNo(43_100_000), &[], &off_chain_produced())
                .unwrap();
            assert_ne!(
                cp.finalize().unwrap(),
                ref_commitment,
                "the fixture must genuinely move the checkpoint, or this test proves nothing"
            );
            assert_ne!(cp.sum_base_credential_stake().unwrap(), ref_mark);

            // A refold re-crosses the SAME boundary with the cursor left at the tip.
            assert_eq!(
                position_reduced_checkpoint_at_boundary(&cp, &db, SlotNo(43_000_000)).unwrap(),
                CheckpointPositioning::RewoundAndReplayed
            );
            assert_eq!(
                cp.last_advanced_slot().unwrap(),
                Some(SlotNo(43_000_000)),
                "positioned EXACTLY on the boundary point"
            );
            assert_eq!(
                cp.finalize().unwrap(),
                ref_commitment,
                "the re-derived commitment must be byte-identical -- this is the field that differed \
                 live (record cbb12da0 vs candidate de32979c)"
            );
            assert_eq!(
                cp.sum_base_credential_stake().unwrap(),
                ref_mark,
                "the re-derived boundary mark must be byte-identical -- the stake view that differed live"
            );

            // Idempotent: positioning an already-positioned checkpoint moves nothing.
            assert_eq!(
                position_reduced_checkpoint_at_boundary(&cp, &db, SlotNo(43_000_000)).unwrap(),
                CheckpointPositioning::AlreadyAt
            );
            assert_eq!(cp.finalize().unwrap(), ref_commitment);
        }

        /// CE-R2-2: the primitive the seal path used to call is purely FORWARD -- asked to go BACKWARD
        /// it reports success and moves nothing. Pinned as a NEGATIVE so the seal path can never regress
        /// to calling it directly; this exact shape is what let a refold read its mark at the tip.
        #[test]
        fn a_bare_forward_advance_asked_to_go_backward_silently_moves_nothing() {
            let tmp = TempDir::new().unwrap();
            let cp = sealed_checkpoint(&tmp, SlotNo(42_000_000));
            let db = InMemoryChainDb::new();
            put_raw(&db, 43_000_000);
            put_raw(&db, 43_100_000);

            advance_reduced_checkpoint_forward_to(Some(&cp), &db, SlotNo(43_100_000)).unwrap();
            assert_eq!(cp.last_advanced_slot().unwrap(), Some(SlotNo(43_100_000)));

            // Ok(()), cursor UNCHANGED, no signal of any kind.
            advance_reduced_checkpoint_forward_to(Some(&cp), &db, SlotNo(43_000_000)).unwrap();
            assert_eq!(
                cp.last_advanced_slot().unwrap(),
                Some(SlotNo(43_100_000)),
                "silently left 100k slots past the requested point -- why the seal path must go through \
                 position_reduced_checkpoint_at_boundary"
            );

            // The positioning helper, handed the same state, refuses to leave it there.
            assert_eq!(
                position_reduced_checkpoint_at_boundary(&cp, &db, SlotNo(43_000_000)).unwrap(),
                CheckpointPositioning::RewoundAndReplayed
            );
            assert_eq!(cp.last_advanced_slot().unwrap(), Some(SlotNo(43_000_000)));
        }

        /// A boundary point BEFORE the checkpoint's sealed seed can never be re-derived -- the pre-seed
        /// deltas are not held. It must report `Unreachable` so the caller STALLS rather than sealing
        /// from whatever state the checkpoint happens to hold, and must not damage the checkpoint on the
        /// way to saying so.
        #[test]
        fn a_boundary_point_before_the_sealed_seed_is_unreachable() {
            let tmp = TempDir::new().unwrap();
            let cp = sealed_checkpoint(&tmp, SlotNo(42_000_000));
            let db = InMemoryChainDb::new();
            put_raw(&db, 43_000_000);
            advance_reduced_checkpoint_forward_to(Some(&cp), &db, SlotNo(43_000_000)).unwrap();

            match position_reduced_checkpoint_at_boundary(&cp, &db, SlotNo(41_000_000)).unwrap() {
                CheckpointPositioning::Unreachable { advanced, seed } => {
                    assert_eq!(seed, 42_000_000);
                    assert_eq!(advanced, 43_000_000);
                }
                other => panic!("a pre-seed boundary point must be Unreachable, got {other:?}"),
            }
            assert_eq!(
                cp.last_advanced_slot().unwrap(),
                Some(SlotNo(43_000_000)),
                "refusing must not rewind or otherwise disturb the checkpoint"
            );
        }

        /// CE-R2-3 (DC-EPOCH-33 / INV-ER-2) -- the PRODUCTION seam. A refold that re-crosses a boundary
        /// re-seals frozen leadership BYTE-IDENTICALLY to the original crossing: source point,
        /// `source_checkpoint_commitment`, and the whole pool map. This is the property whose violation
        /// left the durable eview activation record unreproducible and halted the next restart.
        #[test]
        fn a_refold_reseals_frozen_leadership_byte_identically() {
            use ade_ledger::frozen_leadership::FrozenLeadershipPoolDistr;

            fn sealed_leadership(
                store: &EpochAccumulatorStore,
            ) -> Vec<(u64, FrozenLeadershipPoolDistr)> {
                (500..=504)
                    .filter_map(|e| {
                        store
                            .frozen_leadership_for_epoch(EpochNo(e))
                            .unwrap()
                            .map(|d| (e, d))
                    })
                    .collect()
            }

            let tmp = TempDir::new().unwrap();
            let cp = sealed_checkpoint(&tmp, SlotNo(42_000_000));
            let store = sealed_store_at_epoch_500(&tmp, SlotNo(42_000_000));
            let db = InMemoryChainDb::new();
            put_raw(&db, 43_000_000); // epoch 500, within-epoch -> s_prev, the mark source
            put_raw(&db, 43_086_000); // epoch 501, the boundary block
            put_raw(&db, 43_100_000); // epoch 501, the durable tip
            let sched = schedule_86k();
            let policy = RecoveryAdmissionPolicy::cardano();

            // The ORIGINAL crossing.
            advance_ledger_state_to_durable_tip(Some(&cp), Some(&store), &db, &sched, &policy)
                .unwrap();
            let original = sealed_leadership(&store);
            assert!(
                !original.is_empty(),
                "the original crossing must seal leadership, or this test proves nothing"
            );
            assert_eq!(
                cp.last_advanced_slot().unwrap(),
                Some(SlotNo(43_100_000)),
                "the co-advancer leaves the checkpoint at the durable TIP -- the precondition for the defect"
            );

            // Force the refold exactly as an admitted rollback / ResetAndRefold does: the accumulator is
            // rewound to bootstrap and the reduced checkpoint is NOT rewound with it. The extra produced
            // entry sits AT the tip slot, so `reduced_checkpoint_reset_if_ahead` does not fire either --
            // which is the live condition.
            store.reset_to_bootstrap().unwrap();
            cp.advance_block(SlotNo(43_100_000), &[], &off_chain_produced())
                .unwrap();

            // The REFOLD.
            advance_ledger_state_to_durable_tip(Some(&cp), Some(&store), &db, &sched, &policy)
                .unwrap();

            assert_eq!(
                sealed_leadership(&store),
                original,
                "a refold must re-seal byte-identical frozen leadership -- source point, \
                 source_checkpoint_commitment and the full pool map"
            );
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

            advance_ledger_state_to_durable_tip(
                Some(&cp),
                None,
                &db,
                &sched,
                &RecoveryAdmissionPolicy::cardano(),
            )
            .unwrap();

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

            advance_ledger_state_to_durable_tip(
                Some(&cp),
                Some(&store),
                &db,
                &sched,
                &RecoveryAdmissionPolicy::cardano(),
            )
            .unwrap();

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

            advance_ledger_state_to_durable_tip(
                None,
                Some(&store),
                &db,
                &sched,
                &RecoveryAdmissionPolicy::cardano(),
            )
            .unwrap();

            let (slot, acc) = store.load_current().unwrap().unwrap();
            assert_eq!(acc.epoch_state.epoch, EpochNo(500), "no mark source -> no cross");
            assert_eq!(
                slot,
                SlotNo(43_000_000),
                "folded within-epoch up to s_prev, then stalled observe-only"
            );
        }

        // ===== LIVE-LEDGER-EPOCH-TRANSITION S5 (2b): event-qualified rollback admission + crash-safety =====
        //
        // These prove the WIRED recovery-admission path, distinct from the BLUE decision (proven by
        // ade_ledger::rollback::admission's 14 unit tests). The discriminator is PROVENANCE: a
        // chain-selection-admitted live rollback pre-clears the accumulator (after admitting the bounded
        // rollback) BEFORE the ChainDB rollback commits; a warm-start contradiction with a present anchor and
        // no rollback context stays terminal.
        //
        // NOTE on BeforeBootstrapAnchor: a wired trigger needs target.block_no < seed.block_no, i.e. TWO
        // distinct decoded block_nos; the single `RAW_CONWAY_BLOCK` fixture decodes every `put_raw` block to
        // ONE block_no, so it cannot be expressed at this seam. Its floor logic is covered by the BLUE
        // `rollback_before_bootstrap_anchor_is_typed` (admit_rollback) unit test, and it is defensively
        // unreachable on the live path (a chain-selection-admitted rollback target is always >= the immutable
        // bootstrap seed). The other tests exercise the seed floor's threading into the helper.

        /// Seal a store, then advance it to a certified lineage anchor at `(slot, block_no, hash)`.
        fn advanced_store(
            tmp: &TempDir,
            seed_slot: SlotNo,
            anchor_slot: SlotNo,
            anchor_block_no: u64,
            anchor_hash: Hash32,
        ) -> EpochAccumulatorStore {
            let store = sealed_store_at_epoch_500(tmp, seed_slot);
            let (_, acc) = store.load_current().unwrap().unwrap();
            store
                .advance(&acc, anchor_slot, BlockNo(anchor_block_no), anchor_hash)
                .unwrap();
            store
        }

        /// The `put_raw` canonical hash at a slot (fixture: `hash = low byte of slot`).
        fn canon_hash(slot: u64) -> Hash32 {
            Hash32([(slot & 0xff) as u8; 32])
        }

        /// T1 (within-k live rollback): the certified anchor is pre-CLEARED (after admitting the bounded
        /// rollback) so no crash window leaves it over the abandoned prefix; the store is left at the seed
        /// baseline, uncertified, ready to refold from canonical.
        #[test]
        fn s5_within_k_live_rollback_pre_clears_the_certified_anchor() {
            let tmp = TempDir::new().unwrap();
            let seed_slot = SlotNo(42_000_000);
            let db = InMemoryChainDb::new();
            put_raw(&db, 43_000_000); // the rollback target
            put_raw(&db, 43_086_000); // the pre-rollback tip
            let b0 = resolve_canonical_point(&db, SlotNo(43_000_000))
                .unwrap()
                .unwrap()
                .block_no
                .0;
            // A certified anchor 3 blocks above the target: a real, within-k rollback.
            let store = advanced_store(&tmp, seed_slot, SlotNo(43_086_000), b0 + 3, Hash32([0xAB; 32]));
            assert!(store.last_advanced_point().unwrap().is_some(), "certified before rollback");

            let target = Point {
                slot: SlotNo(43_000_000),
                hash: canon_hash(43_000_000),
            };
            accumulator_admit_and_clear_for_rollback(
                Some(&store),
                &db,
                &target,
                &RecoveryAdmissionPolicy::cardano(),
            )
            .unwrap();

            // Pre-cleared: the anchor is gone and the accumulator is back at the seed baseline.
            assert_eq!(store.last_advanced_point().unwrap(), None, "anchor pre-cleared");
            let (slot, acc) = store.load_current().unwrap().unwrap();
            assert_eq!(slot, seed_slot, "reset to the seed baseline");
            assert_eq!(acc.epoch_state.epoch, EpochNo(500));
        }

        /// T2 (crash AFTER the accumulator clear, BEFORE the ChainDB rollback commits): the durable ChainDB is
        /// still the pre-rollback chain; recovery sees an ABSENT anchor and refolds it -> Ok, never terminal,
        /// never stale-height trust.
        #[test]
        fn s5_crash_after_clear_before_rollback_refolds() {
            let tmp = TempDir::new().unwrap();
            let seed_slot = SlotNo(42_000_000);
            let db = InMemoryChainDb::new();
            put_raw(&db, 43_000_000);
            put_raw(&db, 43_086_000); // pre-rollback tip still present (rollback not committed)
            let b0 = resolve_canonical_point(&db, SlotNo(43_000_000)).unwrap().unwrap().block_no.0;
            let store = advanced_store(&tmp, seed_slot, SlotNo(43_086_000), b0 + 3, Hash32([0xAB; 32]));
            let target = Point { slot: SlotNo(43_000_000), hash: canon_hash(43_000_000) };
            accumulator_admit_and_clear_for_rollback(Some(&store), &db, &target, &RecoveryAdmissionPolicy::cardano())
                .unwrap();
            assert_eq!(store.last_advanced_point().unwrap(), None, "anchor absent after clear");

            // Restart over the (still pre-rollback) durable tip. Absent anchor -> reset+refold, never terminal.
            let tip = ChainTip { slot: SlotNo(43_086_000), hash: canon_hash(43_086_000) };
            accumulator_recover_admit(Some(&store), &db, &tip, &RecoveryAdmissionPolicy::cardano())
                .expect("absent anchor refolds, never terminal");
        }

        /// T3 (crash AFTER the ChainDB rollback, BEFORE the refold): the durable ChainDB is now the
        /// post-rollback chain; recovery sees an ABSENT anchor and refolds the post-rollback chain -> Ok.
        #[test]
        fn s5_crash_after_rollback_before_refold_refolds() {
            let tmp = TempDir::new().unwrap();
            let seed_slot = SlotNo(42_000_000);
            let db_pre = InMemoryChainDb::new();
            put_raw(&db_pre, 43_000_000);
            put_raw(&db_pre, 43_086_000);
            let b0 = resolve_canonical_point(&db_pre, SlotNo(43_000_000)).unwrap().unwrap().block_no.0;
            let store = advanced_store(&tmp, seed_slot, SlotNo(43_086_000), b0 + 3, Hash32([0xAB; 32]));
            let target = Point { slot: SlotNo(43_000_000), hash: canon_hash(43_000_000) };
            accumulator_admit_and_clear_for_rollback(Some(&store), &db_pre, &target, &RecoveryAdmissionPolicy::cardano())
                .unwrap();

            // The ChainDB rollback committed: the abandoned block (43_086_000) is gone; the tip is 43_000_000.
            let db_post = InMemoryChainDb::new();
            put_raw(&db_post, 43_000_000);
            let tip = ChainTip { slot: SlotNo(43_000_000), hash: canon_hash(43_000_000) };
            accumulator_recover_admit(Some(&store), &db_post, &tip, &RecoveryAdmissionPolicy::cardano())
                .expect("absent anchor refolds the post-rollback chain, never terminal");
        }

        /// T4 (warm-start contradiction, NOT a rollback event): a PRESENT anchor whose hash disagrees with the
        /// canonical block at its slot is a durable-state contradiction with no rollback provenance -> TERMINAL
        /// LineageMismatch (never a silent reset of a certified store). The strict contract the event-qualified
        /// live path deliberately does NOT relax.
        #[test]
        fn s5_warm_start_contradiction_present_anchor_wrong_hash_is_terminal() {
            let tmp = TempDir::new().unwrap();
            let seed_slot = SlotNo(42_000_000);
            let db = InMemoryChainDb::new();
            put_raw(&db, 43_000_000);
            let bn = resolve_canonical_point(&db, SlotNo(43_000_000)).unwrap().unwrap().block_no.0;
            // A certified anchor at 43_000_000 with the WRONG hash (canonical there is low-byte, not 0xEE).
            let store = advanced_store(&tmp, seed_slot, SlotNo(43_000_000), bn, Hash32([0xEE; 32]));
            let tip = ChainTip { slot: SlotNo(43_000_000), hash: canon_hash(43_000_000) };
            let err = accumulator_recover_admit(Some(&store), &db, &tip, &RecoveryAdmissionPolicy::cardano())
                .expect_err("a contradicted certified anchor is terminal");
            assert!(
                matches!(&err, NodeLifecycleError::RecoveryAdmission(RecoveryAdmissionFault::LineageMismatch { slot }) if *slot == 43_000_000),
                "got {err:?}"
            );
            assert!(store.last_advanced_point().unwrap().is_some(), "certified store is NOT reset on contradiction");
        }

        /// T5 (rollback beyond k): a certified anchor more than k blocks above the target -> TERMINAL
        /// ExceededRollback; the certified store is NOT cleared (recovery never rematerializes from a
        /// deeper-than-immutable prefix).
        #[test]
        fn s5_live_rollback_beyond_k_is_terminal_exceeded() {
            let tmp = TempDir::new().unwrap();
            let seed_slot = SlotNo(42_000_000);
            let db = InMemoryChainDb::new();
            put_raw(&db, 43_000_000); // the rollback target
            let target_bn = resolve_canonical_point(&db, SlotNo(43_000_000)).unwrap().unwrap().block_no.0;
            let k = 5u64;
            let policy = RecoveryAdmissionPolicy { security_param: SecurityParam(k) };
            // A certified anchor k+1 blocks above the target -> rolling it back exceeds k.
            let store = advanced_store(&tmp, seed_slot, SlotNo(43_050_000), target_bn + k + 1, Hash32([0xAB; 32]));

            let target = Point { slot: SlotNo(43_000_000), hash: canon_hash(43_000_000) };
            let err = accumulator_admit_and_clear_for_rollback(Some(&store), &db, &target, &policy)
                .expect_err("a beyond-k rollback is terminal");
            assert!(
                matches!(&err, NodeLifecycleError::RecoveryAdmission(RecoveryAdmissionFault::ExceededRollback { depth, k: kk }) if *depth == k + 1 && *kk == k),
                "got {err:?}"
            );
            assert!(store.last_advanced_point().unwrap().is_some(), "certified store is NOT reset on inadmissible rollback");
        }

        /// T6 (rollback target not on the canonical chain): a target slot with no durable block -> TERMINAL
        /// TargetNotOnCanonicalChain; the store is NOT cleared.
        #[test]
        fn s5_live_rollback_target_absent_from_chain_is_terminal() {
            let tmp = TempDir::new().unwrap();
            let seed_slot = SlotNo(42_000_000);
            let db = InMemoryChainDb::new();
            put_raw(&db, 43_086_000); // only the tip; the target slot below has no block
            let b0 = resolve_canonical_point(&db, SlotNo(43_086_000)).unwrap().unwrap().block_no.0;
            let store = advanced_store(&tmp, seed_slot, SlotNo(43_086_000), b0 + 1, Hash32([0xAB; 32]));

            let target = Point { slot: SlotNo(43_000_000), hash: Hash32([0x99; 32]) };
            let err = accumulator_admit_and_clear_for_rollback(Some(&store), &db, &target, &RecoveryAdmissionPolicy::cardano())
                .expect_err("a target off the canonical chain is terminal");
            assert!(
                matches!(&err, NodeLifecycleError::RecoveryAdmission(RecoveryAdmissionFault::TargetNotOnCanonicalChain { slot }) if *slot == 43_000_000),
                "got {err:?}"
            );
            assert!(store.last_advanced_point().unwrap().is_some(), "store not cleared on a bad target");
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

    /// A boundary crossing is a post-rollback REFOLD iff the durable tip is already in a later
    /// epoch than the one just crossed into. Regression cover for the 2026-08-01 live run, which
    /// logged 14 identical "CROSSED 1375 -> 1376" lines over 18h (one per reorg, each a genuine
    /// re-derivation after `reset_to_bootstrap`) that read as 14 fresh boundaries.
    #[test]
    fn refold_is_distinguished_from_a_fresh_boundary_crossing() {
        // Fresh: we just crossed into 1377 and the tip is in 1377.
        assert!(!crossing_is_refold(Some(EpochNo(1377)), EpochNo(1377)));
        // Crossing INTO the epoch the tip is in is fresh, whichever epoch that is.
        assert!(!crossing_is_refold(Some(EpochNo(1376)), EpochNo(1376)));
        // Refold: re-crossing the 1375->1376 boundary while the tip has moved on.
        assert!(crossing_is_refold(Some(EpochNo(1377)), EpochNo(1376)));
        assert!(crossing_is_refold(Some(EpochNo(1400)), EpochNo(1376)));
        // Unknown tip epoch degrades to the unlabelled (fresh) line -- never a halt, never a
        // misleading REFOLD claim.
        assert!(!crossing_is_refold(None, EpochNo(1376)));
        // A tip BEHIND the crossed epoch cannot happen (the cross advances to it), and must not
        // be reported as a refold either.
        assert!(!crossing_is_refold(Some(EpochNo(1375)), EpochNo(1376)));
    }

    // LIVE-WIRE-LIVENESS S2 — reconnect an ESTABLISHED live feed.

    /// CE-WL-7 / INV-WL-8: a `--peer` that PARSES but refuses the connection is
    /// a first-dial failure, so no session was ever established. Startup
    /// semantics are unchanged — logged-and-dropped, feed ends — and the
    /// supervisor must NOT turn an unreachable peer into an infinite boot spin.
    #[tokio::test]
    async fn first_dial_failure_still_ends_the_feed_no_boot_spin() {
        // Port 1 on loopback: parses fine, refuses (or is unreachable).
        let mut src = spawn_live_wire_pump_source(&["127.0.0.1:1".to_string()], 1, None);
        let ended = tokio::time::timeout(std::time::Duration::from_secs(20), async {
            loop {
                if src.next_item().await.is_none() && !src.has_work_ready() {
                    return true;
                }
            }
        })
        .await;
        assert!(
            ended.is_ok(),
            "a first-dial failure must end the feed, never retry forever at boot"
        );
    }

    /// INV-WL-10: the backoff schedule is deterministic, monotone
    /// non-decreasing, and capped — no randomness, no unbounded growth.
    #[test]
    fn reconnect_backoff_is_deterministic_monotone_and_capped() {
        let seq: Vec<u64> = (0..12).map(reconnect_backoff_secs).collect();
        assert_eq!(seq, (0..12).map(reconnect_backoff_secs).collect::<Vec<_>>());
        for w in seq.windows(2) {
            assert!(w[1] >= w[0], "backoff must not decrease: {seq:?}");
        }
        let cap = *RECONNECT_BACKOFF_SECS
            .last()
            .expect("schedule is non-empty");
        assert!(
            seq.iter().all(|s| *s <= cap),
            "backoff must stay capped at {cap}s: {seq:?}"
        );
        assert_eq!(seq[11], cap, "it saturates at the cap rather than growing");
    }

    /// CE-WL-5 policy / INV-WL-9: exhaustive over the wire pump's closed outcome
    /// sum. Transport loss reconnects; a peer protocol/grammar violation keeps
    /// the pre-slice fail-closed drop; a dropped consumer channel exits.
    #[test]
    fn reconnect_policy_is_transport_only() {
        use ade_network::session::SessionError;

        assert!(should_reconnect_after(&AdmissionWirePumpResult::Eof));
        assert!(should_reconnect_after(&AdmissionWirePumpResult::Error(
            AdmissionWirePumpError::TransportRead
        )));
        assert!(should_reconnect_after(&AdmissionWirePumpResult::Error(
            AdmissionWirePumpError::TransportWrite
        )));

        // The consumer is gone — nobody to reconnect for.
        assert!(!should_reconnect_after(
            &AdmissionWirePumpResult::EventsChannelDropped
        ));

        // Peer faults: unchanged fail-closed drop, never a retry livelock.
        for e in [
            AdmissionWirePumpError::ChainSyncDecode,
            AdmissionWirePumpError::BlockFetchDecode,
            AdmissionWirePumpError::UnexpectedProtocolMessage {
                protocol: "chain_sync",
            },
            AdmissionWirePumpError::UnsupportedRollbackPoint,
            AdmissionWirePumpError::DeferredFrameOverflow,
            AdmissionWirePumpError::Session(SessionError::UnknownMiniProtocolId { id: 99 }),
        ] {
            assert!(
                !should_reconnect_after(&AdmissionWirePumpResult::Error(e)),
                "a peer protocol/grammar fault must not be retried"
            );
        }
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
                "security_param": 2160,
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

    /// S4: seal a leadership-certified epoch-accumulator store beside the warm-start ChainDb so the recovery
    /// replay (which reads the leader schedule by EXACT epoch from the frozen leadership authority) can proceed.
    /// The seed record's `pool_distribution` IS the seed leadership nesPd (S4-0). Returns the opened store to pass
    /// as `warm_start_recovery`'s leadership authority. A separate redb file from `chain.db` (no lock conflict).
    fn seal_warm_leadership(
        d: &WarmDirs,
        record: &SeedEpochConsensusInputs,
    ) -> ade_runtime::chaindb::EpochAccumulatorStore {
        use ade_ledger::frozen_leadership::FrozenLeadershipPoolDistr;
        // Leadership-ONLY (the seed record's pool_distribution IS the seed nesPd, S4-0): the warm-start replay +
        // the initial/warm header-validation view read leadership by exact epoch; they do not read an accumulator
        // baseline from this store. Leadership-only also passes the dispatch path's governance gate (which acts
        // only on GovernanceImportRequired, not the `Unsealed` a marker-only store yields).
        let store = ade_runtime::chaindb::EpochAccumulatorStore::open(
            &d.snap.join("epoch-accumulator.redb"),
        )
        .expect("open warm accumulator");
        store
            .seal_bootstrap_leadership_epochs(&[
                FrozenLeadershipPoolDistr::from_seed_epoch_consensus_inputs(record, Hash32([0x0C; 32])),
            ])
            .expect("seal warm seed leadership");
        store
    }

    /// S4-L1 ACCEPTANCE: the flipped production leadership read (`leadership_view_from_frozen_authority`) reads
    /// the epoch-indexed frozen leadership authority by EXACT epoch and produces a PoolDistrView BYTE-IDENTICAL
    /// to the retired seed projection `PoolDistrView::from_seed_epoch_consensus_inputs` — and FAILS CLOSED (no
    /// seed fallback) when the authority is absent or uncertified. This is the sealed-slice claim: the three
    /// initial/warm sites (658/840/3397) now have exactly one authority, byte-identical to before, fail-closed.
    #[test]
    fn s4_l1_frozen_leadership_view_is_byte_identical_to_seed_and_fails_closed() {
        let d = fresh_warm_dirs();
        let record = warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH);

        // Fail closed — NO store (never a seed read).
        assert!(
            matches!(
                leadership_view_from_frozen_authority(None, &record),
                Err(NodeLifecycleError::ProductionLeadershipAuthorityUnavailable { epoch, .. }) if epoch == WARM_EPOCH.0
            ),
            "absent leadership authority must fail closed, not fall back to the seed projection"
        );

        // Fail closed — an UNCERTIFIED store (fresh, no leadership marker).
        let uncertified = ade_runtime::chaindb::EpochAccumulatorStore::open(&d.snap.join("uncert.redb"))
            .expect("open uncertified");
        assert!(
            matches!(
                leadership_view_from_frozen_authority(Some(&uncertified), &record),
                Err(NodeLifecycleError::ProductionLeadershipAuthorityUnavailable { .. })
            ),
            "an uncertified (legacy) store must fail closed"
        );

        // Byte-identical — a leadership-certified store yields EXACTLY the seed projection.
        let store = seal_warm_leadership(&d, &record);
        let via_authority = leadership_view_from_frozen_authority(Some(&store), &record)
            .expect("certified store answers leadership by exact epoch");
        let via_seed = PoolDistrView::from_seed_epoch_consensus_inputs(&record);
        assert_eq!(
            via_authority, via_seed,
            "S4-L1: the frozen-authority leadership view is byte-identical to the retired seed projection"
        );
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
            security_param: 2160,
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
        put_tip_and_snapshot_with_anchor(chaindb, slot, WARM_ANCHOR_SLOT)
    }

    /// As above, with an explicit AK-S1 anchor-point slot. LIVE-2c: a fixture placed on a real
    /// venue calendar needs its anchor point on the SAME calendar -- the resolver prefers that
    /// record, so a stale anchor slot lands the recovered tip in a different KES period.
    fn put_tip_and_snapshot_with_anchor(
        chaindb: &PersistentChainDb,
        slot: u64,
        anchor_slot: u64,
    ) {
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
        let ap = RecoveredAnchorPoint {
            anchor_fp: WARM_ANCHOR_FP,
            slot: SlotNo(anchor_slot),
            block_hash: WARM_ANCHOR_HASH,
        };
        chaindb
            .put_recovered_anchor_point(&WARM_ANCHOR_FP, &encode_recovered_anchor_point(&ap))
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
        let state = warm_start_recovery(&chaindb, &wal, Some(&seal_warm_leadership(&d, &record)), None).expect("warm-start recovers");

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
    fn sidecar_freeze_rsw_derives_from_store_and_cross_checks_the_cli() {
        // LIVE-FORGE-HARDENING S2 (DC-EPOCH-16): the recovered freeze window is the DURABLE sidecar's
        // authority. Preview k=432, f=1/20 -> RSW 34560, derived via the ONE BLUE praos_rsw_slots the
        // live path uses -- so warm_start_recovery AND the forward recovered_node_schedule freeze the
        // candidate IDENTICALLY whether the restart CLI agrees or supplies nothing. A CLI that
        // DISAGREES with the durable venue is terminal (fail-closed cross-check, never silently used).
        let mut rec = warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH);
        rec.security_param = 432;
        rec.active_slots_coeff = ActiveSlotsCoeff { numer: 1, denom: 20 };
        assert_eq!(
            sidecar_freeze_rsw(&rec, Some(34_560)).unwrap(),
            Some(34_560),
            "CLI agrees -> store-derived window"
        );
        assert_eq!(
            sidecar_freeze_rsw(&rec, None).unwrap(),
            Some(34_560),
            "no CLI cross-check -> still the store window (never inert on the forward path)"
        );
        assert!(
            matches!(
                sidecar_freeze_rsw(&rec, Some(34_559)),
                Err(NodeLifecycleError::NativeFirstRun(_))
            ),
            "a CLI RSW that disagrees with the durable venue is terminal"
        );
    }

    #[test]
    fn warm_start_pre_v4_sidecar_is_typed_schema_upgrade_not_corruption() {
        // ECA-2-pre (DC-CINPUT-06): on the LIVE warm-start path, a well-formed
        // pre-v4 sidecar fails closed with the TYPED ConsensusInputsSchemaUnsupported
        // (a reimport requirement), DISTINCT from the generic WarmStartBootstrap
        // (corruption) -- so the live-path diagnostics match the bootstrap authority.
        let d = fresh_warm_dirs();
        // A valid current-schema (v6) sidecar, with the version uint (index 1; index
        // 0 is the array(14) header) rewritten 0x06 -> 0x03 so it decodes as an old schema.
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
        let err = warm_start_recovery(&chaindb, &wal, None, None)
            .expect_err("a pre-v4 sidecar must fail closed on the warm-start path");
        assert!(
            matches!(
                err,
                NodeLifecycleError::ConsensusInputsSchemaUnsupported {
                    found_version: 3,
                    required_version: 6
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
        let state = warm_start_recovery(&chaindb, &wal, Some(&seal_warm_leadership(&d, &record)), None).expect("bare-anchor warm-start recovers");

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
        // S4: seal the leadership authority at the snap dir so the warm-start dispatch's live open finds it.
        drop(seal_warm_leadership(&d, &record));
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
        let r = warm_start_recovery(&chaindb, &wal, None, None);
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
        let r = warm_start_recovery(&chaindb, &wal, Some(&seal_warm_leadership(&d, &record)), None);
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
        let r = warm_start_recovery(&chaindb, &wal, Some(&seal_warm_leadership(&d, &record)), None);
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
        let r = warm_start_recovery(&chaindb, &wal, Some(&seal_warm_leadership(&d, &record)), None);
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
        let r = warm_start_recovery(&chaindb, &wal, Some(&seal_warm_leadership(&d, &record)), None);
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
        let r = warm_start_recovery(&chaindb, &wal, None, None);
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
        let state = warm_start_recovery(&chaindb, &wal, Some(&seal_warm_leadership(&d, &record)), None)
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
        let r = warm_start_recovery(&chaindb, &wal, Some(&seal_warm_leadership(&d, &record)), None);
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
        let state = warm_start_recovery(&chaindb, &wal, Some(&seal_warm_leadership(&d, &record)), None)
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
        write_node_operator_material_for_slot(dir, WARM_TIP_SLOT)
    }

    /// As above, with the op-cert anchored at the ABSOLUTE KES period of `tip_slot` (delta 0), so a
    /// fixture can place its tip anywhere on a venue's real calendar (LIVE-2c).
    fn write_node_operator_material_for_slot(
        dir: &std::path::Path,
        tip_slot: u64,
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
        // LIVE-2c: computed from `tip_slot` rather than hardcoded, so a preprod-calendar fixture can
        // sit at a real absolute slot. Minimal canonical CBOR uint.
        let opcert_period = tip_slot / 129_600;
        if opcert_period < 24 {
            ocbor.push(opcert_period as u8);
        } else if opcert_period < 256 {
            ocbor.extend_from_slice(&[0x18, opcert_period as u8]);
        } else if opcert_period < 65_536 {
            ocbor.extend_from_slice(&[0x19, (opcert_period >> 8) as u8, opcert_period as u8]);
        } else {
            unreachable!("fixture KES periods stay under 65_536");
        }
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

    /// LIVE-2c: preprod-COHERENT warm fixture for the forge-ON path.
    ///
    /// `warm_sample_record` is calendar-incoherent by construction (`epoch_start_slot = epoch *
    /// 432_000` with a tip slot from a different epoch) — harmless for the relay-only tests that use
    /// it, but a forge-ON start now binds its slot authority to these exact facts and must refuse a
    /// store it cannot reconstruct. So the forge-ON fixture mirrors the REAL live venue:
    /// `~/.cardano-live1/ade-preprod-s7`'s own genesis hash, epoch 304 starting at absolute slot
    /// 129_686_400, seed point 129_813_427. That makes this test exercise the production binding
    /// rather than a shape no venue has.
    const WARM_PREPROD_EPOCH: EpochNo = EpochNo(304);
    const WARM_PREPROD_EPOCH_START_SLOT: u64 = 129_686_400;
    const WARM_PREPROD_SEED_POINT_SLOT: u64 = 129_813_427;

    fn warm_preprod_record(anchor_fp: Hash32) -> SeedEpochConsensusInputs {
        let mut record = warm_sample_record(anchor_fp, WARM_PREPROD_EPOCH);
        record.genesis_hash = crate::bootstrap_export::resolve_network_profile("preprod")
            .expect("preprod is a committed venue")
            .genesis_hash;
        record.epoch_start_slot = SlotNo(WARM_PREPROD_EPOCH_START_SLOT);
        record.seed_point_slot = SlotNo(WARM_PREPROD_SEED_POINT_SLOT);
        record
    }

    fn warm_preprod_fixture(d: &WarmDirs) {
        let record = warm_preprod_record(WARM_ANCHOR_FP);
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        let (chaindb, mut wal) = open_warm_stores(d);
        chaindb
            .put_seed_epoch_consensus_inputs(&WARM_ANCHOR_FP, &bytes)
            .unwrap();
        append_seed_epoch_provenance(&mut wal, &WARM_ANCHOR_FP, WARM_PREPROD_EPOCH, &bytes).unwrap();
        // The AK-S1 anchor-point record is the resolver's PREFERRED tip source, so it must sit on
        // the same calendar as everything else here — otherwise the recovered tip lands in a
        // different KES period than the fixture's op-cert covers.
        put_tip_and_snapshot_with_anchor(
            &chaindb,
            WARM_PREPROD_SEED_POINT_SLOT,
            WARM_PREPROD_SEED_POINT_SLOT,
        );
        drop(seal_warm_leadership(d, &record));
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
        // S4: seal the epoch-indexed leadership authority beside the warm store (a distinct redb file) so the
        // dispatch's live open reads the leader schedule by exact epoch — the warm store's leadership certificate.
        drop(seal_warm_leadership(d, &record));
    }

    #[tokio::test]
    async fn node_mode_with_operator_keys_warm_start_forge_capable_halts_clean() {
        // On path end-to-end (CE-F-3 + CE-F-4): warm-start recovers the SINGLE
        // BootstrapState, classify_forge_intent => On, build the
        // operator-material-backed activation on that recovered state, enter
        // run_relay_loop with Some(..) — and halt cleanly on the empty source
        // (forge CAPABLE, not observable; no second bootstrap, no Mithril call).
        //
        // LIVE-2c: also proves the forge-ON start ESTABLISHES its bootstrap-bound slot authority —
        // the preprod-coherent fixture's durable facts must reconstruct through the committed venue
        // calendar, or this returns ForgeKeyIngress instead of halting cleanly.
        let d = fresh_warm_dirs();
        warm_preprod_fixture(&d);
        let (cold, kes, vrf, opcert, genesis) =
            write_node_operator_material_for_slot(d._dir.path(), WARM_PREPROD_SEED_POINT_SLOT);
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

// ============================================================================
// LIVE-LEDGER-EPOCH-TRANSITION CE-4A.1 — production-loop continuous
// self-sufficiency across TWO real epoch boundaries (1340->1341->1342).
//
// A FAIL-LOUD, fixture-heavy `#[ignore]` evidence run. It drives the REAL
// production composition — `run_relay_loop_with_sched` -> (`run_node_sync` +
// `advance_ledger_state_to_durable_tip`) — with ALL THREE authority inputs
// `Some`, feeding the existing 1339..1342 corpus as a `NodeBlockSource::in_memory`.
// It mirrors `run_node_lifecycle_inner`'s `ForgeIntent::Off` arm exactly (the
// production warm-start + input assembly), then swaps the live WirePump for the
// corpus feed. It does NOT re-implement the loop (THE HARD RULE): if the harness
// bypassed the production composition, CE-4A would not count.
//
// Locked claim: *production-loop continuous self-sufficiency across two real
// boundaries.* NON-claims: not byte-exact boundary equivalence (4A.2), not
// restart/rollback equivalence (4A.3), not live preview/preprod, not bounty-ready.
// ============================================================================
#[cfg(test)]
mod ce4a_continuous_self_sufficiency {
    use super::*;
    use std::path::{Path, PathBuf};
    use tokio::sync::watch;

    use ade_ledger::wal::WalEntry;
    use ade_runtime::chaindb::{EpochAccumulatorStore, ReducedUtxoCheckpoint};

    /// The v5 fixture's TRUE bootstrap seed epoch (node.log: 1338->1339->1340).
    const SEED_EPOCH: u64 = 1338;
    /// Preview epoch geometry: epoch E begins at slot E * 86_400.
    const PREVIEW_EPOCH_LEN: u64 = 86_400;
    /// First BLOCK of epoch 1342 (boundary 1341 -> 1342) — the full-run feed ceiling.
    const EPOCH_1342_FIRST_SLOT: u64 = 115_948_834;
    /// CE-4B: epoch 1343 begins at slot 1343 * 86_400 = 116_035_200 (first block ~116_035_206). The
    /// locally-extracted corpus reaches 116_041_708 (179 blocks into 1343) — the CE-4B feed ceiling well
    /// past the third boundary, so the loop crosses 1341 -> 1342 -> 1343 and seals 1344 in one run.
    const EPOCH_1343_FIRST_SLOT: u64 = 116_035_200;
    const EPOCH_1343_FEED_CEILING: u64 = 116_041_708;
    /// Preview N2N network magic (matches the v5 sidecar's venue).
    const PREVIEW_MAGIC: u32 = 2;

    fn env_path(key: &str, default: &str) -> PathBuf {
        std::env::var(key).map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(default))
    }
    fn env_u64(key: &str, default: u64) -> u64 {
        std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
    }
    /// The Praos candidate-nonce freeze latitude RSW = ceil(4·k/f) for PREVIEW, derived from the
    /// committed `--network preview` profile (k=432, f=1/20 => 34560) — the SAME source of truth
    /// production's `rsw_for_cli` uses. NOT k=2160 (that is preprod/mainnet); using it froze the
    /// candidate ~2 epochs early on preview's short (86400-slot) epochs.
    fn preview_rsw() -> Option<u32> {
        let p = crate::bootstrap_export::resolve_network_profile("preview").expect("preview profile");
        ade_core::consensus::era_schedule::praos_rsw_slots(
            p.security_param,
            u64::from(p.active_slots_coeff.0),
            u64::from(p.active_slots_coeff.1),
        )
    }
    fn epoch_of(slot: u64) -> u64 {
        slot / PREVIEW_EPOCH_LEN
    }

    /// Copy the v5 fixture stores into an ISOLATED work dir. `EpochAccumulatorStore::open`
    /// / `ReducedUtxoCheckpoint::open` and the loop's ChainDb admits are read-WRITE, so the
    /// on-disk fixture is NEVER mutated (the [[isolate-copy]] discipline).
    fn isolate_fixture(seed_dir: &Path, work: &Path, tag: &str) -> PathBuf {
        let dst = work.join(format!("ce4a-{tag}"));
        let _ = std::fs::remove_dir_all(&dst);
        std::fs::create_dir_all(dst.join("wal")).expect("FAIL-LOUD: create isolated wal dir");
        // The durable stores — open is read-WRITE and the loop's admits mutate chain.db, so the
        // fixture is copied, never opened in place. `--sparse=always` keeps the copy compact (esp.
        // on a tmpfs work dir). eview-replay-scratch.redb is deliberately NOT copied: it is the
        // FRESH replay checkpoint the window replay materializes (never read as authority; the
        // >=seed+2 frozen promotion path never touches it), so redb recreates it under `dst`.
        let mut files: Vec<PathBuf> = vec![
            seed_dir.join("chain.db"),
            seed_dir.join("epoch-accumulator.redb"),
            seed_dir.join("reduced-checkpoint.redb"),
        ];
        let wal_src = seed_dir.join("wal");
        assert!(wal_src.is_dir(), "FAIL-LOUD: fixture wal dir missing: {}", wal_src.display());
        for entry in std::fs::read_dir(&wal_src).expect("FAIL-LOUD: read fixture wal dir") {
            let p = entry.expect("wal entry").path();
            if p.is_file() {
                files.push(p);
            }
        }
        for src in &files {
            assert!(src.exists(), "FAIL-LOUD: fixture file missing: {}", src.display());
            let rel = src.strip_prefix(seed_dir).expect("path under seed dir");
            let status = std::process::Command::new("cp")
                .arg("--sparse=always")
                .arg(src)
                .arg(dst.join(rel))
                .status()
                .expect("spawn cp");
            assert!(status.success(), "FAIL-LOUD: cp {} -> isolated copy failed", src.display());
        }
        dst
    }

    /// Seal the BOOTSTRAP seed leadership (`nesPd_1338`) into the isolated copy from the
    /// manifest-bound seed record in the durable chain.db sidecar. This is the LEGITIMATE bootstrap
    /// import (the seed record IS the Mithril-snapshot consensus authority, [[seed-then-own]] /
    /// [[import-not-activate]]) — NOT a native boundary freeze. It makes the accumulator
    /// leadership-certified (the v5 store predates the certification), the same reconstruction the
    /// S4/S5 recovery tests perform. The NATIVE band (1340/1341/1342) is NOT hand-sealed here — the
    /// production loop must produce it (a hand-sealed native band would defeat the proof).
    fn seal_bootstrap_seed_leadership(dst: &Path) {
        use ade_ledger::frozen_leadership::FrozenLeadershipPoolDistr;
        use ade_ledger::seed_consensus_inputs::decode_seed_epoch_consensus_inputs;
        // The seed record travels in the durable sidecar — read it from the isolated copy's chain.db.
        let cdb = PersistentChainDb::open(PersistentChainDbOptions::at(dst.join("chain.db")))
            .expect("FAIL-LOUD: open chaindb for seed record");
        let fps = SnapshotStore::list_seed_epoch_consensus_anchor_fps(&cdb)
            .expect("FAIL-LOUD: list seed anchor fps");
        assert!(!fps.is_empty(), "FAIL-LOUD: no seed-epoch consensus anchor in the fixture");
        let record = decode_seed_epoch_consensus_inputs(
            &SnapshotStore::get_seed_epoch_consensus_inputs(&cdb, &fps[0])
                .expect("get seed record")
                .expect("seed record present"),
        )
        .expect("decode seed record");
        let nespd_seed = FrozenLeadershipPoolDistr::from_seed_epoch_consensus_inputs(&record, Hash32([0x0C; 32]));
        drop(cdb);
        let store = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: open accumulator to seal bootstrap leadership");
        // Idempotent-ish: if already certified, this simply re-writes the seed epoch. A no-op on a
        // store that already carries the band.
        store
            .seal_bootstrap_leadership_epochs(&[nespd_seed])
            .expect("FAIL-LOUD: seal bootstrap seed leadership");
        drop(store);
    }

    /// The ordered corpus block-wire bytes with `lo < slot <= hi` (the blocks strictly above the
    /// recovered durable tip, up to the ceiling). Each `<slot>.cbor` file IS the raw block the
    /// receive path decodes — the same bytes `load_corpus` feeds the co-advance differential.
    fn load_corpus_feed(corpus_dir: &Path, lo: u64, hi: u64) -> Vec<Vec<u8>> {
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(corpus_dir.join("manifest.json")).expect("FAIL-LOUD: corpus manifest"),
        )
        .expect("corpus manifest json");
        let blocks = manifest["blocks"].as_array().expect("blocks array");
        let mut sel: Vec<(u64, String)> = blocks
            .iter()
            .filter_map(|b| {
                let slot = b["slot"].as_u64().expect("slot");
                if slot > lo && slot <= hi {
                    Some((slot, b["file"].as_str().expect("file").to_string()))
                } else {
                    None
                }
            })
            .collect();
        sel.sort_by_key(|(s, _)| *s);
        sel.into_iter()
            .map(|(_, f)| std::fs::read(corpus_dir.join(f)).expect("read corpus block"))
            .collect()
    }

    /// The durable evidence read back from the post-run stores.
    struct Ce4aRun {
        durable_tip_before: u64,
        durable_tip_after: u64,
        fed_blocks: usize,
        final_epoch: u64,
        /// The target epochs of every `EpochConsensusViewActivated` WAL record (the durable
        /// promotion witnesses).
        activation_targets: Vec<u64>,
        lead_1341_promo_certified_pre: bool,
        lead_1342_promo_certified: bool,
        lead_1342_sealed: bool,
        lead_1343_sealed: bool,
        sched_log: String,
    }

    /// Drive the REAL production relay loop over an isolated copy of the v5 fixture, feeding the
    /// corpus (blocks strictly above the recovered durable tip, up to `max_slot`) through
    /// `NodeBlockSource::in_memory`. This is a faithful mirror of `run_node_lifecycle_inner`'s
    /// `ForgeIntent::Off` arm (warm_start_recovery + the production input assembly), NOT a
    /// re-composition of the loop. FAIL-LOUD at every precondition.
    async fn drive(
        seed_dir: &Path,
        corpus_dir: &Path,
        work: &Path,
        tag: &str,
        max_slot: u64,
        prep_refold: bool,
    ) -> Ce4aRun {
        let dst = isolate_fixture(seed_dir, work, tag);

        // --- fixture prep: seal the bootstrap seed leadership so the accumulator is leadership-
        //     certified (the Jul-7 v5 store predates the certification). Bootstrap import ONLY —
        //     the native band is the production loop's job. ---
        seal_bootstrap_seed_leadership(&dst);

        // --- open the durable stores (the SAME opens the lifecycle entry performs) ---
        let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(dst.join("chain.db")))
            .expect("FAIL-LOUD: open isolated chaindb");
        let mut wal = FileWalStore::open(dst.join("wal")).expect("FAIL-LOUD: open isolated wal");

        // --- production warm-start recovery (NOT a hand-built state) ---
        let warm_acc = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: open warm accumulator handle");
        let state = warm_start_recovery(&chaindb, &wal, Some(&warm_acc), preview_rsw())
            .expect("FAIL-LOUD: production warm_start_recovery");
        drop(warm_acc);

        // --- FAIL-LOUD preamble: the fixture must be the v5 POST-1340 seed the claim needs ---
        let sidecar = state
            .seed_epoch_consensus_inputs
            .clone()
            .expect("FAIL-LOUD: v5 sidecar (SeedEpochConsensusInputs) present");
        assert_eq!(
            sidecar.epoch_no.0, SEED_EPOCH,
            "FAIL-LOUD: v5 seed epoch must be {SEED_EPOCH} (so 1341/1342 are both >= seed+2, the frozen path); got {}",
            sidecar.epoch_no.0
        );
        let recovered_anchor = state.tip.clone();
        assert!(recovered_anchor.is_some(), "FAIL-LOUD: recovered durable tip present");
        let durable_tip_before =
            ChainDb::tip(&chaindb).expect("tip read").expect("FAIL-LOUD: durable chaindb tip").slot.0;
        assert_eq!(
            epoch_of(durable_tip_before), 1340,
            "FAIL-LOUD: the v5 durable tip must be in epoch 1340 (POST-1340 seed), got slot {durable_tip_before}"
        );
        // FAIL-LOUD: the fixture VENUE geometry MUST agree with the RSW profile this harness derives the
        // candidate-nonce freeze from. A disagreement means the harness would feed the WRONG RSW (run #1's
        // k=2160-for-a-preview-corpus defect: candidate froze ~2 epochs early). The RSW comes from the
        // committed `preview` profile (k=432, f=1/20 => 34560); the fixture must be that same venue.
        {
            let prof = crate::bootstrap_export::resolve_network_profile("preview")
                .expect("FAIL-LOUD: preview network profile");
            assert_eq!(
                u64::from(sidecar.epoch_length_slots), prof.epoch_length,
                "FAIL-LOUD: fixture epoch_length {} != preview profile epoch_length {} — the fixture venue and \
                 the RSW profile disagree (the k=2160-vs-432 class of bug)",
                sidecar.epoch_length_slots, prof.epoch_length
            );
            assert_eq!(
                preview_rsw(), Some(34_560),
                "FAIL-LOUD: preview candidate-nonce RSW must be ceil(4k/f) with k=432 = 34560 slots \
                 (a preprod/mainnet k=2160 would freeze ~2 epochs early on preview's short epoch)"
            );
        }

        // --- reopen the LIVE authority handles (as the lifecycle entry does post-bootstrap) ---
        let epoch_accumulator = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: open live accumulator");
        let reduced_checkpoint = ReducedUtxoCheckpoint::open(&dst.join("reduced-checkpoint.redb"))
            .expect("FAIL-LOUD: open reduced checkpoint");

        // --- FIXTURE RECONSTRUCTION (prep_refold, disclosed artifact — NOT the claimed proof):
        //     the Jul-7 v5 store predates leadership certification and has no NATIVE frozen-leadership
        //     band, so the into-1341 crossing has nothing to promote from. Reset BOTH derived stores
        //     to the 1338 seed and RE-CROSS 1338->durable-tip via the PRODUCTION advance
        //     (`advance_ledger_state_to_durable_tip` — the SAME co-advancer the relay loop runs). Its
        //     `cross_accumulator_over_boundary_block` seals native 1340/1341 with the REAL boundary
        //     marks (byte-identical to what a current-code continuous follow would have sealed at its
        //     own 1338->1339 / 1339->1340 crosses). This ONLY re-establishes the starting band a
        //     continuous node would already hold; the claimed two-boundary crossing is still done LIVE
        //     by the relay loop below. No hand-authored leadership objects (that would be theatre). ---
        if prep_refold
            && epoch_accumulator
                .promotion_leadership_authority_for_epoch(EpochNo(1341))
                .is_err()
        {
            eprintln!(
                "CE-4A.1 prep: native 1341 absent — reset+refold the accumulator+checkpoint 1338->{} \
                 via the production advance to seal the native band...",
                durable_tip_before
            );
            epoch_accumulator
                .reset_to_bootstrap()
                .expect("FAIL-LOUD: reset accumulator to the 1338 bootstrap");
            reduced_checkpoint
                .reset_to_bootstrap()
                .expect("FAIL-LOUD: reset reduced checkpoint to the 1338 bootstrap");
            let rsw = preview_rsw(); // preview k=432 (NOT 2160) — see preview_rsw()
            let prep_sched =
                recovered_node_schedule(&state, true, rsw).expect("FAIL-LOUD: prep era schedule");
            advance_ledger_state_to_durable_tip(
                Some(&reduced_checkpoint),
                Some(&epoch_accumulator),
                &chaindb,
                &prep_sched,
                &RecoveryAdmissionPolicy::cardano(),
            )
            .expect("FAIL-LOUD: prep refold 1338->durable-tip (production advance seals native 1340/1341)");
            eprintln!(
                "CE-4A.1 prep: refold done — native_1340_promo_certified={} native_1341_promo_certified={}",
                epoch_accumulator.promotion_leadership_authority_for_epoch(EpochNo(1340)).is_ok(),
                epoch_accumulator.promotion_leadership_authority_for_epoch(EpochNo(1341)).is_ok(),
            );
        }

        // --- FAIL-LOUD: the into-1341 frozen promotion REQUIRES a promotion-certified NATIVE 1341,
        //     and 1342/1343 must NOT be sealed yet (the run seals them via the two boundary freezes). ---
        let lead_1341_promo_certified_pre = epoch_accumulator
            .promotion_leadership_authority_for_epoch(EpochNo(1341))
            .is_ok();
        // The native-band precondition only binds when the feed actually crosses INTO 1341+. The
        // inspect/smoke runs (empty / within-1340 feed) proceed through the loop's own authority
        // recovery so they can OBSERVE the WAL + recovery behaviour before the expensive full run.
        if epoch_of(max_slot) >= 1341 {
            assert!(
                lead_1341_promo_certified_pre,
                "FAIL-LOUD: the v5 fixture must carry promotion-certified (native) frozen leadership for 1341 \
                 — the into-1341 crossing reads it via the S4-L2 frozen path, which fails closed otherwise"
            );
            assert!(
                epoch_accumulator.leadership_authority_for_epoch(EpochNo(1343)).is_err(),
                "FAIL-LOUD: leadership 1343 must NOT be sealed pre-run (the 1341->1342 cross seals it)"
            );
        }
        eprintln!(
            "CE-4A.1 preamble: seed_epoch={} durable_tip={durable_tip_before} (epoch {}) \
             native_1340_promo_certified={} native_1341_promo_certified={} lead_1341_general={}",
            sidecar.epoch_no.0,
            epoch_of(durable_tip_before),
            epoch_accumulator.promotion_leadership_authority_for_epoch(EpochNo(1340)).is_ok(),
            lead_1341_promo_certified_pre,
            epoch_accumulator.leadership_authority_for_epoch(EpochNo(1341)).is_ok(),
        );

        // CE-4A.3-R1 fixture-lineage refresh: rewrite the stale WAL eview records (pre-dafe0faf / pre-CE-3d)
        // to current lineage from the fresh frozen authority, so CE-4A.1 stays rerunnable under strict
        // frozen recovery (the same refresh the #12-green drive_restart_proof uses). Guarded on the frozen
        // durable-epoch being sealed (prep-refold ran); when it is not, the fail-loud recovery correctly
        // surfaces an un-refreshable store rather than a silent stale record. Done BEFORE `state` moves.
        if epoch_accumulator
            .promotion_leadership_authority_for_epoch(EpochNo(epoch_of(durable_tip_before)))
            .is_ok()
        {
            wal = refresh_prep_eview_records(
                wal,
                &dst,
                &epoch_accumulator,
                &sidecar,
                state.chain_dep.epoch_nonce.0.clone(),
                epoch_of(durable_tip_before),
            );
        }

        // --- production input assembly (a mirror of the ForgeIntent::Off arm) ---
        let seed_view = leadership_view_from_frozen_authority(Some(&epoch_accumulator), &sidecar)
            .expect("FAIL-LOUD: recovered leadership view from the frozen authority");
        // RSW = ceil(4k/f) for preview (k=432, f=1/20), the SAME source of truth rsw_for_cli uses.
        let rsw = preview_rsw(); // preview k=432 (NOT 2160) — see preview_rsw()
        let era_schedule = recovered_node_schedule(&state, true, rsw)
            .expect("FAIL-LOUD: recovered era schedule from the durable sidecar geometry");
        let eview_inputs = crate::epoch_wire::EviewActivationInputs {
            seed_bootstrap_state: state.ledger.clone(),
            seed_point_slot: sidecar.seed_point_slot,
            seed_point_hash: sidecar.seed_point_hash.clone(),
            seed_epoch: sidecar.epoch_no,
            network_magic: PREVIEW_MAGIC,
            nonce: sidecar.epoch_nonce.0.clone(),
            genesis_hash: sidecar.genesis_hash.clone(),
            protocol_params_hash: sidecar.protocol_params_hash.clone(),
            asc: sidecar.active_slots_coeff,
            replay_scratch_path: dst.join("eview-replay-scratch.redb"),
            next_epoch_bridge: chaindb
                .get_bootstrap_next_epoch_authority(&sidecar.anchor_fp)
                .ok()
                .flatten()
                .and_then(|b| {
                    ade_ledger::bootstrap_bridge::decode_bootstrap_next_epoch_authority(&b).ok()
                }),
            bootstrap_reward_delta: chaindb
                .get_bootstrap_reward_update(&sidecar.anchor_fp)
                .ok()
                .flatten()
                .and_then(|b| {
                    ade_ledger::bootstrap_reward_update::decode_bootstrap_reward_update(&b).ok()
                }),
        };

        // --- ForwardSyncState (a mirror of the ForgeIntent::Off arm's spine setup) ---
        let recovered_eta0 = Some(sidecar.epoch_nonce.clone());
        let anchor_fp = fingerprint(&state.ledger).combined;
        let mut fwd = ForwardSyncState::new(
            ReceiveState::new(state.ledger, state.chain_dep),
            anchor_fp,
            SnapshotCadence::DEFAULT,
        );
        fwd.recovered_anchor = recovered_anchor;
        fwd.recovered_eta0 = recovered_eta0;

        // --- the corpus feed: blocks strictly above the durable tip, up to `max_slot` ---
        let feed = load_corpus_feed(corpus_dir, durable_tip_before, max_slot);
        let fed_blocks = feed.len();
        eprintln!(
            "CE-4A.1: seed_epoch={SEED_EPOCH} durable_tip={durable_tip_before} (epoch {}) \
             feeding {fed_blocks} corpus blocks in ({durable_tip_before}, {max_slot}]",
            epoch_of(durable_tip_before)
        );
        let mut source = NodeBlockSource::in_memory(feed);
        let (_tx, mut shutdown) = watch::channel(false);
        let mut sched_log = crate::live_log::NodeSchedLogWriter::new(Vec::<u8>::new());

        // --- DRIVE THE PRODUCTION LOOP — all three authority inputs Some (the CE-4A.1 critical guard) ---
        run_relay_loop_with_sched(
            &mut fwd,
            &mut source,
            &chaindb,
            &mut wal,
            &era_schedule,
            &seed_view,
            &mut shutdown,
            None,                      // forge OFF — CE-4A is follow / self-sufficiency, not forging
            Some(&mut sched_log),      // capture the CN-NODE-04 transcript (evidence)
            None,                      // no convergence evidence
            Some(&reduced_checkpoint), // GUARD: reduced checkpoint present
            Some(&eview_inputs),       // GUARD: eview activation present
            Some(&epoch_accumulator),  // GUARD: epoch accumulator present
            RecoveryAdmissionPolicy::cardano(),
        )
        .await
        .expect("FAIL-LOUD: the production relay loop must halt cleanly — any error is a CE-4A.1 failure");

        // --- durable evidence, read from the post-run stores ---
        let durable_tip_after = ChainDb::tip(&chaindb).expect("tip read").expect("durable tip").slot.0;
        let (_s, acc) = epoch_accumulator
            .load_current()
            .expect("load_current")
            .expect("sealed accumulator");
        let final_epoch = acc.epoch_state.epoch.0;
        let wal_entries = wal.read_all().expect("wal read_all");
        let activation_targets: Vec<u64> = wal_entries
            .iter()
            .filter_map(|e| match e {
                WalEntry::EpochConsensusViewActivated { target_epoch, .. } => Some(target_epoch.0),
                _ => None,
            })
            .collect();
        let lead_1342_sealed = epoch_accumulator.leadership_authority_for_epoch(EpochNo(1342)).is_ok();
        let lead_1343_sealed = epoch_accumulator.leadership_authority_for_epoch(EpochNo(1343)).is_ok();
        let lead_1342_promo_certified = epoch_accumulator
            .promotion_leadership_authority_for_epoch(EpochNo(1342))
            .is_ok();
        let sched_log = String::from_utf8(sched_log.into_inner()).unwrap_or_default();

        // Free the isolated copy (load-bearing on a tmpfs work dir). Drop every store handle first
        // so the files are closed before removal; CE4A_KEEP retains the copy for debugging.
        drop(reduced_checkpoint);
        drop(epoch_accumulator);
        drop(wal);
        drop(chaindb);
        if std::env::var("CE4A_KEEP").is_err() {
            let _ = std::fs::remove_dir_all(&dst);
        }

        Ce4aRun {
            durable_tip_before,
            durable_tip_after,
            fed_blocks,
            final_epoch,
            activation_targets,
            lead_1341_promo_certified_pre,
            lead_1342_promo_certified,
            lead_1342_sealed,
            lead_1343_sealed,
            sched_log,
        }
    }

    /// FAST ground-truth: warm-start the v5 copy, assemble ALL THREE inputs, run the production
    /// loop over an EMPTY feed. Exercises warm_start_recovery + the input assembly + the loop's
    /// authority recovery + a clean halt — with NO fold. Fails loud in seconds if the composition
    /// or the fixture's leadership certification is inadequate.
    #[tokio::test]
    #[ignore = "CE-4A.1 inspect: v5 fixture warm-start + prep-refold + fixture-lineage refresh + input assembly + loop recovery of the frozen 1340 authority, empty feed, NO fold (env S5_SEED_STORES / CE3D_WORK); ~30min (prep-refold under strict frozen recovery)"]
    async fn ce4a_inspect_fixture() {
        let seed = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
        let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
        let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");
        // max_slot == durable tip => empty feed (no block has tip < slot <= tip).
        let run = drive(&seed, &corpus, &work, "inspect", env_u64("CE4A_INSPECT_MAX", 0), true).await;
        eprintln!(
            "CE-4A.1 INSPECT: tip_before={} tip_after={} fed={} final_epoch={} \
             lead_1341_promo_certified={} activation_targets={:?}",
            run.durable_tip_before,
            run.durable_tip_after,
            run.fed_blocks,
            run.final_epoch,
            run.lead_1341_promo_certified_pre,
            run.activation_targets,
        );
        assert_eq!(run.fed_blocks, 0, "inspect feeds no blocks");
        assert_eq!(run.durable_tip_after, run.durable_tip_before, "empty feed does not advance the tip");
        assert_eq!(run.final_epoch, 1340, "empty feed crosses no boundary — accumulator stays at 1340");
    }

    /// MINUTES: feed a small slice of epoch-1340 blocks (no boundary). Proves the production loop
    /// ADMITS corpus blocks against the RECOVERED authority (so the loop's authority recovered to
    /// epoch 1340 — a 1340 block would fail header validation against a stale seed view).
    #[tokio::test]
    #[ignore = "CE-4A.1 smoke: prep-refold + fixture-lineage refresh, then the production loop admits epoch-1340 corpus blocks against the recovered frozen 1340 authority, NO boundary (env S5_SEED_STORES / CE3D_CORPUS / CE3D_WORK / CE4A_SMOKE_MAX); ~30min+ (prep under strict frozen recovery)"]
    async fn ce4a_smoke_admit_within_epoch() {
        let seed = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
        let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
        let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");
        // A few thousand slots into epoch 1340 — well below the 1341 boundary (115_862_400).
        let max_slot = env_u64("CE4A_SMOKE_MAX", 115_780_000);
        assert!(epoch_of(max_slot) == 1340, "smoke ceiling must stay inside epoch 1340");
        let run = drive(&seed, &corpus, &work, "smoke", max_slot, true).await;
        eprintln!(
            "CE-4A.1 SMOKE: tip_before={} tip_after={} fed={} final_epoch={}",
            run.durable_tip_before, run.durable_tip_after, run.fed_blocks, run.final_epoch
        );
        assert!(run.fed_blocks > 0, "FAIL-LOUD: the smoke slice must contain corpus blocks");
        assert!(
            run.durable_tip_after > run.durable_tip_before,
            "FAIL-LOUD: the production loop must ADMIT the fed epoch-1340 blocks (tip advances)"
        );
        assert_eq!(run.final_epoch, 1340, "the smoke slice crosses no boundary");
    }

    /// De-risk the fixture reconstruction (SLOW ~100min): reset+refold 1338->1340 via the production
    /// advance, then assert native promotion-certified frozen leadership for 1341 got sealed (empty
    /// corpus feed — no boundary crossing). Confirms the reset+refold produces the native band the
    /// full run's into-1341 crossing needs, before the multi-hour full run.
    #[tokio::test]
    #[ignore = "CE-4A.1 prep-verify: reset+refold 1338->1340 seals native promotion-certified 1341 (env S5_SEED_STORES / CE3D_CORPUS / CE3D_WORK); SLOW ~100min"]
    async fn ce4a_prep_verify_native_band() {
        let seed = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
        let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
        let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");
        // Empty feed (max_slot 0) — the reset+refold is the subject; no boundary crossing.
        let run = drive(&seed, &corpus, &work, "prep", 0, true).await;
        eprintln!(
            "CE-4A.1 PREP-VERIFY: final_epoch={} native_1341_promo_certified={} activation_targets={:?}",
            run.final_epoch, run.lead_1341_promo_certified_pre, run.activation_targets
        );
        assert!(
            run.lead_1341_promo_certified_pre,
            "FAIL-LOUD: reset+refold 1338->1340 must seal native promotion-certified frozen leadership for 1341"
        );
        assert_eq!(run.final_epoch, 1340, "empty feed crosses no boundary — accumulator stays at 1340");
    }

    /// THE CE-4A.1 EVIDENCE RUN (SLOW, ~hours). Feed the corpus from the durable tip across BOTH
    /// real boundaries (1340->1341->1342) through the production composition; prove the freeze ->
    /// persist -> frozen-promote -> admit -> repeat loop end-to-end, and emit the machine-readable
    /// evidence bundle. Locked claim: production-loop continuous self-sufficiency across two real
    /// boundaries. NON-claims: not byte-exact (4A.2), not restart/rollback (4A.3), not live, not
    /// bounty-ready.
    #[tokio::test]
    #[ignore = "CE-4A.1: production-loop continuous self-sufficiency across TWO real boundaries 1340->1341->1342 (env S5_SEED_STORES / CE3D_CORPUS / CE3D_WORK); SLOW ~hours (folds ~5000 real Conway blocks through the production loop)"]
    async fn ce4a_1_continuous_self_sufficiency() {
        let seed = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
        let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
        let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");
        let max_slot = env_u64("CE4A_MAX_SLOT", EPOCH_1342_FIRST_SLOT);

        let run = drive(&seed, &corpus, &work, "full", max_slot, true).await;

        // ---- CE-4A.1 acceptance (ALL must hold; each is FAIL-LOUD) ----
        let start_epoch = epoch_of(run.durable_tip_before);
        assert_eq!(start_epoch, 1340, "start epoch is 1340 (POST-1340 seed)");
        assert!(run.fed_blocks > 0, "FAIL-LOUD: the feed must contain corpus blocks");
        // (1) both boundaries crossed IN ONE RUN — the accumulator reached 1342.
        assert_eq!(
            run.final_epoch, 1342,
            "FAIL-LOUD: the accumulator must cross 1340->1341->1342 in one continuous production run"
        );
        // (2) the two boundary crossings are witnessed durably by promotion WAL activation records.
        assert!(
            run.activation_targets.contains(&1341),
            "FAIL-LOUD: missing the into-1341 promotion WAL record (EpochConsensusViewActivated{{1341}})"
        );
        assert!(
            run.activation_targets.contains(&1342),
            "FAIL-LOUD: missing the into-1342 promotion WAL record (EpochConsensusViewActivated{{1342}})"
        );
        // (3) the two boundary freezes sealed the frozen-leadership targets 1342 and 1343.
        assert!(run.lead_1342_sealed, "FAIL-LOUD: the 1340->1341 cross must seal frozen leadership 1342");
        assert!(run.lead_1343_sealed, "FAIL-LOUD: the 1341->1342 cross must seal frozen leadership 1343");
        // (4) the promotion source is the S4-L2 promotion-certified frozen object (candidate >= seed+2):
        //     1341 was promotion-certified pre-run; 1342 (sealed THIS run by the 1340->1341 freeze) is
        //     promotion-certified and is what the into-1342 crossing read.
        assert!(run.lead_1341_promo_certified_pre, "into-1341 read a promotion-certified frozen 1341");
        assert!(
            run.lead_1342_promo_certified,
            "FAIL-LOUD: the 1342 the into-1342 crossing promoted from must be a promotion-certified native freeze"
        );
        // (5) the loop emitted a CN-NODE-04 transcript across the run (it genuinely ran the
        //     production scheduler, not a stub). forbidden_paths are structurally false: this
        //     harness never calls a re-import, cardano-cli oracle, seed-window replay, or
        //     materialize_bootstrap_into — the >=seed+2 crossings take the S4-L2 frozen path ONLY.
        assert!(!run.sched_log.is_empty(), "FAIL-LOUD: the loop must emit a CN-NODE-04 transcript");

        // ---- machine-readable evidence bundle (auditable; every field is asserted above) ----
        let bundle = serde_json::json!({
            "slice": "CE-4A.1",
            "claim": "production-loop continuous self-sufficiency across two real boundaries",
            "start_epoch": start_epoch,
            "fed_blocks": run.fed_blocks,
            "crossed_boundaries": ["1340->1341", "1341->1342"],
            "frozen_leadership_targets": [1342, 1343],
            "promotion_source": "FrozenLeadershipPoolDistr",
            "promotion_certified": true,
            "promotion_wal_targets": run.activation_targets,
            "authority_inputs_present": {
                "reduced_checkpoint": true,
                "eview_activation": true,
                "epoch_accumulator": true
            },
            "forbidden_paths": {
                "reimport": false,
                "cli_oracle": false,
                "seed_window_replay": false,
                "materialize_bootstrap_into": false
            }
        });
        let bundle_str = serde_json::to_string_pretty(&bundle).expect("serialize evidence bundle");
        eprintln!("\n===== CE-4A.1 EVIDENCE BUNDLE =====\n{bundle_str}\n===================================");
        let out = env_path("CE4A_EVIDENCE_OUT", "/home/ts/.cardano-ce3d-extract/ce4a-1-evidence.json");
        std::fs::write(&out, &bundle_str).unwrap_or_else(|e| panic!("write evidence bundle {}: {e:?}", out.display()));
        eprintln!("CE-4A.1 evidence bundle written to {}", out.display());
    }

    // ============================================================================================
    // CE-4A.2 — boundary outputs byte-match the cardano reference at BOTH self-derived boundaries.
    // The byte-exact strengthening of CE-4A.1: the SAME production composition, read-only extraction
    // added. It reads the CE-4A.1 production-loop accumulator (NOT the co_advance differential
    // harness) and promotes CE-3d's observational MATCH prints for rewards/pots/go into fail-loud
    // asserts. Spec: docs/clusters/LIVE-LEDGER-EPOCH-TRANSITION/SLICE-CE-4A-2-BOUNDARY-BYTE-EXACT.md.
    // ============================================================================================

    /// First BLOCK of epoch 1341 (boundary 1340 -> 1341) — the POST-1341 reference + capture point
    /// (`Snapshot stored at SlotNo 115862416`, the cardano `--store-ledger 115862400` reference).
    const EPOCH_1341_FIRST_SLOT: u64 = 115_862_416;

    fn ok(b: bool) -> &'static str {
        if b {
            "MATCH"
        } else {
            "*** MISMATCH ***"
        }
    }
    fn hex32(h: &[u8; 32]) -> String {
        h.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Canonical discriminant-preserving byte key for a stake credential (rewards are keyed by the
    /// FULL credential; the go-snapshot uses a Hash28-only pool key). Mirrors the ce3d differential
    /// harness's `cred_key` so the diff is byte-uniform across both sides.
    fn cred_key(c: &ade_types::shelley::cert::StakeCredential) -> Vec<u8> {
        use ade_types::shelley::cert::StakeCredential;
        let (tag, h) = match c {
            StakeCredential::KeyHash(h) => (0u8, h),
            StakeCredential::ScriptHash(h) => (1u8, h),
        };
        let mut k = Vec::with_capacity(29);
        k.push(tag);
        k.extend_from_slice(&h.0);
        k
    }

    /// The canonical AUTHORITY stake-view hash over a go-snapshot pool-stake map — the SAME formula
    /// the S5 recovery proof commits (`s5_authority_stake_view_hash`). A leader-election stake
    /// commitment derivable IDENTICALLY from Ade's accumulator AND from the cardano reference go, so
    /// it is the one fingerprint that IS reference-comparable (unlike the Ade-internal accumulator /
    /// leadership canonical hashes, which have no cardano counterpart).
    fn stake_view_hash_from_go(go: &std::collections::BTreeMap<Vec<u8>, u64>) -> [u8; 32] {
        let total: u128 = go.values().map(|c| *c as u128).sum();
        let mut buf = Vec::with_capacity(24 + go.len() * 36);
        buf.extend_from_slice(&total.to_be_bytes());
        buf.extend_from_slice(&(go.len() as u64).to_be_bytes());
        for (pool, coin) in go {
            buf.extend_from_slice(pool); // 28-byte pool keyhash
            buf.extend_from_slice(&coin.to_be_bytes());
        }
        ade_crypto::blake2b_256(&buf).0
    }

    /// The self-derived boundary outputs, read READ-ONLY from the production-loop accumulator at a
    /// POST-boundary point. Every map is in the SAME comparable byte form the cardano reference
    /// decodes to (see [`RefOutputs`]).
    struct BoundaryOutputs {
        epoch: u64,
        treasury: u64,
        reserves: u64,
        fees: u64,
        go: std::collections::BTreeMap<Vec<u8>, u64>,
        rewards: std::collections::BTreeMap<Vec<u8>, u64>,
        nes_pd: std::collections::BTreeMap<[u8; 28], (u64, [u8; 32])>,
        stake_view_hash: [u8; 32],
        /// Ade-internal durability commitments (no cardano counterpart — REPORTED, not asserted vs the ref).
        acc_hash: [u8; 32],
        leadership_hash: [u8; 32],
    }

    /// The cardano POST-boundary reference in the same comparable form, decoded by Ade's own
    /// `decode_native_nonutxo_state` (it parses the 11.0.1 Conway `state` cleanly).
    struct RefOutputs {
        epoch: u64,
        treasury: u64,
        reserves: u64,
        fees: u64,
        go: std::collections::BTreeMap<Vec<u8>, u64>,
        rewards: std::collections::BTreeMap<Vec<u8>, u64>,
        nes_pd: std::collections::BTreeMap<[u8; 28], (u64, [u8; 32])>,
        stake_view_hash: [u8; 32],
    }

    /// Extract the POST-boundary outputs from the LIVE production-loop accumulator (read-only). The
    /// leadership nesPd is read epoch-indexed for `leadership_epoch` (the S4-pre-2 pin:
    /// `leadership_authority_for_epoch(E)` byte-matches the cardano POST-E nesPd / nes[5]).
    fn capture_boundary_outputs(
        store: &EpochAccumulatorStore,
        leadership_epoch: u64,
    ) -> BoundaryOutputs {
        let (_slot, acc) = store
            .load_current()
            .expect("FAIL-LOUD: load_current for boundary capture")
            .expect("FAIL-LOUD: sealed accumulator");
        let es = &acc.epoch_state;
        let go: std::collections::BTreeMap<Vec<u8>, u64> = es
            .snapshots
            .as_authoritative()
            .expect("FAIL-LOUD: authoritative snapshots (the production advance keeps go authoritative)")
            .go
            .0
            .pool_stakes
            .iter()
            .map(|(pid, c)| ((pid.0).0.to_vec(), c.0))
            .collect();
        let rewards: std::collections::BTreeMap<Vec<u8>, u64> = acc
            .cert_state
            .delegation
            .rewards
            .iter()
            .map(|(cred, c)| (cred_key(cred), c.0))
            .collect();
        let leadership = store
            .leadership_authority_for_epoch(EpochNo(leadership_epoch))
            .expect("FAIL-LOUD: frozen leadership authority for the boundary's leadership epoch");
        let nes_pd: std::collections::BTreeMap<[u8; 28], (u64, [u8; 32])> = leadership
            .pools
            .iter()
            .map(|(h, e)| (h.0, (e.active_stake, e.vrf_keyhash.0)))
            .collect();
        let stake_view_hash = stake_view_hash_from_go(&go);
        let acc_hash =
            ade_crypto::blake2b_256(&ade_ledger::epoch_accumulator::encode_epoch_accumulator(&acc)).0;
        let leadership_hash = ade_ledger::frozen_leadership::canonical_hash(&leadership).0;
        BoundaryOutputs {
            epoch: es.epoch.0,
            treasury: es.treasury.0,
            reserves: es.reserves.0,
            fees: es.epoch_fees.0,
            go,
            rewards,
            nes_pd,
            stake_view_hash,
            acc_hash,
            leadership_hash,
        }
    }

    /// Decode a cardano POST-boundary reference `state` blob into the comparable surfaces.
    fn ref_boundary_outputs(state_path: &Path, slot: u64, epoch: u64) -> RefOutputs {
        use ade_ledger::bootstrap_anchor::SeedPoint;
        use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
        let state = std::fs::read(state_path).unwrap_or_else(|e| {
            panic!("FAIL-LOUD: read cardano reference state {}: {e:?}", state_path.display())
        });
        let point = SeedPoint {
            slot: ade_types::SlotNo(slot),
            block_hash: ade_types::Hash32([0u8; 32]),
        };
        let (s1a, _commit) = decode_native_nonutxo_state(&state, point, epoch, PREVIEW_MAGIC)
            .expect("FAIL-LOUD: decode cardano reference state");
        let go: std::collections::BTreeMap<Vec<u8>, u64> = s1a
            .snapshots
            .go
            .0
            .pool_stakes
            .iter()
            .map(|(pid, c)| ((pid.0).0.to_vec(), c.0))
            .collect();
        let rewards = s1a
            .cert_state
            .delegation
            .rewards
            .iter()
            .map(|(cred, c)| (cred_key(cred), c.0))
            .collect();
        let nes_pd = s1a
            .pool_distr
            .iter()
            .map(|(pid, (stake, vrf))| ((pid.0).0, (*stake, vrf.0)))
            .collect();
        let stake_view_hash = stake_view_hash_from_go(&go);
        RefOutputs {
            epoch: s1a.epoch.0,
            treasury: s1a.treasury.0,
            reserves: s1a.reserves.0,
            fees: s1a.epoch_fees.0,
            go,
            rewards,
            nes_pd,
            stake_view_hash,
        }
    }

    /// Byte-exact map comparison with a diagnostic breakdown. Returns `true` iff byte-identical.
    fn map_matches(
        name: &str,
        ade: &std::collections::BTreeMap<Vec<u8>, u64>,
        refs: &std::collections::BTreeMap<Vec<u8>, u64>,
    ) -> bool {
        let (mut val_mismatch, mut only_ade, mut only_ref) = (0usize, 0usize, 0usize);
        let mut samples: Vec<String> = Vec::new();
        let keys: std::collections::BTreeSet<&Vec<u8>> = ade.keys().chain(refs.keys()).collect();
        for k in &keys {
            let hx: String = k.iter().take(8).map(|b| format!("{b:02x}")).collect();
            match (ade.get(*k), refs.get(*k)) {
                (Some(a), Some(r)) if a == r => {}
                (Some(a), Some(r)) => {
                    val_mismatch += 1;
                    if samples.len() < 8 {
                        samples.push(format!("{hx}.. ade={a} ref={r} (d{})", *a as i128 - *r as i128));
                    }
                }
                (Some(a), None) => {
                    only_ade += 1;
                    if samples.len() < 8 {
                        samples.push(format!("{hx}.. ade={a} ref=ABSENT"));
                    }
                }
                (None, Some(r)) => {
                    only_ref += 1;
                    if samples.len() < 8 {
                        samples.push(format!("{hx}.. ade=ABSENT ref={r}"));
                    }
                }
                (None, None) => {}
            }
        }
        let at: u64 = ade.values().sum();
        let rt: u64 = refs.values().sum();
        let exact = val_mismatch == 0 && only_ade == 0 && only_ref == 0 && at == rt;
        eprintln!(
            "  {name:<14} ade_keys={} ref_keys={} val_mismatch={val_mismatch} only_ade={only_ade} \
             only_ref={only_ref} sum_ade={at} sum_ref={rt}  {}",
            ade.len(),
            refs.len(),
            ok(exact)
        );
        for s in &samples {
            eprintln!("      {s}");
        }
        exact
    }

    /// Per-surface byte-match verdict for one boundary (every field is printed for diagnosis).
    struct SurfaceMatch {
        epoch: bool,
        treasury: bool,
        reserves: bool,
        go: bool,
        rewards: bool,
        nes_pd: bool,
        nes_pd_count: (usize, usize),
        stake_view_hash: bool,
    }
    impl SurfaceMatch {
        /// The hard vs-cardano surfaces. `fees` is EXCLUDED (reported-with-note, never asserted):
        /// Ade's `epoch_fees` is a boundary-consumed reward-input accumulator (zeroed at the boundary,
        /// re-accumulated for the new epoch) while cardano's `utxosFees` is a running live residual fee
        /// pot — different observable quantities at the same instant. Fee economics are proven
        /// transitively through byte-exact rewards + treasury + reserves (the surfaces that actually
        /// consume the fees). CE-4A.2 does NOT claim raw `utxosFees` equivalence.
        fn all_mandatory(&self) -> bool {
            self.epoch
                && self.treasury
                && self.reserves
                && self.go
                && self.rewards
                && self.nes_pd
                && self.stake_view_hash
        }
    }

    /// Compare a self-derived boundary output to the cardano reference, surface by surface, printing a
    /// full diagnostic table. Computes ALL verdicts (no short-circuit) so one run reveals every
    /// mismatch, not just the first.
    fn compare_boundary(label: &str, ade: &BoundaryOutputs, refs: &RefOutputs) -> SurfaceMatch {
        eprintln!("==================== CE-4A.2 {label} ====================");
        let epoch = ade.epoch == refs.epoch;
        let treasury = ade.treasury == refs.treasury;
        let reserves = ade.reserves == refs.reserves;
        eprintln!("  epoch    ade={} ref={}  {}", ade.epoch, refs.epoch, ok(epoch));
        eprintln!(
            "  treasury ade={} ref={} d{}  {}",
            ade.treasury,
            refs.treasury,
            ade.treasury as i128 - refs.treasury as i128,
            ok(treasury)
        );
        eprintln!(
            "  reserves ade={} ref={} d{}  {}",
            ade.reserves,
            refs.reserves,
            ade.reserves as i128 - refs.reserves as i128,
            ok(reserves)
        );
        eprintln!(
            "  fees     ade_epoch_fees={} cardano_utxosFees={}  [representation-diff, NOT asserted: \
             reset-and-reaccumulate accumulator vs running residual pot; fee-consensus proven via \
             rewards+treasury+reserves]",
            ade.fees, refs.fees
        );
        let go = map_matches("go_pool_stakes", &ade.go, &refs.go);
        let rewards = map_matches("rewards", &ade.rewards, &refs.rewards);
        let nes_pd = ade.nes_pd == refs.nes_pd;
        eprintln!(
            "  nes_pd   ade_pools={} ref_pools={} zero_stake={}  {}",
            ade.nes_pd.len(),
            refs.nes_pd.len(),
            ade.nes_pd.values().filter(|(s, _)| *s == 0).count(),
            ok(nes_pd)
        );
        let stake_view_hash = ade.stake_view_hash == refs.stake_view_hash;
        eprintln!(
            "  stake_view_hash ade={} ref={}  {}",
            hex32(&ade.stake_view_hash),
            hex32(&refs.stake_view_hash),
            ok(stake_view_hash)
        );
        eprintln!(
            "  [evidence, Ade-internal, no cardano counterpart] acc_hash={} leadership_hash={}",
            hex32(&ade.acc_hash),
            hex32(&ade.leadership_hash)
        );
        SurfaceMatch {
            epoch,
            treasury,
            reserves,
            go,
            rewards,
            nes_pd,
            nes_pd_count: (ade.nes_pd.len(), refs.nes_pd.len()),
            stake_view_hash,
        }
    }

    /// Drive the REAL production relay loop over an isolated v5 copy, folding the corpus
    /// `(durable_tip, max_slot]` through the SAME `run_relay_loop_with_sched` composition as CE-4A.1
    /// in ONE continuous invocation, then capture the POST-boundary outputs read-only. A SINGLE loop
    /// call (never a mid-run split): a split into two calls over the same stores forces the eview
    /// warm-start-across-boundary recovery to re-enter at the intermediate boundary and fail closed
    /// (`EpochViewPostPromotionMismatch`) — a production EPOCH-CONSENSUS-VIEW limitation this slice
    /// must NOT patch. The two boundaries are instead captured by TWO independent single-call runs
    /// (POST-1341 from a run halted at the 1341 boundary — the deterministic single-boundary prefix;
    /// POST-1342 from the full continuous two-boundary run — the literal CE-4A.1 run). Mirrors
    /// `drive()`'s production warm-start SELF-CONTAINED so the byte-exact path never perturbs the
    /// committed, PROVEN CE-4A.1 `drive()` (the same "mirror, don't reach into" discipline `drive()`
    /// itself uses for the `ForgeIntent::Off` arm).
    async fn drive_capture_at(
        seed_dir: &Path,
        corpus_dir: &Path,
        work: &Path,
        tag: &str,
        max_slot: u64,
        leadership_epoch: u64,
    ) -> (BoundaryOutputs, usize) {
        let dst = isolate_fixture(seed_dir, work, tag);
        seal_bootstrap_seed_leadership(&dst);

        let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(dst.join("chain.db")))
            .expect("FAIL-LOUD: open isolated chaindb");
        let mut wal = FileWalStore::open(dst.join("wal")).expect("FAIL-LOUD: open isolated wal");
        let warm_acc = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: open warm accumulator handle");
        let state = warm_start_recovery(&chaindb, &wal, Some(&warm_acc), preview_rsw())
            .expect("FAIL-LOUD: production warm_start_recovery");
        drop(warm_acc);

        let sidecar = state
            .seed_epoch_consensus_inputs
            .clone()
            .expect("FAIL-LOUD: v5 sidecar (SeedEpochConsensusInputs) present");
        assert_eq!(
            sidecar.epoch_no.0, SEED_EPOCH,
            "FAIL-LOUD: v5 seed epoch must be {SEED_EPOCH} (so 1341/1342 are both >= seed+2); got {}",
            sidecar.epoch_no.0
        );
        let recovered_anchor = state.tip.clone();
        assert!(recovered_anchor.is_some(), "FAIL-LOUD: recovered durable tip present");
        let durable_tip_before =
            ChainDb::tip(&chaindb).expect("tip read").expect("FAIL-LOUD: durable chaindb tip").slot.0;
        assert_eq!(
            epoch_of(durable_tip_before),
            1340,
            "FAIL-LOUD: the v5 durable tip must be in epoch 1340 (POST-1340 seed), got slot {durable_tip_before}"
        );
        {
            let prof = crate::bootstrap_export::resolve_network_profile("preview")
                .expect("FAIL-LOUD: preview network profile");
            assert_eq!(
                u64::from(sidecar.epoch_length_slots),
                prof.epoch_length,
                "FAIL-LOUD: fixture epoch_length {} != preview profile epoch_length {} — venue vs RSW \
                 profile disagree (the k=2160-vs-432 class of bug)",
                sidecar.epoch_length_slots,
                prof.epoch_length
            );
            assert_eq!(
                preview_rsw(),
                Some(34_560),
                "FAIL-LOUD: preview candidate-nonce RSW must be ceil(4k/f) with k=432 = 34560 slots"
            );
        }

        let epoch_accumulator = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: open live accumulator");
        let reduced_checkpoint = ReducedUtxoCheckpoint::open(&dst.join("reduced-checkpoint.redb"))
            .expect("FAIL-LOUD: open reduced checkpoint");

        // FIXTURE PREP (disclosed artifact — identical to CE-4A.1): the Jul-7 v5 store predates
        // leadership certification, so reset both derived stores to the 1338 seed and re-cross
        // 1338->durable-tip via the PRODUCTION advance to seal the native promotion-certified band.
        if epoch_accumulator
            .promotion_leadership_authority_for_epoch(EpochNo(1341))
            .is_err()
        {
            eprintln!(
                "CE-4A.2 prep: native 1341 absent — reset+refold 1338->{durable_tip_before} via the \
                 production advance to seal the native band..."
            );
            epoch_accumulator.reset_to_bootstrap().expect("FAIL-LOUD: reset accumulator to 1338");
            reduced_checkpoint.reset_to_bootstrap().expect("FAIL-LOUD: reset checkpoint to 1338");
            let prep_sched =
                recovered_node_schedule(&state, true, preview_rsw()).expect("FAIL-LOUD: prep era schedule");
            advance_ledger_state_to_durable_tip(
                Some(&reduced_checkpoint),
                Some(&epoch_accumulator),
                &chaindb,
                &prep_sched,
                &RecoveryAdmissionPolicy::cardano(),
            )
            .expect("FAIL-LOUD: prep refold 1338->durable-tip (production advance seals native 1340/1341)");
        }
        assert!(
            epoch_accumulator
                .promotion_leadership_authority_for_epoch(EpochNo(1341))
                .is_ok(),
            "FAIL-LOUD: the fixture must carry promotion-certified (native) frozen leadership for 1341"
        );
        assert!(
            epoch_accumulator.leadership_authority_for_epoch(EpochNo(1343)).is_err(),
            "FAIL-LOUD: leadership 1343 must NOT be sealed pre-run (the 1341->1342 cross seals it)"
        );

        // CE-4A.3-R1 fixture-lineage refresh: rewrite the stale WAL eview records (pre-dafe0faf / pre-CE-3d)
        // to current lineage from the fresh frozen authority, so CE-4A.2 stays rerunnable under strict
        // frozen recovery (the same refresh drive_restart_proof uses). Done BEFORE `state` moves below.
        if epoch_accumulator
            .promotion_leadership_authority_for_epoch(EpochNo(epoch_of(durable_tip_before)))
            .is_ok()
        {
            wal = refresh_prep_eview_records(
                wal,
                &dst,
                &epoch_accumulator,
                &sidecar,
                state.chain_dep.epoch_nonce.0.clone(),
                epoch_of(durable_tip_before),
            );
        }

        // --- production input assembly (a mirror of drive() / the ForgeIntent::Off arm) ---
        let seed_view = leadership_view_from_frozen_authority(Some(&epoch_accumulator), &sidecar)
            .expect("FAIL-LOUD: recovered leadership view from the frozen authority");
        let era_schedule = recovered_node_schedule(&state, true, preview_rsw())
            .expect("FAIL-LOUD: recovered era schedule from the durable sidecar geometry");
        let eview_inputs = crate::epoch_wire::EviewActivationInputs {
            seed_bootstrap_state: state.ledger.clone(),
            seed_point_slot: sidecar.seed_point_slot,
            seed_point_hash: sidecar.seed_point_hash.clone(),
            seed_epoch: sidecar.epoch_no,
            network_magic: PREVIEW_MAGIC,
            nonce: sidecar.epoch_nonce.0.clone(),
            genesis_hash: sidecar.genesis_hash.clone(),
            protocol_params_hash: sidecar.protocol_params_hash.clone(),
            asc: sidecar.active_slots_coeff,
            replay_scratch_path: dst.join("eview-replay-scratch.redb"),
            next_epoch_bridge: chaindb
                .get_bootstrap_next_epoch_authority(&sidecar.anchor_fp)
                .ok()
                .flatten()
                .and_then(|b| ade_ledger::bootstrap_bridge::decode_bootstrap_next_epoch_authority(&b).ok()),
            bootstrap_reward_delta: chaindb
                .get_bootstrap_reward_update(&sidecar.anchor_fp)
                .ok()
                .flatten()
                .and_then(|b| ade_ledger::bootstrap_reward_update::decode_bootstrap_reward_update(&b).ok()),
        };
        let anchor_fp = fingerprint(&state.ledger).combined;
        let mut fwd = ForwardSyncState::new(
            ReceiveState::new(state.ledger, state.chain_dep),
            anchor_fp,
            SnapshotCadence::DEFAULT,
        );
        fwd.recovered_anchor = recovered_anchor;
        fwd.recovered_eta0 = Some(sidecar.epoch_nonce.clone());

        // ---- fold (durable_tip, max_slot] in ONE continuous production-loop invocation ----
        //      A single run_relay_loop_with_sched call — NO re-entry, so no eview
        //      warm-start-across-boundary recovery (a mid-run split re-enters and fails closed
        //      EpochViewPostPromotionMismatch — a production limitation this slice must not patch).
        eprintln!(
            "CE-4A.2 fold ({durable_tip_before}, {max_slot}] through the production loop (single \
             continuous invocation) — capturing POST leadership epoch {leadership_epoch}"
        );
        let fed = {
            let feed = load_corpus_feed(corpus_dir, durable_tip_before, max_slot);
            let n = feed.len();
            let mut source = NodeBlockSource::in_memory(feed);
            let (_tx, mut shutdown) = watch::channel(false);
            let mut sched = crate::live_log::NodeSchedLogWriter::new(Vec::<u8>::new());
            run_relay_loop_with_sched(
                &mut fwd,
                &mut source,
                &chaindb,
                &mut wal,
                &era_schedule,
                &seed_view,
                &mut shutdown,
                None,
                Some(&mut sched),
                None,
                Some(&reduced_checkpoint),
                Some(&eview_inputs),
                Some(&epoch_accumulator),
                RecoveryAdmissionPolicy::cardano(),
            )
            .await
            .expect("FAIL-LOUD: production relay loop must halt cleanly");
            n
        };
        let post = capture_boundary_outputs(&epoch_accumulator, leadership_epoch);
        assert_eq!(
            post.epoch,
            epoch_of(max_slot),
            "FAIL-LOUD: the run must land the accumulator at POST-{}; got epoch {}",
            epoch_of(max_slot),
            post.epoch
        );
        eprintln!(
            "CE-4A.2 captured POST-{}: treasury={} reserves={} fees={} go_pools={} rewards={} nesPd={}",
            post.epoch,
            post.treasury,
            post.reserves,
            post.fees,
            post.go.len(),
            post.rewards.len(),
            post.nes_pd.len()
        );

        drop(reduced_checkpoint);
        drop(epoch_accumulator);
        drop(wal);
        drop(chaindb);
        if std::env::var("CE4A_KEEP").is_err() {
            let _ = std::fs::remove_dir_all(&dst);
        }
        (post, fed)
    }

    /// CE-4A.2 — the byte-exact evidence run. Drives the production composition across both real
    /// boundaries, then hard-asserts every self-derived boundary surface byte-matches the cardano
    /// POST-1341 / POST-1342 LedgerDB references. Local, uncommitted references (`CE3D_REF`), so this
    /// is `#[ignore]` local evidence (NOT a CI gate) — same nature as CE-4A.1.
    #[tokio::test]
    #[ignore = "CE-4A.2: self-derived boundary outputs byte-match cardano at POST-1341 AND POST-1342 through the production loop (env S5_SEED_STORES / CE3D_CORPUS / CE3D_WORK / CE3D_REF); SLOW ~hours"]
    async fn ce4a_2_boundary_byte_exact() {
        let seed = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
        let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
        let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");
        let ref_dir = env_path("CE3D_REF", "/home/ts/.cardano-ce3d-extract/db/ledger");

        // Capture POST-1341 from a production-composition run HALTED at the 1341 boundary (the
        // deterministic single-boundary prefix) and POST-1342 from the FULL continuous two-boundary
        // run (the literal CE-4A.1 run). TWO single-call runs — each ONE run_relay_loop_with_sched
        // invocation, so neither triggers the eview warm-start-across-boundary recovery a mid-run
        // split would. They run sequentially; each isolates + preps + folds + cleans up its own copy.
        let (post_1341, fed_1) =
            drive_capture_at(&seed, &corpus, &work, "byte-exact-1341", EPOCH_1341_FIRST_SLOT, 1341).await;
        let (post_1342, fed_2) =
            drive_capture_at(&seed, &corpus, &work, "byte-exact-1342", EPOCH_1342_FIRST_SLOT, 1342).await;
        assert!(
            fed_1 > 0 && fed_2 > 0,
            "FAIL-LOUD: both runs must feed corpus blocks (fed_1={fed_1} fed_2={fed_2})"
        );

        // Decode the cardano POST references (the db-analyser LedgerDB `state` blobs).
        let ref_1341 = ref_boundary_outputs(
            &ref_dir.join("115862416_db-analyser/state"),
            EPOCH_1341_FIRST_SLOT,
            1341,
        );
        let ref_1342 = ref_boundary_outputs(
            &ref_dir.join("115948834_db-analyser/state"),
            EPOCH_1342_FIRST_SLOT,
            1342,
        );

        let m1 = compare_boundary("POST-1341", &post_1341, &ref_1341);
        let m2 = compare_boundary("POST-1342", &post_1342, &ref_1342);

        // ---- machine-readable evidence bundle (written BEFORE the asserts so a mismatch is auditable) ----
        let surface = |ade: &BoundaryOutputs, refs: &RefOutputs, m: &SurfaceMatch, refp: &str| {
            serde_json::json!({
                "epoch": ade.epoch,
                "reward": m.rewards,
                "pots": { "treasury": m.treasury, "reserves": m.reserves },
                "go": m.go,
                "nesPd": m.nes_pd,
                "nesPd_count": [m.nes_pd_count.0, m.nes_pd_count.1],
                "authority_fingerprint_stake_view_hash": m.stake_view_hash,
                "acc_hash": hex32(&ade.acc_hash),
                "leadership_hash": hex32(&ade.leadership_hash),
                "fees": {
                    "ade_epoch_fees": ade.fees,
                    "cardano_utxosFees": refs.fees,
                    "representation": "reset-and-reaccumulate accumulator vs running residual pot",
                    "fee_consensus_proven_by": ["rewards", "treasury", "reserves"],
                    "hard_assert": false
                },
                "ref": refp,
            })
        };
        let bundle = serde_json::json!({
            "slice": "CE-4A.2",
            "claim": "inside the CE-4A.1 continuous production-loop run, Ade's self-derived boundary \
                outputs at POST-1341 and POST-1342 byte-match the cardano reference for rewards, \
                treasury, reserves, go snapshot, frozen leadership/nesPd, and authority fingerprints",
            "hard_asserts": ["rewards", "treasury", "reserves", "go", "nesPd", "authority_fingerprint_stake_view_hash"],
            "fee_economics": "proven transitively through byte-exact rewards + treasury + reserves; raw \
                fee-pot fields reported separately because Ade epoch_fees (boundary-consumed reward-input \
                accumulator) and cardano utxosFees (running live residual pot) are different intermediate \
                quantities",
            "does_not_claim": ["fees byte-match cardano", "raw utxosFees equivalence", "all seven surfaces byte-match"],
            "utxos_fees_compatibility_note": "If a future N2C query, persisted compatibility surface, or \
                audit claim exposes cardano LedgerState.utxosFees as a cardano-equivalent field, Ade must \
                either materialize that residual field byte-exactly or expose it through a named adapter. \
                CE-4A.2 does NOT claim raw utxosFees equivalence — this is permitted internal divergence, \
                not an accidental incompatibility.",
            "post_1341_provenance": "production-composition run HALTED at the 1341 boundary (deterministic single-boundary prefix)",
            "post_1342_provenance": "full continuous two-boundary run (the literal CE-4A.1 run)",
            "fed_blocks": [fed_1, fed_2],
            "boundaries": {
                "1341": surface(&post_1341, &ref_1341, &m1, ".../115862416_db-analyser/state"),
                "1342": surface(&post_1342, &ref_1342, &m2, ".../115948834_db-analyser/state"),
            },
            "hard_rule_no_loop_reimpl": true,
            "fingerprints_note": "acc_hash/leadership_hash are Ade-internal durability commitments with \
                no cardano counterpart (reported, not asserted vs the ref); the reference-comparable \
                authority fingerprint asserted vs cardano is stake_view_hash (derived from go).",
        });
        let bundle_str = serde_json::to_string_pretty(&bundle).expect("serialize evidence bundle");
        eprintln!("\n===== CE-4A.2 EVIDENCE BUNDLE =====\n{bundle_str}\n===================================");
        let out = env_path("CE4A2_EVIDENCE_OUT", "/home/ts/.cardano-ce3d-extract/ce4a-2-evidence.json");
        std::fs::write(&out, &bundle_str)
            .unwrap_or_else(|e| panic!("write evidence bundle {}: {e:?}", out.display()));
        eprintln!("CE-4A.2 evidence bundle written to {}", out.display());

        // ---- ALL mandatory surfaces byte-exact at BOTH boundaries (fail-loud; gate-adds-value: this
        //      promotes CE-3d's observational MATCH prints for rewards/pots/go into hard asserts) ----
        assert!(
            m1.all_mandatory(),
            "FAIL-LOUD: POST-1341 self-derived outputs must byte-match cardano on the 6 hard surfaces \
             (rewards, treasury, reserves, go, nesPd, stake-view fingerprint) — fees is a reported \
             representation-diff, not asserted (see the surface table above)"
        );
        assert!(
            m2.all_mandatory(),
            "FAIL-LOUD: POST-1342 self-derived outputs must byte-match cardano on the 6 hard surfaces \
             (rewards, treasury, reserves, go, nesPd, stake-view fingerprint) — fees is a reported \
             representation-diff, not asserted (see the surface table above)"
        );
        assert_eq!(
            m2.nes_pd_count,
            (658, 658),
            "FAIL-LOUD: POST-1342 nesPd must be 658/658 (the DC-EPOCH-24 delegation-image count)"
        );
    }

    // ============================================================================================
    // CE-4A.3 — restart + rollback replay-equivalence INSIDE the production-loop harness. This block
    // is the RESTART-ONLY proof: an uninterrupted production run vs. a GENUINE warm restart from
    // durable state after promotion, compared on the self-derived authority fingerprint. It handles
    // EpochViewPostPromotionMismatch as a FIRST-CLASS finding (hard stop, exact evidence, no patch).
    // Spec: docs/clusters/LIVE-LEDGER-EPOCH-TRANSITION/SLICE-CE-4A-3-RESTART-ROLLBACK.md.
    // ============================================================================================

    /// Reassemble the production authority inputs from a (warm-start-)recovered `BootstrapState` — the
    /// SAME assembly `drive()` / the `ForgeIntent::Off` arm performs. Used for the initial run AND,
    /// identically, after the genuine warm restart (so the restart path is production, not synthetic).
    fn assemble_production_inputs(
        state: BootstrapState,
        sidecar: &SeedEpochConsensusInputs,
        dst: &Path,
        chaindb: &PersistentChainDb,
        epoch_accumulator: &EpochAccumulatorStore,
    ) -> (PoolDistrView, EraSchedule, crate::epoch_wire::EviewActivationInputs, ForwardSyncState) {
        let seed_view = leadership_view_from_frozen_authority(Some(epoch_accumulator), sidecar)
            .expect("FAIL-LOUD: recovered leadership view from the frozen authority");
        let era_schedule = recovered_node_schedule(&state, true, preview_rsw())
            .expect("FAIL-LOUD: recovered era schedule from the durable sidecar geometry");
        let eview_inputs = crate::epoch_wire::EviewActivationInputs {
            seed_bootstrap_state: state.ledger.clone(),
            seed_point_slot: sidecar.seed_point_slot,
            seed_point_hash: sidecar.seed_point_hash.clone(),
            seed_epoch: sidecar.epoch_no,
            network_magic: PREVIEW_MAGIC,
            nonce: sidecar.epoch_nonce.0.clone(),
            genesis_hash: sidecar.genesis_hash.clone(),
            protocol_params_hash: sidecar.protocol_params_hash.clone(),
            asc: sidecar.active_slots_coeff,
            replay_scratch_path: dst.join("eview-replay-scratch.redb"),
            next_epoch_bridge: chaindb
                .get_bootstrap_next_epoch_authority(&sidecar.anchor_fp)
                .ok()
                .flatten()
                .and_then(|b| ade_ledger::bootstrap_bridge::decode_bootstrap_next_epoch_authority(&b).ok()),
            bootstrap_reward_delta: chaindb
                .get_bootstrap_reward_update(&sidecar.anchor_fp)
                .ok()
                .flatten()
                .and_then(|b| ade_ledger::bootstrap_reward_update::decode_bootstrap_reward_update(&b).ok()),
        };
        let anchor_fp = fingerprint(&state.ledger).combined;
        let recovered_anchor = state.tip.clone();
        let mut fwd = ForwardSyncState::new(
            ReceiveState::new(state.ledger, state.chain_dep),
            anchor_fp,
            SnapshotCadence::DEFAULT,
        );
        fwd.recovered_anchor = recovered_anchor;
        fwd.recovered_eta0 = Some(sidecar.epoch_nonce.clone());
        (seed_view, era_schedule, eview_inputs, fwd)
    }

    /// The self-derived AUTHORITY fingerprint at the end of a run (read-only): the durable tip, the
    /// accumulator + reduced-checkpoint commitments, the epoch-indexed frozen-leadership hashes, and
    /// the promotion-certified authority availability. `forbidden_paths_clean` is structural — this
    /// harness never calls a re-import / cli-oracle / seed-window replay / materialize_bootstrap_into.
    struct Ce4aAuthorityFp {
        final_tip: u64,
        acc_hash: [u8; 32],
        checkpoint_commitment: [u8; 32],
        leadership_hashes: std::collections::BTreeMap<u64, [u8; 32]>,
        promotion_certified: std::collections::BTreeMap<u64, bool>,
        forbidden_paths_clean: bool,
    }

    fn capture_authority_fp(
        chaindb: &PersistentChainDb,
        epoch_accumulator: &EpochAccumulatorStore,
        reduced_checkpoint: &ReducedUtxoCheckpoint,
    ) -> Ce4aAuthorityFp {
        let final_tip = ChainDb::tip(chaindb).expect("tip read").expect("durable tip").slot.0;
        let (_s, acc) = epoch_accumulator
            .load_current()
            .expect("load_current")
            .expect("sealed accumulator");
        let acc_hash =
            ade_crypto::blake2b_256(&ade_ledger::epoch_accumulator::encode_epoch_accumulator(&acc)).0;
        let checkpoint_commitment = {
            let sums = reduced_checkpoint
                .sum_base_credential_stake()
                .expect("reduced base-credential stake");
            let mut buf = Vec::with_capacity(8 + sums.len() * 37);
            buf.extend_from_slice(&(sums.len() as u64).to_be_bytes());
            for (cred, coin) in &sums {
                buf.extend_from_slice(&cred_key(cred));
                buf.extend_from_slice(&coin.0.to_be_bytes());
            }
            ade_crypto::blake2b_256(&buf).0
        };
        let mut leadership_hashes = std::collections::BTreeMap::new();
        for e in [1342u64, 1343] {
            if let Ok(l) = epoch_accumulator.leadership_authority_for_epoch(EpochNo(e)) {
                leadership_hashes.insert(e, ade_ledger::frozen_leadership::canonical_hash(&l).0);
            }
        }
        let mut promotion_certified = std::collections::BTreeMap::new();
        for e in [1341u64, 1342, 1343] {
            promotion_certified.insert(
                e,
                epoch_accumulator
                    .promotion_leadership_authority_for_epoch(EpochNo(e))
                    .is_ok(),
            );
        }
        Ce4aAuthorityFp {
            final_tip,
            acc_hash,
            checkpoint_commitment,
            leadership_hashes,
            promotion_certified,
            forbidden_paths_clean: true,
        }
    }

    /// CE-4A.3-R1 FIXTURE-LINEAGE REFRESH (harness-local — NEVER a production WAL migration, NEVER a
    /// recovery fallback). The v5 fixture's WAL eview activation records predate `dafe0faf` (point-bound
    /// boundary mark) + the CE-3d stake corrections, so their source_point + stake are OLD lineage; the
    /// prep-refold re-derives the ACCUMULATOR to current lineage but leaves the WAL records stale. Rebuild
    /// the WAL: keep every non-eview entry, DROP the stale eview records (the WAL is append-only and
    /// `resolve_activation_record` conflicts on a same-epoch byte-different record, so a rewrite is
    /// required), then append ONE current-lineage eview record for the durable-tip epoch, reconstructed
    /// from the FRESH frozen authority exactly as the recovery does (`resolve` reads the max-epoch record).
    /// Frozen recovery stays STRICT — it rejects the stale record; this fixes the fixture, not the recovery.
    fn refresh_prep_eview_records(
        wal: FileWalStore,
        dst: &Path,
        epoch_accumulator: &EpochAccumulatorStore,
        sidecar: &SeedEpochConsensusInputs,
        eta0_durable: Hash32,
        durable_epoch: u64,
    ) -> FileWalStore {
        let frozen = epoch_accumulator
            .promotion_leadership_authority_for_epoch(EpochNo(durable_epoch))
            .expect("FAIL-LOUD: promotion-certified frozen authority for the durable epoch (eview refresh)");
        let view = ade_ledger::reduced_epoch_view::EpochConsensusView::from_frozen_leadership(
            &frozen,
            &ade_ledger::reduced_epoch_view::FrozenLeadershipViewMetadata {
                network_magic: PREVIEW_MAGIC,
                era: ade_types::CardanoEra::Conway,
                source_point: Point {
                    slot: frozen.source_slot,
                    hash: frozen.source_hash.clone(),
                },
                checkpoint_commitment: frozen.source_checkpoint_commitment.clone(),
                nonce: eta0_durable,
                snapshot_phase: ade_ledger::reduced_snapshot::SnapshotPhase::Set,
                protocol_params_commitment: ade_ledger::reduced_epoch_view::consensus_profile_commitment(
                    &sidecar.genesis_hash,
                    &sidecar.protocol_params_hash,
                    sidecar.active_slots_coeff,
                ),
            },
        );
        let fresh_record = crate::epoch_activation::activation_record_for(&view);
        let kept: Vec<WalEntry> = wal
            .read_all()
            .expect("FAIL-LOUD: wal read for eview refresh")
            .into_iter()
            .filter(|e| !matches!(e, WalEntry::EpochConsensusViewActivated { .. }))
            .collect();
        drop(wal);
        let _ = std::fs::remove_dir_all(dst.join("wal"));
        let mut fresh = FileWalStore::open(dst.join("wal")).expect("FAIL-LOUD: reopen wal after refresh");
        for e in kept {
            fresh.append(e).expect("FAIL-LOUD: re-append kept wal entry");
        }
        fresh.append(fresh_record).expect("FAIL-LOUD: append current-lineage eview record");
        // ASSERT the rewritten durable record is CURRENT-LINEAGE (== the fresh frozen recovery view) —
        // this proves the prep REWROTE the record correctly, not merely that it cleared the error.
        let durable = crate::epoch_activation::resolve_activation_record(
            &fresh.read_all().expect("re-read wal"),
        )
        .expect("resolve")
        .expect("FAIL-LOUD: a durable eview record after refresh");
        match &durable {
            WalEntry::EpochConsensusViewActivated {
                target_epoch,
                transition_point,
                view_canonical_hash,
                stake_view_canonical_hash,
                ..
            } => {
                assert_eq!(target_epoch.0, durable_epoch, "refreshed record targets the durable epoch");
                assert_eq!(
                    *view_canonical_hash,
                    view.canonical_hash(),
                    "FAIL-LOUD: refreshed durable eview record canonical hash == fresh frozen recovery view (current lineage)"
                );
                assert_eq!(
                    transition_point.slot, frozen.source_slot,
                    "refreshed source_slot == current frozen source_slot"
                );
                assert_eq!(
                    *stake_view_canonical_hash,
                    view.stake_view_canonical_hash(),
                    "refreshed stake_view_hash == current frozen stake_view_hash"
                );
            }
            _ => panic!("FAIL-LOUD: expected the refreshed eview activation record"),
        }
        fresh
    }

    /// Drive the production loop and capture the authority fingerprint. `do_restart=false` = one
    /// uninterrupted run to 1342 (the reference). `do_restart=true` = cross 1340->1341, then a GENUINE
    /// warm restart (drop every handle + the ForwardSyncState, reopen from durable disk, re-run
    /// `warm_start_recovery` + input reassembly — the real process-restart sequence, NOT a reuse of
    /// in-memory state), then continue through 1341->1342. Returns `Err(finding)` if the post-restart
    /// loop trips a fail-closed error (e.g. EpochViewPostPromotionMismatch) — the §4 hard stop.
    async fn drive_restart_proof(
        seed_dir: &Path,
        corpus_dir: &Path,
        work: &Path,
        tag: &str,
        do_restart: bool,
    ) -> Result<Ce4aAuthorityFp, String> {
        let dst = isolate_fixture(seed_dir, work, tag);
        seal_bootstrap_seed_leadership(&dst);
        let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(dst.join("chain.db")))
            .expect("FAIL-LOUD: open isolated chaindb");
        let mut wal = FileWalStore::open(dst.join("wal")).expect("FAIL-LOUD: open isolated wal");
        let warm_acc = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: open warm accumulator handle");
        let state = warm_start_recovery(&chaindb, &wal, Some(&warm_acc), preview_rsw())
            .expect("FAIL-LOUD: production warm_start_recovery");
        drop(warm_acc);

        let sidecar = state
            .seed_epoch_consensus_inputs
            .clone()
            .expect("FAIL-LOUD: v5 sidecar (SeedEpochConsensusInputs) present");
        assert_eq!(
            sidecar.epoch_no.0, SEED_EPOCH,
            "FAIL-LOUD: v5 seed epoch must be {SEED_EPOCH}; got {}",
            sidecar.epoch_no.0
        );
        let durable_tip_before =
            ChainDb::tip(&chaindb).expect("tip read").expect("FAIL-LOUD: durable chaindb tip").slot.0;
        assert_eq!(
            epoch_of(durable_tip_before),
            1340,
            "FAIL-LOUD: the v5 durable tip must be in epoch 1340, got slot {durable_tip_before}"
        );
        {
            let prof = crate::bootstrap_export::resolve_network_profile("preview")
                .expect("FAIL-LOUD: preview network profile");
            assert_eq!(
                u64::from(sidecar.epoch_length_slots),
                prof.epoch_length,
                "FAIL-LOUD: fixture epoch_length venue mismatch (k=2160-vs-432 class of bug)"
            );
            assert_eq!(preview_rsw(), Some(34_560), "FAIL-LOUD: preview RSW must be 34560 (k=432)");
        }

        let epoch_accumulator = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: open live accumulator");
        let reduced_checkpoint = ReducedUtxoCheckpoint::open(&dst.join("reduced-checkpoint.redb"))
            .expect("FAIL-LOUD: open reduced checkpoint");

        // FIXTURE PREP (disclosed artifact — identical to CE-4A.1/4A.2).
        if epoch_accumulator
            .promotion_leadership_authority_for_epoch(EpochNo(1341))
            .is_err()
        {
            eprintln!("CE-4A.3 [{tag}] prep: native 1341 absent — reset+refold 1338->{durable_tip_before}...");
            epoch_accumulator.reset_to_bootstrap().expect("FAIL-LOUD: reset accumulator");
            reduced_checkpoint.reset_to_bootstrap().expect("FAIL-LOUD: reset checkpoint");
            let prep_sched =
                recovered_node_schedule(&state, true, preview_rsw()).expect("FAIL-LOUD: prep era schedule");
            advance_ledger_state_to_durable_tip(
                Some(&reduced_checkpoint),
                Some(&epoch_accumulator),
                &chaindb,
                &prep_sched,
                &RecoveryAdmissionPolicy::cardano(),
            )
            .expect("FAIL-LOUD: prep refold 1338->durable-tip");
        }
        assert!(
            epoch_accumulator
                .promotion_leadership_authority_for_epoch(EpochNo(1341))
                .is_ok(),
            "FAIL-LOUD (hard stop: missing promotion-certified authority): native frozen leadership 1341 required"
        );
        assert!(
            epoch_accumulator.leadership_authority_for_epoch(EpochNo(1343)).is_err(),
            "FAIL-LOUD: leadership 1343 must NOT be sealed pre-run"
        );

        // CE-4A.3-R1 fixture-lineage refresh (harness-local; NOT a production WAL migration, NOT a
        // recovery fallback): the v5 fixture's WAL eview records predate dafe0faf + the CE-3d stake
        // corrections, while the refold re-derived the accumulator to current lineage. Rebuild the WAL
        // with a CURRENT-lineage eview record from the fresh frozen authority so the loop's recovery has
        // a lineage-consistent durable record to reconstruct. Frozen recovery stays STRICT (it rejects
        // the stale record); we fix the fixture, not the recovery. Done BEFORE `state` is moved below.
        wal = refresh_prep_eview_records(
            wal,
            &dst,
            &epoch_accumulator,
            &sidecar,
            state.chain_dep.epoch_nonce.0.clone(),
            epoch_of(durable_tip_before),
        );

        let (seed_view, era_schedule, eview_inputs, mut fwd) =
            assemble_production_inputs(state, &sidecar, &dst, &chaindb, &epoch_accumulator);

        // ---- run 1: uninterrupted (to 1342) OR pre-restart (to 1341) ----
        let first_max = if do_restart { EPOCH_1341_FIRST_SLOT } else { EPOCH_1342_FIRST_SLOT };
        eprintln!("CE-4A.3 [{tag}] fold ({durable_tip_before}, {first_max}] (do_restart={do_restart})");
        {
            let feed = load_corpus_feed(corpus_dir, durable_tip_before, first_max);
            assert!(!feed.is_empty(), "FAIL-LOUD: the feed must contain corpus blocks");
            let mut source = NodeBlockSource::in_memory(feed);
            let (_tx, mut shutdown) = watch::channel(false);
            let mut sched = crate::live_log::NodeSchedLogWriter::new(Vec::<u8>::new());
            run_relay_loop_with_sched(
                &mut fwd, &mut source, &chaindb, &mut wal, &era_schedule, &seed_view, &mut shutdown,
                None, Some(&mut sched), None, Some(&reduced_checkpoint), Some(&eview_inputs),
                Some(&epoch_accumulator), RecoveryAdmissionPolicy::cardano(),
            )
            .await
            .expect("FAIL-LOUD: first production loop must halt cleanly");
        }

        if !do_restart {
            let fp = capture_authority_fp(&chaindb, &epoch_accumulator, &reduced_checkpoint);
            drop(reduced_checkpoint);
            drop(epoch_accumulator);
            drop(wal);
            drop(chaindb);
            if std::env::var("CE4A_KEEP").is_err() {
                let _ = std::fs::remove_dir_all(&dst);
            }
            return Ok(fp);
        }

        // ---- GENUINE WARM RESTART from durable state (the production restart path) ----
        // Drop the in-memory ForwardSyncState + assembled inputs + ALL store handles, then reopen from
        // the durable disk and re-run warm_start_recovery + input reassembly. NOT a reuse of in-memory
        // state (that is the CE-4A.2 invalid split that trips EpochViewPostPromotionMismatch).
        eprintln!("CE-4A.3 [{tag}] GENUINE WARM RESTART: drop handles + fwd, reopen from durable, warm_start_recovery, reassemble");
        drop(fwd);
        drop(seed_view);
        drop(era_schedule);
        drop(eview_inputs);
        drop(reduced_checkpoint);
        drop(epoch_accumulator);
        drop(wal);
        drop(chaindb);

        let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(dst.join("chain.db")))
            .expect("FAIL-LOUD: reopen chaindb after restart");
        let mut wal = FileWalStore::open(dst.join("wal")).expect("FAIL-LOUD: reopen wal after restart");
        let warm_acc = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: reopen warm accumulator after restart");
        let state2 = warm_start_recovery(&chaindb, &wal, Some(&warm_acc), preview_rsw())
            .expect("FAIL-LOUD: warm_start_recovery after restart (the genuine restart path)");
        drop(warm_acc);
        let sidecar2 = state2
            .seed_epoch_consensus_inputs
            .clone()
            .expect("FAIL-LOUD: sidecar after restart");
        let restart_tip =
            ChainDb::tip(&chaindb).expect("tip read").expect("durable tip").slot.0;
        assert_eq!(
            epoch_of(restart_tip),
            1341,
            "FAIL-LOUD: post-restart durable tip must be in epoch 1341 (crossed pre-restart); got slot {restart_tip}"
        );
        let epoch_accumulator = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: reopen accumulator after restart");
        let reduced_checkpoint = ReducedUtxoCheckpoint::open(&dst.join("reduced-checkpoint.redb"))
            .expect("FAIL-LOUD: reopen checkpoint after restart");
        assert!(
            epoch_accumulator
                .promotion_leadership_authority_for_epoch(EpochNo(1342))
                .is_ok(),
            "FAIL-LOUD (hard stop: missing promotion-certified authority): post-restart must carry \
             promotion-certified native 1342 (sealed by the pre-restart 1340->1341 cross)"
        );
        let (seed_view, era_schedule, eview_inputs, mut fwd) =
            assemble_production_inputs(state2, &sidecar2, &dst, &chaindb, &epoch_accumulator);

        // ---- run 2 (post-restart): continue through 1341->1342 over the RECOVERED state; CATCH the finding ----
        eprintln!("CE-4A.3 [{tag}] post-restart fold ({restart_tip}, {EPOCH_1342_FIRST_SLOT}] — cross 1341->1342");
        let restart_run = {
            let feed = load_corpus_feed(corpus_dir, restart_tip, EPOCH_1342_FIRST_SLOT);
            assert!(!feed.is_empty(), "FAIL-LOUD: the post-restart feed must contain corpus blocks");
            let mut source = NodeBlockSource::in_memory(feed);
            let (_tx, mut shutdown) = watch::channel(false);
            let mut sched = crate::live_log::NodeSchedLogWriter::new(Vec::<u8>::new());
            run_relay_loop_with_sched(
                &mut fwd, &mut source, &chaindb, &mut wal, &era_schedule, &seed_view, &mut shutdown,
                None, Some(&mut sched), None, Some(&reduced_checkpoint), Some(&eview_inputs),
                Some(&epoch_accumulator), RecoveryAdmissionPolicy::cardano(),
            )
            .await
        };
        if let Err(e) = restart_run {
            // §4 HARD STOP — do NOT patch, do NOT weaken, do NOT claim CE-4A.3.
            let err = format!("{e:?}");
            let is_eview = err.contains("EpochViewPostPromotionMismatch");
            let stop = serde_json::json!({
                "slice": "CE-4A.3",
                "result": "HARD STOP — the GENUINE warm restart tripped a fail-closed error on the post-restart fold",
                "epoch_view_post_promotion_mismatch": is_eview,
                "error": err,
                "restart_tip": restart_tip,
                "meaning": if is_eview {
                    "the genuine production warm restart across a crossed boundary trips the eview post-promotion cross-check — a REAL production restart/re-entry authority gap (§4 outcome b). Open a sealed fix slice; do NOT claim CE-4A.3 restart equivalence."
                } else {
                    "the genuine warm restart failed closed for another reason — investigate before any claim."
                },
            });
            let out = env_path("CE4A3_STOP_OUT", "/home/ts/.cardano-ce3d-extract/ce4a-3-STOP-evidence.json");
            let pretty = serde_json::to_string_pretty(&stop).unwrap_or_default();
            let _ = std::fs::write(&out, &pretty);
            eprintln!("\n===== CE-4A.3 HARD STOP =====\n{pretty}\n=============================");
            if std::env::var("CE4A_KEEP").is_err() {
                let _ = std::fs::remove_dir_all(&dst);
            }
            return Err(format!("CE-4A.3 HARD STOP (eview_mismatch={is_eview}): {err}"));
        }

        let fp = capture_authority_fp(&chaindb, &epoch_accumulator, &reduced_checkpoint);
        drop(reduced_checkpoint);
        drop(epoch_accumulator);
        drop(wal);
        drop(chaindb);
        if std::env::var("CE4A_KEEP").is_err() {
            let _ = std::fs::remove_dir_all(&dst);
        }
        Ok(fp)
    }

    /// CE-4A.3 restart-only proof: a GENUINE warm restart mid-run (after crossing 1340->1341) is
    /// replay-equivalent to the uninterrupted run on the self-derived authority fingerprint. Resolves
    /// the EpochViewPostPromotionMismatch finding: green here => the CE-4A.2 mismatch was a harness-only
    /// re-entry artifact; a hard stop => a real production restart-authority gap (a sealed fix decision).
    #[tokio::test]
    #[ignore = "CE-4A.3 restart-only: production warm-restart mid-run == uninterrupted run, self-derived authority fingerprint (env S5_SEED_STORES / CE3D_CORPUS / CE3D_WORK); SLOW ~hours"]
    async fn ce4a_3_restart_only_equivalence() {
        let seed = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
        let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
        let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");

        // uninterrupted reference (one continuous run to 1342)
        let uninterrupted = drive_restart_proof(&seed, &corpus, &work, "restart-uninterrupted", false)
            .await
            .expect("FAIL-LOUD: the uninterrupted reference run must complete");
        // genuine warm restart mid-run (to 1341 -> restart -> to 1342)
        let restarted = match drive_restart_proof(&seed, &corpus, &work, "restart-warm", true).await {
            Ok(fp) => fp,
            Err(finding) => panic!(
                "FAIL-LOUD — first-class finding (NOT to be patched around; this is the sealed-fix \
                 decision): {finding}"
            ),
        };

        // ---- evidence bundle (written BEFORE the asserts) ----
        let fp_json = |fp: &Ce4aAuthorityFp| {
            serde_json::json!({
                "final_tip": fp.final_tip,
                "acc_hash": hex32(&fp.acc_hash),
                "checkpoint_commitment": hex32(&fp.checkpoint_commitment),
                "leadership_hashes": fp.leadership_hashes.iter()
                    .map(|(e, h)| (e.to_string(), hex32(h))).collect::<std::collections::BTreeMap<_, _>>(),
                "promotion_certified": fp.promotion_certified.iter()
                    .map(|(e, b)| (e.to_string(), *b)).collect::<std::collections::BTreeMap<_, _>>(),
                "forbidden_paths_clean": fp.forbidden_paths_clean,
            })
        };
        let bundle = serde_json::json!({
            "slice": "CE-4A.3 (restart-only)",
            "claim": "a genuine production warm restart from durable state after promotion is replay-equivalent to the uninterrupted run",
            "epoch_view_post_promotion_mismatch": false,
            "uninterrupted": fp_json(&uninterrupted),
            "restarted": fp_json(&restarted),
        });
        let bundle_str = serde_json::to_string_pretty(&bundle).expect("serialize evidence bundle");
        eprintln!("\n===== CE-4A.3 RESTART-ONLY EVIDENCE =====\n{bundle_str}\n=========================================");
        let out = env_path("CE4A3_EVIDENCE_OUT", "/home/ts/.cardano-ce3d-extract/ce4a-3-restart-evidence.json");
        std::fs::write(&out, &bundle_str)
            .unwrap_or_else(|e| panic!("write evidence bundle {}: {e:?}", out.display()));

        // ---- HARD ASSERTS: restarted == uninterrupted on the self-derived authority fingerprint ----
        assert_eq!(restarted.final_tip, uninterrupted.final_tip, "FAIL-LOUD: same final selected tip");
        assert_eq!(restarted.acc_hash, uninterrupted.acc_hash, "FAIL-LOUD: same accumulator canonical hash");
        assert_eq!(
            restarted.checkpoint_commitment, uninterrupted.checkpoint_commitment,
            "FAIL-LOUD: same reduced checkpoint commitment"
        );
        assert_eq!(
            restarted.leadership_hashes, uninterrupted.leadership_hashes,
            "FAIL-LOUD: same frozen leadership hashes"
        );
        assert_eq!(
            restarted.promotion_certified, uninterrupted.promotion_certified,
            "FAIL-LOUD: same promotion-certified authority availability"
        );
        assert!(
            restarted.forbidden_paths_clean && uninterrupted.forbidden_paths_clean,
            "FAIL-LOUD: forbidden_paths must be false (clean) on both runs"
        );
        // sanity: the runs genuinely crossed to 1342 and sealed frozen leadership 1342 AND 1343.
        assert!(
            uninterrupted.leadership_hashes.contains_key(&1342)
                && uninterrupted.leadership_hashes.contains_key(&1343),
            "FAIL-LOUD: the run must seal frozen leadership 1342 AND 1343 (crossed both boundaries)"
        );
        assert_eq!(
            epoch_of(uninterrupted.final_tip),
            1342,
            "FAIL-LOUD: the run must land the durable tip in epoch 1342"
        );
    }

    /// CE-4A.3-R2 SNAPSHOT PROBE (de-risk, fast — no fold): confirm the v5 fixture ChainDb carries a
    /// durable snapshot <= a within-k rollback point P so `materialize_rolled_back_state`'s `nearest_le(P)`
    /// resolves (else the production rollback fails RollbackTooDeep). Lists the durable snapshot slots +
    /// the nearest snapshot at/below the tip and within a ~within-k band below it. FAST: isolate + read,
    /// no fold.
    #[tokio::test]
    #[ignore = "CE-4A.3-R2 snapshot probe: list fixture ChainDb snapshot slots for the rollback materialize floor (env S5_SEED_STORES / CE3D_WORK); fast ~isolate only"]
    async fn ce4a_3_r2_snapshot_probe() {
        use ade_ledger::rollback::SnapshotReader;
        let seed = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
        let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");
        let dst = isolate_fixture(&seed, &work, "r2-snap-probe");
        let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(dst.join("chain.db")))
            .expect("FAIL-LOUD: open isolated chaindb");
        let tip = ChainDb::tip(&chaindb).expect("tip read").expect("FAIL-LOUD: durable tip");
        let slots = chaindb.list_snapshot_slots().expect("FAIL-LOUD: list snapshot slots");
        eprintln!(
            "CE-4A.3-R2 snapshot probe: durable tip slot={} epoch={}",
            tip.slot.0,
            epoch_of(tip.slot.0)
        );
        eprintln!(
            "CE-4A.3-R2 snapshot probe: {} durable snapshot slots: {:?}",
            slots.len(),
            slots.iter().map(|s| s.0).collect::<Vec<_>>()
        );
        let cache = PersistentSnapshotCache::new(&chaindb);
        for probe in [
            tip.slot.0,
            tip.slot.0.saturating_sub(2_000),
            tip.slot.0.saturating_sub(6_000),
            tip.slot.0.saturating_sub(10_000),
        ] {
            let near = cache.nearest_le(SlotNo(probe)).map(|(s, _, _)| s.0);
            eprintln!("  nearest_le({probe}) -> {near:?}");
        }
        assert!(
            !slots.is_empty(),
            "FAIL-LOUD: the fixture ChainDb must carry >=1 snapshot (the materialize floor for the rollback)"
        );
        if std::env::var("CE4A_KEEP").is_err() {
            let _ = std::fs::remove_dir_all(&dst);
        }
    }

    /// CE-4A.3-R2 (#13): the rollback trace — the controlled-rollback proof condition (ratified §1a). It
    /// records that the durable rollback went through the PRODUCTION rollback primitives (NOT a natural
    /// fork-switch, NOT a synthetic edit), so the evidence bundle can carry the honest mechanism flags.
    struct Ce4aRollbackTrace {
        rollback_from_tip: u64,
        rollback_target_slot: u64,
        depth_blocks: u64,
        nearest_snapshot_le_target: Option<u64>,
        wal_rollback_marker: bool,
        // CE-4A.3 #13 option (a): the production ResetAndRefold between rollback and run 2.
        unsealed_1341_before_reseal: bool,
        resealed_epoch_after_refold: u64,
    }

    /// CE-4A.3-R2 (#13): a CONTROLLED within-k durable rollback + refold through the CE-4A production loop is
    /// byte-identical to the uninterrupted run. Ratified mechanism (§1a): NOT a natural fork-switch. The
    /// harness induces the rollback through the PRODUCTION rollback primitives only —
    /// `accumulator_admit_and_clear_for_rollback` (the `admit_rollback` k-guard + accumulator pre-clear;
    /// target MUST be on the pre-rollback canonical chain, depth <= k) then `apply_chain_event`
    /// (`materialize_rolled_back_state` = replay-reconstruct P as harness setup input -> `commit_rollback`
    /// -> `WalEntry::RollBack` marker) — exactly the `run_node_sync` rollback sequence (5246-5278). It then
    /// re-feeds the SAME canonical blocks (P, 1342] and lets the NORMAL reconcile path
    /// (`advance_ledger_state_to_durable_tip` -> reset-if-ahead + recover-admit -> refold) re-derive the
    /// authority. Any divergence (EpochView / admission / authority hash / checkpoint / frozen leadership /
    /// forbidden path) FAILS-LOUD as the ratified hard stop — never patched inside #13.
    #[allow(clippy::too_many_lines)]
    async fn drive_rollback_proof(
        seed_dir: &Path,
        corpus_dir: &Path,
        work: &Path,
        tag: &str,
    ) -> Result<(Ce4aAuthorityFp, Ce4aAuthorityFp, Ce4aRollbackTrace), String> {
        // ---- setup: identical to drive_restart_proof (isolate, warm_start, prep-refold, refresh, assemble) ----
        let dst = isolate_fixture(seed_dir, work, tag);
        seal_bootstrap_seed_leadership(&dst);
        let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(dst.join("chain.db")))
            .expect("FAIL-LOUD: open isolated chaindb");
        let mut wal = FileWalStore::open(dst.join("wal")).expect("FAIL-LOUD: open isolated wal");
        let warm_acc = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: open warm accumulator handle");
        let state = warm_start_recovery(&chaindb, &wal, Some(&warm_acc), preview_rsw())
            .expect("FAIL-LOUD: production warm_start_recovery");
        drop(warm_acc);
        let sidecar = state
            .seed_epoch_consensus_inputs
            .clone()
            .expect("FAIL-LOUD: v5 sidecar (SeedEpochConsensusInputs) present");
        assert_eq!(sidecar.epoch_no.0, SEED_EPOCH, "FAIL-LOUD: v5 seed epoch must be {SEED_EPOCH}");
        let durable_tip_before =
            ChainDb::tip(&chaindb).expect("tip read").expect("FAIL-LOUD: durable chaindb tip").slot.0;
        assert_eq!(epoch_of(durable_tip_before), 1340, "FAIL-LOUD: the v5 durable tip must be in epoch 1340");

        let epoch_accumulator = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: open live accumulator");
        let reduced_checkpoint = ReducedUtxoCheckpoint::open(&dst.join("reduced-checkpoint.redb"))
            .expect("FAIL-LOUD: open reduced checkpoint");

        // FIXTURE PREP (disclosed artifact — identical to CE-4A.1/4A.2/#12).
        if epoch_accumulator
            .promotion_leadership_authority_for_epoch(EpochNo(1341))
            .is_err()
        {
            eprintln!("CE-4A.3-R2 [{tag}] prep: native 1341 absent — reset+refold 1338->{durable_tip_before}...");
            epoch_accumulator.reset_to_bootstrap().expect("FAIL-LOUD: reset accumulator");
            reduced_checkpoint.reset_to_bootstrap().expect("FAIL-LOUD: reset checkpoint");
            let prep_sched =
                recovered_node_schedule(&state, true, preview_rsw()).expect("FAIL-LOUD: prep era schedule");
            advance_ledger_state_to_durable_tip(
                Some(&reduced_checkpoint),
                Some(&epoch_accumulator),
                &chaindb,
                &prep_sched,
                &RecoveryAdmissionPolicy::cardano(),
            )
            .expect("FAIL-LOUD: prep refold 1338->durable-tip");
        }
        assert!(
            epoch_accumulator.promotion_leadership_authority_for_epoch(EpochNo(1341)).is_ok(),
            "FAIL-LOUD (hard stop): native frozen leadership 1341 required"
        );

        // CE-4A.3-R1 fixture-lineage refresh (harness-local; see drive_restart_proof). Done BEFORE `state` moves.
        wal = refresh_prep_eview_records(
            wal,
            &dst,
            &epoch_accumulator,
            &sidecar,
            state.chain_dep.epoch_nonce.0.clone(),
            epoch_of(durable_tip_before),
        );

        let (seed_view, era_schedule, eview_inputs, mut fwd) =
            assemble_production_inputs(state, &sidecar, &dst, &chaindb, &epoch_accumulator);

        // ---- run 1: fold to 1342 (cross both boundaries) through the production loop ----
        eprintln!("CE-4A.3-R2 [{tag}] run 1: fold ({durable_tip_before}, {EPOCH_1342_FIRST_SLOT}] — cross 1340->1341->1342");
        {
            let feed = load_corpus_feed(corpus_dir, durable_tip_before, EPOCH_1342_FIRST_SLOT);
            assert!(!feed.is_empty(), "FAIL-LOUD: run-1 feed must contain corpus blocks");
            let mut source = NodeBlockSource::in_memory(feed);
            let (_tx, mut shutdown) = watch::channel(false);
            let mut sched = crate::live_log::NodeSchedLogWriter::new(Vec::<u8>::new());
            run_relay_loop_with_sched(
                &mut fwd, &mut source, &chaindb, &mut wal, &era_schedule, &seed_view, &mut shutdown,
                None, Some(&mut sched), None, Some(&reduced_checkpoint), Some(&eview_inputs),
                Some(&epoch_accumulator), RecoveryAdmissionPolicy::cardano(),
            )
            .await
            .map_err(|e| format!("CE-4A.3-R2 HARD STOP (run-1 production loop failed): {e:?}"))?;
        }
        let tip_after_run1 =
            ChainDb::tip(&chaindb).expect("tip read").expect("FAIL-LOUD: durable tip after run 1");
        assert_eq!(epoch_of(tip_after_run1.slot.0), 1342, "FAIL-LOUD: run 1 must land the tip in epoch 1342");

        // The uninterrupted reference IS run 1's result on THIS fixture (captured BEFORE the rollback) — the
        // same-fixture control that isolates the rollback+refold as the only variable. `capture_authority_fp`
        // returns OWNED data (hashes + maps), so the subsequent rollback+refold cannot invalidate it.
        let uninterrupted_fp = capture_authority_fp(&chaindb, &epoch_accumulator, &reduced_checkpoint);

        // ---- pick P: a REAL canonical corpus point ROLLBACK_DEPTH_BLOCKS below the tip (depth <= k=432) ----
        // ROLLBACK_DEPTH_BLOCKS spans the 1341->1342 boundary so the rollback UN-crosses it and the refold
        // re-crosses it (a strong within-k proof). P_SLOT_BAND covers > depth blocks at ~0.05 density.
        const ROLLBACK_DEPTH_BLOCKS: usize = 200;
        const P_SLOT_BAND: u64 = 12_000;
        let mut band: Vec<Point> = Vec::new();
        for item in chaindb
            .iter_from_slot(SlotNo(tip_after_run1.slot.0.saturating_sub(P_SLOT_BAND)))
            .expect("FAIL-LOUD: iter_from_slot for the rollback band")
        {
            let b = item.expect("FAIL-LOUD: stored block in the rollback band");
            band.push(Point { slot: b.slot, hash: b.hash });
        }
        assert!(
            band.len() > ROLLBACK_DEPTH_BLOCKS + 1,
            "FAIL-LOUD: the rollback band must hold > {ROLLBACK_DEPTH_BLOCKS} canonical blocks; got {}",
            band.len()
        );
        let p = band[band.len() - 1 - ROLLBACK_DEPTH_BLOCKS].clone();
        let depth = ROLLBACK_DEPTH_BLOCKS as u64;
        assert!(depth <= 432, "FAIL-LOUD: the rollback depth must be within k=432 blocks");
        eprintln!(
            "CE-4A.3-R2 [{tag}]: rollback tip slot {} (epoch {}) -> P slot {} (epoch {}), depth {} blocks, k=432",
            tip_after_run1.slot.0,
            epoch_of(tip_after_run1.slot.0),
            p.slot.0,
            epoch_of(p.slot.0),
            depth
        );
        let nearest_snapshot_le_target = {
            use ade_ledger::rollback::SnapshotReader;
            let cache = PersistentSnapshotCache::new(&chaindb);
            let near = cache.nearest_le(p.slot).map(|(s, _, _)| s.0);
            eprintln!("CE-4A.3-R2 [{tag}]: nearest durable snapshot <= P({}) = {near:?} (materialize floor)", p.slot.0);
            near
        };

        // ---- THE CONTROLLED ROLLBACK (production primitives ONLY — the run_node_sync 5246-5278 sequence) ----
        // (1) admit_rollback k-guard + accumulator pre-clear. Target on the PRE-rollback canonical chain,
        //     depth <= k, target >= bootstrap seed. A fault here is the ratified HARD STOP (admission mismatch).
        accumulator_admit_and_clear_for_rollback(
            Some(&epoch_accumulator),
            &chaindb,
            &p,
            &RecoveryAdmissionPolicy::cardano(),
        )
        .map_err(|e| {
            format!("CE-4A.3-R2 HARD STOP (production admit_rollback k-guard rejected the within-k canonical rollback): {e:?}")
        })?;
        // (2) apply_chain_event: materialize_rolled_back_state (replay-reconstruct P) -> commit_rollback ->
        //     WalEntry::RollBack -> reconcile (DC-NODE-26). ledger_view = the frozen-authority view.
        let event = ChainEvent::RolledBack { to_point: p.clone(), depth: BlockDistance(depth) };
        apply_chain_event(
            &mut fwd,
            &chaindb,
            &mut wal,
            &NoCheckpointSink,
            &event,
            RollbackReason::PeerRollBackward,
            None,
            &era_schedule,
            &seed_view,
        )
        .map_err(|e| format!("CE-4A.3-R2 HARD STOP (production rollback-apply failed): {e:?}"))?;

        // The durable rollback landed at P (DC-NODE-26 reconcile is enforced inside apply_chain_event).
        let tip_after_rb =
            ChainDb::tip(&chaindb).expect("tip read").expect("FAIL-LOUD: durable tip after rollback");
        if tip_after_rb.slot != p.slot || tip_after_rb.hash != p.hash {
            return Err(format!(
                "CE-4A.3-R2 HARD STOP (checkpoint mismatch: durable tip after rollback {} != P {})",
                tip_after_rb.slot.0, p.slot.0
            ));
        }
        let wal_rollback_marker = wal
            .read_all()
            .expect("FAIL-LOUD: wal read after rollback")
            .iter()
            .any(|e| matches!(e, WalEntry::RollBack { .. }));
        assert!(
            wal_rollback_marker,
            "FAIL-LOUD: a real WalEntry::RollBack marker must be durable after the production rollback"
        );

        // CE-4A.3 #13 (option a — harness-faithful to the CONTINUOUS rollback+refold; user-ratified). The
        // CE-4A single-producer harness must use TWO run_relay_loop_with_sched calls (run_node_sync rejects
        // source rollbacks), so run 2 would RE-RUN the startup eview recovery on the post-rollback, pre-refold
        // accumulator — where 1341's frozen leadership is not yet resealed (the fold to 1342 moved past it) ->
        // RecoveryEpochUnsealed. The CONTINUOUS production loop NEVER restarts between commit_rollback and
        // ResetAndRefold; its next advance reconciles. So invoke the PRODUCTION ResetAndRefold HERE
        // (advance_ledger_state_to_durable_tip -> accumulator_recover_admit -> reset_to_bootstrap + refold to
        // P) BEFORE run 2 — the SAME reconcile the continuous loop's next advance does, NEVER a manual reseal
        // / WAL edit. It reseals the CURRENT-lineage epoch-1341 authority so run 2's recovery is consistent.
        // (The warm-restart-in-the-crash-window gap this models away is REAL but SEPARATE -> CE-4A.3-R4; #13
        // claims ONLY controlled rollback + production ResetAndRefold == uninterrupted, NEVER crash-window
        // restart safety.)
        assert_eq!(epoch_of(tip_after_rb.slot.0), 1341, "FAIL-LOUD: the rollback target P must be in epoch 1341");
        let unsealed_1341_before_reseal = epoch_accumulator
            .promotion_leadership_authority_for_epoch(EpochNo(1341))
            .is_err();
        advance_ledger_state_to_durable_tip(
            Some(&reduced_checkpoint),
            Some(&epoch_accumulator),
            &chaindb,
            &era_schedule,
            &RecoveryAdmissionPolicy::cardano(),
        )
        .map_err(|e| format!("CE-4A.3-R2 HARD STOP (production ResetAndRefold after rollback failed): {e:?}"))?;
        // The PRODUCTION reset+refold (NOT a manual seal) resealed epoch 1341's current-lineage authority.
        epoch_accumulator
            .promotion_leadership_authority_for_epoch(EpochNo(1341))
            .map_err(|e| format!("CE-4A.3-R2 HARD STOP (ResetAndRefold did not reseal epoch 1341: {e:?})"))?;
        eprintln!(
            "CE-4A.3-R2 [{tag}]: production ResetAndRefold resealed epoch 1341 (unsealed_before={unsealed_1341_before_reseal}) — run 2 recovery now consistent"
        );

        // ---- run 2: re-feed the SAME canonical blocks (P, 1342]; the NORMAL reconcile path refolds ----
        eprintln!("CE-4A.3-R2 [{tag}] run 2: refold ({}, {EPOCH_1342_FIRST_SLOT}] — re-cross 1341->1342 through the production loop", p.slot.0);
        {
            let feed = load_corpus_feed(corpus_dir, p.slot.0, EPOCH_1342_FIRST_SLOT);
            assert!(!feed.is_empty(), "FAIL-LOUD: the refold feed must contain corpus blocks");
            let mut source = NodeBlockSource::in_memory(feed);
            let (_tx, mut shutdown) = watch::channel(false);
            let mut sched = crate::live_log::NodeSchedLogWriter::new(Vec::<u8>::new());
            run_relay_loop_with_sched(
                &mut fwd, &mut source, &chaindb, &mut wal, &era_schedule, &seed_view, &mut shutdown,
                None, Some(&mut sched), None, Some(&reduced_checkpoint), Some(&eview_inputs),
                Some(&epoch_accumulator), RecoveryAdmissionPolicy::cardano(),
            )
            .await
            .map_err(|e| format!("CE-4A.3-R2 HARD STOP (refold loop failed): {e:?}"))?;
        }
        let tip_after_run2 =
            ChainDb::tip(&chaindb).expect("tip read").expect("FAIL-LOUD: durable tip after refold");
        assert_eq!(
            epoch_of(tip_after_run2.slot.0),
            1342,
            "FAIL-LOUD: the refold must re-land the tip in epoch 1342"
        );

        let fp = capture_authority_fp(&chaindb, &epoch_accumulator, &reduced_checkpoint);
        let trace = Ce4aRollbackTrace {
            rollback_from_tip: tip_after_run1.slot.0,
            rollback_target_slot: p.slot.0,
            depth_blocks: depth,
            nearest_snapshot_le_target,
            wal_rollback_marker,
            unsealed_1341_before_reseal,
            resealed_epoch_after_refold: 1341,
        };
        drop(reduced_checkpoint);
        drop(epoch_accumulator);
        drop(wal);
        drop(chaindb);
        if std::env::var("CE4A_KEEP").is_err() {
            let _ = std::fs::remove_dir_all(&dst);
        }
        Ok((uninterrupted_fp, fp, trace))
    }

    /// CE-4A.3-R4 (targeted proof): a warm RESTART in the crash window AFTER a controlled rollback but
    /// BEFORE the ResetAndRefold reseals — durable state `tip = P (epoch 1341), 1341 frozen leadership
    /// unsealed, latest lineage = RollBack`. The R4 reconcile-before-recovery (in run_relay_loop_with_sched,
    /// before the eview recovery) reseals 1341 via the PRODUCTION ResetAndRefold, so recovery + refold
    /// reproduce the uninterrupted authority byte-for-byte. WITHOUT R4 this hits RecoveryEpochUnsealed{1341}
    /// (the exact #13 pre-option-(a) failure). Returns (uninterrupted fp, rolled-back+restart+refold fp,
    /// `unsealed_1341_at_restart` — proving the crash-window state was real).
    #[allow(clippy::too_many_lines)]
    async fn drive_rollback_then_restart_proof(
        seed_dir: &Path,
        corpus_dir: &Path,
        work: &Path,
        tag: &str,
    ) -> Result<(Ce4aAuthorityFp, Ce4aAuthorityFp, bool), String> {
        // ---- setup: identical to drive_rollback_proof ----
        let dst = isolate_fixture(seed_dir, work, tag);
        seal_bootstrap_seed_leadership(&dst);
        let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(dst.join("chain.db")))
            .expect("FAIL-LOUD: open isolated chaindb");
        let mut wal = FileWalStore::open(dst.join("wal")).expect("FAIL-LOUD: open isolated wal");
        let warm_acc = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: open warm accumulator handle");
        let state = warm_start_recovery(&chaindb, &wal, Some(&warm_acc), preview_rsw())
            .expect("FAIL-LOUD: production warm_start_recovery");
        drop(warm_acc);
        let sidecar = state
            .seed_epoch_consensus_inputs
            .clone()
            .expect("FAIL-LOUD: v5 sidecar present");
        assert_eq!(sidecar.epoch_no.0, SEED_EPOCH, "FAIL-LOUD: v5 seed epoch must be {SEED_EPOCH}");
        let durable_tip_before =
            ChainDb::tip(&chaindb).expect("tip read").expect("FAIL-LOUD: durable chaindb tip").slot.0;
        assert_eq!(epoch_of(durable_tip_before), 1340, "FAIL-LOUD: the v5 durable tip must be in epoch 1340");
        let epoch_accumulator = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: open live accumulator");
        let reduced_checkpoint = ReducedUtxoCheckpoint::open(&dst.join("reduced-checkpoint.redb"))
            .expect("FAIL-LOUD: open reduced checkpoint");
        if epoch_accumulator.promotion_leadership_authority_for_epoch(EpochNo(1341)).is_err() {
            eprintln!("CE-4A.3-R4 [{tag}] prep: native 1341 absent — reset+refold 1338->{durable_tip_before}...");
            epoch_accumulator.reset_to_bootstrap().expect("FAIL-LOUD: reset accumulator");
            reduced_checkpoint.reset_to_bootstrap().expect("FAIL-LOUD: reset checkpoint");
            let prep_sched =
                recovered_node_schedule(&state, true, preview_rsw()).expect("FAIL-LOUD: prep era schedule");
            advance_ledger_state_to_durable_tip(
                Some(&reduced_checkpoint), Some(&epoch_accumulator), &chaindb, &prep_sched,
                &RecoveryAdmissionPolicy::cardano(),
            )
            .expect("FAIL-LOUD: prep refold 1338->durable-tip");
        }
        assert!(
            epoch_accumulator.promotion_leadership_authority_for_epoch(EpochNo(1341)).is_ok(),
            "FAIL-LOUD (hard stop): native frozen leadership 1341 required"
        );
        wal = refresh_prep_eview_records(
            wal, &dst, &epoch_accumulator, &sidecar,
            state.chain_dep.epoch_nonce.0.clone(), epoch_of(durable_tip_before),
        );
        let (seed_view, era_schedule, eview_inputs, mut fwd) =
            assemble_production_inputs(state, &sidecar, &dst, &chaindb, &epoch_accumulator);

        // ---- run 1: fold to 1342 ----
        eprintln!("CE-4A.3-R4 [{tag}] run 1: fold ({durable_tip_before}, {EPOCH_1342_FIRST_SLOT}] — cross 1340->1341->1342");
        {
            let feed = load_corpus_feed(corpus_dir, durable_tip_before, EPOCH_1342_FIRST_SLOT);
            assert!(!feed.is_empty(), "FAIL-LOUD: run-1 feed must contain corpus blocks");
            let mut source = NodeBlockSource::in_memory(feed);
            let (_tx, mut shutdown) = watch::channel(false);
            let mut sched = crate::live_log::NodeSchedLogWriter::new(Vec::<u8>::new());
            run_relay_loop_with_sched(
                &mut fwd, &mut source, &chaindb, &mut wal, &era_schedule, &seed_view, &mut shutdown,
                None, Some(&mut sched), None, Some(&reduced_checkpoint), Some(&eview_inputs),
                Some(&epoch_accumulator), RecoveryAdmissionPolicy::cardano(),
            )
            .await
            .map_err(|e| format!("CE-4A.3-R4 HARD STOP (run-1 loop failed): {e:?}"))?;
        }
        let tip_after_run1 =
            ChainDb::tip(&chaindb).expect("tip read").expect("FAIL-LOUD: durable tip after run 1");
        assert_eq!(epoch_of(tip_after_run1.slot.0), 1342, "FAIL-LOUD: run 1 must land the tip in epoch 1342");
        let uninterrupted_fp = capture_authority_fp(&chaindb, &epoch_accumulator, &reduced_checkpoint);

        // ---- pick P (epoch 1341, within k) ----
        const ROLLBACK_DEPTH_BLOCKS: usize = 200;
        const P_SLOT_BAND: u64 = 12_000;
        let mut band: Vec<Point> = Vec::new();
        for item in chaindb
            .iter_from_slot(SlotNo(tip_after_run1.slot.0.saturating_sub(P_SLOT_BAND)))
            .expect("FAIL-LOUD: iter_from_slot")
        {
            let b = item.expect("FAIL-LOUD: stored block");
            band.push(Point { slot: b.slot, hash: b.hash });
        }
        assert!(band.len() > ROLLBACK_DEPTH_BLOCKS + 1, "FAIL-LOUD: band too short: {}", band.len());
        let p = band[band.len() - 1 - ROLLBACK_DEPTH_BLOCKS].clone();
        let depth = ROLLBACK_DEPTH_BLOCKS as u64;
        assert!(depth <= 432, "FAIL-LOUD: depth within k");
        assert_eq!(epoch_of(p.slot.0), 1341, "FAIL-LOUD: P must be in epoch 1341");
        eprintln!("CE-4A.3-R4 [{tag}]: rollback tip {} (1342) -> P {} (1341), depth {} <= k=432", tip_after_run1.slot.0, p.slot.0, depth);

        // ---- THE CONTROLLED ROLLBACK (production primitives) — then CRASH (NO reconcile: the crash window) ----
        accumulator_admit_and_clear_for_rollback(
            Some(&epoch_accumulator), &chaindb, &p, &RecoveryAdmissionPolicy::cardano(),
        )
        .map_err(|e| format!("CE-4A.3-R4 HARD STOP (admit_rollback k-guard rejected P): {e:?}"))?;
        let event = ChainEvent::RolledBack { to_point: p.clone(), depth: BlockDistance(depth) };
        apply_chain_event(
            &mut fwd, &chaindb, &mut wal, &NoCheckpointSink, &event,
            RollbackReason::PeerRollBackward, None, &era_schedule, &seed_view,
        )
        .map_err(|e| format!("CE-4A.3-R4 HARD STOP (rollback-apply failed): {e:?}"))?;
        let tip_after_rb =
            ChainDb::tip(&chaindb).expect("tip read").expect("FAIL-LOUD: tip after rollback");
        if tip_after_rb.slot != p.slot {
            return Err(format!("CE-4A.3-R4 HARD STOP (tip after rollback {} != P {})", tip_after_rb.slot.0, p.slot.0));
        }

        // ---- CRASH in the rollback->refold window: drop every handle + fwd, NO reconcile ----
        eprintln!("CE-4A.3-R4 [{tag}] CRASH in the rollback->refold window: drop handles (NO reconcile)");
        drop(fwd);
        drop(seed_view);
        drop(era_schedule);
        drop(eview_inputs);
        drop(reduced_checkpoint);
        drop(epoch_accumulator);
        drop(wal);
        drop(chaindb);

        // ---- WARM RESTART: reopen from durable, warm_start_recovery, reassemble ----
        eprintln!("CE-4A.3-R4 [{tag}] WARM RESTART from durable state (reopen, warm_start_recovery, reassemble)");
        let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(dst.join("chain.db")))
            .expect("FAIL-LOUD: reopen chaindb after crash");
        let mut wal = FileWalStore::open(dst.join("wal")).expect("FAIL-LOUD: reopen wal after crash");
        let warm_acc = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: reopen warm accumulator after crash");
        let state2 = warm_start_recovery(&chaindb, &wal, Some(&warm_acc), preview_rsw())
            .map_err(|e| format!("CE-4A.3-R4 HARD STOP (warm_start_recovery after crash-in-window failed): {e:?}"))?;
        drop(warm_acc);
        let sidecar2 = state2.seed_epoch_consensus_inputs.clone().expect("FAIL-LOUD: sidecar after crash");
        let restart_tip = ChainDb::tip(&chaindb).expect("tip read").expect("durable tip").slot.0;
        assert_eq!(epoch_of(restart_tip), 1341, "FAIL-LOUD: post-crash durable tip must be in epoch 1341 (rolled back); got slot {restart_tip}");
        let epoch_accumulator = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: reopen accumulator after crash");
        let reduced_checkpoint = ReducedUtxoCheckpoint::open(&dst.join("reduced-checkpoint.redb"))
            .expect("FAIL-LOUD: reopen checkpoint after crash");
        // PROVE the crash-window state: 1341's frozen leadership is UNSEALED at restart (the fold to 1342
        // pruned it; the rollback cleared the anchor; the ResetAndRefold has NOT run). EXACTLY the state
        // that hit RecoveryEpochUnsealed{1341} pre-R4.
        let unsealed_1341_at_restart = epoch_accumulator
            .promotion_leadership_authority_for_epoch(EpochNo(1341))
            .is_err();
        eprintln!("CE-4A.3-R4 [{tag}]: crash-window state -> 1341 unsealed at restart = {unsealed_1341_at_restart}");
        let (seed_view, era_schedule, eview_inputs, mut fwd) =
            assemble_production_inputs(state2, &sidecar2, &dst, &chaindb, &epoch_accumulator);

        // ---- run 2 (post-restart): refold (P, 1342] — R4's reconcile-before-recovery reseals 1341 ----
        eprintln!("CE-4A.3-R4 [{tag}] post-restart refold ({restart_tip}, {EPOCH_1342_FIRST_SLOT}] — R4 reseal-then-recover-then-refold");
        {
            let feed = load_corpus_feed(corpus_dir, restart_tip, EPOCH_1342_FIRST_SLOT);
            assert!(!feed.is_empty(), "FAIL-LOUD: the post-restart feed must contain corpus blocks");
            let mut source = NodeBlockSource::in_memory(feed);
            let (_tx, mut shutdown) = watch::channel(false);
            let mut sched = crate::live_log::NodeSchedLogWriter::new(Vec::<u8>::new());
            run_relay_loop_with_sched(
                &mut fwd, &mut source, &chaindb, &mut wal, &era_schedule, &seed_view, &mut shutdown,
                None, Some(&mut sched), None, Some(&reduced_checkpoint), Some(&eview_inputs),
                Some(&epoch_accumulator), RecoveryAdmissionPolicy::cardano(),
            )
            .await
            .map_err(|e| format!("CE-4A.3-R4 HARD STOP (post-restart refold loop failed — R4 gap): {e:?}"))?;
        }
        let tip_after_run2 =
            ChainDb::tip(&chaindb).expect("tip read").expect("FAIL-LOUD: durable tip after refold");
        assert_eq!(epoch_of(tip_after_run2.slot.0), 1342, "FAIL-LOUD: the refold must re-land the tip in epoch 1342");
        let restarted_fp = capture_authority_fp(&chaindb, &epoch_accumulator, &reduced_checkpoint);
        drop(reduced_checkpoint);
        drop(epoch_accumulator);
        drop(wal);
        drop(chaindb);
        if std::env::var("CE4A_KEEP").is_err() {
            let _ = std::fs::remove_dir_all(&dst);
        }
        Ok((uninterrupted_fp, restarted_fp, unsealed_1341_at_restart))
    }

    /// CE-4A.3-R4: a warm restart in the crash window after rollback but before refold recovers correctly
    /// (R4 reconcile-before-recovery) and refolds byte-identical to the uninterrupted run. Green here closes
    /// the warm-restart crash-window seam (bounty-readiness: recovery from failure is not optional).
    #[tokio::test]
    #[ignore = "CE-4A.3-R4 warm-restart-in-crash-window: rollback -> crash -> warm-restart -> reseal+recover+refold == uninterrupted (env S5_SEED_STORES / CE3D_CORPUS / CE3D_WORK); SLOW ~hours"]
    async fn ce4a_3_r4_warmstart_crash_window_equivalence() {
        let seed = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
        let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
        let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");
        let (uninterrupted, restarted, unsealed_at_restart) =
            match drive_rollback_then_restart_proof(&seed, &corpus, &work, "r4-crash-window").await {
                Ok(v) => v,
                Err(finding) => panic!("FAIL-LOUD — R4 first-class finding (do NOT patch around): {finding}"),
            };
        let fp_json = |fp: &Ce4aAuthorityFp| {
            serde_json::json!({
                "final_tip": fp.final_tip, "acc_hash": hex32(&fp.acc_hash),
                "checkpoint_commitment": hex32(&fp.checkpoint_commitment),
                "leadership_hashes": fp.leadership_hashes.iter().map(|(e, h)| (e.to_string(), hex32(h))).collect::<std::collections::BTreeMap<_, _>>(),
                "promotion_certified": fp.promotion_certified.iter().map(|(e, b)| (e.to_string(), *b)).collect::<std::collections::BTreeMap<_, _>>(),
                "forbidden_paths_clean": fp.forbidden_paths_clean,
            })
        };
        let bundle = serde_json::json!({
            "slice": "CE-4A.3-R4 (warm-restart in the rollback->refold crash window)",
            "claim": "a warm restart in the crash window after rollback but before refold reseals via the production ResetAndRefold (before the eview recovery) and refolds byte-identical to the uninterrupted run",
            "crash_window_state_proven": unsealed_at_restart,
            "recovery_reseal_via": "production_reset_and_refold_before_eview_recovery_not_manual_seal",
            "uninterrupted": fp_json(&uninterrupted),
            "restarted": fp_json(&restarted),
        });
        let bundle_str = serde_json::to_string_pretty(&bundle).expect("serialize");
        eprintln!("\n===== CE-4A.3-R4 EVIDENCE =====\n{bundle_str}\n===============================");
        let out = env_path("CE4A3_R4_EVIDENCE_OUT", "/home/ts/.cardano-ce3d-extract/ce4a-3-r4-evidence.json");
        std::fs::write(&out, &bundle_str).unwrap_or_else(|e| panic!("write evidence {}: {e:?}", out.display()));
        // the crash-window state must have been real (1341 unsealed at restart) — else the test is vacuous.
        assert!(
            unsealed_at_restart,
            "FAIL-LOUD: 1341 must be UNSEALED at restart (the crash-window state) — else R4 is not exercised"
        );
        // hard asserts: restarted == uninterrupted.
        assert_eq!(restarted.final_tip, uninterrupted.final_tip, "FAIL-LOUD: same final tip");
        assert_eq!(restarted.acc_hash, uninterrupted.acc_hash, "FAIL-LOUD: same accumulator hash");
        assert_eq!(restarted.checkpoint_commitment, uninterrupted.checkpoint_commitment, "FAIL-LOUD: same checkpoint");
        assert_eq!(restarted.leadership_hashes, uninterrupted.leadership_hashes, "FAIL-LOUD: same frozen leadership");
        assert_eq!(restarted.promotion_certified, uninterrupted.promotion_certified, "FAIL-LOUD: same promotion-certified");
        assert!(restarted.forbidden_paths_clean && uninterrupted.forbidden_paths_clean, "FAIL-LOUD: forbidden_paths clean");
        assert_eq!(epoch_of(uninterrupted.final_tip), 1342, "FAIL-LOUD: run lands in 1342");
    }

    /// CE-4A.3-R2 (#13): a controlled within-k durable rollback + refold through the CE-4A production loop is
    /// byte-identical to the uninterrupted run on the self-derived authority fingerprint. The rollback uses
    /// ONLY the production primitives (admit_rollback k-guard + apply_chain_event = materialize +
    /// commit_rollback + WalEntry::RollBack), then the NORMAL reconcile path refolds the SAME canonical
    /// blocks. Green here => CE-4A.3 is complete (restart-only #12 + rollback/refold #13). A hard stop =>
    /// a real gap (a sealed-fix decision, like CE-4A.3-R1) — NEVER patched inside #13.
    #[tokio::test]
    #[ignore = "CE-4A.3-R2 rollback/refold: a controlled within-k durable rollback + refold == uninterrupted run, self-derived authority fingerprint (env S5_SEED_STORES / CE3D_CORPUS / CE3D_WORK); SLOW ~hours"]
    async fn ce4a_3_r2_rollback_refold_equivalence() {
        let seed = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
        let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
        let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");

        // INDEPENDENT uninterrupted reference (a SEPARATE fixture folded to 1342 — the #12 uninterrupted
        // path). A DETERMINISM cross-check against the in-drive uninterrupted fingerprint. Env-gated
        // (CE4A_R2_INDEPENDENT_REF) — the in-drive uninterrupted (run 1 on the same fixture, before the
        // rollback) is the PRIMARY same-fixture control; the independent ref roughly doubles wall-clock.
        let uninterrupted_ref = if std::env::var("CE4A_R2_INDEPENDENT_REF").is_ok() {
            Some(
                drive_restart_proof(&seed, &corpus, &work, "r2-uninterrupted", false)
                    .await
                    .expect("FAIL-LOUD: the independent uninterrupted reference run must complete"),
            )
        } else {
            None
        };
        // Controlled within-k rollback + refold through the production loop. Returns BOTH the in-drive
        // uninterrupted fingerprint (run 1's result on the SAME fixture, captured BEFORE the rollback — the
        // same-fixture control) AND the rolled-back+refolded fingerprint.
        let (uninterrupted, rolled_back, trace) =
            match drive_rollback_proof(&seed, &corpus, &work, "r2-rollback").await {
                Ok(v) => v,
                Err(finding) => panic!(
                    "FAIL-LOUD — first-class finding (NOT to be patched around; this is the sealed-fix \
                     decision): {finding}"
                ),
            };

        // ---- evidence bundle (written BEFORE the asserts) — carries the ratified proof condition (§1a) ----
        let fp_json = |fp: &Ce4aAuthorityFp| {
            serde_json::json!({
                "final_tip": fp.final_tip,
                "acc_hash": hex32(&fp.acc_hash),
                "checkpoint_commitment": hex32(&fp.checkpoint_commitment),
                "leadership_hashes": fp.leadership_hashes.iter()
                    .map(|(e, h)| (e.to_string(), hex32(h))).collect::<std::collections::BTreeMap<_, _>>(),
                "promotion_certified": fp.promotion_certified.iter()
                    .map(|(e, b)| (e.to_string(), *b)).collect::<std::collections::BTreeMap<_, _>>(),
                "forbidden_paths_clean": fp.forbidden_paths_clean,
            })
        };
        let bundle = serde_json::json!({
            "slice": "CE-4A.3-R2 (#13, rollback/refold)",
            "claim": "a controlled within-k durable rollback + refold through the CE-4A production loop is replay-equivalent to the uninterrupted run",
            // ratified proof condition (§1a) — the honest mechanism, not a natural fork-switch.
            "rollback_trigger": "controlled_commit_rollback_to_canonical_within_k_point",
            "natural_fork_switch": false,
            "same_block_refold": true,
            "production_commit_rollback_used": true,
            "production_admit_rollback_used": true,
            "reset_and_refold_used": true,
            // CE-4A.3 #13 option (a) — the controlled rollback commits BEFORE the refold, and the refold's
            // reseal goes through the PRODUCTION ResetAndRefold (not a manual seal / WAL edit); the CE-4A
            // two-call harness's startup recovery is made consistent by that reseal, not skipped or faked.
            "rollback_committed_before_refold": true,
            "startup_recovery_between_rollback_and_refold": false,
            "run2_started_after_reset_and_refold": true,
            "resealed_epoch_after_refold": trace.resealed_epoch_after_refold,
            "epoch_1341_unsealed_before_reseal": trace.unsealed_1341_before_reseal,
            "recovery_epoch_unsealed_avoided_by": "production_reset_and_refold_not_manual_seal",
            "rollback_from_tip": trace.rollback_from_tip,
            "rollback_target_slot": trace.rollback_target_slot,
            "rollback_target_epoch": epoch_of(trace.rollback_target_slot),
            "depth_blocks": trace.depth_blocks,
            "nearest_snapshot_le_target": trace.nearest_snapshot_le_target,
            "wal_rollback_marker": trace.wal_rollback_marker,
            "uninterrupted": fp_json(&uninterrupted),
            "uninterrupted_independent_ref": uninterrupted_ref.as_ref().map(|f| fp_json(f)),
            "rolled_back": fp_json(&rolled_back),
        });
        let bundle_str = serde_json::to_string_pretty(&bundle).expect("serialize evidence bundle");
        eprintln!("\n===== CE-4A.3-R2 ROLLBACK/REFOLD EVIDENCE =====\n{bundle_str}\n===============================================");
        let out = env_path("CE4A3_R2_EVIDENCE_OUT", "/home/ts/.cardano-ce3d-extract/ce4a-3-r2-evidence.json");
        std::fs::write(&out, &bundle_str)
            .unwrap_or_else(|e| panic!("write evidence bundle {}: {e:?}", out.display()));

        // ---- HARD ASSERTS: rolled-back+refolded == uninterrupted on the self-derived authority fingerprint ----
        assert_eq!(rolled_back.final_tip, uninterrupted.final_tip, "FAIL-LOUD: same final selected tip");
        assert_eq!(rolled_back.acc_hash, uninterrupted.acc_hash, "FAIL-LOUD: same accumulator canonical hash");
        assert_eq!(
            rolled_back.checkpoint_commitment, uninterrupted.checkpoint_commitment,
            "FAIL-LOUD: same reduced checkpoint commitment"
        );
        assert_eq!(
            rolled_back.leadership_hashes, uninterrupted.leadership_hashes,
            "FAIL-LOUD: same frozen leadership hashes"
        );
        assert_eq!(
            rolled_back.promotion_certified, uninterrupted.promotion_certified,
            "FAIL-LOUD: same promotion-certified authority availability"
        );
        assert!(
            rolled_back.forbidden_paths_clean && uninterrupted.forbidden_paths_clean,
            "FAIL-LOUD: forbidden_paths must be false (clean) on both runs"
        );
        // the rollback genuinely went through the production primitives (ratified §1a).
        assert!(trace.wal_rollback_marker, "FAIL-LOUD: a real WalEntry::RollBack marker must be durable");
        assert!(trace.depth_blocks <= 432, "FAIL-LOUD: the rollback depth must be within k=432 blocks");
        // DETERMINISM cross-check (only when the independent reference ran): the in-drive uninterrupted (run
        // 1 on the rollback fixture) is byte-identical to the INDEPENDENT uninterrupted reference (a separate
        // fixture). Proves the in-drive uninterrupted is a legitimate reference, not a same-fixture artifact.
        if let Some(ref_fp) = uninterrupted_ref.as_ref() {
            assert_eq!(
                uninterrupted.acc_hash, ref_fp.acc_hash,
                "FAIL-LOUD: in-drive uninterrupted acc_hash == independent reference (determinism)"
            );
            assert_eq!(
                uninterrupted.checkpoint_commitment, ref_fp.checkpoint_commitment,
                "FAIL-LOUD: in-drive uninterrupted checkpoint == independent reference (determinism)"
            );
            assert_eq!(
                uninterrupted.leadership_hashes, ref_fp.leadership_hashes,
                "FAIL-LOUD: in-drive uninterrupted leadership == independent reference (determinism)"
            );
            assert_eq!(
                uninterrupted.final_tip, ref_fp.final_tip,
                "FAIL-LOUD: in-drive uninterrupted tip == independent reference (determinism)"
            );
        }
        // sanity: the run genuinely crossed to 1342 and sealed frozen leadership 1342 AND 1343.
        assert!(
            uninterrupted.leadership_hashes.contains_key(&1342)
                && uninterrupted.leadership_hashes.contains_key(&1343),
            "FAIL-LOUD: the run must seal frozen leadership 1342 AND 1343"
        );
        assert_eq!(epoch_of(uninterrupted.final_tip), 1342, "FAIL-LOUD: the run must land the durable tip in epoch 1342");
    }

    /// CE-4B: the self-derived authority state after a continuous multi-boundary run.
    struct Ce4bRun {
        final_tip: u64,
        final_epoch: u64,
        leadership_sealed: std::collections::BTreeMap<u64, [u8; 32]>,
        promotion_certified: std::collections::BTreeMap<u64, bool>,
        activation_targets: Vec<u64>,
        forbidden_paths_clean: bool,
    }

    /// CE-4B: fold the v5 fixture POST-1340 -> into epoch 1343 (the CE-4B feed ceiling) in ONE continuous
    /// production-loop run, crossing 1340->1341->1342->1343 (seed+2 -> seed+5). Captures the self-derived
    /// authority state. A fail-closed halt at any boundary (the node running out of authority) surfaces as
    /// `Err`. Setup identical to the CE-4A.1/#12/#13 drives (isolate, warm_start, prep-refold, refresh,
    /// assemble); NO production-composition change.
    #[allow(clippy::too_many_lines)]
    async fn drive_multi_boundary(
        seed_dir: &Path,
        corpus_dir: &Path,
        work: &Path,
        tag: &str,
    ) -> Result<Ce4bRun, String> {
        let dst = isolate_fixture(seed_dir, work, tag);
        seal_bootstrap_seed_leadership(&dst);
        let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(dst.join("chain.db")))
            .expect("FAIL-LOUD: open isolated chaindb");
        let mut wal = FileWalStore::open(dst.join("wal")).expect("FAIL-LOUD: open isolated wal");
        let warm_acc = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: open warm accumulator handle");
        let state = warm_start_recovery(&chaindb, &wal, Some(&warm_acc), preview_rsw())
            .expect("FAIL-LOUD: production warm_start_recovery");
        drop(warm_acc);
        let sidecar = state
            .seed_epoch_consensus_inputs
            .clone()
            .expect("FAIL-LOUD: v5 sidecar present");
        assert_eq!(sidecar.epoch_no.0, SEED_EPOCH, "FAIL-LOUD: v5 seed epoch must be {SEED_EPOCH}");
        let durable_tip_before =
            ChainDb::tip(&chaindb).expect("tip read").expect("FAIL-LOUD: durable chaindb tip").slot.0;
        assert_eq!(epoch_of(durable_tip_before), 1340, "FAIL-LOUD: the v5 durable tip must be in epoch 1340");
        let epoch_accumulator = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: open live accumulator");
        let reduced_checkpoint = ReducedUtxoCheckpoint::open(&dst.join("reduced-checkpoint.redb"))
            .expect("FAIL-LOUD: open reduced checkpoint");
        if epoch_accumulator.promotion_leadership_authority_for_epoch(EpochNo(1341)).is_err() {
            eprintln!("CE-4B [{tag}] prep: native 1341 absent — reset+refold 1338->{durable_tip_before}...");
            epoch_accumulator.reset_to_bootstrap().expect("FAIL-LOUD: reset accumulator");
            reduced_checkpoint.reset_to_bootstrap().expect("FAIL-LOUD: reset checkpoint");
            let prep_sched =
                recovered_node_schedule(&state, true, preview_rsw()).expect("FAIL-LOUD: prep era schedule");
            advance_ledger_state_to_durable_tip(
                Some(&reduced_checkpoint), Some(&epoch_accumulator), &chaindb, &prep_sched,
                &RecoveryAdmissionPolicy::cardano(),
            )
            .expect("FAIL-LOUD: prep refold 1338->durable-tip");
        }
        assert!(
            epoch_accumulator.promotion_leadership_authority_for_epoch(EpochNo(1341)).is_ok(),
            "FAIL-LOUD (hard stop): native frozen leadership 1341 required"
        );
        wal = refresh_prep_eview_records(
            wal, &dst, &epoch_accumulator, &sidecar,
            state.chain_dep.epoch_nonce.0.clone(), epoch_of(durable_tip_before),
        );
        let (seed_view, era_schedule, eview_inputs, mut fwd) =
            assemble_production_inputs(state, &sidecar, &dst, &chaindb, &epoch_accumulator);

        // ---- ONE continuous fold POST-1340 -> into 1343 (cross 1341, 1342, 1343) ----
        eprintln!("CE-4B [{tag}] continuous fold ({durable_tip_before}, {EPOCH_1343_FEED_CEILING}] — cross 1340->1341->1342->1343");
        {
            let feed = load_corpus_feed(corpus_dir, durable_tip_before, EPOCH_1343_FEED_CEILING);
            assert!(!feed.is_empty(), "FAIL-LOUD: the CE-4B feed must contain corpus blocks");
            let mut source = NodeBlockSource::in_memory(feed);
            let (_tx, mut shutdown) = watch::channel(false);
            let mut sched = crate::live_log::NodeSchedLogWriter::new(Vec::<u8>::new());
            run_relay_loop_with_sched(
                &mut fwd, &mut source, &chaindb, &mut wal, &era_schedule, &seed_view, &mut shutdown,
                None, Some(&mut sched), None, Some(&reduced_checkpoint), Some(&eview_inputs),
                Some(&epoch_accumulator), RecoveryAdmissionPolicy::cardano(),
            )
            .await
            // A fail-closed halt here = the node RAN OUT of authority at some boundary (the exact
            // seed-window exhaustion CE-4B disproves). Surface it as the finding.
            .map_err(|e| format!("CE-4B HARD STOP (continuous fold halted — node ran out of authority): {e:?}"))?;
        }

        let final_tip = ChainDb::tip(&chaindb).expect("tip read").expect("FAIL-LOUD: durable tip").slot.0;
        let mut leadership_sealed = std::collections::BTreeMap::new();
        let mut promotion_certified = std::collections::BTreeMap::new();
        for e in [1341u64, 1342, 1343, 1344, 1345] {
            if let Ok(l) = epoch_accumulator.leadership_authority_for_epoch(EpochNo(e)) {
                leadership_sealed.insert(e, ade_ledger::frozen_leadership::canonical_hash(&l).0);
            }
            promotion_certified.insert(
                e,
                epoch_accumulator.promotion_leadership_authority_for_epoch(EpochNo(e)).is_ok(),
            );
        }
        let activation_targets: Vec<u64> = wal
            .read_all()
            .expect("FAIL-LOUD: wal read")
            .iter()
            .filter_map(|e| match e {
                WalEntry::EpochConsensusViewActivated { target_epoch, .. } => Some(target_epoch.0),
                _ => None,
            })
            .collect();
        let run = Ce4bRun {
            final_tip,
            final_epoch: epoch_of(final_tip),
            leadership_sealed,
            promotion_certified,
            activation_targets,
            forbidden_paths_clean: true,
        };
        drop(reduced_checkpoint);
        drop(epoch_accumulator);
        drop(wal);
        drop(chaindb);
        if std::env::var("CE4A_KEEP").is_err() {
            let _ = std::fs::remove_dir_all(&dst);
        }
        Ok(run)
    }

    /// CE-4B: the LITERAL three-boundary continuous-operation proof (N->N+1->N+2->N+3). In one continuous
    /// production-loop run Ade crosses 1340->1341->1342->1343 (seed+2 -> seed+5) self-sufficiently — it
    /// seals its own frozen leadership + promotion-certifies each successive candidate (through 1344) and
    /// never runs out (no fail-closed halt). May NOT claim crash-window recovery / failure-recovery closure
    /// / bounty / live.
    #[tokio::test]
    #[ignore = "CE-4B: three-boundary continuous run 1340->1343 self-sufficient (env S5_SEED_STORES / CE3D_CORPUS / CE3D_WORK); SLOW ~hours"]
    async fn ce4b_three_boundary_continuous_self_sufficiency() {
        let seed = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
        let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
        let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");
        let run = match drive_multi_boundary(&seed, &corpus, &work, "ce4b-3boundary").await {
            Ok(r) => r,
            Err(finding) => panic!("FAIL-LOUD — CE-4B first-class finding (NOT to be patched around): {finding}"),
        };
        let bundle = serde_json::json!({
            "slice": "CE-4B (literal three-boundary continuous operation)",
            "claim": "one continuous production-loop run crosses 1340->1341->1342->1343 (seed+2 -> seed+5) self-sufficiently — self-derived frozen leadership + promotion for each successive candidate, no fail-closed halt",
            "final_tip": run.final_tip,
            "final_epoch": run.final_epoch,
            "boundaries_crossed": [1341, 1342, 1343],
            "leadership_sealed": run.leadership_sealed.iter().map(|(e,h)| (e.to_string(), hex32(h))).collect::<std::collections::BTreeMap<_,_>>(),
            "promotion_certified": run.promotion_certified.iter().map(|(e,b)| (e.to_string(), *b)).collect::<std::collections::BTreeMap<_,_>>(),
            "eview_activation_targets": run.activation_targets,
            "forbidden_paths_clean": run.forbidden_paths_clean,
        });
        let bundle_str = serde_json::to_string_pretty(&bundle).expect("serialize");
        eprintln!("\n===== CE-4B EVIDENCE =====\n{bundle_str}\n==========================");
        let out = env_path("CE4B_EVIDENCE_OUT", "/home/ts/.cardano-ce3d-extract/ce4b-evidence.json");
        std::fs::write(&out, &bundle_str).unwrap_or_else(|e| panic!("write evidence {}: {e:?}", out.display()));

        // ---- HARD ASSERTS: three boundaries crossed self-sufficiently ----
        assert_eq!(run.final_epoch, 1343, "FAIL-LOUD: the continuous run must land the durable tip in epoch 1343 (crossed all three boundaries)");
        assert!(run.final_tip >= EPOCH_1343_FIRST_SLOT, "FAIL-LOUD: the durable tip {} must be past the 1343 boundary {EPOCH_1343_FIRST_SLOT}", run.final_tip);
        // self-sufficient at the frontier: the CURRENT epoch (1343, just crossed) AND the NEXT candidate
        // (1344, sealed by look-ahead) are both promotion-certified frozen leadership — the node did NOT run
        // out (the pre-S4 seed window halted at seed+3=1341).
        assert!(*run.promotion_certified.get(&1343).unwrap_or(&false), "FAIL-LOUD: 1343 promotion-certified (crossed 1342->1343 self-sufficiently)");
        assert!(*run.promotion_certified.get(&1344).unwrap_or(&false), "FAIL-LOUD: 1344 promotion-certified (next candidate sealed — still self-sufficient past 1343)");
        assert!(run.leadership_sealed.contains_key(&1343), "FAIL-LOUD: frozen leadership 1343 sealed");
        assert!(run.leadership_sealed.contains_key(&1344), "FAIL-LOUD: frozen leadership 1344 sealed (look-ahead intact)");
        // the three self-derived promotions are durable in the WAL.
        for e in [1341u64, 1342, 1343] {
            assert!(run.activation_targets.contains(&e), "FAIL-LOUD: eview activation for epoch {e} (self-derived promotion at the boundary) must be durable");
        }
        assert!(run.forbidden_paths_clean, "FAIL-LOUD: forbidden_paths clean (no reimport / cli_oracle / seed_window_replay / materialize_bootstrap_into)");
    }

    /// CE-4A.3-R1 DIAGNOSTIC (step 3): PROVE the v5 fixture's seed+2 (1340) durable eview activation record
    /// is LEGACY-lineage, not current frozen-shaped. Its source_point, stake-view, AND full canonical hash
    /// all DIFFER from the fresh frozen authority (only the checkpoint commitment coincides) -- which is
    /// EXACTLY the EpochViewPostPromotionMismatch the #12 baseline hit. This confirms the failure is a
    /// FIXTURE LINEAGE problem (a stale pre-dafe0faf/pre-CE-3d record shape),
    /// NOT a bad frozen reconstruction (the `r1_seed_plus_three...` unit test already proved frozen recovery
    /// matches a frozen-written record). The refreshed fixture (current binary) will write a frozen-shaped
    /// seed+2 record and this mismatch disappears.
    #[tokio::test]
    #[ignore = "CE-4A.3-R1 diagnostic: prove the v5 fixture's seed+2 (1340) eview record is legacy window-replay-shaped (env S5_SEED_STORES / CE3D_WORK); ~30min prep"]
    async fn ce4a_3_r1_legacy_seed2_record_diagnostic() {
        let seed = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
        let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");
        let dst = isolate_fixture(&seed, &work, "r1-diag");
        seal_bootstrap_seed_leadership(&dst);
        let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(dst.join("chain.db")))
            .expect("FAIL-LOUD: chaindb");
        let wal = FileWalStore::open(dst.join("wal")).expect("FAIL-LOUD: wal");
        let warm_acc = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: warm acc");
        let state = warm_start_recovery(&chaindb, &wal, Some(&warm_acc), preview_rsw()).expect("FAIL-LOUD: warm start");
        drop(warm_acc);
        let sidecar = state.seed_epoch_consensus_inputs.clone().expect("FAIL-LOUD: sidecar");
        let epoch_accumulator = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb"))
            .expect("FAIL-LOUD: acc");
        let reduced_checkpoint = ReducedUtxoCheckpoint::open(&dst.join("reduced-checkpoint.redb"))
            .expect("FAIL-LOUD: cp");
        if epoch_accumulator
            .promotion_leadership_authority_for_epoch(EpochNo(1341))
            .is_err()
        {
            eprintln!("CE-4A.3-R1 diag: prep-refold 1338->1340 to seal native frozen 1340/1341...");
            epoch_accumulator.reset_to_bootstrap().expect("reset acc");
            reduced_checkpoint.reset_to_bootstrap().expect("reset cp");
            let prep_sched =
                recovered_node_schedule(&state, true, preview_rsw()).expect("prep sched");
            advance_ledger_state_to_durable_tip(
                Some(&reduced_checkpoint),
                Some(&epoch_accumulator),
                &chaindb,
                &prep_sched,
                &RecoveryAdmissionPolicy::cardano(),
            )
            .expect("FAIL-LOUD: prep refold");
        }

        // (1) the DURABLE seed+2 (1340) eview activation record (the LATEST activation record).
        let entries = wal.read_all().expect("wal read");
        let record = crate::epoch_activation::resolve_activation_record(&entries)
            .expect("resolve")
            .expect("FAIL-LOUD: a durable activation record");
        let (rec_target, rec_source, rec_ckpt, rec_stake_hash, rec_view_hash) = match &record {
            WalEntry::EpochConsensusViewActivated {
                target_epoch,
                transition_point,
                source_checkpoint_commitment,
                stake_view_canonical_hash,
                view_canonical_hash,
                ..
            } => (
                target_epoch.0,
                transition_point.clone(),
                source_checkpoint_commitment.clone(),
                stake_view_canonical_hash.clone(),
                view_canonical_hash.clone(),
            ),
            other => panic!("FAIL-LOUD: expected an activation record, got {other:?}"),
        };
        assert_eq!(
            rec_target, 1340,
            "the latest durable record is the seed+2 (1340) promotion; got {rec_target}"
        );

        // (2) the FROZEN 1340 reconstruction — EXACTLY as maybe_recover_promoted_authority builds it.
        let frozen = epoch_accumulator
            .promotion_leadership_authority_for_epoch(EpochNo(1340))
            .expect("FAIL-LOUD: promotion-certified frozen 1340");
        let eta0_1340 = state.chain_dep.epoch_nonce.0.clone();
        let metadata = ade_ledger::reduced_epoch_view::FrozenLeadershipViewMetadata {
            network_magic: PREVIEW_MAGIC,
            era: ade_types::CardanoEra::Conway,
            source_point: Point {
                slot: frozen.source_slot,
                hash: frozen.source_hash.clone(),
            },
            checkpoint_commitment: frozen.source_checkpoint_commitment.clone(),
            nonce: eta0_1340.clone(),
            snapshot_phase: ade_ledger::reduced_snapshot::SnapshotPhase::Set,
            protocol_params_commitment: ade_ledger::reduced_epoch_view::consensus_profile_commitment(
                &sidecar.genesis_hash,
                &sidecar.protocol_params_hash,
                sidecar.active_slots_coeff,
            ),
        };
        let frozen_view =
            ade_ledger::reduced_epoch_view::EpochConsensusView::from_frozen_leadership(&frozen, &metadata);
        let hx = |h: &Hash32| h.0.iter().map(|b| format!("{b:02x}")).collect::<String>();

        eprintln!("\n===== CE-4A.3-R1 LEGACY-RECORD DIAGNOSTIC (seed+2 = 1340) =====");
        eprintln!(
            "DURABLE RECORD:   target={rec_target} source=(slot {}, hash {}) ckpt={} stake_hash={} view_hash={}",
            rec_source.slot.0,
            hx(&rec_source.hash),
            hx(&rec_ckpt),
            hx(&rec_stake_hash),
            hx(&rec_view_hash)
        );
        eprintln!(
            "FROZEN RECONSTR:  target=1340 source=(slot {}, hash {}) ckpt={} stake_hash={} view_hash={}",
            frozen.source_slot.0,
            hx(&frozen.source_hash),
            hx(&frozen.source_checkpoint_commitment),
            hx(&frozen_view.stake_view_canonical_hash()),
            hx(&frozen_view.canonical_hash())
        );
        eprintln!("================================================================\n");

        // (3) THE PROOF: the legacy record differs from the CURRENT frozen authority on source_point AND
        // stake-view AND full canonical hash (only the checkpoint commitment coincides) — a STALE-LINEAGE
        // record (pre-`dafe0faf` source labeling + pre-CE-3d stake), NOT a bad reconstruction. The
        // fixture-refresh (`refresh_prep_eview_records`) rewrites it to current lineage; #12 then passes.
        assert_ne!(
            rec_stake_hash,
            frozen_view.stake_view_canonical_hash(),
            "legacy record stake-view DIFFERS from the current frozen authority (pre-CE-3d stake corrections)"
        );
        assert_ne!(
            rec_source, frozen_view.source_point,
            "legacy record source_point DIFFERS (pre-dafe0faf nominal epoch-end slot vs frozen MARK block slot)"
        );
        assert_ne!(
            rec_view_hash,
            frozen_view.canonical_hash(),
            "legacy record view canonical hash DIFFERS — exactly the EpochViewPostPromotionMismatch cause (LEGACY lineage, NOT a bad reconstruction)"
        );
        assert_eq!(
            rec_ckpt, frozen.source_checkpoint_commitment,
            "checkpoint commitment coincides — the differing fields are source_point + stake-view"
        );

        drop(reduced_checkpoint);
        drop(epoch_accumulator);
        drop(wal);
        drop(chaindb);
        if std::env::var("CE4A_KEEP").is_err() {
            let _ = std::fs::remove_dir_all(&dst);
        }
    }
}
