// PHASE4-N-AI AI-S4b-ii — live rollback-follow routing + forge gate. Proves:
// Participant RollBack(in-chain point) -> durable rollback via apply_chain_event;
// RollBack to unknown / beyond-k point -> fail closed; bare Competing block ->
// fail closed; a block with no durable tip -> pump_block (cold-start); SP/Unknown
// RollBack -> run_node_sync fails closed; the DC-NODE-28 forge-gate helper.

use std::collections::BTreeMap;

use ade_core::consensus::candidate::{CandidateFragment, TiebreakerView};
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::events::{BlockDistance, Point, SecurityParam};
use ade_core::consensus::header_summary::{HeaderVrf, ValidatedHeaderSummary};
use ade_core::consensus::praos_leader_value;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_ledger::block_validity::decode_block;
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::receive::ReceiveState;
use ade_ledger::state::LedgerState;
use ade_ledger::wal::{RollbackReason, WalEntry, WalError, WalStore};
use ade_network::codec::chain_sync::Point as WirePoint;
use ade_node::fork_switch::{
    fork_switch_fence_resolved, BranchBodySource, BranchProofError, FetchError, ForkSwitchOutcome,
    PrefetchedBranchBodies,
};
use ade_node::node_lifecycle::{apply_fork_switch, run_participant_sync};
use ade_node::selector_state::{project_tiebreaker, ForkAnchor, PendingForkSwitch};
use ade_node::node_sync::{
    pending_reselection_forge_refusal, run_node_sync, ForgeRefused, NodeBlockSource, NodeSyncError,
    NodeSyncItem,
};
use ade_runtime::chaindb::{ChainDb, ChainTip, InMemoryChainDb, StoredBlock};
use ade_types::shelley::block::PrevHash;
use ade_runtime::forward_sync::ForwardSyncState;
use ade_runtime::rollback::{PersistentSnapshotCache, SnapshotCadence};
use ade_testkit::validity::ConwayValidityCorpus;
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use ade_ledger::receive::events::TipPoint;
use ade_node::convergence_evidence::{ConvergenceEvidence, ConvergenceEvidenceSink};

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
    NodeSyncItem::RollBack {
        peer: "peer-1".to_string(),
        point: WirePoint::Block {
            slot: SlotNo(slot),
            hash: h(hash),
        },
    }
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
        SecurityParam(2160),
        &mut None,
        None,
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
        SecurityParam(2160),
        &mut None,
        None,
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
        SecurityParam(2160),
        &mut None,
        None,
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
        SecurityParam(2160),
        &mut None,
        None,
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

// ---------- PHASE4-N-AK AK-S2 (DC-NODE-32): recovered-anchor rollback no-op ----------

fn anchor_tip(slot: u64, hash: u8) -> ChainTip {
    ChainTip {
        slot: SlotNo(slot),
        hash: h(hash),
    }
}

#[tokio::test]
async fn ak_s2_rollback_to_recovered_anchor_is_idempotent_noop() {
    // CE-AK-S2-1: a RollBackward binding EXACTLY (slot AND hash) to the recovered
    // anchor is an idempotent no-op -- run_node_sync returns Ok with NO durable
    // mutation (the node is already at the anchor; a bare anchor is a snapshot).
    let db = InMemoryChainDb::new(); // bare anchor: no stored blocks
    let mut fwd = fwd_at(8);
    fwd.recovered_anchor = Some(anchor_tip(188, 0x2e));
    let mut wal = VecWal::default();
    let ledger_fp_before = fingerprint(&fwd.receive.ledger).combined;
    let chain_dep_before = fwd.receive.chain_dep.clone();
    let mut src = NodeBlockSource::in_memory_items(vec![rollback_item(188, 0x2e)]);
    let out = run_node_sync(&mut src, &mut fwd, &db, &mut wal, &min_schedule(), &view_stub())
        .await
        .expect("rollback-to-recovered-anchor is an accepted no-op");
    assert!(out.is_none(), "a no-op rollback advances no tip");
    // No durable mutation: WAL empty, ChainDb tip absent, ledger + chain_dep intact.
    assert!(wal.entries.is_empty(), "no WAL append on the no-op");
    assert!(db.tip().unwrap().is_none(), "no ChainDb mutation");
    assert_eq!(fingerprint(&fwd.receive.ledger).combined, ledger_fp_before);
    assert_eq!(fwd.receive.chain_dep, chain_dep_before);
}

#[tokio::test]
async fn ak_s2_rollback_to_origin_fails_closed_even_with_anchor() {
    // CE-AK-S2-2: RollBackward(Origin) still fails closed (AI-S4a), even when a
    // recovered anchor is present.
    let db = InMemoryChainDb::new();
    let mut fwd = fwd_at(8);
    fwd.recovered_anchor = Some(anchor_tip(188, 0x2e));
    let mut wal = VecWal::default();
    let mut src = NodeBlockSource::in_memory_items(vec![NodeSyncItem::RollBack { peer: "peer-1".to_string(), point: WirePoint::Origin }]);
    let err = run_node_sync(&mut src, &mut fwd, &db, &mut wal, &min_schedule(), &view_stub())
        .await
        .expect_err("Origin rollback must fail closed");
    assert!(matches!(err, NodeSyncError::UnexpectedRollback), "got {err:?}");
}

#[tokio::test]
async fn ak_s2_non_anchor_rollback_fails_closed_slot_and_hash_bound() {
    // CE-AK-S2-3: the accepted rollback binds BOTH slot and hash -- a fully
    // different point, a slot-only match, and a hash-only match all fail closed
    // (never slot-alone, never hash-alone).
    let anchor = anchor_tip(188, 0x2e);
    for (label, item) in [
        ("different slot+hash", rollback_item(999, 0xFF)),
        ("slot match, hash differs", rollback_item(188, 0xFF)),
        ("hash match, slot differs", rollback_item(999, 0x2e)),
    ] {
        let db = InMemoryChainDb::new();
        let mut fwd = fwd_at(8);
        fwd.recovered_anchor = Some(anchor.clone());
        let mut wal = VecWal::default();
        let mut src = NodeBlockSource::in_memory_items(vec![item]);
        let err = run_node_sync(&mut src, &mut fwd, &db, &mut wal, &min_schedule(), &view_stub())
            .await
            .unwrap_err();
        assert!(
            matches!(err, NodeSyncError::UnexpectedRollback),
            "{label}: got {err:?}"
        );
    }
}

#[tokio::test]
async fn ak_s2_no_recovered_anchor_still_fails_closed() {
    // CE-AK-S2-6 (preserved): with NO recovered anchor (cold-start / non-recover
    // caller), ANY rollback still fails closed -- the pre-AK-S2 behavior is exact.
    let db = InMemoryChainDb::new();
    let mut fwd = fwd_at(8); // recovered_anchor defaults to None
    assert!(fwd.recovered_anchor.is_none());
    let mut wal = VecWal::default();
    let mut src = NodeBlockSource::in_memory_items(vec![rollback_item(188, 0x2e)]);
    let err = run_node_sync(&mut src, &mut fwd, &db, &mut wal, &min_schedule(), &view_stub())
        .await
        .expect_err("no recovered anchor => any rollback fails closed");
    assert!(matches!(err, NodeSyncError::UnexpectedRollback), "got {err:?}");
}

#[tokio::test]
async fn ak_s2_after_anchor_noop_forward_block_reaches_pump_block_validation_holds() {
    // CE-AK-S2-5: after the anchor no-op, the subsequent (here malformed) block
    // reaches the EXISTING pump_block, which fails closed on validation -- proving
    // AK-S2 added NO forward-admit logic and did not turn the rollback into a
    // skip-the-next-block: the error is a Pump validation error, NOT
    // UnexpectedRollback and NOT a silent accept.
    let db = InMemoryChainDb::new();
    let mut fwd = fwd_at(8);
    fwd.recovered_anchor = Some(anchor_tip(188, 0x2e));
    let mut wal = VecWal::default();
    let mut src = NodeBlockSource::in_memory_items(vec![
        rollback_item(188, 0x2e),                          // anchor no-op
        NodeSyncItem::Block { peer: "peer-1".to_string(), bytes: vec![0xDE, 0xAD, 0xBE, 0xEF] }, // malformed -> pump_block rejects
    ]);
    let err = run_node_sync(&mut src, &mut fwd, &db, &mut wal, &min_schedule(), &view_stub())
        .await
        .expect_err("the forward block reaches pump_block, which rejects the malformed bytes");
    assert!(
        matches!(err, NodeSyncError::Pump(_)),
        "got {err:?} (must be a Pump validation error, NOT UnexpectedRollback)"
    );
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
    let mut pending_switch: Option<ade_node::selector_state::PendingForkSwitch> = None;
    let mut src = NodeBlockSource::in_memory_items(vec![NodeSyncItem::Block { peer: "peer-1".to_string(), bytes: block }]);
    let result = run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &corpus_schedule(),
        &view,
        &mut pending,
        SecurityParam(2160),
        &mut pending_switch,
        None,
    )
    .await;
    // PHASE4-N-AO S7 (DC-NODE-38): a bare competing block whose branch cannot reach a
    // durable LCA (its parent is not in the cache -> BranchGap) NO-OPS, keeping the
    // current validated chain -- not a node-halting error. Pre-S7 Err(UnexpectedRollback)
    // was the live-geometry gap CE-AO-6 surfaced.
    assert!(result.is_ok(), "bare competing block -> no-op keep-current, got {result:?}");
    assert!(pending_switch.is_none(), "no fork-switch decision");
    assert!(wal.read_all().unwrap().is_empty(), "no durable mutation");
    assert!(!pending, "no forge fence set");
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
    let mut src = NodeBlockSource::in_memory_items(vec![NodeSyncItem::Block { peer: "peer-1".to_string(), bytes: block }]);
    run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &corpus_schedule(),
        &view,
        &mut pending,
        SecurityParam(2160),
        &mut None,
        None,
    )
    .await
    .expect("cold-start block admits via pump_block");
    let tip = db.tip().unwrap().unwrap();
    assert_eq!(tip.hash, decoded.block_hash);
}

// ---------- AJ-S2 (DC-NODE-30): convergence evidence emission ----------
// Evidence observes authority; evidence never becomes authority. These drive
// the SAME run_participant_sync path with a convergence-evidence sink and assert
// the emitted JSONL, without changing any consensus/rollback/admit behavior.

/// A `Write` backed by a shared buffer the test reads after the run.
#[derive(Clone, Default)]
struct SharedBuf(Rc<RefCell<Vec<u8>>>);
impl Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl SharedBuf {
    fn text(&self) -> String {
        String::from_utf8(self.0.borrow().clone()).expect("utf8")
    }
}

fn evidence_over(buf: &SharedBuf) -> ConvergenceEvidence {
    ConvergenceEvidence::new(
        ConvergenceEvidenceSink::with_writer(Box::new(buf.clone())),
        &h(0xCC),
    )
}

/// Cold-start ForwardSyncState seeded with the corpus epoch nonce (mirrors
/// `participant_block_with_no_durable_tip_pumps`).
fn cold_start_fwd(c: &ConwayValidityCorpus) -> ForwardSyncState {
    ForwardSyncState::new(
        ReceiveState::new(LedgerState::new(CardanoEra::Conway), {
            let mut s = PraosChainDepState::empty();
            s.epoch_nonce = Nonce(Hash32(c.epoch_nonce));
            s.evolving_nonce = Nonce(Hash32(c.epoch_nonce));
            s
        }),
        fingerprint(&LedgerState::new(CardanoEra::Conway)).combined,
        SnapshotCadence::DEFAULT,
    )
}

#[tokio::test]
async fn participant_cold_start_admit_emits_received_admitted_agreed() {
    // A cold-start admit (None durable tip -> pump_block) with the followed peer
    // tip set to the admitted block emits block_received (peer input) +
    // block_admitted (local admission) + agreement_verdict{agreed}; 0 diverged.
    let (c, view) = corpus_view();
    let block = pick_lightest(&c);
    let decoded = decode_block(&block).expect("decode");
    let tip = TipPoint {
        slot: decoded.header_input.slot,
        hash: decoded.block_hash.clone(),
        block_no: decoded.header_input.block_no.0,
    };
    let db = InMemoryChainDb::new();
    let mut fwd = cold_start_fwd(&c);
    let mut wal = VecWal::default();
    let mut pending = false;
    let mut src =
        NodeBlockSource::in_memory_with_followed_tip(vec![block], Some(tip));
    let buf = SharedBuf::default();
    let mut ev = evidence_over(&buf);
    run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &corpus_schedule(),
        &view,
        &mut pending,
        SecurityParam(2160),
        &mut None,
        Some(&mut ev),
    )
    .await
    .expect("cold-start admit");
    let out = buf.text();
    assert!(out.contains(r#""event":"block_received""#), "peer input: {out}");
    assert!(out.contains(r#""event":"block_admitted""#), "local admission: {out}");
    assert!(out.contains(r#""event":"agreement_verdict""#));
    assert!(out.contains(r#""kind":"agreed""#), "followed tip == admitted tip: {out}");
    assert!(!out.contains("diverged"), "no divergence following one peer");
    assert!(!ev.is_incomplete());
}

#[tokio::test]
async fn participant_block_received_does_not_imply_admission() {
    // A bare competing block is REFUSED (fail closed) -- block_received is emitted
    // (peer input) but NO block_admitted (admission is local + authoritative).
    let (c, view) = corpus_view();
    let block = pick_lightest(&c);
    let decoded = decode_block(&block).expect("decode");
    let db = InMemoryChainDb::new();
    db.put_block(&stored(decoded.header_input.slot.0.saturating_sub(1), 0xEE))
        .unwrap();
    let mut fwd = fwd_at(decoded.header_input.block_no.0.saturating_sub(1));
    let mut wal = VecWal::default();
    let mut pending = false;
    let mut src = NodeBlockSource::in_memory_items(vec![NodeSyncItem::Block { peer: "peer-1".to_string(), bytes: block }]);
    let buf = SharedBuf::default();
    let mut ev = evidence_over(&buf);
    // Fails closed on the bare competing block; the transcript is still recorded.
    let _ = run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &corpus_schedule(),
        &view,
        &mut pending,
        SecurityParam(2160),
        &mut None,
        Some(&mut ev),
    )
    .await;
    let out = buf.text();
    assert!(out.contains(r#""event":"block_received""#), "peer input recorded");
    assert!(
        !out.contains(r#""event":"block_admitted""#),
        "a refused block is NOT admitted: {out}"
    );
}

#[tokio::test]
async fn participant_convergence_evidence_replay_byte_identical() {
    // Same recovered store + same ordered events => byte-identical evidence
    // (no wall-clock; OQ-AJ-6). Evidence replay-equivalence.
    async fn run() -> String {
        let (c, view) = corpus_view();
        let block = pick_lightest(&c);
        let decoded = decode_block(&block).expect("decode");
        let tip = TipPoint {
            slot: decoded.header_input.slot,
            hash: decoded.block_hash.clone(),
            block_no: decoded.header_input.block_no.0,
        };
        let db = InMemoryChainDb::new();
        let mut fwd = cold_start_fwd(&c);
        let mut wal = VecWal::default();
        let mut pending = false;
        let mut src = NodeBlockSource::in_memory_with_followed_tip(
            vec![block],
            Some(tip),
        );
        let buf = SharedBuf::default();
        let mut ev = evidence_over(&buf);
        run_participant_sync(
            &mut src,
            &mut fwd,
            &db,
            &mut wal,
            &corpus_schedule(),
            &view,
            &mut pending,
            SecurityParam(2160),
            &mut None,
            Some(&mut ev),
        )
        .await
        .expect("admit");
        buf.text()
    }
    let a = run().await;
    let b = run().await;
    assert!(!a.is_empty());
    assert_eq!(a, b, "convergence evidence is replay-byte-identical");
}

// ---------- PHASE4-N-AL (DC-NODE-33): participant recovered-anchor boundary ----------
// The participant MIRROR of DC-NODE-32 (run_node_sync). A bare-anchor participant
// recover->follow must accept the relay's post-IntersectFound RollBackward(anchor) as
// an idempotent no-op; everything else (Origin, non-anchor, stored-block rollbacks) is
// unchanged. These prove CE-AL-1..5.

/// CE-AL-1: a RollBackward binding EXACTLY (slot AND hash) to the persisted recovered
/// anchor is an idempotent no-op -- the anchor is a recovery snapshot boundary, NOT a
/// stored servable block (so it is absent from the ChainDb). No durable mutation.
#[tokio::test]
async fn participant_rollback_to_recovered_anchor_is_noop() {
    let db = InMemoryChainDb::new(); // bare-anchor recovery: no servable blocks
    let mut fwd = fwd_at(52);
    fwd.recovered_anchor = Some(ChainTip {
        slot: SlotNo(188),
        hash: h(0xA8),
    });
    let mut wal = VecWal::default();
    let mut pending = false;
    let mut src = NodeBlockSource::in_memory_items(vec![rollback_item(188, 0xA8)]);
    run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &min_schedule(),
        &view_stub(),
        &mut pending,
        SecurityParam(2160),
        &mut None,
        None,
    )
    .await
    .expect("rollback-to-recovered-anchor is an idempotent no-op");
    // No durable effect: tip still None (empty db -- the anchor never becomes a stored
    // block), no WAL RollBack, pending never set.
    assert!(
        db.tip().unwrap().is_none(),
        "the anchor must never be synthesized into a stored block"
    );
    assert!(!wal
        .read_all()
        .unwrap()
        .iter()
        .any(|e| matches!(e, WalEntry::RollBack { .. })));
    assert!(!pending);
}

/// CE-AL-2: RollBackward(Origin) still fails closed (AI-S4a) even with a recovered
/// anchor set -- Origin is rejected during point extraction, BEFORE the DC-NODE-33 branch.
#[tokio::test]
async fn participant_rollback_origin_fails_closed() {
    let db = InMemoryChainDb::new();
    let mut fwd = fwd_at(52);
    fwd.recovered_anchor = Some(ChainTip {
        slot: SlotNo(188),
        hash: h(0xA8),
    });
    let mut wal = VecWal::default();
    let mut pending = false;
    let mut src = NodeBlockSource::in_memory_items(vec![NodeSyncItem::RollBack { peer: "peer-1".to_string(), point: WirePoint::Origin }]);
    let err = run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &min_schedule(),
        &view_stub(),
        &mut pending,
        SecurityParam(2160),
        &mut None,
        None,
    )
    .await
    .expect_err("Origin rollback must fail closed even with a recovered anchor");
    assert!(matches!(err, NodeSyncError::UnexpectedRollback), "got {err:?}");
}

/// CE-AL-3: the anchor no-op binds BOTH slot AND hash -- a different point, a slot-only
/// match, and a hash-only match all FAIL CLOSED (never the no-op); they fall through to
/// the unchanged DC-NODE-29 resolution (get_block_by_hash -> None on a bare-anchor store).
#[tokio::test]
async fn participant_rollback_non_anchor_fails_closed() {
    let anchor = ChainTip {
        slot: SlotNo(188),
        hash: h(0xA8),
    };
    for (slot, hash, label) in [
        (200u64, 0xB1u8, "different slot+hash"),
        (188u64, 0xB2u8, "slot-only match (hash differs)"),
        (200u64, 0xA8u8, "hash-only match (slot differs)"),
    ] {
        let db = InMemoryChainDb::new(); // none of these are stored
        let mut fwd = fwd_at(52);
        fwd.recovered_anchor = Some(anchor.clone());
        let mut wal = VecWal::default();
        let mut pending = false;
        let mut src = NodeBlockSource::in_memory_items(vec![rollback_item(slot, hash)]);
        let err = run_participant_sync(
            &mut src,
            &mut fwd,
            &db,
            &mut wal,
            &min_schedule(),
            &view_stub(),
            &mut pending,
            SecurityParam(2160),
            &mut None,
            None,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, NodeSyncError::UnexpectedRollback),
            "{label}: got {err:?}"
        );
        assert!(!pending, "{label}: pending must stay clear (fail before apply)");
    }
}

/// CE-AL-4: after the recovered-anchor rollback no-op, the FIRST forward block admits
/// through the EXISTING cold-start pump_block path (DC-NODE-33 adds no forward-link
/// code) -- the participant analog of the AK-S2 OQ #2 probe.
#[tokio::test]
async fn participant_first_forward_after_anchor_noop_admits_via_pump_block() {
    let (c, view) = corpus_view();
    let block = pick_lightest(&c);
    let decoded = decode_block(&block).expect("decode");
    let db = InMemoryChainDb::new();
    let mut fwd = cold_start_fwd(&c);
    // The bare boundary the feed rewinds to first (absent from the ChainDb).
    fwd.recovered_anchor = Some(ChainTip {
        slot: SlotNo(1),
        hash: h(0xA8),
    });
    let mut wal = VecWal::default();
    let mut pending = false;
    let mut src = NodeBlockSource::in_memory_items(vec![
        rollback_item(1, 0xA8),
        NodeSyncItem::Block {
            peer: "peer-1".to_string(),
            bytes: block,
        },
    ]);
    run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &corpus_schedule(),
        &view,
        &mut pending,
        SecurityParam(2160),
        &mut None,
        None,
    )
    .await
    .expect("anchor no-op, then the forward block admits via pump_block");
    let tip = db.tip().unwrap().unwrap();
    assert_eq!(
        tip.hash, decoded.block_hash,
        "the forward block admitted through the existing pump_block after the no-op"
    );
}

/// CE-AL-5: DC-NODE-29 preserved -- with a recovered anchor set, a rollback to an
/// actually-stored block (NOT the anchor) still routes through the unchanged
/// apply_chain_event stored-block authority; the DC-NODE-33 branch did not capture it.
#[tokio::test]
async fn participant_stored_block_rollback_still_applies() {
    let db = db_with_fork_and_snapshot(); // 100/0xF0 stored (+101/102), snapshot @ 100
    let mut fwd = fwd_at(52);
    // The anchor is a DIFFERENT point (50/0xCC) -- not the rollback target.
    fwd.recovered_anchor = Some(ChainTip {
        slot: SlotNo(50),
        hash: h(0xCC),
    });
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
        SecurityParam(2160),
        &mut None,
        None,
    )
    .await
    .expect("a stored-block rollback still applies via apply_chain_event");
    let tip = db.tip().unwrap().unwrap();
    assert_eq!(tip.slot, SlotNo(100));
    assert_eq!(tip.hash, h(0xF0));
    assert!(
        wal.read_all()
            .unwrap()
            .iter()
            .any(|e| matches!(e, WalEntry::RollBack { .. })),
        "the stored-block rollback produced a durable WAL RollBack (DC-NODE-29 path intact)"
    );
    assert!(!pending);
}

// ---------- PHASE4-N-AO S3 (DC-NODE-36): live selector dispatch (decide-only) ----------
// A competing block with a DURABLE fork anchor is routed to the SOLE BLUE
// select_best_chain; a win is held as a provisional PendingForkSwitch (+ forge
// fence) and NOTHING is applied (no rollback-commit, no body-fetch). The fork
// anchor binds Ade's durable stored (slot, hash) -- never peer data.

/// A durable fork over the Conway corpus: a real decodable durable TIP (a corpus
/// block, so its tiebreaker projects) + a stored fork ANCHOR at the competing
/// block's prev_hash + a snapshot AT the anchor carrying the corpus epoch nonce
/// (degenerate materialize -> the anchor chain_dep). `anchor_block_no` =
/// competing.block_no - 1 so the candidate validates (block_no strictly monotone).
/// Returns (db, prev_hash, anchor_block_no).
fn corpus_durable_fork(
    c: &ConwayValidityCorpus,
    competing: &[u8],
) -> (InMemoryChainDb, Hash32, u64) {
    let decoded = decode_block(competing).expect("decode competing");
    let prev = match decoded.prev_hash {
        PrevHash::Block(h) => h,
        PrevHash::Genesis => panic!("competing block must carry a Block prev_hash"),
    };
    // The durable TIP = a DIFFERENT corpus block (real bytes -> project_tiebreaker).
    // Its hash must be neither the competing block's hash nor its prev_hash (else a
    // linear extend / already-have, not a competing fork).
    let tip_bytes = c
        .blocks
        .iter()
        .find(|b| {
            let d = decode_block(b).expect("decode");
            d.block_hash != decoded.block_hash && d.block_hash != prev
        })
        .expect("a second distinct corpus block")
        .clone();
    let tip_dec = decode_block(&tip_bytes).expect("decode tip");
    let cand_slot = decoded.header_input.slot.0;
    let tip_slot = tip_dec.header_input.slot.0;
    // Anchor/snapshot slot: strictly below BOTH the tip (so the tip is the highest
    // stored block) and the competing block (header slot monotonicity).
    let anchor_slot = cand_slot.min(tip_slot).saturating_sub(1);
    let anchor_block_no = decoded.header_input.block_no.0.saturating_sub(1);
    let db = InMemoryChainDb::new();
    // Stored fork anchor: bound by (slot, hash); bytes never decoded (the snapshot
    // supplies the state).
    db.put_block(&StoredBlock {
        hash: prev.clone(),
        slot: SlotNo(anchor_slot),
        bytes: vec![0xAB; 8],
    })
    .unwrap();
    // Durable TIP: real corpus bytes at its real slot (the highest stored block).
    db.put_block(&StoredBlock {
        hash: tip_dec.block_hash.clone(),
        slot: tip_dec.header_input.slot,
        bytes: tip_bytes.clone(),
    })
    .unwrap();
    // Snapshot AT the anchor: corpus epoch nonce + anchor height + last_slot below
    // the competing block's slot. Degenerate materialize returns this as-is
    // (recovered_eta0 is None on the test fwd), so the candidate validates against
    // the corpus nonce -- the same basis as the cold-start admit path.
    let mut anchor_dep = PraosChainDepState::empty();
    anchor_dep.epoch_nonce = Nonce(Hash32(c.epoch_nonce));
    anchor_dep.evolving_nonce = Nonce(Hash32(c.epoch_nonce));
    anchor_dep.last_block_no = Some(ade_types::BlockNo(anchor_block_no));
    anchor_dep.last_slot = Some(SlotNo(anchor_slot));
    PersistentSnapshotCache::new(&db)
        .capture(
            SlotNo(anchor_slot),
            &LedgerState::new(CardanoEra::Conway),
            &anchor_dep,
        )
        .unwrap();
    (db, prev, anchor_block_no)
}

#[tokio::test]
async fn participant_competing_durable_anchor_loses_no_mutation() {
    // A competing block with a DURABLE fork anchor that validates but is SHORTER
    // than our tip => select_best_chain is reached (the arm no longer fails closed)
    // and returns a block-no loss => NO durable mutation, NO pending switch.
    let (c, view) = corpus_view();
    let competing = pick_lightest(&c);
    let cand_block_no = decode_block(&competing).unwrap().header_input.block_no.0;
    let (db, _prev, _anchor_bn) = corpus_durable_fork(&c, &competing);
    let tip_before = db.tip().unwrap().unwrap();
    // current tip height ABOVE the candidate => candidate loses on block number.
    let mut fwd = fwd_at(cand_block_no + 50);
    let mut wal = VecWal::default();
    let mut pending = false;
    let mut pending_switch: Option<ade_node::selector_state::PendingForkSwitch> = None;
    let mut src = NodeBlockSource::in_memory_items(vec![NodeSyncItem::Block {
        peer: "peer-1".to_string(),
        bytes: competing,
    }]);
    run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &corpus_schedule(),
        &view,
        &mut pending,
        SecurityParam(2160),
        &mut pending_switch,
        None,
    )
    .await
    .expect("a losing competing candidate is a no-op (NOT UnexpectedRollback)");
    assert_eq!(db.tip().unwrap().unwrap(), tip_before, "durable tip unchanged");
    assert!(wal.read_all().unwrap().is_empty(), "no WAL append on a loss");
    assert!(pending_switch.is_none(), "no PendingForkSwitch on a loss");
    assert!(!pending, "no forge fence on a loss");
}

#[tokio::test]
async fn participant_competing_durable_anchor_win_sets_pending_no_mutation() {
    // A competing block with a DURABLE fork anchor that validates and is LONGER
    // than our tip => select_best_chain emits ChainSelected => a PROVISIONAL
    // PendingForkSwitch (bound to the durable stored anchor) + the DC-NODE-28 forge
    // fence are set, and NOTHING is applied (tip + WAL byte-unchanged).
    let (c, view) = corpus_view();
    let competing = pick_lightest(&c);
    let cand_block_no = decode_block(&competing).unwrap().header_input.block_no.0;
    let (db, prev, anchor_bn) = corpus_durable_fork(&c, &competing);
    let tip_before = db.tip().unwrap().unwrap();
    // current tip height BELOW the candidate tip => candidate wins on block number.
    let mut fwd = fwd_at(cand_block_no - 1);
    let mut wal = VecWal::default();
    let mut pending = false;
    let mut pending_switch: Option<ade_node::selector_state::PendingForkSwitch> = None;
    let mut src = NodeBlockSource::in_memory_items(vec![NodeSyncItem::Block {
        peer: "peer-7".to_string(),
        bytes: competing,
    }]);
    run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &corpus_schedule(),
        &view,
        &mut pending,
        SecurityParam(2160),
        &mut pending_switch,
        None,
    )
    .await
    .expect("a winning competing candidate decides (no apply, no error)");
    let sw = pending_switch.expect("a fork-choice win sets PendingForkSwitch");
    assert_eq!(sw.winning_peer, "peer-7");
    assert_eq!(
        sw.fork_anchor.hash, prev,
        "the fork anchor binds Ade's durable STORED hash (never peer data)"
    );
    assert_eq!(sw.fork_anchor.block_no, ade_types::BlockNo(anchor_bn));
    assert!(pending, "the win sets the DC-NODE-28 forge fence");
    assert_eq!(
        db.tip().unwrap().unwrap(),
        tip_before,
        "S3 applies NOTHING: durable tip unchanged"
    );
    assert!(
        wal.read_all().unwrap().is_empty(),
        "S3 applies NOTHING: no WAL append"
    );
}

#[tokio::test]
async fn participant_competing_unknown_anchor_fails_closed() {
    // Proof center: a competing block whose prev_hash is NOT a durable stored block
    // fails closed -- the fork anchor can only be Ade's durable stored point, never
    // peer-supplied. (block_received is still emitted before the fail-closed.)
    let (c, view) = corpus_view();
    let competing = pick_lightest(&c);
    let decoded = decode_block(&competing).unwrap();
    let db = InMemoryChainDb::new();
    // Store ONLY an unrelated block (hash 0xEE != the competing block's prev_hash),
    // so get_block_by_hash(prev_hash) -> None.
    db.put_block(&stored(decoded.header_input.slot.0.saturating_sub(1), 0xEE))
        .unwrap();
    let mut fwd = fwd_at(decoded.header_input.block_no.0.saturating_sub(1));
    let mut wal = VecWal::default();
    let mut pending = false;
    let mut pending_switch: Option<ade_node::selector_state::PendingForkSwitch> = None;
    let mut src = NodeBlockSource::in_memory_items(vec![NodeSyncItem::Block {
        peer: "peer-1".to_string(),
        bytes: competing,
    }]);
    let result = run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &corpus_schedule(),
        &view,
        &mut pending,
        SecurityParam(2160),
        &mut pending_switch,
        None,
    )
    .await;
    // PHASE4-N-AO S7 (DC-NODE-38): an un-anchorable competing block (its parent is
    // neither a durable stored block nor a cached intermediate -> BranchGap) is NOT
    // selectable; it NO-OPS, keeping the current validated chain. The fork anchor is
    // still ONLY Ade's durable stored LCA, never peer-supplied. Pre-S7 this was a
    // node-halting Err(UnexpectedRollback) -- the live-geometry gap CE-AO-6 surfaced;
    // S7 walks the preserved links instead and fails closed as a no-op.
    assert!(result.is_ok(), "un-anchorable competing block -> no-op, got {result:?}");
    assert!(pending_switch.is_none(), "no fork-switch decision on a no-op");
    assert!(wal.read_all().unwrap().is_empty(), "no durable mutation");
    assert!(!pending, "no forge fence set");
}

#[tokio::test]
async fn participant_competing_fork_anchor_older_than_k_no_mutation() {
    // Negative (depth): a durable fork anchor deeper than k => select_best_chain
    // marks the candidate ineligible (ExceededRollback) => S3 emits no selection,
    // sets no PendingForkSwitch, makes no durable mutation, and does NOT invoke S4.
    // (S4 keeps its own independent materialize RollbackTooDeep guard.)
    let (c, view) = corpus_view();
    let competing = pick_lightest(&c);
    let cand_block_no = decode_block(&competing).unwrap().header_input.block_no.0;
    let (db, _prev, _anchor_bn) = corpus_durable_fork(&c, &competing);
    let tip_before = db.tip().unwrap().unwrap();
    // rollback_depth = current - anchor; anchor = cand_block_no - 1. current =
    // anchor + 10 => depth 10 > k(5) => ExceededRollback.
    let mut fwd = fwd_at(cand_block_no + 9);
    let mut wal = VecWal::default();
    let mut pending = false;
    let mut pending_switch: Option<ade_node::selector_state::PendingForkSwitch> = None;
    let mut src = NodeBlockSource::in_memory_items(vec![NodeSyncItem::Block {
        peer: "peer-1".to_string(),
        bytes: competing,
    }]);
    run_participant_sync(
        &mut src,
        &mut fwd,
        &db,
        &mut wal,
        &corpus_schedule(),
        &view,
        &mut pending,
        SecurityParam(5),
        &mut pending_switch,
        None,
    )
    .await
    .expect("a too-deep fork anchor is a no-op (not an error)");
    assert!(pending_switch.is_none(), "too-deep => no PendingForkSwitch");
    assert_eq!(
        db.tip().unwrap().unwrap(),
        tip_before,
        "too-deep => no durable mutation"
    );
    assert!(wal.read_all().unwrap().is_empty(), "no WAL append");
    assert!(!pending, "no forge fence when nothing is selected");
}

// ---------- PHASE4-N-AO S4 (DC-NODE-37): fork-switch apply (prove, then commit) ----------
// A PendingForkSwitch authorizes PROOF of the selected replacement branch, not a
// rollback. apply_fork_switch fetches + binds + links + ledger-prevalidates the
// complete branch BEFORE the irreversible commit_rollback. A failed proof leaves
// the durable chain byte-unchanged and HOLDS the forge fence.

/// In-memory `BranchBodySource` (the hermetic S4 fetch seam; live BlockFetch is
/// CE-AO-6). Serves a body by (peer, slot).
struct InMemBodySource {
    bodies: BTreeMap<(String, u64), Vec<u8>>,
}
impl InMemBodySource {
    fn with(peer: &str, slot: SlotNo, bytes: Vec<u8>) -> Self {
        let mut bodies = BTreeMap::new();
        bodies.insert((peer.to_string(), slot.0), bytes);
        Self { bodies }
    }
    fn empty() -> Self {
        Self {
            bodies: BTreeMap::new(),
        }
    }
}
impl BranchBodySource for InMemBodySource {
    fn fetch_body(&self, peer: &str, slot: SlotNo) -> Result<Vec<u8>, FetchError> {
        self.bodies
            .get(&(peer.to_string(), slot.0))
            .cloned()
            .ok_or(FetchError::Unavailable)
    }
}

/// A durable fork over the corpus (like `corpus_durable_fork`) returning the
/// `ForkAnchor` for the `PendingForkSwitch`. `nonce` overrides the snapshot epoch
/// nonce (Some => block_validity will fail the candidate's VRF: the invalid-body case).
fn s4_fork_setup(
    c: &ConwayValidityCorpus,
    competing: &[u8],
    nonce: Option<Hash32>,
) -> (InMemoryChainDb, ForkAnchor) {
    let decoded = decode_block(competing).expect("decode competing");
    let prev = match decoded.prev_hash {
        PrevHash::Block(h) => h,
        PrevHash::Genesis => panic!("competing block must carry a Block prev_hash"),
    };
    let tip_bytes = c
        .blocks
        .iter()
        .find(|b| {
            let d = decode_block(b).expect("decode");
            d.block_hash != decoded.block_hash && d.block_hash != prev
        })
        .expect("a second distinct corpus block")
        .clone();
    let tip_dec = decode_block(&tip_bytes).expect("decode tip");
    let anchor_slot = decoded
        .header_input
        .slot
        .0
        .min(tip_dec.header_input.slot.0)
        .saturating_sub(1);
    let anchor_block_no = decoded.header_input.block_no.0.saturating_sub(1);
    let db = InMemoryChainDb::new();
    db.put_block(&StoredBlock {
        hash: prev.clone(),
        slot: SlotNo(anchor_slot),
        bytes: vec![0xAB; 8],
    })
    .unwrap();
    db.put_block(&StoredBlock {
        hash: tip_dec.block_hash.clone(),
        slot: tip_dec.header_input.slot,
        bytes: tip_bytes.clone(),
    })
    .unwrap();
    let n = nonce.unwrap_or(Hash32(c.epoch_nonce));
    let mut anchor_dep = PraosChainDepState::empty();
    anchor_dep.epoch_nonce = Nonce(n.clone());
    anchor_dep.evolving_nonce = Nonce(n);
    anchor_dep.last_block_no = Some(ade_types::BlockNo(anchor_block_no));
    anchor_dep.last_slot = Some(SlotNo(anchor_slot));
    PersistentSnapshotCache::new(&db)
        .capture(
            SlotNo(anchor_slot),
            &LedgerState::new(CardanoEra::Conway),
            &anchor_dep,
        )
        .unwrap();
    (
        db,
        ForkAnchor {
            slot: SlotNo(anchor_slot),
            hash: prev,
            block_no: ade_types::BlockNo(anchor_block_no),
        },
    )
}

/// Build a `PendingForkSwitch` for `competing` (a single-header branch), with the
/// header summary derived directly from the block (S3 would have produced the same).
fn s4_switch(competing: &[u8], fork_anchor: ForkAnchor, peer: &str) -> PendingForkSwitch {
    let decoded = decode_block(competing).expect("decode");
    let h = &decoded.header_input;
    let vrf_leader_output = match &h.vrf {
        HeaderVrf::Praos { output, .. } => praos_leader_value(output),
        HeaderVrf::Tpraos { .. } => panic!("corpus is Conway/Praos"),
    };
    let mut first8 = [0u8; 8];
    first8.copy_from_slice(&vrf_leader_output.0[0..8]);
    let summary = ValidatedHeaderSummary {
        slot: h.slot,
        block_no: h.block_no,
        body_hash: h.body_hash.clone(),
        issuer_pool: h.issuer_pool.clone(),
        op_cert_counter: h.op_cert_counter,
        vrf_leader_output,
    };
    let select_view = TiebreakerView {
        slot: h.slot,
        issuer_hash: h.issuer_pool.clone(),
        op_cert_counter: h.op_cert_counter,
        leader_vrf_output_first_8: first8,
    };
    PendingForkSwitch {
        fork_anchor: fork_anchor.clone(),
        winning_peer: peer.to_string(),
        winning_candidate: CandidateFragment {
            anchor: Point {
                slot: fork_anchor.slot,
                hash: fork_anchor.hash.clone(),
            },
            anchor_block_no: fork_anchor.block_no,
            headers: vec![summary],
            select_view,
            rollback_depth: BlockDistance(0),
        },
        // The competing block's tip point -- the S6 BlockFetch endpoint.
        winner_tip: Point {
            slot: h.slot,
            hash: decoded.block_hash.clone(),
        },
    }
}

#[tokio::test]
async fn fork_switch_win_adopts_via_rolledback_then_chainselected() {
    // Happy path: a fully proven branch is durably adopted -- RolledBack(anchor) +
    // ChainSelected(body) with ForkChoiceWin; durable tip = selected tip; fence cleared.
    let (c, view) = corpus_view();
    let competing = pick_lightest(&c);
    let decoded = decode_block(&competing).unwrap();
    let (db, anchor) = s4_fork_setup(&c, &competing, None);
    let switch = s4_switch(&competing, anchor, "peer-7");
    let mut fwd = fwd_at(decoded.header_input.block_no.0);
    let mut wal = VecWal::default();
    let mut pending = Some(switch.clone());
    let mut fence = true; // S3 set the fence on the win
    let mut last_fail = None;
    let src = InMemBodySource::with("peer-7", decoded.header_input.slot, competing.clone());
    let outcome = apply_fork_switch(
        &mut fwd,
        &db,
        &mut wal,
        &switch,
        &mut pending,
        &mut fence,
        &mut last_fail,
        &src,
        &corpus_schedule(),
        &view,
    )
    .expect("apply returns");
    match outcome {
        ForkSwitchOutcome::Adopted { new_tip } => {
            assert_eq!(new_tip.slot, decoded.header_input.slot);
            assert_eq!(new_tip.hash, decoded.block_hash);
        }
        ForkSwitchOutcome::ProofFailed { error } => panic!("expected adoption, got {error:?}"),
    }
    assert_eq!(
        db.tip().unwrap().unwrap().hash,
        decoded.block_hash,
        "durable tip = selected tip"
    );
    assert!(
        wal.read_all().unwrap().iter().any(|e| matches!(
            e,
            WalEntry::RollBack {
                reason: RollbackReason::ForkChoiceWin,
                ..
            }
        )),
        "a ForkChoiceWin rollback was recorded"
    );
    assert!(pending.is_none(), "decision cleared after adoption");
    assert!(!fence, "forge fence cleared after reconcile");
    assert!(last_fail.is_none());
}

#[tokio::test]
async fn selected_peer_missing_body_leaves_chain_unchanged_fence_held() {
    // No body served -> proof fails closed; current chain unchanged; the decision is
    // retired as a structured failure but the forge fence is HELD (no silent resume).
    let (c, view) = corpus_view();
    let competing = pick_lightest(&c);
    let decoded = decode_block(&competing).unwrap();
    let (db, anchor) = s4_fork_setup(&c, &competing, None);
    let switch = s4_switch(&competing, anchor, "peer-7");
    let tip_before = db.tip().unwrap().unwrap();
    let mut fwd = fwd_at(decoded.header_input.block_no.0);
    let mut wal = VecWal::default();
    let mut pending = Some(switch.clone());
    let mut fence = true;
    let mut last_fail = None;
    let src = InMemBodySource::empty();
    let outcome = apply_fork_switch(
        &mut fwd, &db, &mut wal, &switch, &mut pending, &mut fence, &mut last_fail, &src,
        &corpus_schedule(), &view,
    )
    .expect("apply returns");
    assert!(
        matches!(
            outcome,
            ForkSwitchOutcome::ProofFailed {
                error: BranchProofError::BodyUnavailable { .. }
            }
        ),
        "got {outcome:?}"
    );
    assert_eq!(db.tip().unwrap().unwrap(), tip_before, "no durable mutation");
    assert!(wal.read_all().unwrap().is_empty(), "no WAL append");
    assert!(pending.is_none(), "decision retired as failed");
    assert!(
        matches!(last_fail, Some(BranchProofError::BodyUnavailable { .. })),
        "structured failure recorded"
    );
    assert!(fence, "forge fence HELD -- never cleared by an unproven branch");
}

#[tokio::test]
async fn body_hash_mismatch_leaves_chain_unchanged() {
    // A DIFFERENT block served for the selected slot -> bind fails; chain unchanged.
    let (c, view) = corpus_view();
    let competing = pick_lightest(&c);
    let decoded = decode_block(&competing).unwrap();
    let (db, anchor) = s4_fork_setup(&c, &competing, None);
    let switch = s4_switch(&competing, anchor, "peer-7");
    let tip_before = db.tip().unwrap().unwrap();
    let other = c
        .blocks
        .iter()
        .find(|b| decode_block(b).unwrap().block_hash != decoded.block_hash)
        .unwrap()
        .clone();
    let mut fwd = fwd_at(decoded.header_input.block_no.0);
    let mut wal = VecWal::default();
    let mut pending = Some(switch.clone());
    let mut fence = true;
    let mut last_fail = None;
    let src = InMemBodySource::with("peer-7", decoded.header_input.slot, other);
    let outcome = apply_fork_switch(
        &mut fwd, &db, &mut wal, &switch, &mut pending, &mut fence, &mut last_fail, &src,
        &corpus_schedule(), &view,
    )
    .expect("apply returns");
    assert!(
        matches!(
            outcome,
            ForkSwitchOutcome::ProofFailed {
                error: BranchProofError::BodyHeaderMismatch { .. }
            }
        ),
        "got {outcome:?}"
    );
    assert_eq!(db.tip().unwrap().unwrap(), tip_before);
    assert!(wal.read_all().unwrap().is_empty());
    assert!(fence, "fence held");
}

#[tokio::test]
async fn broken_parent_link_leaves_chain_unchanged() {
    // The fork anchor HASH does not match the body's prev_hash -> link fails.
    let (c, view) = corpus_view();
    let competing = pick_lightest(&c);
    let decoded = decode_block(&competing).unwrap();
    let (db, anchor) = s4_fork_setup(&c, &competing, None);
    // Tamper the anchor hash (slot stays, so materialize still finds the snapshot).
    let bad_anchor = ForkAnchor {
        slot: anchor.slot,
        hash: h(0xDD),
        block_no: anchor.block_no,
    };
    let switch = s4_switch(&competing, bad_anchor, "peer-7");
    let tip_before = db.tip().unwrap().unwrap();
    let mut fwd = fwd_at(decoded.header_input.block_no.0);
    let mut wal = VecWal::default();
    let mut pending = Some(switch.clone());
    let mut fence = true;
    let mut last_fail = None;
    let src = InMemBodySource::with("peer-7", decoded.header_input.slot, competing.clone());
    let outcome = apply_fork_switch(
        &mut fwd, &db, &mut wal, &switch, &mut pending, &mut fence, &mut last_fail, &src,
        &corpus_schedule(), &view,
    )
    .expect("apply returns");
    assert!(
        matches!(
            outcome,
            ForkSwitchOutcome::ProofFailed {
                error: BranchProofError::BrokenParentLink { .. }
            }
        ),
        "got {outcome:?}"
    );
    assert_eq!(db.tip().unwrap().unwrap(), tip_before);
    assert!(wal.read_all().unwrap().is_empty());
    assert!(fence, "fence held");
}

#[tokio::test]
async fn invalid_body_rejected_before_commit_no_half_switch() {
    // THE critical case: the body decodes, binds, and links -- but FAILS ledger
    // validation (the materialized anchor carries a WRONG epoch nonce, so the
    // block's VRF fails). The prevalidation fold rejects it BEFORE commit_rollback,
    // so there is no half-switched durable state.
    let (c, view) = corpus_view();
    let competing = pick_lightest(&c);
    let decoded = decode_block(&competing).unwrap();
    let (db, anchor) = s4_fork_setup(&c, &competing, Some(h(0x99))); // wrong nonce
    let switch = s4_switch(&competing, anchor, "peer-7");
    let tip_before = db.tip().unwrap().unwrap();
    let mut fwd = fwd_at(decoded.header_input.block_no.0);
    let mut wal = VecWal::default();
    let mut pending = Some(switch.clone());
    let mut fence = true;
    let mut last_fail = None;
    let src = InMemBodySource::with("peer-7", decoded.header_input.slot, competing.clone());
    let outcome = apply_fork_switch(
        &mut fwd, &db, &mut wal, &switch, &mut pending, &mut fence, &mut last_fail, &src,
        &corpus_schedule(), &view,
    )
    .expect("apply returns");
    assert!(
        matches!(
            outcome,
            ForkSwitchOutcome::ProofFailed {
                error: BranchProofError::BodyInvalid { .. }
            }
        ),
        "the invalid body must be caught BEFORE commit, got {outcome:?}"
    );
    assert_eq!(
        db.tip().unwrap().unwrap(),
        tip_before,
        "no half-switched durable state"
    );
    assert!(
        wal.read_all().unwrap().is_empty(),
        "no commit_rollback, no WAL"
    );
    assert!(fence, "fence held");
}

#[tokio::test]
async fn too_deep_rollback_fails_closed_unchanged() {
    // A fork anchor below the oldest snapshot -> materialize RollbackTooDeep ->
    // AnchorUnreachable, caught in prevalidation BEFORE any commit.
    let (c, view) = corpus_view();
    let competing = pick_lightest(&c);
    let decoded = decode_block(&competing).unwrap();
    let (db, anchor) = s4_fork_setup(&c, &competing, None);
    let deep_anchor = ForkAnchor {
        slot: SlotNo(0),
        hash: anchor.hash.clone(),
        block_no: anchor.block_no,
    };
    let switch = s4_switch(&competing, deep_anchor, "peer-7");
    let tip_before = db.tip().unwrap().unwrap();
    let mut fwd = fwd_at(decoded.header_input.block_no.0);
    let mut wal = VecWal::default();
    let mut pending = Some(switch.clone());
    let mut fence = true;
    let mut last_fail = None;
    let src = InMemBodySource::with("peer-7", decoded.header_input.slot, competing.clone());
    let outcome = apply_fork_switch(
        &mut fwd, &db, &mut wal, &switch, &mut pending, &mut fence, &mut last_fail, &src,
        &corpus_schedule(), &view,
    )
    .expect("apply returns");
    assert!(
        matches!(
            outcome,
            ForkSwitchOutcome::ProofFailed {
                error: BranchProofError::AnchorUnreachable
            }
        ),
        "got {outcome:?}"
    );
    assert_eq!(db.tip().unwrap().unwrap(), tip_before);
    assert!(wal.read_all().unwrap().is_empty());
    assert!(fence, "fence held");
}

// ---------- PHASE4-N-AO S5 (CE-AO-5): selector==durable + fence resolution ----------

#[tokio::test]
async fn selector_equals_durable_post_forkchoicewin() {
    // After a ForkChoiceWin adoption, the selector projection of the durable tip
    // equals the adopted winner -- selector and durable converge (no persisted
    // selector; S3 Option A derives it from the durable tip).
    let (c, view) = corpus_view();
    let competing = pick_lightest(&c);
    let decoded = decode_block(&competing).unwrap();
    let (db, anchor) = s4_fork_setup(&c, &competing, None);
    let switch = s4_switch(&competing, anchor, "peer-7");
    let mut fwd = fwd_at(decoded.header_input.block_no.0);
    let mut wal = VecWal::default();
    let mut pending = Some(switch.clone());
    let mut fence = true;
    let mut last_fail = None;
    let src = InMemBodySource::with("peer-7", decoded.header_input.slot, competing.clone());
    apply_fork_switch(
        &mut fwd, &db, &mut wal, &switch, &mut pending, &mut fence, &mut last_fail, &src,
        &corpus_schedule(), &view,
    )
    .expect("adopt");
    let durable = db.tip().unwrap().unwrap();
    let stored = db.get_block_by_hash(&durable.hash).unwrap().unwrap();
    let tip_dec = decode_block(&stored.bytes).unwrap();
    let projected = project_tiebreaker(&tip_dec.header_input).unwrap();
    assert_eq!(
        projected, switch.winning_candidate.select_view,
        "selector projection of the durable tip == the adopted winner (selector == durable)"
    );
}

#[tokio::test]
async fn proof_failure_holds_fence_then_resolves_when_caught_up() {
    // S4 proof failure HOLDS the fence; it clears ONLY on a resolved state (no
    // pending decision AND caught up) -- never as a failure side effect.
    let (c, view) = corpus_view();
    let competing = pick_lightest(&c);
    let decoded = decode_block(&competing).unwrap();
    let (db, anchor) = s4_fork_setup(&c, &competing, None);
    let switch = s4_switch(&competing, anchor, "peer-7");
    let mut fwd = fwd_at(decoded.header_input.block_no.0);
    let mut wal = VecWal::default();
    let mut pending = Some(switch.clone());
    let mut fence = true;
    let mut last_fail = None;
    let src = InMemBodySource::empty(); // proof fails: no body served
    apply_fork_switch(
        &mut fwd, &db, &mut wal, &switch, &mut pending, &mut fence, &mut last_fail, &src,
        &corpus_schedule(), &view,
    )
    .expect("apply returns");
    assert!(pending.is_none(), "decision retired as failed");
    assert!(fence, "fence HELD on proof failure");
    // The fence resolves ONLY when no pending AND caught up. The failure alone
    // (not yet caught up) does NOT resolve it.
    assert!(
        !fork_switch_fence_resolved(&pending, false),
        "not caught up -> fence stays held"
    );
    assert!(
        fork_switch_fence_resolved(&pending, true),
        "no pending + caught up -> resolved"
    );
}

// ---------- PHASE4-N-AO S6 (CE-AO-6): the live fetch source is byte-only ----------
// The production PrefetchedBranchBodies (what the relay loop fills from a live
// BlockFetch) is NO stronger than the hermetic doubles: a lying or short fetch is
// rejected by S4's prove phase BEFORE commit_rollback. BlockFetch transports
// bytes; it does not grant truth.

#[tokio::test]
async fn live_fetch_lying_body_rejected_before_commit() {
    // The fetch ENDPOINT is correct-looking (winner_tip = the selected block's tip),
    // but the peer serves a DIFFERENT body for that slot -> the returned bytes do
    // NOT bind to the S3-selected header -> BodyHeaderMismatch before commit; chain
    // unchanged, fence held. winner_tip is an address, not adoption authority.
    let (c, view) = corpus_view();
    let competing = pick_lightest(&c);
    let decoded = decode_block(&competing).unwrap();
    let (db, anchor) = s4_fork_setup(&c, &competing, None);
    let switch = s4_switch(&competing, anchor, "peer-7");
    assert_eq!(
        switch.winner_tip.hash, decoded.block_hash,
        "the fetch endpoint (winner_tip) is the correct selected tip"
    );
    let tip_before = db.tip().unwrap().unwrap();
    let other = c
        .blocks
        .iter()
        .find(|b| decode_block(b).unwrap().block_hash != decoded.block_hash)
        .unwrap()
        .clone();
    let mut fwd = fwd_at(decoded.header_input.block_no.0);
    let mut wal = VecWal::default();
    let mut pending = Some(switch.clone());
    let mut fence = true;
    let mut last_fail = None;
    let mut src = PrefetchedBranchBodies::new();
    src.insert("peer-7", decoded.header_input.slot, other); // a lying body
    let outcome = apply_fork_switch(
        &mut fwd, &db, &mut wal, &switch, &mut pending, &mut fence, &mut last_fail, &src,
        &corpus_schedule(), &view,
    )
    .expect("apply returns");
    assert!(
        matches!(
            outcome,
            ForkSwitchOutcome::ProofFailed {
                error: BranchProofError::BodyHeaderMismatch { .. }
            }
        ),
        "a lying live body must be rejected before commit, got {outcome:?}"
    );
    assert_eq!(db.tip().unwrap().unwrap(), tip_before, "no commit");
    assert!(wal.read_all().unwrap().is_empty());
    assert!(fence, "fence held");
}

#[tokio::test]
async fn live_fetch_short_range_rejected_before_commit() {
    // A truncated fetch -- fewer bodies than the candidate's header count (mux/peer
    // truncation) -> BodyUnavailable before commit; chain unchanged, fence held.
    // Distinct from the lying-body case (a missing body, not a wrong one).
    let (c, view) = corpus_view();
    let competing = pick_lightest(&c);
    let decoded = decode_block(&competing).unwrap();
    let (db, anchor) = s4_fork_setup(&c, &competing, None);
    // A two-header candidate: the live fetch must provide BOTH bodies.
    let mut switch = s4_switch(&competing, anchor, "peer-7");
    let mut second = switch.winning_candidate.headers[0].clone();
    second.slot = SlotNo(switch.winning_candidate.headers[0].slot.0 + 1);
    second.block_no = ade_types::BlockNo(switch.winning_candidate.headers[0].block_no.0 + 1);
    let second_slot = second.slot;
    switch.winning_candidate.headers.push(second);
    let tip_before = db.tip().unwrap().unwrap();
    let mut fwd = fwd_at(decoded.header_input.block_no.0);
    let mut wal = VecWal::default();
    let mut pending = Some(switch.clone());
    let mut fence = true;
    let mut last_fail = None;
    // The fetch provides only the FIRST body -> the branch is short.
    let mut src = PrefetchedBranchBodies::new();
    src.insert("peer-7", decoded.header_input.slot, competing.clone());
    let outcome = apply_fork_switch(
        &mut fwd, &db, &mut wal, &switch, &mut pending, &mut fence, &mut last_fail, &src,
        &corpus_schedule(), &view,
    )
    .expect("apply returns");
    assert!(
        matches!(
            outcome,
            ForkSwitchOutcome::ProofFailed {
                error: BranchProofError::BodyUnavailable { slot }
            } if slot == second_slot
        ),
        "a short (truncated) live fetch must be rejected before commit, got {outcome:?}"
    );
    assert_eq!(db.tip().unwrap().unwrap(), tip_before, "no commit");
    assert!(wal.read_all().unwrap().is_empty());
    assert!(fence, "fence held");
}
