// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Integration test for S-A8 (LocalChainSync portion): drives the N2C
// LocalChainSync transition through curated synthetic scenarios and
// asserts:
//   1. The emitted `LocalChainSyncEvent` sequence matches the
//      spec-derived expected sequence for each scenario.
//   2. The event sequence is deterministic — replaying any scenario
//      1000 times yields byte-identical event traces (including the
//      opaque `block_bytes` payloads).
//
// Real-capture verification against captured N2C frames follows in
// S-A9; the test bodies stay unchanged.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_network::codec::local_chain_sync::LocalChainSyncMessage;
use ade_network::codec::version::LocalChainSyncVersion;
use ade_network::n2c::local_chain_sync::{
    local_chain_sync_transition, LocalChainSyncAgency, LocalChainSyncEvent, LocalChainSyncOutput,
    LocalChainSyncState, Point, Tip,
};
use ade_types::{Hash32, SlotNo};

fn v() -> LocalChainSyncVersion {
    LocalChainSyncVersion::new(16)
}

fn block_point(slot: u64, seed: u8) -> Point {
    Point::Block {
        slot: SlotNo(slot),
        hash: Hash32([seed; 32]),
    }
}

fn tip_at(slot: u64, seed: u8, block_no: u64) -> Tip {
    Tip {
        point: block_point(slot, seed),
        block_no,
    }
}

/// Synthetic full-block payload deterministically derived from
/// (slot, size). Mimics an era-prefixed block envelope shape.
fn block_body(slot: u64, size: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size);
    buf.extend_from_slice(&[0x82, 0x05]);
    buf.extend_from_slice(&slot.to_be_bytes());
    let fill = (slot as u8).wrapping_add(0x33);
    while buf.len() < size {
        buf.push(fill);
    }
    buf.truncate(size);
    buf
}

type Step = (LocalChainSyncAgency, LocalChainSyncMessage);

fn drive(steps: &[Step]) -> Vec<LocalChainSyncEvent> {
    let mut state = LocalChainSyncState::Idle;
    let mut events = Vec::new();
    for (agency, msg) in steps {
        let (next, out) =
            local_chain_sync_transition(state, *agency, v(), msg.clone()).expect("legal");
        state = next;
        if let LocalChainSyncOutput::Event(ev) = out {
            events.push(ev);
        }
    }
    events
}

fn immediate_roll_forward() -> (Vec<Step>, Vec<LocalChainSyncEvent>) {
    let block = block_body(1000, 256);
    let tip = tip_at(1000, 0xAA, 555);
    let steps = vec![
        (
            LocalChainSyncAgency::Client,
            LocalChainSyncMessage::RequestNext,
        ),
        (
            LocalChainSyncAgency::Server,
            LocalChainSyncMessage::RollForward {
                block: block.clone(),
                tip: tip.clone(),
            },
        ),
    ];
    let expected = vec![LocalChainSyncEvent::RollForward {
        block_bytes: block,
        tip,
    }];
    (steps, expected)
}

fn await_then_roll_forward() -> (Vec<Step>, Vec<LocalChainSyncEvent>) {
    let block = block_body(2000, 1024);
    let tip = tip_at(2000, 0xBB, 600);
    let steps = vec![
        (
            LocalChainSyncAgency::Client,
            LocalChainSyncMessage::RequestNext,
        ),
        (
            LocalChainSyncAgency::Server,
            LocalChainSyncMessage::AwaitReply,
        ),
        (
            LocalChainSyncAgency::Server,
            LocalChainSyncMessage::RollForward {
                block: block.clone(),
                tip: tip.clone(),
            },
        ),
    ];
    let expected = vec![LocalChainSyncEvent::RollForward {
        block_bytes: block,
        tip,
    }];
    (steps, expected)
}

fn roll_backward_after_request() -> (Vec<Step>, Vec<LocalChainSyncEvent>) {
    let rb_point = block_point(1500, 0xCC);
    let tip = tip_at(2000, 0xCC, 700);
    let steps = vec![
        (
            LocalChainSyncAgency::Client,
            LocalChainSyncMessage::RequestNext,
        ),
        (
            LocalChainSyncAgency::Server,
            LocalChainSyncMessage::RollBackward {
                point: rb_point.clone(),
                tip: tip.clone(),
            },
        ),
    ];
    let expected = vec![LocalChainSyncEvent::RollBackward {
        point: rb_point,
        tip,
    }];
    (steps, expected)
}

fn find_intersect_found() -> (Vec<Step>, Vec<LocalChainSyncEvent>) {
    let points = vec![
        Point::Origin,
        block_point(100, 0xDD),
        block_point(200, 0xDE),
    ];
    let found = block_point(200, 0xDE);
    let tip = tip_at(2500, 0xDE, 800);
    let steps = vec![
        (
            LocalChainSyncAgency::Client,
            LocalChainSyncMessage::FindIntersect {
                points: points.clone(),
            },
        ),
        (
            LocalChainSyncAgency::Server,
            LocalChainSyncMessage::IntersectFound {
                point: found.clone(),
                tip: tip.clone(),
            },
        ),
    ];
    let expected = vec![LocalChainSyncEvent::Intersected { point: found, tip }];
    (steps, expected)
}

fn find_intersect_not_found() -> (Vec<Step>, Vec<LocalChainSyncEvent>) {
    let points = vec![block_point(1, 0x00)];
    let tip = tip_at(3000, 0xEE, 900);
    let steps = vec![
        (
            LocalChainSyncAgency::Client,
            LocalChainSyncMessage::FindIntersect { points },
        ),
        (
            LocalChainSyncAgency::Server,
            LocalChainSyncMessage::IntersectNotFound { tip: tip.clone() },
        ),
    ];
    let expected = vec![LocalChainSyncEvent::NoIntersection { tip }];
    (steps, expected)
}

fn immediate_client_done() -> (Vec<Step>, Vec<LocalChainSyncEvent>) {
    let steps = vec![(LocalChainSyncAgency::Client, LocalChainSyncMessage::Done)];
    let expected: Vec<LocalChainSyncEvent> = Vec::new();
    (steps, expected)
}

fn multi_step_roll_forward_then_done() -> (Vec<Step>, Vec<LocalChainSyncEvent>) {
    let mut steps: Vec<Step> = Vec::new();
    let mut expected: Vec<LocalChainSyncEvent> = Vec::new();
    for i in 0u8..3 {
        let block = block_body(4000 + i as u64, 512);
        let tip = tip_at(4000 + i as u64, 0xF0u8.wrapping_add(i), 1000 + i as u64);
        steps.push((
            LocalChainSyncAgency::Client,
            LocalChainSyncMessage::RequestNext,
        ));
        steps.push((
            LocalChainSyncAgency::Server,
            LocalChainSyncMessage::RollForward {
                block: block.clone(),
                tip: tip.clone(),
            },
        ));
        expected.push(LocalChainSyncEvent::RollForward {
            block_bytes: block,
            tip,
        });
    }
    steps.push((LocalChainSyncAgency::Client, LocalChainSyncMessage::Done));
    (steps, expected)
}

fn scenarios() -> Vec<(&'static str, Vec<Step>, Vec<LocalChainSyncEvent>)> {
    vec![
        {
            let (s, e) = immediate_roll_forward();
            ("immediate_roll_forward", s, e)
        },
        {
            let (s, e) = await_then_roll_forward();
            ("await_then_roll_forward", s, e)
        },
        {
            let (s, e) = roll_backward_after_request();
            ("roll_backward_after_request", s, e)
        },
        {
            let (s, e) = find_intersect_found();
            ("find_intersect_found", s, e)
        },
        {
            let (s, e) = find_intersect_not_found();
            ("find_intersect_not_found", s, e)
        },
        {
            let (s, e) = immediate_client_done();
            ("immediate_client_done", s, e)
        },
        {
            let (s, e) = multi_step_roll_forward_then_done();
            ("multi_step_roll_forward_then_done", s, e)
        },
    ]
}

#[test]
fn local_chain_sync_event_trace() {
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
        let mut first: Option<Vec<LocalChainSyncEvent>> = None;
        for _ in 0..1000 {
            let trace = drive(steps);
            match &first {
                None => first = Some(trace),
                Some(prev) => assert_eq!(*prev, trace, "scenario {name}: event trace drifted"),
            }
        }
    }
}
