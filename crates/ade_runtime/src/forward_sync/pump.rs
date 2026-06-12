// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED forward-sync pump (PHASE4-N-Y S2).
//!
//! Drives the GREEN [`forward_sync_step`] reducer over a fetched
//! block sequence and applies the resulting [`SyncEffect`] plan in
//! order against the persistent stores:
//!   - `StoreBlockBytes` → `ChainDb::put_block` (preserved wire bytes)
//!   - `AppendWal`        → `FileWalStore::append` (Ade-canonical WAL)
//!   - `CommitCheckpoint` → snapshot writer (cadence)
//!   - `AdvanceTip`       → acknowledged only after the preceding
//!                          durability effects returned Ok (DC-SYNC-01)
//!
//! The pump is RED: it owns the redb `ChainDb` + the on-disk WAL. It
//! does NOT decide ordering — that is the GREEN reducer's closed
//! `AdmitPlan`. The pump's only ordering duty is to apply the plan's
//! effects front-to-back and refuse to issue the tip write before the
//! durability writes have returned Ok.
//!
//! A live socket is not required: the pump consumes a pre-fetched
//! ordered block sequence (loopback / in-memory feed), exactly the
//! `follow` / `mux_pump` driver shape.

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_ledger::block_validity::decode_block;
use ade_ledger::receive::{ReceiveError, ReceiveEvent, TipPoint};
use ade_ledger::wal::{WalError, WalStore};
use ade_types::{Hash32, SlotNo};

use crate::chaindb::ChainDb;
use crate::forward_sync::reducer::{
    forward_sync_step, ForwardSyncState, SyncEffect,
};
use crate::receive::ChainDbWriter;

/// Closed pump-failure surface.
#[derive(Debug)]
pub enum PumpError {
    /// The GREEN admit reducer rejected a block (chokepoint).
    Receive(ReceiveError),
    /// A WAL append failed; the pump halts before any tip advance.
    Wal(WalError),
    /// A preserved-byte store write failed; the pump halts before any
    /// tip advance.
    Store(crate::chaindb::ChainDbError),
    /// A snapshot/checkpoint write failed.
    Checkpoint(crate::chaindb::ChainDbError),
    /// DC-SYNC-01 guard tripped: an `AdvanceTip` effect was reached
    /// before its block's durability effects had been applied this
    /// step. Unreachable given the GREEN plan ordering; the guard
    /// fails closed rather than silently advancing.
    TipBeforeDurable,
}

/// The tip the pump has durably advanced to, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PumpTip {
    pub slot: SlotNo,
    pub hash: Hash32,
    /// The admitted block's VALIDATED header `prev_hash` (parent linkage), from
    /// the same `decode_block` the reducer consumed — NEVER peer-supplied. The
    /// genesis predecessor is carried as the all-zero hash (a from-genesis first
    /// block; never seen on a post-switch follow descendant). Capture-only
    /// evidence fidelity for the post-switch branch-continuity verdict
    /// (PHASE4-N-AO S10, DC-EVIDENCE-05); not read by any authority path.
    pub prev_hash: Hash32,
}

/// Apply one fetched block (its full era-tagged envelope) through the
/// reducer + ordered effect application.
///
/// `db` is the persistent preserved-byte store; `wal` the on-disk WAL;
/// `snapshots` the checkpoint sink. The caller supplies the cached
/// header first (RollForward) then the body (BlockDelivered) — the
/// `follow` pattern — but for the in-memory feed both events are
/// derived from the same envelope bytes here.
pub fn pump_block<D, S>(
    state: &mut ForwardSyncState,
    db: &D,
    wal: &mut dyn WalStore,
    snapshots: &S,
    block_bytes: &[u8],
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<Option<PumpTip>, PumpError>
where
    D: ChainDb,
    S: SnapshotSink,
{
    let decoded = decode_block(block_bytes)
        .map_err(|e| PumpError::Receive(ReceiveError::Validity(e)))?;

    // PHASE4-N-AE.F (DC-NODE-16): receive idempotency. A block already durably
    // present byte-identically (same hash at the same slot) in the ChainDb is an
    // idempotent no-op — a peer re-announcing a block Ade already applied (e.g. its
    // own forged tip echoed back over the follow link after the peer adopted it).
    // Gated on HASH equality vs the durable store (`get_block_by_hash` is
    // hash-exact): a DIFFERENT block at/before the last-applied slot returns None
    // here and falls through to the unchanged BLUE chokepoint reducer below, which
    // fails closed (SlotBeforeLastApplied / BlockNoOutOfOrder). The no-op runs no
    // reducer step, appends nothing to the WAL, and writes nothing — the post-state
    // is identical and replay-equivalent (the WAL never records the re-announce).
    if let Some(stored) = db
        .get_block_by_hash(&decoded.block_hash)
        .map_err(PumpError::Store)?
    {
        if stored.slot == decoded.header_input.slot {
            return Ok(None);
        }
    }

    // Header cache step (RollForward), then body (BlockDelivered) —
    // both events feed the same BLUE chokepoint reducer.
    let cache_ev = ReceiveEvent::RollForward {
        slot: decoded.header_input.slot,
        hash: decoded.block_hash.clone(),
        header_bytes: block_bytes.to_vec(),
        tip: TipPoint {
            slot: SlotNo(0),
            hash: Hash32([0; 32]),
            block_no: 0,
        },
    };

    {
        let mut writer = ChainDbWriter::new(db);
        forward_sync_step(state, cache_ev, &mut writer, era_schedule, ledger_view)
            .map_err(PumpError::Receive)?;
    }

    let deliver_ev = ReceiveEvent::BlockDelivered {
        block_bytes: block_bytes.to_vec(),
    };
    let plan = {
        let mut writer = ChainDbWriter::new(db);
        forward_sync_step(state, deliver_ev, &mut writer, era_schedule, ledger_view)
            .map_err(PumpError::Receive)?
    };

    // The admitted block's validated parent link (S10 / DC-EVIDENCE-05). The
    // genesis predecessor (CBOR null) is carried as the all-zero hash; a
    // post-switch follow descendant always has a real `Block(parent)`.
    let prev_hash = decoded
        .prev_hash
        .block_hash()
        .cloned()
        .unwrap_or(Hash32([0; 32]));
    apply_plan(db, wal, snapshots, plan.into_effects(), prev_hash)
}

/// Apply an ordered effect plan. The two durability effects must be
/// acknowledged Ok before any `AdvanceTip` is issued. The GREEN plan
/// already orders them; the pump enforces it at the boundary.
fn apply_plan<D, S>(
    db: &D,
    wal: &mut dyn WalStore,
    snapshots: &S,
    effects: Vec<SyncEffect>,
    prev_hash: Hash32,
) -> Result<Option<PumpTip>, PumpError>
where
    D: ChainDb,
    S: SnapshotSink,
{
    let mut bytes_durable = false;
    let mut wal_durable = false;
    let mut tip: Option<PumpTip> = None;

    for effect in effects {
        match effect {
            SyncEffect::StoreBlockBytes(stored) => {
                db.put_block(&stored).map_err(PumpError::Store)?;
                bytes_durable = true;
            }
            SyncEffect::AppendWal(entry) => {
                wal.append(entry).map_err(PumpError::Wal)?;
                wal_durable = true;
            }
            SyncEffect::CommitCheckpoint { slot } => {
                snapshots
                    .put_checkpoint(slot)
                    .map_err(PumpError::Checkpoint)?;
            }
            SyncEffect::AdvanceTip { slot, hash } => {
                if !(bytes_durable && wal_durable) {
                    return Err(PumpError::TipBeforeDurable);
                }
                tip = Some(PumpTip {
                    slot,
                    hash,
                    prev_hash: prev_hash.clone(),
                });
            }
        }
    }
    Ok(tip)
}

/// Checkpoint sink the pump writes cadence snapshots to. Kept minimal
/// (slot marker) — the snapshot byte payload + atomic write live in
/// the existing `rollback::snapshot_writer` and are exercised in S3.
pub trait SnapshotSink {
    fn put_checkpoint(&self, slot: SlotNo) -> Result<(), crate::chaindb::ChainDbError>;
}

/// A no-op checkpoint sink: forward-sync S2 only proves the durable
/// store + WAL ordering; the snapshot byte payload is S3 scope.
pub struct NoCheckpointSink;

impl SnapshotSink for NoCheckpointSink {
    fn put_checkpoint(&self, _slot: SlotNo) -> Result<(), crate::chaindb::ChainDbError> {
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_core::consensus::praos_state::PraosChainDepState;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::receive::ReceiveState;
    use ade_ledger::state::LedgerState;
    use ade_ledger::wal::WalEntry;
    use ade_types::{CardanoEra, EpochNo, Hash28};

    use crate::chaindb::InMemoryChainDb;
    use crate::rollback::cadence::SnapshotCadence;

    const EPOCH_576: EpochNo = EpochNo(576);
    const EPOCH_577_START: u64 = 163_900_800;
    const MAINNET_EPOCH_LENGTH: u64 = 432_000;

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

    use ade_testkit::validity::ConwayValidityCorpus;

    fn fresh_state(eta0: [u8; 32]) -> ForwardSyncState {
        let mut ledger = LedgerState::new(CardanoEra::Conway);
        ledger.epoch_state.epoch = EPOCH_576;
        let mut chain_dep = PraosChainDepState::empty();
        chain_dep.epoch_nonce = Nonce(Hash32(eta0));
        chain_dep.evolving_nonce = Nonce(Hash32(eta0));
        ForwardSyncState::new(
            ReceiveState::new(ledger, chain_dep),
            Hash32([0xA0; 32]),
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

    /// In-memory WAL store for the pump tests (records append order).
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

    #[test]
    fn pump_block_stores_bytes_and_wal_then_advances_tip() {
        let (c, view) = corpus_view();
        let sched = schedule();
        let bytes = pick_lightest(&c);

        let mut state = fresh_state(c.epoch_nonce);
        let db = InMemoryChainDb::new();
        let mut wal = VecWal::default();

        let tip = pump_block(
            &mut state,
            &db,
            &mut wal,
            &NoCheckpointSink,
            &bytes,
            &sched,
            &view,
        )
        .expect("pump")
        .expect("tip advanced");

        // Block durably stored under its (slot, hash).
        let stored = db
            .get_block_by_hash(&tip.hash)
            .expect("get")
            .expect("present");
        assert_eq!(stored.bytes, bytes, "preserved wire bytes round-trip");
        // WAL appended exactly one entry.
        assert_eq!(wal.entries.len(), 1);
        // ChainDb tip matches the advanced tip.
        let chain_tip = db.tip().expect("tip").expect("non-empty");
        assert_eq!(chain_tip.slot, tip.slot);
        assert_eq!(chain_tip.hash, tip.hash);
    }

    #[test]
    fn pump_block_reannounced_block_is_idempotent_noop() {
        // PHASE4-N-AE.F (DC-NODE-16, CE-F1): re-pumping a block Ade already durably
        // holds (same hash) is an idempotent no-op -- Ok(None) -- leaving the ChainDb
        // tip, the BLUE chain-dep, and the WAL length unchanged. This is the
        // post-CE-A5 echo (the relay re-announcing Ade's own adopted tip).
        let (c, view) = corpus_view();
        let sched = schedule();
        let bytes = pick_lightest(&c);
        let mut state = fresh_state(c.epoch_nonce);
        let db = InMemoryChainDb::new();
        let mut wal = VecWal::default();

        // First pump applies + advances the tip.
        let tip = pump_block(
            &mut state, &db, &mut wal, &NoCheckpointSink, &bytes, &sched, &view,
        )
        .expect("first pump")
        .expect("tip advanced");
        let last_slot_after = state.receive.chain_dep.last_slot;
        let wal_len_after = wal.entries.len();
        let db_tip_after = db.tip().expect("tip").expect("present");

        // Re-pump the SAME bytes (the echo): idempotent no-op.
        let outcome = pump_block(
            &mut state, &db, &mut wal, &NoCheckpointSink, &bytes, &sched, &view,
        )
        .expect("re-pump of an already-have block must NOT be an error");
        assert_eq!(
            outcome, None,
            "a re-announced already-have block advances no tip (Ok(None))"
        );

        // Nothing changed by the no-op.
        assert_eq!(
            state.receive.chain_dep.last_slot, last_slot_after,
            "chain-dep last_slot unchanged by the re-announce"
        );
        assert_eq!(
            wal.entries.len(),
            wal_len_after,
            "no WAL append on the no-op (replay-equivalent)"
        );
        let db_tip_now = db.tip().expect("tip").expect("present");
        assert_eq!(db_tip_now.slot, db_tip_after.slot, "ChainDb tip slot unchanged");
        assert_eq!(db_tip_now.hash, db_tip_after.hash, "ChainDb tip hash unchanged");
        assert_eq!(db_tip_now.hash, tip.hash, "tip is still the originally-applied block");
    }

    #[test]
    fn pump_block_different_block_at_or_before_tip_still_fails_closed() {
        // PHASE4-N-AE.F (AE-F-INV-2, CE-F2): the gate is HASH-keyed, so a DIFFERENT
        // block (different hash) at/before the last-applied slot is NOT short-circuited
        // -- it reaches the unchanged BLUE authority and fails closed.
        let (c, view) = corpus_view();
        let sched = schedule();
        // The two lowest-slot distinct corpus blocks (both within the forecast horizon).
        let mut by_slot: Vec<(SlotNo, Vec<u8>)> = c
            .blocks
            .iter()
            .map(|b| {
                let d = decode_block(b).expect("decode corpus block");
                (d.header_input.slot, b.clone())
            })
            .collect();
        by_slot.sort_by_key(|(s, _)| s.0);
        by_slot.dedup_by_key(|(s, _)| s.0);
        assert!(by_slot.len() >= 2, "need >= 2 distinct-slot corpus blocks");
        let (lo_slot, lo_bytes) = by_slot[0].clone();
        let (hi_slot, hi_bytes) = by_slot[1].clone();
        assert!(hi_slot.0 > lo_slot.0, "two distinct slots");

        let mut state = fresh_state(c.epoch_nonce);
        let db = InMemoryChainDb::new();
        let mut wal = VecWal::default();

        // Apply the higher-slot block -> last_slot = hi_slot.
        pump_block(
            &mut state, &db, &mut wal, &NoCheckpointSink, &hi_bytes, &sched, &view,
        )
        .expect("higher-slot block applies")
        .expect("tip advanced");

        // Pump the lower-slot block (different hash, slot < last) -> fail closed.
        let err = pump_block(
            &mut state, &db, &mut wal, &NoCheckpointSink, &lo_bytes, &sched, &view,
        )
        .expect_err("a different block at/before the last-applied slot must fail closed");
        let s = format!("{err:?}");
        assert!(
            s.contains("SlotBeforeLastApplied") || s.contains("BlockNoOutOfOrder"),
            "expected a monotone-violation header rejection from the BLUE authority, got: {s}"
        );
    }
}
