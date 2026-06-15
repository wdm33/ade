# Slice MEM-MEASURE-A2 — Live C2-LOCAL memory-evidence artifact

### Cluster
MEM-MEASURE (primary invariant for this slice: `OP-MEM-01`).

### Status
In Progress.

### Cluster Exit Criteria Addressed
- [ ] CE-MM-3 (`OP-MEM-01` live C2-LOCAL memory artifact — operator-gated)

CE-MM-1/CE-MM-2 (A1) are done. CE-MM-4 (`CN-MEM-01` enforced via live wiring) is MEM-BOUND-B; CE-MM-5/6 are C/D — out of scope here.

### Slice Dependencies
- MEM-MEASURE-A1 (the RED `rss_sampler` + the measurement discipline) — merged (`a84f9045`).

---

## 3. Implementation Instruction (AI)
Emit a committed, auditable LIVE memory-evidence transcript during a `--mode node` C2-LOCAL run, extending the EXISTING closed convergence-evidence vocabulary (no side-channel JSONL). The RSS sampler observes process memory and influences NO authoritative output; the BLUE block/admission/WAL authority is untouched. The committed artifact flips `OP-MEM-01 declared→partial`. Commit message carries this repo's `Co-Authored-By` trailer.

---

## 4. Intent
Prove operationally that **mempool/peer pressure and memory sampling do not starve block validation, chain selection, or persistence**, and that the live run is **replay-equivalent under memory observation** — by pairing RSS samples across the run with the run's WAL replay verdict. Flips `OP-MEM-01 declared→partial` on a committed, sha256-bound transcript.

---

## 5. Scope
- **Modules / crates:**
  - `crates/ade_node/src/admission_log/event.rs` — +2 closed variants `MemoryMeasure`, `MemorySummary` + discriminators.
  - `crates/ade_node/src/admission_log/writer.rs` — +2 `encode_event` arms + 2 `DISCRIMINATORS` entries.
  - `crates/ade_node/src/convergence_evidence.rs` — `ConvergenceEvidenceSink::emit_memory_{measure,summary}` + `ConvergenceEvidence::emit_memory_{measure,summary}` wrappers (with `note`).
  - `crates/ade_node/src/node_lifecycle.rs` — RED: thread the A1 `rss_sampler::RssWindow` through the relay run; sample + emit at the measurement-point seams; emit the summary (with the replay verdict) at shutdown.
  - `ci/ci_check_convergence_evidence_vocabulary_closed.sh` — `ALLOWED_VARIANTS` += MemoryMeasure/MemorySummary, `ALLOWED_LITERALS` += memory_measure/memory_summary.
  - `ci/ci_check_admission_log_vocabulary_closed.sh` — `ADMISSION_ONLY` += memory_measure/memory_summary (declaration + isolation closure).
  - `ci/ci_check_mem_measure_evidence.sh` — NEW vacuous-until-committed gate for the memory transcript.
  - `docs/ade-invariant-registry.toml` — `OP-MEM-01 declared→partial` (on the committed artifact).
  - `docs/evidence/mem-measure-a2-c2local-memory.{md,jsonl}` — the committed transcript (operator pass).
- **State machines affected:** none.
- **Persistence impact:** none (reads WAL/ChainDb/ledger fingerprints; writes only the evidence JSONL).
- **Network-visible impact:** none.
- **Out of scope:** `CN-MEM-01 partial→enforced` (B); Haskell comparison (D); any forge/stake dependency (the memory run follows + serves + admits; it does NOT need to be elected).

---

## 6. Execution Boundary
- **BLUE:** none. Reads `ChainDb::tip`, `ade_ledger::fingerprint::fingerprint(&LedgerState)`, and `ade_ledger::wal::replay::replay_from_anchor` read-only.
- **GREEN:** the 2 closed `AdmissionLogEvent` variants + their writer encoding + the closed-vocab gates + the A1 `evidence`/percentile helpers reused for the summary.
- **RED:** the `rss_sampler` (A1, the single `/proc/self/status` reader) + the `node_lifecycle.rs` sampling/emit wiring + the operator-pass execution.

The RSS samples flow only into the `memory_measure`/`memory_summary` evidence fields; no authoritative output reads them. The run-level replay verdict is computed from the WAL replay (`replay_from_anchor` tail_fp vs the final durable ledger fingerprint), NOT from RSS.

---

## 7. Invariants Preserved
- `DC-WAL-03` — the WAL replays from the anchor to the recorded tail fingerprint (the basis of the replay verdict).
- The closed evidence-vocabulary discipline (`DC-ADMIT-04` / `DC-NODE-30`): the convergence sink constructs only allowed variants; every literal is a closed discriminator.
- `DC-MEM-04` / A1: RSS observation does not perturb the authoritative output.
- BLUE block/admission/WAL authority unchanged.

---

## 8. Invariants Strengthened or Introduced
- **`OP-MEM-01` `declared→partial`:** a committed C2-LOCAL transcript shows RSS sampled across the run's measurement points while block validation (`block_admitted`) and chain agreement (`agreement_verdict`, `lagging` or `agreed`) continue uninterrupted — i.e. memory pressure/observation did not starve the authoritative work — and the run completed cleanly (`memory_summary{replay_verdict=agreed}`, replay-equivalent by the enforced `DC-WAL-03`).

---

## 9. Design Summary
- **Two closed events** added to `AdmissionLogEvent`:
  - `MemoryMeasure { point: &'static str, slot, durable_tip_slot, durable_tip_fp_hex, rss_kib }` — one per sample at a measurement point.
  - `MemorySummary { sample_count, rss_p50_kib, rss_p95_kib, rss_peak_kib, replay_verdict: &'static str }` — once at run end.
- **Measurement points** (the closed `point` set): `idle_recovered_tip`, `chain_sync_follow`, `block_fetch_serve`, `mempool_admission`, `wal_checkpoint_recovery`, `sustained`. Wired at the natural seams of `run_relay_loop_with_sched` / `run_node_lifecycle`:
  - post-recovery startup → `idle_recovered_tip` + `wal_checkpoint_recovery`,
  - co-located with the existing per-admit `emit_admit_and_verdict` → `chain_sync_follow` + `mempool_admission`,
  - serve path (if exercised) → `block_fetch_serve`,
  - idle-wait / shutdown → `sustained`.
- **Replay verdict** (run-level): the `MemorySummary.replay_verdict` is `agreed` iff the relay loop returns Ok — the run completed with no fatal `Diverged` halt (a fatal `Diverged` propagates the loop error, so the summary is never reached and cannot be committed). Replay-equivalence is NOT recomputed at the seam: it is the independently-enforced `DC-WAL-03` (`replay_from_anchor` in warm-start recovery; `ci_check_admit_replay_equivalence.sh` / `ci_check_wal_rollback_replay_equiv.sh`), so a clean Ok run is replay-equivalent. The gate rejects a `diverged` summary.
- **Percentiles**: the A1 `RssWindow` nearest-rank p50/p95/peak over the run's samples.

---

## 10. Changes Introduced
### Types
- New closed variants `AdmissionLogEvent::{MemoryMeasure, MemorySummary}` + discriminators `memory_measure`, `memory_summary`. No new BLUE/canonical/persisted type.
### State Transitions
- None.
### Persistence
- None.

---

## 11. Replay, Crash, and Epoch Validation
- The run-level replay verdict is a clean relay-loop Ok return (no fatal `Diverged`); the summary seam does NOT re-run `replay_from_anchor`. Replay-equivalence is the enforced `DC-WAL-03` (`replay_from_anchor` in warm-start recovery + `ci_check_admit_replay_equivalence.sh` / `ci_check_wal_rollback_replay_equiv.sh`). A2 records the clean-completion verdict; a `diverged` summary fails the gate.
- Crash/restart: n/a beyond the existing warm-start recovery (which the `wal_checkpoint_recovery` sample observes).

---

## 12. Mechanical Acceptance Criteria
- [ ] Hermetic: `AdmissionLogEvent::{MemoryMeasure,MemorySummary}` round-trip through the writer; `ConvergenceEvidence::emit_memory_*` emit the closed discriminators; a closed-vocab negative test rejects an unknown tag.
- [ ] `ci_check_convergence_evidence_vocabulary_closed.sh` green with the two new variants/literals; `ci_check_admission_log_vocabulary_closed.sh` green.
- [ ] `ci_check_mem_measure_evidence.sh` `--self-test` green (accept a valid memory transcript; reject diverged / unknown-tag / sha256-mismatch / missing-summary).
- [ ] `cargo test -p ade_node` green; BLUE untouched.
- [ ] **Operator pass:** committed `docs/evidence/mem-measure-a2-c2local-memory.{md,jsonl}` — memory_measure events across ≥4 of the 6 points, a memory_summary with `replay_verdict=agreed`, ≥1 `block_admitted` + `agreement_verdict` (`lagging`/`agreed`) interleaved (no starvation), 0 `diverged`, sha256-bound. Then `OP-MEM-01 declared→partial`.

---

## 13. Failure Modes
- Sampler unreadable (`/proc` absent): `rss_kib` omitted / sample skipped (fail-soft); the run is still valid evidence if the replay verdict is `agreed`.
- Replay verdict `diverged`: the run is INVALID evidence (gate rejects; OP-MEM-01 stays declared). Fail-closed on the evidence contract.
- Evidence write failure: `ConvergenceEvidence.incomplete` flips (existing mechanism); the operator must not commit an incomplete transcript.

---

## 14. Hard Prohibitions
- No BLUE change; no new feature flag altering authority.
- RSS magnitude never gates the replay verdict or any authoritative output.
- The convergence sink constructs only allowed variants (no lifecycle events).
- No forge/stake dependency; the memory run is follow/serve/admit only.
- No `OP-MEM-01` flip without the committed, sha256-bound, replay-`agreed` transcript.

---

## 15. Explicit Non-Goals
- `CN-MEM-01 partial→enforced` (MEM-BOUND-B).
- Haskell-node RSS comparison (MEM-COMPARE-D).
- Sustained >k + epoch-transition run (MEM-STRESS-C); A2 is a bounded operator pass exercising the points.
- Any change to block/tx validity, chain selection, persisted bytes, or protocol semantics.

---

## 16. Completion Checklist
- [ ] Closed-vocab extension + gates green hermetically.
- [ ] Live transcript committed + sha256-bound + replay `agreed` + no starvation.
- [ ] `OP-MEM-01 declared→partial` recorded with the artifact as evidence.
- [ ] BLUE untouched; per-slice security review clean.
