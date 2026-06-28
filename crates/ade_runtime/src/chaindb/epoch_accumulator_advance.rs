// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! LIVE-LEDGER-EPOCH-TRANSITION S2 (DC-EPOCH-20) — the within-epoch accumulator ADVANCER.
//!
//! The GREEN orchestration seam between the durable [`EpochAccumulatorStore`] and the BLUE
//! `ade_ledger::epoch_accumulator::apply_selected_block` contract: for one durable selected-chain block it
//! loads the current accumulator, applies the block, and advances the store — or records an OBSERVE-ONLY
//! STALL. It is the analogue of `reduced_window_driver::advance_reduced_checkpoint_over_chaindb` for the
//! non-UTxO accumulator. TCB: RED glue (it drives the RED store); the authority transition it invokes is
//! BLUE and the byte-decisions are the store's / the contract's, never reinvented here.
//!
//! S2 scope — the WITHIN-EPOCH half only. The advancer NEVER supplies `boundary_mark` (it is forced to
//! `None`), so a block that crosses an epoch boundary fail-closes inside the contract
//! (`MissingBoundaryStake`) and surfaces here as a STALL — the boundary transition (POOLREAP, the boundary
//! reward, the KeyHash withdrawal projection) is structurally excluded until S3 supplies the mark + the
//! byte-exact gate. The exclusion is enforced by this type: a caller cannot hand the advancer a mark.
//!
//! Observe-only stall (PO-6): in S2 the accumulator is NOT yet the consensus/leadership authority (S4
//! flips it), so an apply failure — a boundary the mark is withheld for, or a byte-uncertain block — does
//! NOT halt the follow. It returns [`AdvanceOutcome::Stalled`]: the store is left at its last good slot, so
//! `LAST_SLOT < wal_tail` becomes the durable stall signal and the store's readiness gate fail-closes any
//! authoritative read until S3 resolves it. A genuine STORE fault (durability I/O) is distinct — it is an
//! [`AdvanceError`], a real error the caller must not paper over.

use ade_core::consensus::era_schedule::EraSchedule;
use ade_ledger::epoch_accumulator::{apply_selected_block, SelectedBlockCtx};
use ade_types::{CardanoEra, EpochNo, PoolId, SlotNo};

use super::epoch_accumulator_store::{EpochAccumulatorStore, EpochAccumulatorStoreError};
use super::error::ChainDbError;
use super::ChainDb;

/// The canonical, deterministic per-block geometry the advancer needs — derived ONLY from the decoded
/// block + the durable selected-chain context at the admit site (the verified header issuer, the block's
/// slot, its era, and its epoch from the era schedule). NEVER a peer handle, CLI, or wall-clock, and —
/// structurally — NEVER a boundary mark (S2 forces `boundary_mark = None`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithinEpochCtx {
    /// The block's era (must match the decoded envelope; the contract re-checks).
    pub era: CardanoEra,
    /// The block's epoch (live: `era_schedule.locate(slot).epoch`). A value `> acc.epoch` means a boundary
    /// crossing, which S2 stalls.
    pub block_epoch: EpochNo,
    /// The block's slot — must strictly exceed the accumulator's last advanced slot, else it is an
    /// already-applied re-announce / replay (idempotent no-op).
    pub block_slot: SlotNo,
    /// The block's VERIFIED issuer pool (`blake2b_224(header.issuer_vkey)`), for `block_production[issuer]`.
    pub issuer_pool: PoolId,
}

/// The outcome of advancing the accumulator over one block. `Advanced` / `AlreadyApplied` / `Stalled` are
/// all NON-error outcomes — the follow continues regardless (the accumulator is observe-only in S2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdvanceOutcome {
    /// The accumulator applied this within-epoch block and the store advanced to `slot` (epoch `epoch`).
    Advanced { slot: SlotNo, epoch: EpochNo },
    /// The block is at or before the accumulator's tip (a re-announce / replay) — no-op.
    AlreadyApplied { slot: SlotNo, last: SlotNo },
    /// OBSERVE-ONLY STALL: `apply_selected_block` fail-closed (a boundary the mark is withheld for, or a
    /// byte-uncertain block). The store is untouched (stays at its last good slot); the follow continues.
    /// `reason` is the contract's structured error rendered for the log — not read by any authority path.
    Stalled { slot: SlotNo, reason: String },
}

/// A REAL fault advancing the accumulator (distinct from an observe-only stall).
#[derive(Debug)]
pub enum AdvanceError {
    /// The store is not sealed — the bootstrap seal must precede any advance.
    Unsealed,
    /// A durable store I/O fault (load / advance) — a genuine durability failure, never swallowed.
    Store(EpochAccumulatorStoreError),
}

/// Advance the durable accumulator over ONE durable selected-chain block (the within-epoch half, S2).
///
/// Loads the current accumulator, idempotently skips an at-or-before-tip block, then applies the block
/// with `boundary_mark = None` (the S2 structural exclusion): on success it advances the store; on a
/// contract fail-close it returns an observe-only [`AdvanceOutcome::Stalled`] and leaves the store
/// untouched. Only a store I/O fault or an unsealed store is an [`AdvanceError`].
pub fn advance_accumulator_over_block(
    store: &EpochAccumulatorStore,
    block_bytes: &[u8],
    ctx: &WithinEpochCtx,
) -> Result<AdvanceOutcome, AdvanceError> {
    let (last_slot, acc) = store
        .load_current()
        .map_err(AdvanceError::Store)?
        .ok_or(AdvanceError::Unsealed)?;

    // Idempotency: a block at or before the accumulator's tip is a re-announce / already-applied replay.
    // (The live admit path also no-ops a byte-identical re-announce before reaching here; this is the
    // accumulator's own backstop so a replayed prefix never double-applies.)
    if ctx.block_slot.0 <= last_slot.0 {
        return Ok(AdvanceOutcome::AlreadyApplied {
            slot: ctx.block_slot,
            last: last_slot,
        });
    }

    let selected_ctx = SelectedBlockCtx {
        era: ctx.era,
        block_epoch: ctx.block_epoch,
        block_slot: ctx.block_slot,
        issuer_pool: ctx.issuer_pool.clone(),
        // S2: the boundary is structurally excluded — a crossing fail-closes MissingBoundaryStake → Stalled.
        boundary_mark: None,
    };

    match apply_selected_block(&acc, block_bytes, &selected_ctx) {
        Ok(next) => {
            store
                .advance(&next, ctx.block_slot)
                .map_err(AdvanceError::Store)?;
            Ok(AdvanceOutcome::Advanced {
                slot: ctx.block_slot,
                epoch: ctx.block_epoch,
            })
        }
        Err(e) => Ok(AdvanceOutcome::Stalled {
            slot: ctx.block_slot,
            reason: format!("{e:?}"),
        }),
    }
}

/// The outcome of reconciling the accumulator over a durable ChainDB prefix (LIVE-LEDGER-EPOCH-
/// TRANSITION S2 / DC-EPOCH-20). Both arms are NON-error: `ReachedTip` walked the whole `(from, to_slot]`
/// prefix; `StalledAt` hit an observe-only boundary stall and STOPPED at the last good within-epoch slot
/// (the store froze there). A genuine fault is the `Err` arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccumulatorChaindbOutcome {
    /// Walked the full prefix; the store now sits at `last_slot` (None only if it never advanced).
    ReachedTip { last_slot: Option<SlotNo> },
    /// An observe-only boundary stall stopped the walk; the store stays at its prior slot.
    StalledAt { slot: SlotNo, reason: String },
}

/// A REAL fault reconciling the accumulator over the ChainDB (never an observe-only stall).
#[derive(Debug)]
pub enum AccumulatorChaindbError {
    /// A durable ChainDB read fault.
    ChainDb(ChainDbError),
    /// A stored block did not decode (it should never have reached durable admit).
    Decode(String),
    /// The era schedule could not place a stored block's slot.
    Locate(String),
    /// A store I/O fault or unsealed store surfaced by the per-block advancer.
    Advance(AdvanceError),
}

/// Reconcile the durable accumulator over the canonical selected chain in `(from, to_slot]`, where
/// `from` resumes at `last_advanced_slot + 1` (or `bootstrap_slot` if the seed has never advanced).
///
/// This is the SINGLE within-epoch fold authority (LIVE-LEDGER-EPOCH-TRANSITION S2): it reads ONLY the
/// durable ChainDB, in stored (admission) order, deriving each block's geometry from canonical data —
/// `block_slot` from the stored slot, `block_epoch` from the era schedule, `era` + verified `issuer_pool`
/// from the decoded header — and folds via [`advance_accumulator_over_block`] (boundary structurally
/// excluded, `boundary_mark = None`). It is idempotent: a re-walk over an already-folded prefix advances
/// nothing. On a boundary it returns [`AccumulatorChaindbOutcome::StalledAt`] and STOPS (observe-only) so
/// the store freezes at the last within-epoch slot — `LAST_SLOT < tip` is the durable stall signal.
///
/// REORG note: this walk only ever moves FORWARD. A rollback (the accumulator already past `to_slot`) is
/// detected and rematerialized by the caller BEFORE this is invoked — reset to the sealed seed, then
/// replay forward through here. There is no inverse mutation; the accumulator codec exposes none.
pub fn advance_accumulator_over_chaindb(
    store: &EpochAccumulatorStore,
    chaindb: &dyn ChainDb,
    era_schedule: &EraSchedule,
    bootstrap_slot: SlotNo,
    to_slot: SlotNo,
) -> Result<AccumulatorChaindbOutcome, AccumulatorChaindbError> {
    let from = store
        .last_advanced_slot()
        .map_err(|e| AccumulatorChaindbError::Advance(AdvanceError::Store(e)))?
        .map(|s| SlotNo(s.0.saturating_add(1)))
        .unwrap_or(bootstrap_slot);
    let iter = chaindb
        .iter_from_slot(from)
        .map_err(AccumulatorChaindbError::ChainDb)?;
    for stored in iter {
        let stored = stored.map_err(AccumulatorChaindbError::ChainDb)?;
        if stored.slot.0 > to_slot.0 {
            break;
        }
        let decoded = ade_ledger::block_validity::decode_block(&stored.bytes)
            .map_err(|e| AccumulatorChaindbError::Decode(format!("{e:?}")))?;
        let block_epoch = era_schedule
            .locate(stored.slot)
            .map_err(|e| AccumulatorChaindbError::Locate(format!("{e:?}")))?
            .epoch;
        let ctx = WithinEpochCtx {
            era: decoded.era,
            block_epoch,
            block_slot: stored.slot,
            issuer_pool: PoolId(decoded.header_input.issuer_pool.clone()),
        };
        match advance_accumulator_over_block(store, &stored.bytes, &ctx)
            .map_err(AccumulatorChaindbError::Advance)?
        {
            AdvanceOutcome::Advanced { .. } | AdvanceOutcome::AlreadyApplied { .. } => {}
            AdvanceOutcome::Stalled { slot, reason } => {
                return Ok(AccumulatorChaindbOutcome::StalledAt { slot, reason });
            }
        }
    }
    let last_slot = store
        .last_advanced_slot()
        .map_err(|e| AccumulatorChaindbError::Advance(AdvanceError::Store(e)))?;
    Ok(AccumulatorChaindbOutcome::ReachedTip { last_slot })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_ledger::epoch_accumulator::EpochAccumulator;
    use ade_types::tx::Coin;
    use ade_types::Hash28;
    use tempfile::TempDir;

    const RAW_CONWAY_BLOCK: &[u8] =
        include_bytes!("../../../ade_node/tests/fixtures/raw_era_block_conway.cbor");

    fn store(tmp: &TempDir) -> EpochAccumulatorStore {
        EpochAccumulatorStore::open(&tmp.path().join("acc.redb")).unwrap()
    }

    fn pool(b: u8) -> PoolId {
        PoolId(Hash28([b; 28]))
    }

    /// The accumulator the real Conway block applies cleanly to at epoch 500 (mirrors the ade_ledger
    /// determinism test's `fresh_conway_acc`: a fresh Conway accumulator at epoch 500 with reserves).
    fn sealed_store_at_epoch_500(tmp: &TempDir, seed_slot: SlotNo) -> EpochAccumulatorStore {
        let mut acc = EpochAccumulator::new(CardanoEra::Conway);
        acc.epoch_state.epoch = EpochNo(500);
        acc.epoch_state.reserves = Coin(1_000_000_000_000_000);
        let s = store(tmp);
        s.seal_bootstrap(&acc, seed_slot).unwrap();
        s
    }

    #[test]
    fn within_epoch_block_advances_the_store() {
        let tmp = TempDir::new().unwrap();
        let s = sealed_store_at_epoch_500(&tmp, SlotNo(42_000_000));
        let ctx = WithinEpochCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(500), // same epoch — within-epoch, no boundary
            block_slot: SlotNo(43_000_000),
            issuer_pool: pool(0x77),
        };
        let outcome = advance_accumulator_over_block(&s, RAW_CONWAY_BLOCK, &ctx).unwrap();
        assert_eq!(
            outcome,
            AdvanceOutcome::Advanced {
                slot: SlotNo(43_000_000),
                epoch: EpochNo(500)
            }
        );
        // The store advanced and the within-epoch effects landed (the issuer's nesBcur incremented).
        let (slot, acc) = s.load_current().unwrap().unwrap();
        assert_eq!(slot, SlotNo(43_000_000));
        assert_eq!(acc.epoch_state.block_production.get(&pool(0x77)), Some(&1));
        assert_eq!(acc.epoch_state.slot, SlotNo(43_000_000));
    }

    #[test]
    fn boundary_crossing_block_stalls_observe_only() {
        let tmp = TempDir::new().unwrap();
        let s = sealed_store_at_epoch_500(&tmp, SlotNo(42_000_000));
        let ctx = WithinEpochCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(501), // a boundary crossing — S2 withholds the mark
            block_slot: SlotNo(43_000_000),
            issuer_pool: pool(0x77),
        };
        let outcome = advance_accumulator_over_block(&s, RAW_CONWAY_BLOCK, &ctx).unwrap();
        match outcome {
            AdvanceOutcome::Stalled { slot, reason } => {
                assert_eq!(slot, SlotNo(43_000_000));
                assert!(
                    reason.contains("MissingBoundaryStake"),
                    "expected the boundary stall reason, got {reason}"
                );
            }
            other => panic!("expected a boundary Stall, got {other:?}"),
        }
        // Observe-only: the store is untouched — LAST_SLOT stays at the seed (the durable stall signal).
        assert_eq!(s.last_advanced_slot().unwrap(), Some(SlotNo(42_000_000)));
    }

    #[test]
    fn at_or_before_tip_is_already_applied() {
        let tmp = TempDir::new().unwrap();
        let s = sealed_store_at_epoch_500(&tmp, SlotNo(43_000_000));
        // A block at the tip slot (≤ last) is an idempotent no-op — never decoded / applied.
        let ctx = WithinEpochCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(500),
            block_slot: SlotNo(43_000_000),
            issuer_pool: pool(0x77),
        };
        let outcome = advance_accumulator_over_block(&s, b"not even a block", &ctx).unwrap();
        assert_eq!(
            outcome,
            AdvanceOutcome::AlreadyApplied {
                slot: SlotNo(43_000_000),
                last: SlotNo(43_000_000)
            }
        );
    }

    #[test]
    fn unsealed_store_is_an_error_not_a_stall() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        let ctx = WithinEpochCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(500),
            block_slot: SlotNo(43_000_000),
            issuer_pool: pool(0x77),
        };
        let err = advance_accumulator_over_block(&s, RAW_CONWAY_BLOCK, &ctx).unwrap_err();
        assert!(matches!(err, AdvanceError::Unsealed));
    }

    /// An era schedule with 86_000-slot epochs from genesis: `locate(86_000 * E).epoch == E`, so a
    /// stored block at slot 43_000_000 places in epoch 500 (within-epoch vs the sealed store) and one at
    /// 43_086_000 places in epoch 501 (a boundary crossing).
    fn schedule_86k() -> EraSchedule {
        use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
        use ade_types::Hash32;
        EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![EraSummary {
                randomness_stabilisation_window_slots: None,
                era: CardanoEra::Conway,
                start_slot: SlotNo(0),
                start_epoch: EpochNo(0),
                slot_length_ms: 1_000,
                epoch_length_slots: 86_000,
                safe_zone_slots: 25_800,
            }],
        )
        .expect("schedule")
    }

    fn put_raw(db: &crate::chaindb::InMemoryChainDb, slot: u64) {
        use crate::chaindb::types::StoredBlock;
        use crate::chaindb::ChainDb;
        use ade_types::Hash32;
        db.put_block(&StoredBlock {
            hash: Hash32([(slot & 0xff) as u8; 32]),
            slot: SlotNo(slot),
            bytes: RAW_CONWAY_BLOCK.to_vec(),
        })
        .unwrap();
    }

    /// LIVE-LEDGER-EPOCH-TRANSITION S2 (DC-EPOCH-20 / PO-4): warm-start catch-up. A sealed accumulator
    /// behind the durable tip folds the canonical prefix forward to the tip, in ChainDB (admission) order.
    #[test]
    fn over_chaindb_folds_durable_prefix_to_tip() {
        use crate::chaindb::InMemoryChainDb;
        let tmp = TempDir::new().unwrap();
        let s = sealed_store_at_epoch_500(&tmp, SlotNo(42_000_000));
        let db = InMemoryChainDb::new();
        put_raw(&db, 43_000_000); // epoch 500 (86_000 * 500) -> within-epoch
        let outcome = advance_accumulator_over_chaindb(
            &s,
            &db,
            &schedule_86k(),
            SlotNo(42_000_000),
            SlotNo(43_500_000),
        )
        .unwrap();
        assert_eq!(
            outcome,
            AccumulatorChaindbOutcome::ReachedTip {
                last_slot: Some(SlotNo(43_000_000))
            }
        );
        assert_eq!(s.last_advanced_slot().unwrap(), Some(SlotNo(43_000_000)));
    }

    /// A re-walk over an already-folded prefix advances nothing (idempotent resume — replay-safe).
    #[test]
    fn over_chaindb_rewalk_is_idempotent() {
        use crate::chaindb::InMemoryChainDb;
        let tmp = TempDir::new().unwrap();
        let s = sealed_store_at_epoch_500(&tmp, SlotNo(42_000_000));
        let db = InMemoryChainDb::new();
        put_raw(&db, 43_000_000);
        let sched = schedule_86k();
        advance_accumulator_over_chaindb(&s, &db, &sched, SlotNo(42_000_000), SlotNo(43_500_000))
            .unwrap();
        let outcome = advance_accumulator_over_chaindb(
            &s,
            &db,
            &sched,
            SlotNo(42_000_000),
            SlotNo(43_500_000),
        )
        .unwrap();
        assert_eq!(
            outcome,
            AccumulatorChaindbOutcome::ReachedTip {
                last_slot: Some(SlotNo(43_000_000))
            }
        );
        assert_eq!(s.last_advanced_slot().unwrap(), Some(SlotNo(43_000_000)));
    }

    /// A boundary block (epoch + 1) STOPS the walk observe-only — the store freezes at the last
    /// within-epoch slot (`LAST_SLOT < tip` is the durable stall signal), never folding past it.
    #[test]
    fn over_chaindb_stops_at_boundary_observe_only() {
        use crate::chaindb::InMemoryChainDb;
        let tmp = TempDir::new().unwrap();
        let s = sealed_store_at_epoch_500(&tmp, SlotNo(42_000_000));
        let db = InMemoryChainDb::new();
        put_raw(&db, 43_086_000); // 86_000 * 501 -> epoch 501, a boundary crossing
        let outcome = advance_accumulator_over_chaindb(
            &s,
            &db,
            &schedule_86k(),
            SlotNo(42_000_000),
            SlotNo(43_200_000),
        )
        .unwrap();
        match outcome {
            AccumulatorChaindbOutcome::StalledAt { slot, reason } => {
                assert_eq!(slot, SlotNo(43_086_000));
                assert!(
                    reason.contains("MissingBoundaryStake"),
                    "expected the boundary stall reason, got {reason}"
                );
            }
            other => panic!("expected a boundary StalledAt, got {other:?}"),
        }
        // Observe-only: the store stayed at the seed (it never folded the boundary block).
        assert_eq!(s.last_advanced_slot().unwrap(), Some(SlotNo(42_000_000)));
    }

    /// Reorg recovery component (DC-EPOCH-20): reset to the sealed seed, then replay forward
    /// re-materializes the SAME tip — no ad hoc inverse mutation (the accumulator codec exposes none).
    #[test]
    fn reset_then_rewalk_rematerializes() {
        use crate::chaindb::InMemoryChainDb;
        let tmp = TempDir::new().unwrap();
        let s = sealed_store_at_epoch_500(&tmp, SlotNo(42_000_000));
        let db = InMemoryChainDb::new();
        put_raw(&db, 43_000_000);
        let sched = schedule_86k();
        advance_accumulator_over_chaindb(&s, &db, &sched, SlotNo(42_000_000), SlotNo(43_500_000))
            .unwrap();
        assert_eq!(s.last_advanced_slot().unwrap(), Some(SlotNo(43_000_000)));
        // Rematerialize to the sealed seed, then replay the canonical chain forward.
        s.reset_to_bootstrap().unwrap();
        assert_eq!(
            s.last_advanced_slot().unwrap(),
            Some(SlotNo(42_000_000)),
            "reset returns to the sealed seed baseline"
        );
        let outcome = advance_accumulator_over_chaindb(
            &s,
            &db,
            &sched,
            SlotNo(42_000_000),
            SlotNo(43_500_000),
        )
        .unwrap();
        assert_eq!(
            outcome,
            AccumulatorChaindbOutcome::ReachedTip {
                last_slot: Some(SlotNo(43_000_000))
            }
        );
        assert_eq!(
            s.last_advanced_slot().unwrap(),
            Some(SlotNo(43_000_000)),
            "replay restores the same tip"
        );
    }
}
