# SLICE LV-3 — an owner's stake counts for a pool only when it is delegated to that pool

**Status:** APPROVED 2026-10-11 (owner: "its own slice, before or with LV-2"; implement, then LV-2).
Nothing is implemented at this commit. It reverses a recorded project rule; §6 says what that
reversal rests on and what it does not.

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
fn self_delegated_owner_stake(go: &StakeSnapshot, pool_id: &PoolId, owners: &[Hash28]) -> u64
```

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
different reward balances from one under the new. It ships under the SAME `STORE_SEMANTICS_VERSION`
7 → 8 bump as LV-2 — one bump, one refusal of v7, one re-bootstrap — and therefore lands with LV-2,
not ahead of it in a separate release.

---

## 8. EXPLICITLY NOT IN THIS SLICE

- **The key/script credential collision** (E1 evidence §5; 11,802 lovelace). Separate defect, separate
  cause (a bare-hash key in the go snapshot). It fails CE-LV2-3's "all 11,056" as written. Owner
  decision 2026-10-11: it is fixed as its own slice (LV-4) before the live run, so LV-2's bar stays
  "every credential matches" with no named exception.
- **LV-2 itself.** The seed-boundary authority is untouched here.
- **Any preview claim.**
