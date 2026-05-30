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
use std::path::Path;
use std::process::ExitCode;

use ade_core::consensus::era_schedule::{EraSchedule, EraSummary};
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_core::consensus::BootstrapAnchorHash;
use ade_ledger::consensus_view::PoolDistrView;
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::state::LedgerState;
use ade_ledger::wal::{replay_from_anchor, WalStore};
use ade_runtime::bootstrap::{
    bootstrap_initial_state, BootstrapInputs, BootstrapState, SeedEpochConsensusSource,
};
use ade_runtime::chaindb::{
    ChainDb, PersistentChainDb, PersistentChainDbOptions, SnapshotStore,
};
use ade_runtime::consensus_inputs::{import_live_consensus_inputs, LiveConsensusInputsCanonical};
use ade_runtime::mithril_bootstrap::{bootstrap_from_mithril_snapshot, MithrilSeedPointInputs};
use ade_runtime::mithril_import::import_mithril_manifest_from_bytes;
use ade_runtime::seed_import::import_cardano_cli_json_utxo;
use ade_runtime::wal::FileWalStore;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_types::{CardanoEra, EpochNo, Hash32, SlotNo};
use tokio::sync::watch;

use crate::cli::Cli;
use crate::EXIT_GENERIC_STARTUP;

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

/// The `--mode node` owner entry. Returns a process exit code.
///
/// `shutdown` is accepted for signature parity with the other mode
/// entries and the L4 slot loop to come; L2 does not run a loop.
pub async fn run_node_lifecycle(cli: Cli, _shutdown: watch::Receiver<bool>) -> ExitCode {
    match run_node_lifecycle_inner(&cli) {
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
        | NodeLifecycleError::ExtractionRead(_) => EXIT_GENERIC_STARTUP,
        NodeLifecycleError::ManifestImport(_)
        | NodeLifecycleError::EpochMismatch { .. }
        | NodeLifecycleError::MithrilBootstrap(_) => EXIT_NODE_MITHRIL_BOOTSTRAP_FAILED,
        NodeLifecycleError::WarmStartNoAnchorLineage
        | NodeLifecycleError::WarmStartMultipleAnchorLineages { .. }
        | NodeLifecycleError::WarmStartWalReplay(_)
        | NodeLifecycleError::WarmStartNoProvenance
        | NodeLifecycleError::WarmStartForwardReplayUnsupported { .. }
        | NodeLifecycleError::WarmStartBootstrap(_) => EXIT_NODE_WARM_START_RECOVERY_FAILED,
    }
}

fn run_node_lifecycle_inner(cli: &Cli) -> Result<(), NodeLifecycleError> {
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
    let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path))
        .map_err(|e| NodeLifecycleError::ChainDbOpen(format!("{e:?}")))?;
    let mut wal = FileWalStore::open(wal_dir)
        .map_err(|e| NodeLifecycleError::WalOpen(format!("{e:?}")))?;

    // 4. Classify first-run vs warm-start as a pure function of on-disk
    //    state. (The same `(tip, snapshots)` axes `bootstrap_initial_state`
    //    branches on.)
    let tip = ChainDb::tip(&chaindb)
        .map_err(|e| NodeLifecycleError::OnDiskRead(format!("{e:?}")))?;
    let snapshot_slots = SnapshotStore::list_snapshot_slots(&chaindb)
        .map_err(|e| NodeLifecycleError::OnDiskRead(format!("{e:?}")))?;
    let start = classify_start(tip.is_some(), !snapshot_slots.is_empty());

    match start {
        // 5a. FirstRun: Mithril-only bootstrap (L2). Fail-closed; no
        //     genesis/bundle/cold/tip fallback.
        NodeStart::FirstRun => first_run_mithril_bootstrap(cli, &chaindb, &mut wal),
        // 5b. WarmStart: production warm-start recovery (L3). Replay the
        //     WAL, restore + verify the recovered seed-epoch sidecar through
        //     the single bootstrap authority. Fail closed; NO bundle fallback.
        NodeStart::WarmStart => {
            let state = warm_start_recovery(&chaindb, &wal)?;
            let epoch = state
                .seed_epoch_consensus_inputs
                .as_ref()
                .map(|s| s.epoch_no.0);
            let tip_slot = state.tip.as_ref().map(|t| t.slot.0);
            // Honest success record. L3 does not sync (L4) or produce (L5).
            eprintln!(
                "ade_node --mode node: warm-start recovery complete \
                 (recovered seed-epoch consensus inputs epoch={epoch:?}, recovered tip slot={tip_slot:?}). \
                 Sync (L4) and produce (L5) are not wired; NO block produced."
            );
            Ok(())
        }
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
/// L3 scope: snapshot-at-tip only. `bootstrap_initial_state`'s warm-start
/// branch restores the sidecar; for a snapshot exactly at the target it
/// returns BEFORE the replay-forward fold that is the SOLE consumer of
/// `era_schedule` / `ledger_view` (`materialize_rolled_back_state` degenerate
/// branch). A snapshot strictly below the tip would force forward replay —
/// that is L4 durable-apply territory (L4c owns its crash-window proof) — so
/// it fails closed here, making the deterministic placeholder schedule/view
/// passed below provably unconsumed.
///
/// `wal` is read-only here (`read_all` takes `&self`); L3 appends nothing.
fn warm_start_recovery(
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

    // 2. Replay the WAL from the INDEPENDENT anchor_fp. L3 has no AdmitBlock
    //    WAL entries (those arrive with L4's durable apply), so the
    //    preserved-block-bytes map is empty; an AdmitBlock referencing absent
    //    bytes fails closed inside `replay_from_anchor` (BlockBytesMissing).
    let entries = wal
        .read_all()
        .map_err(|e| NodeLifecycleError::WarmStartWalReplay(format!("{e:?}")))?;
    let block_bytes: BTreeMap<Hash32, Vec<u8>> = BTreeMap::new();
    let replay = replay_from_anchor(&anchor_fp, &entries, &block_bytes)
        .map_err(|e| NodeLifecycleError::WarmStartWalReplay(format!("{e:?}")))?;
    let provenance = replay
        .provenance
        .ok_or(NodeLifecycleError::WarmStartNoProvenance)?;

    // 3. Snapshot-at-tip guard. The only consumer of era_schedule/ledger_view
    //    is the replay-forward fold, reached only when the nearest snapshot is
    //    strictly below the target. Require a snapshot exactly at the tip so
    //    that path is unreachable; otherwise fail closed (L4 territory).
    let tip = ChainDb::tip(chaindb).map_err(|e| NodeLifecycleError::OnDiskRead(format!("{e:?}")))?;
    if let Some(t) = &tip {
        if SnapshotStore::get_snapshot(chaindb, t.slot)
            .map_err(|e| NodeLifecycleError::OnDiskRead(format!("{e:?}")))?
            .is_none()
        {
            return Err(NodeLifecycleError::WarmStartForwardReplayUnsupported {
                tip_slot: t.slot.0,
            });
        }
    }

    // Deterministic placeholders, provably unconsumed (snapshot-at-tip guard
    // ⇒ `materialize_rolled_back_state` takes the degenerate branch and never
    // folds a block). NOT bundle-derived: a constant empty schedule + empty
    // leadership view.
    let era_schedule = make_node_schedule(SlotNo(0), EpochNo(0));
    let ledger_view = PoolDistrView::new(
        EpochNo(0),
        0,
        ActiveSlotsCoeff { numer: 0, denom: 1 },
        BTreeMap::new(),
    );

    // 4. The single authority. RequiredFromRecoveredProvenance runs the
    //    fail-closed sidecar verify chain; no `--consensus-inputs-path`
    //    fallback exists inside it.
    bootstrap_initial_state(BootstrapInputs {
        chaindb,
        snapshot_store: chaindb,
        era_schedule: &era_schedule,
        ledger_view: &ledger_view,
        genesis_initial: None,
        seed_epoch_consensus_source:
            SeedEpochConsensusSource::RequiredFromRecoveredProvenance(provenance),
    })
    .map_err(|e| NodeLifecycleError::WarmStartBootstrap(format!("{e:?}")))
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
) -> Result<(), NodeLifecycleError> {
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

    // Honest success record. L2 does not sync (L4) or produce (L5).
    eprintln!(
        "ade_node --mode node: first-run Mithril bootstrap complete \
         (anchor initial_ledger_fingerprint={:?}, epoch={}). \
         Sync (L4) and produce (L5) are not wired; NO block produced.",
        out.anchor.initial_ledger_fingerprint, canonical.epoch_no.0
    );
    Ok(())
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
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

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
    const BLOCK_HASH_HEX: &str =
        "2222222222222222222222222222222222222222222222222222222222222222";
    const CERT_HASH_HEX: &str =
        "6666666666666666666666666666666666666666666666666666666666666666";
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
        };
        Fixture { _dir: dir, cli }
    }

    #[test]
    fn first_run_mithril_positive_persists_and_succeeds() {
        let f = fixture(
            Some(&manifest_json(CERTIFIED_SLOT, NETWORK_MAGIC, GENESIS_HASH_HEX)),
            UTXO_JSON,
            &consensus_inputs_json(EPOCH_NO, EPOCH_START_SLOT),
            GENESIS_HASH_HEX,
            CERTIFIED_SLOT, // operator seed point == manifest certified point => binding ok
            NETWORK_MAGIC,
        );
        let r = run_node_lifecycle_inner(&f.cli);
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

    #[test]
    fn first_run_fails_closed_on_missing_manifest() {
        let f = fixture(
            None, // no --mithril-manifest-path
            UTXO_JSON,
            &consensus_inputs_json(EPOCH_NO, EPOCH_START_SLOT),
            GENESIS_HASH_HEX,
            CERTIFIED_SLOT,
            NETWORK_MAGIC,
        );
        let r = run_node_lifecycle_inner(&f.cli);
        assert_eq!(
            r,
            Err(NodeLifecycleError::MissingFlag("--mithril-manifest-path"))
        );
    }

    #[test]
    fn first_run_fails_closed_on_binding_mismatch() {
        // Operator seed point (seed_slot) ≠ manifest certified point =>
        // verify_mithril_binding CertifiedPointMismatch, before any admit.
        let f = fixture(
            Some(&manifest_json(CERTIFIED_SLOT, NETWORK_MAGIC, GENESIS_HASH_HEX)),
            UTXO_JSON,
            &consensus_inputs_json(EPOCH_NO, EPOCH_START_SLOT),
            GENESIS_HASH_HEX,
            CERTIFIED_SLOT + 1, // genuinely different point
            NETWORK_MAGIC,
        );
        let r = run_node_lifecycle_inner(&f.cli);
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
            SnapshotStore::list_snapshot_slots(&chaindb).unwrap().is_empty(),
            "no state may be admitted when the binding fails"
        );
    }

    #[test]
    fn first_run_fails_closed_on_epoch_mismatch() {
        // Consensus inputs for an epoch whose window does NOT contain the
        // manifest certified slot => EpochMismatch, before the composer.
        // Use an epoch window far from CERTIFIED_SLOT.
        let other_start = EPOCH_START_SLOT + 432_000; // next epoch window
        let f = fixture(
            Some(&manifest_json(CERTIFIED_SLOT, NETWORK_MAGIC, GENESIS_HASH_HEX)),
            UTXO_JSON,
            &consensus_inputs_json(EPOCH_NO + 1, other_start),
            GENESIS_HASH_HEX,
            CERTIFIED_SLOT,
            NETWORK_MAGIC,
        );
        let r = run_node_lifecycle_inner(&f.cli);
        assert!(
            matches!(r, Err(NodeLifecycleError::EpochMismatch { .. })),
            "epoch mismatch must fail closed, got {r:?}"
        );
    }

    #[test]
    fn first_run_fails_closed_on_malformed_extraction() {
        let f = fixture(
            Some(&manifest_json(CERTIFIED_SLOT, NETWORK_MAGIC, GENESIS_HASH_HEX)),
            "{ not valid utxo json",
            &consensus_inputs_json(EPOCH_NO, EPOCH_START_SLOT),
            GENESIS_HASH_HEX,
            CERTIFIED_SLOT,
            NETWORK_MAGIC,
        );
        let r = run_node_lifecycle_inner(&f.cli);
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
            put_tip_and_snapshot(&chaindb, WARM_TIP_SLOT);
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
    fn warm_start_dispatch_succeeds_end_to_end() {
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
        let r = run_node_lifecycle_inner(&cli);
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
            append_seed_epoch_provenance(&mut wal, &Hash32([0x99; 32]), WARM_EPOCH, &bytes).unwrap();
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
    fn warm_start_fails_closed_when_forward_replay_needed() {
        // Valid sidecar + WAL provenance, but the snapshot is BELOW the tip,
        // so recovery would require forward block replay -> L4 territory.
        // Fail closed rather than replay with a non-recovered leadership
        // view (this is what makes the era_schedule/ledger_view placeholders
        // provably unconsumed in the success path).
        let d = fresh_warm_dirs();
        let record = warm_sample_record(WARM_ANCHOR_FP, WARM_EPOCH);
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        {
            let (chaindb, mut wal) = open_warm_stores(&d);
            chaindb
                .put_seed_epoch_consensus_inputs(&WARM_ANCHOR_FP, &bytes)
                .unwrap();
            append_seed_epoch_provenance(&mut wal, &WARM_ANCHOR_FP, WARM_EPOCH, &bytes).unwrap();
            // Block at the tip slot, but snapshot one slot BELOW it.
            chaindb
                .put_block(&StoredBlock {
                    hash: Hash32([0xBB; 32]),
                    slot: SlotNo(WARM_TIP_SLOT),
                    bytes: vec![0xAB; 8],
                })
                .unwrap();
            let ledger = LedgerState::new(CardanoEra::Conway);
            let chain_dep = PraosChainDepState::genesis(Nonce(Hash32([0xCD; 32])));
            PersistentSnapshotCache::new(&chaindb)
                .capture(SlotNo(WARM_TIP_SLOT - 1), &ledger, &chain_dep)
                .unwrap();
        }
        let (chaindb, wal) = open_warm_stores(&d);
        let r = warm_start_recovery(&chaindb, &wal);
        assert!(
            matches!(
                r,
                Err(NodeLifecycleError::WarmStartForwardReplayUnsupported {
                    tip_slot
                }) if tip_slot == WARM_TIP_SLOT
            ),
            "forward replay needed must fail closed, got {r:?}"
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
        }
    }
}
