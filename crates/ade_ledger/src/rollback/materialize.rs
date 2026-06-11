// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE rollback materialize driver (PHASE4-N-I S2).
//!
//! Single canonical authority for materializing the rolled-back
//! `(LedgerState, PraosChainDepState)` at a target point: snapshot
//! lookup + replay-forward fold over `block_validity` (the same
//! admission authority N-H's receive bridge uses). Pure, total,
//! deterministic.
//!
//! CN-STORE-07: this is the SOLE `pub fn` in the project returning
//! `(LedgerState, PraosChainDepState)`. CI grep enforces.

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_types::{CardanoEra, Hash32, SlotNo};

use crate::block_validity::transition::{block_validity, BlockValidityOutcome};
use crate::block_validity::verdict::BlockValidityVerdict;
use crate::state::LedgerState;

use super::error::MaterializeError;
use super::traits::{BlockSource, SnapshotReader};

/// Target point of a rollback. `hash` is recorded but not enforced
/// at S2; S6's integration test layer handles hash equality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetPoint {
    pub slot: SlotNo,
    pub hash: Hash32,
}

/// The sole materialize authority. Composes one snapshot lookup +
/// replay-forward via `block_validity`. Returns `(LedgerState,
/// PraosChainDepState)` at `target.slot` (or a structured error).
pub fn materialize_rolled_back_state(
    target: TargetPoint,
    reader: &dyn SnapshotReader,
    source: &dyn BlockSource,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    recovered_eta0: Option<&Nonce>,
) -> Result<(LedgerState, PraosChainDepState), MaterializeError> {
    // 1. Find nearest snapshot ≤ target.
    let (snapshot_slot, mut ledger, mut chain_dep) = match reader.nearest_le(target.slot) {
        Some(s) => s,
        None => {
            return Err(MaterializeError::RollbackTooDeep {
                target_slot: target.slot,
                oldest_snapshot: None,
            })
        }
    };

    // PHASE4-N-AN (T-REC-06): overlay the recovered seed-epoch eta0 onto the
    // snapshot chain_dep — the SAME overlay WarmStart bootstrap applies to the
    // live-admit chain_dep — BEFORE the degenerate return AND the replay-forward
    // fold, so rollback replay validates the header VRF against the same nonce as
    // live admit (replay-equivalence). The persisted snapshot carries the
    // `Nonce::ZERO` placeholder; eta0 is the recovered canonical input (the
    // seed-epoch sidecar). `None` (cold-start / no recovered sidecar) keeps the
    // snapshot's nonce as-is. NOTE: VRF strength is UNCHANGED — overlaying the
    // correct nonce is not a bypass; a block whose VRF verifies against NEITHER
    // eta0 nor the snapshot nonce still fails closed.
    if let Some(eta0) = recovered_eta0 {
        chain_dep.overlay_recovered_eta0(eta0);
    }

    // 2. Degenerate: snapshot exactly at target.
    if snapshot_slot == target.slot {
        return Ok((ledger, chain_dep));
    }

    // 3. Replay-forward over blocks in (snapshot_slot, target.slot].
    let blocks = source.blocks_in_range(snapshot_slot, target.slot);
    for (slot, block_bytes) in blocks {
        // Era detection — pre-Conway is out of scope per PHASE4-N-I.
        let env = decode_block_envelope(&block_bytes).map_err(|e| {
            MaterializeError::ReplayFailedAt {
                slot,
                error: crate::block_validity::BlockValidityError::Body(
                    crate::error::LedgerError::from(e),
                ),
            }
        })?;
        if !is_supported_era(env.era) {
            return Err(MaterializeError::EraNotSupported {
                era: env.era,
                slot,
            });
        }

        let BlockValidityOutcome {
            verdict,
            ledger: new_ledger,
            chain_dep: new_chain_dep,
        } = block_validity(&ledger, &chain_dep, era_schedule, ledger_view, &block_bytes);
        match verdict {
            BlockValidityVerdict::Valid { .. } => {
                ledger = new_ledger;
                chain_dep = new_chain_dep;
            }
            BlockValidityVerdict::Invalid { error, .. } => {
                return Err(MaterializeError::ReplayFailedAt { slot, error });
            }
        }
    }

    Ok((ledger, chain_dep))
}

fn is_supported_era(era: CardanoEra) -> bool {
    matches!(era, CardanoEra::Babbage | CardanoEra::Conway)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::{EpochNo, Hash28};

    use crate::block_validity::decode_block;
    use crate::consensus_view::{PoolDistrView, PoolEntry};

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

    fn fresh_ledger() -> LedgerState {
        let mut l = LedgerState::new(CardanoEra::Conway);
        l.epoch_state.epoch = EPOCH_576;
        l
    }

    fn fresh_chain_dep(eta0: [u8; 32]) -> PraosChainDepState {
        let mut s = PraosChainDepState::empty();
        s.epoch_nonce = Nonce(Hash32(eta0));
        s.evolving_nonce = Nonce(Hash32(eta0));
        s
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

    /// Test reader: a single in-memory snapshot.
    struct OneSnapshotReader {
        slot: SlotNo,
        ledger: LedgerState,
        chain_dep: PraosChainDepState,
    }
    impl SnapshotReader for OneSnapshotReader {
        fn nearest_le(
            &self,
            target_slot: SlotNo,
        ) -> Option<(SlotNo, LedgerState, PraosChainDepState)> {
            if self.slot <= target_slot {
                Some((self.slot, self.ledger.clone(), self.chain_dep.clone()))
            } else {
                None
            }
        }
    }

    /// Test source: a fixed list of (slot, bytes).
    struct ListBlockSource {
        blocks: Vec<(SlotNo, Vec<u8>)>,
    }
    impl BlockSource for ListBlockSource {
        fn blocks_in_range(
            &self,
            from_exclusive: SlotNo,
            to_inclusive: SlotNo,
        ) -> Vec<(SlotNo, Vec<u8>)> {
            self.blocks
                .iter()
                .filter(|(s, _)| s.0 > from_exclusive.0 && s.0 <= to_inclusive.0)
                .cloned()
                .collect()
        }
    }

    struct EmptyReader;
    impl SnapshotReader for EmptyReader {
        fn nearest_le(
            &self,
            _t: SlotNo,
        ) -> Option<(SlotNo, LedgerState, PraosChainDepState)> {
            None
        }
    }

    fn ledger_fp(state: &LedgerState) -> Hash32 {
        crate::fingerprint::fingerprint(state).combined
    }

    #[test]
    fn materialize_returns_rollback_too_deep_when_no_snapshot() {
        let (_c, view) = corpus_view();
        let source = ListBlockSource { blocks: vec![] };
        let target = TargetPoint {
            slot: SlotNo(100),
            hash: Hash32([0u8; 32]),
        };
        let err =
            materialize_rolled_back_state(target, &EmptyReader, &source, &schedule(), &view, None)
                .expect_err("must reject");
        match err {
            MaterializeError::RollbackTooDeep {
                target_slot,
                oldest_snapshot,
            } => {
                assert_eq!(target_slot, SlotNo(100));
                assert!(oldest_snapshot.is_none());
            }
            other => panic!("expected RollbackTooDeep, got {other:?}"),
        }
    }

    #[test]
    fn materialize_with_snapshot_at_target_returns_snapshot_state() {
        let (c, view) = corpus_view();
        let ledger = fresh_ledger();
        let chain_dep = fresh_chain_dep(c.epoch_nonce);
        let reader = OneSnapshotReader {
            slot: SlotNo(42),
            ledger: ledger.clone(),
            chain_dep: chain_dep.clone(),
        };
        let source = ListBlockSource { blocks: vec![] };
        let target = TargetPoint {
            slot: SlotNo(42),
            hash: Hash32([0u8; 32]),
        };
        let (got_ledger, got_chain_dep) =
            materialize_rolled_back_state(target, &reader, &source, &schedule(), &view, None).expect("ok");
        assert_eq!(ledger_fp(&got_ledger), ledger_fp(&ledger));
        assert_eq!(got_chain_dep, chain_dep);
    }

    #[test]
    fn materialize_with_snapshot_below_target_replays_forward() {
        let (c, view) = corpus_view();
        let bytes = pick_lightest(&c);
        let decoded = decode_block(&bytes).expect("decode");
        let ledger = fresh_ledger();
        let chain_dep = fresh_chain_dep(c.epoch_nonce);
        let snapshot_slot = SlotNo(decoded.header_input.slot.0 - 1);
        let reader = OneSnapshotReader {
            slot: snapshot_slot,
            ledger,
            chain_dep,
        };
        let source = ListBlockSource {
            blocks: vec![(decoded.header_input.slot, bytes.clone())],
        };
        let target = TargetPoint {
            slot: decoded.header_input.slot,
            hash: decoded.block_hash.clone(),
        };
        let (got_ledger, _got_chain_dep) =
            materialize_rolled_back_state(target, &reader, &source, &schedule(), &view, None)
                .expect("ok");
        // Fingerprint must equal direct-apply result.
        let direct = {
            let l = fresh_ledger();
            let cd = fresh_chain_dep(c.epoch_nonce);
            let outcome = block_validity(&l, &cd, &schedule(), &view, &bytes);
            match outcome.verdict {
                BlockValidityVerdict::Valid { .. } => outcome.ledger,
                BlockValidityVerdict::Invalid { .. } => panic!("direct apply must succeed"),
            }
        };
        assert_eq!(ledger_fp(&got_ledger), ledger_fp(&direct));
    }

    /// AN-S1→S2 (PHASE4-N-AN, T-REC-06): the persisted snapshot carries the `Nonce::ZERO` PLACEHOLDER
    /// epoch_nonce, NOT the recovered eta0. WITHOUT the overlay (`None`) rollback materialization replays
    /// the rolled-back block against the placeholder and fails the header VRF — the divergence AN-S1
    /// reproduced. WITH the recovered eta0 overlaid (`Some(eta0)`) the SAME block replays Valid and the
    /// materialized chain_dep carries eta0 — byte-equal to the live-admit nonce basis. That is the
    /// replay-equivalence T-REC-06 demands (CE-AN-2 + CE-AN-3). The `None` arm proves the overlay — not a
    /// VRF skip — is what fixes it.
    #[test]
    fn rollback_materialize_overlays_recovered_eta0_replay_equivalent() {
        let (c, view) = corpus_view();
        let eta0_bytes = c.epoch_nonce;
        let placeholder = [0u8; 32];
        assert_ne!(
            placeholder, eta0_bytes,
            "the persisted-snapshot placeholder nonce differs from the recovered eta0"
        );
        let eta0 = Nonce(Hash32(eta0_bytes));

        let bytes = pick_lightest(&c);
        let decoded = decode_block(&bytes).expect("decode");

        // (a) LIVE-ADMIT control: the corpus block's VRF verifies against the recovered eta0.
        let live = block_validity(&fresh_ledger(), &fresh_chain_dep(eta0_bytes), &schedule(), &view, &bytes);
        assert!(
            matches!(live.verdict, BlockValidityVerdict::Valid { .. }),
            "live admit (eta0 chain_dep) must validate the corpus block, got {:?}",
            live.verdict
        );

        let snapshot_slot = SlotNo(decoded.header_input.slot.0 - 1);
        let mk_reader = || OneSnapshotReader {
            slot: snapshot_slot,
            ledger: fresh_ledger(),
            chain_dep: fresh_chain_dep(placeholder),
        };
        let mk_source = || ListBlockSource {
            blocks: vec![(decoded.header_input.slot, bytes.clone())],
        };
        let mk_target = || TargetPoint {
            slot: decoded.header_input.slot,
            hash: decoded.block_hash.clone(),
        };

        // (b) WITHOUT the overlay (`None`): rollback replay still fails the header VRF against the
        //     placeholder — the bug. The overlay (not a VRF skip) is what fixes it.
        let err =
            materialize_rolled_back_state(mk_target(), &mk_reader(), &mk_source(), &schedule(), &view, None)
                .expect_err("without the eta0 overlay, rollback replay fails VRF against the placeholder");
        assert!(
            format!("{err:?}").contains("VrfCert"),
            "None-overlay path fails on the header VRF, got {err:?}"
        );

        // (c) WITH the overlay (`Some(eta0)`): the SAME block replays Valid AND the materialized
        //     chain_dep carries eta0 — byte-equal to the live-admit nonce basis (replay-equivalence).
        let (_l, got_cd) = materialize_rolled_back_state(
            mk_target(),
            &mk_reader(),
            &mk_source(),
            &schedule(),
            &view,
            Some(&eta0),
        )
        .expect("with the recovered eta0 overlaid, the rolled-back block replays Valid");
        assert_eq!(
            got_cd.epoch_nonce, eta0,
            "materialized chain_dep epoch_nonce == recovered eta0"
        );
        assert_eq!(
            got_cd.epoch_nonce,
            fresh_chain_dep(eta0_bytes).epoch_nonce,
            "rollback-materialized nonce == live-admit nonce basis (replay-equivalence)"
        );
    }

    /// CE-AN-4 (PHASE4-N-AN, T-REC-06): the eta0 overlay is NOT a VRF bypass. Overlaying a WRONG eta0
    /// (not the block's epoch nonce) still fails the header VRF — the fix supplies the correct recovered
    /// nonce; it does not skip verification. Guards the cluster's "no looser than live admit" line.
    #[test]
    fn rollback_materialize_does_not_bypass_vrf_on_wrong_eta0() {
        let (c, view) = corpus_view();
        let bytes = pick_lightest(&c);
        let decoded = decode_block(&bytes).expect("decode");
        let snapshot_slot = SlotNo(decoded.header_input.slot.0 - 1);
        let reader = OneSnapshotReader {
            slot: snapshot_slot,
            ledger: fresh_ledger(),
            chain_dep: fresh_chain_dep([0u8; 32]),
        };
        let source = ListBlockSource {
            blocks: vec![(decoded.header_input.slot, bytes.clone())],
        };
        let target = TargetPoint {
            slot: decoded.header_input.slot,
            hash: decoded.block_hash.clone(),
        };
        // A WRONG eta0 (all 0xAB) — not the block's epoch nonce.
        let wrong = Nonce(Hash32([0xAB; 32]));
        let err = materialize_rolled_back_state(
            target,
            &reader,
            &source,
            &schedule(),
            &view,
            Some(&wrong),
        )
        .expect_err("overlaying a WRONG eta0 must still fail VRF — the overlay is not a bypass");
        assert!(
            format!("{err:?}").contains("VrfCert"),
            "wrong-eta0 overlay still fails the header VRF, got {err:?}"
        );
    }

    #[test]
    fn materialize_fails_closed_on_invalid_block() {
        let (c, view) = corpus_view();
        let bytes = pick_lightest(&c);
        let decoded = decode_block(&bytes).expect("decode");
        // Flip a single byte in the body to invalidate.
        let mut bad = bytes.clone();
        let env = decode_block_envelope(&bad).expect("env");
        // Flip near the end of the inner block (likely body bytes).
        let i = env.block_end - 1;
        bad[i] ^= 0x01;
        let reader = OneSnapshotReader {
            slot: SlotNo(decoded.header_input.slot.0 - 1),
            ledger: fresh_ledger(),
            chain_dep: fresh_chain_dep(c.epoch_nonce),
        };
        let source = ListBlockSource {
            blocks: vec![(decoded.header_input.slot, bad)],
        };
        let target = TargetPoint {
            slot: decoded.header_input.slot,
            hash: decoded.block_hash.clone(),
        };
        let err =
            materialize_rolled_back_state(target, &reader, &source, &schedule(), &view, None)
                .expect_err("must reject invalid block");
        match err {
            MaterializeError::ReplayFailedAt { slot, .. } => {
                assert_eq!(slot, decoded.header_input.slot);
            }
            other => panic!("expected ReplayFailedAt, got {other:?}"),
        }
    }

    #[test]
    fn materialize_replay_forward_equals_direct_apply() {
        // The core DC-CONS-22 closure proof: snapshot+replay-forward
        // produces a state byte-equal to direct-apply.
        let (c, view) = corpus_view();
        let bytes = pick_lightest(&c);
        let decoded = decode_block(&bytes).expect("decode");

        // Path A: direct apply.
        let direct_state = {
            let l = fresh_ledger();
            let cd = fresh_chain_dep(c.epoch_nonce);
            let outcome = block_validity(&l, &cd, &schedule(), &view, &bytes);
            match outcome.verdict {
                BlockValidityVerdict::Valid { .. } => (outcome.ledger, outcome.chain_dep),
                BlockValidityVerdict::Invalid { .. } => panic!("direct apply must succeed"),
            }
        };

        // Path B: snapshot-then-replay-forward (snapshot at the
        // pre-block slot, replay-forward applies the block).
        let snapshot_slot = SlotNo(decoded.header_input.slot.0 - 1);
        let reader = OneSnapshotReader {
            slot: snapshot_slot,
            ledger: fresh_ledger(),
            chain_dep: fresh_chain_dep(c.epoch_nonce),
        };
        let source = ListBlockSource {
            blocks: vec![(decoded.header_input.slot, bytes.clone())],
        };
        let target = TargetPoint {
            slot: decoded.header_input.slot,
            hash: decoded.block_hash.clone(),
        };
        let (replay_ledger, replay_chain_dep) =
            materialize_rolled_back_state(target, &reader, &source, &schedule(), &view, None)
                .expect("replay ok");

        assert_eq!(
            ledger_fp(&direct_state.0),
            ledger_fp(&replay_ledger),
            "fingerprint of replay-forward state must equal direct-apply state"
        );
        assert_eq!(
            direct_state.1, replay_chain_dep,
            "chain_dep must equal direct-apply chain_dep"
        );
    }
}
