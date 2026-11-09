# Invariant Slice S7 — Last-common-ancestor anchor walk for live multi-block candidates

> **A live competing branch is eligible for SELECT only when Ade can walk its preserved parent links back to a *durable stored* fork anchor within k (block depth), then validate the *complete* competing header chain from that anchor before selection.**
>
> Slice of cluster PHASE4-N-AO. The live-geometry gap CE-AO-6 surfaced: S3 resolved the fork anchor as the competing block's *immediate parent* (durable only for a 1-deep hermetic candidate), but a real two-producer branch is multi-block, so its immediate parent is an intermediate block on the competing branch Ade never stored → fail-closed. **Concentrated in S3's anchor resolution** (S2/S4/S5/S6 are already multi-body-ready). RED driver + GREEN walk/aggregation + BLUE-reused.

## 2. Slice Header
- **Slice Name:** Live multi-block fork-anchor discovery — walk preserved parent links to the durable last-common-ancestor; build a true multi-header candidate.
- **Cluster:** PHASE4-N-AO (rung-2 SELECT).
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-AO-8** (`DC-NODE-38` live LCA fork-anchor) — a multi-block competing branch walks its preserved parent links to a durable stored fork anchor within k (block depth); the complete intermediate header chain is validated from that anchor (S2); S3 selects over the multi-header candidate; a 1-deep fork still works; missing-intermediate / older-than-k / lying-parent-link / no-durable-LCA / cache-self-binding-violation fail closed; arrival-order independent. New gate `ci/ci_check_lca_anchor_walk.sh` + `cargo test -p ade_node` green. **Unblocks the CE-AO-6 live retry** (which `DC-NODE-38` gates).
- **Slice Dependencies:** S1–S6 (`DC-NODE-34..37`).

## 4. Intent
Make a **live, multi-block** competing branch selectable by discovering its true fork anchor — the **last common ancestor (LCA)**, a durable stored block — rather than assuming the competing block's immediate parent is the anchor. Introduces `DC-NODE-38`. **Hard line:** SELECT proceeds only over a candidate whose complete header chain validates from a durable LCA within k (block depth); otherwise fail closed.

## 5. Scope
- **RED `ade_node::node_lifecycle::dispatch_competing_fork_choice`:** maintain a **per-peer competing-branch header cache** (transient, in-memory); on a `NeedsForkChoice` block, insert it, then **walk `prev_hash` backward through the cache** until a `ChainDb`-stored block matching **slot AND hash** is reached — the LCA / fork anchor. Collect the intermediate headers (LCA+1 … competing tip) in order.
- **GREEN `ade_node`:** `walk_to_durable_lca(cache, competing_block, chaindb, k, current_tip_block_no) -> Result<(ForkAnchor, Vec<HeaderInput>), LcaError>` — the deterministic walk (block-depth-bounded by k; stops only at a durable stored slot+hash; fails on a gap / no-durable-LCA-within-k / over-k / cache-self-binding violation).
- **Reused, UNCHANGED:** S2 `build_candidate_fragment(&[HeaderInput])` (multi-header — already takes a slice); BLUE `validate_and_apply_header` (validates the complete chain from the anchor); `select_best_chain`; S4 `prove_fork_switch`/`prevalidate_branch` + S6 `prefetch_branch_bodies` (`RequestRange(LCA → winner_tip)` — already range-shaped).
- **Out of scope:** the wire-pump RollBackward-vs-Block interleaving characterization (a separate diagnostic — see the carry-forward note in §9); any `CN-CONS-03` flip (gated on the live CE-AO-6 retry).

## 6. Execution Boundary (TCB color)
- **GREEN:** `walk_to_durable_lca` (the deterministic parent-link walk + intermediate-header collection over the cache + durable ChainDb lookups); the per-peer cache management.
- **RED:** the dispatch driver (cache, store reads, the live orchestration).
- **BLUE (reused, unchanged):** `validate_and_apply_header` (via S2), `select_best_chain`, `materialize_rolled_back_state`, `block_validity`.

## 7. Invariants Preserved (registry IDs)
`DC-NODE-29` (the fork anchor is the durable **stored** slot+hash — now the LCA, still never peer-supplied); `DC-NODE-35` (candidate headers validated, never minted — the cache is *evidence*, validation is authority); `DC-NODE-36` (`select_best_chain` sole selector); `DC-NODE-37` (S4 prove-before-commit over the full branch); `DC-NODE-28` (forge fence); `CN-CONS-01` (arrival-order independence).

## 8. Invariants Strengthened / Introduced
- **Introduces `DC-NODE-38`** (declared): *Live multi-block fork-anchor discovery* — a live competing branch is eligible for SELECT only when Ade walks its preserved parent links back to a durable stored fork anchor **within k (block depth)**, then validates the **complete** competing header chain from that anchor before selection. **The cache is NOT authority** — it is only an indexed memory of received preserved headers. The durable LCA is authority **only when `ChainDb` confirms slot+hash**, and S2 validation is authority for candidate construction. Flips `declared → enforced` at `/cluster-close`.

## 9. Design Summary
On a `NeedsForkChoice` competing block `B` from peer `P`: cache `B` in `P`'s branch cache. **Cache discipline (NOT authority):** the cache is an indexed memory of *received, preserved* headers — never trusted as truth. Each entry is **keyed by the re-derived block hash** of its header (`decode_block(B).block_hash`), and binds `entry.slot == header.slot` and `entry.prev_hash == header.prev_hash`; a violation of `key_hash == rederived header block hash` (or the slot/prev bindings) **fails the branch closed** — the cache may not become a stringly map of peer claims.

`walk_to_durable_lca`: starting from `B`, follow `prev_hash` — at each step, if the hash is a `ChainDb`-stored block whose **stored slot == the link's slot**, that block is the **LCA / fork anchor** (durable, `DC-NODE-29`); else if the hash is in the cache, verify that entry's self-binding and step to it; else (a gap — an intermediate header Ade never saw) **fail closed**. The collected intermediate headers (LCA+1 … B), in order, plus the LCA's read-only-materialized `chain_dep`, feed S2 `build_candidate_fragment` → a **true multi-header `CandidateFragment`**.

**k-bound is BLOCK DEPTH, not slot distance** (empty slots must not affect eligibility): the number of **traversed headers ≤ k**, AND, from the LCA's materialized `chain_dep.last_block_no` (`lca_block_no`), `current_tip_block_no − lca_block_no ≤ k`. **No slot subtraction is used for the k check.** Over-k → fail closed (the `ExceededRollback` analog).

From there the existing path is unchanged: per-peer `assemble_candidate_set` → `select_best_chain` → on a win, S4 `prefetch_branch_bodies(RequestRange(LCA → winner_tip))` + `prevalidate_branch` over N bodies + adopt. The 1-deep case degenerates (the walk finds the durable LCA in one step — the existing behavior).

> **Carry-forward (binding):** If a live peer presents the competing branch through `RollBackward`/FOLLOW sequencing rather than competing `Block` arrivals, that is a **separate wire-interleaving diagnostic** and **must not weaken S7's competing-block LCA invariant.** S7 fixes the competing-Block path the gap exercised.

## 10. Changes Introduced
- **Types:** GREEN `LcaError { NoDurableAncestorWithinK, BranchGap, ExceededK, CacheSelfBindingViolation }`; a per-peer branch-header cache (transient: block hash → `{header_input, prev_hash}`, self-binding-checked). No new BLUE/canonical/persisted type.
- **State transitions:** the `NeedsForkChoice` arm: immediate-parent anchor → LCA-walk anchor + multi-header candidate.

## 11. Replay / Crash / Epoch Validation
No new durable state (the branch cache is transient, like the S3 competing map). Durability/replay of an *adopted* multi-block reselection is already S5's `ForkChoiceWin` replay (multi-body via `replay_from_anchor`). Within the seed epoch.

## 12. Mechanical Acceptance Criteria
- [ ] `one_block_fork_still_selects` — a 1-deep competing block (immediate parent IS the durable LCA) walks in one step and selects as before (no regression).
- [ ] `multi_block_branch_walks_to_durable_lca` — an N-block competing branch walks its parent links to the durable LCA; a complete N-header `CandidateFragment` is built + validated; selection proceeds.
- [ ] `missing_intermediate_header_fails_closed` — a competing block whose branch has a gap (an intermediate header absent from the cache) → `BranchGap`, no candidate, no mutation.
- [ ] `ancestor_older_than_k_fails_closed` — the durable LCA is > k **blocks** below the competing tip (traversed-headers > k, or `current_tip_block_no − lca_block_no > k`) → `ExceededK` / `NoDurableAncestorWithinK`, fail closed, no mutation. (Asserted on **block depth**, not slot distance — empty slots do not affect eligibility.)
- [ ] `lying_parent_link_fails_closed` — a peer header whose `prev_hash` does not chain to a real stored ancestor (slot+hash) → fail closed (no slot-only / hash-only match).
- [ ] `cache_self_binding_violation_fails_closed` — a cache entry whose key ≠ the re-derived block hash of its header (or slot/prev mismatch) → `CacheSelfBindingViolation`, fail closed (the cache is evidence, not peer-claim authority).
- [ ] `arrival_order_permutation_selects_same_winner` — permuted competing-block arrival → identical selected winner (`CN-CONS-01` end-to-end over the multi-header path).
- [ ] `s4_still_rejects_body_mismatch_before_commit` — over the multi-header branch, a fetched body not matching its selected header → `BodyHeaderMismatch` before `commit_rollback` (S4 boundary intact for N>1).
- [ ] New gate **`ci/ci_check_lca_anchor_walk.sh`**: the walk stops only at a `ChainDb`-stored **slot AND hash** match; the k-bound is **block depth** (traversed-header count ≤ k AND `current_tip_block_no − lca_block_no ≤ k`) — **no slot subtraction**; the candidate carries **all** intermediate headers (no gaps); cache entries are **self-bound** (key == re-derived header hash); the anchor is never peer-supplied; `build_candidate_fragment` (validated-only) feeds selection.
- [ ] `cargo test -p ade_node` green.
- [ ] **Live (CE-AO-6 retry, gated — NOT this slice):** the transcript additionally asserts `last_common_ancestor_discovered` · `multi_header_candidate_built` · `RequestRange(LCA → winner_tip)` · `prevalidate_branch` over N bodies · `RollBack{ForkChoiceWin}` · `ChainSelected×N` · `agreement_verdict{agreed}` · 0 diverged.

## 13. Failure Modes (all → no durable mutation, fence held)
No durable ancestor within k → `NoDurableAncestorWithinK`. Branch gap (missing intermediate) → `BranchGap`. Over-k (block depth) → `ExceededK`. Cache self-binding violation → `CacheSelfBindingViolation`. Lying / non-chaining parent link → fail closed. Invalid intermediate header (S2 validation) → `CandidateBuildError`. All deterministic.

## 14. Hard Prohibitions
- **No peer-supplied fork anchor** — the LCA is a `ChainDb`-stored block, found by the walk.
- **The cache is NOT authority** — only an indexed memory of received preserved headers; the durable LCA is authority only on `ChainDb` slot+hash confirmation, and S2 validation is authority for candidate construction.
- **Each cached header entry must self-bind** — `key_hash == rederived header block hash`, `entry.slot == header.slot`, `entry.prev_hash == header.prev_hash` — or the branch fails closed.
- **No slot-only ancestor match; no hash-only without slot binding** — the LCA match requires **slot AND hash** (`DC-NODE-29`).
- **No unbounded walk; the k-bound is BLOCK DEPTH, not slot distance** — traversed-header count ≤ k AND `current_tip_block_no − lca_block_no ≤ k`; over-k fails closed; empty slots never affect eligibility.
- **No candidate fragment with missing intermediate headers** — a complete LCA+1…tip chain or fail closed.
- **No adoption from ChainSync bytes alone** — the cache is evidence; S2 validation + S4 body proof are authority.
- **No `CN-CONS-03` flip** until a clean live two-producer transcript lands (CE-AO-6 retry).

## 15. Explicit Non-Goals
No `CN-CONS-03` flip (CE-AO-6 retry); no wire-pump RollBackward/Block interleaving rework (separate diagnostic, carry-forward §9); no new BLUE; no change to S4/S5/S6's proven cores beyond feeding them N headers/bodies.

## 16. Completion Checklist
- [ ] `walk_to_durable_lca` (slot+hash durable stop, block-depth-k-bounded, gap-closed, cache-self-binding-checked) + per-peer branch cache.
- [ ] Multi-header `CandidateFragment` from the LCA; S3 selects over it; 1-deep unchanged.
- [ ] 8 hermetic tests green; new gate + reused gates green; `cargo test -p ade_node` green.
- [ ] `DC-NODE-38` declared; ready to flip at `/cluster-close`; CE-AO-6 live retry unblocked.
