//! EPOCH-CONSENSUS-VIEW S3f-4d-wire — the live orchestration of the dual-path epoch
//! activation (user-directed 2026-06-21).
//!
//! Production rule: the active epoch view must be reproducible from durable canonical inputs
//! ALONE. The AUTHORITATIVE candidate is derived by DURABLE WINDOW REPLAY — the manifest-bound
//! bootstrap checkpoint + the canonical selected-chain ChainDB window + explicit source-window
//! bounds. The continuously-advanced live reduced checkpoint (DC-EPOCH-11 -mat) is an
//! INDEPENDENT cross-check: the live-derived view must AGREE with the replay candidate on the
//! committed fields before promotion; mismatch / missing range / late candidate is a TERMINAL
//! epoch-activation halt. NO peer fetch, CLI query, cache, or live network response may supply
//! a missing block during derivation; window bounds are explicit roles, never wall-clock.
//!
//! This module is RED glue (it reads the durable ChainDB) over the pure pieces: the window +
//! `validate_source_window` (`epoch_source_window`), `derive_candidate` (`epoch_candidate`),
//! `derive_stake_by_pool` (the live cross-check), and `activate_at_boundary` (`epoch_activate`).

use ade_ledger::block_validity::header_input::decode_block;
use ade_ledger::reduced_epoch_view::EpochConsensusView;
use ade_ledger::reduced_snapshot::SnapshotPhase;
use ade_ledger::state::LedgerState;
use ade_runtime::chaindb::{
    ChainDb, CheckpointReadinessError, ReducedCheckpointError, ReducedUtxoCheckpoint,
};
use ade_types::shelley::block::ShelleyBlock;
use ade_types::{CardanoEra, EpochNo, Hash32, SlotNo};

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::events::Point;
use ade_ledger::wal::WalEntry;

use crate::epoch_activate::{activate_at_boundary, BoundaryActivationOutcome};
use crate::epoch_activation::{ActiveEpochView, EpochViewActivationError};
use crate::epoch_candidate::{derive_candidate, CandidateDeriveError};
use crate::epoch_source_window::{
    target_epoch_for_source, validate_source_window, ActivationSourceWindow, SourceWindowBlock,
    SourceWindowError,
};

/// The extracted, VALIDATED source window: the pinned window roles + its selected-chain block
/// identities (for the completeness proof) + the full decoded blocks (for the replay derive).
pub struct SourceWindowExtract {
    pub window: ActivationSourceWindow,
    pub window_blocks: Vec<SourceWindowBlock>,
    pub shelley_blocks: Vec<ShelleyBlock>,
}

/// Why the live source-window extraction failed (fail-closed -- never yields a window a
/// candidate could be derived from).
#[derive(Debug)]
pub enum SourceWindowExtractError {
    /// Reading the durable ChainDB failed.
    ChainDb(ade_runtime::chaindb::ChainDbError),
    /// A stored block did not decode (corrupt durable store).
    Decode(String),
    /// The completed window failed the pure validation (incomplete / out-of-lineage / unordered
    /// / wrong target epoch -- see `SourceWindowError`).
    Window(SourceWindowError),
    /// The source epoch / phase has no explicit target-epoch mapping.
    TargetEpoch,
}

/// S3f-4d-wire-1 (DC-EPOCH-11): extract the canonical source window from the DURABLE ChainDB
/// for `[source_window_start, source_window_end]` (the explicit bounds the orchestrator
/// computes from the era schedule -- NEVER the wall clock). Reads ONLY the durable selected
/// chain (no peer/network/CLI), decodes each block to its `(slot, hash, prev_hash)` identity +
/// its full `ShelleyBlock`, pins the window (anchor + lineage tip + the explicit target
/// epoch), and runs `validate_source_window` so an incomplete / out-of-lineage / unordered
/// range FAILS CLOSED before it can produce a candidate.
pub fn extract_source_window(
    chaindb: &dyn ChainDb,
    source_epoch: EpochNo,
    source_window_start: SlotNo,
    source_window_end: SlotNo,
    snapshot_phase: SnapshotPhase,
    source_window_anchor: Hash32,
) -> Result<SourceWindowExtract, SourceWindowExtractError> {
    let target_epoch = target_epoch_for_source(source_epoch, snapshot_phase)
        .ok_or(SourceWindowExtractError::TargetEpoch)?;
    let mut window_blocks: Vec<SourceWindowBlock> = Vec::new();
    let mut shelley_blocks: Vec<ShelleyBlock> = Vec::new();
    let iter = chaindb
        .iter_from_slot(source_window_start)
        .map_err(SourceWindowExtractError::ChainDb)?;
    for stored in iter {
        let stored = stored.map_err(SourceWindowExtractError::ChainDb)?;
        if stored.slot.0 > source_window_end.0 {
            break;
        }
        let decoded = decode_block(&stored.bytes)
            .map_err(|e| SourceWindowExtractError::Decode(format!("{e:?}")))?;
        let prev_hash = decoded
            .prev_hash
            .block_hash()
            .cloned()
            .unwrap_or(Hash32([0; 32]));
        window_blocks.push(SourceWindowBlock {
            slot: stored.slot,
            hash: decoded.block_hash.clone(),
            prev_hash,
        });
        let inner = &stored.bytes[decoded.inner_start..decoded.inner_end];
        let sb = ade_codec::conway::decode_conway_block(inner)
            .map_err(|e| SourceWindowExtractError::Decode(format!("conway: {e:?}")))?
            .decoded()
            .clone();
        shelley_blocks.push(sb);
    }
    if window_blocks.is_empty() {
        return Err(SourceWindowExtractError::Window(SourceWindowError::Empty));
    }
    let lineage_pin = window_blocks[window_blocks.len() - 1].hash.clone();
    let window = ActivationSourceWindow {
        source_epoch,
        source_window_start,
        source_window_end,
        snapshot_phase,
        target_epoch,
        source_window_anchor,
        lineage_pin,
    };
    validate_source_window(&window, &window_blocks).map_err(SourceWindowExtractError::Window)?;
    Ok(SourceWindowExtract {
        window,
        window_blocks,
        shelley_blocks,
    })
}

/// Why the live reduced checkpoint is NOT a valid readiness witness for the boundary candidate
/// (TERMINAL -- the activation halts rather than promote against an unproven live state).
#[derive(Debug)]
pub enum ReadinessError {
    /// The live checkpoint is not a healthy advanced-through witness (unsealed / wrong seed /
    /// lagging the source-window end).
    Checkpoint(CheckpointReadinessError),
    /// Reading the durable ChainDB for the lineage commitment failed.
    ChainDb(ade_runtime::chaindb::ChainDbError),
    /// The window's terminal point (`lineage_pin`) is NO LONGER on the durable selected chain
    /// (a reorg removed it between extraction and activation) -- the live checkpoint cannot be
    /// proven to have processed THIS window's lineage.
    TerminalMissing,
}

/// S3f-4d-wire-2 (DC-EPOCH-11): the live checkpoint as a NON-AUTHORITATIVE readiness/health
/// WITNESS for the boundary candidate (NOT a same-state comparator -- the authoritative view is
/// the durable window replay). It must PROVE it processed the EXACT selected-chain source window:
/// (1) it has ADVANCED THROUGH `source_window_end` (last_advanced >= end; beyond is fine) with
/// the matching seed lineage; AND (2) the window's terminal point (`lineage_pin`) is still
/// durably present on the selected chain. Fail-closed (terminal) on any miss -- never promote
/// against a lagging / corrupt / reorged-away readiness state.
pub fn verify_live_readiness(
    live: &ReducedUtxoCheckpoint,
    window: &ActivationSourceWindow,
    expected_seed_slot: SlotNo,
    chaindb: &dyn ChainDb,
) -> Result<(), ReadinessError> {
    live.verify_advanced_through(window.source_window_end, expected_seed_slot)
        .map_err(ReadinessError::Checkpoint)?;
    chaindb
        .get_block_by_hash(&window.lineage_pin)
        .map_err(ReadinessError::ChainDb)?
        .ok_or(ReadinessError::TerminalMissing)?;
    Ok(())
}

/// Why the authoritative candidate derivation failed (TERMINAL).
#[derive(Debug)]
pub enum ActivationDeriveError {
    /// Materializing the fresh seed-state replay checkpoint failed.
    Materialize(ReducedCheckpointError),
    /// Replaying the window over the seed-state checkpoint failed.
    Derive(CandidateDeriveError),
}

/// S3f-4d-wire-2 (DC-EPOCH-11): derive the SOLE AUTHORITATIVE activation candidate by DURABLE
/// WINDOW REPLAY -- materialize a FRESH seed-state checkpoint from the manifest-bound bootstrap
/// baseline (a separate redb, so the live checkpoint is never mutated) and replay the validated
/// durable selected-chain `shelley_blocks` over it to the exact boundary, binding the
/// `EpochConsensusView` at B. Reproducible from durable canonical inputs ALONE; no peer/network/
/// CLI/cache supplies a block.
pub fn derive_authoritative_candidate(
    live: &ReducedUtxoCheckpoint,
    window: &ActivationSourceWindow,
    shelley_blocks: &[ShelleyBlock],
    bootstrap_state: &LedgerState,
    network_magic: u32,
    nonce: Hash32,
    scratch_path: &std::path::Path,
) -> Result<EpochConsensusView, ActivationDeriveError> {
    let replay_cp = live
        .materialize_bootstrap_into(scratch_path)
        .map_err(ActivationDeriveError::Materialize)?;
    derive_candidate(
        window,
        &replay_cp,
        bootstrap_state,
        shelley_blocks,
        CardanoEra::Conway,
        network_magic,
        nonce,
    )
    .map_err(ActivationDeriveError::Derive)
}

/// The explicit, durable-derived bounds of the source window the orchestrator computes from the
/// era schedule for the completed source epoch (NEVER the wall clock).
pub struct WindowBounds {
    pub source_epoch: EpochNo,
    pub source_window_start: SlotNo,
    pub source_window_end: SlotNo,
    pub snapshot_phase: SnapshotPhase,
    /// The durable tip immediately BEFORE the window (the first block's expected parent).
    pub source_window_anchor: Hash32,
    /// The live checkpoint's sealed seed slot (the bootstrap lineage the readiness witness pins).
    pub expected_seed_slot: SlotNo,
}

/// S3f-4d-wire-3b (DC-EPOCH-11): compute the EXPLICIT source-window bounds for the FIRST
/// activation -- the completed SEED epoch -- from the era schedule (NEVER the wall clock). The
/// seed checkpoint sits at `seed_point`; the window is the durable blocks AFTER it up to the
/// seed epoch's LAST slot, so the replay (seed checkpoint + window) yields the mark at the
/// seed-epoch boundary (which becomes the Set the target epoch's leadership reads, source+2).
/// `None` if the seed point does not locate in `seed_epoch` (a malformed schedule / wrong seed).
pub fn compute_first_window_bounds(
    era_schedule: &EraSchedule,
    seed_point_slot: SlotNo,
    seed_point_hash: Hash32,
    seed_epoch: EpochNo,
) -> Option<WindowBounds> {
    let loc = era_schedule.locate(seed_point_slot).ok()?;
    if loc.epoch.0 != seed_epoch.0 {
        return None;
    }
    let epoch_len =
        u64::from(era_schedule.eras().get(loc.era_index as usize)?.epoch_length_slots);
    let epoch_start = seed_point_slot
        .0
        .checked_sub(u64::from(loc.relative_slot_in_epoch))?;
    let seed_epoch_end = epoch_start.checked_add(epoch_len)?.checked_sub(1)?;
    Some(WindowBounds {
        source_epoch: seed_epoch,
        source_window_start: SlotNo(seed_point_slot.0.checked_add(1)?),
        source_window_end: SlotNo(seed_epoch_end),
        snapshot_phase: SnapshotPhase::Set,
        source_window_anchor: seed_point_hash,
        expected_seed_slot: seed_point_slot,
    })
}

/// Why the boundary activation attempt is a TERMINAL epoch-activation failure (halt -- the
/// caller stops admit/forge/follow, NEVER falls back to the seed view).
#[derive(Debug)]
pub enum ActivationError {
    /// The durable source window could not be extracted/validated (incomplete / out-of-lineage).
    SourceWindow(SourceWindowExtractError),
    /// The live checkpoint is not a valid readiness witness (lagging / wrong-seed / reorged tip).
    Readiness(ReadinessError),
    /// Materializing the fresh seed-state replay checkpoint failed.
    Materialize(ReducedCheckpointError),
    /// The atomic activation (derive -> predicate -> WAL -> promote) failed after the predicate
    /// passed -- a terminal halt.
    Activate(EpochViewActivationError),
}

/// S3f-4d-wire-3 (DC-EPOCH-11): the live boundary-activation orchestration -- the SINGLE entry
/// the relay loop calls when the durable tip has crossed an epoch boundary. Composes the dual-
/// path design: (1) extract + validate the durable selected-chain source window (the
/// AUTHORITATIVE input); (2) the live checkpoint readiness WITNESS (advanced-through + terminal
/// lineage); (3) materialize a FRESH seed-state replay checkpoint (the live one is never
/// mutated); (4) `activate_at_boundary` -- the sole authoritative derive (window replay) ->
/// predicate -> durable WAL activation -> atomic promote. Any failure is a TERMINAL
/// `ActivationError` (halt); a predicate decline is `NotYet` (the seed stays authoritative,
/// retry the next boundary). No peer/network/CLI/cache; bounds are explicit, never wall-clock.
#[allow(clippy::too_many_arguments)]
pub fn try_activate_at_boundary(
    live: &ReducedUtxoCheckpoint,
    chaindb: &dyn ChainDb,
    bounds: &WindowBounds,
    bootstrap_state: &LedgerState,
    network_magic: u32,
    nonce: Hash32,
    selected_point: &Point,
    transition_eligible: bool,
    active_view: &mut ActiveEpochView,
    scratch_path: &std::path::Path,
    wal_write: impl FnOnce(&WalEntry) -> bool,
) -> Result<BoundaryActivationOutcome, ActivationError> {
    let extract = extract_source_window(
        chaindb,
        bounds.source_epoch,
        bounds.source_window_start,
        bounds.source_window_end,
        bounds.snapshot_phase,
        bounds.source_window_anchor.clone(),
    )
    .map_err(ActivationError::SourceWindow)?;
    verify_live_readiness(live, &extract.window, bounds.expected_seed_slot, chaindb)
        .map_err(ActivationError::Readiness)?;
    let replay_cp = live
        .materialize_bootstrap_into(scratch_path)
        .map_err(ActivationError::Materialize)?;
    activate_at_boundary(
        &extract.window,
        &extract.window_blocks,
        &replay_cp,
        bootstrap_state,
        &extract.shelley_blocks,
        CardanoEra::Conway,
        network_magic,
        nonce,
        selected_point,
        transition_eligible,
        active_view,
        wal_write,
    )
    .map_err(ActivationError::Activate)
}

/// S3f-4d-wire-3b (DC-EPOCH-11): the GATED relay-loop orchestration. When `armed` is false (the
/// default until the live boundary proofs pass -- the boundary-aligned stake oracle + the
/// leadership-schedule lag proof), this is a strict NO-OP (`Ok(None)`) -- the live follow/forge
/// path is BYTE-IDENTICAL, no activation, no WAL write, no rebind. When armed AND the seed epoch
/// has COMPLETED (the durable tip located in a later epoch) AND no view is promoted yet, it
/// computes the explicit window bounds + runs the sole authoritative activation
/// (`try_activate_at_boundary`). Any terminal `ActivationError` propagates (halt). Idempotent:
/// a no-op once a view is promoted.
#[allow(clippy::too_many_arguments)]
pub fn maybe_activate_first_boundary(
    armed: bool,
    era_schedule: &EraSchedule,
    durable_tip_slot: SlotNo,
    seed_epoch: EpochNo,
    seed_point_slot: SlotNo,
    seed_point_hash: Hash32,
    live: &ReducedUtxoCheckpoint,
    chaindb: &dyn ChainDb,
    bootstrap_state: &LedgerState,
    network_magic: u32,
    nonce: Hash32,
    selected_point: &Point,
    active_view: &mut ActiveEpochView,
    scratch_path: &std::path::Path,
    wal_write: impl FnOnce(&WalEntry) -> bool,
) -> Result<Option<BoundaryActivationOutcome>, ActivationError> {
    // GATED OFF: until the live boundary proofs arm it, NO activation (byte-identical).
    if !armed {
        return Ok(None);
    }
    // idempotent: once a view is promoted, the first-boundary activation is done.
    if active_view.promoted().is_some() {
        return Ok(None);
    }
    // boundary detection: the seed epoch's window is complete only once the durable tip has
    // located into a LATER epoch (never the wall clock).
    let tip_epoch = match era_schedule.locate(durable_tip_slot) {
        Ok(loc) => loc.epoch,
        Err(_) => return Ok(None),
    };
    if tip_epoch.0 <= seed_epoch.0 {
        return Ok(None);
    }
    let bounds =
        match compute_first_window_bounds(era_schedule, seed_point_slot, seed_point_hash, seed_epoch) {
            Some(b) => b,
            None => return Ok(None),
        };
    let outcome = try_activate_at_boundary(
        live,
        chaindb,
        &bounds,
        bootstrap_state,
        network_magic,
        nonce,
        selected_point,
        true, // the boundary is reached -> the transition is eligible
        active_view,
        scratch_path,
        wal_write,
    )?;
    Ok(Some(outcome))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use ade_runtime::chaindb::{InMemoryChainDb, StoredBlock};

    const RAW_CONWAY_BLOCK: &[u8] =
        include_bytes!("../tests/fixtures/raw_era_block_conway.cbor");

    /// S3f-4d-wire-1: a single durable conway block in `[start, end]` extracts into a pinned,
    /// VALIDATED 1-block window (anchor == the block's prev_hash, lineage_pin == its hash), with
    /// its full ShelleyBlock captured for the replay derive.
    #[test]
    fn extract_source_window_pins_and_validates_a_durable_block() {
        // decode the fixture to learn its real slot/hash/prev_hash.
        let d = decode_block(RAW_CONWAY_BLOCK).unwrap();
        let slot = d.header_input.slot;
        let anchor = d.prev_hash.block_hash().cloned().unwrap_or(Hash32([0; 32]));
        let db = InMemoryChainDb::new();
        db.put_block(&StoredBlock {
            hash: d.block_hash.clone(),
            slot,
            bytes: RAW_CONWAY_BLOCK.to_vec(),
        })
        .unwrap();
        let out = extract_source_window(
            &db,
            EpochNo(100),
            slot,
            SlotNo(slot.0 + 10),
            SnapshotPhase::Set,
            anchor,
        )
        .expect("extract");
        assert_eq!(out.window_blocks.len(), 1);
        assert_eq!(out.shelley_blocks.len(), 1, "full ShelleyBlock captured for replay");
        assert_eq!(out.window.lineage_pin, d.block_hash, "pinned to the selected-chain tip");
        assert_eq!(out.window.target_epoch, EpochNo(102), "explicit source+2 target");
    }

    /// An empty range (no durable block in window) fails closed -- never a candidate.
    #[test]
    fn extract_source_window_empty_range_fails_closed() {
        let db = InMemoryChainDb::new();
        let err = extract_source_window(
            &db,
            EpochNo(100),
            SlotNo(1),
            SlotNo(10),
            SnapshotPhase::Set,
            Hash32([0; 32]),
        );
        assert!(matches!(err, Err(SourceWindowExtractError::Window(SourceWindowError::Empty))));
    }

    /// S3f-4d-wire-2: the live checkpoint is a readiness WITNESS -- ready iff it advanced THROUGH
    /// the window end AND the window's terminal point is still on the durable selected chain;
    /// fails closed (terminal) on lagging or a reorged-away terminal.
    #[test]
    fn verify_live_readiness_requires_advanced_through_and_terminal_present() {
        let dir = tempfile::tempdir().unwrap();
        let live = ReducedUtxoCheckpoint::open(&dir.path().join("live.redb")).unwrap();
        live.build_from(&std::collections::BTreeMap::new()).unwrap();
        live.seal_bootstrap(SlotNo(100)).unwrap();
        live.advance_block(SlotNo(300), &[], &[]).unwrap(); // live is at 300
        let d = decode_block(RAW_CONWAY_BLOCK).unwrap();
        let db = InMemoryChainDb::new();
        db.put_block(&StoredBlock {
            hash: d.block_hash.clone(),
            slot: SlotNo(250),
            bytes: RAW_CONWAY_BLOCK.to_vec(),
        })
        .unwrap();
        let window = ActivationSourceWindow {
            source_epoch: EpochNo(10),
            source_window_start: SlotNo(101),
            source_window_end: SlotNo(250),
            snapshot_phase: SnapshotPhase::Set,
            target_epoch: EpochNo(12),
            source_window_anchor: Hash32([1; 32]),
            lineage_pin: d.block_hash.clone(),
        };
        // advanced through 250 (live at 300) + terminal present -> READY.
        assert!(verify_live_readiness(&live, &window, SlotNo(100), &db).is_ok());
        // lagging: a window ending past the live tip -> terminal.
        let mut lagging = window.clone();
        lagging.source_window_end = SlotNo(400);
        assert!(matches!(
            verify_live_readiness(&live, &lagging, SlotNo(100), &db),
            Err(ReadinessError::Checkpoint(_))
        ));
        // terminal reorged away (lineage_pin not in the durable chain) -> terminal.
        let mut gone = window.clone();
        gone.lineage_pin = Hash32([0xff; 32]);
        assert!(matches!(
            verify_live_readiness(&live, &gone, SlotNo(100), &db),
            Err(ReadinessError::TerminalMissing)
        ));
    }

    /// S3f-4d-wire-3: the orchestration FAILS CLOSED (terminal) when the live checkpoint is
    /// lagging the source window -- it never promotes, the seed view stays authoritative.
    #[test]
    fn try_activate_at_boundary_lagging_live_is_terminal_and_keeps_seed() {
        use ade_ledger::state::LedgerState;
        let dir = tempfile::tempdir().unwrap();
        let live = ReducedUtxoCheckpoint::open(&dir.path().join("live.redb")).unwrap();
        live.build_from(&std::collections::BTreeMap::new()).unwrap();
        live.seal_bootstrap(SlotNo(100)).unwrap();
        live.advance_block(SlotNo(200), &[], &[]).unwrap(); // live only at 200
        let d = decode_block(RAW_CONWAY_BLOCK).unwrap();
        let db = InMemoryChainDb::new();
        db.put_block(&StoredBlock {
            hash: d.block_hash.clone(),
            slot: SlotNo(250),
            bytes: RAW_CONWAY_BLOCK.to_vec(),
        })
        .unwrap();
        let anchor = d.prev_hash.block_hash().cloned().unwrap_or(Hash32([0; 32]));
        let bounds = WindowBounds {
            source_epoch: EpochNo(10),
            source_window_start: SlotNo(101),
            source_window_end: SlotNo(500), // window ends past the live tip -> lagging
            snapshot_phase: SnapshotPhase::Set,
            source_window_anchor: anchor,
            expected_seed_slot: SlotNo(100),
        };
        let state = LedgerState::new(CardanoEra::Conway);
        let mut active = ActiveEpochView::new();
        let r = try_activate_at_boundary(
            &live,
            &db,
            &bounds,
            &state,
            2,
            Hash32([0; 32]),
            &Point { slot: SlotNo(600), hash: Hash32([0xab; 32]) },
            true,
            &mut active,
            &dir.path().join("scratch.redb"),
            |_| true,
        );
        assert!(
            matches!(r, Err(ActivationError::Readiness(ReadinessError::Checkpoint(_)))),
            "a lagging live checkpoint is a TERMINAL readiness failure, not a promotion"
        );
        assert!(matches!(active, ActiveEpochView::Seed), "the seed view stays authoritative");
    }

    /// S3f-4d-wire-3b: the first-activation window spans the completed SEED epoch -- from the
    /// block after the seed point to the seed epoch's last slot -- computed from the era
    /// schedule (no wall clock); a wrong seed epoch fails to None.
    #[test]
    fn compute_first_window_bounds_spans_the_seed_epoch() {
        use ade_core::consensus::era_schedule::{BootstrapAnchorHash, EraSchedule, EraSummary};
        let eras = vec![EraSummary {
            era: CardanoEra::Conway,
            start_slot: SlotNo(8_640_000),
            start_epoch: EpochNo(100),
            slot_length_ms: 1000,
            epoch_length_slots: 86_400,
            safe_zone_slots: 4320,
        }];
        let sched = EraSchedule::new(BootstrapAnchorHash(Hash32([0; 32])), 0, eras).unwrap();
        let seed_slot = SlotNo(8_640_000 + 50_000); // mid epoch 100
        let b = compute_first_window_bounds(&sched, seed_slot, Hash32([7; 32]), EpochNo(100)).unwrap();
        assert_eq!(b.source_epoch, EpochNo(100));
        assert_eq!(b.source_window_start, SlotNo(8_640_000 + 50_001), "first block after the seed");
        assert_eq!(b.source_window_end, SlotNo(8_640_000 + 86_400 - 1), "the seed epoch's last slot");
        assert_eq!(b.expected_seed_slot, seed_slot);
        assert_eq!(b.source_window_anchor, Hash32([7; 32]));
        // a seed point that does not locate in the claimed seed epoch -> None.
        assert!(compute_first_window_bounds(&sched, seed_slot, Hash32([7; 32]), EpochNo(101)).is_none());
    }

    /// S3f-4d-wire-3b: the GATED orchestration is a strict NO-OP when not armed (byte-identical
    /// live path), and also when armed but the seed epoch's window is not yet complete -- it
    /// NEVER promotes in either case.
    #[test]
    fn maybe_activate_first_boundary_gated_off_and_pre_boundary_are_noops() {
        use ade_core::consensus::era_schedule::{BootstrapAnchorHash, EraSchedule, EraSummary};
        use ade_ledger::state::LedgerState;
        let dir = tempfile::tempdir().unwrap();
        let live = ReducedUtxoCheckpoint::open(&dir.path().join("live.redb")).unwrap();
        live.build_from(&std::collections::BTreeMap::new()).unwrap();
        live.seal_bootstrap(SlotNo(8_640_000 + 50_000)).unwrap();
        let db = InMemoryChainDb::new();
        let eras = vec![EraSummary {
            era: CardanoEra::Conway,
            start_slot: SlotNo(8_640_000),
            start_epoch: EpochNo(100),
            slot_length_ms: 1000,
            epoch_length_slots: 86_400,
            safe_zone_slots: 4320,
        }];
        let sched = EraSchedule::new(BootstrapAnchorHash(Hash32([0; 32])), 0, eras).unwrap();
        let state = LedgerState::new(CardanoEra::Conway);
        let seed_slot = SlotNo(8_640_000 + 50_000);
        let pt = Point { slot: SlotNo(9_000_000), hash: Hash32([1; 32]) };
        // (1) GATED OFF (armed=false) -> no-op even with the tip PAST the boundary (epoch 101).
        let mut av = ActiveEpochView::new();
        let r = maybe_activate_first_boundary(
            false, &sched, SlotNo(8_640_000 + 90_000), EpochNo(100), seed_slot, Hash32([7; 32]),
            &live, &db, &state, 2, Hash32([0; 32]), &pt, &mut av, &dir.path().join("s1.redb"), |_| true,
        );
        assert!(matches!(r, Ok(None)), "gated off -> no activation");
        assert!(matches!(av, ActiveEpochView::Seed), "the seed view is untouched");
        // (2) armed, but the tip is STILL in the seed epoch (the window is not complete) -> no-op.
        let mut av2 = ActiveEpochView::new();
        let r2 = maybe_activate_first_boundary(
            true, &sched, SlotNo(8_640_000 + 60_000), EpochNo(100), seed_slot, Hash32([7; 32]),
            &live, &db, &state, 2, Hash32([0; 32]), &pt, &mut av2, &dir.path().join("s2.redb"), |_| true,
        );
        assert!(matches!(r2, Ok(None)), "pre-boundary -> no activation");
        assert!(matches!(av2, ActiveEpochView::Seed));
    }
}
