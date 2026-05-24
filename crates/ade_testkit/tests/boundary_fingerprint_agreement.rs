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

use ade_ledger::fingerprint::fingerprint;
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
    let fp = fingerprint(&state);
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
/// Pins regenerated 2026-05-24 against the corpus produced by the
/// reconstructed March recipe (db-truncater + cardano-node v1-in-mem,
/// landing-slot capture). The `byron_pre_hfc` pin is unchanged — its state
/// is the minimal post-HFC Shelley genesis state (43 KB, no UTxO/cert
/// activity yet), deterministic across any capture mechanism that lands at
/// the byron→shelley HFC. The other 11 pins reflect the new captures'
/// landed-slot content.
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
        "2b3562ab1d80b6468e4ffaf09355593f41324b56e97c9200e3d6fe193d822cfa",
    ),
    (
        "shelley_pre_hfc",
        "snapshot_16588800.tar.gz",
        "d12d02bc94557a16055c846fbe995c32b8d822aa1cd27a5323d9ce06cab0423c",
    ),
    (
        "allegra_pre_hfc",
        "snapshot_23068800.tar.gz",
        "75fb8db94c9f00dfd594e568f4b1e44c047cde8b1846f891e37dff75500c2818",
    ),
    (
        "mary_pre_hfc",
        "snapshot_39916975.tar.gz",
        "d5f4eedf08aca3baa1ed65fa48a6946992a9cd4fec953c09e86e677557c1c7b8",
    ),
    (
        "alonzo_pre_hfc",
        "snapshot_72316896.tar.gz",
        "d93040a9f8f46a5b0f668c96e402f11ea1e82fc4dc395c43d9917f27a484035e",
    ),
    (
        "babbage_pre_hfc",
        "snapshot_133660855.tar.gz",
        "fae4e5ca7e01c96b87474d62fcc76cfd261f9526f8af0c54b3862966f7e315ee",
    ),
    (
        "shelley_epoch_209",
        "snapshot_4924880.tar.gz",
        "344afab3bcb226e451e9fb07c60e6ff2d0db735fa8cf0b033670af173416d014",
    ),
    (
        "allegra_epoch_237",
        "snapshot_17020848.tar.gz",
        "1ee30bd659a1c60d660d85e9e0a38416449a6c24a734c6c42dc1cd8e9e226beb",
    ),
    (
        "mary_epoch_252",
        "snapshot_23500962.tar.gz",
        "ac2546496ffc10031a2548d18581180bc8c594466333fcc8e5d4e42a980f6012",
    ),
    (
        "alonzo_epoch_291",
        "snapshot_40348902.tar.gz",
        "d2720aaae4135fcb244a1b7e81e7840ec24071e30763f619b1ebdbe3983e6dbb",
    ),
    (
        "babbage_epoch_366",
        "snapshot_72748820.tar.gz",
        "957e70c947ffb18eea6d6d2ccc4d6bff0829d5ca9ca37c75fee8d94e7131b4ae",
    ),
    (
        "conway_epoch_508",
        "snapshot_134092810.tar.gz",
        "3e710076e6b5974d221aee1f11dd9e730e8e12644cf06f0b37a0ba29c834905a",
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
