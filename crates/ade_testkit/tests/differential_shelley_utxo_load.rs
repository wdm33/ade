//! Differential test: Shelley UTxO oracle load.
//!
//! Loads the Shelley ExtLedgerState binary dump and verifies the UTxO set
//! matches known-good reference values from cardano-node.
//!
//! Verifies:
//! 1. Exactly 84,609 UTxO entries
//! 2. All entries have non-zero coin
//! 3. Total lovelace matches expected value

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use ade_testkit::harness::shelley_loader::load_shelley_oracle_utxo;

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
}

fn shelley_dump_path() -> PathBuf {
    corpus_root()
        .join("ext_ledger_state_dumps")
        .join("shelley")
        .join("slot_10800019.bin")
}

#[test]
fn shelley_utxo_has_84609_entries() {
    let path = shelley_dump_path();
    if !path.exists() {
        eprintln!(
            "SKIP: Shelley dump not found at {}",
            path.display()
        );
        return;
    }

    let utxo = load_shelley_oracle_utxo(&path).unwrap();
    let count = utxo.len();

    eprintln!("Shelley UTxO loaded: {count} entries");
    assert_eq!(count, 84_609, "expected 84,609 Shelley UTxO entries, got {count}");
}

#[test]
fn shelley_utxo_total_lovelace() {
    let path = shelley_dump_path();
    if !path.exists() {
        eprintln!(
            "SKIP: Shelley dump not found at {}",
            path.display()
        );
        return;
    }

    let utxo = load_shelley_oracle_utxo(&path).unwrap();
    let total = utxo.total_lovelace();

    eprintln!("Total lovelace: {total}");
    // Every entry has exactly 2,000,000 lovelace (deposit amount)
    // 84,609 * 2,000,000 = 169,218,000,000
    assert_eq!(
        total, 169_218_000_000,
        "total lovelace mismatch: expected 169,218,000,000, got {total}"
    );
}

#[test]
fn shelley_utxo_all_nonzero_coin() {
    let path = shelley_dump_path();
    if !path.exists() {
        eprintln!(
            "SKIP: Shelley dump not found at {}",
            path.display()
        );
        return;
    }

    let utxo = load_shelley_oracle_utxo(&path).unwrap();

    let mut zero_count = 0u64;
    for entry in utxo.entries.values() {
        if entry.coin == 0 {
            zero_count += 1;
        }
    }

    assert_eq!(
        zero_count, 0,
        "{zero_count} UTxO entries have zero coin"
    );

    eprintln!("All {} entries have non-zero coin", utxo.len());
}
