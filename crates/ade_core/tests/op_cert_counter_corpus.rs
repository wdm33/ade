// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Corpus test for the S-B5 op-cert counter authority: replays pinned
// op-cert observation sequences through apply_op_cert and asserts
// that the final OpCertCounterMap matches expected bindings, that
// regression yields the typed error, and that replay is deterministic.
//
// Integration test (compiled separately from the BLUE library crate).
// File I/O here is shell behavior, not BLUE behavior.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use ade_core::consensus::{
    apply_op_cert, OpCertCounterError, OpCertObservation, PraosChainDepState,
};
use ade_types::Hash28;
use serde_json::Value;

fn corpus_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.push("corpus");
    p.push("consensus");
    p.push("op_cert");
    p.push(name);
    p
}

fn read_file(p: &PathBuf) -> Vec<u8> {
    let mut f = File::open(p).expect("corpus file readable");
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).expect("read entire file");
    buf
}

fn load(name: &str) -> Value {
    let bytes = read_file(&corpus_path(name));
    serde_json::from_slice(&bytes).expect("corpus is valid JSON")
}

fn nib(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => panic!("invalid hex digit"),
    }
}

fn from_hex(s: &str) -> Vec<u8> {
    assert_eq!(s.len() % 2, 0, "hex string must have even length");
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(s.len() / 2);
    let mut i = 0;
    while i < bytes.len() {
        out.push((nib(bytes[i]) << 4) | nib(bytes[i + 1]));
        i += 2;
    }
    out
}

fn pool_from_hex(s: &str) -> Hash28 {
    let raw = from_hex(s);
    assert_eq!(raw.len(), 28, "pool_hex must be 28 bytes");
    let mut out = [0u8; 28];
    out.copy_from_slice(&raw);
    Hash28(out)
}

fn parse_observation(v: &Value) -> OpCertObservation {
    OpCertObservation {
        pool: pool_from_hex(v["pool_hex"].as_str().expect("pool_hex")),
        kes_period: v["kes_period"].as_u64().expect("kes_period"),
        counter: v["counter"].as_u64().expect("counter"),
    }
}

fn apply_all(
    initial: &PraosChainDepState,
    observations: &[Value],
) -> Result<PraosChainDepState, OpCertCounterError> {
    let mut state = initial.clone();
    for obs_v in observations {
        let obs = parse_observation(obs_v);
        state = apply_op_cert(&state, &obs)?;
    }
    Ok(state)
}

#[test]
fn normal_progression_records_highest_counter_per_window() {
    let corpus = load("normal_progression.json");
    let observations = corpus["observations"]
        .as_array()
        .expect("observations array");
    let state = apply_all(&PraosChainDepState::empty(), observations).expect("all succeed");

    let expected_size = corpus["expected_final_map_size"]
        .as_u64()
        .expect("expected_final_map_size") as usize;
    assert_eq!(state.op_cert_counters.len(), expected_size);

    for entry in corpus["expected_final_counters"]
        .as_array()
        .expect("expected_final_counters")
    {
        let pool = pool_from_hex(entry["pool_hex"].as_str().expect("pool_hex"));
        let kes_period = entry["kes_period"].as_u64().expect("kes_period");
        let counter = entry["counter"].as_u64().expect("counter");
        assert_eq!(
            state.op_cert_counters.get(&pool, kes_period),
            Some(counter),
            "expected counter mismatch for pool={pool:?} kes_period={kes_period}"
        );
    }
}

#[test]
fn regression_after_progression_rejected_with_typed_error() {
    let corpus = load("regression_case.json");
    let observations = corpus["observations"]
        .as_array()
        .expect("observations array");
    let state = apply_all(&PraosChainDepState::empty(), observations).expect("progressions ok");

    let reg = parse_observation(&corpus["regression_observation"]);
    let err = apply_op_cert(&state, &reg);
    let expected = &corpus["expected_error"];
    assert_eq!(expected["kind"].as_str(), Some("Regression"));
    let expected_existing = expected["existing"].as_u64().expect("existing");
    let expected_attempted = expected["attempted"].as_u64().expect("attempted");
    assert_eq!(
        err,
        Err(OpCertCounterError::Regression {
            existing: expected_existing,
            attempted: expected_attempted,
        })
    );
}

#[test]
fn op_cert_replay_is_deterministic() {
    for name in ["normal_progression.json", "regression_case.json"] {
        let corpus = load(name);
        let observations = corpus["observations"]
            .as_array()
            .expect("observations array");
        let a = apply_all(&PraosChainDepState::empty(), observations);
        let b = apply_all(&PraosChainDepState::empty(), observations);
        assert_eq!(a, b, "non-deterministic replay for {name}");
    }
}
