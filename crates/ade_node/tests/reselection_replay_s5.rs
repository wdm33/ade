// PHASE4-N-AO S5 (CE-AO-5; DC-NODE-27 + DC-NODE-28). Reselection replay-equivalence
// + crash recovery + the "no fake winner" proof. A ForkChoiceWin reselection
// replays byte-identically via the EXISTING reason-agnostic replay_from_anchor; a
// crash with a RollBack{ForkChoiceWin} but without its bodies recovers the VALID
// PREFIX at the fork anchor (NOT the winner) and remains not-caught-up.

use std::collections::BTreeMap;

use ade_core::consensus::events::{BlockDistance, ChainEvent, Point};
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_core::consensus::era_schedule::EraSchedule;
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::receive::events::TipPoint;
use ade_ledger::receive::ReceiveState;
use ade_ledger::state::LedgerState;
use ade_ledger::wal::{
    replay_from_anchor, BlockVerdictTag, RollbackReason, WalEntry, WalError, WalStore,
};
use ade_node::node_lifecycle::apply_chain_event;
use ade_node::node_sync::{forge_followed_tip_admission, ForgeFollowedTipAdmission};
use ade_runtime::chaindb::{ChainDb, InMemoryChainDb, StoredBlock};
use ade_runtime::forward_sync::{ForwardSyncState, NoCheckpointSink};
use ade_runtime::rollback::{PersistentSnapshotCache, SnapshotCadence};
use ade_testkit::consensus::ledger_view_stub::LedgerViewStub;
use ade_types::{BlockNo, CardanoEra, EpochNo, Hash32, SlotNo};

// ---------- fixtures (mirror apply_driver_ai_s3.rs) ----------

fn h(b: u8) -> Hash32 {
    Hash32([b; 32])
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
        ReceiveState::new(LedgerState::new(CardanoEra::Conway), chain_dep(52)),
        h(0xAB),
        SnapshotCadence::DEFAULT,
    )
}
fn db_with_fork_and_snapshot() -> (InMemoryChainDb, LedgerState) {
    let db = InMemoryChainDb::new();
    db.put_block(&stored(100, 0xF0)).unwrap();
    db.put_block(&stored(101, 0xA1)).unwrap();
    db.put_block(&stored(102, 0xA2)).unwrap();
    let l0 = LedgerState::new(CardanoEra::Conway);
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
/// The original chain WAL: fork(100, post=fork_post) + abandoned 101 (0xAB) + 102 (0xAC).
fn append_original_chain(wal: &mut VecWal, anchor: &Hash32, fork_post: &Hash32) {
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
}

// ---------- 1. success-path replay-equivalence: recovers the SELECTED chain ----------

#[test]
fn forkchoicewin_reselection_replays_byte_identical() {
    let (db, l0) = db_with_fork_and_snapshot();
    let mut fwd = fresh_fwd();
    let mut wal = VecWal::default();
    let anchor = h(0xA0);
    let fork_post = fingerprint(&l0).combined;
    append_original_chain(&mut wal, &anchor, &fork_post);
    // ForkChoiceWin rollback to the fork (supersedes 101/102).
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
    .expect("apply");
    // Adopt the winner above the fork (the ChainSelected pump would append this).
    let winner_fp = h(0x71);
    wal.append(WalEntry::AdmitBlock {
        prior_fp: fork_post.clone(),
        block_hash: h(0xB1),
        slot: SlotNo(101),
        verdict: BlockVerdictTag::Valid,
        post_fp: winner_fp.clone(),
    })
    .unwrap();

    let entries = wal.read_all().unwrap();
    let mut bb = BTreeMap::new();
    // Effective blocks after the rollback: the fork + the winner (101/102 superseded).
    bb.insert(h(0xF0), vec![0xF0]);
    bb.insert(h(0xB1), vec![0xB1]);
    let out = replay_from_anchor(&anchor, &entries, &bb).expect("replay");
    assert_eq!(out.tail_fp, winner_fp, "replay recovers the SELECTED chain (winner)");
    assert_ne!(out.tail_fp, h(0xAC), "never the abandoned branch tip");
}

// ---------- 2. crash before commit: replay = original chain, no mutation ----------

#[test]
fn crash_before_commit_replays_no_mutation() {
    // No RollBack{FCW} was appended (crash before the rollback commit) -> replay
    // reproduces the ORIGINAL chain (tip = 102), byte-unchanged.
    let mut wal = VecWal::default();
    let anchor = h(0xA0);
    let fork_post = h(0x50);
    append_original_chain(&mut wal, &anchor, &fork_post);
    let entries = wal.read_all().unwrap();
    // No RollBack -> all three blocks are effective; each needs its bytes present.
    let mut bb: BTreeMap<Hash32, Vec<u8>> = BTreeMap::new();
    bb.insert(h(0xF0), vec![0xF0]);
    bb.insert(h(0xA1), vec![0xA1]);
    bb.insert(h(0xA2), vec![0xA2]);
    let out = replay_from_anchor(&anchor, &entries, &bb).expect("replay");
    assert_eq!(out.tail_fp, h(0xAC), "no RollBack -> the original chain is recovered");
}

// ---------- 3 + 4. crash after RollBack before bodies: valid prefix, NO fake winner ----------

#[test]
fn crash_after_rollback_before_bodies_recovers_valid_prefix_no_silent_forge() {
    let (db, l0) = db_with_fork_and_snapshot();
    let mut fwd = fresh_fwd();
    let mut wal = VecWal::default();
    let anchor = h(0xA0);
    let fork_post = fingerprint(&l0).combined;
    append_original_chain(&mut wal, &anchor, &fork_post);
    // ForkChoiceWin rollback recorded, but the winner's AdmitBlock NEVER appended
    // (crash before the body pump).
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
    .expect("apply");

    let entries = wal.read_all().unwrap();
    let mut bb = BTreeMap::new();
    bb.insert(h(0xF0), vec![0xF0]);
    let out = replay_from_anchor(&anchor, &entries, &bb).expect("replay");
    // Deterministically recovers the VALID PREFIX at the fork anchor.
    assert_eq!(out.tail_fp, fork_post, "recovers the valid prefix at fork_anchor");
    assert_ne!(out.tail_fp, h(0xAC), "not the abandoned branch");

    // The node is recovered AT the fork anchor (slot 100), BEHIND a peer following
    // the winner (slot 101) -> the DC-NODE-15 catch-up gate refuses forge.
    let recovered_tip = TipPoint {
        slot: SlotNo(100),
        hash: h(0xF0),
        block_no: 100,
    };
    let winner_peer_tip = TipPoint {
        slot: SlotNo(101),
        hash: h(0xB1),
        block_no: 101,
    };
    assert!(
        matches!(
            forge_followed_tip_admission(Some(recovered_tip), Some(winner_peer_tip)),
            ForgeFollowedTipAdmission::NotCaughtUp { .. }
        ),
        "recovered behind the winner -> not caught up -> no silent forge"
    );
}

#[test]
fn forkchoicewin_rollback_without_bodies_is_no_fake_winner() {
    // The core "no fake winner after crash" proof: a RollBack{ForkChoiceWin}
    // without its AdmitBlock bodies MUST NOT produce selector/current-tip agreement
    // with the selected winner. The recovered tip is the fork anchor, not the
    // winner, and the node remains not-caught-up.
    let (db, l0) = db_with_fork_and_snapshot();
    let mut fwd = fresh_fwd();
    let mut wal = VecWal::default();
    let anchor = h(0xA0);
    let fork_post = fingerprint(&l0).combined;
    append_original_chain(&mut wal, &anchor, &fork_post);
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
    .expect("apply");

    // The winner the (lost) decision intended to adopt.
    let winner_fp = h(0x71);
    let winner_tip = TipPoint {
        slot: SlotNo(101),
        hash: h(0xB1),
        block_no: 101,
    };

    let entries = wal.read_all().unwrap();
    let mut bb = BTreeMap::new();
    bb.insert(h(0xF0), vec![0xF0]);
    let out = replay_from_anchor(&anchor, &entries, &bb).expect("replay");

    // current-tip (durable) does NOT agree with the winner.
    assert_ne!(out.tail_fp, winner_fp, "the winner fp is NOT recovered (no fake winner)");
    assert_eq!(out.tail_fp, fork_post, "recovered tip is the fork anchor");
    // The durable ChainDb tip is the fork anchor, not the winner's point.
    let durable = db.tip().unwrap().unwrap();
    assert_eq!(durable.hash, h(0xF0), "durable tip = fork anchor");
    assert_ne!(durable.hash, winner_tip.hash, "durable tip != winner");
    // And it remains not-caught-up to a winner-following peer.
    let recovered_tip = TipPoint {
        slot: durable.slot,
        hash: durable.hash,
        block_no: 100,
    };
    assert!(matches!(
        forge_followed_tip_admission(Some(recovered_tip), Some(winner_tip)),
        ForgeFollowedTipAdmission::NotCaughtUp { .. }
    ));
}
