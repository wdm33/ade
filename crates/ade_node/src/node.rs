// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED node lifecycle (PHASE4-N-K S7).
//!
//! `run_node_until_shutdown` is the binary's main loop. It:
//!   1. Calls `ade_runtime::bootstrap::bootstrap_initial_state`
//!      (CN-NODE-01).
//!   2. Builds the `OrchestratorState` over the bootstrap output.
//!   3. Spawns a `LeadershipSession` over the injected `Clock`.
//!   4. Drives the orchestrator inbox loop until `Shutdown` is
//!      observed.
//!   5. Force-captures a final persistent snapshot via the
//!      writer (`PersistentSnapshotWriter::force_capture`) for
//!      DC-NODE-04 shutdown-resume identity.
//!
//! Authority-fatal exit codes:
//!   - 10: chain-write IO authority-fatal (DC-NODE-04).
//!   - 12: snapshot decode UnknownVersion / FingerprintMismatch
//!     (DC-NODE-04, observed at bootstrap).
//!   -  1: generic startup error (CLI parse, missing genesis, etc.).
//!
//! The function is generic over `Clock`, `ChainDb + SnapshotStore`,
//! and the shutdown signal so tests can drive each in-process.

use std::sync::Arc;

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_ledger::producer::ServedChainSnapshot;
use ade_ledger::receive::ReceiveState;
use ade_ledger::rollback::MaterializeError;
use ade_ledger::snapshot::error::SnapshotDecodeError;
use ade_ledger::state::LedgerState;
use ade_runtime::bootstrap::{
    bootstrap_initial_state, BootstrapError, BootstrapInputs, BootstrapState,
    SeedEpochConsensusSource,
};
use ade_runtime::chaindb::{ChainDb, ChainTip, SnapshotStore};
use ade_runtime::clock::Clock;
use ade_runtime::orchestrator::event::{
    AuthorityFatalKind, OrchestratorEffect, OrchestratorError, OrchestratorEvent,
};
use ade_runtime::orchestrator::leadership_session::{LeadershipSession, SlotEraAnchor};
use ade_runtime::orchestrator::state::OrchestratorState;
use ade_runtime::orchestrator::step;
use ade_runtime::receive::ChainDbWriter;
use ade_runtime::rollback::cadence::SnapshotCadence;
use ade_runtime::rollback::persistent_writer::PersistentSnapshotWriter;
use ade_runtime::rollback::PersistentCacheError;
use ade_types::SlotNo;
use tokio::sync::mpsc;

/// Authority-fatal IO exit code (chain-write failed during commit).
pub const EXIT_AUTHORITY_FATAL_IO: i32 = 10;
/// Authority-fatal decode exit code (snapshot decode rejected at
/// bootstrap with `UnknownVersion` or `FingerprintMismatch`).
pub const EXIT_AUTHORITY_FATAL_DECODE: i32 = 12;
/// Generic startup error (CLI parse / missing genesis / etc).
pub const EXIT_GENERIC_STARTUP: i32 = 1;

/// Inputs to `run_node_until_shutdown`. Borrowed; the caller owns
/// the storage and the clock.
pub struct NodeStartupInputs<'a, D, S, C>
where
    D: ChainDb + SnapshotStore,
    S: SnapshotStore,
    C: Clock + Send + 'static,
{
    pub chaindb: &'a D,
    pub snapshot_store: &'a S,
    pub era_schedule: &'a EraSchedule,
    pub ledger_view: Arc<dyn LedgerView + Send + Sync>,
    pub cadence: SnapshotCadence,
    pub leadership_clock: C,
    pub leadership_anchor: SlotEraAnchor,
    pub genesis_initial: Option<(LedgerState, PraosChainDepState)>,
}

/// Closed run-error sum. Maps to deterministic exit codes via
/// `exit_code_for`.
#[derive(Debug)]
pub enum NodeRunError {
    Bootstrap(BootstrapError),
    AuthorityFatal(OrchestratorError),
    PersistentWriterIo(PersistentCacheError),
}

impl NodeRunError {
    pub fn exit_code(&self) -> i32 {
        match self {
            NodeRunError::Bootstrap(BootstrapError::SnapshotMissing { .. }) => {
                EXIT_AUTHORITY_FATAL_DECODE
            }
            NodeRunError::Bootstrap(BootstrapError::Materialize(MaterializeError::ReplayFailedAt {
                ..
            })) => EXIT_AUTHORITY_FATAL_DECODE,
            NodeRunError::Bootstrap(BootstrapError::Materialize(_)) => EXIT_GENERIC_STARTUP,
            NodeRunError::Bootstrap(BootstrapError::ChainDb(_)) => EXIT_AUTHORITY_FATAL_IO,
            NodeRunError::Bootstrap(BootstrapError::GenesisRequiredButAbsent) => EXIT_GENERIC_STARTUP,
            // A3b: seed-epoch consensus-input warm-start verification
            // failures are authority-fatal decode/binding halts (the
            // recovered consensus inputs could not be trusted).
            NodeRunError::Bootstrap(
                BootstrapError::SeedConsensusProvenanceMissing
                | BootstrapError::SeedConsensusSidecarMissing { .. }
                | BootstrapError::SeedConsensusHashMismatch { .. }
                | BootstrapError::SeedConsensusBindingMismatch { .. }
                | BootstrapError::SeedConsensusSidecarDecode(_)
                // ECA-2-pre (DC-CINPUT-06): a pre-v4 sidecar (typed schema-upgrade
                // requirement) is the same authority-fatal startup-halt class — the
                // recovered consensus inputs cannot be trusted until reimported.
                | BootstrapError::ConsensusInputsSchemaUnsupported { .. },
            ) => EXIT_AUTHORITY_FATAL_DECODE,
            // AK-S1 (DC-NODE-31): a non-Origin recovered store whose anchor-point
            // record is missing / malformed / fingerprint-mismatched is a
            // fail-closed authority-fatal halt (the recovered live-follow start
            // could not be trusted) — same decode/binding class as the
            // seed-consensus warm-start failures above.
            NodeRunError::Bootstrap(
                BootstrapError::RecoveredAnchorPointMissing { .. }
                | BootstrapError::RecoveredAnchorPointDecode(_)
                | BootstrapError::RecoveredAnchorPointBindingMismatch { .. },
            ) => EXIT_AUTHORITY_FATAL_DECODE,
            NodeRunError::AuthorityFatal(OrchestratorError::AuthorityFatal(kind)) => match kind {
                AuthorityFatalKind::ChainWriteIo => EXIT_AUTHORITY_FATAL_IO,
                AuthorityFatalKind::SnapshotDecodeUnknownVersion
                | AuthorityFatalKind::SnapshotDecodeFingerprintMismatch => {
                    EXIT_AUTHORITY_FATAL_DECODE
                }
            },
            NodeRunError::PersistentWriterIo(_) => EXIT_AUTHORITY_FATAL_IO,
        }
    }
}

/// Evidence summary the run loop produces on clean shutdown. The
/// integration test for DC-NODE-04 uses this to assert
/// shutdown-resume identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeShutdownEvidence {
    pub final_chain_tip: Option<ChainTip>,
    pub final_observed_slot: Option<SlotNo>,
    pub final_persistent_snapshot_slot: Option<SlotNo>,
    pub admitted_blocks: u64,
    pub captured_snapshots: u64,
}

/// Drive the node until shutdown. Returns on:
///   - clean shutdown (`Ok(evidence)`); a final snapshot was
///     force-written, the orchestrator is quiescent.
///   - authority-fatal error (`Err(...)`); the binary exits via
///     the `exit_code` mapping.
pub async fn run_node_until_shutdown<D, S, C>(
    inputs: NodeStartupInputs<'_, D, S, C>,
    inbox_tx: mpsc::Sender<OrchestratorEvent>,
    mut inbox_rx: mpsc::Receiver<OrchestratorEvent>,
) -> Result<NodeShutdownEvidence, NodeRunError>
where
    D: ChainDb + SnapshotStore,
    S: SnapshotStore,
    C: Clock + Send + 'static,
{
    // Bootstrap (CN-NODE-01).
    let BootstrapState {
        ledger,
        chain_dep,
        tip: chain_tip,
        ..
    } = bootstrap_initial_state(BootstrapInputs {
        chaindb: inputs.chaindb,
        snapshot_store: inputs.snapshot_store,
        era_schedule: inputs.era_schedule,
        ledger_view: inputs.ledger_view.as_ref(),
        genesis_initial: inputs.genesis_initial,
        // A3b: this orchestrator entry is not the production
        // recovered-sidecar warm-start path (deferred slice).
        seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired,
        // AK-S1: not the live recover→follow path; no anchor-point resolution
        // (the resolver leaves the prior tip behavior exactly with `None`).
        recovered_anchor: None,
    })
    .map_err(NodeRunError::Bootstrap)?;

    let mut state = OrchestratorState::new(
        ReceiveState::new(ledger, chain_dep),
        inputs.cadence,
    );
    let mut writer = PersistentSnapshotWriter::new(inputs.snapshot_store, inputs.cadence);

    // Spawn the leadership session over the injected Clock.
    let leadership = LeadershipSession {
        clock: inputs.leadership_clock,
        events_out: inbox_tx.clone(),
        anchor: inputs.leadership_anchor,
    };
    let leadership_handle = tokio::spawn(leadership.run());

    let mut admitted = 0u64;
    let mut captured = 0u64;
    let served = ServedChainSnapshot::new();

    while let Some(event) = inbox_rx.recv().await {
        let is_shutdown = matches!(event, OrchestratorEvent::Shutdown);
        let mut writer_for_chain = ChainDbWriter::new(inputs.chaindb);
        let effects = step(
            &mut state,
            event,
            &mut writer_for_chain,
            &served,
            inputs.era_schedule,
            inputs.ledger_view.as_ref(),
        )
        .map_err(NodeRunError::AuthorityFatal)?;
        for effect in effects {
            match effect {
                OrchestratorEffect::AdmittedBlock { slot, .. } => {
                    admitted += 1;
                    // Cadence-aware persistent capture; we mirror the
                    // orchestrator's CaptureSnapshot effect via the
                    // writer for cluster-internal evidence (test
                    // observability). The orchestrator's own cadence
                    // decision drives the CaptureSnapshot effect; we
                    // additionally pass through the writer so an
                    // operator-side cadence count matches.
                    if let Some(block_no) = state.receive_state.chain_dep.last_block_no {
                        let _ = writer
                            .on_admitted(
                                slot,
                                block_no,
                                &state.receive_state.ledger,
                                &state.receive_state.chain_dep,
                            )
                            .map_err(NodeRunError::PersistentWriterIo)?;
                    }
                }
                OrchestratorEffect::CaptureSnapshot { .. } => {
                    captured += 1;
                }
                OrchestratorEffect::ShutdownAcknowledged => {
                    // Drain done; force-write final snapshot below.
                    break;
                }
                _ => {}
            }
        }
        if is_shutdown {
            break;
        }
    }

    // Force a final snapshot at the most recent observed slot.
    let final_slot = state
        .last_observed_slot
        .or_else(|| state.receive_state.chain_dep.last_slot)
        .unwrap_or(SlotNo(0));
    writer
        .force_capture(
            final_slot,
            &state.receive_state.ledger,
            &state.receive_state.chain_dep,
        )
        .map_err(NodeRunError::PersistentWriterIo)?;

    // Stop the leadership session.
    leadership_handle.abort();

    Ok(NodeShutdownEvidence {
        final_chain_tip: chain_tip,
        final_observed_slot: state.last_observed_slot,
        final_persistent_snapshot_slot: writer.last_captured_slot(),
        admitted_blocks: admitted,
        captured_snapshots: captured,
    })
}

/// Classify a `SnapshotDecodeError` for the authority-fatal exit
/// code mapping. Exposed for the binary's startup-time decode
/// probe (when the operator has corrupted the snapshot store).
pub fn snapshot_decode_is_authority_fatal(err: &SnapshotDecodeError) -> bool {
    matches!(
        err,
        SnapshotDecodeError::UnknownVersion { .. } | SnapshotDecodeError::FingerprintMismatch { .. }
    )
}
