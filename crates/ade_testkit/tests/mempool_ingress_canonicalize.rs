// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// PHASE4-N-E S3: cross-check that two distinct interleavings of the same
// per-peer submission queues canonicalize to the same IngressEvent
// sequence AND replay identically through `mempool_ingress` — the
// load-bearing CE-N-E-4 multi-peer evidence.

use ade_ledger::mempool::{
    canonicalize_peer_streams, IngressSource, MempoolState, PeerId, PeerSubmissionQueue,
};
use ade_testkit::mempool::replay_ingress_trace;
use ade_testkit::tx_validity::build_valid;

#[test]
fn two_interleavings_replay_byte_identical() {
    // Build three peers, each carrying the same valid synthetic tx.
    // (Re-using the same tx_cbor three times across peers exercises the
    // canonicalizer's round-robin ordering without requiring divergent
    // ledger states — admission of duplicates after the first is a
    // separate property captured in the existing admit tests; here we
    // only assert byte-identical canonicalization+replay across
    // interleavings.)
    let case = build_valid();
    let base = case.ledger.clone();
    let tx = case.tx_cbor.clone();

    let qa = PeerSubmissionQueue {
        peer: PeerId(b"alpha".to_vec()),
        source: IngressSource::N2N,
        txs: vec![tx.clone()],
    };
    let qb = PeerSubmissionQueue {
        peer: PeerId(b"bravo".to_vec()),
        source: IngressSource::N2N,
        txs: vec![tx.clone()],
    };
    let qc = PeerSubmissionQueue {
        peer: PeerId(b"charlie".to_vec()),
        source: IngressSource::N2N,
        txs: vec![tx.clone()],
    };

    // Two distinct interleavings of the same queues (the input order to
    // the canonicalizer differs; the canonical output must not).
    let events_a = canonicalize_peer_streams(&[qa.clone(), qb.clone(), qc.clone()]);
    let events_b = canonicalize_peer_streams(&[qc.clone(), qa.clone(), qb.clone()]);
    assert_eq!(
        events_a, events_b,
        "two distinct interleavings produced different IngressEvent sequences"
    );

    // Replay both canonical sequences against the same base — must yield
    // byte-identical (MempoolState, Vec<AdmitOutcome>).
    let (mempool_a, outcomes_a) = replay_ingress_trace(base.clone(), &events_a);
    let (mempool_b, outcomes_b) = replay_ingress_trace(base, &events_b);

    assert_eq!(mempool_a, mempool_b, "MempoolState diverged across interleavings");
    assert_eq!(outcomes_a, outcomes_b, "outcomes diverged across interleavings");
}

#[test]
fn empty_pool_canonicalizes_and_replays_to_initial_state() {
    let case = build_valid();
    let events = canonicalize_peer_streams(&[]);
    assert!(events.is_empty());

    let (mempool, outcomes) = replay_ingress_trace(case.ledger.clone(), &events);
    assert!(outcomes.is_empty());
    assert_eq!(mempool, MempoolState::new(case.ledger));
}
