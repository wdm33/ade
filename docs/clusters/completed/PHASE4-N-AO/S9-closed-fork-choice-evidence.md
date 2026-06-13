# Invariant Slice S9 — Closed fork-choice convergence evidence for CE-AO-6

> **The live SELECT path emits a CLOSED, sha-bound, observe-only evidence sequence proving candidate discovery, LCA resolution, selector decision, branch proof, fork-switch apply, and final agreement — WITHOUT becoming authority.**
>
> Slice of cluster PHASE4-N-AO. The 2026-06-12 diverged run proved the SELECT mechanism fires live (both peers deliver; S7 LCA walk resolves forks of depth 1→7; 7 fork-choice WINs; `agreed` exact-hash; 0 diverged; no fail-close) — but the proof of the SELECT *middle* was **stderr diagnostics**, not registry-grade evidence. S9 makes the fork-choice path emit closed convergence events so a committed transcript ASSERTS the full chain. GREEN vocabulary + derivation + schema; RED sink; **BLUE unchanged**.

## 2. Slice Header
- **Slice Name:** Closed fork-choice evidence events (candidate discovery → LCA → selection → branch proof → fork-switch apply) + the `fork_switch_id`-paired S4-apply acceptance.
- **Cluster:** PHASE4-N-AO.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-AO-10** (`DC-EVIDENCE-04` closed fork-choice evidence) — the live SELECT path emits the closed event sequence; the vocabulary == its emit-only allow-list (negative test rejects unknown variants); every event carries bounded typed fields (no free-form error strings; `failure_code` is a closed enum); for a given `fork_switch_id`, a `fork_choice_selected{win}` is followed by EXACTLY ONE terminal event (`fork_switch_applied`, `fork_switch_failed`, OR `fork_switch_superseded` — the last when a newer win on the same fork overwrites this provisional pending before apply); evidence is observe-only (never read by BLUE/selection/apply). New gate `ci/ci_check_fork_choice_evidence_closed.sh` + hermetic emit/allow-list/pairing tests + `cargo test -p ade_node` green. **Unblocks the CE-AO-6 flip** (a committed two-producer transcript can now assert the full SELECT path).
- **Slice Dependencies:** S3–S7 (`DC-NODE-36..38`); the convergence-evidence surface (`DC-EVIDENCE-01..03`); the per-block-peer fix (`6846d252`).

## 4. Intent
Promote the (now-proven) live SELECT path from stderr diagnostics to **closed, auditable convergence evidence**. Introduces `DC-EVIDENCE-04`. The evidence **observes** already-computed authority outcomes; it must never feed back into selection, validation, rollback, or fence-clearing.

## 5. Scope
- **GREEN `ade_node::convergence_evidence` + the `AdmissionLogEvent` vocabulary:** add 9 closed events — `needs_fork_choice`, `lca_discovered`, `candidate_fragment_built`, `fork_choice_selected`, `branch_fetch_started`, `branch_fetch_completed`, `branch_prevalidated`, `fork_switch_applied`, `fork_switch_failed` — each with a unique discriminator, an emit-only allow-list entry, and bounded typed fields. A closed `ForkChoiceEvidenceFailure` enum maps `BranchProofError`/`LcaError` → a `failure_code` (NO `String`). A bounded deterministic `fork_switch_id` correlates one decide→apply cycle.
- **GREEN `ConvergenceEvidence::emit_*` methods** for each new event, derived purely from the authority outcomes the fork-choice path already computed.
- **RED emit sites (thread the existing `Option<&mut ConvergenceEvidence>`):** `dispatch_competing_fork_choice` emits `needs_fork_choice` → `lca_discovered` → `candidate_fragment_built` → `fork_choice_selected`; the relay-loop apply path (`prefetch_branch_bodies` + `apply_fork_switch`, node_lifecycle ~1407–1436) emits `branch_fetch_started/completed` → `branch_prevalidated` → `fork_switch_applied | fork_switch_failed`; the dispatch emits `fork_switch_superseded` for a provisional pending overwritten by a newer win on the same fork.
- **RED:** the JSONL sink + transcript file writing (REUSED, unchanged shape).
- **Out of scope:** ANY BLUE / selector / S7 / S4 logic change (S9 only OBSERVES). The `CN-CONS-03` flip itself (it flips only when a committed transcript proves the full path — the live CE-AO-6, gated on this slice + a clean run).

## 6. Execution Boundary (TCB color)
- **GREEN:** the closed event vocabulary + the emit-only allow-list + the schema/field derivation from authority outcomes + the `failure_code` + `fork_switch_id` closed/bounded types.
- **RED:** the emit-site wiring (threading the evidence sink) + the JSONL sink/file writing.
- **BLUE:** unchanged — `select_best_chain`, `walk_to_durable_lca`, `build_candidate_fragment`, `apply_fork_switch`, validate/apply.

## 7. Invariants Preserved (registry IDs)
`DC-EVIDENCE-01..03` (convergence evidence is GREEN, observe-only, never authority; per-event transcript discipline); `DC-NODE-34` (peer identity per event — just fixed); `DC-NODE-36/37/38` (the selector/apply/LCA authority S9 only observes); `CN-CONS-01` (arrival-order independence — evidence never affects selection).

## 8. Invariants Strengthened / Introduced
- **Introduces `DC-EVIDENCE-04`** (declared): *Closed fork-choice convergence evidence.* The live SELECT path emits a closed, observe-only event sequence (`needs_fork_choice` → `lca_discovered` → `candidate_fragment_built` → `fork_choice_selected` → `branch_fetch_*` → `branch_prevalidated` → `fork_switch_applied | fork_switch_failed`; the dispatch emits `fork_switch_superseded` for a provisional pending overwritten by a newer win on the same fork) proving the full path. The vocabulary equals its emit-only allow-list (unknown variants fail closed); every field is bounded + typed (`failure_code` a closed enum, `fork_switch_id` a bounded deterministic id, no free-form strings); for a given `fork_switch_id`, a `fork_choice_selected{result=win}` is followed by EXACTLY ONE terminal event (`fork_switch_applied` or `fork_switch_failed{closed_code}`); NO evidence event is ever consumed by BLUE / selection / apply / fence logic. Flips `declared → enforced` at `/cluster-close`.

## 9. Design Summary
The fork-choice path already computes every authority outcome S9 needs; S9 adds an **observe-only tap** at each decision point.

**Decide half** (`dispatch_competing_fork_choice`, which now receives the evidence sink): on entry → `needs_fork_choice{peer, slot, block_hash}`; on `walk_to_durable_lca` OK → `lca_discovered{peer, fork_anchor_slot/hash, candidate_header_count}`; on `build_candidate_fragment` OK → `candidate_fragment_built{peer, anchor, header_count}`; on `decide_fork_switch` → `fork_choice_selected{fork_switch_id, peer, result=win|loss, winner_tip?, fingerprint}`.

**Apply half** (relay-loop, S4): `branch_fetch_started{fork_switch_id, peer, anchor, winner_tip}` → `branch_fetch_completed{fork_switch_id, peer, block_count}` → `branch_prevalidated{fork_switch_id, peer, block_count}` → on `ForkSwitchOutcome::Adopted` → `fork_switch_applied{fork_switch_id, peer, new_tip, rollback_reason=ForkChoiceWin}`, on `ProofFailed` → `fork_switch_failed{fork_switch_id, peer, failure_code}`.

The `fork_switch_id` is a **bounded deterministic** id derived from a canonical tuple already in `PendingForkSwitch` — `blake2b(winning_peer ‖ fork_anchor.slot ‖ fork_anchor.hash ‖ winner_tip.slot ‖ winner_tip.hash)`, hex prefix — NOT free-form text. It ties one WIN to its single terminal apply outcome, disambiguating multiple WINs in a run. Every event is derived from a value the authority already produced — no recomputation, no feedback.

## 10. Changes Introduced
- **Types:** 9 `AdmissionLogEvent` variants (closed discriminators) + a `ForkChoiceEvidenceFailure` closed enum (`BranchProofError`/`LcaError` → code) + a bounded `ForkSwitchId` (deterministic hex prefix). No new BLUE/canonical type.
- **State transitions:** none — observe-only taps; the fork-choice/apply control flow is byte-identical.

## 11. Replay / Crash / Epoch Validation
Evidence is non-authoritative (DC-EVIDENCE-01); replay equivalence of the durable post-state is unaffected. The transcript is an emitted artifact, not replay input. (S5's `ForkChoiceWin` WAL replay-equivalence is unchanged.)

## 12. Mechanical Acceptance Criteria
- [ ] `fork_choice_event_vocabulary_equals_allowlist` — the 9 new discriminators == the emit-only allow-list exactly; an unknown/added variant fails closed (the `node_sched_event_allowlist_rejects_unknown_variants` pattern).
- [ ] `each_event_serializes_closed_typed_fields` — every event's JSON carries only the bounded typed fields; **no `failure_code` is a free-form string** (it is a closed-enum discriminant); `fork_switch_id` is a bounded hex id.
- [ ] `win_paired_with_exactly_one_terminal_by_fork_switch_id` — for a given `fork_switch_id`, a `fork_choice_selected{result=win}` is followed by EXACTLY ONE of `fork_switch_applied` / `fork_switch_failed`; never zero, never two, never dangling.
- [ ] `evidence_is_observe_only` (gate) — no new evidence event/type is read by `select_best_chain` / `walk_to_durable_lca` / `apply_fork_switch` / fence logic (a containment grep, like the N-Z independence gate).
- [ ] `lca_discovered_carries_anchor_and_header_count` + `fork_switch_applied_carries_new_tip_and_rollback_reason` — field-presence tests for the audit-critical events.
- [ ] New gate **`ci/ci_check_fork_choice_evidence_closed.sh`**: closed discriminants (no `Other`/`String` error), vocabulary==allow-list, observe-only (no BLUE consumer), `failure_code` an enum, `fork_switch_id` derived from the canonical tuple (no free-form text).
- [ ] `cargo test -p ade_node` green.
- [ ] **Live (CE-AO-6 flip, gated — NOT this slice):** a committed two-producer transcript passes the **bounded, branch-bound post-switch convergence window** (`ci/ci_check_post_switch_convergence_window.sh`, RELEASE/evidence-tier — no BLUE change). The **hard fork-switch proof** must precede the window: `block_received`(both peers) → `needs_fork_choice` → `lca_discovered` → `candidate_fragment_built` → `fork_choice_selected{win}` → `branch_fetch_*` → `branch_prevalidated` → `fork_switch_applied{rollback_reason=ForkChoiceWin}` at X (same `fork_switch_id`) → `block_admitted` X. Then a **bounded window** (fixed up front: `max_slots=200`, `max_admitted_blocks=20`) requires: no `diverged`; every fork-choice win has a terminal (`applied | failed | superseded`); every admitted block is X or a descendant (slot ≥ X, strictly forward — no rollback below X); `agreement_verdict{agreed, our_hash==peer_hash}` at X **or a descendant Y** in-window. This proves Ade switched to the selected branch, **stayed on it, did not diverge, and reached exact agreement** — without a lucky exact-tip moment and without freezing the venue. **Refined by S10 (DC-EVIDENCE-05):** the in-window terminal is the replayable `PostSwitchContinuity::ContinuesSelectedBranch` (unbroken `prev_hash` lineage from X) with `agreement_verdict{agreed,our==peer}` at X-or-descendant **OR** a validated-prefix-of-peer (continuity holds + peer observed ahead) — exact-tip agreement is sufficient but no longer necessary, since a clean healthy follower emits only `lagging`. ONLY then does `CN-CONS-03` flip.

## 13. Failure Modes
A write failure flips `incomplete` (DC-EVIDENCE-01, unchanged) — never disrupts authority. A fork-switch proof failure emits `fork_switch_failed{closed_code}` (not a string, not silence). An unknown event variant fails closed at the allow-list test.

## 14. Hard Prohibitions
- **No stderr-only evidence for a registry flip** — the SELECT middle must be in the committed closed-vocabulary transcript.
- **No open-vocabulary event names** — closed discriminants == allow-list.
- **No formatted-string errors as evidence** — `failure_code` is a closed enum.
- **No free-form `fork_switch_id`** — it is a bounded deterministic id derived from the canonical `peer + fork_anchor + winner_tip` tuple.
- **No evidence event consumed by BLUE / selection / apply / fence logic** — observe-only; a write failure cannot alter selection/apply/fence behavior.
- **No `CN-CONS-03` flip** unless the transcript includes the SELECT middle AND the S4 apply result (`fork_switch_applied`, paired by `fork_switch_id`).

## 15. Explicit Non-Goals
No BLUE/selector/S7/S4 change; no `CN-CONS-03` flip in this slice; no new authority; evidence stays observe-only.

## 16. Completion Checklist
- [ ] 9 closed events + `ForkChoiceEvidenceFailure` enum + bounded `ForkSwitchId` + emit-only allow-list (+ negative test).
- [ ] Emit sites wired in `dispatch_competing_fork_choice` (decide) + the relay-loop apply (apply), observe-only.
- [ ] `fork_switch_id`-paired `win ⇒ applied|failed` test; observe-only gate; field tests.
- [ ] `ci_check_fork_choice_evidence_closed.sh` + `cargo test -p ade_node` green.
- [ ] `DC-EVIDENCE-04` declared; ready to flip at `/cluster-close`; CE-AO-6 flip unblocked.
