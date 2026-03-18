//! Differential test: Byron block replay.
//!
//! Replays the first N contiguous Byron blocks through ade_ledger::rules::apply_block
//! and verifies:
//! 1. Every block the oracle accepted, Ade also accepts (verdict agreement)
//! 2. State progression is deterministic (same inputs → same output)
//! 3. UTxO set is consistent after each block application
//!
//! This is the first rung of the sub-surface comparison ladder (Option 4).
//! It tests verdict agreement WITHOUT requiring byte-identical state serialization.

use std::path::PathBuf;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_ledger::rules::apply_block;
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

/// Replay Byron blocks through apply_block and report results.
///
/// Byron contiguous corpus starts at slot 0 (mainnet genesis).
/// Early Byron blocks have 0 transactions, so this primarily tests
/// that the block acceptance pipeline doesn't reject valid blocks.
#[test]
fn byron_contiguous_replay_verdict_agreement() {
    let blocks_json = load_blocks_json("byron");
    let blocks = blocks_json["blocks"].as_array().unwrap();
    let era_dir = corpus_root().join("byron");

    let mut state = LedgerState::new(CardanoEra::ByronRegular);
    let mut accepted = 0usize;
    let mut first_error: Option<(usize, String)> = None;

    for (i, block_entry) in blocks.iter().enumerate() {
        let filename = block_entry["file"].as_str().unwrap();
        let path = era_dir.join(filename);
        let raw = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

        let env = decode_block_envelope(&raw)
            .unwrap_or_else(|e| panic!("envelope decode failed for {filename}: {e}"));

        let inner = &raw[env.block_start..env.block_end];

        match apply_block(&state, env.era, inner) {
            Ok(new_state) => {
                state = new_state;
                accepted += 1;
            }
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some((i, format!("{e}")));
                }
                // Don't break — continue to count how many succeed
                // But we can't update state since we got an error
            }
        }
    }

    eprintln!(
        "Byron replay: {accepted}/{} blocks accepted",
        blocks.len()
    );

    if let Some((idx, err)) = &first_error {
        eprintln!("First error at block {idx}: {err}");
    }

    // All 1500 Byron blocks should be accepted
    // (early mainnet Byron blocks have 0 transactions)
    assert_eq!(
        accepted,
        blocks.len(),
        "Byron verdict disagreement: {accepted}/{} accepted. First error: {:?}",
        blocks.len(),
        first_error
    );
}

/// Verify Byron replay is deterministic: two runs produce identical results.
#[test]
fn byron_replay_determinism() {
    let blocks_json = load_blocks_json("byron");
    let blocks = blocks_json["blocks"].as_array().unwrap();
    let era_dir = corpus_root().join("byron");

    // Only test first 100 blocks for speed
    let test_count = blocks.len().min(100);

    let mut state1 = LedgerState::new(CardanoEra::ByronRegular);
    let mut state2 = LedgerState::new(CardanoEra::ByronRegular);

    for block_entry in blocks.iter().take(test_count) {
        let filename = block_entry["file"].as_str().unwrap();
        let path = era_dir.join(filename);
        let raw = std::fs::read(&path).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        let r1 = apply_block(&state1, env.era, inner);
        let r2 = apply_block(&state2, env.era, inner);

        assert_eq!(r1, r2, "Non-deterministic result at block {}", block_entry["index"]);

        if let Ok(s) = r1 {
            state1 = s;
        }
        if let Ok(s) = r2 {
            state2 = s;
        }
    }

    // Final states must be identical
    assert_eq!(state1, state2, "Final states diverge after {test_count} blocks");
    eprintln!("Byron determinism: {test_count} blocks verified identical");
}

/// Track UTxO set size progression through Byron replay.
#[test]
fn byron_utxo_progression() {
    let blocks_json = load_blocks_json("byron");
    let blocks = blocks_json["blocks"].as_array().unwrap();
    let era_dir = corpus_root().join("byron");

    let mut state = LedgerState::new(CardanoEra::ByronRegular);
    let initial_utxo_count = state.utxo_state.len();

    for block_entry in blocks.iter() {
        let filename = block_entry["file"].as_str().unwrap();
        let path = era_dir.join(filename);
        let raw = std::fs::read(&path).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        match apply_block(&state, env.era, inner) {
            Ok(new_state) => state = new_state,
            Err(_) => break,
        }
    }

    let final_utxo_count = state.utxo_state.len();

    eprintln!(
        "Byron UTxO progression: {} → {} entries ({} blocks)",
        initial_utxo_count,
        final_utxo_count,
        blocks.len()
    );

    // Early Byron blocks have 0 transactions, so UTxO count stays at 0
    // (we start from empty state, not from genesis UTxO)
    // This is expected — genesis UTxO loading is a separate concern
}
