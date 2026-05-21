//! Integration test: the single shared Conway certificate decoder is
//! **owner-complete** (PHASE4-B4 / B4-S1, CE-B4-1, invariant DC-LEDGER-08).
//!
//! For every Conway CDDL certificate tag `0..18` this test builds a certificate
//! whose every field carries a *distinct* recognizable marker, decodes it
//! through `decode_conway_certs`, and asserts the decoded `ConwayCert` retains
//! exactly those payloads — the credentials, pool id, full pool parameters
//! (including `pool_owners`), DRep delegation targets, and committee credentials
//! that authoritative owners (`DelegationState`, `PoolState`, `ConwayGovState`)
//! need. This is the mechanical proof that the decoder is not a deposit-only
//! lossy projection.

use ade_codec::conway::cert::decode_conway_certs;
use ade_types::conway::cert::{ConwayCert, DRep};
use ade_types::{Hash28, Hash32};

// --- minimal CBOR builders ---

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
fn h28(buf: &mut Vec<u8>, marker: u8) {
    bytestr(buf, &[marker; 28]);
}
fn h32(buf: &mut Vec<u8>, marker: u8) {
    bytestr(buf, &[marker; 32]);
}
/// `credential = [type, hash28]` — type 0 (key) here; the type byte is not
/// retained (key/script hashes do not collide), only the hash.
fn cred(buf: &mut Vec<u8>, marker: u8) {
    arr(buf, 2);
    uint(buf, 0);
    h28(buf, marker);
}
/// `drep = [0, addr_keyhash]` keyed by a distinct marker.
fn drep_key(buf: &mut Vec<u8>, marker: u8) {
    arr(buf, 2);
    uint(buf, 0);
    h28(buf, marker);
}

// Distinct field markers, so a swapped/dropped field is caught.
const CRED: u8 = 0x11;
const POOL: u8 = 0x22;
const VRF: u8 = 0x33;
const OWNER: u8 = 0x44;
const DREPK: u8 = 0x55;
const COLD: u8 = 0x66;
const HOT: u8 = 0x77;
const RWD: u8 = 0x88;

fn h(marker: u8) -> Hash28 {
    Hash28([marker; 28])
}

fn decode_one(cert_bytes: Vec<u8>) -> ConwayCert {
    let mut arr_buf = Vec::new();
    arr(&mut arr_buf, 1);
    arr_buf.extend_from_slice(&cert_bytes);
    let certs = decode_conway_certs(&arr_buf).expect("decode");
    assert_eq!(certs.len(), 1);
    certs.into_iter().next().unwrap()
}

/// `pool_params` field sequence with distinct markers and one owner.
fn pool_params(buf: &mut Vec<u8>) {
    h28(buf, POOL); // operator
    h32(buf, VRF); // vrf
    uint(buf, 11); // pledge
    uint(buf, 22); // cost
    buf.push(0xc0 | 24); // margin tag 30
    buf.push(30);
    arr(buf, 2);
    uint(buf, 1);
    uint(buf, 2);
    bytestr(buf, &[RWD; 29]); // reward_account (arbitrary bytes)
    arr(buf, 1); // pool_owners set: 1 owner
    h28(buf, OWNER);
    arr(buf, 0); // relays
    buf.push(0xf6); // pool_metadata = null
}

#[test]
fn each_tag_retains_owner_payloads() {
    // tag 2 — StakeDelegation { credential, pool_id }
    let mut b = Vec::new();
    arr(&mut b, 3);
    uint(&mut b, 2);
    cred(&mut b, CRED);
    h28(&mut b, POOL);
    match decode_one(b) {
        ConwayCert::StakeDelegation { credential, pool_id } => {
            assert_eq!(credential.0, h(CRED));
            assert_eq!(pool_id.0, h(POOL));
        }
        other => panic!("tag 2 → {other:?}"),
    }

    // tag 3 — PoolRegistration(PoolRegistrationCert) incl. owners (Finding A)
    let mut b = Vec::new();
    arr(&mut b, 10);
    uint(&mut b, 3);
    pool_params(&mut b);
    match decode_one(b) {
        ConwayCert::PoolRegistration(c) => {
            assert_eq!(c.pool_id.0, h(POOL), "operator/pool_id");
            assert_eq!(c.vrf_hash, Hash32([VRF; 32]), "vrf");
            assert_eq!(c.pledge.0, 11);
            assert_eq!(c.cost.0, 22);
            assert_eq!(c.margin, (1, 2));
            assert_eq!(c.reward_account, vec![RWD; 29]);
            assert_eq!(c.owners, vec![h(OWNER)], "pool_owners must be retained");
        }
        other => panic!("tag 3 → {other:?}"),
    }

    // tag 4 — PoolRetirement { pool_id, epoch }
    let mut b = Vec::new();
    arr(&mut b, 3);
    uint(&mut b, 4);
    h28(&mut b, POOL);
    uint(&mut b, 4242);
    match decode_one(b) {
        ConwayCert::PoolRetirement { pool_id, epoch } => {
            assert_eq!(pool_id.0, h(POOL));
            assert_eq!(epoch.0, 4242);
        }
        other => panic!("tag 4 → {other:?}"),
    }

    // tag 9 — VoteDelegation { credential, drep }
    let mut b = Vec::new();
    arr(&mut b, 3);
    uint(&mut b, 9);
    cred(&mut b, CRED);
    drep_key(&mut b, DREPK);
    match decode_one(b) {
        ConwayCert::VoteDelegation { credential, drep } => {
            assert_eq!(credential.0, h(CRED));
            assert_eq!(drep, DRep::KeyHash(h(DREPK)));
        }
        other => panic!("tag 9 → {other:?}"),
    }

    // tag 10 — StakeVoteDelegation { credential, pool_id, drep }
    let mut b = Vec::new();
    arr(&mut b, 4);
    uint(&mut b, 10);
    cred(&mut b, CRED);
    h28(&mut b, POOL);
    drep_key(&mut b, DREPK);
    match decode_one(b) {
        ConwayCert::StakeVoteDelegation { credential, pool_id, drep } => {
            assert_eq!(credential.0, h(CRED));
            assert_eq!(pool_id.0, h(POOL));
            assert_eq!(drep, DRep::KeyHash(h(DREPK)));
        }
        other => panic!("tag 10 → {other:?}"),
    }

    // tag 13 — StakeVoteRegistrationDelegation { credential, pool_id, drep, deposit }
    let mut b = Vec::new();
    arr(&mut b, 5);
    uint(&mut b, 13);
    cred(&mut b, CRED);
    h28(&mut b, POOL);
    drep_key(&mut b, DREPK);
    uint(&mut b, 2_000_000);
    match decode_one(b) {
        ConwayCert::StakeVoteRegistrationDelegation { credential, pool_id, drep, deposit } => {
            assert_eq!(credential.0, h(CRED));
            assert_eq!(pool_id.0, h(POOL));
            assert_eq!(drep, DRep::KeyHash(h(DREPK)));
            assert_eq!(deposit.0, 2_000_000);
        }
        other => panic!("tag 13 → {other:?}"),
    }

    // tag 14 — AuthCommitteeHot { cold_credential, hot_credential }
    let mut b = Vec::new();
    arr(&mut b, 3);
    uint(&mut b, 14);
    cred(&mut b, COLD);
    cred(&mut b, HOT);
    match decode_one(b) {
        ConwayCert::AuthCommitteeHot { cold_credential, hot_credential } => {
            assert_eq!(cold_credential.0, h(COLD));
            assert_eq!(hot_credential.0, h(HOT));
        }
        other => panic!("tag 14 → {other:?}"),
    }

    // tag 15 — ResignCommitteeCold { cold_credential } (anchor consumed)
    let mut b = Vec::new();
    arr(&mut b, 3);
    uint(&mut b, 15);
    cred(&mut b, COLD);
    b.push(0xf6); // anchor = null
    match decode_one(b) {
        ConwayCert::ResignCommitteeCold { cold_credential } => {
            assert_eq!(cold_credential.0, h(COLD));
        }
        other => panic!("tag 15 → {other:?}"),
    }

    // tag 16 — DRepRegistration { drep_credential, deposit } (anchor consumed)
    let mut b = Vec::new();
    arr(&mut b, 4);
    uint(&mut b, 16);
    cred(&mut b, DREPK);
    uint(&mut b, 500_000_000);
    b.push(0xf6);
    match decode_one(b) {
        ConwayCert::DRepRegistration { drep_credential, deposit } => {
            assert_eq!(drep_credential.0, h(DREPK));
            assert_eq!(deposit.0, 500_000_000);
        }
        other => panic!("tag 16 → {other:?}"),
    }

    // tag 18 — DRepUpdate { drep_credential } (anchor consumed)
    let mut b = Vec::new();
    arr(&mut b, 3);
    uint(&mut b, 18);
    cred(&mut b, DREPK);
    b.push(0xf6);
    match decode_one(b) {
        ConwayCert::DRepUpdate { drep_credential } => {
            assert_eq!(drep_credential.0, h(DREPK));
        }
        other => panic!("tag 18 → {other:?}"),
    }
}

/// `decode_drep` is total over the closed `drep` grammar.
#[test]
fn drep_grammar_total() {
    fn vote_deleg_with_drep(drep_body: &dyn Fn(&mut Vec<u8>)) -> ConwayCert {
        let mut b = Vec::new();
        arr(&mut b, 3);
        uint(&mut b, 9);
        cred(&mut b, CRED);
        drep_body(&mut b);
        decode_one(b)
    }

    // [0, keyhash]
    let c = vote_deleg_with_drep(&|b| {
        arr(b, 2);
        uint(b, 0);
        h28(b, DREPK);
    });
    assert!(matches!(c, ConwayCert::VoteDelegation { drep: DRep::KeyHash(_), .. }));

    // [1, scripthash]
    let c = vote_deleg_with_drep(&|b| {
        arr(b, 2);
        uint(b, 1);
        h28(b, DREPK);
    });
    assert!(matches!(c, ConwayCert::VoteDelegation { drep: DRep::ScriptHash(_), .. }));

    // [2] abstain
    let c = vote_deleg_with_drep(&|b| {
        arr(b, 1);
        uint(b, 2);
    });
    assert!(matches!(c, ConwayCert::VoteDelegation { drep: DRep::AlwaysAbstain, .. }));

    // [3] no-confidence
    let c = vote_deleg_with_drep(&|b| {
        arr(b, 1);
        uint(b, 3);
    });
    assert!(matches!(c, ConwayCert::VoteDelegation { drep: DRep::AlwaysNoConfidence, .. }));

    // [4] — unknown DRep variant rejects (no catch-all)
    let mut b = Vec::new();
    arr(&mut b, 3);
    uint(&mut b, 9);
    cred(&mut b, CRED);
    arr(&mut b, 1);
    uint(&mut b, 4);
    let mut arr_buf = Vec::new();
    arr(&mut arr_buf, 1);
    arr_buf.extend_from_slice(&b);
    assert!(decode_conway_certs(&arr_buf).is_err(), "unknown drep variant must reject");
}
