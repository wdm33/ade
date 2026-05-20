// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// CE-N-B-4 close — replays a pinned leader-schedule scenario against
// query_leader_schedule and verifies that the answer's
// (expected_vrf_input, stake_fraction, asc) and the error variants for
// unknown-pool / outside-forecast match the corpus byte-for-byte.
//
// Integration test (compiled separately from the BLUE library crate).
// The corpus is `include_str!`d so the test reaches the corpus through
// a compile-time literal — no runtime filesystem access.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::collections::BTreeMap;

use ade_core::consensus::{
    is_leader_for_vrf_output, query_leader_schedule, ActiveSlotsCoeff, BootstrapAnchorHash,
    EraSchedule, EraSummary, LeaderScheduleError, LeaderScheduleQuery, Nonce,
    OutsideForecastRange, PraosChainDepState,
};
use ade_crypto::blake2b::blake2b_256;
use ade_crypto::vrf::{VrfOutput, VrfVerificationKey};
use ade_testkit::consensus::ledger_view_stub::{
    EpochStakeFixture, LedgerViewStub, PoolFixture,
};
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};
use serde_json::Value;

const CORPUS_JSON: &str =
    include_str!("../../../corpus/consensus/leader_schedule/scenario_one_epoch.json");

fn load() -> Value {
    serde_json::from_str(CORPUS_JSON).expect("corpus is valid JSON")
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

fn hash28_from_hex(s: &str) -> Hash28 {
    let raw = from_hex(s);
    assert_eq!(raw.len(), 28, "expected 28 bytes");
    let mut out = [0u8; 28];
    out.copy_from_slice(&raw);
    Hash28(out)
}

fn hash32_from_hex(s: &str) -> Hash32 {
    let raw = from_hex(s);
    assert_eq!(raw.len(), 32, "expected 32 bytes");
    let mut out = [0u8; 32];
    out.copy_from_slice(&raw);
    Hash32(out)
}

fn vrf_key_from_hex(s: &str) -> VrfVerificationKey {
    let raw = from_hex(s);
    assert_eq!(raw.len(), 32, "vrf key must be 32 bytes");
    let mut out = [0u8; 32];
    out.copy_from_slice(&raw);
    VrfVerificationKey(out)
}

fn vrf_output_from_hex(s: &str) -> VrfOutput {
    let raw = from_hex(s);
    assert_eq!(raw.len(), 64, "vrf output must be 64 bytes");
    let mut out = [0u8; 64];
    out.copy_from_slice(&raw);
    VrfOutput(out)
}

fn build_schedule(corpus: &Value) -> EraSchedule {
    let es = &corpus["era_schedule"];
    let mut eras = Vec::new();
    for e in es["eras"].as_array().expect("eras array") {
        let era = match e["era"].as_str().expect("era") {
            "Byron" => CardanoEra::ByronRegular,
            "Shelley" => CardanoEra::Shelley,
            "Allegra" => CardanoEra::Allegra,
            "Mary" => CardanoEra::Mary,
            "Alonzo" => CardanoEra::Alonzo,
            "Babbage" => CardanoEra::Babbage,
            "Conway" => CardanoEra::Conway,
            other => panic!("unknown era {other}"),
        };
        eras.push(EraSummary {
            era,
            start_slot: SlotNo(e["start_slot"].as_u64().expect("start_slot")),
            start_epoch: EpochNo(e["start_epoch"].as_u64().expect("start_epoch")),
            slot_length_ms: e["slot_length_ms"].as_u64().expect("slot_length_ms") as u32,
            epoch_length_slots: e["epoch_length_slots"]
                .as_u64()
                .expect("epoch_length_slots") as u32,
            safe_zone_slots: e["safe_zone_slots"].as_u64().expect("safe_zone_slots") as u32,
        });
    }
    let system_start = es["system_start_unix_ms"]
        .as_u64()
        .expect("system_start_unix_ms");
    EraSchedule::new(BootstrapAnchorHash(Hash32([0u8; 32])), system_start, eras)
        .expect("schedule constructs")
}

fn build_state(corpus: &Value) -> PraosChainDepState {
    let mut s = PraosChainDepState::empty();
    s.epoch_nonce = Nonce(hash32_from_hex(
        corpus["epoch_nonce_hex"].as_str().expect("epoch_nonce_hex"),
    ));
    s
}

fn build_ledger(corpus: &Value) -> LedgerViewStub {
    let epoch = EpochNo(corpus["epoch"].as_u64().expect("epoch"));
    let asc_v = &corpus["asc"];
    let asc = ActiveSlotsCoeff {
        numer: asc_v["numer"].as_u64().expect("asc.numer") as u32,
        denom: asc_v["denom"].as_u64().expect("asc.denom") as u32,
    };
    let total = corpus["total_active_stake"]
        .as_u64()
        .expect("total_active_stake");
    let mut pools = BTreeMap::new();
    for p in corpus["pools"].as_array().expect("pools array") {
        let pool = hash28_from_hex(p["pool_hex"].as_str().expect("pool_hex"));
        let vk = vrf_key_from_hex(p["vrf_key_hex"].as_str().expect("vrf_key_hex"));
        // The ledger holds the registered keyhash, not the vkey; the corpus
        // pins the vkey, so derive the keyhash the same way the chain does.
        let vrf_keyhash = blake2b_256(&vk.0);
        let stake = p["active_stake"].as_u64().expect("active_stake");
        pools.insert(
            pool,
            PoolFixture {
                active_stake: stake,
                vrf_keyhash,
            },
        );
    }
    LedgerViewStub::new().with_epoch(
        epoch,
        EpochStakeFixture {
            total_active_stake: total,
            asc,
            pools,
        },
    )
}

#[test]
fn corpus_returns_canonical_answer_for_known_pools() {
    let corpus = load();
    let schedule = build_schedule(&corpus);
    let state = build_state(&corpus);
    let ledger = build_ledger(&corpus);

    for q in corpus["queries"].as_array().expect("queries") {
        if q.get("expected_error").is_some() {
            continue;
        }
        let slot = SlotNo(q["slot"].as_u64().expect("slot"));
        let pool = hash28_from_hex(q["pool_hex"].as_str().expect("pool_hex"));
        let expected_epoch = EpochNo(q["expected_epoch"].as_u64().expect("expected_epoch"));
        let expected_sf = q["expected_stake_fraction"]
            .as_array()
            .expect("expected_stake_fraction array");
        let expected_numer = expected_sf[0].as_u64().expect("numer");
        let expected_denom = expected_sf[1].as_u64().expect("denom");
        let expected_input = from_hex(
            q["expected_vrf_input_hex"]
                .as_str()
                .expect("expected_vrf_input_hex"),
        );
        assert_eq!(expected_input.len(), 41, "VRF input must be 41 bytes");

        let answer = query_leader_schedule(
            &LeaderScheduleQuery {
                slot,
                pool: pool.clone(),
            },
            &ledger,
            &schedule,
            &state,
        )
        .expect("query succeeds for known pool");

        assert_eq!(answer.slot, slot);
        assert_eq!(answer.pool, pool);
        assert_eq!(answer.epoch, expected_epoch);
        assert_eq!(answer.stake_fraction, (expected_numer, expected_denom));
        assert_eq!(&answer.expected_vrf_input[..], &expected_input[..]);
    }
}

#[test]
fn corpus_rejects_unknown_pool() {
    let corpus = load();
    let schedule = build_schedule(&corpus);
    let state = build_state(&corpus);
    let ledger = build_ledger(&corpus);

    let mut saw_unknown = false;
    for q in corpus["queries"].as_array().expect("queries") {
        let Some(err) = q.get("expected_error").and_then(|v| v.as_str()) else {
            continue;
        };
        if err != "UnknownPool" {
            continue;
        }
        let slot = SlotNo(q["slot"].as_u64().expect("slot"));
        let pool = hash28_from_hex(q["pool_hex"].as_str().expect("pool_hex"));
        let res = query_leader_schedule(
            &LeaderScheduleQuery { slot, pool },
            &ledger,
            &schedule,
            &state,
        );
        assert_eq!(res, Err(LeaderScheduleError::UnknownPool));
        saw_unknown = true;
    }
    assert!(
        saw_unknown,
        "corpus must contain at least one UnknownPool query"
    );
}

#[test]
fn corpus_rejects_out_of_forecast_horizon() {
    let corpus = load();
    let schedule = build_schedule(&corpus);
    let state = build_state(&corpus);
    let ledger = build_ledger(&corpus);

    let probe = &corpus["horizon_probe"];
    let slot = SlotNo(probe["slot"].as_u64().expect("slot"));
    let expected_horizon = SlotNo(
        probe["expected_horizon"]
            .as_u64()
            .expect("expected_horizon"),
    );
    let any_pool = hash28_from_hex(
        corpus["pools"][0]["pool_hex"]
            .as_str()
            .expect("first pool hex"),
    );
    let res = query_leader_schedule(
        &LeaderScheduleQuery {
            slot,
            pool: any_pool,
        },
        &ledger,
        &schedule,
        &state,
    );
    assert_eq!(
        res,
        Err(LeaderScheduleError::OutsideForecastRange(
            OutsideForecastRange {
                requested: slot,
                horizon: expected_horizon,
            }
        ))
    );
}

#[test]
fn corpus_is_leader_helper_matches_pinned_probe() {
    let corpus = load();
    let schedule = build_schedule(&corpus);
    let state = build_state(&corpus);
    let ledger = build_ledger(&corpus);

    let probe = &corpus["leader_probe"];
    let slot = SlotNo(probe["slot"].as_u64().expect("slot"));
    let pool = hash28_from_hex(probe["pool_hex"].as_str().expect("pool_hex"));
    let output = vrf_output_from_hex(probe["vrf_output_hex"].as_str().expect("vrf_output_hex"));
    let expected_leads = probe["expected_leads"].as_bool().expect("expected_leads");

    let answer = query_leader_schedule(
        &LeaderScheduleQuery { slot, pool },
        &ledger,
        &schedule,
        &state,
    )
    .expect("known pool query succeeds");

    let leads = is_leader_for_vrf_output(&answer, &output);
    assert_eq!(leads, expected_leads, "leader probe verdict drift");
}

#[test]
fn corpus_is_deterministic_across_runs() {
    let corpus = load();
    let schedule = build_schedule(&corpus);
    let state = build_state(&corpus);
    let ledger = build_ledger(&corpus);

    let mut first: Option<Vec<Result<_, _>>> = None;
    for _ in 0..2 {
        let mut answers: Vec<Result<_, _>> = Vec::new();
        for q in corpus["queries"].as_array().expect("queries") {
            if q.get("expected_error").is_some() {
                continue;
            }
            let slot = SlotNo(q["slot"].as_u64().expect("slot"));
            let pool = hash28_from_hex(q["pool_hex"].as_str().expect("pool_hex"));
            answers.push(query_leader_schedule(
                &LeaderScheduleQuery { slot, pool },
                &ledger,
                &schedule,
                &state,
            ));
        }
        match &first {
            None => first = Some(answers),
            Some(prev) => assert_eq!(prev, &answers, "non-deterministic replay"),
        }
    }
}
