//! Integration test: Conway-complete certificate decoder + closed deposit-effect
//! classification (B3-S2, CE-B3-2, invariant DC-TXV-06).
//!
//! The tag -> disposition mapping is read from the committed §3.0 fixture
//! `corpus/conway_certs/tags.json`; it is never hand-coded here. This test
//! constructs a minimal real CBOR certificate per tag, decodes it through the
//! closed `decode_conway_certs` grammar, and confirms the decoded variant maps
//! to the fixture-declared disposition category.

use std::fs;
use std::path::PathBuf;

use ade_codec::conway::cert::decode_conway_certs;
use ade_codec::error::CodecError;
use ade_types::conway::cert::ConwayCert;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("conway_certs")
        .join("tags.json")
}

struct FixtureRow {
    tag: u64,
    disposition: String,
}

fn load_fixture() -> Vec<FixtureRow> {
    let bytes = fs::read(fixture_path()).expect("read tags.json fixture");
    let value: serde_json::Value = serde_json::from_slice(&bytes).expect("parse tags.json");
    let certs = value
        .get("certs")
        .and_then(|c| c.as_array())
        .expect("fixture has certs array");
    certs
        .iter()
        .map(|row| FixtureRow {
            tag: row.get("tag").and_then(|t| t.as_u64()).expect("tag"),
            disposition: row
                .get("disposition")
                .and_then(|d| d.as_str())
                .expect("disposition")
                .to_string(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Minimal CBOR cert builders. Each produces a single decodable certificate
// array `[tag, ...]` carrying only the fields the closed decoder parses; any
// remaining fields are present but structurally skipped by the decoder.
// ---------------------------------------------------------------------------

fn cbor_uint(buf: &mut Vec<u8>, major: u8, value: u64) {
    let m = major << 5;
    if value < 24 {
        buf.push(m | value as u8);
    } else if value < 0x100 {
        buf.push(m | 24);
        buf.push(value as u8);
    } else if value < 0x1_0000 {
        buf.push(m | 25);
        buf.extend_from_slice(&(value as u16).to_be_bytes());
    } else if value < 0x1_0000_0000 {
        buf.push(m | 26);
        buf.extend_from_slice(&(value as u32).to_be_bytes());
    } else {
        buf.push(m | 27);
        buf.extend_from_slice(&value.to_be_bytes());
    }
}

fn arr(buf: &mut Vec<u8>, n: u64) {
    cbor_uint(buf, 4, n);
}

fn uint(buf: &mut Vec<u8>, v: u64) {
    cbor_uint(buf, 0, v);
}

fn bytestr(buf: &mut Vec<u8>, b: &[u8]) {
    cbor_uint(buf, 2, b.len() as u64);
    buf.extend_from_slice(b);
}

fn hash28(buf: &mut Vec<u8>) {
    bytestr(buf, &[0xABu8; 28]);
}

fn hash32(buf: &mut Vec<u8>) {
    bytestr(buf, &[0xCDu8; 32]);
}

fn stake_credential(buf: &mut Vec<u8>) {
    arr(buf, 2);
    uint(buf, 0); // key-hash credential type
    hash28(buf);
}

/// Conway pool_params, encoded as the in-line sequence of fields the
/// pool_registration_cert carries after the tag.
fn pool_params(buf: &mut Vec<u8>) {
    hash28(buf); // operator (pool keyhash)
    hash32(buf); // vrf keyhash
    uint(buf, 0); // pledge
    uint(buf, 0); // cost
    // margin = unit_interval = #6.30([uint, uint])
    buf.push(0xc0 | 24); // tag, 1-byte arg
    buf.push(30);
    arr(buf, 2);
    uint(buf, 0);
    uint(buf, 1);
    hash28(buf); // reward_account (28-byte stand-in)
    arr(buf, 0); // pool_owners set
    arr(buf, 0); // relays
    buf.push(0xf6); // pool_metadata = null
}

fn drep(buf: &mut Vec<u8>) {
    arr(buf, 2);
    uint(buf, 0); // drep credential type: key-hash
    hash28(buf);
}

fn anchor_null(buf: &mut Vec<u8>) {
    buf.push(0xf6); // null
}

/// Build a single Conway certificate (just the cert array, no outer wrapper).
fn build_cert(tag: u64) -> Vec<u8> {
    let mut b = Vec::new();
    match tag {
        0 => {
            arr(&mut b, 2);
            uint(&mut b, 0);
            stake_credential(&mut b);
        }
        1 => {
            arr(&mut b, 2);
            uint(&mut b, 1);
            stake_credential(&mut b);
        }
        2 => {
            arr(&mut b, 3);
            uint(&mut b, 2);
            stake_credential(&mut b);
            hash28(&mut b);
        }
        3 => {
            arr(&mut b, 10);
            uint(&mut b, 3);
            pool_params(&mut b);
        }
        4 => {
            arr(&mut b, 3);
            uint(&mut b, 4);
            hash28(&mut b);
            uint(&mut b, 100);
        }
        5 => {
            // Shelley genesis_key_delegation shape (removed in Conway).
            arr(&mut b, 4);
            uint(&mut b, 5);
            hash28(&mut b);
            hash28(&mut b);
            hash32(&mut b);
        }
        6 => {
            // MIR shape (removed in Conway): [6, [target, ...]]
            arr(&mut b, 2);
            uint(&mut b, 6);
            arr(&mut b, 2);
            uint(&mut b, 0);
            arr(&mut b, 0);
        }
        7 => {
            arr(&mut b, 3);
            uint(&mut b, 7);
            stake_credential(&mut b);
            uint(&mut b, 2_000_000);
        }
        8 => {
            arr(&mut b, 3);
            uint(&mut b, 8);
            stake_credential(&mut b);
            uint(&mut b, 2_000_000);
        }
        9 => {
            arr(&mut b, 3);
            uint(&mut b, 9);
            stake_credential(&mut b);
            drep(&mut b);
        }
        10 => {
            arr(&mut b, 4);
            uint(&mut b, 10);
            stake_credential(&mut b);
            hash28(&mut b);
            drep(&mut b);
        }
        11 => {
            arr(&mut b, 4);
            uint(&mut b, 11);
            stake_credential(&mut b);
            hash28(&mut b);
            uint(&mut b, 2_000_000);
        }
        12 => {
            arr(&mut b, 4);
            uint(&mut b, 12);
            stake_credential(&mut b);
            drep(&mut b);
            uint(&mut b, 2_000_000);
        }
        13 => {
            arr(&mut b, 5);
            uint(&mut b, 13);
            stake_credential(&mut b);
            hash28(&mut b);
            drep(&mut b);
            uint(&mut b, 2_000_000);
        }
        14 => {
            arr(&mut b, 3);
            uint(&mut b, 14);
            stake_credential(&mut b); // committee_cold_credential
            stake_credential(&mut b); // committee_hot_credential
        }
        15 => {
            arr(&mut b, 3);
            uint(&mut b, 15);
            stake_credential(&mut b);
            anchor_null(&mut b);
        }
        16 => {
            arr(&mut b, 4);
            uint(&mut b, 16);
            stake_credential(&mut b); // drep credential
            uint(&mut b, 500_000_000);
            anchor_null(&mut b);
        }
        17 => {
            arr(&mut b, 3);
            uint(&mut b, 17);
            stake_credential(&mut b);
            uint(&mut b, 500_000_000);
        }
        18 => {
            arr(&mut b, 3);
            uint(&mut b, 18);
            stake_credential(&mut b);
            anchor_null(&mut b);
        }
        other => panic!("no builder for out-of-grammar tag {other}"),
    }
    b
}

/// Wrap one cert into a single-element certificate array.
fn build_cert_array(tag: u64) -> Vec<u8> {
    let mut b = Vec::new();
    arr(&mut b, 1);
    b.extend_from_slice(&build_cert(tag));
    b
}

/// Disposition category derived purely from the decoded variant — the codec
/// layer's view. The classifier (ade_ledger) refines the coin source; this
/// proves the decoded variant lands in the fixture-declared category.
fn category(cert: &ConwayCert) -> &'static str {
    match cert {
        ConwayCert::AccountRegistration { .. }
        | ConwayCert::PoolRegistration { .. }
        | ConwayCert::AccountRegistrationDeposit { .. }
        | ConwayCert::StakeRegistrationDelegation { .. }
        | ConwayCert::VoteRegistrationDelegation { .. }
        | ConwayCert::StakeVoteRegistrationDelegation { .. }
        | ConwayCert::DRepRegistration { .. } => "NewDeposit",
        ConwayCert::AccountUnregistration { .. }
        | ConwayCert::AccountUnregistrationDeposit { .. }
        | ConwayCert::DRepUnregistration { .. } => "Refund",
        ConwayCert::StakeDelegation
        | ConwayCert::PoolRetirement
        | ConwayCert::VoteDelegation
        | ConwayCert::StakeVoteDelegation
        | ConwayCert::AuthCommitteeHot
        | ConwayCert::ResignCommitteeCold
        | ConwayCert::DRepUpdate => "Neutral",
        ConwayCert::RemovedInConway { .. } => "NotValidInConway",
    }
}

#[test]
fn decode_total_over_tags_0_18() {
    let fixture = load_fixture();
    assert_eq!(fixture.len(), 19, "fixture must cover tags 0..18");

    for row in &fixture {
        let bytes = build_cert_array(row.tag);
        let decoded = decode_conway_certs(&bytes)
            .unwrap_or_else(|e| panic!("tag {} failed to decode: {e}", row.tag));
        assert_eq!(decoded.len(), 1, "tag {} decoded to one cert", row.tag);
        assert_eq!(
            category(&decoded[0]),
            row.disposition,
            "tag {} decoded variant {:?} not in fixture disposition category {}",
            row.tag,
            decoded[0],
            row.disposition
        );
    }
}

#[test]
fn unknown_cert_tag_is_codec_error() {
    let bytes = {
        let mut b = Vec::new();
        arr(&mut b, 1);
        arr(&mut b, 2);
        uint(&mut b, 19); // first tag outside the closed grammar
        uint(&mut b, 0);
        b
    };
    match decode_conway_certs(&bytes) {
        Err(CodecError::UnknownCertTag { tag, .. }) => assert_eq!(tag, 19),
        other => panic!("expected UnknownCertTag, got {other:?}"),
    }
}

#[test]
fn removed_tag_5_6_is_not_valid_in_conway() {
    let fixture = load_fixture();
    for tag in [5u64, 6] {
        let row = fixture
            .iter()
            .find(|r| r.tag == tag)
            .expect("fixture row present");
        assert_eq!(
            row.disposition, "NotValidInConway",
            "fixture must mark tag {tag} as NotValidInConway"
        );
        let bytes = build_cert_array(tag);
        let decoded = decode_conway_certs(&bytes).expect("removed tag still decodes structurally");
        assert!(
            matches!(decoded[0], ConwayCert::RemovedInConway { tag: t } if t == tag),
            "tag {tag} must decode to RemovedInConway, got {:?}",
            decoded[0]
        );
        assert_eq!(category(&decoded[0]), "NotValidInConway");
    }
}

#[test]
fn malformed_cert_cbor_rejected() {
    // Cert array declares 3 elements but the credential hash is the wrong length
    // and the array is truncated.
    let mut b = Vec::new();
    arr(&mut b, 1);
    arr(&mut b, 3);
    uint(&mut b, 2); // delegation_to_stake_pool_cert
    arr(&mut b, 2);
    uint(&mut b, 0);
    bytestr(&mut b, &[0x01, 0x02]); // 2-byte hash where 28 expected
    match decode_conway_certs(&b) {
        Err(CodecError::InvalidLength { .. })
        | Err(CodecError::UnexpectedEof { .. })
        | Err(CodecError::InvalidCborStructure { .. })
        | Err(CodecError::UnexpectedCborType { .. }) => {}
        other => panic!("expected a structured malformed-CBOR reject, got {other:?}"),
    }
}

#[test]
fn decode_is_replay_deterministic() {
    for tag in 0u64..=18 {
        let bytes = build_cert_array(tag);
        let first = decode_conway_certs(&bytes).expect("decode");
        let second = decode_conway_certs(&bytes).expect("decode");
        assert_eq!(first, second, "tag {tag} decode not deterministic");
    }
}
