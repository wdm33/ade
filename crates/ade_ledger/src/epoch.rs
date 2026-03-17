// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeMap;
use ade_types::tx::{Coin, PoolId};
use ade_types::{CardanoEra, EpochNo, Hash28};

use crate::error::{
    EpochError, EpochFailureReason, LedgerError,
};
use crate::pparams::ProtocolParameters;
use crate::rational::Rational;
use crate::state::LedgerState;

// ---------------------------------------------------------------------------
// S-14: Snapshot types and rotation
// ---------------------------------------------------------------------------

/// Stake distribution snapshot at a given epoch boundary.
///
/// Maps stake credential hash -> delegated pool + lovelace amount.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeSnapshot {
    /// Stake credential -> (pool, lovelace).
    pub delegations: BTreeMap<Hash28, (PoolId, Coin)>,
    /// Pool -> total delegated stake.
    pub pool_stakes: BTreeMap<PoolId, Coin>,
}

impl StakeSnapshot {
    pub fn new() -> Self {
        StakeSnapshot {
            delegations: BTreeMap::new(),
            pool_stakes: BTreeMap::new(),
        }
    }
}

impl Default for StakeSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

/// Mark snapshot — taken at the current epoch boundary.
/// This becomes the "set" snapshot in the next epoch, and "go" in the one after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkSnapshot(pub StakeSnapshot);

/// Set snapshot — the stake distribution used for leader schedule computation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetSnapshot(pub StakeSnapshot);

/// Go snapshot — the stake distribution used for reward computation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoSnapshot(pub StakeSnapshot);

/// Snapshot state maintained across epoch boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotState {
    pub mark: MarkSnapshot,
    pub set: SetSnapshot,
    pub go: GoSnapshot,
}

impl SnapshotState {
    pub fn new() -> Self {
        SnapshotState {
            mark: MarkSnapshot(StakeSnapshot::new()),
            set: SetSnapshot(StakeSnapshot::new()),
            go: GoSnapshot(StakeSnapshot::new()),
        }
    }
}

impl Default for SnapshotState {
    fn default() -> Self {
        Self::new()
    }
}

/// Rotate snapshots at an epoch boundary.
///
/// The rotation pipeline is:
///   go   <- set
///   set  <- mark
///   mark <- new_mark (freshly computed from current ledger state)
///
/// This is a pure function: old snapshots in, new snapshots out.
pub fn rotate_snapshots(
    current: &SnapshotState,
    new_mark: StakeSnapshot,
) -> SnapshotState {
    SnapshotState {
        mark: MarkSnapshot(new_mark),
        set: SetSnapshot(current.mark.0.clone()),
        go: GoSnapshot(current.set.0.clone()),
    }
}

// ---------------------------------------------------------------------------
// S-15: Reward computation
// ---------------------------------------------------------------------------

/// Per-pool reward distribution result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolRewards {
    /// Pool operator reward in lovelace.
    pub operator_reward: Coin,
    /// Member rewards: credential hash -> lovelace.
    pub member_rewards: BTreeMap<Hash28, Coin>,
}

/// Compute total rewards available for distribution in an epoch.
///
/// Based on the Shelley reward formula:
///   total_reward = reserves * rho
///
/// Where rho is the monetary expansion parameter.
/// All arithmetic is integer-based via Rational.
pub fn compute_total_reward(
    reserves: Coin,
    rho: &Rational,
) -> Option<Coin> {
    let reserves_rat = Rational::from_integer(reserves.0 as i128);
    let total = reserves_rat.checked_mul(rho)?;
    let floored = total.floor();
    if floored < 0 {
        return Some(Coin(0));
    }
    Some(Coin(floored as u64))
}

/// Compute the pool reward for a single pool.
///
/// Shelley pool reward formula (simplified):
///   pool_reward = total_reward * (pool_stake / total_stake) * pool_performance
///
/// pool_performance is 1 for fully performing pools (simplified for now).
/// The operator takes their margin + cost, rest distributed to delegators pro-rata.
pub fn compute_pool_reward(
    total_reward: Coin,
    pool_stake: Coin,
    total_stake: Coin,
    pool_cost: Coin,
    pool_margin: &Rational,
    delegator_stakes: &BTreeMap<Hash28, Coin>,
) -> Option<PoolRewards> {
    if total_stake.0 == 0 {
        return Some(PoolRewards {
            operator_reward: Coin(0),
            member_rewards: BTreeMap::new(),
        });
    }

    // pool_share = pool_stake / total_stake
    let pool_share = Rational::new(pool_stake.0 as i128, total_stake.0 as i128)?;

    // raw_pool_reward = total_reward * pool_share
    let total_rat = Rational::from_integer(total_reward.0 as i128);
    let raw_pool_reward = total_rat.checked_mul(&pool_share)?;
    let raw_pool_reward_coin = raw_pool_reward.floor().max(0) as u64;

    // If raw reward is less than cost, operator takes everything
    if raw_pool_reward_coin <= pool_cost.0 {
        return Some(PoolRewards {
            operator_reward: Coin(raw_pool_reward_coin),
            member_rewards: BTreeMap::new(),
        });
    }

    // After cost deduction
    let after_cost = raw_pool_reward_coin.saturating_sub(pool_cost.0);

    // Margin for operator: after_cost * margin
    let after_cost_rat = Rational::from_integer(after_cost as i128);
    let margin_reward = after_cost_rat.checked_mul(pool_margin)?;
    let margin_coin = margin_reward.floor().max(0) as u64;

    // Operator gets cost + margin
    let operator_reward = pool_cost.0.saturating_add(margin_coin);

    // Remaining for delegators
    let delegator_pool = after_cost.saturating_sub(margin_coin);

    // Distribute to delegators pro-rata by their stake
    let mut member_rewards = BTreeMap::new();
    let mut distributed: u64 = 0;

    if pool_stake.0 > 0 && delegator_pool > 0 {
        let delegator_pool_rat = Rational::from_integer(delegator_pool as i128);

        for (cred, stake) in delegator_stakes {
            let member_share = Rational::new(stake.0 as i128, pool_stake.0 as i128)?;
            let member_reward = delegator_pool_rat.checked_mul(&member_share)?;
            let member_coin = member_reward.floor().max(0) as u64;
            if member_coin > 0 {
                distributed = distributed.saturating_add(member_coin);
                member_rewards.insert(cred.clone(), Coin(member_coin));
            }
        }
    }

    // Any rounding dust goes to the operator
    let dust = delegator_pool.saturating_sub(distributed);
    let final_operator = operator_reward.saturating_add(dust);

    Some(PoolRewards {
        operator_reward: Coin(final_operator),
        member_rewards,
    })
}

/// Compute rewards for all pools in an epoch.
///
/// This is a skeleton that will be expanded with full pool parameters.
/// Returns a map of pool -> pool rewards.
pub fn compute_rewards(
    total_reward: Coin,
    total_stake: Coin,
    go_snapshot: &GoSnapshot,
    pool_params: &BTreeMap<PoolId, PoolParams>,
) -> Result<BTreeMap<PoolId, PoolRewards>, LedgerError> {
    let mut all_rewards = BTreeMap::new();

    for (pool_id, pool_stake) in &go_snapshot.0.pool_stakes {
        let params = match pool_params.get(pool_id) {
            Some(p) => p,
            None => continue, // Pool not registered, skip
        };

        // Gather delegator stakes for this pool
        let delegator_stakes: BTreeMap<Hash28, Coin> = go_snapshot
            .0
            .delegations
            .iter()
            .filter(|(_, (pid, _))| pid == pool_id)
            .map(|(cred, (_, coin))| (cred.clone(), *coin))
            .collect();

        let pool_rewards = compute_pool_reward(
            total_reward,
            *pool_stake,
            total_stake,
            params.cost,
            &params.margin,
            &delegator_stakes,
        )
        .ok_or(LedgerError::EpochTransition(EpochError {
                epoch: EpochNo(0),
                era: CardanoEra::Shelley,
                reason: EpochFailureReason::RewardOverflow,
        }))?;

        all_rewards.insert(pool_id.clone(), pool_rewards);
    }

    Ok(all_rewards)
}

/// Minimal pool parameters needed for reward computation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolParams {
    pub cost: Coin,
    pub margin: Rational,
    pub pledge: Coin,
    pub reward_account: Hash28,
}

// ---------------------------------------------------------------------------
// S-17: Pool retirement
// ---------------------------------------------------------------------------

/// Pool retirement entry: pool retires at the end of the given epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolRetirement {
    pub pool_id: PoolId,
    pub retirement_epoch: EpochNo,
}

/// Retire pools that are scheduled for retirement at the given epoch.
///
/// Returns:
/// - Updated pool parameters map (retired pools removed)
/// - Updated snapshot state (retired pool stakes removed from all snapshots)
/// - List of pool IDs that were retired
///
/// Pure function: all inputs in, all outputs out. No mutation.
pub fn retire_pools(
    current_epoch: EpochNo,
    pool_params: &BTreeMap<PoolId, PoolParams>,
    retirements: &[PoolRetirement],
    snapshots: &SnapshotState,
) -> (BTreeMap<PoolId, PoolParams>, SnapshotState, Vec<PoolId>) {
    let mut retired_ids = Vec::new();

    // Find pools due for retirement
    for retirement in retirements {
        if retirement.retirement_epoch <= current_epoch {
            retired_ids.push(retirement.pool_id.clone());
        }
    }

    // Sort for deterministic ordering
    retired_ids.sort();

    // Remove retired pools from pool params
    let mut new_params = pool_params.clone();
    for pool_id in &retired_ids {
        new_params.remove(pool_id);
    }

    // Remove retired pools from snapshots
    let new_mark = remove_pool_from_snapshot(&snapshots.mark.0, &retired_ids);
    let new_set = remove_pool_from_snapshot(&snapshots.set.0, &retired_ids);
    let new_go = remove_pool_from_snapshot(&snapshots.go.0, &retired_ids);

    let new_snapshots = SnapshotState {
        mark: MarkSnapshot(new_mark),
        set: SetSnapshot(new_set),
        go: GoSnapshot(new_go),
    };

    (new_params, new_snapshots, retired_ids)
}

/// Remove a set of pools from a stake snapshot.
fn remove_pool_from_snapshot(
    snapshot: &StakeSnapshot,
    retired_pool_ids: &[PoolId],
) -> StakeSnapshot {
    let mut new_delegations = snapshot.delegations.clone();
    let mut new_pool_stakes = snapshot.pool_stakes.clone();

    // Remove pool entries
    for pool_id in retired_pool_ids {
        new_pool_stakes.remove(pool_id);
    }

    // Remove delegations to retired pools
    new_delegations.retain(|_, (pool_id, _)| {
        !retired_pool_ids.contains(pool_id)
    });

    StakeSnapshot {
        delegations: new_delegations,
        pool_stakes: new_pool_stakes,
    }
}

// ---------------------------------------------------------------------------
// Epoch boundary orchestration
// ---------------------------------------------------------------------------

/// Apply a full epoch boundary transition.
///
/// Orchestrates all epoch boundary operations in the correct order:
/// 1. Rotate snapshots
/// 2. Compute rewards (using go snapshot)
/// 3. Retire pools
/// 4. Apply protocol parameter updates
///
/// This is a skeleton that coordinates the individual pure functions.
pub fn apply_epoch_boundary(
    state: &LedgerState,
    new_epoch: EpochNo,
    new_mark: StakeSnapshot,
    pool_params: &BTreeMap<PoolId, PoolParams>,
    retirements: &[PoolRetirement],
    _pending_pp_updates: &BTreeMap<Hash28, ProtocolParameters>,
) -> Result<EpochBoundaryResult, LedgerError> {
    // 1. Rotate snapshots
    let snapshots = &state.epoch_state.snapshots;
    let rotated_snapshots = rotate_snapshots(snapshots, new_mark);

    // 2. Compute total active stake from go snapshot
    let total_stake = compute_total_active_stake(&rotated_snapshots.go);

    // 3. Compute total reward available
    let reserves = state.epoch_state.reserves;
    let rho = Rational::new(3, 1000); // 0.3% monetary expansion (Shelley default)
    let total_reward = match rho {
        Some(r) => compute_total_reward(reserves, &r).unwrap_or(Coin(0)),
        None => Coin(0),
    };

    // 4. Compute per-pool rewards
    let rewards = compute_rewards(
        total_reward,
        total_stake,
        &rotated_snapshots.go,
        pool_params,
    )?;

    // 5. Retire pools
    let (new_pool_params, final_snapshots, retired) =
        retire_pools(new_epoch, pool_params, retirements, &rotated_snapshots);

    // 6. Update reserves: subtract distributed rewards
    let total_distributed = sum_distributed_rewards(&rewards);
    let new_reserves = Coin(reserves.0.saturating_sub(total_distributed.0));

    Ok(EpochBoundaryResult {
        new_epoch,
        snapshots: final_snapshots,
        rewards,
        pool_params: new_pool_params,
        retired_pools: retired,
        new_reserves,
        total_reward,
    })
}

/// Result of applying an epoch boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochBoundaryResult {
    pub new_epoch: EpochNo,
    pub snapshots: SnapshotState,
    pub rewards: BTreeMap<PoolId, PoolRewards>,
    pub pool_params: BTreeMap<PoolId, PoolParams>,
    pub retired_pools: Vec<PoolId>,
    pub new_reserves: Coin,
    pub total_reward: Coin,
}

/// Compute total active stake from the go snapshot.
fn compute_total_active_stake(go: &GoSnapshot) -> Coin {
    let mut total: u64 = 0;
    for stake in go.0.pool_stakes.values() {
        total = total.saturating_add(stake.0);
    }
    Coin(total)
}

/// Sum all distributed rewards across all pools.
fn sum_distributed_rewards(rewards: &BTreeMap<PoolId, PoolRewards>) -> Coin {
    let mut total: u64 = 0;
    for pool_rewards in rewards.values() {
        total = total.saturating_add(pool_rewards.operator_reward.0);
        for member_reward in pool_rewards.member_rewards.values() {
            total = total.saturating_add(member_reward.0);
        }
    }
    Coin(total)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::state::EpochState;

    fn make_pool_id(byte: u8) -> PoolId {
        PoolId(Hash28([byte; 28]))
    }

    fn make_cred(byte: u8) -> Hash28 {
        Hash28([byte; 28])
    }

    // -----------------------------------------------------------------------
    // S-14: Snapshot rotation tests
    // -----------------------------------------------------------------------

    #[test]
    fn rotate_snapshots_pipeline() {
        let mut mark_snap = StakeSnapshot::new();
        mark_snap.pool_stakes.insert(make_pool_id(0x01), Coin(100));

        let mut set_snap = StakeSnapshot::new();
        set_snap.pool_stakes.insert(make_pool_id(0x02), Coin(200));

        let mut go_snap = StakeSnapshot::new();
        go_snap.pool_stakes.insert(make_pool_id(0x03), Coin(300));

        let state = SnapshotState {
            mark: MarkSnapshot(mark_snap),
            set: SetSnapshot(set_snap),
            go: GoSnapshot(go_snap),
        };

        let mut new_mark = StakeSnapshot::new();
        new_mark.pool_stakes.insert(make_pool_id(0x04), Coin(400));

        let rotated = rotate_snapshots(&state, new_mark);

        // new mark = 0x04
        assert!(rotated.mark.0.pool_stakes.contains_key(&make_pool_id(0x04)));
        // new set = old mark = 0x01
        assert!(rotated.set.0.pool_stakes.contains_key(&make_pool_id(0x01)));
        // new go = old set = 0x02
        assert!(rotated.go.0.pool_stakes.contains_key(&make_pool_id(0x02)));
    }

    #[test]
    fn rotate_empty_snapshots() {
        let state = SnapshotState::new();
        let new_mark = StakeSnapshot::new();
        let rotated = rotate_snapshots(&state, new_mark);
        assert!(rotated.mark.0.pool_stakes.is_empty());
        assert!(rotated.set.0.pool_stakes.is_empty());
        assert!(rotated.go.0.pool_stakes.is_empty());
    }

    #[test]
    fn snapshot_rotation_is_deterministic() {
        let mut mark = StakeSnapshot::new();
        mark.pool_stakes.insert(make_pool_id(0x01), Coin(100));
        let state = SnapshotState {
            mark: MarkSnapshot(mark.clone()),
            set: SetSnapshot(StakeSnapshot::new()),
            go: GoSnapshot(StakeSnapshot::new()),
        };

        let new_mark = StakeSnapshot::new();
        let r1 = rotate_snapshots(&state, new_mark.clone());
        let r2 = rotate_snapshots(&state, new_mark);
        assert_eq!(r1, r2);
    }

    // -----------------------------------------------------------------------
    // S-15: Reward computation tests
    // -----------------------------------------------------------------------

    #[test]
    fn compute_total_reward_basic() {
        let reserves = Coin(1_000_000_000);
        let rho = Rational::new(3, 1000).unwrap(); // 0.3%
        let reward = compute_total_reward(reserves, &rho).unwrap();
        // floor(1_000_000_000 * 3/1000) = floor(3_000_000) = 3_000_000
        assert_eq!(reward, Coin(3_000_000));
    }

    #[test]
    fn compute_total_reward_zero_reserves() {
        let reserves = Coin(0);
        let rho = Rational::new(3, 1000).unwrap();
        let reward = compute_total_reward(reserves, &rho).unwrap();
        assert_eq!(reward, Coin(0));
    }

    #[test]
    fn compute_pool_reward_single_delegator() {
        let total_reward = Coin(1_000_000);
        let pool_stake = Coin(500_000);
        let total_stake = Coin(1_000_000);
        let pool_cost = Coin(10_000);
        let pool_margin = Rational::new(1, 10).unwrap(); // 10%

        let mut delegator_stakes = BTreeMap::new();
        delegator_stakes.insert(make_cred(0x01), Coin(500_000));

        let rewards = compute_pool_reward(
            total_reward,
            pool_stake,
            total_stake,
            pool_cost,
            &pool_margin,
            &delegator_stakes,
        )
        .unwrap();

        // pool share = 500000/1000000 = 0.5
        // raw pool reward = floor(1000000 * 0.5) = 500000
        // after cost = 500000 - 10000 = 490000
        // margin = floor(490000 * 0.1) = 49000
        // operator = 10000 + 49000 = 59000
        // delegator pool = 490000 - 49000 = 441000
        // member share = 500000/500000 = 1.0
        // member reward = floor(441000 * 1.0) = 441000
        assert_eq!(rewards.member_rewards[&make_cred(0x01)], Coin(441_000));
        assert_eq!(rewards.operator_reward, Coin(59_000));

        // Total should equal raw pool reward
        let total = rewards.operator_reward.0
            + rewards.member_rewards.values().map(|c| c.0).sum::<u64>();
        assert_eq!(total, 500_000);
    }

    #[test]
    fn compute_pool_reward_zero_stake() {
        let rewards = compute_pool_reward(
            Coin(1_000_000),
            Coin(0),
            Coin(0),
            Coin(0),
            &Rational::zero(),
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(rewards.operator_reward, Coin(0));
        assert!(rewards.member_rewards.is_empty());
    }

    #[test]
    fn compute_pool_reward_reward_less_than_cost() {
        let total_reward = Coin(100);
        let pool_stake = Coin(100);
        let total_stake = Coin(10_000);
        let pool_cost = Coin(500); // cost > raw reward

        let mut delegator_stakes = BTreeMap::new();
        delegator_stakes.insert(make_cred(0x01), Coin(100));

        let rewards = compute_pool_reward(
            total_reward,
            pool_stake,
            total_stake,
            pool_cost,
            &Rational::new(1, 10).unwrap(),
            &delegator_stakes,
        )
        .unwrap();

        // raw pool reward = floor(100 * 100/10000) = floor(1) = 1
        // 1 <= 500 (cost), so operator takes everything
        assert_eq!(rewards.operator_reward, Coin(1));
        assert!(rewards.member_rewards.is_empty());
    }

    #[test]
    fn compute_rewards_deterministic() {
        let mut pool_stakes = BTreeMap::new();
        pool_stakes.insert(make_pool_id(0x01), Coin(500));
        pool_stakes.insert(make_pool_id(0x02), Coin(500));

        let mut delegations = BTreeMap::new();
        delegations.insert(make_cred(0xaa), (make_pool_id(0x01), Coin(500)));
        delegations.insert(make_cred(0xbb), (make_pool_id(0x02), Coin(500)));

        let go = GoSnapshot(StakeSnapshot {
            delegations,
            pool_stakes,
        });

        let mut params = BTreeMap::new();
        params.insert(make_pool_id(0x01), PoolParams {
            cost: Coin(10),
            margin: Rational::new(1, 10).unwrap(),
            pledge: Coin(100),
            reward_account: make_cred(0x01),
        });
        params.insert(make_pool_id(0x02), PoolParams {
            cost: Coin(10),
            margin: Rational::new(1, 10).unwrap(),
            pledge: Coin(100),
            reward_account: make_cred(0x02),
        });

        let r1 = compute_rewards(Coin(1000), Coin(1000), &go, &params).unwrap();
        let r2 = compute_rewards(Coin(1000), Coin(1000), &go, &params).unwrap();
        assert_eq!(r1, r2);
    }

    // -----------------------------------------------------------------------
    // S-17: Pool retirement tests
    // -----------------------------------------------------------------------

    #[test]
    fn retire_pools_removes_due_pools() {
        let mut params = BTreeMap::new();
        params.insert(make_pool_id(0x01), PoolParams {
            cost: Coin(100),
            margin: Rational::new(1, 10).unwrap(),
            pledge: Coin(500),
            reward_account: make_cred(0x01),
        });
        params.insert(make_pool_id(0x02), PoolParams {
            cost: Coin(200),
            margin: Rational::new(1, 5).unwrap(),
            pledge: Coin(1000),
            reward_account: make_cred(0x02),
        });

        let retirements = vec![
            PoolRetirement {
                pool_id: make_pool_id(0x01),
                retirement_epoch: EpochNo(10),
            },
        ];

        let mut mark_snap = StakeSnapshot::new();
        mark_snap.pool_stakes.insert(make_pool_id(0x01), Coin(100));
        mark_snap.pool_stakes.insert(make_pool_id(0x02), Coin(200));
        mark_snap.delegations.insert(make_cred(0xaa), (make_pool_id(0x01), Coin(100)));
        mark_snap.delegations.insert(make_cred(0xbb), (make_pool_id(0x02), Coin(200)));

        let snapshots = SnapshotState {
            mark: MarkSnapshot(mark_snap.clone()),
            set: SetSnapshot(mark_snap.clone()),
            go: GoSnapshot(mark_snap),
        };

        let (new_params, new_snapshots, retired) =
            retire_pools(EpochNo(10), &params, &retirements, &snapshots);

        // Pool 0x01 should be retired
        assert_eq!(retired, vec![make_pool_id(0x01)]);
        assert!(!new_params.contains_key(&make_pool_id(0x01)));
        assert!(new_params.contains_key(&make_pool_id(0x02)));

        // Delegation to retired pool should be removed from all snapshots
        assert!(!new_snapshots.mark.0.delegations.contains_key(&make_cred(0xaa)));
        assert!(new_snapshots.mark.0.delegations.contains_key(&make_cred(0xbb)));
    }

    #[test]
    fn retire_pools_future_epoch_not_retired() {
        let mut params = BTreeMap::new();
        params.insert(make_pool_id(0x01), PoolParams {
            cost: Coin(100),
            margin: Rational::new(1, 10).unwrap(),
            pledge: Coin(500),
            reward_account: make_cred(0x01),
        });

        let retirements = vec![
            PoolRetirement {
                pool_id: make_pool_id(0x01),
                retirement_epoch: EpochNo(20), // Not yet
            },
        ];

        let snapshots = SnapshotState::new();

        let (new_params, _, retired) =
            retire_pools(EpochNo(10), &params, &retirements, &snapshots);

        assert!(retired.is_empty());
        assert!(new_params.contains_key(&make_pool_id(0x01)));
    }

    #[test]
    fn retire_pools_empty_retirements() {
        let params = BTreeMap::new();
        let retirements: Vec<PoolRetirement> = vec![];
        let snapshots = SnapshotState::new();

        let (new_params, _, retired) =
            retire_pools(EpochNo(5), &params, &retirements, &snapshots);

        assert!(retired.is_empty());
        assert!(new_params.is_empty());
    }

    #[test]
    fn retire_pools_deterministic() {
        let mut params = BTreeMap::new();
        params.insert(make_pool_id(0x01), PoolParams {
            cost: Coin(100),
            margin: Rational::new(1, 10).unwrap(),
            pledge: Coin(500),
            reward_account: make_cred(0x01),
        });

        let retirements = vec![
            PoolRetirement {
                pool_id: make_pool_id(0x01),
                retirement_epoch: EpochNo(5),
            },
        ];

        let snapshots = SnapshotState::new();

        let r1 = retire_pools(EpochNo(5), &params, &retirements, &snapshots);
        let r2 = retire_pools(EpochNo(5), &params, &retirements, &snapshots);
        assert_eq!(r1, r2);
    }

    // -----------------------------------------------------------------------
    // Epoch boundary orchestration tests
    // -----------------------------------------------------------------------

    #[test]
    fn apply_epoch_boundary_basic() {
        let epoch_state = EpochState {
            epoch: EpochNo(5),
            slot: ade_types::SlotNo(21600),
            snapshots: SnapshotState::new(),
            reserves: Coin(10_000_000_000),
            treasury: Coin(0),
        };

        let state = LedgerState {
            utxo_state: crate::utxo::UTxOState::new(),
            epoch_state,
            protocol_params: ProtocolParameters::default(),
            era: CardanoEra::Shelley,
        };

        let new_mark = StakeSnapshot::new();
        let pool_params = BTreeMap::new();
        let retirements = vec![];
        let pp_updates = BTreeMap::new();

        let result = apply_epoch_boundary(
            &state,
            EpochNo(6),
            new_mark,
            &pool_params,
            &retirements,
            &pp_updates,
        )
        .unwrap();

        assert_eq!(result.new_epoch, EpochNo(6));
        assert!(result.rewards.is_empty());
        assert!(result.retired_pools.is_empty());
    }

    #[test]
    fn sum_distributed_rewards_basic() {
        let mut rewards = BTreeMap::new();
        let mut member_rewards = BTreeMap::new();
        member_rewards.insert(make_cred(0x01), Coin(100));
        member_rewards.insert(make_cred(0x02), Coin(200));

        rewards.insert(make_pool_id(0x01), PoolRewards {
            operator_reward: Coin(50),
            member_rewards,
        });

        let total = sum_distributed_rewards(&rewards);
        assert_eq!(total, Coin(350));
    }
}
