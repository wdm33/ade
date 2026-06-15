# Slice MEM-OPT-OPS S3 — owned-footprint measurement + honest owned comparison (CE-OPS-3)

> **Status:** Merged — owned sampler (RssAnon + Private_Dirty) + evidence + honest owned comparison. **Outcome: `OP-MEM-02` STAYS `declared`** — owned RssAnon during active admission (4.59 GiB p50) is ABOVE Haskell's windowed owned (2.57 GiB) → verdict `ade_heavier`; Ade idle owned (1.95 GiB) is below. MEM-OPT-OPS alone does NOT clear the preprod owned posture; the snapshot/`chain.db` serialization → MEM-OPT-UTXO-DISK.
> **Cluster:** MEM-OPT-OPS (primary invariant `OP-MEM-02`)
> **Cluster doc:** `docs/clusters/MEM-OPT-OPS/cluster.md` · **Prior:** S1 (`861757f4`), S2 (`54975bb0`)

## 2. Slice Header

### Cluster Exit Criteria Addressed
- [x] **CE-OPS-3** (owned-RSS measurement + honest comparison): `rss_sampler` records the **owned** footprint (`RssAnon`, `Private_Dirty`) alongside gross `VmRSS`; the evidence carries both, clearly labeled; `ci_check_mem_opt_s3_owned.sh` validates the schema + the honest verdict; the S2 scenario is re-measured (same seed, `initial_ledger_fp` == `fb7cb12a…`, replay `agreed`); the comparison verdict is regenerated honestly. **MET 2026-06-15** — `docs/evidence/mem-opt-ops-s3-owned-preprod-memory.{jsonl,md}` (sha256 `f50bf97c…`) + `…-owned-compare-preprod.jsonl`. **The measurement is the deliverable; the OUTCOME is `ade_heavier`** (owned admission 4.59 GiB > Haskell 2.57 GiB), so `OP-MEM-02` is NOT flipped (honest — owned not clearly below).

### Intent
Replace the misleading **gross**-memory comparison with an **owned**-footprint comparison. Gross `VmRSS` is polluted by the `chain.db` mmap (file-backed, reclaimable; S2 finding). The real question OP-MEM-02 asks is: *how much memory does Ade actually OWN after import + recovery + snapshot + steady run?* — i.e. the anonymous heap, not the mmap'd file pages.

## 3. Primary Metric
- **Owned (the gated metric): `RssAnon`** (`/proc/self/status`) — the anonymous resident heap; **excludes** the file-backed `chain.db` mmap. Readable for BOTH Ade and the Haskell node (status is not ptrace-protected), so it is the apples-to-apples owned-comparison metric.
- **Owned (Ade-self, informational): `Private_Dirty`** (`/proc/self/smaps_rollup`) — private dirty pages. Readable only for the own process (smaps_rollup is ptrace-protected), so it is reported for Ade but NOT used in the cross-node comparison.
- **Gross `VmRSS`: informational only** — the mmap'd `chain.db` inflates it without representing owned heap pressure.

## 4. Tier Classification (per the slice mandate)
- **true:** replay outputs remain byte-identical; the memory measurement CANNOT influence BLUE. The S3 re-measurement reproduces the S2 scenario's `initial_ledger_fp` + replay verdict exactly.
- **derived (`DC-MEM-06`):** preserved through the canonical-fingerprint pairing (the re-measured import is byte-identical; the owned sampler never enters a fingerprint).
- **operational (`OP-MEM-02`):** the target. Flip from `declared` **only if** the owned metric (`RssAnon`) is **clearly below** the invariant's target (≤ 3.0 GB owned, aim 2.0–2.5) AND below the reference node's owned footprint.
- **release:** a CI gate validates the owned sampler + the evidence schema (vacuous-until-committed; `--self-test`).

## 5. Scope
- **`crates/ade_node/src/mem_measure/rss_sampler.rs`** (RED): `sample_rss_anon_kib()` (status `RssAnon`) + `sample_private_dirty_kib()` (smaps_rollup `Private_Dirty`). Fail-soft (`None` off-Linux / unreadable).
- **Evidence (GREEN/RED):** `MemoryMeasure` + `MemorySummary` gain owned fields (`rss_anon_kib`, `private_dirty_kib` per-point; owned `RssAnon`/`Private_Dirty` p50/peak in the summary) alongside the existing gross fields — both clearly labeled. Plumbed through `admission_log/{event,writer}.rs`, `convergence_evidence.rs`, `admission/runner.rs`.
- **New gate** `ci/ci_check_mem_opt_s3_owned.sh`: validates the owned-evidence schema + the regenerated owned-comparison verdict; vacuous-until-committed + `--self-test`.
- **Evidence artifacts:** `docs/evidence/mem-opt-ops-s3-owned-preprod-memory.{jsonl,md}` (Ade re-measurement, gross + owned) + the regenerated owned comparison vs the Haskell node's `RssAnon`.
- **Out of scope:** the snapshot/`chain.db` serialization transient — S3 MEASURES it correctly (owned excludes the mmap) but does NOT fix it (that is MEM-OPT-UTXO-DISK). No BLUE change.

## 6. Execution Boundary
- **BLUE:** none.
- **GREEN:** the owned percentile/evidence-schema logic + the gate.
- **RED:** the `/proc/self/{status,smaps_rollup}` reads + the sampling wiring. Observational only; never feeds authority (the S3 re-measurement's replay verdict, not RSS, is the validity gate).

## 7. Invariants Preserved
- Replay-equivalence (`DC-WAL-03`): the re-measurement reproduces the S2 scenario's post-state + `initial_ledger_fp` (`fb7cb12a…`) + replay verdict `agreed`.
- `DC-MEM-06`: the owned sampler never enters a fingerprint; the import is byte-identical.
- The A2 discipline: the run still emits `memory_summary{replay_verdict=agreed}`; a lower-memory run that diverges is invalid.

## 8. Invariants Strengthened
- **`OP-MEM-02`:** measured on the OWNED metric for the first time. Flip from `declared` ONLY if `RssAnon` clearly below the ≤3 GB target AND below the Haskell reference's `RssAnon`. (`VmRSS < Haskell` is a useful signal; `owned < threshold` is the OP-MEM-02 evidence.)

## 9. Design Summary
- `rss_sampler`: two new fail-soft readers — `sample_rss_anon_kib()` parses `RssAnon:` from `/proc/self/status`; `sample_private_dirty_kib()` sums/reads `Private_Dirty:` from `/proc/self/smaps_rollup`. The generic `RssWindow` (nearest-rank p50/p95/peak) is reused for owned samples.
- The admission runner maintains parallel windows (gross `VmRSS`, owned `RssAnon`, owned `Private_Dirty`); each memory point samples all three; the summary emits all three windows' p50/peak, clearly labeled.
- **Comparison (honest regeneration):** Ade `RssAnon` (from the S3 transcript) vs the Haskell `cardano-node-preprod` `RssAnon` (sampled from `/proc/<pid>/status`, same nearest-rank window as MEM-COMPARE-D). The verdict (`ade_below` / `ade_heavier`) is computed on the OWNED metric. Gross `VmRSS` reported alongside for transparency.

## 10. Mechanical Acceptance Criteria
- [x] `rss_sampler` records the owned footprint (`RssAnon` + `Private_Dirty`); fail-soft; RED-only (`owned_samplers_present_on_linux`).
- [x] Evidence carries BOTH gross (`rss_kib`/`rss_hwm_kib`) and owned (`rss_anon_kib`/`private_dirty_kib` + owned summary percentiles), clearly labeled; round-trips through the writer (`admission_log_writer_emits_memory_events`).
- [x] `cargo test -p ade_node` green (310 lib + all binaries); the sampler cannot influence authoritative behavior.
- [x] **Re-measurement (CE-OPS-3):** committed S3 transcript — same seed, `initial_ledger_fp` == S1/S2's `fb7cb12a…`, replay `agreed`, 0 diverged; carries owned `RssAnon`/`Private_Dirty`.
- [x] **Honest comparison:** the owned verdict regenerated + committed (Ade `RssAnon` p50 4.59 GiB vs Haskell p50 2.57 GiB → `ade_heavier`); `ci/ci_check_mem_opt_s3_owned.sh` green (+ `--self-test`).
- [x] **`OP-MEM-02`:** STAYS `declared` — `RssAnon` during active admission (4.59 GiB) is NOT clearly below the ≤3 GB target nor below Haskell's owned (2.57 GiB); the residual points at MEM-OPT-UTXO-DISK (the `seed_to_snapshot`/`chain.db` serialization). (Ade idle owned 1.95 GiB IS below — the import-side wins are real.)

## 11. Hard Prohibitions
Inherits the cluster's "Forbidden During This Cluster". Slice-specific:
- No BLUE change; the sampler is observational RED, never authority.
- **No `OP-MEM-02` flip unless the OWNED metric is clearly below the written target** (not gross VmRSS, not "below Haskell on VmRSS" alone).
- No attempt to FIX the snapshot/`chain.db` transient (measure only; the fix is MEM-OPT-UTXO-DISK).
- The owned comparison must be honest: report gross alongside owned; do not cherry-pick the favorable metric.

## 12. Explicit Non-Goals
- Not the on-disk UTxO / snapshot-write optimization (MEM-OPT-UTXO-DISK).
- Not compact TxOut (MEM-OPT-COMPACT).
- No seed-format change; no new CLI flag.

## 13. Completion Checklist
- [ ] Owned sampler + evidence schema (both metrics, labeled) + gate.
- [ ] S3 re-measurement: same scenario (fp == fb7cb12a), replay agreed, owned recorded.
- [ ] Owned comparison verdict regenerated honestly (Ade RssAnon vs Haskell RssAnon).
- [ ] `OP-MEM-02` flip decision made on the owned metric per §10.
