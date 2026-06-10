// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN forward-sync lifecycle reducer (PHASE4-N-Y S2).
//!
//! Composes the BLUE admit authority
//! (`ade_ledger::receive::receive_apply` /
//! `admit_via_block_validity`) with the durability cadence and emits
//! a closed [`SyncEffect`] set the RED pump applies in order.
//!
//! DC-SYNC-01 (the slice's whole point): the reducer MUST NOT emit
//! [`SyncEffect::AdvanceTip`] for a block until that block's
//! [`SyncEffect::StoreBlockBytes`] + [`SyncEffect::AppendWal`] effects
//! have been emitted in the same step. The ordering is encoded in the
//! type system: the only constructor for an admit plan that contains
//! an `AdvanceTip` is [`AdmitPlan::durable`], which produces the
//! effects in the fixed durable-before-tip order. There is no public
//! way to construct an `AdmitPlan` whose `AdvanceTip` precedes its
//! `StoreBlockBytes`/`AppendWal`.
//!
//! GREEN-by-content: this module holds no socket / clock / redb /
//! tokio / HashMap / float / String-error state. The fetch I/O and
//! the redb + WAL writes live in the RED pump
//! (`super::pump`). CI asserts the purity grep
//! (`ci/ci_check_forward_sync_chokepoint_only.sh`).

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_ledger::receive::{
    receive_apply, ReceiveEffect, ReceiveError, ReceiveEvent, ReceiveState,
};
use ade_ledger::wal::{BlockVerdictTag, WalEntry};
use ade_types::{BlockNo, Hash32, SlotNo};

use crate::chaindb::{ChainTip, StoredBlock};
use crate::rollback::cadence::{should_snapshot_after_block, SnapshotCadence};

/// Closed forward-sync effect set. The RED pump applies these in the
/// order they appear in [`AdmitPlan::effects`].
///
/// `AdvanceTip` is only ever the last effect of a durable admit plan;
/// it is unreachable until `StoreBlockBytes` + `AppendWal` for the
/// same block precede it (DC-SYNC-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncEffect {
    /// Persist the preserved wire bytes (`ChainDb::put_block`). MUST
    /// be applied (and acknowledged durable) before `AdvanceTip`.
    StoreBlockBytes(StoredBlock),
    /// Append the Ade-canonical WAL entry (`FileWalStore::append`).
    /// MUST be applied (and acknowledged durable) before `AdvanceTip`.
    AppendWal(WalEntry),
    /// Capture a checkpoint snapshot per the cadence. Optional; when
    /// present it precedes `AdvanceTip`.
    CommitCheckpoint { slot: SlotNo },
    /// Advance the chain tip to the admitted block. The pump issues
    /// the tip write only after the preceding durability effects ack.
    AdvanceTip { slot: SlotNo, hash: Hash32 },
}

/// The ordered effect plan for one admitted block. Private field: the
/// only constructor is [`AdmitPlan::durable`], which fixes the
/// durable-before-tip order. An out-of-order construction is not
/// expressible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmitPlan {
    effects: Vec<SyncEffect>,
}

impl AdmitPlan {
    /// Build the durable admit plan in the fixed order:
    /// `StoreBlockBytes`, `AppendWal`, [`CommitCheckpoint`],
    /// `AdvanceTip`. This is the sole site that emits `AdvanceTip`,
    /// and it always emits the two durability effects first.
    fn durable(
        stored: StoredBlock,
        wal: WalEntry,
        checkpoint_slot: Option<SlotNo>,
        tip_slot: SlotNo,
        tip_hash: Hash32,
    ) -> Self {
        let mut effects = Vec::with_capacity(4);
        effects.push(SyncEffect::StoreBlockBytes(stored));
        effects.push(SyncEffect::AppendWal(wal));
        if let Some(slot) = checkpoint_slot {
            effects.push(SyncEffect::CommitCheckpoint { slot });
        }
        effects.push(SyncEffect::AdvanceTip {
            slot: tip_slot,
            hash: tip_hash,
        });
        Self { effects }
    }

    /// A non-admit plan (header cached / no-op): emits no durability
    /// or tip effects.
    fn empty() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    /// The ordered effects. The RED pump applies them front-to-back.
    pub fn effects(&self) -> &[SyncEffect] {
        &self.effects
    }

    /// Consume into the ordered effect vector.
    pub fn into_effects(self) -> Vec<SyncEffect> {
        self.effects
    }

    /// The byte-index at which `AdvanceTip` appears, or `None` if this
    /// plan advances no tip. Used by the ordering test to assert the
    /// durability effects strictly precede it.
    pub fn tip_index(&self) -> Option<usize> {
        self.effects
            .iter()
            .position(|e| matches!(e, SyncEffect::AdvanceTip { .. }))
    }
}

/// GREEN forward-sync lifecycle state. Holds only authoritative,
/// replay-derivable values — no I/O handles, no clock, no socket.
///
/// `receive` is the BLUE admit reducer state (ledger + chain_dep +
/// pending headers). `prior_fp` is the running fingerprint chain link
/// for the WAL (the anchor's `initial_ledger_fingerprint` seeds it).
/// `last_checkpoint` is the cadence cursor.
#[derive(Debug, Clone)]
pub struct ForwardSyncState {
    pub receive: ReceiveState,
    pub prior_fp: Hash32,
    pub cadence: SnapshotCadence,
    pub last_checkpoint: Option<SlotNo>,
    /// PHASE4-N-AK AK-S2 (DC-NODE-32): the recovered bootstrap anchor point
    /// (AK-S1 / DC-NODE-31 / `BootstrapState.tip`), carried so the
    /// single-producer follow (`run_node_sync`) can recognise the relay's
    /// post-intersection `RollBackward(anchor)` as an idempotent boundary
    /// rewind. A replay-derivable, store-derived value (the SINGLE anchor
    /// authority — `run_node_sync` consumes this, never re-reads the store).
    /// `None` for cold-start / non-recover callers (pre-AK-S2 behaviour: any
    /// rollback fails closed). It is NOT a servable block and is never
    /// synthesized into one.
    pub recovered_anchor: Option<ChainTip>,
}

impl ForwardSyncState {
    /// Seed from the verified anchor: the receive sub-state plus the
    /// anchor's initial ledger fingerprint as the first WAL link.
    /// `recovered_anchor` defaults to `None`; the recover path
    /// (`run_node_lifecycle`) sets it from `BootstrapState.tip` (AK-S2).
    pub fn new(
        receive: ReceiveState,
        anchor_fingerprint: Hash32,
        cadence: SnapshotCadence,
    ) -> Self {
        Self {
            receive,
            prior_fp: anchor_fingerprint,
            cadence,
            last_checkpoint: None,
            recovered_anchor: None,
        }
    }
}

/// One forward-sync step. Pure, total, deterministic. Composes the
/// BLUE admit reducer and derives the ordered durable effect plan.
///
/// Authoritative shape `(state, event) -> Result<(/* via &mut */),
/// effects), error>`: state is advanced in place by the BLUE reducer
/// (staged-then-committed; on error state is unchanged), and the
/// returned [`AdmitPlan`] carries the ordered effects.
pub fn forward_sync_step<W: ade_ledger::receive::ChainDbWrite>(
    state: &mut ForwardSyncState,
    event: ReceiveEvent,
    chain_write: &mut W,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<AdmitPlan, ReceiveError> {
    // The BLUE admit chokepoint. Block bytes are needed for the
    // preserved-byte store effect; capture them before the move.
    let admitted_bytes = match &event {
        ReceiveEvent::BlockDelivered { block_bytes } => Some(block_bytes.clone()),
        _ => None,
    };

    let prior_fp = state.prior_fp.clone();

    let effect = receive_apply(
        &mut state.receive,
        event,
        chain_write,
        era_schedule,
        ledger_view,
        None,
    )?;

    match effect {
        ReceiveEffect::Admitted { slot, hash } => {
            // BlockDelivered is the only path that yields Admitted, so
            // admitted_bytes is Some here.
            let bytes = admitted_bytes.unwrap_or_default();
            let stored = StoredBlock {
                slot,
                hash: hash.clone(),
                bytes,
            };

            // New running fingerprint = post-admit ledger fingerprint.
            let post_fp =
                ade_ledger::fingerprint::fingerprint(&state.receive.ledger).combined;
            let wal = WalEntry::AdmitBlock {
                prior_fp,
                block_hash: hash.clone(),
                slot,
                verdict: BlockVerdictTag::Valid,
                post_fp: post_fp.clone(),
            };
            state.prior_fp = post_fp;

            let block_no = block_no_of(&state.receive, slot);
            let checkpoint_slot = match block_no {
                Some(bn)
                    if should_snapshot_after_block(
                        slot,
                        bn,
                        state.cadence,
                        state.last_checkpoint,
                    ) =>
                {
                    state.last_checkpoint = Some(slot);
                    Some(slot)
                }
                _ => None,
            };

            Ok(AdmitPlan::durable(stored, wal, checkpoint_slot, slot, hash))
        }
        ReceiveEffect::Cached { .. }
        | ReceiveEffect::RolledBack { .. }
        | ReceiveEffect::NoOp { .. } => Ok(AdmitPlan::empty()),
    }
}

/// The block number for the just-admitted block at `slot`, read from
/// the advanced chain-dep state. Used only for the cadence decision;
/// `None` falls back to no-checkpoint.
fn block_no_of(state: &ReceiveState, slot: SlotNo) -> Option<BlockNo> {
    let _ = slot;
    state.chain_dep.last_block_no
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_core::consensus::praos_state::PraosChainDepState;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
    use ade_ledger::block_validity::decode_block;
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::receive::{AdmittedBlock, ChainDbWrite, ChainWriteError};
    use ade_ledger::state::LedgerState;
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::{CardanoEra, EpochNo, Hash28};

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

    fn fresh_state(eta0: [u8; 32]) -> ForwardSyncState {
        let mut ledger = LedgerState::new(CardanoEra::Conway);
        ledger.epoch_state.epoch = EPOCH_576;
        let mut chain_dep = PraosChainDepState::empty();
        chain_dep.epoch_nonce = Nonce(Hash32(eta0));
        chain_dep.evolving_nonce = Nonce(Hash32(eta0));
        ForwardSyncState::new(
            ReceiveState::new(ledger, chain_dep),
            Hash32([0xA0; 32]),
            SnapshotCadence::DEFAULT,
        )
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

    /// Recording ChainDbWrite that captures admitted bytes.
    #[derive(Default)]
    struct RecordingChainWrite {
        admitted: Vec<Vec<u8>>,
    }

    impl ChainDbWrite for RecordingChainWrite {
        fn write_admitted(&mut self, block: AdmittedBlock) -> Result<(), ChainWriteError> {
            self.admitted.push(block.into_bytes());
            Ok(())
        }
        fn rollback_to_slot(&mut self, _slot: SlotNo) -> Result<(), ChainWriteError> {
            Ok(())
        }
    }

    fn cache_and_deliver_events(bytes: &[u8]) -> (ReceiveEvent, ReceiveEvent) {
        let decoded = decode_block(bytes).expect("decode");
        (
            ReceiveEvent::RollForward {
                slot: decoded.header_input.slot,
                hash: decoded.block_hash.clone(),
                header_bytes: bytes.to_vec(),
                tip: ade_ledger::receive::TipPoint {
                    slot: SlotNo(0),
                    hash: Hash32([0; 32]),
                    block_no: 0,
                },
            },
            ReceiveEvent::BlockDelivered {
                block_bytes: bytes.to_vec(),
            },
        )
    }

    #[test]
    fn forward_sync_wal_and_bytes_precede_tip_advance() {
        let (c, view) = corpus_view();
        let sched = schedule();
        let bytes = pick_lightest(&c);
        let (cache_ev, deliver_ev) = cache_and_deliver_events(&bytes);

        let mut state = fresh_state(c.epoch_nonce);
        let mut cw = RecordingChainWrite::default();

        // Cache step → empty plan (no durability, no tip).
        let cached = forward_sync_step(&mut state, cache_ev, &mut cw, &sched, &view)
            .expect("cache");
        assert!(cached.effects().is_empty(), "cache step emits no effects");
        assert!(cached.tip_index().is_none());

        // Admit step → durable plan.
        let plan = forward_sync_step(&mut state, deliver_ev, &mut cw, &sched, &view)
            .expect("admit");
        let effects = plan.effects();

        let tip_idx = plan.tip_index().expect("admit plan advances a tip");
        // The two durability effects MUST appear strictly before the
        // tip-advance index (DC-SYNC-01).
        let store_idx = effects
            .iter()
            .position(|e| matches!(e, SyncEffect::StoreBlockBytes(_)))
            .expect("StoreBlockBytes present");
        let wal_idx = effects
            .iter()
            .position(|e| matches!(e, SyncEffect::AppendWal(_)))
            .expect("AppendWal present");
        assert!(store_idx < tip_idx, "StoreBlockBytes must precede AdvanceTip");
        assert!(wal_idx < tip_idx, "AppendWal must precede AdvanceTip");
        // AdvanceTip is the final effect.
        assert_eq!(tip_idx, effects.len() - 1, "AdvanceTip is last");

        // No effect after the tip advance.
        assert!(!effects[tip_idx + 1..]
            .iter()
            .any(|e| matches!(e, SyncEffect::AdvanceTip { .. })));
    }

    #[test]
    fn forward_sync_admission_through_chokepoints() {
        // A block whose header was never cached fails the BLUE admit
        // chokepoint (HeaderBodyMismatch) → no plan, no effects.
        let (c, view) = corpus_view();
        let sched = schedule();
        let bytes = pick_lightest(&c);
        let (_cache_ev, deliver_ev) = cache_and_deliver_events(&bytes);

        let mut state = fresh_state(c.epoch_nonce);
        let mut cw = RecordingChainWrite::default();

        // Deliver WITHOUT caching the header first → the chokepoint
        // rejects; no store/WAL/tip occurs.
        let err = forward_sync_step(&mut state, deliver_ev, &mut cw, &sched, &view)
            .expect_err("must reject through chokepoint");
        match err {
            ReceiveError::HeaderBodyMismatch { .. } => {}
            other => panic!("expected HeaderBodyMismatch, got {other:?}"),
        }
        assert!(cw.admitted.is_empty(), "no block stored on chokepoint reject");
        // Running fingerprint unchanged → no WAL link advanced.
        assert_eq!(state.prior_fp, Hash32([0xA0; 32]));
    }
}
