//! One-shot smoke for PHASE4-N-M-A S1 — feed a real preprod
//! cardano-cli UTxO sample through the seed importer + report
//! parsed-entry count and fingerprint.
//!
//! Usage:
//!   cargo run --release -p ade_runtime --example seed_smoke -- <path>

use ade_runtime::seed_import::import_cardano_cli_json_utxo;

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/utxo_sample_mini.json".to_string());
    let path = std::path::Path::new(&path);
    match import_cardano_cli_json_utxo(path) {
        Ok((state, fp)) => {
            println!("OK — parsed {} entries", state.utxos.len());
            println!(
                "fingerprint (Blake2b-256): {}",
                fp.0 .0
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>()
            );
        }
        Err(e) => {
            eprintln!("seed import failed: {:?}", e);
            std::process::exit(1);
        }
    }
}
