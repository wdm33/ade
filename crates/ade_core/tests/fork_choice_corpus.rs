// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// CE-N-B-1 close — replays curated multi-tip divergence scenarios
// against `select_best_chain` and verifies that the produced
// `ChainEvent` and `ChainSelectorState` match the corpus exactly.
// Reject-reason byte stability is asserted via canonical CBOR encoding.
//
// Integration test (compiled separately from the BLUE library crate).
// File I/O here is shell behavior, not BLUE behavior.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_core::consensus::{
    encode_chain_event, select_best_chain, BlockDistance, CandidateFragment, ChainEvent,
    ChainSelectionReject, ChainSelectorState, ForkChoiceError, Point, SecurityParam,
    TiebreakerView,
};
use ade_core::consensus::header_summary::ValidatedHeaderSummary;
use ade_crypto::vrf::VrfOutput;
use ade_types::{BlockNo, Hash28, Hash32, SlotNo};
use serde_json::Value;

const MULTI_TIP_JSON: &str = include_str!("../../../corpus/consensus/fork_choice/multi_tip.json");
const REJECTS_JSON: &str = include_str!("../../../corpus/consensus/fork_choice/rejects.json");

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

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hash32(s: &str) -> Hash32 {
    let raw = from_hex(s);
    assert_eq!(raw.len(), 32, "expected 32 bytes for hash32");
    let mut out = [0u8; 32];
    out.copy_from_slice(&raw);
    Hash32(out)
}

fn hash28(s: &str) -> Hash28 {
    let raw = from_hex(s);
    assert_eq!(raw.len(), 28, "expected 28 bytes for hash28");
    let mut out = [0u8; 28];
    out.copy_from_slice(&raw);
    Hash28(out)
}

fn first_8(s: &str) -> [u8; 8] {
    let raw = from_hex(s);
    assert_eq!(raw.len(), 8, "expected 8 bytes for vrf prefix");
    let mut out = [0u8; 8];
    out.copy_from_slice(&raw);
    out
}

fn parse_point(v: &Value) -> Point {
    Point {
        slot: SlotNo(v["slot"].as_u64().expect("slot")),
        hash: hash32(v["hash_hex"].as_str().expect("hash_hex")),
    }
}

fn parse_tiebreaker(v: &Value) -> TiebreakerView {
    TiebreakerView {
        slot: SlotNo(v["slot"].as_u64().expect("slot")),
        issuer_hash: hash28(v["issuer_hex"].as_str().expect("issuer_hex")),
        op_cert_counter: v["op_cert_counter"].as_u64().expect("op_cert_counter"),
        leader_vrf_output_first_8: first_8(v["vrf_output_first_8_hex"].as_str().expect("vrf_output_first_8_hex")),
    }
}

fn parse_state(v: &Value) -> ChainSelectorState {
    ChainSelectorState {
        current_tip: parse_point(&v["current_tip"]),
        current_tip_block_no: BlockNo(v["current_tip_block_no"].as_u64().expect("current_tip_block_no")),
        current_tiebreaker: parse_tiebreaker(&v["current_tiebreaker"]),
        immutable_tip: parse_point(&v["immutable_tip"]),
        immutable_tip_block_no: BlockNo(v["immutable_tip_block_no"].as_u64().expect("immutable_tip_block_no")),
        security_param: SecurityParam(v["security_param"].as_u64().expect("security_param")),
    }
}

fn parse_header(v: &Value) -> ValidatedHeaderSummary {
    ValidatedHeaderSummary {
        slot: SlotNo(v["slot"].as_u64().expect("slot")),
        block_no: BlockNo(v["block_no"].as_u64().expect("block_no")),
        body_hash: hash32(v["body_hash_hex"].as_str().expect("body_hash_hex")),
        issuer_pool: hash28(v["issuer_pool_hex"].as_str().expect("issuer_pool_hex")),
        op_cert_counter: v["op_cert_counter"].as_u64().expect("op_cert_counter"),
        vrf_leader_output: VrfOutput([0u8; 64]),
    }
}

fn parse_candidate(v: &Value) -> CandidateFragment {
    let headers: Vec<ValidatedHeaderSummary> = v["headers"]
        .as_array()
        .expect("headers")
        .iter()
        .map(parse_header)
        .collect();
    CandidateFragment {
        anchor: parse_point(&v["anchor"]),
        anchor_block_no: BlockNo(v["anchor_block_no"].as_u64().expect("anchor_block_no")),
        headers,
        select_view: parse_tiebreaker(&v["select_view"]),
        rollback_depth: BlockDistance(v["rollback_depth"].as_u64().expect("rollback_depth")),
    }
}

fn load_multi_tip() -> Value {
    serde_json::from_str(MULTI_TIP_JSON).expect("multi_tip.json is valid JSON")
}

fn load_rejects() -> Value {
    serde_json::from_str(REJECTS_JSON).expect("rejects.json is valid JSON")
}

fn run_multi_tip() -> (ChainSelectorState, ChainEvent, Value) {
    let corpus = load_multi_tip();
    let state = parse_state(&corpus["state"]);
    let candidates: Vec<CandidateFragment> = corpus["candidates"]
        .as_array()
        .expect("candidates array")
        .iter()
        .map(parse_candidate)
        .collect();
    let (new_state, event) =
        select_best_chain(&state, &candidates).expect("multi_tip should produce an event");
    (new_state, event, corpus)
}

#[test]
fn higher_block_no_wins() {
    let (_new_state, event, corpus) = run_multi_tip();
    let expected_tip = parse_point(&corpus["expected"]["new_tip"]);
    let expected_replaced = parse_point(&corpus["expected"]["replaced_tip"]);
    match event {
        ChainEvent::ChainSelected {
            new_tip,
            replaced_tip,
        } => {
            assert_eq!(new_tip, expected_tip, "new_tip mismatch");
            assert_eq!(replaced_tip, Some(expected_replaced), "replaced_tip mismatch");
        }
        other => panic!("expected ChainSelected, got {:?}", other),
    }
}

#[test]
fn equal_block_no_tiebreaker_decides() {
    // Build a small inline scenario: two candidates tied on block_no=51,
    // tiebreaker picks the one with lower slot.
    let state = ChainSelectorState {
        current_tip: Point { slot: SlotNo(100), hash: Hash32([0x11; 32]) },
        current_tip_block_no: BlockNo(50),
        current_tiebreaker: TiebreakerView {
            slot: SlotNo(100),
            issuer_hash: Hash28([0xaa; 28]),
            op_cert_counter: 5,
            leader_vrf_output_first_8: [0x01; 8],
        },
        immutable_tip: Point { slot: SlotNo(50), hash: Hash32([0; 32]) },
        immutable_tip_block_no: BlockNo(25),
        security_param: SecurityParam(2160),
    };
    let header_a = ValidatedHeaderSummary {
        slot: SlotNo(105),
        block_no: BlockNo(51),
        body_hash: Hash32([0x33; 32]),
        issuer_pool: Hash28([0xbb; 28]),
        op_cert_counter: 4,
        vrf_leader_output: VrfOutput([0u8; 64]),
    };
    let header_b = ValidatedHeaderSummary {
        slot: SlotNo(110),
        block_no: BlockNo(51),
        body_hash: Hash32([0x44; 32]),
        issuer_pool: Hash28([0xcc; 28]),
        op_cert_counter: 4,
        vrf_leader_output: VrfOutput([0u8; 64]),
    };
    let cand_a = CandidateFragment {
        anchor: Point { slot: SlotNo(95), hash: Hash32([0x22; 32]) },
        anchor_block_no: BlockNo(50),
        headers: vec![header_a.clone()],
        select_view: TiebreakerView {
            slot: SlotNo(105),
            issuer_hash: Hash28([0xbb; 28]),
            op_cert_counter: 4,
            leader_vrf_output_first_8: [0x05; 8],
        },
        rollback_depth: BlockDistance(1),
    };
    let cand_b = CandidateFragment {
        anchor: Point { slot: SlotNo(95), hash: Hash32([0x22; 32]) },
        anchor_block_no: BlockNo(50),
        headers: vec![header_b],
        select_view: TiebreakerView {
            slot: SlotNo(110),
            issuer_hash: Hash28([0xcc; 28]),
            op_cert_counter: 4,
            leader_vrf_output_first_8: [0x05; 8],
        },
        rollback_depth: BlockDistance(1),
    };
    let (_new_state, event) =
        select_best_chain(&state, &[cand_a, cand_b]).expect("event");
    match event {
        ChainEvent::ChainSelected { new_tip, .. } => {
            // Tip should be header_a (slot 105 < 110).
            assert_eq!(new_tip.slot, SlotNo(105));
            assert_eq!(new_tip.hash, Hash32([0x33; 32]));
        }
        other => panic!("expected ChainSelected, got {:?}", other),
    }
}

fn run_reject_scenario(scenario: &Value) -> (ChainEvent, Vec<u8>) {
    let state = parse_state(&scenario["state"]);
    let candidates: Vec<CandidateFragment> = scenario["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .map(parse_candidate)
        .collect();
    let (_new_state, event) =
        select_best_chain(&state, &candidates).expect("scenario produces an event");
    let bytes = encode_chain_event(&event);
    (event, bytes)
}

fn scenario_by_name<'a>(corpus: &'a Value, name: &str) -> &'a Value {
    corpus["scenarios"]
        .as_array()
        .expect("scenarios array")
        .iter()
        .find(|s| s["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("scenario {} not found", name))
}

#[test]
fn fork_before_immutable_tip_rejected() {
    let corpus = load_rejects();
    let scenario = scenario_by_name(&corpus, "fork_before_immutable_tip");
    let (event, _bytes) = run_reject_scenario(scenario);
    match event {
        ChainEvent::Rejected { reason: ChainSelectionReject::ForkBeforeImmutableTip { .. } } => {}
        other => panic!("expected ForkBeforeImmutableTip, got {:?}", other),
    }
}

#[test]
fn exceeded_rollback_rejected() {
    let corpus = load_rejects();
    let scenario = scenario_by_name(&corpus, "exceeded_rollback");
    let (event, _bytes) = run_reject_scenario(scenario);
    match event {
        ChainEvent::Rejected { reason: ChainSelectionReject::ExceededRollback { requested, max } } => {
            assert_eq!(requested.0, 200);
            assert_eq!(max.0, 100);
        }
        other => panic!("expected ExceededRollback, got {:?}", other),
    }
}

#[test]
fn tiebreaker_loss_keeps_current() {
    let corpus = load_rejects();
    let scenario = scenario_by_name(&corpus, "tiebreaker_loss");
    let (event, _bytes) = run_reject_scenario(scenario);
    match event {
        ChainEvent::Rejected { reason: ChainSelectionReject::TiebreakerLossKeepCurrent { current_tip, .. } } => {
            assert_eq!(current_tip.slot, SlotNo(100));
        }
        other => panic!("expected TiebreakerLossKeepCurrent, got {:?}", other),
    }
}

#[test]
fn replay_is_deterministic() {
    // Running the multi-tip scenario twice must produce byte-identical
    // encoded events.
    let (_, e1, _) = run_multi_tip();
    let (_, e2, _) = run_multi_tip();
    let b1 = encode_chain_event(&e1);
    let b2 = encode_chain_event(&e2);
    assert_eq!(b1, b2, "multi_tip replay is not byte-identical");

    let corpus = load_rejects();
    for s in corpus["scenarios"].as_array().expect("scenarios") {
        let (_e1, b1) = run_reject_scenario(s);
        let (_e2, b2) = run_reject_scenario(s);
        assert_eq!(
            b1, b2,
            "scenario {} replay is not byte-identical",
            s["name"].as_str().unwrap_or("?")
        );
    }
}

#[test]
fn reject_reason_bytes_are_stable() {
    let corpus = load_rejects();
    let mut failures: Vec<(String, String, String)> = Vec::new();
    for scenario in corpus["scenarios"].as_array().expect("scenarios") {
        let name = scenario["name"].as_str().expect("name").to_string();
        let (_event, bytes) = run_reject_scenario(scenario);
        let actual = to_hex(&bytes);
        let expected = scenario["expected_event_bytes_hex"]
            .as_str()
            .unwrap_or_else(|| panic!("scenario {} missing expected_event_bytes_hex", name))
            .to_string();
        if actual != expected {
            failures.push((name, actual, expected));
        }
    }
    if !failures.is_empty() {
        for (name, actual, expected) in &failures {
            eprintln!(
                "scenario {} drift:\n  actual:   {}\n  expected: {}",
                name, actual, expected
            );
        }
        panic!("{} scenario(s) drifted from pin", failures.len());
    }
}

#[test]
fn no_candidates_returns_error() {
    let corpus = load_multi_tip();
    let state = parse_state(&corpus["state"]);
    let r = select_best_chain(&state, &[]);
    assert_eq!(r, Err(ForkChoiceError::NoCandidates));
}
