//! SLICE LV-16 CE-LV16-8: compare two copies of an epoch accumulator store byte for byte. One is a
//! recovered run's store, the other a reference (the uninterrupted run's capture, or a second,
//! independent recovery). `#[ignore]`d: reads `ADE_LV16_ACC_A` and `ADE_LV16_ACC_B` (COPIES).
//!
//! - The frozen ratification record is fixed for the whole epoch, so it is compared whatever the cursor:
//!   B's record is put into A's governance state and the two canonical encodings must be identical.
//! - When both stores are at the same cursor, the whole canonical accumulator encoding must be identical.

use ade_ledger::epoch_accumulator::encode_epoch_accumulator;
use ade_ledger::snapshot::gov_state::encode_gov_state;
use ade_ledger::state::RatificationState;
use ade_runtime::chaindb::EpochAccumulatorStore;

fn load(var: &str) -> (u64, ade_ledger::epoch_accumulator::EpochAccumulator) {
    let path = std::env::var(var).unwrap_or_else(|_| panic!("set {var}"));
    let store = EpochAccumulatorStore::open(std::path::Path::new(&path)).expect("open store copy");
    let (slot, acc) = store.load_current().expect("load current").expect("store is sealed");
    (slot.0, acc)
}

#[test]
#[ignore = "reads ADE_LV16_ACC_A and ADE_LV16_ACC_B (store copies)"]
fn lv16_recovered_store_equals_reference() {
    let (slot_a, a) = load("ADE_LV16_ACC_A");
    let (slot_b, b) = load("ADE_LV16_ACC_B");
    eprintln!(
        "A applied through {slot_a} (epoch {}), B applied through {slot_b} (epoch {})",
        a.epoch_state.epoch.0, b.epoch_state.epoch.0
    );
    assert_eq!(a.epoch_state.epoch, b.epoch_state.epoch, "the stores are in different epochs");

    let gov_a = a.gov_state.as_ref().expect("A has a governance state");
    let gov_b = b.gov_state.as_ref().expect("B has a governance state");
    let (RatificationState::Recorded(ra), RatificationState::Recorded(rb)) =
        (&gov_a.ratification, &gov_b.ratification)
    else {
        panic!("both stores must hold a record");
    };
    let mut swapped = gov_a.clone();
    swapped.ratification = RatificationState::Recorded(rb.clone());
    let record_bytes_equal = encode_gov_state(&swapped) == encode_gov_state(gov_a);
    eprintln!("frozen_at A {:?} B {:?}; record encodings identical: {record_bytes_equal}", ra.frozen_at(), rb.frozen_at());
    assert!(record_bytes_equal, "the frozen records differ");

    if slot_a == slot_b {
        let (ea, eb) = (encode_epoch_accumulator(&a), encode_epoch_accumulator(&b));
        eprintln!("same cursor {slot_a}: accumulator encodings {} and {} bytes, identical: {}", ea.len(), eb.len(), ea == eb);
        assert_eq!(ea, eb, "same cursor, different accumulator bytes");
    } else {
        eprintln!("different cursors: whole-accumulator comparison not applicable");
    }
}
