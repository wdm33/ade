# Leadership Distribution Authority Trace (the S4 discovered-proof-failure follow-up)

> **Status: OPEN. A TRACE slice — it fixes nothing.** Its only job is to prove EXACTLY what cardano uses for
> leadership (`nesPd`) and where Ade must source it, so the follow-on `S4-pre: Frozen Leadership Distribution`
> slice can add the right persisted state (not a guessed one). Compatibility oracle, not architecture law:
> learn the observable reference semantics before deciding the internal state shape.

**Cluster:** LIVE-LEDGER-EPOCH-TRANSITION. **Why this exists:** the S4 same-epoch identity gate
(`ce3d_boundary_differential.rs::s4_same_epoch_authority_identity_gate`) FAILED — at epoch 1338 the seed
leadership `PoolDistr` has **659** pools but the accumulator's mark/set/go snapshots have **626/627/626** and
its active `cert_state.pool.pools` has 703; **no accumulator snapshot reproduces the leadership pool-set**,
33 leadership pools are absent from go (32 zero-stake + **1 real 1,000,000,000,000-lovelace pool**), and that
1 pool has NO active cert-state params. So `from_accumulator` (go stake + active-params VRF) cannot be the
leadership authority. The contract premise "stake ← go, vrf ← active cert params" is FALSE for the fixture.

## 1. Deliverables (prove, do not change)

1. **Confirm the exact cardano source of `nesPd` for epoch 1338** (which ledger-state field / computation).
2. **Identify which snapshot phase feeds it** (mark / set / go / a separately-frozen `nesPd`).
3. **Prove why leadership=659 while mark/set/go=626/627/626** (the inclusion-rule / freeze-point difference).
4. **Classify the 33 extra pools:** the 32 zero-stake structural pools, and the 1 retired/non-active 1M-ADA pool.
5. **Identify the VRF source for each leadership pool** (snapshot-frozen `ssPoolParams` vs active `cert_state`).
6. **Confirm the missing 1M-ADA pool's lifecycle:** retired, re-registered, or absent from active pool params.
7. **Produce a reference fixture:** `pool_id -> active_stake -> vrf_keyhash -> source/lifecycle status` for all
   659 leadership pools.

## 2. Acceptance (the trace is green only when)

The reference fixture EXACTLY reproduces the seed leadership `PoolDistr`:
- **659 pools** (same key-set),
- same per-pool `active_stake`,
- same per-pool `vrf_keyhash`,
- same ordering / canonical bytes.

i.e. a test rebuilds the leadership `PoolDistrView` from the classified reference fixture and asserts it is
byte-identical to `from_seed_epoch_consensus_inputs(seed record)` — the proven-byte-exact cardano leadership
view. No production code changes; no authority swap; no ceiling deletion.

## 2b. FINDINGS — GREEN (`ce3d_boundary_differential::ldat_classify_leadership_pools`, v5 seed @ epoch 1338)

The 659 leadership pools cross-referenced against the accumulator's full seed-epoch state:

| classification | count |
|---|---|
| stake matches GO snapshot | 538 |
| stake matches SET snapshot (not go) | 89 |
| stake in NO snapshot (all zero-stake) | 32 |
| VRF == active cert params (byte-exact) | **658** |
| VRF MISSING from active params | **1** |
| zero-stake registered | 32 |
| in `retiring` / `future_pools` | 0 / 0 |

**Answers (deliverables 1–7):**
1–2. **Leadership `nesPd` stake = the SET snapshot.** 538 + 89 = 627 = the SET pool count; every non-zero
   leadership pool's stake matches SET (538 also match go only because their stake was unchanged go→set). The
   **go snapshot (626) is the REWARD stake** (CE-3d) — a DIFFERENT snapshot; leadership is NOT go.
3. **659 = SET's 627 non-zero pools + 32 zero-stake registered pools.** go (626) differs from leadership
   because it is the reward snapshot and drops the one retired pool SET still carries.
4. **The 33 pools absent from go = 32 zero-stake registered + 1 retired 1M-ADA pool** (present in SET, not go).
5. **VRF = the snapshot-FROZEN pool params.** The accumulator's ACTIVE params reproduce **658/659 byte-exact**;
   the single exception is the retired pool.
6. **The 1M-ADA pool (`cd3114d6…`) is RETIRED + POOLREAP'd** — absent from active `pools`, `retiring`, AND
   `future_pools`. Its stake survives (frozen) in the SET snapshot; its VRF does NOT survive in active state.
7. **Reference fixture = the 659-pool distr, reconstructed BYTE-EXACT** from (SET stake + active-params VRF +
   exactly **ONE** frozen-VRF supplement for the retired pool). `frozen_vrf_supplements == 1`.

**The irreducible gap S4-pre must close:** the leadership pool params' **VRF must be FROZEN at the SNAP
boundary** so a retired/POOLREAP'd pool's VRF survives — the active `cert_state.pool.pools` cannot supply it.
Stake is recoverable from SET, but a **self-contained** `FrozenLeadershipPoolDistr { (stake, vrf) per pool }`
(the user's shape) is the robust design: it does not depend on SET-snapshot retention for a leadership answer.

## 3. Out of scope (this slice does NOT do)

- No new persisted accumulator state, no `FrozenLeadershipPoolDistr` type — that is the follow-on `S4-pre`.
- No production authority swap, no seed+2 ceiling deletion — S4 stays BLOCKED.
- No change to `from_accumulator` (the failed hypothesis) beyond quarantining it so it cannot be mistaken for
  production authority (rename to `_go_active_params_for_test_only` or delete once the frozen builder exists).

## 4. Feeds

The output (deliverable #7 fixture + the classification) is the input to `S4-pre: Frozen Leadership
Distribution Authority`, which adds the persisted `FrozenLeadershipPoolDistr { epoch, source_boundary,
pools: BTreeMap<PoolKeyHash, LeadershipPoolEntry{active_stake, vrf_keyhash}> }`, sourced at bootstrap from a
named/bound artifact (fail-closed if absent) and re-frozen at each SNAP from the snapshot-frozen stake +
params — NEVER from go + active cert_state. S5 recovery equivalence must then cover this new authority too.
S4 (the narrow flip) resumes only after S4-pre proves the frozen leadership distribution byte-exact.
