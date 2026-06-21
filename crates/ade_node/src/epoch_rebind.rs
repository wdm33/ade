//! EPOCH-CONSENSUS-VIEW S3f-3 (DC-EVIEW-11, strengthening DC-EPOCH-03) — the
//! deterministic, fail-closed epoch-rebind seam.
//!
//! DC-EPOCH-03 fails the forge closed past the seed-epoch boundary (the recovered eta0 is
//! the seed-epoch nonce, stale past the boundary). This seam adds the ONLY sanctioned way
//! across the boundary: the current seed-epoch view stays authoritative until, AT the
//! deterministic epoch transition (a candidate slot in the IMMEDIATE next epoch), a
//! fully-bound, matching N+1 [`EpochConsensusView`] atomically promotes. Anything else --
//! a missing, stale, conflicting, or wrongly-bound view; a non-immediate epoch; an
//! unlocatable slot -- fails closed.
//!
//! [`decide_epoch_rebind`] is a PURE reducer (deterministic; replay-equivalent; no I/O,
//! clock, rand, float). The live relay loop calls it; today it passes `None` for the
//! bound view (S3f-4 supplies one), so OffEpoch fails closed EXACTLY as the pre-seam wall
//! -- no leader-election behaviour change. It never silently activates anything early,
//! invents a fallback, or changes a leader decision: an unprovable view is INERT.

use crate::node_sync::ForgeEpochAdmission;
use ade_ledger::reduced_epoch_view::{EpochConsensusView, ViewBindings};

/// The rebind decision for a candidate slot. The current view stays authoritative unless
/// `Promote` fires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpochRebindDecision {
    /// The current seed-epoch view stays authoritative (the slot is within the seed epoch).
    KeepCurrent,
    /// Atomic promotion to the bound, matching N+1 view (the slot is in the immediate
    /// next epoch and a fully-bound matching view was supplied).
    Promote(EpochConsensusView),
    /// Fail closed: no leadership / no admit. The current view stays authoritative.
    FailClosed(EpochRebindReject),
}

/// Why a candidate slot past the seed epoch fails closed. Every variant is fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochRebindReject {
    /// The slot does not locate to any era (`OffEpoch{None}`).
    Unlocatable,
    /// The slot is more than one epoch past the seed epoch -- no multi-boundary leap.
    NotImmediateNext,
    /// The slot is in the immediate next epoch but NO bound N+1 view was supplied.
    NoBoundView,
    /// A bound view was supplied but does not match the N+1 context (wrong network / era /
    /// epoch / point / commitment / nonce / phase, or a tampered view whose canonical hash
    /// no longer verifies).
    ViewMismatch,
}

/// Decide the epoch-rebind for a candidate slot. PURE / deterministic.
///
/// `admission` is the DC-EPOCH-03 classification of the slot against the recovered seed
/// epoch. `bound_n1` is the candidate N+1 view PLUS the deterministic context to match it
/// against -- `None` until S3f-4 supplies one (so OffEpoch fails closed exactly as the
/// pre-seam wall). A `Promote` fires ONLY when the slot is in the immediate next epoch
/// (`seed_epoch + 1`), the supplied bindings ARE that epoch's context, and the view
/// matches them (all bindings + `verify_canonical_hash`).
pub fn decide_epoch_rebind(
    admission: ForgeEpochAdmission,
    bound_n1: Option<(&EpochConsensusView, &ViewBindings)>,
) -> EpochRebindDecision {
    match admission {
        ForgeEpochAdmission::WithinSeedEpoch => EpochRebindDecision::KeepCurrent,
        ForgeEpochAdmission::OffEpoch { candidate_epoch: None, .. } => {
            EpochRebindDecision::FailClosed(EpochRebindReject::Unlocatable)
        }
        ForgeEpochAdmission::OffEpoch { candidate_epoch: Some(e), seed_epoch } => {
            // Only the immediate next epoch is eligible -- never leap multiple boundaries.
            if e.0 != seed_epoch.0.wrapping_add(1) {
                return EpochRebindDecision::FailClosed(EpochRebindReject::NotImmediateNext);
            }
            match bound_n1 {
                None => EpochRebindDecision::FailClosed(EpochRebindReject::NoBoundView),
                Some((view, bindings)) => {
                    // The bindings MUST be the immediate-next epoch's context, and the
                    // view must match them (incl. verify_canonical_hash). Either failing
                    // is fail-closed -- never a promotion to a wrong/tampered view.
                    if bindings.epoch == e && view.matches(bindings) {
                        EpochRebindDecision::Promote(view.clone())
                    } else {
                        EpochRebindDecision::FailClosed(EpochRebindReject::ViewMismatch)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ade_ledger::reduced_snapshot::SnapshotPhase;
    use ade_core::consensus::events::Point;
    use ade_types::primitives::SlotNo;
    use ade_types::tx::{Coin, PoolId};
    use ade_types::{CardanoEra, EpochNo, Hash28, Hash32};
    use std::collections::BTreeMap;

    const NET: u32 = 2;
    const SEED: u64 = 100;

    fn bindings(epoch: u64) -> ViewBindings {
        ViewBindings {
            network_magic: NET,
            era: CardanoEra::Conway,
            epoch: EpochNo(epoch),
            source_point: Point { slot: SlotNo(115_000_000), hash: Hash32([0xaa; 32]) },
            checkpoint_commitment: Hash32([0xbb; 32]),
            nonce: Hash32([0xcc; 32]),
            snapshot_phase: SnapshotPhase::Set,
            protocol_params_commitment: Hash32([0xdd; 32]),
        }
    }

    /// A view correctly bound to the bindings of `epoch`.
    fn view_for(epoch: u64) -> EpochConsensusView {
        let b = bindings(epoch);
        let mut stake = BTreeMap::new();
        stake.insert(PoolId(Hash28([0x11; 28])), Coin(1000));
        EpochConsensusView::bind(
            b.network_magic,
            b.era,
            b.epoch,
            b.source_point,
            b.checkpoint_commitment,
            b.nonce,
            b.snapshot_phase,
            stake,
            [(PoolId(Hash28([0x11; 28])), Hash32([0x71; 32]))].into_iter().collect(),
            Coin(1000),
            b.protocol_params_commitment.clone(),
        )
    }

    fn off_epoch(candidate: Option<u64>) -> ForgeEpochAdmission {
        ForgeEpochAdmission::OffEpoch {
            candidate_epoch: candidate.map(EpochNo),
            seed_epoch: EpochNo(SEED),
        }
    }

    // (proof) hermetic epoch-N -> N+1 simulated transition: an immediate-next slot with a
    // fully-bound matching view atomically promotes.
    #[test]
    fn simulated_transition_promotes_bound_n1_view() {
        let view = view_for(SEED + 1);
        let b = bindings(SEED + 1);
        assert_eq!(
            decide_epoch_rebind(off_epoch(Some(SEED + 1)), Some((&view, &b))),
            EpochRebindDecision::Promote(view.clone())
        );
    }

    // (proof) current same-epoch behaviour byte-identical: WithinSeedEpoch keeps the
    // current view regardless of any supplied bound view.
    #[test]
    fn same_epoch_keeps_current() {
        assert_eq!(decide_epoch_rebind(ForgeEpochAdmission::WithinSeedEpoch, None), EpochRebindDecision::KeepCurrent);
        let view = view_for(SEED + 1);
        let b = bindings(SEED + 1);
        assert_eq!(
            decide_epoch_rebind(ForgeEpochAdmission::WithinSeedEpoch, Some((&view, &b))),
            EpochRebindDecision::KeepCurrent
        );
    }

    // (proof) the live default (no bound view) fails OffEpoch closed -- byte-identical to
    // the pre-seam DC-EPOCH-03 wall.
    #[test]
    fn off_epoch_without_bound_view_fails_closed() {
        assert_eq!(
            decide_epoch_rebind(off_epoch(Some(SEED + 1)), None),
            EpochRebindDecision::FailClosed(EpochRebindReject::NoBoundView)
        );
    }

    #[test]
    fn not_immediate_next_fails_closed() {
        let view = view_for(SEED + 2);
        let b = bindings(SEED + 2);
        // seed + 2 is more than one boundary away.
        assert_eq!(
            decide_epoch_rebind(off_epoch(Some(SEED + 2)), Some((&view, &b))),
            EpochRebindDecision::FailClosed(EpochRebindReject::NotImmediateNext)
        );
    }

    #[test]
    fn unlocatable_fails_closed() {
        assert_eq!(
            decide_epoch_rebind(off_epoch(None), None),
            EpochRebindDecision::FailClosed(EpochRebindReject::Unlocatable)
        );
    }

    // (proof) reject wrong-network / era / point / nonce / phase: a view bound to the
    // correct context, checked against a context differing in ONE binding, fails closed.
    #[test]
    fn rejects_each_wrong_binding() {
        let view = view_for(SEED + 1);
        let mut wrong;

        wrong = bindings(SEED + 1); wrong.network_magic = 999;
        assert_eq!(decide_epoch_rebind(off_epoch(Some(SEED + 1)), Some((&view, &wrong))), EpochRebindDecision::FailClosed(EpochRebindReject::ViewMismatch), "wrong network");
        wrong = bindings(SEED + 1); wrong.era = CardanoEra::Babbage;
        assert_eq!(decide_epoch_rebind(off_epoch(Some(SEED + 1)), Some((&view, &wrong))), EpochRebindDecision::FailClosed(EpochRebindReject::ViewMismatch), "wrong era");
        wrong = bindings(SEED + 1); wrong.source_point.hash = Hash32([0xff; 32]);
        assert_eq!(decide_epoch_rebind(off_epoch(Some(SEED + 1)), Some((&view, &wrong))), EpochRebindDecision::FailClosed(EpochRebindReject::ViewMismatch), "wrong point");
        wrong = bindings(SEED + 1); wrong.nonce = Hash32([0xff; 32]);
        assert_eq!(decide_epoch_rebind(off_epoch(Some(SEED + 1)), Some((&view, &wrong))), EpochRebindDecision::FailClosed(EpochRebindReject::ViewMismatch), "wrong nonce");
        wrong = bindings(SEED + 1); wrong.checkpoint_commitment = Hash32([0xff; 32]);
        assert_eq!(decide_epoch_rebind(off_epoch(Some(SEED + 1)), Some((&view, &wrong))), EpochRebindDecision::FailClosed(EpochRebindReject::ViewMismatch), "wrong commitment");
        wrong = bindings(SEED + 1); wrong.snapshot_phase = SnapshotPhase::Go;
        assert_eq!(decide_epoch_rebind(off_epoch(Some(SEED + 1)), Some((&view, &wrong))), EpochRebindDecision::FailClosed(EpochRebindReject::ViewMismatch), "wrong phase");
    }

    // (proof) reject wrong-hash: a view whose pub field was mutated after binding (so its
    // canonical hash no longer verifies) fails closed even against its own bindings.
    #[test]
    fn rejects_tampered_view_wrong_hash() {
        let mut view = view_for(SEED + 1);
        let b = bindings(SEED + 1);
        // tamper a field WITHOUT rebinding -> verify_canonical_hash() is now false.
        view.nonce = Hash32([0x00; 32]);
        assert!(!view.verify_canonical_hash(), "tampering must invalidate the hash");
        assert_eq!(
            decide_epoch_rebind(off_epoch(Some(SEED + 1)), Some((&view, &b))),
            EpochRebindDecision::FailClosed(EpochRebindReject::ViewMismatch)
        );
    }

    // (proof) replay equivalence: the reducer is deterministic -- identical inputs yield
    // identical decisions across all three outcomes.
    #[test]
    fn replay_equivalent_deterministic() {
        let view = view_for(SEED + 1);
        let b = bindings(SEED + 1);
        for (adm, bound) in [
            (ForgeEpochAdmission::WithinSeedEpoch, None),
            (off_epoch(Some(SEED + 1)), None),
            (off_epoch(Some(SEED + 1)), Some((&view, &b))),
        ] {
            assert_eq!(decide_epoch_rebind(adm, bound), decide_epoch_rebind(adm, bound));
        }
    }

    // (proof) crash/restart on both sides: re-deriving the decision from the same durable
    // inputs (the seam holds no hidden state) yields the same outcome -- BEFORE the bound
    // view is available (None -> stays on the seed epoch) and AFTER (Some -> promotes).
    #[test]
    fn crash_restart_redrives_same_decision_both_sides() {
        let view = view_for(SEED + 1);
        let b = bindings(SEED + 1);
        // before the transition view exists: a restart re-derives KeepCurrent / FailClosed.
        let before = || decide_epoch_rebind(off_epoch(Some(SEED + 1)), None);
        assert_eq!(before(), EpochRebindDecision::FailClosed(EpochRebindReject::NoBoundView));
        assert_eq!(before(), before(), "pre-rebind decision stable across restart");
        // after the bound view is available: a restart re-derives the SAME promotion.
        let after = || decide_epoch_rebind(off_epoch(Some(SEED + 1)), Some((&view, &b)));
        assert_eq!(after(), EpochRebindDecision::Promote(view.clone()));
        assert_eq!(after(), after(), "post-rebind decision stable across restart");
    }
}
