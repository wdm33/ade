// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED admission-mode binary bootstrap (PHASE4-N-M-B S5).
//!
//! `dispatch_admission` is the closed entry point invoked by
//! `main.rs` when `--mode admission` is set. It composes:
//!   1. CLI extraction (`AdmissionCli`),
//!   2. JSON UTxO seed import (CN-SEED-01),
//!   3. BootstrapAnchor mint (CN-ANCHOR-01),
//!   4. `seed_to_snapshot` capture (CN-STORE-08, CN-ADMIT-02),
//!   5. `bootstrap_initial_state` warm-start (CN-NODE-01),
//!   6. `FileWalStore` open + chain verify (DC-WAL-02),
//!   7. `run_admission` (CN-ADMIT-01).
//!
//! Sub-cluster C (operator pass) wires the live N2nDialer to the
//! `peer_events` channel; in B5 the channel has no producer, so
//! the runner waits on the shutdown signal. This is intentional:
//! B5 closes the dispatch + bootstrap path, NOT the live wire
//! path. The honest-scope claim is "the binary admits blocks via
//! authority, appends WAL, derives verdicts" — which is what the
//! B6 hermetic loopback test will prove.

use std::fs::{self, File};
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::wal::WalStore;
use ade_network::codec::handshake::VersionTable;
use ade_network::codec::version::N2NVersion;
use ade_network::handshake::version_table::N2N_SUPPORTED;
use ade_runtime::admission::{
    dial_for_admission, run_admission_wire_pump,
    AdmissionPeerEvent as RuntimeAdmissionPeerEvent,
};
use ade_ledger::bootstrap_anchor::SeedProvenance;
use ade_runtime::bootstrap_anchor::{mint, MintInputs};
use ade_runtime::chaindb::{
    InMemoryChainDb, PersistentChainDb, PersistentChainDbOptions, SnapshotStore,
};
use ade_runtime::consensus_inputs::{
    import_live_consensus_inputs, LiveConsensusInputsImportError, LiveLedgerView,
};
use ade_runtime::seed_import::import_cardano_cli_json_utxo;
use ade_runtime::wal::FileWalStore;
use ade_types::{CardanoEra, EpochNo, Hash32, SlotNo};
use tokio::sync::{mpsc, watch};

use super::runner::{
    run_admission, AdmissionExitCode, AdmissionInputs, AdmissionPeerEvent, MemPhaseDiagnostic,
};
use super::seed_to_snapshot::seed_to_snapshot;
use crate::admission_log::AdmissionLogWriter;
use crate::cli::AdmissionCli;
use crate::mem_measure::rss_sampler::{
    sample_private_dirty_kib, sample_rss_anon_kib, sample_vm_hwm_kib, sample_vm_rss_kib,
};

/// Closed admission-bootstrap error sum. The binary maps each
/// variant to the generic-startup exit code; this is the
/// authority-fatal boundary between CLI / setup and the runner.
#[derive(Debug)]
pub enum AdmissionBootstrapError {
    LogFileCreate(io::ErrorKind),
    BadGenesisHashHex,
    BadSeedBlockHashHex,
    JsonSeedImport(String),
    ChainDbOpen(String),
    SnapshotDirCreate(io::ErrorKind),
    WalDirCreate(io::ErrorKind),
    SeedToSnapshot(String),
    /// S3f-2-pre (DC-EVIEW-08): the manifest-bound bootstrap cert-state package failed
    /// to verify/import (missing one side, hash/network/era mismatch, malformed, or a
    /// cert-state that does not decode). FAIL-CLOSED before any bootstrap state durables.
    BootstrapCertState(String),
    /// EPOCH-CONSENSUS-VIEW S3f-4d-mat (DC-EPOCH-11): building the live reduced-UTxO
    /// checkpoint from the seed UTxO failed. FAIL-CLOSED -- without the authoritative
    /// reduced checkpoint, no live EpochConsensusView can be derived.
    ReducedCheckpoint(String),
    BootstrapInitialState(String),
    FileWalStoreOpen(String),
    WalChainBreak(String),
    /// The consensus-inputs bundle's `source_tip` (the chain-sync intersect
    /// point — see `spawn_wire_pumps_for_admission`) does not equal the recovered
    /// ledger tip (the `--seed-point`). The seed UTxO and the bundle MUST be
    /// extracted at the same chain point; otherwise the peer rolls forward from
    /// `source_tip` while the ledger sits at the seed, applying blocks across a
    /// gap (a hollow hash-agreement, not a validated chain). Fail closed
    /// (RO-LIVE-05 / admission catch-up integrity).
    SeedBundleTipMismatch {
        seed_slot: u64,
        source_tip_slot: u64,
    },
    /// LiveConsensusInputs bundle import failed (PHASE4-N-M-C
    /// CN-CONS-IN-01 / DC-CONS-IN-01).
    ConsensusInputsImport(LiveConsensusInputsImportError),
    /// The forge-capable seed import could not source the current protocol
    /// parameters from the bundle: the `protocol_params_json` preimage was
    /// absent, did not hash-bind to `protocol_params_hash`, or did not parse
    /// (PHASE4-N-F-G-A S2a, CE-G-A-2a). Fail closed — no default substitution.
    ForgeCurrentPParams(String),
    /// Persisting the seed-epoch anchor lineage (the anchor-fp-keyed sidecar +
    /// WAL provenance) failed (PHASE4-N-F-G-I). The pre-seed mints the anchor
    /// and MUST persist the lineage a `--mode node` WarmStart recovers; a store
    /// that cannot record it fails rather than silently proceed.
    SeedEpochLineagePersist(String),
}

/// SOLE admission-dispatch entry point. Performs the closed
/// bootstrap sequence + invokes [`run_admission`]. Returns the
/// binary's `ExitCode`.
pub async fn dispatch_admission(
    acli: AdmissionCli,
    shutdown: watch::Receiver<bool>,
) -> ExitCode {
    let log_file = match File::create(&acli.log_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "ade_node: cannot create admission log file {}: {:?}",
                acli.log_path.display(),
                e.kind()
            );
            return ExitCode::from(crate::node::EXIT_GENERIC_STARTUP as u8);
        }
    };
    let writer = AdmissionLogWriter::new(log_file);

    match run_admission_inner(&acli, writer, shutdown).await {
        Ok(code) => ExitCode::from(code.as_i32() as u8),
        Err(e) => {
            eprintln!("ade_node admission bootstrap fatal: {:?}", e);
            ExitCode::from(crate::node::EXIT_GENERIC_STARTUP as u8)
        }
    }
}

/// Fail-closed consistency check: the consensus-inputs bundle's `source_tip`
/// (used as the chain-sync intersect point in `spawn_wire_pumps_for_admission`)
/// MUST equal the recovered ledger tip (the `--seed-point`). If they differ, the
/// seed UTxO and the bundle were extracted at different chain points, so the peer
/// rolls forward from `source_tip` while the ledger sits at the seed — applying
/// blocks across a gap (a hollow hash-agreement rather than a validated chain).
/// S3f-2-pre (DC-EVIEW-08): import the manifest-bound bootstrap cert state, discovered by
/// convention next to the seed (`<seed>.manifest` + `<seed>.certstate`). Both present ->
/// verify the manifest binds the seed + cert-state by hash + network/era, then decode the
/// COMPLETE `CertState` via the canonical codec. Exactly one present -> FAIL CLOSED (a seed
/// without its manifest-bound cert state, or a cert state without its binding manifest).
/// Neither present -> the pre-import empty `CertState` (transition). The verify runs BEFORE
/// any bootstrap state is durably written. This only populates the bootstrap LedgerState's
/// cert_state for later self-derived epoch views; it does NOT change live producer behaviour.
fn import_bootstrap_cert_state(
    seed_path: &std::path::Path,
    network_magic: u32,
) -> Result<ade_ledger::delegation::CertState, AdmissionBootstrapError> {
    use ade_ledger::bootstrap_manifest::verify_and_import_cert_state;
    let read = |p: &std::path::Path, what: &str| {
        fs::read(p).map_err(|e| {
            AdmissionBootstrapError::BootstrapCertState(format!("read {what}: {:?}", e.kind()))
        })
    };
    let manifest_path = PathBuf::from(format!("{}.manifest", seed_path.display()));
    let cert_path = PathBuf::from(format!("{}.certstate", seed_path.display()));
    match (manifest_path.exists(), cert_path.exists()) {
        (false, false) => Ok(ade_ledger::delegation::CertState::new()),
        (true, false) => Err(AdmissionBootstrapError::BootstrapCertState(
            "manifest present but its bound cert-state artifact is missing".into(),
        )),
        (false, true) => Err(AdmissionBootstrapError::BootstrapCertState(
            "cert-state artifact present but its binding manifest is missing".into(),
        )),
        (true, true) => {
            let manifest_bytes = read(&manifest_path, "manifest")?;
            let seed_bytes = read(seed_path, "seed")?;
            let cert_bytes = read(&cert_path, "cert-state")?;
            let (_manifest, cert_state) = verify_and_import_cert_state(
                &manifest_bytes,
                &seed_bytes,
                &cert_bytes,
                network_magic,
                CardanoEra::Conway,
            )
            .map_err(|e| {
                AdmissionBootstrapError::BootstrapCertState(format!("manifest verify: {e:?}"))
            })?;
            Ok(cert_state)
        }
    }
}

/// EPOCH-CONSENSUS-VIEW S3f-4d-mat-1 (DC-EPOCH-11): build the live reduced-UTxO checkpoint
/// from the seed UTxO -- the authoritative reduced-stake state the EVIEW window driver
/// (DC-EVIEW-10) advances per admitted block. Reduces each output (reduce_txout, DC-EVIEW-04)
/// and builds the durable redb checkpoint, returning its commitment fingerprint. Disk-backed
/// (redb in the snapshot dir); the transient reduced map (~70 bytes/entry) is freed here. It
/// adds an ADDITIONAL durable artifact and changes NO existing bootstrap output, so the
/// existing follow/forge path stays byte-identical (DC-EPOCH-11 point 8).
fn build_live_reduced_checkpoint(
    snapshot_dir: &std::path::Path,
    utxo: &ade_ledger::utxo::UTxOState,
    seed_slot: SlotNo,
) -> Result<Hash32, ade_runtime::chaindb::ReducedCheckpointError> {
    use ade_ledger::reduced_utxo::{reduce_txout, ReducedStakeRef};
    let mut reduced: std::collections::BTreeMap<
        ade_types::tx::TxIn,
        (ade_types::tx::Coin, ReducedStakeRef),
    > = std::collections::BTreeMap::new();
    for (txin, txout) in utxo.utxos.iter() {
        reduced.insert(txin.clone(), reduce_txout(txout));
    }
    let checkpoint =
        ade_runtime::chaindb::ReducedUtxoCheckpoint::open(&reduced_checkpoint_path(snapshot_dir))?;
    let fp = checkpoint.build_from(&reduced)?;
    // S3f-4d-mat-3: seal the seed records as the IMMUTABLE bootstrap baseline + record the
    // seed slot (DC-EPOCH-11). The advancer resumes from seed_slot+1 (the seed UTxO already
    // reflects every block up to the anchor, so it is never re-applied), and a reorg rollback
    // re-materializes the live table from this baseline.
    checkpoint.seal_bootstrap(seed_slot)?;
    Ok(fp)
}

/// The durable path of the live reduced checkpoint (in the snapshot dir, beside chain.db).
fn reduced_checkpoint_path(snapshot_dir: &std::path::Path) -> std::path::PathBuf {
    snapshot_dir.join("reduced-checkpoint.redb")
}

fn check_seed_bundle_tip_consistency(
    source_tip_slot: SlotNo,
    source_tip_hash: &Hash32,
    seed_slot: SlotNo,
    seed_hash: &Hash32,
) -> Result<(), AdmissionBootstrapError> {
    if source_tip_slot != seed_slot || source_tip_hash != seed_hash {
        return Err(AdmissionBootstrapError::SeedBundleTipMismatch {
            seed_slot: seed_slot.0,
            source_tip_slot: source_tip_slot.0,
        });
    }
    Ok(())
}

async fn run_admission_inner(
    acli: &AdmissionCli,
    writer: AdmissionLogWriter<File>,
    shutdown: watch::Receiver<bool>,
) -> Result<AdmissionExitCode, AdmissionBootstrapError> {
    // 1. Import the JSON UTxO seed.
    let (utxo, utxo_fp) = import_cardano_cli_json_utxo(&acli.json_seed_path)
        .map_err(|e| AdmissionBootstrapError::JsonSeedImport(format!("{:?}", e)))?;
    // MEM-OPT-OPS S2 (CE-OPS-2): capture the seed-import peak RIGHT HERE -- after
    // import() returns, BEFORE the chain.db snapshot write (a later, larger
    // transient). VmHWM is the import-specific peak; emitted as `seed_import`.
    let seed_import_rss_kib = sample_vm_rss_kib().map(|s| s.0).unwrap_or(0);
    let seed_import_hwm_kib = sample_vm_hwm_kib().map(|h| h.0).unwrap_or(0);
    // MEM-OPT-OPS S3: the OWNED footprint at the SAME post-import instant.
    let seed_import_rss_anon_kib = sample_rss_anon_kib().map(|s| s.0).unwrap_or(0);
    let seed_import_private_dirty_kib = sample_private_dirty_kib().map(|s| s.0).unwrap_or(0);

    // 2. Parse fixed-size hashes.
    let genesis_hash = parse_hash32(&acli.genesis_hash_hex)
        .ok_or(AdmissionBootstrapError::BadGenesisHashHex)?;
    let seed_block_hash = parse_hash32(&acli.seed_block_hash_hex)
        .ok_or(AdmissionBootstrapError::BadSeedBlockHashHex)?;

    // 3. Open persistent ChainDb + ensure dirs exist.
    let snapshot_dir = acli.snapshot_dir.clone();
    fs::create_dir_all(&snapshot_dir)
        .map_err(|e| AdmissionBootstrapError::SnapshotDirCreate(e.kind()))?;
    fs::create_dir_all(&acli.wal_dir)
        .map_err(|e| AdmissionBootstrapError::WalDirCreate(e.kind()))?;

    let chaindb_path: PathBuf = snapshot_dir.join("chain.db");
    let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path))
        .map_err(|e| AdmissionBootstrapError::ChainDbOpen(format!("{:?}", e)))?;

    // S2a (CE-G-A-2a): the forge-capable seed import REQUIRES the current
    // protocol parameters. Import the operator consensus-inputs bundle and bind
    // its `protocol_params_json` preimage to the fingerprinted
    // `protocol_params_hash`, then parse the current ProtocolParameters — fail
    // closed if the preimage is absent or unbound. Installed into BOTH recovered-
    // ledger construction sites (the captured snapshot + the runner ledger) so
    // the node forge reads a truthful current protocol version, not the default.
    let canonical = import_live_consensus_inputs(&acli.consensus_inputs_path)
        .map_err(AdmissionBootstrapError::ConsensusInputsImport)?;
    let current_pparams = canonical
        .require_forge_current_pparams()
        .map_err(|e| AdmissionBootstrapError::ForgeCurrentPParams(format!("{e:?}")))?;

    // Fail-closed (RO-LIVE-05): the bundle's `source_tip` is the chain-sync
    // intersect point (spawn_wire_pumps_for_admission); it MUST equal the
    // recovered ledger tip (the seed point), or the peer's roll-forward applies
    // blocks across a gap on a stale ledger (a hollow hash-agreement). The seed
    // UTxO and the consensus-inputs bundle must be extracted at the same tip.
    check_seed_bundle_tip_consistency(
        canonical.source_tip_slot,
        &canonical.source_tip_hash,
        SlotNo(acli.seed_point_slot),
        &seed_block_hash,
    )?;

    // S3f-2-pre (DC-EVIEW-08): import the manifest-bound bootstrap cert state (the
    // per-credential delegation/reward continuation state) so the captured snapshot --
    // and warm-start -- carry it for later self-derived epoch views. Discovered by
    // convention next to the seed; fail-closed on a partial/mismatched package; empty
    // (transition) when no package is present. NO live producer behaviour change.
    let bootstrap_cert_state =
        import_bootstrap_cert_state(&acli.consensus_inputs_path, acli.network_magic)?;

    // 4. seed_to_snapshot (uses ChainDb as the SnapshotStore).
    let chain_dep_seed = PraosChainDepState::genesis(Nonce::ZERO);
    let initial_fp = seed_to_snapshot(
        utxo.clone(),
        chain_dep_seed.clone(),
        SlotNo(acli.seed_point_slot),
        &chaindb,
        current_pparams.clone(),
        bootstrap_cert_state.clone(),
    )
    .map_err(|e| AdmissionBootstrapError::SeedToSnapshot(format!("{:?}", e)))?;

    // MEM-OPT-UTXO-DISK S0 (CE-UD-0): phase-resolved owned diagnostic. RED-only,
    // gated behind the `ADE_MEM_PHASE_DIAGNOSTIC` env toggle (absent on every
    // normal run). t2 = owned RIGHT AFTER seed_to_snapshot returns (the snapshot-
    // serialization transient is freed by now, but mimalloc's lazy MADV_FREE may
    // retain the pages). t3 = owned right after a forced allocator collect -- the
    // decisive control: if owned drops, the footprint was reclaimable
    // serialization memory; if it stays, it is live working set. The collect is a
    // MEASUREMENT INTERVENTION (ade_mem_diag -- the workspace's quarantined RED
    // unsafe surface); it changes no authoritative output (only freed memory is
    // returned to the OS).
    let mem_phase_diagnostic = if std::env::var_os("ADE_MEM_PHASE_DIAGNOSTIC").is_some() {
        let t2_rss = sample_vm_rss_kib().map(|s| s.0).unwrap_or(0);
        let t2_hwm = sample_vm_hwm_kib().map(|h| h.0).unwrap_or(0);
        let t2_anon = sample_rss_anon_kib().map(|s| s.0).unwrap_or(0);
        let t2_dirty = sample_private_dirty_kib().map(|s| s.0).unwrap_or(0);
        ade_mem_diag::force_allocator_collect_for_diagnostic_only();
        let t3_rss = sample_vm_rss_kib().map(|s| s.0).unwrap_or(0);
        let t3_hwm = sample_vm_hwm_kib().map(|h| h.0).unwrap_or(0);
        let t3_anon = sample_rss_anon_kib().map(|s| s.0).unwrap_or(0);
        let t3_dirty = sample_private_dirty_kib().map(|s| s.0).unwrap_or(0);
        Some(MemPhaseDiagnostic {
            snapshot_serializing_rss_kib: t2_rss,
            snapshot_serializing_hwm_kib: t2_hwm,
            snapshot_serializing_rss_anon_kib: t2_anon,
            snapshot_serializing_private_dirty_kib: t2_dirty,
            post_reclaim_rss_kib: t3_rss,
            post_reclaim_hwm_kib: t3_hwm,
            post_reclaim_rss_anon_kib: t3_anon,
            post_reclaim_private_dirty_kib: t3_dirty,
        })
    } else {
        None
    };

    // 5. Mint anchor (kept — its lineage is persisted in step 7b).
    let anchor = mint(MintInputs {
        network_magic: acli.network_magic,
        genesis_hash,
        seed_slot: SlotNo(acli.seed_point_slot),
        seed_block_hash,
        // For B5, the seed_artifact_hash is a blake2b of the JSON
        // seed bytes — A1 already computes one for the
        // BootstrapAnchor in the importer's caller. For now we
        // recompute it here so this bootstrap function is self-
        // contained; future slice can route it through importer.
        seed_artifact_hash: blake2b_256_of_file(&acli.json_seed_path)
            .unwrap_or(Hash32([0u8; 32])),
        imported_utxo_fingerprint: utxo_fp,
        initial_ledger_fingerprint: initial_fp.clone(),
        seed_provenance: SeedProvenance::CardanoCliJson,
    });

    // 6. Build the post-import ledger + chain_dep used by the
    //    runner. The ledger inherits the imported UTxO; the
    //    chain_dep state's `epoch_nonce` MUST be set to the
    //    imported bundle's epoch nonce (Eta0 of the current
    //    epoch) so the BLUE header validity check verifies VRF
    //    proofs against the correct nonce. (PHASE4-N-M-FRAG
    //    surfaced this: before FRAG + tag-24 unwrap landed, no
    //    block ever reached header validity, so the ZERO-nonce
    //    chain_dep was masked.)
    let mut ledger = ade_ledger::state::LedgerState::new(CardanoEra::Conway);
    // S3f-2-pre: the runner ledger carries the same manifest-bound bootstrap cert state
    // as the captured snapshot (a consistent bootstrap state). Empty (transition) when no
    // package is configured; track_utxo=false still skips live cert accumulation.
    ledger.cert_state = bootstrap_cert_state;
    // MEM-OPT-UTXO-DISK S2b-2c.1b-A.2: compute the constant UTxO-component fingerprint
    // ONCE from the imported UTxO (its durable copy is the snapshot already written by
    // seed_to_snapshot), then DROP the 1.9M-entry in-memory map. The live
    // track_utxo=false admission needs only this fingerprint, never the retained UTxO;
    // ledger.utxo_state stays EMPTY (UTxOState::new()).
    let static_utxo_fp =
        ade_ledger::fingerprint::StaticUtxoFp::from_bootstrap_utxo(&utxo, initial_fp.clone());
    // S3f-4d-mat-1 (DC-EPOCH-11): when the EVIEW cert-state package is present (the
    // activation is configured), build the live reduced checkpoint from the seed UTxO
    // BEFORE it is dropped. Gated on the imported cert-state so non-EVIEW bootstrap is
    // BYTE-IDENTICAL (point 8). Fail-closed: a build failure aborts bootstrap.
    if !ledger.cert_state.delegation.delegations.is_empty() {
        let reduced_checkpoint_fp =
            build_live_reduced_checkpoint(&snapshot_dir, &utxo, SlotNo(acli.seed_point_slot))
            .map_err(|e| AdmissionBootstrapError::ReducedCheckpoint(format!("{:?}", e)))?;
        let _ = reduced_checkpoint_fp; // the binding/lineage check consumes it in -mat-4
    }
    drop(utxo);
    // S2a: the runner's recovered ledger carries the oracle-bound current pparams.
    ledger.protocol_params = current_pparams.clone();

    // 7. Open file WAL + verify the chain head from the anchor.
    let mut wal_store = FileWalStore::open(&acli.wal_dir)
        .map_err(|e| AdmissionBootstrapError::FileWalStoreOpen(format!("{:?}", e)))?;
    wal_store
        .verify_chain(&initial_fp)
        .map_err(|e| AdmissionBootstrapError::WalChainBreak(format!("{:?}", e)))?;

    // 7b. PHASE4-N-F-G-I: persist the seed-epoch anchor lineage the pre-seed
    //     minted — the SAME lineage the mithril/genesis bootstraps persist —
    //     so a `--mode node` WarmStart recovers a forge-capable store seeded
    //     purely from this shared `--json-seed` + `import_live_consensus_inputs`
    //     path. Derived ONLY from the minted anchor (its initial_ledger_fp) +
    //     the imported canonical inputs; never a genesis-derived constructor.
    ade_runtime::seed_epoch_lineage::persist_seed_epoch_consensus_inputs(
        &chaindb,
        &mut wal_store,
        &anchor,
        &canonical,
    )
    .map_err(|e| AdmissionBootstrapError::SeedEpochLineagePersist(format!("{e:?}")))?;

    // 8. Build the era schedule + the LiveLedgerView from the
    //    operator-imported consensus-inputs bundle
    //    (PHASE4-N-M-C CN-CONS-IN-01 / DC-VIEW-01). The minimal
    //    schedule is consistent with the imported epoch window:
    //    the Conway entry's start slot is the imported epoch
    //    start, so the BLUE consensus path can resolve epochs
    //    within that window from a single era summary.
    let era_schedule = make_schedule_for_imported_window(&canonical.epoch_start_slot, canonical.epoch_no);
    let ledger_view = LiveLedgerView::new(canonical.clone());
    let consensus_inputs_fingerprint = canonical.fingerprint.clone();
    let consensus_inputs_epoch = canonical.epoch_no;
    let consensus_inputs_epoch_start_slot = canonical.epoch_start_slot;
    let consensus_inputs_epoch_end_slot = canonical.epoch_end_slot;

    // Build the runner's chain_dep with the imported epoch nonce
    // (Eta0 of the current epoch). The genesis seed used by
    // `seed_to_snapshot` is intentionally Nonce::ZERO (the
    // BootstrapAnchor binds the imported UTxO + the
    // initial_ledger_fp; the chain_dep nonce is a separate axis
    // supplied by the operator-imported consensus_inputs bundle).
    let chain_dep = PraosChainDepState::genesis(canonical.epoch_nonce.clone());

    // 9. Spawn one wire pump per peer; each pump produces
    //    `AdmissionPeerEvent`s into a shared channel that the
    //    runner consumes (PHASE4-N-M-C C3 — CN-PUMP-01).
    let (peer_tx, peer_events) = mpsc::channel::<AdmissionPeerEvent>(64);
    spawn_wire_pumps_for_admission(
        &acli.peer_addrs,
        acli.network_magic,
        &canonical.source_tip_hash,
        canonical.source_tip_slot,
        peer_tx,
    );

    let inputs = AdmissionInputs {
        writer,
        wal_store,
        anchor_initial_ledger_fp: initial_fp,
        ledger,
        static_utxo_fp: Some(static_utxo_fp),
        chain_dep,
        era_schedule: &era_schedule,
        ledger_view: &ledger_view,
        chaindb: &chaindb,
        peer_events,
        shutdown,
        peer_count: acli.peer_addrs.len() as u32,
        json_seed_path: acli
            .json_seed_path
            .to_string_lossy()
            .to_string(),
        wal_dir: acli.wal_dir.to_string_lossy().to_string(),
        initial_chain_tip_slot: acli.seed_point_slot,
        seed_import_rss_kib,
        seed_import_hwm_kib,
        seed_import_rss_anon_kib,
        seed_import_private_dirty_kib,
        mem_phase_diagnostic,
        consensus_inputs_fingerprint,
        consensus_inputs_epoch,
        consensus_inputs_epoch_start_slot,
        consensus_inputs_epoch_end_slot,
    };
    let _ = fingerprint(&ade_ledger::state::LedgerState::new(CardanoEra::Conway));
    // ^ touch fingerprint to ensure it stays in scope; the
    // runner uses it for verdict + WAL post_fp computation.

    Ok(run_admission(inputs).await)
}

fn parse_hash32(hex: &str) -> Option<Hash32> {
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        let pair = &hex[i * 2..i * 2 + 2];
        out[i] = u8::from_str_radix(pair, 16).ok()?;
    }
    Some(Hash32(out))
}

fn blake2b_256_of_file(path: &PathBuf) -> Option<Hash32> {
    let bytes = fs::read(path).ok()?;
    Some(ade_crypto::blake2b::blake2b_256(&bytes))
}

/// Spawn one [`run_admission_wire_pump`] per configured peer
/// address. Each pump dials its peer, completes the N2N
/// handshake, and forwards `AdmissionPeerEvent`s into the
/// shared channel. Dial failures are reported to stderr and
/// drop the corresponding sender clone, which the runner
/// observes via its connected-peer counter (clean shutdown when
/// all peers disconnect).
///
/// Honest-scope (C3): peer-address parse failures are logged
/// and skipped. C5's live-pass operator runbook is responsible
/// for the address syntax + the docker preprod target.
fn spawn_wire_pumps_for_admission(
    peer_addrs: &[String],
    network_magic: u32,
    source_tip_hash: &Hash32,
    source_tip_slot: SlotNo,
    peer_tx: tokio::sync::mpsc::Sender<AdmissionPeerEvent>,
) {
    let our_versions: VersionTable = build_n2n_version_table(network_magic);
    let start_point = ade_network::codec::chain_sync::Point::Block {
        slot: source_tip_slot,
        hash: source_tip_hash.clone(),
    };
    for raw_addr in peer_addrs {
        let addr: std::net::SocketAddr = match raw_addr.parse() {
            Ok(a) => a,
            Err(_) => {
                eprintln!("ade_node admission: skipping unparseable peer addr {raw_addr}");
                continue;
            }
        };
        let _ = network_magic; // routed via the version_data inside the pump
        let pump_versions = our_versions.clone();
        let pump_tx = peer_tx.clone();
        let start = start_point.clone();
        let label = raw_addr.clone();
        tokio::spawn(async move {
            let (transport, version) = match dial_for_admission(addr, pump_versions).await {
                Ok(pair) => pair,
                Err(e) => {
                    eprintln!(
                        "ade_node admission: dial-for-admission failed for {label}: {:?}",
                        e
                    );
                    return;
                }
            };
            let runtime_tx =
                tokio::sync::mpsc::channel::<RuntimeAdmissionPeerEvent>(64);
            let (rt_tx, mut rt_rx) = runtime_tx;
            // Bridge runtime AdmissionPeerEvent -> ade_node AdmissionPeerEvent
            // (parallel-shape closed sums; the bridge converts each variant
            // exhaustively).
            let label_for_bridge = label.clone();
            let bridge = tokio::spawn(async move {
                while let Some(evt) = rt_rx.recv().await {
                    let translated = match evt {
                        RuntimeAdmissionPeerEvent::Block { peer, block_bytes } => {
                            AdmissionPeerEvent::Block { peer, block_bytes }
                        }
                        RuntimeAdmissionPeerEvent::TipUpdate { peer, tip } => {
                            AdmissionPeerEvent::TipUpdate { peer, tip }
                        }
                        RuntimeAdmissionPeerEvent::Disconnected { peer } => {
                            AdmissionPeerEvent::Disconnected { peer }
                        }
                        // PHASE4-N-AI AI-S4a: the admission-mode runner path does
                        // not consume rollback signals; the live --mode node path
                        // consumes them via node_sync. Not forwarded here -- this is
                        // NOT a rollback->TipUpdate downgrade, just a non-consuming path.
                        RuntimeAdmissionPeerEvent::RollBackward { .. } => continue,
                    };
                    if pump_tx.send(translated).await.is_err() {
                        // Runner has dropped its receiver — exit
                        // the bridge cleanly.
                        return;
                    }
                }
                let _ = label_for_bridge;
            });
            let _ = run_admission_wire_pump(
                transport,
                label.clone(),
                start,
                version,
                network_magic,
                rt_tx,
            )
            .await;
            let _ = bridge.await;
        });
    }
}

/// Build a per-version `VersionTable` for the operator-supplied
/// network magic. Mirrors `wire_only::our_n2n_versions` — V11..V15
/// emit a 4-field NodeToNodeVersionData; V16+ emits the 5-field
/// shape adding `perasSupport`.
pub fn build_n2n_version_table(network_magic: u32) -> VersionTable {
    // PHASE4-N-F-G-L (CN-WIRE-10): build the per-version versionData via the SINGLE shared authority
    // the serve responder also uses (encode_n2n_version_params) -- initiator and responder cannot
    // diverge. Byte-identical to the prior inline encoding; now one source of truth.
    VersionTable(
        N2N_SUPPORTED
            .iter()
            .map(|(v, _)| {
                (
                    N2NVersion::new(*v),
                    ade_network::handshake::version_table::encode_n2n_version_params(
                        *v,
                        network_magic,
                    ),
                )
            })
            .collect(),
    )
}

/// Build an era schedule whose single Conway entry starts at the
/// imported epoch's start slot. C2 honest-scope: admission is
/// single-epoch, so a single-entry schedule with safe-zone equal
/// to the epoch length is enough — multi-epoch admission is a
/// future cluster (¬P-C5 / ¬P-C6).
fn make_schedule_for_imported_window(
    epoch_start_slot: &SlotNo,
    epoch_no: EpochNo,
) -> EraSchedule {
    EraSchedule::new(
        ade_core::consensus::BootstrapAnchorHash(Hash32([0u8; 32])),
        epoch_start_slot.0,
        vec![ade_core::consensus::EraSummary {
            randomness_stabilisation_window_slots: None,
            era: CardanoEra::Conway,
            start_slot: *epoch_start_slot,
            // PHASE4-N-M-NONCE: the imported window starts at
            // the bundle's epoch number, NOT epoch 0. Without
            // this `era_schedule.locate(slot).epoch` returns 0
            // for every slot in the window, which causes
            // `LiveLedgerView` to refuse every lookup (it gates
            // by `epoch == inputs.epoch_no`) — surfacing as
            // `Header(VrfCert(VerificationFailed))` at the
            // `pool_active_stake_missing` stage.
            start_epoch: epoch_no,
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 432_000,
        }],
    )
    .expect("era schedule for imported window")
}

// Silence unused-import lint when no callsite uses InMemoryChainDb
// directly; we re-export the type so future slices can swap stores
// without re-importing.
#[allow(dead_code)]
fn _in_memory_chaindb_marker(_: InMemoryChainDb) {}

// Silence unused-import for SnapshotStore — used as a trait bound
// transitively through PersistentChainDb.
#[allow(dead_code)]
fn _snapshot_store_marker(_s: &dyn SnapshotStore) {}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// S3f-4d-mat-1 (DC-EPOCH-11): the live reduced checkpoint builds from the seed UTxO
    /// into a DURABLE, COMPLETE, deterministic redb store -- the authoritative reduced-stake
    /// state the EVIEW window driver advances. The commitment fingerprint is a pure function
    /// of the UTxO (replay-equivalent).
    #[test]
    fn live_reduced_checkpoint_builds_durable_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let utxo = ade_ledger::utxo::UTxOState::new();
        let fp1 = build_live_reduced_checkpoint(dir.path(), &utxo, SlotNo(0)).expect("build");
        // durable + complete + reopenable.
        let cp = ade_runtime::chaindb::ReducedUtxoCheckpoint::open(&reduced_checkpoint_path(dir.path()))
            .expect("reopen");
        assert!(cp.is_complete().expect("complete"), "the built checkpoint is marked complete");
        // deterministic: a fresh build of the same UTxO -> the same commitment fingerprint.
        let dir2 = tempfile::tempdir().unwrap();
        let fp2 = build_live_reduced_checkpoint(dir2.path(), &utxo, SlotNo(0)).expect("build2");
        assert_eq!(fp1, fp2, "the reduced checkpoint commitment is a pure function of the UTxO");
    }

    /// RO-LIVE-05 / #18: the bundle's `source_tip` (chain-sync intersect point)
    /// must equal the recovered ledger tip (the seed point), else the catch-up
    /// applies blocks across a gap. The guard fails closed on slot/hash mismatch.
    #[test]
    fn seed_bundle_tip_consistency_fails_closed_on_mismatch() {
        let h1 = Hash32([1u8; 32]);
        let h2 = Hash32([2u8; 32]);
        // Matching slot + hash → Ok (the proper flow: seed + bundle at the same tip).
        assert!(check_seed_bundle_tip_consistency(SlotNo(100), &h1, SlotNo(100), &h1).is_ok());
        // Slot mismatch → fail closed.
        assert!(matches!(
            check_seed_bundle_tip_consistency(SlotNo(101), &h1, SlotNo(100), &h1),
            Err(AdmissionBootstrapError::SeedBundleTipMismatch {
                seed_slot: 100,
                source_tip_slot: 101
            })
        ));
        // Hash mismatch at the same slot → fail closed (the #18 footgun: a stale
        // seed paired with a fresh bundle that resolves a different block).
        assert!(matches!(
            check_seed_bundle_tip_consistency(SlotNo(100), &h2, SlotNo(100), &h1),
            Err(AdmissionBootstrapError::SeedBundleTipMismatch { .. })
        ));
    }

    /// PHASE4-N-M-SCHED S1 regression: the imported-window era
    /// schedule MUST use the bundle's epoch_no as `start_epoch`,
    /// NOT a hardcoded `EpochNo(0)`. Before this fix,
    /// `era_schedule.locate(slot_in_window).epoch` returned 0,
    /// which caused `LiveLedgerView` (gated by
    /// `epoch == inputs.epoch_no`) to refuse every per-pool
    /// lookup → cascade `Header(VrfCert(VerificationFailed))`.
    #[test]
    fn imported_window_schedule_uses_bundle_epoch() {
        let epoch_start_slot = SlotNo(124_070_400);
        let bundle_epoch = EpochNo(291);
        let schedule = make_schedule_for_imported_window(&epoch_start_slot, bundle_epoch);

        // A slot well inside the window must resolve to the
        // bundle's epoch, not 0.
        let inside_slot = SlotNo(124_140_000);
        let location = schedule
            .locate(inside_slot)
            .expect("imported window must resolve a slot in its declared range");
        assert_eq!(
            location.epoch, bundle_epoch,
            "imported window must resolve slot {} to epoch {}, got {:?}",
            inside_slot.0, bundle_epoch.0, location.epoch
        );

        // The first slot of the window also belongs to the
        // bundle's epoch.
        let first_loc = schedule
            .locate(epoch_start_slot)
            .expect("start of window must resolve");
        assert_eq!(first_loc.epoch, bundle_epoch);
    }
}
