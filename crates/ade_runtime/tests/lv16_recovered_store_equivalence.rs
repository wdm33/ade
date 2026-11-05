//! SLICE LV-16 CE-LV16-8: compare two copies of an epoch accumulator store byte for byte. One is a
//! recovered run's store, the other a reference (the uninterrupted run's capture, or a second,
//! independent recovery). `#[ignore]`d: reads `ADE_LV16_ACC_A` and `ADE_LV16_ACC_B` (COPIES).
//!
//! - The frozen ratification record is fixed for the whole epoch, so it is compared whatever the cursor:
//!   B's record is put into A's governance state and the two canonical encodings must be identical.
//! - When both stores are at the same cursor, the whole canonical accumulator encoding must be identical.
//!
//! The other comparators here work on whole data-directory copies and bring both sides to the SAME
//! canonical cursor before comparing (blocks and WAL over the shared range; a reference accumulator or
//! checkpoint advanced by the production fold to the recovered cursor). Each one, after its real
//! comparison, re-runs on a copy with one authoritative field changed and must report it.

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

// ---------- whole data directories, surface by surface ----------

use ade_crypto::blake2b::blake2b_256;
use ade_ledger::wal::{WalEntry, WalStore};
use ade_runtime::chaindb::{ChainDb, PersistentChainDb, PersistentChainDbOptions, ReducedUtxoCheckpoint};
use ade_runtime::wal::FileWalStore;
use ade_types::{Hash32, SlotNo};

fn hx(h: &Hash32) -> String {
    h.0[..8].iter().map(|b| format!("{b:02x}")).collect()
}

fn digest<'a>(items: impl Iterator<Item = &'a [u8]>) -> Hash32 {
    let mut buf = Vec::new();
    for i in items {
        buf.extend_from_slice(i);
    }
    blake2b_256(&buf)
}

/// The admits on the selected chain after applying every recorded rollback:
/// (slot, block hash, ledger fingerprint after the block).
fn effective_admits(entries: &[WalEntry]) -> Vec<(u64, Hash32, Hash32)> {
    let mut out: Vec<(u64, Hash32, Hash32)> = Vec::new();
    for e in entries {
        match e {
            WalEntry::AdmitBlock { block_hash, slot, post_fp, .. } => {
                out.push((slot.0, block_hash.clone(), post_fp.clone()))
            }
            WalEntry::RollBack { to_point, .. } => {
                while out.last().is_some_and(|(s, _, _)| *s > to_point.slot.0) {
                    out.pop();
                }
            }
            _ => {}
        }
    }
    out
}

struct Dir {
    tip: (u64, Hash32),
    blocks: Vec<(u64, Hash32)>,
    admits: Vec<(u64, Hash32, Hash32)>,
    rollbacks: usize,
    wal: Vec<WalEntry>,
    wal_store: FileWalStore,
    acc_slot: u64,
    acc: ade_ledger::epoch_accumulator::EpochAccumulator,
    cp_slot: Option<u64>,
    cp_fp: Option<Hash32>,
}

fn open_dir(var: &str, from: u64) -> Dir {
    let dir = std::path::PathBuf::from(std::env::var(var).unwrap_or_else(|_| panic!("set {var}")));
    let db = PersistentChainDb::open(PersistentChainDbOptions::at(dir.join("chain.db"))).expect("chain.db copy");
    let tip = db.tip().expect("tip").expect("non-empty chain");
    let blocks = db
        .iter_from_slot(SlotNo(from))
        .expect("iter")
        .map(|b| b.expect("block"))
        .map(|b| (b.slot.0, b.hash))
        .collect();
    let wal_store = FileWalStore::open(dir.join("wal")).expect("wal copy");
    let wal = wal_store.read_all().expect("read wal");
    let rollbacks = wal.iter().filter(|e| matches!(e, WalEntry::RollBack { .. })).count();
    let store = EpochAccumulatorStore::open(&dir.join("epoch-accumulator.redb")).expect("accumulator copy");
    let (acc_slot, acc) = store.load_current().expect("load").expect("sealed");
    let cp = ReducedUtxoCheckpoint::open(&dir.join("reduced-checkpoint.redb")).expect("checkpoint copy");
    Dir {
        tip: (tip.slot.0, tip.hash),
        blocks,
        admits: effective_admits(&wal),
        rollbacks,
        wal,
        wal_store,
        acc_slot: acc_slot.0,
        acc,
        cp_slot: cp.last_advanced_slot().expect("cp slot").map(|s| s.0),
        cp_fp: cp.fingerprint().ok(),
    }
}

fn blocks_agree(a: &[(u64, Hash32)], b: &[(u64, Hash32)], to: u64, print: bool) -> bool {
    let ba: Vec<_> = a.iter().filter(|(s, _)| *s <= to).collect();
    let bb: Vec<_> = b.iter().filter(|(s, _)| *s <= to).collect();
    let dig = |v: &[&(u64, Hash32)]| {
        let enc: Vec<Vec<u8>> = v.iter().map(|(s, h)| [s.to_be_bytes().as_slice(), &h.0].concat()).collect();
        digest(enc.iter().map(|x| x.as_slice()))
    };
    let (da, db) = (dig(&ba), dig(&bb));
    if print {
        eprintln!("blocks: {} vs {} in range, digest A {} B {} equal {}", ba.len(), bb.len(), hx(&da), hx(&db), da == db);
    }
    da == db
}

fn admits_agree(a: &[(u64, Hash32, Hash32)], b: &[(u64, Hash32, Hash32)], from: u64, to: u64, print: bool) -> bool {
    let wa: Vec<_> = a.iter().filter(|(s, _, _)| *s >= from && *s <= to).collect();
    let wb: Vec<_> = b.iter().filter(|(s, _, _)| *s >= from && *s <= to).collect();
    let first_diff = wa.iter().zip(wb.iter()).position(|(x, y)| x != y);
    if print {
        eprintln!("wal: {} vs {} effective admits in range; first difference {:?}", wa.len(), wb.len(), first_diff);
        if let (Some(la), Some(lb)) = (wa.last(), wb.last()) {
            eprintln!("ledger fingerprint at the last shared admit: A {}@{} B {}@{}", hx(&la.2), la.0, hx(&lb.2), lb.0);
        }
    }
    wa.len() == wb.len() && first_diff.is_none()
}

/// CE-LV16-8 / CE-10 shape: two COPIES of data directories (`ADE_LV16_DIR_A`, `ADE_LV16_DIR_B`) compared
/// at the chain points both share, from `ADE_LV16_FROM_SLOT` up to the lower tip (or `ADE_LV16_TO_SLOT`).
/// Each surface prints its chain point and the hashes of both sides; a surface observed at different
/// points on the two sides is reported as not comparable, never as equal.
#[test]
#[ignore = "reads ADE_LV16_DIR_A and ADE_LV16_DIR_B (data directory copies)"]
fn lv16_recovered_data_dirs_agree_at_shared_points() {
    let from: u64 = std::env::var("ADE_LV16_FROM_SLOT").ok().map(|v| v.parse().unwrap()).unwrap_or(137_894_414);
    let a = open_dir("ADE_LV16_DIR_A", from);
    let b = open_dir("ADE_LV16_DIR_B", from);
    let cap: u64 = std::env::var("ADE_LV16_TO_SLOT").ok().map(|v| v.parse().unwrap()).unwrap_or(u64::MAX);
    let to = a.tip.0.min(b.tip.0).min(cap);
    let mut failures = Vec::new();

    eprintln!("tip: A {}/{} B {}/{}; shared range {from}..={to}", a.tip.0, hx(&a.tip.1), b.tip.0, hx(&b.tip.1));

    // Retained block identity, and the WAL's effective admits (block identity and ledger
    // fingerprint after every shared block), over the shared range.
    if !blocks_agree(&a.blocks, &b.blocks, to, true) {
        failures.push("retained blocks differ");
    }
    eprintln!("(raw rollbacks A {} B {})", a.rollbacks, b.rollbacks);
    if !admits_agree(&a.admits, &b.admits, from, to, true) {
        failures.push("effective WAL admits differ");
    }

    // Sensitivity: one changed block hash, and one changed ledger fingerprint, inside the shared range.
    let mid = |len: usize| len / 2;
    let mut mb = a.blocks.clone();
    let k = mb.iter().rposition(|(s, _)| *s <= to).map(|i| mid(i + 1)).expect("a shared block");
    mb[k].1 .0[0] ^= 1;
    let mut ma = a.admits.clone();
    let k2 = ma.iter().rposition(|(s, _, _)| *s <= to).expect("a shared admit");
    let k2 = ma.iter().position(|(s, _, _)| *s >= from).map(|i| (i + k2) / 2).expect("an admit in range");
    ma[k2].2 .0[0] ^= 1;
    let block_detected = !blocks_agree(&mb, &b.blocks, to, false);
    let admit_detected = !admits_agree(&ma, &b.admits, from, to, false);
    eprintln!(
        "mutation block hash at {}: detected {block_detected}; mutation ledger fingerprint at {}: detected {admit_detected}",
        mb[k].0, ma[k2].0
    );
    assert!(block_detected && admit_detected, "a comparator missed a deliberate change");

    // Each side's recorded rollbacks, the WAL's own fingerprint-chain check, and what lies past the shared range.
    for (name, d) in [("A", &a), ("B", &b)] {
        for e in &d.wal {
            if let WalEntry::RollBack { to_point, reason, prior_tip, selected_tip } = e {
                eprintln!(
                    "rollback {name}: to {}/{} prior {}/{} selected {}/{} reason {reason:?}",
                    to_point.slot.0, hx(&to_point.hash), prior_tip.slot.0, hx(&prior_tip.hash),
                    selected_tip.slot.0, hx(&selected_tip.hash)
                );
            }
        }
        let anchor = d.wal.iter().find_map(|e| match e {
            WalEntry::AdmitBlock { prior_fp, .. } => Some(prior_fp.clone()),
            _ => None,
        });
        if let Some(anchor) = anchor {
            let verified = d.wal_store.verify_chain(&anchor);
            eprintln!("wal {name}: fingerprint chain from {} verifies: {verified:?}", hx(&anchor));
            if verified.is_err() {
                failures.push("a WAL fingerprint chain does not verify");
            }
        }
        let past: Vec<_> = d.admits.iter().filter(|(s, _, _)| *s > to).collect();
        if let (Some(f), Some(l)) = (past.first(), past.last()) {
            eprintln!(
                "past the shared range {name}: {} admits, first {}/{} last {}/{} fp {}",
                past.len(), f.0, hx(&f.1), l.0, hx(&l.1), hx(&l.2)
            );
        }
    }

    // Accumulator: record bytes always; whole encoding only at the same cursor.
    let ga = a.acc.gov_state.as_ref().expect("A gov");
    let gb = b.acc.gov_state.as_ref().expect("B gov");
    if a.acc.epoch_state.epoch == b.acc.epoch_state.epoch {
        if let (RatificationState::Recorded(_), RatificationState::Recorded(rb)) = (&ga.ratification, &gb.ratification) {
            let mut swapped = ga.clone();
            swapped.ratification = RatificationState::Recorded(rb.clone());
            let eq = encode_gov_state(&swapped) == encode_gov_state(ga);
            eprintln!("record (epoch {}): encodings identical {eq}", a.acc.epoch_state.epoch.0);
            if !eq {
                failures.push("frozen records differ");
            }
        }
    }
    let (ea, eb) = (encode_epoch_accumulator(&a.acc), encode_epoch_accumulator(&b.acc));
    let (gha, ghb) = (blake2b_256(&encode_gov_state(ga)), blake2b_256(&encode_gov_state(gb)));
    if a.acc_slot == b.acc_slot {
        eprintln!(
            "accumulator at {}: {} B hash A {} B {} equal {}; governance A {} B {}",
            a.acc_slot, ea.len(), hx(&blake2b_256(&ea)), hx(&blake2b_256(&eb)), ea == eb, hx(&gha), hx(&ghb)
        );
        if ea != eb {
            failures.push("accumulator bytes differ at the same cursor");
        }
    } else {
        eprintln!(
            "accumulator: cursors differ (A {} B {}): hashes A {} B {}, governance A {} B {} -- not comparable",
            a.acc_slot, b.acc_slot, hx(&blake2b_256(&ea)), hx(&blake2b_256(&eb)), hx(&gha), hx(&ghb)
        );
    }

    // Reduced checkpoint.
    match (a.cp_slot, b.cp_slot, &a.cp_fp, &b.cp_fp) {
        (Some(sa), Some(sb), Some(fa), Some(fb)) if sa == sb => {
            eprintln!("checkpoint at {sa}: A {} B {} equal {}", hx(fa), hx(fb), fa == fb);
            if fa != fb {
                failures.push("checkpoint fingerprints differ at the same cursor");
            }
        }
        _ => eprintln!("checkpoint: A {:?} B {:?} -- not comparable", a.cp_slot, b.cp_slot),
    }

    assert!(failures.is_empty(), "{failures:?}");
}

/// LV-17 evidence: the accumulator store's lineage anchor, cursor and settled rewind point
/// (`ADE_LV16_ACC_A`, a COPY of an `epoch-accumulator.redb`).
#[test]
#[ignore = "reads ADE_LV16_ACC_A (store copy)"]
fn lv17_report_anchor_and_settled_point() {
    let path = std::env::var("ADE_LV16_ACC_A").expect("set ADE_LV16_ACC_A");
    let store = EpochAccumulatorStore::open(std::path::Path::new(&path)).expect("open store copy");
    let anchor = store.last_advanced_point().expect("anchor");
    let settled = store.settled_rewind_point().expect("settled");
    let pt = |p: Option<(u64, u64, String)>| p.map_or_else(|| "absent".to_string(), |(s, b, h)| format!("{s}/{b}/{h}"));
    eprintln!(
        "cursor {:?}; anchor {}; settled {}",
        store.last_advanced_slot().expect("cursor").map(|s| s.0),
        pt(anchor.map(|p| (p.slot.0, p.block_no.0, hx(&p.header_hash)))),
        pt(settled.map(|p| (p.slot.0, p.block_no.0, hx(&p.header_hash)))),
    );
}

/// CE-LV16-8 at the SAME cursor: a recovered run's boundary capture (`ADE_LV16_ACC_A`) sits a few
/// blocks past the boundary cursor of the reference capture (`ADE_LV16_ACC_REF`). The reference copy is
/// folded forward to A's cursor by the production within-epoch fold (`advance_accumulator_over_chaindb`)
/// over the canonical blocks of `ADE_LV16_DIR/chain.db`, resolving collateral through that directory's
/// reduced checkpoint, and the two are then compared surface by surface. All three paths are COPIES.
#[test]
#[ignore = "reads ADE_LV16_ACC_REF, ADE_LV16_ACC_A and ADE_LV16_DIR (copies)"]
fn lv16_reference_folded_to_the_recovered_cursor_is_identical() {
    use ade_core::consensus::era_schedule::EraSchedule;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
    use ade_runtime::chaindb::{advance_accumulator_over_chaindb, AccumulatorChaindbOutcome};
    use ade_types::{CardanoEra, EpochNo};

    let (slot_a, a) = load("ADE_LV16_ACC_A");
    let ref_path = std::env::var("ADE_LV16_ACC_REF").expect("set ADE_LV16_ACC_REF");
    let ref_store = EpochAccumulatorStore::open(std::path::Path::new(&ref_path)).expect("open reference copy");
    let (slot_r0, r0) = ref_store.load_current().expect("load").expect("sealed");
    assert_eq!(r0.epoch_state.epoch, a.epoch_state.epoch, "reference and recovered capture are in different epochs");
    assert!(slot_r0.0 <= slot_a, "the reference is already past the recovered cursor");

    let dir = std::path::PathBuf::from(std::env::var("ADE_LV16_DIR").expect("set ADE_LV16_DIR"));
    let db = PersistentChainDb::open(PersistentChainDbOptions::at(dir.join("chain.db"))).expect("chain.db copy");
    let cp = ReducedUtxoCheckpoint::open(&dir.join("reduced-checkpoint.redb")).expect("checkpoint copy");
    // Preprod geometry: epoch 4 starts at slot 86,400; 432,000-slot epochs.
    let schedule = EraSchedule::new(
        BootstrapAnchorHash(Hash32([0u8; 32])),
        0,
        vec![EraSummary {
            randomness_stabilisation_window_slots: None,
            era: CardanoEra::Conway,
            start_slot: SlotNo(86_400),
            start_epoch: EpochNo(4),
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 432_000,
        }],
    )
    .expect("schedule");
    let outcome = advance_accumulator_over_chaindb(
        &ref_store,
        &db,
        &schedule,
        SlotNo(0),
        SlotNo(slot_a),
        Some(&cp as &dyn ade_ledger::collateral::CollateralValueResolver),
    )
    .expect("fold");
    let (slot_r, r) = ref_store.load_current().expect("load").expect("sealed");
    eprintln!("reference folded {} -> {}: {outcome:?}; recovered capture at {slot_a}", slot_r0.0, slot_r.0);
    assert!(
        matches!(outcome, AccumulatorChaindbOutcome::ReachedTip { .. }) && slot_r.0 == slot_a,
        "the fold did not reach the recovered cursor"
    );

    let differ = accumulator_surface_diffs(&r, &a, Some(slot_a));
    assert!(differ.is_empty(), "differ at the same cursor: {differ:?}");

    // Sensitivity: the same comparator must name the surface when one authoritative field of the
    // recovered state is changed by one unit.
    let mut m = a.clone();
    let (cred, coin) = m.cert_state.delegation.rewards.iter().next().map(|(c, v)| (c.clone(), *v)).expect("a reward account");
    m.cert_state.delegation.rewards.insert(cred, ade_types::tx::Coin(coin.0 + 1));
    let mut m2 = a.clone();
    m2.epoch_state.reserves = ade_types::tx::Coin(m2.epoch_state.reserves.0 + 1);
    let mut m3 = a.clone();
    m3.gov_state.as_mut().expect("gov").gov_action_lifetime += 1;
    for (field, mutated, surface) in [
        ("one reward balance +1 lovelace", &m, "delegation (cert state)"),
        ("reserves +1 lovelace", &m2, "epoch state"),
        ("governance action lifetime +1", &m3, "governance"),
    ] {
        let d = accumulator_surface_diffs(&r, mutated, None);
        eprintln!("mutation {field}: comparator reports {d:?}");
        assert!(d.contains(&surface) && d.contains(&"whole accumulator"), "mutation {field} not detected");
    }
}

/// The canonical encodings compared surface by surface; returns the surfaces that differ.
fn accumulator_surface_diffs(
    r: &ade_ledger::epoch_accumulator::EpochAccumulator,
    a: &ade_ledger::epoch_accumulator::EpochAccumulator,
    print_at: Option<u64>,
) -> Vec<&'static str> {
    use ade_ledger::snapshot::cert_state::encode_cert_state;
    use ade_ledger::snapshot::epoch_state::encode_epoch_state;
    let h = |b: &[u8]| hx(&blake2b_256(b));
    let surfaces: [(&'static str, Vec<u8>, Vec<u8>); 4] = [
        ("whole accumulator", encode_epoch_accumulator(r), encode_epoch_accumulator(a)),
        ("epoch state", encode_epoch_state(&r.epoch_state), encode_epoch_state(&a.epoch_state)),
        ("delegation (cert state)", encode_cert_state(&r.cert_state), encode_cert_state(&a.cert_state)),
        (
            "governance",
            encode_gov_state(r.gov_state.as_ref().expect("reference gov")),
            encode_gov_state(a.gov_state.as_ref().expect("recovered gov")),
        ),
    ];
    let mut differ = Vec::new();
    for (name, x, y) in &surfaces {
        if let Some(at) = print_at {
            eprintln!("{name} at {at}: {} bytes, reference {} recovered {} identical {}", x.len(), h(x), h(y), x == y);
        }
        if x != y {
            differ.push(*name);
        }
    }
    differ
}

/// CE-LV16-8, reduced checkpoint at the SAME cursor: the recovered checkpoint copy (`ADE_LV16_CP_A`)
/// is advanced by the production advance (`advance_reduced_checkpoint_over_chaindb`) over the
/// canonical blocks of `ADE_LV16_DIR/chain.db` to the reference copy's cursor (`ADE_LV16_CP_REF`);
/// both fingerprints are then recomputed (`finalize`) and compared with the record counts.
/// The fingerprint covers the reduced UTxO records only. The BND-2d retained-collateral table is not
/// in it, so this test makes no claim about that table's equality.
#[test]
#[ignore = "reads ADE_LV16_CP_A, ADE_LV16_CP_REF and ADE_LV16_DIR (copies)"]
fn lv16_recovered_checkpoint_advanced_to_the_reference_cursor_is_identical() {
    use ade_runtime::chaindb::advance_reduced_checkpoint_over_chaindb;
    use ade_types::CardanoEra;

    let open = |var: &str| {
        let p = std::env::var(var).unwrap_or_else(|_| panic!("set {var}"));
        ReducedUtxoCheckpoint::open(std::path::Path::new(&p)).expect("checkpoint copy")
    };
    let (a, r) = (open("ADE_LV16_CP_A"), open("ADE_LV16_CP_REF"));
    let slot_a0 = a.last_advanced_slot().expect("cursor").expect("advanced").0;
    let slot_r = r.last_advanced_slot().expect("cursor").expect("advanced").0;
    assert!(slot_a0 <= slot_r, "the recovered checkpoint is already past the reference cursor");
    let dir = std::path::PathBuf::from(std::env::var("ADE_LV16_DIR").expect("set ADE_LV16_DIR"));
    let db = PersistentChainDb::open(PersistentChainDbOptions::at(dir.join("chain.db"))).expect("chain.db copy");
    advance_reduced_checkpoint_over_chaindb(&a, &db, SlotNo(0), SlotNo(slot_r), CardanoEra::Conway).expect("advance");
    let slot_a = a.last_advanced_slot().expect("cursor").expect("advanced").0;
    let (fa, fr) = (a.finalize().expect("finalize A"), r.finalize().expect("finalize reference"));
    let (na, nr) = (a.len().expect("len"), r.len().expect("len"));
    eprintln!(
        "checkpoint: recovered {slot_a0} -> {slot_a}, reference {slot_r}; fingerprint recovered {} ({na} records) reference {} ({nr} records) identical {}",
        hx(&fa), hx(&fr), fa == fr && na == nr
    );
    assert_eq!(slot_a, slot_r, "the advance did not reach the reference cursor");
    assert_eq!((&fa, na), (&fr, nr), "checkpoints differ at the same cursor");

    // Sensitivity (this mutates the recovered COPY): re-write one output of the block at the cursor
    // with its coin +1 lovelace; the recomputed fingerprint must differ with the record count unchanged.
    let stored = db.iter_from_slot(SlotNo(slot_r)).expect("iter").next().expect("block at cursor").expect("block");
    assert_eq!(stored.slot.0, slot_r);
    let env = ade_codec::cbor::envelope::decode_block_envelope(&stored.bytes).expect("envelope");
    let block = ade_codec::conway::decode_conway_block(&stored.bytes[env.block_start..env.block_end]).expect("conway");
    let delta = ade_ledger::reduced_advance::reduced_block_delta(block.decoded(), CardanoEra::Conway).expect("delta");
    let (txin, coin, stake) = delta.produced.first().cloned().expect("an output produced at the cursor");
    assert!(a.get(&txin).expect("get").is_some(), "the cursor block's output is in the checkpoint");
    a.advance_block(SlotNo(slot_r), &[], &[(txin, ade_types::tx::Coin(coin.0 + 1), stake)], &[]).expect("mutate");
    let (fm, nm) = (a.finalize().expect("finalize mutated"), a.len().expect("len"));
    eprintln!("mutation one output's coin +1 lovelace: fingerprint {} ({nm} records), differs {}", hx(&fm), fm != fr);
    assert!(fm != fr && nm == nr, "the checkpoint comparison did not detect a changed coin");
}
