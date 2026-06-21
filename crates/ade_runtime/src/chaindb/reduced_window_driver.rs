//! EPOCH-CONSENSUS-VIEW S3f-2 (DC-EVIEW-08) — the window driver: advance the reduced
//! UTxO checkpoint + the cert/delegation state forward over a window of blocks, then
//! aggregate per-pool stake.
//!
//! This is the orchestration that ties the proven pieces into one pass over a real
//! epoch's blocks:
//!   1. per block: `reduced_block_delta` (== reduce(track_utxo), proven) -> the
//!      checkpoint's `apply_block_delta`; and `advance_cert_state`
//!      (== `process_block_certificates`) to carry the delegation/reward state forward;
//!   2. once: `sum_base_credential_stake` (the per-base-credential UTxO coin sums) ->
//!      `aggregate_pool_stake` over the advanced delegation map.
//!
//! The starting cert state is the manifest-bound bootstrap cert state (DC-EVIEW-09), so
//! the advanced delegation map includes PRE-bootstrap delegators -- the window does NOT
//! start from an empty map. The starting UTxO is the bootstrap reduced checkpoint. So the
//! aggregate is over Ade's OWN complete state, not a cardano-node import.
//!
//! RED orchestration: it reads/writes the durable redb checkpoint (I/O) and clones a
//! `LedgerState` per call (a transient window operation, never the hot live path). The
//! per-step transforms it sequences are deterministic and individually proven.

use super::reduced_utxo_checkpoint::{ReducedCheckpointError, ReducedUtxoCheckpoint};
use ade_ledger::delegation::{apply_pool_reap, PoolParams};
use ade_ledger::error::LedgerError;
use ade_ledger::reduced_advance::{advance_cert_state, reduced_block_delta};
use ade_ledger::reduced_aggregate::{aggregate_pool_stake, AggregateError, StakeByPool};
use ade_ledger::state::LedgerState;
use ade_types::shelley::block::ShelleyBlock;
use ade_types::tx::PoolId;
use ade_types::{CardanoEra, EpochNo};
use std::collections::BTreeMap;

/// Fail-closed reasons the window drive cannot produce an aggregate. Every variant aborts
/// the window without producing a partial / wrong stake distribution.
#[derive(Debug)]
pub enum WindowDriverError {
    /// The reduced-UTxO checkpoint store rejected an apply / read.
    Checkpoint(ReducedCheckpointError),
    /// A block's reduced delta or cert advance failed (authority-fatal ledger error).
    Ledger(LedgerError),
    /// The per-pool aggregation overflowed (`checked_add` guard).
    Aggregate(AggregateError),
    /// `slots_per_epoch` was zero (a misconfigured era schedule) — boundary detection is
    /// impossible, so the drive fails closed rather than silently collapsing every block to
    /// epoch 0 and never applying POOLREAP (which would yield a consensus-divergent candidate).
    InvalidEpochLength,
}

/// The window-end consensus inputs: the per-pool stake aggregate AND the active pool params
/// (`PoolId -> PoolParams`, incl. the registered VRF keyhash) as of the MARK instant — after the
/// last block's certs and every crossed-boundary POOLREAP, but BEFORE the window-end boundary's
/// reap (SNAP precedes POOLREAP, `Epoch.hs:292-297`). ECA-0b builds the leadership-complete
/// `EpochConsensusView` by intersecting `stake.pool_stakes` with `pool_params`
/// (delegated ∩ registered) and reading each kept pool's `vrf_hash`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowConsensusInputs {
    pub stake: StakeByPool,
    pub pool_params: BTreeMap<PoolId, PoolParams>,
}

/// Drive the reduced checkpoint + cert state forward over `blocks`, applying the epoch-boundary
/// POOLREAP (`apply_pool_reap`: adopt staged re-registrations, reap pools retiring at the entered
/// epoch, clear their delegations) at EACH epoch boundary CROSSED within the block range, then
/// surface the window-end stake aggregate + active pool params (the MARK — captured after the last
/// block, before any further reap).
///
/// `checkpoint` must already be `build_from` the bootstrap reduced UTxO; `bootstrap_state` must
/// carry the manifest-bound bootstrap cert state (DC-EVIEW-09) AS OF the start of the first block's
/// epoch (every earlier boundary already reaped). Boundaries are detected by `slot / slots_per_epoch`
/// and POOLREAP is applied for every entered epoch since the previous block (so empty epochs with no
/// blocks are still reaped, matching cardano). PRECONDITION: `blocks` MUST be slot-ordered (the
/// DC-EPOCH-08 validated source window guarantees this); out-of-order blocks would mis-detect
/// boundaries. A single-epoch block range crosses no boundary — so
/// the live single-epoch DC-EPOCH-08 window is per-block only (the boundary path is exercised by the
/// multi-epoch tests + the deferred model-B sustained-continuity wiring). Pure/deterministic; reads
/// only the durable inputs (no peer/network/wall-clock).
pub fn drive_window_consensus_inputs(
    checkpoint: &ReducedUtxoCheckpoint,
    bootstrap_state: &LedgerState,
    blocks: &[ShelleyBlock],
    era: CardanoEra,
    slots_per_epoch: u64,
) -> Result<WindowConsensusInputs, WindowDriverError> {
    // Fail closed on a zero epoch length: without a valid slots_per_epoch the boundary detection
    // would silently collapse all blocks to epoch 0 and never apply POOLREAP (a consensus-divergent
    // candidate with no error). The per-block wrapper passes u64::MAX, never 0.
    if slots_per_epoch == 0 {
        return Err(WindowDriverError::InvalidEpochLength);
    }
    let mut state = bootstrap_state.clone();
    let mut prev_epoch: Option<u64> = None;
    for block in blocks {
        let blk_epoch = block.header.body.slot / slots_per_epoch; // slots_per_epoch > 0 (guarded above)
        // Apply POOLREAP for EACH epoch boundary crossed since the previous block (cardano runs
        // POOLREAP at every epoch transition; empty epochs with no blocks are still reaped).
        if let Some(pe) = prev_epoch {
            let mut e = pe.saturating_add(1);
            while e <= blk_epoch {
                apply_pool_reap(&mut state.cert_state, EpochNo(e));
                e = e.saturating_add(1);
            }
        }
        prev_epoch = Some(blk_epoch);
        // UTxO side: the reduced block delta (== reduce(track_utxo)) into the checkpoint.
        let delta = reduced_block_delta(block, era).map_err(WindowDriverError::Ledger)?;
        checkpoint
            .apply_block_delta(&delta.spent, &delta.produced)
            .map_err(WindowDriverError::Checkpoint)?;
        // Cert side: advance the delegation/reward (+ gov) state, exactly as the ledger does.
        let (cert_state, gov_state) =
            advance_cert_state(block, era, &state).map_err(WindowDriverError::Ledger)?;
        state.cert_state = cert_state;
        state.gov_state = gov_state;
    }
    let cred_utxo_stake = checkpoint
        .sum_base_credential_stake()
        .map_err(WindowDriverError::Checkpoint)?;
    let stake = aggregate_pool_stake(&cred_utxo_stake, &state.cert_state.delegation)
        .map_err(WindowDriverError::Aggregate)?;
    Ok(WindowConsensusInputs { stake, pool_params: state.cert_state.pool.pools.clone() })
}

/// Drive the reduced checkpoint + cert state over `blocks` and aggregate per-pool stake — a thin
/// wrapper over [`drive_window_consensus_inputs`] for callers that only need the stake. The
/// sentinel `slots_per_epoch = u64::MAX` maps every slot to epoch 0, so no boundary is crossed
/// (per-block only) — the behavior before ECA-0a. ECA-0b switches the candidate derivation to
/// `drive_window_consensus_inputs` to also consume `pool_params`.
pub fn drive_window_aggregate(
    checkpoint: &ReducedUtxoCheckpoint,
    bootstrap_state: &LedgerState,
    blocks: &[ShelleyBlock],
    era: CardanoEra,
) -> Result<StakeByPool, WindowDriverError> {
    drive_window_consensus_inputs(checkpoint, bootstrap_state, blocks, era, u64::MAX)
        .map(|w| w.stake)
}

/// Why the live ChainDB-replay advance could not bring the checkpoint to the tip. Every
/// variant aborts the advance (fail-closed); the checkpoint stays at its last good slot, so
/// the lagging check (DC-EPOCH-11) blocks EpochConsensusView production.
#[derive(Debug)]
pub enum CheckpointAdvanceError {
    Checkpoint(ReducedCheckpointError),
    ChainDb(crate::chaindb::ChainDbError),
    Decode(String),
    Delta(ade_ledger::error::LedgerError),
}

/// EPOCH-CONSENSUS-VIEW S3f-4d-mat-2b (DC-EPOCH-11): advance the live reduced checkpoint over
/// the durable ChainDB -- the authoritative selected chain -- from its last-advanced slot (or
/// `bootstrap_slot` if only built) up to `to_slot`, in ChainDB (selected-chain, WAL) order.
/// Each durably-admitted block: decode -> `reduced_block_delta` (DC-EVIEW-04) ->
/// `advance_block` (slot recorded atomically). Reads ONLY the durable ChainDB (no peer/
/// network/wall-clock), so the advance is replay-equivalent + in admission order. Fail-closed:
/// any decode/delta/checkpoint error aborts with the checkpoint left at its last good slot.
pub fn advance_reduced_checkpoint_over_chaindb(
    checkpoint: &ReducedUtxoCheckpoint,
    chaindb: &dyn crate::chaindb::ChainDb,
    bootstrap_slot: ade_types::SlotNo,
    to_slot: ade_types::SlotNo,
    era: CardanoEra,
) -> Result<(), CheckpointAdvanceError> {
    let from = checkpoint
        .last_advanced_slot()
        .map_err(CheckpointAdvanceError::Checkpoint)?
        .map(|s| ade_types::SlotNo(s.0.saturating_add(1)))
        .unwrap_or(bootstrap_slot);
    let iter = chaindb
        .iter_from_slot(from)
        .map_err(CheckpointAdvanceError::ChainDb)?;
    for stored in iter {
        let stored = stored.map_err(CheckpointAdvanceError::ChainDb)?;
        if stored.slot.0 > to_slot.0 {
            break;
        }
        let block = decode_stored_to_shelley(&stored.bytes)?;
        let delta = reduced_block_delta(&block, era).map_err(CheckpointAdvanceError::Delta)?;
        checkpoint
            .advance_block(stored.slot, &delta.spent, &delta.produced)
            .map_err(CheckpointAdvanceError::Checkpoint)?;
    }
    Ok(())
}

/// Decode a stored block's bytes into a `ShelleyBlock` via the proven envelope + Conway path.
fn decode_stored_to_shelley(
    bytes: &[u8],
) -> Result<ade_types::shelley::block::ShelleyBlock, CheckpointAdvanceError> {
    let env = ade_codec::cbor::envelope::decode_block_envelope(bytes)
        .map_err(|e| CheckpointAdvanceError::Decode(format!("envelope: {e:?}")))?;
    let inner = &bytes[env.block_start..env.block_end];
    Ok(ade_codec::conway::decode_conway_block(inner)
        .map_err(|e| CheckpointAdvanceError::Decode(format!("conway: {e:?}")))?
        .decoded()
        .clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_ledger::reduced_utxo::ReducedStakeRef;
    use ade_types::shelley::cert::StakeCredential;
    use ade_types::tx::{Coin, PoolId, TxIn};
    use ade_types::{Hash28, Hash32};
    use std::collections::BTreeMap;

    const RAW_CONWAY_BLOCK: &[u8] =
        include_bytes!("../../../ade_node/tests/fixtures/raw_era_block_conway.cbor");

    /// S3f-4d-mat-2b (DC-EPOCH-11): the live advancer replays the durable ChainDB into the
    /// checkpoint -- reads the admitted block from the ChainDB, applies its reduced delta,
    /// and records its slot. Resumes idempotently from last_advanced_slot.
    #[test]
    fn advance_over_chaindb_replays_durable_blocks() {
        use crate::chaindb::types::StoredBlock;
        use crate::chaindb::{ChainDb, InMemoryChainDb};
        use ade_types::SlotNo;
        let dir = tempfile::tempdir().unwrap();
        let cp = ReducedUtxoCheckpoint::open(&dir.path().join("rc.redb")).unwrap();
        cp.build_from(&BTreeMap::new()).unwrap(); // empty bootstrap UTxO
        // store the real conway block durably at slot 1000.
        let db = InMemoryChainDb::new();
        db.put_block(&StoredBlock {
            hash: Hash32([0xab; 32]),
            slot: SlotNo(1000),
            bytes: RAW_CONWAY_BLOCK.to_vec(),
        })
        .unwrap();
        // advance from bootstrap_slot 0 up to 2000 -> reads + applies the durable block.
        advance_reduced_checkpoint_over_chaindb(&cp, &db, SlotNo(0), SlotNo(2000), CardanoEra::Conway)
            .expect("advance");
        assert_eq!(
            cp.last_advanced_slot().unwrap(),
            Some(SlotNo(1000)),
            "advanced to the durable block's slot, in ChainDB order"
        );
        // resume: a second advance with no new blocks is an idempotent no-op.
        advance_reduced_checkpoint_over_chaindb(&cp, &db, SlotNo(0), SlotNo(2000), CardanoEra::Conway)
            .expect("advance2");
        assert_eq!(cp.last_advanced_slot().unwrap(), Some(SlotNo(1000)));
    }

    fn temp_checkpoint() -> (ReducedUtxoCheckpoint, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let cp = ReducedUtxoCheckpoint::open(&dir.path().join("reduced.redb")).expect("open");
        (cp, dir)
    }

    fn cred(b: u8) -> StakeCredential {
        StakeCredential::KeyHash(Hash28([b; 28]))
    }
    fn pool(b: u8) -> PoolId {
        PoolId(Hash28([b; 28]))
    }
    fn txin(b: u8) -> TxIn {
        TxIn { tx_hash: Hash32([b; 32]), index: 0 }
    }

    /// Bootstrap-only window (zero blocks): the aggregate is the bootstrap reduced UTxO
    /// grouped by the bootstrap cert state's delegation map -- proving the
    /// checkpoint->sum->aggregate wiring + that the PRE-bootstrap delegators (present in
    /// the bootstrap cert state) are counted.
    #[test]
    fn empty_window_aggregates_bootstrap_state() {
        let (cp, _dir) = temp_checkpoint();
        // bootstrap reduced UTxO: two base creds with coin.
        let mut reduced: BTreeMap<TxIn, (Coin, ReducedStakeRef)> = BTreeMap::new();
        reduced.insert(txin(1), (Coin(1000), ReducedStakeRef::Base(cred(0xA))));
        reduced.insert(txin(2), (Coin(2000), ReducedStakeRef::Base(cred(0xB))));
        reduced.insert(txin(3), (Coin(9999), ReducedStakeRef::NonContributing));
        cp.build_from(&reduced).expect("build_from");

        // bootstrap cert state: both creds delegate to the same pool (a pre-bootstrap fact).
        let mut state = LedgerState::new(CardanoEra::Conway);
        state.cert_state.delegation.delegations.insert(cred(0xA), pool(0x1));
        state.cert_state.delegation.delegations.insert(cred(0xB), pool(0x1));

        let agg = drive_window_aggregate(&cp, &state, &[], CardanoEra::Conway).expect("drive");
        // pool 0x1 = 1000 + 2000; the NonContributing 9999 is excluded.
        assert_eq!(agg.pool_stakes.get(&pool(0x1)).copied(), Some(Coin(3000)));
        assert_eq!(agg.total_active_stake, Coin(3000));
    }

    /// Drive over a REAL conway block: the driver's result equals the SAME pieces composed
    /// by hand (apply the block's reduced delta to the checkpoint + advance the cert state
    /// + aggregate). Proves the loop sequences the proven pieces correctly on real data.
    #[test]
    fn real_conway_block_drive_equals_composed_pieces() {
        let env = decode_block_envelope(RAW_CONWAY_BLOCK).expect("envelope");
        let inner = &RAW_CONWAY_BLOCK[env.block_start..env.block_end];
        let block = ade_codec::conway::decode_conway_block(inner)
            .expect("decode conway block")
            .decoded()
            .clone();
        let era = CardanoEra::Conway;

        // a bootstrap reduced UTxO with a delegated base cred (so the aggregate is non-trivial).
        let mut reduced: BTreeMap<TxIn, (Coin, ReducedStakeRef)> = BTreeMap::new();
        reduced.insert(txin(7), (Coin(5_000_000), ReducedStakeRef::Base(cred(0xC))));
        let mut state = LedgerState::new(era);
        state.cert_state.delegation.delegations.insert(cred(0xC), pool(0x9));

        // (1) the driver.
        let (cp_drv, _d1) = temp_checkpoint();
        cp_drv.build_from(&reduced).expect("build");
        let driven =
            drive_window_aggregate(&cp_drv, &state, std::slice::from_ref(&block), era).expect("drive");

        // (2) the same pieces composed by hand.
        let (cp_ref, _d2) = temp_checkpoint();
        cp_ref.build_from(&reduced).expect("build");
        let delta = reduced_block_delta(&block, era).expect("delta");
        cp_ref.apply_block_delta(&delta.spent, &delta.produced).expect("apply");
        let (cert_state, _gov) = advance_cert_state(&block, era, &state).expect("advance");
        let cred_utxo = cp_ref.sum_base_credential_stake().expect("sum");
        let composed = aggregate_pool_stake(&cred_utxo, &cert_state.delegation).expect("agg");

        assert_eq!(driven, composed, "the window driver == the proven pieces composed in order");
    }

    fn pp(b: u8, vrf: u8) -> PoolParams {
        PoolParams {
            pool_id: pool(b),
            vrf_hash: Hash32([vrf; 32]),
            pledge: Coin(0),
            cost: Coin(0),
            margin: (0, 1),
            reward_account: vec![],
            owners: vec![],
        }
    }

    fn real_conway_block() -> ade_types::shelley::block::ShelleyBlock {
        let env = decode_block_envelope(RAW_CONWAY_BLOCK).expect("envelope");
        let inner = &RAW_CONWAY_BLOCK[env.block_start..env.block_end];
        ade_codec::conway::decode_conway_block(inner).expect("decode").decoded().clone()
    }

    /// An empty block (no txs) at `slot`: `tx_count = 0` + a CBOR empty definite array, so the only
    /// cert-state change a driver makes across it is the epoch-boundary POOLREAP.
    fn empty_block_at(
        template: &ade_types::shelley::block::ShelleyBlock,
        slot: u64,
    ) -> ade_types::shelley::block::ShelleyBlock {
        let mut b = template.clone();
        b.tx_count = 0;
        b.tx_bodies = vec![0x80];
        b.header.body.slot = slot;
        b
    }

    // ECA-0a: across a crossed epoch boundary the driver applies POOLREAP — adopts a staged
    // re-registration's new VRF, reaps a pool retiring at the entered epoch, and clears that pool's
    // delegations (dropping its delegators' stake). The mark (pool_params + stake) is captured after
    // the last block, before any further reap.
    #[test]
    fn drive_boundary_adopts_futures_reaps_retiring_clears_delegations() {
        let era = CardanoEra::Conway;
        let tmpl = real_conway_block();
        let b0 = empty_block_at(&tmpl, 50); // epoch 0 (slots_per_epoch = 100)
        let b1 = empty_block_at(&tmpl, 150); // epoch 1 -> crosses the boundary

        let (cp, _dir) = temp_checkpoint();
        let mut reduced: BTreeMap<TxIn, (Coin, ReducedStakeRef)> = BTreeMap::new();
        reduced.insert(txin(0xDD), (Coin(7_000_000), ReducedStakeRef::Base(cred(0xDD))));
        reduced.insert(txin(0xEE), (Coin(3_000_000), ReducedStakeRef::Base(cred(0xEE))));
        cp.build_from(&reduced).expect("build");

        let mut state = LedgerState::new(era);
        // 0xAA active (old VRF 0x11) + a staged re-registration (new VRF 0x22); 0xEE delegates to it.
        state.cert_state.pool.pools.insert(pool(0xAA), pp(0xAA, 0x11));
        state.cert_state.pool.future_pools.insert(pool(0xAA), pp(0xAA, 0x22));
        state.cert_state.delegation.registrations.insert(cred(0xEE), Coin(2_000_000));
        state.cert_state.delegation.rewards.insert(cred(0xEE), Coin(0));
        state.cert_state.delegation.delegations.insert(cred(0xEE), pool(0xAA));
        // 0xBB active, scheduled to retire entering epoch 1; 0xDD delegates to it.
        state.cert_state.pool.pools.insert(pool(0xBB), pp(0xBB, 0x33));
        state.cert_state.pool.retiring.insert(pool(0xBB), EpochNo(1));
        state.cert_state.delegation.registrations.insert(cred(0xDD), Coin(2_000_000));
        state.cert_state.delegation.rewards.insert(cred(0xDD), Coin(0));
        state.cert_state.delegation.delegations.insert(cred(0xDD), pool(0xBB));

        let out = drive_window_consensus_inputs(&cp, &state, &[b0, b1], era, 100).expect("drive");

        assert_eq!(out.pool_params[&pool(0xAA)].vrf_hash, Hash32([0x22; 32]), "future VRF adopted at boundary");
        assert!(!out.pool_params.contains_key(&pool(0xBB)), "retiring pool reaped at the boundary");
        assert_eq!(out.stake.pool_stakes.get(&pool(0xAA)).copied(), Some(Coin(3_000_000)), "adopted pool keeps its delegated stake");
        assert!(!out.stake.pool_stakes.contains_key(&pool(0xBB)), "reaped pool's delegation cleared -> no stake");
    }

    // ECA-0a replay equivalence: the boundary-crossing drive is deterministic — two fresh runs over
    // the same inputs produce byte-identical consensus inputs (the property a reorg re-materialize
    // relies on: re-running the drive from bootstrap yields the same mark).
    #[test]
    fn drive_boundary_is_deterministic() {
        let era = CardanoEra::Conway;
        let tmpl = real_conway_block();
        let blocks = [empty_block_at(&tmpl, 50), empty_block_at(&tmpl, 150)];
        let run = || {
            let (cp, _d) = temp_checkpoint();
            let mut reduced: BTreeMap<TxIn, (Coin, ReducedStakeRef)> = BTreeMap::new();
            reduced.insert(txin(0xDD), (Coin(7_000_000), ReducedStakeRef::Base(cred(0xDD))));
            cp.build_from(&reduced).unwrap();
            let mut state = LedgerState::new(era);
            state.cert_state.pool.pools.insert(pool(0xAA), pp(0xAA, 0x11));
            state.cert_state.pool.future_pools.insert(pool(0xAA), pp(0xAA, 0x22));
            state.cert_state.pool.pools.insert(pool(0xBB), pp(0xBB, 0x33));
            state.cert_state.pool.retiring.insert(pool(0xBB), EpochNo(1));
            state.cert_state.delegation.registrations.insert(cred(0xDD), Coin(2_000_000));
            state.cert_state.delegation.rewards.insert(cred(0xDD), Coin(0));
            state.cert_state.delegation.delegations.insert(cred(0xDD), pool(0xBB));
            drive_window_consensus_inputs(&cp, &state, &blocks, era, 100).unwrap()
        };
        assert_eq!(run(), run(), "boundary-crossing drive is replay-deterministic");
    }

    // ECA-0a (security M1): a zero epoch length fails closed rather than silently collapsing all
    // blocks to epoch 0 (which would skip every POOLREAP -> a consensus-divergent candidate).
    #[test]
    fn drive_rejects_zero_slots_per_epoch() {
        let (cp, _d) = temp_checkpoint();
        cp.build_from(&BTreeMap::new()).unwrap();
        let state = LedgerState::new(CardanoEra::Conway);
        let b = empty_block_at(&real_conway_block(), 50);
        match drive_window_consensus_inputs(&cp, &state, &[b], CardanoEra::Conway, 0) {
            Err(WindowDriverError::InvalidEpochLength) => {}
            other => panic!("expected InvalidEpochLength, got {other:?}"),
        }
    }
}
