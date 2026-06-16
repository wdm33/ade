# Slice MEM-OPT-UTXO-DISK S0 — active-admission owned-footprint diagnostic (CE-UD-0)

> **Status:** DONE — **verdict `bootstrap_transient_but_admission_live_working_set`.** The forced collect at t3 returns the entire bootstrap transient to ~idle (the `seed_to_snapshot` serialization is fully reclaimable — overturns the original hypothesis); the active-admission footprint (4.59 GiB) **RE-ACCUMULATES the very next block** after a forced collect during admission (t5: 4.59 → 1.78 → 4.59) ⇒ it is a **LIVE working set**. **Next slice: the on-disk / bounded in-memory UTxO backend (`DC-MEM-05` + `DC-MEM-07`).** Evidence: `docs/evidence/mem-opt-utxo-disk-s0-{phase-timeline,classification}-preprod.*`.
> **Cluster:** MEM-OPT-UTXO-DISK (primary invariant `DC-MEM-05` + `DC-MEM-07`)
> **Cluster doc:** `docs/clusters/MEM-OPT-UTXO-DISK/cluster.md` · **Prior:** MEM-OPT-OPS S3 (closed `e0c77492`)

## 2. Slice Header

### Cluster Exit Criteria Addressed
- [x] **CE-UD-0** (active-admission owned-footprint diagnostic): **MET.** Committed live run (`fb7cb12a…`, replay `agreed`, 0 diverged, 34 admits) carrying the t1–t5 phase timeline + the honest verdict `bootstrap_transient_but_admission_live_working_set` + the next-slice recommendation (on-disk / bounded in-memory UTxO backend). The forced collects (t3 post-snapshot, t5 during admission) are the quarantined RED `ade_mem_diag` probe; no BLUE. Evidence + gates: `docs/evidence/mem-opt-utxo-disk-s0-{phase-timeline,classification}-preprod.*`, `ci/ci_check_mem_opt_utxo_disk_s0.sh` + `ci/ci_check_mem_diag_quarantine.sh`.

### Intent
MEM-OPT-OPS S3 established that Ade's active-admission owned `RssAnon` (4.59 GiB p50 / 4.76 GiB peak) is **above** Haskell's owned (2.57 GiB), while Ade **idle** owned (1.95 GiB) is **below**. mimalloc (S1) already returns freed pages to the OS, yet the active footprint stays high. The question this slice answers — **before any structural redesign** — is whether that ~4.6 GiB is **retained-freed serialization memory** (the `seed_to_snapshot` transient, reclaimable), **live required working set** (the UTxO genuinely resident in heap), or **mixed**. The classification decides whether the cluster's next mergeable work is a contained snapshot-streaming fix or the full bounded in-memory UTxO backend. **S0 measures and classifies; it does not fix.**

## 3. Primary Metric
- **The classified signal is the SHAPE of the owned timeline across phases + the response to a forced reclaim — not a single magnitude.** Owned = `RssAnon` (`/proc/self/status`) + `Private_Dirty` (`/proc/self/smaps_rollup`), sampled across the four phase boundaries, reusing the S3 samplers.
- **`allocator_stats` (mimalloc): informational ONLY.** Recorded only if essentially free + non-invasive; **NEVER a merge gate** unless the phase verdict is ambiguous (`mixed`-by-uncertainty). Heap-region attribution is explicitly NOT required to answer the decision question.
- Gross `VmRSS`: informational alongside (it carries the `chain.db` mmap; not the owned signal).

## 4. Tier Classification
- **true:** replay outputs remain byte-identical; the diagnostic CANNOT influence BLUE. The run reproduces the S3 scenario's `initial_ledger_fp` + replay verdict exactly. The forced-reclaim probe runs AFTER the authoritative snapshot/admission work and never feeds it.
- **derived (`DC-MEM-05`):** the diagnostic does NOT change the storage backend ⇒ trivially backend-identical (same fingerprint as S3). It MEASURES the surface the structural slices will change.
- **operational (`OP-MEM-02`):** the diagnostic informs the lever; it does NOT flip `OP-MEM-02` (stays `declared`).
- **release:** a CI gate validates the S0 schema (phase taps + classification ∈ the 3 closed values + replay pairing) — vacuous-until-committed + `--self-test`.

## 5. Scope
- **`crates/ade_node/src/admission/{bootstrap,runner}.rs` (RED):** under the `ADE_MEM_PHASE_DIAGNOSTIC` env toggle (absent on every normal run), the bootstrap captures owned (`RssAnon`/`Private_Dirty`) at the two extra phase boundaries and the runner emits them as closed `memory_measure` points. The four phases (reusing the existing point mechanism): `seed_import` (t1, the existing post-`import()` tap), `t2_snapshot_serializing` (t2, right after `seed_to_snapshot`), `t3_after_forced_allocator_collect_diagnostic_only` (t3), `sustained` (t4, ongoing admission — the existing S3 active-admission window).
- **The forced-reclaim probe — QUARANTINED (RED, diagnostic):** the one `unsafe` FFI call (`mi_collect(force=true)`) lives in a tiny dedicated crate **`ade_mem_diag`** (`force_allocator_collect_for_diagnostic_only`), so `ade_node` keeps `#![deny(unsafe_code)]` with ZERO local exceptions. Invoked at t3 only behind the env toggle; it returns freed-but-retained pages to the OS, then owned is re-sampled. **The t2→t3 owned delta is the decisive control** (mimalloc's lazy `MADV_FREE` keeps freed pages resident, so without the forced collect retained-freed and live memory are indistinguishable).
- **Evidence (GREEN/RED):** the four phase taps' `RssAnon`/`Private_Dirty` (the closed `memory_measure` points) + a separate classification record carrying the verdict + the next-slice recommendation.
- **New gates:** `ci/ci_check_mem_opt_utxo_disk_s0.sh` (validates the 4-phase timeline + the classification ∈ {`serialization_transient`, `live_working_set`, `mixed`} + that the classification is HONEST — consistent with the t2→t3 reclaim the numbers show — + replay `agreed`; vacuous-until-committed + `--self-test`) and `ci/ci_check_mem_diag_quarantine.sh` (enforces the quarantine: `ade_node` stays `#![deny(unsafe_code)]` zero-allows; `ade_mem_diag` dep'd only by `ade_node`; the collect gated by `ADE_MEM_PHASE_DIAGNOSTIC`). The 2 new points are added to the closed POINTS vocabulary in `ci_check_mem_measure_evidence.sh`.
- **Evidence artifacts:** `docs/evidence/mem-opt-utxo-disk-s0-phase-timeline-preprod.{jsonl,md}` (the timeline) + `…-classification.{jsonl,md}` (the verdict + next-slice).
- **Out of scope:** ANY storage change, ANY snapshot-streaming fix, ANY on-disk UTxO. S0 measures + classifies only.

## 6. Execution Boundary
- **BLUE:** none.
- **GREEN:** the phase-marker / classification logic + the evidence schema + the gate.
- **RED:** the owned sampling, the phase taps, and the forced-reclaim probe — the latter quarantined in the dedicated `ade_mem_diag` crate (the workspace's sole unsafe-FFI surface; `ade_node` stays zero-unsafe, CI-enforced). Diagnostic-only, off the authoritative path; never feeds authority — the run's replay verdict, not RSS, is the validity gate.

## 7. Invariants Preserved
- **Replay-equivalence (`DC-WAL-03`):** the run reproduces the S3 scenario's post-state + `initial_ledger_fp` (`fb7cb12a…`) + replay verdict `agreed`. The phase taps + the reclaim probe do NOT perturb it.
- **`DC-MEM-05`:** the backend is UNCHANGED ⇒ trivially backend-identical (same fingerprint).
- **`DC-MEM-06`:** the diagnostic never enters a fingerprint.
- **The A2 discipline:** the run still emits `memory_summary{replay_verdict=agreed}`.
- **The RED-only reclaim guardrail (standing user guardrail):** the forced reclaim is diagnostic — NOT on the authoritative admission path, NOT authoritative behavior, NOT a hidden dependency for passing BA-08. Production memory-release, if ever needed, is a separate operational/runtime-policy scope, not BLUE semantics.

## 8. Invariants Strengthened
- **None mechanically in S0** (it is diagnostic). It SCOPES the eventual strengthening of `DC-MEM-05`/`DC-MEM-06`/`DC-MEM-07` by the structural slices.

## 9. Design Summary
- A **phase label** is threaded through the owned sampling. The periodic owned sampler (from S3) keeps running; the admission bootstrap emits phase-boundary markers so the owned timeline buckets by phase:
  - **t1 point `seed_import`:** sampled right after `import()` returns (the existing S2 tap), before `seed_to_snapshot`. Owned ≈ baseline + the parsed UTxO map (~1–1.3 GiB).
  - **t2 point `t2_snapshot_serializing`:** owned right after `seed_to_snapshot` returns (the run-end gross VmHWM 7.79 GiB territory; the owned peak 4.76 GiB is the suspect).
  - **t3 point `t3_after_forced_allocator_collect_diagnostic_only`:** after the snapshot write, invoke the quarantined RED probe (`ade_mem_diag::force_allocator_collect_for_diagnostic_only` → `mi_collect(force=true)`), then sample owned. **The t2→t3 delta is the decisive control.**
  - **t4 point `sustained` / `mempool_admission`:** owned during ongoing block admission (the S3 active-admission level). The first `mempool_admission` is the admission STEP.
  - **t5 point `t5_active_admission_after_forced_collect`:** in the runner loop, after ≥10 stable admits, invoke the quarantined RED probe ONCE, then sample owned; admits then continue (post-t5 re-sampling whether owned re-accumulates or stays low). **t5 is the decisive control for the admission-time footprint** — t3 only probed the bootstrap.
- **Verdict rule** (t3 = the post-bootstrap-collect idle baseline; t4 = the active level; near-idle = within 15% of t3):
  - **`retained_transient_bootstrap_and_admission`:** t3 drops to idle AND t5 drops near idle ⇒ BOTH the bootstrap serialization AND the admission step are reclaimable. *Next slice: a contained admission-loop allocation cleanup.*
  - **`bootstrap_transient_but_admission_live_working_set`:** t3 drops to idle BUT t5 stays high (near t4) ⇒ the admission step is live working set. *Next slice: the full bounded in-memory / on-disk UTxO backend (redb `TxIn→TxOut` + bounded cache + k-deep changelog).*
  - **`mixed`:** t3 partially drops AND t5 partially drops ⇒ BOTH levers apply. *Next slice: scope both; sequence the cheaper first.*
- The verdict + the next-slice recommendation are recorded in the evidence + the slice close.

## 10. Mechanical Acceptance Criteria
- [ ] Phase-resolved owned sampling (t1, t2, t3, first admission step, t5, post-t5 admits) wired, RED-only, fail-soft; the run reproduces the S3 scenario.
- [ ] The forced-collect probe runs at t3 (post-snapshot) AND t5 (during active admission, after ≥10 admits), diagnostic-only (off the authoritative path), owned values recorded.
- [ ] **Same seed, same recovered anchor, same `initial_ledger_fp` (`fb7cb12a…`), same replay verdict (`agreed`), 0 diverged** — the diagnostic does not perturb authority.
- [ ] A committed **phase-resolved owned-memory artifact** (t1–t5 + ≥12 `mempool_admission` samples), clearly labeled.
- [ ] An **explicit verdict** recorded: exactly one of {`retained_transient_bootstrap_and_admission`, `bootstrap_transient_but_admission_live_working_set`, `mixed`}, **derived from the t3/t4/t5 behavior** (honest — consistent with the numbers).
- [ ] A **next-slice recommendation** derived from the verdict.
- [ ] `ci/ci_check_mem_opt_utxo_disk_s0.sh` + `ci/ci_check_mem_diag_quarantine.sh` green + `--self-test`.
- [ ] `cargo test -p ade_node -p ade_mem_diag` green; the diagnostic cannot influence authoritative behavior.

## 11. Hard Prohibitions
- No BLUE change; the sampler + reclaim probe are observational/diagnostic RED, never authority.
- **The forced reclaim is RED-only diagnostic — NEVER authoritative behavior, NEVER a hidden BA-08 dependency. Production memory-release is a SEPARATE operational/runtime-policy scope, not BLUE semantics.** *(Standing user guardrail.)*
- No storage change / no snapshot-streaming fix / no on-disk UTxO in S0 — measure + classify only.
- `allocator_stats` (if recorded) is informational; it does NOT gate the merge unless the phase verdict is ambiguous.
- No `OP-MEM-02` flip.
- No seed-format change; no new authoritative CLI flag (a diagnostic flag for the reclaim path is RED-only, off the authoritative path).

## 12. Explicit Non-Goals
- Not the `seed_to_snapshot` serialization-streaming fix (a candidate NEXT slice, S0-gated).
- Not the on-disk UTxO backend (a candidate NEXT slice, S0-gated).
- Not compact TxOut (MEM-OPT-COMPACT).
- Not a production memory-release policy.

## 13. Completion Checklist
- [ ] Phase-resolved owned sampling (t1–t4) + the t3 forced-reclaim probe (RED, diagnostic-only).
- [ ] S0 run: same scenario (`fp == fb7cb12a…`), replay `agreed`, phase timeline recorded.
- [ ] Classification {`serialization_transient` | `live_working_set` | `mixed`} recorded with the supporting timeline.
- [ ] Next-slice recommendation derived + recorded.
- [ ] Gate + `--self-test` green; `cargo test -p ade_node` green.
