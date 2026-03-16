//! Integration test: Byron block decode → re-encode byte-identity and
//! differential comparison against reference oracle data.

use std::path::PathBuf;

use ade_codec::byron::{decode_byron_ebb_block, decode_byron_regular_block};
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

fn load_block(filename: &str) -> Vec<u8> {
    let path = corpus_root()
        .join("golden")
        .join("byron")
        .join("blocks")
        .join(filename);
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn load_reference(filename: &str) -> serde_json::Value {
    let path = corpus_root()
        .join("reference")
        .join("block_fields")
        .join("byron")
        .join(filename);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&content).unwrap()
}

/// Test byte-identical round-trip: decode inner block, re-encode, compare.
#[test]
fn ebb_round_trip_byte_identical() {
    let raw = load_block("chunk00000_blk00000.cbor");
    let env = decode_block_envelope(&raw).unwrap();
    assert_eq!(env.era, CardanoEra::ByronEbb);

    let inner = &raw[env.block_start..env.block_end];
    let preserved = decode_byron_ebb_block(inner).unwrap();

    // Wire bytes should be the original inner bytes
    assert_eq!(preserved.wire_bytes(), inner);

    // Re-encode from decoded structure
    let ctx = CodecContext {
        era: CardanoEra::ByronEbb,
    };
    let mut re_encoded = Vec::new();
    preserved
        .decoded()
        .ade_encode(&mut re_encoded, &ctx)
        .unwrap();

    assert_eq!(
        re_encoded,
        inner,
        "EBB block re-encode does not match original wire bytes.\n\
         Original length: {}, re-encoded length: {}\n\
         First divergence at byte {}",
        inner.len(),
        re_encoded.len(),
        first_diff(inner, &re_encoded),
    );
}

#[test]
fn regular_round_trip_byte_identical() {
    for filename in &["chunk00000_blk10793.cbor", "chunk00000_blk21586.cbor"] {
        let raw = load_block(filename);
        let env = decode_block_envelope(&raw).unwrap();
        assert_eq!(env.era, CardanoEra::ByronRegular);

        let inner = &raw[env.block_start..env.block_end];
        let preserved = decode_byron_regular_block(inner).unwrap();

        assert_eq!(preserved.wire_bytes(), inner);

        let ctx = CodecContext {
            era: CardanoEra::ByronRegular,
        };
        let mut re_encoded = Vec::new();
        preserved
            .decoded()
            .ade_encode(&mut re_encoded, &ctx)
            .unwrap();

        assert_eq!(
            re_encoded,
            inner,
            "Regular block {filename} re-encode mismatch.\n\
             Original length: {}, re-encoded length: {}\n\
             First divergence at byte {}",
            inner.len(),
            re_encoded.len(),
            first_diff(inner, &re_encoded),
        );
    }
}

/// Verify that decoded EBB fields match the reference oracle JSON.
#[test]
fn ebb_fields_match_reference() {
    let raw = load_block("chunk00000_blk00000.cbor");
    let env = decode_block_envelope(&raw).unwrap();
    let inner = &raw[env.block_start..env.block_end];
    let preserved = decode_byron_ebb_block(inner).unwrap();
    let h = &preserved.decoded().header;

    let reference = load_reference("chunk00000_blk00000.json");

    assert_eq!(reference["era"], "byron_ebb");
    assert_eq!(reference["protocol_magic"], h.protocol_magic);
    assert_eq!(reference["prev_block_hash"], hex_encode(&h.prev_hash.0));
    assert_eq!(reference["body_proof"], hex_encode(&h.body_proof.0));
    assert_eq!(reference["epoch"], h.epoch);
    assert_eq!(reference["slot"], serde_json::Value::Null);
    assert_eq!(reference["block_number"], serde_json::Value::Null);
    assert_eq!(reference["hfc_era_tag"], 0);
    assert_eq!(reference["source_file_size"], raw.len());
}

/// Verify that decoded regular block fields match the reference oracle JSON.
#[test]
fn regular_fields_match_reference() {
    let cases = [
        ("chunk00000_blk10793.cbor", "chunk00000_blk10793.json"),
        ("chunk00000_blk21586.cbor", "chunk00000_blk21586.json"),
    ];

    for (block_file, ref_file) in &cases {
        let raw = load_block(block_file);
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];
        let preserved = decode_byron_regular_block(inner).unwrap();
        let h = &preserved.decoded().header;
        let cd = &h.consensus_data;

        let reference = load_reference(ref_file);

        assert_eq!(reference["era"], "byron", "era mismatch in {ref_file}");
        assert_eq!(reference["protocol_magic"], h.protocol_magic);
        assert_eq!(
            reference["prev_block_hash"],
            hex_encode(&h.prev_hash.0),
            "prev_hash mismatch in {ref_file}"
        );
        assert_eq!(reference["epoch"], cd.epoch, "epoch mismatch in {ref_file}");
        assert_eq!(
            reference["slot_in_epoch"], cd.slot_in_epoch,
            "slot_in_epoch mismatch in {ref_file}"
        );
        assert_eq!(
            reference["chain_difficulty"], cd.chain_difficulty,
            "chain_difficulty mismatch in {ref_file}"
        );
        assert_eq!(
            reference["delegator_pubkey"],
            hex_encode(&cd.delegator_pubkey),
            "delegator_pubkey mismatch in {ref_file}"
        );
        assert_eq!(reference["slot"], serde_json::Value::Null);
        assert_eq!(reference["block_number"], serde_json::Value::Null);
        assert_eq!(reference["hfc_era_tag"], 1);
        assert_eq!(reference["source_file_size"], raw.len());
    }
}

/// Negative corpus: malformed Byron CBOR produces CodecError, not panic.
#[test]
fn malformed_ebb_produces_error() {
    // Truncated block
    let result = decode_byron_ebb_block(&[0x83, 0x85]);
    assert!(result.is_err());

    // Wrong outer structure (map instead of array)
    let result = decode_byron_ebb_block(&[0xa3]);
    assert!(result.is_err());

    // Empty input
    let result = decode_byron_ebb_block(&[]);
    assert!(result.is_err());
}

#[test]
fn malformed_regular_produces_error() {
    let result = decode_byron_regular_block(&[0x83, 0x85]);
    assert!(result.is_err());

    let result = decode_byron_regular_block(&[]);
    assert!(result.is_err());
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
