# Invariant Slice AI-S5 — Convergence evidence + operator pass

> Slice of PHASE4-N-AI — the **last implementation slice**. AI-S1…S4b-ii closed + pushed
> (`af51b3c8`). Ships the hermetic CE-AI-5 proof + the CE-AI-6 operator-pass harness (closed
> schema gate + runbook); the live transcript is operator-produced. Cluster-close (CE-AI-7) is
> the separate `/cluster-close` step after this.

## 2. Slice Header
- **Slice Name:** Arrival-order-independence proof (hermetic) + convergence-pass evidence harness
  (operator-gated).
- **Cluster:** PHASE4-N-AI. **Status:** Merged.
- **CE Addressed:** **CE-AI-5** (`CN-CONS-01` deterministic, arrival-order-independent — hermetic)
  + **CE-AI-6** (`CN-CONS-03` live convergence — operator-gated, derived-tier).
- **Dependencies:** the existing BLUE `select_best_chain` (DC-CONS-03) + the existing
  `AgreementVerdict` evidence vocabulary + AI-S1/S3 `WalEntry::RollBack`.

## 4. Intent
Make `select_best_chain`'s **arrival-order-independence** mechanically proven (CN-CONS-01: a fixed
candidate set yields the same fork-choice-maximal tip regardless of presentation order — the
determinism surface), and provide a **closed, non-overstating** harness so an operator can
demonstrate live convergence (CN-CONS-03) without the evidence ever claiming more than it shows.

## 5. Scope
- **Hermetic test (ade_core):** a permutation test over `select_best_chain` — **test-only, no
  production change.**
- **CI (`ci/`):** `ci_check_chain_selection_arrival_order_independent.sh` (CE-AI-5) +
  `ci_check_convergence_evidence_schema.sh` (CE-AI-6, vacuous-until-committed).
- **Docs:** the operator runbook for the convergence pass.
- **Evidence vocabulary — REUSE (decision, confirmed):** the existing `AgreementVerdict`
  (`Agreed`/`Diverged`) + the AI-S1/S3 `WalEntry::RollBack` records already capture convergence (a
  sustained `Agreed` run that includes a peer reorg-follow and re-converges). **No new evidence
  enum** — evidence compares observed tips; it does not decide chain validity and does not become a
  second consensus authority.
- **Out of scope:** cluster-close (CE-AI-7); multi-peer ChainSel; the operator transcript itself
  (operator-produced).

## 6. Execution Boundary (TCB color)
- **Hermetic test** over **BLUE** `select_best_chain` (DC-CONS-03 — exercised, not changed).
  **GREEN** schema-validation in the convergence gate (compares already-authoritative tips;
  asserts nothing it doesn't observe). **RED** operator pass (the live run — operator-executed).
  **No new BLUE / GREEN / RED production code.**

## 7. Invariants Preserved (registry IDs)
`DC-CONS-03` (`select_best_chain` — tested, byte-unchanged), `CN-CONS-01` (now proven, not
weakened), `CN-CONS-03` (the convergence claim — scoped honestly), the existing `AgreementVerdict`
vocabulary (`[[feedback-evidence-reducers-are-green-not-authority]]` — reused, not extended),
`[[feedback-shell-must-not-overstate-semantic-truth]]` (the gate never overstates).

## 8. Invariants Strengthened / Introduced
- **`CN-CONS-01` partial → enforced** (the permutation test mechanically proves
  arrival-order-independence). *(flip recorded at cluster-close)*
- **`CN-CONS-03` — SCOPED enforcement, decided at `/cluster-close` against the exact registry
  wording (do NOT over-flip):** this proves convergence only for the **exercised single-best-peer
  competing-producer venue** via the reorg-follow path. At close, read `CN-CONS-03`'s registry
  statement: **if it is narrowly scoped to this venue, flip declared → enforced; if it is broad
  (full multi-peer candidate comparison), DO NOT flip — mark it `strengthened` and introduce a
  narrower venue-scoped enforced rule** (e.g. "single-best-peer competing-producer convergence").
  Full multi-peer candidate comparison remains out of scope (cluster hard line 8).
- One family: chain-selection convergence (determinism + live agreement). **Honesty:** CE-AI-6
  proves the exercised venue, NOT full multi-peer Cardano ChainSel; AI-S4b-ii proved
  single-best-peer rollback *following*, not multi-candidate live selection.

## 9. Design Summary
**CE-AI-5 (hermetic permutation):** build a fixed set of `CandidateFragment`s with synthetic
`ValidatedHeaderSummary`s (distinct `block_no` + tiebreaker so there is one unambiguous maximal
chain, plus a tiebreaker-decided pair at equal height). For **every permutation** of the candidate
slice, `select_best_chain(state, &perm)` returns the **same** winning tip; and the orchestrator
variant — feeding the same headers as `process_stream_input(HeaderArrival)` in permuted orders —
converges to the **same** `selector.current_tip`. Hermetic, deterministic; no real blocks
(selection compares height + tiebreaker, not bodies).

**CE-AI-6 (operator harness, reuse the verdict vocabulary):**
- The convergence transcript is the **existing** live JSONL (`AgreementVerdict` `Agreed`/`Diverged`
  + the `WalEntry::RollBack` reorg-follow records) — committed by the operator at
  `docs/evidence/phase4-n-ai-convergence-pass.{md,jsonl}` (`.md` = the manifest binding the
  `.jsonl` sha256).
- **`ci/ci_check_convergence_evidence_schema.sh`** (here-strings): **vacuous-until-committed** —
  passes when the `.jsonl` is absent; when present it validates **closed vocabulary** (only known
  verdict / rollback event tags — allow-list), **sha256-binding** (`.md` manifest hash ==
  `.jsonl`), and the **convergence-through-reorg assertion**: 0 `Diverged`, **≥1 `RollBack`-follow**
  (the peer reorg was actually followed — a final `Agreed` during a boring run is NOT sufficient),
  final verdict `Agreed` at the same tip. It never treats a weaker signal (lagging, self-accept)
  as convergence. The gate is **parameterized** by transcript path so a hermetic test drives it
  with temp fixtures.
- **Runbook** (`docs/active/phase4-n-ai-convergence-runbook.md`): run `ade_node --mode node
  --participant-venue` + ≥1 Haskell producer on a competing-producer venue (per
  `docs/active/c2-preprod-tip-guide.md`), capture the verbatim JSONL, confirm convergence (same
  tip, 0 diverged, a followed reorg), commit the transcript + manifest.

## 10. Changes Introduced
A hermetic permutation test (`crates/ade_core/tests/chain_selection_arrival_order_ai_s5.rs`); the
two gates; the runbook doc; temp-fixture tests for the convergence gate (valid → pass;
diverged / malformed / unknown-tag / sha256-mismatch / **no-rollback-follow** → fail). **No
production code change** (reuse `select_best_chain` + `AgreementVerdict`).

## 11. Replay / Crash / Epoch
Arrival-order-independence **is** the determinism property (CN-CONS-01) — the permutation test is
the determinism proof. No durable / authoritative state introduced; no replay-corpus change.

## 12. Mechanical Acceptance Criteria
- [ ] `select_best_chain_arrival_order_independent` — all permutations of a fixed candidate set →
  the same maximal tip (incl. a tiebreaker-decided equal-height pair).
- [ ] `orchestrator_header_arrival_order_independent` — permuted `HeaderArrival` sequences → the
  same `selector.current_tip`.
- [ ] New gate **`ci/ci_check_chain_selection_arrival_order_independent.sh`** (the test exists +
  runs; `select_best_chain` unchanged).
- [ ] New gate **`ci/ci_check_convergence_evidence_schema.sh`**: vacuous-until-committed (green
  with no transcript).
- [ ] `convergence_gate_accepts_valid_reorg_follow_fixture` — a valid transcript (0 diverged, ≥1
  RollBack-follow, final Agreed, sha256-bound) → the gate passes.
- [ ] `convergence_gate_rejects_diverged_fixture`, `..._rejects_unknown_tag_fixture`,
  `..._rejects_sha256_mismatch_fixture` → the gate fails closed.
- [ ] `convergence_gate_rejects_transcript_without_rollback_follow` — a transcript with final
  `Agreed` but **no `RollBack`-follow** event → the gate fails closed (CE-AI-6 proves convergence
  *through the reorg-follow path*, not a boring same-tip run).
- [ ] `cargo test -p ade_core` green (the permutation test).

## 13. Failure Modes
The permutation test is deterministic (a failure would be a real DC-CONS-03 order-dependence
regression). The convergence gate fails closed on a present-but-invalid transcript (diverged /
unknown tag / sha256 mismatch / no reorg-follow); vacuous (pass) when absent.

## 14. Hard Prohibitions
No new BLUE; `select_best_chain` byte-unchanged; no new evidence enum (reuse `AgreementVerdict`);
the convergence gate must be **vacuous-until-committed** (never blocks CI before the operator
commits); no overstating (lagging / self-accept ≠ convergence; a boring same-tip run ≠ CE-AI-6;
convergence evidence is GREEN, compares tips, never an authority); **no claim of full multi-peer
CN-CONS-03 / ChainSel** beyond the exercised venue; no `RO-LIVE` flip beyond the operator-gated
transcript; do NOT over-flip `CN-CONS-03` at close (scope or strengthen per its registry wording).

## 15. Explicit Non-Goals
Cluster-close (CE-AI-7 — the declared→enforced flips, strengthenings, grounding-docs refresh,
archive — is the separate `/cluster-close`); multi-peer candidate comparison; the operator
transcript itself (operator-produced).

## 16. Completion Checklist
- [ ] Permutation test (both variants) + `ci_check_chain_selection_arrival_order_independent.sh`
  green.
- [ ] `ci_check_convergence_evidence_schema.sh` vacuous-green + fixture tests (accept valid;
  reject diverged / malformed / unknown-tag / sha256-mismatch / no-rollback-follow) green.
- [ ] Operator runbook committed; honesty narrowing stated; the scoped-CN-CONS-03 close note
  recorded.
- [ ] `cargo test -p ade_core` green; no production code change; no full-ChainSel claim.
