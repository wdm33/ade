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
use ade_ledger::reduced_snapshot::SnapshotPhase;
use ade_runtime::chaindb::ChainDb;
use ade_types::shelley::block::ShelleyBlock;
use ade_types::{EpochNo, Hash32, SlotNo};

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
}
