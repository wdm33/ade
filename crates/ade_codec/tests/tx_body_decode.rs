//! Integration test: Transaction body decode for Byron through Mary golden blocks.
//! S-04: Verifies wire-byte-preserving decode of tx bodies.

use std::path::PathBuf;

use ade_codec::allegra;
use ade_codec::byron;
use ade_codec::cbor;
use ade_codec::cbor::envelope::decode_block_envelope;
use ade_codec::mary;
use ade_types::CardanoEra;

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
}

fn load_block(era: &str, filename: &str) -> Vec<u8> {
    let path = corpus_root()
        .join("golden")
        .join(era)
        .join("blocks")
        .join(filename);
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

// ---------------------------------------------------------------------------
// Byron tx body decoding
// ---------------------------------------------------------------------------

#[test]
fn byron_regular_blocks_tx_body_decode() {
    // Byron regular blocks (era_tag 1) — block body is opaque in Phase 1
    // We need to decode the block body to get the tx_payload
    let regular_blocks = &[
        "chunk00000_blk10793.cbor",
        "chunk00000_blk21586.cbor",
    ];

    for block_file in regular_blocks {
        let raw = load_block("byron", block_file);
        let env = decode_block_envelope(&raw).unwrap();
        assert_eq!(env.era, CardanoEra::ByronRegular);

        let inner = &raw[env.block_start..env.block_end];
        let preserved = byron::decode_byron_regular_block(inner).unwrap();
        let block = preserved.decoded();

        // Decode transactions from the block body
        let txs = byron::tx::decode_byron_block_txs(&block.body).unwrap();

        // Early Byron blocks may have 0 transactions — that's valid
        // Just verify decoding succeeds without error
        eprintln!(
            "{block_file}: decoded {} Byron transactions",
            txs.len()
        );
    }
}

#[test]
fn byron_ebb_blocks_have_no_transactions() {
    let raw = load_block("byron", "chunk00000_blk00000.cbor");
    let env = decode_block_envelope(&raw).unwrap();
    assert_eq!(env.era, CardanoEra::ByronEbb);
    // EBBs contain no transactions — just verify decode succeeds
    let inner = &raw[env.block_start..env.block_end];
    let _preserved = byron::decode_byron_ebb_block(inner).unwrap();
}

// ---------------------------------------------------------------------------
// Allegra tx body decoding
// ---------------------------------------------------------------------------

const ALLEGRA_BLOCKS: &[&str] = &[
    "chunk00900_blk00000.cbor",
    "chunk00900_blk00545.cbor",
    "chunk00900_blk01090.cbor",
];

#[test]
fn allegra_tx_bodies_decode() {
    for block_file in ALLEGRA_BLOCKS {
        let raw = load_block("allegra", block_file);
        let env = decode_block_envelope(&raw).unwrap();
        assert_eq!(env.era, CardanoEra::Allegra);

        let inner = &raw[env.block_start..env.block_end];
        let preserved = allegra::decode_allegra_block(inner).unwrap();
        let block = preserved.decoded();

        // Decode individual tx bodies from the tx_bodies array
        let mut offset = 0;
        let tx_bodies_data = &block.tx_bodies;
        let enc = cbor::read_array_header(tx_bodies_data, &mut offset).unwrap();
        let tx_count = match enc {
            cbor::ContainerEncoding::Definite(n, _) => n,
            cbor::ContainerEncoding::Indefinite => {
                let mut count = 0u64;
                while !cbor::is_break(tx_bodies_data, offset).unwrap() {
                    let tx_start = offset;
                    let _tx = allegra::tx::decode_allegra_tx_body(tx_bodies_data, &mut offset).unwrap();
                    // Verify we consumed some bytes
                    assert!(offset > tx_start);
                    count += 1;
                }
                offset += 1; // break byte
                count
            }
        };

        if matches!(enc, cbor::ContainerEncoding::Definite(_, _)) {
            for i in 0..tx_count {
                let tx_start = offset;
                let tx = allegra::tx::decode_allegra_tx_body(tx_bodies_data, &mut offset).unwrap();
                assert!(offset > tx_start, "tx body {i} consumed no bytes");
                // Basic structural checks
                assert!(!tx.inputs.is_empty(), "tx {i} has no inputs");
                assert!(!tx.outputs.is_empty(), "tx {i} has no outputs");
                assert!(tx.fee.0 > 0, "tx {i} has zero fee");
            }
        }

        assert_eq!(
            tx_count, block.tx_count,
            "{block_file}: tx count mismatch"
        );

        eprintln!(
            "{block_file}: decoded {tx_count} Allegra tx bodies"
        );
    }
}

// ---------------------------------------------------------------------------
// Mary tx body decoding
// ---------------------------------------------------------------------------

const MARY_BLOCKS: &[&str] = &[
    "chunk01400_blk00000.cbor",
    "chunk01400_blk00539.cbor",
    "chunk01400_blk01077.cbor",
];

#[test]
fn mary_tx_bodies_decode() {
    for block_file in MARY_BLOCKS {
        let raw = load_block("mary", block_file);
        let env = decode_block_envelope(&raw).unwrap();
        assert_eq!(env.era, CardanoEra::Mary);

        let inner = &raw[env.block_start..env.block_end];
        let preserved = mary::decode_mary_block(inner).unwrap();
        let block = preserved.decoded();

        let mut offset = 0;
        let tx_bodies_data = &block.tx_bodies;
        let enc = cbor::read_array_header(tx_bodies_data, &mut offset).unwrap();
        let mut tx_count = 0u64;
        match enc {
            cbor::ContainerEncoding::Definite(n, _) => {
                for i in 0..n {
                    let tx_start = offset;
                    let tx = mary::tx::decode_mary_tx_body(tx_bodies_data, &mut offset).unwrap();
                    assert!(offset > tx_start, "Mary tx body {i} consumed no bytes");
                    assert!(!tx.inputs.is_empty(), "Mary tx {i} has no inputs");
                    assert!(!tx.outputs.is_empty(), "Mary tx {i} has no outputs");
                    assert!(tx.fee.0 > 0, "Mary tx {i} has zero fee");
                }
                tx_count = n;
            }
            cbor::ContainerEncoding::Indefinite => {
                while !cbor::is_break(tx_bodies_data, offset).unwrap() {
                    let tx_start = offset;
                    let tx = mary::tx::decode_mary_tx_body(tx_bodies_data, &mut offset).unwrap();
                    assert!(offset > tx_start, "Mary tx consumed no bytes");
                    assert!(!tx.inputs.is_empty(), "Mary tx has no inputs");
                    assert!(!tx.outputs.is_empty(), "Mary tx has no outputs");
                    assert!(tx.fee.0 > 0, "Mary tx has zero fee");
                    tx_count += 1;
                }
            }
        }

        assert_eq!(
            tx_count, block.tx_count,
            "{block_file}: tx count mismatch"
        );

        eprintln!(
            "{block_file}: decoded {tx_count} Mary tx bodies"
        );
    }
}
