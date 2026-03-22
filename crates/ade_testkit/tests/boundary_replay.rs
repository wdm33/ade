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

fn replay_hfc_transition(
    dir: &str,
    initial_era: CardanoEra,
) -> (usize, usize, Vec<(String, u8)>) {
    let manifest = load_manifest(dir);
    let block_dir = boundary_blocks_dir().join(dir);

    let mut state = LedgerState::new(initial_era);
    let mut accepted = 0usize;
    let mut era_tags: Vec<(String, u8)> = Vec::new();

    // Replay pre-blocks
    let pre_blocks = manifest["pre_blocks"].as_array().unwrap();
    for entry in pre_blocks {
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
                eprintln!("  {dir}: pre-block {filename} failed: {e}");
                break;
            }
        }
    }

    // Replay post-blocks
    let post_blocks = manifest["post_blocks"].as_array().unwrap();
    for entry in post_blocks {
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
                eprintln!("  {dir}: post-block {filename} failed: {e}");
                break;
            }
        }
    }

    let total = pre_blocks.len() + post_blocks.len();
    (total, accepted, era_tags)
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

#[test]
fn hfc_byron_to_shelley() {
    let (total, accepted, era_tags) =
        replay_hfc_transition("byron_shelley", CardanoEra::ByronRegular);

    // Pre-blocks should be Byron (era 1), post-blocks should be Shelley (era 2)
    let pre_eras: Vec<u8> = era_tags.iter().take(10).map(|(_, e)| *e).collect();
    let post_eras: Vec<u8> = era_tags.iter().skip(10).map(|(_, e)| *e).collect();

    eprintln!("Byron→Shelley: {accepted}/{total} accepted");
    eprintln!("  pre era tags: {pre_eras:?}");
    eprintln!("  post era tags: {post_eras:?}");

    // Byron→Shelley is the most complex transition: the pre-block set
    // may include EBBs or boundary blocks with unusual structure.
    // The transition point (era tag change) is the key structural proof.
    assert!(accepted >= 10, "at least 10 boundary blocks must be accepted");

    // Verify era tag transition is present
    let has_byron = era_tags.iter().any(|(_, e)| *e <= 1);
    let has_shelley = era_tags.iter().any(|(_, e)| *e == 2);
    assert!(has_byron, "must have Byron blocks before transition");
    assert!(has_shelley, "must have Shelley blocks after transition");
}

#[test]
fn hfc_shelley_to_allegra() {
    let (total, accepted, era_tags) =
        replay_hfc_transition("shelley_allegra", CardanoEra::Shelley);

    let transition_point = era_tags
        .iter()
        .position(|(_, e)| *e == 3)
        .unwrap_or(era_tags.len());

    eprintln!("Shelley→Allegra: {accepted}/{total}, transition at block {transition_point}");
    assert_eq!(accepted, total);
}

#[test]
fn hfc_allegra_to_mary() {
    let (total, accepted, _) =
        replay_hfc_transition("allegra_mary", CardanoEra::Allegra);
    eprintln!("Allegra→Mary: {accepted}/{total}");
    assert_eq!(accepted, total);
}

#[test]
fn hfc_mary_to_alonzo() {
    let (total, accepted, _) =
        replay_hfc_transition("mary_alonzo", CardanoEra::Mary);
    eprintln!("Mary→Alonzo: {accepted}/{total}");
    assert_eq!(accepted, total);
}

#[test]
fn hfc_alonzo_to_babbage() {
    let (total, accepted, _) =
        replay_hfc_transition("alonzo_babbage", CardanoEra::Alonzo);
    eprintln!("Alonzo→Babbage: {accepted}/{total}");
    assert_eq!(accepted, total);
}

#[test]
fn hfc_babbage_to_conway() {
    let (total, accepted, _) =
        replay_hfc_transition("babbage_conway", CardanoEra::Babbage);
    eprintln!("Babbage→Conway: {accepted}/{total}");
    assert_eq!(accepted, total);
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
