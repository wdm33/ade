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
//! flips it), so a failure to advance does NOT halt the follow: the store is left at its last good slot,
//! so `LAST_SLOT < wal_tail` becomes the durable stall signal and the store's readiness gate fail-closes
//! any authoritative read until it is resolved. A genuine STORE fault (durability I/O) is distinct — it
//! is an [`AdvanceError`], a real error the caller must not paper over.
//!
//! BND-1 (DC-EPOCH-39): the two reasons an advance does not happen are SEPARATE STATES —
//! [`AdvanceOutcome::BoundaryMarkRequired`] (a real crossing is due, decided from the epochs before the
//! apply) and [`AdvanceOutcome::ApplyFailed`] (a within-epoch block fail-closed, carrying the ledger's
//! own typed error). They were one `Stalled` variant, and the caller consequently ran boundary machinery
//! for ordinary within-epoch failures.

use ade_ledger::epoch_accumulator::LedgerTransitionError;
use std::collections::BTreeMap;

use ade_core::consensus::era_schedule::EraSchedule;
use ade_ledger::epoch_accumulator::{
    apply_selected_block, apply_selected_block_with_effects, EpochBoundaryEffect, SelectedBlockCtx,
};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::Coin;
use ade_types::{BlockNo, CardanoEra, EpochNo, Hash32, PoolId, SlotNo};

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
    /// The block's number (decoded canonical header block_no) — the height the S5 lineage anchor records +
    /// cardano's `SecurityParam` k bounds.
    pub block_no: BlockNo,
    /// The block's authoritative stored header hash — the S5 lineage-anchor hash (NOT re-derived here).
    pub header_hash: Hash32,
}

/// The outcome of advancing the accumulator over one block. Every variant is a NON-error outcome — the
/// follow continues regardless (the accumulator is observe-only in S2).
///
/// BND-1 (DC-EPOCH-39): the two ways an advance can fail to happen are DISTINCT STATES, not one. They
/// were a single `Stalled { reason: String }` whose own doc named both causes for it — "a boundary the
/// mark is withheld for, or a byte-uncertain block" — and the caller, having no way to tell them apart,
/// treated every one as a boundary: rewind the reduced checkpoint, sum a per-credential mark, attempt a
/// cross. Measured live, that ran 84,783 ms of boundary machinery plus 23,389 ms undoing its own rewind
/// for a block the walk had classified in 195 ms and which is not on a boundary at all
/// (`docs/evidence/run-stores/preprod-live2c/bnd-census-classified.txt`).
#[derive(Debug, Clone, PartialEq)]
pub enum AdvanceOutcome {
    /// The accumulator applied this within-epoch block and the store advanced to `slot` (epoch `epoch`).
    Advanced { slot: SlotNo, epoch: EpochNo },
    /// The block is at or before the accumulator's tip (a re-announce / replay) — no-op.
    AlreadyApplied { slot: SlotNo, last: SlotNo },
    /// A GENUINE epoch crossing is due: this block's epoch is strictly ahead of the accumulator's, so the
    /// within-epoch path (which withholds the boundary mark by construction) cannot apply it. The ONLY
    /// state permitted to reach the boundary machinery. Decided BEFORE the apply, from canonical data on
    /// both sides — never inferred from an error.
    BoundaryMarkRequired {
        slot: SlotNo,
        from_epoch: EpochNo,
        to_epoch: EpochNo,
    },
    /// OBSERVE-ONLY: the block is within the accumulator's own epoch (or below it) and
    /// `apply_selected_block` fail-closed on it. The store is untouched (stays at its last good slot) and
    /// the follow continues. Carries the ledger's OWN typed error, not a rendered string, so a caller
    /// compares a value rather than parsing prose. This is NOT a boundary and must never reach the
    /// boundary machinery.
    ApplyFailed {
        slot: SlotNo,
        error: LedgerTransitionError,
    },
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

    // BND-1 (DC-EPOCH-39): decide "is a crossing due?" HERE, positively, from the block's epoch against
    // the accumulator's own — before any apply, so the answer cannot be an inference from a failure.
    //
    // It predicts exactly what the authority does: `apply_selected_block_core` crosses
    // `acc.epoch + 1 ..= block_epoch`, so that loop fires IFF `block_epoch > acc.epoch`. Strictly
    // greater, never `>=`: an equal epoch is the ordinary within-epoch case and crosses nothing. A
    // block BELOW the accumulator's epoch is left to the apply, which returns the ledger's own typed
    // `BoundaryGap` — a real fail-closed, and correctly NOT a boundary.
    let acc_epoch = acc.epoch_state.epoch;
    if ctx.block_epoch.0 > acc_epoch.0 {
        return Ok(AdvanceOutcome::BoundaryMarkRequired {
            slot: ctx.block_slot,
            from_epoch: acc_epoch,
            to_epoch: ctx.block_epoch,
        });
    }

    let selected_ctx = SelectedBlockCtx {
        era: ctx.era,
        block_epoch: ctx.block_epoch,
        block_slot: ctx.block_slot,
        issuer_pool: ctx.issuer_pool.clone(),
        // S2: the boundary is structurally excluded. Unreachable now that a crossing is classified
        // above, and kept `None` so this path can never seal a mark it was not given.
        boundary_mark: None,
        // Within-epoch: no boundary fires here, so this is never consumed — carry 0.
        active_slots_per_epoch: 0,
    };

    match apply_selected_block(&acc, block_bytes, &selected_ctx) {
        Ok(next) => {
            store
                .advance(&next, ctx.block_slot, ctx.block_no, ctx.header_hash.clone())
                .map_err(AdvanceError::Store)?;
            Ok(AdvanceOutcome::Advanced {
                slot: ctx.block_slot,
                epoch: ctx.block_epoch,
            })
        }
        Err(error) => Ok(AdvanceOutcome::ApplyFailed {
            slot: ctx.block_slot,
            error,
        }),
    }
}

/// The outcome of reconciling the accumulator over a durable ChainDB prefix (LIVE-LEDGER-EPOCH-
/// TRANSITION S2 / DC-EPOCH-20). Every arm is NON-error: `ReachedTip` walked the whole `(from, to_slot]`
/// prefix; the other two stopped at the last good within-epoch slot (the store froze there). A genuine
/// fault is the `Err` arm.
///
/// BND-1 (DC-EPOCH-39): the stop mirrors [`AdvanceOutcome`]'s split. `BoundaryRequiredAt` is the ONLY
/// state whose caller may run boundary machinery; `ApplyFailedAt` is an ordinary within-epoch
/// fail-closed and must not.
#[derive(Debug, Clone, PartialEq)]
pub enum AccumulatorChaindbOutcome {
    /// Walked the full prefix; the store now sits at `last_slot` (None only if it never advanced).
    ReachedTip { last_slot: Option<SlotNo> },
    /// A genuine epoch crossing is due at `slot`; the store stays at its prior slot until the caller
    /// supplies the boundary mark.
    BoundaryRequiredAt {
        slot: SlotNo,
        from_epoch: EpochNo,
        to_epoch: EpochNo,
    },
    /// The walk hit a within-epoch block that fail-closed. Observe-only: the store stays at its prior
    /// slot and the follow continues. Carries the ledger's typed error.
    ApplyFailedAt {
        slot: SlotNo,
        error: LedgerTransitionError,
    },
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
    /// A boundary slot the caller KNOWS is a boundary has no durable block. This is a real fault, not an
    /// observe-only stall: the within-epoch walk simply never reaches an absent slot, but a boundary cross
    /// is directed AT a specific slot, so its absence is a durable-store inconsistency.
    MissingBlock(SlotNo),
    /// LIVE-LEDGER-EPOCH-TRANSITION S4-pre-2: the boundary's authoritative leadership freeze effect was
    /// missing, extra, mislabeled, or not bound to the selected boundary block. Leadership authority is part
    /// of the boundary transition — an inconsistency is a HARD terminal, never an observe-only stall.
    BoundaryLeadership(String),
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
            block_no: decoded.header_input.block_no,
            header_hash: stored.hash.clone(),
        };
        match advance_accumulator_over_block(store, &stored.bytes, &ctx)
            .map_err(AccumulatorChaindbError::Advance)?
        {
            AdvanceOutcome::Advanced { .. } | AdvanceOutcome::AlreadyApplied { .. } => {}
            AdvanceOutcome::BoundaryMarkRequired {
                slot,
                from_epoch,
                to_epoch,
            } => {
                return Ok(AccumulatorChaindbOutcome::BoundaryRequiredAt {
                    slot,
                    from_epoch,
                    to_epoch,
                });
            }
            AdvanceOutcome::ApplyFailed { slot, error } => {
                return Ok(AccumulatorChaindbOutcome::ApplyFailedAt { slot, error });
            }
        }
    }
    let last_slot = store
        .last_advanced_slot()
        .map_err(|e| AccumulatorChaindbError::Advance(AdvanceError::Store(e)))?;
    Ok(AccumulatorChaindbOutcome::ReachedTip { last_slot })
}

/// The outcome of crossing the accumulator over ONE durable boundary block (LIVE-LEDGER-EPOCH-TRANSITION
/// S3 / DC-EPOCH-22, item #2b-i). `Crossed` / `AlreadyCrossed` / `Stalled` are all NON-error outcomes — the
/// follow continues regardless (the accumulator is observe-only in S3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccumulatorBoundaryOutcome {
    /// The boundary block applied WITH the mark: the NEWEPOCH transition fired and the store advanced into
    /// the new epoch (`to_epoch`) at `slot`.
    Crossed {
        from_epoch: EpochNo,
        to_epoch: EpochNo,
        slot: SlotNo,
    },
    /// The boundary block is at or before the store's tip — already crossed (idempotent re-entry); no-op.
    AlreadyCrossed { slot: SlotNo, last: SlotNo },
    /// OBSERVE-ONLY: `apply_selected_block` fail-closed crossing this block. Store untouched; follow continues.
    Stalled { slot: SlotNo, reason: String },
}

/// Cross the durable accumulator over ONE durable boundary block, supplying the boundary MARK
/// (LIVE-LEDGER-EPOCH-TRANSITION S3 / DC-EPOCH-22, item #2b-i).
///
/// This is the ONLY place the S2 structural mark-exclusion is lifted: [`WithinEpochCtx`] /
/// [`advance_accumulator_over_block`] stay mark-free, so a boundary reached through them STALLS
/// (`MissingBoundaryStake`). A boundary the co-advancer has positioned — the accumulator's cursor at the
/// last within-epoch slot `s_prev`, the boundary block at `boundary_block_slot` — is crossed HERE: the
/// caller supplies the per-credential `boundary_mark` captured at `s_prev` (`sum_base_credential_stake()`
/// over the reduced checkpoint), and the BLUE `apply_selected_block` runs the NEWEPOCH transition then the
/// block's within-epoch effects.
///
/// Outcomes: an at-or-before-tip block is an idempotent [`AccumulatorBoundaryOutcome::AlreadyCrossed`]
/// (never re-decoded / re-applied — the BLUE cross is not itself idempotent, only this guard is); a contract
/// fail-close is an observe-only [`AccumulatorBoundaryOutcome::Stalled`] (store untouched). A boundary slot
/// with NO durable block is a real [`AccumulatorChaindbError::MissingBlock`] (the cross was directed AT that
/// slot), distinct from the within-epoch walk that simply never reaches an absent slot.
pub fn cross_accumulator_over_boundary_block(
    store: &EpochAccumulatorStore,
    chaindb: &dyn ChainDb,
    era_schedule: &EraSchedule,
    boundary_block_slot: SlotNo,
    boundary_mark: &BTreeMap<StakeCredential, Coin>,
    // S4-L2 (v6): the MARK SOURCE point `s_prev` (the boundary point the mark + reduced checkpoint are settled
    // at, NOT the crossing trigger block `boundary_block_slot`) and the reduced-checkpoint commitment finalized
    // THERE, both captured by the run loop (the only layer holding the boundary context AND the reduced
    // checkpoint advanced to `s_prev`). Sealed into the frozen leadership object so the promoted candidate
    // authority is fully self-contained (leadership + source + provenance, no window replay).
    mark_source_slot: SlotNo,
    mark_source_hash: &Hash32,
    source_checkpoint_commitment: &Hash32,
) -> Result<AccumulatorBoundaryOutcome, AccumulatorChaindbError> {
    let (last_slot, acc) = store
        .load_current()
        .map_err(|e| AccumulatorChaindbError::Advance(AdvanceError::Store(e)))?
        .ok_or(AccumulatorChaindbError::Advance(AdvanceError::Unsealed))?;

    // Idempotent re-entry: a boundary at or before the store's tip was already crossed — never re-decode or
    // re-apply (the cross mutates pots/snapshots; only this guard makes the call idempotent).
    if boundary_block_slot.0 <= last_slot.0 {
        return Ok(AccumulatorBoundaryOutcome::AlreadyCrossed {
            slot: boundary_block_slot,
            last: last_slot,
        });
    }

    let stored = chaindb
        .get_block_by_slot(boundary_block_slot)
        .map_err(AccumulatorChaindbError::ChainDb)?
        .ok_or(AccumulatorChaindbError::MissingBlock(boundary_block_slot))?;

    let decoded = ade_ledger::block_validity::decode_block(&stored.bytes)
        .map_err(|e| AccumulatorChaindbError::Decode(format!("{e:?}")))?;
    let block_epoch = era_schedule
        .locate(boundary_block_slot)
        .map_err(|e| AccumulatorChaindbError::Locate(format!("{e:?}")))?
        .epoch;

    // The monetary-expansion expected-blocks denominator = `epochLength × activeSlotCoeff`, derived
    // from the era schedule's REAL per-era epoch length (preview 86_400, mainnet/preprod 432_000).
    // The reward calc previously hardcoded the mainnet `21_600`, under-expanding preview 5×.
    //
    // FOLLOW-UP (canonical sourcing): `f` is fixed here at 1/20 — the Cardano active-slot coefficient,
    // identical across mainnet/preprod/preview, so no current target diverges. The CANONICAL source is
    // `SeedConsensusInputs.active_slots_coeff` (persisted in the seed sidecar, already consumed by the
    // leader schedule via `ledger_view.active_slots_coeff`); a refinement should thread that here
    // instead of the literal `/ 20` so a non-1/20 network can never silently mis-expand.
    let active_slots_per_epoch = u64::from(
        era_schedule
            .epoch_length_slots(boundary_block_slot)
            .map_err(|e| AccumulatorChaindbError::Locate(format!("{e:?}")))?,
    ) / 20;

    let ctx = SelectedBlockCtx {
        era: decoded.era,
        block_epoch,
        block_slot: boundary_block_slot,
        issuer_pool: PoolId(decoded.header_input.issuer_pool.clone()),
        // S3 / DC-EPOCH-22: the boundary mark captured at the prior tip — the ONLY point the S2
        // mark-exclusion is lifted.
        boundary_mark: Some(boundary_mark.clone()),
        active_slots_per_epoch,
    };

    let from_epoch = acc.epoch_state.epoch;
    match apply_selected_block_with_effects(
        &acc,
        &stored.bytes,
        &ctx,
        mark_source_slot,
        mark_source_hash,
        source_checkpoint_commitment,
    ) {
        Ok((next, effects)) => {
            // S4-pre-2: the boundary's leadership freeze is AUTHORITATIVE, not optional evidence — every
            // inconsistency below is a HARD terminal (never an observe-only Stalled, never a warning).
            //
            // A co-advancer-positioned cross crosses EXACTLY ONE boundary (the immediate next epoch), so it
            // must emit exactly one FreezeLeadership effect. A multi-epoch batch (empty epochs) is NOT
            // supported for leadership sealing — each boundary's leadership must seal with its OWN accumulator
            // transition, and the one-advance-per-block store persists no intermediate epoch state; fail
            // closed rather than let "latest leadership wins" drop an intermediate boundary's leadership.
            let boundaries = block_epoch.0.saturating_sub(from_epoch.0);
            if boundaries != 1 {
                return Err(AccumulatorChaindbError::BoundaryLeadership(format!(
                    "boundary cross spans {boundaries} epoch(s) ({} -> {}); S4-pre-2 seals exactly one boundary's leadership per advance",
                    from_epoch.0, block_epoch.0
                )));
            }
            if effects.len() != 1 {
                return Err(AccumulatorChaindbError::BoundaryLeadership(format!(
                    "one boundary crossed but produced {} leadership effect(s) — a missing/extra boundary freeze",
                    effects.len()
                )));
            }
            let EpochBoundaryEffect::FreezeLeadership { source_epoch, target_leadership_epoch, distr } =
                &effects[0];
            // source_epoch == the boundary's into-epoch (block_epoch); target == source+1; the distr is bound
            // to THIS selected boundary block (slot + header hash).
            if source_epoch.0 != block_epoch.0 {
                return Err(AccumulatorChaindbError::BoundaryLeadership(format!(
                    "effect source_epoch {} != boundary into-epoch {}",
                    source_epoch.0, block_epoch.0
                )));
            }
            if target_leadership_epoch.0 != source_epoch.0 + 1 {
                return Err(AccumulatorChaindbError::BoundaryLeadership(format!(
                    "effect target_leadership_epoch {} != source_epoch+1 {}",
                    target_leadership_epoch.0,
                    source_epoch.0 + 1
                )));
            }
            // The frozen leadership source binds to the MARK SOURCE `s_prev` (where the shell settled the mark +
            // reduced checkpoint), NOT the crossing trigger `boundary_block_slot` (s_bb). Ground it in the REAL
            // durable block at `s_prev` so a shell that declares a bogus source fails closed: BLUE must echo the
            // exact (slot, hash), that hash must be the durable block's hash, and the v6 checkpoint commitment
            // must be the one the shell captured at `s_prev`.
            let mark_stored = chaindb
                .get_block_by_slot(mark_source_slot)
                .map_err(AccumulatorChaindbError::ChainDb)?
                .ok_or(AccumulatorChaindbError::MissingBlock(mark_source_slot))?;
            if distr.source_slot != mark_source_slot
                || distr.source_hash != mark_stored.hash
                || mark_stored.hash != *mark_source_hash
            {
                return Err(AccumulatorChaindbError::BoundaryLeadership(
                    "effect source_slot/source_hash not bound to the mark source block (s_prev)".to_string(),
                ));
            }
            if distr.source_checkpoint_commitment != *source_checkpoint_commitment {
                return Err(AccumulatorChaindbError::BoundaryLeadership(
                    "effect source_checkpoint_commitment != the mark-source-finalized commitment".to_string(),
                ));
            }
            // ATOMIC: accumulator blob + LAST_SLOT + anchor + current leadership + marker, one redb commit.
            store
                .advance_with_current_leadership(
                    &next,
                    boundary_block_slot,
                    decoded.header_input.block_no,
                    stored.hash.clone(),
                    distr,
                )
                .map_err(|e| AccumulatorChaindbError::Advance(AdvanceError::Store(e)))?;
            Ok(AccumulatorBoundaryOutcome::Crossed {
                from_epoch,
                to_epoch: block_epoch,
                slot: boundary_block_slot,
            })
        }
        Err(e) => Ok(AccumulatorBoundaryOutcome::Stalled {
            slot: boundary_block_slot,
            reason: format!("{e:?}"),
        }),
    }
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
        // v2: a persisted Conway accumulator carries the deposit params (the codec fails closed on None).
        acc.conway_deposit_params = Some(ade_ledger::pparams::ConwayOnlyDepositParams {
            drep_deposit: Coin(500_000_000),
            gov_action_deposit: Coin(100_000_000_000),
            drep_activity: 20,
        });
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
            block_no: BlockNo(1),
            header_hash: Hash32([0x77; 32]),
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
            block_no: BlockNo(1),
            header_hash: Hash32([0x77; 32]),
        };
        let outcome = advance_accumulator_over_block(&s, RAW_CONWAY_BLOCK, &ctx).unwrap();
        match outcome {
            // BND-1 (CE-BND1-2): a genuine crossing is now named by BOTH epochs, decided before the
            // apply — a strictly stronger assertion than matching an error string, and one that no
            // longer passes if the classification comes from a failure.
            AdvanceOutcome::BoundaryMarkRequired {
                slot,
                from_epoch,
                to_epoch,
            } => {
                assert_eq!(slot, SlotNo(43_000_000));
                assert_eq!(from_epoch, EpochNo(500));
                assert_eq!(to_epoch, EpochNo(501));
            }
            other => panic!("expected BoundaryMarkRequired, got {other:?}"),
        }
        // Observe-only: the store is untouched — LAST_SLOT stays at the seed (the durable stall signal).
        assert_eq!(s.last_advanced_slot().unwrap(), Some(SlotNo(42_000_000)));
    }

    /// BND-1 / CE-BND1-1 + CE-BND1-4 (DC-EPOCH-39). A block in the accumulator's OWN epoch whose apply
    /// fail-closes is an `ApplyFailed` carrying the ledger's typed error — NEVER a boundary state.
    ///
    /// This is the shape the live preprod store has been stuck in since LIVE-2c: slot 130,350,133,
    /// epoch 305, accumulator cursor 130,350,114, also epoch 305 — measured in
    /// `docs/evidence/run-stores/preprod-live2c/bnd-census-classified.txt`. Before this slice the two
    /// states were one, so this block drove a checkpoint rewind, a per-credential mark sum and a cross
    /// attempt: 84,783 ms of boundary machinery for a block that is not on a boundary.
    ///
    /// The era MISMATCH is used as the failure trigger deliberately: it is a fail-closed the contract
    /// reaches without needing a hand-built phase-2-invalid block, and the assertion is on the CLASS of
    /// outcome and the typed error value, not on which error it is.
    #[test]
    fn a_within_epoch_apply_failure_is_apply_failed_not_a_boundary() {
        let tmp = TempDir::new().unwrap();
        let s = sealed_store_at_epoch_500(&tmp, SlotNo(42_000_000));
        let ctx = WithinEpochCtx {
            era: CardanoEra::Babbage, // disagrees with the block envelope ⇒ the contract fail-closes
            block_epoch: EpochNo(500), // SAME epoch as the accumulator — not a boundary
            block_slot: SlotNo(43_000_000),
            issuer_pool: pool(0x77),
            block_no: BlockNo(1),
            header_hash: Hash32([0x77; 32]),
        };
        let outcome = advance_accumulator_over_block(&s, RAW_CONWAY_BLOCK, &ctx).unwrap();
        match outcome {
            AdvanceOutcome::ApplyFailed { slot, error } => {
                assert_eq!(slot, SlotNo(43_000_000));
                // CE-BND1-4: the ledger's own typed error survives, compared BY VALUE — a rendered
                // string would make this assertion a substring match on prose.
                assert_eq!(
                    error,
                    LedgerTransitionError::EraMismatch {
                        ctx: CardanoEra::Babbage as u64,
                        block: CardanoEra::Conway as u64,
                    }
                );
            }
            other => panic!("a within-epoch apply failure must not be a boundary state, got {other:?}"),
        }
        // Observe-only and cursor-preserving, exactly as the old flattened state was.
        assert_eq!(s.last_advanced_slot().unwrap(), Some(SlotNo(42_000_000)));
    }

    /// BND-1 / CE-BND1-2. The boundary classification is POSITIVE — taken from the epochs before the
    /// apply — so it holds even when the block would ALSO have failed to apply. Under the old
    /// error-derived classification this case was indistinguishable from an ordinary apply failure.
    #[test]
    fn a_crossing_is_classified_from_the_epochs_even_when_the_apply_would_fail() {
        let tmp = TempDir::new().unwrap();
        let s = sealed_store_at_epoch_500(&tmp, SlotNo(42_000_000));
        let ctx = WithinEpochCtx {
            era: CardanoEra::Babbage,  // would fail-close inside the contract...
            block_epoch: EpochNo(501), // ...but a crossing is due, and that is decided FIRST
            block_slot: SlotNo(43_000_000),
            issuer_pool: pool(0x77),
            block_no: BlockNo(1),
            header_hash: Hash32([0x77; 32]),
        };
        match advance_accumulator_over_block(&s, RAW_CONWAY_BLOCK, &ctx).unwrap() {
            AdvanceOutcome::BoundaryMarkRequired {
                from_epoch,
                to_epoch,
                ..
            } => {
                assert_eq!((from_epoch, to_epoch), (EpochNo(500), EpochNo(501)));
            }
            other => panic!("expected BoundaryMarkRequired ahead of the apply, got {other:?}"),
        }
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
            block_no: BlockNo(1),
            header_hash: Hash32([0x77; 32]),
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
            block_no: BlockNo(1),
            header_hash: Hash32([0x77; 32]),
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

    /// ACCUMULATOR-REFOLD-BOUND S1 — CE-AR-5 / INV-AR-6 (replay equivalence).
    ///
    /// Refolding from the SETTLED rewind point must land on state byte-identical to folding
    /// straight through from bootstrap over the same canonical chain. This is THE safety claim of
    /// the slice: rewinding to a nearer baseline moves only the STARTING POINT of a deterministic
    /// re-derivation, so the derived result cannot differ. If it ever could, the bounded rewind
    /// would be trading correctness for speed.
    #[test]
    fn refold_from_settled_point_equals_fold_from_bootstrap() {
        use crate::chaindb::InMemoryChainDb;
        let sched = schedule_86k();
        let seed = SlotNo(42_000_000);
        let tip = SlotNo(43_500_000);
        // All within epoch 500 (86_000 * 500 = 43_000_000), so this exercises the within-epoch fold.
        let slots = [43_000_000u64, 43_010_000, 43_020_000, 43_030_000];
        let db = InMemoryChainDb::new();
        for s in slots {
            put_raw(&db, s);
        }

        // PATH A — the pre-slice behaviour: one straight fold from the bootstrap baseline.
        let tmp_a = TempDir::new().unwrap();
        let sa = sealed_store_at_epoch_500(&tmp_a, seed);
        advance_accumulator_over_chaindb(&sa, &db, &sched, seed, tip).unwrap();
        let expected = sa.load_current().unwrap().expect("path A sealed");

        // PATH B — fold partway, promote that point as SETTLED, fold on to the tip, then rewind to
        // the settled point and refold the remainder.
        let tmp_b = TempDir::new().unwrap();
        let sb = sealed_store_at_epoch_500(&tmp_b, seed);
        advance_accumulator_over_chaindb(&sb, &db, &sched, seed, SlotNo(43_010_000)).unwrap();
        // The synthetic fixture reuses ONE raw block, so every stored block decodes to the SAME
        // block_no — there is no height separation to earn a promotion with. The tip height is
        // supplied by the caller (as the live path does, from the ChainDb), so take it from the
        // staged point itself and use k=0: that isolates the promotion mechanism from the height
        // arithmetic, which `settled_point_is_only_promoted_once_k_blocks_settled` covers.
        let staged_bn = sb
            .last_advanced_point()
            .unwrap()
            .expect("certified after fold")
            .block_no;
        assert!(!sb.roll_settled_rewind_point(staged_bn, 0).unwrap());
        assert!(sb.roll_settled_rewind_point(staged_bn, 0).unwrap());
        let settled = sb.settled_rewind_point().unwrap().expect("promoted");
        assert_eq!(settled.slot, SlotNo(43_010_000));

        advance_accumulator_over_chaindb(&sb, &db, &sched, seed, tip).unwrap();
        assert_eq!(
            sb.load_current().unwrap().expect("path B pre-rewind"),
            expected,
            "sanity: both paths reach the same tip state before any rewind"
        );

        // Rewind to the settled point -- the accumulator goes back to 43_010_000...
        assert!(sb.reset_to_settled().unwrap());
        assert_eq!(sb.last_advanced_slot().unwrap(), Some(SlotNo(43_010_000)));
        // ...and the refold from there reproduces the SAME state, byte for byte.
        advance_accumulator_over_chaindb(&sb, &db, &sched, seed, tip).unwrap();
        let refolded = sb.load_current().unwrap().expect("path B refolded");
        assert_eq!(
            refolded, expected,
            "refold from the settled rewind point must be byte-identical to the \
             fold-from-bootstrap it replaces (INV-AR-6)"
        );
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
            AccumulatorChaindbOutcome::BoundaryRequiredAt {
                slot,
                from_epoch,
                to_epoch,
            } => {
                assert_eq!(slot, SlotNo(43_086_000));
                assert_eq!(from_epoch.0 + 1, to_epoch.0, "one crossing is due");
            }
            other => panic!("expected BoundaryRequiredAt, got {other:?}"),
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

    fn cred(b: u8) -> StakeCredential {
        StakeCredential::KeyHash(Hash28([b; 28]))
    }

    /// LIVE-LEDGER-EPOCH-TRANSITION S3 (DC-EPOCH-22 / #2b-i): the boundary block CROSSES when the mark is
    /// supplied — the counterpart of `over_chaindb_stops_at_boundary_observe_only` (mark=None stalls). The
    /// NEWEPOCH transition fires and the store advances into the new epoch.
    #[test]
    fn boundary_block_crosses_with_mark() {
        use crate::chaindb::InMemoryChainDb;
        let tmp = TempDir::new().unwrap();
        let s = sealed_store_at_epoch_500(&tmp, SlotNo(42_000_000));
        let db = InMemoryChainDb::new();
        put_raw(&db, 42_000_000); // s_prev: the MARK SOURCE = the settled store cursor (epoch 500)
        put_raw(&db, 43_086_000); // s_bb: 86_000 * 501 -> epoch 501, the boundary crossing block
        // The leadership source binds to s_prev (NOT s_bb): its REAL durable hash is the freeze provenance.
        let s_prev_hash = db.get_block_by_slot(SlotNo(42_000_000)).unwrap().unwrap().hash;
        let mut mark: BTreeMap<StakeCredential, Coin> = BTreeMap::new();
        mark.insert(cred(0x11), Coin(5_000_000));
        mark.insert(cred(0x22), Coin(7_000_000));
        let outcome = cross_accumulator_over_boundary_block(
            &s,
            &db,
            &schedule_86k(),
            SlotNo(43_086_000),
            &mark,
            SlotNo(42_000_000),
            &s_prev_hash,
            &Hash32([0x0C; 32]),
        )
        .unwrap();
        assert_eq!(
            outcome,
            AccumulatorBoundaryOutcome::Crossed {
                from_epoch: EpochNo(500),
                to_epoch: EpochNo(501),
                slot: SlotNo(43_086_000),
            }
        );
        // The store advanced into epoch 501 at the boundary slot.
        let (slot, acc) = s.load_current().unwrap().unwrap();
        assert_eq!(slot, SlotNo(43_086_000));
        assert_eq!(acc.epoch_state.epoch, EpochNo(501));
    }

    /// A boundary at or before the store's tip is an idempotent no-op — the cross is never re-decoded or
    /// re-applied (proven by an EMPTY chaindb: if the guard didn't short-circuit, the read would fault).
    #[test]
    fn boundary_cross_is_idempotent() {
        use crate::chaindb::InMemoryChainDb;
        let tmp = TempDir::new().unwrap();
        // The store's tip is already AT the boundary slot.
        let s = sealed_store_at_epoch_500(&tmp, SlotNo(43_086_000));
        let db = InMemoryChainDb::new(); // empty — the idempotent arm must not read it
        let mark: BTreeMap<StakeCredential, Coin> = BTreeMap::new();
        let outcome = cross_accumulator_over_boundary_block(
            &s,
            &db,
            &schedule_86k(),
            SlotNo(43_086_000),
            &mark,
            SlotNo(43_086_000),
            &Hash32([0x07; 32]),
            &Hash32([0x0C; 32]),
        )
        .unwrap();
        assert_eq!(
            outcome,
            AccumulatorBoundaryOutcome::AlreadyCrossed {
                slot: SlotNo(43_086_000),
                last: SlotNo(43_086_000),
            }
        );
        // Nothing re-applied: still epoch 500 (the cross never ran).
        let (slot, acc) = s.load_current().unwrap().unwrap();
        assert_eq!(slot, SlotNo(43_086_000));
        assert_eq!(acc.epoch_state.epoch, EpochNo(500));
    }

    /// A boundary cross directed at a slot with no durable block is a REAL fault (`MissingBlock`), not an
    /// observe-only stall — the caller asserted that slot is a boundary, so its absence is a store fault.
    #[test]
    fn boundary_cross_missing_block_is_a_fault() {
        use crate::chaindb::InMemoryChainDb;
        let tmp = TempDir::new().unwrap();
        let s = sealed_store_at_epoch_500(&tmp, SlotNo(42_000_000));
        let db = InMemoryChainDb::new(); // empty — no block at the boundary slot
        let mark: BTreeMap<StakeCredential, Coin> = BTreeMap::new();
        let err = cross_accumulator_over_boundary_block(
            &s,
            &db,
            &schedule_86k(),
            SlotNo(43_086_000),
            &mark,
            SlotNo(43_086_000),
            &Hash32([0x07; 32]),
            &Hash32([0x0C; 32]),
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                AccumulatorChaindbError::MissingBlock(SlotNo(43_086_000))
            ),
            "expected MissingBlock at the boundary slot, got {err:?}"
        );
    }
}
