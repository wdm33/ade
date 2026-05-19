// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Integration test for S-A7 keep-alive: drives the keep-alive
// transition through 6 curated synthetic scenarios and asserts:
//   1. The emitted `KeepAliveEvent` sequence matches the spec-derived
//      expected sequence for each scenario.
//   2. The event sequence is deterministic — replaying any scenario
//      1000 times yields byte-identical event traces.
//
// Real-capture verification against corpus/network/n2n/keep_alive/
// follows in S-A9; the test bodies stay unchanged, only the input
// scripts change source (synthetic vs. captured frames).

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_network::codec::keep_alive::{KeepAliveCookie, KeepAliveMessage};
use ade_network::codec::version::KeepAliveVersion;
use ade_network::keep_alive::{
    keep_alive_transition, KeepAliveAgency, KeepAliveEvent, KeepAliveOutput, KeepAliveState,
};

fn v() -> KeepAliveVersion {
    KeepAliveVersion::new(9)
}

type Step = (KeepAliveAgency, KeepAliveMessage);

/// Drive a scenario through the state machine and collect every
/// emitted event in order. `KeepAliveOutput::Done` is discarded — the
/// integration test asserts on event traces only.
fn drive(steps: &[Step]) -> Vec<KeepAliveEvent> {
    let mut state = KeepAliveState::ClientIdle;
    let mut events = Vec::new();
    for (agency, msg) in steps {
        let (next, out) =
            keep_alive_transition(state, *agency, v(), msg.clone()).expect("legal transition");
        state = next;
        if let KeepAliveOutput::Event(ev) = out {
            events.push(ev);
        }
    }
    events
}

// ---------------------------------------------------------------------------
// Scenarios — each function returns (script, expected_event_trace).
// All scenarios begin from `KeepAliveState::ClientIdle`.
// ---------------------------------------------------------------------------

/// Scenario 1: single ping-pong then Done.
fn single_ping_pong() -> (Vec<Step>, Vec<KeepAliveEvent>) {
    let cookie = KeepAliveCookie(0x1234);
    let steps = vec![
        (KeepAliveAgency::Client, KeepAliveMessage::KeepAlive(cookie)),
        (
            KeepAliveAgency::Server,
            KeepAliveMessage::ResponseKeepAlive(cookie),
        ),
        (KeepAliveAgency::Client, KeepAliveMessage::Done),
    ];
    let expected = vec![
        KeepAliveEvent::PingSent { cookie },
        KeepAliveEvent::PongReceived { cookie },
    ];
    (steps, expected)
}

/// Scenario 2: five sequential ping-pongs sharing the same cookie.
fn sequential_ping_pongs() -> (Vec<Step>, Vec<KeepAliveEvent>) {
    let cookie = KeepAliveCookie(0xAA55);
    let mut steps = Vec::new();
    let mut expected = Vec::new();
    for _ in 0..5 {
        steps.push((KeepAliveAgency::Client, KeepAliveMessage::KeepAlive(cookie)));
        steps.push((
            KeepAliveAgency::Server,
            KeepAliveMessage::ResponseKeepAlive(cookie),
        ));
        expected.push(KeepAliveEvent::PingSent { cookie });
        expected.push(KeepAliveEvent::PongReceived { cookie });
    }
    (steps, expected)
}

/// Scenario 3: four ping-pongs each carrying a distinct cookie.
fn mixed_cookie_sequence() -> (Vec<Step>, Vec<KeepAliveEvent>) {
    let cookies = [
        KeepAliveCookie(0x0001),
        KeepAliveCookie(0x0100),
        KeepAliveCookie(0xBEEF),
        KeepAliveCookie(0xCAFE),
    ];
    let mut steps = Vec::new();
    let mut expected = Vec::new();
    for c in cookies.iter().copied() {
        steps.push((KeepAliveAgency::Client, KeepAliveMessage::KeepAlive(c)));
        steps.push((
            KeepAliveAgency::Server,
            KeepAliveMessage::ResponseKeepAlive(c),
        ));
        expected.push(KeepAliveEvent::PingSent { cookie: c });
        expected.push(KeepAliveEvent::PongReceived { cookie: c });
    }
    (steps, expected)
}

/// Scenario 4: client immediately terminates without any ping.
fn immediate_done() -> (Vec<Step>, Vec<KeepAliveEvent>) {
    let steps = vec![(KeepAliveAgency::Client, KeepAliveMessage::Done)];
    (steps, Vec::new())
}

/// Scenario 5: single ping-pong at the u16 boundary (cookie =
/// u16::MAX). Covers the upper extreme of the 16-bit nonce space.
fn max_cookie_u16_boundary() -> (Vec<Step>, Vec<KeepAliveEvent>) {
    let cookie = KeepAliveCookie(u16::MAX);
    let steps = vec![
        (KeepAliveAgency::Client, KeepAliveMessage::KeepAlive(cookie)),
        (
            KeepAliveAgency::Server,
            KeepAliveMessage::ResponseKeepAlive(cookie),
        ),
    ];
    let expected = vec![
        KeepAliveEvent::PingSent { cookie },
        KeepAliveEvent::PongReceived { cookie },
    ];
    (steps, expected)
}

/// Scenario 6: one ping-pong, then Done. Exercises termination
/// immediately after a successful round trip.
fn ping_then_done() -> (Vec<Step>, Vec<KeepAliveEvent>) {
    let cookie = KeepAliveCookie(0x4242);
    let steps = vec![
        (KeepAliveAgency::Client, KeepAliveMessage::KeepAlive(cookie)),
        (
            KeepAliveAgency::Server,
            KeepAliveMessage::ResponseKeepAlive(cookie),
        ),
        (KeepAliveAgency::Client, KeepAliveMessage::Done),
    ];
    let expected = vec![
        KeepAliveEvent::PingSent { cookie },
        KeepAliveEvent::PongReceived { cookie },
    ];
    (steps, expected)
}

fn scenarios() -> Vec<(&'static str, Vec<Step>, Vec<KeepAliveEvent>)> {
    vec![
        {
            let (s, e) = single_ping_pong();
            ("single_ping_pong", s, e)
        },
        {
            let (s, e) = sequential_ping_pongs();
            ("sequential_ping_pongs", s, e)
        },
        {
            let (s, e) = mixed_cookie_sequence();
            ("mixed_cookie_sequence", s, e)
        },
        {
            let (s, e) = immediate_done();
            ("immediate_done", s, e)
        },
        {
            let (s, e) = max_cookie_u16_boundary();
            ("max_cookie_u16_boundary", s, e)
        },
        {
            let (s, e) = ping_then_done();
            ("ping_then_done", s, e)
        },
    ]
}

#[test]
fn keep_alive_event_trace() {
    let cases = scenarios();
    assert!(cases.len() >= 6, "S-A7 keep-alive requires >=6 scenarios");

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
    // every iteration (T-DET-01 + DC-PROTO-01).
    for (name, steps, _) in &cases {
        let mut first: Option<Vec<KeepAliveEvent>> = None;
        for _ in 0..1000 {
            let trace = drive(steps);
            match &first {
                None => first = Some(trace),
                Some(prev) => assert_eq!(*prev, trace, "scenario {name}: event trace drifted"),
            }
        }
    }
}
