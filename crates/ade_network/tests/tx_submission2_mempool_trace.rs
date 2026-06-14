// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Integration test for S-A6: drives the tx-submission2 transition
// through 10 curated synthetic mempool scenarios and asserts:
//   1. The emitted `InventoryEvent` sequence matches the spec-derived
//      expected sequence for each scenario.
//   2. The event sequence is deterministic — replaying any scenario
//      1000 times yields byte-identical event traces (including the
//      opaque `tx_bytes` payloads delivered by `TxsDelivered`).
//
// This closes the state-machine-correctness portion of CE-N-A-4.
// Real-capture verification against corpus/network/n2n/tx_submission2/
// follows in S-A9; the test bodies stay unchanged, only the input
// scripts change source (synthetic vs. captured frames).

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_network::codec::tx_submission::{TxIdAndSize, TxSubmission2Message, TxSubmissionTxId};
use ade_network::codec::version::TxSubmission2Version;
use ade_network::tx_submission::{
    tx_submission2_transition, InventoryEvent, TxSubmission2Agency, TxSubmission2Output,
    TxSubmission2State,
};
use ade_types::{Hash32, TxId};

fn v() -> TxSubmission2Version {
    TxSubmission2Version::new(13)
}

fn tx_id(seed: u8) -> TxSubmissionTxId {
    TxSubmissionTxId { era: 6, id: TxId(Hash32([seed; 32])) }
}

/// Synthetic tx body deterministically derived from (seed, size). The
/// 2-byte prefix mimics a CBOR-ish tag; the trailing fill makes
/// payloads distinguishable across scenarios while staying
/// byte-deterministic across replays.
fn tx_body(seed: u8, size: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size);
    buf.extend_from_slice(&[0x82, seed]);
    let fill = seed.wrapping_add(0x33);
    while buf.len() < size {
        buf.push(fill);
    }
    buf.truncate(size);
    buf
}

fn entry(seed: u8, size: u32) -> TxIdAndSize {
    TxIdAndSize {
        tx_id: tx_id(seed),
        size,
    }
}

type Step = (TxSubmission2Agency, TxSubmission2Message);

/// Drive a scenario through the state machine and collect every
/// emitted inventory event in order. Done outputs are not events;
/// `TxSubmission2Output::Done` is discarded — the integration test
/// asserts on event traces only.
fn drive(steps: &[Step]) -> Vec<InventoryEvent> {
    let mut state = TxSubmission2State::Init;
    let mut events = Vec::new();
    for (agency, msg) in steps {
        let (next, out) = tx_submission2_transition(state, *agency, v(), msg.clone())
            .expect("legal transition");
        state = next;
        if let TxSubmission2Output::Event(ev) = out {
            events.push(ev);
        }
    }
    events
}

// ---------------------------------------------------------------------------
// Scenarios — each function returns (script, expected_event_trace).
// All scenarios begin from `TxSubmission2State::Init`; the first step
// is always (Client, Init) so they share the standard handshake.
// ---------------------------------------------------------------------------

fn init() -> (Step, InventoryEvent) {
    (
        (TxSubmission2Agency::Client, TxSubmission2Message::Init),
        InventoryEvent::ServerOpened,
    )
}

fn done() -> (Step, ()) {
    (
        (TxSubmission2Agency::Server, TxSubmission2Message::Done),
        (),
    )
}

/// Scenario 1: server requests a blocking inventory; client advertises
/// some IDs; server fetches all of them; client delivers bodies; done.
fn single_round_blocking_ad_then_fetch() -> (Vec<Step>, Vec<InventoryEvent>) {
    let (init_step, init_ev) = init();
    let (done_step, _) = done();
    let entries = vec![entry(0x01, 200), entry(0x02, 250), entry(0x03, 300)];
    let ids: Vec<TxSubmissionTxId> = entries.iter().map(|e| e.tx_id.clone()).collect();
    let bodies = vec![tx_body(0x01, 200), tx_body(0x02, 250), tx_body(0x03, 300)];
    let steps = vec![
        init_step,
        (
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxIds {
                blocking: true,
                ack: 0,
                req: 5,
            },
        ),
        (
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxIds(entries.clone()),
        ),
        (
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxs(ids.clone()),
        ),
        (
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxs(bodies.clone()),
        ),
        done_step,
    ];
    let expected = vec![
        init_ev,
        InventoryEvent::IdsRequested {
            blocking: true,
            ack: 0,
            req: 5,
        },
        InventoryEvent::IdsDelivered { entries },
        InventoryEvent::TxsRequested { ids },
        InventoryEvent::TxsDelivered { tx_bytes: bodies },
    ];
    (steps, expected)
}

/// Scenario 2: three rounds of non-blocking inventory advertisement,
/// no body fetches, then Done. Exercises Idle re-entry across multiple
/// inventory rounds.
fn multi_round_inventory_advertisement() -> (Vec<Step>, Vec<InventoryEvent>) {
    let (init_step, init_ev) = init();
    let (done_step, _) = done();
    let rounds = [
        (vec![entry(0x10, 100), entry(0x11, 150)], 2u16, 0u16),
        (vec![entry(0x20, 200)], 1, 2),
        (vec![entry(0x30, 300), entry(0x31, 350), entry(0x32, 400)], 3, 1),
    ];
    let mut steps = vec![init_step];
    let mut expected = vec![init_ev];
    for (entries, req, ack) in rounds.iter() {
        steps.push((
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxIds {
                blocking: false,
                ack: *ack,
                req: *req,
            },
        ));
        expected.push(InventoryEvent::IdsRequested {
            blocking: false,
            ack: *ack,
            req: *req,
        });
        steps.push((
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxIds(entries.clone()),
        ));
        expected.push(InventoryEvent::IdsDelivered {
            entries: entries.clone(),
        });
    }
    steps.push(done_step);
    (steps, expected)
}

/// Scenario 3: single tx fetch. Server asks for one tx, client delivers
/// exactly one body.
fn single_tx_fetch() -> (Vec<Step>, Vec<InventoryEvent>) {
    let (init_step, init_ev) = init();
    let (done_step, _) = done();
    let id = tx_id(0x42);
    let body = tx_body(0x42, 512);
    let steps = vec![
        init_step,
        (
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxs(vec![id.clone()]),
        ),
        (
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxs(vec![body.clone()]),
        ),
        done_step,
    ];
    let expected = vec![
        init_ev,
        InventoryEvent::TxsRequested { ids: vec![id] },
        InventoryEvent::TxsDelivered {
            tx_bytes: vec![body],
        },
    ];
    (steps, expected)
}

/// Scenario 4: server requests 5 txs, client only delivers 3 (some
/// requested IDs unavailable — partial reply is grammatically legal).
fn multi_tx_fetch_partial_reply() -> (Vec<Step>, Vec<InventoryEvent>) {
    let (init_step, init_ev) = init();
    let (done_step, _) = done();
    let ids: Vec<TxSubmissionTxId> = (0u8..5).map(|i| tx_id(i + 0x50)).collect();
    let bodies = vec![tx_body(0x50, 64), tx_body(0x51, 128), tx_body(0x52, 256)];
    let steps = vec![
        init_step,
        (
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxs(ids.clone()),
        ),
        (
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxs(bodies.clone()),
        ),
        done_step,
    ];
    let expected = vec![
        init_ev,
        InventoryEvent::TxsRequested { ids },
        InventoryEvent::TxsDelivered { tx_bytes: bodies },
    ];
    (steps, expected)
}

/// Scenario 5: blocking call with the server's full request (req=10,
/// reply=10). Exercises the "exactly at the limit" boundary on
/// ReplyTxIds count.
fn blocking_full_reply() -> (Vec<Step>, Vec<InventoryEvent>) {
    let (init_step, init_ev) = init();
    let (done_step, _) = done();
    let entries: Vec<TxIdAndSize> =
        (0u8..10).map(|i| entry(i + 0x60, 100 + i as u32)).collect();
    let steps = vec![
        init_step,
        (
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxIds {
                blocking: true,
                ack: 0,
                req: 10,
            },
        ),
        (
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxIds(entries.clone()),
        ),
        done_step,
    ];
    let expected = vec![
        init_ev,
        InventoryEvent::IdsRequested {
            blocking: true,
            ack: 0,
            req: 10,
        },
        InventoryEvent::IdsDelivered { entries },
    ];
    (steps, expected)
}

/// Scenario 6: blocking then non-blocking call mixed in one session.
fn blocking_then_non_blocking_call() -> (Vec<Step>, Vec<InventoryEvent>) {
    let (init_step, init_ev) = init();
    let (done_step, _) = done();
    let blocking_entries = vec![entry(0x70, 100), entry(0x71, 200)];
    let non_blocking_entries = vec![entry(0x72, 150)];
    let steps = vec![
        init_step,
        (
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxIds {
                blocking: true,
                ack: 0,
                req: 4,
            },
        ),
        (
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxIds(blocking_entries.clone()),
        ),
        (
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxIds {
                blocking: false,
                ack: 2,
                req: 3,
            },
        ),
        (
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxIds(non_blocking_entries.clone()),
        ),
        done_step,
    ];
    let expected = vec![
        init_ev,
        InventoryEvent::IdsRequested {
            blocking: true,
            ack: 0,
            req: 4,
        },
        InventoryEvent::IdsDelivered {
            entries: blocking_entries,
        },
        InventoryEvent::IdsRequested {
            blocking: false,
            ack: 2,
            req: 3,
        },
        InventoryEvent::IdsDelivered {
            entries: non_blocking_entries,
        },
    ];
    (steps, expected)
}

/// Scenario 7: server immediately terminates after the Init handshake
/// without any inventory exchange. Init -> Idle -> Done.
fn immediate_done() -> (Vec<Step>, Vec<InventoryEvent>) {
    let (init_step, init_ev) = init();
    let (done_step, _) = done();
    (vec![init_step, done_step], vec![init_ev])
}

/// Scenario 8: ad-fetch-ad-fetch alternation — exercises Idle re-entry
/// between mixed inventory and body rounds.
fn interleaved_request_reply() -> (Vec<Step>, Vec<InventoryEvent>) {
    let (init_step, init_ev) = init();
    let (done_step, _) = done();
    let ad1 = vec![entry(0x80, 100), entry(0x81, 110)];
    let fetch1_ids = vec![tx_id(0x80)];
    let fetch1_bodies = vec![tx_body(0x80, 100)];
    let ad2 = vec![entry(0x82, 120)];
    let fetch2_ids = vec![tx_id(0x82)];
    let fetch2_bodies = vec![tx_body(0x82, 120)];
    let steps = vec![
        init_step,
        (
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxIds {
                blocking: false,
                ack: 0,
                req: 2,
            },
        ),
        (
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxIds(ad1.clone()),
        ),
        (
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxs(fetch1_ids.clone()),
        ),
        (
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxs(fetch1_bodies.clone()),
        ),
        (
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxIds {
                blocking: false,
                ack: 1,
                req: 2,
            },
        ),
        (
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxIds(ad2.clone()),
        ),
        (
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxs(fetch2_ids.clone()),
        ),
        (
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxs(fetch2_bodies.clone()),
        ),
        done_step,
    ];
    let expected = vec![
        init_ev,
        InventoryEvent::IdsRequested {
            blocking: false,
            ack: 0,
            req: 2,
        },
        InventoryEvent::IdsDelivered { entries: ad1 },
        InventoryEvent::TxsRequested { ids: fetch1_ids },
        InventoryEvent::TxsDelivered {
            tx_bytes: fetch1_bodies,
        },
        InventoryEvent::IdsRequested {
            blocking: false,
            ack: 1,
            req: 2,
        },
        InventoryEvent::IdsDelivered { entries: ad2 },
        InventoryEvent::TxsRequested { ids: fetch2_ids },
        InventoryEvent::TxsDelivered {
            tx_bytes: fetch2_bodies,
        },
    ];
    (steps, expected)
}

/// Scenario 9: server fetches a mixed-size batch (small + large + small)
/// in a single round; asserts heterogeneous payloads pass through
/// byte-identically.
fn varying_tx_body_sizes() -> (Vec<Step>, Vec<InventoryEvent>) {
    let (init_step, init_ev) = init();
    let (done_step, _) = done();
    let ids = vec![tx_id(0x90), tx_id(0x91), tx_id(0x92)];
    let bodies = vec![
        tx_body(0x90, 32),
        tx_body(0x91, 16_384),
        tx_body(0x92, 64),
    ];
    let steps = vec![
        init_step,
        (
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxs(ids.clone()),
        ),
        (
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxs(bodies.clone()),
        ),
        done_step,
    ];
    let expected = vec![
        init_ev,
        InventoryEvent::TxsRequested { ids },
        InventoryEvent::TxsDelivered { tx_bytes: bodies },
    ];
    (steps, expected)
}

/// Scenario 10: boundary case — non-blocking RequestTxIds with req=0
/// and ack=0, immediately followed by an empty reply, then Done. The
/// non-blocking grammar permits empty replies; req=0 is grammatically
/// at the lower boundary.
fn zero_ack_zero_req_then_done() -> (Vec<Step>, Vec<InventoryEvent>) {
    let (init_step, init_ev) = init();
    let (done_step, _) = done();
    let steps = vec![
        init_step,
        (
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxIds {
                blocking: false,
                ack: 0,
                req: 0,
            },
        ),
        (
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxIds(Vec::new()),
        ),
        done_step,
    ];
    let expected = vec![
        init_ev,
        InventoryEvent::IdsRequested {
            blocking: false,
            ack: 0,
            req: 0,
        },
        InventoryEvent::IdsDelivered {
            entries: Vec::new(),
        },
    ];
    (steps, expected)
}

fn scenarios() -> Vec<(&'static str, Vec<Step>, Vec<InventoryEvent>)> {
    vec![
        {
            let (s, e) = single_round_blocking_ad_then_fetch();
            ("single_round_blocking_ad_then_fetch", s, e)
        },
        {
            let (s, e) = multi_round_inventory_advertisement();
            ("multi_round_inventory_advertisement", s, e)
        },
        {
            let (s, e) = single_tx_fetch();
            ("single_tx_fetch", s, e)
        },
        {
            let (s, e) = multi_tx_fetch_partial_reply();
            ("multi_tx_fetch_partial_reply", s, e)
        },
        {
            let (s, e) = blocking_full_reply();
            ("blocking_full_reply", s, e)
        },
        {
            let (s, e) = blocking_then_non_blocking_call();
            ("blocking_then_non_blocking_call", s, e)
        },
        {
            let (s, e) = immediate_done();
            ("immediate_done", s, e)
        },
        {
            let (s, e) = interleaved_request_reply();
            ("interleaved_request_reply", s, e)
        },
        {
            let (s, e) = varying_tx_body_sizes();
            ("varying_tx_body_sizes", s, e)
        },
        {
            let (s, e) = zero_ack_zero_req_then_done();
            ("zero_ack_zero_req_then_done", s, e)
        },
    ]
}

#[test]
fn tx_submission2_mempool_trace() {
    let cases = scenarios();
    assert!(cases.len() >= 10, "S-A6 requires ≥10 scenarios");

    // Pass 1: every scenario's emitted event trace matches its
    // spec-derived expected sequence.
    for (name, steps, expected) in &cases {
        let actual = drive(steps);
        assert_eq!(
            actual, *expected,
            "scenario {name}: event trace mismatched expected"
        );
    }

    // Pass 2: 1000-run determinism. Each scenario replays through the
    // state machine and must produce a byte-identical event trace
    // every iteration (T-DET-01 + DC-PROTO-01), including the opaque
    // TxsDelivered.tx_bytes payloads.
    for (name, steps, _) in &cases {
        let mut first: Option<Vec<InventoryEvent>> = None;
        for _ in 0..1000 {
            let trace = drive(steps);
            match &first {
                None => first = Some(trace),
                Some(prev) => assert_eq!(*prev, trace, "scenario {name}: event trace drifted"),
            }
        }
    }
}
