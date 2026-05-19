// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Substrate corpus test for CE-N-B-4: replays a pinned ordered stream
// of NonceInput values through apply_nonce_input and asserts that the
// final state matches the expected hex bytes recorded in the corpus
// JSON. Replay determinism is asserted by running each scenario twice
// and comparing byte-identical state.
//
// Integration test (compiled separately from the BLUE library crate).
// File I/O here is shell behavior, not BLUE behavior.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::path::PathBuf;

use ade_core::consensus::{apply_nonce_input, Nonce, NonceInput, PraosChainDepState};
use ade_crypto::vrf::VrfOutput;
use ade_types::{BlockNo, EpochNo, Hash32, SlotNo};
use serde_json::Value;

fn corpus_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.push("corpus");
    p.push("consensus");
    p.push("nonce_evolution");
    p.push(name);
    p
}

fn open_for_read(p: &PathBuf) -> std::fs::File {
    std::fs::File::open(p).expect("corpus file readable")
}

fn read_to_end(mut f: std::fs::File) -> Vec<u8> {
    use std::io::Read;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).expect("read entire file");
    buf
}

fn read_file(p: &PathBuf) -> Vec<u8> {
    let f = open_for_read(p);
    read_to_end(f)
}

fn load(name: &str) -> Value {
    let p = corpus_path(name);
    let bytes = read_file(&p);
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

fn hash32_from_hex(s: &str) -> Hash32 {
    let raw = from_hex(s);
    assert_eq!(raw.len(), 32, "expected 32 bytes");
    let mut out = [0u8; 32];
    out.copy_from_slice(&raw);
    Hash32(out)
}

fn nonce_from_hex(s: &str) -> Nonce {
    Nonce(hash32_from_hex(s))
}

fn vrf_from_hex(s: &str) -> VrfOutput {
    let raw = from_hex(s);
    assert_eq!(raw.len(), 64, "vrf output must be 64 bytes");
    let mut out = [0u8; 64];
    out.copy_from_slice(&raw);
    VrfOutput(out)
}

fn build_initial_state(v: &Value) -> PraosChainDepState {
    let init = &v["initial_state"];
    let mut state = PraosChainDepState::empty();
    state.evolving_nonce = nonce_from_hex(init["evolving_nonce"].as_str().expect("evolving_nonce"));
    state.candidate_nonce =
        nonce_from_hex(init["candidate_nonce"].as_str().expect("candidate_nonce"));
    state.epoch_nonce = nonce_from_hex(init["epoch_nonce"].as_str().expect("epoch_nonce"));
    state.previous_epoch_nonce = nonce_from_hex(
        init["previous_epoch_nonce"]
            .as_str()
            .expect("previous_epoch_nonce"),
    );
    state.lab_nonce = nonce_from_hex(init["lab_nonce"].as_str().expect("lab_nonce"));
    if let Some(n) = init.get("last_epoch_block").and_then(|x| x.as_u64()) {
        state.last_epoch_block = Some(EpochNo(n));
    }
    if let Some(n) = init.get("last_slot").and_then(|x| x.as_u64()) {
        state.last_slot = Some(SlotNo(n));
    }
    if let Some(n) = init.get("last_block_no").and_then(|x| x.as_u64()) {
        state.last_block_no = Some(BlockNo(n));
    }
    state
}

fn parse_input(v: &Value) -> NonceInput {
    let kind = v["kind"].as_str().expect("kind");
    match kind {
        "HeaderContribution" => NonceInput::HeaderContribution {
            slot: SlotNo(v["slot"].as_u64().expect("slot")),
            vrf_output: vrf_from_hex(v["vrf_output_hex"].as_str().expect("vrf_output_hex")),
        },
        "CandidateFreeze" => NonceInput::CandidateFreeze {
            at_slot: SlotNo(v["at_slot"].as_u64().expect("at_slot")),
            epoch: EpochNo(v["epoch"].as_u64().expect("epoch")),
        },
        "EpochBoundary" => NonceInput::EpochBoundary {
            new_epoch: EpochNo(v["new_epoch"].as_u64().expect("new_epoch")),
            last_block_of_prev_epoch: v
                .get("last_block_of_prev_epoch")
                .and_then(|x| x.as_u64())
                .map(EpochNo),
        },
        other => panic!("unknown input kind: {other}"),
    }
}

fn run_sequence(corpus: &Value) -> PraosChainDepState {
    let mut state = build_initial_state(corpus);
    for input_v in corpus["inputs"].as_array().expect("inputs array") {
        let input = parse_input(input_v);
        state = apply_nonce_input(&state, &input).expect("transition succeeds");
    }
    state
}

#[test]
fn within_epoch_evolving_nonce_matches_corpus() {
    let corpus = load("within_epoch.json");
    let final_state = run_sequence(&corpus);
    let expected = nonce_from_hex(
        corpus["expected_final_evolving_nonce"]
            .as_str()
            .expect("expected_final_evolving_nonce"),
    );
    assert_eq!(final_state.evolving_nonce, expected);
    let expected_slot = SlotNo(
        corpus["expected_final_last_slot"]
            .as_u64()
            .expect("expected_final_last_slot"),
    );
    assert_eq!(final_state.last_slot, Some(expected_slot));
}

#[test]
fn epoch_boundary_freezes_and_rotates_correctly() {
    let corpus = load("epoch_boundary.json");
    let final_state = run_sequence(&corpus);
    let expected_epoch = nonce_from_hex(
        corpus["expected_final_epoch_nonce"]
            .as_str()
            .expect("expected_final_epoch_nonce"),
    );
    let expected_prev = nonce_from_hex(
        corpus["expected_final_previous_epoch"]
            .as_str()
            .expect("expected_final_previous_epoch"),
    );
    let expected_evolving = nonce_from_hex(
        corpus["expected_final_evolving"]
            .as_str()
            .expect("expected_final_evolving"),
    );
    let expected_candidate = nonce_from_hex(
        corpus["expected_final_candidate"]
            .as_str()
            .expect("expected_final_candidate"),
    );
    let expected_last_epoch_block = EpochNo(
        corpus["expected_final_last_epoch_block"]
            .as_u64()
            .expect("expected_final_last_epoch_block"),
    );
    let expected_lab = nonce_from_hex(
        corpus["expected_final_lab_nonce"]
            .as_str()
            .expect("expected_final_lab_nonce"),
    );

    assert_eq!(final_state.epoch_nonce, expected_epoch);
    assert_eq!(final_state.previous_epoch_nonce, expected_prev);
    assert_eq!(final_state.evolving_nonce, expected_evolving);
    assert_eq!(final_state.candidate_nonce, expected_candidate);
    assert_eq!(final_state.last_epoch_block, Some(expected_last_epoch_block));
    assert_eq!(final_state.lab_nonce, expected_lab);
}

#[test]
fn nonce_evolution_replay_is_deterministic() {
    for name in ["within_epoch.json", "epoch_boundary.json"] {
        let corpus = load(name);
        let a = run_sequence(&corpus);
        let b = run_sequence(&corpus);
        assert_eq!(a, b, "non-deterministic replay for {name}");
    }
}
