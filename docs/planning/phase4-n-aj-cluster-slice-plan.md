# Cluster/Slice Plan — Ade · PHASE4-N-AJ

> IDD cluster-planning artifact (overall plan only; full cluster doc is `/cluster-doc`).
> From the confirmed sketch `docs/planning/phase4-n-aj-participant-convergence-evidence-invariants.md`
> (resolved decisions D-1/D-2/D-3). Per-cluster plan — does **not** overwrite the global
> `docs/active/phase_4_cluster_plan.md`. Status verbs below are **targets** (the rules are
> `declared` at `/cluster-doc`; the status flips listed are what AJ-close aims to reach, not what
> has already happened).

## Cluster Index (Dependency Order)

1. **PHASE4-N-AJ** — Participant-path convergence evidence emission — primary invariant: *the live
   participant rollback-follow path emits the existing closed `AgreementVerdict` evidence
   vocabulary to a dedicated, opt-in convergence-evidence sink as a deterministic GREEN side-output
   of already-authoritative outcomes — never becoming authority — so one participant run through a
   peer reorg yields a gate-valid CE-AI-6 transcript; absent the sink, behavior is byte-unchanged.*

A single cluster. It is **evidence-only** — it does **not** reopen consensus behavior, rollback
logic, fork-choice, WAL semantics, or admission authority. The rollback-follow itself
(DC-NODE-23..29) already exists and is proven; AJ builds the missing **bridge** between two facts
that today cannot meet in one transcript:

- *participant mode can follow a rollback* (DC-NODE-23..29), and
- *the evidence gate can observe that convergence honestly* (`ci_check_convergence_evidence_schema.sh`).

It does **not** flip `CN-CONS-03`. AJ only makes the live convergence transcript **producible and
mechanically checkable**; the flip happens later, when the operator runs the (now-unblocked) live
pass and commits a real transcript.

## PHASE4-N-AJ — Participant-path convergence evidence emission

- **Primary invariant:** above.

- **TCB partition:**
  - **BLUE** — *none changed.* Reused unchanged: `ade_ledger` `pump_block` (sole roll-forward
    admit), `apply_chain_event` (sole rollback authority), ledger/WAL.
  - **GREEN** — `ade_node::admission::verdict::derive` (reused), `ade_node::admission_log` closed
    vocabulary + writer serialization (reused against a new sink), the new **emission glue** (what
    to emit after each authoritative receive/admit/rollback).
  - **RED** — `ade_node::node_lifecycle::run_participant_sync` (the *when* — existing loop),
    `ade_node::cli` (`--convergence-evidence-path`), the dedicated JSONL file sink.

- **Cluster Exit Criteria:**
  - **CE-AJ-1** — *Dedicated, opt-in, isolated sink.* `--convergence-evidence-path <path>` opens a
    dedicated convergence-evidence JSONL reusing the closed vocabulary; **no path ⇒ no file ⇒ node
    behavior byte-unchanged**; the file carries **only** its declared closed vocabulary (new
    `ci_check_convergence_evidence_vocabulary_closed.sh`), never co-mingled with the sched/`forge_*`
    log. *(Targets: DC-ADMIT-04 strengthened.)*
  - **CE-AJ-2** — *Participant emits convergence evidence.* On the participant path: **`BlockReceived`
    for each peer block considered by the participant receive path — before drop/admit/refuse** (so
    evidence is preserved on already-have and on fail-closed, and the strict slot regression is
    observable from peer *input*, not only from admitted blocks); `BlockAdmitted` per `pump_block`
    admit; `AgreementVerdict` = `verdict::derive(outcome, observed_peer_tip)` — each carrying
    `consensus_inputs_fingerprint_hex`. A hermetic `InMemory` run (`Block→RollBack→Block`) produces a
    JSONL that **`ci_check_convergence_evidence_schema.sh` accepts** (closed vocab · 0 diverged · ≥1
    strict slot regression in the observed peer block sequence · final `agreed`). *(Targets:
    DC-NODE-30 enforced.)*
  - **CE-AJ-3** — *Evidence-only guards.* A `Diverged` verdict is emitted but **does not** halt /
    mutate authority / trigger rollback (emit-only); a convergence-evidence **write failure is
    non-fatal to authority** (loop continues, no state corruption; the sink is distinct from the
    authoritative WAL); `pump_block` / `apply_chain_event` / `classify_receive` byte-unchanged.
    *(Targets: DC-NODE-30 guards.)*
  - **CE-AJ-4** — *Transcript-shape rule + corrected runbook.* `DC-EVIDENCE-03`
    (convergence-through-reorg transcript shape) registered and mechanically backed by the existing
    gate (vacuous-until-committed; self-test accepts valid / rejects diverged / rejects
    no-regression); `docs/active/phase4-n-ai-convergence-runbook.md` corrected to the
    `--convergence-evidence-path` participant-evidence flow. *(Targets: DC-EVIDENCE-03
    enforced_scaffolding.)*

  > **Not a CE (complete-work-only):** the *live* operator transcript + the `CN-CONS-03` flip. AJ's
  > CEs are all hermetically/mechanically reachable; the real live pass is the **follow-on operator
  > pass** that AJ unblocks. **`CN-CONS-03` stays `declared` through AJ-close** — AJ must not claim
  > live convergence.

- **Slices** (ordered; ordering is the safety guard — no slice may merge before its predecessor):
  - **AJ-S1** — Dedicated convergence-evidence sink + `--convergence-evidence-path` + closed-vocabulary
    writer + isolation **(inert)** — invariant: a supplied path opens a dedicated writer reusing the
    closed `AgreementVerdict` vocabulary; absent ⇒ no file ⇒ unchanged; the file's vocabulary is
    closed (new CI check) and isolated from other-mode files; **not yet fed** by the participant
    path — addresses: CE-AJ-1 — TCB: RED (CLI + sink) + GREEN (reused vocabulary) — targets:
    **DC-ADMIT-04 strengthened**; DC-NODE-30 declared (sink half).
  - **AJ-S2** — Wire emission into `run_participant_sync` **(the go-live evidence flip)** — invariant:
    emit `BlockReceived` for each considered peer block (before drop/admit/refuse), then
    `BlockAdmitted`+`AgreementVerdict(derive(outcome, observed_peer_tip))` after each `pump_block`
    admit, to the dedicated sink; observe the peer tip (OQ-AJ-4); emit-only on `Diverged`;
    writer-failure non-fatal to authority; resolves OQ-AJ-1 (admit outcome surface) + OQ-AJ-6 (no
    wall-clock) — addresses: CE-AJ-2, CE-AJ-3 — TCB: GREEN glue invoked from the RED loop, **no new
    BLUE** — targets: **DC-NODE-30 enforced** — acceptance: hermetic `Block→RollBack→Block` →
    gate-accepts; `Diverged`-fixture → emit-only (no halt/mutation); writer-failure → authority
    continues; absent-path → byte-unchanged.
  - **AJ-S3** — Runbook correction + `DC-EVIDENCE-03` transcript-shape rule **(close-out)** —
    invariant: the runbook describes the landed `--convergence-evidence-path` flow; `DC-EVIDENCE-03`
    tied to the existing gate (strict slot regression in the **observed peer block sequence** + ≥1
    `agreed` + 0 `diverged` + sha256-bound; single-best-peer scope) — addresses: CE-AJ-4 — TCB: docs
    + registry + GREEN gate reuse — targets: **DC-EVIDENCE-03 enforced_scaffolding**.

- **Replay obligations:**
  - **No new authoritative state, no new canonical types, no new replay-corpus entries.** The
    convergence-evidence JSONL is a side-output (not WAL'd, not authority); authoritative
    replay-equivalence (checkpoint + WAL → post-state) is **unchanged**.
  - **New GREEN evidence replay-equivalence** (AJ-S2 hermetic test, per
    `feedback_evidence_reducers_are_green_not_authority`): same recovered store + same ordered
    receive events → **byte-identical** convergence-evidence JSONL **and** same durable post-state
    **and** same verdict sequence.

- **Registry (targets — `declared` at `/cluster-doc`, status flips *aimed for* at AJ-close):**
  - **DC-NODE-30** — *targeted for enforcement at AJ close* (hermetic; participant emits convergence
    evidence as a GREEN side-output, with the evidence-only guards).
  - **DC-ADMIT-04** — *targeted for strengthening at AJ close* (closed-vocabulary isolation extended
    to name the convergence-evidence file).
  - **DC-EVIDENCE-03** — *targeted for enforced_scaffolding at AJ close* (gate + hermetic fixture +
    corrected runbook in place; the real committed transcript is pending the follow-on operator
    pass).
  - **CN-CONS-03** — **untouched; stays `declared`.** It flips only after the post-AJ live operator
    pass commits a real transcript.
