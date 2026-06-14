// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// PHASE4-N-E S4: deterministic GREEN bridge from N-A InventoryEvents
// into N-E mempool_ingress. Tests synthesize InventoryEvents (no
// live socket); the live half is operator-action per
// docs/clusters/PHASE4-N-E/CE-N-E-6_PROCEDURE.md.

use ade_core_interop::tx_submission::{
    event_to_ingress, ingest_n2n_events, PeerAccumulator,
};
use ade_ledger::mempool::{AdmitOutcome, IngressSource, MempoolState, PeerId};
use ade_network::tx_submission::{InventoryEvent, TxIdAndSize};
use ade_network::codec::tx_submission::TxSubmissionTxId;
use ade_testkit::mempool::{b_track_corpus_as_ingress, replay_ingress_trace, BTrackCase};
use ade_testkit::tx_validity::build_valid;
use ade_types::TxId;

fn id(seed: u8) -> TxId {
    TxId(ade_types::Hash32([seed; 32]))
}

/// Era-tagged tx-submission2 txid (Conway = era 6) for InventoryEvent ids.
fn sub_id(seed: u8) -> TxSubmissionTxId {
    TxSubmissionTxId { era: 6, id: id(seed) }
}

#[test]
fn event_to_ingress_maps_txs_delivered() {
    let event = InventoryEvent::TxsDelivered {
        tx_bytes: vec![b"tx0".to_vec(), b"tx1".to_vec(), b"tx2".to_vec()],
    };
    let events = event_to_ingress(&event, IngressSource::N2N);
    assert_eq!(events.len(), 3);
    for e in &events {
        assert_eq!(e.source(), IngressSource::N2N);
    }
    assert_eq!(events[0].tx_bytes(), b"tx0");
    assert_eq!(events[1].tx_bytes(), b"tx1");
    assert_eq!(events[2].tx_bytes(), b"tx2");
}

#[test]
fn event_to_ingress_other_events_emit_nothing() {
    let opened = InventoryEvent::ServerOpened;
    let ids_req = InventoryEvent::IdsRequested {
        blocking: true,
        ack: 0,
        req: 5,
    };
    let ids_del = InventoryEvent::IdsDelivered {
        entries: vec![TxIdAndSize { tx_id: sub_id(0xAA), size: 100 }],
    };
    let txs_req = InventoryEvent::TxsRequested {
        ids: vec![sub_id(0xBB)],
    };

    for ev in [opened, ids_req, ids_del, txs_req] {
        let out = event_to_ingress(&ev, IngressSource::N2N);
        assert!(
            out.is_empty(),
            "non-tx event must produce no IngressEvents: {ev:?}"
        );
    }
}

#[test]
fn peer_accumulator_round_trip() {
    let peer = PeerId(b"peer-A".to_vec());
    let mut acc = PeerAccumulator::new(peer.clone());
    assert!(acc.is_empty());

    acc.observe(&InventoryEvent::ServerOpened); // ignored
    acc.observe(&InventoryEvent::TxsDelivered {
        tx_bytes: vec![b"a0".to_vec(), b"a1".to_vec()],
    });
    acc.observe(&InventoryEvent::TxsRequested {
        ids: vec![sub_id(0x01)],
    }); // ignored
    acc.observe(&InventoryEvent::TxsDelivered {
        tx_bytes: vec![b"a2".to_vec()],
    });

    assert_eq!(acc.len(), 3);
    let q = acc.drain();
    assert_eq!(q.peer, peer);
    assert_eq!(q.source, IngressSource::N2N);
    assert_eq!(q.txs, vec![b"a0".to_vec(), b"a1".to_vec(), b"a2".to_vec()]);
}

/// Feed the B-track corpus as synthetic InventoryEvent::TxsDelivered
/// streams (one event per case for a single peer) and assert that the
/// bridge produces the same (MempoolState, Vec<AdmitOutcome>) as
/// `replay_ingress_trace` over the manually-built ingress trace.
#[test]
fn ingest_n2n_events_admits_b_track_corpus() {
    // Pick the valid case's base ledger as the shared base; all corpus
    // events are fed against it. Adversarial entries reject because the
    // base does not hold their UTxOs — the assertion below is equality
    // of behavior between the two bridge paths, not their absolute
    // outcomes.
    let valid = build_valid();
    let base = valid.ledger.clone();

    // Build per-peer InventoryEvent streams: one peer, one
    // TxsDelivered per case, in order.
    let cases = b_track_corpus_as_ingress(IngressSource::N2N);
    let tx_bytes: Vec<Vec<u8>> = cases
        .iter()
        .map(|c: &BTrackCase| c.event.tx_bytes().to_vec())
        .collect();
    let events = vec![(
        PeerId(b"single-peer".to_vec()),
        vec![InventoryEvent::TxsDelivered { tx_bytes: tx_bytes.clone() }],
    )];

    let (mem_bridge, outcomes_bridge) = ingest_n2n_events(base.clone(), &events);

    // Reference path: build the same IngressEvent trace directly and
    // replay it.
    let manual: Vec<_> = tx_bytes
        .into_iter()
        .map(|b| ade_ledger::mempool::IngressEvent::new(IngressSource::N2N, b))
        .collect();
    let (mem_ref, outcomes_ref) = replay_ingress_trace(base, &manual);

    assert_eq!(mem_bridge, mem_ref, "bridge MempoolState != reference");
    assert_eq!(outcomes_bridge, outcomes_ref, "bridge outcomes != reference");
}

/// Multi-peer N2N events under two distinct per-peer-stream input
/// orderings produce byte-identical (MempoolState, Vec<AdmitOutcome>).
/// This is the load-bearing CE-N-E-6 mechanical evidence at the
/// adapter layer (live-log evidence is operator-action).
#[test]
fn multi_peer_n2n_events_canonicalize_deterministically() {
    let case = build_valid();
    let base = case.ledger.clone();
    let tx = case.tx_cbor.clone();

    let pa = PeerId(b"alpha".to_vec());
    let pb = PeerId(b"bravo".to_vec());
    let pc = PeerId(b"charlie".to_vec());

    let mk = |id: PeerId, bytes: Vec<u8>| {
        (
            id,
            vec![InventoryEvent::TxsDelivered { tx_bytes: vec![bytes] }],
        )
    };

    let order_1 = vec![
        mk(pa.clone(), tx.clone()),
        mk(pb.clone(), tx.clone()),
        mk(pc.clone(), tx.clone()),
    ];
    let order_2 = vec![
        mk(pc, tx.clone()),
        mk(pa, tx.clone()),
        mk(pb, tx),
    ];

    let (mem_1, out_1) = ingest_n2n_events(base.clone(), &order_1);
    let (mem_2, out_2) = ingest_n2n_events(base, &order_2);

    assert_eq!(mem_1, mem_2, "MempoolState diverged across per-peer orderings");
    assert_eq!(out_1, out_2, "outcomes diverged across per-peer orderings");
}

#[test]
fn ingest_empty_input_returns_empty_mempool() {
    let case = build_valid();
    let (mem, outcomes) = ingest_n2n_events(case.ledger.clone(), &[]);
    assert!(outcomes.is_empty());
    assert_eq!(mem, MempoolState::new(case.ledger));
}

#[test]
fn ingest_admit_outcome_variant_carried_through() {
    let case = build_valid();
    let base = case.ledger.clone();
    let events = vec![(
        PeerId(b"alpha".to_vec()),
        vec![InventoryEvent::TxsDelivered {
            tx_bytes: vec![case.tx_cbor.clone()],
        }],
    )];

    let (_mem, outcomes) = ingest_n2n_events(base, &events);
    assert_eq!(outcomes.len(), 1);
    match &outcomes[0] {
        AdmitOutcome::Admitted { .. } => {}
        AdmitOutcome::Rejected { class, error } => {
            panic!("valid tx rejected through bridge: {class:?} ({error:?})")
        }
    }
}
