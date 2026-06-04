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
use ade_crypto::blake2b_256;
use ade_ledger::rollback::{
    materialize_rolled_back_state, MaterializeError, TargetPoint,
};
use ade_ledger::seed_consensus_inputs::{
    decode_seed_epoch_consensus_inputs, encode_seed_epoch_consensus_inputs,
    SeedConsensusInputsError, SeedEpochConsensusInputs,
};
use ade_ledger::state::LedgerState;
use ade_ledger::wal::RecoveredBootstrapProvenance;
use ade_types::{EpochNo, Hash32, SlotNo};

use crate::chaindb::{ChainDb, ChainDbError, ChainTip, SnapshotStore};
use crate::rollback::chaindb_block_source::ChainDbBlockSource;
use crate::rollback::persistent_cache::PersistentSnapshotCache;

/// Whether the warm-start branch must restore + verify the
/// seed-epoch consensus-input sidecar (PHASE4-N-F-A A3b).
///
/// A closed two-variant enum (not `Option<view>` + a `require`
/// bool) so "required-but-absent" and "present-but-not-required"
/// are unrepresentable.
///
/// **Scope (A3b is a capability, not production wiring):** every
/// current caller passes `NotRequired`. A future production-wiring
/// slice replays the WAL to obtain the
/// [`RecoveredBootstrapProvenance`] view and passes
/// `RequiredFromRecoveredProvenance`.
pub enum SeedEpochConsensusSource {
    /// No seed-epoch provenance demanded. Cold-start, and every
    /// current (not-yet-wired) warm-start caller. Pre-A3b behavior;
    /// the recovered record in [`BootstrapState`] is `None`.
    NotRequired,
    /// Warm-start MUST restore + verify the sidecar against this
    /// already-WAL-replayed (A3a) provenance view, fail-closed.
    RequiredFromRecoveredProvenance(RecoveredBootstrapProvenance),
}

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
    /// PHASE4-N-F-A A3b: whether warm-start restores + verifies the
    /// seed-epoch consensus-input sidecar. `NotRequired` for every
    /// current caller (cold-start / not-yet-wired warm-start).
    pub seed_epoch_consensus_source: SeedEpochConsensusSource,
}

/// Output of [`bootstrap_initial_state`] (PHASE4-N-F-A A3b). A
/// named struct rather than a widened tuple so the recovered
/// consensus-input record is auditable at every call site.
#[derive(Debug)]
pub struct BootstrapState {
    pub ledger: LedgerState,
    pub chain_dep: PraosChainDepState,
    pub tip: Option<ChainTip>,
    /// The recovered seed-epoch consensus inputs — `Some` only on a
    /// `RequiredFromRecoveredProvenance` warm-start that verified;
    /// `None` on cold-start and `NotRequired` warm-start.
    pub seed_epoch_consensus_inputs: Option<SeedEpochConsensusInputs>,
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
    /// PHASE4-N-F-A A3b: a `RequiredFromRecoveredProvenance`
    /// warm-start could not obtain a usable provenance view to
    /// verify against (the warm-start branch was reached in
    /// required mode but the view is unavailable for this anchor).
    /// Exists for the future production wiring; fail-closed.
    SeedConsensusProvenanceMissing,
    /// A3b: the WAL provenance named an `anchor_fp` for which the
    /// anchor-keyed sidecar was absent in the `SnapshotStore`.
    SeedConsensusSidecarMissing { anchor_fp: Hash32 },
    /// A3b: `blake2b_256(sidecar_bytes)` did not equal the
    /// provenance `sidecar_hash`. The persisted sidecar does not
    /// match the WAL-recorded fact. Fail-closed.
    SeedConsensusHashMismatch { expected: Hash32, actual: Hash32 },
    /// A3b: the decoded sidecar's `anchor_fp` / `epoch_no` did not
    /// match the provenance binding, or the sidecar bytes failed to
    /// decode / re-encode byte-identically. Fail-closed.
    SeedConsensusBindingMismatch {
        expected_anchor_fp: Hash32,
        actual_anchor_fp: Hash32,
        expected_epoch: EpochNo,
        actual_epoch: EpochNo,
    },
    /// A3b: the sidecar bytes failed `decode_seed_epoch_consensus_inputs`
    /// (malformed / unknown version / non-canonical). Fail-closed.
    SeedConsensusSidecarDecode(SeedConsensusInputsError),
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
) -> Result<BootstrapState, BootstrapError>
where
    D: ChainDb,
    S: SnapshotStore + ?Sized,
{
    let tip = inputs.chaindb.tip().map_err(BootstrapError::ChainDb)?;
    let snapshot_slots = inputs
        .snapshot_store
        .list_snapshot_slots()
        .map_err(BootstrapError::ChainDb)?;

    // Cold-start: nothing on disk; require a genesis seed. The
    // seed-epoch consensus source is irrelevant here — a cold-start
    // has not imported a sidecar yet, so the recovered record is
    // `None` regardless of `NotRequired` vs required (A3b).
    if tip.is_none() && snapshot_slots.is_empty() {
        let (ledger, chain_dep) = inputs
            .genesis_initial
            .ok_or(BootstrapError::GenesisRequiredButAbsent)?;
        return Ok(BootstrapState {
            ledger,
            chain_dep,
            tip: None,
            seed_epoch_consensus_inputs: None,
        });
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

    let (ledger, mut chain_dep) = materialize_rolled_back_state(
        target,
        &reader,
        &source,
        inputs.era_schedule,
        inputs.ledger_view,
    )
    .map_err(BootstrapError::Materialize)?;

    // A3b: restore + verify the seed-epoch consensus-input sidecar
    // when (and only when) the caller demands it. `NotRequired`
    // preserves the pre-A3b warm-start behavior exactly.
    let seed_epoch_consensus_inputs = match inputs.seed_epoch_consensus_source {
        SeedEpochConsensusSource::NotRequired => None,
        SeedEpochConsensusSource::RequiredFromRecoveredProvenance(provenance) => {
            Some(restore_seed_epoch_consensus_inputs(inputs.snapshot_store, &provenance)?)
        }
    };

    // PHASE4-N-F-G-N (T-REC-04 / DC-CINPUT-03): overlay the recovered seed-epoch
    // eta0 onto the recovered chain_dep. The snapshot materializes the
    // ledger/chain skeleton, but its chain_dep carries the admission seed's
    // PLACEHOLDER eta0 (`Nonce::ZERO`, admission/bootstrap.rs:164); the
    // authoritative seed-epoch eta0 lives in the recovered consensus-input
    // sidecar. Without this, forge/self_accept sign the header VRF over ZERO and
    // a real Conway peer rejects it (VRFKeyBadProof). At the seed epoch (no
    // blocks applied since the seed) the evolving nonce equals eta0, so both are
    // set — reconstructing `genesis(eta0)`. This is the explicit recovered-input
    // overlay, NOT a snapshot replacement; eta0 is sourced from the sidecar, not
    // genesis-derived and not a placeholder.
    if let Some(sidecar) = &seed_epoch_consensus_inputs {
        chain_dep.epoch_nonce = sidecar.epoch_nonce.clone();
        chain_dep.evolving_nonce = sidecar.epoch_nonce.clone();
    }

    Ok(BootstrapState {
        ledger,
        chain_dep,
        tip,
        seed_epoch_consensus_inputs,
    })
}

/// A3b warm-start verification chain: restore the anchor-keyed
/// sidecar and verify it against the WAL-replayed provenance view,
/// fail-closed. No `--consensus-inputs-path` fallback. Pure except
/// for the single `SnapshotStore` read (the verification itself is
/// BLUE: hash + binding + byte-identity over already-read bytes).
fn restore_seed_epoch_consensus_inputs<S>(
    snapshot_store: &S,
    provenance: &RecoveredBootstrapProvenance,
) -> Result<SeedEpochConsensusInputs, BootstrapError>
where
    S: SnapshotStore + ?Sized,
{
    // 1. Sidecar bytes for the provenance anchor (absent => fail).
    let bytes = snapshot_store
        .get_seed_epoch_consensus_inputs(&provenance.anchor_fp)
        .map_err(BootstrapError::ChainDb)?
        .ok_or(BootstrapError::SeedConsensusSidecarMissing {
            anchor_fp: provenance.anchor_fp.clone(),
        })?;

    // 2. Hash binds the WAL fact to these exact bytes.
    let actual_hash = blake2b_256(&bytes);
    if actual_hash != provenance.sidecar_hash {
        return Err(BootstrapError::SeedConsensusHashMismatch {
            expected: provenance.sidecar_hash.clone(),
            actual: actual_hash,
        });
    }

    // 3. Decode via the A1 sole codec (byte-canonical; a malformed
    //    or non-canonical buffer fails here).
    let sidecar = decode_seed_epoch_consensus_inputs(&bytes)
        .map_err(BootstrapError::SeedConsensusSidecarDecode)?;

    // 4. Anchor + epoch binding: the sidecar must describe this
    //    anchor and the provenance's seed epoch.
    if sidecar.anchor_fp != provenance.anchor_fp || sidecar.epoch_no != provenance.epoch_no {
        return Err(BootstrapError::SeedConsensusBindingMismatch {
            expected_anchor_fp: provenance.anchor_fp.clone(),
            actual_anchor_fp: sidecar.anchor_fp.clone(),
            expected_epoch: provenance.epoch_no,
            actual_epoch: sidecar.epoch_no,
        });
    }

    // 5. Byte-identity: re-encoding the decoded record reproduces
    //    the persisted bytes exactly (CE-A-3, explicit at the
    //    authority surface).
    if encode_seed_epoch_consensus_inputs(&sidecar) != bytes {
        return Err(BootstrapError::SeedConsensusHashMismatch {
            expected: provenance.sidecar_hash.clone(),
            actual: actual_hash,
        });
    }

    Ok(sidecar)
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
            seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired,
        })
        .expect("bootstrap");
        assert_eq!(result.ledger.epoch_state.epoch, EPOCH_576);
        assert_eq!(result.chain_dep.epoch_nonce, genesis.1.epoch_nonce);
        assert!(result.tip.is_none(), "cold-start has no tip");
        assert!(
            result.seed_epoch_consensus_inputs.is_none(),
            "NotRequired cold-start recovers no sidecar"
        );
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
            seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired,
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

        let BootstrapState {
            ledger: out_ledger,
            chain_dep: out_chain_dep,
            tip: out_tip,
            ..
        } = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
            seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired,
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
                seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired,
            })
            .expect("bootstrap")
        };
        let r1 = run();
        let r2 = run();
        assert_eq!(
            fingerprint(&r1.ledger).combined,
            fingerprint(&r2.ledger).combined
        );
        assert_eq!(r1.chain_dep, r2.chain_dep);
        assert_eq!(r1.tip, r2.tip);
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

        let BootstrapState {
            ledger: boot_l,
            chain_dep: boot_cd,
            ..
        } = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
            seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired,
        })
        .expect("bootstrap");

        assert_eq!(
            fingerprint(&direct_l).combined,
            fingerprint(&boot_l).combined,
            "single-authority equivalence"
        );
        assert_eq!(direct_cd, boot_cd);
    }

    // ===== PHASE4-N-F-A A3b: warm-start seed-epoch sidecar restore =====

    /// Build a warm-startable `InMemoryChainDb` (one stored block +
    /// a snapshot at its slot, so `bootstrap_initial_state` takes the
    /// warm-start branch) plus the `(schedule, view)` it needs.
    /// Mirrors `bootstrap_warm_start_materializes_from_persistent_snapshot`.
    fn warm_started_db() -> (InMemoryChainDb, EraSchedule, PoolDistrView) {
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
        PersistentSnapshotCache::new(&db)
            .capture(snapshot_slot, &ledger, &chain_dep)
            .expect("capture");
        (db, sched, view)
    }

    const A3B_ANCHOR: Hash32 = Hash32([0x5A; 32]);

    /// A canonical sample sidecar record bound to `anchor_fp` / `epoch`.
    fn sample_sidecar(anchor_fp: Hash32, epoch: EpochNo) -> SeedEpochConsensusInputs {
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        pools.insert(
            Hash28([0x01; 28]),
            PoolEntry {
                active_stake: 1_000,
                vrf_keyhash: Hash32([0x07; 32]),
            },
        );
        SeedEpochConsensusInputs {
            anchor_fp,
            epoch_no: epoch,
            epoch_nonce: Nonce(Hash32([0x99; 32])),
            active_slots_coeff: ActiveSlotsCoeff {
                numer: 5,
                denom: 100,
            },
            total_active_stake: 1_000,
            pool_distribution: pools,
        }
    }

    /// Persist `record` into `db`'s anchor-keyed sidecar surface and
    /// return `(bytes, provenance)` where `provenance.sidecar_hash`
    /// binds the persisted bytes (the A3a view a warm-start consumes).
    fn seed_sidecar(
        db: &InMemoryChainDb,
        record: &SeedEpochConsensusInputs,
    ) -> (Vec<u8>, RecoveredBootstrapProvenance) {
        let bytes = encode_seed_epoch_consensus_inputs(record);
        db.put_seed_epoch_consensus_inputs(&record.anchor_fp, &bytes)
            .expect("put sidecar");
        let provenance = RecoveredBootstrapProvenance {
            anchor_fp: record.anchor_fp.clone(),
            sidecar_hash: blake2b_256(&bytes),
            epoch_no: record.epoch_no,
        };
        (bytes, provenance)
    }

    #[test]
    fn warm_start_restores_seed_epoch_consensus_inputs_byte_identical() {
        // CE-A-3 (authority surface): a warm-start in required mode,
        // given the persisted sidecar + its A3a provenance, recovers
        // the byte-identical record through `bootstrap_initial_state`.
        let (db, sched, view) = warm_started_db();
        let record = sample_sidecar(A3B_ANCHOR, EPOCH_576);
        let (_bytes, provenance) = seed_sidecar(&db, &record);

        let out = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
            seed_epoch_consensus_source:
                SeedEpochConsensusSource::RequiredFromRecoveredProvenance(provenance),
        })
        .expect("required warm-start");

        let recovered = out
            .seed_epoch_consensus_inputs
            .expect("required warm-start recovers the sidecar");
        assert_eq!(recovered, record);
        // Byte-identity: re-encoding the recovered record reproduces
        // exactly the persisted sidecar bytes.
        assert_eq!(encode_seed_epoch_consensus_inputs(&recovered), _bytes);
    }

    #[test]
    fn warm_start_overlays_recovered_eta0_onto_chain_dep_g_n() {
        // PHASE4-N-F-G-N (T-REC-04 / DC-CINPUT-03): the WarmStart-recovered forge
        // chain_dep.epoch_nonce MUST come from the seed-epoch sidecar, NOT the
        // snapshot. The snapshot's chain_dep here carries the corpus eta0 (a
        // stand-in for the C1 admission seed's Nonce::ZERO placeholder); the
        // persisted sidecar carries a DISTINCT eta0 ([0x99;32]). The recovered
        // chain_dep must equal the SIDECAR's eta0 — the exact bug fix (pre-G-N
        // the forge signed the header VRF over the snapshot's wrong nonce ->
        // VRFKeyBadProof from a real Conway peer).
        let (corpus, _v) = corpus_view();
        let snapshot_eta0 = Nonce(Hash32(corpus.epoch_nonce));
        let (db, sched, view) = warm_started_db();
        let record = sample_sidecar(A3B_ANCHOR, EPOCH_576); // epoch_nonce = [0x99;32]
        assert_ne!(
            record.epoch_nonce, snapshot_eta0,
            "precondition: the sidecar eta0 must differ from the snapshot's eta0"
        );
        let (_bytes, provenance) = seed_sidecar(&db, &record);

        let out = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
            seed_epoch_consensus_source:
                SeedEpochConsensusSource::RequiredFromRecoveredProvenance(provenance),
        })
        .expect("required warm-start");

        assert_eq!(
            out.chain_dep.epoch_nonce, record.epoch_nonce,
            "recovered forge eta0 must be the sidecar's, not the snapshot's"
        );
        assert_eq!(
            out.chain_dep.evolving_nonce, record.epoch_nonce,
            "seed-epoch evolving nonce equals eta0"
        );
        assert_ne!(
            out.chain_dep.epoch_nonce, snapshot_eta0,
            "the snapshot's placeholder eta0 must NOT reach the forge"
        );
    }

    #[test]
    fn warm_start_not_required_is_unchanged() {
        // NotRequired warm-start: pre-A3b behavior; no sidecar even
        // when one is persisted.
        let (db, sched, view) = warm_started_db();
        let record = sample_sidecar(A3B_ANCHOR, EPOCH_576);
        seed_sidecar(&db, &record);

        let out = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
            seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired,
        })
        .expect("not-required warm-start");
        assert!(out.seed_epoch_consensus_inputs.is_none());
    }

    #[test]
    fn cold_start_ignores_seed_epoch_source() {
        // Cold-start with a RequiredFromRecoveredProvenance source must
        // NOT error and must recover no sidecar — the verification chain
        // is warm-start-only.
        let db = InMemoryChainDb::new();
        let (_corpus, view) = corpus_view();
        let sched = schedule();
        let genesis = fresh_genesis([0xAB; 32]);
        let provenance = RecoveredBootstrapProvenance {
            anchor_fp: A3B_ANCHOR,
            sidecar_hash: Hash32([0xFF; 32]),
            epoch_no: EPOCH_576,
        };
        let out = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: Some(genesis),
            seed_epoch_consensus_source:
                SeedEpochConsensusSource::RequiredFromRecoveredProvenance(provenance),
        })
        .expect("cold-start ignores the source");
        assert!(out.tip.is_none());
        assert!(out.seed_epoch_consensus_inputs.is_none());
    }

    #[test]
    fn warm_start_fails_closed_on_missing_sidecar() {
        // Required warm-start, provenance present, but no sidecar
        // persisted for that anchor → fail closed.
        let (db, sched, view) = warm_started_db();
        let provenance = RecoveredBootstrapProvenance {
            anchor_fp: A3B_ANCHOR,
            sidecar_hash: Hash32([0x11; 32]),
            epoch_no: EPOCH_576,
        };
        let err = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
            seed_epoch_consensus_source:
                SeedEpochConsensusSource::RequiredFromRecoveredProvenance(provenance),
        })
        .expect_err("missing sidecar must fail closed");
        assert!(
            matches!(&err, BootstrapError::SeedConsensusSidecarMissing { anchor_fp } if *anchor_fp == A3B_ANCHOR),
            "got {err:?}"
        );
    }

    #[test]
    fn warm_start_fails_closed_on_hash_mismatch() {
        // Sidecar present, but provenance.sidecar_hash does not bind
        // the persisted bytes → fail closed.
        let (db, sched, view) = warm_started_db();
        let record = sample_sidecar(A3B_ANCHOR, EPOCH_576);
        let (_bytes, mut provenance) = seed_sidecar(&db, &record);
        provenance.sidecar_hash = Hash32([0xAA; 32]); // wrong hash

        let err = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
            seed_epoch_consensus_source:
                SeedEpochConsensusSource::RequiredFromRecoveredProvenance(provenance),
        })
        .expect_err("hash mismatch must fail closed");
        assert!(
            matches!(err, BootstrapError::SeedConsensusHashMismatch { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn warm_start_fails_closed_on_anchor_mismatch() {
        // The persisted sidecar describes anchor X; provenance claims
        // anchor Y (and hashes Y's bytes). Reading by Y's anchor_fp
        // finds nothing → SidecarMissing; reading a sidecar whose body
        // anchor != provenance anchor → BindingMismatch. Here we seed a
        // sidecar at the provenance anchor but with a different body
        // anchor_fp to hit the binding check directly.
        let (db, sched, view) = warm_started_db();
        // Body says anchor 0x99, but we store it under A3B_ANCHOR and
        // build provenance for A3B_ANCHOR with the matching hash.
        let mut record = sample_sidecar(A3B_ANCHOR, EPOCH_576);
        record.anchor_fp = Hash32([0x99; 32]);
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        db.put_seed_epoch_consensus_inputs(&A3B_ANCHOR, &bytes)
            .expect("put under A3B_ANCHOR");
        let provenance = RecoveredBootstrapProvenance {
            anchor_fp: A3B_ANCHOR,
            sidecar_hash: blake2b_256(&bytes),
            epoch_no: EPOCH_576,
        };

        let err = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
            seed_epoch_consensus_source:
                SeedEpochConsensusSource::RequiredFromRecoveredProvenance(provenance),
        })
        .expect_err("anchor binding mismatch must fail closed");
        assert!(
            matches!(
                &err,
                BootstrapError::SeedConsensusBindingMismatch { actual_anchor_fp, .. }
                    if *actual_anchor_fp == Hash32([0x99; 32])
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn warm_start_fails_closed_on_epoch_mismatch() {
        // Sidecar binds epoch 576; provenance claims epoch 999 (and we
        // hash the real bytes so the hash check passes first).
        let (db, sched, view) = warm_started_db();
        let record = sample_sidecar(A3B_ANCHOR, EPOCH_576);
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        db.put_seed_epoch_consensus_inputs(&A3B_ANCHOR, &bytes)
            .expect("put");
        let provenance = RecoveredBootstrapProvenance {
            anchor_fp: A3B_ANCHOR,
            sidecar_hash: blake2b_256(&bytes),
            epoch_no: EpochNo(999), // wrong epoch
        };

        let err = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
            seed_epoch_consensus_source:
                SeedEpochConsensusSource::RequiredFromRecoveredProvenance(provenance),
        })
        .expect_err("epoch binding mismatch must fail closed");
        assert!(
            matches!(
                err,
                BootstrapError::SeedConsensusBindingMismatch { actual_epoch, expected_epoch, .. }
                    if actual_epoch == EPOCH_576 && expected_epoch == EpochNo(999)
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn warm_start_required_provenance_rejects_malformed_sidecar() {
        // Sidecar bytes present + hash matches, but the bytes do not
        // decode as a SeedEpochConsensusInputs → fail closed at decode.
        let (db, sched, view) = warm_started_db();
        let junk = vec![0xDE, 0xAD, 0xBE, 0xEF];
        db.put_seed_epoch_consensus_inputs(&A3B_ANCHOR, &junk)
            .expect("put junk");
        let provenance = RecoveredBootstrapProvenance {
            anchor_fp: A3B_ANCHOR,
            sidecar_hash: blake2b_256(&junk),
            epoch_no: EPOCH_576,
        };

        let err = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
            seed_epoch_consensus_source:
                SeedEpochConsensusSource::RequiredFromRecoveredProvenance(provenance),
        })
        .expect_err("malformed sidecar must fail closed");
        assert!(
            matches!(err, BootstrapError::SeedConsensusSidecarDecode(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn warm_start_never_falls_back_to_consensus_inputs_path() {
        // The no-fallback property is structural: the PRODUCTION
        // portion of this module (everything before the `#[cfg(test)]`
        // marker) references no forge-time bundle path token. A
        // required warm-start that cannot verify the sidecar fails
        // closed (the fail-closed tests above) — it never reaches for a
        // bundle. Scanning only the production prefix avoids self-
        // tripping on this test's own forbidden-token list, and avoids
        // matching doc-comments by also stripping line comments.
        let src = include_str!("bootstrap.rs");
        let production = match src.find("#[cfg(test)]") {
            Some(i) => &src[..i],
            None => src,
        };
        let code: String = production
            .lines()
            .map(|l| match l.find("//") {
                Some(i) => &l[..i],
                None => l,
            })
            .collect::<Vec<_>>()
            .join("\n");
        // Build the forbidden tokens from fragments so this assertion
        // text itself is not a literal occurrence of them.
        let forbidden = [
            format!("consensus{}inputs{}path", "-", "-"),
            format!("import{}live{}consensus{}inputs", "_", "_", "_"),
            format!("pool{}distr{}view{}from{}consensus{}inputs", "_", "_", "_", "_", "_"),
        ];
        for tok in &forbidden {
            assert!(
                !code.contains(tok.as_str()),
                "production bootstrap must not reference forge-time bundle token `{tok}` (no consensus-inputs-path fallback)"
            );
        }
    }
}
