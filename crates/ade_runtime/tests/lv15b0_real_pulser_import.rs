//! LV-15b-0 CE-8 on real bytes: decode a cardano-node LedgerDB `state` file natively and import its finished
//! ratification (`govState[6]`) as the snapshot epoch's record. Prints the outcome and the evidence sizes.
//! `#[ignore]`d: reads a local state file named by `ADE_LV15B0_STATE` (use a copy; never a live node's directory).

use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::governance::frozen::RatificationOutcome;
use ade_ledger::ledgerdb_state::{decode_native_nonutxo_state, NativeNonUtxoError};
use ade_types::{Hash32, SlotNo};

const PREPROD_NETWORK_MAGIC: u32 = 1;

#[test]
#[ignore = "reads a local LedgerDB state file named by ADE_LV15B0_STATE"]
fn lv15b0_real_snapshot_pulser_imports() {
    let path = std::env::var("ADE_LV15B0_STATE").expect("set ADE_LV15B0_STATE");
    let state = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let point = SeedPoint { slot: SlotNo(0), block_hash: Hash32([0xab; 32]) };
    let epoch = match decode_native_nonutxo_state(&state, point.clone(), u64::MAX, PREPROD_NETWORK_MAGIC) {
        Err(NativeNonUtxoError::EpochMismatch { decoded_epoch, .. }) => decoded_epoch,
        other => panic!("epoch discovery: {:?}", other.err()),
    };
    let (s, _) = decode_native_nonutxo_state(&state, point, epoch, PREPROD_NETWORK_MAGIC).expect("decode");
    let r = &s.ratification;
    let sizes = r.differential(r);
    eprintln!("epoch {epoch}: frozen_at {:?} source {:?}", r.frozen_at(), r.source());
    eprintln!("  evidence: {} DReps, {} pools", sizes.drep.matching, sizes.pool.matching);
    match r.outcome() {
        RatificationOutcome::Decided(d) => eprintln!(
            "  Decided: enacted {:?}, expired {:?}",
            d.enacted.as_ref().map(|e| &e.action_id),
            d.expired
        ),
        RatificationOutcome::Unenactable(u) => eprintln!("  Unenactable: {u:?}"),
    }
    assert_eq!(r.frozen_at().0, epoch);
}
