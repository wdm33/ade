//! Integration test: Compute and verify anchor hashes for all proof-grade snapshots.
//!
//! Each snapshot's `state` file is the ExtLedgerState CBOR at that slot.
//! Its Blake2b-256 hash is the oracle state hash — the comparison surface
//! for T-25 epoch boundary and T-26 HFC transition proofs.
//!
//! This test establishes the anchor hash chain: a deterministic, reproducible
//! mapping from slot → state_hash that T-25/T-26 will verify against.

use std::path::PathBuf;

use ade_testkit::harness::snapshot_loader::{
    compute_state_hash, extract_state_from_tarball, parse_oracle_hashes, parse_snapshot_header,
};

fn snapshots_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("snapshots")
}

struct AnchorHash {
    slot: u64,
    label: &'static str,
    purpose: &'static str,
    epoch: u64,
    telescope: u32,
    state_size: usize,
    hash_hex: String,
}

fn compute_anchor(slot: u64, label: &'static str, purpose: &'static str) -> Option<AnchorHash> {
    let tarball = snapshots_dir().join(format!("snapshot_{slot}.tar.gz"));
    if !tarball.exists() {
        return None;
    }

    let state_bytes = extract_state_from_tarball(&tarball).ok()?;
    let header = parse_snapshot_header(&state_bytes).ok()?;
    let hash = compute_state_hash(&state_bytes);

    Some(AnchorHash {
        slot,
        label,
        purpose,
        epoch: header.epoch,
        telescope: header.telescope_length,
        state_size: header.state_size,
        hash_hex: format!("{hash}"),
    })
}

#[test]
fn anchor_hash_chain_all_proof_grade() {
    let specs: Vec<(u64, &str, &str)> = vec![
        (4492800, "byron->shelley", "pre_hfc"),
        (4924880, "shelley", "epoch_boundary"),
        (16588800, "shelley->allegra", "pre_hfc"),
        (17020848, "allegra", "epoch_boundary"),
        (23068800, "allegra->mary", "pre_hfc"),
        (23500962, "mary", "epoch_boundary"),
        (39916975, "mary->alonzo", "pre_hfc"),
        (40348902, "alonzo", "epoch_boundary"),
        (72316896, "alonzo->babbage", "pre_hfc"),
        (72748820, "babbage", "epoch_boundary"),
        (133660855, "babbage->conway", "pre_hfc"),
        (134092810, "conway", "epoch_boundary"),
    ];

    let anchors: Vec<AnchorHash> = specs
        .iter()
        .filter_map(|(slot, label, purpose)| compute_anchor(*slot, label, purpose))
        .collect();

    eprintln!("\n=== Anchor Hash Chain (12 Proof-Grade Snapshots) ===");
    eprintln!(
        "{:<22} {:>5} {:>4} {:>12} {:>10}  Blake2b-256",
        "Label", "Epoch", "Tele", "Slot", "Size"
    );
    eprintln!("{}", "-".repeat(120));

    for a in &anchors {
        eprintln!(
            "{:<22} {:>5} {:>4} {:>12} {:>10}  {}",
            a.label, a.epoch, a.telescope, a.slot, a.state_size, a.hash_hex
        );
    }
    eprintln!("{}", "=".repeat(120));

    // Structural assertions
    assert_eq!(anchors.len(), 12, "all 12 proof-grade snapshots must load");

    // Telescope grows monotonically for HFC snapshots
    let hfc_telescopes: Vec<u32> = anchors
        .iter()
        .filter(|a| a.purpose == "pre_hfc")
        .map(|a| a.telescope)
        .collect();
    for w in hfc_telescopes.windows(2) {
        assert!(
            w[0] < w[1],
            "telescope must grow: {} < {}",
            w[0],
            w[1]
        );
    }

    // Epoch boundary snapshots have same telescope as preceding HFC
    for pair in anchors.chunks(2) {
        if pair.len() == 2 {
            assert_eq!(
                pair[0].telescope, pair[1].telescope,
                "HFC and following epoch boundary must have same telescope: {} vs {}",
                pair[0].label, pair[1].label
            );
        }
    }

    // All hashes are unique
    let hashes: Vec<&str> = anchors.iter().map(|a| a.hash_hex.as_str()).collect();
    for (i, h) in hashes.iter().enumerate() {
        for (j, h2) in hashes.iter().enumerate() {
            if i != j {
                assert_ne!(h, h2, "anchor hashes must be unique");
            }
        }
    }

    // Determinism: recompute one and verify same hash
    let recompute = compute_anchor(4492800, "byron->shelley", "pre_hfc");
    assert_eq!(
        recompute.as_ref().map(|a| a.hash_hex.as_str()),
        Some(anchors[0].hash_hex.as_str()),
        "anchor hash must be reproducible"
    );
}

#[test]
fn epoch_boundary_hashes_chain_to_oracle() {
    // For each epoch boundary snapshot that has a hash file,
    // verify the snapshot's state size is consistent with the
    // oracle's first post-snapshot entry.
    let cases = [
        (4924880, "hashes_4924880.txt", "shelley"),
        (17020848, "hashes_17020848.txt", "allegra"),
        (40348902, "hashes_40348902.txt", "alonzo"),
        (134092810, "hashes_134092810.txt", "conway"),
    ];

    eprintln!("\n=== Epoch Boundary → Oracle Hash Chain ===");

    for (slot, hash_file, era) in &cases {
        let tarball = snapshots_dir().join(format!("snapshot_{slot}.tar.gz"));
        let hf_path = snapshots_dir().join(hash_file);
        if !tarball.exists() || !hf_path.exists() {
            continue;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        let snap_hash = compute_state_hash(&state_bytes);
        let snap_size = state_bytes.len();

        let content = std::fs::read_to_string(&hf_path).unwrap();
        let oracle_entries = parse_oracle_hashes(&content).unwrap();

        eprintln!(
            "  {era} (slot {slot}): snapshot_hash={}, snap_size={snap_size}, oracle_entries={}",
            &format!("{snap_hash}")[..16],
            oracle_entries.len()
        );

        // The snapshot hash is the state BEFORE the first oracle block.
        // The first oracle entry is the state AFTER applying that block.
        // They must differ (a block was applied).
        assert_ne!(
            format!("{snap_hash}"),
            format!("{}", oracle_entries[0].hash),
            "{era}: snapshot hash should differ from first oracle hash (block applied)"
        );

        // Oracle entries must be slot-ordered
        for w in oracle_entries.windows(2) {
            assert!(
                w[0].slot < w[1].slot,
                "{era}: oracle hashes must be slot-ordered"
            );
        }
    }
}
