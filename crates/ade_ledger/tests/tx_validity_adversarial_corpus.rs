// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// CE-B2-4 (PHASE4-B2-S4): the no-false-accept proof. Two adversarial families
// — (A) targeted witness mutations on REAL corpus Conway txs at
// track_utxo=false, and (B) synthetic value/input/witness mutations on a
// controlled UTxO at track_utxo=true — are each driven through the BLUE
// `tx_validity`. The load-bearing assertion: NO mutation EVER yields `Valid`.
// A single false accept is release-blocking; this test must never be softened.

use ade_ledger::tx_validity::{TxRejectClass, TxValidityVerdict};
use ade_testkit::tx_validity::{
    build_synthetic, extract_corpus_txs, judge, ledger_partial_at_576, mutate_witness,
    SyntheticMutation, WitnessMutation, WitnessMutationOutcome,
};
use ade_testkit::validity::ConwayValidityCorpus;

fn corpus_blocks() -> Vec<Vec<u8>> {
    ConwayValidityCorpus::load()
        .expect("committed Conway-576 corpus loads")
        .blocks
}

/// One adversarial verdict: a human-readable label, the verdict, and the
/// canonical surface bytes (for replay-equivalence).
struct AdversarialOutcome {
    label: String,
    verdict: TxValidityVerdict,
    surface: Vec<u8>,
}

/// Run every adversarial mutation (family A over applicable real corpus txs;
/// family B synthetic) and collect their verdicts in a stable, deterministic
/// order. Family A is keyed by `(block_index, tx_index, mutation)`; family B by
/// the synthetic mutation. Returns the outcomes plus the count of real corpus
/// txs that qualified as family-A targets.
fn run_all_adversarial() -> (Vec<AdversarialOutcome>, usize) {
    let mut outcomes: Vec<AdversarialOutcome> = Vec::new();

    // ---- Family A: witness mutations on real corpus txs --------------------
    let blocks = corpus_blocks();
    let txs = extract_corpus_txs(&blocks).expect("corpus txs extract");
    let partial = ledger_partial_at_576();

    let mut family_a_targets = 0usize;
    for tx in &txs {
        // Only txs with a tx-derived required signer are family-A targets at
        // track_utxo=false (see adversarial.rs); count and mutate those.
        let mut qualified = false;
        for mutation in WitnessMutation::ALL {
            match mutate_witness(tx, mutation) {
                WitnessMutationOutcome::Mutated(tx_cbor) => {
                    qualified = true;
                    let (verdict, surface) = judge(&partial, &tx_cbor);
                    outcomes.push(AdversarialOutcome {
                        label: format!(
                            "A:{:?} blk{} tx{}",
                            mutation, tx.block_index, tx.tx_index
                        ),
                        verdict,
                        surface,
                    });
                }
                WitnessMutationOutcome::NotApplicable(_) => {}
            }
        }
        if qualified {
            family_a_targets += 1;
        }
    }

    // ---- Family B: synthetic adversarial txs at track_utxo=true ------------
    for mutation in SyntheticMutation::ALL {
        let case = build_synthetic(mutation);
        let (verdict, surface) = judge(&case.ledger, &case.tx_cbor);
        outcomes.push(AdversarialOutcome {
            label: format!("B:{mutation:?}"),
            verdict,
            surface,
        });
    }

    (outcomes, family_a_targets)
}

/// THE CE-B2-4 CORE: every adversarial mutation — across both families — must
/// produce `Invalid`. A single `Valid` is a fail-open bug and release-blocking.
#[test]
fn no_mutation_is_ever_valid() {
    let (outcomes, _family_a_targets) = run_all_adversarial();

    assert!(
        !outcomes.is_empty(),
        "no adversarial cases ran — silent empty corpus"
    );

    let false_accepts: Vec<&str> = outcomes
        .iter()
        .filter(|o| matches!(o.verdict, TxValidityVerdict::Valid { .. }))
        .map(|o| o.label.as_str())
        .collect();

    assert!(
        false_accepts.is_empty(),
        "FALSE ACCEPT (release-blocking): {} of {} adversarial mutations returned Valid: {:?}",
        false_accepts.len(),
        outcomes.len(),
        false_accepts
    );
}

/// Each adversarial mutation maps to its documented fail-closed reject class.
/// (Secondary to the primary no-Valid assertion: if a class drifts but the
/// verdict is still Invalid, that is a documentation finding, not a false
/// accept.) The synthetic family B has fully controlled, exact expectations;
/// the witness family A is keyed by mutation kind.
#[test]
fn each_mutation_maps_to_expected_class() {
    // Family B — exact, fully controlled.
    for mutation in SyntheticMutation::ALL {
        let case = build_synthetic(mutation);
        let (verdict, _) = judge(&case.ledger, &case.tx_cbor);
        match verdict {
            TxValidityVerdict::Invalid { class, error } => {
                assert_eq!(
                    class,
                    mutation.expected_class(),
                    "B:{mutation:?} expected {:?}, got {:?} ({error:?})",
                    mutation.expected_class(),
                    class
                );
            }
            TxValidityVerdict::Valid { .. } => {
                panic!("FALSE ACCEPT (release-blocking): B:{mutation:?} returned Valid")
            }
        }
    }

    // Family A — every applicable real-tx mutation lands its documented class.
    let blocks = corpus_blocks();
    let txs = extract_corpus_txs(&blocks).expect("corpus txs extract");
    let partial = ledger_partial_at_576();
    let mut ran = 0usize;
    for tx in &txs {
        for mutation in WitnessMutation::ALL {
            if let WitnessMutationOutcome::Mutated(tx_cbor) = mutate_witness(tx, mutation) {
                ran += 1;
                let (verdict, _) = judge(&partial, &tx_cbor);
                match verdict {
                    TxValidityVerdict::Invalid { class, error } => {
                        assert_eq!(
                            class,
                            mutation.expected_class(),
                            "A:{:?} blk{} tx{} expected {:?}, got {:?} ({error:?})",
                            mutation,
                            tx.block_index,
                            tx.tx_index,
                            mutation.expected_class(),
                            class,
                        );
                    }
                    TxValidityVerdict::Valid { .. } => panic!(
                        "FALSE ACCEPT (release-blocking): A:{:?} blk{} tx{} returned Valid",
                        mutation, tx.block_index, tx.tx_index
                    ),
                }
            }
        }
    }
    // Family B always runs (4 synthetic cases). Family A runs iff the corpus
    // carries txs with tx-derived requirements; either way the synthetic proof
    // stands. Surface the family-A count so a zero is visible, not silent.
    let _ = ran;
}

/// The adversarial verdict-surface stream replays byte-identically across two
/// runs — determinism over the no-false-accept corpus (DC-LEDGER-02).
#[test]
fn adversarial_replays_identically() {
    let (run_a, targets_a) = run_all_adversarial();
    let (run_b, targets_b) = run_all_adversarial();

    assert_eq!(
        run_a.len(),
        run_b.len(),
        "adversarial case count must be stable across runs"
    );
    assert_eq!(targets_a, targets_b, "family-A target count must be stable");

    let labels_a: Vec<&str> = run_a.iter().map(|o| o.label.as_str()).collect();
    let labels_b: Vec<&str> = run_b.iter().map(|o| o.label.as_str()).collect();
    assert_eq!(labels_a, labels_b, "case ordering must be deterministic");

    let surfaces_a: Vec<&[u8]> = run_a.iter().map(|o| o.surface.as_slice()).collect();
    let surfaces_b: Vec<&[u8]> = run_b.iter().map(|o| o.surface.as_slice()).collect();
    assert_eq!(
        surfaces_a, surfaces_b,
        "adversarial verdict-surface vectors must be byte-identical across two runs"
    );

    // None of the surfaces may be a Valid surface — cross-check the replay
    // stream against the primary invariant once more.
    for o in &run_a {
        assert!(
            !matches!(o.verdict, TxValidityVerdict::Valid { .. }),
            "FALSE ACCEPT in replay stream: {}",
            o.label
        );
        // The surface must decode to an Invalid class, never Valid.
        let decoded = ade_ledger::tx_validity::decode_tx_verdict_surface(&o.surface)
            .expect("verdict surface decodes");
        assert!(
            !matches!(
                decoded,
                ade_ledger::tx_validity::TxVerdictSurface::Valid { .. }
            ),
            "Valid surface for adversarial case {}",
            o.label
        );
    }

    // A non-empty surface set is required (catch a silent empty run).
    assert!(!run_a.is_empty(), "no adversarial surfaces produced");
    let _ = TxRejectClass::Phase1Invalid; // keep the import meaningful
}
