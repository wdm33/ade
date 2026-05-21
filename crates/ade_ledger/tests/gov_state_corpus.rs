//! PHASE4-B5-S4 (CE-5/CE-6): Conway governance-state accumulation corpus.
//!
//! Exercises the B5 authority — `apply_conway_gov_cert` threaded over a cert
//! sequence into `ConwayGovState`, the same dispatch the block path
//! (`accumulate_tx_certs`) performs.
//!
//! SCOPE OF THIS FILE (what is mechanically closed here):
//!   1. POSITIVE (synthetic): a real-shaped Conway governance-cert sequence
//!      (vote delegation, committee hot-key auth, DRep registration) accumulates
//!      into the correct ConwayGovState (vote_delegations, committee_hot_keys,
//!      drep_expiry) under a controlled base state and env.
//!   2. REPLAY: the accumulation is byte-identical across two runs, asserted over
//!      the canonical gov-state fingerprint surface (`fingerprint`) — T-DET-01.
//!   3. ADVERSARIAL (no false accept): a DRep register/update with the env absent
//!      fails fast (never a defaulted expiry); decode-layer hazards (unknown tag,
//!      removed tag, truncated array) reject; a double resignation is
//!      deterministic.
//!
//! ENVIRONMENT-BLOCKED (NOT closed here — documented open obligation, identical
//! to the B4-S5 / B3-S5 constraint): the REAL epoch-576 governance-state
//! (VState)-vs-cardano-node oracle. The epoch-576 ledger-state / UMap snapshot
//! was deleted post-extraction and is NOT in this repo (see
//! corpus/gov_state/README.md and corpus/validity/conway_epoch576/README.md).
//! Real-chain gov-state agreement therefore remains an open obligation for
//! DC-LEDGER-09, reclassified environment-blocked per the project tier doctrine.
//! This file does NOT claim real-chain agreement.

#![allow(clippy::unwrap_used)]

use ade_codec::conway::cert::decode_conway_certs;
use ade_ledger::error::{LedgerError, ValidationEnvironmentError};
use ade_ledger::gov_cert::apply_conway_gov_cert;
use ade_ledger::state::{ConwayGovState, GovCertEnv, LedgerState};
use ade_types::conway::cert::{ConwayCert, DRep};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::Coin;
use ade_types::{CardanoEra, Hash28};
use std::collections::BTreeMap;

fn cred(b: u8) -> StakeCredential {
    StakeCredential(Hash28([b; 28]))
}
fn h(b: u8) -> Hash28 {
    Hash28([b; 28])
}

fn base_gov() -> ConwayGovState {
    ConwayGovState {
        proposals: Vec::new(),
        committee: BTreeMap::new(),
        committee_quorum: (2, 3),
        drep_expiry: BTreeMap::new(),
        gov_action_lifetime: 6,
        vote_delegations: BTreeMap::new(),
        pool_voting_thresholds: Vec::new(),
        drep_voting_thresholds: Vec::new(),
        committee_hot_keys: BTreeMap::new(),
    }
}

fn env() -> GovCertEnv {
    GovCertEnv {
        current_epoch: 576,
        drep_activity: 20,
    }
}

/// Thread a governance-cert sequence into ConwayGovState exactly as the block
/// path does (apply_conway_gov_cert per cert), returning the accumulated state.
fn accumulate_gov(base: &ConwayGovState, certs: &[ConwayCert], env: Option<&GovCertEnv>)
    -> Result<ConwayGovState, LedgerError>
{
    let mut gov = base.clone();
    for cert in certs {
        gov = apply_conway_gov_cert(&gov, cert, env)?;
    }
    Ok(gov)
}

/// A real-shaped governance-cert sequence: delegate vote to a DRep, authorize a
/// committee hot key, register a DRep.
fn positive_sequence() -> Vec<ConwayCert> {
    vec![
        ConwayCert::VoteDelegation { credential: cred(1), drep: DRep::KeyHash(h(0xDD)) },
        ConwayCert::AuthCommitteeHot { cold_credential: cred(0xC0), hot_credential: cred(0x40) },
        ConwayCert::DRepRegistration { drep_credential: cred(0xAA), deposit: Coin(500_000_000) },
    ]
}

#[test]
fn positive_synthetic_gov_state_accumulates() {
    let gov = accumulate_gov(&base_gov(), &positive_sequence(), Some(&env())).unwrap();
    assert_eq!(gov.vote_delegations.get(&h(1)), Some(&DRep::KeyHash(h(0xDD))), "vote delegation");
    assert_eq!(gov.committee_hot_keys.get(&h(0x40)), Some(&h(0xC0)), "committee hot key");
    assert_eq!(gov.drep_expiry.get(&h(0xAA)), Some(&(576 + 20)), "DRep expiry = epoch + activity");
}

#[test]
fn gov_state_accumulation_replays_byte_identical() {
    // The accumulated gov-state is byte-identical across two runs, asserted over
    // the canonical fingerprint surface (governance component + combined).
    let g1 = accumulate_gov(&base_gov(), &positive_sequence(), Some(&env())).unwrap();
    let g2 = accumulate_gov(&base_gov(), &positive_sequence(), Some(&env())).unwrap();
    assert_eq!(g1, g2, "value identity");

    let mut s1 = LedgerState::new(CardanoEra::Conway);
    s1.gov_state = Some(g1);
    let mut s2 = LedgerState::new(CardanoEra::Conway);
    s2.gov_state = Some(g2);
    let f1 = ade_ledger::fingerprint::fingerprint(&s1);
    let f2 = ade_ledger::fingerprint::fingerprint(&s2);
    assert_eq!(f1.governance, f2.governance, "gov fingerprint byte-identical (T-DET-01)");
    assert_eq!(f1.combined, f2.combined, "combined fingerprint byte-identical");
}

#[test]
fn adversarial_drep_register_update_missing_env_rejected() {
    // A DRep register/update needs drep_activity; with env absent, accumulation
    // fails fast — never a defaulted expiry, never a silent accept.
    for cert in [
        ConwayCert::DRepRegistration { drep_credential: cred(0xAA), deposit: Coin(500_000_000) },
        ConwayCert::DRepUpdate { drep_credential: cred(0xAA) },
    ] {
        let res = accumulate_gov(&base_gov(), &[cert.clone()], None);
        assert!(
            matches!(
                res,
                Err(LedgerError::ValidationEnvironment(
                    ValidationEnvironmentError::MissingDRepActivityParam
                ))
            ),
            "missing-env DRep cert must reject: {cert:?}",
        );
    }
}

#[test]
fn adversarial_decode_layer_rejects_guard_gov_path() {
    // The gov path consumes the same closed Conway decoder; decode-layer hazards
    // reject before any gov apply — no false accept.
    fn u(buf: &mut Vec<u8>, major: u8, v: u64) {
        let m = major << 5;
        if v < 24 { buf.push(m | v as u8); }
        else { buf.push(m | 24); buf.push(v as u8); }
    }
    let mut cases: Vec<(&str, Vec<u8>)> = Vec::new();

    // unknown tag (>= 19)
    let mut b = Vec::new(); u(&mut b, 4, 1); u(&mut b, 4, 1); u(&mut b, 0, 19);
    cases.push(("unknown_tag_19", b));
    // removed tag 5 decodes to RemovedInConway (then era-rejected on apply)
    let mut b = Vec::new(); u(&mut b, 4, 1); u(&mut b, 4, 1); u(&mut b, 0, 5);
    cases.push(("removed_tag_5", b));
    // truncated: array claims 1 element, none present
    let mut b = Vec::new(); u(&mut b, 4, 1);
    cases.push(("truncated_array", b));

    for (name, bytes) in cases {
        // unknown_tag / truncated reject at decode; removed_tag decodes but is not
        // a governance mutation (RemovedInConway -> gov unchanged) and is rejected
        // by the B4 cert-state apply in the real wiring. Here we assert the decode
        // surface rejects the two malformed cases.
        if name == "removed_tag_5" {
            // decodes; gov dispatch leaves state unchanged (B4 rejects it on apply).
            let certs = decode_conway_certs(&bytes).unwrap();
            let gov = accumulate_gov(&base_gov(), &certs, Some(&env())).unwrap();
            assert_eq!(gov, base_gov(), "removed tag is not a gov mutation");
        } else {
            assert!(decode_conway_certs(&bytes).is_err(), "{name} must reject at decode");
        }
    }
}

#[test]
fn adversarial_drep_expiry_overflow_rejected() {
    // An absurd drep_activity (overflowing current_epoch + drep_activity) is a
    // deterministic fail-closed halt, never a silent wrap to a wrong expiry.
    let e = GovCertEnv { current_epoch: 10, drep_activity: u64::MAX };
    let res = accumulate_gov(
        &base_gov(),
        &[ConwayCert::DRepRegistration { drep_credential: cred(0xAA), deposit: Coin(500_000_000) }],
        Some(&e),
    );
    assert!(
        matches!(
            res,
            Err(LedgerError::ValidationEnvironment(
                ValidationEnvironmentError::DRepActivityOverflow
            ))
        ),
        "overflowing DRep expiry must reject fail-closed",
    );
}

#[test]
fn adversarial_double_resign_is_deterministic() {
    // Resigning the same cold credential twice is idempotent and deterministic:
    // the first removes the hot authorization, the second is a no-op.
    let mut g = base_gov();
    g.committee_hot_keys.insert(h(0x40), h(0xC0));
    let resign = ConwayCert::ResignCommitteeCold { cold_credential: cred(0xC0) };

    let once = accumulate_gov(&g, &[resign.clone()], Some(&env())).unwrap();
    let twice = accumulate_gov(&g, &[resign.clone(), resign.clone()], Some(&env())).unwrap();
    assert_eq!(once.committee_hot_keys.get(&h(0x40)), None, "first resign clears authorization");
    assert_eq!(once, twice, "double resign is idempotent and deterministic");
}
