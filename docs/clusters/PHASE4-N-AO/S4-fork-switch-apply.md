# Invariant Slice S4 — Selected-range fetch + fork-switch apply (prove, then commit)

> **A `PendingForkSwitch` is not authority to roll back; it is only authority to *attempt proof* of the selected replacement branch.**
>
> Slice of cluster PHASE4-N-AO. The slice where SELECT either becomes safe or becomes dangerous. Consumes S3's provisional decision (`DC-NODE-36`) and makes a fork-choice win a durable adoption **only after** the complete replacement branch is fetched, bound, linked, and ledger-validated. **Prevalidate-then-commit.** The current durable chain is byte-unchanged until proof completes (FC-6). RED fetch + GREEN sequencing + BLUE-reused; **no new BLUE authority.**

## 2. Slice Header
- **Slice Name:** Fork-switch apply — prove the selected replacement branch in full, then adopt via the existing rollback + roll-forward authorities.
- **Cluster:** PHASE4-N-AO — live multi-candidate fork-choice SELECT + adopt (rung-2).
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-AO-4** (`DC-NODE-37` never-abandon-until-validated) — on a forking win the rollback is **not** committed until the replacement branch is fetched + bound + linked + ledger-prevalidated; an invalid / lying / incomplete winner leaves `ChainDb` / `LedgerState` / `PraosChainDepState` **byte-unchanged**; a valid winner adopts via `RolledBack(fork_anchor)+ChainSelected(body)×N` with `RollbackReason::ForkChoiceWin`; the fork anchor binds the durable **stored** slot+hash (`DC-NODE-29`). New gate `ci/ci_check_fork_switch_never_abandons.sh` + reused `ci_check_rollback_target_canonical_binding.sh` + `ci_check_live_fork_choice_apply.sh` green.
- **Slice Dependencies:** S1 (`DC-NODE-34`, `31efec44`), S2 (`DC-NODE-35`, `6bcfc9e5`), S3 (`DC-NODE-36`, `a8c12327`).

## 4. Intent
Make a fork-choice win a **durable adoption only by proof of the complete replacement branch**, and make that proof **strictly precede** the irreversible `commit_rollback`. Strengthens `DC-NODE-37`. **Hard line (FC-6, the proof center):** Ade never abandons its current durable chain until the replacement branch's bodies are fetched, bound to the S3-selected headers, linked from the durable fork anchor, and ledger-validated as a complete branch. A failed proof leaves the current chain byte-identical.

## 5. Scope
- **Modules / crates:**
  - RED `ade_node::node_lifecycle` — a new fork-switch apply driver invoked by the relay loop **after `run_participant_sync` returns** when `act.pending_fork_switch.is_some()`; consumes the `PendingForkSwitch`; owns the (hermetic) body fetch + the prove→commit sequencing + the structured outcome.
  - RED `ade_node` — a `BranchBodySource` seam (trait): `fetch_body(peer, point) -> Result<Vec<u8>, FetchError>`. **S4 proves the fork-switch safety core against a `BranchBodySource` seam. Live `BlockFetch` `RequestRange` wiring is out of S4 scope and must not weaken the prevalidate-before-commit contract** — the live anchor→tip mux fetch rides with the operator pass (CE-AO-6) or a small operator-wiring slice after the S4 safety core is green, mirroring S1–S3's live-deferral.
  - GREEN `ade_node` — `prevalidate_branch`: the deterministic prove fold (body↔header bind + parent-link + `block_validity` ledger fold) producing a `ProvenBranch` (or a typed `BranchProofError`).
- **Reused (no change):** BLUE `block_validity` (per-block ledger validate), `materialize_rolled_back_state` (`CN-STORE-07`, read-only for the prove fold's start state), `commit_rollback`, `pump_block` (`DC-NODE-05/12`) — all **via the existing `apply_chain_event` `RolledBack`+`ChainSelected` arms** (`DC-NODE-25`); `decode_block` + `block_body_hash` (header re-derivation + body-hash).
- **State machines affected:** the relay loop gains an apply step that drains `pending_fork_switch`; on a proven win the durable spine rolls back to the anchor and rolls forward the proven bodies, and `pending_reselection` + `pending_fork_switch` clear **only after** reconcile (`DC-NODE-26`); on a proof failure the decision is retired as failed and **the forge fence is held** (see §9).
- **Persistence impact:** on a **proven** win — one `WalEntry::RollBack{reason: ForkChoiceWin, …}` + the rolled-forward bodies (the existing `apply_chain_event` durable effects). On **any** proof failure — **none**.
- **Out of scope (→ S5):** byte-identical **replay-equivalence** of a `ForkChoiceWin` reselection (CE-AO-5); the multi-peer replay corpus; the forge-fence *resolution* path that re-enables forging after an unresolved disagreement. (→ CE-AO-6): the live mux `RequestRange` fetch + the two-producer operator transcript that flips `CN-CONS-03`.

## 6. Execution Boundary (TCB color)
- **BLUE (reused, unchanged):** `block_validity`, `materialize_rolled_back_state`, `commit_rollback`, `pump_block`, `decode_block` / `block_body_hash`. **No new BLUE.**
- **GREEN:** `prevalidate_branch` — the body↔header binding, parent-link proof, and `block_validity` fold over the fetched bodies from the read-only materialized anchor (deterministic glue; no I/O, no durable mutation).
- **RED:** the fork-switch apply driver (relay-loop hook, prove→commit sequencing, the `BranchBodySource` fetch shell, the structured outcome + fence handling). The durable mutation itself is delegated to `apply_chain_event` (the existing RED composition, `DC-NODE-25`).

## 7. Invariants Preserved (registry IDs)
`DC-NODE-34/35/36` (consumed, not weakened); **`DC-NODE-25`** (apply authority — the sole durable-mutation path; S4 calls it, never re-implements rollback/admit); **`DC-NODE-29`** (the fork anchor is the durable **stored** slot+hash from S3's `PendingForkSwitch`, never peer-supplied); **`CN-STORE-07`** (materialize — reused read-only in the prove fold AND inside `apply_chain_event`); **`DC-CONS-20`** (lockstep commit); **`DC-NODE-05/12`** (`pump_block` sole roll-forward admit — no header-only tip advance); **`DC-NODE-28`** (no forge across the pending reselection — held set across the whole apply, and **held on proof failure**, never cleared as a side effect of an unproven branch); **`DC-NODE-26`** (reconcile — durable tip must equal the selected tip); **`DC-CONS-03`** (`select_best_chain` — not re-run here; S4 acts on its decision). FC-2 (a competing candidate is decided over the set, not adopt-then-reselect — **OQ-AO-3 resolved: buffer-and-decide**, then prove-and-apply).

## 8. Invariants Strengthened / Introduced
- **Strengthens toward enforced** `DC-NODE-37` (fork-switch never-abandon): a winning fork is durably adopted **only** after its complete branch is fetched + bound + linked + ledger-prevalidated, and the irreversible `commit_rollback` is **gated on** that proof; any proof failure leaves `ChainDb`/`LedgerState`/`PraosChainDepState` byte-unchanged and does not re-enable forging on the old chain. **One invariant family:** prevalidate-before-commit fork-switch apply. `DC-NODE-37` flips `declared → enforced` at `/cluster-close`.

## 9. Design Summary — the security choice, stated explicitly

**S4 prevalidates the complete replacement branch BEFORE `commit_rollback`. It does NOT commit-then-repair.**

On `act.pending_fork_switch = Some(PendingForkSwitch { fork_anchor, winning_peer, winning_candidate })`:

1. **Fetch** every body of `winning_candidate.headers` (anchor→tip) from `winning_peer` via `BranchBodySource`. A missing/short fetch → `BranchProofError::BodyUnavailable` → **abort, no commit**.
2. **Bind** each fetched body to its S3-selected header: `decode_block` → re-derive `header_input`; require field-equality with the selected `ValidatedHeaderSummary` (slot, block_no, body_hash, issuer_pool, op_cert_counter, vrf_leader_output) **and** `computed_body_hash == header.body_hash`. (S3's summary carries no block hash — S4 **re-derives** it, trusting nothing peer-asserted.) Mismatch → `BranchProofError::BodyHeaderMismatch` → **abort, no commit**.
3. **Link** the branch: `body[0].prev_hash == fork_anchor.hash`; `body[i].prev_hash == recompute_hash(body[i-1])`. Break → `BranchProofError::BrokenParentLink` → **abort, no commit**.
4. **Ledger-prevalidate (the load-bearing step):** read-only `materialize_rolled_back_state(fork_anchor)` → `(ledger₀, chain_dep₀)`; **fold `block_validity`** over the bodies. Any non-`Valid` verdict → `BranchProofError::BodyInvalid{index}` → **abort, no commit**. This is the dry-run that makes a post-`commit_rollback` `pump_block` failure impossible except crash.
5. **Only now adopt** via the existing authorities: `apply_chain_event(RolledBack{ to_point: fork_anchor, .. }, reason = ForkChoiceWin)` (materialize + `commit_rollback` + `WalEntry::RollBack{ForkChoiceWin}`), then `apply_chain_event(ChainSelected{ new_tip }, roll_forward_block = body)` for each body in order. `pump_block` re-validates each body (now guaranteed valid).
6. **Reconcile** (`DC-NODE-26`): the durable tip must equal the selected tip. **Then** clear `pending_fork_switch` + `pending_reselection`. A reconcile failure fails closed (the existing `apply_chain_event` reconcile error).

**Proof-failure / fence discipline (the no-silent-resume rule).** On proof failure, **no durable mutation occurs**. The pending decision is **retired as failed in a structured way** (a typed `ForkSwitchOutcome::ProofFailed{error}` + an in-memory `last_fork_switch_failure` observation surface — never a silent drop), so the driver does not re-attempt the same dead branch and returns to normal receive/selection. **The `pending_reselection` forge fence may clear only after the participant loop has returned to a resolved no-pending-reselection state — it MUST NOT clear as a side effect of an unproven branch.** A missing/lying body means the selected branch was *not* proven; resuming forging on the old chain while the node holds an unresolved peer disagreement is the hole this rule closes. (The fence's eventual *resolution*-driven clearing is S5 / a follow-on, not an S4 failure side-effect.)

**Too-deep:** step 5's `materialize` inside `apply_chain_event` keeps the **independent** `RollbackTooDeep` authority (`DC-CONS-05`) — S4 fails closed even if S3's `rollback_depth ≤ k` guard were bypassed. **k-authority is durable/config (S3's `ForgeActivation.security_param`), never peer.**

**Current coverage (no overclaiming).** S4's proof machinery supports multi-body branches, but **the current S3 live candidate producer exercises single-header candidates**, so today's "branch" is one block above the anchor. `prevalidate_branch` is written for N≥1 so it is correct when multi-block aggregation lands, but **multi-header candidate aggregation and long-branch live geometry remain follow-on coverage, not claimed by CE-AO-4.** This is current coverage, not a semantic rule.

## 10. Changes Introduced
- **Types:** RED `BranchBodySource` trait + `FetchError`; GREEN `ProvenBranch` (ordered validated bodies + the proven start/tip state) + closed `BranchProofError { BodyUnavailable, BodyHeaderMismatch, BrokenParentLink, BodyInvalid{index} }`; RED `ForkSwitchOutcome { Adopted{new_tip}, ProofFailed{error} }`. **No new BLUE / canonical / persisted type** (`RollbackReason::ForkChoiceWin` already exists).
- **State transitions:** the relay-loop apply step: `pending_fork_switch = Some(..)` → prove → (proven) adopt + reconcile + clear fence, or (proof failure) retire-as-failed, current chain unchanged, **fence held**.

## 11. Replay / Crash / Epoch Validation
- **Prove phase is read-only** → a crash before `commit_rollback` leaves durable state byte-identical (no half-switch).
- **Apply phase reuses `apply_chain_event`** (`WalEntry::RollBack{ForkChoiceWin}` + `pump_block`), inheriting the existing crash-safe WAL recovery (`DC-NODE-27`). S4 introduces **no** new durable mutation path.
- **Replay-equivalence of a `ForkChoiceWin` reselection is S5 (CE-AO-5)** — S4 asserts only the no-half-switch property + the durable WAL record shape.
- **Epoch boundary:** within the seed epoch; the materialized anchor chain_dep carries the eta0 basis (`T-REC-06`).
- Tests by name: `ade_node` integration over the S3 `live_fork_choice_ai_s4bii.rs` fixture family + the `corpus_durable_fork` helper, extended with a `BranchBodySource` double.

## 12. Mechanical Acceptance Criteria
- [ ] `fork_switch_win_adopts_via_rolledback_then_chainselected` — a proven branch → durable tip = selected tip; WAL contains exactly one `RollBack{ForkChoiceWin}`; `pending_fork_switch`/`pending_reselection` cleared **after** reconcile. *(happy path)*
- [ ] **Negative** `selected_peer_missing_body_leaves_chain_unchanged_fence_held` — fetch fails → no `commit_rollback`; ChainDb/WAL/ledger byte-unchanged; the decision is **retired as failed** (`ProofFailed{BodyUnavailable}` recorded, `pending_fork_switch` cleared); **`pending_reselection` (the forge fence) is STILL SET** — never cleared as a side effect of the unproven branch.
- [ ] **Negative** `body_hash_mismatch_leaves_chain_unchanged` — a body whose `computed_body_hash` ≠ the selected header → `BodyHeaderMismatch`, no commit, unchanged, fence held.
- [ ] **Negative** `broken_parent_link_leaves_chain_unchanged` — a body whose `prev_hash` breaks the anchor→tip chain → `BrokenParentLink`, no commit, unchanged, fence held.
- [ ] **Negative (THE critical one)** `invalid_body_rejected_before_commit_no_half_switch` — a branch with a body that would fail `pump_block` → the **prevalidation fold** rejects it **before** `commit_rollback` → ChainDb/ledger/chain_dep byte-unchanged (no half-switched durable state), fence held. *(Proves prevalidation gates the irreversible step.)*
- [ ] **Negative** `too_deep_rollback_fails_closed_unchanged` — a fork anchor beyond k/retention → `apply_chain_event`'s `materialize` `RollbackTooDeep` → fail closed, unchanged (S4's independent guard).
- [ ] New gate **`ci/ci_check_fork_switch_never_abandons.sh`**: in the apply driver, **every** `commit_rollback` / `apply_chain_event(RolledBack …)` is reachable **only after** a successful `prevalidate_branch` (no `commit_rollback` on any `BranchProofError` path); the fork anchor is the `PendingForkSwitch.fork_anchor` (durable-stored, `DC-NODE-29`); `RollbackReason::ForkChoiceWin` is the reason; the fence clears **after** reconcile on success and is **not** cleared on a proof-failure path. Reused `ci_check_rollback_target_canonical_binding.sh` + `ci_check_live_fork_choice_apply.sh` green.
- [ ] `cargo test -p ade_node` green.

## 13. Failure Modes (all → current chain byte-unchanged, no commit, fence held)
Body unavailable / short branch → `BodyUnavailable`. Body≠selected header → `BodyHeaderMismatch`. Broken link → `BrokenParentLink`. Any body fails ledger validation → `BodyInvalid` (caught in the prove fold, before commit). Anchor too deep → `RollbackTooDeep` (in apply's materialize). A post-reconcile mismatch → the existing `apply_chain_event` `ReconciliationMismatch` (fail-fast). All deterministic. Every failure is **retired as a structured `ForkSwitchOutcome::ProofFailed`** — never a silent drop, never a fence-clear.

## 14. Hard Prohibitions
**Inherits all nine cluster hard lines.** **Slice-specific (the sharp boundary):**
- **No `commit_rollback` / `apply_chain_event(RolledBack…)` before `prevalidate_branch` returns success.** The `PendingForkSwitch` authorizes proof, not mutation.
- **No half-switched durable state** — a proof failure (incl. a body that would fail `pump_block`) must leave ChainDb/ledger/chain_dep byte-identical.
- **The `pending_reselection` forge fence MUST NOT clear as a side effect of an unproven branch** — only a resolved no-pending-reselection state (success-reconcile, or S5's resolution path) may clear it. No silent "failed winner, resume forging on the old chain."
- The fork anchor is **only** `PendingForkSwitch.fork_anchor` (S3's durable-stored point); never re-derived from peer data.
- Adoption is **only** via the existing `apply_chain_event` `RolledBack`+`ChainSelected` arms (`pump_block` sole admit) — **no** second rollback/admit implementation, no header-only tip advance.
- No new BLUE; no `select_best_chain` re-run (S4 acts on S3's decision).

## 15. Explicit Non-Goals
No replay-equivalence proof of the reselection (S5); no live mux `RequestRange` fetch / operator transcript (CE-AO-6); **no multi-header candidate aggregation or long-branch live geometry** (follow-on coverage, not claimed by CE-AO-4 — see §9 current coverage); no forge-fence *resolution* path (S5); no `CN-CONS-03` flip (CE-AO-6 at `/cluster-close`).

## 16. Completion Checklist
- [ ] Relay-loop apply step consumes `act.pending_fork_switch`; prove→commit ordering enforced.
- [ ] `prevalidate_branch` (fetch + bind + link + `block_validity` fold) gates `commit_rollback`; all 4 `BranchProofError` variants fail closed unchanged.
- [ ] Proven win adopts via `RolledBack(ForkChoiceWin)+ChainSelected×N`; reconcile; fence cleared last.
- [ ] Proof failure: structured `ProofFailed` retire, current chain unchanged, **fence held** (no silent resume).
- [ ] New gate + reused canonical-binding/apply gates green; `cargo test -p ade_node` green.
- [ ] No half-switched state on any proof failure (the critical negative test green); `DC-NODE-37` ready to flip at `/cluster-close`.
