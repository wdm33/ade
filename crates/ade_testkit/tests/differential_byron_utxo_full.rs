//! Differential test: Byron UTxO equality across the full 1,500-block window.
//!
//! Replays all 1,500 contiguous Byron blocks with genesis UTxO loaded,
//! and verifies:
//! 1. Every block accepted (verdict agreement)
//! 2. UTxO set remains consistent throughout (no spurious mutations)
//! 3. UTxO set at final block matches oracle dump at slot 1
//!    (since all 1,500 blocks have 0 transactions, UTxO is invariant)
//! 4. Determinism: full replay produces identical final state on two runs
//!
//! Sub-surface ladder: Byron UTxO equality across full window.

use std::collections::BTreeMap;
use std::path::PathBuf;

use ade_codec::cbor;
use ade_codec::cbor::envelope::decode_block_envelope;
use ade_ledger::rules::apply_block;
use ade_ledger::state::{EpochState, LedgerState};
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::utxo::TxOut;
use ade_testkit::harness::genesis_loader::load_genesis_utxo;
use ade_types::tx::TxIn;
use ade_types::{CardanoEra, Hash32};

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
}

fn dumps_root() -> PathBuf {
    corpus_root().join("ext_ledger_state_dumps")
}

fn contiguous_root() -> PathBuf {
    corpus_root().join("contiguous")
}

fn load_blocks_json() -> serde_json::Value {
    let path = contiguous_root().join("byron").join("blocks.json");
    let content = std::fs::read_to_string(&path).unwrap();
    serde_json::from_str(&content).unwrap()
}

/// Extract oracle UTxO from a raw ExtLedgerState dump.
fn extract_oracle_utxo(dump_path: &std::path::Path) -> BTreeMap<TxIn, (Vec<u8>, u64)> {
    let data = std::fs::read(dump_path).unwrap();
    let mut off = 0;

    // Navigate: array(2) > array(1) > array(2) > skip Bound > array(3) > skip tip > array(5) > skip ver > skip slot > map
    let _ = cbor::read_array_header(&data, &mut off).unwrap();
    let _ = cbor::read_array_header(&data, &mut off).unwrap();
    let _ = cbor::read_array_header(&data, &mut off).unwrap();
    let _ = cbor::skip_item(&data, &mut off).unwrap();
    let _ = cbor::read_array_header(&data, &mut off).unwrap();
    let _ = cbor::skip_item(&data, &mut off).unwrap();
    let _ = cbor::read_array_header(&data, &mut off).unwrap();
    let _ = cbor::skip_item(&data, &mut off).unwrap();
    let _ = cbor::skip_item(&data, &mut off).unwrap();

    let map_enc = cbor::read_map_header(&data, &mut off).unwrap();
    let count = match map_enc {
        cbor::ContainerEncoding::Definite(n, _) => n,
        _ => panic!("expected definite map"),
    };

    let mut result = BTreeMap::new();
    for _ in 0..count {
        let _ = cbor::read_array_header(&data, &mut off).unwrap();
        let _ = cbor::read_array_header(&data, &mut off).unwrap();
        let mut hash_bytes = [0u8; 32];
        for chunk in 0..4 {
            let (val, _) = cbor::read_uint(&data, &mut off).unwrap();
            hash_bytes[chunk * 8..(chunk + 1) * 8].copy_from_slice(&val.to_be_bytes());
        }
        let (index, _) = cbor::read_uint(&data, &mut off).unwrap();
        let _ = cbor::read_array_header(&data, &mut off).unwrap();
        let (addr_bytes, _) = cbor::read_bytes(&data, &mut off).unwrap();
        let (coin, _) = cbor::read_uint(&data, &mut off).unwrap();

        result.insert(
            TxIn { tx_hash: Hash32(hash_bytes), index: index as u16 },
            (addr_bytes, coin),
        );
    }
    result
}

/// Compare Ade UTxO state against oracle UTxO.
/// Returns (matches, mismatches, first_mismatch_description).
fn compare_utxo_sets(
    ade: &ade_ledger::utxo::UTxOState,
    oracle: &BTreeMap<TxIn, (Vec<u8>, u64)>,
) -> (usize, usize, Option<String>) {
    let mut matches = 0usize;
    let mut mismatches = 0usize;
    let mut first_mismatch: Option<String> = None;

    for (tx_in, (oracle_addr, oracle_coin)) in oracle {
        match ade.utxos.get(tx_in) {
            None => {
                mismatches += 1;
                if first_mismatch.is_none() {
                    first_mismatch = Some(format!(
                        "missing TxIn {:?}#{}",
                        tx_in.tx_hash, tx_in.index
                    ));
                }
            }
            Some(tx_out) => {
                let (ade_addr, ade_coin) = match tx_out {
                    TxOut::Byron { address, coin } => (address.as_bytes(), coin.0),
                    TxOut::ShelleyMary { .. } => {
                        mismatches += 1;
                        continue;
                    }
                };

                if ade_coin != *oracle_coin || ade_addr != oracle_addr.as_slice() {
                    mismatches += 1;
                    if first_mismatch.is_none() {
                        first_mismatch = Some(format!(
                            "value mismatch at TxIn {:?}#{}",
                            tx_in.tx_hash, tx_in.index
                        ));
                    }
                } else {
                    matches += 1;
                }
            }
        }
    }

    // Check for entries in Ade not in oracle
    for tx_in in ade.utxos.keys() {
        if !oracle.contains_key(tx_in) {
            mismatches += 1;
            if first_mismatch.is_none() {
                first_mismatch = Some(format!(
                    "extra TxIn {:?}#{} in Ade",
                    tx_in.tx_hash, tx_in.index
                ));
            }
        }
    }

    (matches, mismatches, first_mismatch)
}

fn make_byron_state(utxo: ade_ledger::utxo::UTxOState) -> LedgerState {
    LedgerState {
        utxo_state: utxo,
        epoch_state: EpochState::new(),
        protocol_params: ProtocolParameters::default(),
        era: CardanoEra::ByronRegular,
        track_utxo: false,
        cert_state: ade_ledger::delegation::CertState::new(),
        max_lovelace_supply: 45_000_000_000_000_000,
        gov_state: None,
    }
}

#[test]
fn byron_utxo_equality_full_1500_blocks() {
    // Load genesis UTxO
    let genesis_path = dumps_root().join("byron").join("slot_0.bin");
    let genesis_utxo = load_genesis_utxo(&genesis_path).unwrap();
    assert_eq!(genesis_utxo.len(), 14505);

    // Load oracle UTxO at slot 1 for comparison
    let oracle_slot1 = extract_oracle_utxo(&dumps_root().join("byron").join("slot_1.bin"));
    assert_eq!(oracle_slot1.len(), 14505);

    // Create initial state
    let mut state = make_byron_state(genesis_utxo);

    // Replay all 1,500 blocks
    let blocks_json = load_blocks_json();
    let blocks = blocks_json["blocks"].as_array().unwrap();
    let era_dir = contiguous_root().join("byron");

    let mut accepted = 0usize;
    let mut utxo_count_changes = 0usize;
    let prev_count = state.utxo_state.len();

    for block_entry in blocks.iter() {
        let filename = block_entry["file"].as_str().unwrap();
        let raw = std::fs::read(era_dir.join(filename)).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        state = apply_block(&state, env.era, inner)
            .unwrap_or_else(|e| panic!("block {filename} rejected: {e}"));
        accepted += 1;

        if state.utxo_state.len() != prev_count {
            utxo_count_changes += 1;
        }
    }

    // All blocks accepted
    assert_eq!(accepted, 1500);

    // UTxO count never changed (0 transactions in all 1,500 blocks)
    assert_eq!(utxo_count_changes, 0, "UTxO count changed during replay");
    assert_eq!(state.utxo_state.len(), 14505);

    // Final UTxO matches oracle at slot 1
    let (matches, mismatches, first_mismatch) = compare_utxo_sets(&state.utxo_state, &oracle_slot1);
    assert_eq!(
        mismatches, 0,
        "UTxO divergence after 1,500 blocks: {mismatches} mismatches, first: {:?}",
        first_mismatch
    );
    assert_eq!(matches, 14505);

    eprintln!("Byron full-window UTxO equality:");
    eprintln!("  Blocks replayed: {accepted}");
    eprintln!("  UTxO count changes: {utxo_count_changes}");
    eprintln!("  Final UTxO: {}/{} match oracle", matches, oracle_slot1.len());
}

#[test]
fn byron_full_replay_determinism() {
    let genesis_path = dumps_root().join("byron").join("slot_0.bin");
    let blocks_json = load_blocks_json();
    let blocks = blocks_json["blocks"].as_array().unwrap();
    let era_dir = contiguous_root().join("byron");

    // Run 1
    let utxo1 = load_genesis_utxo(&genesis_path).unwrap();
    let mut state1 = make_byron_state(utxo1);
    for block_entry in blocks.iter() {
        let filename = block_entry["file"].as_str().unwrap();
        let raw = std::fs::read(era_dir.join(filename)).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];
        state1 = apply_block(&state1, env.era, inner).unwrap();
    }

    // Run 2
    let utxo2 = load_genesis_utxo(&genesis_path).unwrap();
    let mut state2 = make_byron_state(utxo2);
    for block_entry in blocks.iter() {
        let filename = block_entry["file"].as_str().unwrap();
        let raw = std::fs::read(era_dir.join(filename)).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];
        state2 = apply_block(&state2, env.era, inner).unwrap();
    }

    // States must be identical
    assert_eq!(state1, state2, "Non-deterministic replay");
    eprintln!("Byron full replay determinism: 2 runs identical after 1,500 blocks");
}
