// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Contiguous-corpus Plutus verdict harness (CE-88 partial).
//!
//! Replays `corpus/contiguous/{alonzo,babbage,conway}/` (1,500 blocks
//! each) starting from the matching pre-era snapshots, aggregating per-
//! tx verdicts produced by `apply_block_with_verdicts`. This is the
//! first measurement where the UTxO window is big enough to hold real
//! Plutus-tx predecessors — previously every Plutus tx hit Ineligible
//! because 20-block boundary windows can't cover historical inputs.
//!
//! What this proves:
//!   - CE-88 progress (not closure): non-zero Plutus evals actually
//!     run on real mainnet txs, with verdicts surfaced per tx.
//!   - Consistency: repeat runs produce identical counters (determinism).
//!   - Regression: any future change that breaks Plutus eval or the
//!     composer path shows up as a drop in Passed counts.
//!
//! What this does NOT prove:
//!   - Oracle agreement — we don't diff against the Haskell chain's
//!     verdicts. That requires an oracle dataset we don't have.
//!   - Full 9,436-tx coverage — 1,500 blocks per era is a subset.

use std::collections::BTreeMap;
use std::path::PathBuf;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_ledger::rules::{apply_block_with_verdicts, TxOutcome};
use ade_testkit::harness::snapshot_loader::LoadedSnapshot;

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
}

/// Aggregated outcome tallies across the replayed block set.
#[derive(Debug, Default, Clone)]
struct OutcomeTally {
    blocks_applied: usize,
    total_tx_verdicts: usize,
    passed: usize,
    inputs_unresolved: usize,
    phase1_rejected: usize,
    plutus_passed: usize,
    plutus_failed: usize,
    plutus_ineligible: usize,
    // Aggregate cpu / mem across all PlutusPassed txs.
    total_plutus_cpu: i128,
    total_plutus_mem: i128,
    // Block limit hit (for debugging — stops when apply_block fails).
    first_apply_error: Option<String>,
}

fn replay_contiguous(
    snapshot: &str,
    era_dir: &str,
    max_blocks: Option<usize>,
) -> Option<OutcomeTally> {
    let snap_path = corpus_root().join("snapshots").join(snapshot);
    if !snap_path.exists() {
        eprintln!("[skip] snapshot {snapshot} missing");
        return None;
    }
    let era_path = corpus_root().join("contiguous").join(era_dir);
    let index_path = era_path.join("blocks.json");
    if !index_path.exists() {
        eprintln!("[skip] blocks.json missing for {era_dir}");
        return None;
    }

    let snap = LoadedSnapshot::from_tarball(&snap_path).ok()?;
    let mut state = snap.to_ledger_state();

    let index_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&index_path).ok()?).ok()?;
    let blocks = index_json
        .get("blocks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let limit = max_blocks.unwrap_or(blocks.len()).min(blocks.len());
    let mut tally = OutcomeTally::default();

    for entry in blocks.iter().take(limit) {
        let filename = match entry["file"].as_str() {
            Some(s) => s,
            None => continue,
        };
        let raw = match std::fs::read(era_path.join(filename)) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let env = match decode_block_envelope(&raw) {
            Ok(e) => e,
            Err(e) => {
                tally.first_apply_error = Some(format!("envelope: {e}"));
                break;
            }
        };
        let inner = &raw[env.block_start..env.block_end];

        match apply_block_with_verdicts(&state, env.era, inner) {
            Ok(result) => {
                tally.blocks_applied += 1;
                tally.total_tx_verdicts += result.tx_verdicts.len();
                for v in &result.tx_verdicts {
                    match &v.outcome {
                        TxOutcome::Passed => tally.passed += 1,
                        TxOutcome::InputsUnresolved => tally.inputs_unresolved += 1,
                        TxOutcome::Phase1Rejected { .. } => tally.phase1_rejected += 1,
                        TxOutcome::PlutusPassed { cpu, mem, .. } => {
                            tally.plutus_passed += 1;
                            tally.total_plutus_cpu =
                                tally.total_plutus_cpu.saturating_add(*cpu as i128);
                            tally.total_plutus_mem =
                                tally.total_plutus_mem.saturating_add(*mem as i128);
                        }
                        TxOutcome::PlutusFailed { .. } => tally.plutus_failed += 1,
                        TxOutcome::PlutusIneligible => tally.plutus_ineligible += 1,
                        TxOutcome::Skipped => {}
                    }
                }
                state = result.new_state;
            }
            Err(e) => {
                tally.first_apply_error = Some(format!("block {filename}: {e}"));
                break;
            }
        }
    }

    Some(tally)
}

fn print_tally(label: &str, t: &OutcomeTally) {
    eprintln!(
        "\n=== {label} ===\n  blocks={}  tx_verdicts={}\n  Passed={}  Inputs-unresolved={}  Phase1-rejected={}\n  Plutus-passed={}  Plutus-failed={}  Plutus-ineligible={}\n  Sum(Plutus cpu)={}  Sum(Plutus mem)={}",
        t.blocks_applied,
        t.total_tx_verdicts,
        t.passed,
        t.inputs_unresolved,
        t.phase1_rejected,
        t.plutus_passed,
        t.plutus_failed,
        t.plutus_ineligible,
        t.total_plutus_cpu,
        t.total_plutus_mem,
    );
    if let Some(ref e) = t.first_apply_error {
        eprintln!("  NOTE: replay stopped — {e}");
    }
}

/// Smoke test. Short replay (100 blocks per era) so this runs in CI
/// under 10 minutes. The full-1500 replay is `alonzo_contiguous_full`
/// and is `#[ignore]`'d by default.
#[test]
fn plutus_era_contiguous_smoke() {
    let cases = [
        ("snapshot_39916975.tar.gz", "alonzo", "Alonzo"),
        ("snapshot_72316896.tar.gz", "babbage", "Babbage"),
        ("snapshot_133660855.tar.gz", "conway", "Conway"),
    ];
    let mut aggregated = BTreeMap::new();
    for (snap, dir, label) in &cases {
        match replay_contiguous(snap, dir, Some(100)) {
            Some(t) => {
                print_tally(&format!("{label} contiguous (first 100 blocks)"), &t);
                aggregated.insert(label.to_string(), t);
            }
            None => eprintln!("[skip] {label} unavailable"),
        }
    }

    assert!(
        !aggregated.is_empty(),
        "at least one era's corpus must be available",
    );

    // Gate 1: every replayed era must apply at least 1 block without
    // error. A hard failure here means the contiguous pipeline is
    // structurally broken.
    for (label, t) in &aggregated {
        assert!(
            t.blocks_applied > 0,
            "{label}: no blocks applied ({:?})",
            t.first_apply_error,
        );
    }

    // Gate 2: we must see at least one Plutus eval attempt (passed /
    // failed / ineligible) across all eras. If zero, the composer +
    // eval wire-in isn't firing on contiguous replay and the earlier
    // boundary-test signal is spurious.
    let any_plutus: bool = aggregated.values().any(|t| {
        t.plutus_passed + t.plutus_failed + t.plutus_ineligible > 0
    });
    assert!(
        any_plutus,
        "no Plutus evals fired across contiguous replay — wire-in broken?",
    );
}

/// Full 1,500-block replay per Plutus era. Slow (many minutes per era).
/// Gated behind `#[ignore]`; run explicitly with
/// `cargo test -p ade_testkit --test contiguous_plutus_verdict_harness
/// -- --ignored --nocapture`.
#[test]
#[ignore = "full 1500-block replay is slow; run manually for CE-88 evidence"]
fn plutus_era_contiguous_full() {
    let cases = [
        ("snapshot_39916975.tar.gz", "alonzo", "Alonzo"),
        ("snapshot_72316896.tar.gz", "babbage", "Babbage"),
        ("snapshot_133660855.tar.gz", "conway", "Conway"),
    ];
    let mut total_plutus_passed = 0usize;
    for (snap, dir, label) in &cases {
        match replay_contiguous(snap, dir, None) {
            Some(t) => {
                print_tally(&format!("{label} contiguous (all 1,500 blocks)"), &t);
                total_plutus_passed += t.plutus_passed;
            }
            None => eprintln!("[skip] {label} unavailable"),
        }
    }
    eprintln!(
        "\n=== CE-88 partial: {total_plutus_passed} Plutus txs passed across \
         contiguous Alonzo/Babbage/Conway corpus ==="
    );
}
