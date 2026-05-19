// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Integration test for S-A8 (LocalTxMonitor portion): drives the N2C
// LocalTxMonitor transition through curated synthetic scenarios and
// asserts:
//   1. The emitted `LocalTxMonitorEvent` sequence matches the
//      spec-derived expected sequence for each scenario.
//   2. The event sequence is deterministic — replaying any scenario
//      1000 times yields byte-identical event traces (including the
//      opaque `LocalTxMonitorQuery` and `LocalTxMonitorReply` bytes).
//
// Real-capture verification against captured N2C frames follows in
// S-A9; the test bodies stay unchanged.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_network::codec::local_tx_monitor::LocalTxMonitorMessage;
use ade_network::codec::version::LocalTxMonitorVersion;
use ade_network::n2c::local_tx_monitor::{
    local_tx_monitor_transition, LocalTxMonitorAgency, LocalTxMonitorEvent, LocalTxMonitorOutput,
    LocalTxMonitorQuery, LocalTxMonitorReply, LocalTxMonitorState,
};
use ade_types::SlotNo;

fn v() -> LocalTxMonitorVersion {
    LocalTxMonitorVersion::new(16)
}

fn query_body(seed: u8, size: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size);
    buf.extend_from_slice(&[0x82, seed]);
    let fill = seed.wrapping_add(0x77);
    while buf.len() < size {
        buf.push(fill);
    }
    buf.truncate(size);
    buf
}

fn reply_body(seed: u8, size: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size);
    buf.extend_from_slice(&[0x83, seed]);
    let fill = seed.wrapping_add(0xBB);
    while buf.len() < size {
        buf.push(fill);
    }
    buf.truncate(size);
    buf
}

type Step = (LocalTxMonitorAgency, LocalTxMonitorMessage);

fn drive(steps: &[Step]) -> Vec<LocalTxMonitorEvent> {
    let mut state = LocalTxMonitorState::Idle;
    let mut events = Vec::new();
    for (agency, msg) in steps {
        let (next, out) =
            local_tx_monitor_transition(state, *agency, v(), msg.clone()).expect("legal");
        state = next;
        if let LocalTxMonitorOutput::Event(ev) = out {
            events.push(ev);
        }
    }
    events
}

fn acquire_query_release() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let slot = SlotNo(100_000);
    let q = query_body(0x11, 32);
    let r = reply_body(0x11, 64);
    let steps = vec![
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Acquire,
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot },
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Query(LocalTxMonitorQuery(q.clone())),
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Reply(LocalTxMonitorReply(r.clone())),
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Release,
        ),
    ];
    let expected = vec![
        LocalTxMonitorEvent::AcquireRequested,
        LocalTxMonitorEvent::MempoolAcquired { slot },
        LocalTxMonitorEvent::QueryRequested {
            payload: LocalTxMonitorQuery(q),
        },
        LocalTxMonitorEvent::QueryReplied {
            payload: LocalTxMonitorReply(r),
        },
        LocalTxMonitorEvent::MempoolReleased,
    ];
    (steps, expected)
}

fn acquire_with_await_loop() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let slot = SlotNo(200_000);
    let steps = vec![
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Acquire,
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::AwaitAcquire,
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::AwaitAcquire,
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot },
        ),
    ];
    let expected = vec![
        LocalTxMonitorEvent::AcquireRequested,
        LocalTxMonitorEvent::AwaitingAcquisition,
        LocalTxMonitorEvent::AwaitingAcquisition,
        LocalTxMonitorEvent::MempoolAcquired { slot },
    ];
    (steps, expected)
}

fn multiple_queries_same_snapshot() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let slot = SlotNo(300_000);
    let q1 = query_body(0x31, 16);
    let r1 = reply_body(0x31, 32);
    let q2 = query_body(0x32, 24);
    let r2 = reply_body(0x32, 48);
    let q3 = query_body(0x33, 8);
    let r3 = reply_body(0x33, 16);
    let steps = vec![
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Acquire,
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot },
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Query(LocalTxMonitorQuery(q1.clone())),
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Reply(LocalTxMonitorReply(r1.clone())),
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Query(LocalTxMonitorQuery(q2.clone())),
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Reply(LocalTxMonitorReply(r2.clone())),
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Query(LocalTxMonitorQuery(q3.clone())),
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Reply(LocalTxMonitorReply(r3.clone())),
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Release,
        ),
    ];
    let expected = vec![
        LocalTxMonitorEvent::AcquireRequested,
        LocalTxMonitorEvent::MempoolAcquired { slot },
        LocalTxMonitorEvent::QueryRequested {
            payload: LocalTxMonitorQuery(q1),
        },
        LocalTxMonitorEvent::QueryReplied {
            payload: LocalTxMonitorReply(r1),
        },
        LocalTxMonitorEvent::QueryRequested {
            payload: LocalTxMonitorQuery(q2),
        },
        LocalTxMonitorEvent::QueryReplied {
            payload: LocalTxMonitorReply(r2),
        },
        LocalTxMonitorEvent::QueryRequested {
            payload: LocalTxMonitorQuery(q3),
        },
        LocalTxMonitorEvent::QueryReplied {
            payload: LocalTxMonitorReply(r3),
        },
        LocalTxMonitorEvent::MempoolReleased,
    ];
    (steps, expected)
}

fn acquire_done_terminates_from_acquired() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let slot = SlotNo(400_000);
    let steps = vec![
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Acquire,
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot },
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Done),
    ];
    let expected = vec![
        LocalTxMonitorEvent::AcquireRequested,
        LocalTxMonitorEvent::MempoolAcquired { slot },
    ];
    (steps, expected)
}

fn re_acquire_after_release() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let slot_a = SlotNo(500_000);
    let slot_b = SlotNo(500_100);
    let steps = vec![
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Acquire,
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot: slot_a },
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Release,
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Acquire,
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot: slot_b },
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Release,
        ),
    ];
    let expected = vec![
        LocalTxMonitorEvent::AcquireRequested,
        LocalTxMonitorEvent::MempoolAcquired { slot: slot_a },
        LocalTxMonitorEvent::MempoolReleased,
        LocalTxMonitorEvent::AcquireRequested,
        LocalTxMonitorEvent::MempoolAcquired { slot: slot_b },
        LocalTxMonitorEvent::MempoolReleased,
    ];
    (steps, expected)
}

fn immediate_client_done() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let steps = vec![(LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Done)];
    (steps, Vec::new())
}

fn scenarios() -> Vec<(&'static str, Vec<Step>, Vec<LocalTxMonitorEvent>)> {
    vec![
        {
            let (s, e) = acquire_query_release();
            ("acquire_query_release", s, e)
        },
        {
            let (s, e) = acquire_with_await_loop();
            ("acquire_with_await_loop", s, e)
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
            let (s, e) = re_acquire_after_release();
            ("re_acquire_after_release", s, e)
        },
        {
            let (s, e) = immediate_client_done();
            ("immediate_client_done", s, e)
        },
    ]
}

#[test]
fn local_tx_monitor_event_trace() {
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
        let mut first: Option<Vec<LocalTxMonitorEvent>> = None;
        for _ in 0..1000 {
            let trace = drive(steps);
            match &first {
                None => first = Some(trace),
                Some(prev) => assert_eq!(*prev, trace, "scenario {name}: event trace drifted"),
            }
        }
    }
}
