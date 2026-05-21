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
    // Filter DRep stake: exclude AlwaysAbstain AND inactive DReps
    let active_drep_stake: DRepStakeDistribution = drep_stake.iter()
        .filter(|(drep, _)| {
            match drep {
                DRep::AlwaysAbstain => false,
                // A DRep's key/script discriminant maps to the matching credential
                // variant — the drep_expiry map is keyed by the discriminated credential.
                DRep::KeyHash(h) => drep_expiry
                    .get(&StakeCredential::KeyHash(h.clone()))
                    .map(|e| *e >= current_epoch)
                    .unwrap_or(true),
                DRep::ScriptHash(h) => drep_expiry
                    .get(&StakeCredential::ScriptHash(h.clone()))
                    .map(|e| *e >= current_epoch)
                    .unwrap_or(true),
                _ => true,
            }
        })
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    let total_drep_active_stake = active_drep_stake.values().sum::<u64>();
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
}
