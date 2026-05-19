// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Integration test for S-A8 (LocalStateQuery portion): drives the N2C
// LocalStateQuery transition through curated synthetic scenarios and
// asserts:
//   1. The emitted `LocalStateQueryEvent` sequence matches the
//      spec-derived expected sequence for each scenario.
//   2. The event sequence is deterministic — replaying any scenario
//      1000 times yields byte-identical event traces (including the
//      opaque `QueryPayload` and `ResultPayload` bytes).
//
// Real-capture verification against captured N2C frames follows in
// S-A9; the test bodies stay unchanged.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_network::codec::local_state_query::LocalStateQueryMessage;
use ade_network::codec::version::LocalStateQueryVersion;
use ade_network::n2c::local_state_query::{
    local_state_query_transition, AcquireFailure, LocalStateQueryAgency, LocalStateQueryEvent,
    LocalStateQueryOutput, LocalStateQueryState, Point, QueryPayload, ResultPayload,
};
use ade_types::{Hash32, SlotNo};

fn v() -> LocalStateQueryVersion {
    LocalStateQueryVersion::new(16)
}

fn block_point(slot: u64, seed: u8) -> Point {
    Point::Block {
        slot: SlotNo(slot),
        hash: Hash32([seed; 32]),
    }
}

fn query_body(seed: u8, size: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size);
    buf.extend_from_slice(&[0x82, seed]);
    let fill = seed.wrapping_add(0x55);
    while buf.len() < size {
        buf.push(fill);
    }
    buf.truncate(size);
    buf
}

fn result_body(seed: u8, size: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size);
    buf.extend_from_slice(&[0x83, seed]);
    let fill = seed.wrapping_add(0xAA);
    while buf.len() < size {
        buf.push(fill);
    }
    buf.truncate(size);
    buf
}

type Step = (LocalStateQueryAgency, LocalStateQueryMessage);

fn drive(steps: &[Step]) -> Vec<LocalStateQueryEvent> {
    let mut state = LocalStateQueryState::Idle;
    let mut events = Vec::new();
    for (agency, msg) in steps {
        let (next, out) =
            local_state_query_transition(state, *agency, v(), msg.clone()).expect("legal");
        state = next;
        if let LocalStateQueryOutput::Event(ev) = out {
            events.push(ev);
        }
    }
    events
}

fn acquire_query_release() -> (Vec<Step>, Vec<LocalStateQueryEvent>) {
    let pt = block_point(1000, 0x11);
    let q = query_body(0x11, 64);
    let r = result_body(0x11, 96);
    let steps = vec![
        (
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Acquire(pt.clone()),
        ),
        (
            LocalStateQueryAgency::Server,
            LocalStateQueryMessage::Acquired,
        ),
        (
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Query(QueryPayload(q.clone())),
        ),
        (
            LocalStateQueryAgency::Server,
            LocalStateQueryMessage::Result(ResultPayload(r.clone())),
        ),
        (
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Release,
        ),
    ];
    let expected = vec![
        LocalStateQueryEvent::AcquireRequested { point: Some(pt) },
        LocalStateQueryEvent::SnapshotAcquired,
        LocalStateQueryEvent::QueryRequested {
            payload: QueryPayload(q),
        },
        LocalStateQueryEvent::QueryReplied {
            payload: ResultPayload(r),
        },
        LocalStateQueryEvent::SnapshotReleased,
    ];
    (steps, expected)
}

fn acquire_then_failure() -> (Vec<Step>, Vec<LocalStateQueryEvent>) {
    let pt = block_point(99, 0xAA);
    let steps = vec![
        (
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Acquire(pt.clone()),
        ),
        (
            LocalStateQueryAgency::Server,
            LocalStateQueryMessage::Failure(AcquireFailure::PointTooOld),
        ),
    ];
    let expected = vec![
        LocalStateQueryEvent::AcquireRequested { point: Some(pt) },
        LocalStateQueryEvent::AcquireFailed {
            reason: AcquireFailure::PointTooOld,
        },
    ];
    (steps, expected)
}

fn re_acquire_chain() -> (Vec<Step>, Vec<LocalStateQueryEvent>) {
    let pt_a = block_point(2000, 0x22);
    let pt_b = block_point(2100, 0x23);
    let steps = vec![
        (
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Acquire(pt_a.clone()),
        ),
        (
            LocalStateQueryAgency::Server,
            LocalStateQueryMessage::Acquired,
        ),
        (
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::ReAcquire(pt_b.clone()),
        ),
        (
            LocalStateQueryAgency::Server,
            LocalStateQueryMessage::Acquired,
        ),
        (
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Release,
        ),
    ];
    let expected = vec![
        LocalStateQueryEvent::AcquireRequested { point: Some(pt_a) },
        LocalStateQueryEvent::SnapshotAcquired,
        LocalStateQueryEvent::ReAcquireRequested { point: Some(pt_b) },
        LocalStateQueryEvent::SnapshotAcquired,
        LocalStateQueryEvent::SnapshotReleased,
    ];
    (steps, expected)
}

fn multiple_queries_same_snapshot() -> (Vec<Step>, Vec<LocalStateQueryEvent>) {
    let q1 = query_body(0x31, 32);
    let r1 = result_body(0x31, 48);
    let q2 = query_body(0x32, 64);
    let r2 = result_body(0x32, 64);
    let q3 = query_body(0x33, 16);
    let r3 = result_body(0x33, 16);
    let steps = vec![
        (
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::AcquireNoPoint,
        ),
        (
            LocalStateQueryAgency::Server,
            LocalStateQueryMessage::Acquired,
        ),
        (
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Query(QueryPayload(q1.clone())),
        ),
        (
            LocalStateQueryAgency::Server,
            LocalStateQueryMessage::Result(ResultPayload(r1.clone())),
        ),
        (
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Query(QueryPayload(q2.clone())),
        ),
        (
            LocalStateQueryAgency::Server,
            LocalStateQueryMessage::Result(ResultPayload(r2.clone())),
        ),
        (
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Query(QueryPayload(q3.clone())),
        ),
        (
            LocalStateQueryAgency::Server,
            LocalStateQueryMessage::Result(ResultPayload(r3.clone())),
        ),
        (
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Release,
        ),
    ];
    let expected = vec![
        LocalStateQueryEvent::AcquireRequested { point: None },
        LocalStateQueryEvent::SnapshotAcquired,
        LocalStateQueryEvent::QueryRequested {
            payload: QueryPayload(q1),
        },
        LocalStateQueryEvent::QueryReplied {
            payload: ResultPayload(r1),
        },
        LocalStateQueryEvent::QueryRequested {
            payload: QueryPayload(q2),
        },
        LocalStateQueryEvent::QueryReplied {
            payload: ResultPayload(r2),
        },
        LocalStateQueryEvent::QueryRequested {
            payload: QueryPayload(q3),
        },
        LocalStateQueryEvent::QueryReplied {
            payload: ResultPayload(r3),
        },
        LocalStateQueryEvent::SnapshotReleased,
    ];
    (steps, expected)
}

fn acquire_done_terminates_from_acquired() -> (Vec<Step>, Vec<LocalStateQueryEvent>) {
    let steps = vec![
        (
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::AcquireNoPoint,
        ),
        (
            LocalStateQueryAgency::Server,
            LocalStateQueryMessage::Acquired,
        ),
        (LocalStateQueryAgency::Client, LocalStateQueryMessage::Done),
    ];
    let expected = vec![
        LocalStateQueryEvent::AcquireRequested { point: None },
        LocalStateQueryEvent::SnapshotAcquired,
    ];
    (steps, expected)
}

fn immediate_client_done() -> (Vec<Step>, Vec<LocalStateQueryEvent>) {
    let steps = vec![(LocalStateQueryAgency::Client, LocalStateQueryMessage::Done)];
    (steps, Vec::new())
}

fn point_not_on_chain_failure() -> (Vec<Step>, Vec<LocalStateQueryEvent>) {
    let pt = block_point(5000, 0x88);
    let steps = vec![
        (
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Acquire(pt.clone()),
        ),
        (
            LocalStateQueryAgency::Server,
            LocalStateQueryMessage::Failure(AcquireFailure::PointNotOnChain),
        ),
    ];
    let expected = vec![
        LocalStateQueryEvent::AcquireRequested { point: Some(pt) },
        LocalStateQueryEvent::AcquireFailed {
            reason: AcquireFailure::PointNotOnChain,
        },
    ];
    (steps, expected)
}

fn scenarios() -> Vec<(&'static str, Vec<Step>, Vec<LocalStateQueryEvent>)> {
    vec![
        {
            let (s, e) = acquire_query_release();
            ("acquire_query_release", s, e)
        },
        {
            let (s, e) = acquire_then_failure();
            ("acquire_then_failure", s, e)
        },
        {
            let (s, e) = re_acquire_chain();
            ("re_acquire_chain", s, e)
        },
        {
            let (s, e) = multiple_queries_same_snapshot();
            ("multiple_queries_same_snapshot", s, e)
        },
        {
            let (s, e) = acquire_done_terminates_from_acquired();
            ("acquire_done_terminates_from_acquired", s, e)
        },
        {
            let (s, e) = immediate_client_done();
            ("immediate_client_done", s, e)
        },
        {
            let (s, e) = point_not_on_chain_failure();
            ("point_not_on_chain_failure", s, e)
        },
    ]
}

#[test]
fn local_state_query_event_trace() {
    let cases = scenarios();
    assert!(cases.len() >= 6, "S-A8 requires ≥6 scenarios");

    for (name, steps, expected) in &cases {
        let actual = drive(steps);
        assert_eq!(
            actual, *expected,
            "scenario {name}: event trace mismatched expected"
        );
    }

    for (name, steps, _) in &cases {
        let mut first: Option<Vec<LocalStateQueryEvent>> = None;
        for _ in 0..1000 {
            let trace = drive(steps);
            match &first {
                None => first = Some(trace),
                Some(prev) => assert_eq!(*prev, trace, "scenario {name}: event trace drifted"),
            }
        }
    }
}
