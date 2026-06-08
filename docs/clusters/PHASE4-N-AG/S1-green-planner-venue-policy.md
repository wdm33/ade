# Invariant Slice — GREEN planner VenuePolicy refinement (DC-NODE-19 S1)

## 2. Slice Header
**Slice Name:** GREEN planner VenuePolicy refinement (DC-NODE-19, PHASE4-N-AG S1)
**Cluster:** PHASE4-N-AG — single-producer loop continuation after follow-link EOF; **rung-1 only, single-producer only**
**Status:** Proposed
**Authority source:** `docs/clusters/PHASE4-N-AG/cluster.md` (§3, §5, CE-AG-1); `docs/planning/single-producer-loop-continuation-after-feed-eof-invariants.md`; registry `DC-NODE-19` (declared — **not flipped** by this slice)

**Cluster Exit Criteria Addressed:**
- [ ] **CE-AG-1:** `plan_loop_step` total over **5** inputs — a 32-case exhaustive table test (no wildcard); a reduction test proves `VenuePolicy::HaltOnFeedEnd` reproduces the prior 16-case table exactly; the `(VenueRole,ForgeMode)→VenuePolicy` projection test; planner stays content-blind (no `SlotNo` in `plan_loop_step`).

Exit criteria not listed (CE-AG-2..6) are out of scope for this slice.

**Slice Dependencies:** none — first slice of N-AG; builds only on invariants enforced at HEAD (`DC-NODE-05/09/12/15/18`, `T-REC-03/05`, `CN-NODE-02`, `DC-CONS-03`).

## 3. Implementation Instruction (AI)
**Read §§9–10 + the cluster doc + `ci_check_loop_planner_closed.sh` first.** GREEN-primary, **behavior-preserving**. Add `VenuePolicy` to `run_loop_planner`; extend `plan_loop_step` to a 32-case total table (**no wildcard, no `SlotNo`** — the planner-closed gate stays green); add the pure `venue_policy` projection in `node_sync`; update the single `node_lifecycle` call site to pass `VenuePolicy::HaltOnFeedEnd` (default — observable behavior byte-unchanged). Do **not** touch the loop's continuation logic, the fence, the Idle wait, or add a CI gate (all S2). Do **not** flip `DC-NODE-19`. Obey §14/§15; §12 is the only completion proof. Commit carries the repo's model trailer (per `CLAUDE.md`).

## 4. Intent
Make DC-NODE-19's "continue vs halt on feed-end" decision an **explicit, total, content-blind input** to the pure planner: extend `plan_loop_step`'s closed decision table with a 5th `VenuePolicy` dimension (32 cases, no wildcard) while the default `HaltOnFeedEnd` reduces **exactly** to the prior 16-case behavior. Strengthens the planner's closed/total/content-blind property; introduces the GREEN surface S2 threads. **No runtime behavior change.**

## 5. Scope
- **Modules:** `ade_node::run_loop_planner` (new `VenuePolicy` enum; `plan_loop_step` gains the 5th param; extended exhaustive-table tests); `ade_node::node_sync` (new pure `venue_policy(VenueRole, &ForgeMode) -> VenuePolicy` + its test); `ade_node::node_lifecycle` (the single `run_relay_loop_with_sched` call site passes `VenuePolicy::HaltOnFeedEnd` — **call-site default-pass only**, no logic/behavior change).
- **State machines:** none new (`VenuePolicy` is a content-blind planner input, not a state machine).
- **Persistence:** none.
- **Network-visible:** none.
- **Out of scope:** the RED loop continuation + fence + per-continuation cert re-validation + Idle-under-dead-feed wakeup + the new CI gate (S2); replay (S3); live (S4). `DC-NODE-19` stays declared.

## 6. Execution Boundary (TCB color)
- **BLUE (UNCHANGED — zero diff):** none touched.
- **GREEN:** `ade_node::run_loop_planner` — `VenuePolicy` enum + `plan_loop_step` 32-case total table (pure/total/deterministic/content-blind); `ade_node::node_sync` — the `venue_policy` projection (pure/total over `VenueRole × ForgeMode`).
- **RED:** `ade_node::node_lifecycle` — **call-site default-pass only** (`plan_loop_step(…, VenuePolicy::HaltOnFeedEnd)`); no logic/behavior change, no continuation, no fence (S2).
- **Placement resolution:** `VenuePolicy` in `run_loop_planner` (with the other closed planner-input enums; no new import); the projection in `node_sync` (owns `VenueRole`/`ForgeMode`, matches freely — the planner-closed gate's no-`_ =>` rule does not burden the `ForgeMode` match; `run_loop_planner` stays a dep-leaf). New edge `node_sync → run_loop_planner::VenuePolicy` (GREEN→GREEN, cycle-free; `run_loop_planner` imports no `node_sync`).

## 7. Invariants Preserved (registry IDs)
`DC-NODE-05` (planner stays closed/subordinate; `plan_loop_step` content-blind, no `SlotNo`, exhaustive) · `CN-NODE-02` (planner half — closed lifecycle decision fn, no authority) · `T-REC-03` (loop-as-replay — the planner is pure/deterministic and the default reduces to the prior table, so loop behavior is byte-unchanged) · `DC-NODE-18` (`ForgeMode`/`VenueRole` read-only by the projection; the DC-NODE-18 fence untouched) · `DC-CONS-03` (**untouched** — planner names no chain selector) · `T-DET-01` (the new GREEN fns are pure/deterministic).

## 8. Invariants Strengthened or Introduced
Strengthens the **GREEN planner closed/total/content-blind property** — `plan_loop_step` becomes total over a 5th `VenuePolicy` input (32-case, no wildcard) with the default reducing exactly to the prior 16-case table — locked by the extended exhaustive-table tests + `ci_check_loop_planner_closed.sh` (still green). The first, behavior-preserving step toward **`DC-NODE-19`** (declared; enforced at cluster close after S2–S4). Exactly **one** invariant family (loop-planner totality/closedness). **Does NOT flip `DC-NODE-19`.**

## 9. Design Summary
- New closed enum `VenuePolicy { HaltOnFeedEnd, ContinueInSingleProducerExtend }` in `run_loop_planner` (no `#[non_exhaustive]`).
- `plan_loop_step(loop_state, sync_status, forge_slot_status, shutdown_status, venue_policy) -> LoopStep`: precedence **shutdown → sync → (NoWorkReady) loop_state**; on `Ending` → `match venue_policy { ContinueInSingleProducerExtend => match forge_slot { Due => ForgeTick, NotDue => Idle }, HaltOnFeedEnd => HaltCleanly }`; `Continuing` arm unchanged. Exhaustive by name (no `_ =>`); content-blind (no `SlotNo`). **For all prior 4-input combinations, `VenuePolicy::HaltOnFeedEnd` produces exactly the same `LoopStep` as the pre-S1 planner table** (the reduction obligation, locked by `plan_loop_step_halt_policy_reduces_to_prior_16`).
- `venue_policy(venue_role: VenueRole, forge_mode: &ForgeMode) -> VenuePolicy` in `node_sync`: `ContinueInSingleProducerExtend` iff `venue_role == SingleProducer && matches!(forge_mode, SingleProducerExtendOwnDurableSpine { .. })`; else `HaltOnFeedEnd`. Pure/total.
- The `node_lifecycle` call site passes `VenuePolicy::HaltOnFeedEnd` (S1); **S2** replaces it with `venue_policy(act.venue_role, &act.forge_mode)`.

## 10. Changes Introduced
- **Types:** `VenuePolicy` enum (`run_loop_planner`). No type modified (`LoopStep`/`LoopState`/`SyncStatus`/`ForgeSlotStatus`/`ShutdownStatus` unchanged; `plan_loop_step` gains a param).
- **State transitions:** `plan_loop_step` table extended 16 → 32 cases — the `Ending` + `ContinueInSingleProducerExtend` cells are new (`ForgeTick`/`Idle` instead of `HaltCleanly`); `HaltOnFeedEnd` identical to prior.
- **Persistence:** none.
- **Removal/Refactors:** none (the call-site default-pass is additive; S2 changes that one line).

## 11. Replay, Crash, and Epoch Validation
- **Replay:** the post-feed-end replay proof is **CE-AG-4 (S3)**, out of scope here; S1 must not perturb `T-REC-03` — `plan_loop_step_halt_policy_reduces_to_prior_16` proves the default path is byte-identical to the prior table, so loop-as-replay is unchanged. No new persisted/replayed state (`VenuePolicy` is a pure input, never WAL'd).
- **Crash/restart:** n/a (no persistence change).
- **Epoch boundary:** n/a.

## 12. Mechanical Acceptance Criteria
- [ ] `plan_loop_step_venue_policy_table_is_total` — 2⁵ = 32 exhaustive cases vs an independent `expected` oracle (`run_loop_planner::tests`).
- [ ] `plan_loop_step_halt_policy_reduces_to_prior_16` — with `VenuePolicy::HaltOnFeedEnd`, `plan_loop_step` reproduces the prior 16-case mapping exactly (a `prior_expected` oracle = the pre-S1 4-input table over all 16 combinations).
- [ ] `venue_policy_projection_is_continue_only_in_extend` — returns `ContinueInSingleProducerExtend` only for `(SingleProducer, SingleProducerExtendOwnDurableSpine{..})`; `HaltOnFeedEnd` for every other `VenueRole × ForgeMode` (all 4 modes × 2 roles enumerated) (`node_sync::tests`).
- [ ] `ci/ci_check_loop_planner_closed.sh` green — planner stays closed/total/content-blind after the `VenuePolicy` addition (no `#[non_exhaustive]`, no forbidden token, no `_ =>`, no `SlotNo` in `plan_loop_step`).
- [ ] `cargo test -p ade_node` green (extended planner tests + projection test + all existing).
- [ ] **No new CI gate; `DC-NODE-19` not flipped** (the gate + flip are S2/close).

## 13. Failure Modes
None new. S1 is a pure total-function extension + a default-pass; `plan_loop_step` stays total (exhaustive, no panic path). No new error shape.

## 14. Hard Prohibitions
**Inherited (cluster §8):** no RED `LoopState` re-derivation / planner "lie"; no new durable tip path; no chain-selector reference (**DC-CONS-03 untouched**); zero BLUE change; no new CLI flag.
**Slice-specific:**
- No wildcard `_ =>` anywhere in `run_loop_planner` production (the 32-case table is exhaustive by name).
- No `SlotNo` in `plan_loop_step` (content-blind; `SlotNo` only in `forge_slot_status`).
- No `#[non_exhaustive]` on `VenuePolicy` (closed vocabulary).
- No behavior change at the call site (S1 passes the default; continuation is S2).
- No flip of `DC-NODE-19`; no new CI gate.
- No determinism tripwires (no wall-clock / float / `HashMap` / `String`/`anyhow` errors) in the new GREEN fns.
- No threading of the real venue policy into the loop (S2).

## 15. Explicit Non-Goals
The RED loop continuation, the certified-run fence, the per-continuation cert re-validation, the Idle-under-dead-feed wakeup (S2) · replay-equivalence over a post-feed-end chain (S3) · the operator-gated live run (S4) · flipping `DC-NODE-19` · the DC-NODE-19 CI gate · any CLI flag.

## 16. Completion Checklist
- [ ] `plan_loop_step` total over 5 inputs (32-case test green).
- [ ] default `HaltOnFeedEnd` byte-reduces to the prior 16-case table (reduction test green).
- [ ] `venue_policy` projection correct + total (projection test green).
- [ ] `ci_check_loop_planner_closed.sh` green (closed/content-blind preserved).
- [ ] `cargo test -p ade_node` green.
- [ ] Zero BLUE change; loop observably identical to pre-S1.
- [ ] `DC-NODE-19` NOT flipped (stays declared).
```
