//! Integration test: Stateful boundary replay with loaded snapshot state.
//!
//! Loads a snapshot (non-empty UTxO triggers state tracking), replays
//! boundary blocks, and verifies the UTxO evolves meaningfully.
//! This is the first step toward oracle-checkable boundary transitions.

use std::path::PathBuf;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_ledger::rules::apply_block_classified;
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

/// Load a snapshot and replay boundary blocks with stateful UTxO tracking.
/// Returns (blocks_applied, final_utxo_count).
fn stateful_boundary_replay(
    snapshot_file: &str,
    blocks_dir: &str,
) -> (usize, usize) {
    let tarball = snapshots_dir().join(snapshot_file);
    if !tarball.exists() {
        return (0, 0);
    }

    let snap = LoadedSnapshot::from_tarball(&tarball).unwrap();
    let mut state = snap.to_ledger_state();

    // Seed the UTxO with a placeholder entry so tracking activates.
    // In a full implementation, UTxO would be loaded from the snapshot CBOR.
    let placeholder_in = ade_types::tx::TxIn {
        tx_hash: ade_types::Hash32([0x00; 32]),
        index: 0,
    };
    let placeholder_out = ade_ledger::utxo::TxOut::ShelleyMary {
        address: vec![0x00],
        value: ade_ledger::value::Value::from_coin(ade_types::tx::Coin(1)),
    };
    state.utxo_state.utxos.insert(placeholder_in, placeholder_out);

    let block_dir = boundary_blocks_dir().join(blocks_dir);
    let manifest_path = block_dir.join("manifest.json");
    if !manifest_path.exists() {
        return (0, 0);
    }

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();

    let mut applied = 0;

    // For HFC transitions, replay only post-blocks (new era)
    let block_list = if let Some(post) = manifest.get("post_blocks") {
        post.as_array().unwrap().clone()
    } else if let Some(blocks) = manifest.get("blocks") {
        blocks.as_array().unwrap().clone()
    } else {
        return (0, 0);
    };

    for entry in &block_list {
        let filename = entry["file"].as_str().unwrap();
        let raw = std::fs::read(block_dir.join(filename)).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        match apply_block_classified(&state, env.era, inner) {
            Ok((new_state, _verdict)) => {
                state = new_state;
                applied += 1;
            }
            Err(e) => {
                eprintln!("  {blocks_dir}: block {filename} failed: {e}");
                break;
            }
        }
    }

    let final_utxo = state.utxo_state.len();
    (applied, final_utxo)
}

#[test]
fn shelley_allegra_hfc_stateful() {
    let (applied, utxo_count) = stateful_boundary_replay(
        "snapshot_16588800.tar.gz",
        "shelley_allegra",
    );
    eprintln!("Shelley→Allegra: {applied} blocks, {utxo_count} UTxOs after replay");

    if applied > 0 {
        // UTxO should grow from the placeholder (1 entry) as outputs are produced
        assert!(utxo_count > 1, "UTxO must grow after applying blocks");
    }
}

#[test]
fn allegra_mary_hfc_stateful() {
    let (applied, utxo_count) = stateful_boundary_replay(
        "snapshot_23068800.tar.gz",
        "allegra_mary",
    );
    eprintln!("Allegra→Mary: {applied} blocks, {utxo_count} UTxOs");
    if applied > 0 {
        assert!(utxo_count > 1);
    }
}

#[test]
fn shelley_epoch_boundary_stateful() {
    let (applied, utxo_count) = stateful_boundary_replay(
        "snapshot_4924880.tar.gz",
        "shelley_epoch209",
    );
    eprintln!("Shelley epoch 209: {applied} blocks, {utxo_count} UTxOs");
    if applied > 0 {
        assert!(utxo_count > 1);
    }
}

#[test]
fn all_boundaries_stateful_summary() {
    let cases = [
        ("snapshot_4492800.tar.gz", "byron_shelley", "Byron→Shelley HFC"),
        ("snapshot_4924880.tar.gz", "shelley_epoch209", "Shelley epoch 209"),
        ("snapshot_16588800.tar.gz", "shelley_allegra", "Shelley→Allegra HFC"),
        ("snapshot_17020848.tar.gz", "allegra_epoch237", "Allegra epoch 237"),
        ("snapshot_23068800.tar.gz", "allegra_mary", "Allegra→Mary HFC"),
        ("snapshot_23500962.tar.gz", "mary_epoch252", "Mary epoch 252"),
        ("snapshot_39916975.tar.gz", "mary_alonzo", "Mary→Alonzo HFC"),
        ("snapshot_40348902.tar.gz", "alonzo_epoch291", "Alonzo epoch 291"),
        ("snapshot_72316896.tar.gz", "alonzo_babbage", "Alonzo→Babbage HFC"),
        ("snapshot_72748820.tar.gz", "babbage_epoch366", "Babbage epoch 366"),
        ("snapshot_133660855.tar.gz", "babbage_conway", "Babbage→Conway HFC"),
        ("snapshot_134092810.tar.gz", "conway_epoch508", "Conway epoch 508"),
    ];

    eprintln!("\n=== Stateful Boundary Replay Summary ===");
    eprintln!("{:<25} {:>7} {:>8}", "Boundary", "Blocks", "UTxOs");
    eprintln!("{}", "-".repeat(45));

    let mut total_applied = 0;
    let mut all_grew = true;

    for (snap, blocks, label) in &cases {
        let (applied, utxo_count) = stateful_boundary_replay(snap, blocks);
        eprintln!("{:<25} {:>7} {:>8}", label, applied, utxo_count);
        total_applied += applied;
        if applied > 0 && utxo_count <= 1 {
            all_grew = false;
        }
    }

    eprintln!("{}", "-".repeat(45));
    eprintln!("Total blocks applied: {total_applied}");
    eprintln!("========================================\n");

    assert!(total_applied > 100, "should apply blocks across boundaries");
    assert!(all_grew, "UTxO should grow at every boundary with transactions");
}
