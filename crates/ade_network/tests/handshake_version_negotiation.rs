// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Integration test for S-A3: exercises the N2N and N2C handshake
// transition surfaces against a curated synthetic matrix of
// (proposed, supported) tuples and asserts:
//   1. select_*_version returns a deterministic result for every tuple
//   2. n2n/n2c_transition through Idle -> Done | error matches the
//      expected outcome
//   3. Encoded reply bytes are byte-identical across 1000 replay runs
//
// This closes the state-machine-correctness portion of CE-N-A-1.
// Real-capture verification follows in S-A9.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::assertions_on_constants)]

use ade_network::codec::handshake::{
    encode_handshake_message, HandshakeMessage, VersionParams, VersionTable,
};
use ade_network::codec::n2c_handshake::{
    encode_n2c_handshake_message, N2cHandshakeMessage, N2cVersionParams, N2cVersionTable,
};
use ade_network::codec::primitives::encode_u64;
use ade_network::codec::version::{N2CVersion, N2NVersion};
use ade_network::handshake::{
    n2c_transition, n2n_transition, select_n2c_version, select_n2n_version, HandshakeAgency,
    HandshakeError, HandshakeState, N2cHandshakeOutput, N2nHandshakeOutput, N2C_SUPPORTED,
    N2N_SUPPORTED,
};
use ade_network::handshake::selection::SelectionOutcome;

fn params_uint(v: u64) -> VersionParams {
    let mut buf = Vec::new();
    encode_u64(&mut buf, v);
    VersionParams(buf)
}

fn n2c_params_uint(v: u64) -> N2cVersionParams {
    let mut buf = Vec::new();
    encode_u64(&mut buf, v);
    N2cVersionParams(buf)
}

#[derive(Debug, Clone)]
enum Expected {
    Selected(u16),
    Mismatch,
}

fn n2n_test_matrix() -> Vec<(Vec<u16>, Expected)> {
    vec![
        // Single-version overlap at each supported version.
        (vec![11], Expected::Selected(11)),
        (vec![12], Expected::Selected(12)),
        (vec![13], Expected::Selected(13)),
        (vec![14], Expected::Selected(14)),
        // Multi-version overlap: pick the max.
        (vec![11, 12], Expected::Selected(12)),
        (vec![12, 13], Expected::Selected(13)),
        (vec![13, 14], Expected::Selected(14)),
        (vec![11, 12, 13, 14], Expected::Selected(14)),
        // Partial overlap.
        (vec![9, 10, 11], Expected::Selected(11)),
        (vec![10, 14, 99], Expected::Selected(14)),
        // No overlap.
        (vec![1, 2, 3], Expected::Mismatch),
        (vec![9, 10], Expected::Mismatch),
        (vec![15, 16, 17], Expected::Mismatch),
    ]
}

fn n2c_test_matrix() -> Vec<(Vec<u16>, Expected)> {
    vec![
        // Single-version overlap at each supported version.
        (vec![15], Expected::Selected(15)),
        (vec![16], Expected::Selected(16)),
        (vec![17], Expected::Selected(17)),
        (vec![18], Expected::Selected(18)),
        (vec![19], Expected::Selected(19)),
        (vec![20], Expected::Selected(20)),
        // Multi-version overlap: pick the max.
        (vec![15, 16], Expected::Selected(16)),
        (vec![18, 20], Expected::Selected(20)),
        (vec![15, 16, 17, 18, 19, 20], Expected::Selected(20)),
        // Partial overlap.
        (vec![13, 14, 15], Expected::Selected(15)),
        (vec![14, 20, 99], Expected::Selected(20)),
        // No overlap.
        (vec![1, 2, 3], Expected::Mismatch),
        (vec![10, 11, 12], Expected::Mismatch),
        (vec![21, 22, 23], Expected::Mismatch),
    ]
}

fn build_n2n_table(versions: &[u16]) -> VersionTable {
    let mut entries: Vec<(N2NVersion, VersionParams)> = Vec::with_capacity(versions.len());
    for v in versions {
        entries.push((N2NVersion::new(*v), params_uint(*v as u64)));
    }
    VersionTable(entries)
}

fn build_n2c_table(versions: &[u16]) -> N2cVersionTable {
    let mut entries: Vec<(N2CVersion, N2cVersionParams)> = Vec::with_capacity(versions.len());
    for v in versions {
        entries.push((N2CVersion::new(*v), n2c_params_uint(*v as u64)));
    }
    N2cVersionTable(entries)
}

#[test]
fn version_negotiation_across_supported_table() {
    // Pass 1: selection is deterministic for the curated matrix.
    for (proposed, expected) in n2n_test_matrix() {
        let table = build_n2n_table(&proposed);
        match (select_n2n_version(&table, N2N_SUPPORTED), expected) {
            (SelectionOutcome::Selected(v, _), Expected::Selected(want)) => {
                assert_eq!(v.get(), want, "N2N selection mismatch for {proposed:?}");
            }
            (SelectionOutcome::Mismatch(_), Expected::Mismatch) => {}
            (got, want) => {
                let _ = (got, want);
                assert!(false, "N2N: matrix entry {proposed:?} produced unexpected pairing");
            }
        }
    }
    for (proposed, expected) in n2c_test_matrix() {
        let table = build_n2c_table(&proposed);
        match (select_n2c_version(&table, N2C_SUPPORTED), expected) {
            (SelectionOutcome::Selected(v, _), Expected::Selected(want)) => {
                assert_eq!(v.get(), want, "N2C selection mismatch for {proposed:?}");
            }
            (SelectionOutcome::Mismatch(_), Expected::Mismatch) => {}
            (_got, _want) => {
                assert!(false, "N2C: matrix entry {proposed:?} produced unexpected pairing");
            }
        }
    }

    // Pass 2: full Idle -> (Done | error) for the same matrix.
    for (proposed, expected) in n2n_test_matrix() {
        let table = build_n2n_table(&proposed);
        let result = n2n_transition(
            HandshakeState::Idle,
            HandshakeAgency::ClientHasAgency,
            N2N_SUPPORTED,
            HandshakeMessage::ProposeVersions(table),
        );
        match (result, expected) {
            (Ok((HandshakeState::Done, N2nHandshakeOutput::Selected(v, _))), Expected::Selected(want)) => {
                assert_eq!(v.get(), want, "n2n_transition selected wrong version for {proposed:?}");
            }
            (Err(HandshakeError::VersionMismatch { .. }), Expected::Mismatch) => {}
            (other, want) => {
                assert!(false, "n2n_transition: matrix {proposed:?} produced {other:?} expected {want:?}");
            }
        }
    }
    for (proposed, expected) in n2c_test_matrix() {
        let table = build_n2c_table(&proposed);
        let result = n2c_transition(
            HandshakeState::Idle,
            HandshakeAgency::ClientHasAgency,
            N2C_SUPPORTED,
            N2cHandshakeMessage::ProposeVersions(table),
        );
        match (result, expected) {
            (Ok((HandshakeState::Done, N2cHandshakeOutput::Selected(v, _))), Expected::Selected(want)) => {
                assert_eq!(v.get(), want, "n2c_transition selected wrong version for {proposed:?}");
            }
            (Err(HandshakeError::VersionMismatch { .. }), Expected::Mismatch) => {}
            (other, want) => {
                assert!(false, "n2c_transition: matrix {proposed:?} produced {other:?} expected {want:?}");
            }
        }
    }

    // Pass 3: 1000-run encoded-reply byte identity for every selecting
    // matrix entry. The state machine is pure, so re-running it and
    // encoding the resulting AcceptVersion via the S-A2 codec must
    // produce byte-identical output every time (T-DET-01 + T-ENC-03).
    for (proposed, expected) in n2n_test_matrix() {
        let want = match expected {
            Expected::Selected(v) => v,
            Expected::Mismatch => continue,
        };
        let table = build_n2n_table(&proposed);
        let mut first: Option<Vec<u8>> = None;
        for _ in 0..1000 {
            let (st, out) = match n2n_transition(
                HandshakeState::Idle,
                HandshakeAgency::ClientHasAgency,
                N2N_SUPPORTED,
                HandshakeMessage::ProposeVersions(table.clone()),
            ) {
                Ok(p) => p,
                Err(e) => {
                    assert!(false, "n2n_transition unexpectedly failed: {e:?}");
                    return;
                }
            };
            assert_eq!(st, HandshakeState::Done);
            let v = match out {
                N2nHandshakeOutput::Selected(v, _) => v,
                other => {
                    assert!(false, "expected Selected, got {other:?}");
                    return;
                }
            };
            assert_eq!(v.get(), want);
            let reply = HandshakeMessage::AcceptVersion(v, params_uint(v.get() as u64));
            let bytes = encode_handshake_message(&reply);
            match &first {
                None => first = Some(bytes),
                Some(prev) => assert_eq!(*prev, bytes, "N2N reply bytes drifted across runs"),
            }
        }
    }
    for (proposed, expected) in n2c_test_matrix() {
        let want = match expected {
            Expected::Selected(v) => v,
            Expected::Mismatch => continue,
        };
        let table = build_n2c_table(&proposed);
        let mut first: Option<Vec<u8>> = None;
        for _ in 0..1000 {
            let (st, out) = match n2c_transition(
                HandshakeState::Idle,
                HandshakeAgency::ClientHasAgency,
                N2C_SUPPORTED,
                N2cHandshakeMessage::ProposeVersions(table.clone()),
            ) {
                Ok(p) => p,
                Err(e) => {
                    assert!(false, "n2c_transition unexpectedly failed: {e:?}");
                    return;
                }
            };
            assert_eq!(st, HandshakeState::Done);
            let v = match out {
                N2cHandshakeOutput::Selected(v, _) => v,
                other => {
                    assert!(false, "expected Selected, got {other:?}");
                    return;
                }
            };
            assert_eq!(v.get(), want);
            let reply = N2cHandshakeMessage::AcceptVersion(v, n2c_params_uint(v.get() as u64));
            let bytes = encode_n2c_handshake_message(&reply);
            match &first {
                None => first = Some(bytes),
                Some(prev) => assert_eq!(*prev, bytes, "N2C reply bytes drifted across runs"),
            }
        }
    }
}
