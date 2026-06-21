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
use ade_ledger::wal::event::{activation_replay_outcome, ActivationReplayOutcome};
use ade_ledger::wal::WalEntry;
use ade_types::EpochNo;
use std::collections::BTreeMap;

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

// ---- S3f-4c (DC-EPOCH-06): durable-before-visible + replay-identical crash recovery ----

/// Build the durable WAL activation record (S3f-4a) for a candidate view (S3f-4b). The
/// record's identity fields are taken verbatim from the view, so a recovery that re-derives
/// the SAME view reproduces this record exactly. The live emit (S3f-4d) writes this BEFORE
/// publishing the active view.
pub fn activation_record_for(view: &EpochConsensusView) -> WalEntry {
    WalEntry::EpochConsensusViewActivated {
        target_epoch: view.epoch,
        network_magic: view.network_magic,
        era: view.era,
        transition_point: view.source_point.clone(),
        source_checkpoint_commitment: view.checkpoint_commitment.clone(),
        snapshot_phase: view.snapshot_phase,
        nonce_commitment: view.nonce.clone(),
        stake_view_canonical_hash: view.stake_view_canonical_hash(),
        view_canonical_hash: view.canonical_hash(),
    }
}

/// Whether a re-derived candidate reproduces a durable activation record's ENTIRE identity
/// (every binding + the stake-view hash + the full-view hash + its own hash verifies). The
/// recovery match: only a byte-for-byte-identical re-derivation may republish.
fn activation_record_matches(record: &WalEntry, candidate: &EpochConsensusView) -> bool {
    match record {
        WalEntry::EpochConsensusViewActivated {
            target_epoch,
            network_magic,
            era,
            transition_point,
            source_checkpoint_commitment,
            snapshot_phase,
            nonce_commitment,
            stake_view_canonical_hash,
            view_canonical_hash,
        } => {
            candidate.verify_canonical_hash()
                && candidate.epoch == *target_epoch
                && candidate.network_magic == *network_magic
                && candidate.era == *era
                && candidate.source_point == *transition_point
                && candidate.checkpoint_commitment == *source_checkpoint_commitment
                && candidate.snapshot_phase == *snapshot_phase
                && candidate.nonce == *nonce_commitment
                && candidate.stake_view_canonical_hash() == *stake_view_canonical_hash
                && candidate.canonical_hash() == *view_canonical_hash
        }
        _ => false,
    }
}

/// Durable-before-visible (DC-EPOCH-06): publish the active view ONLY after the activation
/// WAL record is durable. A failed write is a TERMINAL `EpochViewActivationFailed` (halt
/// before promotion), NEVER a publish on a non-durable record.
pub fn activate_durable_before_visible(
    candidate: EpochConsensusView,
    wal_write_durable: bool,
) -> Result<ActiveEpochView, EpochViewActivationError> {
    if !wal_write_durable {
        return Err(EpochViewActivationError::EpochViewActivationFailed);
    }
    Ok(ActiveEpochView::Promoted(candidate))
}

/// Reconstruct the active view on recovery (DC-EPOCH-06, replay-identical). `record` is the
/// resolved durable activation record (or `None`); `candidate` is the re-derived view for
/// the recorded transition (or `None` if it could not be re-derived).
/// - no record => `Seed` (crash before the durable WAL: the old epoch stays active);
/// - record + a candidate that reproduces its identity => `Promoted(candidate)` (republish
///   the SAME view -- crash after the WAL, or after publication: recovered == WAL);
/// - record + a mismatched or absent candidate => TERMINAL `EpochViewPostPromotionMismatch`
///   (NEVER a fallback to the epoch-wrong seed view).
pub fn recover_active_view(
    record: Option<&WalEntry>,
    candidate: Option<&EpochConsensusView>,
) -> Result<ActiveEpochView, EpochViewActivationError> {
    match (record, candidate) {
        (None, _) => Ok(ActiveEpochView::Seed),
        (Some(rec), Some(cand)) if activation_record_matches(rec, cand) => {
            Ok(ActiveEpochView::Promoted(cand.clone()))
        }
        (Some(_), _) => Err(EpochViewActivationError::EpochViewPostPromotionMismatch),
    }
}

/// Fold the WAL's activation records into the single resolved one (DC-EPOCH-04 applied on
/// replay): repeated records for the same target epoch are idempotent iff byte-identical,
/// else a TERMINAL `EpochViewActivationConflict`. Returns the (last) resolved activation
/// record, or `None` if the WAL has none. Records for DIFFERENT epochs supersede in order
/// (the latest activation wins; a real chain activates successive epochs).
pub fn resolve_activation_record(
    entries: &[WalEntry],
) -> Result<Option<WalEntry>, EpochViewActivationError> {
    // Fold by target epoch (NOT by position), so a same-epoch conflict is surfaced
    // regardless of interleaving with other-epoch records -- the fail-closed property does
    // not rely on the caller's ordering. Deterministic (BTreeMap, keyed by EpochNo).
    let mut by_epoch: BTreeMap<EpochNo, &WalEntry> = BTreeMap::new();
    for entry in entries {
        let target_epoch = match entry {
            WalEntry::EpochConsensusViewActivated { target_epoch, .. } => *target_epoch,
            _ => continue,
        };
        match by_epoch.get(&target_epoch) {
            // same target epoch already seen: idempotent iff byte-identical, else conflict
            // (DC-EPOCH-04). `None` from activation_replay_outcome is unreachable here (the
            // map key IS the epoch) -- treated as keep, never a silent supersede.
            Some(existing) => match activation_replay_outcome(existing, entry) {
                Some(ActivationReplayOutcome::Conflict) => {
                    return Err(EpochViewActivationError::EpochViewActivationConflict)
                }
                Some(ActivationReplayOutcome::Idempotent) | None => {}
            },
            None => {
                by_epoch.insert(target_epoch, entry);
            }
        }
    }
    // The active view is the LATEST (max target epoch) activation; earlier epochs are
    // superseded in deterministic epoch order.
    Ok(by_epoch.into_iter().next_back().map(|(_, v)| v.clone()))
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
            protocol_params_commitment: Hash32([0xdd; 32]),
        }
    }
    fn view(stake: u64) -> EpochConsensusView {
        let b = bindings();
        let mut s = BTreeMap::new();
        s.insert(PoolId(Hash28([0x11; 28])), Coin(stake));
        EpochConsensusView::bind(
            b.network_magic, b.era, b.epoch, b.source_point, b.checkpoint_commitment, b.nonce,
            b.snapshot_phase, s,
            [(PoolId(Hash28([0x11; 28])), Hash32([0x71; 32]))].into_iter().collect(),
            Coin(stake),
            b.protocol_params_commitment.clone(),
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

    // ---- S3f-4c (DC-EPOCH-06): durable-before-visible + crash recovery ----

    fn record_with(epoch: u64, view_hash: u8) -> WalEntry {
        WalEntry::EpochConsensusViewActivated {
            target_epoch: EpochNo(epoch),
            network_magic: NET,
            era: CardanoEra::Conway,
            transition_point: point(),
            source_checkpoint_commitment: Hash32([0xbb; 32]),
            snapshot_phase: SnapshotPhase::Set,
            nonce_commitment: Hash32([0xcc; 32]),
            stake_view_canonical_hash: Hash32([0xd4; 32]),
            view_canonical_hash: Hash32([view_hash; 32]),
        }
    }

    // crash BEFORE the durable WAL: no activation record -> the old (seed) epoch stays active.
    #[test]
    fn crash_before_durable_wal_keeps_seed() {
        assert_eq!(recover_active_view(None, None), Ok(ActiveEpochView::Seed));
        assert_eq!(recover_active_view(None, Some(&view(1000))), Ok(ActiveEpochView::Seed));
    }

    // crash AFTER the durable WAL (or after publication): recovery re-derives the SAME view
    // and republishes it -- recovered == WAL.
    #[test]
    fn crash_after_wal_republishes_same_view() {
        let v = view(1000);
        let rec = activation_record_for(&v);
        assert_eq!(
            recover_active_view(Some(&rec), Some(&v)),
            Ok(ActiveEpochView::Promoted(v.clone()))
        );
    }

    // a re-derived view that does NOT reproduce the record's identity (or none) is a TERMINAL
    // mismatch -- never a fallback to the (epoch-wrong) seed view.
    #[test]
    fn recovered_view_mismatch_is_terminal() {
        let rec = activation_record_for(&view(1000));
        assert_eq!(
            recover_active_view(Some(&rec), Some(&view(2000))),
            Err(EpochViewActivationError::EpochViewPostPromotionMismatch)
        );
        assert_eq!(
            recover_active_view(Some(&rec), None),
            Err(EpochViewActivationError::EpochViewPostPromotionMismatch)
        );
    }

    // durable-before-visible: a non-durable WAL write halts (terminal) -- never a publish.
    #[test]
    fn durable_before_visible_halts_on_wal_failure() {
        let v = view(1000);
        assert_eq!(
            activate_durable_before_visible(v.clone(), false),
            Err(EpochViewActivationError::EpochViewActivationFailed)
        );
        assert_eq!(
            activate_durable_before_visible(v.clone(), true),
            Ok(ActiveEpochView::Promoted(v))
        );
    }

    // the replay application (DC-EPOCH-04 on recovery): idempotent / conflict / supersede.
    #[test]
    fn resolve_activation_idempotent_conflict_supersede() {
        assert_eq!(resolve_activation_record(&[]), Ok(None));
        assert_eq!(
            resolve_activation_record(&[record_with(577, 0xe5)]),
            Ok(Some(record_with(577, 0xe5)))
        );
        // same epoch + identical -> idempotent.
        assert_eq!(
            resolve_activation_record(&[record_with(577, 0xe5), record_with(577, 0xe5)]),
            Ok(Some(record_with(577, 0xe5)))
        );
        // same epoch + differing -> terminal conflict.
        assert_eq!(
            resolve_activation_record(&[record_with(577, 0xe5), record_with(577, 0xff)]),
            Err(EpochViewActivationError::EpochViewActivationConflict)
        );
        // different epochs -> the later activation supersedes.
        assert_eq!(
            resolve_activation_record(&[record_with(577, 0xe5), record_with(578, 0xaa)]),
            Ok(Some(record_with(578, 0xaa)))
        );
        // INTERLEAVED same-epoch divergence (a different-epoch record between them) still
        // surfaces the conflict -- the fold keys by epoch, not by position.
        assert_eq!(
            resolve_activation_record(&[
                record_with(577, 0xe5),
                record_with(578, 0xaa),
                record_with(577, 0xff),
            ]),
            Err(EpochViewActivationError::EpochViewActivationConflict)
        );
        // non-activation entries are skipped (only activation records fold).
        assert_eq!(
            resolve_activation_record(&[sample_admit(), record_with(577, 0xe5)]),
            Ok(Some(record_with(577, 0xe5)))
        );
    }

    fn sample_admit() -> WalEntry {
        WalEntry::AdmitBlock {
            prior_fp: Hash32([0x01; 32]),
            block_hash: Hash32([0x02; 32]),
            slot: SlotNo(1),
            verdict: ade_ledger::wal::BlockVerdictTag::Valid,
            post_fp: Hash32([0x03; 32]),
        }
    }
}
