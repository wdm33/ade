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
use ade_core::consensus::BootstrapAnchorHash;
use ade_ledger::consensus_view::PoolDistrView;
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::state::LedgerState;
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
    /// Warm-start arm reached: the production warm-start recovery (L3)
    /// is not wired yet. Fail closed — NO bundle fallback is permitted.
    WarmStartRecoveryNotWired,
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
        NodeLifecycleError::WarmStartRecoveryNotWired => EXIT_NODE_LIFECYCLE_UNWIRED,
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
        // 5b. WarmStart: production warm-start recovery (L3) — not wired.
        //     Fail closed; NO bundle fallback.
        NodeStart::WarmStart => Err(NodeLifecycleError::WarmStartRecoveryNotWired),
    }
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
        NodeLifecycleError::WarmStartRecoveryNotWired => {
            eprintln!(
                "ade_node --mode node: warm start detected (non-empty store). The production \
                 warm-start recovery (PHASE4-N-F-C L3) is not wired yet; failing closed. \
                 No bundle fallback is permitted."
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
}
