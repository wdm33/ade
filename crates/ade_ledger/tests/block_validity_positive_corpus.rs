// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// B1-S6 — positive agreement corpus + replay-equivalence. Drives the BLUE
// `block_validity` authority over the committed Conway-576 corpus.
//
// Oracle: on-chain inclusion — every one of the 14 real mainnet Conway-576
// blocks IS valid. Outcome (observed 2026-05-20): all 14 are fully `Valid`,
// including full body application — none hit the externally-blocked CE-88 /
// aiken Plutus-eval limitation. CE-B1-3 therefore closes outright; no carve-out
// is asserted. Should a future corpus refresh introduce a block whose ONLY
// failure is CE-88 Plutus-eval, this test (and the corpus README) is where the
// carve-out would be named — but no such block exists in the present corpus.
//
// `verdict_stream_replays_identically` asserts determinism (`T-DET-01`)
// independently of Valid/Invalid: the same ordered inputs yield byte-identical
// verdict-surface bytes across two runs.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_ledger::block_validity::BlockValidityVerdict;
use ade_testkit::validity::{replay_block_validity, ConwayValidityCorpus};

const EXPECTED_BLOCK_COUNT: usize = 14;

fn corpus() -> ConwayValidityCorpus {
    ConwayValidityCorpus::load().expect("corpus loads")
}

#[test]
fn corpus_block_count_is_14() {
    // Guards against silent corpus shrinkage.
    assert_eq!(corpus().blocks.len(), EXPECTED_BLOCK_COUNT);
}

#[test]
fn all_corpus_blocks_valid() {
    let corpus = corpus();
    let replays = replay_block_validity(&corpus).expect("replay");
    assert_eq!(replays.len(), EXPECTED_BLOCK_COUNT);

    for (i, r) in replays.iter().enumerate() {
        match &r.verdict {
            BlockValidityVerdict::Valid { .. } => {}
            BlockValidityVerdict::Invalid { class, error } => {
                // A non-CE-88 Invalid is a real disagreement — a correctness
                // bug, not a test to soften (B1-S6 §13/§14). All 14 are fully
                // Valid in the present corpus, so reaching here is a regression.
                panic!(
                    "corpus block {i} (slot/block in README order) must be Valid; \
                     got {class:?}: {error:?}"
                );
            }
        }
    }
}

#[test]
fn verdict_stream_replays_identically() {
    // CE-B1-5 (replay half): the verdict-surface byte stream is byte-identical
    // across two independent runs over the same ordered corpus inputs.
    let corpus = corpus();

    let run_a: Vec<Vec<u8>> = replay_block_validity(&corpus)
        .expect("replay a")
        .into_iter()
        .map(|r| r.surface)
        .collect();
    let run_b: Vec<Vec<u8>> = replay_block_validity(&corpus)
        .expect("replay b")
        .into_iter()
        .map(|r| r.surface)
        .collect();

    assert_eq!(
        run_a, run_b,
        "verdict-surface stream must replay byte-identically"
    );
}
