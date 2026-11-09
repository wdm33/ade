# Invariant Slice AJ-S3 — Runbook correction + DC-EVIDENCE-03 transcript-shape rule

> Slice of PHASE4-N-AJ — the **close-out** slice. Docs + registry only: correct the
> now-inaccurate convergence runbook to the `--convergence-evidence-path` flow AJ-S1/S2 landed,
> and tie `DC-EVIDENCE-03` to the existing gate. No code change.

## 2. Slice Header
- **Slice Name:** Convergence runbook correction + DC-EVIDENCE-03 tie-in.
- **Cluster:** PHASE4-N-AJ. **Status:** Merged.
- **Cluster Exit Criteria Addressed:** **CE-AJ-4** (transcript-shape rule + corrected runbook).
- **Dependencies:** AJ-S1 (`--convergence-evidence-path`), AJ-S2 (the emission).

## 4. Intent
Make the operator runbook **producible**: it currently instructs `--mode node --participant-venue`
+ "capture the verbatim JSONL live log", which is the sched/`forge_*` log — **not** a gate-valid
convergence transcript. Correct it to the dedicated `--convergence-evidence-path` sink (AJ-S1/S2),
and register `DC-EVIDENCE-03` against the existing gate as the mechanical transcript-shape contract.

## 5. Scope
- **Docs:** `docs/active/phase4-n-ai-convergence-runbook.md` — Step 2 uses
  `--convergence-evidence-path docs/evidence/phase4-n-ai-convergence-pass.jsonl` (the sink writes the
  closed `block_received`/`block_admitted`/`agreement_verdict` transcript directly; **not** `--log`);
  note the **keyed-but-unstaked (σ=0)** operator so Ade is a pure follower (no forge ⇒ no `diverged`).
- **Registry:** `docs/ade-invariant-registry.toml` — `DC-EVIDENCE-03` `ci_scripts +=
  ci/ci_check_convergence_evidence_schema.sh` (the gate that validates the committed transcript).
- **Out of scope:** any code; the live transcript itself (operator-produced, post-cluster); the
  `CN-CONS-03` / `DC-NODE-30` / `DC-EVIDENCE-03` status flips (`/cluster-close`).

## 6. Execution Boundary (TCB color)
- **BLUE / GREEN / RED:** none changed — docs + registry only. The mechanical backing is the
  existing GREEN gate `ci_check_convergence_evidence_schema.sh`.

## 7. Invariants Preserved
`DC-NODE-30` (AJ-S2, unchanged), `DC-ADMIT-04` (AJ-S1, unchanged), `CN-CONS-03` (untouched —
**stays `declared`**), the gate's vacuous-until-committed property.

## 8. Invariants Strengthened or Introduced
- **`DC-EVIDENCE-03`** — its `ci_scripts` now names `ci_check_convergence_evidence_schema.sh` (the
  transcript-shape gate). *Targeted for `enforced_scaffolding` at `/cluster-close`* (gate + corrected
  runbook in place; the real committed transcript is the follow-on operator pass). *One family:
  convergence-evidence transcript shape.*

## 9. Design Summary
Replace runbook Step 2's "capture the verbatim JSONL live log" with the `--convergence-evidence-path`
sink path; add a one-line note that the sched/`forge_*` `--log` is a SEPARATE file and is **not** the
convergence transcript, and that the operator runs keyed-but-unstaked (σ=0). Append the gate to
`DC-EVIDENCE-03.ci_scripts`. No new gate (the schema gate already exists + self-tests).

## 11. Replay, Crash, Epoch Validation
None — docs/registry. The transcript-shape mechanical check is `ci_check_convergence_evidence_schema.sh`
(closed vocab · 0 diverged · ≥1 strict slot regression · sha256-bound), self-tested.

## 12. Mechanical Acceptance Criteria
- [ ] `docs/active/phase4-n-ai-convergence-runbook.md` Step 2 names `--convergence-evidence-path`
  (not "verbatim JSONL live log").
- [ ] `DC-EVIDENCE-03.ci_scripts` contains `ci/ci_check_convergence_evidence_schema.sh`; registry
  parses as TOML.
- [ ] `bash ci/ci_check_convergence_evidence_schema.sh --self-test` green; the gate stays
  vacuous-until-committed (green with no transcript).
- [ ] `cargo test -p ade_node` green (no code change).

## 14. Hard Prohibitions
Inherited cluster Forbidden. Slice-specific: no code change; no `CN-CONS-03` flip; no claim of a
produced live transcript (the runbook describes how to produce it, the gate validates it once
committed); the runbook must not re-introduce the "verbatim node log" inaccuracy.

## 15. Explicit Non-Goals
The live operator pass; the status flips (`/cluster-close`); any multi-peer / ChainSel claim.

## 16. Completion Checklist
- [ ] Runbook corrected; `DC-EVIDENCE-03.ci_scripts` populated; TOML parses; schema self-test green;
  `cargo test -p ade_node` green; no code change.
