//! SLICE LV-1 (DC-EPOCH-40) — the leader-check σ denominator is a snapshot fact, and the leadership
//! pool-set membership filter cannot move it.
//!
//! The oracle, quoted at two points in cardano-ledger's history
//! (`leadervalue-oracle-extraction-sigma-denominator.md`):
//!
//!   Conway  `let total = sumAllStakeCompact stake`   (`VMap.foldl (<>) mempty . unStake`)
//!   master  `ssTotalActiveStake = sumAllActiveStake ssActiveStake`  in `mkSnapShot`,
//!           then `calculatePoolDistr' … = PoolDistr { …, pdTotalActiveStake = activeStake }`
//!
//! In both, the denominator is folded over the STAKE (credential) map, and the membership guards
//! (`includeHash`, `spssNumDelegators > 0`) filter `unPoolDistr` ONLY — they run after the total is
//! already fixed.
//!
//! All ratio comparisons here are integer cross-multiplications in `u128`. This is a BLUE crate: no
//! floating point, which is also how the production threshold arithmetic works (Q.123 fixed point).
//!
//! The operands are REAL: the sealed preprod epoch-306 leadership distribution, the retired pool that
//! inflated its denominator, and the header that was deterministically rejected because of it.

use ade_core::consensus::{ActiveSlotsCoeff, LedgerView};
use ade_ledger::frozen_leadership::{FrozenLeadershipPoolDistr, LeadershipPoolEntry};
use ade_types::{EpochNo, Hash28, Hash32, SlotNo};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// The real preprod epoch-306 operands (docs/evidence/run-stores/preprod-live2c/).
// ---------------------------------------------------------------------------

/// Ade's SUMMED epoch-306 denominator — the wrong quantity, kept as the control.
const SUMMED_TOTAL_306: u64 = 1_626_066_239_242_875;
/// The retired pool present in 306's set and absent from 307's.
const RETIRED_POOL_STAKE: u64 = 63_075_223_742_053;
/// The issuer of the rejected header.
const ISSUER_STAKE: u64 = 1_730_595_594_678;
/// Epoch 307's denominator, which matches cardano's `total.stakeGo` to five significant figures —
/// what a membership-invariant denominator looks like on this chain.
const TOTAL_307: u64 = 1_563_586_879_499_918;

fn pid(b: u8) -> Hash28 {
    Hash28([b; 28])
}
fn h32(b: u8) -> Hash32 {
    Hash32([b; 32])
}

fn distr(total: u64, pools: &[(u8, u64)]) -> FrozenLeadershipPoolDistr {
    FrozenLeadershipPoolDistr {
        target_leadership_epoch: EpochNo(306),
        source_slot: SlotNo(130_118_358),
        source_hash: h32(0xB9),
        source_checkpoint_commitment: h32(0xF3),
        total_active_stake: total,
        pools: pools
            .iter()
            .map(|(b, stake)| {
                (
                    pid(*b),
                    LeadershipPoolEntry {
                        active_stake: *stake,
                        vrf_keyhash: h32(*b),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
    }
}

const ASC: ActiveSlotsCoeff = ActiveSlotsCoeff {
    numer: 1,
    denom: 20,
};

/// `a/b > c/d` for positive integers, without division.
fn ratio_gt(a: u64, b: u64, c: u64, d: u64) -> bool {
    (a as u128) * (d as u128) > (c as u128) * (b as u128)
}

// ===========================================================================
// CE-LV1-1 — THE INVARIANT. Membership cannot move the denominator.
// ===========================================================================

#[test]
fn adding_or_removing_a_pool_does_not_move_the_denominator() {
    let with = distr(
        TOTAL_307,
        &[(0x01, ISSUER_STAKE), (0x02, RETIRED_POOL_STAKE)],
    );
    let without = distr(TOTAL_307, &[(0x01, ISSUER_STAKE)]);

    let a = with.to_pool_distr_view(ASC);
    let b = without.to_pool_distr_view(ASC);

    assert_eq!(
        a.total_active_stake(EpochNo(306)),
        b.total_active_stake(EpochNo(306)),
        "the denominator is a SNAPSHOT fact — cardano fixes pdTotalActiveStake before its membership \
         guards run, so adding or removing a pool must not move it"
    );
    assert_eq!(
        a.total_active_stake(EpochNo(306)),
        Some(TOTAL_307),
        "and it is the CARRIED total, not a sum of whatever survived the filter"
    );
    assert_eq!(
        a.pool_active_stake(EpochNo(306), &pid(0x01)),
        b.pool_active_stake(EpochNo(306), &pid(0x01)),
        "so the surviving pool's sigma is identical either way — the whole slice in one assertion"
    );
}

// ===========================================================================
// CE-LV1-2 — THE REAL CASE. This is the bar; the live run is only a regression check.
// ===========================================================================

#[test]
fn the_retired_pool_no_longer_deflates_every_other_pools_sigma() {
    // Control: the summed denominator reproduces the sigma that rejected the header. Expressed as an
    // integer bound in parts per 10^13 rather than a float compare.
    //   ISSUER_STAKE / SUMMED_TOTAL_306 = 0.0010642836…
    let ppt = (ISSUER_STAKE as u128) * 10_000_000_000_000u128 / (SUMMED_TOTAL_306 as u128);
    assert_eq!(
        ppt, 10_642_835_776,
        "control: the summed denominator must reproduce the sigma that rejected the header \
         (0.0010642835776 — the census reported it rounded to 0.10642836%)"
    );

    // Post-fix: the carried snapshot total. The retired pool STAYS in the set (membership is correct
    // per the oracle and is NOT what changes) but no longer moves the denominator.
    let fixed = distr(
        TOTAL_307,
        &[(0x01, ISSUER_STAKE), (0x02, RETIRED_POOL_STAKE)],
    );
    let view = fixed.to_pool_distr_view(ASC);
    let numer = view
        .pool_active_stake(EpochNo(306), &pid(0x01))
        .expect("issuer present");
    let denom = view
        .total_active_stake(EpochNo(306))
        .expect("total present");

    assert!(
        ratio_gt(numer, denom, ISSUER_STAKE, SUMMED_TOTAL_306),
        "the fix must RAISE sigma — the summed denominator was inflated by the retired pool"
    );

    // The rejected header needed sigma multiplied by at least 1.00031523. Integer form:
    //   fixed/summed > 1.00031523  <=>  fixed * 100_000_000 > summed * 100_031_523
    let fixed_num = (numer as u128) * (SUMMED_TOTAL_306 as u128);
    let summed_num = (ISSUER_STAKE as u128) * (denom as u128);
    assert!(
        fixed_num * 100_000_000u128 > summed_num * 100_031_523u128,
        "sigma must clear the threshold the rejected header actually needed (+0.031523%)"
    );

    assert!(
        fixed.pools.contains_key(&pid(0x02)),
        "membership is correct per the oracle — this slice must NOT fix the symptom by deleting the \
         retired pool"
    );
}

// ===========================================================================
// CE-LV1-3 — the freeze captures the credential-side sum, UNFILTERED.
// ===========================================================================

#[test]
fn the_credential_side_sum_includes_stake_the_membership_filter_would_drop() {
    use ade_ledger::epoch::StakeSnapshot;
    use ade_types::tx::PoolId;
    use ade_types::Coin;

    let mut snap = StakeSnapshot::new();
    // Two credentials delegating to two different pools. Only ONE pool will survive the VRF
    // intersection, but cardano's sumAllStake folds the credential map regardless.
    snap.delegations
        .insert(pid(0xAA), (PoolId(pid(0x01)), Coin(700)));
    snap.delegations
        .insert(pid(0xBB), (PoolId(pid(0x02)), Coin(300)));
    snap.pool_stakes.insert(PoolId(pid(0x01)), Coin(700));
    snap.pool_stakes.insert(PoolId(pid(0x02)), Coin(300));

    assert_eq!(
        snap.total_active_stake(),
        1000,
        "sumAllStake folds the CREDENTIAL side and is blind to which pools survive any filter"
    );

    let mut delegated = std::collections::BTreeSet::new();
    delegated.insert(PoolId(pid(0x01)));
    delegated.insert(PoolId(pid(0x02)));
    let mut vrfs = BTreeMap::new();
    vrfs.insert(PoolId(pid(0x01)), h32(0x01));

    let frozen = FrozenLeadershipPoolDistr::from_boundary_snapshot(
        EpochNo(306),
        SlotNo(130_118_358),
        h32(0xB9),
        h32(0xF3),
        &delegated,
        &snap.pool_stakes,
        &vrfs,
        snap.total_active_stake(),
    );

    assert_eq!(
        frozen.pools.len(),
        1,
        "pool 0x02 has no registered VRF and is filtered out"
    );
    assert_eq!(
        frozen.total_active_stake, 1000,
        "but its 300 still counts in the denominator — dropping it would DEFLATE the denominator, \
         inflate every sigma and every threshold, and make Ade ACCEPT a header cardano rejects"
    );
    assert_eq!(
        frozen
            .to_pool_distr_view(ASC)
            .total_active_stake(EpochNo(306)),
        Some(1000)
    );
}

// ===========================================================================
// CE-LV1-6 / CE-LV1-7 — codec round-trip, and the total is inside the commitment.
// ===========================================================================

#[test]
fn the_total_round_trips_and_is_committed_to() {
    use ade_ledger::frozen_leadership::{
        canonical_hash, decode_frozen_leadership, encode_frozen_leadership,
    };

    let d = distr(TOTAL_307, &[(0x01, ISSUER_STAKE)]);
    let bytes = encode_frozen_leadership(&d);
    let back = decode_frozen_leadership(&bytes).expect("round-trip");
    assert_eq!(back, d);
    assert_eq!(back.total_active_stake, TOTAL_307);

    // The field is AUTHORITY, not decoration: identical pool sets with different totals must commit
    // differently, or a wrong denominator could be sealed undetected.
    let other = distr(SUMMED_TOTAL_306, &[(0x01, ISSUER_STAKE)]);
    assert_eq!(other.pools, d.pools, "control: identical pool sets");
    assert_ne!(
        canonical_hash(&d),
        canonical_hash(&other),
        "the denominator must be inside the canonical commitment"
    );
}
