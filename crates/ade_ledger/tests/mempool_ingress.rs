// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// PHASE4-N-E S1 (DC-MEM-03): the single BLUE chokepoint `mempool_ingress`
// is a thin pass-through to `admit`; an `IngressEvent.source` MUST NOT
// change the verdict. Adversarial cases reuse the B-track synthetic corpus
// via `ade_testkit::tx_validity::*`; full corpus-wide replay lands in S2.

use ade_ledger::mempool::{
    mempool_ingress, AdmitOutcome, IngressEvent, IngressSource, MempoolState,
};
use ade_testkit::tx_validity::{build_synthetic, build_valid, SyntheticMutation};

/// A valid synthetic tx admitted via N2N: outcome `Admitted`, accepted grows
/// by one, accumulating state evolves. Mirrors the existing B2-S5 admit test
/// but routes through the new `mempool_ingress` chokepoint.
#[test]
fn ingress_admits_valid_tx_via_n2n() {
    let case = build_valid();
    let mempool = MempoolState::new(case.ledger.clone());
    let event = IngressEvent::new(IngressSource::N2N, case.tx_cbor.clone());

    let (next, outcome) = mempool_ingress(&mempool, &event);

    match outcome {
        AdmitOutcome::Admitted { .. } => {}
        AdmitOutcome::Rejected { class, error } => {
            panic!("valid tx via N2N was rejected: {class:?} ({error:?})")
        }
    }
    assert_eq!(next.accepted().len(), 1, "accepted must grow by one");
    assert_ne!(
        next.accumulating(),
        mempool.accumulating(),
        "accumulating state must evolve when a tx is admitted"
    );
}

/// Same tx, routed through N2C instead of N2N — must Admit identically.
#[test]
fn ingress_admits_valid_tx_via_n2c() {
    let case = build_valid();
    let mempool = MempoolState::new(case.ledger.clone());
    let event = IngressEvent::new(IngressSource::N2C, case.tx_cbor.clone());

    let (next, outcome) = mempool_ingress(&mempool, &event);

    match outcome {
        AdmitOutcome::Admitted { .. } => {}
        AdmitOutcome::Rejected { class, error } => {
            panic!("valid tx via N2C was rejected: {class:?} ({error:?})")
        }
    }
    assert_eq!(next.accepted().len(), 1);
}

/// The load-bearing no-false-accept assertion at the ingress layer:
/// adversarial synthetic mutations are Rejected and the mempool is
/// returned UNCHANGED. Mirrors B2-S5's direct-admit shape.
#[test]
fn ingress_rejects_invalid_tx_no_false_accept() {
    for mutation in [
        SyntheticMutation::ForgedInputWitness,
        SyntheticMutation::MissingInputWitness,
        SyntheticMutation::ValueImbalance,
        SyntheticMutation::DanglingInput,
    ] {
        let case = build_synthetic(mutation);
        let mempool = MempoolState::new(case.ledger.clone());
        let event = IngressEvent::new(IngressSource::N2N, case.tx_cbor.clone());

        let (next, outcome) = mempool_ingress(&mempool, &event);

        match outcome {
            AdmitOutcome::Rejected { .. } => {}
            AdmitOutcome::Admitted { .. } => panic!(
                "FALSE ACCEPT via mempool_ingress (release-blocking): adversarial {mutation:?} was admitted"
            ),
        }
        assert_eq!(
            next, mempool,
            "mempool must be UNCHANGED after a rejected tx ({mutation:?})"
        );
    }
}

/// N-E-N7 / N-E-8: same `(state, tx_bytes)` under `IngressSource::N2N` vs
/// `IngressSource::N2C` produces byte-identical `(MempoolState, AdmitOutcome)`.
/// Property holds across a valid case and adversarial cases.
#[test]
fn ingress_source_does_not_change_verdict_valid() {
    let case = build_valid();
    let mempool = MempoolState::new(case.ledger.clone());
    let ev_n2n = IngressEvent::new(IngressSource::N2N, case.tx_cbor.clone());
    let ev_n2c = IngressEvent::new(IngressSource::N2C, case.tx_cbor.clone());

    let (next_n2n, outcome_n2n) = mempool_ingress(&mempool, &ev_n2n);
    let (next_n2c, outcome_n2c) = mempool_ingress(&mempool, &ev_n2c);

    assert_eq!(
        next_n2n, next_n2c,
        "N2N and N2C must produce byte-identical MempoolState (valid case)"
    );
    assert_eq!(
        outcome_n2n, outcome_n2c,
        "N2N and N2C must produce byte-identical AdmitOutcome (valid case)"
    );
}

#[test]
fn ingress_source_does_not_change_verdict_adversarial() {
    for mutation in [
        SyntheticMutation::ForgedInputWitness,
        SyntheticMutation::ValueImbalance,
    ] {
        let case = build_synthetic(mutation);
        let mempool = MempoolState::new(case.ledger.clone());
        let ev_n2n = IngressEvent::new(IngressSource::N2N, case.tx_cbor.clone());
        let ev_n2c = IngressEvent::new(IngressSource::N2C, case.tx_cbor.clone());

        let (next_n2n, outcome_n2n) = mempool_ingress(&mempool, &ev_n2n);
        let (next_n2c, outcome_n2c) = mempool_ingress(&mempool, &ev_n2c);

        assert_eq!(
            next_n2n, next_n2c,
            "N2N/N2C MempoolState diverged on adversarial {mutation:?}"
        );
        assert_eq!(
            outcome_n2n, outcome_n2c,
            "N2N/N2C AdmitOutcome diverged on adversarial {mutation:?}"
        );
    }
}

/// `mempool_ingress` must equal direct `admit(&mempool, event.tx_bytes())`
/// — proving the bridge is a thin pass-through, no extra logic. This is the
/// load-bearing CE-N-E-2 property in miniature; S2's harness extends it
/// across the full B-track corpus.
#[test]
fn ingress_equals_direct_admit_on_synthetic_corpus() {
    use ade_ledger::mempool::admit;

    let cases: Vec<_> = [
        SyntheticMutation::ForgedInputWitness,
        SyntheticMutation::ValueImbalance,
        SyntheticMutation::DanglingInput,
    ]
    .into_iter()
    .map(build_synthetic)
    .collect();

    for case in cases {
        let mempool = MempoolState::new(case.ledger.clone());
        let event = IngressEvent::new(IngressSource::N2N, case.tx_cbor.clone());

        let (next_ingress, outcome_ingress) = mempool_ingress(&mempool, &event);
        let (next_admit, outcome_admit) = admit(&mempool, &case.tx_cbor);

        assert_eq!(
            next_ingress, next_admit,
            "mempool_ingress and admit diverged in MempoolState"
        );
        assert_eq!(
            outcome_ingress, outcome_admit,
            "mempool_ingress and admit diverged in AdmitOutcome"
        );
    }

    // Also covers the valid path.
    let case = build_valid();
    let mempool = MempoolState::new(case.ledger.clone());
    let event = IngressEvent::new(IngressSource::N2N, case.tx_cbor.clone());
    let (next_ingress, outcome_ingress) = mempool_ingress(&mempool, &event);
    let (next_admit, outcome_admit) = admit(&mempool, &case.tx_cbor);
    assert_eq!(next_ingress, next_admit);
    assert_eq!(outcome_ingress, outcome_admit);
}
