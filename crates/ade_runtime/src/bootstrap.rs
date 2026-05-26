// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN single bootstrap authority (PHASE4-N-K S1).
//!
//! `bootstrap_initial_state` is the SOLE `pub fn` in the workspace
//! returning the initial `(LedgerState, PraosChainDepState,
//! Option<ChainTip>)` triple at node startup. Cold-start
//! (genesis-only, no chaindb tip, no snapshots) and warm-start
//! (snapshot resume + replay-forward to chaindb tip) are two
//! branches of one function — never parallel paths.
//!
//! CN-NODE-01: enforced by `ci/ci_check_bootstrap_closure.sh`.
//!
//! This module composes the BLUE authorities; it never bypasses
//! them. Warm-start delegates to
//! `ade_ledger::rollback::materialize_rolled_back_state`
//! (CN-STORE-07) for the replay-forward step;
//! `ade_runtime::rollback::PersistentSnapshotCache` for the
//! snapshot-byte read (CN-STORE-08, CN-STORE-07 input source).

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_ledger::rollback::{
    materialize_rolled_back_state, MaterializeError, TargetPoint,
};
use ade_ledger::state::LedgerState;
use ade_types::SlotNo;

use crate::chaindb::{ChainDb, ChainDbError, ChainTip, SnapshotStore};
use crate::rollback::chaindb_block_source::ChainDbBlockSource;
use crate::rollback::persistent_cache::PersistentSnapshotCache;

/// Inputs to [`bootstrap_initial_state`]. Borrows everything; the
/// caller (the `ade_node` binary) owns the storage backends.
pub struct BootstrapInputs<'a, D, S>
where
    D: ChainDb,
    S: SnapshotStore + ?Sized,
{
    pub chaindb: &'a D,
    pub snapshot_store: &'a S,
    pub era_schedule: &'a EraSchedule,
    pub ledger_view: &'a dyn LedgerView,
    /// Cold-start seed: the genesis-derived `(ledger, chain_dep)`
    /// pair. Required iff both `chaindb.tip()` returns `None` and
    /// `snapshot_store.list_snapshot_slots()` is empty.
    pub genesis_initial: Option<(LedgerState, PraosChainDepState)>,
}

/// Closed bootstrap-error sum. Authority-fatal at the binary
/// boundary (DC-NODE-04).
#[derive(Debug)]
pub enum BootstrapError {
    /// `chaindb.tip()` or `snapshot_store.list_snapshot_slots()` /
    /// `get_snapshot` returned an underlying storage error.
    ChainDb(ChainDbError),
    /// Snapshot decoded with an authority-fatal error
    /// (UnknownVersion, FingerprintMismatch, ...). Halt the binary.
    SnapshotMissing {
        chain_tip_slot: SlotNo,
    },
    /// `materialize_rolled_back_state` failed (replay-forward
    /// rejected a block, era not supported, etc.).
    Materialize(MaterializeError),
    /// Cold-start branch entered but no genesis seed supplied.
    GenesisRequiredButAbsent,
}

/// The SOLE bootstrap authority. Cold-start vs warm-start is a
/// branch on `(chaindb.tip(), snapshot_store has any slot)`:
///
/// | chaindb.tip() | snapshots non-empty | branch         |
/// |---------------|---------------------|----------------|
/// | None          | empty               | cold-start     |
/// | Some(tip)     | any                 | warm-start     |
/// | None          | non-empty           | warm-start at largest stored slot (recover from chaindb truncation) |
///
/// Cold-start returns `(genesis_ledger, genesis_chain_dep, None)`.
/// Warm-start delegates to `materialize_rolled_back_state` with
/// `target = chaindb.tip()` (or the largest snapshot slot when the
/// chaindb is empty).
pub fn bootstrap_initial_state<D, S>(
    inputs: BootstrapInputs<'_, D, S>,
) -> Result<(LedgerState, PraosChainDepState, Option<ChainTip>), BootstrapError>
where
    D: ChainDb,
    S: SnapshotStore + ?Sized,
{
    let tip = inputs.chaindb.tip().map_err(BootstrapError::ChainDb)?;
    let snapshot_slots = inputs
        .snapshot_store
        .list_snapshot_slots()
        .map_err(BootstrapError::ChainDb)?;

    // Cold-start: nothing on disk; require a genesis seed.
    if tip.is_none() && snapshot_slots.is_empty() {
        let (ledger, chain_dep) = inputs
            .genesis_initial
            .ok_or(BootstrapError::GenesisRequiredButAbsent)?;
        return Ok((ledger, chain_dep, None));
    }

    // Warm-start: pick a materialization target.
    let target = match &tip {
        Some(t) => TargetPoint {
            slot: t.slot,
            hash: t.hash.clone(),
        },
        None => {
            // chaindb empty but snapshots exist — materialize at the
            // largest snapshot slot. We do not synthesize a tip
            // hash; emit a null hash and let the caller (orchestrator)
            // re-discover the canonical tip from the materialized
            // ledger.
            let largest = *snapshot_slots
                .last()
                .ok_or(BootstrapError::SnapshotMissing {
                    chain_tip_slot: SlotNo(0),
                })?;
            TargetPoint {
                slot: largest,
                hash: ade_types::Hash32([0u8; 32]),
            }
        }
    };

    let reader = PersistentSnapshotCache::new(inputs.snapshot_store);
    let source = ChainDbBlockSource::new(inputs.chaindb);

    let (ledger, chain_dep) = materialize_rolled_back_state(
        target,
        &reader,
        &source,
        inputs.era_schedule,
        inputs.ledger_view,
    )
    .map_err(BootstrapError::Materialize)?;

    Ok((ledger, chain_dep, tip))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_core::consensus::praos_state::Nonce;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
    use ade_ledger::block_validity::decode_block;
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::fingerprint::fingerprint;
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

    use crate::chaindb::{InMemoryChainDb, StoredBlock};
    use crate::rollback::persistent_cache::PersistentSnapshotCache;

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

    fn fresh_genesis(eta0: [u8; 32]) -> (LedgerState, PraosChainDepState) {
        let mut ledger = LedgerState::new(CardanoEra::Conway);
        ledger.epoch_state.epoch = EPOCH_576;
        let mut chain_dep = PraosChainDepState::empty();
        chain_dep.epoch_nonce = Nonce(Hash32(eta0));
        chain_dep.evolving_nonce = Nonce(Hash32(eta0));
        (ledger, chain_dep)
    }

    #[test]
    fn bootstrap_cold_start_returns_genesis_when_empty() {
        let db = InMemoryChainDb::new();
        let (_corpus, view) = corpus_view();
        let sched = schedule();
        let genesis = fresh_genesis([0xAB; 32]);
        let result = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: Some(genesis.clone()),
        })
        .expect("bootstrap");
        assert_eq!(result.0.epoch_state.epoch, EPOCH_576);
        assert_eq!(result.1.epoch_nonce, genesis.1.epoch_nonce);
        assert!(result.2.is_none(), "cold-start has no tip");
    }

    #[test]
    fn bootstrap_cold_start_without_genesis_errors() {
        let db = InMemoryChainDb::new();
        let (_corpus, view) = corpus_view();
        let sched = schedule();
        let err = bootstrap_initial_state(BootstrapInputs::<_, InMemoryChainDb> {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
        })
        .expect_err("must fail");
        assert!(matches!(err, BootstrapError::GenesisRequiredButAbsent));
    }

    #[test]
    fn bootstrap_warm_start_materializes_from_persistent_snapshot() {
        let (corpus, view) = corpus_view();
        let sched = schedule();
        // Seed: take the lightest corpus block, build a snapshot at
        // its slot, then bootstrap with that snapshot.
        let idx = (0..corpus.blocks.len())
            .min_by_key(|&i| {
                let env = decode_block_envelope(&corpus.blocks[i]).expect("env");
                env.block_end - env.block_start
            })
            .expect("non-empty");
        let bytes = corpus.blocks[idx].clone();
        let decoded = decode_block(&bytes).expect("decode");
        let snapshot_slot = decoded.header_input.slot;

        let (mut ledger, mut chain_dep) = fresh_genesis(corpus.epoch_nonce);
        // The orchestrator would call apply_block_with_verdicts; here
        // we shortcut by directly using block_validity to get the
        // post-block state, then store the snapshot at that slot. The
        // snapshot is therefore *at the tip*; warm-start should
        // return identical state.
        use ade_ledger::block_validity::transition::{block_validity, BlockValidityOutcome};
        use ade_ledger::block_validity::verdict::BlockValidityVerdict;
        let BlockValidityOutcome {
            verdict,
            ledger: new_ledger,
            chain_dep: new_chain_dep,
        } = block_validity(&ledger, &chain_dep, &sched, &view, &bytes);
        match verdict {
            BlockValidityVerdict::Valid { .. } => {
                ledger = new_ledger;
                chain_dep = new_chain_dep;
            }
            BlockValidityVerdict::Invalid { error, .. } => {
                panic!("seed block must be valid, got {error:?}")
            }
        }

        let db = InMemoryChainDb::new();
        // Put the block in chaindb at its slot so tip == snapshot_slot.
        db.put_block(&StoredBlock {
            slot: snapshot_slot,
            hash: decoded.block_hash.clone(),
            bytes: bytes.clone(),
        })
        .expect("put");
        let cache = PersistentSnapshotCache::new(&db);
        cache
            .capture(snapshot_slot, &ledger, &chain_dep)
            .expect("capture");

        let pre_fp = fingerprint(&ledger).combined;

        let (out_ledger, out_chain_dep, out_tip) =
            bootstrap_initial_state(BootstrapInputs {
                chaindb: &db,
                snapshot_store: &db,
                era_schedule: &sched,
                ledger_view: &view,
                genesis_initial: None,
            })
            .expect("warm bootstrap");

        let post_fp = fingerprint(&out_ledger).combined;
        assert_eq!(pre_fp, post_fp, "warm-start fingerprint must match");
        assert_eq!(out_chain_dep, chain_dep);
        assert_eq!(out_tip.as_ref().map(|t| t.slot), Some(snapshot_slot));
    }

    #[test]
    fn bootstrap_two_runs_produce_byte_identical_state() {
        let (corpus, view) = corpus_view();
        let sched = schedule();
        let db = InMemoryChainDb::new();
        let (genesis_ledger, genesis_chain_dep) = fresh_genesis(corpus.epoch_nonce);

        let run = || {
            bootstrap_initial_state(BootstrapInputs {
                chaindb: &db,
                snapshot_store: &db,
                era_schedule: &sched,
                ledger_view: &view,
                genesis_initial: Some((genesis_ledger.clone(), genesis_chain_dep.clone())),
            })
            .expect("bootstrap")
        };
        let (l1, c1, t1) = run();
        let (l2, c2, t2) = run();
        assert_eq!(fingerprint(&l1).combined, fingerprint(&l2).combined);
        assert_eq!(c1, c2);
        assert_eq!(t1, t2);
    }

    #[test]
    fn bootstrap_warm_start_equals_direct_materialize() {
        // The warm-start branch must produce the same ledger as a
        // direct call to materialize_rolled_back_state with the same
        // target / reader / source. (Single-authority equivalence.)
        let (corpus, view) = corpus_view();
        let sched = schedule();

        let idx = (0..corpus.blocks.len())
            .min_by_key(|&i| {
                let env = decode_block_envelope(&corpus.blocks[i]).expect("env");
                env.block_end - env.block_start
            })
            .expect("non-empty");
        let bytes = corpus.blocks[idx].clone();
        let decoded = decode_block(&bytes).expect("decode");
        let snapshot_slot = decoded.header_input.slot;

        let (mut ledger, mut chain_dep) = fresh_genesis(corpus.epoch_nonce);
        use ade_ledger::block_validity::transition::{block_validity, BlockValidityOutcome};
        use ade_ledger::block_validity::verdict::BlockValidityVerdict;
        let BlockValidityOutcome {
            verdict,
            ledger: new_ledger,
            chain_dep: new_chain_dep,
        } = block_validity(&ledger, &chain_dep, &sched, &view, &bytes);
        match verdict {
            BlockValidityVerdict::Valid { .. } => {
                ledger = new_ledger;
                chain_dep = new_chain_dep;
            }
            BlockValidityVerdict::Invalid { error, .. } => {
                panic!("seed block must be valid, got {error:?}")
            }
        }

        let db = InMemoryChainDb::new();
        db.put_block(&StoredBlock {
            slot: snapshot_slot,
            hash: decoded.block_hash.clone(),
            bytes: bytes.clone(),
        })
        .expect("put");
        let cache = PersistentSnapshotCache::new(&db);
        cache
            .capture(snapshot_slot, &ledger, &chain_dep)
            .expect("capture");

        let target = TargetPoint {
            slot: snapshot_slot,
            hash: decoded.block_hash.clone(),
        };
        let reader = PersistentSnapshotCache::new(&db);
        let source = ChainDbBlockSource::new(&db);
        let (direct_l, direct_cd) = materialize_rolled_back_state(
            target,
            &reader,
            &source,
            &sched,
            &view,
        )
        .expect("direct");

        let (boot_l, boot_cd, _tip) = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
        })
        .expect("bootstrap");

        assert_eq!(
            fingerprint(&direct_l).combined,
            fingerprint(&boot_l).combined,
            "single-authority equivalence"
        );
        assert_eq!(direct_cd, boot_cd);
    }
}
