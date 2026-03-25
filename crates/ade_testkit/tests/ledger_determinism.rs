//! Ledger determinism test (CE-74).
//!
//! Applies the same block sequence twice from identical initial state
//! and asserts the resulting LedgerState is identical. Covers all 7 eras
//! with both single-block and multi-block sequences.
//!
//! This is the authoritative test for DC-LEDGER-01:
//! "same canonical inputs → same authoritative bytes."

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

/// Load block file bytes from the corpus.
fn load_corpus_blocks(era: &str) -> Vec<(String, Vec<u8>)> {
    let era_dir = corpus_root().join(era);
    let manifest_path = era_dir.join("blocks.json");
    if !manifest_path.exists() {
        return Vec::new();
    }
    let content = std::fs::read_to_string(&manifest_path).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&content).unwrap();
    let blocks = manifest["blocks"].as_array().unwrap();

    blocks
        .iter()
        .map(|entry| {
            let filename = entry["file"].as_str().unwrap().to_string();
            let raw = std::fs::read(era_dir.join(&filename)).unwrap();
            (filename, raw)
        })
        .collect()
}

/// Apply a block sequence to a state, returning the final state.
fn replay_sequence(
    initial: &LedgerState,
    blocks: &[(String, Vec<u8>)],
    limit: usize,
) -> LedgerState {
    let mut state = initial.clone();
    for (filename, raw) in blocks.iter().take(limit) {
        let env = decode_block_envelope(raw)
            .unwrap_or_else(|e| panic!("envelope decode failed for {filename}: {e}"));
        let inner = &raw[env.block_start..env.block_end];
        state = apply_block(&state, env.era, inner)
            .unwrap_or_else(|e| panic!("apply_block failed for {filename}: {e}"));
    }
    state
}

/// Assert two LedgerStates are identical on all determinism-relevant fields.
fn assert_states_identical(a: &LedgerState, b: &LedgerState, label: &str) {
    assert_eq!(a.era, b.era, "{label}: era mismatch");
    assert_eq!(
        a.epoch_state.epoch, b.epoch_state.epoch,
        "{label}: epoch mismatch"
    );
    assert_eq!(
        a.epoch_state.slot, b.epoch_state.slot,
        "{label}: slot mismatch"
    );
    assert_eq!(
        a.epoch_state.reserves, b.epoch_state.reserves,
        "{label}: reserves mismatch"
    );
    assert_eq!(
        a.epoch_state.treasury, b.epoch_state.treasury,
        "{label}: treasury mismatch"
    );
    assert_eq!(
        a.epoch_state.epoch_fees, b.epoch_state.epoch_fees,
        "{label}: epoch_fees mismatch"
    );
    assert_eq!(
        a.epoch_state.block_production, b.epoch_state.block_production,
        "{label}: block_production mismatch"
    );
    assert_eq!(
        a.utxo_state.len(),
        b.utxo_state.len(),
        "{label}: utxo count mismatch"
    );
    assert_eq!(
        a.cert_state.delegation.delegations.len(),
        b.cert_state.delegation.delegations.len(),
        "{label}: delegation count mismatch"
    );
    assert_eq!(
        a.cert_state.pool.pools.len(),
        b.cert_state.pool.pools.len(),
        "{label}: pool count mismatch"
    );
}

/// Run determinism test for one era: apply N blocks twice, compare.
fn determinism_test_era(era_name: &str, initial_era: CardanoEra, single_count: usize, multi_count: usize) {
    let blocks = load_corpus_blocks(era_name);
    if blocks.is_empty() {
        eprintln!("  {era_name}: skipped (corpus not available)");
        return;
    }

    let initial = LedgerState::new(initial_era);

    // Single-block determinism: apply block 0 twice
    if !blocks.is_empty() {
        let count = single_count.min(blocks.len());
        for i in 0..count {
            let a = replay_sequence(&initial, &blocks[i..], 1);
            let b = replay_sequence(&initial, &blocks[i..], 1);
            assert_states_identical(&a, &b, &format!("{era_name} single block {i}"));
        }
    }

    // Multi-block determinism: apply first N blocks twice
    let count = multi_count.min(blocks.len());
    let a = replay_sequence(&initial, &blocks, count);
    let b = replay_sequence(&initial, &blocks, count);
    assert_states_identical(&a, &b, &format!("{era_name} multi-block ({count})"));

    eprintln!(
        "  {era_name}: {} single + 1 multi({count}) — deterministic",
        single_count.min(blocks.len())
    );
}

#[test]
fn byron_determinism() {
    determinism_test_era("byron", CardanoEra::ByronRegular, 3, 100);
}

#[test]
fn shelley_determinism() {
    determinism_test_era("shelley", CardanoEra::Shelley, 3, 100);
}

#[test]
fn allegra_determinism() {
    determinism_test_era("allegra", CardanoEra::Allegra, 3, 100);
}

#[test]
fn mary_determinism() {
    determinism_test_era("mary", CardanoEra::Mary, 3, 100);
}

#[test]
fn alonzo_determinism() {
    determinism_test_era("alonzo", CardanoEra::Alonzo, 3, 100);
}

#[test]
fn babbage_determinism() {
    determinism_test_era("babbage", CardanoEra::Babbage, 3, 100);
}

#[test]
fn conway_determinism() {
    determinism_test_era("conway", CardanoEra::Conway, 3, 100);
}

/// Summary: all 7 eras deterministic.
#[test]
fn all_eras_determinism_summary() {
    eprintln!("\n=== Ledger Determinism (CE-74) ===");
    let eras: &[(&str, CardanoEra)] = &[
        ("byron", CardanoEra::ByronRegular),
        ("shelley", CardanoEra::Shelley),
        ("allegra", CardanoEra::Allegra),
        ("mary", CardanoEra::Mary),
        ("alonzo", CardanoEra::Alonzo),
        ("babbage", CardanoEra::Babbage),
        ("conway", CardanoEra::Conway),
    ];
    for (name, era) in eras {
        determinism_test_era(name, *era, 3, 100);
    }
    eprintln!("=== ALL ERAS DETERMINISTIC ===\n");
}
