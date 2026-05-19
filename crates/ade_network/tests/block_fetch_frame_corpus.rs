// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Integration test for S-A5: drives the block-fetch transition through
// 10 curated synthetic batch scenarios and asserts:
//   1. The emitted `BatchDeliveryEvent` sequence matches the spec-derived
//      expected sequence for each scenario.
//   2. The event sequence is deterministic — replaying any scenario
//      1000 times yields byte-identical event traces (including the
//      opaque `block_bytes` payloads).
//
// This closes the state-machine-correctness portion of CE-N-A-3.
// Real-capture verification against corpus/network/n2n/block_fetch/
// follows in S-A9; the test bodies stay unchanged, only the input
// scripts change source (synthetic vs. captured frames).

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_network::block_fetch::{
    block_fetch_transition, BatchDeliveryEvent, BlockFetchAgency, BlockFetchOutput,
    BlockFetchState, Point, Range,
};
use ade_network::codec::block_fetch::BlockFetchMessage;
use ade_network::codec::version::BlockFetchVersion;
use ade_types::{Hash32, SlotNo};

fn v() -> BlockFetchVersion {
    BlockFetchVersion::new(9)
}

fn block_point(slot: u64, seed: u8) -> Point {
    Point::Block {
        slot: SlotNo(slot),
        hash: Hash32([seed; 32]),
    }
}

/// Synthetic block payload deterministically derived from (slot, size).
/// The 4-byte CBOR-ish tag prefix mimics the shape of an era-prefixed
/// block envelope; the trailing fill makes payloads distinguishable
/// across scenarios while staying byte-deterministic.
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

type Step = (BlockFetchAgency, BlockFetchMessage);

/// Drive a scenario through the state machine and collect every
/// emitted batch-delivery event in order. Replies and Done outputs are
/// discarded — the integration test asserts on event traces only.
fn drive(steps: &[Step]) -> Vec<BatchDeliveryEvent> {
    let mut state = BlockFetchState::Idle;
    let mut events = Vec::new();
    for (agency, msg) in steps {
        let (next, out) =
            block_fetch_transition(state, *agency, v(), msg.clone()).expect("legal transition");
        state = next;
        if let BlockFetchOutput::Event(ev) = out {
            events.push(ev);
        }
    }
    events
}

/// Build a fetch sequence: client RequestRange, server StartBatch, N
/// server Blocks, server BatchDone. Returns (steps, expected events).
fn batch_sequence(
    seed: u8,
    block_sizes: &[(u64, usize)],
) -> (Vec<Step>, Vec<BatchDeliveryEvent>) {
    let first_slot = block_sizes.first().map(|(s, _)| *s).unwrap_or(0);
    let last_slot = block_sizes.last().map(|(s, _)| *s).unwrap_or(first_slot);
    let range = Range {
        from: block_point(first_slot, seed),
        to: block_point(last_slot, seed),
    };
    let mut steps = vec![
        (
            BlockFetchAgency::Client,
            BlockFetchMessage::RequestRange(range),
        ),
        (BlockFetchAgency::Server, BlockFetchMessage::StartBatch),
    ];
    let mut expected = vec![BatchDeliveryEvent::BatchStarted];
    for (slot, size) in block_sizes {
        let bytes = block_body(*slot, *size);
        steps.push((
            BlockFetchAgency::Server,
            BlockFetchMessage::Block {
                bytes: bytes.clone(),
            },
        ));
        expected.push(BatchDeliveryEvent::BlockDelivered { block_bytes: bytes });
    }
    steps.push((BlockFetchAgency::Server, BlockFetchMessage::BatchDone));
    expected.push(BatchDeliveryEvent::BatchCompleted);
    (steps, expected)
}

// ---------------------------------------------------------------------------
// Scenarios — each function returns (script, expected_event_trace).
// ---------------------------------------------------------------------------

fn single_block_batch() -> (Vec<Step>, Vec<BatchDeliveryEvent>) {
    batch_sequence(0xB0, &[(100, 256)])
}

fn ten_block_batch() -> (Vec<Step>, Vec<BatchDeliveryEvent>) {
    let sizes: Vec<(u64, usize)> = (0u64..10).map(|i| (1000 + i, 512)).collect();
    batch_sequence(0xB1, &sizes)
}

fn one_hundred_block_batch() -> (Vec<Step>, Vec<BatchDeliveryEvent>) {
    let sizes: Vec<(u64, usize)> = (0u64..100).map(|i| (10_000 + i, 256)).collect();
    batch_sequence(0xB2, &sizes)
}

fn empty_range_no_blocks() -> (Vec<Step>, Vec<BatchDeliveryEvent>) {
    // Client requests a forward-oriented but server-empty range; server
    // replies NoBlocks. Idle -> Busy -> Idle, single NoBlocks event.
    let range = Range {
        from: block_point(50_000, 0xB3),
        to: block_point(50_000, 0xB3),
    };
    let steps = vec![
        (
            BlockFetchAgency::Client,
            BlockFetchMessage::RequestRange(range),
        ),
        (BlockFetchAgency::Server, BlockFetchMessage::NoBlocks),
    ];
    let expected = vec![BatchDeliveryEvent::NoBlocks];
    (steps, expected)
}

fn origin_to_origin_no_blocks() -> (Vec<Step>, Vec<BatchDeliveryEvent>) {
    // Origin-to-Origin is the genesis-only legal range; server returns
    // NoBlocks. Exercises the Origin-endpoint legality at the state
    // machine.
    let range = Range {
        from: Point::Origin,
        to: Point::Origin,
    };
    let steps = vec![
        (
            BlockFetchAgency::Client,
            BlockFetchMessage::RequestRange(range),
        ),
        (BlockFetchAgency::Server, BlockFetchMessage::NoBlocks),
    ];
    let expected = vec![BatchDeliveryEvent::NoBlocks];
    (steps, expected)
}

fn varying_block_sizes() -> (Vec<Step>, Vec<BatchDeliveryEvent>) {
    // small + large + small to exercise per-block size variance and
    // assert order preservation across heterogeneous payloads.
    let sizes: &[(u64, usize)] = &[(2000, 64), (2001, 16_384), (2002, 128)];
    batch_sequence(0xB4, sizes)
}

fn immediate_client_done() -> (Vec<Step>, Vec<BatchDeliveryEvent>) {
    let steps = vec![(BlockFetchAgency::Client, BlockFetchMessage::ClientDone)];
    let expected: Vec<BatchDeliveryEvent> = Vec::new();
    (steps, expected)
}

fn mid_size_batch_with_large_blocks() -> (Vec<Step>, Vec<BatchDeliveryEvent>) {
    let sizes: Vec<(u64, usize)> = (0u64..5).map(|i| (3000 + i, 32_768)).collect();
    batch_sequence(0xB5, &sizes)
}

fn two_back_to_back_batches() -> (Vec<Step>, Vec<BatchDeliveryEvent>) {
    // Idle -> Busy -> Streaming -> Idle -> Busy -> Streaming -> Idle.
    // Verifies the Streaming-to-Idle transition truly returns to Idle
    // so a second RequestRange is accepted.
    let (mut steps1, mut expected1) = batch_sequence(0xB6, &[(4000, 256), (4001, 256)]);
    let (steps2, expected2) = batch_sequence(0xB7, &[(4100, 256), (4101, 256), (4102, 256)]);
    steps1.extend(steps2);
    expected1.extend(expected2);
    (steps1, expected1)
}

fn interleaved_request_and_done() -> (Vec<Step>, Vec<BatchDeliveryEvent>) {
    // Multiple successful requests interleaved over the session,
    // terminated with ClientDone. Exercises Idle re-entry between
    // batches and the final Idle -> Done edge.
    let (mut steps, mut expected) = batch_sequence(0xB8, &[(5000, 128)]);
    let (s2, e2) = batch_sequence(0xB8, &[(5100, 128), (5101, 128)]);
    steps.extend(s2);
    expected.extend(e2);
    let (s3, e3) = batch_sequence(0xB8, &[(5200, 128)]);
    steps.extend(s3);
    expected.extend(e3);
    steps.push((BlockFetchAgency::Client, BlockFetchMessage::ClientDone));
    (steps, expected)
}

fn scenarios() -> Vec<(&'static str, Vec<Step>, Vec<BatchDeliveryEvent>)> {
    vec![
        {
            let (s, e) = single_block_batch();
            ("single_block_batch", s, e)
        },
        {
            let (s, e) = ten_block_batch();
            ("ten_block_batch", s, e)
        },
        {
            let (s, e) = one_hundred_block_batch();
            ("one_hundred_block_batch", s, e)
        },
        {
            let (s, e) = empty_range_no_blocks();
            ("empty_range_no_blocks", s, e)
        },
        {
            let (s, e) = origin_to_origin_no_blocks();
            ("origin_to_origin_no_blocks", s, e)
        },
        {
            let (s, e) = varying_block_sizes();
            ("varying_block_sizes", s, e)
        },
        {
            let (s, e) = immediate_client_done();
            ("immediate_client_done", s, e)
        },
        {
            let (s, e) = mid_size_batch_with_large_blocks();
            ("mid_size_batch_with_large_blocks", s, e)
        },
        {
            let (s, e) = two_back_to_back_batches();
            ("two_back_to_back_batches", s, e)
        },
        {
            let (s, e) = interleaved_request_and_done();
            ("interleaved_request_and_done", s, e)
        },
    ]
}

#[test]
fn block_fetch_frame_corpus() {
    let cases = scenarios();
    assert!(cases.len() >= 10, "S-A5 requires ≥10 scenarios");

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
    // BlockDelivered.block_bytes payloads.
    for (name, steps, _) in &cases {
        let mut first: Option<Vec<BatchDeliveryEvent>> = None;
        for _ in 0..1000 {
            let trace = drive(steps);
            match &first {
                None => first = Some(trace),
                Some(prev) => assert_eq!(*prev, trace, "scenario {name}: event trace drifted"),
            }
        }
    }
}
