//! Differential test: UTxO set equality between Ade and oracle.
//!
//! Extracts all UTxO entries from the oracle's ExtLedgerState dump
//! and compares them against Ade's genesis-loaded UTxO state.
//! This verifies semantic equivalence: same TxIn keys, same TxOut values.
//!
//! Sub-surface ladder rung 3: UTxO set equality.

use std::collections::BTreeMap;
use std::path::PathBuf;

use ade_codec::cbor;
use ade_ledger::utxo::TxOut;
use ade_testkit::harness::genesis_loader::load_genesis_utxo;
use ade_types::tx::TxIn;
use ade_types::Hash32;

fn dumps_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("ext_ledger_state_dumps")
}

/// Extract UTxO entries from a raw ExtLedgerState dump.
///
/// Returns a BTreeMap<TxIn, (Vec<u8>, u64)> where the value is (address_bytes, coin).
fn extract_oracle_utxo(dump_path: &std::path::Path) -> BTreeMap<TxIn, (Vec<u8>, u64)> {
    let data = std::fs::read(dump_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", dump_path.display()));

    let mut off = 0;

    // Top: array(2)
    let _ = cbor::read_array_header(&data, &mut off).unwrap();
    // LedgerState telescope: array(1)
    let _ = cbor::read_array_header(&data, &mut off).unwrap();
    // Current: array(2)
    let _ = cbor::read_array_header(&data, &mut off).unwrap();
    // Bound: array(3) — skip
    let _ = cbor::skip_item(&data, &mut off).unwrap();
    // ByronLedgerState: array(3)
    let _ = cbor::read_array_header(&data, &mut off).unwrap();
    // [0] WithOrigin tipBlockNo — skip
    let _ = cbor::skip_item(&data, &mut off).unwrap();
    // [1] ChainState: array(5)
    let _ = cbor::read_array_header(&data, &mut off).unwrap();
    // [0] version — skip
    let _ = cbor::skip_item(&data, &mut off).unwrap();
    // [1] slot_info — skip
    let _ = cbor::skip_item(&data, &mut off).unwrap();
    // [2] UTxO map
    let map_enc = cbor::read_map_header(&data, &mut off).unwrap();
    let count = match map_enc {
        cbor::ContainerEncoding::Definite(n, _) => n,
        _ => panic!("expected definite map"),
    };

    let mut result = BTreeMap::new();
    for _ in 0..count {
        // Key: array(2) [array(4) [u64,u64,u64,u64], uint]
        let _ = cbor::read_array_header(&data, &mut off).unwrap(); // array(2)
        let _ = cbor::read_array_header(&data, &mut off).unwrap(); // array(4)

        let mut hash_bytes = [0u8; 32];
        for chunk in 0..4 {
            let (val, _) = cbor::read_uint(&data, &mut off).unwrap();
            hash_bytes[chunk * 8..(chunk + 1) * 8].copy_from_slice(&val.to_be_bytes());
        }
        let tx_hash = Hash32(hash_bytes);
        let (index, _) = cbor::read_uint(&data, &mut off).unwrap();

        // Value: array(2) [bytes(address), uint(coin)]
        let _ = cbor::read_array_header(&data, &mut off).unwrap(); // array(2)
        let (addr_bytes, _) = cbor::read_bytes(&data, &mut off).unwrap();
        let (coin, _) = cbor::read_uint(&data, &mut off).unwrap();

        let tx_in = TxIn {
            tx_hash,
            index: index as u16,
        };
        result.insert(tx_in, (addr_bytes, coin));
    }

    result
}

#[test]
fn genesis_utxo_set_equality() {
    let dump_path = dumps_root().join("byron").join("slot_0.bin");
    let oracle_utxo = extract_oracle_utxo(&dump_path);

    let ade_state = load_genesis_utxo(&dump_path).unwrap();

    assert_eq!(
        oracle_utxo.len(),
        ade_state.len(),
        "UTxO count mismatch: oracle={}, ade={}",
        oracle_utxo.len(),
        ade_state.len()
    );

    let mut mismatches = 0;
    let mut first_mismatch: Option<String> = None;

    for (tx_in, (oracle_addr, oracle_coin)) in &oracle_utxo {
        match ade_state.utxos.get(tx_in) {
            None => {
                mismatches += 1;
                if first_mismatch.is_none() {
                    first_mismatch = Some(format!(
                        "TxIn {:?}#{} missing from Ade state",
                        tx_in.tx_hash, tx_in.index
                    ));
                }
            }
            Some(tx_out) => {
                let (ade_addr, ade_coin) = match tx_out {
                    TxOut::Byron { address, coin } => (address.as_bytes().to_vec(), coin.0),
                    TxOut::ShelleyMary { .. } => {
                        mismatches += 1;
                        if first_mismatch.is_none() {
                            first_mismatch = Some(format!(
                                "TxIn {:?}#{}: expected Byron TxOut, got ShelleyMary",
                                tx_in.tx_hash, tx_in.index
                            ));
                        }
                        continue;
                    }
                };

                if ade_coin != *oracle_coin {
                    mismatches += 1;
                    if first_mismatch.is_none() {
                        first_mismatch = Some(format!(
                            "TxIn {:?}#{}: coin mismatch oracle={} ade={}",
                            tx_in.tx_hash, tx_in.index, oracle_coin, ade_coin
                        ));
                    }
                }

                if ade_addr != *oracle_addr {
                    mismatches += 1;
                    if first_mismatch.is_none() {
                        first_mismatch = Some(format!(
                            "TxIn {:?}#{}: address mismatch (oracle {} bytes, ade {} bytes)",
                            tx_in.tx_hash, tx_in.index, oracle_addr.len(), ade_addr.len()
                        ));
                    }
                }
            }
        }
    }

    // Also check for entries in Ade not in oracle
    for tx_in in ade_state.utxos.keys() {
        if !oracle_utxo.contains_key(tx_in) {
            mismatches += 1;
            if first_mismatch.is_none() {
                first_mismatch = Some(format!(
                    "TxIn {:?}#{} in Ade but not in oracle",
                    tx_in.tx_hash, tx_in.index
                ));
            }
        }
    }

    eprintln!(
        "UTxO set equality: {}/{} entries match, {} mismatches",
        oracle_utxo.len() - mismatches,
        oracle_utxo.len(),
        mismatches
    );

    if let Some(ref m) = first_mismatch {
        eprintln!("First mismatch: {m}");
    }

    assert_eq!(
        mismatches, 0,
        "UTxO set divergence: {} mismatches. First: {:?}",
        mismatches, first_mismatch
    );
}

#[test]
fn post_block_1_utxo_matches_oracle() {
    // Load genesis, apply block 0, compare against slot_1 dump
    let genesis_path = dumps_root().join("byron").join("slot_0.bin");
    let ade_state = load_genesis_utxo(&genesis_path).unwrap();

    let slot1_path = dumps_root().join("byron").join("slot_1.bin");
    let oracle_utxo_post = extract_oracle_utxo(&slot1_path);

    // Apply the first Byron block
    let corpus = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("contiguous")
        .join("byron");

    let blocks_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(corpus.join("blocks.json")).unwrap(),
    )
    .unwrap();
    let first_block_file = blocks_json["blocks"][0]["file"].as_str().unwrap();
    let raw = std::fs::read(corpus.join(first_block_file)).unwrap();
    let env = ade_codec::cbor::envelope::decode_block_envelope(&raw).unwrap();
    let inner = &raw[env.block_start..env.block_end];

    let mut state = ade_ledger::state::LedgerState {
        utxo_state: ade_state,
        epoch_state: ade_ledger::state::EpochState::new(),
        protocol_params: ade_ledger::pparams::ProtocolParameters::default(),
        era: ade_types::CardanoEra::ByronRegular,
        track_utxo: false,
        cert_state: ade_ledger::delegation::CertState::new(),
    };

    state = ade_ledger::rules::apply_block(&state, env.era, inner).unwrap();

    // Compare UTxO counts
    assert_eq!(
        state.utxo_state.len(),
        oracle_utxo_post.len(),
        "Post-block-1 UTxO count mismatch: ade={}, oracle={}",
        state.utxo_state.len(),
        oracle_utxo_post.len()
    );

    // Compare entries
    let mut mismatches = 0;
    for (tx_in, (_oracle_addr, oracle_coin)) in &oracle_utxo_post {
        match state.utxo_state.utxos.get(tx_in) {
            None => mismatches += 1,
            Some(tx_out) => {
                if tx_out.coin().0 != *oracle_coin {
                    mismatches += 1;
                }
            }
        }
    }

    eprintln!(
        "Post-block-1 UTxO: {}/{} entries match",
        oracle_utxo_post.len() - mismatches,
        oracle_utxo_post.len()
    );

    assert_eq!(mismatches, 0, "Post-block-1 UTxO divergence: {mismatches} mismatches");
}
