//! OQ5-S2 (CE-5/CE-6): credential key/script discriminant fidelity corpus.
//!
//! Proves the DC-LEDGER-10 authority: a key-hash and a script-hash credential
//! sharing the same 28 bytes are DISTINCT authoritative-state keys in both
//! CertState (B4-owned) and ConwayGovState (B5-owned) — never a silent collapse
//! — and the accumulation replays byte-identical over the discriminated
//! fingerprint surface.
//!
//! ENVIRONMENT-BLOCKED (NOT closed here): real-chain agreement of the
//! discriminated keys vs cardano-node's Credential-keyed UMap/VState — the
//! epoch-576 / boundary snapshots are absent locally (recoverable from the
//! ImmutableDB EBS snapshots). Reclassified per tier doctrine, as
//! DC-LEDGER-08/09. See corpus/credential_fidelity/README.md.

#![allow(clippy::unwrap_used)]

use ade_ledger::delegation::{apply_conway_cert, CertState, ConwayCertEnv};
use ade_ledger::gov_cert::apply_conway_gov_cert;
use ade_ledger::state::{ConwayGovState, GovCertEnv, LedgerState};
use ade_types::conway::cert::{ConwayCert, DRep};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::Coin;
use ade_types::{CardanoEra, Hash28};
use std::collections::BTreeMap;

fn key(b: u8) -> StakeCredential {
    StakeCredential::KeyHash(Hash28([b; 28]))
}
fn script(b: u8) -> StakeCredential {
    StakeCredential::ScriptHash(Hash28([b; 28]))
}
fn env() -> GovCertEnv {
    GovCertEnv { current_epoch: 576, drep_activity: 20 }
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

/// CE-3/CE-6: a key-hash and a script-hash registration over identical 28 bytes
/// are two distinct CertState entries — no collision, no silent overwrite.
#[test]
fn keyhash_scripthash_same_bytes_are_distinct_certstate() {
    let cenv = ConwayCertEnv { key_deposit: Coin(2_000_000), cert_index: 0 };
    let s = CertState::new();
    let s = apply_conway_cert(&s, &ConwayCert::AccountRegistration { credential: key(7) }, &cenv)
        .unwrap()
        .state;
    let s = apply_conway_cert(&s, &ConwayCert::AccountRegistration { credential: script(7) }, &cenv)
        .unwrap()
        .state;
    assert_eq!(s.delegation.registrations.len(), 2, "key vs script same bytes are distinct");
    assert!(s.delegation.registrations.contains_key(&key(7)));
    assert!(s.delegation.registrations.contains_key(&script(7)));
}

/// CE-3/CE-6: same distinctness in ConwayGovState (vote_delegations) — the gov
/// half keyed on the discriminated credential.
#[test]
fn keyhash_scripthash_same_bytes_are_distinct_govstate() {
    let g = base_gov();
    let g = apply_conway_gov_cert(
        &g,
        &ConwayCert::VoteDelegation { credential: key(7), drep: DRep::AlwaysAbstain },
        Some(&env()),
    ).unwrap();
    let g = apply_conway_gov_cert(
        &g,
        &ConwayCert::VoteDelegation { credential: script(7), drep: DRep::AlwaysNoConfidence },
        Some(&env()),
    ).unwrap();
    assert_eq!(g.vote_delegations.len(), 2, "key vs script same bytes are distinct gov keys");
    assert_eq!(g.vote_delegations.get(&key(7)), Some(&DRep::AlwaysAbstain));
    assert_eq!(g.vote_delegations.get(&script(7)), Some(&DRep::AlwaysNoConfidence));
}

/// COMMITTEE-CRED-FIDELITY CE-3: a key-hash and a script-hash committee member
/// over identical 28 bytes are distinct members — no collapse.
#[test]
fn committee_keyhash_scripthash_same_bytes_distinct() {
    let mut g = base_gov();
    g.committee.insert(key(7), 100);
    g.committee.insert(script(7), 100);
    assert_eq!(g.committee.len(), 2, "key vs script committee members are distinct");
    assert!(g.committee.contains_key(&key(7)));
    assert!(g.committee.contains_key(&script(7)));
}

/// COMMITTEE-CRED-FIDELITY CE-4: two states differing only in a committee
/// member's discriminant fingerprint differently.
#[test]
fn committee_discriminant_changes_fingerprint() {
    let mut g_key = base_gov();
    g_key.committee.insert(key(7), 100);
    let mut g_script = base_gov();
    g_script.committee.insert(script(7), 100);

    let mut s_key = LedgerState::new(CardanoEra::Conway);
    s_key.gov_state = Some(g_key);
    let mut s_script = LedgerState::new(CardanoEra::Conway);
    s_script.gov_state = Some(g_script);
    assert_ne!(
        ade_ledger::fingerprint::fingerprint(&s_key).governance,
        ade_ledger::fingerprint::fingerprint(&s_script).governance,
        "committee member discriminant must change the fingerprint",
    );
}

/// CE-4: two states differing only in a credential's key/script discriminant
/// fingerprint differently (the discriminant is in the canonical encoding).
#[test]
fn discriminant_changes_fingerprint_corpus() {
    let mut g_key = base_gov();
    g_key.vote_delegations.insert(key(7), DRep::AlwaysAbstain);
    let mut g_script = base_gov();
    g_script.vote_delegations.insert(script(7), DRep::AlwaysAbstain);

    let mut s_key = LedgerState::new(CardanoEra::Conway);
    s_key.gov_state = Some(g_key);
    let mut s_script = LedgerState::new(CardanoEra::Conway);
    s_script.gov_state = Some(g_script);

    let f_key = ade_ledger::fingerprint::fingerprint(&s_key);
    let f_script = ade_ledger::fingerprint::fingerprint(&s_script);
    assert_ne!(f_key.governance, f_script.governance, "discriminant must change the fingerprint");
}

/// CE-5: accumulating a mixed key/script credential sequence replays
/// byte-identical (T-DET-01) over the discriminated fingerprint.
#[test]
fn credential_accumulation_replays_byte_identical() {
    let seq = [
        ConwayCert::VoteDelegation { credential: key(7), drep: DRep::KeyHash(Hash28([0xDD; 28])) },
        ConwayCert::VoteDelegation { credential: script(7), drep: DRep::AlwaysAbstain },
        ConwayCert::DRepRegistration { drep_credential: script(0xAA), deposit: Coin(500_000_000) },
        ConwayCert::AuthCommitteeHot { cold_credential: key(0xC0), hot_credential: script(0x40) },
    ];
    let run = || {
        let mut g = base_gov();
        for c in &seq {
            g = apply_conway_gov_cert(&g, c, Some(&env())).unwrap();
        }
        g
    };
    let g1 = run();
    let g2 = run();
    assert_eq!(g1, g2, "value identity");

    // The script-hash DRep registration landed under the ScriptHash key, not a
    // collapsed bare hash.
    assert!(g1.drep_expiry.contains_key(&script(0xAA)));
    assert!(!g1.drep_expiry.contains_key(&key(0xAA)), "no collapse to key-hash");

    let mut s1 = LedgerState::new(CardanoEra::Conway);
    s1.gov_state = Some(g1);
    let mut s2 = LedgerState::new(CardanoEra::Conway);
    s2.gov_state = Some(g2);
    assert_eq!(
        ade_ledger::fingerprint::fingerprint(&s1).combined,
        ade_ledger::fingerprint::fingerprint(&s2).combined,
        "discriminated-credential accumulation replays byte-identical (T-DET-01)",
    );
}
