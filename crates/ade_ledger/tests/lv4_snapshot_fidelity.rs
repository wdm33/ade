//! SLICE LV-4 (DC-EPOCH-43, DC-EPOCH-44) — a stake snapshot keeps what cardano's keeps: the whole
//! credential (key/script tag included) and each pool's reward parameters frozen when it was taken.
//! The reward calculation reads pool parameters from the go snapshot only, and credits a reward to
//! exactly the credential it was computed for.
//!
//! The reference (cardano-node 11.0.1, preprod, `esSnapshots` of the captured ledger state): credentials
//! in `activeStake` are keyed `keyHash-…` / `scriptHash-…`, and `stakePoolsSnapShot` carries per pool
//! `accountId` (tagged), `cost`, `margin`, `pledge` and `selfDelegatedOwners`, frozen at SNAP.
//!
//! The real cases this closes (`docs/clusters/PREPROD-LIVE-2-FORGE-READINESS/SLICE-LV-4-snapshot-fidelity.md`):
//! a script credential's reward credited to a key credential with the same bytes (11,802 lovelace at
//! 304→305), and a retired pool's last blocks left unpaid (42,082,928,795 lovelace at 305→306).

use ade_ledger::delegation::PoolParams;
use ade_ledger::epoch::{GoSnapshot, StakeSnapshot};
use ade_ledger::epoch_accumulator::{snapshot_pool_params, BoundaryBaseStake};
use ade_ledger::governance::GovernanceTerminal;
use ade_ledger::rational::Rational;
use ade_ledger::rules::{apply_epoch_boundary_with_registrations, EpochBoundaryAccounting};
use ade_ledger::state::LedgerState;
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, PoolId};
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};
use std::collections::BTreeMap;

const POOL_P: u8 = 0x11;
const POOL_Q: u8 = 0x22;
const OPERATOR: u8 = 0xC0;
const OWNER: u8 = 0xA0;
const MEMBER: u8 = 0xB0;

const ADA: u64 = 1_000_000;
const MEMBER_STAKE: u64 = 600_000_000 * ADA;
const OWNER_STAKE: u64 = 5_275_349 * ADA;
const PLEDGE: u64 = 4_500_000 * ADA;

fn h(b: u8) -> Hash28 {
    Hash28([b; 28])
}
fn key(b: u8) -> StakeCredential {
    StakeCredential::KeyHash(h(b))
}
fn script(b: u8) -> StakeCredential {
    StakeCredential::ScriptHash(h(b))
}
fn pool(b: u8) -> PoolId {
    PoolId(h(b))
}

/// A registration whose reward account is `account` (header 0xE0 key / 0xF0 script, testnet).
fn registration(id: u8, account: &StakeCredential, pledge: u64, cost: u64, margin: (u64, u64), owners: &[u8]) -> PoolParams {
    let (header, bytes) = match account {
        StakeCredential::KeyHash(h) => (0xE0u8, h),
        StakeCredential::ScriptHash(h) => (0xF0u8, h),
    };
    let mut reward_account = vec![header];
    reward_account.extend_from_slice(&bytes.0);
    PoolParams {
        pool_id: pool(id),
        vrf_hash: Hash32([id; 32]),
        pledge: Coin(pledge),
        cost: Coin(cost),
        margin,
        reward_account,
        owners: owners.iter().map(|o| h(*o)).collect(),
    }
}

fn standard(id: u8) -> PoolParams {
    registration(id, &key(OPERATOR), 0, 340 * ADA, (1, 10), &[])
}

/// A Conway boundary where only pool P produced blocks. `go` is the go snapshot; `frozen` are the pool
/// registrations as they stood when it was taken; `current` the registrations at this boundary.
struct World {
    state: LedgerState,
}

impl World {
    fn new(go: &[(StakeCredential, u8, u64)], frozen: &[PoolParams], current: &[PoolParams]) -> Self {
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

        let mut snap = StakeSnapshot::new();
        for (cred, p, stake) in go {
            snap.delegations.insert(cred.clone(), (pool(*p), Coin(*stake)));
            snap.pool_stakes.entry(pool(*p)).or_insert(Coin(0)).0 += *stake;
        }
        let frozen_regs: BTreeMap<PoolId, PoolParams> =
            frozen.iter().map(|p| (p.pool_id.clone(), p.clone())).collect();
        let go_delegations: BTreeMap<StakeCredential, PoolId> =
            go.iter().map(|(c, p, _)| (c.clone(), pool(*p))).collect();
        snap.pool_params = snapshot_pool_params(&frozen_regs, &go_delegations);
        state.epoch_state.snapshots.as_authoritative_mut().unwrap().go = GoSnapshot(snap);

        let cert = state.cert_state.as_authoritative_mut().unwrap();
        for p in current {
            cert.pool.pools.insert(p.pool_id.clone(), p.clone());
        }
        World { state }
    }

    fn register(mut self, cred: StakeCredential) -> Self {
        self.state
            .cert_state
            .as_authoritative_mut()
            .unwrap()
            .delegation
            .registrations
            .insert(cred, Coin(2 * ADA));
        self
    }

    fn run(&self) -> Result<(LedgerState, EpochBoundaryAccounting), GovernanceTerminal> {
        self.run_with_base(BTreeMap::new())
    }

    fn run_with_base(
        &self,
        base: BTreeMap<StakeCredential, Coin>,
    ) -> Result<(LedgerState, EpochBoundaryAccounting), GovernanceTerminal> {
        let base = BoundaryBaseStake { boundary_point: SlotNo(0), canonical_credential_stake: base };
        apply_epoch_boundary_with_registrations(&self.state, EpochNo(501), None, Some(&base), 21_600)
    }
}

fn reward(state: &LedgerState, cred: &StakeCredential) -> u64 {
    state
        .cert_state
        .as_authoritative()
        .unwrap()
        .delegation
        .rewards
        .get(cred)
        .map(|c| c.0)
        .unwrap_or(0)
}

/// CE-LV4-1 — a key and a script credential with the same 28 bytes are two stakers in the mark the
/// boundary builds: two entries, both stakes in the pool total, and the credential-side total
/// (DC-EPOCH-40) agrees with the per-pool sum.
#[test]
fn a_key_and_a_script_credential_with_the_same_bytes_are_two_stakers() {
    let mut world = World::new(&[], &[], &[standard(POOL_P)]);
    {
        let deleg = &mut world.state.cert_state.as_authoritative_mut().unwrap().delegation;
        for cred in [key(MEMBER), script(MEMBER)] {
            deleg.registrations.insert(cred.clone(), Coin(2 * ADA));
            deleg.delegations.insert(cred, pool(POOL_P));
        }
    }
    let base: BTreeMap<_, _> = [(key(MEMBER), Coin(300)), (script(MEMBER), Coin(500))].into_iter().collect();
    let (after, _) = world.run_with_base(base).expect("boundary");
    let mark = &after.epoch_state.snapshots.as_authoritative().unwrap().mark.0;
    assert_eq!(mark.delegations.get(&key(MEMBER)), Some(&(pool(POOL_P), Coin(300))));
    assert_eq!(mark.delegations.get(&script(MEMBER)), Some(&(pool(POOL_P), Coin(500))));
    assert_eq!(mark.pool_stakes.get(&pool(POOL_P)), Some(&Coin(800)));
    assert_eq!(mark.total_active_stake(), 800, "the credential side counts both stakers");
}

/// CE-LV4-2 — THE 304→305 CASE. A script credential delegated to the pool earns a member reward. With only
/// the KEY credential of the same bytes registered, the reward goes to the treasury, never to that key
/// account. With the script credential registered, it is credited there.
#[test]
fn a_script_credentials_reward_is_never_credited_to_the_key_credential_with_the_same_bytes() {
    let go = [(script(MEMBER), POOL_P, MEMBER_STAKE)];
    let regs = [standard(POOL_P)];

    let (after, acct) = World::new(&go, &regs, &regs)
        .register(key(OPERATOR))
        .register(key(MEMBER))
        .run()
        .expect("boundary");
    assert_eq!(reward(&after, &key(MEMBER)), 0, "the key account with the same bytes gets nothing");
    assert!(acct.delta_t2 > 0, "the unregistered script staker's reward goes to the treasury");
    let to_treasury = acct.delta_t2;

    let (after, acct) = World::new(&go, &regs, &regs)
        .register(key(OPERATOR))
        .register(key(MEMBER))
        .register(script(MEMBER))
        .run()
        .expect("boundary");
    assert_eq!(reward(&after, &script(MEMBER)), to_treasury, "credited to the script credential instead");
    assert_eq!(reward(&after, &key(MEMBER)), 0);
    assert_eq!(acct.delta_t2, 0);
}

/// CE-LV4-3 — THE 305→306 CASE. A pool that retired after the go snapshot was taken is absent from the
/// current registrations but present, with blocks, in the go snapshot. It is paid from its frozen
/// parameters — operator and members — exactly as if it were still registered with the same values.
#[test]
fn a_retired_pool_is_paid_for_its_last_blocks_from_its_frozen_parameters() {
    let go = [(key(MEMBER), POOL_P, MEMBER_STAKE)];
    let regs = [standard(POOL_P)];

    let (retired, acct) =
        World::new(&go, &regs, &[]).register(key(MEMBER)).register(key(OPERATOR)).run().expect("boundary");
    assert!(reward(&retired, &key(MEMBER)) > 0, "the member is paid");
    assert!(reward(&retired, &key(OPERATOR)) > 0, "the operator is paid");
    assert!(acct.sum_rewards > 0);

    let (registered, _) =
        World::new(&go, &regs, &regs).register(key(MEMBER)).register(key(OPERATOR)).run().expect("boundary");
    assert_eq!(reward(&retired, &key(MEMBER)), reward(&registered, &key(MEMBER)));
    assert_eq!(reward(&retired, &key(OPERATOR)), reward(&registered, &key(OPERATOR)));
}

/// CE-LV4-4 — a pool that re-registered with a different pledge, cost and margin after the snapshot is
/// paid on the FROZEN values: identical to the run where the registrations never changed, and different
/// from the run where the snapshot itself carries the new values.
#[test]
fn a_re_registered_pool_is_paid_on_the_values_frozen_in_the_go_snapshot() {
    let go = [(key(MEMBER), POOL_P, MEMBER_STAKE)];
    let old = standard(POOL_P);
    let new = registration(POOL_P, &key(OPERATOR), 0, 1_000 * ADA, (1, 2), &[]);
    let pay = |frozen: &PoolParams, current: &PoolParams| {
        let (after, _) = World::new(&go, &[frozen.clone()], &[current.clone()])
            .register(key(MEMBER))
            .register(key(OPERATOR))
            .run()
            .expect("boundary");
        (reward(&after, &key(MEMBER)), reward(&after, &key(OPERATOR)))
    };
    let frozen_old_now_new = pay(&old, &new);
    assert_eq!(frozen_old_now_new, pay(&old, &old), "current registrations do not matter");
    assert_ne!(frozen_old_now_new, pay(&new, &new), "the frozen values do");
}

/// CE-LV4-5 (operator) — an operator with a SCRIPT reward account is paid the leader reward on the script
/// credential, even when a key credential with the same bytes is registered.
#[test]
fn an_operator_with_a_script_reward_account_is_paid_on_the_script_credential() {
    let go = [(key(MEMBER), POOL_P, MEMBER_STAKE)];
    let regs = [registration(POOL_P, &script(OPERATOR), 0, 340 * ADA, (1, 10), &[])];
    let (after, _) = World::new(&go, &regs, &regs)
        .register(key(MEMBER))
        .register(key(OPERATOR))
        .register(script(OPERATOR))
        .run()
        .expect("boundary");
    assert!(reward(&after, &script(OPERATOR)) > 0, "leader reward on the script credential");
    assert_eq!(reward(&after, &key(OPERATOR)), 0);
}

/// CE-LV4-5 (owner) — owners are key hashes. A SCRIPT credential with the owner's bytes delegated to the
/// pool does not satisfy the pledge; the owner's KEY credential does.
#[test]
fn an_owner_is_matched_as_a_key_credential_only() {
    let regs = [registration(POOL_P, &key(OPERATOR), PLEDGE, 340 * ADA, (1, 10), &[OWNER])];
    let pay = |owner_cred: StakeCredential| {
        let go = [(owner_cred, POOL_P, OWNER_STAKE), (key(MEMBER), POOL_P, MEMBER_STAKE)];
        let (after, acct) = World::new(&go, &regs, &regs)
            .register(key(MEMBER))
            .register(key(OPERATOR))
            .run()
            .expect("boundary");
        (reward(&after, &key(MEMBER)), acct.sum_rewards)
    };
    assert_eq!(pay(script(OWNER)), (0, 0), "a script credential is not the owner: pledge not met");
    let (member, total) = pay(key(OWNER));
    assert!(member > 0 && total > 0, "the owner's key credential meets the pledge");
}

/// CE-LV4-6 — a pool with blocks and go-snapshot stake but no frozen parameters is a typed terminal, never
/// a silent skip and never a fallback to the current registrations.
#[test]
fn a_pool_that_earned_with_no_frozen_parameters_is_a_terminal() {
    let go = [(key(MEMBER), POOL_P, MEMBER_STAKE)];
    let world = World::new(&go, &[], &[standard(POOL_P)]).register(key(MEMBER));
    match world.run() {
        Err(GovernanceTerminal::RewardPoolParamsMissing { pool: p, .. }) => assert_eq!(p, pool(POOL_P)),
        other => panic!("expected RewardPoolParamsMissing, got {:?}", other.map(|(_, a)| a.sum_rewards)),
    }
}

/// DC-EPOCH-44 (mark side) — the mark built at a boundary freezes the registrations as they stand at SNAP:
/// every registered pool (one with no delegators included), the values BEFORE a staged re-registration is
/// adopted, and a pool retiring at this boundary still present.
#[test]
fn the_mark_freezes_parameters_before_adoption_and_before_poolreap() {
    let old = standard(POOL_P);
    let staged = registration(POOL_P, &key(OPERATOR), 77 * ADA, 999 * ADA, (1, 3), &[]);
    let mut world = World::new(&[], &[], &[old.clone(), standard(POOL_Q)]);
    {
        let cert = world.state.cert_state.as_authoritative_mut().unwrap();
        cert.pool.future_pools.insert(pool(POOL_P), staged);
        cert.pool.retiring.insert(pool(POOL_Q), EpochNo(501));
    }
    let (after, _) = world.run().expect("boundary");
    let mark = &after.epoch_state.snapshots.as_authoritative().unwrap().mark.0;
    let p = &mark.pool_params[&pool(POOL_P)];
    assert_eq!((p.pledge, p.cost, p.margin), (old.pledge, old.cost, old.margin), "pre-adoption values");
    assert!(mark.pool_params.contains_key(&pool(POOL_Q)), "a pool reaped at this boundary is in the mark");
    let cert = after.cert_state.as_authoritative().unwrap();
    assert!(!cert.pool.pools.contains_key(&pool(POOL_Q)), "and it is reaped from the registrations");
    assert_eq!(cert.pool.pools[&pool(POOL_P)].cost, Coin(999 * ADA), "the staged values are adopted after SNAP");
}
