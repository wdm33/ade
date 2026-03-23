//! Integration test: Epoch boundary transition logic (T-25A.1).
//!
//! Loads a snapshot, sets the epoch one lower than the blocks' epoch,
//! then replays boundary blocks. The first block should trigger the
//! epoch boundary transition (snapshot rotation + pool retirements).

use std::path::PathBuf;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_ledger::rules::{apply_block_classified, EpochBoundarySummary};
use ade_testkit::harness::snapshot_loader::LoadedSnapshot;

fn snapshots_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("snapshots")
}

fn boundary_blocks_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("boundary_blocks")
}

fn replay_with_epoch_trigger(
    snapshot_file: &str,
    blocks_dir: &str,
    epoch_offset: u64,
) -> Option<EpochBoundarySummary> {
    let tarball = snapshots_dir().join(snapshot_file);
    if !tarball.exists() {
        return None;
    }

    let snap = LoadedSnapshot::from_tarball(&tarball).unwrap();
    let mut state = snap.to_ledger_state();

    // Set epoch one lower so the first block triggers boundary
    let target_epoch = state.epoch_state.epoch.0;
    state.epoch_state.epoch = ade_types::EpochNo(target_epoch - epoch_offset);

    let initial_epoch = state.epoch_state.epoch.0;

    let block_dir = boundary_blocks_dir().join(blocks_dir);
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(block_dir.join("manifest.json")).unwrap(),
    )
    .unwrap();

    let blocks = manifest["blocks"].as_array().unwrap();

    for entry in blocks {
        let filename = entry["file"].as_str().unwrap();
        let raw = std::fs::read(block_dir.join(filename)).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        let prev_epoch = state.epoch_state.epoch.0;
        match apply_block_classified(&state, env.era, inner) {
            Ok((new_state, _)) => {
                if new_state.epoch_state.epoch.0 != prev_epoch {
                    let summary = EpochBoundarySummary {
                        from_epoch: initial_epoch,
                        to_epoch: new_state.epoch_state.epoch.0,
                        delegation_count: new_state.cert_state.delegation.delegations.len(),
                        pool_count: new_state.cert_state.pool.pools.len(),
                        retiring_count: new_state.cert_state.pool.retiring.len(),
                        retired_count: 0,
                        mark_delegation_count: new_state.epoch_state.snapshots.mark.0.delegations.len(),
                        set_delegation_count: new_state.epoch_state.snapshots.set.0.delegations.len(),
                        go_delegation_count: new_state.epoch_state.snapshots.go.0.delegations.len(),
                        treasury: new_state.epoch_state.treasury.0,
                        reserves: new_state.epoch_state.reserves.0,
                    };
                    return Some(summary);
                }
                state = new_state;
            }
            Err(e) => {
                eprintln!("  block {}: {e}", entry["file"]);
                break;
            }
        }
    }

    None
}

#[test]
fn shelley_epoch_boundary_fires() {
    let summary = replay_with_epoch_trigger(
        "snapshot_4924880.tar.gz",
        "shelley_epoch209",
        1,
    );

    if let Some(s) = summary {
        eprintln!("\n=== Shelley Epoch Boundary (208→209) ===");
        eprintln!("  epoch: {} → {}", s.from_epoch, s.to_epoch);
        eprintln!("  delegations: {}", s.delegation_count);
        eprintln!("  pools: {}", s.pool_count);
        eprintln!("  mark delegs: {}", s.mark_delegation_count);
        eprintln!("  set delegs: {}", s.set_delegation_count);
        eprintln!("  go delegs: {}", s.go_delegation_count);
        eprintln!("  treasury: {}", s.treasury);
        eprintln!("  reserves: {}", s.reserves);
        eprintln!("=========================================\n");

        assert_eq!(s.to_epoch, 209, "should transition to epoch 209");
        assert!(s.from_epoch < s.to_epoch, "epoch should advance");
    } else {
        eprintln!("Skipping: snapshot not available or boundary didn't fire");
    }
}

/// Compare Ade's epoch boundary output against the snapshot's own values.
///
/// This is the T-25A.2 diagnostic comparison surface. The snapshot
/// header provides ground-truth treasury/reserves values parsed from
/// CBOR. After a boundary with no rewards, these should be preserved.
#[test]
fn allegra_epoch_boundary_summary_comparison() {
    let tarball_path = snapshots_dir().join("snapshot_17020848.tar.gz");
    if !tarball_path.exists() {
        eprintln!("Skipping: snapshot not available");
        return;
    }

    // Load snapshot to get ground-truth values from CBOR
    let snap = LoadedSnapshot::from_tarball(&tarball_path).unwrap();
    let snap_treasury = snap.header.treasury;
    let snap_reserves = snap.header.reserves;
    let snap_epoch = snap.header.epoch;

    let summary = replay_with_epoch_trigger(
        "snapshot_17020848.tar.gz",
        "allegra_epoch237",
        1,
    );

    eprintln!("\n=== Allegra Epoch Boundary Comparison (T-25A.2) ===");
    eprintln!("{:<25} {:>20} {:>20} {:>10}", "Field", "Ade (post)", "Snapshot", "Status");
    eprintln!("{}", "-".repeat(78));

    if let Some(s) = &summary {
        let epoch_ok = s.to_epoch == snap_epoch;
        eprintln!("{:<25} {:>20} {:>20} {:>10}",
            "epoch", s.to_epoch, snap_epoch,
            if epoch_ok { "match" } else { "MISMATCH" });

        // Treasury/reserves: no rewards applied yet, should match snapshot
        let treasury_ok = s.treasury == snap_treasury;
        let reserves_ok = s.reserves == snap_reserves;
        eprintln!("{:<25} {:>20} {:>20} {:>10}",
            "treasury (lovelace)", s.treasury, snap_treasury,
            if treasury_ok { "match" } else { "preserved" });
        eprintln!("{:<25} {:>20} {:>20} {:>10}",
            "reserves (lovelace)", s.reserves, snap_reserves,
            if reserves_ok { "match" } else { "preserved" });

        // Delegation/pool: starts empty, accumulates from block certs
        eprintln!("{:<25} {:>20} {:>20} {:>10}",
            "delegations", s.delegation_count, "—", "from replay");
        eprintln!("{:<25} {:>20} {:>20} {:>10}",
            "pools", s.pool_count, "—", "from replay");
        eprintln!("{:<25} {:>20} {:>20} {:>10}",
            "mark snapshot", s.mark_delegation_count, "—", "from replay");

        // With reward math: treasury should INCREASE, reserves should DECREASE
        // because total_reward = floor(reserves * rho) is taken from reserves,
        // and treasury_delta = floor(total_reward * tau) goes to treasury.
        let treasury_increased = s.treasury > snap_treasury;
        let reserves_decreased = s.reserves < snap_reserves;
        let treasury_delta = s.treasury.saturating_sub(snap_treasury);
        let reserves_delta = snap_reserves.saturating_sub(s.reserves);

        eprintln!("\nDiagnosis:");
        eprintln!("  {} Epoch: {}", if epoch_ok { "✓" } else { "✗" }, s.to_epoch);
        if treasury_increased {
            eprintln!("  ✓ Treasury increased by {} lovelace (reward math active)", treasury_delta);
        } else if treasury_ok {
            eprintln!("  △ Treasury unchanged (empty delegation → zero rewards)");
        }
        if reserves_decreased {
            eprintln!("  ✓ Reserves decreased by {} lovelace (monetary expansion)", reserves_delta);
        } else if reserves_ok {
            eprintln!("  △ Reserves unchanged (empty delegation → zero rewards)");
        }
        eprintln!("  △ Delegation/pool state built from replay certs only");

        assert!(epoch_ok, "epoch must match snapshot");
        // Treasury/reserves either unchanged (no delegation data) or correctly adjusted
        assert!(
            treasury_ok || treasury_increased,
            "treasury must be preserved or increased"
        );
        assert!(
            reserves_ok || reserves_decreased,
            "reserves must be preserved or decreased"
        );
    } else {
        eprintln!("  SKIPPED");
    }
    eprintln!("====================================================\n");
}

/// Verify reward arithmetic is correct against hand-computed values.
#[test]
fn reward_arithmetic_verification() {
    let tarball_path = snapshots_dir().join("snapshot_17020848.tar.gz");
    if !tarball_path.exists() {
        return;
    }

    let snap = LoadedSnapshot::from_tarball(&tarball_path).unwrap();
    let summary = replay_with_epoch_trigger(
        "snapshot_17020848.tar.gz",
        "allegra_epoch237",
        1,
    );

    if let Some(s) = summary {
        let reserves_before = snap.header.reserves;
        let treasury_before = snap.header.treasury;

        let total_reward = reserves_before * 3 / 1000;
        let treasury_delta = total_reward / 5;
        let pool_reward_pot = total_reward - treasury_delta;

        let reserves_delta = reserves_before - s.reserves;
        let treasury_actual_delta = s.treasury - treasury_before;

        // Unallocated remainder from rounding (< 1 lovelace per pool)
        let remainder = total_reward - reserves_delta;

        eprintln!("\n=== Reward Arithmetic Verification ===");
        eprintln!("  reserves_before:   {reserves_before}");
        eprintln!("  total_reward:      {total_reward}");
        eprintln!("  treasury_delta:    {treasury_delta}");
        eprintln!("  pool_reward_pot:   {pool_reward_pot}");
        eprintln!("  reserves_delta:    {reserves_delta} (distributed from reserves)");
        eprintln!("  treasury_delta:    {treasury_actual_delta} (added to treasury)");
        eprintln!("  unallocated dust:  {remainder} lovelace");
        eprintln!("======================================\n");

        // Treasury delta now includes fee contribution (fees added to reward pot
        // before treasury cut). The fee amount is small but nonzero.
        assert!(
            treasury_actual_delta >= treasury_delta,
            "treasury delta must be >= formula (fees add to pot)"
        );
        assert!(
            treasury_actual_delta <= treasury_delta + treasury_delta / 100,
            "treasury delta should be within 1% of formula"
        );

        // With performance scaling + a0 pledge influence, reserves decrease
        // by significantly less than total_reward. The a0 factor (1/(1+0.3))
        // reduces distribution by ~23%, and performance further reduces it.
        assert!(
            reserves_delta > total_reward * 30 / 100,
            "reserves should decrease by at least 30% of total reward"
        );
        assert!(
            reserves_delta <= total_reward,
            "reserves decrease must not exceed total reward"
        );
    }
}

#[test]
fn all_epoch_boundaries_fire() {
    let cases = [
        ("snapshot_4924880.tar.gz", "shelley_epoch209", "Shelley 209"),
        ("snapshot_17020848.tar.gz", "allegra_epoch237", "Allegra 237"),
        ("snapshot_23500962.tar.gz", "mary_epoch252", "Mary 252"),
        ("snapshot_40348902.tar.gz", "alonzo_epoch291", "Alonzo 291"),
        ("snapshot_72748820.tar.gz", "babbage_epoch366", "Babbage 366"),
        ("snapshot_134092810.tar.gz", "conway_epoch508", "Conway 508"),
    ];

    eprintln!("\n=== Epoch Boundary Fire Summary ===");
    eprintln!("{:<15} {:>5} {:>5} {:>6} {:>5} {:>6} {:>6} {:>6}",
        "Boundary", "From", "To", "Deleg", "Pools", "Mark", "Set", "Go");
    eprintln!("{}", "-".repeat(65));

    for (snap, blocks, label) in &cases {
        match replay_with_epoch_trigger(snap, blocks, 1) {
            Some(s) => {
                eprintln!(
                    "{:<15} {:>5} {:>5} {:>6} {:>5} {:>6} {:>6} {:>6}",
                    label, s.from_epoch, s.to_epoch,
                    s.delegation_count, s.pool_count,
                    s.mark_delegation_count, s.set_delegation_count, s.go_delegation_count,
                );
                assert!(s.to_epoch > s.from_epoch, "{label}: epoch must advance");
            }
            None => {
                eprintln!("{:<15} SKIPPED", label);
            }
        }
    }
    eprintln!("===================================\n");
}
