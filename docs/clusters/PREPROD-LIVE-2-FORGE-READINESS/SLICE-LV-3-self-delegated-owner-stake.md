# SLICE LV-3 — an owner's stake counts for a pool only when it is delegated to that pool

**Status:** IMPLEMENTED 2026-10-11 — every CE below is green; results in §9. DC-EPOCH-42 is
registered `enforced`. It reverses a recorded project rule; §6 says what that reversal rests on and
what it does not.

**Entry state:** `8c1328df`. Evidence:
`docs/evidence/run-stores/preprod-live2c/lv2-e1-native-seed-reward-vs-cardano.md` §4.

---

## 1. THE CONTRACT — answered before any design

> **Whose stake satisfies a pool's pledge, and whose stake is the `s` in the leader reward?**

**cardano** (node 11.0.1, PV 11, measured; and cardano-ledger `master`, quoted):

```haskell
selfDelegatedOwners =
  Set.filter (\ownerKeyHash -> KeyHashObj ownerKeyHash `Set.member` spsDelegators) spsOwners

maxP = if pledge <= selfDelegatedOwnersStake then maxPool' … else mempty
poolOwnerRelativeStake = selfDelegatedOwnersStake %? totalStake        -- the leader term's s
notPoolOwner (spssSelfDelegatedOwners …) hk                            -- member exclusion
```

One set — the owners **who delegate to this pool** — drives all three: the pledge gate, the leader
term, and which delegators are excluded from member rewards.

**Ade** (`rules.rs`): the pledge gate and the leader term each sum every listed owner's go stake
"regardless of their delegation". The member exclusion already uses the self-delegated set in effect,
because it only ever iterates this pool's delegators.

> **Therefore: an owner who delegates elsewhere contributes nothing to this pool's pledge and nothing
> to its leader term.**

### 1.1 Consequence, measured

Preprod, boundary 304→305. Pool `13e8600b…2418a`: pledge 4,500,000,000,000; sole owner
`d2cfce07…8ee5f` holds 5,275,349,445,153 delegated to pool `8aa46908…0258b`. Cardano's go snapshot:
`selfDelegatedOwners: []`, `selfDelegatedOwnersStake: 0` → the pool earns nothing. Ade pays it
6,218,191,443 (136 accounts) and over-draws reserves by the same amount — at every boundary the pool
produces in.

---

## 2. INVARIANT

**DC-EPOCH-42 (new; registered with the implementation).** In the reward calculation a pool's owner
stake is the go-snapshot stake of the owners **delegated to that pool in the go snapshot**. It is
defined once and that one value feeds the pledge-satisfaction gate and the leader-reward term. An
owner delegated to another pool, or not delegated, contributes zero to both.

The pledge gate currently has **no registry rule and no test** — 898 `ade_ledger` tests pass
unchanged with either definition (§4, P2). This slice gives it both.

---

## 3. DESIGN

One helper, used at both sites; the two inline sums are deleted:

```rust
/// cardano `spssSelfDelegatedOwnersStake`: go stake of the owners delegated to THIS pool.
impl StakeSnapshot {
    pub fn self_delegated_owner_stake(&self, pool_id: &PoolId, owners: &[Hash28]) -> u64
}
```

(As built it is a method on the snapshot, next to `total_active_stake`, so an integration test can
call it directly. The reward path calls it once per pool and both sites read that binding.)

Two definitions of one quantity is how the sites could drift; one definition is the structural part
of the fix.

### 3.1 What does NOT change

`maxPool`, apparent performance, the member formula, the member exclusion, the pre-Mary
reward-account fallback (`protocol_major < 4`), registration filtering, pots arithmetic.

### 3.2 Era gating — decided by P3, not assumed

Ade's reward function also serves the historical-era corpus comparisons (CE-71/CE-72, mainnet
Shelley→Conway). If any of those change under the new rule, the rule is gated to the protocol
versions where the reference demonstrably uses it and the gate is justified from the reference's
history. If none change, no gate is added — an unexercised gate is an unproven branch.

**Decided by P3:** the comparisons do change, and toward the reference (exact at Conway PV 9). No
gate.

---

## 4. SLICE-ENTRY OBLIGATIONS

| # | Question | Result |
|---|---|---|
| **P1** | With the self-delegated rule, does the preprod 304→305 comparison close? | **YES — run 2026-10-10** with a temporary local edit (reverted). Reserves draw 7,422,111,617,574 = cardano's `deltaR` exactly (was +6,218,191,443). The pool's 135 members and its reward account drop out of the residuals. What remains is exactly the non-pledge residuals already attributed in the E1 evidence |
| **P2** | Does any `ade_ledger` test pin either definition? | **NO — run 2026-10-10.** 898 pass under the new rule, as under the old. The gate is unwitnessed today |
| **P3** | Do the mainnet historical-era reward comparisons (`ade_testkit` `epoch_oracle_comparison`) change under the new rule? | **YES, toward the reference — run 2026-10-10.** §4.1 |
| **P4** | The preview CE-3d differential that motivated the old rule: does it still hold under the new one? | **CANNOT BE RE-RUN LOCALLY.** Its reference ledger states (`~/.cardano-ce3d-extract/db/ledger/*`) are gone and its seed stores pre-date store v7. See §6 |

### 4.1 P3 result — mainnet agrees with the new rule

`epoch_oracle_comparison`, all 27 tests, full output, single-threaded, once per rule (same sources,
one temporary edit, reverted). 26 pass and 1 fails under BOTH rules — the failure
(`committee_oracle_mainnet_575_576_noop_agreement`, a governance ratify gate) is pre-existing and
does not touch rewards. 52 output lines differ, all reward sums in two diagnostic tests.

The decisive one is mainnet **Conway 507→508 (PV 9)**, the variant that tracks the reference:

```
reference reserves decrease                    13,468,467,572,031
Ade, full-go owner stake (today)               13,468,475,149,300     +7,577,269
Ade, self-delegated owner stake                13,468,467,572,031     EXACT
```

(`dr1` 23,284,829,384,930 − (`pool_pot` 18,675,457,270,785 − `sum_rewards`); `sum_rewards` moves
8,859,103,035,155 → 8,859,095,457,886.)

Babbage 365→366 (PV 7) moves the same direction, 100.0182% → 100.0177%, with an unrelated residual
remaining. Alonzo and Mary have no variant close enough to judge.

So the new rule is not a preprod or PV-11 peculiarity: on mainnet at PV 9 it turns a 7.6M-lovelace
miss into an exact match. **No era gate is added** (§3.2).

---

## 5. MECHANICAL ACCEPTANCE CRITERIA

| CE | Criterion | judged by |
|---|---|---|
| **CE-LV3-1** | **THE INVARIANT.** A pool whose only owner delegates to a different pool, with pledge > 0, earns nothing: no leader reward, no member rewards, no reserves draw on its behalf | unit |
| **CE-LV3-2** | An owner delegated to the pool counts; the same owner re-pointed to another pool stops counting — for the gate AND the leader term, from one definition | unit |
| **CE-LV3-3** | Mixed owners: only the self-delegated subset is summed, in both sites | unit |
| **CE-LV3-4** | **THE REAL CASE.** On the preprod seed state the native reward update's reserves draw equals cardano's 7,422,111,617,574 and pool `13e8600b…2418a`'s members receive nothing | evidence harness (`seed_boundary_reward_census`) |
| **CE-LV3-5** | There is exactly one definition of owner stake in the reward path; neither site sums owners inline | structural gate |
| **CE-LV3-6** | Every existing reward suite passes, or each change is named and matched to the reference (P3) | existing suites |
| **CE-LV3-7** | Negative-tested | mutations below |

### Required mutations

Restore the full-go sum at the pledge gate (must fail CE-LV3-1 and CE-LV3-4) · restore it at the
leader term only (must fail CE-LV3-2) · sum all owners in the helper (must fail CE-LV3-3) · inline a
second sum (must fail CE-LV3-5).

---

## 6. ON REVERSING THE RECORDED RULE

The rule being reversed: *leader/pledge owner stake is the owner's FULL go stake regardless of pool;
never filter owners by delegation target* (2026-06-30).

**What the reversal rests on:** a direct measurement on the venue Ade must follow — the reference's
own snapshot field reads zero for this pool and its reward update pays the pool nothing — plus the
reference source defining the quantity that way, plus P1 closing the gap to the lovelace.

**And on mainnet history (P3):** Conway 507→508 becomes exact under the new rule and is 7,577,269
off under the old one — a third, independent venue and protocol version.

**What it does not rest on:** an explanation of the earlier result. The 2026-06-30 note records that
a this-pool-only filter caused underpayment in the preview CE-3d differential. That differential
cannot be re-run from local data (P4). Possible readings, none verified: the filter tried then was
implemented differently; the underpayment had another cause since fixed (the CE-3d line later closed
several — post-RUPD mark, fee pot, discriminant); or no preview pool in those epochs discriminated.

**The owed check, stated so it is not lost:** when LV-2's live run crosses its first native boundary
on preprod, the sealed distribution is compared pool-by-pool against the reference (CE-LV2-10b). That
is a full-distribution test of this rule on a second boundary. A preview re-check needs a reference
node rebuilt for preview by the same method used here; it is not part of this slice.

---

## 7. STORE SEMANTICS

This changes what a boundary computes, so a store that crossed a boundary under the old rule can hold
different reward balances from one under the new.

**Amended at implementation.** The draft said LV-3 would share LV-2's 7 → 8 bump. The semantics lock
does not allow that across two commits: the reward rule is inside the hashed surface, so this
commit's gate fails until it records its own choice, and a later non-neutral entry must strictly
increase the version. So **LV-3 takes 7 → 8 here and LV-2 will take 8 → 9**. What the draft was
protecting is unchanged: no store is created at v8 in between, so there is still one refusal of v7
and one re-bootstrap. LV-2's doc is corrected to 8 → 9 when LV-2 lands.

---

## 8. EXPLICITLY NOT IN THIS SLICE

- **The key/script credential collision** (E1 evidence §5; 11,802 lovelace). Separate defect, separate
  cause (a bare-hash key in the go snapshot). It fails CE-LV2-3's "all 11,056" as written. Owner
  decision 2026-10-11: it is fixed as its own slice (LV-4) before the live run, so LV-2's bar stays
  "every credential matches" with no named exception.
- **LV-2 itself.** The seed-boundary authority is untouched here.
- **Any preview claim.**

---

## 9. RESULT — 2026-10-11

| CE | Result |
|---|---|
| CE-LV3-1 | GREEN — `a_pool_whose_owner_delegates_elsewhere_earns_nothing`: no leader reward, no member reward, `sum_rewards == 0`, the whole pot returns to reserves |
| CE-LV3-2 | GREEN — `the_same_owner_delegated_to_the_pool_satisfies_the_pledge` (gate, and the non-vacuity control for CE-LV3-1); `the_leader_term_counts_only_owners_delegated_to_the_pool` (leader term, pledge 0) |
| CE-LV3-3 | GREEN — `only_the_self_delegated_subset_of_owners_is_summed` (the definition directly; also a repeated and an undelegated owner); `a_pledge_met_only_by_counting_owners_elsewhere_is_not_met` (one lovelace either side of the pledge) |
| CE-LV3-4 | GREEN — `seed_boundary_reward_census` on the preprod seed, now ASSERTING: reserves draw 7,422,111,617,574 = the reference `deltaR`; no credential of pool `13e8600b…2418a` is paid. 11,051 of 11,056 credentials exact; the five that differ are the non-pledge residuals of the E1 evidence |
| CE-LV3-5 | GREEN — `ci/ci_check_owner_stake_authority.sh` |
| CE-LV3-6 | GREEN — `ade_ledger` suite passes; `epoch_oracle_comparison` 26 pass / 1 fail, the same pre-existing governance failure as before the change (§4.1), with the reward sums moving exactly as P3 recorded |
| CE-LV3-7 | GREEN — below |

### Mutations (each asserted applied; tree restored byte-exact)

| Mutation | Tests that fail | Gate |
|---|---|---|
| full-go sum at the pledge gate | CE-LV3-1, the mixed-owner boundary test; CE-LV3-4 (draw 7,424,810,407,082 ≠ 7,422,111,617,574) | fails |
| full-go sum at the leader term only | `the_leader_term_counts_only_owners_delegated_to_the_pool` | fails |
| the helper sums all owners | four of five, including CE-LV3-3's direct test | fails |
| a second, arithmetically equivalent inline sum | none — the tests cannot see it | **fails** (the only witness, as intended) |

### The harness now reads the seed directly

`EpochAccumulatorStore::open` refuses the archived v7 store under this binary, which is correct. The
evidence harness therefore reads the sealed bootstrap blob straight from the store's meta table and
decodes it with the production decoder. The seed is imported state that no boundary rule has
touched, so it stays valid evidence across this bump. No production gate is weakened.

### Not covered

Pools with an EMPTY owner list still skip the pledge check, as before. cardano would compare the
pledge against zero. No such pool with a positive pledge was observed on preprod or in the mainnet
comparisons, so it is left as found and named here rather than changed without a witness.
