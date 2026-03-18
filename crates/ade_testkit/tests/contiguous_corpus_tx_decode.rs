//! Integration test: Transaction body decoding on the contiguous corpus.
//!
//! Verifies that all transaction bodies within the 6,000 contiguous blocks
//! decode correctly through the era-specific tx body decoders. This exercises
//! S-04 (wire-byte decode boundary) at scale.

use std::path::PathBuf;

use ade_codec::allegra;
use ade_codec::byron;
use ade_codec::cbor;
use ade_codec::cbor::envelope::decode_block_envelope;
use ade_codec::mary;
use ade_codec::shelley;
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

#[test]
fn byron_contiguous_tx_decode() {
    let blocks_json = load_blocks_json("byron");
    let blocks = blocks_json["blocks"].as_array().unwrap();
    let era_dir = corpus_root().join("byron");

    let mut total_txs = 0;
    for block_entry in blocks {
        let filename = block_entry["file"].as_str().unwrap();
        let path = era_dir.join(filename);
        let raw = std::fs::read(&path).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        if env.era == CardanoEra::ByronRegular {
            let preserved = byron::decode_byron_regular_block(inner).unwrap();
            let block = preserved.decoded();
            let txs = byron::tx::decode_byron_block_txs(&block.body).unwrap();
            total_txs += txs.len();
        }
    }
    eprintln!("Byron contiguous: {total_txs} transactions decoded from 1500 blocks");
}

#[test]
fn shelley_contiguous_tx_decode() {
    let blocks_json = load_blocks_json("shelley");
    let blocks = blocks_json["blocks"].as_array().unwrap();
    let era_dir = corpus_root().join("shelley");

    let mut total_txs = 0;
    for block_entry in blocks {
        let filename = block_entry["file"].as_str().unwrap();
        let path = era_dir.join(filename);
        let raw = std::fs::read(&path).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        let preserved = shelley::decode_shelley_block(inner).unwrap();
        let block = preserved.decoded();

        if block.tx_count > 0 {
            let mut offset = 0;
            let tx_data = &block.tx_bodies;
            let enc = cbor::read_array_header(tx_data, &mut offset).unwrap();
            match enc {
                cbor::ContainerEncoding::Definite(n, _) => {
                    for _ in 0..n {
                        let _tx = shelley::tx::decode_shelley_tx_body(tx_data, &mut offset).unwrap();
                        total_txs += 1;
                    }
                }
                cbor::ContainerEncoding::Indefinite => {
                    while !cbor::is_break(tx_data, offset).unwrap() {
                        let _tx = shelley::tx::decode_shelley_tx_body(tx_data, &mut offset).unwrap();
                        total_txs += 1;
                    }
                }
            }
        }
    }
    eprintln!("Shelley contiguous: {total_txs} transactions decoded from 1500 blocks");
}

#[test]
fn allegra_contiguous_tx_decode() {
    let blocks_json = load_blocks_json("allegra");
    let blocks = blocks_json["blocks"].as_array().unwrap();
    let era_dir = corpus_root().join("allegra");

    let mut total_txs = 0;
    for block_entry in blocks {
        let filename = block_entry["file"].as_str().unwrap();
        let path = era_dir.join(filename);
        let raw = std::fs::read(&path).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        let preserved = allegra::decode_allegra_block(inner).unwrap();
        let block = preserved.decoded();

        if block.tx_count > 0 {
            let mut offset = 0;
            let tx_data = &block.tx_bodies;
            let enc = cbor::read_array_header(tx_data, &mut offset).unwrap();
            match enc {
                cbor::ContainerEncoding::Definite(n, _) => {
                    for _ in 0..n {
                        let _tx = allegra::tx::decode_allegra_tx_body(tx_data, &mut offset).unwrap();
                        total_txs += 1;
                    }
                }
                cbor::ContainerEncoding::Indefinite => {
                    while !cbor::is_break(tx_data, offset).unwrap() {
                        let _tx = allegra::tx::decode_allegra_tx_body(tx_data, &mut offset).unwrap();
                        total_txs += 1;
                    }
                }
            }
        }
    }
    eprintln!("Allegra contiguous: {total_txs} transactions decoded from 1500 blocks");
}

#[test]
fn mary_contiguous_tx_decode() {
    let blocks_json = load_blocks_json("mary");
    let blocks = blocks_json["blocks"].as_array().unwrap();
    let era_dir = corpus_root().join("mary");

    let mut total_txs = 0;
    for block_entry in blocks {
        let filename = block_entry["file"].as_str().unwrap();
        let path = era_dir.join(filename);
        let raw = std::fs::read(&path).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        let preserved = mary::decode_mary_block(inner).unwrap();
        let block = preserved.decoded();

        if block.tx_count > 0 {
            let mut offset = 0;
            let tx_data = &block.tx_bodies;
            let enc = cbor::read_array_header(tx_data, &mut offset).unwrap();
            match enc {
                cbor::ContainerEncoding::Definite(n, _) => {
                    for _ in 0..n {
                        let _tx = mary::tx::decode_mary_tx_body(tx_data, &mut offset).unwrap();
                        total_txs += 1;
                    }
                }
                cbor::ContainerEncoding::Indefinite => {
                    while !cbor::is_break(tx_data, offset).unwrap() {
                        let _tx = mary::tx::decode_mary_tx_body(tx_data, &mut offset).unwrap();
                        total_txs += 1;
                    }
                }
            }
        }
    }
    eprintln!("Mary contiguous: {total_txs} transactions decoded from 1500 blocks");
}
