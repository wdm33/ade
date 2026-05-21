//! PHASE4-B4-S5 (CE-B4-5): Conway cert-state accumulation corpus.
//!
//! Mirrors the B4 wiring (`accumulate_tx_certs`) at the public API:
//! `decode_conway_certs` + `apply_conway_cert` threaded over a cert sequence.
//!
//! SCOPE OF THIS FILE (what is mechanically closed here):
//!   1. POSITIVE (synthetic): a real-shaped Conway cert sequence accumulates into
//!      the correct B4-owned CertState (delegation + pool) under controlled state.
//!   2. REPLAY: the accumulation is byte-identical across two runs (T-DET-01).
//!   3. ADVERSARIAL (no false accept): malformed / unknown-tag / removed-tag /
//!      truncated / trailing-bytes cert arrays each reject — never a silent accept.
//!
//! ENVIRONMENT-BLOCKED (NOT closed here — documented open obligation, identical
//! to the B3-S5 constraint): the REAL epoch-576 cert-state-vs-cardano-node oracle.
//! The epoch-576 ledger-state/UMap snapshot was deleted post-extraction and is NOT
//! in this repo (see corpus/validity/conway_epoch576/README.md). Real Conway certs
//! cannot be accumulated at track_utxo=true here because their prior delegation/
//! pool state would not resolve — apply_conway_cert would fail-closed on
//! StakeNotRegistered/PoolNotRegistered for credentials registered in blocks/epochs
//! not present locally. The real-corpus CertState agreement therefore remains an
//! open obligation for CE-B4-5, reclassified environment-blocked per the project's
//! tier doctrine. This file does NOT claim real-chain agreement.

#![allow(clippy::unwrap_used)]

use ade_codec::conway::cert::decode_conway_certs;
use ade_ledger::delegation::{apply_conway_cert, CertState, ConwayCertEnv};
use ade_types::tx::Coin;

const KD: Coin = Coin(2_000_000);

// --- minimal CBOR cert-array builders ---
fn u(buf: &mut Vec<u8>, major: u8, v: u64) {
    let m = major << 5;
    if v < 24 {
        buf.push(m | v as u8);
    } else if v < 0x100 {
        buf.push(m | 24);
        buf.push(v as u8);
    } else {
        buf.push(m | 25);
        buf.extend_from_slice(&(v as u16).to_be_bytes());
    }
}
fn arr(b: &mut Vec<u8>, n: u64) {
    u(b, 4, n);
}
fn uint(b: &mut Vec<u8>, v: u64) {
    u(b, 0, v);
}
fn cred(b: &mut Vec<u8>, m: u8) {
    arr(b, 2);
    uint(b, 0);
    u(b, 2, 28);
    b.extend_from_slice(&[m; 28]);
}
fn h28(b: &mut Vec<u8>, m: u8) {
    u(b, 2, 28);
    b.extend_from_slice(&[m; 28]);
}
fn reg(b: &mut Vec<u8>, m: u8) {
    arr(b, 2);
    uint(b, 0);
    cred(b, m);
}
fn pool_reg(b: &mut Vec<u8>, m: u8) {
    arr(b, 10);
    uint(b, 3);
    h28(b, m); // operator
    u(b, 2, 32); // vrf (32 bytes)
    b.extend_from_slice(&[0xCD; 32]);
    uint(b, 0); // pledge
    uint(b, 0); // cost
    b.push(0xc0 | 24); // margin tag 30
    b.push(30);
    arr(b, 2);
    uint(b, 0);
    uint(b, 1);
    u(b, 2, 29); // reward_account
    b.extend_from_slice(&[0xe0; 29]);
    arr(b, 0); // owners
    arr(b, 0); // relays
    b.push(0xf6); // metadata null
}
fn deleg(b: &mut Vec<u8>, cm: u8, pm: u8) {
    arr(b, 3);
    uint(b, 2);
    cred(b, cm);
    h28(b, pm);
}

/// Accumulate a Conway cert array exactly as `accumulate_tx_certs` does (public
/// surface), returning the final B4-owned CertState or the first structured error.
fn accumulate(cert_bytes: &[u8]) -> Result<CertState, String> {
    let certs = decode_conway_certs(cert_bytes).map_err(|e| format!("decode: {e:?}"))?;
    let mut state = CertState::new();
    for (idx, cert) in certs.iter().enumerate() {
        let env = ConwayCertEnv { key_deposit: KD, cert_index: idx as u16 };
        let out = apply_conway_cert(&state, cert, &env).map_err(|e| format!("apply: {e:?}"))?;
        state = out.state;
    }
    Ok(state)
}

/// A real-shaped, balanced cert sequence: register a stake key, register a pool,
/// delegate the key to that pool.
fn positive_sequence() -> Vec<u8> {
    let mut b = Vec::new();
    arr(&mut b, 3);
    reg(&mut b, 1);
    pool_reg(&mut b, 9);
    deleg(&mut b, 1, 9);
    b
}

#[test]
fn positive_synthetic_cert_state_accumulates() {
    let state = accumulate(&positive_sequence()).expect("balanced sequence accumulates");
    let c1 = ade_types::shelley::cert::StakeCredential::KeyHash(ade_types::Hash28([1u8; 28]));
    let p9 = ade_types::tx::PoolId(ade_types::Hash28([9u8; 28]));
    assert!(state.delegation.registrations.contains_key(&c1), "key registered");
    assert!(state.pool.pools.contains_key(&p9), "pool registered");
    assert_eq!(state.delegation.delegations.get(&c1), Some(&p9), "delegated to pool");
}

#[test]
fn cert_state_replay_byte_identical() {
    let a = accumulate(&positive_sequence()).unwrap();
    let b = accumulate(&positive_sequence()).unwrap();
    assert_eq!(a, b, "two accumulations of the same sequence are identical (T-DET-01)");
}

#[test]
fn adversarial_no_false_accept() {
    // Each adversarial cert array must reject (decode or apply) — never accept.
    let mut cases: Vec<(&str, Vec<u8>)> = Vec::new();

    // unknown tag (>= 19)
    let mut b = Vec::new();
    arr(&mut b, 1);
    arr(&mut b, 1);
    uint(&mut b, 19);
    cases.push(("unknown_tag_19", b));

    // removed tag 5 (decodes to RemovedInConway, apply rejects era-invalid)
    let mut b = Vec::new();
    arr(&mut b, 1);
    arr(&mut b, 1);
    uint(&mut b, 5);
    cases.push(("removed_tag_5", b));

    // truncated: array claims 1 element, none present
    let mut b = Vec::new();
    arr(&mut b, 1);
    cases.push(("truncated_array", b));

    // malformed: a delegation cert missing its pool keyhash
    let mut b = Vec::new();
    arr(&mut b, 1);
    arr(&mut b, 3);
    uint(&mut b, 2);
    cred(&mut b, 1);
    // (pool keyhash omitted)
    cases.push(("malformed_delegation", b));

    // trailing bytes after the cert array
    let mut b = positive_sequence();
    b.push(0xff);
    cases.push(("trailing_bytes", b));

    // apply error: delegate an unregistered credential (no false accept)
    let mut b = Vec::new();
    arr(&mut b, 1);
    deleg(&mut b, 7, 8);
    cases.push(("delegate_unregistered", b));

    for (name, bytes) in cases {
        assert!(
            accumulate(&bytes).is_err(),
            "adversarial case '{name}' must reject, not silently accept",
        );
    }
}
