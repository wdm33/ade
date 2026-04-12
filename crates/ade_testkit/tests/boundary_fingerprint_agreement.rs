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
        "7d86f964e77db3e098c7d11be3c933c2f0a3ce5a30bd4967e8d518d0f0f84843",
    ),
    (
        "allegra_pre_hfc",
        "snapshot_23068800.tar.gz",
        "be36eaf718add815947894e8573060cee2b20a14dede102b8aba1573cf4bc06f",
    ),
    (
        "mary_pre_hfc",
        "snapshot_39916975.tar.gz",
        "af5b4c546476b9dd9a25081372cdf5c3445f6b57982eec2da130b106545e715b",
    ),
    (
        "alonzo_pre_hfc",
        "snapshot_72316896.tar.gz",
        "54f8258a593dc7fb4a4245ab21bdb5d31c0c7187e6a16174cd19e60cd04c7ef2",
    ),
    (
        "babbage_pre_hfc",
        "snapshot_133660855.tar.gz",
        "e59152ed1460ccd7fe65400401fb98f66054717ebb3ba7bf92dd625d9714d22d",
    ),
    (
        "shelley_epoch_209",
        "snapshot_4924880.tar.gz",
        "cfb0358eafcea3ace170ce7d9f12ca740315de91f7867daa11999ef5a594528e",
    ),
    (
        "allegra_epoch_237",
        "snapshot_17020848.tar.gz",
        "098c36a298caf6b56be9893f75d9b2b9a9c5af8fe858fecef7debcfc8b4dd790",
    ),
    (
        "mary_epoch_252",
        "snapshot_23500962.tar.gz",
        "d4c26baa9dd65928d065350e8dbd8dd59e08c3c577ce0ca989d019b155428f8c",
    ),
    (
        "alonzo_epoch_291",
        "snapshot_40348902.tar.gz",
        "94a2f45b7131ec7cc3a4d257f05479b0194eb3eafd84cf9ea2f9c0520737d348",
    ),
    (
        "babbage_epoch_366",
        "snapshot_72748820.tar.gz",
        "61e7ce7f3a30bc10e467c447ff7df7d2cf3a32212fa4e8bcb5b0f4eb97f9f0c7",
    ),
    (
        "conway_epoch_508",
        "snapshot_134092810.tar.gz",
        "655e93a50777aa6e529aaabe36990ac31d8c7a711e71ca2b2b34911f671bb885",
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
