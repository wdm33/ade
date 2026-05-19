// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Integration test for S-A8 (LocalTxSubmission portion): drives the
// N2C LocalTxSubmission transition through curated synthetic scenarios
// and asserts:
//   1. The emitted `LocalTxSubmissionEvent` sequence matches the
//      spec-derived expected sequence for each scenario.
//   2. The event sequence is deterministic — replaying any scenario
//      1000 times yields byte-identical event traces (including the
//      opaque `tx_bytes` payloads and rejection reason bytes).
//
// Real-capture verification against captured N2C frames follows in
// S-A9; the test bodies stay unchanged.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_network::codec::local_tx_submission::{
    LocalTxSubmissionMessage, TxAcceptance, TxRejection,
};
use ade_network::codec::version::LocalTxSubmissionVersion;
use ade_network::n2c::local_tx_submission::{
    local_tx_submission_transition, LocalTxSubmissionAgency, LocalTxSubmissionEvent,
    LocalTxSubmissionOutput, LocalTxSubmissionState,
};

fn v() -> LocalTxSubmissionVersion {
    LocalTxSubmissionVersion::new(16)
}

/// Synthetic tx body deterministically derived from (seed, size).
fn tx_body(seed: u8, size: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size);
    buf.extend_from_slice(&[0x84, seed]);
    let fill = seed.wrapping_add(0x33);
    while buf.len() < size {
        buf.push(fill);
    }
    buf.truncate(size);
    buf
}

/// Synthetic rejection bytes derived from (seed, size).
fn reject_body(seed: u8, size: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size);
    buf.extend_from_slice(&[0xA1, seed]);
    let fill = seed.wrapping_add(0xC0);
    while buf.len() < size {
        buf.push(fill);
    }
    buf.truncate(size);
    buf
}

type Step = (LocalTxSubmissionAgency, LocalTxSubmissionMessage);

fn drive(steps: &[Step]) -> Vec<LocalTxSubmissionEvent> {
    let mut state = LocalTxSubmissionState::Idle;
    let mut events = Vec::new();
    for (agency, msg) in steps {
        let (next, out) =
            local_tx_submission_transition(state, *agency, v(), msg.clone()).expect("legal");
        state = next;
        if let LocalTxSubmissionOutput::Event(ev) = out {
            events.push(ev);
        }
    }
    events
}

fn submit_then_accept() -> (Vec<Step>, Vec<LocalTxSubmissionEvent>) {
    let tx = tx_body(0x11, 256);
    let steps = vec![
        (
            LocalTxSubmissionAgency::Client,
            LocalTxSubmissionMessage::SubmitTx {
                tx_bytes: tx.clone(),
            },
        ),
        (
            LocalTxSubmissionAgency::Server,
            LocalTxSubmissionMessage::AcceptTx(TxAcceptance),
        ),
    ];
    let expected = vec![
        LocalTxSubmissionEvent::TxSubmitted { tx_bytes: tx },
        LocalTxSubmissionEvent::TxAccepted,
    ];
    (steps, expected)
}

fn submit_then_reject() -> (Vec<Step>, Vec<LocalTxSubmissionEvent>) {
    let tx = tx_body(0x22, 512);
    let reject = reject_body(0x22, 64);
    let steps = vec![
        (
            LocalTxSubmissionAgency::Client,
            LocalTxSubmissionMessage::SubmitTx {
                tx_bytes: tx.clone(),
            },
        ),
        (
            LocalTxSubmissionAgency::Server,
            LocalTxSubmissionMessage::RejectTx(TxRejection(reject.clone())),
        ),
    ];
    let expected = vec![
        LocalTxSubmissionEvent::TxSubmitted { tx_bytes: tx },
        LocalTxSubmissionEvent::TxRejected {
            rejection: TxRejection(reject),
        },
    ];
    (steps, expected)
}

fn three_submits_alternating() -> (Vec<Step>, Vec<LocalTxSubmissionEvent>) {
    let tx_a = tx_body(0x31, 128);
    let tx_b = tx_body(0x32, 128);
    let tx_c = tx_body(0x33, 128);
    let reject_b = reject_body(0x32, 32);
    let steps = vec![
        (
            LocalTxSubmissionAgency::Client,
            LocalTxSubmissionMessage::SubmitTx {
                tx_bytes: tx_a.clone(),
            },
        ),
        (
            LocalTxSubmissionAgency::Server,
            LocalTxSubmissionMessage::AcceptTx(TxAcceptance),
        ),
        (
            LocalTxSubmissionAgency::Client,
            LocalTxSubmissionMessage::SubmitTx {
                tx_bytes: tx_b.clone(),
            },
        ),
        (
            LocalTxSubmissionAgency::Server,
            LocalTxSubmissionMessage::RejectTx(TxRejection(reject_b.clone())),
        ),
        (
            LocalTxSubmissionAgency::Client,
            LocalTxSubmissionMessage::SubmitTx {
                tx_bytes: tx_c.clone(),
            },
        ),
        (
            LocalTxSubmissionAgency::Server,
            LocalTxSubmissionMessage::AcceptTx(TxAcceptance),
        ),
    ];
    let expected = vec![
        LocalTxSubmissionEvent::TxSubmitted { tx_bytes: tx_a },
        LocalTxSubmissionEvent::TxAccepted,
        LocalTxSubmissionEvent::TxSubmitted { tx_bytes: tx_b },
        LocalTxSubmissionEvent::TxRejected {
            rejection: TxRejection(reject_b),
        },
        LocalTxSubmissionEvent::TxSubmitted { tx_bytes: tx_c },
        LocalTxSubmissionEvent::TxAccepted,
    ];
    (steps, expected)
}

fn submit_accept_then_done() -> (Vec<Step>, Vec<LocalTxSubmissionEvent>) {
    let tx = tx_body(0x44, 64);
    let steps = vec![
        (
            LocalTxSubmissionAgency::Client,
            LocalTxSubmissionMessage::SubmitTx {
                tx_bytes: tx.clone(),
            },
        ),
        (
            LocalTxSubmissionAgency::Server,
            LocalTxSubmissionMessage::AcceptTx(TxAcceptance),
        ),
        (
            LocalTxSubmissionAgency::Client,
            LocalTxSubmissionMessage::Done,
        ),
    ];
    let expected = vec![
        LocalTxSubmissionEvent::TxSubmitted { tx_bytes: tx },
        LocalTxSubmissionEvent::TxAccepted,
    ];
    (steps, expected)
}

fn immediate_client_done() -> (Vec<Step>, Vec<LocalTxSubmissionEvent>) {
    let steps = vec![(
        LocalTxSubmissionAgency::Client,
        LocalTxSubmissionMessage::Done,
    )];
    (steps, Vec::new())
}

fn large_tx_byte_identity() -> (Vec<Step>, Vec<LocalTxSubmissionEvent>) {
    let tx: Vec<u8> = (0u8..=255).collect();
    let steps = vec![
        (
            LocalTxSubmissionAgency::Client,
            LocalTxSubmissionMessage::SubmitTx {
                tx_bytes: tx.clone(),
            },
        ),
        (
            LocalTxSubmissionAgency::Server,
            LocalTxSubmissionMessage::AcceptTx(TxAcceptance),
        ),
    ];
    let expected = vec![
        LocalTxSubmissionEvent::TxSubmitted { tx_bytes: tx },
        LocalTxSubmissionEvent::TxAccepted,
    ];
    (steps, expected)
}

fn scenarios() -> Vec<(&'static str, Vec<Step>, Vec<LocalTxSubmissionEvent>)> {
    vec![
        {
            let (s, e) = submit_then_accept();
            ("submit_then_accept", s, e)
        },
        {
            let (s, e) = submit_then_reject();
            ("submit_then_reject", s, e)
        },
        {
            let (s, e) = three_submits_alternating();
            ("three_submits_alternating", s, e)
        },
        {
            let (s, e) = submit_accept_then_done();
            ("submit_accept_then_done", s, e)
        },
        {
            let (s, e) = immediate_client_done();
            ("immediate_client_done", s, e)
        },
        {
            let (s, e) = large_tx_byte_identity();
            ("large_tx_byte_identity", s, e)
        },
    ]
}

#[test]
fn local_tx_submission_event_trace() {
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
        let mut first: Option<Vec<LocalTxSubmissionEvent>> = None;
        for _ in 0..1000 {
            let trace = drive(steps);
            match &first {
                None => first = Some(trace),
                Some(prev) => assert_eq!(*prev, trace, "scenario {name}: event trace drifted"),
            }
        }
    }
}
