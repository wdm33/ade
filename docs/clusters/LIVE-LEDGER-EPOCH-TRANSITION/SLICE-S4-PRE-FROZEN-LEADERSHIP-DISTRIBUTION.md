# S4-pre — Frozen Leadership Distribution Authority

> **Status: OPEN. Prerequisite for S4.** Persist + replay the EXACT cardano leadership `PoolDistr` authority
> that Praos leader checks / forging consume, without relying on the seed-window EVIEW after bootstrap. A
> DERIVED cardano-compatibility invariant supporting the replay law: observable behaviour must match the
> reference node; the internal state shape may differ (Haskell node = compatibility oracle, not architecture
> template). S4 (the narrow flip) stays BLOCKED until S4-pre is green.

**Why (proven, `67890681`):** the Leadership Distribution Authority Trace proved leadership uses **SET-derived
stake + snapshot-FROZEN pool params/VRF**, including zero-stake registered pools AND the retired/POOLREAP'd 1M
ADA pool whose VRF is absent from active params. "Derive leadership later from go + active params" is a
DISPROVEN hypothesis (`from_accumulator_go_active_params_for_test_only`), not an optimization target.

## 1. The self-contained authority object (do NOT derive at use time)

```
FrozenLeadershipPoolDistr {
    epoch,
    source_boundary,
    pools: PoolKeyHash -> LeadershipPoolEntry { active_stake, vrf_keyhash },
}
```

`from_frozen_leadership(acc, asc) -> PoolDistrView` reads THIS object and nothing else. Leadership
reconstruction must NOT depend on `cert_state.pool.pools` (active), the go snapshot, `future_pools`, the
`retiring` map, or any retired-pool supplement at use time. Those are freeze-time SOURCE inputs, never runtime
leadership authority.

## 2. Bootstrap import

At bootstrap, import the frozen leadership distribution from a NAMED, manifest-bound artifact: `epoch`,
source point/boundary, pool count, `pool_id -> active_stake -> vrf_keyhash`, canonical hash, reference
provenance. **Missing / malformed leadership data = terminal typed failure** — no empty default, no inferred
fallback. Old v5/schema-v4 stores WITHOUT this field are NOT leadership-certified: re-bootstrap or explicit
re-materialization is required before S4 can use them.

## 3. Boundary freezing

At each cardano leadership freeze point, freeze the NEXT leadership distribution from the snapshot-frozen
stake + snapshot-frozen pool params/VRF. **No active-param lookup at leadership-use time** — the lesson from
`67890681` is that active params can forget a pool that stays leadership-relevant.

## 4. Recovery extension (S5 must cover this new surface)

S5 recovery equivalence must extend to this authority: clean advance, warm restart, within-k rollback, reset
+ refold — all reproduce the same frozen leadership distribution hash + `PoolDistrView`. The replay law
governs consensus outcomes, state roots, WAL, checkpoints, receipts, routing — byte-identical outputs from
the same canonical inputs.

## 5. Acceptance (S4-pre is green only when ALL hold)

- bootstrap frozen leadership distr == the seed leadership `PoolDistr`: **659/659 pools**, stake exact, VRF
  exact, zero-stake pools included exactly where cardano includes them, the retired 1M ADA pool included with
  its frozen VRF, canonical bytes exact;
- warm restart exact; within-k rollback/refold exact;
- missing frozen leadership fails closed; malformed frozen leadership fails closed;
- an old store cannot silently become leadership authority;
- CI/static guard: `from_accumulator_go_active_params_for_test_only` stays test/oracle/negative-regression
  only (never a production leadership path).

## 6. Slice shape

- **S4-pre-1** — types, canonical encoding, store schema, bootstrap import, seed identity test.
- **S4-pre-2** — boundary freeze from snapshot-frozen stake + params/VRF.
- **S4-pre-3** — re-bootstrap the v5 lineage + recovery replay evidence + registry flip.

Do NOT touch the three production seed-window call sites yet. Do NOT delete the seed+2 ceiling yet. That is
S4 proper, which resumes only after S4-pre proves `from_frozen_leadership(accumulator) == seed leadership
PoolDistr`.
