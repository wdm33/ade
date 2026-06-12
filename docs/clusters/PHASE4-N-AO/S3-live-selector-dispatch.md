# Invariant Slice S3 — Live selector dispatch (RED driver: decide, don't switch)

> Slice of cluster PHASE4-N-AO. The slice where **SELECT becomes live** — the Participant `NeedsForkChoice` arm (today fail-closed) is wired to the existing BLUE `select_best_chain`. **RED driver.** Consumes S1 (`DC-NODE-34`, peer identity) + S2 (`DC-NODE-35`, the pure GREEN construction core). **Decides only — never switches** (the fork-switch apply is S4).

## 2. Slice Header
- **Slice Name:** Live selector dispatch — wire `NeedsForkChoice → select_best_chain`; represent a win as a provisional decision for S4.
- **Cluster:** PHASE4-N-AO — live multi-candidate fork-choice SELECT + adopt (rung-2).
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-AO-3** (`DC-NODE-36` single-selector dispatch) — the `NeedsForkChoice` arm routes the candidate **set** to `select_best_chain`; no second selector; arrival-order-independent; `TiebreakerLossKeepCurrent` makes no durable change. New gate `ci/ci_check_live_selector_dispatch.sh` + reused `ci_check_chain_selection_arrival_order_independent.sh` green.
- **Slice Dependencies:** S1 (`DC-NODE-34`, `31efec44`) + S2 (`DC-NODE-35`, `6bcfc9e5`).

## 4. Intent
Make `select_best_chain` the **sole, live** authority that resolves a competing Participant candidate — and make a fork-choice **win a provisional decision, never an application**. S3 may DECIDE; S4 APPLIES. Strengthens `DC-NODE-36`. **Hard line: S3 must not abandon current durable state** — no rollback, no body-fetch, no commit. The current chain is preserved until S4 validates a replacement (FC-6).

## 5. Scope
- **Modules / crates:** RED `ade_node::node_lifecycle::run_participant_sync` — replace the `NeedsForkChoice` fail-closed arm (`node_lifecycle.rs:2566-2569`) with the dispatch driver; **bind `peer`** on `NodeSyncItem::Block { peer, bytes }` (consume S1); maintain a per-peer competing-candidate tracker (`BTreeMap<peer, …>`). Reused: S2 `candidate_aggregator::{build_candidate_fragment, assemble_candidate_set}`; BLUE `select_best_chain`; **read-only** `materialize_rolled_back_state` (`CN-STORE-07`) for the fork-point chain_dep; `ChainDb::get_block_by_hash` / `tip` (read); `decode_block`.
- **State machines affected:** the provisional decision — a new closed `PendingForkSwitch { fork_anchor: (slot, hash, block_no), winning_peer, winning_candidate }` held transiently (a `ForwardSyncState` field or a threaded `Option`), set on a win, **consumed by S4** (latent until then).
- **Persistence impact:** **none** — S3 writes no WAL, mutates no ChainDb / ledger / chain_dep. (`materialize` is read-only; the pending decision is transient.)
- **Network-visible impact:** none.
- **Out of scope (→ S4):** body block-fetch of the winning branch; `materialize` + `commit_rollback` to the fork anchor; `pump_block` of the winner; `WalEntry::RollBack{ForkChoiceWin}`. (→ S2, done): the pure construction core. (→ S5): replay-equivalence of the applied reselection.

## 6. Execution Boundary (TCB color)
- **BLUE:** none new. Reused: `select_best_chain`, `validate_and_apply_header` (via S2), `materialize_rolled_back_state` (read-only).
- **GREEN:** reused — S2 `candidate_aggregator`; the `ChainSelectorState`-from-durable reconciliation projection.
- **RED:** `ade_node::node_lifecycle::run_participant_sync` — the live dispatch driver (store reads, read-only materialize, per-peer grouping, the decision handoff). This is where the store / materialize orchestration the S2 boundary excluded now lives.

## 7. Invariants Preserved (registry IDs)
`DC-NODE-34/35` (consumed, not weakened), `DC-CONS-03` (`select_best_chain` — routed-to, never duplicated; density forbidden), `CN-CONS-01` (arrival-order independence — preserved end-to-end), `CN-STORE-07` (materialize — reused **read-only**, no second path, **no commit**), `DC-CONS-20` (lockstep — untouched; no durable mutation in S3), `DC-NODE-25` (apply authority — **not invoked** in S3), `DC-NODE-28` (no forge across pending reselection — preserved / strengthened), `DC-NODE-29` (canonical durable binding — the fork anchor binds the **stored** slot+hash, generalized here), `T-REC-06` (eta0 overlay — the read-only materialize passes the recovered eta0), `pump_block` (sole admit — **not invoked** in S3).

## 8. Invariants Strengthened / Introduced
- **Strengthens toward enforced** `DC-NODE-36` (live single-selector dispatch): the Participant `NeedsForkChoice` arm routes the candidate set to the single existing `select_best_chain`; a `TiebreakerLossKeepCurrent` is a no-op; a win is a **provisional decision**, never an application. `DC-NODE-36` flips at `/cluster-close`. **One invariant family:** live single-selector dispatch (decide-not-switch).

## 9. Design Summary
On a Participant `Competing` block (`classify_receive` → `NeedsForkChoice`): **(proof center)** resolve the fork anchor by `ChainDb::get_block_by_hash(decoded.prev_hash)` → bind the **durable stored** `(slot, hash, block_no)` (never peer-supplied; `DC-NODE-29` discipline); obtain `anchor_chain_dep` by a **read-only `materialize_rolled_back_state`** at that durable anchor (no commit; passes the recovered eta0, `T-REC-06`). Accumulate the peer's header inputs (`BTreeMap`-keyed by `peer` from S1); call S2 `build_candidate_fragment(anchor, anchor_chain_dep, current_tip_block_no, headers, …)` + `assemble_candidate_set`; derive `ChainSelectorState` from the durable ChainDb (current tip, immutable tip, k); call **BLUE `select_best_chain`**. Dispatch the result: `Rejected{TiebreakerLossKeepCurrent}` or any ineligible reject → **no-op** (current chain untouched); `ChainSelected{…}` → set `PendingForkSwitch` + the `DC-NODE-28` forge fence (`pending_reselection`), **emit toward S4, apply nothing**. A `prev_hash` that is neither durable nor a known peer-candidate parent → **fail closed**.

## 10. Changes Introduced
- **Types:** closed `PendingForkSwitch` (the provisional decision; transient, not persisted) + a per-peer competing-candidate tracker (transient, `BTreeMap`-keyed). No new BLUE type; no canonical / persisted type.
- **State transitions:** the `NeedsForkChoice` arm: fail-closed → decide-and-hold (set `PendingForkSwitch` + fence forging on a win; no-op on a loss).
- **Persistence:** none.

## 11. Replay / Crash / Epoch Validation
- Hermetic dispatch tests (`ade_node` integration, the `live_fork_choice_ai_s4bii.rs` fixture family): permuted competing-block arrival → identical decision (`select_best_chain` order-independence); a loss → ChainDb / WAL / ledger byte-unchanged. No persisted state in S3 → no new replay corpus (the applied reselection's replay is S5). Read-only `materialize` is replay-neutral (no commit).
- **Crash/restart:** not applicable (no persisted state; the pending decision is transient).
- **Epoch boundary:** not applicable (within the seed epoch; the anchor chain_dep carries the eta0 basis).

## 12. Mechanical Acceptance Criteria
- [ ] `participant_competing_candidate_dispatches_to_select_best_chain` — a Participant `Competing` block whose fork anchor is durable + validates → `select_best_chain` is called; **no `UnexpectedRollback`** (the arm no longer fails closed). *(criterion 1)*
- [ ] `dispatch_arrival_order_independent` — permuted competing-block order → identical selected decision. *(criterion 3)*
- [ ] `tiebreaker_loss_makes_no_durable_change` — a losing competing candidate → ChainDb tip + WAL + ledger unchanged; no `PendingForkSwitch`. *(criterion 4)*
- [ ] `fork_choice_win_sets_pending_decision_not_committed` — a winning competing candidate → `PendingForkSwitch = Some(fork anchor + winning candidate)`; **ChainDb / WAL / ledger UNCHANGED** (nothing applied). *(criterion 5)*
- [ ] `pending_reselection_fences_forge` — reuse `DC-NODE-28`: a pending decision refuses a forge tick. *(criterion 6)*
- [ ] **Proof center** `fork_anchor_bound_to_durable_stored_slot_hash` — the fork anchor is `get_block_by_hash(prev_hash)`'s stored `(slot, hash)`; a peer-supplied anchor slot/hash is **not** used; an unknown `prev_hash` (not durable, not a known peer-candidate parent) **fails closed**.
- [ ] **Proof center** `anchor_chain_dep_from_readonly_materialize_no_mutation` — `anchor_chain_dep` comes from `materialize_rolled_back_state` at the durable anchor; **no durable mutation** (read-only); peer cannot supply the anchor state.
- [ ] New gate **`ci/ci_check_live_selector_dispatch.sh`**: (A) `select_best_chain` is the **only** selector called in the dispatch path; (B) the fork anchor is resolved via `get_block_by_hash` + bound to the stored slot/hash, `anchor_chain_dep` via `materialize_rolled_back_state`, no peer-supplied anchor; (C) **no S3 apply** — no `commit_rollback` / `pump_block` of the winner / `WalEntry::RollBack` in the `NeedsForkChoice` path; (D) `DC-NODE-28` forge fence set on a win; (E) candidates come from S2 `build_candidate_fragment` (no minting). Reused `ci_check_chain_selection_arrival_order_independent.sh` green.
- [ ] `cargo test -p ade_node` green.

## 13. Failure Modes
- `prev_hash` not durable + not a known peer-candidate parent → fail closed (typed; no decision, current chain unchanged).
- `materialize` `RollbackTooDeep` (anchor beyond k / retention) → fail closed (the candidate is unreachable; `DC-CONS-05`).
- Candidate header invalid (S2 `CandidateBuildError`) → the candidate is dropped (fail closed), current chain unchanged.
- All deterministic; none mutates durable state.

## 14. Hard Prohibitions
**Inherits all nine cluster hard lines.** **Slice-specific (the sharp boundary):**
- **S3 must not abandon current durable state** — **no `commit_rollback`, no `pump_block` of a winning branch, no `WalEntry::RollBack`, no block-fetch.** S3 decides; **S4 applies.**
- The fork anchor comes from Ade's **durable stored `(slot, hash)`** (`get_block_by_hash`), and `anchor_chain_dep` from a **read-only `materialize`** — **never peer-supplied.** (The proof center.)
- `select_best_chain` is the **only** selector; no second selector / parallel preference / density / operator heuristic.
- Candidate fragments come **only** from S2 `build_candidate_fragment` (no minting; no `follow.rs` summary).
- A fork-choice win is **provisional** (`PendingForkSwitch` + forge fence) — never an application.
- No `HashMap` (deterministic `BTreeMap` per-peer grouping).

## 15. Explicit Non-Goals
No fork-switch apply / body-fetch / rollback-commit / `WalEntry::RollBack` (S4); no replay-equivalence of an applied reselection (S5); no new BLUE; no `select_best_chain` change; no durable mutation.

## 16. Completion Checklist
- [ ] `NeedsForkChoice` arm dispatches to `select_best_chain` (no longer fail-closed); `peer` bound (S1 consumed); per-peer `BTreeMap` tracker.
- [ ] Fork anchor durable-bound; `anchor_chain_dep` read-only-materialized; peer-supplied anchor impossible (proof center).
- [ ] Loss → no durable change; win → `PendingForkSwitch` + forge fence, **nothing applied**.
- [ ] New gate + reused arrival-order gate green; `cargo test -p ade_node` green.
- [ ] No durable mutation, no body-fetch, no rollback-commit (S4's boundary intact); `DC-NODE-36` ready to flip at `/cluster-close`.
