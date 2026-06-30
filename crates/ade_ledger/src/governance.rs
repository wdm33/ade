// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Conway governance: ratification, enactment, and expiry.
//!
//! This module implements the Conway-era governance epoch boundary logic:
//! 1. DRep stake distribution (computed from vote delegations + stake snapshot)
//! 2. Proposal ratification (evaluate votes against thresholds)
//! 3. Proposal enactment (apply ratified proposals)
//! 4. Proposal and DRep expiry
//!
//! All functions are pure and deterministic. No I/O.
//!
//! Reference: CIP-1694, Haskell cardano-ledger Conway.Epoch rules.

use ade_types::conway::cert::DRep;
use ade_types::conway::governance::{GovAction, GovActionState, Vote};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::Coin;
use ade_types::Hash28;
use crate::rational::Rational;

use std::collections::BTreeMap;

/// DRep stake distribution: maps each DRep to its total delegated voting stake.
pub type DRepStakeDistribution = BTreeMap<DRep, u64>;

/// Result of ratification evaluation for a single proposal.
#[derive(Debug, Clone)]
pub struct RatificationResult {
    /// Proposals that met their ratification thresholds.
    pub ratified: Vec<GovActionState>,
    /// Proposals that expired without ratification.
    pub expired: Vec<GovActionState>,
    /// Proposals still active (not ratified, not expired).
    pub remaining: Vec<GovActionState>,
}

/// Active DRep stake (denominator for the DRep ratification gate): excludes `AlwaysAbstain` and DReps
/// whose term has expired (`drep_expiry < current_epoch`; absent ⇒ assumed active). Returns the filtered
/// distribution and its total. Shared by [`evaluate_ratification`] and the S4.0 ratification census
/// observer so both read the SAME denominator — there is one filter, not two.
pub(crate) fn active_drep_stake_filtered(
    drep_stake: &DRepStakeDistribution,
    drep_expiry: &BTreeMap<StakeCredential, u64>,
    current_epoch: u64,
) -> (DRepStakeDistribution, u64) {
    let active: DRepStakeDistribution = drep_stake
        .iter()
        .filter(|(drep, _)| match drep {
            DRep::AlwaysAbstain => false,
            // A DRep's key/script discriminant maps to the matching credential variant — the
            // drep_expiry map is keyed by the discriminated credential.
            DRep::KeyHash(h) => drep_expiry
                .get(&StakeCredential::KeyHash(h.clone()))
                .map(|e| *e >= current_epoch)
                .unwrap_or(true),
            DRep::ScriptHash(h) => drep_expiry
                .get(&StakeCredential::ScriptHash(h.clone()))
                .map(|e| *e >= current_epoch)
                .unwrap_or(true),
            _ => true,
        })
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    let total = active.values().sum::<u64>();
    (active, total)
}

/// Evaluate ratification for all proposals.
///
/// For each proposal, checks whether DRep votes, committee votes, and SPO votes
/// meet the per-action-type thresholds from protocol parameters.
///
/// The `current_epoch` is used only for expiry checks.
/// `gov_action_lifetime` is the number of epochs a proposal lives before expiring.
///
/// Ratification order: proposals evaluated in `GovActionId` order (deterministic).
pub fn evaluate_ratification(
    proposals: &[GovActionState],
    drep_stake: &DRepStakeDistribution,
    pool_stake: &BTreeMap<ade_types::tx::PoolId, Coin>,
    committee_members: &BTreeMap<StakeCredential, u64>, // cold credential → expiry epoch
    committee_quorum: &Rational,
    pool_thresholds: &[(u64, u64)],   // per-action-type pool voting thresholds
    drep_thresholds: &[(u64, u64)],   // per-action-type DRep voting thresholds
    current_epoch: u64,
    committee_hot_keys: &BTreeMap<StakeCredential, StakeCredential>, // hot → cold mapping
    drep_expiry: &BTreeMap<StakeCredential, u64>, // DRep credential → expiry epoch
) -> RatificationResult {
    // Active DRep stake (exclude AlwaysAbstain + expired DReps) — shared with the S4.0 census observer.
    let (active_drep_stake, total_drep_active_stake) =
        active_drep_stake_filtered(drep_stake, drep_expiry, current_epoch);
    let total_pool_stake: u64 = pool_stake.values().map(|c| c.0).sum();

    let mut ratified = Vec::new();
    let mut expired = Vec::new();
    let mut remaining = Vec::new();

    for proposal in proposals {
        // Check expiry first
        if proposal.expires_after.0 < current_epoch {
            expired.push(proposal.clone());
            continue;
        }

        let is_ratified = match &proposal.gov_action {
            GovAction::InfoAction => {
                // InfoAction is always "ratified" but has no enactment effect.
                // It stays in the proposal list until natural expiry.
                remaining.push(proposal.clone());
                continue;
            }
            action => {
                let action_idx = gov_action_threshold_index(action);
                check_ratification(
                    proposal,
                    action_idx,
                    &total_drep_active_stake,
                    &active_drep_stake,
                    total_pool_stake,
                    pool_stake,
                    committee_members,
                    committee_quorum,
                    pool_thresholds,
                    drep_thresholds,
                    current_epoch,
                    committee_hot_keys,
                )
            }
        };

        if is_ratified {
            ratified.push(proposal.clone());
        } else {
            remaining.push(proposal.clone());
        }
    }

    RatificationResult { ratified, expired, remaining }
}

/// Compute total active DRep stake (excluding AlwaysAbstain).
/// This is the denominator for DRep threshold checks (Haskell: dRepAcceptedRatio).
///
/// The Haskell DRepPulser computes this from live InstantStake (post-applyRUpd).
/// Our approximation uses the most recent snapshot. Known gap: ~400M ADA at
/// epoch 576 (7% of total), causing 1 of 2 oracle-enacted TreasuryWithdrawals
/// to not meet our threshold. Closing requires a Conway mid-epoch dump with
/// the DRepPulser's computed distribution.
fn compute_active_drep_stake(drep_stake: &DRepStakeDistribution) -> u64 {
    drep_stake.iter()
        .filter(|(drep, _)| !matches!(drep, DRep::AlwaysAbstain))
        .map(|(_, stake)| *stake)
        .sum()
}

/// Map a governance action to its threshold index in the poolVotingThresholds
/// and dRepVotingThresholds arrays.
///
/// CIP-1694 threshold ordering:
///   Pool thresholds (5): [motionNoConfidence, committeeNormal, committeeNoConfidence,
///                         hardForkInitiation, ppSecurityGroup]
///   DRep thresholds (10): [motionNoConfidence, committeeNormal, committeeNoConfidence,
///                          updateConstitution, hardForkInitiation, ppNetworkGroup,
///                          ppEconomicGroup, ppTechnicalGroup, ppGovernanceGroup,
///                          treasuryWithdrawal]
fn gov_action_threshold_index(action: &GovAction) -> (Option<usize>, Option<usize>) {
    match action {
        GovAction::NoConfidence { .. } => (Some(0), Some(0)),
        GovAction::UpdateCommittee { .. } => (Some(1), Some(1)), // normal case
        GovAction::NewConstitution { .. } => (None, Some(3)),
        GovAction::HardForkInitiation { .. } => (Some(3), Some(4)),
        GovAction::ParameterChange { .. } => (Some(4), Some(5)), // network group default
        GovAction::TreasuryWithdrawals { .. } => (None, Some(9)),
        GovAction::InfoAction => (None, None),
    }
}

/// Check if a proposal meets all required ratification thresholds.
#[allow(clippy::too_many_arguments)]
fn check_ratification(
    proposal: &GovActionState,
    action_thresholds: (Option<usize>, Option<usize>),
    total_drep_active_stake: &u64,
    drep_stake: &DRepStakeDistribution,
    _total_pool_stake: u64,
    pool_stake: &BTreeMap<ade_types::tx::PoolId, Coin>,
    committee_members: &BTreeMap<StakeCredential, u64>,
    committee_quorum: &Rational,
    pool_thresholds: &[(u64, u64)],
    drep_thresholds: &[(u64, u64)],
    current_epoch: u64,
    committee_hot_keys: &BTreeMap<StakeCredential, StakeCredential>,
) -> bool {
    let (pool_idx, drep_idx) = action_thresholds;

    // Committee check: if the action requires committee approval
    let needs_committee = !matches!(
        proposal.gov_action,
        GovAction::NoConfidence { .. } | GovAction::UpdateCommittee { .. }
    );
    if needs_committee && !committee_members.is_empty() {
        let active_members: Vec<_> = committee_members.iter()
            .filter(|(_, expiry)| **expiry >= current_epoch)
            .collect();
        if !active_members.is_empty() {
            // Committee votes use HOT credentials. Resolve via hot→cold mapping.
            let yes_votes = proposal.committee_votes.iter()
                .filter(|(hot_cred, vote)| {
                    if !matches!(vote, Vote::Yes) { return false; }
                    // Resolve hot→cold. If mapping exists, check cold is active member.
                    // If no mapping, fall back to counting all Yes votes.
                    // Hot voter, hot→cold mapping, and cold member are all
                    // discriminated credentials; resolution is full-credential
                    // equality so a key-hash hot key never cross-resolves to a
                    // script-hash member of equal bytes.
                    if let Some(cold) = committee_hot_keys
                        .iter()
                        .find(|(hot, _)| *hot == hot_cred)
                        .map(|(_, cold)| cold)
                    {
                        active_members.iter().any(|(c, _)| **c == *cold)
                    } else {
                        // No hot key mapping — count vote if we have enough votes
                        // (fallback for when VState parsing doesn't cover all keys)
                        true
                    }
                })
                .count();
            let yes_rat = Rational::new(yes_votes as i128, active_members.len() as i128)
                .unwrap_or_else(Rational::zero);
            if yes_rat.numerator() * committee_quorum.denominator()
                < committee_quorum.numerator() * yes_rat.denominator()
            {
                return false;
            }
        }
    }

    // DRep check (Haskell: dRepAcceptedRatio):
    // ratio = yes_stake / (total_active_stake - abstain_stake - inactive_stake)
    // Non-voting DReps count against ratification (stay in denominator).
    // Only AlwaysAbstain and inactive DReps are excluded.
    if let Some(idx) = drep_idx {
        if idx < drep_thresholds.len() && *total_drep_active_stake > 0 {
            let (thresh_num, thresh_den) = drep_thresholds[idx];
            if thresh_den > 0 {
                // DRep-voter discriminant fidelity: the voter credential carries
                // its key/script tag, so it resolves to EXACTLY one DRep stake
                // key — never a key/script OR-fallback that would let a key-hash
                // voter tally a script-hash DRep's stake of equal bytes.
                let lookup_stake = |cred: &StakeCredential| -> u64 {
                    let drep = match cred {
                        StakeCredential::KeyHash(h) => DRep::KeyHash(h.clone()),
                        StakeCredential::ScriptHash(h) => DRep::ScriptHash(h.clone()),
                    };
                    drep_stake.get(&drep).copied().unwrap_or(0)
                };
                let yes_stake: u64 = proposal.drep_votes.iter()
                    .filter(|(_, vote)| matches!(vote, Vote::Yes))
                    .map(|(cred, _)| lookup_stake(cred))
                    .sum();
                // Denominator = total active DRep stake (already excludes AlwaysAbstain)
                let yes_128 = yes_stake as u128;
                let td_128 = thresh_den as u128;
                let tn_128 = thresh_num as u128;
                let total_128 = *total_drep_active_stake as u128;
                if yes_128 * td_128 < tn_128 * total_128 {
                    return false;
                }
            }
        }
    }

    // SPO check: same yes/(yes+no) semantics as DRep
    if let Some(idx) = pool_idx {
        if idx < pool_thresholds.len() {
            let (thresh_num, thresh_den) = pool_thresholds[idx];
            if thresh_den > 0 {
                let lookup_pool = |hash: &Hash28| -> u64 {
                    pool_stake.get(&ade_types::tx::PoolId(hash.clone()))
                        .map(|c| c.0)
                        .unwrap_or(0)
                };
                let yes_stake: u64 = proposal.spo_votes.iter()
                    .filter(|(_, vote)| matches!(vote, Vote::Yes))
                    .map(|(hash, _)| lookup_pool(hash))
                    .sum();
                let no_stake: u64 = proposal.spo_votes.iter()
                    .filter(|(_, vote)| matches!(vote, Vote::No))
                    .map(|(hash, _)| lookup_pool(hash))
                    .sum();
                let voted_stake = yes_stake + no_stake;
                if voted_stake > 0 {
                    let yes_128 = yes_stake as u128;
                    let td_128 = thresh_den as u128;
                    let tn_128 = thresh_num as u128;
                    let voted_128 = voted_stake as u128;
                    if yes_128 * td_128 < tn_128 * voted_128 {
                        return false;
                    }
                }
                // If no SPO votes cast, SPO check passes (no quorum required)
            }
        }
    }

    true
}

// ─── Ratification census (observe-only, CONWAY-PROPOSAL-DEPOSIT-EXPIRY S4.0) ───

/// A per-proposal observation of the REAL ratification path at `current_epoch` — observe-only, on NO
/// mutation or runtime authority path. The S4.0 negative-proof census reads it to decide whether Ade's
/// CURRENT (committee-only) authority can resolve the WHOLE tracked proposal set, or whether a threshold /
/// DRep-stake import gap must close before the S4 boundary refund evaluator exists.
///
/// `potentially_ratifiable` is the EXACT outcome of [`check_ratification`] (the same gates
/// [`evaluate_ratification`] runs), evaluated WITHOUT the expiry short-circuit (Conway: ratification
/// precedes expiry). The trace fields are INPUT-presence inspection that EXPLAINS the outcome — they do
/// not re-derive the ratification decision.
#[derive(Debug, Clone)]
pub struct RatificationObservation {
    /// `false` ⟺ a PRESENT gate definitively failed ⟹ provably unratifiable (the sound negative proof);
    /// `true` ⟺ every required gate passed OR was skipped for absent inputs ⟹ potentially ratifiable
    /// (boundary-terminal). `InfoAction` never enacts ⟹ `false`.
    pub potentially_ratifiable: bool,
    /// `InfoAction` — no enactment effect (handled exactly as `evaluate_ratification` special-cases it).
    pub is_info_action: bool,
    /// The action requires constitutional-committee approval (everything except NoConfidence /
    /// UpdateCommittee).
    pub requires_committee: bool,
    /// Imported constitutional-committee size.
    pub committee_size: usize,
    /// Committee members ACTIVE at `current_epoch` (`expiry >= current_epoch`). If 0 while
    /// `requires_committee`, the committee gate SKIPS and the proof would rest on other (possibly absent)
    /// gates — the decisive activity check for the census.
    pub committee_active_members: usize,
    /// RAW count of `Vote::Yes` committee votes recorded on this proposal — NOT the gate's effective
    /// tally (the committee gate resolves hot→cold before counting). Annotation only; never consumed by
    /// `potentially_ratifiable`.
    pub committee_yes: usize,
    /// DRep voting-threshold index for the action (`None` = no DRep gate).
    pub drep_threshold_index: Option<usize>,
    /// The DRep gate's inputs are present (threshold imported AND active DRep stake > 0).
    pub drep_inputs_present: bool,
    /// SPO voting-threshold index for the action (`None` = no SPO gate).
    pub pool_threshold_index: Option<usize>,
    /// The SPO gate's inputs are present (threshold imported with a non-zero denominator).
    pub spo_inputs_present: bool,
}

/// Observe (do NOT mutate) one proposal's ratification disposition at `current_epoch` — see
/// [`RatificationObservation`]. Exercises the real [`check_ratification`]; the S4.0 census's only entry
/// point into the ratification authority.
#[allow(clippy::too_many_arguments)]
pub fn proposal_ratification_observation(
    proposal: &GovActionState,
    drep_stake: &DRepStakeDistribution,
    pool_stake: &BTreeMap<ade_types::tx::PoolId, Coin>,
    committee_members: &BTreeMap<StakeCredential, u64>,
    committee_quorum: &Rational,
    pool_thresholds: &[(u64, u64)],
    drep_thresholds: &[(u64, u64)],
    current_epoch: u64,
    committee_hot_keys: &BTreeMap<StakeCredential, StakeCredential>,
    drep_expiry: &BTreeMap<StakeCredential, u64>,
) -> RatificationObservation {
    let is_info_action = matches!(proposal.gov_action, GovAction::InfoAction);
    let requires_committee = !matches!(
        proposal.gov_action,
        GovAction::NoConfidence { .. } | GovAction::UpdateCommittee { .. }
    );
    let (active_drep_stake, total_drep_active_stake) =
        active_drep_stake_filtered(drep_stake, drep_expiry, current_epoch);
    let total_pool_stake: u64 = pool_stake.values().map(|c| c.0).sum();
    let (pool_idx, drep_idx) = gov_action_threshold_index(&proposal.gov_action);

    let committee_active_members = committee_members
        .iter()
        .filter(|(_, expiry)| **expiry >= current_epoch)
        .count();
    let committee_yes = proposal
        .committee_votes
        .iter()
        .filter(|(_, vote)| matches!(vote, Vote::Yes))
        .count();
    let drep_inputs_present =
        drep_idx.map_or(false, |i| i < drep_thresholds.len()) && total_drep_active_stake > 0;
    let spo_inputs_present =
        pool_idx.map_or(false, |i| i < pool_thresholds.len() && pool_thresholds[i].1 > 0);

    // InfoAction never enacts (mirrors evaluate_ratification's special-case); else the REAL gate outcome.
    let potentially_ratifiable = if is_info_action {
        false
    } else {
        check_ratification(
            proposal,
            (pool_idx, drep_idx),
            &total_drep_active_stake,
            &active_drep_stake,
            total_pool_stake,
            pool_stake,
            committee_members,
            committee_quorum,
            pool_thresholds,
            drep_thresholds,
            current_epoch,
            committee_hot_keys,
        )
    };

    RatificationObservation {
        potentially_ratifiable,
        is_info_action,
        requires_committee,
        committee_size: committee_members.len(),
        committee_active_members,
        committee_yes,
        drep_threshold_index: drep_idx,
        drep_inputs_present,
        pool_threshold_index: pool_idx,
        spo_inputs_present,
    }
}

// ─── Enactment ───────────────────────────────────────────────────────

/// Priority class for enactment ordering.
/// Ratified proposals are enacted in this order (highest priority first).
/// Within each class, proposals are enacted in GovActionId order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EnactmentPriority {
    HardForkInitiation = 0,
    UpdateCommitteeOrNoConfidence = 1,
    NewConstitution = 2,
    ParameterChange = 3,
    TreasuryWithdrawals = 4,
    InfoAction = 5,
}

fn enactment_priority(action: &GovAction) -> EnactmentPriority {
    match action {
        GovAction::HardForkInitiation { .. } => EnactmentPriority::HardForkInitiation,
        GovAction::UpdateCommittee { .. } | GovAction::NoConfidence { .. } =>
            EnactmentPriority::UpdateCommitteeOrNoConfidence,
        GovAction::NewConstitution { .. } => EnactmentPriority::NewConstitution,
        GovAction::ParameterChange { .. } => EnactmentPriority::ParameterChange,
        GovAction::TreasuryWithdrawals { .. } => EnactmentPriority::TreasuryWithdrawals,
        GovAction::InfoAction => EnactmentPriority::InfoAction,
    }
}

/// The effects of enacting ratified governance proposals.
#[derive(Debug, Clone, Default)]
pub struct EnactmentEffects {
    /// Treasury withdrawals to execute: (reward_account, amount).
    pub treasury_withdrawals: Vec<(Vec<u8>, Coin)>,
    /// Total ADA withdrawn from treasury.
    pub treasury_withdrawn: u64,
    /// Protocol parameter update (raw CBOR, applied later).
    pub parameter_updates: Vec<Vec<u8>>,
    /// Hard fork initiation: target protocol version.
    pub hard_fork: Option<(u64, u64)>,
    /// Committee dissolved (NoConfidence enacted).
    pub committee_dissolved: bool,
    /// Committee changes: (removed, added with expiry). Discriminated
    /// `StakeCredential` (cold committee credentials) — never bare `Hash28` — so
    /// when `UpdateCommittee` enactment is implemented it cannot re-collapse the
    /// discriminated `ConwayGovState.committee` map on write-back (DC-LEDGER-10,
    /// ENACTMENT-COMMITTEE-FIDELITY). Currently dormant (always `None`).
    pub committee_changes: Option<(Vec<StakeCredential>, Vec<(StakeCredential, u64)>)>,
    /// New committee quorum threshold (numerator, denominator) set by a ratified
    /// `UpdateCommittee`. Applied to `ConwayGovState.committee_quorum` on
    /// write-back; `None` leaves the quorum unchanged.
    pub committee_threshold: Option<(u64, u64)>,
    /// Constitution replaced (raw CBOR).
    pub new_constitution: Option<Vec<u8>>,
    /// Number of InfoActions enacted (no state effect).
    pub info_actions: u32,
    /// Deposits returned to proposers for enacted proposals.
    pub deposits_returned: Vec<(Vec<u8>, Coin)>,
}

/// Enact ratified proposals in priority-class order.
///
/// Within each priority class, proposals are enacted in GovActionId order.
/// Each enactment produces effects that modify the ledger state.
///
/// Conway spec: enactment is atomic at the epoch boundary. All ratified
/// proposals are enacted before any state is committed.
pub fn enact_proposals(
    ratified: &[GovActionState],
) -> EnactmentEffects {
    let mut effects = EnactmentEffects::default();

    // Sort by (priority_class, GovActionId) for deterministic ordering
    let mut sorted: Vec<&GovActionState> = ratified.iter().collect();
    sorted.sort_by(|a, b| {
        let pa = enactment_priority(&a.gov_action);
        let pb = enactment_priority(&b.gov_action);
        pa.cmp(&pb).then(a.action_id.cmp(&b.action_id))
    });

    for proposal in &sorted {
        match &proposal.gov_action {
            GovAction::InfoAction => {
                effects.info_actions += 1;
            }
            GovAction::TreasuryWithdrawals { withdrawals, .. } => {
                for (addr, amount) in withdrawals {
                    effects.treasury_withdrawals.push((addr.clone(), *amount));
                    effects.treasury_withdrawn += amount.0;
                }
            }
            GovAction::ParameterChange { update, .. } => {
                if !update.is_empty() {
                    effects.parameter_updates.push(update.clone());
                }
            }
            GovAction::HardForkInitiation { protocol_version, .. } => {
                effects.hard_fork = Some(*protocol_version);
            }
            GovAction::NoConfidence { .. } => {
                effects.committee_dissolved = true;
            }
            GovAction::UpdateCommittee { removed, added, threshold, .. } => {
                // Removed + added members and the new quorum threshold become the
                // committee write-back effect, applied at the epoch boundary
                // (rules.rs). BTreeSet/BTreeMap iteration is sorted, so the Vecs
                // are deterministic. Cold credentials stay discriminated
                // (DC-LEDGER-10). If more than one UpdateCommittee ratified
                // (prevented in practice by prev-action lineage), the last in the
                // deterministic sort order wins.
                effects.committee_changes = Some((
                    removed.iter().cloned().collect(),
                    added.iter().map(|(c, e)| (c.clone(), *e)).collect(),
                ));
                effects.committee_threshold = Some(*threshold);
            }
            GovAction::NewConstitution { raw, .. } => {
                effects.new_constitution = Some(raw.clone());
            }
        }

        // Return deposit to proposer
        effects.deposits_returned.push((
            proposal.return_addr.clone(),
            proposal.deposit,
        ));
    }

    effects
}

/// Apply the committee-changing enactment effects to the committee map and
/// quorum, producing the next-epoch committee state. Pure, total, deterministic
/// (BLUE): the sole authority for committee write-back at the epoch boundary.
///
/// - `NoConfidence` (`committee_dissolved`) clears the committee.
/// - `UpdateCommittee` (`committee_changes`) removes the removed cold
///   credentials, then inserts the added ones with their term-expiry epoch.
/// - `committee_threshold` sets the new quorum; `None` leaves it unchanged.
///
/// Dissolve is applied before the add/remove so that members from a (non-spec)
/// co-ratified `UpdateCommittee` still take effect. Cold credentials stay
/// discriminated `StakeCredential` (DC-LEDGER-10) — the map cannot re-collapse.
pub fn apply_committee_enactment(
    committee: &BTreeMap<StakeCredential, u64>,
    quorum: (u64, u64),
    effects: &EnactmentEffects,
) -> (BTreeMap<StakeCredential, u64>, (u64, u64)) {
    let mut next = committee.clone();
    let mut next_quorum = quorum;
    if effects.committee_dissolved {
        next.clear();
    }
    if let Some((removed, added)) = &effects.committee_changes {
        for cred in removed {
            next.remove(cred);
        }
        for (cred, expiry) in added {
            next.insert(cred.clone(), *expiry);
        }
    }
    if let Some(threshold) = effects.committee_threshold {
        next_quorum = threshold;
    }
    (next, next_quorum)
}

// ─── Expiry ──────────────────────────────────────────────────────────

/// Remove expired proposals from the governance state.
///
/// A proposal expires if `expires_after < current_epoch`.
/// Returns (active_proposals, expired_proposals).
pub fn expire_proposals(
    proposals: &[GovActionState],
    current_epoch: u64,
) -> (Vec<GovActionState>, Vec<GovActionState>) {
    let mut active = Vec::new();
    let mut expired = Vec::new();

    for p in proposals {
        if p.expires_after.0 < current_epoch {
            expired.push(p.clone());
        } else {
            active.push(p.clone());
        }
    }

    (active, expired)
}

/// Mark inactive DReps: those whose last activity was more than
/// `drep_activity` epochs ago. Inactive DReps are excluded from
/// the ratification quorum denominator.
///
/// Returns the set of active DRep credential hashes.
pub fn compute_active_dreps(
    drep_last_activity: &BTreeMap<Hash28, u64>, // credential → last active epoch
    current_epoch: u64,
    drep_activity_period: u64,
) -> std::collections::BTreeSet<Hash28> {
    drep_last_activity.iter()
        .filter(|(_, last_active)| {
            current_epoch.saturating_sub(**last_active) <= drep_activity_period
        })
        .map(|(cred, _)| cred.clone())
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod committee_fidelity_tests {
    use super::*;
    use ade_types::conway::governance::{GovAction, GovActionId, GovActionState, Vote};
    use ade_types::shelley::cert::StakeCredential;
    use ade_types::{EpochNo, Hash28, Hash32};

    fn key(b: u8) -> StakeCredential {
        StakeCredential::KeyHash(Hash28([b; 28]))
    }
    fn script(b: u8) -> StakeCredential {
        StakeCredential::ScriptHash(Hash28([b; 28]))
    }

    /// A ParameterChange proposal with one committee Yes vote from hot KeyHash(X),
    /// hot->cold mapping KeyHash(X)->KeyHash(C). DRep/pool checks are skipped
    /// (no thresholds), so the committee gate is the sole determinant.
    fn proposal_with_committee_yes() -> GovActionState {
        GovActionState {
            action_id: GovActionId { tx_hash: Hash32([0u8; 32]), index: 0 },
            committee_votes: vec![(key(0), Vote::Yes)], // placeholder, replaced in `ratifies`
            drep_votes: Vec::new(),
            spo_votes: Vec::new(),
            deposit: Coin(0),
            return_addr: Vec::new(),
            gov_action: GovAction::ParameterChange {
                prev_action: None,
                update: Vec::new(),
                policy_hash: None,
            },
            proposed_in: EpochNo(0),
            expires_after: EpochNo(100),
        }
    }

    fn ratifies(committee_members: &BTreeMap<StakeCredential, u64>) -> bool {
        let mut p = proposal_with_committee_yes();
        p.committee_votes = vec![(key(0x11), Vote::Yes)]; // hot voter KeyHash(0x11)
        let hot_keys: BTreeMap<StakeCredential, StakeCredential> =
            [(key(0x11), key(0xCC))].into_iter().collect(); // hot KeyHash(0x11) -> cold KeyHash(0xCC)
        let quorum = Rational::new(1, 1).unwrap();
        let empty_drep: DRepStakeDistribution = BTreeMap::new();
        let empty_pool: BTreeMap<ade_types::tx::PoolId, Coin> = BTreeMap::new();
        check_ratification(
            &p,
            (None, None), // pool_idx / drep_idx absent -> those checks skipped
            &0,
            &empty_drep,
            0,
            &empty_pool,
            committee_members,
            &quorum,
            &[],
            &[],
            0,
            &hot_keys,
        )
    }

    /// CE-2 (no cross-resolve): the resolved cold credential KeyHash(0xCC) must
    /// NOT match a ScriptHash(0xCC) committee member of equal bytes — the vote
    /// does not count, committee quorum fails, ratification is denied.
    #[test]
    fn committee_keyhash_scripthash_do_not_cross_resolve() {
        let cross: BTreeMap<StakeCredential, u64> =
            [(script(0xCC), 1000u64)].into_iter().collect(); // member is ScriptHash, hot resolves to KeyHash
        assert!(!ratifies(&cross), "key-hash cold must not cross-resolve to a script-hash member of equal bytes");

        // Positive control: a KeyHash(0xCC) member of the same bytes DOES match.
        let matching: BTreeMap<StakeCredential, u64> =
            [(key(0xCC), 1000u64)].into_iter().collect();
        assert!(ratifies(&matching), "matching-variant member ratifies (discriminant is the only difference)");
    }

    /// A ParameterChange proposal with one DRep Yes vote from KeyHash(0x11);
    /// committee empty (skipped), pool absent. The DRep gate (need 50% of the
    /// 1000 active stake) is the sole determinant.
    fn ratifies_drep(drep_stake: &DRepStakeDistribution) -> bool {
        let mut p = proposal_with_committee_yes();
        p.committee_votes = Vec::new();
        p.drep_votes = vec![(key(0x11), Vote::Yes)];
        let no_committee: BTreeMap<StakeCredential, u64> = BTreeMap::new();
        let no_hot: BTreeMap<StakeCredential, StakeCredential> = BTreeMap::new();
        let no_pool: BTreeMap<ade_types::tx::PoolId, Coin> = BTreeMap::new();
        let quorum = Rational::new(1, 1).unwrap();
        check_ratification(
            &p,
            (None, Some(0)), // drep_idx = 0; pool_idx absent
            &1000,           // total_drep_active_stake
            drep_stake,
            0,
            &no_pool,
            &no_committee, // committee empty -> committee gate skipped
            &quorum,
            &[],          // pool_thresholds (unused, pool_idx None)
            &[(1, 2)],    // drep_thresholds[0] = 50%
            0,
            &no_hot,
        )
    }

    /// CE-2 (no cross-resolve): a key-hash DRep voter (resolving to DRep::KeyHash)
    /// must NOT tally a ScriptHash DRep's stake of equal bytes — yes-stake is 0,
    /// the DRep threshold fails, ratification is denied.
    #[test]
    fn drep_keyhash_scripthash_do_not_cross_resolve() {
        let cross: DRepStakeDistribution =
            [(DRep::ScriptHash(Hash28([0x11; 28])), 1000u64)].into_iter().collect();
        assert!(!ratifies_drep(&cross), "key-hash DRep voter must not tally a script-hash DRep's stake of equal bytes");

        // Positive control: the matching KeyHash(0x11) DRep holds the stake.
        let matching: DRepStakeDistribution =
            [(DRep::KeyHash(Hash28([0x11; 28])), 1000u64)].into_iter().collect();
        assert!(ratifies_drep(&matching), "matching-variant DRep stake ratifies (discriminant is the only difference)");
    }

    /// CPDE-S4.0: the shared-preamble extraction is MEANING-PRESERVING and the census observer is
    /// OBSERVATIONAL-ONLY. For representative committee-fail / committee-pass / empty-gate / InfoAction
    /// cases, the REAL `evaluate_ratification` (now routing through `active_drep_stake_filtered`) yields the
    /// expected classification, and `proposal_ratification_observation` AGREES with it — proving the
    /// observer reads the same outcome the authority path produces, never a second implementation.
    #[test]
    fn s4_0_extraction_preserves_outcomes_and_observer_agrees() {
        let quorum = Rational::new(2, 3).unwrap();
        let empty_drep: DRepStakeDistribution = BTreeMap::new();
        let empty_pool: BTreeMap<ade_types::tx::PoolId, Coin> = BTreeMap::new();
        let empty_hot: BTreeMap<StakeCredential, StakeCredential> = BTreeMap::new();
        let empty_drep_expiry: BTreeMap<StakeCredential, u64> = BTreeMap::new();
        // 3 active committee members (term expiry 100 >= the epoch-0 evaluation).
        let committee: BTreeMap<StakeCredential, u64> =
            [(key(0xC1), 100u64), (key(0xC2), 100), (key(0xC3), 100)].into_iter().collect();

        let tw = |id: u8, votes: Vec<(StakeCredential, Vote)>| GovActionState {
            action_id: GovActionId { tx_hash: Hash32([id; 32]), index: 0 },
            committee_votes: votes,
            drep_votes: Vec::new(),
            spo_votes: Vec::new(),
            deposit: Coin(100_000_000_000),
            return_addr: vec![0xe0; 29],
            gov_action: GovAction::TreasuryWithdrawals { withdrawals: Vec::new(), policy_hash: None },
            proposed_in: EpochNo(0),
            expires_after: EpochNo(100), // non-expiring at epoch 0 (so no expiry short-circuit)
        };

        // Run the REAL evaluate_ratification on [p] + the observer; return (evaluate→ratified?, observed).
        let run = |p: &GovActionState, cm: &BTreeMap<StakeCredential, u64>| -> (bool, bool) {
            let res = evaluate_ratification(
                std::slice::from_ref(p), &empty_drep, &empty_pool, cm, &quorum,
                &[], &[], 0, &empty_hot, &empty_drep_expiry,
            );
            let ratified = res.ratified.iter().any(|q| q.action_id == p.action_id);
            let obs = proposal_ratification_observation(
                p, &empty_drep, &empty_pool, cm, &quorum,
                &[], &[], 0, &empty_hot, &empty_drep_expiry,
            );
            (ratified, obs.potentially_ratifiable)
        };

        // (1) committee-fail: 0 committee Yes, active committee, quorum 2/3 -> a PRESENT gate fails.
        let (r, o) = run(&tw(0x01, Vec::new()), &committee);
        assert!(!r && !o, "committee-fail: not ratified; observer agrees (provably unratifiable)");

        // (2) committee-pass: 2 of 3 Yes (= 2/3 >= quorum; no hot map -> Yes counted) -> passes.
        let (r, o) = run(&tw(0x02, vec![(key(0xC1), Vote::Yes), (key(0xC2), Vote::Yes)]), &committee);
        assert!(r && o, "committee-pass: 2/3 Yes ratifies; observer agrees (potentially ratifiable)");

        // (3) empty-gate: TW with EMPTY committee + empty thresholds -> every required gate skipped.
        let no_committee: BTreeMap<StakeCredential, u64> = BTreeMap::new();
        let (r, o) = run(&tw(0x03, Vec::new()), &no_committee);
        assert!(r && o, "empty-gate: required gates skipped -> passed-or-skipped; observer agrees (the danger category)");

        // (4) InfoAction: never enacts -> evaluate_ratification -> remaining (never ratified); observer not
        //     ratifiable + flagged is_info.
        let mut info = tw(0x04, Vec::new());
        info.gov_action = GovAction::InfoAction;
        let res = evaluate_ratification(
            std::slice::from_ref(&info), &empty_drep, &empty_pool, &committee, &quorum,
            &[], &[], 0, &empty_hot, &empty_drep_expiry,
        );
        assert!(
            res.ratified.is_empty() && res.remaining.iter().any(|q| q.action_id == info.action_id),
            "InfoAction -> remaining, never ratified",
        );
        let obs = proposal_ratification_observation(
            &info, &empty_drep, &empty_pool, &committee, &quorum,
            &[], &[], 0, &empty_hot, &empty_drep_expiry,
        );
        assert!(!obs.potentially_ratifiable && obs.is_info_action, "InfoAction never enacts (observer)");
    }

    /// ENACTMENT-COMMITTEE-FIDELITY CE-2: the `EnactmentEffects.committee_changes`
    /// type holds discriminated committee credentials — a key-hash and a
    /// script-hash member of equal bytes are distinct entries (the field cannot
    /// re-collapse the committee map when enactment is wired). The field stays
    /// dormant (`None`) by default; this pins the type, not live behavior.
    #[test]
    fn enactment_committee_changes_keyhash_scripthash_distinct() {
        let removed = vec![key(0xC0), script(0xC0)];
        let added = vec![(key(0xC1), 580u64), (script(0xC1), 580u64)];
        let effects = EnactmentEffects {
            committee_changes: Some((removed.clone(), added.clone())),
            ..EnactmentEffects::default()
        };
        let (rem, add) = effects.committee_changes.unwrap();
        assert_eq!(rem.len(), 2, "key vs script removed members are distinct");
        assert_ne!(rem[0], rem[1], "KeyHash(0xC0) != ScriptHash(0xC0)");
        assert_eq!(add.len(), 2, "key vs script added members are distinct");
        assert_ne!(add[0].0, add[1].0, "KeyHash(0xC1) != ScriptHash(0xC1)");
        // Default stays dormant.
        assert!(EnactmentEffects::default().committee_changes.is_none());
    }

    // ── ENACTMENT-COMMITTEE-WRITEBACK S2: enactment write-back (CE-4..CE-6) ──

    fn ratified_with(action: GovAction) -> GovActionState {
        GovActionState {
            action_id: GovActionId { tx_hash: Hash32([0x09; 32]), index: 0 },
            committee_votes: Vec::new(),
            drep_votes: Vec::new(),
            spo_votes: Vec::new(),
            deposit: Coin(0),
            return_addr: Vec::new(),
            gov_action: action,
            proposed_in: EpochNo(500),
            expires_after: EpochNo(506),
        }
    }

    fn base_committee() -> std::collections::BTreeMap<StakeCredential, u64> {
        [(key(0xA0), 600u64), (script(0xA1), 600u64)].into_iter().collect()
    }

    /// CE-4: a ratified NoConfidence dissolves the committee to empty on
    /// write-back (the gap this cluster closes — the apply site used to clone
    /// the committee unchanged).
    #[test]
    fn enact_noconfidence_dissolves_committee() {
        let effects = enact_proposals(&[ratified_with(GovAction::NoConfidence { prev_action: None })]);
        assert!(effects.committee_dissolved, "NoConfidence sets committee_dissolved");
        assert!(effects.committee_changes.is_none());

        let (next, quorum) = apply_committee_enactment(&base_committee(), (2, 3), &effects);
        assert!(next.is_empty(), "committee dissolved to empty");
        assert_eq!(quorum, (2, 3), "NoConfidence does not change the quorum");
    }

    /// CE-5: a ratified UpdateCommittee removes the removed members, inserts the
    /// added ones with their expiry, and sets the new quorum threshold.
    #[test]
    fn enact_update_committee_applies_changes() {
        let removed: std::collections::BTreeSet<StakeCredential> =
            [key(0xA0)].into_iter().collect();
        let added: std::collections::BTreeMap<StakeCredential, u64> =
            [(key(0xB0), 720u64), (script(0xB1), 730u64)].into_iter().collect();
        let action = GovAction::UpdateCommittee {
            prev_action: None,
            removed,
            added,
            threshold: (3, 5),
        };
        let effects = enact_proposals(&[ratified_with(action)]);
        assert_eq!(effects.committee_threshold, Some((3, 5)));
        let (rem, add) = effects.committee_changes.as_ref().unwrap();
        assert_eq!(rem.len(), 1);
        assert_eq!(add.len(), 2);

        let (next, quorum) = apply_committee_enactment(&base_committee(), (2, 3), &effects);
        assert!(!next.contains_key(&key(0xA0)), "removed member is gone");
        assert!(next.contains_key(&script(0xA1)), "untouched member survives");
        assert_eq!(next.get(&key(0xB0)), Some(&720), "added key member with its expiry");
        assert_eq!(next.get(&script(0xB1)), Some(&730), "added script member with its expiry");
        assert_eq!(quorum, (3, 5), "quorum becomes the new threshold");
    }

    /// CE-5 (no collapse): a removed key-hash member does NOT remove a
    /// script-hash member of equal bytes, and an added key/script pair of equal
    /// bytes are two distinct entries (DC-LEDGER-10 through the write-back).
    #[test]
    fn enact_update_committee_keyhash_scripthash_distinct() {
        let added: std::collections::BTreeMap<StakeCredential, u64> =
            [(key(0x55), 700u64), (script(0x55), 701u64)].into_iter().collect();
        let removed: std::collections::BTreeSet<StakeCredential> =
            [key(0x55)].into_iter().collect();
        let effects = enact_proposals(&[ratified_with(GovAction::UpdateCommittee {
            prev_action: None, removed, added, threshold: (1, 2),
        })]);
        // Base committee holds a script member of the same bytes as the removed key.
        let base: std::collections::BTreeMap<StakeCredential, u64> =
            [(script(0x55), 600u64)].into_iter().collect();
        let (next, _) = apply_committee_enactment(&base, (2, 3), &effects);
        // The pre-existing script(0x55) is overwritten by the added script(0x55)=701,
        // and the added key(0x55)=700 is a distinct entry; removing key(0x55) only
        // affects the key variant.
        assert_eq!(next.get(&key(0x55)), Some(&700), "added key member present");
        assert_eq!(next.get(&script(0x55)), Some(&701), "script member distinct, not collapsed by the key removal");
        assert_eq!(next.len(), 2, "key and script of equal bytes are two entries");
    }

    /// CE-6: committee enactment is deterministic and the post-enactment
    /// gov-state fingerprint is byte-identical across two runs (R-1 / T-DET-01).
    #[test]
    fn committee_enactment_replays_byte_identical() {
        use crate::state::{ConwayGovState, LedgerState};
        use ade_types::CardanoEra;

        let added: std::collections::BTreeMap<StakeCredential, u64> =
            [(key(0xB0), 720u64), (script(0xB1), 730u64)].into_iter().collect();
        let removed: std::collections::BTreeSet<StakeCredential> =
            [key(0xA0)].into_iter().collect();
        let effects = enact_proposals(&[ratified_with(GovAction::UpdateCommittee {
            prev_action: None, removed, added, threshold: (3, 5),
        })]);

        let build = || {
            let (committee, quorum) =
                apply_committee_enactment(&base_committee(), (2, 3), &effects);
            let mut s = LedgerState::new(CardanoEra::Conway);
            s.gov_state = Some(ConwayGovState {
                proposals: Vec::new(),
                committee,
                committee_quorum: quorum,
                drep_expiry: Default::default(),
                gov_action_lifetime: 6,
                vote_delegations: Default::default(),
                pool_voting_thresholds: Vec::new(),
                drep_voting_thresholds: Vec::new(),
                committee_hot_keys: Default::default(),
            });
            crate::fingerprint::fingerprint(&s).governance
        };

        // Deterministic helper output.
        assert_eq!(
            apply_committee_enactment(&base_committee(), (2, 3), &effects),
            apply_committee_enactment(&base_committee(), (2, 3), &effects),
        );
        // Byte-identical gov-state fingerprint across runs.
        assert_eq!(build(), build());
    }
}
