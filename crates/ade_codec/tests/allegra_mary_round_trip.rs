//! Integration test: Allegra and Mary block decode → re-encode byte-identity
//! and differential comparison against reference oracle data.

use std::path::PathBuf;

use ade_codec::allegra::decode_allegra_block;
use ade_codec::cbor::envelope::decode_block_envelope;
use ade_codec::mary::decode_mary_block;
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

const ALLEGRA_BLOCKS: &[(&str, &str)] = &[
    ("chunk00900_blk00000.cbor", "chunk00900_blk00000.json"),
    ("chunk00900_blk00545.cbor", "chunk00900_blk00545.json"),
    ("chunk00900_blk01090.cbor", "chunk00900_blk01090.json"),
];

const MARY_BLOCKS: &[(&str, &str)] = &[
    ("chunk01400_blk00000.cbor", "chunk01400_blk00000.json"),
    ("chunk01400_blk00539.cbor", "chunk01400_blk00539.json"),
    ("chunk01400_blk01077.cbor", "chunk01400_blk01077.json"),
];

#[test]
fn allegra_round_trip_byte_identical() {
    for (block_file, _) in ALLEGRA_BLOCKS {
        let raw = load_block("allegra", block_file);
        let env = decode_block_envelope(&raw).unwrap();
        assert_eq!(env.era, CardanoEra::Allegra);

        let inner = &raw[env.block_start..env.block_end];
        let preserved = decode_allegra_block(inner).unwrap();
        assert_eq!(preserved.wire_bytes(), inner);

        let ctx = CodecContext {
            era: CardanoEra::Allegra,
        };
        let mut re_encoded = Vec::new();
        preserved
            .decoded()
            .ade_encode(&mut re_encoded, &ctx)
            .unwrap();

        assert_eq!(
            re_encoded,
            inner,
            "Allegra block {block_file} re-encode mismatch. First diff at byte {}",
            first_diff(inner, &re_encoded),
        );
    }
}

#[test]
fn mary_round_trip_byte_identical() {
    for (block_file, _) in MARY_BLOCKS {
        let raw = load_block("mary", block_file);
        let env = decode_block_envelope(&raw).unwrap();
        assert_eq!(env.era, CardanoEra::Mary);

        let inner = &raw[env.block_start..env.block_end];
        let preserved = decode_mary_block(inner).unwrap();
        assert_eq!(preserved.wire_bytes(), inner);

        let ctx = CodecContext {
            era: CardanoEra::Mary,
        };
        let mut re_encoded = Vec::new();
        preserved
            .decoded()
            .ade_encode(&mut re_encoded, &ctx)
            .unwrap();

        assert_eq!(
            re_encoded,
            inner,
            "Mary block {block_file} re-encode mismatch. First diff at byte {}",
            first_diff(inner, &re_encoded),
        );
    }
}

#[test]
fn allegra_fields_match_reference() {
    for (block_file, ref_file) in ALLEGRA_BLOCKS {
        let raw = load_block("allegra", block_file);
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];
        let preserved = decode_allegra_block(inner).unwrap();
        let hb = &preserved.decoded().header.body;

        let reference = load_reference("allegra", ref_file);

        assert_eq!(reference["era"], "allegra", "{ref_file}");
        assert_eq!(reference["block_number"], hb.block_number, "{ref_file}");
        assert_eq!(reference["slot"], hb.slot, "{ref_file}");
        assert_eq!(
            reference["prev_block_hash"],
            hex_encode(&hb.prev_hash.0),
            "{ref_file}"
        );
        assert_eq!(
            reference["issuer_vkey_hash"],
            hex_encode(&hb.issuer_vkey),
            "{ref_file}"
        );
        assert_eq!(reference["hfc_era_tag"], 3, "{ref_file}");
        assert_eq!(reference["source_file_size"], raw.len(), "{ref_file}");
    }
}

#[test]
fn mary_fields_match_reference() {
    for (block_file, ref_file) in MARY_BLOCKS {
        let raw = load_block("mary", block_file);
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];
        let preserved = decode_mary_block(inner).unwrap();
        let hb = &preserved.decoded().header.body;

        let reference = load_reference("mary", ref_file);

        assert_eq!(reference["era"], "mary", "{ref_file}");
        assert_eq!(reference["block_number"], hb.block_number, "{ref_file}");
        assert_eq!(reference["slot"], hb.slot, "{ref_file}");
        assert_eq!(
            reference["prev_block_hash"],
            hex_encode(&hb.prev_hash.0),
            "{ref_file}"
        );
        assert_eq!(reference["hfc_era_tag"], 4, "{ref_file}");
        assert_eq!(reference["source_file_size"], raw.len(), "{ref_file}");
    }
}
