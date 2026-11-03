//! LV-15b-0 CE-9: compare the ratification record Ade froze natively, read from a copy of a live store's
//! accumulator, with the record imported from a cardano-node LedgerDB `state` file of the same epoch.
//! `#[ignore]`d: reads `ADE_LV15B0_ACC` (a COPY of `epoch-accumulator.redb`) and `ADE_LV15B0_STATE` (a copy of
//! the reference `state` file). The reference is imported unchanged; nothing here adjusts either side.

use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::governance::frozen::{RatificationOutcome, RatificationSource};
use ade_ledger::ledgerdb_state::{decode_native_nonutxo_state, NativeNonUtxoError};
use ade_ledger::state::RatificationState;
use ade_runtime::chaindb::EpochAccumulatorStore;
use ade_types::{Hash32, SlotNo};

const PREPROD_NETWORK_MAGIC: u32 = 1;

#[test]
#[ignore = "reads ADE_LV15B0_ACC (store copy) and ADE_LV15B0_STATE (reference state copy)"]
fn lv15b0_native_record_equals_reference() {
    let acc_path = std::env::var("ADE_LV15B0_ACC").expect("set ADE_LV15B0_ACC");
    let state_path = std::env::var("ADE_LV15B0_STATE").expect("set ADE_LV15B0_STATE");

    let store = EpochAccumulatorStore::open(std::path::Path::new(&acc_path)).expect("open store copy");
    let (slot, acc) = store.load_current().expect("load current").expect("store is sealed");
    let gov = acc.gov_state.as_ref().expect("Conway accumulator has a governance state");
    let native = match &gov.ratification {
        RatificationState::Recorded(r) => r,
        RatificationState::Unversioned => panic!("store holds no record"),
    };

    let state = std::fs::read(&state_path).unwrap_or_else(|e| panic!("read {state_path}: {e}"));
    let point = SeedPoint { slot: SlotNo(0), block_hash: Hash32([0xab; 32]) };
    let epoch = match decode_native_nonutxo_state(&state, point.clone(), u64::MAX, PREPROD_NETWORK_MAGIC) {
        Err(NativeNonUtxoError::EpochMismatch { decoded_epoch, .. }) => decoded_epoch,
        other => panic!("epoch discovery: {:?}", other.err()),
    };
    let (s, _) = decode_native_nonutxo_state(&state, point, epoch, PREPROD_NETWORK_MAGIC).expect("decode");
    let reference = &s.ratification;

    let d = native.differential(reference);
    eprintln!(
        "store applied through {} (accumulator epoch {}); native frozen_at {:?} source {:?}; reference frozen_at {:?}",
        slot.0,
        acc.epoch_state.epoch.0,
        native.frozen_at(),
        native.source(),
        reference.frozen_at()
    );
    eprintln!("{d:?}");
    for (name, r) in [("native", native), ("reference", reference)] {
        match r.outcome() {
            RatificationOutcome::Decided(x) => eprintln!(
                "  {name}: Decided enacted {:?} expired {:?}",
                x.enacted.as_ref().map(|e| &e.action_id),
                x.expired
            ),
            RatificationOutcome::Unenactable(u) => eprintln!("  {name}: Unenactable {u:?}"),
        }
    }

    assert_eq!(native.source(), RatificationSource::Frozen, "the store's record must be Ade's own freeze");
    assert!(d.same_epoch, "frozen_at differs");
    assert!(d.same_proposals, "frozen proposal sets differ");
    assert!(d.same_outcome, "outcomes differ");
    for (name, x) in [("DRep", &d.drep), ("pool", &d.pool)] {
        assert_eq!(
            (x.differing, x.only_self, x.only_reference),
            (0, 0, 0),
            "{name} distribution differs: {x:?}"
        );
    }
}
