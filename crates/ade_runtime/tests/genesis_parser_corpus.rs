// Integration test for the RED genesis parser. The slice doc (S-B1)
// permits using synthetic JSON fixtures here when vendoring real
// cardano-node mainnet genesis blobs is out of scope. The intent is
// to prove the parser is deterministic and well-formed; the bound
// against real mainnet ground truth is deferred to S-B10 (gated below
// under `#[ignore]`).
//
// Synthetic fixtures live alongside this test as embedded strings,
// and a small JSON oracle at `corpus/consensus/hfc_schedule/<net>.json`
// declares the expected `expected_eras` to compare against. We do
// not yet pin the exact `expected_anchor_hex` — see
// `mainnet_parser_anchor_pinned` below for the future evidence pass.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::path::PathBuf;

use ade_runtime::consensus::{
    compute_anchor_hash, parse_genesis, GenesisBundle, NetworkMagic,
};
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

fn read_file(p: &PathBuf) -> Vec<u8> {
    use std::io::Read;
    let mut f = std::fs::File::open(p).expect("file readable");
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).expect("read");
    buf
}

const BYRON_FIXTURE: &str = r#"{
    "protocolConsts": { "k": 2160 },
    "startTime": 1506203091,
    "blockVersionData": { "slotDuration": "20000" }
}"#;

const SHELLEY_FIXTURE: &str = r#"{
    "epochLength": 432000,
    "slotLength": 1,
    "activeSlotsCoeff": { "numerator": 1, "denominator": 20 },
    "securityParam": 2160,
    "_ade_boundaries": {
        "mainnet": {
            "byron_start_epoch": 0,
            "shelley":  { "start_slot": 4492800,   "start_epoch": 208 },
            "allegra":  { "start_slot": 16588800,  "start_epoch": 236 },
            "mary":     { "start_slot": 23068800,  "start_epoch": 251 },
            "alonzo":   { "start_slot": 39916800,  "start_epoch": 290 },
            "babbage":  { "start_slot": 72316796,  "start_epoch": 365 },
            "conway":   { "start_slot": 133660800, "start_epoch": 507 }
        },
        "preprod": {
            "byron_start_epoch": 0,
            "shelley":  { "start_slot": 86400,    "start_epoch": 4 },
            "allegra":  { "start_slot": 518400,   "start_epoch": 5 },
            "mary":     { "start_slot": 950400,   "start_epoch": 6 },
            "alonzo":   { "start_slot": 1382400,  "start_epoch": 7 },
            "babbage":  { "start_slot": 1814400,  "start_epoch": 8 },
            "conway":   { "start_slot": 55814400, "start_epoch": 132 }
        }
    }
}"#;

const ALONZO_FIXTURE: &str = r#"{ "lovelacePerUTxOWord": 34482 }"#;
const CONWAY_FIXTURE: &str = r#"{ "poolVotingThresholds": {} }"#;

fn synthetic_bundle<'a>() -> GenesisBundle<'a> {
    GenesisBundle {
        byron_json: BYRON_FIXTURE.as_bytes(),
        shelley_json: SHELLEY_FIXTURE.as_bytes(),
        alonzo_json: ALONZO_FIXTURE.as_bytes(),
        conway_json: CONWAY_FIXTURE.as_bytes(),
    }
}

#[test]
fn mainnet_parser_eras_match_corpus_oracle() {
    let schedule = parse_genesis(&synthetic_bundle(), NetworkMagic::MAINNET)
        .expect("parser succeeds on synthetic mainnet fixture");
    let bytes = read_file(&corpus_path("mainnet.json"));
    let oracle: Value = serde_json::from_slice(&bytes).expect("oracle parses");
    let oracle_eras = oracle["expected_eras"].as_array().expect("eras array");
    assert_eq!(schedule.eras().len(), oracle_eras.len());
    for (era, expected) in schedule.eras().iter().zip(oracle_eras.iter()) {
        assert_eq!(
            era.start_slot.0,
            expected["start_slot"].as_u64().expect("oracle slot"),
            "boundary slot mismatch for era {:?}",
            era.era
        );
        assert_eq!(
            era.start_epoch.0,
            expected["start_epoch"].as_u64().expect("oracle epoch"),
            "start_epoch mismatch for era {:?}",
            era.era
        );
        assert_eq!(
            u64::from(era.slot_length_ms),
            expected["slot_length_ms"].as_u64().expect("oracle slot_length"),
            "slot_length_ms mismatch for era {:?}",
            era.era
        );
        assert_eq!(
            u64::from(era.epoch_length_slots),
            expected["epoch_length_slots"]
                .as_u64()
                .expect("oracle epoch_length"),
            "epoch_length_slots mismatch for era {:?}",
            era.era
        );
        assert_eq!(
            u64::from(era.safe_zone_slots),
            expected["safe_zone_slots"]
                .as_u64()
                .expect("oracle safe_zone"),
            "safe_zone_slots mismatch for era {:?}",
            era.era
        );
    }
}

#[test]
fn preprod_parser_eras_match_corpus_oracle() {
    let schedule = parse_genesis(&synthetic_bundle(), NetworkMagic::PREPROD)
        .expect("parser succeeds on synthetic preprod fixture");
    let bytes = read_file(&corpus_path("preprod.json"));
    let oracle: Value = serde_json::from_slice(&bytes).expect("oracle parses");
    let oracle_eras = oracle["expected_eras"].as_array().expect("eras array");
    assert_eq!(schedule.eras().len(), oracle_eras.len());
    for (era, expected) in schedule.eras().iter().zip(oracle_eras.iter()) {
        assert_eq!(
            era.start_slot.0,
            expected["start_slot"].as_u64().expect("oracle slot"),
            "boundary slot mismatch for era {:?}",
            era.era
        );
        assert_eq!(
            era.start_epoch.0,
            expected["start_epoch"].as_u64().expect("oracle epoch"),
            "start_epoch mismatch for era {:?}",
            era.era
        );
    }
}

#[test]
fn parser_anchor_is_deterministic() {
    let h1 = compute_anchor_hash(&synthetic_bundle());
    let h2 = compute_anchor_hash(&synthetic_bundle());
    assert_eq!(h1, h2);
}

/// Future evidence pass (S-B10): the synthetic fixtures here are
/// stand-ins for the real cardano-node 10.6.2 mainnet genesis blobs.
/// Vendoring those blobs and pinning the expected anchor against them
/// is deferred. Run only with `--ignored` against pinned vendored
/// blobs at `corpus/consensus/hfc_schedule/genesis/{byron,shelley,
/// alonzo,conway}.json`.
#[test]
#[ignore = "real mainnet genesis blobs not vendored — pinned in S-B10"]
fn mainnet_parser_anchor_pinned_against_vendored_genesis() {
    let byron = read_file(&corpus_path("genesis/byron.json"));
    let shelley = read_file(&corpus_path("genesis/shelley.json"));
    let alonzo = read_file(&corpus_path("genesis/alonzo.json"));
    let conway = read_file(&corpus_path("genesis/conway.json"));
    let bundle = GenesisBundle {
        byron_json: &byron,
        shelley_json: &shelley,
        alonzo_json: &alonzo,
        conway_json: &conway,
    };
    let schedule = parse_genesis(&bundle, NetworkMagic::MAINNET).expect("parses");
    let oracle: Value =
        serde_json::from_slice(&read_file(&corpus_path("mainnet.json"))).expect("oracle");
    let expected_hex = oracle["expected_anchor_hex"].as_str().expect("anchor hex");
    let actual_hex = format!("{}", schedule.anchor().0);
    assert_eq!(actual_hex, expected_hex);
}
