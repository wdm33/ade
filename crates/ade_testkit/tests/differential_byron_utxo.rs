//! Differential test: Byron replay with genesis UTxO.
//!
//! Loads the genesis UTxO set from the ExtLedgerState binary dump,
//! then replays all 1,500 contiguous Byron blocks through apply_block.
//! This is the first test that starts from a realistic initial state
//! rather than an empty UTxO set.
//!
//! Verifies:
//! 1. Genesis UTxO has exactly 14,505 entries
//! 2. All 1,500 blocks are accepted (verdict agreement WITH genesis state)
//! 3. UTxO count is tracked through the replay

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_ledger::rules::apply_block;
use ade_ledger::state::LedgerState;
use ade_testkit::harness::genesis_loader::load_genesis_utxo;
use ade_types::CardanoEra;

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
}

fn genesis_dump_path() -> PathBuf {
    corpus_root()
        .join("ext_ledger_state_dumps")
        .join("byron")
        .join("slot_0.bin")
}

fn load_blocks_json() -> serde_json::Value {
    let path = corpus_root().join("contiguous").join("byron").join("blocks.json");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&content).unwrap()
}

/// Replay all 1,500 Byron blocks starting from genesis UTxO and verify
/// verdict agreement + UTxO progression.
#[test]
fn byron_replay_with_genesis_utxo() {
    let dump = genesis_dump_path();
    if !dump.exists() {
        eprintln!(
            "SKIP: genesis dump not found at {}",
            dump.display()
        );
        return;
    }

    // 1. Load genesis UTxO
    let genesis_utxo = load_genesis_utxo(&dump).unwrap();
    let initial_count = genesis_utxo.len();
    assert_eq!(
        initial_count, 14_505,
        "expected 14,505 genesis UTxO entries, got {initial_count}"
    );
    eprintln!("Genesis UTxO loaded: {initial_count} entries");

    // 2. Create LedgerState with genesis UTxO
    let mut state = LedgerState::new(CardanoEra::ByronRegular);
    state.utxo_state = genesis_utxo;

    // 3. Replay all contiguous Byron blocks
    let blocks_json = load_blocks_json();
    let blocks = blocks_json["blocks"].as_array().unwrap();
    let era_dir = corpus_root().join("contiguous").join("byron");

    let mut accepted = 0usize;
    let mut first_error: Option<(usize, String)> = None;
    let mut min_utxo = state.utxo_state.len();
    let mut max_utxo = state.utxo_state.len();

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

                let count = state.utxo_state.len();
                if count < min_utxo {
                    min_utxo = count;
                }
                if count > max_utxo {
                    max_utxo = count;
                }
            }
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some((i, format!("{e}")));
                }
            }
        }
    }

    let final_count = state.utxo_state.len();

    eprintln!(
        "Byron replay with genesis UTxO: {accepted}/{} blocks accepted",
        blocks.len()
    );
    eprintln!(
        "UTxO progression: {initial_count} → {final_count} (min={min_utxo}, max={max_utxo})"
    );

    if let Some((idx, err)) = &first_error {
        eprintln!("First error at block {idx}: {err}");
    }

    // 4. Assert all blocks accepted
    assert_eq!(
        accepted,
        blocks.len(),
        "verdict disagreement: {accepted}/{} blocks accepted. First error: {:?}",
        blocks.len(),
        first_error
    );
}
