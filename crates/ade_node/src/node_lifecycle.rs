// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED `--mode node` Ade node lifecycle owner (PHASE4-N-F-C, L1).
//!
//! `PHASE4-N-F-C-LIFECYCLE-OWNER`: this module is THE single production
//! recovered-state lifecycle owner for PHASE4-N-F-C — see
//! `docs/clusters/PHASE4-N-F-C/cluster.md` and the L1 slice doc
//! `docs/clusters/PHASE4-N-F-C/C1-production-lifecycle-owner.md`.
//!
//! L1 scope (this slice) is the owner skeleton + the first-run-vs-
//! warm-start branch only:
//!   1. open a persistent `ChainDb` + `FileWalStore`,
//!   2. classify first-run (empty store) vs warm-start (non-empty) as a
//!      PURE function of on-disk state (`classify_start`), and
//!   3. route both arms toward the single `bootstrap_initial_state`
//!      authority (CN-NODE-01) — but L1 leaves both arms as typed
//!      FAIL-CLOSED stubs, because honestly obtaining initial state
//!      needs the Mithril first-run composition (L2) or the recovered
//!      warm-start provenance (L3), neither of which exists yet.
//!
//! What L1 deliberately does NOT do (each is a later slice):
//!   - L2: Mithril-only first-run bootstrap (`bootstrap_from_mithril_snapshot`
//!     + `verify_mithril_binding`), persisting the seed-epoch sidecar.
//!   - L3: production warm-start recovery (`replay_from_anchor` →
//!     `bootstrap_initial_state(RequiredFromRecoveredProvenance)`).
//!   - L4: peer BlockFetch → durable `pump_block` apply.
//!   - L5: produce from the recovered selected tip + recovered inputs.
//!   - L6: BA-02 peer-acceptance evidence.
//!
//! The owner NEVER cold-starts from genesis / `--consensus-inputs-path`
//! / a tip bundle / `InMemoryChainDb`. There is no fallback: an
//! unwired arm fails closed (CN-NODE-01 / the cluster's Mithril-only,
//! fail-closed rule). `produce_mode` and `admission` remain diagnostic
//! modes and are unchanged.

use std::process::ExitCode;

use ade_runtime::chaindb::{
    ChainDb, PersistentChainDb, PersistentChainDbOptions, SnapshotStore,
};
use ade_runtime::wal::FileWalStore;
use tokio::sync::watch;

use crate::cli::Cli;
use crate::EXIT_GENERIC_STARTUP;

/// Clean-exit code (mirrors the local constant in `wire_only`; the
/// crate root does not re-export a single `EXIT_OK`).
const EXIT_OK: i32 = 0;

/// Exit code emitted when the node lifecycle owner reaches an arm whose
/// production wiring has not landed yet (L2 first-run / L3 warm-start).
/// Distinct from a generic startup error so an operator can tell a
/// "not-yet-wired, fail-closed" exit from a bad-CLI exit.
pub const EXIT_NODE_LIFECYCLE_UNWIRED: i32 = 40;

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
/// fail-closed halt — none performs a genesis / bundle / cold-start
/// fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeLifecycleError {
    /// A required persistence path (`--snapshot-dir` / `--wal-dir`) was
    /// not supplied.
    MissingPath(&'static str),
    /// Opening the persistent `ChainDb` failed.
    ChainDbOpen(String),
    /// Opening the `FileWalStore` failed.
    WalOpen(String),
    /// Reading on-disk state (tip / snapshot slots) failed.
    OnDiskRead(String),
    /// First-run arm reached: the Mithril-only first-run bootstrap (L2)
    /// is not wired yet. Fail closed — NO genesis / bundle / cold
    /// fallback is permitted.
    FirstRunRequiresMithril,
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

/// The `--mode node` owner entry. L1: open the stores, classify, and
/// fail closed on both arms pending L2/L3. Returns a process exit code.
///
/// `shutdown` is accepted for signature parity with the other mode
/// entries and the L4 slot loop to come; L1 does not run a loop.
pub async fn run_node_lifecycle(cli: Cli, _shutdown: watch::Receiver<bool>) -> ExitCode {
    match run_node_lifecycle_inner(&cli) {
        Ok(()) => ExitCode::from(EXIT_OK as u8),
        Err(e) => {
            report(&e);
            let code = match e {
                NodeLifecycleError::MissingPath(_) => EXIT_GENERIC_STARTUP,
                NodeLifecycleError::ChainDbOpen(_)
                | NodeLifecycleError::WalOpen(_)
                | NodeLifecycleError::OnDiskRead(_) => EXIT_GENERIC_STARTUP,
                NodeLifecycleError::FirstRunRequiresMithril
                | NodeLifecycleError::WarmStartRecoveryNotWired => EXIT_NODE_LIFECYCLE_UNWIRED,
            };
            ExitCode::from(code as u8)
        }
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
        .ok_or(NodeLifecycleError::MissingPath("--snapshot-dir"))?;
    let wal_dir = cli
        .wal_dir
        .as_ref()
        .ok_or(NodeLifecycleError::MissingPath("--wal-dir"))?;

    // 2. Ensure the persistence directories exist (mirrors
    //    admission/bootstrap.rs). On a true first run the dirs are
    //    absent; creating them is what lets the first-run arm be
    //    REACHED (and then fail closed) rather than erroring at store
    //    open. Creating an empty dir persists no chain facts.
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
    let _wal = FileWalStore::open(wal_dir)
        .map_err(|e| NodeLifecycleError::WalOpen(format!("{e:?}")))?;

    // 4. Classify first-run vs warm-start as a pure function of on-disk
    //    state. (The same `(tip, snapshots)` axes `bootstrap_initial_state`
    //    branches on.)
    let tip = chaindb
        .tip()
        .map_err(|e| NodeLifecycleError::OnDiskRead(format!("{e:?}")))?;
    let snapshot_slots = chaindb
        .list_snapshot_slots()
        .map_err(|e| NodeLifecycleError::OnDiskRead(format!("{e:?}")))?;
    let start = classify_start(tip.is_some(), !snapshot_slots.is_empty());

    // 5. Both arms fail closed in L1. NO cold-start, NO genesis /
    //    --consensus-inputs-path / tip-bundle fallback. The single
    //    `bootstrap_initial_state` authority will be invoked here once
    //    L2 (Mithril first-run) / L3 (warm-start recovery) supply its
    //    inputs.
    match start {
        NodeStart::FirstRun => Err(NodeLifecycleError::FirstRunRequiresMithril),
        NodeStart::WarmStart => Err(NodeLifecycleError::WarmStartRecoveryNotWired),
    }
}

fn report(e: &NodeLifecycleError) {
    match e {
        NodeLifecycleError::MissingPath(flag) => {
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
        NodeLifecycleError::FirstRunRequiresMithril => {
            eprintln!(
                "ade_node --mode node: first run detected (empty store). The Mithril-only \
                 first-run bootstrap (PHASE4-N-F-C L2) is not wired yet; failing closed. \
                 No genesis / --consensus-inputs-path / cold-start fallback is permitted."
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
mod tests {
    use super::*;

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
        // Pure function of its inputs: same inputs => same output, no
        // wall-clock / env dependence.
        for &has_tip in &[false, true] {
            for &has_snap in &[false, true] {
                assert_eq!(
                    classify_start(has_tip, has_snap),
                    classify_start(has_tip, has_snap),
                );
            }
        }
    }
}
