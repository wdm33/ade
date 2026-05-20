// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// CE-B2-5 (PHASE4-B2-S5): the mempool admission gate. `admit` is a Tier-1 gate
// over the BLUE `tx_validity`: a tx is admitted iff `tx_validity(accumulating,
// tx)` is Valid — NO FALSE ACCEPT into the mempool — and the Tier-5 `policy`
// (eviction/ordering) provably cannot change that verdict. Synthetic txs over a
// controlled UTxO (track_utxo=true), reusing the B2-S1/S4 synthetic recipe.

use ade_ledger::mempool::{admit, order, AdmitOutcome, MempoolState, OrderPolicy};
use ade_ledger::tx_validity::{tx_validity, TxValidityVerdict};
use ade_testkit::tx_validity::{
    build_dependent_pair, build_synthetic, build_valid, SyntheticMutation,
};

/// A valid synthetic tx is admitted: outcome `Admitted`, `accepted` grows by
/// one, and the accumulating state evolves (the spent input is consumed).
#[test]
fn valid_tx_admitted_and_accumulates() {
    let case = build_valid();
    let mempool = MempoolState::new(case.ledger.clone());
    assert_eq!(mempool.accepted().len(), 0);

    let (next, outcome) = admit(&mempool, &case.tx_cbor);

    let tx_id = match outcome {
        AdmitOutcome::Admitted { tx_id } => tx_id,
        AdmitOutcome::Rejected { class, error } => {
            panic!("valid tx was rejected: {class:?} ({error:?})")
        }
    };
    assert_eq!(next.accepted().len(), 1, "accepted must grow by one");
    assert_eq!(next.accepted()[0], tx_id, "accepted carries the admitted id");
    assert_ne!(
        next.accumulating(),
        mempool.accumulating(),
        "accumulating state must evolve when a tx is admitted"
    );
}

/// A forged-witness / value-imbalance synthetic tx is rejected, and the mempool
/// is returned UNCHANGED. The load-bearing no-false-accept assertion: an invalid
/// tx never enters the mempool.
#[test]
fn invalid_tx_rejected_no_false_accept() {
    // Cover the witness-forgery and value-imbalance adversarial shapes.
    for mutation in [
        SyntheticMutation::ForgedInputWitness,
        SyntheticMutation::MissingInputWitness,
        SyntheticMutation::ValueImbalance,
        SyntheticMutation::DanglingInput,
    ] {
        let case = build_synthetic(mutation);
        let mempool = MempoolState::new(case.ledger.clone());

        let (next, outcome) = admit(&mempool, &case.tx_cbor);

        match outcome {
            AdmitOutcome::Rejected { .. } => {}
            AdmitOutcome::Admitted { .. } => panic!(
                "FALSE ACCEPT (release-blocking): adversarial {mutation:?} was admitted"
            ),
        }
        assert_eq!(
            next, mempool,
            "mempool must be UNCHANGED after a rejected tx ({mutation:?})"
        );
        assert_eq!(
            next.accepted().len(),
            0,
            "no invalid tx may enter the mempool ({mutation:?})"
        );
    }
}

/// `admit`'s outcome matches `tx_validity`'s verdict 1:1 over a mixed
/// valid+invalid set: Valid ⇒ Admitted with the same tx id; Invalid ⇒ Rejected
/// with the same class. The admission gate is exactly the validity gate.
#[test]
fn admission_equals_tx_validity_verdict() {
    // (label, tx_cbor, ledger) — one valid, several invalid.
    let valid = build_valid();
    let mut cases: Vec<(String, Vec<u8>, ade_ledger::state::LedgerState)> =
        vec![("valid".to_string(), valid.tx_cbor, valid.ledger)];
    for mutation in SyntheticMutation::ALL {
        let c = build_synthetic(mutation);
        cases.push((format!("{mutation:?}"), c.tx_cbor, c.ledger));
    }

    for (label, tx_cbor, ledger) in &cases {
        let verdict = tx_validity(ledger, tx_cbor).verdict;
        let mempool = MempoolState::new(ledger.clone());
        let (_next, outcome) = admit(&mempool, tx_cbor);

        match (&verdict, &outcome) {
            (
                TxValidityVerdict::Valid { tx_id, .. },
                AdmitOutcome::Admitted { tx_id: admitted_id },
            ) => {
                assert_eq!(tx_id, admitted_id, "{label}: admitted id must equal validity tx id");
            }
            (
                TxValidityVerdict::Invalid { class, .. },
                AdmitOutcome::Rejected {
                    class: reject_class,
                    ..
                },
            ) => {
                assert_eq!(
                    class, reject_class,
                    "{label}: reject class must equal validity class"
                );
            }
            _ => panic!(
                "{label}: admit outcome does not match tx_validity verdict ({verdict:?} vs {outcome:?})"
            ),
        }
    }
}

/// Tier-5 independence: the admit verdict is identical regardless of the policy
/// applied or the queue ordering. Running the Tier-5 `order` (under either
/// policy) before and after admission cannot change which txs are admitted.
#[test]
fn policy_does_not_change_validity() {
    let valid = build_valid();
    let invalid = build_synthetic(SyntheticMutation::ForgedInputWitness);

    for (label, tx_cbor, ledger) in [
        ("valid", &valid.tx_cbor, &valid.ledger),
        ("invalid", &invalid.tx_cbor, &invalid.ledger),
    ] {
        let base = MempoolState::new(ledger.clone());

        // Baseline admit verdict (no policy interaction).
        let (_n0, baseline) = admit(&base, tx_cbor);

        // For each policy, run `order` first (a no-op on the verdict by
        // construction) then admit, and confirm the outcome is identical.
        for policy in [OrderPolicy::ArrivalOrder, OrderPolicy::TxIdAscending] {
            let _ordered = order(&base, policy); // Tier-5 projection — discarded.
            let (_n1, after_policy) = admit(&base, tx_cbor);
            assert_eq!(
                baseline, after_policy,
                "{label}: policy {policy:?} changed the admit verdict (Tier-5 leaked into Tier-1)"
            );
        }

        // Also confirm `order` is a pure permutation of the admitted set: it
        // neither adds nor drops ids, so it cannot admit/evict anything.
        let (admitted, _) = admit(&base, tx_cbor);
        let arrival = order(&admitted, OrderPolicy::ArrivalOrder);
        let ascending = order(&admitted, OrderPolicy::TxIdAscending);
        assert_eq!(
            arrival.len(),
            admitted.accepted().len(),
            "{label}: ArrivalOrder must preserve the admitted-set size"
        );
        assert_eq!(
            ascending.len(),
            admitted.accepted().len(),
            "{label}: TxIdAscending must preserve the admitted-set size"
        );
        let mut a = arrival.clone();
        let mut b = ascending.clone();
        a.sort_by_key(|x| x.0);
        b.sort_by_key(|x| x.0);
        assert_eq!(a, b, "{label}: both policies must be permutations of the same set");
    }
}

/// A dependent tx B (spending tx A's output) is admitted ONLY after A is
/// admitted: against the base mempool B is unresolvable (Rejected); after A is
/// admitted, B re-validates against the accumulating UTxO and is Admitted.
#[test]
fn dependent_tx_admitted_against_accumulating_state() {
    let pair = build_dependent_pair();
    let base = MempoolState::new(pair.ledger.clone());

    // B against the base mempool: A's output does not yet exist → Rejected.
    let (after_b_first, b_first) = admit(&base, &pair.tx_b);
    match b_first {
        AdmitOutcome::Rejected { .. } => {}
        AdmitOutcome::Admitted { .. } => {
            panic!("dependent tx B admitted before A — stale-state false accept")
        }
    }
    assert_eq!(after_b_first, base, "B rejection must leave the mempool unchanged");

    // Admit A.
    let (after_a, a_outcome) = admit(&base, &pair.tx_a);
    assert!(
        matches!(a_outcome, AdmitOutcome::Admitted { .. }),
        "tx A must be admitted"
    );
    assert_eq!(after_a.accepted().len(), 1);

    // Now B re-validates against the accumulating state (which holds A's output).
    let (after_b, b_outcome) = admit(&after_a, &pair.tx_b);
    assert!(
        matches!(b_outcome, AdmitOutcome::Admitted { .. }),
        "tx B must be admitted against the accumulating state after A"
    );
    assert_eq!(
        after_b.accepted().len(),
        2,
        "both A and B admitted in sequence"
    );
}

/// Determinism: the same admit sequence produces the same final mempool and the
/// same outcome stream across two independent runs.
#[test]
fn determinism() {
    fn run() -> (MempoolState, Vec<AdmitOutcome>) {
        let pair = build_dependent_pair();
        let invalid = build_synthetic(SyntheticMutation::ForgedInputWitness);
        // The invalid tx targets a different controlled UTxO; admitting it
        // against this mempool must reject (unresolvable input) and leave the
        // mempool unchanged — exercised here purely for the outcome stream.
        let mut mempool = MempoolState::new(pair.ledger.clone());
        let mut outcomes = Vec::new();

        let seq: [&[u8]; 3] = [&pair.tx_a, &invalid.tx_cbor, &pair.tx_b];
        for tx in seq {
            let (next, outcome) = admit(&mempool, tx);
            mempool = next;
            outcomes.push(outcome);
        }
        (mempool, outcomes)
    }

    let (mempool_a, outcomes_a) = run();
    let (mempool_b, outcomes_b) = run();

    assert_eq!(
        mempool_a, mempool_b,
        "same admit sequence must yield the same final mempool"
    );
    assert_eq!(
        outcomes_a, outcomes_b,
        "same admit sequence must yield the same outcome stream"
    );
    // A and B admitted, the adversarial tx rejected → two accepted ids.
    assert_eq!(mempool_a.accepted().len(), 2);
}
