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
use ade_ledger::reduced_epoch_view::{consensus_profile_commitment, EpochConsensusView};
use ade_ledger::reduced_snapshot::SnapshotPhase;
use ade_ledger::state::LedgerState;
use ade_runtime::chaindb::{
    ChainDb, CheckpointReadinessError, ReducedCheckpointError, ReducedUtxoCheckpoint,
};
use ade_types::shelley::block::ShelleyBlock;
use ade_types::{CardanoEra, EpochNo, Hash32, SlotNo};

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::events::Point;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_ledger::bootstrap_bridge::BootstrapNextEpochAuthority;
use ade_ledger::wal::WalEntry;
use ade_types::tx::{Coin, PoolId};

use crate::epoch_activate::{activate_at_boundary, recover_at_boundary, BoundaryActivationOutcome};
use crate::epoch_activation::{
    activation_record_for, ActiveEpochAuthority, EpochViewActivationError,
};
use crate::epoch_candidate::{derive_candidate, CandidateDeriveError, CandidateProfile};
use crate::epoch_source_window::{
    target_epoch_for_source, validate_source_window, ActivationSourceWindow, SourceWindowBlock,
    SourceWindowError,
};

/// EPOCH-CONTINUITY-ACTIVATION ECA-1 (DC-EPOCH-13): the SEED-derived activation inputs the relay
/// loop holds for the first-boundary activation -- bound ONCE at bootstrap to the manifest-bound
/// seed. The seed `bootstrap_state` is the cert state at the bootstrap point (the window replay
/// advances it); the relay loop's advanced ledger is NOT it. Activation is AUTOMATIC: there is NO
/// arming flag; the only gate is the deterministic predicate over canonical durable state.
pub struct EviewActivationInputs {
    pub seed_bootstrap_state: LedgerState,
    pub seed_point_slot: SlotNo,
    pub seed_point_hash: Hash32,
    pub seed_epoch: EpochNo,
    pub network_magic: u32,
    pub nonce: Hash32,
    /// The canonical leadership consensus profile (ECA-0b), bound ONCE at bootstrap: the genesis
    /// hash + the protocol-params hash + the ASC. The candidate's `protocol_params_commitment` is
    /// computed from these; the projection verifies against them. All already-bound canonical values
    /// (no filesystem/config/network read in derivation).
    pub genesis_hash: Hash32,
    pub protocol_params_hash: Hash32,
    pub asc: ActiveSlotsCoeff,
    /// The durable path for the FRESH replay checkpoint the authoritative window replay
    /// materializes (a separate redb -- never the live checkpoint).
    pub replay_scratch_path: std::path::PathBuf,
    /// ECA-5 (DC-EPOCH-15): the one-time bootstrap BRIDGE -- the seed+1 leadership projected from the
    /// imported MARK snapshot, recovered from durable storage at relay-loop start (first-run OR warm-
    /// start). `Some` for a native-Mithril-started node; the seam REQUIRES it for the first boundary.
    pub next_epoch_bridge: Option<BootstrapNextEpochAuthority>,
    /// Option B (B3c): the snapshot-bound bootstrap reward update, recovered from durable storage at
    /// relay-loop start (first-run OR warm-start), bound to the SAME `anchor_fp` as the seed sidecar.
    /// REQUIRED for the seed+2 authority (the first replay-derived leader schedule); the seam fails
    /// closed if it is absent at that boundary. Applied at the WINDOW-END of the seed+2 replay (the
    /// `CandidateProfile` carries it to `derive_candidate`), NEVER mutated into the seed cert state.
    pub bootstrap_reward_delta:
        Option<ade_ledger::bootstrap_reward_update::BootstrapRewardUpdate>,
}

impl EviewActivationInputs {
    /// Run the first-boundary activation with the loop's runtime args. A strict NO-OP (byte-
    /// identical) pre-boundary or once a view is promoted; a terminal `ActivationError`
    /// propagates (halt). Activation is AUTOMATIC -- no arming flag (DC-EPOCH-13).
    #[allow(clippy::too_many_arguments)]
    pub fn maybe_activate(
        &self,
        era_schedule: &EraSchedule,
        durable_tip_slot: SlotNo,
        live: &ReducedUtxoCheckpoint,
        chaindb: &dyn ChainDb,
        selected_point: &Point,
        active_view: &mut ActiveEpochAuthority,
        scratch_path: &std::path::Path,
        wal_write: impl FnOnce(&WalEntry) -> bool,
    ) -> Result<Option<BoundaryActivationOutcome>, ActivationError> {
        maybe_activate_first_boundary(
            era_schedule,
            durable_tip_slot,
            self.seed_epoch,
            self.seed_point_slot,
            self.seed_point_hash.clone(),
            live,
            chaindb,
            &self.seed_bootstrap_state,
            self.network_magic,
            self.nonce.clone(),
            self.genesis_hash.clone(),
            self.protocol_params_hash.clone(),
            self.asc,
            self.bootstrap_reward_delta.as_ref(),
            selected_point,
            active_view,
            scratch_path,
            wal_write,
        )
    }
}

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
#[allow(clippy::too_many_arguments)]
pub fn derive_authoritative_candidate(
    live: &ReducedUtxoCheckpoint,
    window: &ActivationSourceWindow,
    shelley_blocks: &[ShelleyBlock],
    bootstrap_state: &LedgerState,
    network_magic: u32,
    nonce: Hash32,
    profile: &CandidateProfile,
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
        profile,
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
    /// The source epoch's length (slots) — for the window driver's boundary detection (ECA-0b).
    pub slots_per_epoch: u64,
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
        slots_per_epoch: epoch_len,
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
    /// ECA-5: the first post-boundary candidate's slot skips BEYOND the seed epoch's immediate
    /// successor (N+2 or later) -- a far-future candidate must never force promotion; terminal.
    CandidateSlotSkipsBoundary { candidate_epoch: EpochNo, seed_epoch: EpochNo },
    /// ECA-5: the first post-boundary candidate's parent does NOT bind to the durable selected tip --
    /// a forked / non-extending candidate must not force promotion; terminal.
    CandidateParentNotDurableTip { candidate_parent: Hash32, durable_tip: Hash32 },
    /// ECA-5: catching the live reduced checkpoint up to the durable tip (the seed-epoch window the
    /// readiness witness needs) failed -- terminal.
    LiveCheckpointAdvance(String),
    /// ECA-5 (DC-EPOCH-15): the first boundary needs the bootstrap bridge (seed+1) but none was
    /// recovered -- a native-Mithril node MUST have persisted it at bootstrap; terminal (no fallback).
    BridgeMissing { target_epoch: EpochNo },
    /// ECA-5: the recovered bridge answers for a DIFFERENT epoch than the first-boundary candidate --
    /// terminal (the bridge is bound to seed+1 alone).
    BridgeEpochMismatch {
        bridge_epoch: EpochNo,
        candidate_epoch: EpochNo,
    },
    /// ECA-5: projecting the bridge view to a `PoolDistrView` (or the WAL activation-record write)
    /// failed -- terminal.
    BridgeProjection(String),
    /// DC-EPOCH-17 (B3): the boundary-2+ window-replay preparation failed -- the eta0 boundary tick
    /// over the live chain-dep, the C-2 window bounds, or the (not-yet-wired) beyond-seed+2 case.
    /// Terminal: there is no silent bridge fallback past seed+1.
    WindowReplayPrepare(String),
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
    profile: &CandidateProfile,
    selected_point: &Point,
    transition_eligible: bool,
    active_view: &mut ActiveEpochAuthority,
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
        profile,
        selected_point,
        chaindb,
        transition_eligible,
        active_view,
        wal_write,
    )
    .map_err(ActivationError::Activate)
}

/// EPOCH-CONTINUITY-ACTIVATION ECA-1 (DC-EPOCH-13): the relay-loop boundary-activation
/// orchestration. Activation is AUTOMATIC and DETERMINISTIC -- there is NO arming flag. It is a
/// strict NO-OP (`Ok(None)`, byte-identical) until the seed epoch's window is COMPLETE (the
/// durable tip located in a LATER epoch -- never the wall clock) and no view is promoted yet;
/// then it computes the explicit window bounds + runs the sole authoritative activation
/// (`try_activate_at_boundary`). Any terminal `ActivationError` propagates (halt). Idempotent:
/// a no-op once a view is promoted. The ONLY gate is the deterministic activation predicate over
/// canonical durable state.
#[allow(clippy::too_many_arguments)]
pub fn maybe_activate_first_boundary(
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
    genesis_hash: Hash32,
    protocol_params_hash: Hash32,
    asc: ActiveSlotsCoeff,
    bootstrap_reward_update: Option<&ade_ledger::bootstrap_reward_update::BootstrapRewardUpdate>,
    selected_point: &Point,
    active_view: &mut ActiveEpochAuthority,
    scratch_path: &std::path::Path,
    wal_write: impl FnOnce(&WalEntry) -> bool,
) -> Result<Option<BoundaryActivationOutcome>, ActivationError> {
    // idempotent: once a view is promoted, the first-boundary activation is done.
    if active_view.is_promoted() {
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
    let profile = CandidateProfile {
        slots_per_epoch: bounds.slots_per_epoch,
        genesis_hash,
        protocol_params_hash,
        asc,
        bootstrap_reward_update: bootstrap_reward_update.cloned(),
        seed_epoch,
    };
    let outcome = try_activate_at_boundary(
        live,
        chaindb,
        &bounds,
        bootstrap_state,
        network_magic,
        nonce,
        &profile,
        selected_point,
        true, // the boundary is reached -> the transition is eligible
        active_view,
        scratch_path,
        wal_write,
    )?;
    Ok(Some(outcome))
}

/// ECA-5 authority-preparation seam: prepare the N+1 epoch authority for the FIRST post-boundary
/// candidate, BEFORE that candidate's forecast/leader validation. The incoming header supplies ONLY its
/// slot + parent (which durable transition to prepare) -- never stake/nonce/authority. The promotion
/// derives EXCLUSIVELY from durable state (compute_first_window_bounds + try_activate_at_boundary over
/// the N reduced checkpoint + canonical N window blocks + v4 sidecar geometry + cert-state/lineage), so
/// it is replay-deterministic. Returns Ok(true) if it promoted (the caller MUST then extend the forecast
/// schedule), Ok(false) for a no-op (not at the boundary / already promoted), or a TERMINAL
/// ActivationError on a guard violation (parent not binding the durable tip, candidate skipping N+1).
/// Replaces the deadlocked durable-tip-in-N+1 trigger for the FOLLOWER (a follower must validate the
/// first N+1 header before it can admit it, so the tip can never enter N+1 on its own).
#[allow(clippy::too_many_arguments)]
pub fn prepare_authority_for_candidate_slot(
    inputs: &EviewActivationInputs,
    era_schedule: &EraSchedule,
    durable_tip_slot: SlotNo,
    durable_tip_hash: &Hash32,
    candidate_slot: SlotNo,
    candidate_parent: &Hash32,
    live: &ReducedUtxoCheckpoint,
    chaindb: &dyn ChainDb,
    chain_dep: &ade_core::consensus::praos_state::PraosChainDepState,
    active_view: &mut ActiveEpochAuthority,
    wal_write: impl FnOnce(&WalEntry) -> bool,
) -> Result<bool, ActivationError> {
    let seed_epoch = inputs.seed_epoch;
    // The epoch the CURRENT authority answers for: the seed epoch N before any promotion, else the
    // promoted epoch P. The seam ADVANCES ONE boundary per call (DC-EPOCH-17), so the boundary-1 weld
    // (tip/candidate == seed) generalizes to (tip/candidate == the current authority's epoch).
    let current_epoch = active_view.epoch();
    // (a) the durable SELECTED tip must be in epoch C-1 (= the current authority's epoch); the C-1
    // window is complete only at its end (boundary 1: C-1 == seed; boundary 2: C-1 == seed+1).
    let tip_epoch = match era_schedule.locate(durable_tip_slot) {
        Ok(loc) => loc.epoch,
        Err(_) => return Ok(false),
    };
    if tip_epoch.0 != current_epoch.0 {
        return Ok(false);
    }
    // (b) the candidate slot's epoch C.
    let candidate_epoch = match era_schedule.locate(candidate_slot) {
        Ok(loc) => loc.epoch,
        Err(_) => return Ok(false),
    };
    if candidate_epoch.0 <= current_epoch.0 {
        return Ok(false); // still in the current authority's epoch -- not a boundary candidate.
    }
    // the boundary must advance EXACTLY one epoch (P -> P+1) -- a skip is terminal.
    if candidate_epoch.0 != current_epoch.0 + 1 {
        return Err(ActivationError::CandidateSlotSkipsBoundary {
            candidate_epoch,
            seed_epoch: current_epoch,
        });
    }
    // (c) the candidate parent must bind to the durable selected tip -- no fork can force promotion.
    if candidate_parent != durable_tip_hash {
        return Err(ActivationError::CandidateParentNotDurableTip {
            candidate_parent: candidate_parent.clone(),
            durable_tip: durable_tip_hash.clone(),
        });
    }

    // WINDOW-REPLAY (DC-EPOCH-17, boundary 2+): for a candidate past seed+1 the seed+C authority is
    // replay(C-2). For boundary 2 (C == seed+2) that is the seed (N) window Ade followed from bootstrap.
    // eta0(C) is the chain-dep epoch tick over the LIVE chain-dep (candidate (X) last_epoch_block_nonce
    // -- the value the boundary-2 live gate proved, DC-EPOCH-16); node_sync applies the SAME tick to the
    // chain-dep after, so the bound view's nonce and the live chain-dep agree by construction. Boundary
    // 3+ (the general C-2 window) is the continuous-crossing refinement -- fail closed here.
    if candidate_epoch.0 >= seed_epoch.0 + 2 {
        if candidate_epoch.0 != seed_epoch.0 + 2 {
            return Err(ActivationError::WindowReplayPrepare(format!(
                "window-replay beyond seed+2 not yet wired (candidate {candidate_epoch:?}, seed {seed_epoch:?})"
            )));
        }
        // Option B (B3c, DC-EPOCH-18): the seed+2 authority's stake snapshot REQUIRES the snapshot-bound
        // bootstrap reward update; the fail-closed (absent / wrong-epoch is terminal) is now enforced
        // MECHANICALLY at the single derivation site (derive_candidate over the seed window), so it
        // cannot drift across the activate / recover / first-boundary callers. The update is carried on
        // the profile below.
        let ticked = ade_core::consensus::apply_nonce_input(
            chain_dep,
            &ade_core::consensus::NonceInput::EpochBoundary {
                new_epoch: candidate_epoch,
            },
        )
        .map_err(|e| ActivationError::WindowReplayPrepare(format!("eta0 boundary tick: {e:?}")))?;
        let eta0 = ticked.epoch_nonce.0.clone();
        let bounds = compute_first_window_bounds(
            era_schedule,
            inputs.seed_point_slot,
            inputs.seed_point_hash.clone(),
            seed_epoch,
        )
        .ok_or_else(|| {
            ActivationError::WindowReplayPrepare("seed window bounds unavailable".to_string())
        })?;
        let profile = CandidateProfile {
            slots_per_epoch: bounds.slots_per_epoch,
            genesis_hash: inputs.genesis_hash.clone(),
            protocol_params_hash: inputs.protocol_params_hash.clone(),
            asc: inputs.asc,
            bootstrap_reward_update: inputs.bootstrap_reward_delta.clone(),
            seed_epoch,
        };
        let selected_point = Point {
            slot: durable_tip_slot,
            hash: durable_tip_hash.clone(),
        };
        try_activate_at_boundary(
            live,
            chaindb,
            &bounds,
            &inputs.seed_bootstrap_state,
            inputs.network_magic,
            eta0,
            &profile,
            &selected_point,
            true,
            active_view,
            &inputs.replay_scratch_path,
            wal_write,
        )?;
        // B3b (DC-EPOCH-17): return DID-THIS-CALL-ADVANCE (not is_promoted). A window-replay that
        // declines (NotYet -- e.g. the source is not yet ancestor-or-equal of the selected tip)
        // leaves the authority at the current epoch and MUST NOT be reported as a boundary crossing.
        return Ok(active_view.epoch().0 > current_epoch.0);
    }

    // (a)+(b)+(c) => the durable tip IS the last N block; the canonical N window is COMPLETE. Promote the
    // N+1 authority from the durable BRIDGE -- the seed+1 leadership projected from the imported MARK
    // snapshot at bootstrap (DC-EPOCH-15). The bridge is REQUIRED here; there is NO fallback to the +2
    // window-replay (the leadership snapshot lag makes that derive seed+2, not seed+1). The live reduced
    // checkpoint is NOT advanced here -- the first N+1 block's txs validate against the durable-tip UTxO,
    // and the relay loop advances the checkpoint as it admits, so no pre-seal is needed.
    let _ = (live, chaindb);
    let bridge = match inputs.next_epoch_bridge.as_ref() {
        Some(b) => b,
        None => {
            return Err(ActivationError::BridgeMissing {
                target_epoch: candidate_epoch,
            })
        }
    };
    if bridge.target_epoch.0 != candidate_epoch.0 {
        return Err(ActivationError::BridgeEpochMismatch {
            bridge_epoch: bridge.target_epoch,
            candidate_epoch,
        });
    }
    // Bind the N+1 view from the bridge (phase = Mark: the seed+1 leadership IS the MARK snapshot).
    let mut stake_by_pool: std::collections::BTreeMap<PoolId, Coin> = std::collections::BTreeMap::new();
    let mut pool_vrf_keyhashes: std::collections::BTreeMap<PoolId, Hash32> =
        std::collections::BTreeMap::new();
    for (keyhash, entry) in &bridge.pool_distribution {
        let pool = PoolId(keyhash.clone());
        stake_by_pool.insert(pool.clone(), Coin(entry.active_stake));
        pool_vrf_keyhashes.insert(pool, entry.vrf_keyhash.clone());
    }
    let profile_commitment = consensus_profile_commitment(
        &bridge.genesis_hash,
        &bridge.protocol_params_hash,
        bridge.active_slots_coeff,
    );
    let source = EpochConsensusView::bind(
        inputs.network_magic,
        CardanoEra::Conway,
        bridge.target_epoch,
        Point {
            slot: bridge.source_point_slot,
            hash: bridge.source_point_hash.clone(),
        },
        bridge.canonical_commitment.clone(),
        bridge.epoch_nonce.0.clone(),
        SnapshotPhase::Mark,
        stake_by_pool,
        pool_vrf_keyhashes,
        Coin(bridge.total_active_stake),
        profile_commitment,
    );
    let projected = source
        .to_pool_distr_view(
            &bridge.genesis_hash,
            &bridge.protocol_params_hash,
            bridge.active_slots_coeff,
        )
        .map_err(|e| ActivationError::BridgeProjection(format!("{e:?}")))?;
    // Durable-before-visible: write the WAL activation record BEFORE publishing the active view.
    let record = activation_record_for(&source);
    if !wal_write(&record) {
        return Err(ActivationError::BridgeProjection(
            "wal activation-record write rejected".to_string(),
        ));
    }
    active_view
        .advance(source, projected)
        .map_err(ActivationError::Activate)?;
    // B3b (DC-EPOCH-17): DID-THIS-CALL-ADVANCE (the bridge always advances Seed -> seed+1 here).
    Ok(active_view.epoch().0 > current_epoch.0)
}

/// EPOCH-CONTINUITY-ACTIVATION ECA-4 (DC-EPOCH-06): the WARM-START recovery twin of
/// [`try_activate_at_boundary`]. SAME assembly (extract the durable source window → verify live
/// readiness → materialize the replay checkpoint), then [`recover_at_boundary`] (re-derive + recover
/// against the durable `record`) instead of the live activate. Promotes the authority from the
/// VERIFIED record, or a TERMINAL `ActivationError` on a mismatch / un-recomputable candidate.
#[allow(clippy::too_many_arguments)]
pub fn try_recover_at_boundary(
    live: &ReducedUtxoCheckpoint,
    chaindb: &dyn ChainDb,
    bounds: &WindowBounds,
    bootstrap_state: &LedgerState,
    network_magic: u32,
    nonce: Hash32,
    profile: &CandidateProfile,
    record: &WalEntry,
    active_view: &mut ActiveEpochAuthority,
    scratch_path: &std::path::Path,
) -> Result<(), ActivationError> {
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
    recover_at_boundary(
        &extract.window,
        &extract.window_blocks,
        &replay_cp,
        bootstrap_state,
        &extract.shelley_blocks,
        CardanoEra::Conway,
        network_magic,
        nonce,
        profile,
        record,
        active_view,
    )
    .map_err(ActivationError::Activate)
}

/// EPOCH-CONTINUITY-ACTIVATION ECA-4 (DC-EPOCH-06): warm-start recovery of a promoted authority, run
/// BEFORE the relay loop. Given the resolved durable activation `record` (or `None`), it re-derives
/// the candidate from the SAME first-boundary window replay + recovers (reject-non-recomputable) +
/// promotes the ONE authority. A `None` record is a no-op (Seed stays — a crash before the durable
/// WAL keeps the old epoch active). Idempotent: a no-op once promoted, so the live first-boundary
/// re-fire is itself idempotent on the recovered authority. The bounds / profile are computed EXACTLY
/// as the live `maybe_activate_first_boundary` (the recorded target epoch is the seed epoch's
/// successor), so the re-derivation is byte-identical to the live one.
#[allow(clippy::too_many_arguments)]
pub fn maybe_recover_promoted_authority(
    record: Option<&WalEntry>,
    era_schedule: &EraSchedule,
    seed_epoch: EpochNo,
    seed_point_slot: SlotNo,
    seed_point_hash: Hash32,
    live: &ReducedUtxoCheckpoint,
    chaindb: &dyn ChainDb,
    bootstrap_state: &LedgerState,
    network_magic: u32,
    nonce: Hash32,
    genesis_hash: Hash32,
    protocol_params_hash: Hash32,
    asc: ActiveSlotsCoeff,
    bootstrap_reward_update: Option<&ade_ledger::bootstrap_reward_update::BootstrapRewardUpdate>,
    active_view: &mut ActiveEpochAuthority,
    scratch_path: &std::path::Path,
) -> Result<(), ActivationError> {
    // idempotent: nothing to recover once promoted.
    if active_view.is_promoted() {
        return Ok(());
    }
    // no durable activation record => Seed stays authoritative (a crash before the WAL).
    let Some(record) = record else {
        return Ok(());
    };
    // Option B (B3c, DC-EPOCH-18): reproducing the recovered seed+2 authority REQUIRES the
    // snapshot-bound bootstrap reward update; the fail-closed (absent / wrong-epoch is terminal) is
    // enforced MECHANICALLY at the shared derivation site (derive_candidate over the seed window), so a
    // legacy store (no rupd sidecar, e.g. a mutated post-RUPD seed) cannot re-derive this authority via
    // the accidental-correctness path. The update is carried on the profile below.
    // a durable record is present (checked above), so the boundary WAS crossed and its first-boundary
    // window WAS computable when the record was written. If the bounds are now uncomputable from the
    // SAME durable seed point/epoch, the store is inconsistent with the record -- the recorded promotion
    // CANNOT be reproduced => TERMINAL (reject-non-recomputable surfaces at the recovery seam, louder +
    // earlier, never a deferred no-op). (The live path's bounds-None IS a no-op -- there is no record.)
    let bounds =
        match compute_first_window_bounds(era_schedule, seed_point_slot, seed_point_hash, seed_epoch) {
            Some(b) => b,
            None => {
                return Err(ActivationError::Activate(
                    EpochViewActivationError::EpochViewPostPromotionMismatch,
                ))
            }
        };
    let profile = CandidateProfile {
        slots_per_epoch: bounds.slots_per_epoch,
        genesis_hash,
        protocol_params_hash,
        asc,
        bootstrap_reward_update: bootstrap_reward_update.cloned(),
        seed_epoch,
    };
    try_recover_at_boundary(
        live,
        chaindb,
        &bounds,
        bootstrap_state,
        network_magic,
        nonce,
        &profile,
        record,
        active_view,
        scratch_path,
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use ade_runtime::chaindb::{InMemoryChainDb, StoredBlock};

    fn pdv() -> ade_ledger::consensus_view::PoolDistrView {
        ade_ledger::consensus_view::PoolDistrView::new(
            EpochNo(100),
            0,
            ActiveSlotsCoeff { numer: 1, denom: 20 },
            std::collections::BTreeMap::new(),
        )
    }

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
            slots_per_epoch: 86_400,
        };
        let state = LedgerState::new(CardanoEra::Conway);
        let active_sv = pdv();
        let mut active = ActiveEpochAuthority::seed(&active_sv);
        let profile = CandidateProfile {
            slots_per_epoch: 86_400,
            genesis_hash: Hash32([0x91; 32]),
            protocol_params_hash: Hash32([0x92; 32]),
            asc: ActiveSlotsCoeff { numer: 1, denom: 20 },
            bootstrap_reward_update: None,
            // window source is 10 -> not the seed+2 window -> the rupd gate is a no-op.
            seed_epoch: EpochNo(0),
        };
        let r = try_activate_at_boundary(
            &live,
            &db,
            &bounds,
            &state,
            2,
            Hash32([0; 32]),
            &profile,
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
        assert!(!active.is_promoted(), "the seed view stays authoritative");
    }

    /// S3f-4d-wire-3b: the first-activation window spans the completed SEED epoch -- from the
    /// block after the seed point to the seed epoch's last slot -- computed from the era
    /// schedule (no wall clock); a wrong seed epoch fails to None.
    #[test]
    fn compute_first_window_bounds_spans_the_seed_epoch() {
        use ade_core::consensus::era_schedule::{BootstrapAnchorHash, EraSchedule, EraSummary};
        let eras = vec![EraSummary {
            randomness_stabilisation_window_slots: None,
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

    /// EPOCH-CONTINUITY-ACTIVATION ECA-1 (DC-EPOCH-13): with the arming flag REMOVED, the
    /// orchestration is a strict NO-OP only while the seed epoch's window is incomplete (the tip
    /// still in the seed epoch); once the boundary is CROSSED it AUTOMATICALLY drives the sole
    /// authoritative activation -- here it FAILS CLOSED (terminal) on an empty durable window,
    /// proving it proceeds by the deterministic predicate over canonical state, NOT by any flag,
    /// and never promotes against an unproven state (the seed view stays authoritative).
    #[test]
    fn maybe_activate_first_boundary_is_automatic_and_fails_closed_not_flag_gated() {
        use ade_core::consensus::era_schedule::{BootstrapAnchorHash, EraSchedule, EraSummary};
        use ade_ledger::state::LedgerState;
        let dir = tempfile::tempdir().unwrap();
        let live = ReducedUtxoCheckpoint::open(&dir.path().join("live.redb")).unwrap();
        live.build_from(&std::collections::BTreeMap::new()).unwrap();
        live.seal_bootstrap(SlotNo(8_640_000 + 50_000)).unwrap();
        let db = InMemoryChainDb::new();
        let eras = vec![EraSummary {
            randomness_stabilisation_window_slots: None,
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
        // (1) PRE-BOUNDARY: the tip is STILL in the seed epoch (the window is not complete) -> a
        //     strict no-op (byte-identical); the seed view is untouched.
        let av_sv = pdv();
        let mut av = ActiveEpochAuthority::seed(&av_sv);
        let r = maybe_activate_first_boundary(
            &sched, SlotNo(8_640_000 + 60_000), EpochNo(100), seed_slot, Hash32([7; 32]),
            &live, &db, &state, 2, Hash32([0; 32]), Hash32([0x91; 32]), Hash32([0x92; 32]), ActiveSlotsCoeff { numer: 1, denom: 20 }, None, &pt, &mut av, &dir.path().join("s1.redb"), |_| true,
        );
        assert!(matches!(r, Ok(None)), "pre-boundary -> no activation");
        assert!(!av.is_promoted(), "the seed view is untouched");
        // (2) BOUNDARY CROSSED (the tip located in epoch 101): activation is AUTOMATIC -- with no
        //     flag it proceeds to the authoritative window replay and FAILS CLOSED on the empty
        //     durable window (terminal), never no-opping and never promoting against an unproven
        //     state. Before ECA-1 this same call would have returned Ok(None) on `armed == false`.
        let av2_sv = pdv();
        let mut av2 = ActiveEpochAuthority::seed(&av2_sv);
        let r2 = maybe_activate_first_boundary(
            &sched, SlotNo(8_640_000 + 90_000), EpochNo(100), seed_slot, Hash32([7; 32]),
            &live, &db, &state, 2, Hash32([0; 32]), Hash32([0x91; 32]), Hash32([0x92; 32]), ActiveSlotsCoeff { numer: 1, denom: 20 }, None, &pt, &mut av2, &dir.path().join("s2.redb"), |_| true,
        );
        assert!(
            matches!(r2, Err(ActivationError::SourceWindow(_))),
            "a crossed boundary AUTOMATICALLY drives the activation (fail-closed on an empty window), not a flag no-op"
        );
        assert!(!av2.is_promoted(), "fail-closed: no promotion, the seed stays authoritative");
    }
}
