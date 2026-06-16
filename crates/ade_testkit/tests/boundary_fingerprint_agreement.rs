//! Boundary fingerprint agreement (CE-73 / CE-75 in-repo).
//!
//! For each of the 12 proof-grade boundary snapshots, load the oracle
//! ExtLedgerState into Ade's LedgerState via the snapshot loader and
//! fingerprint the result. The resulting hashes are pinned — any change
//! in the snapshot loader (the state bridge) or in the fingerprint
//! encoding will cause a regression here.
//!
//! This is the in-repo surrogate for ShadowBox-based per-block
//! differential agreement: it verifies that Ade's canonical view of
//! the oracle state at each boundary is stable and reproducible.
//!
//! Snapshots come from corpus/snapshots/, registered in registry.toml.
//! Loading uses `LoadedSnapshot::to_ledger_state`, which is documented
//! as partial (UTxO is not reconstructed from the compact on-disk
//! format) — so the UTxO component fingerprint reflects the empty UTxO
//! rather than the full oracle UTxO. All other components carry the
//! full loaded state.

use std::path::PathBuf;

use ade_ledger::fingerprint::fingerprint_v1;
use ade_testkit::harness::snapshot_loader::LoadedSnapshot;

fn snapshots_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("snapshots")
}

/// Load a snapshot and return its component fingerprints as hex strings.
/// Returns None if the snapshot file is missing (graceful skip).
fn load_and_fingerprint(tarball: &str) -> Option<BoundaryHashes> {
    let path = snapshots_dir().join(tarball);
    if !path.exists() {
        eprintln!("  {tarball}: SKIP (not present)");
        return None;
    }
    let snap = LoadedSnapshot::from_tarball(&path)
        .unwrap_or_else(|e| panic!("load {tarball}: {e}"));
    let state = snap.to_ledger_state();
    // MEM-OPT-UTXO-DISK S1.5b cutover: these pins are the FROZEN v1 fingerprint of
    // the boundary snapshots (historical regression guard). Production is v2
    // (`fingerprint`); the v2 fingerprint of boundary data is verified via the
    // shared 6 components + the ECMH golden vectors, so v1 pins stay load-bearing.
    let fp = fingerprint_v1(&state);
    Some(BoundaryHashes {
        era: format!("{}", fp.era),
        utxo: format!("{}", fp.utxo),
        cert: format!("{}", fp.cert),
        epoch: format!("{}", fp.epoch),
        snapshots: format!("{}", fp.snapshots),
        pparams: format!("{}", fp.pparams),
        governance: format!("{}", fp.governance),
        combined: format!("{}", fp.combined),
    })
}

struct BoundaryHashes {
    era: String,
    utxo: String,
    cert: String,
    epoch: String,
    snapshots: String,
    pparams: String,
    governance: String,
    combined: String,
}

/// The 12 proof-grade boundary snapshots with pinned `combined` fingerprint hex.
///
/// Hashes are captured at `FINGERPRINT_VERSION = 1` against the
/// cardano-node 10.6.2 snapshots registered in `corpus/snapshots/registry.toml`
/// (Mithril epoch 618, immutable 8419). Any drift is either:
///   - a snapshot_loader change (state bridge semantics moved)
///   - a fingerprint encoding change (requires a `FINGERPRINT_VERSION` bump)
///   - a fresh snapshot set with different state
/// In all three cases the fix is deliberate: investigate cause, update pins.
///
/// Pins regenerated 2026-05-26 after `168ac02` — the snapshot loader
/// now propagates `state.epoch_state.slot` from the snapshot header
/// instead of leaving it at 0. The `epoch` component encodes `slot`, so
/// every snapshot's combined hash shifted (including `byron_pre_hfc`,
/// whose header carries slot 4492800 — the immutable-boundary slot of
/// the byron→shelley HFC capture). Source for this regen run: corpus
/// described in `corpus/snapshots/registry.toml`.
///
/// Note: the `utxo` component is the same across all snapshots because
/// `LoadedSnapshot::to_ledger_state` does not reconstruct the UTxO from
/// the compact on-disk format (documented limitation of the bridge).
/// Component-level divergence is still available via `print_boundary_fingerprints`.
const SNAPSHOTS: &[(&str, &str, &str)] = &[
    // label, tarball, expected combined hex
    (
        "byron_pre_hfc",
        "snapshot_4492800.tar.gz",
        "dc6b3f749fd93aae50b2bb5168b2cff5de1327c664eee270ef6a7b3316f28fcd",
    ),
    (
        "shelley_pre_hfc",
        "snapshot_16588800.tar.gz",
        "6be624078c9e68693b378d1d431b1e13ef8e4ead1c9d1008f1e6d18b158f9509",
    ),
    (
        "allegra_pre_hfc",
        "snapshot_23068800.tar.gz",
        "24244dbc8567cabc7bd60cdb0bc519db23446654fae44216b1cb7cd6c9e952d9",
    ),
    (
        "mary_pre_hfc",
        "snapshot_39916975.tar.gz",
        "0a4a65ad1d92996cc4cb446b75a2c1c098a45aebbf197a8f17e5aaf3dbe0bfcc",
    ),
    (
        "alonzo_pre_hfc",
        "snapshot_72316896.tar.gz",
        "207c688a0137873ee93b71868c0fc1b6a49e9574089d8e627e643c3b9b125127",
    ),
    (
        "babbage_pre_hfc",
        "snapshot_133660855.tar.gz",
        "3a761e47a8963284b74383db934c31e3334d7fe34ef9d359b4138d763fac9e15",
    ),
    (
        "shelley_epoch_209",
        "snapshot_4924880.tar.gz",
        "663844b3b4848721ac72db4dfc07fb40a85408c2572066a1c9bd3aa9e8f8c8bf",
    ),
    (
        "allegra_epoch_237",
        "snapshot_17020848.tar.gz",
        "6b5a101946bbf566a4d43025ef8fd446b0acbe89da54416c102f9800e8b5db0c",
    ),
    (
        "mary_epoch_252",
        "snapshot_23500962.tar.gz",
        "505d3be518c191e1d93387d8d35cef08b897a648ed18e3cb5fa5064678e669ab",
    ),
    (
        "alonzo_epoch_291",
        "snapshot_40348902.tar.gz",
        "031b367d6dcf2fcd52950d4f1cae7198b2147541723c8abdd0c777ef2b598cdf",
    ),
    (
        "babbage_epoch_366",
        "snapshot_72748820.tar.gz",
        "ac675fbbb0a09abb20ddacc34e8be42f22c4370689e425d8c43ff13d0a320280",
    ),
    (
        "conway_epoch_508",
        "snapshot_134092810.tar.gz",
        "e49c176c3f5a6e9b79f48b9c3b190de56dc1948a369a7ae5637f89c200266dfb",
    ),
];

/// Asserts each boundary snapshot loads into a state whose fingerprint
/// matches the pinned `combined` hash. Single load pass per snapshot —
/// determinism is implicit (pinned hashes match on every run).
///
/// Skips gracefully if corpus snapshots are missing. Requires at least
/// one snapshot present to pass.
#[test]
fn boundary_fingerprint_matches_pins() {
    let mut verified = 0usize;
    let mut skipped = 0usize;
    for (label, tarball, expected) in SNAPSHOTS {
        let Some(h) = load_and_fingerprint(tarball) else {
            skipped += 1;
            continue;
        };
        assert_eq!(
            h.combined, *expected,
            "{label} ({tarball}) fingerprint drift:\n  expected: {expected}\n  actual:   {}",
            h.combined
        );
        verified += 1;
    }
    eprintln!(
        "  {verified}/{} boundary fingerprints matched pins ({} skipped)",
        SNAPSHOTS.len(),
        skipped
    );
    assert!(
        verified > 0,
        "no boundary snapshots available — corpus/snapshots/ missing?"
    );
}

/// Prints per-snapshot per-component fingerprints. Use when regenerating
/// pins after a deliberate schema bump:
///
///   cargo test --package ade_testkit --test boundary_fingerprint_agreement \
///       print_boundary_fingerprints -- --ignored --nocapture
#[test]
#[ignore]
fn print_boundary_fingerprints() {
    eprintln!("\n=== Boundary fingerprint pins ===");
    for (label, tarball, _) in SNAPSHOTS {
        match load_and_fingerprint(tarball) {
            Some(h) => {
                eprintln!("{label:22} ({tarball})");
                eprintln!("  combined   = {}", h.combined);
                eprintln!("  era        = {}", h.era);
                eprintln!("  utxo       = {}", h.utxo);
                eprintln!("  cert       = {}", h.cert);
                eprintln!("  epoch      = {}", h.epoch);
                eprintln!("  snapshots  = {}", h.snapshots);
                eprintln!("  pparams    = {}", h.pparams);
                eprintln!("  governance = {}", h.governance);
            }
            None => eprintln!("{label:22} — missing"),
        }
    }
    eprintln!("=== end ===\n");
}
