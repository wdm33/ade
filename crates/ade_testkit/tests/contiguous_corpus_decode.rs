//! Integration test: Contiguous corpus block decoding.
//!
//! Verifies that all 10,500 contiguous blocks (Byron–Conway) decode successfully
//! through the HFC envelope and era-specific block decoders. This is a
//! prerequisite for differential comparison — blocks must parse before
//! ledger rules can be applied.

use std::path::PathBuf;

use ade_codec::cbor::envelope::decode_block_envelope;
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

fn load_state_hashes(era: &str) -> Vec<(u64, String)> {
    let path = corpus_root().join(format!("{era}_state_hashes.txt"));
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            let slot_str = parts[0].trim().strip_prefix("SlotNo ").unwrap_or(parts[0].trim());
            let slot: u64 = slot_str.parse().unwrap();
            let hash = parts[1].to_string();
            (slot, hash)
        })
        .collect()
}

fn expected_era(era_name: &str) -> &[CardanoEra] {
    match era_name {
        "byron" => &[CardanoEra::ByronEbb, CardanoEra::ByronRegular],
        "shelley" => &[CardanoEra::Shelley],
        "allegra" => &[CardanoEra::Allegra],
        "mary" => &[CardanoEra::Mary],
        "alonzo" => &[CardanoEra::Alonzo],
        "babbage" => &[CardanoEra::Babbage],
        "conway" => &[CardanoEra::Conway],
        _ => &[],
    }
}

/// Decode all contiguous blocks for a given era, verifying HFC envelope
/// and era-specific block decoding succeeds.
fn decode_era_blocks(era_name: &str) -> usize {
    let blocks_json = load_blocks_json(era_name);
    let blocks = blocks_json["blocks"].as_array().unwrap();
    let era_dir = corpus_root().join(era_name);
    let valid_eras = expected_era(era_name);

    let mut decoded = 0;
    for block_entry in blocks {
        let filename = block_entry["file"].as_str().unwrap();
        let path = era_dir.join(filename);
        let raw = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

        // Decode HFC envelope
        let env = decode_block_envelope(&raw)
            .unwrap_or_else(|e| panic!("envelope decode failed for {filename}: {e}"));

        assert!(
            valid_eras.contains(&env.era),
            "{filename}: unexpected era {:?}, expected one of {:?}",
            env.era,
            valid_eras
        );

        // Decode inner block based on era
        let inner = &raw[env.block_start..env.block_end];
        match env.era {
            CardanoEra::ByronEbb => {
                ade_codec::byron::decode_byron_ebb_block(inner)
                    .unwrap_or_else(|e| panic!("Byron EBB decode failed for {filename}: {e}"));
            }
            CardanoEra::ByronRegular => {
                ade_codec::byron::decode_byron_regular_block(inner)
                    .unwrap_or_else(|e| panic!("Byron regular decode failed for {filename}: {e}"));
            }
            CardanoEra::Shelley => {
                ade_codec::shelley::decode_shelley_block(inner)
                    .unwrap_or_else(|e| panic!("Shelley decode failed for {filename}: {e}"));
            }
            CardanoEra::Allegra => {
                ade_codec::allegra::decode_allegra_block(inner)
                    .unwrap_or_else(|e| panic!("Allegra decode failed for {filename}: {e}"));
            }
            CardanoEra::Mary => {
                ade_codec::mary::decode_mary_block(inner)
                    .unwrap_or_else(|e| panic!("Mary decode failed for {filename}: {e}"));
            }
            CardanoEra::Alonzo => {
                ade_codec::alonzo::decode_alonzo_block(inner)
                    .unwrap_or_else(|e| panic!("Alonzo decode failed for {filename}: {e}"));
            }
            CardanoEra::Babbage => {
                ade_codec::babbage::decode_babbage_block(inner)
                    .unwrap_or_else(|e| panic!("Babbage decode failed for {filename}: {e}"));
            }
            CardanoEra::Conway => {
                ade_codec::conway::decode_conway_block(inner)
                    .unwrap_or_else(|e| panic!("Conway decode failed for {filename}: {e}"));
            }
        }

        decoded += 1;
    }

    decoded
}

#[test]
fn byron_contiguous_blocks_decode() {
    let count = decode_era_blocks("byron");
    assert_eq!(count, 1500, "expected 1500 Byron blocks");
    eprintln!("Byron: {count} blocks decoded successfully");
}

#[test]
fn shelley_contiguous_blocks_decode() {
    let count = decode_era_blocks("shelley");
    assert_eq!(count, 1500, "expected 1500 Shelley blocks");
    eprintln!("Shelley: {count} blocks decoded successfully");
}

#[test]
fn allegra_contiguous_blocks_decode() {
    let count = decode_era_blocks("allegra");
    assert_eq!(count, 1500, "expected 1500 Allegra blocks");
    eprintln!("Allegra: {count} blocks decoded successfully");
}

#[test]
fn mary_contiguous_blocks_decode() {
    let count = decode_era_blocks("mary");
    assert_eq!(count, 1500, "expected 1500 Mary blocks");
    eprintln!("Mary: {count} blocks decoded successfully");
}

#[test]
fn alonzo_contiguous_blocks_decode() {
    let count = decode_era_blocks("alonzo");
    assert_eq!(count, 1500, "expected 1500 Alonzo blocks");
    eprintln!("Alonzo: {count} blocks decoded successfully");
}

#[test]
fn babbage_contiguous_blocks_decode() {
    let count = decode_era_blocks("babbage");
    assert_eq!(count, 1500, "expected 1500 Babbage blocks");
    eprintln!("Babbage: {count} blocks decoded successfully");
}

#[test]
fn conway_contiguous_blocks_decode() {
    let count = decode_era_blocks("conway");
    assert_eq!(count, 1500, "expected 1500 Conway blocks");
    eprintln!("Conway: {count} blocks decoded successfully");
}

#[test]
fn state_hash_files_have_correct_counts() {
    let byron_hashes = load_state_hashes("byron");
    assert_eq!(byron_hashes.len(), 1502, "Byron: expected 1502 hashes");

    let shelley_hashes = load_state_hashes("shelley");
    assert_eq!(shelley_hashes.len(), 1500, "Shelley: expected 1500 hashes");

    let allegra_hashes = load_state_hashes("allegra");
    assert_eq!(allegra_hashes.len(), 1500, "Allegra: expected 1500 hashes");

    let mary_hashes = load_state_hashes("mary");
    assert_eq!(mary_hashes.len(), 1500, "Mary: expected 1500 hashes");

    let alonzo_hashes = load_state_hashes("alonzo");
    assert_eq!(alonzo_hashes.len(), 1500, "Alonzo: expected 1500 hashes");

    let babbage_hashes = load_state_hashes("babbage");
    assert_eq!(babbage_hashes.len(), 1500, "Babbage: expected 1500 hashes");

    let conway_hashes = load_state_hashes("conway");
    assert_eq!(conway_hashes.len(), 1500, "Conway: expected 1500 hashes");
}

#[test]
fn state_hashes_are_valid_hex() {
    for era in &["byron", "shelley", "allegra", "mary", "alonzo", "babbage", "conway"] {
        let hashes = load_state_hashes(era);
        for (slot, hash) in &hashes {
            assert_eq!(
                hash.len(),
                64,
                "{era} slot {slot}: hash must be 64 hex chars, got {}",
                hash.len()
            );
            assert!(
                hash.chars().all(|c| c.is_ascii_hexdigit()),
                "{era} slot {slot}: hash contains non-hex characters"
            );
        }
    }
}

#[test]
fn oracle_manifest_exists_and_parses() {
    let path = corpus_root().join("oracle_manifest.toml");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read oracle_manifest.toml: {e}"));

    // Basic TOML parse
    let parsed: toml::Value = toml::from_str(&content)
        .unwrap_or_else(|e| panic!("oracle_manifest.toml parse failed: {e}"));

    // Verify required fields
    let oracle = parsed.get("oracle").unwrap();
    assert_eq!(
        oracle.get("cardano_node_version").unwrap().as_str().unwrap(),
        "10.6.2"
    );
    assert_eq!(
        oracle.get("comparison_surface").unwrap().as_str().unwrap(),
        "Blake2b-256 of encodeDiskExtLedgerState"
    );

    // Verify era entries
    let eras = parsed.get("eras").unwrap().as_array().unwrap();
    assert_eq!(eras.len(), 7, "expected 7 era entries");
}
