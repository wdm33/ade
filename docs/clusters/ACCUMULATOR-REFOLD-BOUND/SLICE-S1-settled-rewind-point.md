# SLICE S1 — bound the accumulator refold to a settled rewind point

> A rollback must rewind the epoch accumulator to a **settled** point (older than `k`), not to the
> bootstrap anchor. Bounds post-rollback re-derivation from *unbounded* to ~`k` slots without
> weakening the S5 crash-safety argument. `ade_runtime` + `ade_node`; **no BLUE edit**.

## Problem (measured on the 2026-08-01 sustained preview run)

`accumulator_admit_and_clear_for_rollback` calls `EpochAccumulatorStore::reset_to_bootstrap()` on
every admitted rollback. That restores the current blob from the **bootstrap** blob and rewinds
`LAST_SLOT` to the seed, so the next `advance_ledger_state_to_durable_tip` re-derives every boundary
and every within-epoch block since the bootstrap anchor.

Measured refold cost against distance from the anchor (`~/.cardano-live1/ade-1376-s1s2.log`):

| distance from anchor | refold |
|---|---|
| 25,838 slots | 225 s |
| 53,954 | 592 s |
| 81,272 | 941 s |
| 85,690 | **1595 s (26.6 min)** |

Per-slot cost *rises* with distance (0.009 → 0.019 s/slot), so the total is super-linear. 14 reorgs
in 18 h — preview produces routine 1-block reorgs — at ~77 min apart, i.e. already a ~35% duty
cycle, and **growing with uptime without bound**.

### Why this is a forge blocker, not just waste

The accumulator is the frozen-leadership authority (S4-L2: the sole promotion source for candidate
epochs ≥ seed+2). Refold time grows with uptime; the inter-reorg interval does not. Once refold
exceeds that interval the accumulator never becomes current, so leadership promotion cannot happen
at a boundary and **the node cannot forge**. It arms itself the longer a node stays up, which is
exactly the multi-day operation the BA-08 sustained certification and any real forge window need.

## The safety rule being preserved

`reset_to_bootstrap` is trustworthy for two reasons, and any replacement must reproduce both:

1. **Lineage.** It clears `LAST_ADVANCED_POINT`, so the store is *uncertified* until a canonical
   re-fold rewrites it — "it never trusts a reset store as lineage authority."
2. **Leadership coherence.** It restores `CURRENT_LEADERSHIP := BOOTSTRAP_LEADERSHIP`, because stale
   post-boundary objects "would outrun the refolded accumulator, violating replay equivalence."

## Design — rewind to a settled rolling save point

### Why NOT "rewind to the last epoch boundary"

The first design considered was a per-boundary save point. It is safe but leaves the worst case at a
full epoch (~86,400 slots ≈ 26 min), because a reorg just before a boundary refolds the whole epoch.
Its stated justification — that mid-epoch leadership truncation is ambiguous — **is false**:
`advance_with_current_leadership` (the only recurrent writer of `CURRENT_LEADERSHIP_BY_EPOCH`) is
called from exactly one site, inside `cross_accumulator_over_boundary_block`, riding the same commit
as the accumulator advance. **Leadership changes ONLY at boundary crossings**, so at any mid-epoch
point the leadership set is exactly what the last boundary left. There is no ambiguity to avoid.

The real rule is therefore *settledness*, not *boundary-ness*.

### The rewind point

Persist ONE rolling `SettledRewindPoint` alongside the existing blobs:

```
settled_blob        the accumulator state at that point
settled_slot        its slot
settled_point       its canonical lineage (slot + header hash)
settled_leadership  the leadership epochs valid there (== as of the last boundary at-or-before it)
```

**Refresh** (forward only): when the accumulator's `last_advanced_point` is at least `k` slots behind
the durable tip, overwrite the save point with it. One blob, overwritten in place — no retention set,
no pruning, none of the N-snapshot surface.

**Rewind**: on an admitted rollback, reset to the save point instead of bootstrap **iff all** hold —
otherwise fall back to `reset_to_bootstrap()` unchanged:

1. `settled_slot + k <= tip.slot` — the point is settled; no admissible reorg can reach it.
2. `settled_slot <= rollback_target.slot` — never rewind *forward* past the target.
3. `settled_point` still resolves to the same canonical hash — lineage intact (the DC-EPOCH-22
   `bind_boundary_mark` pattern).

(1) makes (2) automatic: a rollback target is within `k` of the tip and the save point is at least
`k` behind it, so the save point is always at or before the target. (2) is asserted anyway rather
than inferred.

### Invariants

- **INV-AR-1 (settled target).** The accumulator is never rewound to a point within `k` of the
  durable tip. Every rewind target is beyond the reach of an admissible reorg.
- **INV-AR-2 (lineage-bound).** A rewind target whose canonical hash no longer matches is refused
  and falls back to the bootstrap anchor. A rewind never trusts a point the chain has abandoned.
- **INV-AR-3 (leadership coherence).** A rewind truncates `CURRENT_LEADERSHIP_BY_EPOCH` to exactly
  the epochs valid at the rewind point, so no sealed leadership object can outrun the refolded
  accumulator (the existing `reset_to_bootstrap` guarantee, generalised).
- **INV-AR-4 (uncertified after rewind).** As today, a rewind clears `LAST_ADVANCED_POINT`: the store
  is uncertified until a canonical re-fold rewrites it.
- **INV-AR-5 (bounded refold).** Post-rollback re-derivation is bounded by `k` + the refresh margin,
  independent of node uptime.
- **INV-AR-6 (replay equivalence).** The refolded state is byte-identical to what the pre-slice
  bootstrap refold would have produced. Only the *starting point* of a deterministic re-derivation
  moves; the derived result cannot.

## Mechanical acceptance criteria

- **CE-AR-1.** A rollback with a settled save point rewinds to it, not to bootstrap, and the
  resulting refold covers ≤ `k` + margin slots.
- **CE-AR-2.** A save point within `k` of the tip is REFUSED (falls back to bootstrap) — INV-AR-1.
- **CE-AR-3.** A save point whose lineage no longer matches canonical is REFUSED — INV-AR-2.
- **CE-AR-4.** After a rewind, no leadership epoch beyond the rewind point survives — INV-AR-3.
- **CE-AR-5 (replay equivalence).** Refolding from the save point yields a state fingerprint
  byte-identical to refolding from bootstrap over the same canonical chain — INV-AR-6.
- **CE-AR-6 (live).** A sustained preview run shows post-rollback refold bounded and NOT growing
  with uptime.

## NOT claimed here

Reducing the *steady-state* fold cost, or the reorg→CPU amplification below ~`k` slots (that would
need sub-`k` snapshots, which are reorg-able by construction and were rejected on safety grounds).
The schema addition is versioned; an older store without the save point degrades to today's
bootstrap rewind, which is why this cannot regress an existing deployment.
