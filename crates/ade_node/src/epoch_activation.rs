//! EPOCH-CONSENSUS-VIEW S3f-4b (DC-EPOCH-05 / DC-EPOCH-07) — the activation predicate, the
//! atomically-published active view, and the terminal activation states.
//!
//! The activation is ONE atomic, WAL-backed path — NOT a feature-flagged alternate
//! consensus mode. The safe gate is NOT a flag; it is the [`activation_predicate`]: all
//! required bindings verify AND the activation WAL record is durable AND the selected-chain
//! point is correct AND the epoch transition is eligible ⇒ promote; otherwise no promotion.
//!
//! Before promotion the recovered SEED view is authoritative; after, the PROMOTED N+1 view
//! is — with NO "choose old or new by config" state ([`ActiveEpochView`], a one-way
//! Seed→Promoted transition). DC-EPOCH-05: epoch N+1 leadership/validation read ONLY the
//! promoted view; the seed inputs are not observable post-promotion.
//!
//! DC-EPOCH-07: a missing / stale / conflicting / mismatched candidate causes a TERMINAL
//! fail-closed state ([`EpochViewActivationError`]), NEVER a fallback to the (epoch-wrong)
//! seed view.

use ade_core::consensus::events::Point;
use ade_ledger::reduced_epoch_view::{EpochConsensusView, ViewBindings};

/// Whether a candidate view may be PUBLISHED as the active view. The activation predicate
/// is the gate (not a flag): it returns `Promote` only when EVERY precondition holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationOutcome {
    Promote,
    NoPromotion(ActivationReject),
}

/// Why a candidate view is not promoted. Each is "no promotion" (the seed stays
/// authoritative) — distinct from the TERMINAL [`EpochViewActivationError`] states, which
/// halt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationReject {
    /// The slot is not at the deterministic epoch transition.
    TransitionIneligible,
    /// The candidate does not match the N+1 context (bindings + `verify_canonical_hash`).
    BindingsUnverified,
    /// The candidate's transition point is not the selected-chain point.
    WrongSelectedPoint,
    /// The activation WAL record is not yet durable.
    WalNotDurable,
}

/// The activation predicate (DC-EPOCH-06 ordering). `Promote` requires, in order: the
/// transition is eligible, the candidate matches the N+1 bindings (incl.
/// `verify_canonical_hash`), the candidate's transition point IS the selected-chain point,
/// and the activation WAL record is durable. Any failure ⇒ `NoPromotion` (no flag, no
/// fallback — the seed view simply stays authoritative until a later eligible transition).
pub fn activation_predicate(
    candidate: &EpochConsensusView,
    n1_bindings: &ViewBindings,
    selected_point: &Point,
    transition_eligible: bool,
    wal_durable: bool,
) -> ActivationOutcome {
    if !transition_eligible {
        return ActivationOutcome::NoPromotion(ActivationReject::TransitionIneligible);
    }
    if !candidate.matches(n1_bindings) {
        return ActivationOutcome::NoPromotion(ActivationReject::BindingsUnverified);
    }
    if candidate.source_point != *selected_point {
        return ActivationOutcome::NoPromotion(ActivationReject::WrongSelectedPoint);
    }
    if !wal_durable {
        return ActivationOutcome::NoPromotion(ActivationReject::WalNotDurable);
    }
    ActivationOutcome::Promote
}

/// The TERMINAL, fail-closed activation states (DC-EPOCH-07). NEVER a fallback to the seed
/// view — a structured terminal halt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpochViewActivationError {
    /// The activation WAL record could not be made durable ⇒ halt before promotion.
    EpochViewActivationFailed,
    /// A conflicting activation already happened for the target epoch ⇒ halt.
    EpochViewActivationConflict,
    /// After publication, the active view does not match the WAL record ⇒ halt.
    EpochViewPostPromotionMismatch,
}

/// The atomically-published active epoch view (DC-EPOCH-05). A ONE-WAY Seed→Promoted
/// transition: before promotion the recovered seed view is authoritative; after, ONLY the
/// promoted view is. There is no config that selects between them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ActiveEpochView {
    /// The recovered seed view is authoritative (no promotion has occurred).
    #[default]
    Seed,
    /// The bound N+1 view is authoritative.
    Promoted(EpochConsensusView),
}

impl ActiveEpochView {
    pub fn new() -> Self {
        ActiveEpochView::Seed
    }

    /// Atomically promote to the bound view. ONE-WAY: from `Seed`, publish the view. From
    /// `Promoted`, the SAME view is idempotent (replay), a DIFFERENT view is a terminal
    /// `EpochViewActivationConflict` (DC-EPOCH-04/07 — never a silent re-publish).
    pub fn promote(&mut self, view: EpochConsensusView) -> Result<(), EpochViewActivationError> {
        match self {
            ActiveEpochView::Seed => {
                *self = ActiveEpochView::Promoted(view);
                Ok(())
            }
            ActiveEpochView::Promoted(existing) if *existing == view => Ok(()),
            ActiveEpochView::Promoted(_) => {
                Err(EpochViewActivationError::EpochViewActivationConflict)
            }
        }
    }

    /// The promoted view, if any. `None` while the seed is still authoritative — DC-EPOCH-05:
    /// N+1 leadership reads this; it is `Some` ONLY after a promotion, so N+1 can never read
    /// the seed inputs as if they were the N+1 view.
    pub fn promoted(&self) -> Option<&EpochConsensusView> {
        match self {
            ActiveEpochView::Promoted(v) => Some(v),
            ActiveEpochView::Seed => None,
        }
    }

    /// Whether a promotion has occurred.
    pub fn is_promoted(&self) -> bool {
        matches!(self, ActiveEpochView::Promoted(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ade_ledger::reduced_snapshot::SnapshotPhase;
    use ade_types::primitives::SlotNo;
    use ade_types::tx::{Coin, PoolId};
    use ade_types::{CardanoEra, EpochNo, Hash28, Hash32};
    use std::collections::BTreeMap;

    const NET: u32 = 2;
    fn point() -> Point {
        Point { slot: SlotNo(115_000_000), hash: Hash32([0xaa; 32]) }
    }
    fn bindings() -> ViewBindings {
        ViewBindings {
            network_magic: NET,
            era: CardanoEra::Conway,
            epoch: EpochNo(578),
            source_point: point(),
            checkpoint_commitment: Hash32([0xbb; 32]),
            nonce: Hash32([0xcc; 32]),
            snapshot_phase: SnapshotPhase::Set,
        }
    }
    fn view(stake: u64) -> EpochConsensusView {
        let b = bindings();
        let mut s = BTreeMap::new();
        s.insert(PoolId(Hash28([0x11; 28])), Coin(stake));
        EpochConsensusView::bind(
            b.network_magic, b.era, b.epoch, b.source_point, b.checkpoint_commitment, b.nonce,
            b.snapshot_phase, s, Coin(stake),
        )
    }

    #[test]
    fn predicate_promotes_only_when_every_precondition_holds() {
        let v = view(1000);
        assert_eq!(
            activation_predicate(&v, &bindings(), &point(), true, true),
            ActivationOutcome::Promote
        );
    }

    #[test]
    fn predicate_rejects_each_failed_precondition() {
        let v = view(1000);
        assert_eq!(
            activation_predicate(&v, &bindings(), &point(), false, true),
            ActivationOutcome::NoPromotion(ActivationReject::TransitionIneligible)
        );
        let mut wrong = bindings();
        wrong.nonce = Hash32([0xff; 32]);
        assert_eq!(
            activation_predicate(&v, &wrong, &point(), true, true),
            ActivationOutcome::NoPromotion(ActivationReject::BindingsUnverified)
        );
        let other_point = Point { slot: SlotNo(999), hash: Hash32([0x00; 32]) };
        assert_eq!(
            activation_predicate(&v, &bindings(), &other_point, true, true),
            ActivationOutcome::NoPromotion(ActivationReject::WrongSelectedPoint)
        );
        assert_eq!(
            activation_predicate(&v, &bindings(), &point(), true, false),
            ActivationOutcome::NoPromotion(ActivationReject::WalNotDurable)
        );
    }

    #[test]
    fn active_view_one_way_promote_and_idempotence() {
        let mut active = ActiveEpochView::new();
        assert_eq!(active, ActiveEpochView::Seed);
        assert!(active.promoted().is_none());
        // promote.
        active.promote(view(1000)).expect("first promotion");
        assert!(active.is_promoted());
        assert_eq!(active.promoted(), Some(&view(1000)));
        // idempotent: the SAME view re-promotes Ok.
        active.promote(view(1000)).expect("idempotent re-promotion");
        assert_eq!(active.promoted(), Some(&view(1000)));
    }

    #[test]
    fn active_view_conflicting_promotion_is_terminal() {
        let mut active = ActiveEpochView::new();
        active.promote(view(1000)).expect("first");
        // a DIFFERENT view for the same active slot -> terminal conflict, never a silent swap.
        assert_eq!(
            active.promote(view(2000)),
            Err(EpochViewActivationError::EpochViewActivationConflict)
        );
        // the original promoted view is unchanged (no partial mutation).
        assert_eq!(active.promoted(), Some(&view(1000)));
    }

    // DC-EPOCH-05: before promotion the active view exposes NO N+1 view (the seed is
    // authoritative elsewhere); after, it exposes ONLY the promoted view -- N+1 leadership
    // can never read the seed inputs as the N+1 view.
    #[test]
    fn seed_exposes_no_n1_view_until_promotion() {
        let active = ActiveEpochView::Seed;
        assert!(active.promoted().is_none(), "the seed state exposes no N+1 view");
    }
}
