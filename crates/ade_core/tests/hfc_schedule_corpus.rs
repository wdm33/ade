// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// CE-N-B-3 corpus test: builds an EraSchedule from the corpus
// `expected_eras` and asserts that probe-point `locate()` and
// `slot_to_time_ms()` answers match the oracle, and the horizon probe
// triggers `OutsideForecastRange`. This test exercises the BLUE
// translation path only — RED parser coverage lives in
// `ade_runtime::tests::genesis_parser_corpus`.
//
// Integration test (under `crates/ade_core/tests/`) — compiled
// separately from the BLUE library crate. File I/O here is shell
// behavior, not BLUE behavior.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::path::PathBuf;

use ade_core::consensus::{
    BootstrapAnchorHash, EraSchedule, EraSummary, OutsideForecastRange,
};
use ade_types::{CardanoEra, EpochNo, Hash32, SlotNo};
use serde_json::Value;

fn corpus_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.push("corpus");
    p.push("consensus");
    p.push("hfc_schedule");
    p.push(name);
    p
}

fn parse_era(name: &str) -> CardanoEra {
    match name {
        "ByronEbb" => CardanoEra::ByronEbb,
        "ByronRegular" => CardanoEra::ByronRegular,
        "Shelley" => CardanoEra::Shelley,
        "Allegra" => CardanoEra::Allegra,
        "Mary" => CardanoEra::Mary,
        "Alonzo" => CardanoEra::Alonzo,
        "Babbage" => CardanoEra::Babbage,
        "Conway" => CardanoEra::Conway,
        other => panic!("unknown era name in corpus: {other}"),
    }
}

fn parse_anchor_hex(hex: &str) -> BootstrapAnchorHash {
    let raw = hex_decode(hex);
    let mut out = [0u8; 32];
    out.copy_from_slice(&raw);
    BootstrapAnchorHash(Hash32(out))
}

fn hex_decode(s: &str) -> Vec<u8> {
    assert_eq!(s.len() % 2, 0, "hex string must have even length");
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = nib(bytes[i]);
        let lo = nib(bytes[i + 1]);
        out.push((hi << 4) | lo);
        i += 2;
    }
    out
}

fn nib(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => panic!("invalid hex digit"),
    }
}

fn load_corpus(name: &str) -> Value {
    let bytes = read_corpus_bytes(name);
    serde_json::from_slice(&bytes).expect("corpus is valid JSON")
}

fn read_corpus_bytes(name: &str) -> Vec<u8> {
    let p = corpus_path(name);
    read_file(&p)
}

fn read_file(p: &PathBuf) -> Vec<u8> {
    let f = open_for_read(p);
    read_to_end(f)
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

fn build_schedule(v: &Value) -> EraSchedule {
    let anchor_hex = v["expected_anchor_hex"].as_str().expect("anchor hex");
    let anchor = parse_anchor_hex(anchor_hex);
    let system_start_unix_ms = v["system_start_unix_ms"].as_u64().expect("system_start");
    let eras: Vec<EraSummary> = v["expected_eras"]
        .as_array()
        .expect("expected_eras array")
        .iter()
        .map(|e| EraSummary {
            era: parse_era(e["era"].as_str().expect("era name")),
            start_slot: SlotNo(e["start_slot"].as_u64().expect("start_slot")),
            start_epoch: EpochNo(e["start_epoch"].as_u64().expect("start_epoch")),
            slot_length_ms: e["slot_length_ms"].as_u64().expect("slot_length_ms") as u32,
            epoch_length_slots: e["epoch_length_slots"]
                .as_u64()
                .expect("epoch_length_slots") as u32,
            safe_zone_slots: e["safe_zone_slots"].as_u64().expect("safe_zone_slots") as u32,
        })
        .collect();
    EraSchedule::new(anchor, system_start_unix_ms, eras).expect("schedule constructs")
}

fn assert_probes(v: &Value, schedule: &EraSchedule) {
    for probe in v["probe_points"].as_array().expect("probe_points array") {
        let slot = SlotNo(probe["slot"].as_u64().expect("probe slot"));
        let expected_era = parse_era(probe["era"].as_str().expect("probe era"));
        let expected_epoch = EpochNo(probe["epoch"].as_u64().expect("probe epoch"));
        let expected_relative = probe["relative"].as_u64().expect("probe relative") as u32;
        let expected_time_ms = probe["time_ms"].as_u64().expect("probe time_ms");
        let mut answers: Vec<(CardanoEra, EpochNo, u32, u64)> = Vec::new();
        for _ in 0..2 {
            let loc = schedule.locate(slot).expect("probe slot must locate");
            let time = schedule
                .slot_to_time_ms(slot)
                .expect("probe slot must convert");
            answers.push((loc.era, loc.epoch, loc.relative_slot_in_epoch, time));
        }
        assert_eq!(answers[0], answers[1], "non-deterministic slot {}", slot.0);
        let (era, epoch, relative, time) = answers[0];
        assert_eq!(era, expected_era, "era mismatch at slot {}", slot.0);
        assert_eq!(epoch, expected_epoch, "epoch mismatch at slot {}", slot.0);
        assert_eq!(
            relative, expected_relative,
            "relative-slot mismatch at slot {}",
            slot.0
        );
        assert_eq!(time, expected_time_ms, "time_ms mismatch at slot {}", slot.0);
    }
}

fn assert_horizon(v: &Value, schedule: &EraSchedule) {
    let probe = &v["horizon_probe"];
    let slot = SlotNo(probe["slot"].as_u64().expect("horizon probe slot"));
    let expected_err = probe["expected_horizon_error"]
        .as_bool()
        .expect("expected_horizon_error");
    let result: Result<(), OutsideForecastRange> = schedule.check_forecast_horizon(slot);
    if expected_err {
        assert!(result.is_err(), "horizon must fail at slot {}", slot.0);
        if let Err(e) = result {
            assert_eq!(e.requested, slot);
            assert!(e.horizon.0 < slot.0);
        }
    } else {
        assert!(result.is_ok());
    }
}

#[test]
fn mainnet_corpus_translation_matches_oracle() {
    let v = load_corpus("mainnet.json");
    let schedule = build_schedule(&v);
    assert_probes(&v, &schedule);
    assert_horizon(&v, &schedule);
}

#[test]
fn preprod_corpus_translation_matches_oracle() {
    let v = load_corpus("preprod.json");
    let schedule = build_schedule(&v);
    assert_probes(&v, &schedule);
    assert_horizon(&v, &schedule);
}

#[test]
fn bootstrap_anchor_hash_distinguishes_genesis_variants() {
    let mainnet = load_corpus("mainnet.json");
    let preprod = load_corpus("preprod.json");
    let main_anchor = parse_anchor_hex(mainnet["expected_anchor_hex"].as_str().expect("anchor"));
    let pre_anchor = parse_anchor_hex(preprod["expected_anchor_hex"].as_str().expect("anchor"));
    assert_ne!(
        main_anchor, pre_anchor,
        "mainnet and preprod anchors must differ"
    );
}
