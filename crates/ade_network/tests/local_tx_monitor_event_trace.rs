// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Integration test for S-A8b (LocalTxMonitor wire-grammar rework):
// drives the N2C LocalTxMonitor transition through curated synthetic
// scenarios covering the four query kinds, release / re-acquire
// patterns, and version-gated Measures use cases. Asserts:
//   1. The emitted `LocalTxMonitorEvent` sequence matches the
//      spec-derived expected sequence for each scenario.
//   2. The event sequence is deterministic — replaying any scenario
//      1000 times yields byte-identical event traces.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::collections::BTreeMap;

use ade_network::codec::local_tx_monitor::{
    LocalTxMonitorMessage, MeasureName, MeasureSizeAndCapacity, MempoolMeasures,
    MempoolSizeAndCapacity,
};
use ade_network::codec::version::LocalTxMonitorVersion;
use ade_network::n2c::local_tx_monitor::{
    local_tx_monitor_transition, LocalTxMonitorAgency, LocalTxMonitorEvent, LocalTxMonitorOutput,
    LocalTxMonitorState,
};
use ade_types::{Hash32, SlotNo, TxId};

fn v() -> LocalTxMonitorVersion {
    LocalTxMonitorVersion::new(2)
}

fn tx_id(seed: u8) -> TxId {
    TxId(Hash32([seed; 32]))
}

fn sizes(capacity_bytes: u32, size_bytes: u32, tx_count: u32) -> MempoolSizeAndCapacity {
    MempoolSizeAndCapacity {
        capacity_bytes,
        size_bytes,
        tx_count,
    }
}

fn measures(tx_count: u32, entries: &[(&str, u64, u64)]) -> MempoolMeasures {
    let mut m = BTreeMap::new();
    for (name, size, capacity) in entries {
        m.insert(
            MeasureName::new(name),
            MeasureSizeAndCapacity {
                size: *size,
                capacity: *capacity,
            },
        );
    }
    MempoolMeasures {
        tx_count,
        measures: m,
    }
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

fn acquire_release() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let slot = SlotNo(100_000);
    let steps = vec![
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Acquire),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot },
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Release),
    ];
    let expected = vec![
        LocalTxMonitorEvent::AcquireRequested,
        LocalTxMonitorEvent::MempoolAcquired { slot },
        LocalTxMonitorEvent::MempoolReleased,
    ];
    (steps, expected)
}

fn acquire_await_acquire_reacquire() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let slot_a = SlotNo(200_000);
    let slot_b = SlotNo(200_100);
    let steps = vec![
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Acquire),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot: slot_a },
        ),
        // Client sends MsgAwaitAcquire (encoded as tag-1 same as Acquire);
        // the state machine reinterprets `(Acquired, Client, Acquire)`
        // as ReAcquireRequested.
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Acquire),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot: slot_b },
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Release),
    ];
    let expected = vec![
        LocalTxMonitorEvent::AcquireRequested,
        LocalTxMonitorEvent::MempoolAcquired { slot: slot_a },
        LocalTxMonitorEvent::ReAcquireRequested,
        LocalTxMonitorEvent::MempoolAcquired { slot: slot_b },
        LocalTxMonitorEvent::MempoolReleased,
    ];
    (steps, expected)
}

fn single_next_tx_round() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let slot = SlotNo(300_000);
    let tx = vec![0xCA, 0xFE, 0xBA, 0xBE];
    let steps = vec![
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Acquire),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot },
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::NextTx),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyNextTx {
                tx_bytes: Some(tx.clone()),
            },
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Release),
    ];
    let expected = vec![
        LocalTxMonitorEvent::AcquireRequested,
        LocalTxMonitorEvent::MempoolAcquired { slot },
        LocalTxMonitorEvent::NextTxRequested,
        LocalTxMonitorEvent::NextTxReplied {
            tx_bytes: Some(tx),
        },
        LocalTxMonitorEvent::MempoolReleased,
    ];
    (steps, expected)
}

fn multi_next_tx_until_empty() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let slot = SlotNo(310_000);
    let tx1 = vec![0x01, 0x02, 0x03];
    let tx2 = vec![0x04, 0x05, 0x06, 0x07];
    let steps = vec![
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Acquire),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot },
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::NextTx),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyNextTx {
                tx_bytes: Some(tx1.clone()),
            },
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::NextTx),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyNextTx {
                tx_bytes: Some(tx2.clone()),
            },
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::NextTx),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyNextTx { tx_bytes: None },
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Release),
    ];
    let expected = vec![
        LocalTxMonitorEvent::AcquireRequested,
        LocalTxMonitorEvent::MempoolAcquired { slot },
        LocalTxMonitorEvent::NextTxRequested,
        LocalTxMonitorEvent::NextTxReplied {
            tx_bytes: Some(tx1),
        },
        LocalTxMonitorEvent::NextTxRequested,
        LocalTxMonitorEvent::NextTxReplied {
            tx_bytes: Some(tx2),
        },
        LocalTxMonitorEvent::NextTxRequested,
        LocalTxMonitorEvent::NextTxReplied { tx_bytes: None },
        LocalTxMonitorEvent::MempoolReleased,
    ];
    (steps, expected)
}

fn has_tx_present() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let slot = SlotNo(400_000);
    let id = tx_id(0x11);
    let steps = vec![
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Acquire),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot },
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::HasTx { tx_id: id.clone() },
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyHasTx { present: true },
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Release),
    ];
    let expected = vec![
        LocalTxMonitorEvent::AcquireRequested,
        LocalTxMonitorEvent::MempoolAcquired { slot },
        LocalTxMonitorEvent::HasTxRequested { tx_id: id },
        LocalTxMonitorEvent::HasTxReplied { present: true },
        LocalTxMonitorEvent::MempoolReleased,
    ];
    (steps, expected)
}

fn has_tx_absent() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let slot = SlotNo(410_000);
    let id = tx_id(0x22);
    let steps = vec![
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Acquire),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot },
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::HasTx { tx_id: id.clone() },
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyHasTx { present: false },
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Release),
    ];
    let expected = vec![
        LocalTxMonitorEvent::AcquireRequested,
        LocalTxMonitorEvent::MempoolAcquired { slot },
        LocalTxMonitorEvent::HasTxRequested { tx_id: id },
        LocalTxMonitorEvent::HasTxReplied { present: false },
        LocalTxMonitorEvent::MempoolReleased,
    ];
    (steps, expected)
}

fn get_sizes_round() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let slot = SlotNo(500_000);
    let s = sizes(1_048_576, 12_345, 7);
    let steps = vec![
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Acquire),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot },
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::GetSizes,
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyGetSizes(s),
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Release),
    ];
    let expected = vec![
        LocalTxMonitorEvent::AcquireRequested,
        LocalTxMonitorEvent::MempoolAcquired { slot },
        LocalTxMonitorEvent::SizesRequested,
        LocalTxMonitorEvent::SizesReplied(s),
        LocalTxMonitorEvent::MempoolReleased,
    ];
    (steps, expected)
}

fn get_measures_round() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let slot = SlotNo(600_000);
    let m = measures(
        9,
        &[
            ("bytes", 4096, 65536),
            ("txs", 9, 256),
        ],
    );
    let steps = vec![
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Acquire),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot },
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::GetMeasures,
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyGetMeasures(m.clone()),
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Release),
    ];
    let expected = vec![
        LocalTxMonitorEvent::AcquireRequested,
        LocalTxMonitorEvent::MempoolAcquired { slot },
        LocalTxMonitorEvent::MeasuresRequested,
        LocalTxMonitorEvent::MeasuresReplied(m),
        LocalTxMonitorEvent::MempoolReleased,
    ];
    (steps, expected)
}

fn mixed_query_kinds_one_session() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let slot = SlotNo(700_000);
    let id = tx_id(0xAB);
    let tx = vec![0x10, 0x20, 0x30];
    let s = sizes(1024, 16, 1);
    let m = measures(1, &[("bytes", 16, 1024)]);
    let steps = vec![
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Acquire),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot },
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::NextTx),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyNextTx {
                tx_bytes: Some(tx.clone()),
            },
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::HasTx { tx_id: id.clone() },
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyHasTx { present: true },
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::GetSizes,
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyGetSizes(s),
        ),
        (
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::GetMeasures,
        ),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyGetMeasures(m.clone()),
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Release),
    ];
    let expected = vec![
        LocalTxMonitorEvent::AcquireRequested,
        LocalTxMonitorEvent::MempoolAcquired { slot },
        LocalTxMonitorEvent::NextTxRequested,
        LocalTxMonitorEvent::NextTxReplied {
            tx_bytes: Some(tx),
        },
        LocalTxMonitorEvent::HasTxRequested { tx_id: id },
        LocalTxMonitorEvent::HasTxReplied { present: true },
        LocalTxMonitorEvent::SizesRequested,
        LocalTxMonitorEvent::SizesReplied(s),
        LocalTxMonitorEvent::MeasuresRequested,
        LocalTxMonitorEvent::MeasuresReplied(m),
        LocalTxMonitorEvent::MempoolReleased,
    ];
    (steps, expected)
}

fn immediate_client_done() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let steps = vec![(LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Done)];
    (steps, Vec::new())
}

fn re_acquire_after_release() -> (Vec<Step>, Vec<LocalTxMonitorEvent>) {
    let slot_a = SlotNo(800_000);
    let slot_b = SlotNo(800_100);
    let steps = vec![
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Acquire),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot: slot_a },
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Release),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Acquire),
        (
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot: slot_b },
        ),
        (LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Release),
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

fn scenarios() -> Vec<(&'static str, Vec<Step>, Vec<LocalTxMonitorEvent>)> {
    vec![
        {
            let (s, e) = acquire_release();
            ("acquire_release", s, e)
        },
        {
            let (s, e) = acquire_await_acquire_reacquire();
            ("acquire_await_acquire_reacquire", s, e)
        },
        {
            let (s, e) = single_next_tx_round();
            ("single_next_tx_round", s, e)
        },
        {
            let (s, e) = multi_next_tx_until_empty();
            ("multi_next_tx_until_empty", s, e)
        },
        {
            let (s, e) = has_tx_present();
            ("has_tx_present", s, e)
        },
        {
            let (s, e) = has_tx_absent();
            ("has_tx_absent", s, e)
        },
        {
            let (s, e) = get_sizes_round();
            ("get_sizes_round", s, e)
        },
        {
            let (s, e) = get_measures_round();
            ("get_measures_round", s, e)
        },
        {
            let (s, e) = mixed_query_kinds_one_session();
            ("mixed_query_kinds_one_session", s, e)
        },
        {
            let (s, e) = immediate_client_done();
            ("immediate_client_done", s, e)
        },
        {
            let (s, e) = re_acquire_after_release();
            ("re_acquire_after_release", s, e)
        },
    ]
}

#[test]
fn local_tx_monitor_event_trace() {
    let cases = scenarios();
    assert!(cases.len() >= 10, "S-A8b requires >=10 scenarios");

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
