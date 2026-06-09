// PHASE4-N-AI AI-S4b-ii — live rollback-follow routing + forge gate. Proves:
// Participant RollBack(in-chain point) -> durable rollback via apply_chain_event;
// RollBack to unknown / beyond-k point -> fail closed; bare Competing block ->
// fail closed; a block with no durable tip -> pump_block (cold-start); SP/Unknown
// RollBack -> run_node_sync fails closed; the DC-NODE-28 forge-gate helper.

use std::collections::BTreeMap;

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_ledger::block_validity::decode_block;
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::receive::ReceiveState;
use ade_ledger::state::LedgerState;
use ade_ledger::wal::{WalEntry, WalError, WalStore};
use ade_network::codec::chain_sync::Point as WirePoint;
use ade_node::node_lifecycle::run_participant_sync;
use ade_node::node_sync::{
    pending_reselection_forge_refusal, run_node_sync, ForgeRefused, NodeBlockSource, NodeSyncError,
    NodeSyncItem,
};
use ade_runtime::chaindb::{ChainDb, InMemoryChainDb, StoredBlock};
use ade_runtime::forward_sync::ForwardSyncState;
use ade_runtime::rollback::{PersistentSnapshotCache, SnapshotCadence};
use ade_testkit::validity::ConwayValidityCorpus;
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

// ---------- shared helpers ----------

fn h(b: u8) -> Hash32 {
    Hash32([b; 32])
}

fn stored(slot: u64, hash: u8) -> StoredBlock {
    StoredBlock {
        hash: h(hash),
        slot: SlotNo(slot),
        bytes: vec![hash; 8],
    }
}

fn chain_dep(block_no: u64) -> PraosChainDepState {
    let mut s = PraosChainDepState::genesis(Nonce(h(0xCD)));
    s.last_block_no = Some(ade_types::BlockNo(block_no));
    s.last_slot = Some(SlotNo(block_no * 2));
    s
}

fn min_schedule() -> EraSchedule {
    EraSchedule::new(
        BootstrapAnchorHash(Hash32([0u8; 32])),
        0,
        vec![EraSummary {
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

fn view_stub() -> ade_testkit::consensus::ledger_view_stub::LedgerViewStub {
    ade_testkit::consensus::ledger_view_stub::LedgerViewStub::new()
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

fn fwd_at(block_no: u64) -> ForwardSyncState {
    ForwardSyncState::new(
        ReceiveState::new(LedgerState::new(CardanoEra::Conway), chain_dep(block_no)),
        h(0xAB),
        SnapshotCadence::DEFAULT,
    )
}

/// InMemoryChainDb with the fork block (slot 100, 0xF0) + abandoned 101/102, and
/// a snapshot AT slot 100 (degenerate materialize → returns the rolled-back state).
fn db_with_fork_and_snapshot() -> InMemoryChainDb {
    let db = InMemoryChainDb::new();
    db.put_block(&stored(100, 0xF0)).unwrap();
    db.put_block(&stored(101, 0xA1)).unwrap();
    db.put_block(&stored(102, 0xA2)).unwrap();
    PersistentSnapshotCache::new(&db)
        .capture(SlotNo(100), &LedgerState::new(CardanoEra::Conway), &chain_dep(50))
        .unwrap();
    db
}

fn rollback_item(slot: u64, hash: u8) -> NodeSyncItem {
    NodeSyncItem::RollBack(WirePoint::Block {
        slot: SlotNo(slot),
        hash: h(hash),
    })
}

// ---------- rollback path (Participant) ----------

#[tokio::test]
async fn participant_rollback_applies_durably() {
    let db = db_with_fork_and_snapshot();
    let mut fwd = fwd_at(52);
    let mut wal = VecWal::default();
    let mut pending = false;
    let mut src = NodeBlockSource::in_memory_items(vec![rollback_item(100, 0xF0)]);
    run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &min_schedule(),
        &view_stub(),
        &mut pending,
    )
    .await
    .expect("rollback applies");
    // Durable rollback to the fork (101/102 dropped; tip = 100/0xF0).
    let tip = db.tip().unwrap().unwrap();
    assert_eq!(tip.slot, SlotNo(100));
    assert_eq!(tip.hash, h(0xF0));
    // A RollBack WAL record was produced (via apply_chain_event / AI-S1).
    assert!(wal
        .read_all()
        .unwrap()
        .iter()
        .any(|e| matches!(e, WalEntry::RollBack { .. })));
    // Pending is cleared after the apply returns.
    assert!(!pending);
}

#[tokio::test]
async fn participant_rollback_to_unknown_point_fails_closed() {
    let db = db_with_fork_and_snapshot();
    let mut fwd = fwd_at(52);
    let mut wal = VecWal::default();
    let mut pending = false;
    // 0x99 is not a block in the durable chain.
    let mut src = NodeBlockSource::in_memory_items(vec![rollback_item(100, 0x99)]);
    let err = run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &min_schedule(),
        &view_stub(),
        &mut pending,
    )
    .await
    .expect_err("unknown rollback point must fail closed");
    assert!(matches!(err, NodeSyncError::UnexpectedRollback), "got {err:?}");
    // No apply: the durable tip is unchanged (still 102), no WAL RollBack.
    assert_eq!(db.tip().unwrap().unwrap().slot, SlotNo(102));
    assert!(!wal
        .read_all()
        .unwrap()
        .iter()
        .any(|e| matches!(e, WalEntry::RollBack { .. })));
}

#[tokio::test]
async fn participant_rollback_beyond_k_fails_closed_clears_pending() {
    // A block at slot 100 exists (verify passes) but NO snapshot -> materialize
    // RollbackTooDeep -> apply_chain_event fails -> pending cleared, fail closed.
    let db = InMemoryChainDb::new();
    db.put_block(&stored(100, 0xF0)).unwrap();
    let mut fwd = fwd_at(52);
    let mut wal = VecWal::default();
    let mut pending = false;
    let mut src = NodeBlockSource::in_memory_items(vec![rollback_item(100, 0xF0)]);
    let err = run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &min_schedule(),
        &view_stub(),
        &mut pending,
    )
    .await
    .expect_err("no snapshot -> fail closed");
    assert!(matches!(err, NodeSyncError::Pump(_)), "got {err:?}");
    // DC-NODE-28: pending was set during the apply attempt and cleared after it
    // returned (the failure path) -- a later forge tick sees pending == false.
    assert!(!pending);
}

#[tokio::test]
async fn rollback_slot_hash_mismatch_fails_before_mutation() {
    // AI-S6 (DC-NODE-29 / H-1): a peer names a real in-chain hash (0xF0, stored at
    // slot 100) but a DIFFERENT slot (99) -- mixed peer/local authority. Must fail
    // closed BEFORE any durable mutation. The 7 must-holds: typed error; no
    // commit_rollback; no WAL RollBack; ChainDb tip unchanged; ledger unchanged;
    // chain_dep unchanged; replay clean (the WAL is untouched -> not bricked).
    let db = db_with_fork_and_snapshot(); // tip 102/0xA2; 0xF0 stored at slot 100
    let tip_before = db.tip().unwrap().unwrap();
    let mut fwd = fwd_at(52);
    let ledger_fp_before = fingerprint(&fwd.receive.ledger).combined;
    let chain_dep_before = fwd.receive.chain_dep.clone();
    let prior_fp_before = fwd.prior_fp.clone();
    let mut wal = VecWal::default();
    let mut pending = false;
    // 0xF0 IS in the chain (at slot 100); the peer claims slot 99 -> mismatch.
    let mut src = NodeBlockSource::in_memory_items(vec![rollback_item(99, 0xF0)]);
    let err = run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &min_schedule(),
        &view_stub(),
        &mut pending,
    )
    .await
    .expect_err("a slot/hash-mismatched rollback target must fail closed");
    // (1) typed error binding the peer slot vs the stored slot.
    assert!(
        matches!(
            err,
            NodeSyncError::RollbackPointSlotMismatch { peer_slot, stored_slot, .. }
                if peer_slot == SlotNo(99) && stored_slot == SlotNo(100)
        ),
        "got {err:?}"
    );
    // (2)+(3) no commit_rollback / no WAL RollBack append -- the WAL is untouched.
    assert!(wal.read_all().unwrap().is_empty(), "no durable WAL mutation");
    // (4) ChainDb tip unchanged (no truncation of the durable chain).
    let tip_after = db.tip().unwrap().unwrap();
    assert_eq!(tip_after.slot, tip_before.slot);
    assert_eq!(tip_after.hash, tip_before.hash);
    // (5) ledger unchanged.
    assert_eq!(fingerprint(&fwd.receive.ledger).combined, ledger_fp_before);
    // (6) chain_dep unchanged (+ the WAL anchor fp).
    assert_eq!(fwd.receive.chain_dep, chain_dep_before);
    assert_eq!(fwd.prior_fp, prior_fp_before);
    // (7) pending never set (failed before the set) -> a forge tick is unblocked,
    //     and the untouched WAL replays clean (the node is not bricked).
    assert!(!pending);
}

// ---------- SP/Unknown reject rollback; the forge gate helper ----------

#[tokio::test]
async fn singleproducer_rollback_refused_by_run_node_sync() {
    let db = InMemoryChainDb::new();
    let mut fwd = fwd_at(52);
    let mut wal = VecWal::default();
    // run_node_sync is the SP/Unknown path -- a RollBack item fails closed.
    let mut src = NodeBlockSource::in_memory_items(vec![rollback_item(100, 0xF0)]);
    let err = run_node_sync(&mut src, &mut fwd, &db, &mut wal, &min_schedule(), &view_stub())
        .await
        .expect_err("SP/Unknown do not follow peer rollbacks");
    assert!(matches!(err, NodeSyncError::UnexpectedRollback), "got {err:?}");
}

#[test]
fn pending_reselection_forge_refusal_gate() {
    // DC-NODE-28: pending -> typed ForgeRefused::ReselectionPending; else none.
    assert!(matches!(
        pending_reselection_forge_refusal(true),
        Some(ForgeRefused::ReselectionPending)
    ));
    assert!(pending_reselection_forge_refusal(false).is_none());
}

// ---------- block path (detector live) — Conway corpus ----------

fn corpus_view() -> (ConwayValidityCorpus, PoolDistrView) {
    let c = ConwayValidityCorpus::load().expect("corpus");
    let total = c.pd_total_active_stake;
    let asc = ActiveSlotsCoeff {
        numer: c.asc.numer as u32,
        denom: c.asc.denom as u32,
    };
    let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
    for (pool_id, p) in &c.pools {
        let scale = total / p.sigma.denom;
        pools.insert(
            Hash28(*pool_id),
            PoolEntry {
                active_stake: p.sigma.numer * scale,
                vrf_keyhash: Hash32(p.vrf_keyhash),
            },
        );
    }
    (c, PoolDistrView::new(EpochNo(576), total, asc, pools))
}

fn pick_lightest(c: &ConwayValidityCorpus) -> Vec<u8> {
    use ade_codec::cbor::envelope::decode_block_envelope;
    let idx = (0..c.blocks.len())
        .min_by_key(|&i| {
            let env = decode_block_envelope(&c.blocks[i]).expect("env");
            env.block_end - env.block_start
        })
        .expect("non-empty");
    c.blocks[idx].clone()
}

fn corpus_schedule() -> EraSchedule {
    const EPOCH_577_START: u64 = 163_900_800;
    const MAINNET_EPOCH_LENGTH: u64 = 432_000;
    EraSchedule::new(
        BootstrapAnchorHash(Hash32([0u8; 32])),
        0,
        vec![EraSummary {
            era: CardanoEra::Conway,
            start_slot: SlotNo(EPOCH_577_START - MAINNET_EPOCH_LENGTH),
            start_epoch: EpochNo(576),
            slot_length_ms: 1_000,
            epoch_length_slots: MAINNET_EPOCH_LENGTH as u32,
            safe_zone_slots: MAINNET_EPOCH_LENGTH as u32,
        }],
    )
    .expect("schedule")
}

#[tokio::test]
async fn participant_bare_competing_block_fails_closed() {
    let (c, view) = corpus_view();
    let block = pick_lightest(&c);
    let decoded = decode_block(&block).expect("decode");
    // A durable tip whose hash is NOT the block's prev_hash -> the block is a
    // bare competing candidate (not a linear extension, not already-have).
    let db = InMemoryChainDb::new();
    db.put_block(&stored(decoded.header_input.slot.0.saturating_sub(1), 0xEE))
        .unwrap();
    let mut fwd = fwd_at(decoded.header_input.block_no.0.saturating_sub(1));
    let mut wal = VecWal::default();
    let mut pending = false;
    let mut src = NodeBlockSource::in_memory_items(vec![NodeSyncItem::Block(block)]);
    let err = run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &corpus_schedule(),
        &view,
        &mut pending,
    )
    .await
    .expect_err("a bare competing block has no fork point -> fail closed");
    assert!(matches!(err, NodeSyncError::UnexpectedRollback), "got {err:?}");
}

#[tokio::test]
async fn participant_block_with_no_durable_tip_pumps() {
    // No durable tip (cold-start) -> the block is admitted via pump_block (the
    // sole roll-forward admit), the existing behavior; proves the block path
    // reaches pump_block with a real validating block.
    let (c, view) = corpus_view();
    let block = pick_lightest(&c);
    let decoded = decode_block(&block).expect("decode");
    let db = InMemoryChainDb::new();
    let mut fwd = ForwardSyncState::new(
        ReceiveState::new(
            LedgerState::new(CardanoEra::Conway),
            {
                let mut s = PraosChainDepState::empty();
                s.epoch_nonce = Nonce(Hash32(c.epoch_nonce));
                s.evolving_nonce = Nonce(Hash32(c.epoch_nonce));
                s
            },
        ),
        fingerprint(&LedgerState::new(CardanoEra::Conway)).combined,
        SnapshotCadence::DEFAULT,
    );
    let mut wal = VecWal::default();
    let mut pending = false;
    let mut src = NodeBlockSource::in_memory_items(vec![NodeSyncItem::Block(block)]);
    run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &corpus_schedule(),
        &view,
        &mut pending,
    )
    .await
    .expect("cold-start block admits via pump_block");
    let tip = db.tip().unwrap().unwrap();
    assert_eq!(tip.hash, decoded.block_hash);
}
