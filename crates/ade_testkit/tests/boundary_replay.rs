//! Integration test: Replay blocks across HFC and epoch boundaries.
//!
//! Proves that the boundary blocks decode and apply through the existing
//! pipeline. For HFC transitions, the era tag changes mid-sequence.
//! For epoch boundaries, blocks span the epoch transition slot.
//!
//! This is the template for T-25 (epoch boundary logic) and T-26
//! (HFC translation). Full translation/epoch-boundary semantics
//! are not yet implemented — this test proves the block-level
//! pipeline works at boundaries.

use std::path::PathBuf;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_ledger::rules::apply_block_classified;
use ade_ledger::state::LedgerState;
use ade_types::CardanoEra;

fn boundary_blocks_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("boundary_blocks")
}

fn load_manifest(dir: &str) -> serde_json::Value {
    let path = boundary_blocks_dir().join(dir).join("manifest.json");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&content).unwrap()
}

/// Result of replaying blocks across an HFC boundary.
struct HfcReplayResult {
    total: usize,
    accepted: usize,
    era_tags: Vec<(String, u8)>,
    first_failure: Option<(usize, String, String)>, // (index, filename, error)
}

fn replay_hfc_transition(
    dir: &str,
    initial_era: CardanoEra,
) -> HfcReplayResult {
    let manifest = load_manifest(dir);
    let block_dir = boundary_blocks_dir().join(dir);

    let mut state = LedgerState::new(initial_era);
    let mut accepted = 0usize;
    let mut era_tags: Vec<(String, u8)> = Vec::new();
    let mut first_failure: Option<(usize, String, String)> = None;

    // Manifests use a flat "blocks" array. Pre-boundary blocks have filenames
    // starting with "pre_", post-boundary blocks start with "blk_".
    let blocks = manifest["blocks"].as_array().unwrap();
    for (idx, entry) in blocks.iter().enumerate() {
        let filename = entry["file"].as_str().unwrap();
        let raw = std::fs::read(block_dir.join(filename)).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        era_tags.push((filename.to_string(), env.era.as_u8()));

        match apply_block_classified(&state, env.era, inner) {
            Ok((new_state, _verdict)) => {
                state = new_state;
                accepted += 1;
            }
            Err(e) => {
                if first_failure.is_none() {
                    first_failure = Some((idx, filename.to_string(), format!("{e}")));
                }
                // Continue past failures to collect era tags from all blocks.
                // This is diagnostic-only — the authoritative replay path
                // (apply_block_classified) still fails-fast.
            }
        }
    }

    HfcReplayResult {
        total: blocks.len(),
        accepted,
        era_tags,
        first_failure,
    }
}

fn replay_epoch_boundary(dir: &str, era: CardanoEra) -> (usize, usize) {
    let manifest = load_manifest(dir);
    let block_dir = boundary_blocks_dir().join(dir);

    let mut state = LedgerState::new(era);
    let mut accepted = 0usize;

    let blocks = manifest["blocks"].as_array().unwrap();
    for entry in blocks {
        let filename = entry["file"].as_str().unwrap();
        let raw = std::fs::read(block_dir.join(filename)).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        match apply_block_classified(&state, env.era, inner) {
            Ok((new_state, _verdict)) => {
                state = new_state;
                accepted += 1;
            }
            Err(e) => {
                eprintln!("  {dir}: block {filename} failed: {e}");
                break;
            }
        }
    }

    (blocks.len(), accepted)
}

// ---- HFC Transition Tests ----

/// Assert that the replay result shows the expected era transition.
/// Prints diagnostic info on failure: era tags, first failure, block counts.
fn assert_hfc_result(
    label: &str,
    result: &HfcReplayResult,
    expected_pre_era: u8,
    expected_post_era: u8,
) {
    let has_pre = result.era_tags.iter().any(|(_, e)| *e == expected_pre_era);
    let has_post = result.era_tags.iter().any(|(_, e)| *e == expected_post_era);
    let eras: Vec<u8> = result.era_tags.iter().map(|(_, e)| *e).collect();

    eprintln!("{label}: {}/{} accepted, era tags: {eras:?}",
        result.accepted, result.total);
    if let Some((idx, ref file, ref err)) = result.first_failure {
        eprintln!("  first failure: block {idx} ({file}): {err}");
    }

    assert!(has_pre, "{label}: fixture must contain pre-era blocks (era {expected_pre_era})");

    if !has_post {
        // Informative failure: post-era blocks exist in fixture but replay
        // didn't reach them (earlier blocks failed).
        let post_in_fixture = result.era_tags.iter()
            .any(|(f, _)| f.starts_with("blk_"));
        if post_in_fixture {
            if let Some((idx, ref file, ref err)) = result.first_failure {
                panic!(
                    "{label}: replay did not reach post-era blocks (era {expected_post_era}). \
                     First failure at block {idx} ({file}): {err}. \
                     {} blocks accepted of {} total.",
                    result.accepted, result.total,
                );
            }
        }
        panic!(
            "{label}: expected post-era blocks (era {expected_post_era}) in fixture, \
             era tags seen: {eras:?}",
        );
    }
}

#[test]
fn hfc_byron_to_shelley() {
    let result = replay_hfc_transition("byron_shelley", CardanoEra::ByronRegular);
    assert_hfc_result("Byron→Shelley", &result, 1, 2);
}

#[test]
fn hfc_shelley_to_allegra() {
    let result = replay_hfc_transition("shelley_allegra", CardanoEra::Shelley);
    assert_hfc_result("Shelley→Allegra", &result, 2, 3);
    assert_eq!(result.accepted, result.total);
}

#[test]
fn hfc_allegra_to_mary() {
    let result = replay_hfc_transition("allegra_mary", CardanoEra::Allegra);
    assert_hfc_result("Allegra→Mary", &result, 3, 4);
    assert_eq!(result.accepted, result.total);
}

#[test]
fn hfc_mary_to_alonzo() {
    let result = replay_hfc_transition("mary_alonzo", CardanoEra::Mary);
    assert_hfc_result("Mary→Alonzo", &result, 4, 5);
    assert_eq!(result.accepted, result.total);
}

#[test]
fn hfc_alonzo_to_babbage() {
    let result = replay_hfc_transition("alonzo_babbage", CardanoEra::Alonzo);
    assert_hfc_result("Alonzo→Babbage", &result, 5, 6);
    assert_eq!(result.accepted, result.total);
}

#[test]
fn hfc_babbage_to_conway() {
    let result = replay_hfc_transition("babbage_conway", CardanoEra::Babbage);
    assert_hfc_result("Babbage→Conway", &result, 6, 7);
    assert_eq!(result.accepted, result.total);
}

// ---- Epoch Boundary Tests ----

#[test]
fn epoch_boundary_shelley() {
    let (total, accepted) = replay_epoch_boundary("shelley_epoch209", CardanoEra::Shelley);
    eprintln!("Shelley epoch 209: {accepted}/{total}");
    assert_eq!(accepted, total);
}

#[test]
fn epoch_boundary_allegra() {
    let (total, accepted) = replay_epoch_boundary("allegra_epoch237", CardanoEra::Allegra);
    eprintln!("Allegra epoch 237: {accepted}/{total}");
    assert_eq!(accepted, total);
}

#[test]
fn epoch_boundary_mary() {
    let (total, accepted) = replay_epoch_boundary("mary_epoch252", CardanoEra::Mary);
    eprintln!("Mary epoch 252: {accepted}/{total}");
    assert_eq!(accepted, total);
}

#[test]
fn epoch_boundary_alonzo() {
    let (total, accepted) = replay_epoch_boundary("alonzo_epoch291", CardanoEra::Alonzo);
    eprintln!("Alonzo epoch 291: {accepted}/{total}");
    assert_eq!(accepted, total);
}

#[test]
fn epoch_boundary_babbage() {
    let (total, accepted) = replay_epoch_boundary("babbage_epoch366", CardanoEra::Babbage);
    eprintln!("Babbage epoch 366: {accepted}/{total}");
    assert_eq!(accepted, total);
}

#[test]
fn epoch_boundary_conway() {
    let (total, accepted) = replay_epoch_boundary("conway_epoch508", CardanoEra::Conway);
    eprintln!("Conway epoch 508: {accepted}/{total}");
    assert_eq!(accepted, total);
}
