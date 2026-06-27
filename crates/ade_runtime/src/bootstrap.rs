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
use ade_ledger::recovered_anchor_point::RecoveredAnchorPointError;
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
    /// PHASE4-N-AK AK-S1 (DC-NODE-31): the persisted recovered anchor point,
    /// loaded + fail-closed verified by the recover path
    /// (`warm_start_recovery`) and threaded in as a canonical input. `None`
    /// for cold-start / true-Origin and for every non-recover caller (which
    /// preserves the pre-AK tip behavior exactly); `Some` only on a warm-start
    /// recovery whose anchor-point record loaded and bound to the recovered
    /// `anchor_fp`. [`bootstrap_initial_state`] resolves the live-follow start
    /// tip from it via [`resolve_live_follow_start`] when ChainDb has no
    /// servable post-anchor block, so a bare-anchor recovery starts the
    /// FindIntersect at the anchor, not Origin.
    pub recovered_anchor: Option<ChainTip>,
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
    /// PHASE4-N-AH S4b (DC-NODE-22). Block number of the replay anchor, derived
    /// during warm-start recovery as:
    ///   recovered_tip.block_no - replayed_admit_count.
    /// This is NOT an independently persisted chain point. It is an auditable
    /// recovery summary used to distinguish bare-anchor recovery from recovery
    /// with a replayed local continuation spine. `None` on cold-start / first-run
    /// (only `warm_start_recovery` populates it).
    pub replayed_anchor_block_no: Option<u64>,
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
    /// (malformed / non-canonical). Fail-closed. (A schema-version mismatch is
    /// surfaced as the distinct `ConsensusInputsSchemaUnsupported` below, NOT here.)
    SeedConsensusSidecarDecode(SeedConsensusInputsError),
    /// ECA-2-pre (DC-CINPUT-06): the durable sidecar is an OLD schema version
    /// (pre-v4 — missing the consensus-profile hashes / eta0 / venue geometry). A
    /// TYPED upgrade/reimport requirement, DISTINCT from corruption: the store is
    /// well-formed but its schema predates this node's required version. Fail
    /// closed (no defaulting, no CLI/genesis re-supply); re-import to upgrade the
    /// sidecar to the current schema. Recoverable + auditable.
    ConsensusInputsSchemaUnsupported {
        found_version: u32,
        required_version: u32,
    },
    /// AK-S1 (DC-NODE-31): the recover path demanded the persisted recovered
    /// anchor-point record for `anchor_fp`, but none was stored — a non-Origin
    /// recovered store missing its anchor-point provenance. Fail-closed (no
    /// silent Origin fallback); re-recover to write the record.
    RecoveredAnchorPointMissing { anchor_fp: Hash32 },
    /// AK-S1: the recovered anchor-point bytes failed
    /// `decode_recovered_anchor_point` (malformed / unknown version /
    /// non-canonical / trailing bytes). Fail-closed.
    RecoveredAnchorPointDecode(RecoveredAnchorPointError),
    /// AK-S1: the decoded anchor-point record's `anchor_fp` did not match the
    /// recovered `anchor_fp` it was loaded for — the record is not bound to
    /// this recovered lineage. Fail-closed.
    RecoveredAnchorPointBindingMismatch {
        expected_anchor_fp: Hash32,
        actual_anchor_fp: Hash32,
    },
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
            replayed_anchor_block_no: None,
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

    // A3b: restore + verify the seed-epoch consensus-input sidecar BEFORE
    // materialize, so the recovered eta0 can overlay the replay chain_dep.
    // `NotRequired` preserves the pre-A3b warm-start behavior exactly. (Reordered
    // ahead of materialize in PHASE4-N-AN; the restore is independent of the
    // materialize result, so this changes order only, not outcome.)
    let seed_epoch_consensus_inputs = match inputs.seed_epoch_consensus_source {
        SeedEpochConsensusSource::NotRequired => None,
        SeedEpochConsensusSource::RequiredFromRecoveredProvenance(provenance) => {
            Some(restore_seed_epoch_consensus_inputs(inputs.snapshot_store, &provenance)?)
        }
    };

    // PHASE4-N-AN (T-REC-06): overlay the recovered eta0 INTO materialize so the
    // replay-forward fold validates each replayed block's header VRF against eta0,
    // NOT the snapshot's `Nonce::ZERO` placeholder. Without this, a WarmStart from
    // a NON-bare store (the WAL carries post-anchor blocks) fails replay VRF
    // (ReplayFailedAt VrfCert) — the SAME root cause as the live-rollback bug. For
    // a bare-anchor recovery (degenerate snapshot-at-target, no replay) it is a
    // no-op and the explicit post-overlay below still carries eta0.
    let (ledger, mut chain_dep) = materialize_rolled_back_state(
        target,
        &reader,
        &source,
        inputs.era_schedule,
        inputs.ledger_view,
        seed_epoch_consensus_inputs.as_ref().map(|s| &s.epoch_nonce),
    )
    .map_err(BootstrapError::Materialize)?;

    // PHASE4-N-F-G-N (T-REC-04 / DC-CINPUT-03): the explicit post-materialize
    // recovered-eta0 overlay (the SOLE site `ci_check_warmstart_eta0_overlay.sh`
    // checks). The snapshot's chain_dep carries the admission seed's PLACEHOLDER
    // eta0 (`Nonce::ZERO`, admission/bootstrap.rs:164); the authoritative
    // seed-epoch eta0 lives in the recovered consensus-input sidecar. Without
    // this, forge/self_accept sign the header VRF over ZERO and a real Conway peer
    // rejects it (VRFKeyBadProof). At the seed epoch (no blocks applied since the
    // seed) the evolving nonce equals eta0, so both are set — reconstructing
    // `genesis(eta0)`. Idempotent after the materialize overlay above.
    if let Some(sidecar) = &seed_epoch_consensus_inputs {
        // Only overlay onto the ZERO placeholder (the seed / cold-start snapshot). A snapshot
        // captured PAST the seed epoch already persists its real eta0 (the materialize overlay above
        // now preserves it); stamping the seed nonce over it would CLOBBER the correct nonce and fail
        // the first post-boundary header VRF. Mirrors the guard in
        // PraosChainDepState::overlay_recovered_eta0 (same zero-placeholder predicate).
        if chain_dep.epoch_nonce == ade_core::consensus::Nonce::ZERO {
            chain_dep.epoch_nonce = sidecar.epoch_nonce.clone();
            chain_dep.evolving_nonce = sidecar.epoch_nonce.clone();
        }
    }

    Ok(BootstrapState {
        ledger,
        chain_dep,
        // AK-S1 (DC-NODE-31): the live-follow start tip. A servable ChainDb tip
        // (a real post-anchor block) always wins; otherwise — a BARE-anchor
        // recovery, where `chaindb.tip()` is `None` because no servable block
        // exists above the anchor — fall back to the persisted recovered anchor
        // point (when the recover path loaded one). This is the FindIntersect
        // start surface ONLY; the materialization `target` above (OQ-AK-2) and
        // `ChainDb::tip()` are untouched, and a synthetic block is never made.
        tip: resolve_live_follow_start(tip, inputs.recovered_anchor),
        seed_epoch_consensus_inputs,
        // Cold-start / first-run / NotRequired warm-start: no replay anchor summary.
        // `warm_start_recovery` overrides this with the derived value when it replays.
        replayed_anchor_block_no: None,
    })
}

/// BLUE live-follow start-tip resolution (PHASE4-N-AK AK-S1, DC-NODE-31).
/// Pure and total — the single authority for which point a recovered node
/// FindIntersects from when starting its live follow.
///
/// Resolution order:
///   1. a servable ChainDb tip (a real post-anchor block durable in the store)
///      always wins;
///   2. else the persisted recovered anchor point, IF it is non-Origin
///      (non-zero hash) — a bare-anchor recovery surfaces the anchor as the
///      FindIntersect start so the wire pump does not start from Origin (which
///      the relay answers with `RollBackward(Origin)`, tripping the AI-S4a
///      fail-close);
///   3. else `None` (truly Origin / cold-start).
///
/// A zero/null-hash `recovered_anchor` is treated as Origin (rule 3): a genesis
/// seed point carries no block a peer could intersect, so it is not a usable
/// start. This does NOT change `ChainDb::tip()` semantics and never synthesizes
/// a servable block — it only chooses the start point from already-authoritative
/// inputs.
///
/// Module-private: the sole caller is [`bootstrap_initial_state`] (kept so
/// `bootstrap.rs` stays the single-`pub fn` bootstrap authority, CN-NODE-01).
fn resolve_live_follow_start(
    servable_chaindb_tip: Option<ChainTip>,
    recovered_anchor: Option<ChainTip>,
) -> Option<ChainTip> {
    if let Some(tip) = servable_chaindb_tip {
        return Some(tip);
    }
    match recovered_anchor {
        Some(anchor) if anchor.hash != Hash32([0u8; 32]) => Some(anchor),
        _ => None,
    }
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
    //    or non-canonical buffer fails here). ECA-2-pre (DC-CINPUT-06): a
    //    schema-VERSION mismatch (a pre-v4 sidecar) is a TYPED upgrade/reimport
    //    requirement, distinct from corruption — never lumped into the generic
    //    decode error, so an operator can tell "reimport the store" from "the
    //    store is corrupt".
    let sidecar = decode_seed_epoch_consensus_inputs(&bytes).map_err(|e| match e {
        SeedConsensusInputsError::UnknownVersion { expected, found } => {
            BootstrapError::ConsensusInputsSchemaUnsupported {
                found_version: found,
                required_version: expected,
            }
        }
        other => BootstrapError::SeedConsensusSidecarDecode(other),
    })?;

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

    use crate::recovered_anchor::load_recovered_anchor_point;

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
                randomness_stabilisation_window_slots: None,
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
            recovered_anchor: None,
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
            recovered_anchor: None,
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
            recovered_anchor: None,
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
                recovered_anchor: None,
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
            None,
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
            recovered_anchor: None,
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
        // The snapshot keeps the corpus (seed-epoch) eta0 -- a real, NON-zero nonce. This exercises
        // the post-seed path where the recovered-eta0 overlay is a NO-OP (the snapshot owns its nonce).
        warm_started_db_capture(None)
    }

    /// As `warm_started_db`, but the captured snapshot carries the genuine `Nonce::ZERO` placeholder
    /// the C1 admission seed persists (admission/bootstrap.rs). The real seed eta0 lives in the
    /// recovered sidecar; the overlay (DC-CINPUT-03) supplies it -- the case the overlay GUARD admits.
    fn warm_started_db_zero_eta0() -> (InMemoryChainDb, EraSchedule, PoolDistrView) {
        warm_started_db_capture(Some(Nonce::ZERO))
    }

    fn warm_started_db_capture(
        snapshot_eta0_override: Option<Nonce>,
    ) -> (InMemoryChainDb, EraSchedule, PoolDistrView) {
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
        // The C1 admission seed persists the `Nonce::ZERO` placeholder, NOT the live nonce; simulate
        // that when requested so the eta0-overlay guard (overlay ONLY the ZERO placeholder) is tested.
        if let Some(eta0) = snapshot_eta0_override {
            chain_dep.epoch_nonce = eta0.clone();
            chain_dep.evolving_nonce = eta0;
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
            epoch_start_slot: SlotNo(epoch.0 * 432_000),
            epoch_length_slots: 432_000,
            epoch_nonce: Nonce(Hash32([0x99; 32])),
            genesis_hash: Hash32([0x9a; 32]),
            protocol_params_hash: Hash32([0x9b; 32]),
            seed_point_slot: SlotNo(epoch.0 * 432_000 + 100),
            seed_point_hash: Hash32([0x6c; 32]),
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
            recovered_anchor: None,
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
        // snapshot. The snapshot's chain_dep here carries the genuine C1 admission seed
        // `Nonce::ZERO` placeholder; the persisted sidecar carries a DISTINCT eta0 ([0x99;32]). The
        // recovered chain_dep must equal the SIDECAR's eta0 — the exact bug fix (pre-G-N the forge
        // signed the header VRF over the snapshot's wrong nonce -> VRFKeyBadProof from a real Conway
        // peer). The eta0-overlay guard admits this because the snapshot's `epoch_nonce == ZERO`.
        let snapshot_eta0 = Nonce::ZERO;
        let (db, sched, view) = warm_started_db_zero_eta0();
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
            recovered_anchor: None,
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
    fn warm_start_keeps_post_seed_snapshot_eta0() {
        // The eta0-overlay GUARD (warm-start multi-boundary recovery, layer 2): a snapshot captured
        // PAST the seed epoch already persists its REAL epoch nonce (NOT the ZERO placeholder), so the
        // recovered-sidecar overlay must be a NO-OP -- stamping the seed eta0 over it would clobber the
        // correct nonce and fail the first post-boundary header VRF. Here the snapshot carries a
        // NON-zero corpus eta0; the recovered chain_dep must KEEP it, never take the sidecar's [0x99].
        let (corpus, _v) = corpus_view();
        let snapshot_eta0 = Nonce(Hash32(corpus.epoch_nonce));
        let (db, sched, view) = warm_started_db();
        let record = sample_sidecar(A3B_ANCHOR, EPOCH_576); // epoch_nonce = [0x99;32]
        assert_ne!(
            record.epoch_nonce, snapshot_eta0,
            "precondition: the sidecar eta0 must differ from the snapshot's real eta0"
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
            recovered_anchor: None,
        })
        .expect("required warm-start");

        assert_eq!(
            out.chain_dep.epoch_nonce, snapshot_eta0,
            "a post-seed snapshot's real eta0 must be KEPT (the overlay is a no-op on a non-ZERO nonce)"
        );
        assert_ne!(
            out.chain_dep.epoch_nonce, record.epoch_nonce,
            "the sidecar eta0 must NOT clobber the snapshot's persisted nonce"
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
            recovered_anchor: None,
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
            recovered_anchor: None,
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
            recovered_anchor: None,
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
            recovered_anchor: None,
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
            recovered_anchor: None,
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
            recovered_anchor: None,
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
            recovered_anchor: None,
        })
        .expect_err("malformed sidecar must fail closed");
        assert!(
            matches!(err, BootstrapError::SeedConsensusSidecarDecode(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn warm_start_pre_v4_sidecar_is_typed_schema_upgrade_not_corruption() {
        // ECA-2-pre (DC-CINPUT-06): a WELL-FORMED sidecar of an OLD schema
        // version (here v3) is a TYPED upgrade/reimport requirement
        // (ConsensusInputsSchemaUnsupported), DISTINCT from a corrupt buffer
        // (SeedConsensusSidecarDecode) — so an operator can tell "reimport the
        // store" from "the store is corrupt". Fail-closed but recoverable.
        let (db, sched, view) = warm_started_db();
        // Encode a valid current-schema (v5) sidecar, then rewrite the version
        // uint (index 1; index 0 is the array(13) header) from 0x05 to 0x03 so it
        // decodes as a pre-v5 sidecar. The provenance hash binds the spliced bytes,
        // so the hash check passes and the decode is what fails closed (on version).
        let mut old = encode_seed_epoch_consensus_inputs(&sample_sidecar(A3B_ANCHOR, EPOCH_576));
        old[1] = 0x03;
        db.put_seed_epoch_consensus_inputs(&A3B_ANCHOR, &old)
            .expect("put pre-v4 sidecar");
        let provenance = RecoveredBootstrapProvenance {
            anchor_fp: A3B_ANCHOR,
            sidecar_hash: blake2b_256(&old),
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
            recovered_anchor: None,
        })
        .expect_err("a pre-v4 sidecar must fail closed");
        assert!(
            matches!(
                err,
                BootstrapError::ConsensusInputsSchemaUnsupported {
                    found_version: 3,
                    required_version: 5
                }
            ),
            "a pre-v4 sidecar is a typed schema-upgrade requirement, not corruption; got {err:?}"
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

    // ===== PHASE4-N-AK AK-S1: recovered-anchor live-follow start (DC-NODE-31) =====

    /// Build a BARE-anchor warm-startable `InMemoryChainDb`: a snapshot
    /// captured at the anchor slot, with NO stored block — so `chaindb.tip()`
    /// is `None` (no servable post-anchor block) yet `list_snapshot_slots()` is
    /// non-empty, the exact shape `warm_start_recovery` faces for a recovered
    /// anchor. Returns the `(db, schedule, view, anchor_slot)`. Mirrors
    /// `warm_started_db` minus the `put_block`.
    fn bare_anchor_db() -> (InMemoryChainDb, EraSchedule, PoolDistrView, SlotNo) {
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
        // NO `put_block`: this is a bare anchor (snapshot only, no servable
        // post-anchor block). Capture the snapshot at the anchor slot.
        PersistentSnapshotCache::new(&db)
            .capture(snapshot_slot, &ledger, &chain_dep)
            .expect("capture");
        (db, sched, view, snapshot_slot)
    }

    const AK_ANCHOR_FP: Hash32 = Hash32([0x42; 32]);
    // A real (non-zero) anchor block hash — the recovered-tip case AK restores.
    const AK_ANCHOR_HASH: Hash32 = Hash32([0x2e; 32]);

    #[test]
    fn resolve_live_follow_start_treats_zero_hash_anchor_as_origin() {
        // Pure-fn unit (CE-AK-1). Resolution order: servable -> non-Origin
        // recovered anchor -> None; a zero-hash anchor is Origin (None).
        let zero = ChainTip {
            slot: SlotNo(188),
            hash: Hash32([0u8; 32]),
        };
        let real = ChainTip {
            slot: SlotNo(188),
            hash: AK_ANCHOR_HASH,
        };
        let servable = ChainTip {
            slot: SlotNo(200),
            hash: Hash32([0xAB; 32]),
        };
        // (3) zero-hash anchor is truly Origin => None.
        assert_eq!(resolve_live_follow_start(None, Some(zero)), None);
        // (2) a non-Origin recovered anchor surfaces as the start tip.
        assert_eq!(
            resolve_live_follow_start(None, Some(real.clone())),
            Some(real.clone())
        );
        // (1) a servable ChainDb tip always wins, even over a real anchor.
        assert_eq!(
            resolve_live_follow_start(Some(servable.clone()), Some(real)),
            Some(servable)
        );
        // No inputs => None (true Origin / cold-start).
        assert_eq!(resolve_live_follow_start(None, None), None);
    }

    #[test]
    fn bootstrap_bare_anchor_recovery_surfaces_anchor_as_live_follow_tip() {
        // A bare-anchor warm-start with a loaded recovered anchor (non-zero
        // hash) surfaces that anchor (slot + REAL hash) as the live-follow tip
        // -- NOT None. The materialization target used a null hash internally
        // (OQ-AK-2, unchanged); the live-follow tip carries the real hash.
        let (db, sched, view, anchor_slot) = bare_anchor_db();
        let anchor_tip = ChainTip {
            slot: anchor_slot,
            hash: AK_ANCHOR_HASH,
        };
        let out = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
            seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired,
            recovered_anchor: Some(anchor_tip.clone()),
        })
        .expect("bare-anchor bootstrap");
        assert_eq!(out.tip, Some(anchor_tip));
    }

    #[test]
    fn bootstrap_true_origin_recovery_surfaces_none_tip() {
        // A true cold-start (empty store) with no recovered anchor resolves to
        // None -- live-follow starts from Origin, correctly.
        let db = InMemoryChainDb::new();
        let (_corpus, view) = corpus_view();
        let sched = schedule();
        let genesis = fresh_genesis([0xAB; 32]);
        let out = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: Some(genesis),
            seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired,
            recovered_anchor: None,
        })
        .expect("cold-start");
        assert!(out.tip.is_none(), "true Origin / cold-start has no tip");
    }

    #[test]
    fn bootstrap_servable_chaindb_tip_wins_over_anchor() {
        // A warm-start with a SERVABLE post-anchor block (chaindb.tip() Some)
        // resolves to the servable tip, NEVER the recovered anchor -- even when
        // a (bogus, different) recovered anchor is supplied.
        let (db, sched, view) = warm_started_db();
        let servable = ChainDb::tip(&db)
            .expect("tip")
            .expect("warm_started_db has a servable block");
        let bogus_anchor = ChainTip {
            slot: SlotNo(999_999),
            hash: Hash32([0xEE; 32]),
        };
        let out = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
            seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired,
            recovered_anchor: Some(bogus_anchor.clone()),
        })
        .expect("warm-start");
        assert_eq!(out.tip, Some(servable));
        assert_ne!(out.tip, Some(bogus_anchor), "servable tip wins over anchor");
    }

    #[test]
    fn warm_start_loads_persisted_anchor_point() {
        // The store -> load -> resolve chain: persist the anchor-point record,
        // load + verify it (BLUE), then bootstrap resolves it as the tip.
        use ade_ledger::recovered_anchor_point::{
            encode_recovered_anchor_point, RecoveredAnchorPoint,
        };
        let (db, sched, view, anchor_slot) = bare_anchor_db();
        let record = RecoveredAnchorPoint {
            anchor_fp: AK_ANCHOR_FP,
            slot: anchor_slot,
            block_hash: AK_ANCHOR_HASH,
        };
        db.put_recovered_anchor_point(&AK_ANCHOR_FP, &encode_recovered_anchor_point(&record))
            .expect("put");

        let loaded = load_recovered_anchor_point(&db, &AK_ANCHOR_FP).expect("load");
        assert_eq!(
            loaded,
            ChainTip {
                slot: anchor_slot,
                hash: AK_ANCHOR_HASH
            }
        );

        let out = bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
            seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired,
            recovered_anchor: Some(loaded.clone()),
        })
        .expect("warm-start");
        assert_eq!(out.tip, Some(loaded));
    }

    #[test]
    fn warm_start_non_origin_anchor_missing_anchor_point_fails_closed() {
        // A non-Origin recovered store (snapshot present) with NO anchor-point
        // record fails closed at the load -- no silent Origin fallback.
        let (db, _sched, _view, _slot) = bare_anchor_db();
        let err = load_recovered_anchor_point(&db, &AK_ANCHOR_FP)
            .expect_err("missing anchor-point record must fail closed");
        assert!(
            matches!(&err, BootstrapError::RecoveredAnchorPointMissing { anchor_fp } if *anchor_fp == AK_ANCHOR_FP),
            "got {err:?}"
        );
    }

    #[test]
    fn warm_start_anchor_point_fingerprint_mismatch_fails_closed() {
        // A record whose BODY anchor_fp does not bind the recovered anchor_fp
        // it is loaded for fails closed (BindingMismatch). Store a record whose
        // body fp = X under key Y, then load by Y.
        use ade_ledger::recovered_anchor_point::{
            encode_recovered_anchor_point, RecoveredAnchorPoint,
        };
        let (db, _sched, _view, anchor_slot) = bare_anchor_db();
        let body_fp = Hash32([0x99; 32]);
        let record = RecoveredAnchorPoint {
            anchor_fp: body_fp.clone(),
            slot: anchor_slot,
            block_hash: AK_ANCHOR_HASH,
        };
        // Store under AK_ANCHOR_FP but with a body bound to a different fp.
        db.put_recovered_anchor_point(&AK_ANCHOR_FP, &encode_recovered_anchor_point(&record))
            .expect("put under AK_ANCHOR_FP");

        let err = load_recovered_anchor_point(&db, &AK_ANCHOR_FP)
            .expect_err("anchor-point binding mismatch must fail closed");
        assert!(
            matches!(
                &err,
                BootstrapError::RecoveredAnchorPointBindingMismatch { actual_anchor_fp, expected_anchor_fp }
                    if *actual_anchor_fp == body_fp && *expected_anchor_fp == AK_ANCHOR_FP
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn same_store_same_anchor_point_same_findintersect_start() {
        // Replay-equivalence of the recovered-tip surface (extends T-REC-05):
        // the same store + same persisted anchor point => byte-identical loaded
        // ChainTip => byte-identical BootstrapState.tip (the FindIntersect
        // start) AND byte-identical recovered ledger.
        use ade_ledger::recovered_anchor_point::{
            encode_recovered_anchor_point, RecoveredAnchorPoint,
        };
        let (db, sched, view, anchor_slot) = bare_anchor_db();
        let record = RecoveredAnchorPoint {
            anchor_fp: AK_ANCHOR_FP,
            slot: anchor_slot,
            block_hash: AK_ANCHOR_HASH,
        };
        db.put_recovered_anchor_point(&AK_ANCHOR_FP, &encode_recovered_anchor_point(&record))
            .expect("put");

        let load_and_bootstrap = || {
            let loaded = load_recovered_anchor_point(&db, &AK_ANCHOR_FP).expect("load");
            bootstrap_initial_state(BootstrapInputs {
                chaindb: &db,
                snapshot_store: &db,
                era_schedule: &sched,
                ledger_view: &view,
                genesis_initial: None,
                seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired,
                recovered_anchor: Some(loaded),
            })
            .expect("warm-start")
        };
        let r1 = load_and_bootstrap();
        let r2 = load_and_bootstrap();
        assert_eq!(r1.tip, r2.tip, "same store => same FindIntersect start tip");
        assert_eq!(
            r1.tip,
            Some(ChainTip {
                slot: anchor_slot,
                hash: AK_ANCHOR_HASH
            })
        );
        assert_eq!(
            fingerprint(&r1.ledger).combined,
            fingerprint(&r2.ledger).combined
        );
    }
}
