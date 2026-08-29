# LEADERVALUE ORACLE EXTRACTION — the leader-check σ denominator, from cardano-ledger

> **No production code.** This answers the two entry-obligation questions left open by
> `leadervalue-census-sigma-denominator.txt` and specifies the minimal change. Implementation is a
> separate slice. Nothing in `from_boundary_snapshot`, `to_pool_distr_view`, `check_leader_claim` or
> the B12 signal is touched here.

## The two questions

From the census, after the "stale copy" reading was withdrawn:

> **(a)** Does cardano's `nesPd` for epoch E include a pool that retires at the boundary into E−1?
> **(b)** Is the leader-check σ denominator the sum of the `PoolDistr` entries, or the snapshot's
> total stake?

## Method — the rule, at TWO independent points in the ledger's history

`IntersectMBO/cardano-ledger`, quoted verbatim. Two versions deliberately, because `master` has been
refactored since Conway (`StakePoolSnapShot`, Leios BLS keys) and a finding that held only on `master`
would not be evidence about the chain preprod is actually running:

1. **`cardano-ledger-conway-1.17.0.0`** — contemporary with the venue (cardano-node 11.0.1).
2. **`master`** — to show the property survived the refactor rather than being an artefact of one release.

---

## (b) THE DENOMINATOR — answered, and Ade is wrong

### Conway-era, `libs/cardano-ledger-core/src/Cardano/Ledger/EpochBoundary.hs`

```haskell
data SnapShot c = SnapShot
  { ssStake       :: !(Stake c)
  , ssDelegations :: !(VMap VB VB (Credential 'Staking c) (KeyHash 'StakePool c))
  , ssPoolParams  :: !(VMap VB VB (KeyHash 'StakePool c) (PoolParams c))
  }

-- inside calculatePoolDistr'
let total = sumAllStakeCompact stake

sumAllStakeCompact :: Stake c -> CompactForm Coin
sumAllStakeCompact = VMap.foldl (<>) mempty . unStake
```

### master, `libs/cardano-ledger-core/src/Cardano/Ledger/State/SnapShots.hs`

```haskell
data SnapShot = SnapShot
  { ssActiveStake      :: !ActiveStake
  , ssTotalActiveStake :: !(NonZero Coin)
  , ssStakePoolsSnapShot :: !(VMap VB VB (KeyHash StakePool) StakePoolSnapShot)
  }

mkSnapShot ssActiveStake ssStakePoolsSnapShot =
  let ssTotalActiveStake = sumAllActiveStake ssActiveStake

calculatePoolDistr' :: (KeyHash StakePool -> Bool) -> SnapShot -> PoolDistr
calculatePoolDistr' includeHash (SnapShot _ activeStake stakePoolSnapShot) =
  let toIndividualPoolStake poolId spss = do
        guard (includeHash poolId)
        guard (spssNumDelegators spss > 0)
        Just IndividualPoolStake {...}
      poolDistr = PoolDistr
        { unPoolDistr = VMap.toMap $ VMap.mapMaybeWithKey toIndividualPoolStake stakePoolSnapShot
        , pdTotalActiveStake = activeStake
        }
   in poolDistr
```

```haskell
data PoolDistr = PoolDistr
  { unPoolDistr        :: !(Map (KeyHash StakePool) IndividualPoolStake)
  , pdTotalActiveStake :: !(NonZero Coin)
  -- ^ Total stake delegated to registered stake pools. …
  }

data IndividualPoolStake = IndividualPoolStake
  { individualPoolStake :: !Rational
  -- ^ … a ratio of `individualTotalPoolStake`/`pdTotalActiveStake`
  , individualTotalPoolStake :: !(CompactForm Coin)
  , … }
```

### THE ANSWER

**The denominator is folded over the STAKE (credential) map, never over the pool map.** Conway folds
`unStake`; `master` folds `ssActiveStake` in `mkSnapShot` and then `calculatePoolDistr'` merely
*copies* it (`pdTotalActiveStake = activeStake`). Both are the same property stated two ways.

**The membership filter cannot move the denominator.** `guard (includeHash poolId)` and
`guard (spssNumDelegators spss > 0)` remove entries from `unPoolDistr` **only**. `pdTotalActiveStake`
is already fixed by the time the filter runs.

Ade does the opposite:

```rust
// crates/ade_ledger/src/frozen_leadership.rs — to_pool_distr_view
let mut total_active_stake: u64 = 0;
for (pool_id, entry) in &self.pools {
    total_active_stake = total_active_stake.saturating_add(entry.active_stake);
```

**Ade derives the denominator from the pool map.** So in Ade — and in nothing cardano does — a pool
entering or leaving the leadership set moves *every other pool's* σ. That is the defect, and it is
structural rather than a one-off: it is wrong in both directions.

* A pool cardano keeps in the map but whose stake cardano counts anyway → no divergence (the common
  case, which is why epoch 307 matched to five significant figures).
* A pool present in Ade's map whose delegators' stake is **not** in cardano's `ssStake` at that
  snapshot → Ade's denominator is too **high**, every σ too **low**, every leader threshold too
  **low**, and Ade spuriously rejects headers whose VRF value lands in the resulting band. **This is
  the observed failure**, worth 3.879% at epoch 306.
* The inverse — stake in cardano's `ssStake` delegated to a pool Ade filtered out — makes Ade's
  denominator too **low**, every σ too **high**, and Ade would spuriously **ACCEPT** a header cardano
  rejects. That direction has never been observed and is the more dangerous one.

## (a) MEMBERSHIP — answered, and Ade is RIGHT

Conway derives membership by folding `ssDelegations`; `master` states it directly as
`guard (spssNumDelegators spss > 0)`. Both mean *pools with at least one delegator*.

Ade's rule — `delegated_pools` = the image of the pre-POOLREAP delegation map, intersected with the
registered pool VRFs — is the same rule. **Ade's membership, including its deliberate retention of
pools retiring at the boundary, matches.** The `from_boundary_snapshot` doc comment and DC-EPOCH-24's
658-of-703 finding stand.

So the retired pool `8ed5ab11…eea88` being present in epoch 306's set is **not** the bug. Counting its
63,075,223,742,053 lovelace in the denominator is.

## The live cross-check, so this is not source-reading alone

`cardano-cli query stake-snapshot` reports `total.stakeGo/Set/Mark` — that is `ssTotalActiveStake`.
Ade's epoch-307 σ for the probe pool is `0.10957157%`; cardano's derived from that field is
`0.10958%`. Five significant figures, at the one epoch where no membership change exposes the
difference. The source reading predicts exactly that, and it is what the venue shows.

## THE MINIMAL CHANGE (specified, NOT implemented)

`FrozenLeadershipPoolDistr` must **carry** the snapshot-level total as its own field — cardano's
`pdTotalActiveStake` — captured at freeze time from the boundary mark's credential-side stake sum, and
`to_pool_distr_view` must return it instead of summing `pools`.

That is a one-field change to a durable, canonically-encoded, hash-committed authority object:

* `FROZEN_LEADERSHIP_SCHEMA_VERSION` bump, and the canonical encoder/decoder gain the field;
* every existing sealed object lacks it, so **`STORE_SEMANTICS_VERSION` must bump too** (a v6 store
  cannot be reinterpreted — its leadership objects have no total to read, and re-deriving one by
  summing entries is precisely the bug). `ci/ci_check_store_semantics_lock.sh` runs **in the same
  commit**;
* the field is fingerprint-bearing: it changes leader schedules, so it is authority, not diagnostics.

The non-vacuity test writes itself from the data already in hand: at epoch 306 the sealed set contains
a pool worth 3.879% of the summed total; a correct denominator must NOT move when that pool is added
to or removed from the map, and the issuer's σ must land at cardano's ~0.1096% rather than 0.10643%.

## WHAT IS NOT CONCLUDED

* **The value** of `ssTotalActiveStake` at the 304→305 boundary is still not known. This extraction
  establishes *which quantity* Ade must capture, not what it equals. Confirming the fixed number
  against a reference still needs the epoch-306 `nesPd` (AWS reference node / LDAT harness).
* **Whether the denominator is the only defect.** The census also showed Ade's epoch-306 numerator for
  the probe pool sitting ~0.8% above cardano's nearby values. That may be ordinary epoch drift or a
  second, smaller issue; it is not settled here and must not be assumed away by a green header.
* **No claim that removing the retired pool is the fix.** It is not. Membership is correct; the
  denominator's *derivation* is what changes.
