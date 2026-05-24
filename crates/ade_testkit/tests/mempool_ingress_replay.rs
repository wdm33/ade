// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// PHASE4-N-E S2 (DC-MEM-04 + DC-MEM-01 strengthening): replay the B-track
// adversarial corpus through `mempool_ingress` and assert byte-identical
// agreement with direct `admit`. The corpus tx bytes are reused verbatim;
// only the `IngressEvent` envelope is added.

use ade_ledger::mempool::{admit, mempool_ingress, AdmitOutcome, IngressEvent, IngressSource, MempoolState};
use ade_testkit::mempool::{
    b_track_corpus_as_ingress, replay_ingress_trace, wrap_as_ingress, BTrackCase,
    ExpectedOutcome,
};
use ade_testkit::tx_validity::build_dependent_pair;

/// CE-N-E-2: `mempool_ingress` ≡ direct `admit` on every B-track case
/// (valid + every `SyntheticMutation`). Byte-identical
/// `(MempoolState, AdmitOutcome)`.
#[test]
fn ingress_admit_equals_direct_admit_on_b_track_corpus() {
    let cases = b_track_corpus_as_ingress(IngressSource::N2N);
    assert!(cases.len() >= 5, "B-track corpus must include valid + all 4 mutations");

    for BTrackCase { event, base, expected: _ } in cases {
        let mempool = MempoolState::new(base.clone());

        let (next_ingress, outcome_ingress) = mempool_ingress(&mempool, &event);
        let (next_admit, outcome_admit) = admit(&mempool, event.tx_bytes());

        assert_eq!(
            next_ingress, next_admit,
            "mempool_ingress and admit diverged in MempoolState"
        );
        assert_eq!(
            outcome_ingress, outcome_admit,
            "mempool_ingress and admit diverged in AdmitOutcome"
        );
    }
}

/// CE-N-E-5: the B-track adversarial rejections are preserved through
/// the ingress bridge — every mutation's expected `TxRejectClass` matches
/// what `mempool_ingress` returns.
#[test]
fn b_track_adversarial_rejections_preserved_through_ingress() {
    let cases = b_track_corpus_as_ingress(IngressSource::N2N);
    let mut saw_admit = 0;
    let mut saw_reject = 0;

    for BTrackCase { event, base, expected } in cases {
        let mempool = MempoolState::new(base);
        let (next, outcome) = mempool_ingress(&mempool, &event);

        match (expected, outcome) {
            (ExpectedOutcome::Admit, AdmitOutcome::Admitted { .. }) => {
                saw_admit += 1;
                assert_eq!(
                    next.accepted().len(),
                    1,
                    "valid case must admit one tx"
                );
            }
            (ExpectedOutcome::Reject(want), AdmitOutcome::Rejected { class, .. }) => {
                saw_reject += 1;
                assert_eq!(
                    class, want,
                    "adversarial mutation produced wrong rejection class"
                );
                assert_eq!(
                    next, mempool,
                    "mempool must be UNCHANGED after a rejected tx"
                );
            }
            (ExpectedOutcome::Admit, AdmitOutcome::Rejected { class, error }) => {
                panic!("valid B-track case rejected: {class:?} ({error:?})");
            }
            (ExpectedOutcome::Reject(want), AdmitOutcome::Admitted { tx_id }) => {
                panic!(
                    "FALSE ACCEPT (release-blocking): adversarial case expected Reject({want:?}) admitted as {tx_id:?}"
                );
            }
        }
    }

    assert!(saw_admit >= 1, "expected at least one valid case");
    assert!(saw_reject >= 4, "expected at least 4 adversarial cases");
}

/// CE-N-E-4 (single-peer half): replaying the same ordered ingress trace
/// against the same base ledger produces byte-identical
/// `(MempoolState, [AdmitOutcome])`. Replay the corpus twice and compare.
#[test]
fn ingress_trace_replay_byte_identical() {
    let cases = b_track_corpus_as_ingress(IngressSource::N2N);
    assert!(!cases.is_empty());

    // Build a single trace + a common base by picking the valid case's
    // base ledger and threading every event through it. Adversarial events
    // will reject against this base (which doesn't hold their UTxOs), but
    // the determinism property is what we assert here: same trace, same base
    // -> same outcomes, both runs.
    let base = cases[0].base.clone();
    let events: Vec<IngressEvent> = cases.iter().map(|c| c.event.clone()).collect();

    let (mempool1, outcomes1) = replay_ingress_trace(base.clone(), &events);
    let (mempool2, outcomes2) = replay_ingress_trace(base, &events);

    assert_eq!(
        mempool1, mempool2,
        "replay produced divergent final MempoolState"
    );
    assert_eq!(
        outcomes1, outcomes2,
        "replay produced divergent AdmitOutcome sequence"
    );
}

/// N-E-6: a dependent pair (B spending A's output) admits B against the
/// accumulating state after A is admitted, when routed through
/// `mempool_ingress`. Mirrors the PHASE4-B2 dependent-pair test through
/// the new chokepoint.
#[test]
fn dependent_pair_through_ingress_admits_b_after_a() {
    let pair = build_dependent_pair();
    let events = [
        wrap_as_ingress(IngressSource::N2N, pair.tx_a.clone()),
        wrap_as_ingress(IngressSource::N2N, pair.tx_b.clone()),
    ];

    let (mempool, outcomes) = replay_ingress_trace(pair.ledger, &events);

    assert_eq!(outcomes.len(), 2);
    match &outcomes[0] {
        AdmitOutcome::Admitted { .. } => {}
        AdmitOutcome::Rejected { class, error } => {
            panic!("tx A rejected: {class:?} ({error:?})")
        }
    }
    match &outcomes[1] {
        AdmitOutcome::Admitted { .. } => {}
        AdmitOutcome::Rejected { class, error } => {
            panic!("tx B (dependent on A) rejected: {class:?} ({error:?})")
        }
    }
    assert_eq!(
        mempool.accepted().len(),
        2,
        "both A and B must be admitted in admission order"
    );
}

/// Source-invariance at the trace level: replaying the same ordered bytes
/// trace once under N2N and once under N2C produces byte-identical
/// `(MempoolState, [AdmitOutcome])`.
#[test]
fn ingress_trace_source_invariant_n2n_vs_n2c() {
    let n2n_cases = b_track_corpus_as_ingress(IngressSource::N2N);
    let n2c_cases = b_track_corpus_as_ingress(IngressSource::N2C);
    assert_eq!(n2n_cases.len(), n2c_cases.len());

    let base = n2n_cases[0].base.clone();
    let events_n2n: Vec<IngressEvent> = n2n_cases.iter().map(|c| c.event.clone()).collect();
    let events_n2c: Vec<IngressEvent> = n2c_cases.iter().map(|c| c.event.clone()).collect();

    let (mem_n2n, out_n2n) = replay_ingress_trace(base.clone(), &events_n2n);
    let (mem_n2c, out_n2c) = replay_ingress_trace(base, &events_n2c);

    assert_eq!(mem_n2n, mem_n2c, "N2N/N2C trace replay diverged in MempoolState");
    assert_eq!(out_n2n, out_n2c, "N2N/N2C trace replay diverged in outcomes");
}
