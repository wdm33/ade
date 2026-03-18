//! Differential test: Contiguous replay across all Phase 2B eras.
//!
//! Replays 6,000 contiguous blocks (Byron–Mary) through apply_block.
//! Verifies structural verdict agreement: every block the oracle accepted,
//! Ade also accepts (block + tx body decoding succeeds).
//!
//! Sub-surface comparison ladder rung 1: verdict agreement.

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

struct ReplayResult {
    era: &'static str,
    total: usize,
    accepted: usize,
    first_error: Option<(usize, String)>,
}

fn replay_era(era_name: &'static str, initial_era: CardanoEra) -> ReplayResult {
    let blocks_json = load_blocks_json(era_name);
    let blocks = blocks_json["blocks"].as_array().unwrap();
    let era_dir = corpus_root().join(era_name);

    let mut state = LedgerState::new(initial_era);
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
                // Continue counting but can't advance state
                break;
            }
        }
    }

    ReplayResult {
        era: era_name,
        total: blocks.len(),
        accepted,
        first_error,
    }
}

#[test]
fn byron_replay_all_1500() {
    let r = replay_era("byron", CardanoEra::ByronRegular);
    eprintln!("Byron: {}/{} accepted", r.accepted, r.total);
    if let Some((idx, ref err)) = r.first_error {
        eprintln!("  First error at block {idx}: {err}");
    }
    assert_eq!(r.accepted, r.total, "Byron: {}/{}", r.accepted, r.total);
}

#[test]
fn shelley_replay_all_1500() {
    let r = replay_era("shelley", CardanoEra::Shelley);
    eprintln!("Shelley: {}/{} accepted", r.accepted, r.total);
    if let Some((idx, ref err)) = r.first_error {
        eprintln!("  First error at block {idx}: {err}");
    }
    assert_eq!(r.accepted, r.total, "Shelley: {}/{}", r.accepted, r.total);
}

#[test]
fn allegra_replay_all_1500() {
    let r = replay_era("allegra", CardanoEra::Allegra);
    eprintln!("Allegra: {}/{} accepted", r.accepted, r.total);
    if let Some((idx, ref err)) = r.first_error {
        eprintln!("  First error at block {idx}: {err}");
    }
    assert_eq!(r.accepted, r.total, "Allegra: {}/{}", r.accepted, r.total);
}

#[test]
fn mary_replay_all_1500() {
    let r = replay_era("mary", CardanoEra::Mary);
    eprintln!("Mary: {}/{} accepted", r.accepted, r.total);
    if let Some((idx, ref err)) = r.first_error {
        eprintln!("  First error at block {idx}: {err}");
    }
    assert_eq!(r.accepted, r.total, "Mary: {}/{}", r.accepted, r.total);
}

/// Summary test: all 4 eras, 6,000 blocks total.
#[test]
fn all_eras_replay_summary() {
    let results = vec![
        replay_era("byron", CardanoEra::ByronRegular),
        replay_era("shelley", CardanoEra::Shelley),
        replay_era("allegra", CardanoEra::Allegra),
        replay_era("mary", CardanoEra::Mary),
    ];

    let mut total_blocks = 0usize;
    let mut total_accepted = 0usize;
    let mut any_failed = false;

    eprintln!("\n=== Phase 2B Contiguous Replay Summary ===");
    for r in &results {
        let status = if r.accepted == r.total { "PASS" } else { "FAIL" };
        eprintln!("  {}: {}/{} [{}]", r.era, r.accepted, r.total, status);
        if let Some((idx, ref err)) = r.first_error {
            eprintln!("    First error at block {idx}: {err}");
            any_failed = true;
        }
        total_blocks += r.total;
        total_accepted += r.accepted;
    }
    eprintln!("  Total: {total_accepted}/{total_blocks}");
    eprintln!("==========================================\n");

    assert!(
        !any_failed,
        "Verdict disagreement: {total_accepted}/{total_blocks} accepted"
    );
}
