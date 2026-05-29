// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED node-binary restart recovery wiring (PHASE4-N-Y S3).
//!
//! Composes the EXISTING authorities — no second recovery engine:
//!   1. `WalStore::read_all` over the reopened on-disk WAL. A corrupt
//!      sealed WAL file fails closed in `FileWalStore::open`
//!      (`WalError::CorruptCrc`) before this point; a partial tail
//!      frame is dropped there too (the un-acked append never
//!      happened).
//!   2. `ade_ledger::wal::replay_from_anchor` (BLUE) over
//!      `(anchor_fp, WAL entries, preserved block bytes keyed by
//!      block hash)`. This is the fail-fast integrity gate: a WAL
//!      entry whose preserved bytes are absent surfaces
//!      `WalError::BlockBytesMissing`; a broken fingerprint link
//!      surfaces `WalError::ChainBreak`. No silent partial recovery.
//!   3. `bootstrap_initial_state` (GREEN, warm-start branch) — the
//!      snapshot + forward-replay recovery authority. It loads the
//!      latest valid checkpoint ≤ the chaindb tip and replays forward
//!      over preserved bytes via `materialize_rolled_back_state`
//!      (re-validating through `block_validity`). A partially-written
//!      checkpoint is treated as absent by the snapshot decoder
//!      (DC-STORE-03), so recovery falls back to the prior valid one.
//!
//! The recovered ledger fingerprint MUST equal a clean run's
//! (DC-STORE-01, T-DET-01). There is no operator-repair step:
//! recovery is `{anchor + preserved bytes + WAL + latest checkpoint +
//! forward replay}` and nothing else.

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::state::LedgerState;
use ade_ledger::wal::{replay_from_anchor, WalEntry, WalError, WalStore};
use ade_types::SlotNo;
use ade_types::Hash32;

use crate::bootstrap::{bootstrap_initial_state, BootstrapError, BootstrapInputs};
use crate::chaindb::{ChainDb, ChainDbError, ChainTip, SnapshotStore};

/// The authoritative state reconstructed by an unclean restart.
#[derive(Debug)]
pub struct RecoveredNode {
    pub ledger: LedgerState,
    pub chain_dep: PraosChainDepState,
    pub tip: Option<ChainTip>,
    /// `fingerprint(ledger).combined` of the recovered state. MUST
    /// equal the clean-run fingerprint (DC-STORE-01).
    pub recovered_fp: Hash32,
    /// WAL entries replayed by the integrity gate (step 2).
    pub wal_entries_verified: u64,
}

/// Closed restart-recovery error surface. Every variant is a
/// deterministic fail-fast halt — none permits a silent partial
/// recovery.
#[derive(Debug)]
pub enum NodeRecoveryError {
    /// WAL read / integrity failure: corrupt CRC, missing preserved
    /// block bytes for a WAL-referenced hash, or a broken fingerprint
    /// chain link. Carries the upstream `WalError` verbatim
    /// (`CorruptCrc` / `BlockBytesMissing` / `ChainBreak`).
    Wal(WalError),
    /// Underlying chaindb / snapshot-store read error.
    ChainDb(ChainDbError),
    /// The snapshot + forward-replay authority rejected recovery
    /// (replay re-validation failed, snapshot missing, etc.).
    Bootstrap(BootstrapError),
    /// After reconciling the chaindb to the WAL tail and re-running
    /// warm-start, the recovered ledger fingerprint did not equal the
    /// WAL-tail post-fp that the BLUE integrity gate computed. The WAL
    /// is the admission authority; a residual mismatch is a
    /// deterministic fail-fast halt, never a silent divergence.
    WalTailFingerprintMismatch { expected: Hash32, actual: Hash32 },
}

impl std::fmt::Display for NodeRecoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeRecoveryError::Wal(e) => write!(f, "wal recovery: {e:?}"),
            NodeRecoveryError::ChainDb(e) => write!(f, "chaindb recovery: {e}"),
            NodeRecoveryError::Bootstrap(e) => write!(f, "bootstrap recovery: {e:?}"),
            NodeRecoveryError::WalTailFingerprintMismatch { expected, actual } => write!(
                f,
                "wal-tail fingerprint mismatch after reconciliation: expected {expected:?}, got {actual:?}"
            ),
        }
    }
}

impl std::error::Error for NodeRecoveryError {}

/// Reconstruct authoritative state after an unclean restart.
///
/// `anchor_fp` is the verified `BootstrapAnchor`'s
/// `initial_ledger_fingerprint` — the first link of the WAL
/// fingerprint chain (CN-ANCHOR-01: recovery binds to the same anchor
/// lineage). `genesis_initial` is the cold-start seed forwarded
/// straight to `bootstrap_initial_state`; on a warm restart (non-empty
/// store) it is unused.
///
/// Order is fixed and total:
///   WAL read → BLUE integrity replay (fail-fast) → warm-start
///   materialize → fingerprint.
pub fn recover_node_state<D, S>(
    chaindb: &D,
    snapshot_store: &S,
    wal: &dyn WalStore,
    anchor_fp: &Hash32,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    genesis_initial: Option<(LedgerState, PraosChainDepState)>,
) -> Result<RecoveredNode, NodeRecoveryError>
where
    D: ChainDb + SnapshotStore,
    S: SnapshotStore,
{
    // 1. Reopened-WAL entries (a corrupt sealed file already failed in
    //    `FileWalStore::open`; a partial tail frame was dropped there).
    let entries = wal.read_all().map_err(NodeRecoveryError::Wal)?;

    // 2. BLUE integrity gate. Build the preserved-bytes map keyed by
    //    block hash from the chaindb; `replay_from_anchor` fails closed
    //    with `BlockBytesMissing` if any WAL-referenced block's bytes
    //    are absent, or `ChainBreak` on a broken fingerprint link.
    let mut block_bytes = std::collections::BTreeMap::new();
    for entry in &entries {
        let WalEntry::AdmitBlock { block_hash, .. } = entry;
        if let Some(stored) = chaindb
            .get_block_by_hash(block_hash)
            .map_err(NodeRecoveryError::ChainDb)?
        {
            block_bytes.insert(block_hash.clone(), stored.bytes);
        }
    }
    let wal_tail_fp = replay_from_anchor(anchor_fp, &entries, &block_bytes)
        .map_err(NodeRecoveryError::Wal)?;

    // 3. Reconcile the chaindb to the WAL tail BEFORE warm-start. The
    //    WAL — not `chaindb.tip()` — is the admission authority. S2's
    //    pump applies `StoreBlockBytes` before `AppendWal`, so a crash
    //    between them leaves an orphan block durable in the chaindb but
    //    absent from the WAL; warm-start materializes from the highest
    //    stored slot, so the orphan would otherwise be incorporated.
    //    Dropping every block at a slot strictly greater than the
    //    WAL-tail slot is deterministic and idempotent. An empty WAL has
    //    no admitted blocks: every stored block is an orphan, so we roll
    //    back to slot 0.
    let wal_tail_slot = entries
        .last()
        .map(|WalEntry::AdmitBlock { slot, .. }| *slot)
        .unwrap_or(SlotNo(0));
    chaindb
        .rollback_to_slot(wal_tail_slot)
        .map_err(NodeRecoveryError::ChainDb)?;

    // 4. The snapshot + forward-replay recovery authority (warm-start
    //    branch). A partially-written checkpoint decodes as absent
    //    (DC-STORE-03) and the authority falls back to the prior valid
    //    snapshot, replaying forward over preserved bytes.
    let (ledger, chain_dep, tip) = bootstrap_initial_state(BootstrapInputs {
        chaindb,
        snapshot_store,
        era_schedule,
        ledger_view,
        genesis_initial,
    })
    .map_err(NodeRecoveryError::Bootstrap)?;

    let recovered_fp = fingerprint(&ledger).combined;

    // 5. The recovered state MUST equal the WAL-tail post-fp the BLUE
    //    integrity gate computed. After reconciliation this is expected
    //    to hold; a residual mismatch is a deterministic fail-fast halt,
    //    never a silent divergence (the WAL is the authority). Each WAL
    //    `post_fp` is a genuine `fingerprint(ledger).combined` (the
    //    admit reducer computes it that way), so the tail post-fp is
    //    directly comparable to the materialized recovered fingerprint.
    //    The empty-WAL case is exempt: there `wal_tail_fp` is the
    //    anchor seed fingerprint, whose comparability is the anchor
    //    materialization's own concern (CN-ANCHOR-01), not this guard.
    if !entries.is_empty() && recovered_fp != wal_tail_fp {
        return Err(NodeRecoveryError::WalTailFingerprintMismatch {
            expected: wal_tail_fp,
            actual: recovered_fp,
        });
    }

    Ok(RecoveredNode {
        ledger,
        chain_dep,
        tip,
        recovered_fp,
        wal_entries_verified: entries.len() as u64,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;
    use std::path::Path;

    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_core::consensus::praos_state::Nonce;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
    use ade_ledger::block_validity::decode_block;
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::receive::ReceiveState;
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::{CardanoEra, EpochNo, Hash28, SlotNo};
    use tempfile::TempDir;

    use crate::chaindb::{PersistentChainDb, PersistentChainDbOptions, StoredBlock};
    use crate::forward_sync::{pump_block, ForwardSyncState, SnapshotSink};
    use crate::rollback::cadence::SnapshotCadence;
    use crate::rollback::persistent_cache::PersistentSnapshotCache;
    use crate::wal::file_wal_store::FileWalStore;

    const EPOCH_576: EpochNo = EpochNo(576);
    const EPOCH_577_START: u64 = 163_900_800;
    const MAINNET_EPOCH_LENGTH: u64 = 432_000;
    const ANCHOR_FP: Hash32 = Hash32([0xA0; 32]);

    fn schedule() -> EraSchedule {
        let start_576 = EPOCH_577_START - MAINNET_EPOCH_LENGTH;
        EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![EraSummary {
                era: CardanoEra::Conway,
                start_slot: SlotNo(start_576),
                start_epoch: EPOCH_576,
                slot_length_ms: 1_000,
                epoch_length_slots: MAINNET_EPOCH_LENGTH as u32,
                safe_zone_slots: MAINNET_EPOCH_LENGTH as u32,
            }],
        )
        .expect("schedule")
    }

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
        (c, PoolDistrView::new(EPOCH_576, total, asc, pools))
    }

    fn fresh_sync_state(eta0: [u8; 32]) -> ForwardSyncState {
        let mut ledger = LedgerState::new(CardanoEra::Conway);
        ledger.epoch_state.epoch = EPOCH_576;
        let mut chain_dep = PraosChainDepState::empty();
        chain_dep.epoch_nonce = Nonce(Hash32(eta0));
        chain_dep.evolving_nonce = Nonce(Hash32(eta0));
        ForwardSyncState::new(
            ReceiveState::new(ledger, chain_dep),
            ANCHOR_FP,
            SnapshotCadence::DEFAULT,
        )
    }

    fn pick_lightest(c: &ConwayValidityCorpus) -> Vec<u8> {
        let idx = (0..c.blocks.len())
            .min_by_key(|&i| {
                let env = decode_block_envelope(&c.blocks[i]).expect("env");
                env.block_end - env.block_start
            })
            .expect("non-empty");
        c.blocks[idx].clone()
    }

    fn open_chaindb(dir: &Path) -> PersistentChainDb {
        PersistentChainDb::open(PersistentChainDbOptions::at(dir.join("chaindb")))
            .expect("open chaindb")
    }

    /// Capture a checkpoint snapshot for the current sync state at
    /// `slot`. Mirrors the pump's `CommitCheckpoint` effect but driven
    /// explicitly so the crash tests control exactly when a checkpoint
    /// becomes durable.
    fn capture_checkpoint(db: &PersistentChainDb, state: &ForwardSyncState, slot: SlotNo) {
        PersistentSnapshotCache::new(db)
            .capture(slot, &state.receive.ledger, &state.receive.chain_dep)
            .expect("capture checkpoint");
    }

    /// Drive a clean import+sync+admit+checkpoint cycle over a single
    /// corpus block against persistent stores, returning the recovered
    /// fingerprint a clean restart would observe.
    ///
    /// `anchor_slot` is the genesis/import checkpoint slot (the
    /// "import" phase). The block is admitted on top of it.
    fn run_clean(dir: &Path, eta0: [u8; 32], block: &[u8]) -> Hash32 {
        let (_c, view) = corpus_view();
        let sched = schedule();
        let db = open_chaindb(dir);
        let mut wal = FileWalStore::open(dir.join("wal")).expect("open wal");
        let mut state = fresh_sync_state(eta0);

        // Import phase: checkpoint the genesis-equivalent initial state
        // at the pre-block slot so warm-start has a starting point.
        let decoded = decode_block(block).expect("decode");
        let import_slot = SlotNo(decoded.header_input.slot.0 - 1);
        capture_checkpoint(&db, &state, import_slot);

        // Sync + admit phase: pump the block (stores preserved bytes,
        // appends WAL, advances tip).
        let sink = NoSink;
        pump_block(&mut state, &db, &mut wal, &sink, block, &sched, &view)
            .expect("pump");

        // Checkpoint phase: capture a snapshot at the admitted tip.
        capture_checkpoint(&db, &state, decoded.header_input.slot);

        // Recover (clean): warm-start materializes the tip state.
        let recovered =
            recover_node_state(&db, &db, &wal, &ANCHOR_FP, &sched, &view, None)
                .expect("clean recover");
        recovered.recovered_fp
    }

    struct NoSink;
    impl SnapshotSink for NoSink {
        fn put_checkpoint(&self, _slot: SlotNo) -> Result<(), ChainDbError> {
            Ok(())
        }
    }

    /// Drive import+sync+admit into persistent stores but DO NOT write
    /// the final checkpoint, then truncate/skip per the crash phase.
    /// Returns the recovered fingerprint after restart.
    fn run_crashed_recover(
        dir: &Path,
        eta0: [u8; 32],
        block: &[u8],
        crash: Crash,
    ) -> Result<Hash32, NodeRecoveryError> {
        let (_c, view) = corpus_view();
        let sched = schedule();
        let decoded = decode_block(block).expect("decode");
        let import_slot = SlotNo(decoded.header_input.slot.0 - 1);

        {
            let db = open_chaindb(dir);
            let mut wal = FileWalStore::open(dir.join("wal")).expect("open wal");
            let mut state = fresh_sync_state(eta0);

            // Import phase always lands a checkpoint at the pre-block
            // slot (this is the verified anchor's materialized state).
            capture_checkpoint(&db, &state, import_slot);

            match crash {
                Crash::AtImport => {
                    // Crash immediately after the import checkpoint, before
                    // any block sync. Drop handles (simulated power loss).
                }
                Crash::AtSync => {
                    // Sync phase: store preserved bytes + WAL + advance
                    // tip, then crash before the checkpoint-phase snapshot.
                    let sink = NoSink;
                    pump_block(&mut state, &db, &mut wal, &sink, block, &sched, &view)
                        .expect("pump");
                }
                Crash::AtAdmit => {
                    // Same durable writes as AtSync; the distinction is
                    // the crash point is right after admit/tip-advance,
                    // before the checkpoint phase begins.
                    let sink = NoSink;
                    pump_block(&mut state, &db, &mut wal, &sink, block, &sched, &view)
                        .expect("pump");
                }
                Crash::AtCheckpoint => {
                    // Sync + admit durable. The checkpoint phase was
                    // reached but its commit was lost to power loss. A
                    // checkpoint write is atomic (fully committed or
                    // absent, DC-STORE-03); a torn commit leaves NO
                    // snapshot at the tip slot. Recovery therefore falls
                    // back to the prior valid checkpoint (the
                    // import-phase one at slot-1) + forward replay over
                    // the preserved block bytes → identical state.
                    let sink = NoSink;
                    pump_block(&mut state, &db, &mut wal, &sink, block, &sched, &view)
                        .expect("pump");
                    // Confirm the torn checkpoint is absent: no snapshot
                    // at the tip slot exists.
                    assert!(
                        db.get_snapshot(decoded.header_input.slot)
                            .expect("get_snapshot")
                            .is_none(),
                        "torn checkpoint commit leaves the tip-slot snapshot absent",
                    );
                }
            }
            // handles dropped here → simulated crash
        }

        // Restart: reopen the same persistent stores and recover.
        let db = open_chaindb(dir);
        let wal = FileWalStore::open(dir.join("wal")).expect("reopen wal");
        recover_node_state(&db, &db, &wal, &ANCHOR_FP, &sched, &view, None)
            .map(|r| r.recovered_fp)
    }

    #[derive(Clone, Copy)]
    enum Crash {
        AtImport,
        AtSync,
        AtAdmit,
        AtCheckpoint,
    }

    #[test]
    fn recovery_crash_at_phase_import_byte_identical() {
        let (c, _v) = corpus_view();
        let block = pick_lightest(&c);

        // Clean reference: import only (no block synced). The clean run
        // mirrors the crash run's durable state at the import boundary.
        let clean_dir = TempDir::new().expect("tmp");
        let clean_fp = {
            let (_c, view) = corpus_view();
            let sched = schedule();
            let db = open_chaindb(clean_dir.path());
            let wal = FileWalStore::open(clean_dir.path().join("wal")).expect("wal");
            let state = fresh_sync_state(c.epoch_nonce);
            let decoded = decode_block(&block).expect("decode");
            capture_checkpoint(&db, &state, SlotNo(decoded.header_input.slot.0 - 1));
            recover_node_state(&db, &db, &wal, &ANCHOR_FP, &sched, &view, None)
                .expect("clean")
                .recovered_fp
        };

        let crash_dir = TempDir::new().expect("tmp");
        let recovered =
            run_crashed_recover(crash_dir.path(), c.epoch_nonce, &block, Crash::AtImport)
                .expect("recover");
        assert_eq!(recovered, clean_fp, "import-phase crash recovers identically");
    }

    #[test]
    fn recovery_crash_at_phase_sync_byte_identical() {
        let (c, _v) = corpus_view();
        let block = pick_lightest(&c);

        let clean_dir = TempDir::new().expect("tmp");
        let clean_fp = run_clean(clean_dir.path(), c.epoch_nonce, &block);

        let crash_dir = TempDir::new().expect("tmp");
        let recovered =
            run_crashed_recover(crash_dir.path(), c.epoch_nonce, &block, Crash::AtSync)
                .expect("recover");
        assert_eq!(recovered, clean_fp, "sync-phase crash recovers identically");
    }

    #[test]
    fn recovery_crash_at_phase_admit_byte_identical() {
        let (c, _v) = corpus_view();
        let block = pick_lightest(&c);

        let clean_dir = TempDir::new().expect("tmp");
        let clean_fp = run_clean(clean_dir.path(), c.epoch_nonce, &block);

        let crash_dir = TempDir::new().expect("tmp");
        let recovered =
            run_crashed_recover(crash_dir.path(), c.epoch_nonce, &block, Crash::AtAdmit)
                .expect("recover");
        assert_eq!(recovered, clean_fp, "admit-phase crash recovers identically");
    }

    #[test]
    fn recovery_crash_at_phase_checkpoint_byte_identical() {
        let (c, _v) = corpus_view();
        let block = pick_lightest(&c);

        let clean_dir = TempDir::new().expect("tmp");
        let clean_fp = run_clean(clean_dir.path(), c.epoch_nonce, &block);

        // Crash mid-checkpoint: a partially-written checkpoint at the
        // tip is treated as absent (DC-STORE-03). Recovery falls back
        // to the prior valid checkpoint + forward replay → identical fp.
        let crash_dir = TempDir::new().expect("tmp");
        let recovered = run_crashed_recover(
            crash_dir.path(),
            c.epoch_nonce,
            &block,
            Crash::AtCheckpoint,
        )
        .expect("recover");
        assert_eq!(
            recovered, clean_fp,
            "partial checkpoint treated as absent; falls back to prior valid (DC-STORE-03)"
        );
    }

    #[test]
    fn recovery_fails_fast_on_missing_block_bytes() {
        // A WAL entry whose preserved block bytes are absent from the
        // chaindb must fail closed (BlockBytesMissing), never silently
        // partial-recover.
        let (c, view) = corpus_view();
        let sched = schedule();
        let block = pick_lightest(&c);
        let decoded = decode_block(&block).expect("decode");

        let dir = TempDir::new().expect("tmp");
        let db = open_chaindb(dir.path());
        let mut wal = FileWalStore::open(dir.path().join("wal")).expect("wal");
        let mut state = fresh_sync_state(c.epoch_nonce);
        capture_checkpoint(&db, &state, SlotNo(decoded.header_input.slot.0 - 1));

        // Append a WAL entry whose block bytes are NOT stored in the db.
        let post_fp = {
            let sink = NoSink;
            // Pump to compute the post-fp + entry, but then delete the
            // block bytes to simulate a torn preserved-byte write.
            pump_block(&mut state, &db, &mut wal, &sink, &block, &sched, &view)
                .expect("pump");
            fingerprint(&state.receive.ledger).combined
        };
        let _ = post_fp;
        // Roll back the chaindb to drop the block bytes (torn write)
        // while the WAL entry referencing them remains.
        db.rollback_to_slot(SlotNo(decoded.header_input.slot.0 - 1))
            .expect("rollback drops block bytes");

        let err = recover_node_state(&db, &db, &wal, &ANCHOR_FP, &sched, &view, None)
            .expect_err("must fail closed");
        match err {
            NodeRecoveryError::Wal(WalError::BlockBytesMissing { .. }) => {}
            other => panic!("expected BlockBytesMissing, got {other:?}"),
        }
    }

    #[test]
    fn recovery_torn_put_block_before_wal_append_drops_orphan() {
        // S2's pump applies `StoreBlockBytes` before `AppendWal`. A
        // crash between them leaves an orphan block durable in the
        // chaindb but absent from the WAL. Recovery must reconcile the
        // chaindb to the WAL tail — the orphan is dropped, the recovered
        // state equals the WAL-tail state, byte-identical to a clean run
        // that never stored the orphan.
        let (c, _v) = corpus_view();
        let block = pick_lightest(&c);
        let decoded = decode_block(&block).expect("decode");
        let admitted_slot = decoded.header_input.slot;

        // Clean reference: import + admit ONE block (the orphan was
        // never stored).
        let clean_dir = TempDir::new().expect("tmp");
        let clean_fp = run_clean(clean_dir.path(), c.epoch_nonce, &block);

        // Torn run: same clean import + admit, then store ONE more
        // block's bytes via `put_block` (advancing the chaindb tip)
        // WITHOUT appending its WAL entry — exactly the torn-crash
        // window between `StoreBlockBytes` and `AppendWal`.
        let torn_dir = TempDir::new().expect("tmp");
        {
            let (_c, view) = corpus_view();
            let sched = schedule();
            let db = open_chaindb(torn_dir.path());
            let mut wal = FileWalStore::open(torn_dir.path().join("wal"))
                .expect("open wal");
            let mut state = fresh_sync_state(c.epoch_nonce);

            let import_slot = SlotNo(admitted_slot.0 - 1);
            capture_checkpoint(&db, &state, import_slot);
            let sink = NoSink;
            pump_block(&mut state, &db, &mut wal, &sink, &block, &sched, &view)
                .expect("pump");
            capture_checkpoint(&db, &state, admitted_slot);

            // Orphan: bytes durable at a slot strictly beyond the WAL
            // tail, no WAL entry. Bytes are never decoded by recovery
            // (no WAL entry references the hash), so synthetic content
            // suffices to exercise the reconciliation.
            let orphan = StoredBlock {
                hash: Hash32([0xEE; 32]),
                slot: SlotNo(admitted_slot.0 + 1),
                bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
            };
            db.put_block(&orphan).expect("store orphan bytes");
            assert_eq!(
                db.tip().expect("tip").map(|t| t.slot),
                Some(orphan.slot),
                "orphan advanced the chaindb tip beyond the WAL tail",
            );
            // handles dropped → simulated crash
        }

        let db = open_chaindb(torn_dir.path());
        let wal = FileWalStore::open(torn_dir.path().join("wal"))
            .expect("reopen wal");
        let recovered =
            recover_node_state(&db, &db, &wal, &ANCHOR_FP, &schedule(), &corpus_view().1, None)
                .expect("recover drops orphan");

        // (a) Recovered fp == WAL-tail fp == clean-run fp: the orphan
        //     beyond the WAL tail was NOT incorporated.
        assert_eq!(
            recovered.recovered_fp, clean_fp,
            "orphan beyond WAL tail not incorporated; recovered == clean",
        );
        // (b) The orphan is gone; the chaindb tip is the Nth (admitted)
        //     block.
        assert_eq!(
            recovered.tip.as_ref().map(|t| t.slot),
            Some(admitted_slot),
            "chaindb reconciled to the WAL tail (admitted block is the tip)",
        );
        assert!(
            db.get_block_by_hash(&Hash32([0xEE; 32]))
                .expect("get orphan")
                .is_none(),
            "orphan block bytes dropped by reconciliation",
        );
    }
}
