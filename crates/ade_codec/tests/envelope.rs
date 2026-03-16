//! Integration test: decode_block_envelope on all 42 golden corpus blocks.
//!
//! Proof obligation: all 42 corpus blocks produce the correct CardanoEra
//! via decode_block_envelope().

use std::path::PathBuf;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_types::CardanoEra;

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("golden")
}

/// Load a golden CBOR block file and return its bytes.
fn load_block(era_dir: &str, filename: &str) -> Vec<u8> {
    let path = corpus_root().join(era_dir).join("blocks").join(filename);
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Parse the manifest to get expected era tags for each block.
fn parse_manifest(era_dir: &str) -> Vec<(String, u8)> {
    let path = corpus_root().join(era_dir).join("manifest.toml");
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read manifest: {e}"));
    let table: toml::Value = content.parse().unwrap();
    let fixtures = table["fixtures"].as_array().unwrap();
    fixtures
        .iter()
        .map(|f| {
            let file = f["file"].as_str().unwrap();
            let filename = file.strip_prefix("blocks/").unwrap_or(file);
            let era_tag = f["era_tag"].as_integer().unwrap() as u8;
            (filename.to_string(), era_tag)
        })
        .collect()
}

#[test]
fn envelope_dispatch_byron() {
    for (filename, expected_tag) in parse_manifest("byron") {
        let data = load_block("byron", &filename);
        let env = decode_block_envelope(&data)
            .unwrap_or_else(|e| panic!("envelope decode failed for byron/{filename}: {e}"));
        assert_eq!(
            env.era.as_u8(),
            expected_tag,
            "era mismatch for byron/{filename}: expected tag {expected_tag}, got {:?}",
            env.era
        );
        let expected_era = CardanoEra::try_from(expected_tag).unwrap();
        assert_eq!(env.era, expected_era);
        // Block body span is non-empty
        assert!(env.block_end > env.block_start);
    }
}

#[test]
fn envelope_dispatch_shelley() {
    for (filename, expected_tag) in parse_manifest("shelley") {
        let data = load_block("shelley", &filename);
        let env = decode_block_envelope(&data)
            .unwrap_or_else(|e| panic!("envelope decode failed for shelley/{filename}: {e}"));
        assert_eq!(env.era, CardanoEra::Shelley);
        assert_eq!(expected_tag, 2);
    }
}

#[test]
fn envelope_dispatch_allegra() {
    for (filename, expected_tag) in parse_manifest("allegra") {
        let data = load_block("allegra", &filename);
        let env = decode_block_envelope(&data)
            .unwrap_or_else(|e| panic!("envelope decode failed for allegra/{filename}: {e}"));
        assert_eq!(env.era, CardanoEra::Allegra);
        assert_eq!(expected_tag, 3);
    }
}

#[test]
fn envelope_dispatch_mary() {
    for (filename, expected_tag) in parse_manifest("mary") {
        let data = load_block("mary", &filename);
        let env = decode_block_envelope(&data)
            .unwrap_or_else(|e| panic!("envelope decode failed for mary/{filename}: {e}"));
        assert_eq!(env.era, CardanoEra::Mary);
        assert_eq!(expected_tag, 4);
    }
}

#[test]
fn envelope_dispatch_alonzo() {
    for (filename, expected_tag) in parse_manifest("alonzo") {
        let data = load_block("alonzo", &filename);
        let env = decode_block_envelope(&data)
            .unwrap_or_else(|e| panic!("envelope decode failed for alonzo/{filename}: {e}"));
        assert_eq!(env.era, CardanoEra::Alonzo);
        assert_eq!(expected_tag, 5);
    }
}

#[test]
fn envelope_dispatch_babbage() {
    for (filename, expected_tag) in parse_manifest("babbage") {
        let data = load_block("babbage", &filename);
        let env = decode_block_envelope(&data)
            .unwrap_or_else(|e| panic!("envelope decode failed for babbage/{filename}: {e}"));
        assert_eq!(env.era, CardanoEra::Babbage);
        assert_eq!(expected_tag, 6);
    }
}

#[test]
fn envelope_dispatch_conway() {
    for (filename, expected_tag) in parse_manifest("conway") {
        let data = load_block("conway", &filename);
        let env = decode_block_envelope(&data)
            .unwrap_or_else(|e| panic!("envelope decode failed for conway/{filename}: {e}"));
        assert_eq!(env.era, CardanoEra::Conway);
        assert_eq!(expected_tag, 7);
    }
}

#[test]
fn all_42_corpus_blocks_dispatch_correctly() {
    let eras = [
        "byron", "shelley", "allegra", "mary", "alonzo", "babbage", "conway",
    ];
    let mut total = 0;
    for era_dir in &eras {
        for (filename, expected_tag) in parse_manifest(era_dir) {
            let data = load_block(era_dir, &filename);
            let env = decode_block_envelope(&data)
                .unwrap_or_else(|e| panic!("envelope decode failed for {era_dir}/{filename}: {e}"));
            assert_eq!(
                env.era.as_u8(),
                expected_tag,
                "era mismatch for {era_dir}/{filename}"
            );
            total += 1;
        }
    }
    assert_eq!(total, 42, "expected 42 corpus blocks, got {total}");
}
