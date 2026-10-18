//! SLICE LV-3 (DC-EPOCH-42) — in the reward calculation a pool's owner stake is the stake of the
//! owners DELEGATED TO THAT POOL, and that one value feeds both the pledge gate and the leader term.
//!
//! The oracle (cardano-ledger `master`, `Cardano.Ledger.State.SnapShots` / `Shelley.Rewards`):
//!
//!   selfDelegatedOwners =
//!     Set.filter (\o -> KeyHashObj o `Set.member` spsDelegators) spsOwners
//!   maxP = if pledge <= selfDelegatedOwnersStake then maxPool' … else mempty
//!   poolOwnerRelativeStake = selfDelegatedOwnersStake %? totalStake
//!
//! The real case this closes: preprod pool `13e8600b…2418a` pledges 4.5M ADA while its only owner
//! delegates 5.275M ADA to a different pool. cardano pays the pool nothing; Ade paid it
//! 6,218,191,443 lovelace per producing epoch
//! (`docs/evidence/run-stores/preprod-live2c/lv2-e1-native-seed-reward-vs-cardano.md` §4).

use ade_ledger::delegation::PoolParams;
use ade_ledger::epoch::{GoSnapshot, StakeSnapshot};
use ade_ledger::epoch_accumulator::{snapshot_pool_params, BoundaryBaseStake};
use ade_ledger::rational::Rational;
use ade_ledger::rules::{apply_epoch_boundary_with_registrations, EpochBoundaryAccounting};
use ade_ledger::state::LedgerState;
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, PoolId};
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};
use std::collections::BTreeMap;

const POOL_P: u8 = 0x11;
const POOL_Q: u8 = 0x22;
/// Pool P's reward account. Never a delegator, so everything it receives is the leader reward.
const REWARD_ACCT_P: u8 = 0xC0;
const OWNER: u8 = 0xA0;
const OWNER_2: u8 = 0xA1;
const MEMBER: u8 = 0xB0;
const BYSTANDER: u8 = 0xB1;

const ADA: u64 = 1_000_000;

fn h(b: u8) -> Hash28 {
    Hash28([b; 28])
}
fn pool(b: u8) -> PoolId {
    PoolId(h(b))
}

fn pool_params(id: u8, reward_acct: u8, pledge: u64, owners: &[u8]) -> PoolParams {
    let mut reward_account = vec![0xE0];
    reward_account.extend_from_slice(&[reward_acct; 28]);
    PoolParams {
        pool_id: pool(id),
        vrf_hash: Hash32([id; 32]),
        pledge: Coin(pledge),
        cost: Coin(340 * ADA),
        margin: (1, 10),
        reward_account,
        owners: owners.iter().map(|o| h(*o)).collect(),
    }
}

fn go_snapshot(delegations: &[(u8, u8, u64)]) -> StakeSnapshot {
    let mut snap = StakeSnapshot::new();
    for (cred, pool_byte, stake) in delegations {
        snap.delegations.insert(StakeCredential::KeyHash(h(*cred)), (pool(*pool_byte), Coin(*stake)));
        let entry = snap.pool_stakes.entry(pool(*pool_byte)).or_insert(Coin(0));
        entry.0 += *stake;
    }
    snap
}

/// A Conway boundary where only pool P produced blocks. Every credential named in `delegations`
/// and P's reward account are registered, so a computed reward lands in an account rather than the
/// treasury.
fn boundary(
    p_pledge: u64,
    p_owners: &[u8],
    delegations: &[(u8, u8, u64)],
) -> (LedgerState, EpochBoundaryAccounting) {
    let mut state = LedgerState::new(CardanoEra::Conway);
    state.track_utxo = true;
    state.epoch_state.epoch = EpochNo(500);
    state.epoch_state.reserves = Coin(1_000_000_000_000_000);
    state.epoch_state.treasury = Coin(0);
    state.protocol_params.protocol_major = 10;
    state.protocol_params.n_opt = 10;
    state.protocol_params.pool_influence = Rational::new(3, 10).unwrap();
    state.protocol_params.decentralization = Rational::zero();
    state.protocol_params.monetary_expansion = Rational::new(3, 1000).unwrap();
    state.protocol_params.treasury_growth = Rational::new(1, 5).unwrap();
    state.epoch_state.block_production.insert(pool(POOL_P), 21_600);

    state.epoch_state.snapshots.as_authoritative_mut().unwrap().go =
        GoSnapshot(go_snapshot(delegations));

    let cert = state.cert_state.as_authoritative_mut().unwrap();
    cert.pool
        .pools
        .insert(pool(POOL_P), pool_params(POOL_P, REWARD_ACCT_P, p_pledge, p_owners));
    cert.pool
        .pools
        .insert(pool(POOL_Q), pool_params(POOL_Q, 0xC1, 0, &[]));
    for (cred, _, _) in delegations {
        cert.delegation
            .registrations
            .insert(StakeCredential::KeyHash(h(*cred)), Coin(2 * ADA));
    }
    cert.delegation
        .registrations
        .insert(StakeCredential::KeyHash(h(REWARD_ACCT_P)), Coin(2 * ADA));

    // LV-4: both pools were registered when the go snapshot was taken; their parameters, and the owners
    // delegated to them in that snapshot, are frozen in it.
    let go_delegations: BTreeMap<StakeCredential, PoolId> = delegations
        .iter()
        .map(|(cred, pool_byte, _)| (StakeCredential::KeyHash(h(*cred)), pool(*pool_byte)))
        .collect();
    let frozen = snapshot_pool_params(&cert.pool.pools, &go_delegations);
    state.epoch_state.snapshots.as_authoritative_mut().unwrap().go.0.pool_params = frozen;

    let base = BoundaryBaseStake {
        boundary_point: SlotNo(0),
        canonical_credential_stake: BTreeMap::new(),
    };
    apply_epoch_boundary_with_registrations(&state, EpochNo(501), None, Some(&base), 21_600)
        .expect("boundary")
}

fn reward(state: &LedgerState, cred: u8) -> u64 {
    state
        .cert_state
        .as_authoritative()
        .unwrap()
        .delegation
        .rewards
        .get(&StakeCredential::KeyHash(h(cred)))
        .map(|c| c.0)
        .unwrap_or(0)
}

const PLEDGE: u64 = 4_500_000 * ADA;
const OWNER_STAKE: u64 = 5_275_349 * ADA;
const MEMBER_STAKE: u64 = 600_000_000 * ADA;

/// CE-LV3-1 — THE INVARIANT. The preprod shape: pledge > 0, the only owner holds more than the
/// pledge but delegates it to a different pool. The pool earns nothing: no leader reward, no member
/// reward, and the whole pool pot returns to reserves.
#[test]
fn a_pool_whose_owner_delegates_elsewhere_earns_nothing() {
    let (after, acct) = boundary(
        PLEDGE,
        &[OWNER],
        &[(OWNER, POOL_Q, OWNER_STAKE), (MEMBER, POOL_P, MEMBER_STAKE)],
    );

    assert!(acct.pool_reward_pot > 0, "there is a pot to distribute");
    assert_eq!(reward(&after, REWARD_ACCT_P), 0, "no leader reward");
    assert_eq!(reward(&after, MEMBER), 0, "no member reward");
    assert_eq!(reward(&after, OWNER), 0, "the owner earns nothing from either pool");
    assert_eq!(acct.sum_rewards, 0, "nothing is distributed on the pool's behalf");
    assert_eq!(
        acct.delta_r2, acct.pool_reward_pot,
        "the undistributed pot returns to reserves in full"
    );
}

/// CE-LV3-2 (gate half) — the control for CE-LV3-1, so it cannot pass vacuously: the SAME owner with
/// the SAME stake, pointed at the pool itself, satisfies the pledge and the pool is paid.
#[test]
fn the_same_owner_delegated_to_the_pool_satisfies_the_pledge() {
    let (after, acct) = boundary(
        PLEDGE,
        &[OWNER],
        &[(OWNER, POOL_P, OWNER_STAKE), (MEMBER, POOL_P, MEMBER_STAKE)],
    );

    assert!(reward(&after, REWARD_ACCT_P) > 0, "the leader is paid");
    assert!(reward(&after, MEMBER) > 0, "the member is paid");
    assert_eq!(reward(&after, OWNER), 0, "an owner's share rides in the leader term");
    assert!(acct.sum_rewards > 0);
}

/// CE-LV3-2 (leader-term half) — with the pledge gate out of the way (pledge 0), the leader reward
/// reads the same self-delegated value. Stake delegated to pool Q does not change P's leader reward
/// whether or not the credential holding it is one of P's owners; stake delegated to P does.
#[test]
fn the_leader_term_counts_only_owners_delegated_to_the_pool() {
    let q_stake = 300_000_000 * ADA;
    let own_stake = 200_000_000 * ADA;

    // Pool P and pool Q hold identical stake in all three; only WHO holds it differs.
    let owner_2_elsewhere = boundary(
        0,
        &[OWNER, OWNER_2],
        &[
            (OWNER, POOL_P, own_stake),
            (MEMBER, POOL_P, MEMBER_STAKE),
            (OWNER_2, POOL_Q, q_stake),
        ],
    );
    let bystander_elsewhere = boundary(
        0,
        &[OWNER, OWNER_2],
        &[
            (OWNER, POOL_P, own_stake),
            (MEMBER, POOL_P, MEMBER_STAKE),
            (BYSTANDER, POOL_Q, q_stake),
        ],
    );
    let no_owner_in_pool = boundary(
        0,
        &[OWNER_2],
        &[
            (OWNER, POOL_P, own_stake),
            (MEMBER, POOL_P, MEMBER_STAKE),
            (OWNER_2, POOL_Q, q_stake),
        ],
    );

    let leader = reward(&owner_2_elsewhere.0, REWARD_ACCT_P);
    assert!(leader > 0);
    assert_eq!(
        leader,
        reward(&bystander_elsewhere.0, REWARD_ACCT_P),
        "an owner's stake in ANOTHER pool adds nothing to this pool's leader reward"
    );
    assert_eq!(
        reward(&owner_2_elsewhere.0, MEMBER),
        reward(&bystander_elsewhere.0, MEMBER),
        "and leaves the members' rewards alone"
    );
    assert!(
        leader > reward(&no_owner_in_pool.0, REWARD_ACCT_P),
        "an owner's stake in THIS pool does raise the leader reward"
    );
}

/// CE-LV3-3 — the single definition, tested directly: of several owners only those delegated to
/// this pool are summed. An owner delegated elsewhere, an undelegated owner and a repeated owner
/// contribute nothing extra.
#[test]
fn only_the_self_delegated_subset_of_owners_is_summed() {
    let snap = go_snapshot(&[
        (OWNER, POOL_P, 700),
        (OWNER_2, POOL_Q, 50_000),
        (MEMBER, POOL_P, 9_000),
    ]);
    let undelegated = 0xA2;
    let owners = [h(OWNER), h(OWNER_2), h(undelegated), h(OWNER)];

    assert_eq!(snap.self_delegated_owner_stake(&pool(POOL_P), &owners), 700);
    assert_eq!(snap.self_delegated_owner_stake(&pool(POOL_Q), &owners), 50_000);
    assert_eq!(snap.self_delegated_owner_stake(&pool(0x33), &owners), 0);
    assert_eq!(snap.self_delegated_owner_stake(&pool(POOL_P), &[]), 0);
}

/// CE-LV3-3 at the boundary — mixed owners against a pledge that only the FULL owner stake would
/// meet. The self-delegated part falls short, so the pool earns nothing.
#[test]
fn a_pledge_met_only_by_counting_owners_elsewhere_is_not_met() {
    let (after, acct) = boundary(
        PLEDGE,
        &[OWNER, OWNER_2],
        &[
            (OWNER, POOL_P, PLEDGE - 1),
            (OWNER_2, POOL_Q, OWNER_STAKE),
            (MEMBER, POOL_P, MEMBER_STAKE),
        ],
    );
    assert_eq!(acct.sum_rewards, 0);
    assert_eq!(reward(&after, MEMBER), 0);

    // One lovelace more in the pool and the same pledge is met.
    let (after, _) = boundary(
        PLEDGE,
        &[OWNER, OWNER_2],
        &[
            (OWNER, POOL_P, PLEDGE),
            (OWNER_2, POOL_Q, OWNER_STAKE),
            (MEMBER, POOL_P, MEMBER_STAKE),
        ],
    );
    assert!(reward(&after, MEMBER) > 0, "pledge <= self-delegated owner stake is satisfied");
}
