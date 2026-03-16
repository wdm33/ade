//! Integration test: byte-identical round-trip and differential comparison
//! for ALL 42 golden corpus blocks across all 7 eras.

use std::path::PathBuf;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_codec::traits::AdeEncode;
use ade_codec::CodecContext;
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

fn load_reference(era: &str, filename: &str) -> serde_json::Value {
    let path = corpus_root()
        .join("reference")
        .join("block_fields")
        .join(era)
        .join(filename);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&content).unwrap()
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{byte:02x}");
    }
    s
}

fn first_diff(a: &[u8], b: &[u8]) -> usize {
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if x != y {
            return i;
        }
    }
    a.len().min(b.len())
}

fn parse_manifest(era: &str) -> Vec<(String, String)> {
    let path = corpus_root().join("golden").join(era).join("manifest.toml");
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read manifest: {e}"));
    let table: toml::Value = content.parse().unwrap();
    let fixtures = table["fixtures"].as_array().unwrap();
    fixtures
        .iter()
        .map(|f| {
            let file = f["file"].as_str().unwrap();
            let cbor = file.strip_prefix("blocks/").unwrap_or(file).to_string();
            let json = cbor.replace(".cbor", ".json");
            (cbor, json)
        })
        .collect()
}

/// Decode a block from any era, re-encode, and verify byte identity.
fn round_trip_block(era_name: &str, cbor_file: &str) {
    let raw = load_block(era_name, cbor_file);
    let env = decode_block_envelope(&raw)
        .unwrap_or_else(|e| panic!("envelope decode failed for {era_name}/{cbor_file}: {e}"));

    let inner = &raw[env.block_start..env.block_end];
    let ctx = CodecContext { era: env.era };

    let mut re_encoded = Vec::new();

    match env.era {
        CardanoEra::ByronEbb => {
            let p = ade_codec::byron::decode_byron_ebb_block(inner).unwrap();
            p.decoded().ade_encode(&mut re_encoded, &ctx).unwrap();
        }
        CardanoEra::ByronRegular => {
            let p = ade_codec::byron::decode_byron_regular_block(inner).unwrap();
            p.decoded().ade_encode(&mut re_encoded, &ctx).unwrap();
        }
        CardanoEra::Shelley => {
            let p = ade_codec::shelley::decode_shelley_block(inner).unwrap();
            p.decoded().ade_encode(&mut re_encoded, &ctx).unwrap();
        }
        CardanoEra::Allegra => {
            let p = ade_codec::allegra::decode_allegra_block(inner).unwrap();
            p.decoded().ade_encode(&mut re_encoded, &ctx).unwrap();
        }
        CardanoEra::Mary => {
            let p = ade_codec::mary::decode_mary_block(inner).unwrap();
            p.decoded().ade_encode(&mut re_encoded, &ctx).unwrap();
        }
        CardanoEra::Alonzo => {
            let p = ade_codec::alonzo::decode_alonzo_block(inner).unwrap();
            p.decoded().ade_encode(&mut re_encoded, &ctx).unwrap();
        }
        CardanoEra::Babbage => {
            let p = ade_codec::babbage::decode_babbage_block(inner).unwrap();
            p.decoded().ade_encode(&mut re_encoded, &ctx).unwrap();
        }
        CardanoEra::Conway => {
            let p = ade_codec::conway::decode_conway_block(inner).unwrap();
            p.decoded().ade_encode(&mut re_encoded, &ctx).unwrap();
        }
    }

    assert_eq!(
        re_encoded,
        inner,
        "{era_name}/{cbor_file}: re-encode mismatch.\n\
         Original length: {}, re-encoded length: {}\n\
         First divergence at byte {}",
        inner.len(),
        re_encoded.len(),
        first_diff(inner, &re_encoded),
    );
}

/// Verify decoded fields match reference oracle JSON.
fn check_fields(era_name: &str, cbor_file: &str, ref_file: &str) {
    let raw = load_block(era_name, cbor_file);
    let env = decode_block_envelope(&raw).unwrap();
    let inner = &raw[env.block_start..env.block_end];

    let reference = load_reference(era_name, ref_file);

    // Byron uses different field extraction
    if env.era.is_byron() {
        check_byron_fields(&raw, inner, env.era, &reference);
        return;
    }

    // Post-Byron: use common header decoder
    let mut offset = 0;
    let block = ade_codec::shelley::block::decode_shelley_block_inner(inner, &mut offset).unwrap();
    let hb = &block.header.body;

    assert_eq!(
        reference["block_number"], hb.block_number,
        "{era_name}/{ref_file}"
    );
    assert_eq!(reference["slot"], hb.slot, "{era_name}/{ref_file}");
    assert_eq!(
        reference["prev_block_hash"],
        hex_encode(&hb.prev_hash.0),
        "{era_name}/{ref_file}"
    );
    assert_eq!(
        reference["issuer_vkey_hash"],
        hex_encode(&hb.issuer_vkey),
        "{era_name}/{ref_file}"
    );
    assert_eq!(
        reference["body_size"], hb.body_size,
        "{era_name}/{ref_file}"
    );
    assert_eq!(
        reference["body_hash"],
        hex_encode(&hb.body_hash.0),
        "{era_name}/{ref_file}"
    );
    assert_eq!(
        reference["tx_count"], block.tx_count,
        "{era_name}/{ref_file}"
    );
    assert_eq!(
        reference["protocol_version"]["major"], hb.protocol_version.major,
        "{era_name}/{ref_file}"
    );
    assert_eq!(
        reference["protocol_version"]["minor"], hb.protocol_version.minor,
        "{era_name}/{ref_file}"
    );
    assert_eq!(
        reference["source_file_size"],
        raw.len(),
        "{era_name}/{ref_file}"
    );
}

fn check_byron_fields(raw: &[u8], inner: &[u8], era: CardanoEra, reference: &serde_json::Value) {
    match era {
        CardanoEra::ByronEbb => {
            let p = ade_codec::byron::decode_byron_ebb_block(inner).unwrap();
            let h = &p.decoded().header;
            assert_eq!(reference["protocol_magic"], h.protocol_magic);
            assert_eq!(reference["prev_block_hash"], hex_encode(&h.prev_hash.0));
            assert_eq!(reference["epoch"], h.epoch);
            assert_eq!(reference["source_file_size"], raw.len());
        }
        CardanoEra::ByronRegular => {
            let p = ade_codec::byron::decode_byron_regular_block(inner).unwrap();
            let h = &p.decoded().header;
            let cd = &h.consensus_data;
            assert_eq!(reference["protocol_magic"], h.protocol_magic);
            assert_eq!(reference["prev_block_hash"], hex_encode(&h.prev_hash.0));
            assert_eq!(reference["epoch"], cd.epoch);
            assert_eq!(reference["slot_in_epoch"], cd.slot_in_epoch);
            assert_eq!(reference["chain_difficulty"], cd.chain_difficulty);
            assert_eq!(reference["source_file_size"], raw.len());
        }
        _ => {}
    }
}

/// Full corpus round-trip: all 42 blocks across all 7 eras.
#[test]
fn all_42_blocks_round_trip_byte_identical() {
    let eras = [
        "byron", "shelley", "allegra", "mary", "alonzo", "babbage", "conway",
    ];
    let mut total = 0;
    for era in &eras {
        for (cbor, _) in parse_manifest(era) {
            round_trip_block(era, &cbor);
            total += 1;
        }
    }
    assert_eq!(total, 42, "expected 42 corpus blocks, got {total}");
}

/// Full corpus differential: all 42 blocks match reference oracle.
#[test]
fn all_42_blocks_fields_match_reference() {
    let eras = [
        "byron", "shelley", "allegra", "mary", "alonzo", "babbage", "conway",
    ];
    let mut total = 0;
    for era in &eras {
        for (cbor, json) in parse_manifest(era) {
            check_fields(era, &cbor, &json);
            total += 1;
        }
    }
    assert_eq!(total, 42, "expected 42 corpus blocks, got {total}");
}
