# Invariant Cluster — PHASE4-N-AJ — Participant-path Convergence Evidence Emission

> **Status:** Planning Artifact (Non-Normative). Organizes/sequences work; introduces no
> requirement beyond the cited registry rules. Normative sources: `docs/ade-invariant-registry.toml`
> + the invariants sketch. Per-cluster doc — in-flight under `docs/clusters/PHASE4-N-AJ/`.

## Primary invariant

> The live participant rollback-follow path emits the **existing** closed `AgreementVerdict`
> evidence vocabulary to a **dedicated, opt-in** `--convergence-evidence-path` sink as a
> deterministic **GREEN side-output** of already-authoritative outcomes — **never becoming
> authority** — so one participant run through a peer reorg yields a transcript the existing
> convergence gate accepts; **absent the sink, node behavior is byte-unchanged.**

Registry: **DC-NODE-30** (new), **DC-EVIDENCE-03** (new), **DC-ADMIT-04** (strengthened). Reused
unchanged: **DC-NODE-23…29** (rollback-follow), **DC-CONS-03** (fork-choice), the GREEN-evidence
discipline (**DC-ADMIT-08** / `feedback-evidence-reducers-are-green-not-authority`).
**CN-CONS-03 untouched — stays `declared`.**

## Normative anchors

- `docs/planning/phase4-n-aj-participant-convergence-evidence-invariants.md` — I-AJ-1…7, ¬AJ-1…6, decisions D-1/D-2/D-3.
- `docs/planning/phase4-n-aj-cluster-slice-plan.md` — the ordered slice plan.
- `docs/ade-invariant-registry.toml` — DC-NODE-30, DC-EVIDENCE-03, DC-ADMIT-04, DC-EVIDENCE-01/02, DC-NODE-23…29, DC-CONS-03, CN-CONS-03.
- `ci/ci_check_convergence_evidence_schema.sh` (PHASE4-N-AI AI-S5, existing gate).
- Memory: `feedback-evidence-reducers-are-green-not-authority`, `feedback-shell-must-not-overstate-semantic-truth`.

## Entry Conditions (guaranteed by prior clusters)

- **PHASE4-N-AI:** participant rollback-follow exists + proven (DC-NODE-23…29 enforced);
  `run_participant_sync` follows a peer `RollBackward` via `apply_chain_event`; `pump_block` is the
  sole roll-forward admit.
- **PHASE4-N-M-B:** GREEN `verdict::derive` + the closed `AdmissionLogEvent` vocabulary + writer
  exist + enforced (DC-ADMIT-04, DC-EVIDENCE-01/02); admission mode emits them.
- **AI-S5:** `ci_check_convergence_evidence_schema.sh` exists (vacuous-until-committed) + self-tests.
- **The split AJ bridges (confirmed in code):** admission emits the vocabulary but ignores rollbacks
  (`admission/bootstrap.rs:377` `RollBackward => continue`); the participant path follows rollbacks
  but emits no verdict.

## Exit Criteria (CI-Verifiable — named checks, not intent)

- [ ] **CE-AJ-1** (dedicated/opt-in/isolated sink): NEW
  `ci/ci_check_convergence_evidence_vocabulary_closed.sh` enforces the convergence-evidence file
  carries only its declared closed vocabulary **and** bidirectional isolation vs.
  wire-only/admission/sched literals; Rust tests `convergence_evidence_absent_path_emits_no_file` +
  `convergence_evidence_writer_emits_closed_vocabulary` pass. *(Targets DC-ADMIT-04 strengthened.)*
- [ ] **CE-AJ-2** (participant emits convergence evidence): Rust test
  `participant_convergence_run_through_reorg_is_gate_valid` — an `InMemory` participant run
  (`Block→RollBack→Block`) writes a `--convergence-evidence-path` JSONL satisfying all four
  `ci_check_convergence_evidence_schema.sh` conditions (closed vocab · 0 diverged · ≥1 strict slot
  regression in the observed peer block sequence · final `agreed`); the existing gate stays green.
  *(Targets DC-NODE-30 enforced.)*
- [ ] **CE-AJ-3** (evidence-only guards): NEW `ci/ci_check_convergence_evidence_emit_only.sh`
  (here-strings) asserts the verdict/emit result never feeds
  `classify_receive`/`apply_chain_event`/`pump_block`/forge control flow and the sink is distinct
  from the WAL append; Rust tests `participant_diverged_verdict_is_emit_only_no_halt` +
  `participant_convergence_write_failure_non_fatal_to_authority` +
  `participant_convergence_evidence_replay_byte_identical` pass. *(Targets DC-NODE-30 guards.)*
- [ ] **CE-AJ-4** (transcript-shape rule + corrected runbook):
  `ci/ci_check_convergence_evidence_schema.sh --self-test` green (accept valid / reject diverged /
  reject no-regression / reject sha256-mismatch); `docs/active/phase4-n-ai-convergence-runbook.md`
  describes the `--convergence-evidence-path` participant-evidence flow (no longer "capture the
  verbatim node JSONL"). *(Targets DC-EVIDENCE-03 enforced_scaffolding.)*
- [ ] `cargo test -p ade_node` green; **no BLUE change** (FC/IS audit).

> **Not a CE (complete-work-only):** the live operator transcript + the `CN-CONS-03` flip — the
> follow-on operator pass AJ unblocks. **AJ enforces DC-NODE-30 + DC-EVIDENCE-03 scaffolding; AJ must
> not flip `CN-CONS-03`. `CN-CONS-03` flips only after the post-AJ operator transcript is committed
> and passes the gate.**

## Expected Slice Types

- **AJ-S1** — CLI flag + dedicated sink/writer (reused vocabulary) + new vocabulary-closed gate +
  DC-ADMIT-04 strengthening. *Inert (writer not yet fed by the participant path).*
- **AJ-S2** — emission wiring into `run_participant_sync` (the evidence flip) + hermetic gate-accept
  test + evidence-only guard tests + evidence replay-equivalence.
- **AJ-S3** — runbook correction + DC-EVIDENCE-03 registry tie-in.

## TCB Color Map (FC/IS Partition)

| Module | Color | Note |
|---|---|---|
| `ade_ledger` `pump_block`, `apply_chain_event`, ledger/WAL | **BLUE** | reused **unchanged** |
| `ade_node::admission::verdict::derive` | **GREEN** | reused unchanged |
| `ade_node::admission_log` (vocabulary + writer) | **GREEN** | reused against the new dedicated sink |
| new emission glue (what to emit) | **GREEN** | deterministic; compares already-authoritative outputs |
| `ade_node::node_lifecycle::run_participant_sync` | **RED** | the *when* — existing loop |
| `ade_node::cli` (`--convergence-evidence-path`) | **RED** | flag parse |
| the JSONL file sink | **RED** | I/O; emits GREEN-computed closed events |

## Forbidden during this cluster (slices inherit)

No BLUE change · no new evidence enum (reuse `AgreementVerdict`/`AdmissionLogEvent`) · no second
rollback-follow path (admission stays `RollBackward => continue`) · no change to
fork-choice/admission/rollback/WAL semantics (`pump_block` sole roll-forward admit;
`apply_chain_event` sole rollback authority; `classify_receive` unchanged) · evidence emit-only (a
verdict, incl. `Diverged`, never halts/mutates authority/triggers rollback) · a writer failure must
never corrupt state **and** never silently produce a partial transcript that later passes the gate
· convergence-evidence file never co-mingled with the sched/`forge_*` log · no hand-filtered
transcript · **no `CN-CONS-03` flip / no live-convergence claim in AJ**.

## Registry declarations (this cluster-doc appends as `declared`)

Full statements live in `docs/ade-invariant-registry.toml` (referenced by ID, not restated here):

- **DC-NODE-30** (derived, `declared`) — participant path emits convergence evidence as a GREEN
  side-output to `--convergence-evidence-path`; emit-only on `Diverged`; writer-failure non-fatal to
  authority but marks the transcript incomplete/unusable for CE-AI-6; absent path ⇒ byte-unchanged.
  **Targeted for enforcement at AJ close (hermetic).**
- **DC-EVIDENCE-03** (derived, `declared`) — convergence-through-reorg transcript shape (strict slot
  regression in the observed peer block sequence + ≥1 `agreed` + 0 `diverged` + sha256-bound);
  single-best-peer scope. **Targeted for enforced_scaffolding at AJ close.**
- **DC-ADMIT-04** (strengthen at AJ-S1) — closed-vocabulary isolation extended to name the
  convergence-evidence file as a third isolated closed-vocabulary file
  (`strengthened_in += PHASE4-N-AJ`).
