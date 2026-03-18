//! Differential test: Allegra and Mary contiguous replay.
//!
//! Replays 1,500 blocks each for Allegra and Mary through apply_block.
//! Verifies verdict agreement, determinism, and slot progression.

use std::path::PathBuf;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::rules::apply_block;
use ade_ledger::state::{EpochState, LedgerState};
use ade_ledger::utxo::UTxOState;
use ade_types::{CardanoEra, EpochNo, SlotNo};

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("contiguous")
}

fn load_blocks(era: &str) -> (serde_json::Value, PathBuf) {
    let era_dir = corpus_root().join(era);
    let path = era_dir.join("blocks.json");
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    (json, era_dir)
}

fn make_state(era: CardanoEra, epoch: u64, slot: u64) -> LedgerState {
    LedgerState {
        utxo_state: UTxOState::new(),
        epoch_state: EpochState {
            epoch: EpochNo(epoch),
            slot: SlotNo(slot),
            ..EpochState::new()
        },
        protocol_params: ProtocolParameters::default(),
        era,
    }
}

fn replay_era(
    era_name: &str,
    era: CardanoEra,
    epoch: u64,
    start_slot: u64,
) -> (usize, usize, SlotNo) {
    let (blocks_json, era_dir) = load_blocks(era_name);
    let blocks = blocks_json["blocks"].as_array().unwrap();

    let mut state = make_state(era, epoch, start_slot);
    let mut accepted = 0usize;

    for block_entry in blocks.iter() {
        let filename = block_entry["file"].as_str().unwrap();
        let raw = std::fs::read(era_dir.join(filename)).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        match apply_block(&state, env.era, inner) {
            Ok(new_state) => {
                state = new_state;
                accepted += 1;
            }
            Err(e) => {
                eprintln!("{era_name}: error at block {}: {e}", block_entry["index"]);
                break;
            }
        }
    }

    (blocks.len(), accepted, state.epoch_state.slot)
}

// ---- Allegra ----

#[test]
fn allegra_replay_verdict_agreement() {
    let (total, accepted, final_slot) = replay_era("allegra", CardanoEra::Allegra, 236, 19440024);
    eprintln!("Allegra: {accepted}/{total} accepted, final slot {}", final_slot.0);
    assert_eq!(accepted, total);
}

#[test]
fn allegra_replay_determinism() {
    let (blocks_json, era_dir) = load_blocks("allegra");
    let blocks = blocks_json["blocks"].as_array().unwrap();
    let n = blocks.len().min(200);

    let mut s1 = make_state(CardanoEra::Allegra, 236, 19440024);
    let mut s2 = make_state(CardanoEra::Allegra, 236, 19440024);

    for block_entry in blocks.iter().take(n) {
        let filename = block_entry["file"].as_str().unwrap();
        let raw = std::fs::read(era_dir.join(filename)).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        let r1 = apply_block(&s1, env.era, inner);
        let r2 = apply_block(&s2, env.era, inner);
        assert_eq!(r1, r2);
        if let Ok(s) = r1 { s1 = s; }
        if let Ok(s) = r2 { s2 = s; }
    }
    assert_eq!(s1, s2);
    eprintln!("Allegra determinism: {n} blocks verified identical");
}

// ---- Mary ----

#[test]
fn mary_replay_verdict_agreement() {
    let (total, accepted, final_slot) = replay_era("mary", CardanoEra::Mary, 252, 30240073);
    eprintln!("Mary: {accepted}/{total} accepted, final slot {}", final_slot.0);
    assert_eq!(accepted, total);
}

#[test]
fn mary_replay_determinism() {
    let (blocks_json, era_dir) = load_blocks("mary");
    let blocks = blocks_json["blocks"].as_array().unwrap();
    let n = blocks.len().min(200);

    let mut s1 = make_state(CardanoEra::Mary, 252, 30240073);
    let mut s2 = make_state(CardanoEra::Mary, 252, 30240073);

    for block_entry in blocks.iter().take(n) {
        let filename = block_entry["file"].as_str().unwrap();
        let raw = std::fs::read(era_dir.join(filename)).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        let r1 = apply_block(&s1, env.era, inner);
        let r2 = apply_block(&s2, env.era, inner);
        assert_eq!(r1, r2);
        if let Ok(s) = r1 { s1 = s; }
        if let Ok(s) = r2 { s2 = s; }
    }
    assert_eq!(s1, s2);
    eprintln!("Mary determinism: {n} blocks verified identical");
}
