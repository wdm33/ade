# Invariant Slice S13 — Rolled-back branch evidence retention for the LCA walk

## 2. Slice Header

- **Cluster:** PHASE4-N-AO (live multi-candidate fork-choice SELECT + adopt).
- **Type:** fix slice (the S11 diagnostic confirmed an Ade-side fault, reproducible
  every run; S12 + a hermetic test are the deterministic vehicles).
- **Depends on:** S7 (LCA walk, DC-NODE-38), S11 floor (DC-NODE-39).
- **Declares:** `DC-NODE-40` (rolled-back branch evidence retention).
- **Cluster Exit Criteria addressed:** CE-AO-6 — resolves the `MissingBridge` over-fire
  (Fault 1) so the forge fence is not held by normal post-switch loser traffic, and
  reserves `MissingBridge` for a genuine winning-branch bridge gap (Fault 2).

## 4. Intent

**Fault 1 (confirmed Ade-side, every run):** when Ade switches cn1→cn2, it rolls back
its own followed cn1 branch and **discards those blocks** — the S7 branch cache caches
only *competing* blocks, never `LinearExtend`-admitted ones. So when cn1 keeps producing
(bno 15+), the LCA walk needs the rolled-back bno 14 to bridge, can't find it (not
cached, not durable) → `BranchGap` → `MissingBridge` + fence-hold, on **every** loser
block (cn1's block_no was fully consecutive — Ade *had* the whole branch and threw it
away on rollback).

The missing data is not arbitrary: it is **exactly the blocks Ade itself rolled back**.
S13 retains those blocks as **walk-visible evidence** (bounded, hash-keyed, self-binding)
so the walk can traverse the non-durable rolled-back intermediates until it reaches a
real durable ancestor — making the rolled-back branch *evaluable* again (fork-choice
resolves it: it loses) instead of falsely un-bridgeable.

## 5. Scope

A bounded, hash-keyed **rollback-retention cache** (`BTreeMap<Hash32, CachedHeader>`):
1. **Populated** in `apply_fork_switch` (`crates/ade_node/src/node_lifecycle.rs`): BEFORE
   the `ChainEvent::RolledBack { to_point: fork_anchor }` apply, walk the durable chain
   `old_tip → fork_anchor+1` and insert each rolled-back block as a self-bound
   `CachedHeader` (key == re-derived block hash). k-bounded.
2. **Consulted** by `walk_to_durable_lca` (`crates/ade_node/src/lca_walk.rs`): a new
   `retention: &BTreeMap<Hash32, CachedHeader>` param consulted ON A PER-PEER-CACHE MISS
   (`cache.get(h).or_else(|| retention.get(h))`). The durable-anchor check
   (`chaindb.get_block_by_hash`) is UNCHANGED — a retained block is never the LCA anchor.
3. **Owned** in `run_participant_sync` alongside `branch_caches`; threaded into
   `apply_fork_switch` (populate) and `dispatch_competing_fork_choice` → the walk (consult).

## 6. Execution Boundary (TCB color)

- **BLUE — UNCHANGED.** `select_best_chain`, `apply_fork_switch` adoption authority,
  `pump_block`, validation, the durable LCA anchor (still `ChainDb` slot+hash only).
- **GREEN (evidence).** The rollback-retention cache + the walk's consult-on-miss. The
  retention is **evidence, not authority** — same status as the S7 branch cache.
- No RED change.

## 7. Invariants Preserved (registry IDs)

- `DC-NODE-38` (LCA walk: durable-anchor + block-depth-k + cache self-binding — all
  preserved; the retention is an additional evidence source consulted under the SAME
  self-binding + k discipline). `DC-NODE-29` (durable LCA = stored slot+hash).
- `DC-NODE-37` (S4 prevalidate-before-commit), `DC-NODE-25/26/28` (apply/reconcile),
  `DC-NODE-39` (MissingBridge floor — still fires on a GENUINE gap), `CN-CONS-01`
  (fail-closed). The retention does not weaken any of these.

## 8. Invariants Strengthened / Introduced

- **`DC-NODE-40` (introduced, declared).** *Rolled-back blocks MAY be retained only as
  walk-visible EVIDENCE for future competing-branch reconstruction: k-bounded,
  hash-keyed, self-binding (key == re-derived block hash). They never become durable
  authority, a rollback target, the LCA anchor, and never bypass S2 header validation or
  S4 body prevalidation. The durable LCA remains the ChainDb stored slot+hash only; the
  retention merely lets the LCA walk traverse non-durable intermediate headers (the
  blocks Ade itself rolled back) until it reaches a real durable ancestor. Replay-
  equivalent: same rollback + same retained set → same walk outcome.*

## 11. Replay / Crash / Epoch Validation

- The retention is in-memory per-session evidence (not persisted, not WAL). Same
  rollback sequence → same retention contents → same walk verdict (deterministic;
  `BTreeMap`, no HashMap iteration affecting ordering).

## 12. Mechanical Acceptance Criteria

- [ ] `rollback_retains_removed_blocks_for_lca_walk` — cn1 blocks admitted then rolled
  back by a ForkChoiceWin; cn1 later produces a descendant; the walk traverses the
  RETAINED rolled-back headers to the durable LCA; **no `MissingBridge` over-fire** (the
  competing branch resolves via fork-choice).
- [ ] `retained_blocks_are_not_anchors` — the walk must NOT stop at a retained rolled-back
  block as the LCA; it continues until `ChainDb` confirms a durable slot+hash.
- [ ] `retained_blocks_are_k_bounded` — retained evidence older than k (block depth) is
  evicted/rejected; no unbounded growth.
- [ ] `retained_block_hash_self_binds` — a retained entry's map key == its re-derived
  block hash; a mismatch fails closed (`CacheSelfBindingViolation`), like the S7 cache.
- [ ] `genuine_gap_still_missing_bridge` — if neither durable nor cache nor retention
  contains the bridge, `MissingBridge` STILL fires and the fence holds (DC-NODE-39).
- [ ] New gate `ci/ci_check_rollback_retention_evidence.sh` (DC-NODE-40): retention is
  evidence-only (never a durable/anchor/rollback-target; anchor still ChainDb-only),
  hash-keyed + self-binding, k-bounded.
- [ ] `cargo test -p ade_node` green; **S12 harness `bridge_gap_injection_emits_missing_bridge`
  still emits `MissingBridge`** (a genuine gap is unaffected); all prior AO gates green.
- [ ] **Live (CE-AO-6):** a fresh follower run shows the loser-orphan over-fire GONE
  (no `missing_bridge` for the losing peer's normally-evaluable blocks) + post-switch
  descendants admitted. Contributes to the eventual `CN-CONS-03` flip (with a post-fix
  convergence transcript).

## 14. Hard Prohibitions

- No retained rolled-back block may be treated as durable authority.
- No retained rolled-back block may be a rollback target.
- No retained rolled-back block may be the LCA anchor (the anchor is `ChainDb` durable
  slot+hash only).
- No retained header/body may bypass S2 header validation or S4 body prevalidation.
- No unbounded retention (k-bounded by block depth, evicted beyond k).
- No `HashMap` iteration may affect selector ordering — a hash-keyed `BTreeMap` is an
  index only; do not iterate it for semantic ordering.

## 15. Explicit Non-Goals

- Not full per-peer candidate-chain tracking (the Ouroboros-faithful endpoint) — that is
  a larger cluster. S13 is the targeted, minimal retention tied exactly to the observed
  fault (the blocks Ade rolled back).
- Not a fix for Fault 2 (winner-descendant never-received bridge) — that remains the
  S12-harness-driven follow-up; `MissingBridge` stays its correct response.

## 16. Completion Checklist

- [ ] Rollback-retention cache (BTreeMap, self-bound, k-bounded) populated in
  `apply_fork_switch` before the rollback; consulted by `walk_to_durable_lca` on a
  per-peer-cache miss; owned in `run_participant_sync`.
- [ ] 5 hermetic tests + `ci/ci_check_rollback_retention_evidence.sh`.
- [ ] `cargo test -p ade_node` + S12 harness + all AO gates green.
- [ ] Live: over-fire gone + post-switch descendants admitted.
- [ ] `DC-NODE-40` declared → enforced at `/cluster-close`; CE-AO-6 over-fire resolved.
