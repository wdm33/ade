# Invariant Slice AJ-S1 — Dedicated convergence-evidence sink (inert)

> Slice of PHASE4-N-AJ — the **first** slice. Builds the closed-vocabulary convergence-evidence
> sink + `--convergence-evidence-path` flag, **inert** (parsed but not yet fed by the participant
> path — that is AJ-S2). The IDD analog of AI-S4b-i (inert declaration before the go-live flip).

## 2. Slice Header
- **Slice Name:** Dedicated convergence-evidence sink + `--convergence-evidence-path` + closed-vocabulary writer (inert).
- **Cluster:** PHASE4-N-AJ. **Status:** Proposed.
- **Cluster Exit Criteria Addressed:** **CE-AJ-1** (dedicated/opt-in/isolated sink; no path ⇒ no file; closed vocabulary). *(CE-AJ-2/3/4 explicitly out of scope.)*
- **Dependencies:** PHASE4-N-M-B (`AdmissionLogWriter` + `AdmissionLogEvent`, reused unchanged); PHASE4-N-AI (the participant venue exists — **not** touched here).

## 4. Intent
Make the convergence-evidence transcript a **closed-vocabulary, opt-in, isolated** surface: the only
way to write it is through a sink that (type-level) restricts emission to the reused closed
`AdmissionLogEvent` subset `{block_received, block_admitted, agreement_verdict}`, and that opens
**no file / emits nothing** when no `--convergence-evidence-path` is supplied. Strengthens
**DC-ADMIT-04** (closed-vocabulary isolation now covers a third file type). No authoritative behavior
changes.

## 5. Scope
- **Modules/crates:** `ade_node::cli` (new flag → `Option<PathBuf>`); **new** `ade_node::convergence_evidence` (the `ConvergenceEvidenceSink` wrapper); `ci/ci_check_convergence_evidence_vocabulary_closed.sh` (new gate). **Reused unchanged:** `ade_node::admission_log` (`AdmissionLogWriter`, `AdmissionLogEvent`).
- **State machines affected:** none.
- **Persistence impact:** none — the convergence-evidence JSONL is a side-output (not WAL/checkpoint).
- **Network-visible impact:** none.
- **Out of scope:** participant-path emission (AJ-S2); runbook + DC-EVIDENCE-03 (AJ-S3).

## 6. Execution Boundary (TCB color)
The wrapper's *event selection* and the *file I/O* are different colors and are kept separate:
- **BLUE:** none.
- **GREEN:** the `ConvergenceEvidenceSink<W>` API shape + allowed-event construction (which closed
  `AdmissionLogEvent` variant each of the three methods builds) — deterministic, generic over
  `W: Write`, performs **no I/O itself**; `ade_node::admission_log` serialization
  (`AdmissionLogEvent` + `encode_event`, reused).
- **RED:** the **instantiated file-backed sink** — `File::create(path)` and the JSONL byte writes
  (`AdmissionLogWriter<File>::emit` → `write_all`/`flush`); `ade_node::cli` flag parse.

> The generic construction rules are GREEN-by-function; the file-backed *instantiation*
> (`ConvergenceEvidenceSink<File>`) is RED. The hermetic tests instantiate the sink over `Vec<u8>`
> to exercise the GREEN construction without touching the filesystem.

## 7. Invariants Preserved
`DC-ADMIT-04` (admission/wire-only isolation — preserved, then extended), `DC-NODE-23…29`
(rollback-follow — untouched; `run_participant_sync` byte-unchanged in S1), `DC-CONS-03`,
`DC-ADMIT-08` (Lagging/verdict GREEN-evidence discipline), `CN-CONS-03` (untouched). With no
`--convergence-evidence-path` supplied: **no convergence-evidence file is opened or written;
consensus behavior and existing logs are unchanged.**

## 8. Invariants Strengthened or Introduced
- **`DC-ADMIT-04` — strengthened** (`strengthened_in += "PHASE4-N-AJ"`): the closed-vocabulary
  isolation now names the convergence-evidence surface as a third isolated closed-vocabulary file;
  new gate `ci_check_convergence_evidence_vocabulary_closed.sh` added to its `ci_scripts`.
- **`DC-NODE-30` — sink half only** (stays `declared`): the type-level closed sink exists; the
  emission flip is AJ-S2. *One family: convergence-evidence vocabulary closure/isolation.*

## 9. Design Summary
- **Reuse `AdmissionLogWriter<W>`** (already generic over `W: Write`) as the serializer — **no new
  writer, no new event enum** (¬AJ-3).
- **New GREEN `ConvergenceEvidenceSink<W>`** = a thin wrapper over `Option<AdmissionLogWriter<W>>`
  exposing **only three** emit methods — `emit_block_received`, `emit_block_admitted`,
  `emit_agreement_verdict` — each constructing the corresponding **existing** `AdmissionLogEvent`
  variant and forwarding to the inner writer. The sink exposes **no** method that returns/borrows
  the raw inner `AdmissionLogWriter` and **no** method that emits any other variant ⇒ compiler-level
  vocabulary closure (the type half of the property). `open(path: Option<&Path>) -> io::Result<Self>`:
  `None` ⇒ inner `None` (no file created, emits are no-ops); `Some(p)` ⇒ `File::create(p)` (RED) +
  `AdmissionLogWriter::new`.
- **Closed convergence vocabulary** = `{block_received, block_admitted, agreement_verdict}` — all
  reused `AdmissionLogEvent` variants, all in `ci_check_convergence_evidence_schema.sh`'s ALLOWED
  set; `block_admitted` + `agreement_verdict` carry `consensus_inputs_fingerprint_hex` (I-AJ-4
  oracle binding). **Excluded by construction** (the sink has no method for them):
  `admission_started`, `snapshot_imported`, `bootstrap_complete`, `admission_halted`,
  `admission_shutdown`, and all `sched_*` / `forge_*` / wire-only literals — the convergence file is
  an evidence transcript, not a lifecycle log.
- **CLI:** `--convergence-evidence-path <path>` parsed in `cli.rs`'s arg loop → `Option<PathBuf>` on
  the Node config. **Parsed but inert** (not yet read by the node lifecycle — that wiring is AJ-S2).
- **New gate** `ci/ci_check_convergence_evidence_vocabulary_closed.sh` (mirrors
  `ci_check_admission_log_vocabulary_closed.sh`): (1) the `convergence_evidence` module constructs
  only the three subset variants; (2) those three ⊆ the schema gate's ALLOWED set; (3) the module
  contains no `sched_*`/`forge_*`/wire-only literals and none of the excluded admission lifecycle
  literals; (4) the module exposes no accessor to the raw inner writer (`into_inner` / `writer` /
  any `-> …AdmissionLogWriter`).

## 10. Changes Introduced
- **Types:** new `ConvergenceEvidenceSink<W>` (wrapper struct over `Option<AdmissionLogWriter<W>>`).
  `AdmissionLogEvent` / `AdmissionLogWriter` **unchanged**.
- **CLI:** `--convergence-evidence-path` → `Option<PathBuf>` (parsed, stored, **unused** in S1).
- **State transitions / persistence:** none.

## 11. Replay, Crash, Epoch Validation
- No authoritative state; the sink is a side-output. **Determinism:** reuses the existing
  flush-per-line `AdmissionLogWriter` (no wall-clock, no float, no `HashMap`). Test
  `convergence_evidence_writer_emits_closed_vocabulary` (over a `Vec<u8>` sink — hermetic, no
  filesystem) proves the three methods serialize exactly their literals + round-trip discriminators.
  No replay-corpus change.
- **Crash:** absent path ⇒ no file; flush-per-line ⇒ complete lines (existing writer property).
  Emission-time crash semantics are AJ-S2 (S1 emits nothing).
- **Epoch:** not applicable.

## 12. Mechanical Acceptance Criteria
- [ ] `convergence_evidence_absent_path_emits_no_file` — `ConvergenceEvidenceSink::open(None)` creates no file (asserted over a temp dir); its emits are no-ops.
- [ ] `convergence_evidence_writer_emits_closed_vocabulary` — the three emit methods (over a `Vec<u8>` sink) produce exactly `block_received` / `block_admitted` / `agreement_verdict` lines; discriminators round-trip.
- [ ] `convergence_evidence_sink_does_not_expose_inner_writer` — the wrapper exposes no public accessor that yields the raw `AdmissionLogWriter` (no `into_inner`, no public inner field), so a caller cannot bypass the three-variant subset.
- [ ] `ci/ci_check_convergence_evidence_vocabulary_closed.sh` green (NEW) — convergence module closed to the three-variant subset + isolation + subset ⊆ schema-gate ALLOWED + no inner-writer accessor.
- [ ] `ci/ci_check_admission_log_vocabulary_closed.sh` + `ci/ci_check_wire_only_event_vocabulary_closed.sh` stay green.
- [ ] Registry: `DC-ADMIT-04` `strengthened_in += "PHASE4-N-AJ"` and `ci_scripts += "ci/ci_check_convergence_evidence_vocabulary_closed.sh"`.
- [ ] `cargo test -p ade_node` green.

## 13. Failure Modes
- `File::create` failure at `open(Some(p))` ⇒ `io::Error` surfaced at open (RED shell; deterministic). Emission-time writer-failure-non-fatal-to-authority is AJ-S2's guard; S1 emits nothing.
- Absent path ⇒ `Disabled` no-op (designed, not a failure).

## 14. Hard Prohibitions
- **Inherited** (cluster Forbidden): no BLUE change; no new evidence enum; no participant emission yet; no co-mingling with the sched/`forge_*` log; `CN-CONS-03` untouched.
- **Slice-specific:** `run_participant_sync` and the whole `node_lifecycle` receive path are **byte-unchanged** in S1 (the flag is parsed but unread); the sink emits nothing until AJ-S2; `AdmissionLogEvent`/`AdmissionLogWriter` unchanged; `ConvergenceEvidenceSink` exposes **no** method that emits a non-subset variant **and no** accessor to the raw inner writer.

## 15. Explicit Non-Goals
The emission flip (AJ-S2); the runbook + DC-EVIDENCE-03 (AJ-S3); the live transcript; any
consensus/rollback/admission/fork-choice/WAL change; `DC-NODE-30` enforcement (stays `declared`
after S1).

## 16. Completion Checklist
- [ ] Sink + flag (inert) + new gate + DC-ADMIT-04 strengthening landed; no TODOs/placeholders.
- [ ] The three Rust tests + the new gate + the two existing vocab gates green; `cargo test -p ade_node` green.
- [ ] `run_participant_sync` byte-unchanged; no new evidence enum; no BLUE change; CI enforces the strengthened DC-ADMIT-04.
