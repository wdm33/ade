# PHASE4-N-AJ — Participant-path convergence evidence emission — Invariant Sketch

> IDD Part I artifact (`/invariants`). Frames the concept before any cluster/slice/code.
> Status: **confirmed** (open questions OQ-AJ-2/3/5/7 resolved by operator steer, recorded in §7).
> Registry entries are **deferred to `/cluster-doc`** (declared there, enforced at close) — this
> sketch only *shapes* them (§9).

## 0. Why this cluster exists (the gap)

CE-AI-6 (the `CN-CONS-03` live convergence transcript) requires **one** JSONL proving **both**
(1) `agreement_verdict` / `Agreed` evidence and (2) a real rollback-follow (a strict slot
regression). Today those halves are split across two modes that cannot be combined into one
honest transcript:

- **`--mode admission`** (`admission/bootstrap.rs:377` `RollBackward { .. } => continue`): emits
  the gate-valid closed `AgreementVerdict` vocabulary, but **ignores rollbacks** → never a slot
  regression. (Proven empirically: the N-M-C / N-M-follow transcripts are gate-vocab-clean with
  **0** slot regressions.)
- **`--mode node --participant-venue`** (`node_lifecycle::run_participant_sync`): **follows** the
  peer's `RollBackward` durably via `apply_chain_event` (→ strict slot regression), but emits **no**
  agreement evidence (only the sched/`forge_*` live-log vocabulary, which the gate rejects).

The fix is **not** consensus logic — the rollback-follow is correct and proven (DC-NODE-23..29).
What is missing is the **evidence bridge**: wire the existing GREEN `AgreementVerdict` reducer +
closed event vocabulary into the participant path, as a side-output.

## 0a. Pure-transformation test (is the concept understood?)

**Yes.** It is a pure transformation:

```
(recovered store + imported LiveConsensusInputs,
 ordered peer receive events [Block | RollBackward | TipUpdate])
   → (durable chain state            [AUTHORITY — byte-unchanged],
      convergence-evidence JSONL      [NEW — pure GREEN side-output])
```

`verdict::derive(&BlockAdmitOutcome, &Tip) -> AgreementVerdict` is **already** total, pure,
deterministic GREEN. This cluster serializes outcomes the node already computes. It introduces
**no** new authority and **no** nondeterminism on any authoritative path.

---

## 1. What must always be true

- **I-AJ-1 (core invariant):** the *same* live participant path that performs rollback-following
  emits the closed `AgreementVerdict` evidence vocabulary, verbatim and sha256-bindable, **without
  becoming semantic authority**. Evidence compares observed tips only; it never decides
  admission / fork-choice / rollback / validity.
- **I-AJ-2 (emission completeness):** on the participant path — `BlockReceived` per peer-received
  block, `BlockAdmitted` after each `pump_block` admit, `AgreementVerdict` (= `verdict::derive`)
  after each admit, comparing Ade's durable admitted `(slot,hash)` vs the **observed** peer tip.
- **I-AJ-3 (reuse, no new enum):** the emitted events are the existing
  `AdmissionLogEvent::{BlockReceived, BlockAdmitted, AgreementVerdict}` closed variants via
  `verdict::derive` + `verdict_kind`. No new evidence enum / truth language.
- **I-AJ-4 (oracle binding preserved):** each `BlockAdmitted` / `AgreementVerdict` carries the
  `consensus_inputs_fingerprint_hex` of the recovered bundle (DC-ADMIT-10 parity) — the participant
  transcript is bound to the operator oracle exactly as the admission transcript is.
- **I-AJ-5 (gate satisfiable):** one participant run through a real peer reorg yields a transcript
  passing `ci_check_convergence_evidence_schema.sh` (closed vocab · 0 diverged · ≥1 strict slot
  regression in the observed peer block sequence · sha256-bound `.md`).
- **I-AJ-6 (absent-sink ⇒ unchanged — guard G3):** with **no** `--convergence-evidence-path`
  supplied, **no** evidence file is emitted and node behavior is **byte-unchanged**. The evidence
  path is purely additive and opt-in.
- **I-AJ-7 (writer-failure isolation — guard G1):** a convergence-evidence **write failure is
  non-fatal to authority** — it never halts consensus, never aborts the authoritative receive/admit
  loop, and never corrupts durable state. The evidence sink is **distinct from the authoritative
  WAL**: a WAL append failure still fails-closed as before (`AdmissionHalted`-class authority); an
  evidence-JSONL write failure is at most a RED operational log. (The two must not be conflated.)

## 2. What must never be possible

- **¬AJ-1:** the verdict/emission is **never** a second authority — never gates admission, never
  triggers/parameterizes a rollback, never influences fork-choice, never mutates the durable chain.
  Authority stays: `pump_block` (sole roll-forward admit), `apply_chain_event` (sole rollback
  authority), `classify_receive` (receive disposition).
- **¬AJ-2:** **no second rollback-follow path** — the admission bridge stays
  `RollBackward => continue`; the participant path stays the **sole** live rollback follower. The
  evidence wiring does not give admission mode (or any other surface) a way to follow rollbacks.
- **¬AJ-3:** **no BLUE change** — `pump_block`, `apply_chain_event`, `classify_receive`,
  `select_best_chain`, ledger, WAL byte-unchanged. (`verdict::derive` is GREEN, also unchanged.)
- **¬AJ-4 (vocabulary isolation):** the convergence-evidence file carries **only** its declared
  closed convergence vocabulary; it is its **own file**, never co-mingled with the node sched /
  `forge_*` log (else the gate's closed-vocabulary check fails). Convergence literals do not leak
  into other-mode files and vice versa (extends DC-ADMIT-04).
- **¬AJ-5 (no overstating):** `Lagging` / self-accept ≠ convergence; a same-tip-only run (no
  regression) is **not** CE-AI-6; no `Healthy` / `Synced` / `Ready` / `LiveReady` sentinel exists.
- **¬AJ-6 (Diverged never dropped, never authority — guard G2):** a `Diverged` verdict is emitted
  as evidence (and the gate rejects any transcript containing it), but it **does not** mutate
  authority, halt consensus, or trigger a rollback. The node's semantic behavior on a true
  divergence remains governed by the existing fail-closed authority (`classify_receive` refuses a
  bare competing block); the verdict is the *evidence label* for that same condition, not a second
  decider.

## 3. Determinism surface (identical across executions)

`verdict::derive` is already pure / total / deterministic. **New obligation:** for a fixed ordered
receive-event sequence, the emitted JSONL event sequence is **byte-identical** across runs — no
wall-clock, no rand, no `HashMap` iteration, canonical lowercase hex; `consensus_inputs_fingerprint_hex`
constant for a given recovered bundle. (Confirm the existing admission writer carries no wall-clock
field — OQ-AJ-6 — and match it.)

## 4. Replay-equivalence

- **Authoritative replay-equivalence UNCHANGED** (the emission is a side-output, not WAL'd
  authority): same checkpoint + WAL → same post-state.
- **Evidence replay-equivalence** (per `feedback_evidence_reducers_are_green_not_authority`'s
  per-evidence obligation): same recovered store + same ordered receive events → byte-identical
  convergence-evidence JSONL **and** same durable post-state **and** same verdict sequence. The
  JSONL is an *output*, never a replay *input*.

## 5. State transitions in scope (authority unchanged; emit added as effect)

| (prior, input) | → Result |
|---|---|
| `(durable_tip, Block) | LinearExtend` | `Ok(durable_tip', [WAL-append (BLUE, unchanged); emit BlockReceived; emit BlockAdmitted; emit AgreementVerdict(derive(Valid, observed_peer_tip))])` |
| `(durable_tip, Block) | AlreadyHave` | `Ok(durable_tip, [emit BlockReceived])` *(echo drop — emit `BlockReceived` only; no admit, no verdict)* |
| `(durable_tip, Block) | NeedsForkChoice` | `Err(fail-closed)` *(unchanged; the divergence is the existing authority's, not the verdict's)* |
| `(durable_tip, RollBack(point)) valid` | `Ok(rolled_back_tip, [apply_chain_event (BLUE, unchanged)])` — the **strict slot regression** then appears in the *next* peer `BlockReceived` on re-extension |
| `(durable_tip, RollBack(point)) invalid` | `Err(fail-closed)` *(unchanged)* |
| `(durable_tip, TipUpdate(tip))` | observe peer tip (verdict input); emit nothing |

Effects beyond the existing WAL/apply are **append-only writes to the dedicated evidence JSONL**;
no durable authority effect.

## 6. TCB color hypothesis

- **BLUE (unchanged, reused):** `pump_block`, `apply_chain_event`, ledger, WAL.
- **GREEN:** `verdict::derive` (existing), the `AdmissionLogEvent` vocabulary + writer (existing),
  and the **new glue** that — after each authoritative admit / rollback — calls `derive` and hands
  the closed event to the writer. Deterministic; compares already-authoritative outputs; decides no
  authority.
- **RED:** the JSONL file sink (`--convergence-evidence-path`) + the operator run harness.
- **Resolved:** *what* to emit is GREEN; *when* is the RED loop's existing control flow; *writing*
  is a RED sink. **No new BLUE.**

## 7. Resolved design decisions (operator steer — were OQ-AJ-2/3/5/7)

- **D-1 (dedicated sink — OQ-AJ-2/3):** a **separate convergence-evidence JSONL**, never mixed with
  the sched / `forge_*` log. CLI shape: **`--convergence-evidence-path <path>`**.
  **Hard rule:** *no path supplied → no evidence file emitted; path supplied → only closed
  convergence-evidence events written there.* The file has its own declared closed allow-list
  (reusing the existing literals); DC-ADMIT-04 isolation is **strengthened** to name it.
- **D-2 (emit-only on Diverged — OQ-AJ-5):** a `Diverged` evidence event does **not** mutate
  authority, halt consensus, trigger rollback, or decide anything. The gate rejects transcripts
  containing it; the node's semantic behavior stays governed by the existing receive/rollback/admit
  authorities. (= ¬AJ-6 / guard G2.)
- **D-3 (regression via existing vocabulary — OQ-AJ-7):** the reorg is proven by a **strict slot
  regression in the observed peer block sequence**, using the existing `BlockReceived` event. **Do
  not** add a new rollback evidence enum unless the existing vocabulary proves insufficient — the
  purpose of AJ is to *bridge* the existing evidence vocabulary into the participant path, not mint
  a new truth language. *(Constraint for the slice: the convergence `BlockReceived` sequence must
  mean **peer-received block** — the σ=0 participant never forges into it — so the regression
  unambiguously signals the peer branch rewinding, not any local observation.)*

## 8. Remaining open questions (implementation confirmations for `/cluster-plan` / `/slice-doc`)

- **OQ-AJ-1 (load-bearing):** does `pump_block` surface the `(slot, block_hash, post_fp)` that
  `BlockAdmitOutcome::Valid` needs — or must the glue decode the admitted header + read the
  post-admit fingerprint the WAL entry already carries? (Non-BLUE either way; resolve at slice-doc.)
- **OQ-AJ-4:** confirm the observed peer tip (currently `TipUpdate` "observe-skip" in the drain) is
  available to `derive` at admit time on the participant path.
- **OQ-AJ-6:** confirm the existing admission writer emits **no wall-clock field**; the participant
  emission must match (determinism, §3).

## 9. Proposed registry entries (shaped here; **declared at `/cluster-doc`**)

- **DC-NODE-30** (derived) — *Participant-path convergence evidence emission.* The live
  `--mode node --participant-venue` rollback-follow path emits, to a dedicated convergence-evidence
  JSONL (`--convergence-evidence-path`), the existing closed `AgreementVerdict` vocabulary
  (`BlockReceived`/`BlockAdmitted`/`AgreementVerdict` via `verdict::derive`) as a deterministic GREEN
  side-output of each already-authoritative admit + observed peer tip. MUST NOT become authority
  (no admission gate, no rollback trigger, no fork-choice influence, no durable mutation);
  `pump_block` sole roll-forward admit; `apply_chain_event` sole rollback authority; no new evidence
  enum; no BLUE change; absent path ⇒ behavior byte-unchanged; write failure non-fatal to authority.
- **DC-EVIDENCE-03** (derived) — *Convergence-through-reorg transcript shape (CE-AI-6).* The
  participant convergence pass produces ONE JSONL with AT LEAST: **a strict slot regression in the
  observed peer block sequence** (a peer `RollBackward` was followed) + ≥1
  `AgreementVerdict{kind:"agreed"}` at the re-converged tip; AT MOST: 0
  `AgreementVerdict{kind:"diverged"}`. The `.md` manifest binds the `.jsonl` sha256.
  Vacuous-until-committed; validated by `ci_check_convergence_evidence_schema.sh`. **Single-best-peer
  scope — NOT full multi-peer ChainSel.**
- **DC-ADMIT-04** (strengthen) — closed-vocabulary isolation extended to name the convergence-evidence
  file as a third declared closed-vocabulary file (`strengthened_in += PHASE4-N-AJ`).

*(This registry tracks its count in commit / HEAD_DELTAS, not an in-file `registry_count`. New rules
land `declared` at cluster-doc, flip `enforced` at close.)*

## 10. Scope note

- **Runbook correction is IN SCOPE for AJ:** `docs/active/phase4-n-ai-convergence-runbook.md`
  currently instructs an operator pass that cannot produce a gate-valid transcript; it is **known
  inaccurate until this bridge exists** and must be corrected within AJ (to the
  `--convergence-evidence-path` participant-evidence path).
- **Out of scope:** multi-candidate `select_best_chain` live selection (full multi-peer ChainSel);
  teaching `--mode admission` to follow rollbacks (forbidden — ¬AJ-2); the operator transcript
  itself (operator-produced, after the bridge lands).
