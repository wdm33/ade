// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// PROPOSAL-PROCEDURES-DECODE PP-S2 (CE-PP-6): canonical corpus
// round-trip evidence for `proposal_procedures`. The corpus is
// synthetic-canonical per OQ-5 (no real-chain Conway txs carrying
// proposals in the in-tree corpus at PP-S1 HEAD).

use ade_testkit::governance::{
    canonical_corpus, replay_canonical_corpus, CorpusEntry, ReplayOutcome,
};
use ade_types::conway::governance::GovAction;
use ade_types::shelley::cert::StakeCredential;

/// CE-PP-6 (decode half): every corpus entry decodes to the typed
/// shape it declares as `expected`. Encoding-side is checked by the
/// next test; this isolates decode failures from encode failures in
/// the diagnostic.
#[test]
fn canonical_corpus_decodes_to_expected_shape() {
    use ade_codec::conway::governance::decode_proposal_procedures;

    for CorpusEntry { label, bytes, expected } in canonical_corpus() {
        let decoded = decode_proposal_procedures(&bytes)
            .unwrap_or_else(|e| panic!("{label}: decode failed: {e:?}"));
        assert_eq!(decoded, expected, "{label}: typed shape mismatch");
    }
}

/// CE-PP-6 (round-trip half): every corpus entry round-trips
/// byte-identically through decode → encode. The `replay_canonical_corpus`
/// helper combines both directions; here we assert every entry is
/// `Ok`. Failures surface the per-entry label so a regression names
/// the offending shape.
#[test]
fn canonical_corpus_round_trips_byte_identical() {
    let outcomes = replay_canonical_corpus();
    let mut failures = Vec::new();
    for (label, outcome) in &outcomes {
        if outcome != &ReplayOutcome::Ok {
            failures.push(format!("{label}: {outcome:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "canonical corpus round-trip failures: {}",
        failures.join("; ")
    );
    assert!(outcomes.len() >= 7, "corpus must cover at least 7 entries");
}

/// Defensive: the corpus must contain at least one entry for every
/// `GovAction` variant. A future refactor that silently drops a
/// variant from the corpus would weaken this slice's evidence; this
/// test catches it.
#[test]
fn canonical_corpus_covers_all_gov_action_variants() {
    let entries = canonical_corpus();
    let mut saw_info = false;
    let mut saw_no_confidence = false;
    let mut saw_hard_fork = false;
    let mut saw_treasury = false;
    let mut saw_param_change = false;
    let mut saw_new_const = false;
    let mut saw_update_committee = false;

    for CorpusEntry { expected, .. } in &entries {
        for p in expected {
            match p.gov_action {
                GovAction::InfoAction => saw_info = true,
                GovAction::NoConfidence { .. } => saw_no_confidence = true,
                GovAction::HardForkInitiation { .. } => saw_hard_fork = true,
                GovAction::TreasuryWithdrawals { .. } => saw_treasury = true,
                GovAction::ParameterChange { .. } => saw_param_change = true,
                GovAction::NewConstitution { .. } => saw_new_const = true,
                GovAction::UpdateCommittee { .. } => saw_update_committee = true,
            }
        }
    }

    assert!(saw_info, "corpus missing InfoAction");
    assert!(saw_no_confidence, "corpus missing NoConfidence");
    assert!(saw_hard_fork, "corpus missing HardForkInitiation");
    assert!(saw_treasury, "corpus missing TreasuryWithdrawals");
    assert!(saw_param_change, "corpus missing ParameterChange");
    assert!(saw_new_const, "corpus missing NewConstitution");
    assert!(saw_update_committee, "corpus missing UpdateCommittee");
}

/// DC-LEDGER-10 cross-check in the corpus: at least one entry must
/// have an `UpdateCommittee` with both `KeyHash(h)` and
/// `ScriptHash(h)` of equal 28 bytes in the `removed` set. This
/// proves the harness exercises the discriminated-credential
/// preservation property (not just the structural decode path).
#[test]
fn canonical_corpus_includes_update_committee_discriminant_case() {
    let mut found = false;
    for CorpusEntry { expected, .. } in canonical_corpus() {
        for p in expected {
            if let GovAction::UpdateCommittee { ref removed, .. } = p.gov_action {
                let key_hashes: Vec<&[u8; 28]> = removed
                    .iter()
                    .filter_map(|c| match c {
                        StakeCredential::KeyHash(h) => Some(&h.0),
                        _ => None,
                    })
                    .collect();
                let script_hashes: Vec<&[u8; 28]> = removed
                    .iter()
                    .filter_map(|c| match c {
                        StakeCredential::ScriptHash(h) => Some(&h.0),
                        _ => None,
                    })
                    .collect();
                for kh in &key_hashes {
                    if script_hashes.iter().any(|sh| sh == kh) {
                        found = true;
                    }
                }
            }
        }
    }
    assert!(
        found,
        "corpus must include UpdateCommittee with KeyHash + ScriptHash of equal 28 bytes \
         (DC-LEDGER-10 cross-check)"
    );
}
