# SLICE LV-2 — "not computed yet" is not "computed, and empty": the seed boundary's reward has an owner

> **CLOSED 2026-10-18 — DC-EPOCH-41 `enforced`.** CE-LV2-10 was met in full by LV-4's v10 run
> (448/448). The owner marked CE-LV2-3 **superseded** by a live-equivalent proof: right after the
> live 304→305 crossing, every one of the 33,257 reward balances equals cardano's. The single
> difference is a withdrawal made in the 15k-slot gap between the two points, found on chain. Each
> leftover of the isolated crossing is equal on the real transition, and the treasury and reserves
> are exact. The isolated result, 11,052 of 11,056, stays on record below. Evidence:
> `lv5-treasury-divergence-census.md` §6.

**Status:** IMPLEMENTED 2026-10-11; results in §9. Every in-tree criterion is green. The live bar CE-LV2-10(b) is **447 of 448 pools** — one credential short, the residual owned by LV-4 — so the slice is NOT closed and DC-EPOCH-41 is registered `partial`.

**Entry state:** `029e082c`. Evidence:
`docs/evidence/run-stores/preprod-live2c/lv1-ce9-v7-live-NOT-FIXED-seed-boundary-reward-update-absent.md`.
That file proves the defect and holds the reference captures this slice is judged against. This
slice re-opens nothing in LV-1: DC-EPOCH-40 stands and is untouched.

---

## 1. THE CONTRACT — answered before any design

> **Who owns the reward paid at the first boundary after the seed?**

**cardano:** the reward update for boundary `E → E+1` is computed during epoch `E` from four facts
frozen at the start of `E` — the go snapshot, `nesBprev`, `ssFee`, and the reserves — and applied at
the boundary. `nesRu` records how far that computation has got:

| `nesRu` at the seed | meaning | slot-in-epoch on preprod/mainnet |
|---|---|---|
| `SNothing` | **not started** | 0 – 172,800 (`4k/f`) |
| `Pulsing` | in progress | 172,800 – at most 345,600 |
| `Complete` | finished; `rs` and the pot deltas are final | after completion |

In all three the same reward is paid at the boundary. The state says only how much of the work the
snapshot happens to contain.

**Ade:** `Complete` → import cardano's result and apply it once (DC-EPOCH-18, proven). `Pulsing` →
refuse the bootstrap. `SNothing` → **an empty result, applied once** — and because a result is
"present", the native calculation is switched off for that boundary. Nobody pays.

> **Therefore: exactly one of {imported result, native calculation} owns the seed boundary's reward,
> and the choice is made by a typed fact about the snapshot, never by an empty map.**

### 1.1 Consequence, measured

Seed at slot-in-epoch 127,027 of epoch 304. At 304→305 Ade logged `pool_pot=0`. Cardano paid
416,110,584,109 to reward accounts, added 7,029,700,573,599 to the treasury and drew
7,422,111,617,574 from reserves. Ade's end-of-304 mark is 408,132,859,573 low; the leadership σ for
the probe pool is 0.053% low in epoch 306 and 0.058% low in 307; a real preprod header is rejected.
The error is permanent for the life of the store.

---

## 2. INVARIANT

**DC-EPOCH-41 (new; registered with the implementation, not with this doc).** The reward update
applied at the seed→seed+1 boundary has exactly one authority, selected by the seed snapshot's
`nesRu` state and carried as a closed sum:

- `Complete` → the imported, commitment-bound cardano result (unchanged from DC-EPOCH-18);
- `NotYetComputed` → the **native** reward calculation, run at that boundary from imported `nesBprev`
  and imported `ssFee` with the held go snapshot — the same function every later boundary uses.

An absent reward update is **never** represented as a zero-valued present one. A seed boundary that
pays a zero pool pot while `nesBprev` is non-empty is a terminal, not a result.

**DC-EPOCH-18 — strengthened, not modified.** Its `Complete` path is byte-unchanged. Its precondition
("the snapshot carries a Complete `nesRu`") becomes a checked variant instead of an assumption.

---

## 3. DESIGN

### 3.1 The type

```rust
pub enum SeedRewardAuthority {
    /// cardano finished the calculation before the snapshot. Apply its result exactly once.
    Imported(BootstrapRewardUpdate),
    /// cardano had not started. Ade computes it at the seed boundary.
    /// `fee_ss` is the snapshot's `ssFee` — epoch (seed-1)'s fees, the one reward input that
    /// cannot be rebuilt from post-seed blocks.
    Native { fee_ss: Coin, /* + the same manifest/point/epoch binding and commitment */ },
}
```

`EpochAccumulator.pending_reward_update: Option<BootstrapRewardUpdate>` becomes
`Option<SeedRewardAuthority>` (`None` only after consumption, as today). No boolean, no
"empty means native" convention, no `Default`.

### 3.2 Decode

`read_reward_update_deltas` stops returning `(empty, 0, 0, 0)` for `SNothing`. It returns the
variant. `ssFee` is decoded instead of `skip_item`'d and travels with `NotYetComputed`.

### 3.3 The seed boundary

In `cross_epoch_boundary_transition`:

- `Imported` → today's path, unchanged.
- `Native { fee_ss }` → the ordinary native path with `block_production = prev_block_production`
  (already imported) and `epoch_fees = fee_ss`. Then the fee pot is reduced by `fee_ss` before it is
  rotated to `prev_epoch_fees` — the same reduction the `Imported` path performs with `delta_fees`,
  and for the same reason: the imported fee pot still contains epoch (seed-1)'s fees.

One reward function, no seed-only arithmetic.

### 3.4 What does NOT change

The reward formula, `apply_epoch_boundary_with_registrations`, the mark construction order
(post-RUPD), POOLREAP, the `Imported` path, `Pulsing` refusal, LV-1's denominator, membership.

---

## 4. SLICE-ENTRY OBLIGATIONS — unknowns resolved BEFORE code, each by measurement

| # | Question | How it is answered |
|---|---|---|
| **E1** | Does the native calculation, fed the seed state's go + `nesBprev` + `ssFee` + reserves, reproduce cardano's `rs` exactly? | Evidence harness on snapshot 6009 vs `rupd-304to305-rs-by-credential.json` (11,056 credentials). If it does not, the gap is named and this slice STOPS — it becomes a reward-formula question, not a wiring one. |
| **E2** | Cardano rewards from the pool parameters frozen in the go snapshot; Ade's native path reads the current `cert.pool.pools`. Do they differ for any of the 79 producing pools here? | Same harness; a difference shows up as specific pools. |
| **E3** | The seed+2 window-replay derive (`epoch_candidate.rs` → `reduced_window_driver.rs`) applies the same record's `reward_delta` at the window end. Is that path still reachable in production after the S4 authority flip? | Read the call graph; record the answer. Reachable or not, it must not apply an empty delta for `Native` (CE-LV2-6). |
| **E4** | Is `utxosFees(seed) − ssFee` exactly the seed epoch's own fees so far? | From `ledger-state-seed304.json` and the block fees up to the seed; and at the far end, Ade's `prev_epoch_fees` after 304→305 vs cardano's `feeSS` in epoch 305 (one more offline query on the kept reference node). |

### 4.1 E1 RESULT, 2026-10-10 — wiring confirmed; one pre-existing reward defect blocks the bar

`docs/evidence/run-stores/preprod-live2c/lv2-e1-native-seed-reward-vs-cardano.md`.

Fed `nesBprev` + `ssFee`, the existing reward function matches cardano for **11,050 of 11,056**
credentials; `pool_pot` goes from 0 to 28,118,802,294,396. Every residual is attributed. Two are
defects in the shared reward function, outside this slice as drafted:

- **Pledge check.** Ade counts an owner's stake wherever it is delegated; cardano counts only owners
  delegated to the pool itself. Ade pays pool `13e8600b…2418a` 6,218,191,443 that cardano does not
  pay. This contradicts a recorded project rule and needs an owner ruling.
- **Credential discriminant.** A reward for a script credential lands on the key credential with the
  same 28 bytes (11,802 lovelace).

E1 as written is NOT met, so per the rule above **implementation has not started**. CE-LV2-3 and
CE-LV2-10(b) cannot pass until the pledge check is settled. E2: no residual attributable. E3, E4: not
run.

E1 is the load-bearing one. LV-1 was implemented on a theory nobody had checked against a reference;
E1 exists so that this slice is not.

### 4.2 E1 RE-RUN, E3 and E4 — 2026-10-11

**E1, after LV-3 (`cb65ec6e`).** The pledge defect is fixed. The native seed reward now matches
cardano for **11,051 of 11,056** credentials and the reserves draw is **exact**
(7,422,111,617,574). The five that still differ are not reward-formula errors:

| credential | what | status |
|---|---|---|
| `94857be3…4dfd5b` (6,296,662), `85ecddcb…99163` (11,847) | unregistered at the seed point; Ade's emulation runs on the SEED state, where the later re-registration has not happened yet | harness artifact — disappears in a real run that replays the rest of the epoch. The +6,308,509 treasury difference is the same two amounts |
| `900fbcf7…7fedb` (11,802) | a script credential's reward credited to the key credential with the same 28 bytes | real, pre-existing; slice **LV-4** |
| `7a507e54…98756` (+500,000,000), `4245236a…a6454` (+1,000,000,000) | credits applied at the same boundary that are not part of `rs`: the first is a pool deposit refund; the second's source is not identified (neither pot moves for it) | not a reward difference; the second stays open as recorded in the E1 evidence |

So CE-LV2-3's "all 11,056" has exactly one real exception left, owned by LV-4.

**E3 — the window-replay path.** `derive_candidate` (`epoch_candidate.rs`) is still compiled into
production: `node_lifecycle::maybe_activate_epoch_boundary` → `maybe_activate_first_boundary` →
`activate_at_boundary` → `derive_candidate`, which applies the imported record's `reward_delta` at the
end of the seed window. It is not what supplies leadership any more: `seed+1` comes from the bootstrap
bridge and every epoch from `seed+2` on comes from the accumulator's frozen leadership object
(S4-L2), and `maybe_activate_first_boundary` returns early once a view has been promoted, which the
bridge does before the tip can enter `seed+1`. The last real bootstrap log shows only the bridge and
the frozen path.

It stays reachable in code, so it gets a rule rather than an assumption: for a `Native` seed there is
no imported delta, and a window replay cannot reproduce a reward the accumulator computes. **For
`Native`, `derive_candidate` on the seed window is a typed terminal** — never an empty delta, never
`None` (CE-LV2-6).

**E4 — the fee pot.** Exact, from three independent sources.

```
ssFee in the seed snapshot (epoch 304)                         23,699,540,134
public explorer, total fees of epoch 303 (19,463 blocks)       23,699,540,134   = ssFee
public explorer, total fees of epoch 304 (19,064 blocks)       22,236,845,053
Ade v7 store, fee pot rotated out of 304 (never reduced)       45,936,385,187
  minus fee_ss                                                 22,236,845,053   = epoch 304's fees
Ade v7 store, blocks counted for 304                                   19,064   = explorer
```

So `ssFee` is the previous epoch's fees, the imported fee pot still contains it, and Ade's own
per-block fee accumulation over the rest of the seed epoch is exact. Reducing the pot by `fee_ss`
once at the seed boundary — what `Imported` already does with `delta_fees` — leaves exactly the seed
epoch's fees. The reference `ssFee` was constant across both captures inside epoch 304. Source for
the explorer figures: Koios preprod `epoch_info`, epochs 303 and 304, fetched 2026-10-11. The v7
figures are read by `seed_boundary_reward_census::current_accumulator_fee_pots`.

CE-LV2-4's expected value is therefore **22,236,845,053**.

### 4.3 Design amendments made by the obligations

- **§3.1 shape.** The `Native` variant carries a small closed record of its own — the same manifest /
  point / epoch binding as the imported one, `fee_ss`, and its own commitment — so both variants are
  provenance-bound and commitment-checked the same way.
- **§3.3.** The zero-pot terminal (CE-LV2-5) is checked on the `Native` path only. `Imported`
  deliberately runs the native calculation at zero.
- **§5 store version.** LV-3 took 7 → 8 (the lock does not allow one bump across two commits), so
  this slice takes **8 → 9**. Read every "7 → 8" / "v8" below as "8 → 9" / "v9". No store exists at
  v8, so it is still one refusal and one re-bootstrap.
- **Snapshot commitment.** `commit_native_nonutxo_state` already binds the two pot deltas as a legacy
  double-binding; for `NotYetComputed` it keeps writing zeros there, byte-identical to today. The
  variant and `fee_ss` are bound by the reward record's own commitment, as `rs` and `deltaF` are.

---

## 5. STORE SEMANTICS

The accumulator's canonical encoding carries the pending record, and the record is commitment-bound.

- Bootstrap reward record schema **3 → 4**; commitment domain bumped with it. A v3 record is
  REFUSED, not reinterpreted: a v3 record with empty deltas cannot be told apart from a genuine
  empty result, and guessing is the defect.
- `STORE_SEMANTICS_VERSION` **7 → 8**, with `ci/ci_check_store_semantics_lock.sh` run in the same
  commit. A v7 store seeded from an `SNothing` snapshot is wrong in every epoch and cannot be
  repaired in place.

⚠ This retires `~/.cardano-live1/ade-preprod-v7`. Unlike LV-1, the replacement is the real test:
snapshot 6009 is on disk, re-bootstraps in ~48 min, and reaches the failing header.

---

## 6. MECHANICAL ACCEPTANCE CRITERIA

| CE | Criterion | judged by |
|---|---|---|
| **CE-LV2-1** | `SNothing` decodes to `NotYetComputed` carrying `ssFee`; it can never produce a present, empty update. `Complete` decodes as before; `Pulsing` still fails closed | unit |
| **CE-LV2-2** | A `Native` seed boundary runs the native reward on `nesBprev` + `fee_ss`: pool pot > 0, rewards credited before the mark, pots moved, authority consumed exactly once | unit |
| **CE-LV2-3** | **THE REAL CASE.** On snapshot 6009's seed state Ade's reward update equals cardano's: per-credential `rs` for all 11,056 credentials (sum 416,110,584,109), `deltaT` 7,029,700,573,599, `deltaR` 7,422,111,617,574, `deltaF` 23,699,540,134 | evidence harness, real operands (E1) |
| **CE-LV2-4** | After the seed boundary the fee pot rotated to `prev_epoch_fees` equals cardano's `feeSS` for the next epoch | unit + E4 capture |
| **CE-LV2-5** | A seed boundary with non-empty `nesBprev` and a zero pool pot is a terminal | unit |
| **CE-LV2-6** | No path — accumulator boundary or seed+2 window replay — applies an empty reward delta for a `Native` seed | unit + structural gate |
| **CE-LV2-7** | Record schema 4 round-trips both variants; a v3 record is refused with the typed `UnknownVersion`; the variant is inside the commitment | unit |
| **CE-LV2-8** | `STORE_SEMANTICS_VERSION` = 8 and the lock gate passes, in the same commit | gate |
| **CE-LV2-9** | The `Imported` path is byte-unchanged: every existing DC-EPOCH-18 / CE-3d test passes unedited | existing suites |
| **CE-LV2-10** | **LIVE.** A fresh v8 bootstrap from snapshot 6009: (a) `pool_pot` ≠ 0 at 304→305; (b) the sealed epoch-306 distribution equals `stake-snapshot-epoch305-all.json` for all 448 pools, total 1,626,474,372,102,448; (c) the header after durable tip 130,733,645 is ACCEPTED; (d) the node follows on | live |
| **CE-LV2-11** | Negative-tested | mutations below |

**CE-LV2-10(b) is the bar, not (c).** One header passing proves σ moved far enough for that header.
448 pools byte-equal proves the snapshot is right.

### Required mutations

Map `SNothing` back to an empty `Imported` (must fail CE-LV2-1 and CE-LV2-2) · feed the native seed
path `fee_ss = 0` (must fail CE-LV2-3 and CE-LV2-4) · skip the fee-pot reduction (must fail CE-LV2-4)
· remove the zero-pot terminal (must fail CE-LV2-5) · let the window-replay path apply an empty delta
(must fail CE-LV2-6) · exclude the variant from the commitment (must fail CE-LV2-7) · leave
`STORE_SEMANTICS_VERSION` at 7 (must fail CE-LV2-8).

---

## 7. EXPLICITLY NOT IN THIS SLICE

- **`Pulsing` seeds stay refused.** The native path would serve them too (the inputs are frozen when
  pulsing starts), but there is no reference capture for one and no need for one here. Refusal is
  safe; widening it is a separate decision.
- **No repair of existing stores.** v7 is refused, not migrated.
- **No change to LV-1.** DC-EPOCH-40 keeps its statement and its `partial` status; whether CE-LV1-9
  is re-read as met after CE-LV2-10 is decided then, on what that run shows.
- **No CE-B12-10 claim.** B12's live bar is the next leg after this one, on the same store, with the
  rotated KES key.
- **No claim about other venues.** Whether earlier preview proofs used early-epoch seeds was not
  checked; any that did carry this defect.

---

## 8. OPEN DECISIONS FOR THE OWNER

1. **Scope of `Pulsing`** — recommended: out, as §7. Bringing it in removes a refusal window of up
   to 40% of an epoch for operators, at the cost of one more reference capture and CE.
2. **v7 refusal vs migration** — recommended: refusal, as §5.
3. **Slice identity** — drafted as LV-2 inside PREPROD-LIVE-2-FORGE-READINESS because it is the
   blocker for that cluster's live CEs. It is a DC-EPOCH-18 defect by lineage and could instead be a
   slice of the reward-evolution cluster.

---

## 9. RESULT — 2026-10-11

Evidence: `docs/evidence/run-stores/preprod-live2c/lv2-ce10-v9-live-seed-boundary-reward-native.md`.

| CE | Result |
|---|---|
| CE-LV2-1 | GREEN — `snothing_decodes_to_not_yet_computed_carrying_ss_fee`, `an_empty_complete_update_is_not_not_yet_computed`, `a_pulsing_update_still_fails_closed` |
| CE-LV2-2 | GREEN — `native_seed_boundary_runs_the_native_reward_on_nesbprev_and_fee_ss` (equal to an ordinary boundary fed the same inputs; mark built post-reward; consumed once), `native_seed_boundary_reward_depends_on_fee_ss` |
| CE-LV2-3 | **NOT MET AS WRITTEN — 11,051 of 11,056.** The sealed `Native` seed, crossed by the production transition: reserves draw exact (7,422,111,617,574), fee pot rotated correctly. Two of the five are artifacts of crossing the seed state (the live run's pools come out exact), two are not reward-update credits, **one is LV-4** |
| CE-LV2-4 | GREEN — `native_seed_boundary_rotates_exactly_the_seed_epochs_own_fees` with the real operands (45,936,385,187 − 23,699,540,134 = 22,236,845,053, the public fee total of epoch 304); underflow fails closed |
| CE-LV2-5 | GREEN — `native_seed_boundary_with_blocks_and_a_zero_pot_is_a_terminal` (and no terminal when no blocks were made) |
| CE-LV2-6 | GREEN — `seed2_window_with_a_native_seed_authority_is_a_typed_terminal`; gate: `apply_bootstrap_reward_deltas` has only its two known callers, each fed from an `Imported` record |
| CE-LV2-7 | GREEN — `both_variants_round_trip_as_themselves`, `a_v3_record_is_refused_not_reinterpreted`, `the_variant_is_inside_the_commitment`, `accumulator_codec_round_trips_a_native_seed_authority` |
| CE-LV2-8 | GREEN — store semantics **9** (not 8: §4.3), lock reconciled in the same commit |
| CE-LV2-9 | GREEN with one named exception to "unedited": the `Imported` arithmetic is unchanged and every DC-EPOCH-18 / CE-3d test passes with its assertions untouched. Seven fixtures were mechanically wrapped in `SeedRewardAuthority::Imported(…)` because the field's type changed, and the two tests that pin the record's version number now expect 4 |
| CE-LV2-10 | (a) GREEN `pool_pot=28118802294396` · **(b) 447 of 448, total −11,802** — the one pool is the LV-4 credential's · (c) GREEN, the header after 130,733,645 is accepted · (d) GREEN, following on in epoch 306 |
| CE-LV2-11 | GREEN — below |

**The slice is not closed.** (b) is the bar and it is one pool short. The cause is identified, is
outside this slice, and is the same 11,802 lovelace in CE-LV2-3.

### Mutations (each asserted applied; tree restored byte-exact)

| Mutation | Caught by |
|---|---|
| decoder: `SNothing` → an empty `Complete` | CE-LV2-1 test; gate |
| accumulator: a `Native` seed handled as an applied import | three CE-LV2-2/5 tests; gate |
| first-run: `NotYetComputed` sealed as an empty `Imported` record | gate only — there is no unit test around first-run; the live receipt line is the runtime witness |
| native seed path fed `fee_ss = 0` | two CE-LV2-2 tests; gate |
| fee-pot reduction skipped | CE-LV2-4 tests; gate |
| zero-pot terminal removed | CE-LV2-5 test |
| window replay proceeds with no delta for `Native` | CE-LV2-6 test; gate |
| variant excluded from the commitment | CE-LV2-7 test; gate |
| `STORE_SEMANTICS_VERSION` left at 8 | gate; lock gate |

The doc's first required mutation named CE-LV2-2 as a witness for the decoder change. It is not: the
accumulator tests start from a sealed authority. The decoder, the accumulator and first-run were
mutated separately instead.

### Other gates

`ci_check_bootstrap_rupd_fee_reduction.sh` and `ci_check_epoch_accumulator_no_utxo.sh` pinned the old
field names and the literal schema version 3. Both were updated to the new shapes; neither assertion
was loosened (the version check became "≥ 3", the same form the accumulator's version check already
had).

A sweep of all 276 `ci/ci_check_*.sh` scripts: 229 pass, 47 fail — and the same 47 fail on the commit
before this slice. None is caused by it. They are not addressed here.

### The boundary after the seed (305→306) — recorded 2026-10-11, after the slice commit

Evidence §6. The fee pot this slice rotates is confirmed there: the second boundary's pool pot equals
an independent recomputation exactly, and the fees rotated out of epoch 305 equal the reference's
`feeSS`. CE-LV2-4 holds live.

The same boundary shows a defect this slice does not own. **E2, re-opened:** E2 asked whether reading
current pool registrations instead of the go snapshot's frozen pool parameters makes a difference, and
recorded "no residual attributable" — true at the first boundary, where it was checked. At the second
it does: a pool that retired at 304→305 is owed its epoch-304 rewards at 305→306, cardano pays them
from the snapshot, Ade finds no registration and pays nothing (42,082,928,795 less drawn from
reserves; one re-delegated member already shows as −16,453,955 in the sealed epoch-307 stake).

Also seen: the node halts at an epoch boundary if the accumulator has not sealed that epoch's
leadership yet, and a warm restart carries on. That is fail-closed behaviour on a catch-up ordering,
not a wrong result, and it is not addressed here.

### Not done

- `EPOCH_ACCUMULATOR_SCHEMA_VERSION` is unchanged. The accumulator embeds the reward record as its
  own versioned bytes, and that record's version gate (3 → 4) refuses an old blob.
- The archived v7 store can no longer be read by the evidence harness (its record is schema 3). The
  fresh v9 store has the same seed and replaces it.
