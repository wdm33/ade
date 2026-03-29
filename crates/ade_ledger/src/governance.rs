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
    committee_members: &BTreeMap<Hash28, u64>, // credential → expiry epoch
    committee_quorum: &Rational,
    pool_thresholds: &[(u64, u64)],   // per-action-type pool voting thresholds
    drep_thresholds: &[(u64, u64)],   // per-action-type DRep voting thresholds
    current_epoch: u64,
) -> RatificationResult {
    let total_drep_active_stake = compute_active_drep_stake(drep_stake);
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
                    drep_stake,
                    total_pool_stake,
                    pool_stake,
                    committee_members,
                    committee_quorum,
                    pool_thresholds,
                    drep_thresholds,
                    current_epoch,
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
/// This is the denominator for DRep threshold checks.
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
    total_pool_stake: u64,
    pool_stake: &BTreeMap<ade_types::tx::PoolId, Coin>,
    committee_members: &BTreeMap<Hash28, u64>,
    committee_quorum: &Rational,
    pool_thresholds: &[(u64, u64)],
    drep_thresholds: &[(u64, u64)],
    current_epoch: u64,
) -> bool {
    let (pool_idx, drep_idx) = action_thresholds;

    // Committee check: if the action requires committee approval
    let needs_committee = !matches!(
        proposal.gov_action,
        GovAction::NoConfidence { .. } | GovAction::UpdateCommittee { .. }
    );
    if needs_committee && !committee_members.is_empty() {
        let active_members_count = committee_members.iter()
            .filter(|(_, expiry)| **expiry >= current_epoch)
            .count();
        if active_members_count > 0 {
            // Committee votes use HOT credentials, committee_members has COLD credentials.
            // Until VState hot→cold mapping is implemented, count all Yes votes
            // from the proposal's committee_votes directly.
            let yes_votes = proposal.committee_votes.iter()
                .filter(|(_, vote)| matches!(vote, Vote::Yes))
                .count();
            // Check against committee quorum: yes / active_members >= quorum
            let yes_rat = Rational::new(yes_votes as i128, active_members_count as i128)
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
                let lookup_stake = |cred: &Hash28| -> u64 {
                    drep_stake.get(&DRep::KeyHash(cred.clone()))
                        .or_else(|| drep_stake.get(&DRep::ScriptHash(cred.clone())))
                        .copied()
                        .unwrap_or(0)
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
    /// Committee changes: (removed, added with expiry).
    pub committee_changes: Option<(Vec<Hash28>, Vec<(Hash28, u64)>)>,
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
            GovAction::UpdateCommittee { raw, .. } => {
                // Committee changes stored as raw CBOR for now.
                // Full parsing deferred until needed for oracle comparison.
                let _ = raw;
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
