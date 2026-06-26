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
use ade_core::consensus::ledger_view::LedgerView;
use ade_ledger::consensus_view::PoolDistrView;
use ade_ledger::reduced_epoch_view::{EpochConsensusView, ViewBindings};
use ade_ledger::wal::event::{activation_replay_outcome, ActivationReplayOutcome};
use ade_ledger::wal::WalEntry;
use ade_types::{EpochNo, Hash32};
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

/// EPOCH-CONTINUITY-ACTIVATION ECA-3 (DC-EPOCH-14, criterion #9): why an authoritative decision is
/// refused because the active authority's epoch does not equal the protocol epoch implied by the
/// block/slot context. Slot-aware + bidirectional; each is a structured TERMINAL halt — NEVER an
/// implicit fallback to the seed view. (The two variants partition every inequality: `!=` is always
/// either `<` or `>`.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityEpochError {
    /// `authority_epoch < protocol_epoch(slot)`: the boundary was crossed but the N+1 view was never
    /// promoted (a MISSING promotion). The seed view is epoch-wrong for this block — halt, never fall
    /// back to it.
    MissingPromotion {
        authority_epoch: EpochNo,
        protocol_epoch: EpochNo,
    },
    /// `authority_epoch > protocol_epoch(slot)`: a PREMATURE promotion — the active view is ahead of
    /// the block/slot's protocol epoch.
    PrematurePromotion {
        authority_epoch: EpochNo,
        protocol_epoch: EpochNo,
    },
}

/// EPOCH-CONTINUITY-ACTIVATION ECA-3 (DC-EPOCH-14): the producer's epoch-authority CONTRACT — a
/// CANONICAL mode established from durable bootstrap/sidecar/manifest state (NOT an ambient runtime
/// flag, which would be a forbidden semantic switch deciding consensus), recovered IDENTICALLY on
/// warm-start, part of the authority's identity. It decides whether a missing N+1 promotion at an
/// N+1 slot is a graceful no-forge (a seed-only limited producer past its supported epoch — keeps
/// following) or a TERMINAL authority failure (a continuity-capable producer whose boundary should
/// have activated).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpochAuthorityMode {
    /// The producer supports forging ONLY within its seed epoch — no continuity mechanism is bound.
    /// Past `supported_epoch` it does NOT forge (no N+1 view) but keeps following; this is NOT a
    /// fault, so it never halts the node.
    SeedOnly { supported_epoch: EpochNo },
    /// The producer is continuity-capable: an N+1 activation is EXPECTED at the boundary because the
    /// EVIEW package (the reduced checkpoint + the v4 consensus-profile sidecar) is bound in durable
    /// state. A missing promotion at an N+1 slot is a TERMINAL authority failure.
    ContinuityRequired {
        /// The bound source chain point the activation derives from (the bootstrap/seed point).
        source_binding: Point,
        /// The consensus-profile commitment (genesis ++ protocol-params ++ ASC) the activation must
        /// match — the DC-CINPUT-06 hashes recovered from the v4 sidecar.
        activation_profile_commitment: Hash32,
        /// The leadership snapshot / target-epoch policy (the DC-EPOCH-08 lag).
        target_epoch_policy: TargetEpochPolicy,
    },
}

/// The leadership target-epoch / snapshot policy bound into [`EpochAuthorityMode::ContinuityRequired`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetEpochPolicy {
    /// Leadership reads the SET snapshot at the documented lag (`LEADERSHIP_SNAPSHOT_LAG_EPOCHS`).
    SetSnapshotLag { lag_epochs: u32 },
}

/// DC-EPOCH-14 #9: the verdict of the per-call epoch guard. `Permitted` proceeds; `Terminal` halts
/// (a structured [`AuthorityEpochError`]); `SeedOnlyPastSupport` is a graceful no-forge for a
/// seed-only producer past its supported epoch — the follow loop stays ALIVE (never a halt).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityEpochVerdict {
    Permitted,
    Terminal(AuthorityEpochError),
    SeedOnlyPastSupport {
        supported_epoch: EpochNo,
        slot_epoch: EpochNo,
    },
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

/// EPOCH-CONTINUITY-ACTIVATION ECA-3 (DC-EPOCH-14): the ONE owned, epoch-versioned authority the
/// relay loop holds — the SOLE view source for BOTH header validation AND leadership/forge. It
/// carries the recovered seed `PoolDistrView` (epoch N) and, after an atomic boundary promotion,
/// the projected N+1 `PoolDistrView` (derived from the bound `EpochConsensusView` via DC-EPOCH-12's
/// `to_pool_distr_view`). Consumers resolve `ledger_view()` / `pool_distr_view()` FRESH at each
/// authoritative decision, so a borrowed view never outlives the swap. The promotion reuses the
/// `ActiveEpochView` activation-state logic (one-way Seed→Promoted; idempotent on the SAME source;
/// a conflicting source is terminal), with the projected view as a derived cache kept in lockstep
/// (`activation.is_promoted() == promoted_view.is_some()`, established by the constructors).
#[derive(Debug, Clone)]
pub struct ActiveEpochAuthority<'a> {
    /// The recovered seed (epoch-N) view — BORROWED for the loop's lifetime (the relay loop's seed
    /// `PoolDistrView`, the same value the pre-refactor `ledger_view` param carried); the authority
    /// before any promotion.
    seed_view: &'a PoolDistrView,
    /// The activation state: `Seed` until the boundary promotes the bound N+1 `EpochConsensusView`.
    /// The single source of truth for the one-way + conflict + idempotence semantics.
    activation: ActiveEpochView,
    /// The projected N+1 `PoolDistrView` (DC-EPOCH-12), `Some` IFF `activation.is_promoted()` —
    /// OWNED, installed atomically with the promotion. A derived cache of the promoted view.
    promoted_view: Option<PoolDistrView>,
    /// The CANONICAL authority mode (DC-EPOCH-14): `SeedOnly` (a limited producer) vs
    /// `ContinuityRequired` (continuity-capable). Established from durable state, part of the
    /// authority's identity; decides whether a missing promotion at an N+1 slot is terminal.
    mode: EpochAuthorityMode,
}

impl<'a> ActiveEpochAuthority<'a> {
    /// Construct a SEED-ONLY authority from the recovered seed `PoolDistrView` (epoch N): a limited
    /// producer that forges only within its seed epoch and gracefully no-forges (keeps following)
    /// past it. No promotion; the mode is `SeedOnly` keyed to the seed view's epoch.
    pub fn seed(seed_view: &'a PoolDistrView) -> Self {
        let supported_epoch = seed_view.epoch();
        Self {
            seed_view,
            activation: ActiveEpochView::Seed,
            promoted_view: None,
            mode: EpochAuthorityMode::SeedOnly { supported_epoch },
        }
    }

    /// Construct a CONTINUITY-REQUIRED authority (DC-EPOCH-14): a continuity-capable producer whose
    /// boundary activation is EXPECTED — a missing promotion at an N+1 slot is terminal. The mode's
    /// bindings (the source point + the consensus-profile commitment + the target-epoch policy) are
    /// established from durable state, recovered identically on warm-start.
    pub fn continuity(
        seed_view: &'a PoolDistrView,
        source_binding: Point,
        activation_profile_commitment: Hash32,
        target_epoch_policy: TargetEpochPolicy,
    ) -> Self {
        Self {
            seed_view,
            activation: ActiveEpochView::Seed,
            promoted_view: None,
            mode: EpochAuthorityMode::ContinuityRequired {
                source_binding,
                activation_profile_commitment,
                target_epoch_policy,
            },
        }
    }

    /// The current authoritative `LedgerView` (header VRF validation): the promoted N+1 view once
    /// promoted, else the seed view. Resolve FRESH at each decision — never retained across a swap.
    pub fn ledger_view(&self) -> &dyn LedgerView {
        self.pool_distr_view()
    }

    /// The current authoritative `PoolDistrView` (leadership / leader-schedule): the promoted N+1
    /// view once promoted, else the seed view. The concrete companion of `ledger_view()`.
    pub fn pool_distr_view(&self) -> &PoolDistrView {
        self.promoted_view.as_ref().unwrap_or(self.seed_view)
    }

    /// Whether the N+1 view has been promoted.
    pub fn is_promoted(&self) -> bool {
        self.activation.is_promoted()
    }

    /// The promoted source `EpochConsensusView` (the bound N+1 identity), if promoted.
    pub fn promoted_source(&self) -> Option<&EpochConsensusView> {
        self.activation.promoted()
    }

    /// Atomically promote to the bound N+1 view: record the source `EpochConsensusView` (one-way;
    /// idempotent on the SAME source; a DIFFERENT source for an existing promotion is a terminal
    /// `EpochViewActivationConflict`) and, on success, install the projected `PoolDistrView` as the
    /// active view. The projected view is the caller's `source.to_pool_distr_view(...)`
    /// (DC-EPOCH-12), so the authority never re-projects or reads an unbound parameter.
    pub fn promote(
        &mut self,
        source: EpochConsensusView,
        projected: PoolDistrView,
    ) -> Result<(), EpochViewActivationError> {
        self.activation.promote(source)?;
        self.promoted_view = Some(projected);
        Ok(())
    }

    /// Advance the authority to the NEXT epoch's bound view (DC-EPOCH-17 / B3): the per-boundary
    /// generalization of `promote`. From an ALREADY-promoted authority the new view must answer for
    /// EXACTLY one epoch past the current (Promoted(P) -> P+1 at boundary k>=2); the FIRST promotion
    /// from `Seed` is unconstrained -- the ECA-5 bridge derives seed+1, the window-replay derives
    /// seed+2 from the seed authority (the leadership lag = 2). Re-advancing to the SAME promoted
    /// source is an idempotent replay; from a promotion, a boundary skip / regression / same-epoch
    /// view is a terminal `EpochViewActivationConflict`. The projected `PoolDistrView` (the caller's
    /// `source.to_pool_distr_view(...)`) is installed as the active view in lockstep. Durable-before-
    /// visible is the caller's contract (the WAL activation record precedes this), exactly as `promote`.
    pub fn advance(
        &mut self,
        source: EpochConsensusView,
        projected: PoolDistrView,
    ) -> Result<(), EpochViewActivationError> {
        // Idempotent replay: re-advancing to the SAME promoted source is a no-op.
        if self.activation.promoted().is_some_and(|p| *p == source) {
            return Ok(());
        }
        // From an ALREADY-promoted authority the new view must be EXACTLY one epoch ahead -- the
        // per-boundary advance (P -> P+1). From `Seed` the first promotion is unconstrained: the
        // window-replay legitimately derives seed+2 from a seed-epoch authority (the MARK/SET/GO
        // leadership snapshot lag = 2), and the ECA-5 bridge derives seed+1 -- both are first
        // promotions, never an advance.
        if self.activation.is_promoted() {
            let current = self.epoch().0;
            let next = projected.epoch().0;
            if Some(next) != current.checked_add(1) {
                return Err(EpochViewActivationError::EpochViewActivationConflict);
            }
        }
        self.activation = ActiveEpochView::Promoted(source);
        self.promoted_view = Some(projected);
        Ok(())
    }

    /// The epoch the current authoritative view answers for (the seed epoch N before promotion, the
    /// target epoch N+1 after). The input to the epoch-match guard.
    pub fn epoch(&self) -> EpochNo {
        self.pool_distr_view().epoch()
    }

    /// The identity of the currently-active view (DC-EPOCH-14 #11 cross-consumer identity): its
    /// epoch + the promoted source's canonical hash (`None` while seed). Two consumers reading THIS
    /// one holder at the same slot resolve the SAME identity BY CONSTRUCTION; the boundary test
    /// asserts header-validation and forge agree on `(epoch, canonical hash)`, not merely the epoch.
    pub fn active_view_identity(&self) -> (EpochNo, Option<Hash32>) {
        (self.epoch(), self.promoted_source().map(|v| v.canonical_hash()))
    }

    /// DC-EPOCH-14 #9 (slot-aware, bidirectional, MODE-aware): the per-call epoch guard. Equality ⇒
    /// `Permitted`. `authority > slot` ⇒ terminal `PrematurePromotion` (BOTH modes — a promoted
    /// authority ahead of a real slot is a routing bug). `authority < slot` depends on the CANONICAL
    /// mode: `SeedOnly` ⇒ `SeedOnlyPastSupport` (a limited producer past its supported epoch — no
    /// forge, but the follow loop stays ALIVE, never a halt); `ContinuityRequired` ⇒ terminal
    /// `MissingPromotion` (the boundary should have activated — NEVER fall back to the seed view).
    pub fn guard_epoch(&self, slot_epoch: EpochNo) -> AuthorityEpochVerdict {
        let authority_epoch = self.epoch();
        match authority_epoch.0.cmp(&slot_epoch.0) {
            std::cmp::Ordering::Equal => AuthorityEpochVerdict::Permitted,
            std::cmp::Ordering::Greater => AuthorityEpochVerdict::Terminal(
                AuthorityEpochError::PrematurePromotion {
                    authority_epoch,
                    protocol_epoch: slot_epoch,
                },
            ),
            std::cmp::Ordering::Less => match &self.mode {
                EpochAuthorityMode::SeedOnly { supported_epoch } => {
                    AuthorityEpochVerdict::SeedOnlyPastSupport {
                        supported_epoch: *supported_epoch,
                        slot_epoch,
                    }
                }
                EpochAuthorityMode::ContinuityRequired { .. } => AuthorityEpochVerdict::Terminal(
                    AuthorityEpochError::MissingPromotion {
                        authority_epoch,
                        protocol_epoch: slot_epoch,
                    },
                ),
            },
        }
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

    fn pdv(epoch: u64, marker: u8) -> PoolDistrView {
        use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
        use ade_ledger::consensus_view::PoolEntry as BluePoolEntry;
        let mut pools = BTreeMap::new();
        pools.insert(
            Hash28([marker; 28]),
            BluePoolEntry { active_stake: 1_000, vrf_keyhash: Hash32([marker; 32]) },
        );
        PoolDistrView::new(EpochNo(epoch), 1_000, ActiveSlotsCoeff { numer: 1, denom: 20 }, pools)
    }

    // DC-EPOCH-17 (B3): the authority ADVANCES per boundary (P -> P+1), not once. Boundary 1
    // (seed -> seed+1) then boundary 2 (seed+1 -> seed+2); a boundary SKIP is terminal; the same
    // source is an idempotent replay; a rejected advance leaves the authority unchanged.
    #[test]
    fn authority_advances_per_boundary_and_rejects_skip() {
        let seed = pdv(577, 0x10);
        let mut auth = ActiveEpochAuthority::seed(&seed);
        // boundary 1: seed 577 -> 578.
        auth.advance(view(1000), pdv(578, 0x20)).expect("advance to 578");
        assert_eq!(auth.epoch(), EpochNo(578));
        // boundary 2: 578 -> 579 -- the advance PAST the first promotion (the welded seam could not).
        auth.advance(view(2000), pdv(579, 0x30)).expect("advance to 579");
        assert_eq!(auth.epoch(), EpochNo(579));
        assert_eq!(auth.pool_distr_view(), &pdv(579, 0x30));
        // idempotent: re-advancing to the SAME promoted source is a no-op.
        auth.advance(view(2000), pdv(579, 0x30)).expect("idempotent re-advance");
        assert_eq!(auth.epoch(), EpochNo(579));
        // a boundary SKIP (579 -> 581) is terminal -- never a silent multi-epoch jump.
        assert_eq!(
            auth.advance(view(3000), pdv(581, 0x40)),
            Err(EpochViewActivationError::EpochViewActivationConflict)
        );
        // no partial mutation: the authority still answers for 579 after the rejected skip.
        assert_eq!(auth.epoch(), EpochNo(579));
    }

    // ECA-3 (DC-EPOCH-14): the ONE owned authority yields the seed view until promotion, then the
    // promoted N+1 view -- for BOTH ledger_view() (header validation) AND pool_distr_view()
    // (leadership). Promotion is one-way + idempotent + conflict-terminal.
    #[test]
    fn authority_seed_then_promote_swaps_the_view_for_both_consumers() {
        let seed = pdv(577, 0x10);
        let mut auth = ActiveEpochAuthority::seed(&seed);
        assert!(!auth.is_promoted());
        assert_eq!(auth.pool_distr_view(), &seed, "seed view active before promotion");
        assert_eq!(auth.ledger_view().total_active_stake(EpochNo(577)), Some(1_000));
        assert!(auth.promoted_source().is_none());

        let src = view(2_000); // an EpochConsensusView (epoch 578 per bindings())
        let promoted = pdv(578, 0x20);
        auth.promote(src.clone(), promoted.clone()).expect("first promotion");
        assert!(auth.is_promoted());
        assert_eq!(auth.pool_distr_view(), &promoted, "promoted N+1 view now active (leadership)");
        assert_eq!(
            auth.ledger_view().total_active_stake(EpochNo(578)),
            Some(1_000),
            "header validation now reads the promoted view"
        );
        assert!(
            auth.ledger_view().total_active_stake(EpochNo(577)).is_none(),
            "the seed epoch is no longer observable post-promotion (DC-EPOCH-05)"
        );
        assert_eq!(auth.promoted_source(), Some(&src));

        // idempotent on the SAME source.
        auth.promote(src.clone(), promoted.clone()).expect("idempotent re-promotion");
        assert_eq!(auth.pool_distr_view(), &promoted);

        // a DIFFERENT source is a terminal conflict; the active view is unchanged.
        assert_eq!(
            auth.promote(view(9_999), promoted.clone()),
            Err(EpochViewActivationError::EpochViewActivationConflict)
        );
        assert_eq!(auth.pool_distr_view(), &promoted, "unchanged on conflict (no partial mutation)");
    }

    // DC-EPOCH-14 #9 (mode-aware, slot-aware, bidirectional guard) + #11 (exact active-view identity).
    #[test]
    fn authority_epoch_guard_is_mode_aware_and_identity_is_exact() {
        let seed = pdv(577, 0x10);

        // SEED-ONLY (a limited producer): at its epoch -> Permitted; AHEAD slot is a routing bug
        // (terminal); PAST its supported epoch -> a graceful no-forge (NOT terminal), the loop lives.
        let seed_only = ActiveEpochAuthority::seed(&seed);
        assert_eq!(seed_only.epoch(), EpochNo(577));
        assert_eq!(seed_only.guard_epoch(EpochNo(577)), AuthorityEpochVerdict::Permitted);
        assert_eq!(
            seed_only.guard_epoch(EpochNo(578)),
            AuthorityEpochVerdict::SeedOnlyPastSupport {
                supported_epoch: EpochNo(577),
                slot_epoch: EpochNo(578),
            },
            "a seed-only producer past its supported epoch is a graceful no-forge, NEVER terminal"
        );
        assert_eq!(
            seed_only.guard_epoch(EpochNo(576)),
            AuthorityEpochVerdict::Terminal(AuthorityEpochError::PrematurePromotion {
                authority_epoch: EpochNo(577),
                protocol_epoch: EpochNo(576),
            }),
            "an authority AHEAD of the slot is a routing bug -> terminal in either mode"
        );

        // CONTINUITY-REQUIRED: behind its slot -> TERMINAL MissingPromotion (the boundary should
        // have activated -- never fall back to the seed view).
        let cont = ActiveEpochAuthority::continuity(
            &seed,
            Point { slot: SlotNo(1), hash: Hash32([0; 32]) },
            Hash32([0xAB; 32]),
            TargetEpochPolicy::SetSnapshotLag { lag_epochs: 2 },
        );
        assert_eq!(cont.guard_epoch(EpochNo(577)), AuthorityEpochVerdict::Permitted);
        assert_eq!(
            cont.guard_epoch(EpochNo(578)),
            AuthorityEpochVerdict::Terminal(AuthorityEpochError::MissingPromotion {
                authority_epoch: EpochNo(577),
                protocol_epoch: EpochNo(578),
            }),
            "a continuity-capable producer with no promotion at an N+1 slot is TERMINAL"
        );

        // #11: identity carries the promoted source's canonical hash, not merely the epoch -- so two
        // N+1 candidates with different bindings are distinguishable.
        let mut auth = ActiveEpochAuthority::seed(&seed);
        assert_eq!(auth.active_view_identity(), (EpochNo(577), None), "seed identity: (epoch, None)");
        let src = view(2_000);
        let promoted = pdv(578, 0x20);
        auth.promote(src.clone(), promoted).expect("promote");
        assert_eq!(auth.active_view_identity(), (EpochNo(578), Some(src.canonical_hash())));
    }

    // DC-EPOCH-14 #11 (cross-consumer identity): at a given slot, header validation and the forge
    // decision MUST resolve the SAME authority epoch AND the SAME active-view canonical hash from the
    // ONE holder. Epoch alone is INSUFFICIENT -- two N+1 candidates with different source bindings
    // both report 578 -- so identity binds the canonical hash; reading the one authority guarantees it.
    #[test]
    fn cross_consumer_identity_validation_and_forge_resolve_one_authority_view() {
        let seed = pdv(577, 0x10);
        let mut auth = ActiveEpochAuthority::seed(&seed);

        // SEED: the forge view (pool_distr_view) and the validation view (ledger_view) are the SAME
        // underlying PoolDistrView; identity is (577, None).
        assert_eq!(auth.active_view_identity(), (EpochNo(577), None));
        assert_eq!(auth.pool_distr_view().epoch(), EpochNo(577));
        assert_eq!(
            auth.ledger_view().total_active_stake(EpochNo(577)),
            auth.pool_distr_view().total_active_stake(EpochNo(577)),
            "validation + forge resolve the SAME seed view"
        );

        // PROMOTE to an N+1 candidate; identity now binds the source's canonical hash, and BOTH
        // consumers resolve the promoted (578) view identically.
        let src = view(2_000);
        auth.promote(src.clone(), pdv(578, 0x20)).expect("promote");
        let identity = auth.active_view_identity();
        assert_eq!(identity, (EpochNo(578), Some(src.canonical_hash())));
        assert_eq!(auth.pool_distr_view().epoch(), EpochNo(578));
        assert_eq!(
            auth.ledger_view().total_active_stake(EpochNo(578)),
            auth.pool_distr_view().total_active_stake(EpochNo(578)),
            "validation + forge resolve the SAME promoted view"
        );

        // A DIFFERENT source binding at the SAME epoch 578 yields a DIFFERENT identity hash -- so
        // epoch alone would NOT distinguish the two candidates; cross-consumer agreement must bind the
        // canonical hash, which reading the ONE holder guarantees.
        let seed_b = pdv(577, 0x10);
        let mut auth_b = ActiveEpochAuthority::seed(&seed_b);
        auth_b.promote(view(9_999), pdv(578, 0x20)).expect("promote");
        assert_ne!(
            auth_b.active_view_identity().1,
            identity.1,
            "two N+1 candidates with different source bindings have distinct active-view hashes"
        );
    }

    // DC-EPOCH-14 (header-validation GATED-FAIL-CLOSED contract, user 2026-06-22 option 2): a SeedOnly
    // producer past its seed epoch CANNOT validate an N+1 peer header -- its SOLE ledger view (the
    // authority's, criterion #7) is epoch-gated, so EVERY N+1 leadership query returns None. The
    // leader/VRF check cannot resolve the header's pool at N+1 -> the header is REJECTED before
    // acceptance (a wrong-epoch view can NEVER admit an N+1 block). On the single-producer admit path
    // this rejection is the EXISTING fail-closed halt (run_node_sync Err -> NodeLifecycleError::
    // RelaySync, node_lifecycle ~1962) -- the Cardano-compatible REQUIRED closure (DC-NODE-12: never
    // admit an unvalidatable block). The structured AuthorityEpochMismatch classification for THIS
    // path is DEFERRED as diagnostic refinement; the SEMANTIC enforcement -- reject, never admit -- is
    // proven here, and the forge-tick SeedOnly follow stays alive (forge_tick_off_epoch_slot...local).
    #[test]
    fn seed_only_sole_view_cannot_validate_n1_header_rejects_before_acceptance() {
        let seed = pdv(577, 0x10);
        let auth = ActiveEpochAuthority::seed(&seed);
        let sole_view = auth.ledger_view();
        let pool = Hash28([0x11; 28]);
        // EVERY N+1 (578) leadership query -> None: an N+1 header cannot be validated/admitted.
        assert_eq!(sole_view.total_active_stake(EpochNo(578)), None);
        assert_eq!(sole_view.pool_active_stake(EpochNo(578), &pool), None);
        assert_eq!(sole_view.pool_vrf_keyhash(EpochNo(578), &pool), None);
        assert_eq!(sole_view.active_slots_coeff(EpochNo(578)), None);
        // ... while it DOES answer for its own seed epoch 577 -> it follows WITHIN the epoch normally
        // (the rejection is scoped to past-epoch headers, not a blanket follow halt).
        assert!(sole_view.active_slots_coeff(EpochNo(577)).is_some());
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
