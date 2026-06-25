// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// CE-N-B-2 close — replays curated rollback scenarios against
// `apply_rollback` and verifies that the produced `ChainEvent` and
// `RollBackApplied` shape match the corpus exactly. Reject-reason byte
// stability is asserted via canonical CBOR encoding. Truncated-replay
// equivalence is the load-bearing assertion for DC-CONS-06.
//
// Integration test (compiled separately from the BLUE library crate).
// File I/O here is shell behaviour, not BLUE behaviour.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::collections::BTreeMap;

use ade_core::consensus::vrf_cert::vrf_input;
use ade_core::consensus::{
    apply_rollback, encode_chain_event, validate_and_apply_header, ActiveSlotsCoeff,
    BlockDistance, BootstrapAnchorHash, ChainEvent, ChainSelectionReject, ChainSelectorState,
    EraSchedule, EraSummary, HeaderInput, HeaderVrf, Nonce, Point, PraosChainDepState,
    RollBackRequest, SecurityParam, TiebreakerView, VrfRole,
};
use ade_crypto::vrf::{VrfProof, VrfVerificationKey};
use ade_testkit::consensus::ledger_view_stub::{
    EpochStakeFixture, LedgerViewStub, PoolFixture,
};
use ade_types::{BlockNo, CardanoEra, EpochNo, Hash28, Hash32, SlotNo};
use cardano_crypto::vrf::VrfDraft03;
use serde_json::Value;

const WITHIN_K_JSON: &str =
    include_str!("../../../corpus/consensus/rollback/within_k.json");
const EXCEEDS_K_JSON: &str =
    include_str!("../../../corpus/consensus/rollback/exceeds_k.json");
const BEFORE_IMMUTABLE_JSON: &str =
    include_str!("../../../corpus/consensus/rollback/before_immutable.json");

// =============================================================================
// Hex helpers
// =============================================================================

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

// =============================================================================
// Corpus → typed value parsers
// =============================================================================

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
        leader_vrf_output_first_8: first_8(
            v["vrf_output_first_8_hex"].as_str().expect("vrf_output_first_8_hex"),
        ),
    }
}

fn parse_state(v: &Value) -> ChainSelectorState {
    ChainSelectorState {
        current_tip: parse_point(&v["current_tip"]),
        current_tip_block_no: BlockNo(
            v["current_tip_block_no"].as_u64().expect("current_tip_block_no"),
        ),
        current_tiebreaker: parse_tiebreaker(&v["current_tiebreaker"]),
        immutable_tip: parse_point(&v["immutable_tip"]),
        immutable_tip_block_no: BlockNo(
            v["immutable_tip_block_no"].as_u64().expect("immutable_tip_block_no"),
        ),
        security_param: SecurityParam(v["security_param"].as_u64().expect("security_param")),
    }
}

fn parse_request(v: &Value) -> RollBackRequest {
    RollBackRequest {
        to_point: parse_point(&v["to_point"]),
        to_block_no: BlockNo(v["to_block_no"].as_u64().expect("to_block_no")),
        depth: BlockDistance(v["depth"].as_u64().expect("depth")),
    }
}

fn parse_chain_dep_seed(v: &Value) -> PraosChainDepState {
    let h = hash32(v["rolled_back_chain_dep_seed_hex"].as_str().expect("seed"));
    PraosChainDepState::genesis(Nonce(h))
}

fn load(json: &str) -> Value {
    serde_json::from_str(json).expect("corpus json is valid")
}

// =============================================================================
// Scenario runner
// =============================================================================

fn run_scenario(corpus: &Value) -> (ade_core::consensus::RollBackApplied, Vec<u8>) {
    let state = parse_state(&corpus["state"]);
    let request = parse_request(&corpus["request"]);
    let rolled_back_tb = parse_tiebreaker(&corpus["rolled_back_tiebreaker"]);
    let rolled_back_cd = parse_chain_dep_seed(corpus);
    // chain_dep at "now" — distinct seed so a wrongful reject path
    // that returned `rolled_back_chain_dep` would change the bytes.
    let now_cd = PraosChainDepState::genesis(Nonce(Hash32([0x99; 32])));
    let applied =
        apply_rollback(&state, &now_cd, &rolled_back_cd, &rolled_back_tb, &request);
    let bytes = encode_chain_event(&applied.event);
    (applied, bytes)
}

// =============================================================================
// CE-N-B-2 mandatory scenarios
// =============================================================================

#[test]
fn rollback_within_k_succeeds() {
    let corpus = load(WITHIN_K_JSON);
    let (applied, _bytes) = run_scenario(&corpus);
    match &applied.event {
        ChainEvent::RolledBack { to_point, depth } => {
            assert_eq!(to_point, &parse_point(&corpus["request"]["to_point"]));
            assert_eq!(
                depth.0,
                corpus["expected_depth"].as_u64().expect("expected_depth")
            );
        }
        other => panic!("expected RolledBack, got {:?}", other),
    }
    // State adopts the request's tip + supplied tiebreaker.
    assert_eq!(
        applied.new_state.current_tip,
        parse_point(&corpus["request"]["to_point"])
    );
    assert_eq!(
        applied.new_state.current_tip_block_no.0,
        corpus["expected_to_block_no"]
            .as_u64()
            .expect("expected_to_block_no")
    );
    assert_eq!(
        applied.new_state.current_tiebreaker,
        parse_tiebreaker(&corpus["rolled_back_tiebreaker"])
    );
    // Immutable tip + security param unchanged.
    let original_state = parse_state(&corpus["state"]);
    assert_eq!(applied.new_state.immutable_tip, original_state.immutable_tip);
    assert_eq!(
        applied.new_state.immutable_tip_block_no,
        original_state.immutable_tip_block_no
    );
    assert_eq!(
        applied.new_state.security_param,
        original_state.security_param
    );
    // Chain-dep state is the supplied rolled-back chain-dep verbatim.
    assert_eq!(applied.new_chain_dep, parse_chain_dep_seed(&corpus));
}

#[test]
fn rollback_exceeding_k_rejected_with_typed_reason() {
    let corpus = load(EXCEEDS_K_JSON);
    let (applied, _bytes) = run_scenario(&corpus);
    match &applied.event {
        ChainEvent::Rejected {
            reason: ChainSelectionReject::ExceededRollback { requested, max },
        } => {
            assert_eq!(
                requested.0,
                corpus["expected_requested"].as_u64().expect("requested")
            );
            assert_eq!(max.0, corpus["expected_max"].as_u64().expect("max"));
        }
        other => panic!("expected ExceededRollback, got {:?}", other),
    }
    // State + chain_dep unchanged on reject.
    let original_state = parse_state(&corpus["state"]);
    let now_cd = PraosChainDepState::genesis(Nonce(Hash32([0x99; 32])));
    assert_eq!(applied.new_state, original_state);
    assert_eq!(applied.new_chain_dep, now_cd);
}

#[test]
fn rollback_before_immutable_tip_rejected() {
    let corpus = load(BEFORE_IMMUTABLE_JSON);
    let (applied, _bytes) = run_scenario(&corpus);
    match &applied.event {
        ChainEvent::Rejected {
            reason:
                ChainSelectionReject::ForkBeforeImmutableTip {
                    rollback_depth,
                    security_param,
                    ..
                },
        } => {
            assert_eq!(
                rollback_depth.0,
                corpus["expected_rollback_depth"]
                    .as_u64()
                    .expect("rollback_depth")
            );
            assert_eq!(
                security_param.0,
                corpus["expected_security_param"]
                    .as_u64()
                    .expect("security_param")
            );
        }
        other => panic!("expected ForkBeforeImmutableTip, got {:?}", other),
    }
    // State + chain_dep unchanged on reject.
    let original_state = parse_state(&corpus["state"]);
    let now_cd = PraosChainDepState::genesis(Nonce(Hash32([0x99; 32])));
    assert_eq!(applied.new_state, original_state);
    assert_eq!(applied.new_chain_dep, now_cd);
}

#[test]
fn rollback_event_bytes_are_stable() {
    let mut failures: Vec<(String, String, String)> = Vec::new();
    for (name, json) in [
        ("within_k", WITHIN_K_JSON),
        ("exceeds_k", EXCEEDS_K_JSON),
        ("before_immutable", BEFORE_IMMUTABLE_JSON),
    ] {
        let corpus = load(json);
        let (_applied, bytes) = run_scenario(&corpus);
        let actual = to_hex(&bytes);
        let expected = corpus["expected_event_bytes_hex"]
            .as_str()
            .unwrap_or_else(|| panic!("scenario {} missing expected_event_bytes_hex", name))
            .to_string();
        if actual != expected {
            failures.push((name.to_string(), actual, expected));
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
fn rollback_is_deterministic() {
    for json in [WITHIN_K_JSON, EXCEEDS_K_JSON, BEFORE_IMMUTABLE_JSON] {
        let corpus = load(json);
        let (a1, b1) = run_scenario(&corpus);
        let (a2, b2) = run_scenario(&corpus);
        assert_eq!(a1, a2, "RollBackApplied differs across runs");
        assert_eq!(b1, b2, "encoded event bytes differ across runs");
    }
}

// =============================================================================
// Truncated-replay equivalence — DC-CONS-06 load-bearing
// =============================================================================

fn schedule() -> EraSchedule {
    let eras = vec![EraSummary {
        era: CardanoEra::Shelley,
        start_slot: SlotNo(0),
        start_epoch: EpochNo(0),
        slot_length_ms: 1_000,
        epoch_length_slots: 432_000,
        safe_zone_slots: 129_600,
    }];
    EraSchedule::new(BootstrapAnchorHash(Hash32([0u8; 32])), 0, eras)
        .expect("schedule constructs")
}

fn pool() -> Hash28 {
    Hash28([0xAA; 28])
}

fn key_material(seed: [u8; 32]) -> ([u8; 64], VrfVerificationKey) {
    let (sk, vk_bytes) = VrfDraft03::keypair_from_seed(&seed);
    (sk, VrfVerificationKey(vk_bytes))
}

fn prove(sk: &[u8; 64], slot: SlotNo, epoch_nonce: &Nonce, role: VrfRole) -> VrfProof {
    let alpha = vrf_input(slot, epoch_nonce, role);
    let bytes = VrfDraft03::prove(sk, &alpha).expect("VRF prove");
    VrfProof(bytes)
}

fn ledger(vk: VrfVerificationKey) -> LedgerViewStub {
    // asc = 1/1 and sigma = 1/1: every VRF output trivially leads, so
    // we can chain headers without arranging pool-fraction puzzles.
    let mut pools = BTreeMap::new();
    pools.insert(
        pool(),
        PoolFixture {
            active_stake: 1,
            vrf_keyhash: ade_crypto::blake2b::blake2b_256(&vk.0),
        },
    );
    LedgerViewStub::new().with_epoch(
        EpochNo(0),
        EpochStakeFixture {
            total_active_stake: 1,
            asc: ActiveSlotsCoeff { numer: 1, denom: 1 },
            pools,
        },
    )
}

fn genesis_chain_dep() -> PraosChainDepState {
    let mut s = PraosChainDepState::empty();
    s.epoch_nonce = Nonce(Hash32([0xCD; 32]));
    s.evolving_nonce = Nonce(Hash32([0xEE; 32]));
    s
}

fn header_at(
    sk: &[u8; 64],
    vk: &VrfVerificationKey,
    state: &PraosChainDepState,
    slot: SlotNo,
    block_no: BlockNo,
    op_cert_counter: u64,
) -> HeaderInput {
    HeaderInput {
        prev_hash: Hash32([0u8; 32]),
        slot,
        block_no,
        body_hash: Hash32([0x55; 32]),
        issuer_pool: pool(),
        op_cert_kes_period: 0,
        op_cert_counter,
        vrf_vk: vk.clone(),
        vrf: HeaderVrf::Tpraos {
            nonce_proof: prove(sk, slot, &state.epoch_nonce, VrfRole::NonceContribution),
            leader_proof: prove(sk, slot, &state.epoch_nonce, VrfRole::LeaderEligibility),
        },
        kes: None,
    }
}

/// DC-CONS-06: rollback(state, depth) produces state byte-identical to
/// truncated replay from the nearest checkpoint. We exercise this by:
///   1. Apply N = 5 validated headers from genesis (op_cert_counter is
///      strictly increasing so all 5 admit cleanly).
///   2. Snapshot `PraosChainDepState` at block K = 3 (after 3 headers).
///   3. Apply 2 more headers — chain-dep state advances past the
///      snapshot.
///   4. Build a `ChainSelectorState` at block 5 and `RollBackRequest`
///      with `to_block_no = 3` and `depth = 2`.
///   5. Call `apply_rollback` supplying the snapshot as the
///      rolled-back chain-dep state.
///   6. Assert `applied.new_chain_dep == snapshot`. This is the
///      byte-identity claim that closes DC-CONS-06.
#[test]
fn rollback_equivalent_to_truncated_replay() {
    let (sk, vk) = key_material([42u8; 32]);

    // Step 1: apply N = 5 headers from genesis.
    let mut chain_dep = genesis_chain_dep();
    let mut applied_summaries = Vec::new();
    for i in 1u64..=5u64 {
        let h = header_at(
            &sk,
            &vk,
            &chain_dep,
            SlotNo(i),
            BlockNo(i),
            i - 1, // counter strictly increasing: 0,1,2,3,4
        );
        let res = validate_and_apply_header(&chain_dep, &h, &ledger(vk.clone()), &schedule())
            .expect("header admits");
        chain_dep = res.new_state;
        applied_summaries.push(res.summary);

        // Step 2: snapshot the chain-dep state immediately after
        // applying the K = 3rd header.
        if i == 3 {
            let snapshot = chain_dep.clone();
            // Continue to step 3 below — applies headers 4 and 5.
            for j in 4u64..=5u64 {
                let h2 = header_at(
                    &sk,
                    &vk,
                    &chain_dep,
                    SlotNo(j),
                    BlockNo(j),
                    j - 1,
                );
                let res2 = validate_and_apply_header(
                    &chain_dep,
                    &h2,
                    &ledger(vk.clone()),
                    &schedule(),
                )
                .expect("subsequent header admits");
                chain_dep = res2.new_state;
                applied_summaries.push(res2.summary);
            }
            // The applied chain-dep is now past the snapshot.
            assert_ne!(
                chain_dep, snapshot,
                "advancing the chain should change the chain-dep state"
            );

            // Step 4 + 5: build the selector state and request, then
            // call `apply_rollback`.
            let summary_at_block_5 = applied_summaries.last().expect("five headers applied");
            let current_tip = Point {
                slot: summary_at_block_5.slot,
                hash: summary_at_block_5.body_hash.clone(),
            };
            let current_tiebreaker = TiebreakerView {
                slot: summary_at_block_5.slot,
                issuer_hash: summary_at_block_5.issuer_pool.clone(),
                op_cert_counter: summary_at_block_5.op_cert_counter,
                leader_vrf_output_first_8: {
                    let mut b = [0u8; 8];
                    b.copy_from_slice(&summary_at_block_5.vrf_leader_output.0[0..8]);
                    b
                },
            };
            let summary_at_block_3 = &applied_summaries[2];
            let rolled_back_point = Point {
                slot: summary_at_block_3.slot,
                hash: summary_at_block_3.body_hash.clone(),
            };
            let rolled_back_tiebreaker = TiebreakerView {
                slot: summary_at_block_3.slot,
                issuer_hash: summary_at_block_3.issuer_pool.clone(),
                op_cert_counter: summary_at_block_3.op_cert_counter,
                leader_vrf_output_first_8: {
                    let mut b = [0u8; 8];
                    b.copy_from_slice(&summary_at_block_3.vrf_leader_output.0[0..8]);
                    b
                },
            };
            let selector_state = ChainSelectorState {
                current_tip,
                current_tip_block_no: BlockNo(5),
                current_tiebreaker,
                // Immutable tip set well behind block 3 so the request
                // does not refuse on the immutable-tip rule.
                immutable_tip: Point {
                    slot: SlotNo(0),
                    hash: Hash32([0u8; 32]),
                },
                immutable_tip_block_no: BlockNo(0),
                security_param: SecurityParam(2160),
            };
            let request = RollBackRequest {
                to_point: rolled_back_point,
                to_block_no: BlockNo(3),
                depth: BlockDistance(2),
            };
            let result = apply_rollback(
                &selector_state,
                &chain_dep,
                &snapshot,
                &rolled_back_tiebreaker,
                &request,
            );

            // Step 6: byte-identity assertion. This is the load-bearing
            // claim for DC-CONS-06.
            assert_eq!(
                result.new_chain_dep, snapshot,
                "rolled-back chain-dep state is not byte-identical to the K=3 snapshot",
            );

            // Event shape is RolledBack with the supplied depth.
            match result.event {
                ChainEvent::RolledBack { depth, .. } => {
                    assert_eq!(depth, BlockDistance(2));
                }
                other => panic!("expected RolledBack, got {:?}", other),
            }
            return;
        }
    }
    panic!("loop should have exercised the K=3 snapshot branch");
}
