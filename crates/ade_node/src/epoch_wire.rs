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
}
