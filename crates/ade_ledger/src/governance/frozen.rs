// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! LV-15b-0 (DC-EPOCH-47): ratification for the boundary E→E+1 is decided from the authority frozen at the start
//! of E, and only applied at E→E+1.
//!
//! At E−1→E the boundary builds a [`FrozenRatificationInput`] from the state it just constructed, and
//! [`ratify_frozen`] turns it into a [`RatificationRecord`]. At E→E+1, [`apply_ratification_record`] applies that
//! record to the proposals and registrations current then. It reads no votes, stake, committee, thresholds or
//! expiry.
//!
//! The three frozen sources are observed at different points, and each has its own type:
//! - [`FrozenPoolStake`]: the mark built at the freezing boundary, before its credits (SPO distribution only);
//! - [`PostBoundaryAccounts`]: UTxO stake and account balances after every boundary credit (account terms only);
//! - [`FrozenGovernanceView`]: proposals, votes, DReps, committee, parameters and roots of the new epoch.
//!
//! The consuming boundary sees a fourth, [`CurrentGovernanceState`], which converts into none of them.

use std::collections::BTreeMap;

use ade_types::conway::cert::DRep;
use ade_types::conway::governance::{GovActionId, GovActionState};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, PoolId};
use ade_types::EpochNo;

use super::{
    apply_decision, decide_ratification, CanonicalFieldSet, ConwayGovernanceEpochInput,
    ConwayGovernanceEpochPlan, DRepStakeDistribution, DanglingVoteDelegationKind, GovernanceTerminal,
    MalformedGovDetail, RatificationDecision, UnsupportedActionKind,
};
use crate::delegation::DelegationState;
use crate::epoch::StakeSnapshot;
use crate::epoch_accumulator::BoundaryBaseStake;
use crate::pparams::ProtocolParameters;
use crate::state::{ConwayGovState, LedgerState};

/// The mark built at the freezing boundary (SNAP), before the boundary's credits. Read only by the SPO
/// distribution.
pub struct FrozenPoolStake<'a> {
    mark: &'a StakeSnapshot,
}

/// Account state after every credit of the freezing boundary (rewards, refunds), with the boundary's per-credential
/// base UTxO stake. Read only by the account terms of the two distributions.
pub struct PostBoundaryAccounts<'a> {
    utxo_stake: &'a BTreeMap<StakeCredential, Coin>,
    delegation: &'a DelegationState,
}

/// The governance state of the epoch the freezing boundary entered.
pub struct FrozenGovernanceView<'a> {
    gov: &'a ConwayGovState,
    pparams: &'a ProtocolParameters,
    epoch: EpochNo,
}

/// Everything ratification for the epoch just entered may read, and nothing else.
pub struct FrozenRatificationInput<'a> {
    pool: FrozenPoolStake<'a>,
    accounts: PostBoundaryAccounts<'a>,
    gov: FrozenGovernanceView<'a>,
}

impl<'a> FrozenRatificationInput<'a> {
    /// The only constructor: `next` is the state a Conway boundary just constructed (its mark is the one built at
    /// that boundary; its balances include every boundary credit) and `base` is that boundary's base UTxO stake.
    /// `Ok(None)` when `next` tracks no governance. A non-authoritative projection is a terminal, never a read.
    pub fn at_boundary(
        next: &'a LedgerState,
        base: &'a BoundaryBaseStake,
    ) -> Result<Option<Self>, GovernanceTerminal> {
        let point = next.epoch_state.slot;
        let snaps = next
            .epoch_state
            .snapshots
            .as_authoritative()
            .ok_or(GovernanceTerminal::FullBoundaryStateRequired { boundary_point: point })?;
        let cert = next.cert_state.require_full(point)?;
        let Some(gov) = next.gov_state.require_full(point)?.as_ref() else {
            return Ok(None);
        };
        Ok(Some(Self {
            pool: FrozenPoolStake { mark: &snaps.mark.0 },
            accounts: PostBoundaryAccounts {
                utxo_stake: &base.canonical_credential_stake,
                delegation: &cert.delegation,
            },
            gov: FrozenGovernanceView { gov, pparams: &next.protocol_params, epoch: next.epoch_state.epoch },
        }))
    }
}

/// The proposals visible at the start of the epoch, as current state at the consuming boundary. Converts into no
/// frozen type.
pub struct CurrentGovernanceState<'a> {
    proposals: &'a [GovActionState],
}

impl<'a> CurrentGovernanceState<'a> {
    /// The pre-boundary governance state of the consuming boundary.
    pub fn pre_boundary(gov: &'a ConwayGovState) -> Self {
        Self { proposals: &gov.proposals }
    }
}

/// Where a record came from. Every record names one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RatificationSource {
    /// Computed by [`ratify_frozen`] at a boundary Ade crossed.
    Frozen,
    /// Decoded from a bootstrap snapshot's finished ratification (`DRComplete`).
    ImportedFromSnapshot,
}

/// What a frozen ratification decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RatificationOutcome {
    Decided(RatificationDecision),
    /// A threshold-passing action Ade cannot enact, or a ratification-time terminal. Persisted, not raised at the
    /// freeze: it halts at the consuming boundary, with zero mutation. Votes cast later cannot clear it.
    Unenactable(UnenactableReason),
}

/// The ratification-time terminals a record may persist: exactly those [`decide_ratification`] returns. Any other
/// terminal is not a decision about the proposals and is raised at the freeze instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnenactableReason {
    /// [`GovernanceTerminal::UnsupportedRatifiedAction`].
    UnsupportedRatifiedAction {
        action_id: GovActionId,
        kind: UnsupportedActionKind,
        unsupported_keys: CanonicalFieldSet,
    },
    /// [`GovernanceTerminal::UnversionedStateOnEnactPath`].
    UnversionedStateOnEnactPath { action_id: GovActionId },
    /// [`GovernanceTerminal::DormantRequired`].
    DormantRequired,
}

impl UnenactableReason {
    fn from_terminal(t: GovernanceTerminal) -> Result<Self, GovernanceTerminal> {
        match t {
            GovernanceTerminal::UnsupportedRatifiedAction { action_id, kind, unsupported_keys } => {
                Ok(Self::UnsupportedRatifiedAction { action_id, kind, unsupported_keys })
            }
            GovernanceTerminal::UnversionedStateOnEnactPath { action_id } => {
                Ok(Self::UnversionedStateOnEnactPath { action_id })
            }
            GovernanceTerminal::DormantRequired => Ok(Self::DormantRequired),
            other => Err(other),
        }
    }

    /// The terminal the consuming boundary halts with.
    pub fn terminal(&self) -> GovernanceTerminal {
        match self {
            Self::UnsupportedRatifiedAction { action_id, kind, unsupported_keys } => {
                GovernanceTerminal::UnsupportedRatifiedAction {
                    action_id: action_id.clone(),
                    kind: *kind,
                    unsupported_keys: unsupported_keys.clone(),
                }
            }
            Self::UnversionedStateOnEnactPath { action_id } => {
                GovernanceTerminal::UnversionedStateOnEnactPath { action_id: action_id.clone() }
            }
            Self::DormantRequired => GovernanceTerminal::DormantRequired,
        }
    }
}

/// The ratification frozen at the start of `frozen_at`, consumed once at `frozen_at`→`frozen_at + 1`.
///
/// The two distributions are evidence for the decision already made — what makes the record comparable with
/// Cardano's `psDRepDistr` / `psPoolDistr` — never authority for a later transition. They are private here, and
/// only encode, decode, fingerprint and the differential comparator read them
/// (`ci/ci_check_frozen_ratification_authority.sh`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatificationRecord {
    frozen_at: EpochNo,
    source: RatificationSource,
    frozen_proposals: std::collections::BTreeSet<GovActionId>,
    drep_distribution: BTreeMap<DRep, Coin>,
    pool_distribution: BTreeMap<PoolId, Coin>,
    outcome: RatificationOutcome,
}

impl RatificationRecord {
    pub fn frozen_at(&self) -> EpochNo {
        self.frozen_at
    }
    pub fn source(&self) -> RatificationSource {
        self.source
    }
    pub fn outcome(&self) -> &RatificationOutcome {
        &self.outcome
    }

    /// The evidence: the proposal ids the freeze saw, and the DRep and pool distributions. Only the record codec
    /// may call this; the gate fails on any other caller.
    #[allow(clippy::type_complexity)]
    pub(crate) fn evidence(
        &self,
    ) -> (&std::collections::BTreeSet<GovActionId>, &BTreeMap<DRep, Coin>, &BTreeMap<PoolId, Coin>) {
        (&self.frozen_proposals, &self.drep_distribution, &self.pool_distribution)
    }

    /// Rebuild a record from its persisted or imported parts. Only the record codec and the bootstrap import may
    /// call this; the gate fails on any other caller.
    pub(crate) fn from_parts(
        frozen_at: EpochNo,
        source: RatificationSource,
        frozen_proposals: std::collections::BTreeSet<GovActionId>,
        drep_distribution: BTreeMap<DRep, Coin>,
        pool_distribution: BTreeMap<PoolId, Coin>,
        outcome: RatificationOutcome,
    ) -> Self {
        Self { frozen_at, source, frozen_proposals, drep_distribution, pool_distribution, outcome }
    }
}

/// Proposal deposits per return credential (Cardano's `proposalsDeposits`). A return address that is not a reward
/// account is a terminal.
fn deposits_by_return_credential(
    proposals: &[GovActionState],
) -> Result<BTreeMap<StakeCredential, Coin>, GovernanceTerminal> {
    let mut out: BTreeMap<StakeCredential, Coin> = BTreeMap::new();
    for p in proposals {
        let cred = crate::epoch_accumulator::reward_account_credential(&p.return_addr).ok_or(
            GovernanceTerminal::Malformed {
                action_id: p.action_id.clone(),
                detail: MalformedGovDetail::ReturnAddrNotRewardAccount,
            },
        )?;
        let e = out.entry(cred).or_insert(Coin(0));
        e.0 = e.0.saturating_add(p.deposit.0);
    }
    Ok(out)
}

/// The frozen DRep distribution (Cardano's `computeDRepDistr`). Every vote delegation counts its delegator's base
/// UTxO stake, post-boundary reward balance and returning proposal deposits toward its DRep, even when the total is
/// zero.
///
/// A delegation from an unregistered account, or to an unregistered credential DRep, is a
/// [`GovernanceTerminal::DanglingVoteDelegation`]: Cardano makes both unrepresentable from PV10, so it is never
/// silently excluded.
pub fn derive_frozen_drep_distribution(
    accounts: &PostBoundaryAccounts<'_>,
    gov: &FrozenGovernanceView<'_>,
) -> Result<BTreeMap<DRep, Coin>, GovernanceTerminal> {
    let deposits = deposits_by_return_credential(&gov.gov.proposals)?;
    let mut out: BTreeMap<DRep, Coin> = BTreeMap::new();
    for (cred, drep) in &gov.gov.vote_delegations {
        if !accounts.delegation.registrations.contains_key(cred) {
            return Err(GovernanceTerminal::DanglingVoteDelegation {
                credential: cred.clone(),
                kind: DanglingVoteDelegationKind::UnregisteredAccount,
            });
        }
        if let Some(target) = crate::gov_cert::drep_credential(drep) {
            if !gov.gov.drep_expiry.contains_key(&target) {
                return Err(GovernanceTerminal::DanglingVoteDelegation {
                    credential: cred.clone(),
                    kind: DanglingVoteDelegationKind::UnregisteredDRep,
                });
            }
        }
        let term = |m: &BTreeMap<StakeCredential, Coin>| m.get(cred).map_or(0, |c| c.0);
        let stake = term(accounts.utxo_stake)
            .saturating_add(term(&accounts.delegation.rewards))
            .saturating_add(term(&deposits));
        let e = out.entry(drep.clone()).or_insert(Coin(0));
        e.0 = e.0.saturating_add(stake);
    }
    Ok(out)
}

/// The frozen SPO distribution: the mark built at the freezing boundary, plus the proposal deposits of accounts
/// delegated (after the boundary) to a pool present in that mark. Every pool registered at SNAP appears, at zero if
/// it has no stake.
pub fn derive_frozen_pool_distribution(
    pool: &FrozenPoolStake<'_>,
    accounts: &PostBoundaryAccounts<'_>,
    gov: &FrozenGovernanceView<'_>,
) -> Result<BTreeMap<PoolId, Coin>, GovernanceTerminal> {
    let mut out: BTreeMap<PoolId, Coin> = pool.mark.pool_stakes.clone();
    for p in pool.mark.pool_params.keys() {
        out.entry(p.clone()).or_insert(Coin(0));
    }
    for (cred, deposit) in deposits_by_return_credential(&gov.gov.proposals)? {
        if let Some(p) = accounts.delegation.delegations.get(&cred) {
            if let Some(stake) = out.get_mut(p) {
                stake.0 = stake.0.saturating_add(deposit.0);
            }
        }
    }
    Ok(out)
}

/// DECIDE at the freeze: derive both distributions and ratify the epoch's proposals as of `frozen_at`. A
/// ratification terminal is recorded as [`RatificationOutcome::Unenactable`]; only an invariant failure in the
/// derivation is an immediate error.
pub fn ratify_frozen(input: &FrozenRatificationInput<'_>) -> Result<RatificationRecord, GovernanceTerminal> {
    let drep_distribution = derive_frozen_drep_distribution(&input.accounts, &input.gov)?;
    let pool_distribution = derive_frozen_pool_distribution(&input.pool, &input.accounts, &input.gov)?;
    let gov = input.gov.gov;
    let drep_stake: DRepStakeDistribution =
        drep_distribution.iter().map(|(d, c)| (d.clone(), c.0)).collect();
    let committee_quorum = crate::rational::Rational::new(
        gov.committee_quorum.0 as i128,
        gov.committee_quorum.1.max(1) as i128,
    )
    .unwrap_or_else(crate::rational::Rational::one);
    let frozen_at = input.gov.epoch;
    let decide_input = ConwayGovernanceEpochInput {
        proposals: &gov.proposals,
        drep_stake: &drep_stake,
        pool_stake: &pool_distribution,
        committee_members: &gov.committee,
        committee_quorum: &committee_quorum,
        pool_thresholds: &gov.pool_voting_thresholds,
        drep_thresholds: &gov.drep_voting_thresholds,
        committee_hot_keys: &gov.committee_hot_keys,
        drep_expiry: &gov.drep_expiry,
        num_dormant: &gov.num_dormant,
        current_pparams: input.gov.pparams,
        current_prev_pparam_action: &gov.prev_pparam_action,
        // The boundary that will consume the record; `decide_ratification` reads `frozen_at` directly.
        new_epoch: frozen_at.0.saturating_add(1),
    };
    let outcome = match decide_ratification(&decide_input, frozen_at.0) {
        Ok(decision) => RatificationOutcome::Decided(decision),
        Err(terminal) => RatificationOutcome::Unenactable(UnenactableReason::from_terminal(terminal)?),
    };
    Ok(RatificationRecord {
        frozen_at,
        source: RatificationSource::Frozen,
        frozen_proposals: gov.proposals.iter().map(|p| p.action_id.clone()).collect(),
        drep_distribution,
        pool_distribution,
        outcome,
    })
}

/// APPLY at the consuming boundary into `new_epoch`: the record must have been frozen at `new_epoch − 1`
/// ([`GovernanceTerminal::RatificationRecordEpochMismatch`] otherwise). An `Unenactable` record halts here with its
/// terminal; a decided one is applied to `current` with deposits routed by `is_registered_now`.
pub fn apply_ratification_record(
    record: &RatificationRecord,
    new_epoch: EpochNo,
    current: &CurrentGovernanceState<'_>,
    is_registered_now: impl Fn(&StakeCredential) -> bool,
) -> Result<ConwayGovernanceEpochPlan, GovernanceTerminal> {
    if record.frozen_at.0.checked_add(1) != Some(new_epoch.0) {
        return Err(GovernanceTerminal::RatificationRecordEpochMismatch {
            frozen_at: record.frozen_at,
            boundary: new_epoch,
        });
    }
    match &record.outcome {
        RatificationOutcome::Decided(decision) => apply_decision(decision, current.proposals, is_registered_now),
        RatificationOutcome::Unenactable(reason) => Err(reason.terminal()),
    }
}

/// The finished ratification a bootstrap snapshot carries (`govState[6]` = `DRComplete PulsingSnapshot
/// RatifyState`), decoded but not yet interpreted.
pub struct SnapshotRatification {
    /// The ids of `psProposals`: the proposals the pulser saw.
    pub proposals: std::collections::BTreeSet<GovActionId>,
    /// `psDRepDistr`.
    pub drep_distribution: BTreeMap<DRep, Coin>,
    /// `psPoolDistr`, stake only.
    pub pool_distribution: BTreeMap<PoolId, Coin>,
    /// `rsEnacted`, in enactment order.
    pub enacted: Vec<GovActionState>,
    /// `rsExpired`.
    pub expired: std::collections::BTreeSet<GovActionId>,
    /// `rsDelayed`.
    pub delayed: bool,
    /// `rsEnactState.ensCurPParams`: the parameters after the enactments.
    pub enacted_pparams: ProtocolParameters,
    /// `rsEnactState.ensPrevGovActionIds.pgaPParamUpdate`: the parameter root after the enactments.
    pub enacted_pparam_root: Option<GovActionId>,
}

/// Why a snapshot's finished ratification cannot be imported. Each is a bootstrap refusal; nothing is repaired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RatificationImportRefusal {
    /// `rsDelayed` is set but Ade's classification of `rsEnacted` is a decision it can apply: a delaying action
    /// was enacted that Ade did not recognise as one.
    DelayedWithoutUnenactableAction,
    /// The parameters Ade builds for the decision differ from `ensCurPParams`.
    EnactedParametersMismatch,
    /// The parameter root after the decision differs from `ensPrevGovActionIds.pgaPParamUpdate`.
    EnactedRootMismatch { expected: Option<GovActionId>, snapshot: Option<GovActionId> },
}

/// Import a snapshot's finished ratification as the record frozen at `frozen_at` (the snapshot's epoch), applying
/// Ade's own classification to `rsEnacted`:
/// - nothing enacted: `Decided` with the snapshot's expired set;
/// - one `ParameterChange` in the supported subset: `Decided`, its parameters built from `current_pparams`
///   exactly as Ade's ratifier builds them;
/// - anything else: `Unenactable`, with the terminal Ade's ratifier gives (the first non-`ParameterChange` in
///   `GovActionId` order, else `ChainedEnactment` on the second of two chained changes).
///
/// A `Decided` import is cross-checked against the snapshot's own post-enactment parameters and root.
pub(crate) fn import_snapshot_ratification(
    frozen_at: EpochNo,
    snap: SnapshotRatification,
    current_pparams: &ProtocolParameters,
    current_root: Option<&GovActionId>,
) -> Result<RatificationRecord, RatificationImportRefusal> {
    let outcome = match snap.enacted.as_slice() {
        [] => RatificationOutcome::Decided(RatificationDecision { enacted: None, expired: snap.expired.clone() }),
        [one] => match super::build_parameter_change_enactment(one, current_pparams) {
            Ok(e) => RatificationOutcome::Decided(RatificationDecision { enacted: Some(e), expired: snap.expired.clone() }),
            Err(t) => RatificationOutcome::Unenactable(
                UnenactableReason::from_terminal(t).map_err(|_| RatificationImportRefusal::EnactedParametersMismatch)?,
            ),
        },
        many => {
            let mut sorted: Vec<&GovActionState> = many.iter().collect();
            sorted.sort_by(|a, b| a.action_id.cmp(&b.action_id));
            let non_pc = sorted
                .iter()
                .find(|p| !matches!(p.gov_action, ade_types::conway::governance::GovAction::ParameterChange { .. }));
            let (action_id, kind) = match non_pc {
                Some(p) => (p.action_id.clone(), UnsupportedActionKind::NotParameterChange),
                None => (many[1].action_id.clone(), UnsupportedActionKind::ChainedEnactment),
            };
            RatificationOutcome::Unenactable(UnenactableReason::UnsupportedRatifiedAction {
                action_id,
                kind,
                unsupported_keys: CanonicalFieldSet::default(),
            })
        }
    };
    if let RatificationOutcome::Decided(d) = &outcome {
        if snap.delayed {
            return Err(RatificationImportRefusal::DelayedWithoutUnenactableAction);
        }
        let (built, root) = match &d.enacted {
            Some(e) => (&*e.pparams, Some(&e.action_id)),
            None => (current_pparams, current_root),
        };
        if *built != snap.enacted_pparams {
            return Err(RatificationImportRefusal::EnactedParametersMismatch);
        }
        if root != snap.enacted_pparam_root.as_ref() {
            return Err(RatificationImportRefusal::EnactedRootMismatch {
                expected: root.cloned(),
                snapshot: snap.enacted_pparam_root.clone(),
            });
        }
    }
    Ok(RatificationRecord {
        frozen_at,
        source: RatificationSource::ImportedFromSnapshot,
        frozen_proposals: snap.proposals,
        drep_distribution: snap.drep_distribution,
        pool_distribution: snap.pool_distribution,
        outcome,
    })
}

/// Entry-by-entry comparison of one evidence distribution against a reference (CE-LV15b0-9).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DistributionDifferential {
    /// Keys in both with the same value.
    pub matching: usize,
    /// Keys in both with different values.
    pub differing: usize,
    /// Keys only in this record.
    pub only_self: usize,
    /// Keys only in the reference.
    pub only_reference: usize,
}

impl DistributionDifferential {
    fn of<K: Ord, V: PartialEq>(a: &BTreeMap<K, V>, b: &BTreeMap<K, V>) -> Self {
        let mut d = Self::default();
        for (k, v) in a {
            match b.get(k) {
                Some(w) if w == v => d.matching += 1,
                Some(_) => d.differing += 1,
                None => d.only_self += 1,
            }
        }
        d.only_reference = b.keys().filter(|k| !a.contains_key(k)).count();
        d
    }

    /// Every entry of both sides matches.
    pub fn is_exact(&self) -> bool {
        self.differing == 0 && self.only_self == 0 && self.only_reference == 0
    }
}

/// A record compared with a reference record for the same epoch: the CE-LV15b0-9 differential between Ade's
/// native freeze and the snapshot's own finished ratification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordDifferential {
    pub same_epoch: bool,
    pub same_outcome: bool,
    pub same_proposals: bool,
    pub drep: DistributionDifferential,
    pub pool: DistributionDifferential,
}

impl RecordDifferential {
    pub fn is_exact(&self) -> bool {
        self.same_epoch && self.same_outcome && self.same_proposals && self.drep.is_exact() && self.pool.is_exact()
    }
}

impl RatificationRecord {
    /// Compare this record with `reference` entry by entry. The CE-9 comparator: read-only, and the only reader
    /// of the evidence outside the codec and the fingerprint.
    pub fn differential(&self, reference: &RatificationRecord) -> RecordDifferential {
        RecordDifferential {
            same_epoch: self.frozen_at == reference.frozen_at,
            same_outcome: self.outcome == reference.outcome,
            same_proposals: self.frozen_proposals == reference.frozen_proposals,
            drep: DistributionDifferential::of(&self.drep_distribution, &reference.drep_distribution),
            pool: DistributionDifferential::of(&self.pool_distribution, &reference.pool_distribution),
        }
    }

    /// Fixture only: a record frozen at `frozen_at` that decided nothing (no enactment, nothing expired), with
    /// empty evidence. For tests outside this crate that need a governance state able to cross one boundary.
    /// `ci/ci_check_frozen_ratification_authority.sh` fails if production code calls it.
    pub fn fixture_decided_nothing(frozen_at: EpochNo) -> Self {
        Self {
            frozen_at,
            source: RatificationSource::Frozen,
            frozen_proposals: std::collections::BTreeSet::new(),
            drep_distribution: BTreeMap::new(),
            pool_distribution: BTreeMap::new(),
            outcome: RatificationOutcome::Decided(RatificationDecision {
                enacted: None,
                expired: std::collections::BTreeSet::new(),
            }),
        }
    }
}

/// Test fixtures only: give `state` the record a boundary entering its epoch would have frozen over it, so a
/// synthetic state can cross a Conway boundary. Its proposals and votes are treated as present at the freeze.
#[cfg(test)]
#[allow(clippy::expect_used)]
pub(crate) fn freeze_as_entered_for_test(state: &mut LedgerState, base: &BoundaryBaseStake) {
    let record = {
        let input = FrozenRatificationInput::at_boundary(state, base)
            .expect("authoritative fixture")
            .expect("fixture tracks governance");
        ratify_frozen(&input).expect("fixture freezes")
    };
    if let crate::state::GovStateProjection::Authoritative(Some(g)) = &mut state.gov_state {
        g.ratification = crate::state::RatificationState::Recorded(record);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::epoch::SnapshotPoolParams;
    use crate::governance::{DepositReturn, PParamsDelta, RemovalCause};
    use crate::pparams::MaxBlockExUnits;
    use crate::state::{DormantEpochs, PreviousPParamAction};
    use ade_types::conway::governance::{GovAction, GovActionId, Vote};
    use ade_types::{Hash28, Hash32};

    /// The real 69c948cd..#0 memory-only exec-units update (see `cre_s4_3c_enactment_tests`).
    const WITNESS_UPDATE: &[u8] = &[
        0xa2, 0x14, 0x82, 0x1a, 0x00, 0xfb, 0xc5, 0x20, 0x1b, 0x00, 0x00, 0x00, 0x02, 0x54, 0x0b, 0xe4,
        0x00, 0x15, 0x82, 0x1a, 0x04, 0x4a, 0xa2, 0x00, 0x1b, 0x00, 0x00, 0x00, 0x04, 0xa8, 0x17, 0xc8,
        0x00,
    ];
    const E: u64 = 1340;

    fn key(b: u8) -> StakeCredential {
        StakeCredential::KeyHash(Hash28([b; 28]))
    }
    fn pool(b: u8) -> PoolId {
        PoolId(Hash28([b; 28]))
    }
    fn gaid(b: u8) -> GovActionId {
        GovActionId { tx_hash: Hash32([b; 32]), index: 0 }
    }
    /// A key-hash reward account for credential `key(b)`.
    fn reward_addr(b: u8) -> Vec<u8> {
        let mut v = vec![0xe0];
        v.extend_from_slice(&[b; 28]);
        v
    }
    fn prop(id: u8, prev: Option<GovActionId>, committee_votes: Vec<(StakeCredential, Vote)>, expires_after: u64, deposit: u64, ret: u8) -> GovActionState {
        GovActionState {
            action_id: gaid(id),
            committee_votes,
            drep_votes: Vec::new(),
            spo_votes: Vec::new(),
            deposit: Coin(deposit),
            return_addr: reward_addr(ret),
            gov_action: GovAction::ParameterChange { prev_action: prev, update: WITNESS_UPDATE.to_vec(), policy_hash: None },
            proposed_in: EpochNo(E - 2),
            expires_after: EpochNo(expires_after),
        }
    }
    fn yes_2of3() -> Vec<(StakeCredential, Vote)> {
        vec![(key(0xC1), Vote::Yes), (key(0xC2), Vote::Yes)]
    }
    fn pp() -> ProtocolParameters {
        ProtocolParameters {
            max_block_ex_units: MaxBlockExUnits::Bound { mem: 62_000_000, steps: 20_000_000_000 },
            ..Default::default()
        }
    }
    fn gov(proposals: Vec<GovActionState>) -> ConwayGovState {
        ConwayGovState {
            ratification: crate::state::RatificationState::Unversioned,
            prev_pparam_action: PreviousPParamAction::Enacted(gaid(0x60)),
            proposals,
            committee: [(key(0xC1), 1400u64), (key(0xC2), 1400), (key(0xC3), 1400)].into_iter().collect(),
            committee_quorum: (2, 3),
            drep_expiry: BTreeMap::new(),
            gov_action_lifetime: 6,
            vote_delegations: BTreeMap::new(),
            pool_voting_thresholds: Vec::new(),
            drep_voting_thresholds: Vec::new(),
            committee_hot_keys: BTreeMap::new(),
            num_dormant: DormantEpochs::Bound(0),
        }
    }
    fn spp() -> SnapshotPoolParams {
        SnapshotPoolParams { pledge: Coin(0), cost: Coin(0), margin: (0, 1), reward_account: key(0xEE), self_delegated_owners: Default::default() }
    }

    struct Fixture {
        mark: StakeSnapshot,
        utxo: BTreeMap<StakeCredential, Coin>,
        deleg: DelegationState,
        gov: ConwayGovState,
        pp: ProtocolParameters,
    }
    impl Fixture {
        fn new(gov: ConwayGovState) -> Self {
            Self {
                mark: StakeSnapshot { delegations: BTreeMap::new(), pool_stakes: BTreeMap::new(), pool_params: BTreeMap::new() },
                utxo: BTreeMap::new(),
                deleg: DelegationState::new(),
                gov,
                pp: pp(),
            }
        }
        fn input(&self) -> FrozenRatificationInput<'_> {
            FrozenRatificationInput {
                pool: FrozenPoolStake { mark: &self.mark },
                accounts: PostBoundaryAccounts { utxo_stake: &self.utxo, delegation: &self.deleg },
                gov: FrozenGovernanceView { gov: &self.gov, pparams: &self.pp, epoch: EpochNo(E) },
            }
        }
        fn register(&mut self, b: u8) {
            self.deleg.registrations.insert(key(b), Coin(2_000_000));
        }
    }

    /// CE-LV15b0-3 (distribution): every vote delegator counts UTxO stake + post-boundary reward balance +
    /// returning deposits, whether or not it delegates to a pool; the predefined DReps count; zero totals appear.
    #[test]
    fn ce_lv15b0_drep_distribution_counts_every_term_for_every_delegator() {
        let mut g = gov(vec![prop(0x10, Some(gaid(0x60)), Vec::new(), E + 3, 7_000, 1)]);
        g.drep_expiry.insert(key(0xD1), 1400);
        g.drep_expiry.insert(key(0xD2), 1400);
        g.vote_delegations.insert(key(1), DRep::KeyHash(Hash28([0xD1; 28])));
        g.vote_delegations.insert(key(2), DRep::KeyHash(Hash28([0xD1; 28])));
        g.vote_delegations.insert(key(3), DRep::AlwaysAbstain);
        g.vote_delegations.insert(key(4), DRep::KeyHash(Hash28([0xD2; 28])));
        let mut f = Fixture::new(g);
        for b in [1, 2, 3, 4] {
            f.register(b);
        }
        f.utxo.insert(key(1), Coin(100));
        f.deleg.rewards.insert(key(1), Coin(20));
        f.deleg.delegations.insert(key(1), pool(9));
        f.utxo.insert(key(2), Coin(50)); // vote-only delegator: no pool delegation
        f.deleg.rewards.insert(key(3), Coin(5));
        let d = derive_frozen_drep_distribution(&f.input().accounts, &f.input().gov).unwrap();
        assert_eq!(d.get(&DRep::KeyHash(Hash28([0xD1; 28]))), Some(&Coin(100 + 20 + 7_000 + 50)));
        assert_eq!(d.get(&DRep::AlwaysAbstain), Some(&Coin(5)));
        assert_eq!(d.get(&DRep::KeyHash(Hash28([0xD2; 28]))), Some(&Coin(0)), "zero total still appears");
    }

    /// CE-LV15b0-3b: a dangling delegation is a terminal, never excluded — both ends.
    #[test]
    fn ce_lv15b0_3b_dangling_delegation_is_a_terminal_not_excluded() {
        let mut g = gov(Vec::new());
        g.vote_delegations.insert(key(1), DRep::KeyHash(Hash28([0xD1; 28])));
        let mut f = Fixture::new(g.clone());
        f.register(1);
        assert_eq!(
            derive_frozen_drep_distribution(&f.input().accounts, &f.input().gov),
            Err(GovernanceTerminal::DanglingVoteDelegation {
                credential: key(1),
                kind: DanglingVoteDelegationKind::UnregisteredDRep,
            }),
        );
        g.drep_expiry.insert(key(0xD1), 1400);
        let f = Fixture::new(g);
        assert_eq!(
            derive_frozen_drep_distribution(&f.input().accounts, &f.input().gov),
            Err(GovernanceTerminal::DanglingVoteDelegation {
                credential: key(1),
                kind: DanglingVoteDelegationKind::UnregisteredAccount,
            }),
        );
    }

    /// CE-LV15b0-4 (distribution): the SPO distribution is the mark plus the deposits of each pool's delegators,
    /// over every pool registered at SNAP (zero-stake ones included); a deposit whose account delegates to a pool
    /// outside the mark adds nothing.
    #[test]
    fn ce_lv15b0_4_pool_distribution_is_mark_plus_deposits() {
        let g = gov(vec![
            prop(0x10, Some(gaid(0x60)), Vec::new(), E + 3, 7_000, 1),
            prop(0x11, Some(gaid(0x60)), Vec::new(), E + 3, 3_000, 2),
            prop(0x12, Some(gaid(0x60)), Vec::new(), E + 3, 1_000, 3),
        ]);
        let mut f = Fixture::new(g);
        f.mark.pool_stakes.insert(pool(9), Coin(1_000_000));
        f.mark.pool_params.insert(pool(9), spp());
        f.mark.pool_params.insert(pool(8), spp()); // registered, no stake
        f.deleg.delegations.insert(key(1), pool(9));
        f.deleg.delegations.insert(key(2), pool(8));
        f.deleg.delegations.insert(key(3), pool(7)); // not in the mark
        let p = derive_frozen_pool_distribution(&f.input().pool, &f.input().accounts, &f.input().gov).unwrap();
        assert_eq!(p.get(&pool(9)), Some(&Coin(1_007_000)));
        assert_eq!(p.get(&pool(8)), Some(&Coin(3_000)));
        assert_eq!(p.get(&pool(7)), None);
    }

    /// The record is frozen at E, decides over the frozen proposals, and lists the ones past their lifetime at E.
    #[test]
    fn ratify_frozen_decides_at_frozen_epoch() {
        let winner = prop(0x69, Some(gaid(0x60)), yes_2of3(), E + 3, 100, 1);
        let old = prop(0x20, None, Vec::new(), E - 1, 100, 1);
        let f = Fixture::new(gov(vec![winner.clone(), old.clone()]));
        let r = ratify_frozen(&f.input()).unwrap();
        assert_eq!(r.frozen_at(), EpochNo(E));
        assert_eq!(r.source(), RatificationSource::Frozen);
        match r.outcome() {
            RatificationOutcome::Decided(d) => {
                assert_eq!(d.enacted.as_ref().map(|e| &e.action_id), Some(&winner.action_id));
                assert_eq!(d.enacted.as_ref().map(|e| &e.superseded_root), Some(&Some(gaid(0x60))));
                assert_eq!(d.expired.iter().collect::<Vec<_>>(), vec![&old.action_id]);
            }
            other => panic!("expected Decided, got {other:?}"),
        }
    }

    /// CE-LV15b0-1 (pure): votes reaching a proposal after the freeze do not enact it at the consuming boundary;
    /// it stays, with those votes, for the next cycle.
    #[test]
    fn ce_lv15b0_1_late_votes_do_not_enact_at_the_consuming_boundary() {
        let frozen = prop(0x69, Some(gaid(0x60)), Vec::new(), E + 3, 100, 1);
        let f = Fixture::new(gov(vec![frozen.clone()]));
        let record = ratify_frozen(&f.input()).unwrap();
        let mut voted = frozen.clone();
        voted.committee_votes = yes_2of3();
        let current = gov(vec![voted.clone()]);
        let plan =
            apply_ratification_record(&record, EpochNo(E + 1), &CurrentGovernanceState::pre_boundary(&current), |_| true)
                .unwrap();
        assert_eq!(plan.pparams, PParamsDelta::Unchanged);
        assert!(plan.removals.is_empty());
        assert_eq!(plan.proposals, vec![voted], "kept, with its late votes, for the next cycle");
    }

    /// CE-LV15b0-2 (pure): a proposal submitted after the freeze is not in the record and is carried.
    #[test]
    fn ce_lv15b0_2_late_proposal_is_carried() {
        let f = Fixture::new(gov(Vec::new()));
        let record = ratify_frozen(&f.input()).unwrap();
        let late = prop(0x70, Some(gaid(0x60)), yes_2of3(), E + 6, 100, 1);
        let current = gov(vec![late.clone()]);
        let plan =
            apply_ratification_record(&record, EpochNo(E + 1), &CurrentGovernanceState::pre_boundary(&current), |_| true)
                .unwrap();
        assert!(plan.removals.is_empty());
        assert_eq!(plan.proposals, vec![late]);
    }

    /// CE-LV15b0-6 (pure): the enacted winner, a sibling submitted after the freeze and that sibling's child are
    /// removed from the CURRENT set; deposits route by registration at the consuming boundary.
    #[test]
    fn ce_lv15b0_6_application_prunes_late_siblings_and_routes_by_registration_now() {
        let winner = prop(0x69, Some(gaid(0x60)), yes_2of3(), E + 3, 100, 1);
        let f = Fixture::new(gov(vec![winner.clone()]));
        let record = ratify_frozen(&f.input()).unwrap();
        let late_sibling = prop(0x71, Some(gaid(0x60)), Vec::new(), E + 6, 200, 2);
        let late_child = prop(0x72, Some(late_sibling.action_id.clone()), Vec::new(), E + 6, 300, 3);
        let winner_child = prop(0x73, Some(winner.action_id.clone()), Vec::new(), E + 6, 400, 1);
        let current = gov(vec![winner.clone(), late_sibling.clone(), late_child.clone(), winner_child.clone()]);
        let registered_now = |c: &StakeCredential| *c != key(2);
        let plan = apply_ratification_record(
            &record,
            EpochNo(E + 1),
            &CurrentGovernanceState::pre_boundary(&current),
            registered_now,
        )
        .unwrap();
        let cause = |id: &GovActionId| plan.removals.iter().find(|r| r.action_id == *id).map(|r| r.cause);
        assert_eq!(cause(&winner.action_id), Some(RemovalCause::Enacted));
        assert_eq!(cause(&late_sibling.action_id), Some(RemovalCause::PrunedByEnactment));
        assert_eq!(cause(&late_child.action_id), Some(RemovalCause::PrunedByEnactment));
        assert_eq!(plan.proposals, vec![winner_child], "the winner's child is the new lineage");
        assert!(plan.deposit_returns.contains(&DepositReturn::ToTreasury {
            action_id: late_sibling.action_id.clone(),
            amount: Coin(200),
        }));
        assert!(matches!(plan.pparams, PParamsDelta::Set(_)));
    }

    /// CE-LV15b0-6: an `Unenactable` record halts at the consuming boundary, not at the freeze.
    #[test]
    fn ce_lv15b0_6_unenactable_halts_at_the_consuming_boundary() {
        let withdrawal = GovActionState {
            gov_action: GovAction::TreasuryWithdrawals { withdrawals: vec![(reward_addr(1), Coin(5))], policy_hash: None },
            ..prop(0x80, None, yes_2of3(), E + 3, 100, 1)
        };
        let mut g = gov(vec![withdrawal.clone()]);
        g.drep_voting_thresholds = vec![(0, 1); 10];
        let f = Fixture::new(g.clone());
        let record = ratify_frozen(&f.input()).expect("the freeze itself does not halt");
        assert!(matches!(record.outcome(), RatificationOutcome::Unenactable(_)));
        let err = apply_ratification_record(&record, EpochNo(E + 1), &CurrentGovernanceState::pre_boundary(&g), |_| true)
            .unwrap_err();
        assert!(matches!(err, GovernanceTerminal::UnsupportedRatifiedAction { .. }), "{err:?}");
    }

    /// DC-EPOCH-47: a record is consumed only at the boundary ending its epoch.
    #[test]
    fn record_from_another_epoch_is_refused() {
        let f = Fixture::new(gov(Vec::new()));
        let record = ratify_frozen(&f.input()).unwrap();
        let current = gov(Vec::new());
        for boundary in [E, E + 2] {
            assert_eq!(
                apply_ratification_record(
                    &record,
                    EpochNo(boundary),
                    &CurrentGovernanceState::pre_boundary(&current),
                    |_| true
                ),
                Err(GovernanceTerminal::RatificationRecordEpochMismatch {
                    frozen_at: EpochNo(E),
                    boundary: EpochNo(boundary),
                }),
            );
        }
    }

    /// A decision naming a proposal the current set lacks is a terminal.
    #[test]
    fn decided_id_missing_from_current_set_is_a_terminal() {
        let winner = prop(0x69, Some(gaid(0x60)), yes_2of3(), E + 3, 100, 1);
        let f = Fixture::new(gov(vec![winner.clone()]));
        let record = ratify_frozen(&f.input()).unwrap();
        let current = gov(Vec::new());
        assert_eq!(
            apply_ratification_record(&record, EpochNo(E + 1), &CurrentGovernanceState::pre_boundary(&current), |_| true),
            Err(GovernanceTerminal::RatificationRecordProposalMissing { action_id: winner.action_id }),
        );
    }

    fn snap(enacted: Vec<GovActionState>, enacted_pparams: ProtocolParameters, root: Option<GovActionId>) -> SnapshotRatification {
        SnapshotRatification {
            proposals: std::collections::BTreeSet::new(),
            drep_distribution: BTreeMap::new(),
            pool_distribution: BTreeMap::new(),
            enacted,
            expired: [gaid(0x20)].into_iter().collect(),
            delayed: false,
            enacted_pparams,
            enacted_pparam_root: root,
        }
    }

    /// CE-LV15b0-8 (import): a supported enacted `ParameterChange` imports as `Decided`, its parameters built by
    /// Ade's own builder and cross-checked against `ensCurPParams` and the root.
    #[test]
    fn ce_lv15b0_8_import_supported_enactment_is_decided_and_cross_checked() {
        let winner = prop(0x69, Some(gaid(0x60)), yes_2of3(), E + 3, 100, 1);
        let built = super::super::build_parameter_change_enactment(&winner, &pp()).unwrap();
        let r = import_snapshot_ratification(
            EpochNo(E),
            snap(vec![winner.clone()], (*built.pparams).clone(), Some(winner.action_id.clone())),
            &pp(),
            Some(&gaid(0x60)),
        )
        .unwrap();
        assert_eq!(r.source(), RatificationSource::ImportedFromSnapshot);
        match r.outcome() {
            RatificationOutcome::Decided(d) => {
                assert_eq!(d.enacted.as_ref().map(|e| &e.action_id), Some(&winner.action_id));
                assert_eq!(d.expired.iter().collect::<Vec<_>>(), vec![&gaid(0x20)]);
            }
            other => panic!("expected Decided, got {other:?}"),
        }
        // The snapshot's parameters disagree with what Ade builds: refused.
        assert_eq!(
            import_snapshot_ratification(
                EpochNo(E),
                snap(vec![winner.clone()], pp(), Some(winner.action_id.clone())),
                &pp(),
                Some(&gaid(0x60)),
            ),
            Err(RatificationImportRefusal::EnactedParametersMismatch)
        );
    }

    /// CE-LV15b0-8 (import): an enacted action Ade cannot enact imports as `Unenactable` with the terminal Ade's
    /// ratifier gives; nothing is cross-checked, because it halts at the seed boundary.
    #[test]
    fn ce_lv15b0_8_import_unsupported_enactments_are_unenactable() {
        let withdrawal = GovActionState {
            gov_action: GovAction::TreasuryWithdrawals { withdrawals: vec![(reward_addr(1), Coin(5))], policy_hash: None },
            ..prop(0x80, None, Vec::new(), E + 3, 100, 1)
        };
        let r = import_snapshot_ratification(EpochNo(E), snap(vec![withdrawal.clone()], pp(), None), &pp(), None).unwrap();
        assert!(matches!(
            r.outcome(),
            RatificationOutcome::Unenactable(UnenactableReason::UnsupportedRatifiedAction {
                kind: UnsupportedActionKind::NotParameterChange,
                ..
            })
        ));
        let parent = prop(0x69, Some(gaid(0x60)), Vec::new(), E + 3, 100, 1);
        let child = prop(0x6A, Some(parent.action_id.clone()), Vec::new(), E + 3, 100, 1);
        let r = import_snapshot_ratification(EpochNo(E), snap(vec![parent, child.clone()], pp(), None), &pp(), None)
            .unwrap();
        assert_eq!(
            r.outcome(),
            &RatificationOutcome::Unenactable(UnenactableReason::UnsupportedRatifiedAction {
                action_id: child.action_id,
                kind: UnsupportedActionKind::ChainedEnactment,
                unsupported_keys: CanonicalFieldSet::default(),
            })
        );
    }

    #[test]
    fn ratify_frozen_is_deterministic() {
        let f = Fixture::new(gov(vec![prop(0x69, Some(gaid(0x60)), yes_2of3(), E + 3, 100, 1)]));
        assert_eq!(ratify_frozen(&f.input()).unwrap(), ratify_frozen(&f.input()).unwrap());
    }
}
