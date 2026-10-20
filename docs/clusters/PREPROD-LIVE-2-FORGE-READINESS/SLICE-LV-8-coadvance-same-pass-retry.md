# SLICE LV-8 — a co-advance pass never leaves leadership stale only because the checkpoint was behind

**Status:** DESIGN APPROVED 2026-10-20 by the owner (option (b) of LV-7 §6); implementation next.
**Entry state:** `f7c7c7ad` (LV-7 census, pushed). Cause proven there: at 305→306 and 308→309 the
accumulator walk asks the reduced checkpoint for a collateral value before the checkpoint has applied
the block that spends it. The observe-only `ApplyFailedAt` arm stops the walk, and the crossing that
seals leadership for N+2 never runs.

---

## 1. INVARIANT — DC-EPOCH-46 (new; registered with the implementation)

In the owner's words: **a co-advance pass may not return with leadership authority stale solely
because the reduced checkpoint had not yet reached data needed by the accumulator. If checkpoint lag
is the only blocker, the checkpoint must advance and the accumulator must retry within that same
pass.**

Two clauses:

- **(a) Bounded same-pass repair.** Exactly one condition is retryable: the walk's apply failed with
  `CollateralBalance(UnresolvedCollateralInput)` at block B while the checkpoint's cursor is before B.
  The checkpoint then advances forward, and the walk is retried once from its unchanged cursor.
- **(b) No observe-only escape for the authoritative walk.** Every other apply failure, a retry that
  fails again at the same block, and a walk fault all halt the pass with a typed cause, at the block
  that caused it.

`LeadershipEpochNotSealed` stays exactly as it is, as defence in depth. It is no longer the normal way
this defect surfaces.

The owner deliberately rejected a stronger form: "the checkpoint has applied every block before the
accumulator applies it". That would force the two stores into lockstep.

## 2. WHY NOT REORDER (owner, 2026-10-20)

Reordering changes the sequencing of two authoritative stores on every pass. That includes the
boundary and snapshot path that already works (the DC-EPOCH-32 rewind). The retry changes behaviour
only when the measured temporary dependency occurs, and it keeps the existing boundary positioning
and rewind semantics, so they need no new proof.

## 3. DESIGN

Everything is in `crates/ade_node/src/node_lifecycle.rs`, inside `advance_ledger_state_to_durable_tip_memo`,
in the walk loop's `ApplyFailedAt` and `Err` arms.

```
walk
 ├─ ReachedTip / BoundaryRequiredAt ─────────────► unchanged
 ├─ ApplyFailedAt { B, UnresolvedCollateralInput } and checkpoint cursor < B, not yet retried at B
 │     └─ advance checkpoint forward to B ─► retry the walk once (loop) ─► success: continue
 │                                                                    └─ fails again at B: HALT
 ├─ ApplyFailedAt, any other case ──────────────► HALT typed
 └─ Err(walk fault) ────────────────────────────► HALT typed
```

- **One classification function**, pure: `classify_walk_apply_failure(&error, checkpoint_cursor, B)`
  returns either `CheckpointBehind` or `Terminal`. It is the only place a retry is decided.
  `CheckpointBehind` requires all three conditions together: the typed `UnresolvedCollateralInput`, a
  checkpoint present, and its cursor strictly before B.
- **The bound.** The retry is remembered per pass as the slot B it was spent on. The walk is retried at
  most once at any given B. A second failure at that B halts. A later checkpoint-behind failure at a
  different B′ > B gets its own single retry. Each retry strictly advances the cursor past an earlier
  failing block, so the number of retries in a pass is bounded by the blocks in it. Nothing loops at a
  fixed point. *(This is my reading of the owner's "retry ONCE" for a pass that meets the case twice;
  it is stated here so it can be checked.)*
- **Advance to B, not to the tip.** The retry advances the checkpoint with the existing forward-only
  `advance_reduced_checkpoint_forward_to(cp, chaindb, B)`.
  - B is the smallest target at which BND-2d's retention holds B's collateral.
  - It keeps the checkpoint at or before the next boundary point, so the boundary arm's
    `position_reduced_checkpoint_at_boundary` still advances forward, rather than rewinding and
    replaying from the seed.
  - The end-of-pass "checkpoint to tip" step is unchanged.
- **Typed halt.** A new variant, `NodeLifecycleError::AccumulatorWalkHalted { slot, cause, retried }`.
  It returns through `?` from both callers in `run_relay_loop_with_sched`: the startup recovery pass and
  the per-iteration co-advance. The relay loop's existing fail-closed exit takes it from there.
- **Removed:** the `ApplyFailedAt` arm's observe-only log, its `StalledBoundary` memo write, and the
  walk `Err` arm's observe-only log. That `Err` arm's comment already said "at the S4 authority flip
  this Err arm MUST halt".
- **Unchanged:** the boundary arm's stalls (no reduced checkpoint, unreachable boundary point, mark
  capture, bind, finalize, `Stalled`, cross fault). They are named in §6 as outside this slice.
- **Versions:** no persisted shape or meaning changes. A pass that used to stall now reaches the same
  state that a warm restart already reaches deterministically, as LV-7 §3.4 shows. Store semantics
  stay at 11.

## 4. MECHANICAL ACCEPTANCE CRITERIA

| CE | criterion | witness |
|---|---|---|
| **CE-LV8-1** | **Pair A, real bytes, through the co-advance lifecycle.** Fixture blocks: the creating block (preprod 130,178,856) and the spending block (130,350,133, phase-2 invalid, collateral `0326ab20…#1`), plus a boundary block. Start with the checkpoint at the seed, before the creating block. Then: before the call the checkpoint resolves the input neither live nor retained; after one `advance_ledger_state_to_durable_tip` the accumulator cursor has passed B, the boundary is crossed, leadership for N+2 is sealed (`leadership_authority_for_epoch`), and the retention answers 2,060,733 | `ade_node` unit |
| **CE-LV8-2** | **Pair B**, the same with 131,625,032 → 131,626,888 (collateral `90a5961d…#1`, 17,308,819) | `ade_node` unit |
| **CE-LV8-3** | **The retry is bounded.** The same as pair A, but with the creating block withheld from the chain, so no checkpoint advance can ever produce the value. The pass halts with `AccumulatorWalkHalted { slot: B, retried: true }`, the accumulator does not cross, and nothing is sealed | `ade_node` unit |
| **CE-LV8-4** | **Not checkpoint-behind halts at once.** The checkpoint is already at or past B and still cannot answer. The result is `AccumulatorWalkHalted { retried: false }` | `ade_node` unit |
| **CE-LV8-5** | **Classification is exact.** `CheckpointBehind` only for the typed `UnresolvedCollateralInput` with the cursor strictly before B. Cursor equal to B, cursor past B, no checkpoint, and every other `LedgerTransitionError` are all `Terminal` | `ade_node` unit |
| **CE-LV8-6** | **A walk fault halts** (a durable block that does not decode), typed, with no observe-only return | `ade_node` unit |
| **CE-LV8-7** | **Structural.** One retry decision (`classify_walk_apply_failure`), the walk-loop arms carry no observe-only log, and the halt variant is returned from both arms | `ci/ci_check_coadvance_retry.sh` (new) |
| **CE-LV8-8** | Every existing suite passes; no new CI-script failures | existing |
| **CE-LV8-9** | **Live, fresh follower-only bootstrap** of snapshot 6009 against the local docker `cardano-node-preprod`, **with no restart**. It crosses 305→306 and 306→307, then 308→309 and the first header of 310, and the run emits **no** `LeadershipEpochNotSealed` and no `AccumulatorWalkHalted` | live run |
| **CE-LV8-10** | Negative-tested | mutations below |

The permanent tests use committed fixtures. The LV-7 census tools stay as evidence utilities only, so
CI does not depend on a local ImmutableDB.

### Required mutations

Each mutation is asserted applied, the named witness must fail for the intended reason, and the tree
is restored byte-exact afterwards.

| mutation | must fail |
|---|---|
| No retry (`CheckpointBehind` handled as `Terminal`) | CE-LV8-1, CE-LV8-2 |
| A second retry allowed at the same B | CE-LV8-3 (the halt never comes, or `retried` is wrong); the test carries a bound, not a hang |
| Every apply failure classified as `CheckpointBehind` | CE-LV8-4, CE-LV8-5 |
| Cursor test `<` weakened to `<=` | CE-LV8-5 |
| `ApplyFailedAt` back to an observe-only `break` | CE-LV8-3, CE-LV8-4, gate |
| Walk `Err` arm back to an observe-only `break` | CE-LV8-6, gate |
| The retry advances the checkpoint to nothing (the advance call removed) | CE-LV8-1, CE-LV8-2 |

## 5. FIXTURES

Extracted from cardano's ImmutableDB on the local docker node (envelope bytes, as the existing
fixtures):

| block | slot | bytes | sha256 (first 16) | role |
|---|---|---|---|---|
| `block_130178856.cbor` | 130,178,856 | 8,759 | `1ecd78b410226c66` | creates `0326ab20…#1` |
| `block_130350133.cbor` | 130,350,133 | 15,252 | `f9cf6b9223bcd0e4` | already committed (BND-2) |
| `block_131625032.cbor` | 131,625,032 | 8,927 | `a0135e48fc80fe61` | creates `90a5961d…#1` |
| `block_131626888.cbor` | 131,626,888 | 50,679 | `266f137dc83d9a53` | spends it, phase-2 invalid |

## 6. EXPLICITLY NOT IN THIS SLICE

- The boundary arm's observe-only stalls (§3). Each one also leaves leadership stale. None of them
  occurred in LV-7's measurement. Converting them to halts is a separate decision.
- Catch-up step size (one sync step admits about an epoch). LV-7 found it contributing, not causal.
- The forge leg (CE-B12-10), and the 46 CI scripts that already fail.

## 7. RESULTS

Run 2026-10-20 on the implementation, before the live leg.

**Tests (CE-LV8-1..6).** `cargo test -p ade_node --lib -- lv8_`: 6 passed.

- **Pairs A and B (CE-LV8-1, CE-LV8-2).** Each test proves the first attempt on an identical, separate
  chain: the bare walk returns `ApplyFailedAt` at B, classified `CheckpointBehind`. It then runs one
  `advance_ledger_state_to_durable_tip` on the second chain:
  - before: the input is neither live nor retained, and N+2 is unavailable;
  - after: the retained value is 2,060,733 (pair A) or 17,308,819 (pair B), the accumulator is in 501
    at the boundary slot, and `promotion_leadership_authority_for_epoch(502)` returns the sealed object
    with target 502.
- **The fixture's leadership state.** The synthetic seed store holds no leadership band at all, so its
  "before" answer is `OldAccumulatorSchemaNotLeadershipCertified`, not `LeadershipEpochNotSealed`. The
  test asserts unavailability, not that variant.
- **Walk fault (CE-LV8-6).** It uses an era schedule that begins after the block. An undecodable block
  cannot reach the walk, because recovery admission refuses it first (`NonContiguousCanonicalSpan`).

**Suites (CE-LV8-8).** `cargo test -p ade_node`: every test binary ok (lib 493 passed, 11 ignored).
`ci/ci_check_coadvance_retry.sh` passes. The store-semantics lock passes, because `node_lifecycle.rs` is
not on its surface and no version moved. The registry gates pass with DC-EPOCH-46 registered as
partial.

**Mutations (CE-LV8-10).** Each was asserted applied and restored byte-exact. The test run is bounded
(`timeout 600`), and none timed out.

| mutation | caught by |
|---|---|
| M1 no retry | pair A, pair B, the bound test; gate (c) |
| M2 a second retry at the same B | the bound test (`retried` is false); gate (c) "not bounded per failing block" |
| M3 everything retryable | the classification test, the not-behind test; gate (a) |
| M4 `<` weakened to `<=` | the classification test; gate (a) "STRICTLY before" |
| M5 `ApplyFailedAt` back to an observe-only `break` | the bound test, the not-behind test; gate (c) |
| M6 walk `Err` arm back to an observe-only `break` | the walk-fault test; gate (d) |
| M7 the retry without the checkpoint advance | pair A, pair B, the bound test; gate (c) |

Under M2 the second retry is still stopped, by the classification itself: after the first retry the
checkpoint's cursor equals B, which is not strictly before it. The per-block marker and the strict
cursor test are two independent bounds, and the gate pins both.

**Live (CE-LV8-9).** Not yet run. DC-EPOCH-46 stays partial until it is.
