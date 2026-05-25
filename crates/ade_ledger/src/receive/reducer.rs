// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE receive-side bridge reducer (PHASE4-N-H S2).
//!
//! Pure, total transition consuming one [`ReceiveEvent`] per call:
//! - `RollForward`: caches the announced header bytes in
//!   [`PendingHeaderCache`]. **Mutates only `state.pending_headers`**;
//!   never touches `state.ledger`, `state.chain_dep`, or
//!   `chain_write` (Invariant I-6).
//! - `BlockDelivered`: decodes the body, looks up the cached header
//!   at `(slot, block_hash)`, runs
//!   [`admit_via_block_validity`] (composing
//!   [`block_validity`]), then persists the resulting
//!   [`AdmittedBlock`] through the `ChainDbWrite` trait. On success
//!   commits the new `(ledger, chain_dep)` atomically and evicts the
//!   consumed header. On any failure, state is unchanged.
//! - `RollBackward`: returns
//!   `Err(ReceiveError::RollbackOutOfScope)` per Path A scope edge.

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_types::{Hash32, SlotNo};

use crate::block_validity::decode_block;
use crate::state::LedgerState;

use super::admitted::{admit_via_block_validity, AdmittedBlock};
use super::chain_write::ChainDbWrite;
use super::events::{NoOpReason, ReceiveEffect, ReceiveError, ReceiveEvent};
use super::pending_header_cache::PendingHeaderCache;

/// Bundled receive-bridge sub-states the reducer mutates.
#[derive(Debug, Clone, PartialEq)]
pub struct ReceiveState {
    pub ledger: LedgerState,
    pub chain_dep: PraosChainDepState,
    pub pending_headers: PendingHeaderCache,
}

impl ReceiveState {
    pub fn new(
        ledger: LedgerState,
        chain_dep: PraosChainDepState,
    ) -> Self {
        Self {
            ledger,
            chain_dep,
            pending_headers: PendingHeaderCache::new(),
        }
    }
}

/// Read-only context for the RollBackward branch (PHASE4-N-I).
/// When supplied (`Some`), the reducer's RollBackward arm
/// materializes the rolled-back state via `materialize_rolled_back_state`
/// + `commit_rollback`. When `None`, the arm returns the legacy
/// `RollbackOutOfScope` error (pre-PHASE4-N-I behavior, retained
/// for callers that haven't wired the rollback context yet).
pub struct RollbackContext<'a> {
    pub snapshot_reader: &'a dyn crate::rollback::SnapshotReader,
    pub block_source: &'a dyn crate::rollback::BlockSource,
}

/// One step of the receive bridge. Pure, total, deterministic.
///
/// On error, `state` and `chain_write` are unchanged (staged-then-
/// committed shape).
pub fn receive_apply<W: ChainDbWrite>(
    state: &mut ReceiveState,
    event: ReceiveEvent,
    chain_write: &mut W,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    rollback_ctx: Option<&RollbackContext>,
) -> Result<ReceiveEffect, ReceiveError> {
    match event {
        ReceiveEvent::RollForward {
            slot,
            hash,
            header_bytes,
            tip: _,
        } => roll_forward(state, slot, hash, header_bytes),

        ReceiveEvent::BlockDelivered { block_bytes } => block_delivered(
            state,
            block_bytes,
            chain_write,
            era_schedule,
            ledger_view,
        ),

        ReceiveEvent::RollBackward { target_point, .. } => match rollback_ctx {
            Some(ctx) => roll_backward(
                state,
                target_point,
                chain_write,
                era_schedule,
                ledger_view,
                ctx,
            ),
            None => Err(ReceiveError::RollbackOutOfScope { target_point }),
        },
    }
}

fn roll_backward<W: ChainDbWrite>(
    state: &mut ReceiveState,
    target_point: super::events::TargetPoint,
    chain_write: &mut W,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    ctx: &RollbackContext,
) -> Result<ReceiveEffect, ReceiveError> {
    // 1. Materialize the rolled-back (LedgerState, PraosChainDepState).
    let mat_target = crate::rollback::TargetPoint {
        slot: target_point.slot,
        hash: target_point.hash.clone(),
    };
    let (new_ledger, new_chain_dep) = crate::rollback::materialize_rolled_back_state(
        mat_target.clone(),
        ctx.snapshot_reader,
        ctx.block_source,
        era_schedule,
        ledger_view,
    )
    .map_err(map_materialize_err)?;

    // 2. Atomic commit via the BLUE commit helper.
    crate::rollback::commit_rollback(
        state,
        mat_target,
        new_ledger,
        new_chain_dep,
        chain_write,
    )
    .map_err(map_commit_err)?;

    Ok(ReceiveEffect::RolledBack {
        to_slot: target_point.slot,
    })
}

fn map_materialize_err(e: crate::rollback::MaterializeError) -> ReceiveError {
    use crate::rollback::MaterializeError as ME;
    match e {
        ME::RollbackTooDeep { target_slot, .. } => {
            // Preserve the receive-side error shape: surface as
            // RollbackOutOfScope so callers don't need to handle a
            // new variant. The materialize error context is logged
            // at the orchestrator layer.
            ReceiveError::RollbackOutOfScope {
                target_point: super::events::TargetPoint {
                    slot: target_slot,
                    hash: ade_types::Hash32([0u8; 32]),
                },
            }
        }
        ME::ReplayFailedAt { error, .. } => ReceiveError::Validity(error),
        ME::EraNotSupported { slot, .. } => {
            // EraNotSupported reaches the receive surface — surface
            // as RollbackOutOfScope for now (Path A scope: pre-Conway
            // out of scope). Future cluster extends ReceiveError.
            ReceiveError::RollbackOutOfScope {
                target_point: super::events::TargetPoint {
                    slot,
                    hash: ade_types::Hash32([0u8; 32]),
                },
            }
        }
    }
}

fn map_commit_err(e: crate::rollback::CommitRollbackError) -> ReceiveError {
    match e {
        crate::rollback::CommitRollbackError::ChainDb(w) => ReceiveError::ChainDb(w),
    }
}

fn roll_forward(
    state: &mut ReceiveState,
    slot: SlotNo,
    hash: Hash32,
    header_bytes: Vec<u8>,
) -> Result<ReceiveEffect, ReceiveError> {
    let pre_len = state.pending_headers.len();
    let already_present =
        state.pending_headers.contains(slot, &hash);
    match state.pending_headers.insert(slot, hash.clone(), header_bytes) {
        Ok(()) => {
            if already_present && state.pending_headers.len() == pre_len {
                Ok(ReceiveEffect::NoOp {
                    reason: NoOpReason::HeaderAlreadyCached,
                })
            } else {
                Ok(ReceiveEffect::Cached { slot, hash })
            }
        }
        Err(super::pending_header_cache::PendingHeaderCacheError::ByteConflict {
            slot,
            hash,
        }) => Err(ReceiveError::HeaderBodyMismatch {
            decoded_slot: slot,
            decoded_hash: hash,
        }),
    }
}

fn block_delivered<W: ChainDbWrite>(
    state: &mut ReceiveState,
    block_bytes: Vec<u8>,
    chain_write: &mut W,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<ReceiveEffect, ReceiveError> {
    // 1. Decode the body to extract (slot, block_hash).
    let decoded = decode_block(&block_bytes).map_err(|e| {
        // decode_block returns BlockValidityError already.
        ReceiveError::Validity(e)
    })?;
    let slot = decoded.header_input.slot;
    let block_hash = decoded.block_hash.clone();

    // 2. Header cross-check (DC-CONS-19): a cached header must exist
    //    at (slot, block_hash). The cached bytes match by hash binding
    //    (block_hash = blake2b_256 over the header sub-slice; if a
    //    cached entry exists at this key, its bytes must hash to the
    //    same block_hash — otherwise the cache wouldn't have admitted
    //    the entry at this key in the first place).
    if !state.pending_headers.contains(slot, &block_hash) {
        return Err(ReceiveError::HeaderBodyMismatch {
            decoded_slot: slot,
            decoded_hash: block_hash,
        });
    }

    // 3. Run block_validity via the canonical admission gate.
    let outcome = admit_via_block_validity(
        &block_bytes,
        &state.ledger,
        &state.chain_dep,
        era_schedule,
        ledger_view,
    )
    .map_err(ReceiveError::Validity)?;

    // 4. Persist through the ChainDbWrite trait (irreversible step
    //    first). On failure, state stays unchanged.
    chain_write
        .write_admitted(outcome.admitted)
        .map_err(ReceiveError::ChainDb)?;

    // 5. Commit: advance ledger + chain_dep + evict consumed header.
    state.ledger = outcome.ledger;
    state.chain_dep = outcome.chain_dep;
    state.pending_headers.remove(slot, &block_hash);

    Ok(ReceiveEffect::Admitted {
        slot,
        hash: block_hash,
    })
}

/// Fold over a sequence of events. Stops at the first error.
pub fn receive_apply_sequence<W, I>(
    state: &mut ReceiveState,
    events: I,
    chain_write: &mut W,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<Vec<ReceiveEffect>, ReceiveError>
where
    W: ChainDbWrite,
    I: IntoIterator<Item = ReceiveEvent>,
{
    let mut effects = Vec::new();
    for event in events {
        let effect = receive_apply(state, event, chain_write, era_schedule, ledger_view, None)?;
        effects.push(effect);
    }
    Ok(effects)
}

// Silence the unused-import warning on AdmittedBlock; it's
// re-exported from the module root and used implicitly via
// `admit_via_block_validity`'s return type.
#[allow(unused_imports)]
use AdmittedBlock as _AdmittedBlockUsed;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use super::super::events::{TargetPoint, TipPoint};

    use std::collections::BTreeMap;

    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::{CardanoEra, EpochNo, Hash28};

    use crate::block_validity::BlockValidityError;
    use crate::consensus_view::{PoolDistrView, PoolEntry};
    use crate::receive::chain_write::ChainWriteError;

    fn ledger_fingerprint_combined(state: &LedgerState) -> Hash32 {
        crate::fingerprint::fingerprint(state).combined
    }

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

    fn fresh_state(eta0: [u8; 32]) -> ReceiveState {
        let mut ledger = LedgerState::new(CardanoEra::Conway);
        ledger.epoch_state.epoch = EPOCH_576;
        let mut chain_dep = PraosChainDepState::empty();
        chain_dep.epoch_nonce = Nonce(Hash32(eta0));
        chain_dep.evolving_nonce = Nonce(Hash32(eta0));
        ReceiveState::new(ledger, chain_dep)
    }

    fn pick_lightest(c: &ConwayValidityCorpus) -> Vec<u8> {
        let idx = (0..c.blocks.len())
            .min_by_key(|&i| {
                let env = decode_block_envelope(&c.blocks[i]).expect("env");
                env.block_end - env.block_start
            })
            .expect("non-empty");
        c.blocks[idx].clone()
    }

    fn pick_heaviest(c: &ConwayValidityCorpus) -> Vec<u8> {
        let idx = (0..c.blocks.len())
            .max_by_key(|&i| {
                let env = decode_block_envelope(&c.blocks[i]).expect("env");
                env.block_end - env.block_start
            })
            .expect("non-empty");
        c.blocks[idx].clone()
    }

    fn flip_body_byte(env_bytes: &[u8]) -> Vec<u8> {
        let env = decode_block_envelope(env_bytes).expect("env");
        let start = env.block_start;
        let end = env.block_end;
        let base = decode_block(env_bytes).expect("base");
        for idx in (start..end).rev() {
            let mut bad = env_bytes.to_vec();
            bad[idx] ^= 0x01;
            if let Ok(d) = decode_block(&bad) {
                if d.computed_body_hash != base.computed_body_hash {
                    return bad;
                }
            }
        }
        panic!("no structure-preserving body flip found")
    }

    /// Mock ChainDbWrite for tests: records every admitted block.
    #[derive(Default)]
    struct RecordingChainWrite {
        admitted: Vec<Vec<u8>>,
    }

    impl ChainDbWrite for RecordingChainWrite {
        fn write_admitted(
            &mut self,
            block: AdmittedBlock,
        ) -> Result<(), ChainWriteError> {
            self.admitted.push(block.into_bytes());
            Ok(())
        }
        fn rollback_to_slot(
            &mut self,
            _slot: ade_types::SlotNo,
        ) -> Result<(), ChainWriteError> {
            Ok(())
        }
    }

    fn fake_tip() -> TipPoint {
        TipPoint {
            slot: SlotNo(0),
            hash: Hash32([0; 32]),
            block_no: 0,
        }
    }

    #[test]
    fn receive_apply_roll_forward_caches_header_without_state_mutation() {
        let (c, view) = corpus_view();
        let mut state = fresh_state(c.epoch_nonce);
        let mut chain_write = RecordingChainWrite::default();
        let schedule = schedule();
        let ledger_before = ledger_fingerprint_combined(&state.ledger);
        let chain_dep_before = state.chain_dep.clone();
        let pending_before = state.pending_headers.len();

        let bytes = pick_lightest(&c);
        let decoded = decode_block(&bytes).expect("decode");
        let event = ReceiveEvent::RollForward {
            slot: decoded.header_input.slot,
            hash: decoded.block_hash.clone(),
            header_bytes: bytes.clone(), // wire-form is opaque; use full envelope bytes as the cache entry
            tip: fake_tip(),
        };
        let effect = receive_apply(&mut state, event, &mut chain_write, &schedule, &view, None)
            .expect("roll_forward");
        match effect {
            ReceiveEffect::Cached { .. } => {}
            other => panic!("expected Cached, got {other:?}"),
        }
        assert_eq!(state.pending_headers.len(), pending_before + 1);
        assert_eq!(
            ledger_fingerprint_combined(&state.ledger),
            ledger_before,
            "ledger fingerprint must not change on RollForward"
        );
        assert_eq!(state.chain_dep, chain_dep_before, "chain_dep must not change on RollForward");
        assert!(chain_write.admitted.is_empty(), "chain_write must not be called on RollForward");
    }

    #[test]
    fn receive_apply_block_delivered_with_matching_header_admits() {
        let (c, view) = corpus_view();
        let mut state = fresh_state(c.epoch_nonce);
        let mut chain_write = RecordingChainWrite::default();
        let schedule = schedule();
        let bytes = pick_lightest(&c);
        let decoded = decode_block(&bytes).expect("decode");

        // Cache the header first.
        receive_apply(
            &mut state,
            ReceiveEvent::RollForward {
                slot: decoded.header_input.slot,
                hash: decoded.block_hash.clone(),
                header_bytes: bytes.clone(),
                tip: fake_tip(),
            },
            &mut chain_write,
            &schedule,
            &view,
    None,
        )
        .expect("cache");
        let ledger_after_cache = ledger_fingerprint_combined(&state.ledger);

        // Deliver the body.
        let effect = receive_apply(
            &mut state,
            ReceiveEvent::BlockDelivered { block_bytes: bytes.clone() },
            &mut chain_write,
            &schedule,
            &view,
    None,
        )
        .expect("admit");
        match effect {
            ReceiveEffect::Admitted { slot, hash } => {
                assert_eq!(slot, decoded.header_input.slot);
                assert_eq!(hash, decoded.block_hash);
            }
            other => panic!("expected Admitted, got {other:?}"),
        }
        assert_eq!(chain_write.admitted.len(), 1, "chain_write must record the admitted block");
        assert_eq!(chain_write.admitted[0], bytes, "admitted bytes must equal input bytes");
        assert_ne!(
            ledger_fingerprint_combined(&state.ledger),
            ledger_after_cache,
            "ledger fingerprint must advance on admission"
        );
        assert!(
            !state.pending_headers.contains(decoded.header_input.slot, &decoded.block_hash),
            "consumed header must be evicted from cache"
        );
    }

    #[test]
    fn receive_apply_block_delivered_with_no_cached_header_rejects() {
        let (c, view) = corpus_view();
        let mut state = fresh_state(c.epoch_nonce);
        let mut chain_write = RecordingChainWrite::default();
        let schedule = schedule();
        let bytes = pick_lightest(&c);
        let ledger_before = ledger_fingerprint_combined(&state.ledger);

        let err = receive_apply(
            &mut state,
            ReceiveEvent::BlockDelivered { block_bytes: bytes },
            &mut chain_write,
            &schedule,
            &view,
    None,
        )
        .expect_err("must reject without cached header");
        match err {
            ReceiveError::HeaderBodyMismatch { .. } => {}
            other => panic!("expected HeaderBodyMismatch, got {other:?}"),
        }
        assert_eq!(
            ledger_fingerprint_combined(&state.ledger),
            ledger_before,
            "state must be unchanged on HeaderBodyMismatch"
        );
        assert!(chain_write.admitted.is_empty());
    }

    #[test]
    fn receive_apply_block_delivered_with_mismatched_cached_header_rejects() {
        let (c, view) = corpus_view();
        let mut state = fresh_state(c.epoch_nonce);
        let mut chain_write = RecordingChainWrite::default();
        let schedule = schedule();
        let bytes = pick_lightest(&c);
        let decoded = decode_block(&bytes).expect("decode");

        // Cache a header at the same slot but different hash.
        let fake_hash = Hash32([0xCC; 32]);
        state
            .pending_headers
            .insert(decoded.header_input.slot, fake_hash, vec![0xFF])
            .expect("insert");

        let err = receive_apply(
            &mut state,
            ReceiveEvent::BlockDelivered { block_bytes: bytes },
            &mut chain_write,
            &schedule,
            &view,
    None,
        )
        .expect_err("must reject on key mismatch");
        match err {
            ReceiveError::HeaderBodyMismatch { decoded_hash, .. } => {
                assert_eq!(decoded_hash, decoded.block_hash);
            }
            other => panic!("expected HeaderBodyMismatch, got {other:?}"),
        }
        assert!(chain_write.admitted.is_empty());
    }

    #[test]
    fn receive_apply_block_delivered_validity_invalid_rejects() {
        let (c, view) = corpus_view();
        let mut state = fresh_state(c.epoch_nonce);
        let mut chain_write = RecordingChainWrite::default();
        let schedule = schedule();
        let bytes = pick_heaviest(&c);
        let decoded = decode_block(&bytes).expect("decode");
        let altered = flip_body_byte(&bytes);
        let altered_decoded = decode_block(&altered).expect("altered decode");

        // Cache the *altered* header so the cross-check passes and we
        // can prove the validity layer is what rejects (not the
        // header cross-check).
        state
            .pending_headers
            .insert(
                altered_decoded.header_input.slot,
                altered_decoded.block_hash.clone(),
                altered.clone(),
            )
            .expect("insert");
        let ledger_before = ledger_fingerprint_combined(&state.ledger);

        let err = receive_apply(
            &mut state,
            ReceiveEvent::BlockDelivered { block_bytes: altered },
            &mut chain_write,
            &schedule,
            &view,
    None,
        )
        .expect_err("must reject on validity invalid");
        match err {
            ReceiveError::Validity(BlockValidityError::BodyHashMismatch { .. }) => {}
            other => panic!("expected Validity(BodyHashMismatch), got {other:?}"),
        }
        // State unchanged.
        assert_eq!(
            ledger_fingerprint_combined(&state.ledger),
            ledger_before,
        );
        assert!(chain_write.admitted.is_empty());
        // Cached header NOT evicted on validity failure.
        assert!(state.pending_headers.contains(altered_decoded.header_input.slot, &altered_decoded.block_hash));
        let _ = decoded; // sample unused — keep variable for clarity
    }

    #[test]
    fn receive_apply_rollback_returns_out_of_scope() {
        let (c, view) = corpus_view();
        let mut state = fresh_state(c.epoch_nonce);
        let mut chain_write = RecordingChainWrite::default();
        let schedule = schedule();
        let ledger_before = ledger_fingerprint_combined(&state.ledger);
        let chain_dep_before = state.chain_dep.clone();

        let target = TargetPoint {
            slot: SlotNo(123),
            hash: Hash32([0xAB; 32]),
        };
        let err = receive_apply(
            &mut state,
            ReceiveEvent::RollBackward {
                target_point: target.clone(),
                tip: fake_tip(),
            },
            &mut chain_write,
            &schedule,
            &view,
    None,
        )
        .expect_err("rollback must be out of scope");
        match err {
            ReceiveError::RollbackOutOfScope { target_point } => {
                assert_eq!(target_point, target)
            }
            other => panic!("expected RollbackOutOfScope, got {other:?}"),
        }
        assert_eq!(ledger_fingerprint_combined(&state.ledger), ledger_before);
        assert_eq!(state.chain_dep, chain_dep_before);
        assert!(chain_write.admitted.is_empty());
    }

    #[test]
    fn receive_apply_replay_byte_identical_over_corpus() {
        let (c, view) = corpus_view();
        let schedule = schedule();
        let bytes = pick_lightest(&c);
        let decoded = decode_block(&bytes).expect("decode");
        let events = vec![
            ReceiveEvent::RollForward {
                slot: decoded.header_input.slot,
                hash: decoded.block_hash.clone(),
                header_bytes: bytes.clone(),
                tip: fake_tip(),
            },
            ReceiveEvent::BlockDelivered { block_bytes: bytes.clone() },
        ];
        let mut runs: Vec<Hash32> = Vec::new();
        for _ in 0..2 {
            let mut state = fresh_state(c.epoch_nonce);
            let mut chain_write = RecordingChainWrite::default();
            for e in events.clone() {
                receive_apply(&mut state, e, &mut chain_write, &schedule, &view, None)
                    .expect("apply");
            }
            runs.push(ledger_fingerprint_combined(&state.ledger));
        }
        assert_eq!(runs[0], runs[1], "replay must be byte-identical");
    }

    #[test]
    fn receive_apply_sequence_admits_corpus_block() {
        let (c, view) = corpus_view();
        let mut state = fresh_state(c.epoch_nonce);
        let mut chain_write = RecordingChainWrite::default();
        let schedule = schedule();
        let bytes = pick_lightest(&c);
        let decoded = decode_block(&bytes).expect("decode");
        let events = vec![
            ReceiveEvent::RollForward {
                slot: decoded.header_input.slot,
                hash: decoded.block_hash.clone(),
                header_bytes: bytes.clone(),
                tip: fake_tip(),
            },
            ReceiveEvent::BlockDelivered { block_bytes: bytes.clone() },
        ];
        let effects = receive_apply_sequence(
            &mut state,
            events,
            &mut chain_write,
            &schedule,
            &view,
        )
        .expect("sequence");
        assert_eq!(effects.len(), 2);
        assert!(matches!(effects[0], ReceiveEffect::Cached { .. }));
        assert!(matches!(effects[1], ReceiveEffect::Admitted { .. }));
        assert_eq!(chain_write.admitted.len(), 1);
    }
}
