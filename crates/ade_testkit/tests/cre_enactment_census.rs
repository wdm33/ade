//! CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY — the ratify→enact ground-truth census (read-only, GROUND
//! TRUTH for a FUTURE ratify/enact cluster; NOT authorization to expand S4).
//!
//! Target: the Preview param-update action 69c948cd..#0 ("Increase Tx/Block Memory Units pt1"), submitted
//! epoch 1088, ENACTED (obs: maxTxExUnits.mem=16,500,000, maxBlockExUnits.mem=72,000,000). The local
//! ChainDB — not explorer metadata — establishes the exact ratify/enact boundary across epochs 1087-1103.
//! States are extracted via LOCAL db-analyser `--store-ledger` (see scratchpad/cre-census/extract_all.sh).
//!
//! This file starts with a single-epoch PROBE (does the decoder work at epoch 1088, ~2 years before the
//! CE-3d corpus?); the full per-transition census + fixture follows as the window extraction completes.
//! `#[ignore]`'d (reads local artifacts).

use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
use ade_types::conway::governance::GovActionId;
use ade_types::{Hash32, SlotNo};

fn h32(hex: &str) -> Hash32 {
    let b: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let mut h = [0u8; 32];
    h.copy_from_slice(&b);
    Hash32(h)
}

/// The target action id: tx 69c948cd..1f69, index 0.
fn target() -> GovActionId {
    GovActionId {
        tx_hash: h32("69c948cde90c6b9d7d61595e8534c106ec44132cb049ab2558399db1260c1f69"),
        index: 0,
    }
}

fn ledger_state_path(slot: u64) -> String {
    format!("/home/ts/.cardano-ce3d-extract/db/ledger/{slot}_db-analyser/state")
}

#[test]
#[ignore = "reads the local db-analyser epoch-1088 state; run explicitly (CRE enactment-census probe)"]
fn cre_census_probe_epoch_1088() {
    // Epoch 1088 first block = slot 94003205 (115862400 - (1341-1088)*86400, first block >= 94003200).
    let path = ledger_state_path(94003205);
    let state = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let point = SeedPoint { slot: SlotNo(94003205), block_hash: Hash32([0u8; 32]) };
    let (s1a, commit) = decode_native_nonutxo_state(&state, point, 1088, 2)
        .unwrap_or_else(|e| panic!("decode epoch 1088 @94003205: {e:?}"));
    let g = &s1a.imported_gov;
    let present = g.proposals.iter().any(|p| p.action_id == target());
    eprintln!(
        "epoch 1088 @94003205: {} proposals | target 69c948cd..#0 present={} | committee={} quorum={:?} | \
         maxTxExUnits.mem={} | gov-commit={:02x}{:02x}{:02x}{:02x}",
        g.proposals.len(),
        present,
        g.committee.len(),
        g.committee_quorum,
        s1a.protocol_params.max_tx_ex_units_mem,
        commit.0[0], commit.0[1], commit.0[2], commit.0[3],
    );
    // The first block of epoch 1088 is BEFORE the in-epoch submission, so the action is expected ABSENT
    // here (it appears at the 1088->1089 boundary). The probe's job is only to prove the decoder reads this
    // far-earlier Conway state at all; the presence/ratify/enact lifecycle is the full census below.
    assert!(
        s1a.protocol_params.max_tx_ex_units_mem > 0,
        "the decoder reads epoch-1088 curPParams (a real maxTxExUnits.mem)"
    );
}

/// Partial census over whatever window states are extracted so far (auto-discovers the *_db-analyser
/// snapshots in the 1087-1103 slot range). Reports the lifecycle-so-far: action presence + maxTxExUnits.mem.
#[test]
#[ignore = "reads local db-analyser window states as they extract; run for a live census status"]
fn cre_census_partial_available() {
    let dir = "/home/ts/.cardano-ce3d-extract/db/ledger";
    let mut slots: Vec<u64> = std::fs::read_dir(dir)
        .expect("ledger dir")
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            e.file_name()
                .to_str()?
                .strip_suffix("_db-analyser")
                .and_then(|s| s.parse::<u64>().ok())
        })
        .filter(|s| (93_900_000..=95_400_000).contains(s))
        .collect();
    slots.sort_unstable();
    eprintln!("=== CRE ENACTMENT-CENSUS (partial, {} epochs extracted) ===", slots.len());
    for slot in slots {
        // The stored slot is a few slots INTO its epoch; round the (115862400-slot) gap UP to whole epochs.
        let epoch = 1341 - (115_862_400 - slot + 86_399) / 86_400;
        let state = std::fs::read(format!("{dir}/{slot}_db-analyser/state")).expect("state");
        let point = SeedPoint { slot: SlotNo(slot), block_hash: Hash32([0u8; 32]) };
        let (s1a, _) = match decode_native_nonutxo_state(&state, point, epoch, 2) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("epoch {epoch} @{slot}: DECODE FAILED -> {e:?}");
                continue;
            }
        };
        let g = &s1a.imported_gov;
        let present = g.proposals.iter().any(|p| p.action_id == target());
        eprintln!(
            "epoch {epoch} @{slot}: {:>3} proposals | target present={:<5} | maxTxMem={} maxBlockMem={} | deposit_pot={}",
            g.proposals.len(),
            present,
            s1a.protocol_params.max_tx_ex_units_mem,
            s1a.max_block_ex_units_mem,
            s1a.gov_deposit_pot.0,
        );
    }
}
