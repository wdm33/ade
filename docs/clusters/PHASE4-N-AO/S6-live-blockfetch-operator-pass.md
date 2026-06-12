# Invariant Slice S6 — Live BlockFetch bridge + two-producer operator pass (CE-AO-6)

> **The live fetch provides BYTES. It does not certify selection, does not certify validity, does not bypass `prevalidate_branch`, and does not clear the fence. S4 remains the authority. BlockFetch transports bytes; it does not grant truth.**
>
> The final, operator-gated slice of PHASE4-N-AO. Replaces `NullBranchBodySource` with a live `BlockFetch RequestRange` source feeding the **existing** S4 seam, then exercises a real two-producer competing fork end-to-end. **No new selector, apply, or rollback logic.** Flips `CN-CONS-03` **only** on the committed transcript.

## 2. Slice Header
- **Slice Name:** Live BlockFetch body bridge + two-producer convergence operator pass.
- **Cluster:** PHASE4-N-AO (rung-2 SELECT) — final slice.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-AO-6** (`CN-CONS-03` flip, operator-gated) — a real two-producer competing fork: `NeedsForkChoice` observed → `select_best_chain` winner → **live `BlockFetch` body retrieval** (not `NullBranchBodySource`) → `prevalidate_branch` success **before** `commit_rollback` → `RollBack{ForkChoiceWin}` WAL → `ChainSelected` bodies admitted → `agreement_verdict{agreed}`, `our_hash == peer_hash`, 0 diverged, 0 VrfCert deaths, 0 half-switch. Committed, sha256-bound evidence pair. `CN-CONS-03` flips `declared → enforced` **only** on this transcript.
- **Slice Dependencies:** S1–S5 (`DC-NODE-34..37`, `DC-NODE-27/28` ext).

## 4. Intent
Exercise the complete live SELECT path against a real peer with **live body fetch**, earning the `CN-CONS-03` flip. The live fetch is a strictly **weaker** input than S4: it transports bytes; it never certifies or bypasses. Preserves `DC-NODE-37` (prove-before-commit), `DC-NODE-35` (no minting), `DC-NODE-30` (evidence observes, never becomes, authority).

## 5. Scope
- **RED `ade_node` (new):** `PrefetchedBranchBodies` (impl `BranchBodySource`, a sync in-memory `(peer, slot) → bytes` map) + an **async** `prefetch_branch_bodies(peer_mux, fork_anchor, winning_candidate)` that issues `BlockFetchMessage::RequestRange(fork_anchor → winner_tip)` to the **winning peer's** mux and collects the body bytes. Reuses the existing `wire_pump` BlockFetch client + `serve_dispatch` RequestRange shapes.
- **RED `ade_node::node_lifecycle`:** in the relay loop, when `pending_fork_switch.is_some()` **and** the winning peer's mux is reachable → `prefetch_branch_bodies` → `apply_fork_switch(..., &prefetched, ...)`. `NullBranchBodySource` remains the fallback (no mux → fence held, as today).
- **GREEN `ade_node::convergence_evidence`:** a **closed** fork-switch evidence vocabulary extension (`needs_fork_choice` / `fork_choice_selected` / `branch_fetched` / `branch_prevalidated` / `fork_switch_applied`) — allow-listed + negative-tested (`CN-OPERATOR-EVIDENCE-01`); sha256-bound transcript.
- **Reused, UNCHANGED:** `apply_fork_switch` / `prove_fork_switch` / `prevalidate_branch` (S4); `select_best_chain` (S3); `replay_from_anchor` (S5). **No new selector/apply/rollback/BLUE.**
- **Operator-gated (the live run):** the two-producer venue (the CE-AI-6 venue extended to two competing producers) + the committed transcript pair. The hermetic **loopback proof** (a local served competing fork → fetch + select + apply + transcript) is in-scope + CI; the **real two-producer pass** is `blocked_until_operator_pass_executed` (the CE-AI-6 / CN-CONS-06 pattern) — user-gated.

## 6. Execution Boundary (TCB color)
- **RED:** `prefetch_branch_bodies` (mux I/O), `PrefetchedBranchBodies`, the relay integration. The fetch source emits **only bytes** — no verdict, no fence handle, no selection.
- **GREEN:** the closed evidence vocabulary (observes authority; never becomes it, `DC-NODE-30`).
- **BLUE / GREEN-reused, unchanged:** the entire S2–S5 mechanism.

## 7. Invariants Preserved (registry IDs)
`DC-NODE-37` (the fetch feeds `prove_fork_switch`; it never bypasses prevalidation or commits); `DC-NODE-35` (no minting — bytes only, validated by S4); `DC-NODE-36` (`select_best_chain` is the sole selector); `DC-NODE-28` (the fetch never clears the fence); `DC-NODE-30` + `CN-OPERATOR-EVIDENCE-01` (evidence is a closed observe-only vocabulary); `feedback_shell_must_not_overstate_semantic_truth` (wire success ≠ admission ≠ agreement).

## 8. Invariants Strengthened / Flipped
- **Flips `CN-CONS-03` `declared → enforced`** — live multi-candidate fork-choice SELECT, exercised against a real peer with live body fetch, recorded in the committed transcript. **Flip is gated on the transcript landing**, not on hermetic-green (S1–S5).

## 9. Design Summary
On a live `pending_fork_switch` (set by S3): the relay loop **pre-fetches** the winning branch's bodies — `RequestRange(fork_anchor.point → winner_tip.point)` to the **winning peer's** mux — into a `PrefetchedBranchBodies` source, then calls the **unchanged** `apply_fork_switch` with it. S4 then does exactly what it does hermetically: bind each fetched body to the S3-selected header, link, ledger-prevalidate the complete branch, and **only then** `commit_rollback` + `ChainSelected×N` + reconcile. The live fetch is **upstream of and weaker than** the proof — a lying / incomplete / truncated / Byzantine fetch is caught by `prove_fork_switch` / `prevalidate_branch` exactly as the hermetic `BranchBodySource` doubles proved (S4's negatives). On no reachable mux, `NullBranchBodySource` holds the fence (unchanged). The convergence transcript records each step as a closed evidence event, sha256-bound.

**The live edge:** the only new authority surface is "can we fetch the bytes"; everything that decides whether to *adopt* them is the existing S2–S5 mechanism.

## 10. Changes Introduced
- **Types:** `PrefetchedBranchBodies` (RED, in-memory `BranchBodySource`); a closed fork-switch evidence event extension (GREEN). No new BLUE/canonical/persisted type.
- **State transitions:** relay loop — `pending_fork_switch.is_some()` + reachable mux → prefetch → `apply_fork_switch`. No change to the apply itself.

## 11. Replay / Crash / Epoch Validation
No new durable state (the prefetched bodies are transient; durability is S4's `apply_chain_event` + S5's replay, unchanged). **The live pass is expected to exercise the seed epoch in the rung-2 venue. The transcript records successful admission and convergence; eta0 correctness remains mechanically covered by `T-REC-06` and the S4/S5 replay/admission tests, not inferred solely from the transcript.**

## 12. Mechanical Acceptance Criteria
- [ ] **Hermetic loopback** `live_fetch_loopback_selects_and_applies` — a local served competing fork: the relay prefetches via `RequestRange`, `apply_fork_switch` adopts, durable tip = winner, WAL has `RollBack{ForkChoiceWin}`, fence cleared. (`ade_node` integration; reuses the loopback-serve infra.)
- [ ] **Boundary** `live_fetch_lying_body_rejected_before_commit` — the live source serves a body that does not match the selected header (a wrong body for a present slot) → `prevalidate_branch` rejects it (`BodyHeaderMismatch`) **before** `commit_rollback`; chain unchanged, fence held.
- [ ] **Boundary** `live_fetch_short_range_rejected_before_commit` — `RequestRange` returns fewer bodies than the selected candidate's header count (mux/peer truncation) → the prove phase (`prove_fork_switch` fetch / `prevalidate_branch` length check) yields `BodyUnavailable` **before** `commit_rollback`; chain unchanged, fence held. **Distinct from the lying-body case** (a missing body, not a wrong one).
- [ ] New gate **`ci/ci_check_live_blockfetch_byte_only.sh`**: the live fetch source / prefetch path references **no** `select_best_chain` / `prevalidate_branch` / `commit_rollback` / `pending_reselection` / verdict — it returns bytes only; `apply_fork_switch` is still the sole adopter; `NullBranchBodySource` remains the no-mux fallback.
- [ ] **Closed evidence vocabulary** + negative test (an out-of-allow-list event fails) + sha256-binding (`CN-OPERATOR-EVIDENCE-01`).
- [ ] **Operator pass (gated):** committed `docs/evidence/phase4-n-ao-multiproducer-convergence.{md,jsonl}` with the §2 transcript claims: real two-producer fork · `NeedsForkChoice` · `select_best_chain` winner · live BlockFetch used (not Null) · `prevalidate_branch` success before `commit_rollback` · `RollBack{ForkChoiceWin}` WAL · `ChainSelected` admitted · `agreement_verdict{agreed}` · `our_hash == peer_hash` · 0 diverged · 0 VrfCert deaths · 0 half-switch · sha256-bound. **`CN-CONS-03` flips only here.**
- [ ] `cargo test -p ade_node` green.

## 13. Failure Modes
No reachable mux → fence held (no fetch, `NullBranchBodySource`). Lying / incomplete / truncated / Byzantine fetch → `prove_fork_switch` / `prevalidate_branch` rejects before commit (S4). Operator venue failure → close proceeds later with `CN-CONS-03` remaining `declared`/operator-gated + the carry-forward (not a hermetic regression).

## 14. Hard Prohibitions
- The live fetch source emits **bytes only** — it must NOT certify selection, certify validity, bypass `prevalidate_branch`, clear the fence, or mint a summary. (Gate-enforced. No new rule — this preserves `DC-NODE-35/37`.)
- `apply_fork_switch` remains the sole adopter; `prevalidate_branch` remains the sole gate on `commit_rollback`. No new selector/apply/rollback.
- **Do NOT flip `CN-CONS-03` on hermetic-green alone** — only on the committed two-producer transcript.
- Evidence is a closed observe-only vocabulary (no authority feedback).

## 15. Explicit Non-Goals
No new selection/apply/rollback/BLUE; no new rule (`DC-NODE-38` deliberately NOT declared — the byte-only boundary is covered by `DC-NODE-35/37/30`); no multi-header candidate aggregation (follow-on); no change to S2–S5; no durable body-staging (S5 policy).

## 16. Completion Checklist
- [ ] `PrefetchedBranchBodies` + `prefetch_branch_bodies` (byte-only) + relay integration; `NullBranchBodySource` fallback intact.
- [ ] Hermetic loopback proof + lying-body + short-range rejected-before-commit green; byte-only gate + closed-evidence negative test green.
- [ ] Operator two-producer pass executed; transcript pair committed + sha256-bound; **then** `CN-CONS-03` flipped.
- [ ] `cargo test -p ade_node` green.
