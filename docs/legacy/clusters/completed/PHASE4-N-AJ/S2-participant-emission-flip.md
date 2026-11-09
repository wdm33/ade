# Invariant Slice AJ-S2 — Participant-path convergence evidence emission (the flip)

> Slice of PHASE4-N-AJ — the **go-live evidence flip**: `run_participant_sync` now *emits* the
> closed convergence vocabulary to the AJ-S1 sink. The IDD analog of AI-S4b-ii. **No BLUE change;
> no consensus / rollback / fork-choice / WAL / admission semantic change** — evidence is a
> side-output of already-authoritative outcomes.
>
> **Hard line: evidence observes authority; evidence never becomes authority.**

## 2. Slice Header
- **Slice Name:** Wire `ConvergenceEvidenceSink` into `run_participant_sync` (emit-only convergence evidence).
- **Cluster:** PHASE4-N-AJ. **Status:** Merged.
- **Cluster Exit Criteria Addressed:** **CE-AJ-2** (participant emits convergence evidence; hermetic gate-accept) + **CE-AJ-3** (evidence-only guards). *(CE-AJ-1 done in AJ-S1; CE-AJ-4 is AJ-S3.)*
- **Dependencies:** AJ-S1 (the sink + `--convergence-evidence-path`); PHASE4-N-AI (`run_participant_sync`); PHASE4-N-M-B (`verdict::derive`, `AdmissionLogEvent`).

## 4. Intent
The *same* participant rollback-follow path that admits via `pump_block` and follows `RollBackward`
also emits the closed `AgreementVerdict` evidence vocabulary, as a deterministic GREEN side-output —
**never becoming authority** — so one participant run through a peer reorg yields a transcript the
existing gate accepts. **Enforces DC-NODE-30.**

## 5. Scope
- **Modules:** `ade_node::node_lifecycle` (`run_participant_sync` emit calls + the call-site sink
  construction); `ade_node::convergence_evidence` (the emit methods now return a `#[must_use]`
  `EvidenceEmitResult`; add an internal poison flag + `is_poisoned()` — the AJ-S1 io::Result return
  is replaced and the AJ-S1 tests updated accordingly).
- **State machines:** none changed — the receive **routing** is byte-identical; emission is additive.
- **Persistence:** none (side-output; not WAL/checkpoint).
- **Out of scope:** runbook + DC-EVIDENCE-03 (AJ-S3); the live transcript.

> `EvidenceEmitResult` is an emit-**status** type (control flow), **not** a new evidence
> **vocabulary** enum — ¬AJ-3 (no new evidence enum / truth language) is preserved; the transcript
> vocabulary stays the reused `AdmissionLogEvent` subset.

## 6. Execution Boundary (TCB color)
- **BLUE:** none — `pump_block`, `apply_chain_event`, ledger/WAL byte-unchanged.
- **GREEN:** the emit glue (what to emit after each authoritative receive/admit) + `verdict::derive`
  (reused) + the `TipPoint→Tip` shim + the `fingerprint(&state.ledger)` read + `EvidenceEmitResult`
  routing.
- **RED:** the `ConvergenceEvidenceSink` file writes (reused); the call-site
  `open(cli.convergence_evidence_path)`; the post-run "evidence incomplete" diagnostic.

## 7. Invariants Preserved
`DC-NODE-23…29` (rollback-follow — the receive/rollback routing is byte-unchanged), `DC-CONS-03`,
`DC-NODE-05/12` (`pump_block` sole roll-forward admit), `DC-NODE-28` (no forge across pending
reselection), `DC-ADMIT-04`/`-08` (closed vocab + GREEN-evidence), `CN-CONS-03` (untouched).

## 8. Invariants Strengthened or Introduced
- **`DC-NODE-30` — enforced** (hermetic): the participant path emits the closed convergence
  vocabulary as a GREEN side-output; emit-only on `Diverged`; writer-failure non-fatal to authority
  **but surfaced** (poisons + marks the transcript incomplete); absent-sink ⇒ unchanged. *One
  family: participant convergence-evidence emission.*

## 9. Design Summary (resolved OQs baked in)
**Emit status type (adjustment 1 — no silent swallow):**
```
#[must_use]
pub enum EvidenceEmitResult { Written, Disabled, FailedAndPoisoned }
```
The three `ConvergenceEvidenceSink` emit methods return it (replacing AJ-S1's `io::Result`): a
configured sink → `Written`; no path → `Disabled`; an inner `Write` error → the sink sets its
internal `poisoned` flag and returns **`FailedAndPoisoned`** (it does **not** return `Ok`/pretend
success). Once poisoned, every later emit returns `FailedAndPoisoned`. `is_poisoned()` exposes it
for the post-run check.

**Signature:** `run_participant_sync(…, evidence: &mut ConvergenceEvidence)` where
`ConvergenceEvidence { sink: ConvergenceEvidenceSink<File>, consensus_inputs_fingerprint_hex: String,
peer_label: String, incomplete: bool }`. The call site (`node_lifecycle` ~L1249) builds it:
`sink = ConvergenceEvidenceSink::open(cli.convergence_evidence_path.as_deref())`;
`consensus_inputs_fingerprint_hex = hex(canonical.fingerprint)` (**the same DC-ADMIT-10 binding
admission uses — `bootstrap.rs:236`**); `peer_label` = the followed peer addr.

**Caller routing (write failure non-fatal but never invisible):** every emit's `EvidenceEmitResult`
is funnelled through `evidence.note(result)` — `FailedAndPoisoned ⇒ evidence.incomplete = true`;
the authoritative loop **continues** (no propagation into its `Result`). After the run,
`evidence.incomplete || sink.is_poisoned()` ⇒ the node emits a **RED "convergence evidence
incomplete"** diagnostic so the operator does **not** commit a partial transcript.

**Per `NodeSyncItem::Block(bytes)`** (additive; routing unchanged):
1. decode → candidate (already done) → **`emit_block_received(peer_label, candidate.slot,
   candidate.hash_hex)`** — for **every considered block, BEFORE** classify/route. *`BlockReceived`
   is evidence of **peer input**, not of local admission* — it is the one event that legitimately
   precedes an authoritative outcome (it describes only what the peer served). This is what makes
   the strict slot regression observable even when a later block is already-have or refused.
2. route (unchanged): **LinearExtend** → `pump_block` → `PumpTip{slot, hash}` (local durable admit
   succeeded); **only now** `post_fp = fingerprint(&state.ledger).combined` (OQ-AJ-1 — *no
   `pump_block` change*; `derive` ignores `post_fp`); **`emit_block_admitted(slot, hash, post_fp,
   fp)`** (*`BlockAdmitted` is the proof of local durable admission*); build
   `BlockAdmitOutcome::Valid{slot, block_hash, post_fp}`; peer tip =
   `source.followed_peer_tip_signal().latest` → `Tip` shim (OQ-AJ-4; `None`→`Origin`);
   `verdict = derive(outcome, &peer_tip)`; **`emit_agreement_verdict(verdict_kind(&verdict), …)`**.
   **AlreadyHave** → drop (no `pump_block`, **no `BlockAdmitted`**, no verdict). **NeedsForkChoice**
   → fail closed (**unchanged**; no `BlockAdmitted`).

**Per `NodeSyncItem::RollBack`** → **byte-unchanged** (`apply_chain_event`); the strict slot
regression then appears in the *next* `emit_block_received` on re-extension.

**Evidence-only guards (CE-AJ-3):**
- **Emit-only on Diverged (D-2):** `verdict` feeds only `emit_agreement_verdict` — never branches the
  loop, never halts, never triggers rollback. `classify_receive` remains the sole receive authority.
- **Writer-failure (G1):** surfaced via `FailedAndPoisoned` + `incomplete` (above) — non-fatal to
  authority, never invisible to evidence status.
- **Absent path (G3):** `open(None)` ⇒ `Disabled` ⇒ every emit a no-op ⇒ consensus + existing logs
  unchanged.

## 10. Changes Introduced
- **Types:** `EvidenceEmitResult` (status enum); `ConvergenceEvidenceSink` gains `poisoned: bool` +
  `is_poisoned()`, emit methods return `EvidenceEmitResult`. `ConvergenceEvidence` context struct.
- **`run_participant_sync`:** one new `&mut ConvergenceEvidence` param; additive emit calls; routing
  byte-unchanged. Call site opens the sink + builds the context.
- **Persistence / BLUE:** none.

## 11. Replay, Crash, Epoch Validation
- Authoritative replay-equivalence **unchanged** (emission is a side-output). **Evidence
  replay-equivalence** test `participant_convergence_evidence_replay_byte_identical`: same recovered
  store + same ordered events → byte-identical JSONL + same durable post-state + same verdict
  sequence (no wall-clock — OQ-AJ-6; reused `AdmissionLogWriter`).
- **Crash:** a sink write error poisons + marks incomplete (non-fatal); flush-per-line ⇒ complete
  lines.

## 12. Mechanical Acceptance Criteria
- [ ] `participant_convergence_run_through_reorg_is_gate_valid` — `InMemory` source `Block→RollBack→Block` + a dedicated sink ⇒ the JSONL satisfies all four `ci_check_convergence_evidence_schema.sh` conditions **and** has **≥1 `agreed`** (DC-EVIDENCE-03), 0 `diverged`, ≥1 strict slot regression in the peer block sequence.
- [ ] `participant_block_received_does_not_imply_admission` — a considered block that is **refused** (`NeedsForkChoice`) or **already-have** ⇒ `BlockReceived` emitted but **no `BlockAdmitted`**; `BlockAdmitted` appears only when `pump_block` succeeds.
- [ ] `participant_diverged_verdict_is_emit_only_no_halt` — a verdict deriving `Diverged` is emitted but the durable tip / loop control flow is unchanged (no halt, no rollback).
- [ ] `participant_convergence_write_failure_non_fatal_to_authority` — a failing sink ⇒ emit returns `FailedAndPoisoned`, `evidence.incomplete`/`is_poisoned()` set, the authoritative loop completes, durable state byte-identical to the no-sink run.
- [ ] `participant_convergence_evidence_replay_byte_identical` — as §11.
- [ ] `participant_absent_convergence_sink_byte_unchanged` — `open(None)` ⇒ durable post-state + existing logs byte-identical to pre-AJ.
- [ ] `ci/ci_check_convergence_evidence_emit_only.sh` (NEW, here-strings) — the verdict/emit result never feeds `classify_receive`/`apply_chain_event`/`pump_block`/forge control flow; the sink write is distinct from the WAL append.
- [ ] `cargo test -p ade_node` green; the AJ-S1 gates (`ci_check_convergence_evidence_vocabulary_closed.sh` + the two existing vocab gates) stay green; **no BLUE change** (FC/IS audit).

## 13. Failure Modes
- Sink write error ⇒ `FailedAndPoisoned` + `incomplete` ⇒ RED diagnostic post-run; authority unaffected; transcript not committed.
- `Diverged` verdict ⇒ emitted as evidence; the gate rejects any transcript containing it; authority governed by the existing fail-closed (`classify_receive`).

## 14. Hard Prohibitions
- **Inherited** (cluster Forbidden).
- **Slice-specific:** the receive **routing** (`classify_receive`/`resolve_disposition`), the
  `RollBack` arm, and the DC-NODE-28 forge gate are **byte-unchanged** (only additive emit calls);
  `pump_block`/`apply_chain_event`/`verdict::derive` unchanged; the verdict **never** influences
  authority; a sink write error **never** propagates into the authoritative `Result`; no new evidence
  **vocabulary** enum. **No evidence emit method may be called before the authoritative outcome it
  describes exists — except `BlockReceived`, which describes only peer input** (`BlockAdmitted` only
  after a successful `pump_block`; `AgreementVerdict` only over a real `BlockAdmitOutcome`).

## 15. Explicit Non-Goals
The runbook + DC-EVIDENCE-03 (AJ-S3); the live transcript; multi-candidate selection; any
`CN-CONS-03` flip.

## 16. Completion Checklist
- [ ] Emit wiring + `EvidenceEmitResult` + poison/incomplete handling landed; routing byte-unchanged.
- [ ] All §12 tests + the new emit-only gate + the AJ-S1 gates green; `cargo test -p ade_node` green.
- [ ] No BLUE change; evidence never reads back into authority; DC-NODE-30 mechanically enforced.
