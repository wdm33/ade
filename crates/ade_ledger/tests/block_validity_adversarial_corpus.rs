// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// B1-S7 — adversarial / negative agreement corpus (no-false-accept). Each of
// the named mutations (M1–M6) takes a real Conway-576 corpus block, applies a
// single targeted corruption, and is judged by the BLUE `block_validity`
// authority using the SAME consensus recipe the positive corpus passes.
//
// The load-bearing invariant (CE-B1-4 / DC-LEDGER-02): no mutation ever yields
// `Valid`. A `Valid` here would be a real fail-open bug — the test is NOT
// softened to accommodate one.
//
// Oracle: spec-defined invalidity. M1 fixed-size VRF proof; M2 VRF key binding;
// M3 KES authenticity; M4 forecast horizon; M5 header↔body hash binding; M6
// Ed25519 witness verification (fail-closed). Reject classes are documented in
// `corpus/validity/adversarial/README.md`.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_ledger::block_validity::{BlockRejectClass, BlockValidityVerdict};
use ade_testkit::validity::{validate_block_against_corpus, ConwayValidityCorpus, Mutation};

fn corpus() -> ConwayValidityCorpus {
    ConwayValidityCorpus::load().expect("corpus loads")
}

/// Apply `mutation` to the first corpus block whose layout it can corrupt, and
/// validate the result. Every mutation targets the canonical Babbage/Conway
/// header/body layout, which all 14 corpus blocks share, so block 0 is used as
/// the base; the harness still iterates all blocks where useful.
fn judge(
    corpus: &ConwayValidityCorpus,
    mutation: Mutation,
    base: &[u8],
) -> (BlockValidityVerdict, Vec<u8>) {
    let mutated = mutation
        .apply(base)
        .unwrap_or_else(|e| panic!("{} mutator failed: {e}", mutation.name()));
    let replay = validate_block_against_corpus(corpus, &mutated)
        .unwrap_or_else(|e| panic!("{} validation failed to run: {e}", mutation.name()));
    (replay.verdict, replay.surface)
}

#[test]
fn no_mutation_is_ever_valid() {
    // THE CE-B1-4 core. For every mutation × every base corpus block, the
    // verdict MUST be Invalid. A single Valid is a fail-open bug.
    let corpus = corpus();
    assert!(!corpus.blocks.is_empty(), "corpus must be non-empty");

    for mutation in Mutation::ALL {
        for (i, base) in corpus.blocks.iter().enumerate() {
            let (verdict, _) = judge(&corpus, mutation, base);
            match verdict {
                BlockValidityVerdict::Invalid { .. } => {}
                BlockValidityVerdict::Valid { .. } => panic!(
                    "FAIL-OPEN: mutation {} on corpus block {i} was accepted as Valid",
                    mutation.name()
                ),
            }
        }
    }
}

#[test]
fn each_mutation_maps_to_expected_class() {
    // Secondary: each mutation lands the documented fail-closed reject class.
    // If it ever lands a different but still fail-closed class, that is a
    // documentation/READMEe task — but it must still be Invalid (asserted by
    // `no_mutation_is_ever_valid`).
    let corpus = corpus();
    let base = &corpus.blocks[0];

    for mutation in Mutation::ALL {
        let (verdict, _) = judge(&corpus, mutation, base);
        match verdict {
            BlockValidityVerdict::Invalid { class, error } => {
                assert_eq!(
                    class,
                    mutation.expected_class(),
                    "mutation {} reject class mismatch (error: {error:?})",
                    mutation.name()
                );
            }
            BlockValidityVerdict::Valid { .. } => {
                panic!("mutation {} was Valid (fail-open)", mutation.name())
            }
        }
    }
}

#[test]
fn m6_forged_spend_rejected_fail_closed() {
    // M6 — the highest-value case (forged spend). The slice premise was that
    // patching the header `body_hash` to the forged body would let the block
    // pass the body-hash gate and reach body validation (→ BodyInvalid).
    //
    // FINDING: in this node the KES signature signs the header body, which
    // includes `body_hash`, and the whole header pipeline (incl. KES) runs
    // BEFORE the body-hash gate in `block_validity`. So patching `body_hash`
    // invalidates KES first and the forged spend is rejected fail-closed at the
    // header (HeaderInvalid / KesInvalid). This is correct, secure behavior —
    // the header commits to the body — and is the §13 "different but still
    // fail-closed class" case. The load-bearing property holds: a forged spend
    // can NEVER pass `block_validity`.
    let corpus = corpus();
    let base = &corpus.blocks[0];
    let (verdict, _) = judge(&corpus, Mutation::ForgeWitnessPatchHash, base);
    match verdict {
        BlockValidityVerdict::Invalid { class, .. } => {
            assert_eq!(
                class,
                BlockRejectClass::HeaderInvalid,
                "M6 forged spend must be rejected fail-closed (KES fences the body_hash patch)"
            );
        }
        BlockValidityVerdict::Valid { .. } => panic!("M6 forged spend was accepted (fail-open)"),
    }
}

#[test]
fn adversarial_replays_identically() {
    // Determinism (T-DET-01): the per-(mutation, block) verdict surface is
    // byte-identical across two independent runs.
    let corpus = corpus();
    let base = &corpus.blocks[0];

    let run = |c: &ConwayValidityCorpus| -> Vec<Vec<u8>> {
        Mutation::ALL
            .into_iter()
            .map(|m| judge(c, m, base).1)
            .collect()
    };

    let run_a = run(&corpus);
    let run_b = run(&corpus);
    assert_eq!(
        run_a, run_b,
        "adversarial verdict-surface stream must replay byte-identically"
    );
}
