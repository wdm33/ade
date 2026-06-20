// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! EPOCH-CONSENSUS-VIEW S3c (DC-EVIEW-05) — per-pool stake aggregation (the linchpin).
//!
//! Aggregate the advanced reduced-UTxO checkpoint (DC-EVIEW-04/04b) into the per-pool
//! active stake — the value the next-epoch leader schedule is computed over. This is
//! the cardano-ledger snapshot rule, Conway-specialized: for each REGISTERED +
//! DELEGATED stake credential, its active stake is `Σ(its base-address UTxO coin) +
//! its reward-account balance`, grouped by its delegated pool. (Pointer stake is
//! retired at Conway, so the reduced checkpoint holds only `Base(cred)` references —
//! the era gate is already applied; pointer/enterprise/Byron outputs are
//! `NonContributing` and never reach here.)
//!
//! The two inputs come from the single ledger authority's own projection:
//! - `cred_utxo_stake` — per-base-credential coin sums from the reduced checkpoint
//!   (`ReducedUtxoCheckpoint::sum_base_credential_stake`).
//! - `delegation` — the cred→pool map + reward balances, accumulated by the window's
//!   `advance_cert_state` (the ledger's own `process_block_certificates`).
//!
//! This is OBSERVE-ONLY: it computes the aggregate; the rewire of `apply_epoch_boundary`'s
//! `new_mark` stub to consume it, and feeding it to live leader election, are the
//! activation slice (DC-EVIEW-08) — NO live-path change here. Acceptance is the
//! DIFFERENTIAL ORACLE vs `cardano-cli query stake-snapshot` (stakeSet) at >=2 Conway
//! boundaries (a LIVE gate, declared — run at activation, NOT faked green here).

use std::collections::BTreeMap;

use ade_types::tx::{Coin, PoolId};

use crate::delegation::DelegationState;
use ade_types::shelley::cert::StakeCredential;

/// The per-pool active stake distribution + the total, for one epoch snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeByPool {
    pub pool_stakes: BTreeMap<PoolId, Coin>,
    pub total_active_stake: Coin,
}

/// Why aggregation failed (fail-closed; never a silently wrapped stake total).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateError {
    /// A pool stake or the total exceeded u64 (unreachable under the Cardano
    /// max-supply bound, but never silently wrapped).
    StakeOverflow,
}

/// Aggregate per-pool active stake from the per-credential UTxO sums + the delegation
/// state. Iterates the REGISTERED+DELEGATED credentials (the `delegations` map — you
/// cannot delegate an unregistered credential), summing each credential's UTxO coin +
/// reward balance into its delegated pool. A credential with UTxO but no delegation
/// contributes nothing (not iterated); a delegated credential with a reward balance but
/// no UTxO still contributes its reward (Conway). Pure, total, deterministic,
/// fail-closed on overflow.
pub fn aggregate_pool_stake(
    cred_utxo_stake: &BTreeMap<StakeCredential, Coin>,
    delegation: &DelegationState,
) -> Result<StakeByPool, AggregateError> {
    let mut pool_stakes: BTreeMap<PoolId, Coin> = BTreeMap::new();
    for (cred, pool) in &delegation.delegations {
        let utxo = cred_utxo_stake.get(cred).copied().unwrap_or(Coin(0));
        let reward = delegation.rewards.get(cred).copied().unwrap_or(Coin(0));
        let cred_total = utxo.checked_add(reward).ok_or(AggregateError::StakeOverflow)?;
        if cred_total.0 == 0 {
            continue; // a delegated credential with neither UTxO nor reward adds nothing
        }
        let entry = pool_stakes.entry(pool.clone()).or_insert(Coin(0));
        *entry = entry
            .checked_add(cred_total)
            .ok_or(AggregateError::StakeOverflow)?;
    }
    let mut total = Coin(0);
    for v in pool_stakes.values() {
        total = total.checked_add(*v).ok_or(AggregateError::StakeOverflow)?;
    }
    Ok(StakeByPool {
        pool_stakes,
        total_active_stake: total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ade_types::Hash28;

    fn key_cred(fill: u8) -> StakeCredential {
        StakeCredential::KeyHash(Hash28([fill; 28]))
    }
    fn pool(fill: u8) -> PoolId {
        PoolId(Hash28([fill; 28]))
    }

    fn deleg(
        delegations: &[(StakeCredential, PoolId)],
        rewards: &[(StakeCredential, Coin)],
    ) -> DelegationState {
        let mut d = DelegationState::default();
        for (c, p) in delegations {
            d.delegations.insert(c.clone(), p.clone());
        }
        for (c, r) in rewards {
            d.rewards.insert(c.clone(), *r);
        }
        d
    }

    // The core: UTxO coin + reward, per registered+delegated credential, grouped by pool.
    #[test]
    fn sums_utxo_plus_reward_per_delegated_pool() {
        let (ca, cb, cc) = (key_cred(0xaa), key_cred(0xbb), key_cred(0xcc));
        let cred_utxo: BTreeMap<_, _> =
            [(ca.clone(), Coin(100)), (cb.clone(), Coin(50)), (cc.clone(), Coin(999))]
                .into_iter()
                .collect();
        // ca,cb -> pool1; cc is NOT delegated (its 999 must not count).
        let d = deleg(
            &[(ca.clone(), pool(1)), (cb.clone(), pool(1))],
            &[(ca.clone(), Coin(10))], // ca has a reward balance
        );
        let agg = aggregate_pool_stake(&cred_utxo, &d).unwrap();
        // pool1 = (ca 100 + reward 10) + (cb 50) = 160; cc excluded.
        assert_eq!(agg.pool_stakes.get(&pool(1)), Some(&Coin(160)));
        assert_eq!(agg.pool_stakes.get(&pool(2)), None);
        assert_eq!(agg.total_active_stake, Coin(160));
    }

    // A delegated credential with a reward balance but NO UTxO still contributes (Conway).
    #[test]
    fn reward_without_utxo_contributes() {
        let c = key_cred(0x11);
        let cred_utxo: BTreeMap<StakeCredential, Coin> = BTreeMap::new();
        let d = deleg(&[(c.clone(), pool(7))], &[(c.clone(), Coin(42))]);
        let agg = aggregate_pool_stake(&cred_utxo, &d).unwrap();
        assert_eq!(agg.pool_stakes.get(&pool(7)), Some(&Coin(42)));
    }

    // A credential with UTxO but NOT delegated contributes nothing.
    #[test]
    fn undelegated_credential_contributes_nothing() {
        let c = key_cred(0x22);
        let cred_utxo: BTreeMap<_, _> = [(c.clone(), Coin(1000))].into_iter().collect();
        let d = DelegationState::default(); // no delegations
        let agg = aggregate_pool_stake(&cred_utxo, &d).unwrap();
        assert!(agg.pool_stakes.is_empty());
        assert_eq!(agg.total_active_stake, Coin(0));
    }

    // A delegated credential with neither UTxO nor reward adds no pool entry.
    #[test]
    fn delegated_but_zero_stake_adds_no_pool_entry() {
        let c = key_cred(0x33);
        let cred_utxo: BTreeMap<StakeCredential, Coin> = BTreeMap::new();
        let d = deleg(&[(c.clone(), pool(9))], &[]);
        let agg = aggregate_pool_stake(&cred_utxo, &d).unwrap();
        assert!(agg.pool_stakes.is_empty(), "a zero-stake delegated cred adds no pool");
    }

    // Multiple pools aggregate independently; the total is the sum.
    #[test]
    fn multiple_pools_aggregate_independently() {
        let (ca, cb) = (key_cred(0x01), key_cred(0x02));
        let cred_utxo: BTreeMap<_, _> =
            [(ca.clone(), Coin(300)), (cb.clone(), Coin(700))].into_iter().collect();
        let d = deleg(&[(ca.clone(), pool(1)), (cb.clone(), pool(2))], &[]);
        let agg = aggregate_pool_stake(&cred_utxo, &d).unwrap();
        assert_eq!(agg.pool_stakes.get(&pool(1)), Some(&Coin(300)));
        assert_eq!(agg.pool_stakes.get(&pool(2)), Some(&Coin(700)));
        assert_eq!(agg.total_active_stake, Coin(1000));
    }

    // Overflow is fail-closed, never a wrapped total.
    #[test]
    fn overflow_is_fail_closed() {
        let (ca, cb) = (key_cred(0x01), key_cred(0x02));
        let cred_utxo: BTreeMap<_, _> =
            [(ca.clone(), Coin(u64::MAX)), (cb.clone(), Coin(1))].into_iter().collect();
        let d = deleg(&[(ca.clone(), pool(1)), (cb.clone(), pool(1))], &[]);
        assert_eq!(
            aggregate_pool_stake(&cred_utxo, &d),
            Err(AggregateError::StakeOverflow)
        );
    }

    // Determinism.
    #[test]
    fn aggregation_is_deterministic() {
        let c = key_cred(0x55);
        let cred_utxo: BTreeMap<_, _> = [(c.clone(), Coin(5))].into_iter().collect();
        let d = deleg(&[(c.clone(), pool(3))], &[]);
        assert_eq!(
            aggregate_pool_stake(&cred_utxo, &d),
            aggregate_pool_stake(&cred_utxo, &d)
        );
    }
}
