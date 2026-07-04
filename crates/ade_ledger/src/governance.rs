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
use ade_types::conway::governance::{GovAction, GovActionId, GovActionState, Vote};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::Coin;
use ade_types::Hash28;
use crate::epoch::StakeSnapshot;
use crate::rational::Rational;
use crate::state::DormantEpochs;

use std::collections::BTreeMap;

/// DRep stake distribution: maps each DRep to its total delegated voting stake.
pub type DRepStakeDistribution = BTreeMap<DRep, u64>;

/// Derive the DRep voting-stake distribution (CRE S3, the "distribution authority"): each DRep's voting
/// stake is the exact sum of the MARK stake of the credentials that delegated their vote to it. Only
/// positive stake contributes; an absent delegator is 0, never a default/guess. The mark snapshot is the
/// most recent (current-epoch) stake — the closest native analogue of the Haskell DRepPulser's InstantStake
/// (the byte-exact InstantStake match is S6's oracle gate, not this slice). Pure, deterministic,
/// replay-identical (ordered containers, no I/O). NOT threaded into the live ratification gate in S3 —
/// import-not-activate; S4 is the deliberate, oracle-verified activation.
pub fn derive_drep_voting_stake(
    vote_delegations: &BTreeMap<StakeCredential, DRep>,
    mark: &StakeSnapshot,
) -> DRepStakeDistribution {
    let mut out: DRepStakeDistribution = BTreeMap::new();
    for (cred, drep) in vote_delegations {
        let stake = mark.delegations.get(cred.hash()).map(|(_, c)| c.0).unwrap_or(0);
        if stake > 0 {
            *out.entry(drep.clone()).or_insert(0) += stake;
        }
    }
    out
}

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

/// The DRep-expiry/ratification path needs the `num_dormant` offset (a non-empty `drep_expiry` is being
/// evaluated) but the governance state is [`DormantEpochs::Unversioned`] — a TERMINAL, so the offset is never
/// fabricated as 0. See `feedback_versioned_authoritative_state_no_fabricated_default`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DormantRequired;

/// Active DRep stake (denominator for the DRep ratification gate): excludes `AlwaysAbstain` and DReps whose
/// term has expired under cardano's rule `drepExpiry + numDormant >= current_epoch` (absent ⇒ assumed
/// active). Returns the filtered distribution and its total. Shared by [`evaluate_ratification`] and the
/// S4.0 ratification census observer so both read the SAME denominator — one filter, not two.
///
/// The dormancy offset only affects DReps actually expiry-checked (present in `drep_expiry`). A
/// [`DormantEpochs::Unversioned`] state cannot supply the offset: if any expiry check would run this is a
/// TERMINAL [`DormantRequired`], never a fabricated 0.
pub(crate) fn active_drep_stake_filtered(
    drep_stake: &DRepStakeDistribution,
    drep_expiry: &BTreeMap<StakeCredential, u64>,
    num_dormant: &DormantEpochs,
    current_epoch: u64,
) -> Result<(DRepStakeDistribution, u64), DormantRequired> {
    let dormant = match num_dormant {
        DormantEpochs::Bound(n) => *n,
        // No DRep is expiry-checked ⇒ the offset is never applied ⇒ a V1 state is fine; else fail-closed.
        DormantEpochs::Unversioned if drep_expiry.is_empty() => 0,
        DormantEpochs::Unversioned => return Err(DormantRequired),
    };
    let active: DRepStakeDistribution = drep_stake
        .iter()
        .filter(|(drep, _)| match drep {
            DRep::AlwaysAbstain => false,
            // A DRep's key/script discriminant maps to the matching credential variant — the
            // drep_expiry map is keyed by the discriminated credential.
            DRep::KeyHash(h) => drep_expiry
                .get(&StakeCredential::KeyHash(h.clone()))
                .map(|e| e.saturating_add(dormant) >= current_epoch)
                .unwrap_or(true),
            DRep::ScriptHash(h) => drep_expiry
                .get(&StakeCredential::ScriptHash(h.clone()))
                .map(|e| e.saturating_add(dormant) >= current_epoch)
                .unwrap_or(true),
            _ => true,
        })
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    let total = active.values().sum::<u64>();
    Ok((active, total))
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
    num_dormant: &DormantEpochs,                  // versioned dormancy offset (authoritative)
) -> Result<RatificationResult, DormantRequired> {
    // Active DRep stake (exclude AlwaysAbstain + expired DReps) — shared with the S4.0 census observer.
    // Fail-closed if the dormancy offset is needed but the state is Unversioned.
    let (active_drep_stake, total_drep_active_stake) =
        active_drep_stake_filtered(drep_stake, drep_expiry, num_dormant, current_epoch)?;
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

    Ok(RatificationResult { ratified, expired, remaining })
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
    num_dormant: &DormantEpochs,
) -> Result<RatificationObservation, DormantRequired> {
    let is_info_action = matches!(proposal.gov_action, GovAction::InfoAction);
    let requires_committee = !matches!(
        proposal.gov_action,
        GovAction::NoConfidence { .. } | GovAction::UpdateCommittee { .. }
    );
    let (active_drep_stake, total_drep_active_stake) =
        active_drep_stake_filtered(drep_stake, drep_expiry, num_dormant, current_epoch)?;
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

    Ok(RatificationObservation {
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
    })
}

// ─── CRE S4.3: the single Conway governance epoch authority ─────────────

/// Why a proposal leaves the proposal set at an epoch boundary. S4.3a emits only [`RemovalCause::Expired`];
/// `Enacted`/`PrunedByEnactment` are populated when atomic enactment lands (S4.3c).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemovalCause {
    /// Ratified and enacted this boundary.
    Enacted,
    /// Removed because an enacted action broke this proposal's previous-action lineage.
    PrunedByEnactment,
    /// Expired without ratification (`expires_after < ending_epoch`) and provably threshold-failed.
    Expired,
}

/// A proposal removed at the boundary, with the reason. Emitted in canonical `GovActionId` order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedProposal {
    pub action_id: GovActionId,
    pub cause: RemovalCause,
}

/// Where a removed proposal's deposit goes — decided by the planner (the single authority), applied verbatim by the
/// boundary. There is a `DepositReturn` for EVERY removed proposal (including `NoDeposit`) so the accounting is total
/// and auditable: no proposal ever leaves the set without an explicit deposit disposition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepositReturn {
    /// The return address is a registered credential → credit its reward account.
    ToRewardAccount { action_id: GovActionId, credential: StakeCredential, amount: Coin },
    /// The return address is deregistered → the deposit is unclaimed → treasury.
    ToTreasury { action_id: GovActionId, amount: Coin },
    /// The proposal carried no deposit — nothing was ever escrowed to return.
    NoDeposit { action_id: GovActionId },
}

/// A closed protocol-parameter delta (never `Option`). S4.3a always produces `Unchanged`; the exec-memory parameter
/// enactment (S4.3c) produces `Set`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PParamsDelta {
    Unchanged,
    Set(Box<crate::pparams::ProtocolParameters>),
}

/// A closed previous-pparam-action-root delta (never `Option`). S4.3a always `Unchanged`; the root advances (S4.3c)
/// via `Set`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrevPParamActionDelta {
    Unchanged,
    Set(GovActionId),
}

/// The complete, atomic Conway governance epoch delta. The boundary transition APPLIES this whole delta or halts on
/// the [`GovernanceTerminal`]; NO other code path decides proposal removal, refund routing, the next proposal
/// structure, or the pparam/root changes. Single authority (CRE S4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConwayGovernanceEpochPlan {
    /// The next proposal set: the canonical `ConwayGovState.proposals` representation with removals filtered out,
    /// ORIGINAL order preserved (the fingerprint iterates `proposals` in order — order is identity-significant).
    pub proposals: Vec<GovActionState>,
    /// The proposals removed this boundary, in canonical `GovActionId` order.
    pub removals: Vec<RemovedProposal>,
    /// Explicit deposit routing for every removed proposal, in canonical `GovActionId` order (parallel to `removals`).
    pub deposit_returns: Vec<DepositReturn>,
    /// Protocol-parameter delta. S4.3a: `Unchanged`.
    pub pparams: PParamsDelta,
    /// Previous-pparam-action-root delta. S4.3a: `Unchanged`.
    pub prev_pparam_action: PrevPParamActionDelta,
}

/// A closed reason a refundable proposal's representation is malformed (never a free-form string).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MalformedGovDetail {
    /// The proposal's return address is not a 29-byte reward account.
    ReturnAddrNotRewardAccount,
}

/// Why a threshold-passing ratified action is OUTSIDE the S4.3c exec-units enactment subset (closed; never a
/// free-form string). Each is a fail-closed terminal: the boundary halts with ZERO mutation rather than enact an
/// action it does not support, silently drop it, or fabricate an effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedActionKind {
    /// A ratified action that is not a `ParameterChange` (a delaying action — no-confidence / update-committee /
    /// new-constitution / hard-fork — or a treasury withdrawal). Conservatively treated as ratified-and-unsupported:
    /// halting is safe whether it is delaying (it would delay the parameter change) or simply un-enactable here.
    NotParameterChange,
    /// A ratified `ParameterChange` touching a protocol-parameter field OUTSIDE the exec-units subset {20, 21}.
    NonExecUnitsField,
    /// A ratified exec-units `ParameterChange` that carried NEITHER `maxTxExUnits` nor `maxBlockExUnits` (nothing
    /// to enact).
    NoExecUnitsField,
    /// A ratified exec-units `ParameterChange` whose `steps` differs from the current bound `steps` — S4.3c is the
    /// MEMORY-ONLY subset (steps preserved); a steps change is a later slice, never a silent memory-only enactment.
    ChangedSteps,
    /// The ratified `ParameterChange.update` bytes exceed the fixed length / CBOR nesting-depth bound (checked
    /// BEFORE the recursive-descent decoder runs on attacker-influenced bytes). See T-RESOURCE-01.
    OversizedUpdate,
    /// The ratified `ParameterChange.update` bytes are not a well-formed exec-units `protocol_param_update` map.
    MalformedUpdate,
    /// A ratified (threshold-passing) `ParameterChange` chains directly onto the winner (`prev_action` == the
    /// winner id). Conway would chain-enact it in the SAME boundary (tip: parent → winner → child); S4.3c enacts
    /// AT MOST ONE action per boundary, so it halts rather than silently drop the child or diverge. A later slice
    /// generalises to multi-enactment.
    ChainedEnactment,
    /// MORE THAN ONE ratifiable `ParameterChange` shares the current enacted root (competing siblings both
    /// chaining onto the tip, both threshold-passing). cardano-ledger enacts the SUBMISSION-first (OMap order),
    /// which Ade cannot reconstruct from the canonical state — so it halts rather than pick the GovActionId-first
    /// and risk silently diverging. Fail-closed, symmetric with [`ChainedEnactment`]; a later slice resolves it.
    CompetingRatifiableActions,
}

/// The single governance-boundary terminal surface. ANY terminal halts the boundary with ZERO mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernanceTerminal {
    /// A threshold-passing ratified action falls OUTSIDE the S4.3c supported exec-units-memory `ParameterChange`
    /// subset (see [`UnsupportedActionKind`]). Terminal on BOTH the accumulator and replay paths (identical), zero
    /// mutation. This REPLACES the pre-S4.3c `PotentiallyRatifiable` threshold-pass terminal: a supported exec-units
    /// change now ENACTS atomically; every other ratified kind stays terminal until its own slice.
    UnsupportedRatifiedAction { action_id: GovActionId, kind: UnsupportedActionKind },
    /// A supported exec-units `ParameterChange` reached the enact path but the boundary state's `max_block_ex_units`
    /// or `prev_pparam_action` is `Unversioned` (a pre-V11 store) — fail-closed rather than fabricate the block
    /// ExUnits or the lineage root. A miswire (the enact path requires a V11 source-bound state).
    UnversionedStateOnEnactPath { action_id: GovActionId },
    /// The DRep-expiry gate needs the `num_dormant` offset but the governance state is `Unversioned` (S4.1) —
    /// fail-closed rather than fabricate the offset.
    DormantRequired,
    /// A refundable proposal's representation is malformed.
    Malformed { action_id: GovActionId, detail: MalformedGovDetail },
}

/// Canonical inputs to [`plan_conway_governance_epoch`] — all borrows, pure (no I/O, no ledger/accumulator handle).
pub struct ConwayGovernanceEpochInput<'a> {
    pub proposals: &'a [GovActionState],
    pub drep_stake: &'a DRepStakeDistribution,
    pub pool_stake: &'a BTreeMap<ade_types::tx::PoolId, Coin>,
    pub committee_members: &'a BTreeMap<StakeCredential, u64>,
    pub committee_quorum: &'a Rational,
    pub pool_thresholds: &'a [(u64, u64)],
    pub drep_thresholds: &'a [(u64, u64)],
    pub committee_hot_keys: &'a BTreeMap<StakeCredential, StakeCredential>,
    pub drep_expiry: &'a BTreeMap<StakeCredential, u64>,
    pub num_dormant: &'a DormantEpochs,
    /// The current (pre-boundary) protocol parameters — the exec-units enactment reads `max_tx_ex_units_cpu` /
    /// `max_block_ex_units` to enforce the memory-only (steps-unchanged) rule and to build the new limits. Source-bound
    /// (a V11 store binds `max_block_ex_units`); a `MaxBlockExUnits::Unversioned` on the enact path is a terminal.
    pub current_pparams: &'a crate::pparams::ProtocolParameters,
    /// The current (pre-boundary) enacted previous-`ParameterChange` root — the exec-units enactment verifies the
    /// winner's `prev_action` lineage against it and advances it. `Unversioned` on the enact path is a terminal (never
    /// a fabricated root).
    pub current_prev_pparam_action: &'a crate::state::PreviousPParamAction,
    /// The epoch being entered; ratification/expiry use `new_epoch - 1` (the ending epoch).
    pub new_epoch: u64,
}

/// The SINGLE Conway governance epoch authority (CRE S4.3c). Pure, deterministic, whole-set; examined BEFORE any
/// mutation, producing ONE atomic [`ConwayGovernanceEpochPlan`] or ONE [`GovernanceTerminal`]. In `GovActionId`
/// order it runs full RATIFY (thresholds — [`check_ratification`] — AND previous-action lineage), then, for the
/// supported subset, one atomic enactment:
///
/// - **Ratify scan.** For each proposal (skipping `InfoAction`, which never enacts): if it fails thresholds it is
///   not ratifiable. A threshold-passing action that is NOT an exec-units `ParameterChange` is a fail-closed
///   [`GovernanceTerminal::UnsupportedRatifiedAction`] (`NotParameterChange` — a delaying action would delay the
///   change; a treasury withdrawal cannot be dropped). A threshold-passing `ParameterChange` whose `prev_action`
///   does NOT match the current root is not ratifiable ⇒ carried (never enacted). A threshold-passing
///   `ParameterChange` with MATCHING lineage is RATIFIED: it must be a bounded, well-formed, exec-units-ONLY,
///   memory-only (steps unchanged) update, else a structured `UnsupportedRatifiedAction`; a pre-V11
///   (`Unversioned`) boundary state on this path is [`GovernanceTerminal::UnversionedStateOnEnactPath`]. The FIRST
///   such winner (canonical order) enacts; later matching siblings are pruned.
/// - **Removals + refunds** (all in `GovActionId` order, each with a total [`DepositReturn`]): the enacted winner
///   (`Enacted`); the superseded siblings/subtree sharing the winner's parent root (`PrunedByEnactment`); and any
///   other proposal past its lifetime (`Expired`). Registered return-addr → reward account, deregistered →
///   treasury, none → `NoDeposit`; a malformed return address is [`GovernanceTerminal::Malformed`].
/// - **Atomic delta.** On enactment: `pparams` = the new Tx/block memory limits (`Set`, steps preserved) and
///   `prev_pparam_action` = the winner id (`Set`); the next proposal set is the ORIGINAL order minus every removal.
///   With no enactment both deltas are `Unchanged` (the S4.3a expiry-only shape).
///
/// `is_registered` decides deposit routing — the routing lives HERE (the authority), never in a caller. Enacting
/// this plan is one construction at the single application point; no half of it can land without the others.
pub fn plan_conway_governance_epoch(
    input: &ConwayGovernanceEpochInput<'_>,
    is_registered: impl Fn(&StakeCredential) -> bool,
) -> Result<ConwayGovernanceEpochPlan, GovernanceTerminal> {
    let ending_epoch = input.new_epoch.saturating_sub(1);
    // Fail-closed if the dormancy offset is needed (non-empty drep_expiry) but the state is Unversioned.
    let (active_drep_stake, total_drep_active_stake) =
        active_drep_stake_filtered(input.drep_stake, input.drep_expiry, input.num_dormant, ending_epoch)
            .map_err(|_| GovernanceTerminal::DormantRequired)?;
    let total_pool_stake: u64 = input.pool_stake.values().map(|c| c.0).sum();

    // Canonical `GovActionId` order over the WHOLE set for the ratify scan and the removal/return lists.
    let mut sorted: Vec<&GovActionState> = input.proposals.iter().collect();
    sorted.sort_by(|a, b| a.action_id.cmp(&b.action_id));

    // ── Phase 1: scan for the boundary's enactment CANDIDATES; fail-closed on unsupported ratified kinds. ──
    // A candidate is a threshold-passing `ParameterChange` whose `prev_action` == the current enacted root (it
    // chains onto the tip). cardano-ledger enacts the SUBMISSION-first (OMap order) candidate; Ade cannot
    // reconstruct submission order from the canonical state, so >1 candidate is fail-closed below (never a
    // GovActionId-order guess that could silently diverge). `threshold_passing_pc_ids` feeds the chain check.
    let mut threshold_passing_pc_ids: std::collections::BTreeSet<GovActionId> =
        std::collections::BTreeSet::new();
    let mut candidates: Vec<&GovActionState> = Vec::new();
    for p in &sorted {
        if matches!(p.gov_action, GovAction::InfoAction) {
            continue; // InfoAction never enacts (mirrors evaluate_ratification's special case).
        }
        let thresholds_accepted = check_ratification(
            p,
            gov_action_threshold_index(&p.gov_action),
            &total_drep_active_stake,
            &active_drep_stake,
            total_pool_stake,
            input.pool_stake,
            input.committee_members,
            input.committee_quorum,
            input.pool_thresholds,
            input.drep_thresholds,
            ending_epoch,
            input.committee_hot_keys,
        );
        if !thresholds_accepted {
            continue; // Not a threshold-passer — carried/expired below, never enacted.
        }
        match &p.gov_action {
            GovAction::ParameterChange { prev_action, .. } => {
                threshold_passing_pc_ids.insert(p.action_id.clone());
                // A pre-V11 (Unversioned) boundary state cannot supply the lineage root or the block ExUnits —
                // fail-closed (the enact path requires a source-bound V11 state), never a fabricated value.
                if matches!(input.current_prev_pparam_action, crate::state::PreviousPParamAction::Unversioned)
                    || matches!(
                        input.current_pparams.max_block_ex_units,
                        crate::pparams::MaxBlockExUnits::Unversioned
                    )
                {
                    return Err(GovernanceTerminal::UnversionedStateOnEnactPath {
                        action_id: p.action_id.clone(),
                    });
                }
                // Previous-action lineage: a candidate's `prev_action` must equal the current enacted root. A
                // mismatch ⇒ NOT ratifiable this boundary ⇒ carried (a chain child prev==winner is caught by the
                // Phase-2 chain check). Collect the ratifiable candidates; the single-winner decision is below.
                let lineage_ok = match input.current_prev_pparam_action {
                    crate::state::PreviousPParamAction::NoPreviousAction => prev_action.is_none(),
                    crate::state::PreviousPParamAction::Enacted(root) => prev_action.as_ref() == Some(root),
                    crate::state::PreviousPParamAction::Unversioned => false, // guarded above
                };
                if lineage_ok {
                    candidates.push(p);
                }
            }
            _ => {
                // A ratified action outside the exec-units subset — fail-closed (halt, zero mutation).
                return Err(GovernanceTerminal::UnsupportedRatifiedAction {
                    action_id: p.action_id.clone(),
                    kind: UnsupportedActionKind::NotParameterChange,
                });
            }
        }
    }

    // At most ONE action enacts per boundary. cardano-ledger enacts the submission-first candidate; >1 competing
    // candidate is submission-order dependent ⇒ fail-closed. Exactly one ⇒ the unambiguous winner, which must be
    // a supported memory-only exec-units update (else a structured terminal). `enactment` = (winner id, the
    // winner's superseded parent root, the fully-built new pparams).
    let enactment: Option<(GovActionId, Option<GovActionId>, crate::pparams::ProtocolParameters)> =
        match candidates.as_slice() {
            [] => None,
            [winner] => {
                let (prev_action, update) = match &winner.gov_action {
                    GovAction::ParameterChange { prev_action, update, .. } => (prev_action, update),
                    _ => unreachable!("candidates holds only ParameterChanges (pushed in the ParameterChange arm)"),
                };
                let unsupported = |kind| GovernanceTerminal::UnsupportedRatifiedAction {
                    action_id: winner.action_id.clone(),
                    kind,
                };
                // Bound the attacker-influenced bytes BEFORE the recursive-descent decoder (T-RESOURCE-01).
                if !exec_units_update_within_bounds(update) {
                    return Err(unsupported(UnsupportedActionKind::OversizedUpdate));
                }
                let decoded = decode_exec_units_param_update(update)
                    .map_err(|_| unsupported(UnsupportedActionKind::MalformedUpdate))?;
                if !decoded.unsupported_fields.0.is_empty() {
                    return Err(unsupported(UnsupportedActionKind::NonExecUnitsField));
                }
                if decoded.max_tx_ex_units.is_none() && decoded.max_block_ex_units.is_none() {
                    return Err(unsupported(UnsupportedActionKind::NoExecUnitsField));
                }
                // MEMORY-ONLY: every supplied `steps` must equal the current bound `steps`.
                if let Some(tx) = decoded.max_tx_ex_units {
                    if tx.steps != input.current_pparams.max_tx_ex_units_cpu {
                        return Err(unsupported(UnsupportedActionKind::ChangedSteps));
                    }
                }
                if let Some(blk) = decoded.max_block_ex_units {
                    let cur_block_steps = match input.current_pparams.max_block_ex_units {
                        crate::pparams::MaxBlockExUnits::Bound { steps, .. } => steps,
                        crate::pparams::MaxBlockExUnits::Unversioned => {
                            return Err(GovernanceTerminal::UnversionedStateOnEnactPath {
                                action_id: winner.action_id.clone(),
                            }) // guarded above; defensive.
                        }
                    };
                    if blk.steps != cur_block_steps {
                        return Err(unsupported(UnsupportedActionKind::ChangedSteps));
                    }
                }
                let new_pparams = build_enacted_pparams(input.current_pparams, &decoded);
                Some((winner.action_id.clone(), prev_action.clone(), new_pparams))
            }
            _ => {
                // More than one ratifiable candidate competes for the single enactment slot — cardano-ledger's
                // pick is submission-order dependent, which Ade cannot reconstruct ⇒ fail-closed (a later slice
                // may enact multiply). Report the canonical-first for a stable diagnostic id.
                return Err(GovernanceTerminal::UnsupportedRatifiedAction {
                    action_id: candidates[0].action_id.clone(),
                    kind: UnsupportedActionKind::CompetingRatifiableActions,
                });
            }
        };

    // ── Phase 2: enactment-driven removals — the winner's LOSING competitors, plus the fail-closed chain check. ──
    let winner_id: Option<GovActionId> = enactment.as_ref().map(|(id, _, _)| id.clone());
    // Proposals orphaned by the enactment: the winner's SIBLINGS (ParameterChanges sharing the winner's parent
    // root, EXCLUDING the winner) and their descendant subtrees (to a fixpoint). The enacted action re-roots the
    // PParamUpdate lineage, so every competing subtree is dead. The winner's OWN descendants are deliberately NOT
    // pruned: a child chaining onto the winner is the new lineage (Conway carries it if pending), never a loser.
    let mut pruned: std::collections::BTreeSet<GovActionId> = std::collections::BTreeSet::new();
    if let Some((wid, superseded_parent, _)) = enactment.as_ref() {
        for p in input.proposals.iter() {
            if p.action_id == *wid {
                continue;
            }
            if let GovAction::ParameterChange { prev_action, .. } = &p.gov_action {
                if prev_action == superseded_parent {
                    pruned.insert(p.action_id.clone());
                }
            }
        }
        loop {
            let mut added = false;
            for p in input.proposals.iter() {
                if p.action_id == *wid || pruned.contains(&p.action_id) {
                    continue;
                }
                if let GovAction::ParameterChange { prev_action, .. } = &p.gov_action {
                    if prev_action.as_ref().map_or(false, |pa| pruned.contains(pa)) {
                        pruned.insert(p.action_id.clone());
                        added = true;
                    }
                }
            }
            if !added {
                break;
            }
        }
        // Fail-closed chain check: a ratifiable (threshold-passing) `ParameterChange` chaining DIRECTLY onto the
        // winner would chain-enact in Conway (tip: parent → winner → child). S4.3c enacts at most one action per
        // boundary — halt with ZERO mutation rather than silently drop the child or diverge from Conway.
        for p in input.proposals.iter() {
            if let GovAction::ParameterChange { prev_action, .. } = &p.gov_action {
                if prev_action.as_ref() == Some(wid) && threshold_passing_pc_ids.contains(&p.action_id) {
                    return Err(GovernanceTerminal::UnsupportedRatifiedAction {
                        action_id: p.action_id.clone(),
                        kind: UnsupportedActionKind::ChainedEnactment,
                    });
                }
            }
        }
    }

    // ── Phase 3: classify every proposal (GovActionId order) → removal + total deposit return, or carry. ──
    let route_deposit = |p: &GovActionState| -> Result<DepositReturn, GovernanceTerminal> {
        if p.deposit.0 == 0 {
            return Ok(DepositReturn::NoDeposit { action_id: p.action_id.clone() });
        }
        match crate::epoch_accumulator::reward_account_credential(&p.return_addr) {
            Some(cred) => Ok(if is_registered(&cred) {
                DepositReturn::ToRewardAccount {
                    action_id: p.action_id.clone(),
                    credential: cred,
                    amount: p.deposit,
                }
            } else {
                DepositReturn::ToTreasury { action_id: p.action_id.clone(), amount: p.deposit }
            }),
            None => Err(GovernanceTerminal::Malformed {
                action_id: p.action_id.clone(),
                detail: MalformedGovDetail::ReturnAddrNotRewardAccount,
            }),
        }
    };

    let mut removals: Vec<RemovedProposal> = Vec::new();
    let mut deposit_returns: Vec<DepositReturn> = Vec::new();
    let mut removed_ids: std::collections::BTreeSet<GovActionId> = std::collections::BTreeSet::new();
    for p in &sorted {
        let cause = if winner_id.as_ref() == Some(&p.action_id) {
            RemovalCause::Enacted
        } else if pruned.contains(&p.action_id) {
            RemovalCause::PrunedByEnactment
        } else if p.expires_after.0 < ending_epoch {
            // Not the winner, not pruned, past its lifetime ⇒ expired (threshold-failed, or a threshold-passing
            // but lineage-mismatched change that can never ratify). Conway returns its deposit too.
            RemovalCause::Expired
        } else {
            continue; // carried forward
        };
        deposit_returns.push(route_deposit(p)?);
        removals.push(RemovedProposal { action_id: p.action_id.clone(), cause });
        removed_ids.insert(p.action_id.clone());
    }

    // The next proposal set: filter the ORIGINAL (order-preserving) — the fingerprint is order-significant.
    let proposals: Vec<GovActionState> =
        input.proposals.iter().filter(|p| !removed_ids.contains(&p.action_id)).cloned().collect();

    let (pparams, prev_pparam_action) = match enactment {
        Some((wid, _, new_pparams)) => (
            PParamsDelta::Set(Box::new(new_pparams)),
            PrevPParamActionDelta::Set(wid),
        ),
        None => (PParamsDelta::Unchanged, PrevPParamActionDelta::Unchanged),
    };

    Ok(ConwayGovernanceEpochPlan { proposals, removals, deposit_returns, pparams, prev_pparam_action })
}

/// Build the enacted protocol parameters for a supported exec-units, memory-only `ParameterChange`: clone the
/// current parameters and overwrite ONLY the supplied memory limits, leaving `steps` (verified equal by the
/// caller) and every other parameter untouched. The block limit stays `Bound` (the caller guaranteed the current
/// value is `Bound`), so the versioned lineage is preserved.
fn build_enacted_pparams(
    current: &crate::pparams::ProtocolParameters,
    decoded: &ExecUnitsParamUpdate,
) -> crate::pparams::ProtocolParameters {
    let mut pp = current.clone();
    if let Some(tx) = decoded.max_tx_ex_units {
        pp.max_tx_ex_units_mem = tx.mem; // steps == max_tx_ex_units_cpu (verified) — leave cpu unchanged.
    }
    if let Some(blk) = decoded.max_block_ex_units {
        pp.max_block_ex_units = crate::pparams::MaxBlockExUnits::Bound { mem: blk.mem, steps: blk.steps };
    }
    pp
}

/// Fail-closed pre-decode bound (CRE S4.3c hard boundary #3 / T-RESOURCE-01): reject a ratified
/// `ParameterChange.update` whose byte length or CBOR nesting depth exceeds a fixed cap BEFORE the recursive
/// `skip_item`-based [`decode_exec_units_param_update`] runs on attacker-influenced bytes, so the decoder's
/// recursion can never be driven to a stack overflow. Iterative (no recursion); parses only CBOR heads. A
/// legitimate `protocol_param_update` is a shallow map of small values — the real witness is 33 bytes, depth 3.
fn exec_units_update_within_bounds(update: &[u8]) -> bool {
    const MAX_LEN: usize = 4096;
    const MAX_DEPTH: usize = 16;
    if update.is_empty() || update.len() > MAX_LEN {
        return false;
    }
    // `stack[i]` = items still to parse at nesting level i; depth = stack.len(). A synthetic top level holds the
    // single expected data item (the update map). A container pushes a child level and is "consumed" from its
    // parent only when that child level empties; a leaf is consumed immediately.
    let mut stack: Vec<u64> = vec![1];
    let mut o = 0usize;
    loop {
        // Propagate completed levels upward (an emptied container consumes one slot of its parent).
        while let Some(&top) = stack.last() {
            if top == 0 {
                stack.pop();
                if let Some(parent) = stack.last_mut() {
                    *parent -= 1;
                }
            } else {
                break;
            }
        }
        if stack.is_empty() {
            break;
        }
        if o >= update.len() {
            return false; // truncated
        }
        let ib = update[o];
        o += 1;
        let major = ib >> 5;
        let ai = ib & 0x1f;
        let arg: u64 = match ai {
            0..=23 => ai as u64,
            24 => {
                if o >= update.len() {
                    return false;
                }
                let v = update[o] as u64;
                o += 1;
                v
            }
            25 => {
                if o + 2 > update.len() {
                    return false;
                }
                let v = u16::from_be_bytes([update[o], update[o + 1]]) as u64;
                o += 2;
                v
            }
            26 => {
                if o + 4 > update.len() {
                    return false;
                }
                let v = u32::from_be_bytes([update[o], update[o + 1], update[o + 2], update[o + 3]]) as u64;
                o += 4;
                v
            }
            27 => {
                if o + 8 > update.len() {
                    return false;
                }
                let mut b = [0u8; 8];
                b.copy_from_slice(&update[o..o + 8]);
                o += 8;
                u64::from_be_bytes(b)
            }
            _ => return false, // 28..=30 reserved; 31 indefinite — non-canonical for a Conway pparam update.
        };
        match major {
            0 | 1 => {
                *stack.last_mut().expect("non-empty") -= 1; // uint / nint leaf
            }
            2 | 3 => {
                // byte / text string: skip its `arg` payload bytes (a leaf).
                if o.checked_add(arg as usize).map_or(true, |e| e > update.len()) {
                    return false;
                }
                o += arg as usize;
                *stack.last_mut().expect("non-empty") -= 1;
            }
            4 => {
                if stack.len() >= MAX_DEPTH {
                    return false;
                }
                stack.push(arg); // array of `arg` items
            }
            5 => {
                if stack.len() >= MAX_DEPTH {
                    return false;
                }
                match arg.checked_mul(2) {
                    Some(items) => stack.push(items), // map of `arg` pairs
                    None => return false,
                }
            }
            6 => {
                if stack.len() >= MAX_DEPTH {
                    return false;
                }
                stack.push(1); // tag wraps exactly one following item
            }
            7 => {
                if ai == 31 {
                    return false; // BREAK in a definite context
                }
                *stack.last_mut().expect("non-empty") -= 1; // simple / float leaf
            }
            _ => return false,
        }
    }
    o == update.len()
}

// ─── CRE S4.3b: closed Conway exec-units parameter-update decoder (INERT) ──

/// The two components `[mem, steps]` of a Conway `ex_units` value (a `maxTxExUnits` / `maxBlockExUnits` update).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExUnits {
    pub mem: u64,
    pub steps: u64,
}

/// The canonically-ordered set of `protocol_param_update` map keys the exec-units decoder does not support.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CanonicalFieldSet(pub std::collections::BTreeSet<u64>);

/// The closed, structured result of decoding a Conway `ParameterChange.update` for the exec-units subset (CRE
/// S4.3b). BOTH exec-units fields are decoded COMPLETELY as `[mem, steps]`; every OTHER present key is preserved
/// in `unsupported_fields`, never silently dropped. INERT: nothing applies this in S4.3b — S4.3c's exec-units
/// enactment consumes it, supporting an update ONLY when `unsupported_fields` is empty AND each supplied `steps`
/// equals the current bound `steps` (a changed `steps` is `UnsupportedRatifiedAction`, not malformed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecUnitsParamUpdate {
    pub max_tx_ex_units: Option<ExUnits>,
    pub max_block_ex_units: Option<ExUnits>,
    pub unsupported_fields: CanonicalFieldSet,
}

/// A structured failure decoding a Conway `ParameterChange.update` (CRE S4.3b). No fallback parsing, no
/// last-write-wins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecUnitsUpdateError {
    /// Not a well-formed definite CBOR `protocol_param_update` map (bad header, indefinite, or trailing bytes).
    Malformed,
    /// A recognized exec-units key appeared more than once.
    DuplicateKey { key: u64 },
    /// A recognized exec-units value was not a well-formed `array(2)[mem, steps]`.
    MalformedExUnits { key: u64 },
}

/// Conway `protocol_param_update` map keys for the two exec-units limits. From conway.cddl: `20 = maxTxExUnits`,
/// `21 = maxBlockExUnits` (Conway renumbered vs Alonzo's 21/22). Validated against the real witness update in the
/// S4.3b gate tests.
const PPU_KEY_MAX_TX_EX_UNITS: u64 = 20;
const PPU_KEY_MAX_BLOCK_EX_UNITS: u64 = 21;

/// Decode a Conway `ParameterChange.update` (a `protocol_param_update` CBOR map) for the exec-units subset,
/// reading BOTH exec-units fields COMPLETELY as `[mem, steps]`. Every other present key is recorded in
/// `unsupported_fields` (never silently dropped). Fail-closed: a non-map / bad header / indefinite map /
/// trailing bytes, a duplicate recognized key, or a recognized value that is not `array(2)[mem, steps]` is a
/// structured error. Pure; the raw `update` bytes remain the caller's. INERT (no applier in S4.3b).
pub fn decode_exec_units_param_update(
    update: &[u8],
) -> Result<ExecUnitsParamUpdate, ExecUnitsUpdateError> {
    use ade_codec::cbor::{read_map_header, read_uint, skip_item, ContainerEncoding};
    let mut o = 0usize;
    let n = match read_map_header(update, &mut o).map_err(|_| ExecUnitsUpdateError::Malformed)? {
        ContainerEncoding::Definite(n, _) => n,
        // An indefinite-length map is not canonical Conway wire form.
        _ => return Err(ExecUnitsUpdateError::Malformed),
    };
    let mut max_tx_ex_units: Option<ExUnits> = None;
    let mut max_block_ex_units: Option<ExUnits> = None;
    let mut unsupported: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    for _ in 0..n {
        let (key, _) = read_uint(update, &mut o).map_err(|_| ExecUnitsUpdateError::Malformed)?;
        match key {
            PPU_KEY_MAX_TX_EX_UNITS => {
                if max_tx_ex_units.is_some() {
                    return Err(ExecUnitsUpdateError::DuplicateKey { key });
                }
                max_tx_ex_units = Some(read_ex_units_value(update, &mut o, key)?);
            }
            PPU_KEY_MAX_BLOCK_EX_UNITS => {
                if max_block_ex_units.is_some() {
                    return Err(ExecUnitsUpdateError::DuplicateKey { key });
                }
                max_block_ex_units = Some(read_ex_units_value(update, &mut o, key)?);
            }
            other => {
                // Record the unsupported key (never dropped) and skip its arbitrary value uninterpreted.
                unsupported.insert(other);
                skip_item(update, &mut o).map_err(|_| ExecUnitsUpdateError::Malformed)?;
            }
        }
    }
    if o != update.len() {
        return Err(ExecUnitsUpdateError::Malformed);
    }
    Ok(ExecUnitsParamUpdate {
        max_tx_ex_units,
        max_block_ex_units,
        unsupported_fields: CanonicalFieldSet(unsupported),
    })
}

/// Read a Conway `ex_units = array(2)[mem, steps]`; anything else is `MalformedExUnits`.
fn read_ex_units_value(d: &[u8], o: &mut usize, key: u64) -> Result<ExUnits, ExecUnitsUpdateError> {
    use ade_codec::cbor::{read_array_header, read_uint, ContainerEncoding};
    match read_array_header(d, o) {
        Ok(ContainerEncoding::Definite(2, _)) => {}
        _ => return Err(ExecUnitsUpdateError::MalformedExUnits { key }),
    }
    let (mem, _) = read_uint(d, o).map_err(|_| ExecUnitsUpdateError::MalformedExUnits { key })?;
    let (steps, _) = read_uint(d, o).map_err(|_| ExecUnitsUpdateError::MalformedExUnits { key })?;
    Ok(ExUnits { mem, steps })
}

#[cfg(test)]
mod cre_s4_3b_decoder_tests {
    use super::*;
    use ade_codec::cbor::{
        write_array_header, write_map_header, write_uint_canonical, ContainerEncoding, IntWidth,
    };

    fn ex_units(buf: &mut Vec<u8>, mem: u64, steps: u64) {
        write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        write_uint_canonical(buf, mem);
        write_uint_canonical(buf, steps);
    }

    /// GATE 5: a Conway `protocol_param_update` with the two exec-units keys (20 = maxTx, 21 = maxBlock) decodes
    /// deterministically into the supported subset — BOTH full `[mem, steps]`, `unsupported_fields` empty. The
    /// memory-only-effect witness shape (steps present, to be checked unchanged in S4.3c).
    #[test]
    fn cre_s4_3b_gate5_witness_shape_decodes_into_supported_subset() {
        let (steps_tx, steps_block) = (10_000_000_000u64, 40_000_000_000u64);
        let mut u = Vec::new();
        write_map_header(&mut u, ContainerEncoding::Definite(2, IntWidth::Inline));
        write_uint_canonical(&mut u, 20);
        ex_units(&mut u, 16_500_000, steps_tx);
        write_uint_canonical(&mut u, 21);
        ex_units(&mut u, 72_000_000, steps_block);
        let d = decode_exec_units_param_update(&u).expect("decode");
        assert_eq!(d.max_tx_ex_units, Some(ExUnits { mem: 16_500_000, steps: steps_tx }));
        assert_eq!(d.max_block_ex_units, Some(ExUnits { mem: 72_000_000, steps: steps_block }));
        assert!(d.unsupported_fields.0.is_empty(), "the witness touches only the exec-units subset");
    }

    /// GATE 6: unknown / duplicate / malformed / mixed updates all yield deterministic structured outcomes.
    #[test]
    fn cre_s4_3b_gate6_deterministic_structured_outcomes() {
        // (a) MIXED: an unknown key (0 = minFeeA) is PRESERVED in unsupported_fields, never dropped; the
        //     supported maxTx key still decodes.
        let mut mixed = Vec::new();
        write_map_header(&mut mixed, ContainerEncoding::Definite(2, IntWidth::Inline));
        write_uint_canonical(&mut mixed, 0);
        write_uint_canonical(&mut mixed, 44); // minFeeA value (a coin) — skipped, key recorded
        write_uint_canonical(&mut mixed, 20);
        ex_units(&mut mixed, 16_500_000, 1);
        let d = decode_exec_units_param_update(&mixed).expect("decode");
        assert_eq!(d.max_tx_ex_units, Some(ExUnits { mem: 16_500_000, steps: 1 }));
        assert_eq!(d.max_block_ex_units, None);
        assert_eq!(d.unsupported_fields.0.iter().copied().collect::<Vec<_>>(), vec![0]);

        // (b) DUPLICATE recognized key → structured error (no last-write-wins).
        let mut dup = Vec::new();
        write_map_header(&mut dup, ContainerEncoding::Definite(2, IntWidth::Inline));
        write_uint_canonical(&mut dup, 20);
        ex_units(&mut dup, 1, 2);
        write_uint_canonical(&mut dup, 20);
        ex_units(&mut dup, 3, 4);
        assert_eq!(
            decode_exec_units_param_update(&dup),
            Err(ExecUnitsUpdateError::DuplicateKey { key: 20 })
        );

        // (c) MALFORMED recognized value (not array(2)[mem, steps]).
        let mut bad = Vec::new();
        write_map_header(&mut bad, ContainerEncoding::Definite(1, IntWidth::Inline));
        write_uint_canonical(&mut bad, 21);
        write_uint_canonical(&mut bad, 999);
        assert_eq!(
            decode_exec_units_param_update(&bad),
            Err(ExecUnitsUpdateError::MalformedExUnits { key: 21 })
        );

        // (d) a non-map input is Malformed.
        assert_eq!(decode_exec_units_param_update(&[0x00]), Err(ExecUnitsUpdateError::Malformed));

        // (e) trailing bytes after a definite map → Malformed (no fallback).
        let mut trailing = Vec::new();
        write_map_header(&mut trailing, ContainerEncoding::Definite(0, IntWidth::Inline));
        trailing.push(0xFF);
        assert_eq!(decode_exec_units_param_update(&trailing), Err(ExecUnitsUpdateError::Malformed));
    }

    /// GATE 5 (real witness — HERMETIC): the DEFINITIVE key-20/21 validation over REAL on-chain bytes.
    /// `WITNESS_UPDATE` is proposal 69c948cd..#0's ACTUAL `ParameterChange.update`, extracted from the local
    /// Preview epoch-1095 ledger state and committed here as the canonical witness. That the committed bytes
    /// ARE the chain's is proven by the `#[ignore]` live re-extraction
    /// `cre_s4_3b_gate5_real_witness_update_decodes` (ade_testkit tests/cre_enactment_census.rs), which
    /// re-derives them from the db-analyser state and asserts the identical bytes + blake2b.
    ///
    /// Manifest — point: slot 94608021 / epoch 1095 / era Conway / network Preview (magic 2); source:
    /// `db-analyser --store-ledger 94608021` on cardano-node 11.0.1; raw-update blake2b256 =
    /// 4b70f9513bb1768b34680ada28c9bf27bfe4f0cdf885d4003de9f9c78cec4d2b; decoded = maxTxExUnits{mem 16_500_000,
    /// steps 10_000_000_000} + maxBlockExUnits{mem 72_000_000, steps 20_000_000_000}, unsupported {}. The
    /// effect is MEMORY-ONLY: both steps equal the 1095 curPParams steps (10e9 / 20e9), so S4.3c supports it;
    /// a changed steps would be UnsupportedRatifiedAction (never a silent memory-only enactment).
    #[test]
    fn cre_s4_3b_gate5_real_witness_manifest() {
        const WITNESS_UPDATE: &[u8] = &[
            0xa2, 0x14, 0x82, 0x1a, 0x00, 0xfb, 0xc5, 0x20, 0x1b, 0x00, 0x00, 0x00, 0x02, 0x54, 0x0b, 0xe4,
            0x00, 0x15, 0x82, 0x1a, 0x04, 0x4a, 0xa2, 0x00, 0x1b, 0x00, 0x00, 0x00, 0x04, 0xa8, 0x17, 0xc8,
            0x00,
        ];
        let decoded = decode_exec_units_param_update(WITNESS_UPDATE).expect("decode the real witness update");
        assert_eq!(
            decoded.max_tx_ex_units,
            Some(ExUnits { mem: 16_500_000, steps: 10_000_000_000 }),
            "maxTxExUnits (map key 20) from the REAL on-chain update"
        );
        assert_eq!(
            decoded.max_block_ex_units,
            Some(ExUnits { mem: 72_000_000, steps: 20_000_000_000 }),
            "maxBlockExUnits (map key 21) from the REAL on-chain update"
        );
        assert!(
            decoded.unsupported_fields.0.is_empty(),
            "the witness touches ONLY the two exec-units keys: {:?}",
            decoded.unsupported_fields
        );
    }
}

#[cfg(test)]
mod cre_s4_3c_enactment_tests {
    //! CRE S4.3c — the atomic exec-units enactment authority. These pure-planner gates prove the supported
    //! memory-only `ParameterChange` subset ENACTS (limits change, steps preserved, root advances, siblings
    //! prune, deposits refund) and that EVERYTHING outside the subset is a fail-closed terminal with zero
    //! mutation. Mirrors the real 1095→1096 witness structure (winner + five siblings sharing the superseded
    //! root); the `#[ignore]` corpus differential drives the real state.
    use super::*;
    use ade_types::conway::governance::{GovAction, GovActionId, GovActionState, Vote};
    use ade_types::shelley::cert::StakeCredential;
    use ade_types::tx::Coin;
    use ade_types::{EpochNo, Hash28, Hash32};
    use crate::pparams::{MaxBlockExUnits, ProtocolParameters};
    use crate::state::PreviousPParamAction;
    use crate::rational::Rational;
    use std::collections::BTreeMap;

    /// The REAL 69c948cd..#0 witness update: maxTx {16.5M mem, 10e9 steps} + maxBlock {72M mem, 20e9 steps} —
    /// memory-only vs the 1095 curPParams. Same canonical bytes proven in `cre_s4_3b_gate5_real_witness_manifest`.
    const WITNESS_UPDATE: &[u8] = &[
        0xa2, 0x14, 0x82, 0x1a, 0x00, 0xfb, 0xc5, 0x20, 0x1b, 0x00, 0x00, 0x00, 0x02, 0x54, 0x0b, 0xe4,
        0x00, 0x15, 0x82, 0x1a, 0x04, 0x4a, 0xa2, 0x00, 0x1b, 0x00, 0x00, 0x00, 0x04, 0xa8, 0x17, 0xc8,
        0x00,
    ];

    fn key(b: u8) -> StakeCredential { StakeCredential::KeyHash(Hash28([b; 28])) }
    fn gaid(b: u8) -> GovActionId { GovActionId { tx_hash: Hash32([b; 32]), index: 0 } }
    fn pc(prev: Option<GovActionId>, update: Vec<u8>) -> GovAction {
        GovAction::ParameterChange { prev_action: prev, update, policy_hash: None }
    }
    /// 1095 curPParams shape: maxTx {14M mem, 10e9 steps} (the `Default`) + maxBlock `Bound {62M, 20e9}`.
    fn pp_1095() -> ProtocolParameters {
        ProtocolParameters {
            max_block_ex_units: MaxBlockExUnits::Bound { mem: 62_000_000, steps: 20_000_000_000 },
            ..Default::default()
        }
    }
    fn committee() -> BTreeMap<StakeCredential, u64> {
        [(key(0xC1), 1400u64), (key(0xC2), 1400), (key(0xC3), 1400)].into_iter().collect()
    }
    fn yes_2of3() -> Vec<(StakeCredential, Vote)> { vec![(key(0xC1), Vote::Yes), (key(0xC2), Vote::Yes)] }
    /// A gov-action proposal with a 100k deposit and a registered reward-account return address.
    fn prop(id: u8, action: GovAction, votes: Vec<(StakeCredential, Vote)>, expires_after: u64) -> GovActionState {
        GovActionState {
            action_id: gaid(id),
            committee_votes: votes,
            drep_votes: Vec::new(),
            spo_votes: Vec::new(),
            deposit: Coin(100_000_000_000),
            return_addr: vec![0xe0; 29],
            gov_action: action,
            proposed_in: EpochNo(1309),
            expires_after: EpochNo(expires_after),
        }
    }
    fn plan_enact(
        proposals: &[GovActionState], cur_pparams: &ProtocolParameters, cur_prev: &PreviousPParamAction,
    ) -> Result<ConwayGovernanceEpochPlan, GovernanceTerminal> {
        let quorum = Rational::new(2, 3).unwrap();
        let empty_stake = BTreeMap::new();
        let empty_pool = BTreeMap::new();
        let comm = committee();
        let empty_hot = BTreeMap::new();
        let empty_expiry = BTreeMap::new();
        let dormant = DormantEpochs::Unversioned;
        let input = ConwayGovernanceEpochInput {
            proposals,
            drep_stake: &empty_stake,
            pool_stake: &empty_pool,
            committee_members: &comm,
            committee_quorum: &quorum,
            pool_thresholds: &[],
            drep_thresholds: &[],
            committee_hot_keys: &empty_hot,
            drep_expiry: &empty_expiry,
            num_dormant: &dormant,
            current_pparams: cur_pparams,
            current_prev_pparam_action: cur_prev,
            new_epoch: 1341, // ending_epoch 1340
        };
        plan_conway_governance_epoch(&input, |_| true)
    }
    /// A `protocol_param_update` map carrying ONLY `maxTxExUnits = [mem, steps]` (map key 20).
    fn tx_update(mem: u64, steps: u64) -> Vec<u8> {
        let mut v = vec![0xa1, 0x14, 0x82, 0x1b];
        v.extend_from_slice(&mem.to_be_bytes());
        v.push(0x1b);
        v.extend_from_slice(&steps.to_be_bytes());
        v
    }

    #[test]
    fn cre_s4_3c_supported_witness_enacts_prunes_siblings_refunds_all() {
        let root = gaid(0x60);
        let cur_prev = PreviousPParamAction::Enacted(root.clone());
        let winner = prop(0x69, pc(Some(root.clone()), WITNESS_UPDATE.to_vec()), yes_2of3(), 1339);
        let mut proposals = vec![winner.clone()];
        for b in [0xF0u8, 0xF1, 0xF2, 0xF3, 0xF4] {
            // The five siblings share the superseded root, FAIL thresholds (no votes), and are pruned.
            proposals.push(prop(b, pc(Some(root.clone()), WITNESS_UPDATE.to_vec()), Vec::new(), 1339));
        }
        let plan = plan_enact(&proposals, &pp_1095(), &cur_prev).expect("the supported memory-only witness enacts");

        // (1) The two memory limits change; BOTH steps preserved.
        match &plan.pparams {
            PParamsDelta::Set(pp) => {
                assert_eq!(pp.max_tx_ex_units_mem, 16_500_000, "maxTx mem 14M -> 16.5M");
                assert_eq!(pp.max_tx_ex_units_cpu, 10_000_000_000, "maxTx steps preserved");
                assert_eq!(
                    pp.max_block_ex_units,
                    MaxBlockExUnits::Bound { mem: 72_000_000, steps: 20_000_000_000 },
                    "maxBlock mem 62M -> 72M, steps preserved"
                );
            }
            other => panic!("expected pparams Set, got {other:?}"),
        }
        // (2) The enacted root advances to the winner.
        assert_eq!(plan.prev_pparam_action, PrevPParamActionDelta::Set(winner.action_id.clone()));
        // (3) Six removals: 1 Enacted (winner, canonical-first) + 5 PrunedByEnactment (siblings).
        assert_eq!(plan.removals.len(), 6);
        assert_eq!(plan.removals[0].action_id, winner.action_id);
        assert_eq!(plan.removals[0].cause, RemovalCause::Enacted);
        assert_eq!(plan.removals.iter().filter(|r| r.cause == RemovalCause::Enacted).count(), 1);
        assert_eq!(plan.removals.iter().filter(|r| r.cause == RemovalCause::PrunedByEnactment).count(), 5);
        // (4) Every deposit (100k) refunds to its registered reward account.
        assert_eq!(plan.deposit_returns.len(), 6);
        assert!(plan.deposit_returns.iter().all(|d| matches!(
            d, DepositReturn::ToRewardAccount { amount: Coin(100_000_000_000), .. }
        )));
        // (5) The next proposal set: the whole competing forest leaves the set.
        assert!(plan.proposals.is_empty(), "winner + all siblings removed");
    }

    #[test]
    fn cre_s4_3c_changed_steps_is_unsupported_zero_mutation() {
        let root = gaid(0x60);
        // maxTx steps 5e9 != the current 10e9 -> a steps change (outside the memory-only subset).
        let winner = prop(0x69, pc(Some(root.clone()), tx_update(16_500_000, 5_000_000_000)), yes_2of3(), 1339);
        let err = plan_enact(&[winner], &pp_1095(), &PreviousPParamAction::Enacted(root)).unwrap_err();
        assert!(matches!(
            err, GovernanceTerminal::UnsupportedRatifiedAction { kind: UnsupportedActionKind::ChangedSteps, .. }
        ), "got {err:?}");
    }

    #[test]
    fn cre_s4_3c_foreign_pparam_field_is_unsupported() {
        let root = gaid(0x60);
        // A valid maxTx exec-units field RIDING ALONGSIDE a non-exec-units key (0 = minFeeA) -> fail-closed;
        // the exec part must NOT be applied when a foreign field is present.
        let mut update = vec![0xa2, 0x14, 0x82, 0x1b];
        update.extend_from_slice(&16_500_000u64.to_be_bytes());
        update.push(0x1b);
        update.extend_from_slice(&10_000_000_000u64.to_be_bytes());
        update.extend_from_slice(&[0x00, 0x18, 44]); // key 0 -> uint 44
        let winner = prop(0x69, pc(Some(root.clone()), update), yes_2of3(), 1339);
        let err = plan_enact(&[winner], &pp_1095(), &PreviousPParamAction::Enacted(root)).unwrap_err();
        assert!(matches!(
            err, GovernanceTerminal::UnsupportedRatifiedAction { kind: UnsupportedActionKind::NonExecUnitsField, .. }
        ), "got {err:?}");
    }

    #[test]
    fn cre_s4_3c_lineage_mismatch_carries_never_enacts() {
        let root = gaid(0x60);
        // Threshold-passing, but prev_action != the current root -> NOT ratifiable this boundary. Non-expiring
        // (1366 >= ending 1340) -> carried; no enactment, no terminal.
        let p = prop(0x69, pc(Some(gaid(0x99)), WITNESS_UPDATE.to_vec()), yes_2of3(), 1366);
        let plan = plan_enact(std::slice::from_ref(&p), &pp_1095(), &PreviousPParamAction::Enacted(root))
            .expect("a lineage-mismatched change carries (not a terminal)");
        assert_eq!(plan.pparams, PParamsDelta::Unchanged);
        assert_eq!(plan.prev_pparam_action, PrevPParamActionDelta::Unchanged);
        assert!(plan.removals.is_empty());
        assert_eq!(plan.proposals.len(), 1, "carried forward");
    }

    #[test]
    fn cre_s4_3c_unversioned_state_on_enact_path_is_terminal() {
        let root = gaid(0x60);
        let winner = prop(0x69, pc(Some(root.clone()), WITNESS_UPDATE.to_vec()), yes_2of3(), 1339);
        // (a) block ExUnits Unversioned (the pre-V11 `Default`) with a threshold-passing enact candidate.
        let err = plan_enact(
            std::slice::from_ref(&winner), &ProtocolParameters::default(),
            &PreviousPParamAction::Enacted(root.clone()),
        ).unwrap_err();
        assert!(matches!(err, GovernanceTerminal::UnversionedStateOnEnactPath { .. }), "block Unversioned: {err:?}");
        // (b) prev_pparam_action Unversioned (with Bound pparams).
        let err2 = plan_enact(std::slice::from_ref(&winner), &pp_1095(), &PreviousPParamAction::Unversioned)
            .unwrap_err();
        assert!(matches!(err2, GovernanceTerminal::UnversionedStateOnEnactPath { .. }), "prev Unversioned: {err2:?}");
    }

    #[test]
    fn cre_s4_3c_malformed_update_is_terminal() {
        let root = gaid(0x60);
        // Valid CBOR, but key 20's value is a uint, not array(2)[mem,steps].
        let winner = prop(0x69, pc(Some(root.clone()), vec![0xa1, 0x14, 0x01]), yes_2of3(), 1339);
        let err = plan_enact(&[winner], &pp_1095(), &PreviousPParamAction::Enacted(root)).unwrap_err();
        assert!(matches!(
            err, GovernanceTerminal::UnsupportedRatifiedAction { kind: UnsupportedActionKind::MalformedUpdate, .. }
        ), "got {err:?}");
    }

    #[test]
    fn cre_s4_3c_oversized_or_overnested_update_is_terminal() {
        let root = gaid(0x60);
        // (a) length > 4096 bytes -> rejected before the decoder runs.
        let big = prop(0x69, pc(Some(root.clone()), vec![0x00; 5000]), yes_2of3(), 1339);
        let err = plan_enact(std::slice::from_ref(&big), &pp_1095(), &PreviousPParamAction::Enacted(root.clone()))
            .unwrap_err();
        assert!(matches!(
            err, GovernanceTerminal::UnsupportedRatifiedAction { kind: UnsupportedActionKind::OversizedUpdate, .. }
        ), "oversized: {err:?}");
        // (b) CBOR nesting depth > 16 (20 nested arrays) -> rejected before the recursive-descent decoder.
        let nested = prop(0x69, pc(Some(root.clone()), vec![0x81; 20]), yes_2of3(), 1339);
        let err2 = plan_enact(std::slice::from_ref(&nested), &pp_1095(), &PreviousPParamAction::Enacted(root))
            .unwrap_err();
        assert!(matches!(
            err2, GovernanceTerminal::UnsupportedRatifiedAction { kind: UnsupportedActionKind::OversizedUpdate, .. }
        ), "over-nested: {err2:?}");
    }

    #[test]
    fn cre_s4_3c_ratifiable_chain_child_of_winner_halts() {
        let root = gaid(0x60);
        let winner = prop(0x69, pc(Some(root.clone()), WITNESS_UPDATE.to_vec()), yes_2of3(), 1339);
        // A child chaining onto the winner (prev = winner id) that ALSO passes thresholds -> Conway would
        // chain-enact (tip: root -> winner -> child); S4.3c enacts at most one -> ChainedEnactment, zero mutation.
        let child = prop(0x70, pc(Some(winner.action_id.clone()), WITNESS_UPDATE.to_vec()), yes_2of3(), 1339);
        let err = plan_enact(&[winner, child], &pp_1095(), &PreviousPParamAction::Enacted(root)).unwrap_err();
        assert!(matches!(
            err, GovernanceTerminal::UnsupportedRatifiedAction { kind: UnsupportedActionKind::ChainedEnactment, .. }
        ), "got {err:?}");
    }

    #[test]
    fn cre_s4_3c_pending_chain_child_is_carried_not_pruned() {
        // THE regression guard for the Phase-2 divergence: a NON-ratifiable child chaining onto the winner
        // (prev = winner, fails thresholds) is the NEW lineage -> Conway carries it. It must NOT be pruned as a
        // "descendant of the enacted action" (which would drop the proposal AND wrongly refund its deposit).
        let root = gaid(0x60);
        let winner = prop(0x69, pc(Some(root.clone()), WITNESS_UPDATE.to_vec()), yes_2of3(), 1339);
        let child = prop(0x70, pc(Some(winner.action_id.clone()), WITNESS_UPDATE.to_vec()), Vec::new(), 1366);
        let plan = plan_enact(&[winner.clone(), child.clone()], &pp_1095(), &PreviousPParamAction::Enacted(root))
            .expect("winner enacts; the pending chain child carries");
        assert!(matches!(plan.pparams, PParamsDelta::Set(_)), "the winner still enacts");
        // The winner enacts and is removed; the child is NOT removed and its deposit stays escrowed.
        assert_eq!(plan.removals.len(), 1, "only the winner leaves; the chain child is carried");
        assert_eq!(plan.removals[0].action_id, winner.action_id);
        assert_eq!(plan.removals[0].cause, RemovalCause::Enacted);
        assert_eq!(plan.deposit_returns.len(), 1, "the child's deposit is NOT refunded");
        assert_eq!(plan.proposals.len(), 1);
        assert_eq!(plan.proposals[0].action_id, child.action_id, "the chain child remains in the set");
    }

    #[test]
    fn cre_s4_3c_competing_ratifiable_siblings_halt() {
        // Two threshold-passing ParameterChanges sharing the current root both chain onto the tip; cardano-ledger
        // enacts the SUBMISSION-first (OMap order), which Ade cannot reconstruct from canonical state -> halt
        // (CompetingRatifiableActions), symmetric with the chain-child halt. NEVER a GovActionId-order guess that
        // could silently diverge. (The single-winner path is proven above where only one sibling passes votes.)
        let root = gaid(0x60);
        let a = prop(0x61, pc(Some(root.clone()), WITNESS_UPDATE.to_vec()), yes_2of3(), 1339);
        let b = prop(0x62, pc(Some(root.clone()), WITNESS_UPDATE.to_vec()), yes_2of3(), 1339);
        let err = plan_enact(&[a, b], &pp_1095(), &PreviousPParamAction::Enacted(root)).unwrap_err();
        assert!(
            matches!(
                err,
                GovernanceTerminal::UnsupportedRatifiedAction {
                    kind: UnsupportedActionKind::CompetingRatifiableActions,
                    ..
                }
            ),
            "got {err:?}"
        );
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
                &[], &[], 0, &empty_hot, &empty_drep_expiry, &DormantEpochs::Unversioned,
            )
            .expect("empty drep_expiry needs no dormancy offset");
            let ratified = res.ratified.iter().any(|q| q.action_id == p.action_id);
            let obs = proposal_ratification_observation(
                p, &empty_drep, &empty_pool, cm, &quorum,
                &[], &[], 0, &empty_hot, &empty_drep_expiry, &DormantEpochs::Unversioned,
            )
            .expect("empty drep_expiry needs no dormancy offset");
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
            &[], &[], 0, &empty_hot, &empty_drep_expiry, &DormantEpochs::Unversioned,
        )
        .expect("empty drep_expiry needs no dormancy offset");
        assert!(
            res.ratified.is_empty() && res.remaining.iter().any(|q| q.action_id == info.action_id),
            "InfoAction -> remaining, never ratified",
        );
        let obs = proposal_ratification_observation(
            &info, &empty_drep, &empty_pool, &committee, &quorum,
            &[], &[], 0, &empty_hot, &empty_drep_expiry, &DormantEpochs::Unversioned,
        )
        .expect("empty drep_expiry needs no dormancy offset");
        assert!(!obs.potentially_ratifiable && obs.is_info_action, "InfoAction never enacts (observer)");
    }

    // CPDE-S4: the whole-set deposit-expiry-refund PLANNER (pure; no mutation). new_epoch 1341 -> ending 1340.

    fn s4_committee() -> BTreeMap<StakeCredential, u64> {
        // 3 active members (term expiry 1400 >= ending_epoch 1340).
        [(key(0xC1), 1400u64), (key(0xC2), 1400), (key(0xC3), 1400)].into_iter().collect()
    }
    fn s4_plan(proposals: &[GovActionState]) -> Result<ConwayGovernanceEpochPlan, GovernanceTerminal> {
        let quorum = Rational::new(2, 3).unwrap();
        let empty_stake = BTreeMap::new();
        let empty_pool = BTreeMap::new();
        let committee = s4_committee();
        let empty_hot = BTreeMap::new();
        let empty_expiry = BTreeMap::new();
        let dormant = DormantEpochs::Unversioned;
        // These CPDE-S4 planner tests exercise only non-`ParameterChange` actions (treasury / no-confidence /
        // info) and expiry — the exec-units enact path (which reads the two below) is never reached, so
        // defaults suffice: a `NotParameterChange` terminal returns before the pparams/lineage are consulted.
        let cur_pparams = crate::pparams::ProtocolParameters::default();
        let cur_prev_action = crate::state::PreviousPParamAction::NoPreviousAction;
        let input = ConwayGovernanceEpochInput {
            proposals,
            drep_stake: &empty_stake,
            pool_stake: &empty_pool,
            committee_members: &committee,
            committee_quorum: &quorum,
            pool_thresholds: &[],
            drep_thresholds: &[],
            committee_hot_keys: &empty_hot,
            drep_expiry: &empty_expiry,
            num_dormant: &dormant,
            current_pparams: &cur_pparams,
            current_prev_pparam_action: &cur_prev_action,
            new_epoch: 1341,
        };
        // These planner unit tests don't model registration: treat every return address as registered so a
        // refundable deposit routes to its reward account (the old `credit = Some(cred)` semantics). The
        // deregistered->treasury routing is covered by the accumulator's `s4_boundary_deregistered_*` test.
        plan_conway_governance_epoch(&input, |_| true)
    }
    fn s4_prop(
        id: u8, action: GovAction, votes: Vec<(StakeCredential, Vote)>,
        expires_after: u64, deposit: u64, return_addr: Vec<u8>,
    ) -> GovActionState {
        GovActionState {
            action_id: GovActionId { tx_hash: Hash32([id; 32]), index: 0 },
            committee_votes: votes,
            drep_votes: Vec::new(),
            spo_votes: Vec::new(),
            deposit: Coin(deposit),
            return_addr,
            gov_action: action,
            proposed_in: EpochNo(1309),
            expires_after: EpochNo(expires_after),
        }
    }
    fn tw_action() -> GovAction {
        GovAction::TreasuryWithdrawals { withdrawals: Vec::new(), policy_hash: None }
    }

    #[test]
    fn s4_unvoted_expiring_tw_refunds_to_return_address() {
        let p = s4_prop(0x01, tw_action(), Vec::new(), 1339, 100_000_000_000, vec![0xe0; 29]);
        let plan = s4_plan(std::slice::from_ref(&p)).expect("clean plan");
        assert_eq!(plan.removals.len(), 1);
        assert_eq!(plan.removals[0].action_id, p.action_id);
        assert_eq!(plan.removals[0].cause, RemovalCause::Expired);
        assert_eq!(
            plan.deposit_returns[0],
            DepositReturn::ToRewardAccount {
                action_id: p.action_id.clone(),
                credential: StakeCredential::KeyHash(Hash28([0xe0; 28])),
                amount: Coin(100_000_000_000),
            },
            "expired + committee-fail -> refund to the return-address key-hash credential",
        );
    }

    #[test]
    fn s4_committee_pass_is_terminal() {
        // 2 of 3 committee Yes (= 2/3 >= quorum) -> a ratified TreasuryWithdrawals (non-ParameterChange) -> the
        // whole boundary terminals (UnsupportedRatifiedAction / NotParameterChange), zero mutation.
        let p = s4_prop(0x01, tw_action(), vec![(key(0xC1), Vote::Yes), (key(0xC2), Vote::Yes)], 1339, 1, vec![0xe0; 29]);
        assert!(matches!(
            s4_plan(std::slice::from_ref(&p)).unwrap_err(),
            GovernanceTerminal::UnsupportedRatifiedAction { kind: UnsupportedActionKind::NotParameterChange, .. }
        ));
    }

    #[test]
    fn s4_noconfidence_skips_committee_then_empty_thresholds_is_terminal() {
        // NoConfidence skips the committee gate; empty drep/pool thresholds -> all gates skipped -> passes
        // -> potentially ratifiable -> terminal (committee-only authority cannot disprove it). The census
        // proved no such proposal exists in the CE-3d set; this pins the fail-closed behavior if one did.
        let p = s4_prop(0x01, GovAction::NoConfidence { prev_action: None }, Vec::new(), 1339, 1, vec![0xe0; 29]);
        assert!(matches!(
            s4_plan(std::slice::from_ref(&p)).unwrap_err(),
            GovernanceTerminal::UnsupportedRatifiedAction { kind: UnsupportedActionKind::NotParameterChange, .. }
        ));
    }

    #[test]
    fn s4_malformed_return_addr_on_refund_is_terminal() {
        // Expiring + unratifiable, but the return address is not a 29-byte reward account -> terminal.
        let p = s4_prop(0x01, tw_action(), Vec::new(), 1339, 100_000_000_000, vec![0xe0; 20]);
        assert!(matches!(s4_plan(std::slice::from_ref(&p)).unwrap_err(), GovernanceTerminal::Malformed { .. }));
    }

    #[test]
    fn s4_non_expiring_unratifiable_carries_forward() {
        // Provably unratifiable but NOT expiring (1366 >= ending 1340) -> no refund, no terminal.
        let p = s4_prop(0x01, tw_action(), Vec::new(), 1366, 100_000_000_000, vec![0xe0; 29]);
        assert!(s4_plan(std::slice::from_ref(&p)).expect("clean plan").removals.is_empty());
    }

    #[test]
    fn s4_info_action_expiring_refunds_deposit_else_removed_without_credit() {
        // InfoAction never enacts -> provably unratifiable; expiring + deposit > 0 -> refund.
        let p = s4_prop(0x01, GovAction::InfoAction, Vec::new(), 1339, 100_000_000_000, vec![0xe0; 29]);
        let plan = s4_plan(std::slice::from_ref(&p)).expect("clean plan");
        assert_eq!(plan.removals.len(), 1);
        assert!(matches!(plan.deposit_returns[0], DepositReturn::ToRewardAccount { .. }), "an InfoAction that carried a deposit is refunded");
        // 0-deposit -> removed, no credit (the protocol never escrowed a deposit to return).
        let z = s4_prop(0x02, GovAction::InfoAction, Vec::new(), 1339, 0, vec![0xe0; 29]);
        let plan = s4_plan(std::slice::from_ref(&z)).expect("clean plan");
        assert_eq!(plan.removals.len(), 1);
        assert!(matches!(plan.deposit_returns[0], DepositReturn::NoDeposit { .. }));
    }

    #[test]
    fn s4_script_hash_return_addr_credits_script_credential() {
        let mut addr = vec![0xf0u8]; // 0xf0 & 0x10 != 0 -> ScriptHash
        addr.extend_from_slice(&[0xab; 28]);
        let p = s4_prop(0x01, tw_action(), Vec::new(), 1339, 100_000_000_000, addr);
        let plan = s4_plan(std::slice::from_ref(&p)).expect("clean plan");
        assert_eq!(
            plan.deposit_returns[0],
            DepositReturn::ToRewardAccount {
                action_id: p.action_id.clone(),
                credential: StakeCredential::ScriptHash(Hash28([0xab; 28])),
                amount: Coin(100_000_000_000),
            },
            "a 0xF_ reward account credits the SCRIPT-hash credential, not a key-hash projection",
        );
    }

    #[test]
    fn s4_whole_set_one_ratifiable_makes_the_whole_plan_terminal_else_gov_action_id_ordered() {
        let r_late = s4_prop(0x03, tw_action(), Vec::new(), 1339, 1, vec![0xe0; 29]);
        let r_early = s4_prop(0x01, tw_action(), Vec::new(), 1339, 1, vec![0xe0; 29]);
        let ratifiable = s4_prop(0x02, tw_action(), vec![(key(0xC1), Vote::Yes), (key(0xC2), Vote::Yes)], 1339, 1, vec![0xe0; 29]);
        // ANY ratified action -> the WHOLE plan is terminal (zero mutation), even though two others would refund.
        // Here the ratified one is a TreasuryWithdrawals (non-ParameterChange) -> UnsupportedRatifiedAction.
        assert!(matches!(
            s4_plan(&[r_late.clone(), r_early.clone(), ratifiable]).unwrap_err(),
            GovernanceTerminal::UnsupportedRatifiedAction { kind: UnsupportedActionKind::NotParameterChange, .. }
        ));
        // Without it, the two refunds come back in GovActionId order (0x01 before 0x03).
        let plan = s4_plan(&[r_late, r_early]).expect("clean plan");
        assert_eq!(plan.removals.len(), 2);
        assert_eq!(plan.removals[0].action_id.tx_hash, Hash32([0x01; 32]), "GovActionId order");
        assert_eq!(plan.removals[1].action_id.tx_hash, Hash32([0x03; 32]));
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
                prev_pparam_action: crate::state::PreviousPParamAction::Unversioned,
                proposals: Vec::new(),
                committee,
                committee_quorum: quorum,
                drep_expiry: Default::default(),
                gov_action_lifetime: 6,
                vote_delegations: Default::default(),
                pool_voting_thresholds: Vec::new(),
                drep_voting_thresholds: Vec::new(),
                committee_hot_keys: Default::default(),
                num_dormant: crate::state::DormantEpochs::Unversioned,
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod drep_voting_stake_derivation_tests {
    use super::*;
    use ade_types::shelley::cert::StakeCredential;
    use ade_types::tx::PoolId;
    use ade_types::Hash28;

    fn key(b: u8) -> StakeCredential {
        StakeCredential::KeyHash(Hash28([b; 28]))
    }

    /// A mark snapshot where each `(byte, coin)` is a credential (hash = `[byte;28]`) with `coin` stake
    /// delegated to a throwaway pool. Only `delegations` matters to the DRep derivation.
    fn mark_with(entries: &[(u8, u64)]) -> StakeSnapshot {
        let mut delegations = BTreeMap::new();
        for (b, coin) in entries {
            delegations.insert(Hash28([*b; 28]), (PoolId(Hash28([0xEE; 28])), Coin(*coin)));
        }
        StakeSnapshot { delegations, pool_stakes: BTreeMap::new() }
    }

    #[test]
    fn sums_delegator_mark_stake_per_drep() {
        let drep_a = DRep::KeyHash(Hash28([0xAA; 28]));
        let drep_b = DRep::ScriptHash(Hash28([0xBB; 28]));
        let vd: BTreeMap<StakeCredential, DRep> =
            [(key(1), drep_a.clone()), (key(2), drep_a.clone()), (key(3), drep_b.clone())]
                .into_iter()
                .collect();
        let d = derive_drep_voting_stake(&vd, &mark_with(&[(1, 100), (2, 250), (3, 70)]));
        assert_eq!(d.get(&drep_a), Some(&350), "DRep A = 100 + 250");
        assert_eq!(d.get(&drep_b), Some(&70));
        assert_eq!(d.len(), 2);
    }

    #[test]
    fn zero_and_absent_stake_contribute_nothing() {
        let drep = DRep::KeyHash(Hash28([0xAA; 28]));
        // cred 1 absent from mark; cred 2 has zero stake; cred 3 has real stake.
        let vd: BTreeMap<StakeCredential, DRep> =
            [(key(1), drep.clone()), (key(2), drep.clone()), (key(3), drep.clone())]
                .into_iter()
                .collect();
        let d = derive_drep_voting_stake(&vd, &mark_with(&[(2, 0), (3, 500)]));
        assert_eq!(d.get(&drep), Some(&500), "only the positive-stake delegator counts");
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn empty_delegations_yield_empty_distribution() {
        // The live path (import-not-activate) feeds empty vote_delegations -> no distribution -> no gate.
        let d = derive_drep_voting_stake(&BTreeMap::new(), &mark_with(&[(1, 100)]));
        assert!(d.is_empty());
    }

    #[test]
    fn always_abstain_is_derived_raw_but_filtered_from_the_active_denominator() {
        // The derivation records AlwaysAbstain's raw delegated stake; the SEPARATE active-denominator filter
        // (the single `active_drep_stake_filtered`) is what excludes it downstream — no second filter.
        let vd: BTreeMap<StakeCredential, DRep> =
            [(key(1), DRep::AlwaysAbstain), (key(2), DRep::KeyHash(Hash28([0xAA; 28])))]
                .into_iter()
                .collect();
        let raw = derive_drep_voting_stake(&vd, &mark_with(&[(1, 900), (2, 100)]));
        assert_eq!(raw.get(&DRep::AlwaysAbstain), Some(&900), "derivation keeps the raw delegated stake");
        let (active, total) =
            active_drep_stake_filtered(&raw, &BTreeMap::new(), &DormantEpochs::Unversioned, 0).unwrap();
        assert_eq!(total, 100, "the active denominator excludes AlwaysAbstain");
        assert!(!active.contains_key(&DRep::AlwaysAbstain));
    }

    #[test]
    fn derivation_is_replay_deterministic() {
        let drep = DRep::KeyHash(Hash28([0x11; 28]));
        let vd: BTreeMap<StakeCredential, DRep> = (0u8..50).map(|b| (key(b), drep.clone())).collect();
        let entries: Vec<(u8, u64)> = (0u8..50).map(|b| (b, (b as u64 + 1) * 7)).collect();
        let mark = mark_with(&entries);
        assert_eq!(derive_drep_voting_stake(&vd, &mark), derive_drep_voting_stake(&vd, &mark));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod s4_1_dormant_fail_closed_tests {
    use super::*;
    use crate::state::DormantEpochs;

    fn one_drep_stake() -> (DRepStakeDistribution, BTreeMap<StakeCredential, u64>) {
        let drep = DRep::KeyHash(Hash28([0x11; 28]));
        let stake: DRepStakeDistribution = [(drep, 100u64)].into_iter().collect();
        let mut expiry = BTreeMap::new();
        expiry.insert(StakeCredential::KeyHash(Hash28([0x11; 28])), 300u64);
        (stake, expiry)
    }

    /// GATE (missing dormant fails): a NON-EMPTY drep_expiry means the dormancy offset WOULD be applied, but
    /// a V1 (`Unversioned`) state cannot supply it — TERMINAL, never a fabricated 0.
    #[test]
    fn unversioned_with_live_drep_expiry_is_terminal() {
        let (stake, expiry) = one_drep_stake();
        assert_eq!(
            active_drep_stake_filtered(&stake, &expiry, &DormantEpochs::Unversioned, 305),
            Err(DormantRequired),
        );
    }

    /// GATE (no needless failure): with an EMPTY drep_expiry no DRep is expiry-checked, so the offset is
    /// never applied and a V1 state is fine (no fabrication needed).
    #[test]
    fn unversioned_with_empty_drep_expiry_is_ok() {
        let (stake, _) = one_drep_stake();
        let (_a, total) =
            active_drep_stake_filtered(&stake, &BTreeMap::new(), &DormantEpochs::Unversioned, 305).unwrap();
        assert_eq!(total, 100);
    }

    /// GATE (the offset is authoritative): a DRep expiring at 300 is EXCLUDED at epoch 305 with `Bound(0)`
    /// but INCLUDED with `Bound(10)` (300 + 10 >= 305) — the dormancy offset shifts the active denominator.
    #[test]
    fn bound_applies_the_dormancy_offset() {
        let (stake, expiry) = one_drep_stake();
        let (_a0, t0) =
            active_drep_stake_filtered(&stake, &expiry, &DormantEpochs::Bound(0), 305).unwrap();
        assert_eq!(t0, 0, "expired (300 < 305), no offset → excluded");
        let (_a10, t10) =
            active_drep_stake_filtered(&stake, &expiry, &DormantEpochs::Bound(10), 305).unwrap();
        assert_eq!(t10, 100, "300 + 10 >= 305 → still active");
    }
}
