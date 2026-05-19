// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Integration test for S-A7 peer-sharing: drives the peer-sharing
// transition through 6 curated synthetic scenarios and asserts:
//   1. The emitted `PeerSharingEvent` sequence matches the
//      spec-derived expected sequence for each scenario.
//   2. The event sequence is deterministic — replaying any scenario
//      1000 times yields byte-identical event traces (including the
//      `PeersShared.peers` payloads).
//
// Real-capture verification against corpus/network/n2n/peer_sharing/
// follows in S-A9; the test bodies stay unchanged, only the input
// scripts change source (synthetic vs. captured frames).

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_network::codec::peer_sharing::{PeerAddress, PeerSharingMessage};
use ade_network::codec::version::PeerSharingVersion;
use ade_network::peer_sharing::{
    peer_sharing_transition, PeerSharingAgency, PeerSharingEvent, PeerSharingOutput,
    PeerSharingState,
};

fn v() -> PeerSharingVersion {
    PeerSharingVersion::new(11)
}

fn v4(seed: u8, port: u16) -> PeerAddress {
    PeerAddress::V4 {
        addr: 0xC0A80000 | seed as u32,
        port,
    }
}

fn v6(seed: u8, port: u16) -> PeerAddress {
    let mut a = [0u8; 16];
    a[0] = 0x20;
    a[1] = 0x01;
    a[2] = 0x0D;
    a[3] = 0xB8;
    a[15] = seed;
    PeerAddress::V6 {
        addr: a,
        port,
        flowinfo: 0,
        scope: 0,
    }
}

type Step = (PeerSharingAgency, PeerSharingMessage);

/// Drive a scenario through the state machine and collect every
/// emitted event in order. `PeerSharingOutput::Done` is discarded —
/// the integration test asserts on event traces only.
fn drive(steps: &[Step]) -> Vec<PeerSharingEvent> {
    let mut state = PeerSharingState::Idle;
    let mut events = Vec::new();
    for (agency, msg) in steps {
        let (next, out) =
            peer_sharing_transition(state, *agency, v(), msg.clone()).expect("legal transition");
        state = next;
        if let PeerSharingOutput::Event(ev) = out {
            events.push(ev);
        }
    }
    events
}

// ---------------------------------------------------------------------------
// Scenarios — each function returns (script, expected_event_trace).
// All scenarios begin from `PeerSharingState::Idle`.
// ---------------------------------------------------------------------------

/// Scenario 1: request 5 peers and receive a full reply (5 peers).
fn request_5_reply_5() -> (Vec<Step>, Vec<PeerSharingEvent>) {
    let amount: u8 = 5;
    let peers = vec![
        v4(0x01, 3001),
        v4(0x02, 3001),
        v4(0x03, 3001),
        v4(0x04, 3001),
        v4(0x05, 3001),
    ];
    let steps = vec![
        (
            PeerSharingAgency::Client,
            PeerSharingMessage::ShareRequest { amount },
        ),
        (
            PeerSharingAgency::Server,
            PeerSharingMessage::SharePeers {
                peers: peers.clone(),
            },
        ),
        (PeerSharingAgency::Client, PeerSharingMessage::Done),
    ];
    let expected = vec![
        PeerSharingEvent::SharingRequested { amount },
        PeerSharingEvent::PeersShared { peers },
    ];
    (steps, expected)
}

/// Scenario 2: request 5 peers and receive an empty reply (legal —
/// server has no peers to share).
fn request_5_reply_0() -> (Vec<Step>, Vec<PeerSharingEvent>) {
    let amount: u8 = 5;
    let steps = vec![
        (
            PeerSharingAgency::Client,
            PeerSharingMessage::ShareRequest { amount },
        ),
        (
            PeerSharingAgency::Server,
            PeerSharingMessage::SharePeers { peers: Vec::new() },
        ),
        (PeerSharingAgency::Client, PeerSharingMessage::Done),
    ];
    let expected = vec![
        PeerSharingEvent::SharingRequested { amount },
        PeerSharingEvent::PeersShared { peers: Vec::new() },
    ];
    (steps, expected)
}

/// Scenario 3: request 10 peers, server replies with 3 (partial reply
/// is legal as long as count <= amount).
fn request_10_reply_3() -> (Vec<Step>, Vec<PeerSharingEvent>) {
    let amount: u8 = 10;
    let peers = vec![v4(0x10, 3001), v4(0x11, 3001), v4(0x12, 3001)];
    let steps = vec![
        (
            PeerSharingAgency::Client,
            PeerSharingMessage::ShareRequest { amount },
        ),
        (
            PeerSharingAgency::Server,
            PeerSharingMessage::SharePeers {
                peers: peers.clone(),
            },
        ),
        (PeerSharingAgency::Client, PeerSharingMessage::Done),
    ];
    let expected = vec![
        PeerSharingEvent::SharingRequested { amount },
        PeerSharingEvent::PeersShared { peers },
    ];
    (steps, expected)
}

/// Scenario 4: request 4 peers, server replies with a mixed IPv4 +
/// IPv6 batch — asserts heterogeneous address payloads pass through
/// byte-identically.
fn mixed_ipv4_ipv6_peers() -> (Vec<Step>, Vec<PeerSharingEvent>) {
    let amount: u8 = 4;
    let peers = vec![
        v4(0x20, 3001),
        v6(0x21, 3002),
        v4(0x22, 3003),
        v6(0x23, 3004),
    ];
    let steps = vec![
        (
            PeerSharingAgency::Client,
            PeerSharingMessage::ShareRequest { amount },
        ),
        (
            PeerSharingAgency::Server,
            PeerSharingMessage::SharePeers {
                peers: peers.clone(),
            },
        ),
        (PeerSharingAgency::Client, PeerSharingMessage::Done),
    ];
    let expected = vec![
        PeerSharingEvent::SharingRequested { amount },
        PeerSharingEvent::PeersShared { peers },
    ];
    (steps, expected)
}

/// Scenario 5: request u8::MAX (255) peers — covers the upper
/// boundary of the on-wire `amount: u8`. Server replies with one
/// peer.
fn max_amount_u8() -> (Vec<Step>, Vec<PeerSharingEvent>) {
    let amount: u8 = u8::MAX;
    let peers = vec![v4(0xFF, 3001)];
    let steps = vec![
        (
            PeerSharingAgency::Client,
            PeerSharingMessage::ShareRequest { amount },
        ),
        (
            PeerSharingAgency::Server,
            PeerSharingMessage::SharePeers {
                peers: peers.clone(),
            },
        ),
        (PeerSharingAgency::Client, PeerSharingMessage::Done),
    ];
    let expected = vec![
        PeerSharingEvent::SharingRequested { amount },
        PeerSharingEvent::PeersShared { peers },
    ];
    (steps, expected)
}

/// Scenario 6: client immediately terminates without any request.
fn immediate_done() -> (Vec<Step>, Vec<PeerSharingEvent>) {
    let steps = vec![(PeerSharingAgency::Client, PeerSharingMessage::Done)];
    (steps, Vec::new())
}

fn scenarios() -> Vec<(&'static str, Vec<Step>, Vec<PeerSharingEvent>)> {
    vec![
        {
            let (s, e) = request_5_reply_5();
            ("request_5_reply_5", s, e)
        },
        {
            let (s, e) = request_5_reply_0();
            ("request_5_reply_0", s, e)
        },
        {
            let (s, e) = request_10_reply_3();
            ("request_10_reply_3", s, e)
        },
        {
            let (s, e) = mixed_ipv4_ipv6_peers();
            ("mixed_ipv4_ipv6_peers", s, e)
        },
        {
            let (s, e) = max_amount_u8();
            ("max_amount_u8", s, e)
        },
        {
            let (s, e) = immediate_done();
            ("immediate_done", s, e)
        },
    ]
}

#[test]
fn peer_sharing_event_trace() {
    let cases = scenarios();
    assert!(cases.len() >= 6, "S-A7 peer-sharing requires >=6 scenarios");

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
    // every iteration (T-DET-01 + DC-PROTO-01), including the
    // `PeersShared.peers` payloads.
    for (name, steps, _) in &cases {
        let mut first: Option<Vec<PeerSharingEvent>> = None;
        for _ in 0..1000 {
            let trace = drive(steps);
            match &first {
                None => first = Some(trace),
                Some(prev) => assert_eq!(*prev, trace, "scenario {name}: event trace drifted"),
            }
        }
    }
}
