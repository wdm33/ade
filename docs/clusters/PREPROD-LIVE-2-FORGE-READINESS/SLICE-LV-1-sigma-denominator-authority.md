# SLICE LV-1 — the σ denominator is a snapshot fact, not a sum of the pools that survived a filter

**Entry state:** `4f3cd689`. The oracle extraction
(`leadervalue-oracle-extraction-sigma-denominator.md`) answered both entry-obligation questions from
verbatim cardano-ledger source at two points in its history. This slice implements what that
answered. It re-opens nothing: membership stays exactly as it is.

---

## 1. THE CONTRACT — answered before any design

> **What is the denominator of the leader-check σ, and what may change it?**

**cardano:** `pdTotalActiveStake` is folded over the **stake (credential) map** — Conway
`sumAllStakeCompact = VMap.foldl (<>) mempty . unStake`; master `sumAllActiveStake ssActiveStake` in
`mkSnapShot`, which `calculatePoolDistr'` then copies verbatim. The membership guards
(`includeHash`, `spssNumDelegators > 0`) filter `unPoolDistr` **only**, and run *after* the total is
fixed.

> **Therefore: which pools appear in the distribution CANNOT change the denominator.**

**Ade:** `to_pool_distr_view` sums `self.pools` — the set that survived
`delegated_pools ∩ registered_pool_vrfs`. So membership moves the denominator, and with it every
other pool's σ.

That is the defect. It is not a wrong value; it is a wrong *kind* of quantity.

### 1.1 Why this is a correctness bug in both directions

| case | Ade's denominator | every σ | every threshold | consequence |
|---|---|---|---|---|
| a pool is in Ade's set whose delegators' stake cardano does not count | too **high** | too **low** | too **low** | **spurious REJECT** — the observed failure, 3.879% at epoch 306 |
| stake cardano counts, delegated to a pool Ade filtered out | too **low** | too **high** | too **high** | **spurious ACCEPT** — never observed, and the dangerous half |

The second row is why this cannot be left as an operational nuisance. A denominator that is too low
makes Ade admit a header cardano rejects — a consensus divergence, silent, on the receive path.

---

## 2. INVARIANT

**DC-EPOCH-47 (new).** The leadership σ denominator is the **snapshot's total active stake**, captured
at freeze time as the credential-side sum of the boundary mark, and carried on the frozen leadership
authority as its own field. It is **invariant under the leadership pool-set membership filter**:
adding or removing a pool from `FrozenLeadershipPoolDistr::pools` must not change it. It is never
re-derived by summing the surviving entries, at freeze time or at read time.

**DC-EPOCH-24 / DC-EPOCH-25 — untouched.** Membership (`numDelegators > 0`, intersected with the
registered VRFs, retaining pools retiring at this boundary) matches `calculatePoolDistr'` and is
proven at 658/703. This slice re-opens none of it.

---

## 3. DESIGN

### 3.1 The field

```rust
pub struct FrozenLeadershipPoolDistr {
    pub target_leadership_epoch: EpochNo,
    pub source_slot: SlotNo,
    pub source_hash: Hash32,
    pub source_checkpoint_commitment: Hash32,
    pub total_active_stake: u64,   // DC-EPOCH-47 — cardano's pdTotalActiveStake
    pub pools: BTreeMap<Hash28, LeadershipPoolEntry>,
}
```

`to_pool_distr_view` returns that field. The summing loop is **deleted**, not bypassed — a fallback
that sums when the field looks unset would reintroduce the bug on exactly the stores that need the
fix most.

### 3.2 Where the value comes from

Cardano folds `unStake` — the credential side. Ade's `StakeSnapshot` carries it directly:

```rust
pub struct StakeSnapshot {
    pub delegations: BTreeMap<Hash28, (PoolId, Coin)>,   // <- cardano's ssStake
    pub pool_stakes: BTreeMap<PoolId, Coin>,
}
```

So at the boundary freeze the total is `Σ mark.delegations.values().1` — **unfiltered**, taken before
any membership decision. `from_boundary_snapshot` gains it as a parameter;
`cross_epoch_boundary_transition` supplies it from the same just-rotated mark it already reads
`pool_stakes` from, so no new input and no new read.

Deliberately the credential side rather than `Σ pool_stakes`, even where the two agree today: the
oracle folds credentials, and matching the oracle's *definition* is what stops the next membership
change from reopening this.

### 3.3 The other two constructors

- **`from_mark_pool_distr`** (bootstrap, seed+1): the imported mark PoolDistr must carry the imported
  snapshot's total. If the bootstrap record cannot supply it, this **fails closed** — it does not sum
  the entries as a fallback. [[versioned-field]]
- **`from_seed_epoch_consensus_inputs`** (seed): `SeedEpochConsensusInputs` already carries
  `total_active_stake` (`consensus_view.rs` reads it today), so the seed path uses it directly.

### 3.4 What does NOT change

`from_boundary_snapshot`'s membership rule, `check_leader_claim`, the Q.123 arithmetic,
`taylor_exp_cmp_le`, the ForgeTick path, B12's signal, the co-advance pass order, the boundary
positioner.

---

## 4. STORE SEMANTICS — TWO bumps, and they are not optional

`FrozenLeadershipPoolDistr` is durable, canonically encoded, and hash-committed
(`canonical_hash`). Adding an authority-bearing field changes its bytes and its hash.

- `FROZEN_LEADERSHIP_SCHEMA_VERSION` **6 → 7**; encoder/decoder gain the field; the outer array grows
  6 → 7 elements.
- `STORE_SEMANTICS_VERSION` **6 → 7**. A v6 store's sealed leadership objects have **no total to
  read**, and reconstructing one by summing their entries is precisely the defect. They are not
  reinterpretable, so they are refused. `ci/ci_check_store_semantics_lock.sh` runs **in the same
  commit** — the v3→v4 and v4→v5 bumps both skipped their own gate and only a 100-commit audit found
  it. [[version-bump-gate]]

⚠ **This retires the reproducer.** The v6 preprod store halts deterministically on the failing header
in ~2 minutes, and a v7 binary will refuse to open it. That cost is accepted knowingly, and §6 says
what replaces it.

---

## 5. MECHANICAL ACCEPTANCE CRITERIA

| CE | Criterion | judged by |
|---|---|---|
| **CE-LV1-1** | **THE INVARIANT.** Adding a pool to `pools`, or removing one, leaves `to_pool_distr_view`'s `total_active_stake` **unchanged** | unit |
| **CE-LV1-2** | **THE REAL CASE.** With epoch 306's sealed set and the retired pool 8ed5ab11…eea88 (63,075,223,742,053) present, the issuer's σ is the same as with it absent | unit, real operands |
| **CE-LV1-3** | The freeze captures the credential-side sum of the mark, unfiltered — a pool dropped by the VRF intersection still contributes | unit |
| **CE-LV1-4** | `to_pool_distr_view` contains no summing loop and no sum-based fallback | structural gate |
| **CE-LV1-5** | The seed and bootstrap constructors carry a real total; a missing one **fails closed** rather than summing | unit + gate |
| **CE-LV1-6** | Codec round-trips the new field; a v6-schema object is REFUSED with the typed `UnknownVersion`, never zero-filled | unit |
| **CE-LV1-7** | `canonical_hash` changes for an object whose total changes but whose pools do not — the field is authority, not decoration | unit |
| **CE-LV1-8** | `STORE_SEMANTICS_VERSION` = 7 and the lock gate passes, run in the same commit | gate |
| **CE-LV1-9** | **LIVE**: a fresh v7 bootstrap follows preprod through the epoch that rejected under v6 | live |
| **CE-LV1-10** | Negative-tested | mutations below |

### Required mutations

Restore the summing loop in `to_pool_distr_view` (must fail CE-LV1-1 **and** CE-LV1-2) · sum the
*filtered* set at freeze time instead of the mark (must fail CE-LV1-3) · default a missing total to
the entry sum (must fail CE-LV1-5) · exclude the field from `canonical_hash` (must fail CE-LV1-7) ·
accept a v6-schema object by zero-filling (must fail CE-LV1-6) · leave `STORE_SEMANTICS_VERSION` at 6
(must fail CE-LV1-8).

---

## 6. THE LIVE BAR IS WEAKER THAN THE IN-TREE ONE, AND THAT IS STATED UP FRONT

CE-LV1-9 cannot reproduce the original rejection: a fresh v7 bootstrap anchors near the current tip
(epoch ~310), far past slot 130,739,648, so it never validates that header. **A green live run is
therefore evidence of no regression, NOT evidence the defect is fixed.**

The proof that the defect is fixed is **CE-LV1-2**, in-tree, on the real sealed epoch-306 operands
already extracted by the census: the same distribution, the same retired pool, the issuer's σ moving
from `0.10642836%` to a membership-invariant value, and the header's threshold clearing its VRF
value. That test is the bar. The live run is the regression check.

Saying this before the run, because the reverse — running first and then deciding what the run proved
— is the habit this cluster has been paying for.

---

## 7. EXPLICITLY NOT IN THIS SLICE

- **No membership change.** DC-EPOCH-24's rule is correct per the oracle and stays byte-for-byte.
- **No removal of the retired pool.** It belongs in the set. Only the denominator's derivation moves.
- **No claim about the epoch-306 numerator.** The census showed the probe pool's epoch-306 stake
  ~0.8% above cardano's nearby values. That is unexplained, may be ordinary drift, and is **not**
  addressed here — a green header must not be read as settling it.
- **No CE-B12-10 claim.** B12's live bar is still owed and still blocked until a store can follow
  preprod.
- **No harness change.** Extending the continuous-operation proof back to the seed window is the
  separate, larger fix for the *class* — and it needs an epoch-306 reference that no longer exists on
  disk.
