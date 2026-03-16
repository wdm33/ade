//! Integration test: Shelley block decode → re-encode byte-identity and
//! differential comparison against reference oracle data.

use std::path::PathBuf;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_codec::shelley::decode_shelley_block;
use ade_codec::traits::AdeEncode;
use ade_codec::CodecContext;
use ade_types::CardanoEra;

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
}

fn load_block(filename: &str) -> Vec<u8> {
    let path = corpus_root()
        .join("golden")
        .join("shelley")
        .join("blocks")
        .join(filename);
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn load_reference(filename: &str) -> serde_json::Value {
    let path = corpus_root()
        .join("reference")
        .join("block_fields")
        .join("shelley")
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

const SHELLEY_BLOCKS: &[(&str, &str)] = &[
    ("chunk00500_blk00000.cbor", "chunk00500_blk00000.json"),
    ("chunk00500_blk00539.cbor", "chunk00500_blk00539.json"),
    ("chunk00500_blk01077.cbor", "chunk00500_blk01077.json"),
];

#[test]
fn shelley_round_trip_byte_identical() {
    for (block_file, _) in SHELLEY_BLOCKS {
        let raw = load_block(block_file);
        let env = decode_block_envelope(&raw).unwrap();
        assert_eq!(env.era, CardanoEra::Shelley);

        let inner = &raw[env.block_start..env.block_end];
        let preserved = decode_shelley_block(inner).unwrap();
        assert_eq!(preserved.wire_bytes(), inner);

        let ctx = CodecContext {
            era: CardanoEra::Shelley,
        };
        let mut re_encoded = Vec::new();
        preserved
            .decoded()
            .ade_encode(&mut re_encoded, &ctx)
            .unwrap();

        assert_eq!(
            re_encoded,
            inner,
            "Shelley block {block_file} re-encode mismatch.\n\
             Original length: {}, re-encoded length: {}\n\
             First divergence at byte {}",
            inner.len(),
            re_encoded.len(),
            first_diff(inner, &re_encoded),
        );
    }
}

#[test]
fn shelley_fields_match_reference() {
    for (block_file, ref_file) in SHELLEY_BLOCKS {
        let raw = load_block(block_file);
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];
        let preserved = decode_shelley_block(inner).unwrap();
        let hb = &preserved.decoded().header.body;
        let oc = &hb.operational_cert;
        let pv = &hb.protocol_version;

        let reference = load_reference(ref_file);

        assert_eq!(reference["era"], "shelley", "{ref_file}");
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
        assert_eq!(
            reference["vrf_vkey"],
            hex_encode(&hb.vrf_vkey),
            "{ref_file}"
        );
        assert_eq!(reference["body_size"], hb.body_size, "{ref_file}");
        assert_eq!(
            reference["body_hash"],
            hex_encode(&hb.body_hash.0),
            "{ref_file}"
        );
        assert_eq!(
            reference["operational_cert"]["hot_vkey"],
            hex_encode(&oc.hot_vkey),
            "{ref_file}"
        );
        assert_eq!(
            reference["operational_cert"]["sequence_number"], oc.sequence_number,
            "{ref_file}"
        );
        assert_eq!(
            reference["operational_cert"]["kes_period"], oc.kes_period,
            "{ref_file}"
        );
        assert_eq!(
            reference["protocol_version"]["major"], pv.major,
            "{ref_file}"
        );
        assert_eq!(
            reference["protocol_version"]["minor"], pv.minor,
            "{ref_file}"
        );
        assert_eq!(
            reference["tx_count"],
            preserved.decoded().tx_count,
            "{ref_file}"
        );
        assert_eq!(reference["hfc_era_tag"], 2, "{ref_file}");
        assert_eq!(reference["source_file_size"], raw.len(), "{ref_file}");
    }
}

#[test]
fn malformed_shelley_produces_error() {
    assert!(decode_shelley_block(&[]).is_err());
    assert!(decode_shelley_block(&[0x84, 0x82]).is_err());
    assert!(decode_shelley_block(&[0xa1]).is_err());
}
