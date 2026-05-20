// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// CE-B2-3 (PHASE4-B2-S3): every real on-chain Conway tx extracted from the
// committed corpus blocks is judged by the BLUE `tx_validity` authority, and
// the per-tx verdict stream replays byte-identically across two runs.
//
// Oracle: on-chain inclusion (a tx in a real Conway block is valid). Scope is
// the same as B1's positive corpus — `track_utxo = false`: structural +
// supplied-witness sig validity + tx-derived required-signer coverage. Full
// UTxO-resolved validation (input-credential coverage, value/fee balance) needs
// extracted UTxOs and is out of scope (proven synthetically in B2-S1).

use ade_testkit::tx_validity::{replay_tx_validity, TxReplay};
use ade_testkit::validity::ConwayValidityCorpus;
use ade_ledger::tx_validity::{TxRejectClass, TxValidityVerdict};

fn corpus_blocks() -> Vec<Vec<u8>> {
    ConwayValidityCorpus::load()
        .expect("committed Conway-576 corpus loads")
        .blocks
}

fn replay() -> Vec<TxReplay> {
    replay_tx_validity(&corpus_blocks()).expect("every corpus block extracts + validates")
}

#[test]
fn corpus_tx_count_nonzero() {
    let replays = replay();
    assert!(
        !replays.is_empty(),
        "extraction produced zero txs — silent empty extraction"
    );
}

#[test]
fn all_corpus_txs_valid() {
    let replays = replay();

    let mut valid = 0usize;
    let mut other_invalid: Vec<(usize, usize, TxRejectClass)> = Vec::new();

    for r in &replays {
        match &r.verdict {
            TxValidityVerdict::Valid { .. } => valid += 1,
            TxValidityVerdict::Invalid { class, .. } => {
                other_invalid.push((r.block_index, r.tx_index, *class));
            }
        }
    }

    assert!(
        other_invalid.is_empty(),
        "{} of {} real on-chain Conway txs returned Invalid (non-CE-88): {:?}",
        other_invalid.len(),
        replays.len(),
        other_invalid
    );
    assert_eq!(
        valid,
        replays.len(),
        "every extracted real Conway tx must be Valid"
    );
}

#[test]
fn tx_verdict_stream_replays_identically() {
    let blocks = corpus_blocks();
    let run_a = replay_tx_validity(&blocks).expect("run A");
    let run_b = replay_tx_validity(&blocks).expect("run B");

    assert_eq!(run_a.len(), run_b.len(), "tx count must be stable across runs");
    let surfaces_a: Vec<&[u8]> = run_a.iter().map(|r| r.surface.as_slice()).collect();
    let surfaces_b: Vec<&[u8]> = run_b.iter().map(|r| r.surface.as_slice()).collect();
    assert_eq!(
        surfaces_a, surfaces_b,
        "verdict-surface vectors must be byte-identical across two runs"
    );
}
