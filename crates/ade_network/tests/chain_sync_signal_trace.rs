// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Integration test for S-A4: drives the chain-sync transition through
// 10 curated synthetic divergence scenarios and asserts:
//   1. The emitted `ForkChoiceSignal` sequence matches the spec-derived
//      expected sequence for each scenario.
//   2. The signal sequence is deterministic — replaying any scenario
//      1000 times yields byte-identical signal traces.
//
// This closes the state-machine-correctness portion of CE-N-A-2.
// Real-capture verification against corpus/network/n2n/chain_sync/
// follows in S-A9; the test bodies stay unchanged, only the input
// scripts change source (synthetic vs. captured frames).

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_network::chain_sync::{
    chain_sync_transition, ChainSyncAgency, ChainSyncOutput, ChainSyncState, ForkChoiceSignal,
    Point, Tip,
};
use ade_network::codec::chain_sync::ChainSyncMessage;
use ade_network::codec::version::ChainSyncVersion;
use ade_types::{Hash32, SlotNo};

fn v() -> ChainSyncVersion {
    ChainSyncVersion::new(9)
}

fn block_point(slot: u64, seed: u8) -> Point {
    Point::Block {
        slot: SlotNo(slot),
        hash: Hash32([seed; 32]),
    }
}

fn tip_at(slot: u64, block_no: u64, seed: u8) -> Tip {
    Tip {
        point: block_point(slot, seed),
        block_no,
    }
}

fn header_for(slot: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(16);
    buf.extend_from_slice(&[0xCB, 0x05]);
    buf.extend_from_slice(&slot.to_be_bytes());
    buf.extend_from_slice(&[0xEE, 0xEE, 0xEE, 0xEE]);
    buf
}

type Step = (ChainSyncAgency, ChainSyncMessage);

/// Drive a scenario through the state machine and collect every
/// emitted fork-choice signal in order. Replies and Done outputs are
/// discarded — the integration test asserts on signal traces only.
fn drive(steps: &[Step]) -> Vec<ForkChoiceSignal> {
    let mut state = ChainSyncState::Idle;
    let mut signals = Vec::new();
    for (agency, msg) in steps {
        let (next, out) =
            chain_sync_transition(state, *agency, v(), msg.clone()).expect("legal transition");
        state = next;
        if let ChainSyncOutput::Signal(sig) = out {
            signals.push(sig);
        }
    }
    signals
}

// ---------------------------------------------------------------------------
// Scenarios — each function returns (script, expected_signal_trace).
// ---------------------------------------------------------------------------

fn linear_rollforward_5_blocks() -> (Vec<Step>, Vec<ForkChoiceSignal>) {
    let mut steps = Vec::new();
    let mut expected = Vec::new();
    for i in 1u64..=5 {
        steps.push((ChainSyncAgency::Client, ChainSyncMessage::RequestNext));
        let header = header_for(i);
        let tip = tip_at(i * 100, i, 0xA0);
        steps.push((
            ChainSyncAgency::Server,
            ChainSyncMessage::RollForward {
                header: header.clone(),
                tip: tip.clone(),
            },
        ));
        expected.push(ForkChoiceSignal::RollForward {
            header_bytes: header,
            tip,
        });
    }
    (steps, expected)
}

fn single_rollback_to_intersect() -> (Vec<Step>, Vec<ForkChoiceSignal>) {
    let mut steps = Vec::new();
    let mut expected = Vec::new();
    for i in 1u64..=3 {
        steps.push((ChainSyncAgency::Client, ChainSyncMessage::RequestNext));
        let header = header_for(i);
        let tip = tip_at(i * 100, i, 0xA1);
        steps.push((
            ChainSyncAgency::Server,
            ChainSyncMessage::RollForward {
                header: header.clone(),
                tip: tip.clone(),
            },
        ));
        expected.push(ForkChoiceSignal::RollForward {
            header_bytes: header,
            tip,
        });
    }
    steps.push((ChainSyncAgency::Client, ChainSyncMessage::RequestNext));
    let rb_point = block_point(150, 0xA1);
    let rb_tip = tip_at(300, 3, 0xA1);
    steps.push((
        ChainSyncAgency::Server,
        ChainSyncMessage::RollBackward {
            point: rb_point.clone(),
            tip: rb_tip.clone(),
        },
    ));
    expected.push(ForkChoiceSignal::RollBackward {
        point: rb_point,
        tip: rb_tip,
    });
    (steps, expected)
}

fn multi_rollback_alternating() -> (Vec<Step>, Vec<ForkChoiceSignal>) {
    let mut steps = Vec::new();
    let mut expected = Vec::new();
    for i in 1u64..=4 {
        steps.push((ChainSyncAgency::Client, ChainSyncMessage::RequestNext));
        if i % 2 == 1 {
            let header = header_for(i);
            let tip = tip_at(i * 200, i, 0xA2);
            steps.push((
                ChainSyncAgency::Server,
                ChainSyncMessage::RollForward {
                    header: header.clone(),
                    tip: tip.clone(),
                },
            ));
            expected.push(ForkChoiceSignal::RollForward {
                header_bytes: header,
                tip,
            });
        } else {
            let pt = block_point(i * 200 - 50, 0xA2);
            let tip = tip_at(i * 200, i, 0xA2);
            steps.push((
                ChainSyncAgency::Server,
                ChainSyncMessage::RollBackward {
                    point: pt.clone(),
                    tip: tip.clone(),
                },
            ));
            expected.push(ForkChoiceSignal::RollBackward { point: pt, tip });
        }
    }
    (steps, expected)
}

fn deep_rollback_50_blocks() -> (Vec<Step>, Vec<ForkChoiceSignal>) {
    let mut steps = Vec::new();
    let mut expected = Vec::new();
    for i in 1u64..=50 {
        steps.push((ChainSyncAgency::Client, ChainSyncMessage::RequestNext));
        let header = header_for(i);
        let tip = tip_at(i * 10, i, 0xA3);
        steps.push((
            ChainSyncAgency::Server,
            ChainSyncMessage::RollForward {
                header: header.clone(),
                tip: tip.clone(),
            },
        ));
        expected.push(ForkChoiceSignal::RollForward {
            header_bytes: header,
            tip,
        });
    }
    steps.push((ChainSyncAgency::Client, ChainSyncMessage::RequestNext));
    let rb_point = Point::Origin;
    let rb_tip = tip_at(500, 50, 0xA3);
    steps.push((
        ChainSyncAgency::Server,
        ChainSyncMessage::RollBackward {
            point: rb_point.clone(),
            tip: rb_tip.clone(),
        },
    ));
    expected.push(ForkChoiceSignal::RollBackward {
        point: rb_point,
        tip: rb_tip,
    });
    (steps, expected)
}

fn no_intersection_disjoint_chains() -> (Vec<Step>, Vec<ForkChoiceSignal>) {
    let points = vec![
        block_point(10, 0xA4),
        block_point(20, 0xA4),
        block_point(30, 0xA4),
    ];
    let tip = tip_at(99999, 9999, 0xA4);
    let steps = vec![
        (
            ChainSyncAgency::Client,
            ChainSyncMessage::FindIntersect {
                points: points.clone(),
            },
        ),
        (
            ChainSyncAgency::Server,
            ChainSyncMessage::IntersectNotFound { tip: tip.clone() },
        ),
    ];
    let expected = vec![ForkChoiceSignal::NoIntersection { tip }];
    (steps, expected)
}

fn server_stall_pattern() -> (Vec<Step>, Vec<ForkChoiceSignal>) {
    // Idle -> CanAwait (RequestNext), CanAwait -> MustReply (AwaitReply),
    // ... eventual server RollForward from MustReply -> Idle + Signal.
    let header = header_for(777);
    let tip = tip_at(7700, 77, 0xA5);
    let steps = vec![
        (ChainSyncAgency::Client, ChainSyncMessage::RequestNext),
        (ChainSyncAgency::Server, ChainSyncMessage::AwaitReply),
        (
            ChainSyncAgency::Server,
            ChainSyncMessage::RollForward {
                header: header.clone(),
                tip: tip.clone(),
            },
        ),
    ];
    let expected = vec![ForkChoiceSignal::RollForward {
        header_bytes: header,
        tip,
    }];
    (steps, expected)
}

fn find_intersect_first_point_known() -> (Vec<Step>, Vec<ForkChoiceSignal>) {
    let p_first = block_point(1000, 0xA6);
    let p_other = block_point(900, 0xA6);
    let tip = tip_at(2000, 200, 0xA6);
    let steps = vec![
        (
            ChainSyncAgency::Client,
            ChainSyncMessage::FindIntersect {
                points: vec![p_first.clone(), p_other],
            },
        ),
        (
            ChainSyncAgency::Server,
            ChainSyncMessage::IntersectFound {
                point: p_first.clone(),
                tip: tip.clone(),
            },
        ),
    ];
    let expected = vec![ForkChoiceSignal::Intersected {
        point: p_first,
        tip,
    }];
    (steps, expected)
}

fn find_intersect_only_last_point_known() -> (Vec<Step>, Vec<ForkChoiceSignal>) {
    let p1 = block_point(100, 0xA7);
    let p2 = block_point(200, 0xA7);
    let p_last = block_point(300, 0xA7);
    let tip = tip_at(500, 50, 0xA7);
    let steps = vec![
        (
            ChainSyncAgency::Client,
            ChainSyncMessage::FindIntersect {
                points: vec![p1, p2, p_last.clone()],
            },
        ),
        (
            ChainSyncAgency::Server,
            ChainSyncMessage::IntersectFound {
                point: p_last.clone(),
                tip: tip.clone(),
            },
        ),
    ];
    let expected = vec![ForkChoiceSignal::Intersected {
        point: p_last,
        tip,
    }];
    (steps, expected)
}

fn immediate_done() -> (Vec<Step>, Vec<ForkChoiceSignal>) {
    let steps = vec![(ChainSyncAgency::Client, ChainSyncMessage::Done)];
    let expected = Vec::new();
    (steps, expected)
}

fn interleaved_rollforward_rollback_pattern() -> (Vec<Step>, Vec<ForkChoiceSignal>) {
    let mut steps = Vec::new();
    let mut expected = Vec::new();

    // RF, RF, RB, RF, RB, RB, RF
    let actions: &[bool] = &[true, true, false, true, false, false, true];
    for (i, is_forward) in actions.iter().enumerate() {
        let n = (i as u64) + 1;
        steps.push((ChainSyncAgency::Client, ChainSyncMessage::RequestNext));
        if *is_forward {
            let header = header_for(n * 17);
            let tip = tip_at(n * 50, n, 0xA8);
            steps.push((
                ChainSyncAgency::Server,
                ChainSyncMessage::RollForward {
                    header: header.clone(),
                    tip: tip.clone(),
                },
            ));
            expected.push(ForkChoiceSignal::RollForward {
                header_bytes: header,
                tip,
            });
        } else {
            let pt = block_point(n * 50 - 25, 0xA8);
            let tip = tip_at(n * 50, n, 0xA8);
            steps.push((
                ChainSyncAgency::Server,
                ChainSyncMessage::RollBackward {
                    point: pt.clone(),
                    tip: tip.clone(),
                },
            ));
            expected.push(ForkChoiceSignal::RollBackward { point: pt, tip });
        }
    }
    (steps, expected)
}

fn scenarios() -> Vec<(&'static str, Vec<Step>, Vec<ForkChoiceSignal>)> {
    vec![
        {
            let (s, e) = linear_rollforward_5_blocks();
            ("linear_rollforward_5_blocks", s, e)
        },
        {
            let (s, e) = single_rollback_to_intersect();
            ("single_rollback_to_intersect", s, e)
        },
        {
            let (s, e) = multi_rollback_alternating();
            ("multi_rollback_alternating", s, e)
        },
        {
            let (s, e) = deep_rollback_50_blocks();
            ("deep_rollback_50_blocks", s, e)
        },
        {
            let (s, e) = no_intersection_disjoint_chains();
            ("no_intersection_disjoint_chains", s, e)
        },
        {
            let (s, e) = server_stall_pattern();
            ("server_stall_pattern", s, e)
        },
        {
            let (s, e) = find_intersect_first_point_known();
            ("find_intersect_first_point_known", s, e)
        },
        {
            let (s, e) = find_intersect_only_last_point_known();
            ("find_intersect_only_last_point_known", s, e)
        },
        {
            let (s, e) = immediate_done();
            ("immediate_done", s, e)
        },
        {
            let (s, e) = interleaved_rollforward_rollback_pattern();
            ("interleaved_rollforward_rollback_pattern", s, e)
        },
    ]
}

#[test]
fn signal_trace_synthetic_divergence_corpus() {
    let cases = scenarios();
    assert!(cases.len() >= 10, "S-A4 requires ≥10 scenarios");

    // Pass 1: every scenario's emitted signal trace matches its
    // spec-derived expected sequence.
    for (name, steps, expected) in &cases {
        let actual = drive(steps);
        assert_eq!(
            actual, *expected,
            "scenario {name}: signal trace mismatched expected"
        );
    }

    // Pass 2: 1000-run determinism. Each scenario replays through the
    // state machine and must produce a byte-identical signal trace
    // every iteration (T-DET-01 + DC-PROTO-01).
    for (name, steps, _) in &cases {
        let mut first: Option<Vec<ForkChoiceSignal>> = None;
        for _ in 0..1000 {
            let trace = drive(steps);
            match &first {
                None => first = Some(trace),
                Some(prev) => assert_eq!(*prev, trace, "scenario {name}: signal trace drifted"),
            }
        }
    }
}
