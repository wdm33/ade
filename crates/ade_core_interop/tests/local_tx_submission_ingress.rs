// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// PHASE4-N-E S5: deterministic GREEN bridge from N-A
// LocalTxSubmissionEvents into N-E mempool_ingress. The cross-bridge
// test (n2n_and_n2c_bridges_produce_identical_outcomes) is the
// CE-N-E-7 load-bearing mechanical evidence: same tx bytes routed via
// N2N vs N2C produce byte-identical outcomes.

use ade_core_interop::local_tx_submission::{
    ingest_n2c_events, local_event_to_ingress, ClientAccumulator,
};
use ade_core_interop::tx_submission::ingest_n2n_events;
use ade_ledger::mempool::{AdmitOutcome, IngressSource, MempoolState, PeerId};
use ade_network::n2c::local_tx_submission::{LocalTxSubmissionEvent, TxRejection};
use ade_network::tx_submission::InventoryEvent;
use ade_testkit::mempool::{b_track_corpus_as_ingress, replay_ingress_trace, BTrackCase};
use ade_testkit::tx_validity::build_valid;

#[test]
fn local_event_to_ingress_maps_tx_submitted() {
    let ev = LocalTxSubmissionEvent::TxSubmitted {
        tx_bytes: b"some-tx-cbor".to_vec(),
    };
    let out = local_event_to_ingress(&ev);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].source(), IngressSource::N2C);
    assert_eq!(out[0].tx_bytes(), b"some-tx-cbor");
}

#[test]
fn local_event_to_ingress_other_events_emit_nothing() {
    let accepted = LocalTxSubmissionEvent::TxAccepted;
    let rejected = LocalTxSubmissionEvent::TxRejected {
        rejection: TxRejection(b"reason-bytes".to_vec()),
    };
    for ev in [accepted, rejected] {
        let out = local_event_to_ingress(&ev);
        assert!(out.is_empty(), "non-TxSubmitted event must produce no IngressEvents: {ev:?}");
    }
}

#[test]
fn client_accumulator_round_trip() {
    let client = PeerId(b"cli-instance-1".to_vec());
    let mut acc = ClientAccumulator::new(client.clone());
    assert!(acc.is_empty());

    acc.observe(&LocalTxSubmissionEvent::TxSubmitted {
        tx_bytes: b"tx-0".to_vec(),
    });
    acc.observe(&LocalTxSubmissionEvent::TxAccepted); // ignored
    acc.observe(&LocalTxSubmissionEvent::TxSubmitted {
        tx_bytes: b"tx-1".to_vec(),
    });
    acc.observe(&LocalTxSubmissionEvent::TxRejected {
        rejection: TxRejection(b"reason".to_vec()),
    }); // ignored

    assert_eq!(acc.len(), 2);
    let q = acc.drain();
    assert_eq!(q.peer, client);
    assert_eq!(q.source, IngressSource::N2C);
    assert_eq!(q.txs, vec![b"tx-0".to_vec(), b"tx-1".to_vec()]);
}

#[test]
fn ingest_n2c_events_admits_b_track_corpus() {
    // Same shape as the N2N test: feed the B-track corpus as a single
    // client's submission stream and assert the bridge produces the
    // same (MempoolState, Vec<AdmitOutcome>) as direct replay.
    let valid = build_valid();
    let base = valid.ledger.clone();

    let cases = b_track_corpus_as_ingress(IngressSource::N2C);
    let events: Vec<LocalTxSubmissionEvent> = cases
        .iter()
        .map(|c: &BTrackCase| LocalTxSubmissionEvent::TxSubmitted {
            tx_bytes: c.event.tx_bytes().to_vec(),
        })
        .collect();
    let per_client = vec![(PeerId(b"single-client".to_vec()), events)];

    let (mem_bridge, outcomes_bridge) = ingest_n2c_events(base.clone(), &per_client);

    // Reference: direct replay over the same IngressEvents.
    let manual: Vec<_> = cases.iter().map(|c| c.event.clone()).collect();
    let (mem_ref, outcomes_ref) = replay_ingress_trace(base, &manual);

    assert_eq!(mem_bridge, mem_ref);
    assert_eq!(outcomes_bridge, outcomes_ref);
}

/// CE-N-E-7 mechanical core: the same tx bytes routed via N2N vs N2C
/// produce byte-identical `(MempoolState, Vec<AdmitOutcome>)`.
/// Source-invariance at the wire-event layer.
#[test]
fn n2n_and_n2c_bridges_produce_identical_outcomes() {
    let valid = build_valid();
    let base = valid.ledger.clone();

    // Build a small mixed batch.
    let cases = b_track_corpus_as_ingress(IngressSource::N2N);
    let tx_bytes: Vec<Vec<u8>> = cases
        .iter()
        .map(|c: &BTrackCase| c.event.tx_bytes().to_vec())
        .collect();

    // N2N path: one peer delivering all txs in one TxsDelivered.
    let n2n_events = vec![(
        PeerId(b"peer".to_vec()),
        vec![InventoryEvent::TxsDelivered {
            tx_bytes: tx_bytes.clone(),
        }],
    )];
    let (mem_n2n, out_n2n) = ingest_n2n_events(base.clone(), &n2n_events);

    // N2C path: same peer-id, one TxSubmitted per tx (the natural
    // cardano-cli shape — one submission per invocation).
    let n2c_events = vec![(
        PeerId(b"peer".to_vec()),
        tx_bytes
            .into_iter()
            .map(|b| LocalTxSubmissionEvent::TxSubmitted { tx_bytes: b })
            .collect(),
    )];
    let (mem_n2c, out_n2c) = ingest_n2c_events(base, &n2c_events);

    assert_eq!(
        mem_n2n, mem_n2c,
        "N2N and N2C bridges produced different MempoolState for the same tx bytes"
    );
    assert_eq!(
        out_n2n, out_n2c,
        "N2N and N2C bridges produced different AdmitOutcome sequences for the same tx bytes"
    );
}

/// Multi-client N2C events under two distinct per-client orderings
/// produce byte-identical outcomes.
#[test]
fn multi_client_n2c_canonicalize_deterministically() {
    let case = build_valid();
    let base = case.ledger.clone();
    let tx = case.tx_cbor.clone();

    let ca = PeerId(b"client-alpha".to_vec());
    let cb = PeerId(b"client-bravo".to_vec());
    let cc = PeerId(b"client-charlie".to_vec());

    let mk = |id: PeerId, bytes: Vec<u8>| {
        (
            id,
            vec![LocalTxSubmissionEvent::TxSubmitted { tx_bytes: bytes }],
        )
    };

    let order_1 = vec![
        mk(ca.clone(), tx.clone()),
        mk(cb.clone(), tx.clone()),
        mk(cc.clone(), tx.clone()),
    ];
    let order_2 = vec![
        mk(cc, tx.clone()),
        mk(ca, tx.clone()),
        mk(cb, tx),
    ];

    let (mem_1, out_1) = ingest_n2c_events(base.clone(), &order_1);
    let (mem_2, out_2) = ingest_n2c_events(base, &order_2);

    assert_eq!(mem_1, mem_2);
    assert_eq!(out_1, out_2);
}

#[test]
fn ingest_n2c_empty_input_returns_initial_state() {
    let case = build_valid();
    let (mem, outcomes) = ingest_n2c_events(case.ledger.clone(), &[]);
    assert!(outcomes.is_empty());
    assert_eq!(mem, MempoolState::new(case.ledger));
}

#[test]
fn ingest_n2c_admit_outcome_carried_through() {
    let case = build_valid();
    let base = case.ledger.clone();
    let events = vec![(
        PeerId(b"cli".to_vec()),
        vec![LocalTxSubmissionEvent::TxSubmitted {
            tx_bytes: case.tx_cbor.clone(),
        }],
    )];

    let (_mem, outcomes) = ingest_n2c_events(base, &events);
    assert_eq!(outcomes.len(), 1);
    match &outcomes[0] {
        AdmitOutcome::Admitted { .. } => {}
        AdmitOutcome::Rejected { class, error } => {
            panic!("valid tx rejected through N2C bridge: {class:?} ({error:?})")
        }
    }
}
