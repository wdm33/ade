# Invariant Slice S5 — Reselection replay-equivalence + fence resolution (close the loop)

> **A fork-choice reselection is complete only when durable replay, selector state, ChainDb tip, ledger fingerprint, and forge fence all converge to the same post-decision state.**
>
> Slice of cluster PHASE4-N-AO. The slice that closes the durability/replay/fence loop S4 opened. **No new selection or fetch behavior.** Proves a `ForkChoiceWin` reselection is replay-equivalent and crash-safe, and defines exactly how the forge fence *resolves*. BLUE-reused (`replay_from_anchor`, `DC-NODE-15` catch-up) + GREEN fence resolution + RED relay wiring.

## 2. Slice Header
- **Slice Name:** Reselection replay-equivalence + reconcile + forge-fence resolution.
- **Cluster:** PHASE4-N-AO (rung-2 SELECT).
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-AO-5** (`DC-NODE-27` ext. replay-equivalence) — a multi-peer feed containing a `ForkChoiceWin` reselection replays byte-identically (durable tip + ledger fp + chain_dep), recovering the **selected** chain; `selector == durable` post-decision; no forge across pending reselection. Reused `ci_check_wal_rollback_replay_equiv.sh` (extended for `ForkChoiceWin`) + `ci_check_live_fork_choice_wiring.sh` green; `cargo test -p ade_ledger` + `-p ade_node` green.
- **Slice Dependencies:** S1–S4 (`DC-NODE-34..37`).

## 4. Intent
Make a `ForkChoiceWin` reselection **durably complete and crash-safe**, and give the forge fence a **defined resolution**. Strengthens `DC-NODE-27` (replay-equivalence → multi-peer reselection) and `DC-NODE-28` (forge fence → set/hold/resolve lifecycle). **Hard line:** no crash-recovery half-switch, no fake winner after a crash, and the fence never re-enables forging until the disagreement is *resolved* (not merely failed).

## 5. Scope
- **Reused, no change:** BLUE `replay_from_anchor` (the reason-agnostic fp-walk over `AdmitBlock`/`RollBack` + the body store) — `ForkChoiceWin` already replays through it; `warm_start_recovery` (the restart path that calls it); `forge_followed_tip_admission` (`DC-NODE-15` catch-up gate); `pending_reselection_forge_refusal` (`DC-NODE-28` forge gate); `apply_chain_event` reconcile (`DC-NODE-26`).
- **RED `ade_node::node_lifecycle`:** a fence-**resolution** step in the relay loop after the S4 apply — clears `pending_reselection` only on a *resolved* state.
- **GREEN `ade_node`:** `fork_switch_fence_resolved(pending_fork_switch, caught_up) -> bool` — the pure resolution predicate.
- **Out of scope:** live `BlockFetch RequestRange` (CE-AO-6); `CN-CONS-03` flip (CE-AO-6 / cluster-close); any new selection/fetch/BLUE; **a durable body-staging store** (deliberately NOT added — see §9 (4)).

## 6. Execution Boundary (TCB color)
- **BLUE (reused):** `replay_from_anchor`, `materialize_rolled_back_state`, `block_validity`. **No new BLUE.**
- **GREEN:** `fork_switch_fence_resolved` (deterministic predicate over pending-decision + caught-up).
- **RED:** the relay-loop fence-resolution wiring; the (reused) `warm_start_recovery` crash path.

## 7. Invariants Preserved (registry IDs)
`DC-NODE-37` (S4 prove-then-commit — unchanged); `DC-NODE-25` (apply authority); `DC-NODE-29` (anchor binding); `CN-STORE-07` (materialize); `DC-CONS-20` (lockstep); `DC-NODE-15` (catch-up forge gate — the crash-recovery forge protection); `DC-NODE-05/12` (pump_block sole admit); `T-REC-03/05` (recovery fingerprint).
- **Doc correction carried (S4 finding):** *rollback depth is independently checked before mutation by S4's prevalidation `materialize` (`AnchorUnreachable`); the apply-time `materialize` remains a second guard on the success path.* `k`-authority is durable/config (`ForgeActivation.security_param`), never peer.

## 8. Invariants Strengthened
- **`DC-NODE-27` → multi-peer reselection replay-equivalence:** a WAL sequence `RollBack{ForkChoiceWin, to=fork_anchor}` + `AdmitBlock(body)×N` replays byte-identically (durable tip + ledger fp + chain_dep), recovering the **selected** chain — via the existing reason-agnostic `replay_from_anchor`, not a second replay path.
- **`DC-NODE-28` → fence set/hold/resolve lifecycle:** the forge fence is **set** by S3 on a win, **held** by S4 on proof failure, and **resolved** (cleared) only by (a) successful adoption + reconcile, or (b) a resolved no-pending-selection state (caught up to the followed peer with no pending decision). **Never** cleared by a fetch/proof failure.

## 9. Design Summary — the five cases

**(1) Success-path replay-equivalence.** `ForkChoiceWin` adoption appends `RollBack{FCW, to=fork_anchor}` + `AdmitBlock(body)`. On restart, `warm_start_recovery → replay_from_anchor` (reason-agnostic) replays it → the **same** durable tip + ledger fp + chain_dep — the selected chain. Reuses `DC-NODE-27`; nothing new in the replay path.

**(2) Selector == durable.** Ade derives the selector state from the durable tip each dispatch (S3 Option A — no persisted selector). Post-adoption, `project_tiebreaker(durable tip)` reflects the winner — selector and durable converge by construction. A test asserts the consistency.

**(3) Crash before commit.** No `RollBack{FCW}` was appended → replay reproduces the **original** chain, byte-unchanged. The in-memory decision is lost; the node re-syncs + re-decides. No half-switch.

**(4) Crash after `RollBack` WAL, before all bodies (the scary one).** The WAL holds `RollBack{FCW, to=fork_anchor}` but the body's `AdmitBlock` was not appended (and the body is not in the store). `replay_from_anchor` **deterministically recovers the valid prefix at `fork_anchor`; the selected chain is NOT considered adopted until its bodies are present and replayed/admitted.** This is a valid shorter prefix, *not* a fork-choice adoption — never read the anchor recovery as a successful adoption. The node recovers **behind** the peers → `forge_followed_tip_admission → NotCaughtUp` → the **existing `DC-NODE-15` catch-up gate refuses forge** until re-sync. No half-switch, **no silent forge, no fake winner**. (The pending decision is in-memory; the node re-decides on re-sync.) **We deliberately do NOT add a durable body-staging store:** recovering to a valid shorter prefix is acceptable; forging from it while not caught up is not — and `DC-NODE-15` already forbids the second. Staging would add a new persistence surface, new replay semantics, new crash states, and a new audit burden only to save a re-sync.

**(5) Reconcile mismatch.** `apply_chain_event`'s `ReconciliationMismatch` (`DC-NODE-26`) → `apply_fork_switch` returns `Err` **before** the fence-clear → the fence stays set, forge refused (fail-fast).

**Fence resolution (live).** After S4's apply step the relay loop calls `fork_switch_fence_resolved(pending_fork_switch, caught_up)`:
```
fork_switch_fence_resolved =
    pending_fork_switch.is_none()
    && forge_followed_tip_admission == CaughtUp
```
Clear `pending_reselection` iff the predicate holds. A proof failure leaves the fence set (it only says *that branch was not proven*, not *the disagreement is resolved*); it clears only when the loop reaches this resolved state. (S4's direct success-path clear stays; this covers the held-then-resolved path.)

## 10. Changes Introduced
- **GREEN** `fork_switch_fence_resolved` predicate. **RED** one relay-loop resolution call after the S4 apply. No new types; no canonical/persisted change (`pending_reselection` stays in-memory; crash protection is `DC-NODE-15`).

## 11. Replay / Crash / Epoch Validation
- Tests by name: `ade_node` integration (the `live_fork_choice_ai_s4bii.rs` family + `corpus_durable_fork` + `BranchBodySource`/WAL doubles) and/or `ade_ledger` WAL-replay tests. Crash cases drive `replay_from_anchor` with the three WAL prefixes. Epoch: within the seed epoch (anchor chain_dep carries eta0, `T-REC-06`).

## 12. Mechanical Acceptance Criteria
- [ ] `forkchoicewin_reselection_replays_byte_identical` — `RollBack{FCW}`+`AdmitBlock` → same durable tip + ledger fp + chain_dep on replay (recovers the selected chain).
- [ ] `selector_equals_durable_post_forkchoicewin` — post-adoption, the selector projection equals the durable tip.
- [ ] `crash_before_commit_replays_no_mutation` — WAL without `RollBack{FCW}` → replay = original chain.
- [ ] **`crash_after_rollback_before_bodies_recovers_valid_prefix_no_silent_forge`** — WAL = `RollBack{FCW}` only → `replay_from_anchor` deterministically recovers the **valid prefix at `fork_anchor`**; `forge_followed_tip_admission` = `NotCaughtUp` (forge refused). *(the scary case)*
- [ ] **`forkchoicewin_rollback_without_bodies_is_no_fake_winner`** — a `RollBack{ForkChoiceWin}` WITHOUT the following `AdmitBlock` bodies **MUST NOT** produce selector/current-tip agreement with the selected winner: the recovered durable/selector tip does **not** equal the winner's tip (it equals `fork_anchor`), and the node remains **not-caught-up**. The selected chain is adopted ONLY when its bodies are present + replayed/admitted. *(the core "no fake winner after crash" proof)*
- [ ] `proof_failure_holds_fence_then_resolves_when_caught_up` — S4 proof failure → fence held; a resolved (caught-up, no-pending) state → `fork_switch_fence_resolved` clears it; the failure itself never clears it.
- [ ] `reconcile_mismatch_holds_fence_no_forge` — `ReconciliationMismatch` → `apply_fork_switch` `Err`, fence not cleared, forge refused.
- [ ] Extend **`ci_check_wal_rollback_replay_equiv.sh`**: `RollbackReason::ForkChoiceWin` is a covered reason in the replay walks; the re-anchor still binds `to_point` only (never `selected_tip`). Reused `ci_check_live_fork_choice_wiring.sh` green.
- [ ] `cargo test -p ade_ledger` + `cargo test -p ade_node` green.

## 13. Failure Modes
Crash before commit → original chain. Crash mid-apply → deterministic valid prefix at `fork_anchor` + catch-up gate holds forge (no fake winner). Reconcile mismatch → fail-fast, fence held. All deterministic; none half-switches; none silently forges; none fabricates a winner.

## 14. Hard Prohibitions
- The fence **MUST NOT** clear on a fetch/proof failure — only on success-reconcile or a resolved caught-up-no-pending state.
- A `RollBack{ForkChoiceWin}` without its bodies **MUST NOT** be read as an adoption — recovery yields the valid prefix at `fork_anchor`, never the winner's tip.
- No second replay/rollback path — `replay_from_anchor` is the sole WAL replay; the re-anchor binds `to_point`, never `selected_tip` (no header-only adoption from WAL metadata).
- No durable persistence of `pending_reselection`, and no durable body-staging store (crash protection is `DC-NODE-15`, not persisted state).
- No new selection/fetch/BLUE; no `CN-CONS-03` flip; no live `BlockFetch`.

## 15. Explicit Non-Goals
Live mux `RequestRange` fetch (CE-AO-6); the two-producer operator transcript + `CN-CONS-03` flip (CE-AO-6/close); multi-header candidate aggregation (follow-on); a durable body-staging store (deliberately deferred — valid-prefix + catch-up is the recovery policy).

## 16. Completion Checklist
- [ ] `ForkChoiceWin` replay-equivalence proven; replay-equiv gate extended.
- [ ] Three crash cases proven (no mutation / valid-prefix-no-fake-winner-no-silent-forge / winner recovered).
- [ ] Fence resolution implemented + proven (held on failure; cleared only on resolved state); reconcile-mismatch holds fence.
- [ ] `cargo test -p ade_ledger` + `-p ade_node` green; gates green.
- [ ] `DC-NODE-27` + `DC-NODE-28` strengthenings ready to record at `/cluster-close`.
