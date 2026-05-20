//! Integration test: Conway withdrawals decoder + exact sum (B3-S3, CE-B3-3).
//!
//! The withdrawals field is a definite-length CBOR map of `reward_account =>
//! coin`. The decoder is a closed grammar: every malformed shape is a structured
//! reject, never a partial sum. All fixtures are constructed inline as real CBOR.

use std::collections::BTreeMap;

use ade_codec::conway::withdrawals::{decode_withdrawals, withdrawals_sum};
use ade_codec::error::CodecError;
use ade_types::tx::{Coin, RewardAccount};

// ---------------------------------------------------------------------------
// Minimal CBOR builders.
// ---------------------------------------------------------------------------

/// Major-type-tagged unsigned argument (used for uint values and map/array
/// headers and byte-string lengths).
fn cbor_head(buf: &mut Vec<u8>, major: u8, value: u64) {
    let m = major << 5;
    if value < 24 {
        buf.push(m | value as u8);
    } else if value <= u64::from(u8::MAX) {
        buf.push(m | 24);
        buf.push(value as u8);
    } else if value <= u64::from(u16::MAX) {
        buf.push(m | 25);
        buf.extend_from_slice(&(value as u16).to_be_bytes());
    } else if value <= u64::from(u32::MAX) {
        buf.push(m | 26);
        buf.extend_from_slice(&(value as u32).to_be_bytes());
    } else {
        buf.push(m | 27);
        buf.extend_from_slice(&value.to_be_bytes());
    }
}

fn cbor_uint(buf: &mut Vec<u8>, value: u64) {
    cbor_head(buf, 0, value);
}

fn cbor_bytes(buf: &mut Vec<u8>, bytes: &[u8]) {
    cbor_head(buf, 2, bytes.len() as u64);
    buf.extend_from_slice(bytes);
}

fn definite_map_header(buf: &mut Vec<u8>, n: u64) {
    cbor_head(buf, 5, n);
}

fn account(header: u8, fill: u8) -> [u8; 29] {
    let mut a = [fill; 29];
    a[0] = header;
    a
}

// ---------------------------------------------------------------------------
// Happy path.
// ---------------------------------------------------------------------------

#[test]
fn conway_withdrawals() {
    let a1 = account(0xe1, 0x11);
    let a2 = account(0xe1, 0x22);
    let a3 = account(0xe0, 0x33);

    // Encode in non-sorted wire order to prove the BTreeMap normalizes.
    let mut buf = Vec::new();
    definite_map_header(&mut buf, 3);
    cbor_bytes(&mut buf, &a3);
    cbor_uint(&mut buf, 7);
    cbor_bytes(&mut buf, &a1);
    cbor_uint(&mut buf, 1_000_000);
    cbor_bytes(&mut buf, &a2);
    cbor_uint(&mut buf, 2_500_000);

    let decoded = decode_withdrawals(&buf).expect("decode withdrawals");

    let mut expected = BTreeMap::new();
    expected.insert(RewardAccount(a1), Coin(1_000_000));
    expected.insert(RewardAccount(a2), Coin(2_500_000));
    expected.insert(RewardAccount(a3), Coin(7));
    assert_eq!(decoded, expected);

    // Deterministic: decode twice => identical map.
    let decoded2 = decode_withdrawals(&buf).expect("decode withdrawals again");
    assert_eq!(decoded, decoded2);

    // Sum exact and order-independent.
    assert_eq!(withdrawals_sum(&decoded), 3_500_007_i128);
}

// ---------------------------------------------------------------------------
// Rejections.
// ---------------------------------------------------------------------------

#[test]
fn withdrawals_truncated_rejected() {
    let a1 = account(0xe1, 0x11);
    let mut buf = Vec::new();
    definite_map_header(&mut buf, 2);
    cbor_bytes(&mut buf, &a1);
    cbor_uint(&mut buf, 42);
    // Second entry declared but absent.
    let err = decode_withdrawals(&buf).expect_err("truncated must reject");
    assert!(matches!(err, CodecError::UnexpectedEof { .. }));
}

#[test]
fn withdrawals_indefinite_map_rejected() {
    let a1 = account(0xe1, 0x11);
    let mut buf = Vec::new();
    buf.push(0xbf); // indefinite-length map header
    cbor_bytes(&mut buf, &a1);
    cbor_uint(&mut buf, 42);
    buf.push(0xff); // break
    let err = decode_withdrawals(&buf).expect_err("indefinite map must reject");
    assert!(matches!(err, CodecError::InvalidCborStructure { .. }));
}

#[test]
fn withdrawals_non_bytes_key_rejected() {
    let mut buf = Vec::new();
    definite_map_header(&mut buf, 1);
    cbor_uint(&mut buf, 5); // key is a uint, not a byte string
    cbor_uint(&mut buf, 42);
    let err = decode_withdrawals(&buf).expect_err("non-bytes key must reject");
    assert!(matches!(err, CodecError::UnexpectedCborType { .. }));
}

#[test]
fn withdrawals_wrong_account_length_rejected() {
    let mut buf = Vec::new();
    definite_map_header(&mut buf, 1);
    cbor_bytes(&mut buf, &[0xe1; 28]); // 28 bytes, not 29
    cbor_uint(&mut buf, 42);
    let err = decode_withdrawals(&buf).expect_err("wrong-length account must reject");
    assert!(matches!(err, CodecError::InvalidLength { .. }));
}

#[test]
fn withdrawals_value_exceeds_u64_rejected() {
    let a1 = account(0xe1, 0x11);
    let mut buf = Vec::new();
    definite_map_header(&mut buf, 1);
    cbor_bytes(&mut buf, &a1);
    // A value > u64::MAX is encoded as a CBOR bignum (tag 2 + byte string),
    // which is not a major-type-0 uint. The decoder must reject, not truncate.
    buf.push(0xc2); // tag(2) — unsigned bignum
    cbor_bytes(&mut buf, &[0x01; 9]); // 9-byte magnitude > u64::MAX
    let err = decode_withdrawals(&buf).expect_err("oversize coin must reject");
    assert!(matches!(err, CodecError::UnexpectedCborType { .. }));
}

#[test]
fn withdrawals_duplicate_key_rejected() {
    let a1 = account(0xe1, 0x11);
    let mut buf = Vec::new();
    definite_map_header(&mut buf, 2);
    cbor_bytes(&mut buf, &a1);
    cbor_uint(&mut buf, 100);
    cbor_bytes(&mut buf, &a1); // duplicate key
    cbor_uint(&mut buf, 200);
    let err = decode_withdrawals(&buf).expect_err("duplicate key must reject");
    assert!(matches!(err, CodecError::DuplicateMapKey { .. }));
}

#[test]
fn withdrawals_trailing_bytes_rejected() {
    let a1 = account(0xe1, 0x11);
    let mut buf = Vec::new();
    definite_map_header(&mut buf, 1);
    cbor_bytes(&mut buf, &a1);
    cbor_uint(&mut buf, 42);
    buf.push(0xff); // extra trailing byte after a complete map
    let err = decode_withdrawals(&buf).expect_err("trailing bytes must reject");
    assert!(matches!(err, CodecError::TrailingBytes { .. }));
}

#[test]
fn withdrawals_sum_is_exact() {
    let a1 = account(0xe1, 0x11);
    let a2 = account(0xe1, 0x22);
    let a3 = account(0xe0, 0x33);

    let mut map = BTreeMap::new();
    map.insert(RewardAccount(a1), Coin(u64::MAX));
    map.insert(RewardAccount(a2), Coin(u64::MAX));
    map.insert(RewardAccount(a3), Coin(1));

    // Exact i128 accumulation: no overflow, no saturation, no rounding.
    let expected = i128::from(u64::MAX) + i128::from(u64::MAX) + 1;
    assert_eq!(withdrawals_sum(&map), expected);

    // Order-independent: inserting in a different order yields the same sum.
    let mut map2 = BTreeMap::new();
    map2.insert(RewardAccount(a3), Coin(1));
    map2.insert(RewardAccount(a2), Coin(u64::MAX));
    map2.insert(RewardAccount(a1), Coin(u64::MAX));
    assert_eq!(withdrawals_sum(&map2), expected);
}
