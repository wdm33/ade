# Slice MEM-OPT-UTXO-DISK S0 — active-admission owned-footprint diagnostic (CE-UD-0)

> **Status:** Scoped — **diagnostic only (measurement, NOT a redesign).** Classifies the ~4.6 GiB active-admission owned footprint as `serialization_transient` | `live_working_set` | `mixed` via a phase-resolved owned-RSS timeline (t1–t4) + a RED-only forced-reclaim probe. Decides the cluster's structural direction.
> **Cluster:** MEM-OPT-UTXO-DISK (primary invariant `DC-MEM-05` + `DC-MEM-07`)
> **Cluster doc:** `docs/clusters/MEM-OPT-UTXO-DISK/cluster.md` · **Prior:** MEM-OPT-OPS S3 (closed `e0c77492`)

## 2. Slice Header

### Cluster Exit Criteria Addressed
- [ ] **CE-UD-0** (active-admission owned-footprint diagnostic): a committed run reproducing the S3 scenario (same seed, same recovered anchor, same `initial_ledger_fp` `fb7cb12a…`, same replay verdict `agreed`) + a phase-resolved owned-memory artifact (t1–t4 `RssAnon`/`Private_Dirty` + the t3 pre/post-reclaim delta) + an explicit classification ∈ {`serialization_transient`, `live_working_set`, `mixed`} + a next-slice recommendation derived from the classification. RED-only reclaim probe; no BLUE.

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
  - **t4 point `sustained`:** sampled during ongoing block admission (the existing S3 active-admission window).
- **Classification rule:**
  - **`serialization_transient`:** t3 (post-reclaim) drops sharply toward t1/idle (~1.95–2.0 GiB) AND the high owned is localized to t2 ⇒ the ~4.6 GiB is reclaimable serialization. *Next slice: a contained `seed_to_snapshot` serialization-streaming fix.*
  - **`live_working_set`:** t3 stays high (~4.6 GiB) AND t4 sustains it ⇒ the UTxO is genuinely resident. *Next slice: the full bounded in-memory UTxO backend (redb `TxIn→TxOut` + bounded cache + k-deep changelog).*
  - **`mixed`:** t3 drops partially (a transient component reclaims; a working-set component persists above idle but below the t2 peak) ⇒ BOTH levers apply. *Next slice: scope both; sequence the cheaper (serialization-streaming) first.*
- The classification + the next-slice recommendation are recorded in the evidence + the slice close.

## 10. Mechanical Acceptance Criteria
- [ ] Phase-resolved owned sampling (t1–t4) wired, RED-only, fail-soft; the run still reproduces the S3 scenario.
- [ ] The forced-reclaim probe runs at t3, diagnostic-only (off the authoritative path), with the pre/post-reclaim owned delta recorded.
- [ ] **Same seed, same recovered anchor, same `initial_ledger_fp` (`fb7cb12a…`), same replay verdict (`agreed`)** as S3 — the diagnostic does not perturb authority.
- [ ] A committed **phase-resolved owned-memory artifact** (the four taps + the t3 reclaim delta), clearly labeled.
- [ ] An **explicit classification** recorded: exactly one of {`serialization_transient`, `live_working_set`, `mixed`}.
- [ ] A **next-slice recommendation** derived from the classification.
- [ ] `ci/ci_check_mem_opt_utxo_disk_s0.sh` green (schema + classification ∈ the 3 closed values + replay pairing) + `--self-test`.
- [ ] `cargo test -p ade_node` green; the diagnostic cannot influence authoritative behavior.

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
