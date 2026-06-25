// PHASE4-N-AI AI-S3 — live fork-choice apply driver (DC-NODE-25 + DC-NODE-26;
// CE-AI-1 production half). Proves: RolledBack -> durable rollback via
// commit_rollback + WalEntry::RollBack appended AFTER commit + prior_fp
// re-anchor + reconcile; replay-equivalence (the produced RollBack recovers
// the fork point, never the abandoned branch); fail-closed on no snapshot
// (no WAL); reconciliation mismatch fail-fast; Rejected no-op; ChainSelected
// routes through pump_block (invalid body -> no tip advance); missing block.

use std::collections::BTreeMap;

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::events::{BlockDistance, ChainEvent, ChainSelectionReject, Point, SecurityParam};
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::receive::ReceiveState;
use ade_ledger::state::LedgerState;
use ade_ledger::wal::{
    replay_from_anchor, BlockVerdictTag, RollbackReason, WalEntry, WalError, WalStore,
};
use ade_node::node_lifecycle::{apply_chain_event, AppliedTip, ApplyError};
use ade_runtime::chaindb::{ChainDb, InMemoryChainDb, StoredBlock};
use ade_runtime::forward_sync::{ForwardSyncState, NoCheckpointSink};
use ade_runtime::rollback::{PersistentSnapshotCache, SnapshotCadence};
use ade_testkit::consensus::ledger_view_stub::LedgerViewStub;
use ade_types::{BlockNo, CardanoEra, EpochNo, Hash32, SlotNo};

// ---------- fixtures ----------

fn h(b: u8) -> Hash32 {
    Hash32([b; 32])
}

fn rolled_back_ledger() -> LedgerState {
    LedgerState::new(CardanoEra::Conway)
}

fn chain_dep(block_no: u64) -> PraosChainDepState {
    let mut s = PraosChainDepState::genesis(Nonce(h(0xCD)));
    s.last_block_no = Some(BlockNo(block_no));
    s.last_slot = Some(SlotNo(block_no * 2));
    s
}

fn stored(slot: u64, hash: u8) -> StoredBlock {
    StoredBlock {
        hash: h(hash),
        slot: SlotNo(slot),
        bytes: vec![hash; 8],
    }
}

fn schedule() -> EraSchedule {
    EraSchedule::new(
        BootstrapAnchorHash(Hash32([0u8; 32])),
        0,
        vec![EraSummary {
            randomness_stabilisation_window_slots: None,
            era: CardanoEra::Conway,
            start_slot: SlotNo(0),
            start_epoch: EpochNo(0),
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 129_600,
        }],
    )
    .expect("schedule")
}

fn view() -> LedgerViewStub {
    LedgerViewStub::new()
}

#[derive(Default)]
struct VecWal {
    entries: Vec<WalEntry>,
}
impl WalStore for VecWal {
    fn append(&mut self, entry: WalEntry) -> Result<(), WalError> {
        self.entries.push(entry);
        Ok(())
    }
    fn read_all(&self) -> Result<Vec<WalEntry>, WalError> {
        Ok(self.entries.clone())
    }
}

fn fresh_fwd() -> ForwardSyncState {
    ForwardSyncState::new(
        ReceiveState::new(rolled_back_ledger(), chain_dep(52)),
        h(0xAB),
        SnapshotCadence::DEFAULT,
    )
}

/// A ChainDb with the fork block (slot 100, 0xF0) + abandoned 101/102, and a
/// snapshot AT slot 100 (so materialize takes the degenerate snapshot-at-target
/// path and returns the rolled-back state directly).
fn db_with_fork_and_snapshot() -> (InMemoryChainDb, LedgerState) {
    let db = InMemoryChainDb::new();
    db.put_block(&stored(100, 0xF0)).unwrap();
    db.put_block(&stored(101, 0xA1)).unwrap();
    db.put_block(&stored(102, 0xA2)).unwrap();
    let l0 = rolled_back_ledger();
    PersistentSnapshotCache::new(&db)
        .capture(SlotNo(100), &l0, &chain_dep(50))
        .unwrap();
    (db, l0)
}

fn rolled_back_event() -> ChainEvent {
    ChainEvent::RolledBack {
        to_point: Point {
            slot: SlotNo(100),
            hash: h(0xF0),
        },
        depth: BlockDistance(2),
    }
}

// ---------- 1. rollback applied; WAL record appended AFTER commit ----------

#[test]
fn apply_rolledback_rolls_back_and_appends_wal_record_after_commit() {
    let (db, l0) = db_with_fork_and_snapshot();
    let mut fwd = fresh_fwd();
    let mut wal = VecWal::default();
    let applied = apply_chain_event(
        &mut fwd,
        &db,
        &mut wal,
        &NoCheckpointSink,
        &rolled_back_event(),
        RollbackReason::ForkChoiceWin,
        None,
        &schedule(),
        &view(),
    )
    .expect("apply ok");

    assert_eq!(
        applied,
        Some(AppliedTip {
            slot: SlotNo(100),
            hash: h(0xF0)
        })
    );
    // ChainDb rolled back to the fork (101/102 dropped; tip = 100/0xF0).
    let tip = db.tip().unwrap().unwrap();
    assert_eq!(tip.slot, SlotNo(100));
    assert_eq!(tip.hash, h(0xF0));
    // Live ReceiveState ledger == the materialized rolled-back ledger;
    // prior_fp re-anchored to it.
    let fp_l0 = fingerprint(&l0).combined;
    assert_eq!(fingerprint(&fwd.receive.ledger).combined, fp_l0);
    assert_eq!(fwd.prior_fp, fp_l0);
    // Exactly one RollBack record appended.
    let entries = wal.read_all().unwrap();
    assert_eq!(entries.len(), 1);
    assert!(matches!(entries[0], WalEntry::RollBack { .. }));
}

// ---------- 2. replay-equivalence: recovers the fork point, not the abandoned branch ----------

#[test]
fn apply_rolledback_replays_byte_identical_recovers_forkpoint() {
    let (db, l0) = db_with_fork_and_snapshot();
    let mut fwd = fresh_fwd();
    let mut wal = VecWal::default();

    let anchor = h(0xA0);
    let fork_post = fingerprint(&l0).combined; // the rolled-back state's fp
    // Pre-populate the WAL: fork AdmitBlock (post = fork_post) then the
    // abandoned branch (101, 102).
    wal.append(WalEntry::AdmitBlock {
        prior_fp: anchor.clone(),
        block_hash: h(0xF0),
        slot: SlotNo(100),
        verdict: BlockVerdictTag::Valid,
        post_fp: fork_post.clone(),
    })
    .unwrap();
    wal.append(WalEntry::AdmitBlock {
        prior_fp: fork_post.clone(),
        block_hash: h(0xA1),
        slot: SlotNo(101),
        verdict: BlockVerdictTag::Valid,
        post_fp: h(0xAB),
    })
    .unwrap();
    wal.append(WalEntry::AdmitBlock {
        prior_fp: h(0xAB),
        block_hash: h(0xA2),
        slot: SlotNo(102),
        verdict: BlockVerdictTag::Valid,
        post_fp: h(0xAC),
    })
    .unwrap();

    // Apply the rollback -> appends the RollBack record.
    apply_chain_event(
        &mut fwd,
        &db,
        &mut wal,
        &NoCheckpointSink,
        &rolled_back_event(),
        RollbackReason::ForkChoiceWin,
        None,
        &schedule(),
        &view(),
    )
    .expect("apply ok");

    // Replay the produced WAL: abandoned 101/102 superseded (bytes not
    // required), re-anchor to the fork's post_fp.
    let entries = wal.read_all().unwrap();
    let mut bb: BTreeMap<Hash32, Vec<u8>> = BTreeMap::new();
    bb.insert(h(0xF0), vec![0xF0]); // only the effective fork block's bytes
    let out = replay_from_anchor(&anchor, &entries, &bb).expect("replay ok");
    assert_eq!(out.tail_fp, fork_post, "recovers the fork point");
    assert_ne!(out.tail_fp, h(0xAC), "never the abandoned branch tip");
}

// ---------- 3. no snapshot -> fail closed, no WAL ----------

#[test]
fn apply_rollback_no_snapshot_fails_closed_appends_no_wal() {
    let db = InMemoryChainDb::new();
    db.put_block(&stored(100, 0xF0)).unwrap(); // a block, but NO snapshot
    let mut fwd = fresh_fwd();
    let mut wal = VecWal::default();
    let err = apply_chain_event(
        &mut fwd,
        &db,
        &mut wal,
        &NoCheckpointSink,
        &rolled_back_event(),
        RollbackReason::ForkChoiceWin,
        None,
        &schedule(),
        &view(),
    )
    .expect_err("must fail closed");
    assert!(matches!(err, ApplyError::Materialize(_)), "got {err:?}");
    // No WAL appended (the failure is before the WAL step); ChainDb unchanged.
    assert!(wal.read_all().unwrap().is_empty());
    assert!(db.get_block_by_slot(SlotNo(100)).unwrap().is_some());
}

// ---------- 4. reconciliation mismatch -> fail-fast ----------

#[test]
fn apply_reconciliation_mismatch_fails_fast() {
    let (db, _l0) = db_with_fork_and_snapshot();
    let mut fwd = fresh_fwd();
    let mut wal = VecWal::default();
    // to_point hash 0x99 does NOT match the ChainDb block at slot 100 (0xF0);
    // materialize (by slot) + rollback succeed, but reconcile fails.
    let event = ChainEvent::RolledBack {
        to_point: Point {
            slot: SlotNo(100),
            hash: h(0x99),
        },
        depth: BlockDistance(2),
    };
    let err = apply_chain_event(
        &mut fwd,
        &db,
        &mut wal,
        &NoCheckpointSink,
        &event,
        RollbackReason::ForkChoiceWin,
        None,
        &schedule(),
        &view(),
    )
    .expect_err("reconcile mismatch");
    assert!(
        matches!(err, ApplyError::ReconciliationMismatch { .. }),
        "got {err:?}"
    );
}

// ---------- 5. Rejected -> no durable change ----------

#[test]
fn apply_rejected_makes_no_durable_change() {
    let db = InMemoryChainDb::new();
    db.put_block(&stored(100, 0xF0)).unwrap();
    let mut fwd = fresh_fwd();
    let mut wal = VecWal::default();
    let event = ChainEvent::Rejected {
        reason: ChainSelectionReject::ExceededRollback {
            requested: BlockDistance(1),
            max: SecurityParam(2160),
        },
    };
    let applied = apply_chain_event(
        &mut fwd,
        &db,
        &mut wal,
        &NoCheckpointSink,
        &event,
        RollbackReason::ForkChoiceWin,
        None,
        &schedule(),
        &view(),
    )
    .expect("ok");
    assert_eq!(applied, None);
    assert!(wal.read_all().unwrap().is_empty());
    assert_eq!(db.tip().unwrap().unwrap().slot, SlotNo(100));
}

// ---------- 6. ChainSelected routes through pump_block; invalid body -> no advance ----------

#[test]
fn apply_chain_selected_invalid_body_fails_via_pump_no_advance() {
    let db = InMemoryChainDb::new();
    db.put_block(&stored(100, 0xF0)).unwrap();
    let mut fwd = fresh_fwd();
    let mut wal = VecWal::default();
    let event = ChainEvent::ChainSelected {
        new_tip: Point {
            slot: SlotNo(101),
            hash: h(0xB1),
        },
        replaced_tip: None,
    };
    let invalid = vec![0xFFu8; 4]; // not a decodable block
    let err = apply_chain_event(
        &mut fwd,
        &db,
        &mut wal,
        &NoCheckpointSink,
        &event,
        RollbackReason::ForkChoiceWin,
        Some(&invalid),
        &schedule(),
        &view(),
    )
    .expect_err("pump must reject");
    assert!(matches!(err, ApplyError::Pump(_)), "got {err:?}");
    // No tip advance.
    assert_eq!(db.tip().unwrap().unwrap().slot, SlotNo(100));
}

#[test]
fn apply_chain_selected_without_block_bytes_fails_closed() {
    let db = InMemoryChainDb::new();
    db.put_block(&stored(100, 0xF0)).unwrap();
    let mut fwd = fresh_fwd();
    let mut wal = VecWal::default();
    let event = ChainEvent::ChainSelected {
        new_tip: Point {
            slot: SlotNo(101),
            hash: h(0xB1),
        },
        replaced_tip: None,
    };
    let err = apply_chain_event(
        &mut fwd,
        &db,
        &mut wal,
        &NoCheckpointSink,
        &event,
        RollbackReason::ForkChoiceWin,
        None, // no roll-forward block
        &schedule(),
        &view(),
    )
    .expect_err("must fail closed");
    assert!(matches!(err, ApplyError::MissingRollForwardBlock), "got {err:?}");
}
