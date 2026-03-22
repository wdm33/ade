//! Integration test: Structural classification of corpus blocks.
//!
//! Replays all 10,500 contiguous blocks through `apply_block_classified`
//! and reports the script posture classification for each era.
//! Proves that the harness can cleanly separate:
//! - ordinary accepted blocks (non-Plutus)
//! - structurally valid but script-execution-deferred blocks (Plutus present)
//! - structural rejects (should be zero on valid corpus)

use std::path::PathBuf;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_ledger::rules::apply_block_classified;
use ade_ledger::state::LedgerState;
use ade_types::CardanoEra;

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("contiguous")
}

fn load_blocks_json(era: &str) -> serde_json::Value {
    let path = corpus_root().join(era).join("blocks.json");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&content).unwrap()
}

struct EraClassification {
    era: &'static str,
    total_blocks: usize,
    accepted: usize,
    rejected: usize,
    total_txs: u64,
    non_plutus_txs: u64,
    plutus_deferred_txs: u64,
}

fn classify_era(era_name: &'static str, initial_era: CardanoEra) -> EraClassification {
    let blocks_json = load_blocks_json(era_name);
    let blocks = blocks_json["blocks"].as_array().unwrap();
    let era_dir = corpus_root().join(era_name);

    let mut state = LedgerState::new(initial_era);
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut total_txs = 0u64;
    let mut non_plutus_txs = 0u64;
    let mut plutus_deferred_txs = 0u64;

    for block_entry in blocks {
        let filename = block_entry["file"].as_str().unwrap();
        let path = era_dir.join(filename);
        let raw = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let env = decode_block_envelope(&raw)
            .unwrap_or_else(|e| panic!("envelope decode failed for {filename}: {e}"));
        let inner = &raw[env.block_start..env.block_end];

        match apply_block_classified(&state, env.era, inner) {
            Ok((new_state, verdict)) => {
                state = new_state;
                accepted += 1;
                total_txs += verdict.tx_count;
                non_plutus_txs += verdict.non_plutus_count;
                plutus_deferred_txs += verdict.plutus_deferred_count;
            }
            Err(_) => {
                rejected += 1;
                break;
            }
        }
    }

    EraClassification {
        era: era_name,
        total_blocks: blocks.len(),
        accepted,
        rejected,
        total_txs,
        non_plutus_txs,
        plutus_deferred_txs,
    }
}

#[test]
fn structural_classification_all_eras() {
    let eras = vec![
        classify_era("byron", CardanoEra::ByronRegular),
        classify_era("shelley", CardanoEra::Shelley),
        classify_era("allegra", CardanoEra::Allegra),
        classify_era("mary", CardanoEra::Mary),
        classify_era("alonzo", CardanoEra::Alonzo),
        classify_era("babbage", CardanoEra::Babbage),
        classify_era("conway", CardanoEra::Conway),
    ];

    eprintln!("\n=== Structural Classification Report ===");
    eprintln!(
        "{:<10} {:>6} {:>6} {:>6} {:>8} {:>10} {:>10}",
        "Era", "Blocks", "Accept", "Reject", "TotalTx", "NonPlutus", "Deferred"
    );
    eprintln!("{}", "-".repeat(70));

    let mut total_blocks = 0usize;
    let mut total_accepted = 0usize;
    let mut total_rejected = 0usize;
    let mut grand_total_txs = 0u64;
    let mut grand_non_plutus = 0u64;
    let mut grand_deferred = 0u64;

    for c in &eras {
        eprintln!(
            "{:<10} {:>6} {:>6} {:>6} {:>8} {:>10} {:>10}",
            c.era,
            c.total_blocks,
            c.accepted,
            c.rejected,
            c.total_txs,
            c.non_plutus_txs,
            c.plutus_deferred_txs,
        );
        total_blocks += c.total_blocks;
        total_accepted += c.accepted;
        total_rejected += c.rejected;
        grand_total_txs += c.total_txs;
        grand_non_plutus += c.non_plutus_txs;
        grand_deferred += c.plutus_deferred_txs;
    }

    eprintln!("{}", "-".repeat(70));
    eprintln!(
        "{:<10} {:>6} {:>6} {:>6} {:>8} {:>10} {:>10}",
        "TOTAL",
        total_blocks,
        total_accepted,
        total_rejected,
        grand_total_txs,
        grand_non_plutus,
        grand_deferred,
    );
    eprintln!("========================================\n");

    // Assertions
    assert_eq!(total_rejected, 0, "No corpus blocks should be rejected");
    assert_eq!(total_accepted, total_blocks, "All corpus blocks accepted");

    // Pre-Alonzo eras must have zero Plutus-deferred txs
    for c in &eras {
        match c.era {
            "byron" | "shelley" | "allegra" | "mary" => {
                assert_eq!(
                    c.plutus_deferred_txs, 0,
                    "{}: pre-Alonzo era must have zero Plutus-deferred txs",
                    c.era
                );
            }
            _ => {}
        }
    }

    // Alonzo+ eras should have some Plutus-deferred txs (Alonzo introduced Plutus)
    let alonzo = &eras[4];
    assert!(
        alonzo.plutus_deferred_txs > 0,
        "Alonzo should have Plutus-bearing transactions"
    );
}

#[test]
fn alonzo_classification_detail() {
    let c = classify_era("alonzo", CardanoEra::Alonzo);
    assert_eq!(c.accepted, 1500);
    assert_eq!(c.rejected, 0);
    assert!(c.total_txs > 0, "Alonzo blocks should contain transactions");
    eprintln!(
        "Alonzo: {} txs total, {} non-Plutus, {} Plutus-deferred",
        c.total_txs, c.non_plutus_txs, c.plutus_deferred_txs
    );
}

#[test]
fn babbage_classification_detail() {
    let c = classify_era("babbage", CardanoEra::Babbage);
    assert_eq!(c.accepted, 1500);
    assert_eq!(c.rejected, 0);
    assert!(c.total_txs > 0);
    eprintln!(
        "Babbage: {} txs total, {} non-Plutus, {} Plutus-deferred",
        c.total_txs, c.non_plutus_txs, c.plutus_deferred_txs
    );
}

#[test]
fn conway_classification_detail() {
    let c = classify_era("conway", CardanoEra::Conway);
    assert_eq!(c.accepted, 1500);
    assert_eq!(c.rejected, 0);
    assert!(c.total_txs > 0);
    eprintln!(
        "Conway: {} txs total, {} non-Plutus, {} Plutus-deferred",
        c.total_txs, c.non_plutus_txs, c.plutus_deferred_txs
    );
}
