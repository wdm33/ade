# Cluster MEM-OPT-OPS — operational memory quick-wins (allocator + streaming import + owned-RSS measure)

**Primary invariant:** `OP-MEM-02` (Ade's owned resident memory stays clearly below the reference Haskell node's, with NO change to ledger semantics, chain selection, persisted bytes, or replay-equivalence).
**Status:** **S1 (ALLOC) + S2 (IMPORT) + S3 (MEASURE) all MERGED — ready for cluster-close.** No BLUE change. S1: VmRSS p50 6,874,024 → 4,824,884 kiB (−29.8%). S2: streaming import halves the import peak (seed_import VmHWM 6.56 → 3.25 GiB), byte-identical. **S3 (the decisive owned measurement): the gross-VmRSS signal was MISLEADING. On the OWNED metric (RssAnon), Ade idle is 1.95 GiB (below target — the import-side wins are real), but active-admission owned is 4.59 GiB (p50) — ABOVE the Haskell node's windowed owned (2.57 GiB) → verdict `ade_heavier`.** `OP-MEM-02` STAYS `declared` (owned NOT clearly below — honest, no flip). **Conclusion: MEM-OPT-OPS does NOT clear the preprod owned posture; the dominant owned cost is the `seed_to_snapshot`/`chain.db` serialization during admission → MEM-OPT-UTXO-DISK is required** (it was the "mainstream-scalable floor"; it is in fact the gating lever).
**Cluster split:** see `docs/planning/mem-opt-cluster-plan.md` §3. Grounding: `docs/planning/mem-opt-grounding.md`.

## 1. Primary Invariant
**`OP-MEM-02`** — owned RSS (`Private_Dirty`/`RssAnon`) under a representative preprod follow stays clearly below Haskell `cardano-node-preprod` (baseline 5.50 GB; target ≤ 3 GB owned, aim 2.0–2.5). This cluster banks the cheapest reduction: the ~4 GB of **retained transient import memory** (the seed import peaks at ~6.8 GB, then glibc malloc pins the freed pages — `mem-opt-grounding.md §A`). Every lever is a representation/runtime change that **never** alters an authoritative output; each run still emits `memory_summary{replay_verdict=agreed}` (the A2 discipline).

## 2. Normative Anchors
- Registry: `OP-MEM-02` (primary), `DC-MEM-06` (allocator determinism-neutral), `DC-WAL-03` (replay-equivalence), `DC-MEM-04` (ingress replay), the A1/A2 floor (`CN-MEM-01`/`OP-MEM-01`).
- Grounding: `docs/planning/mem-opt-grounding.md` (§A Ade memory map, §C levers, §D target). Plan: `docs/planning/mem-opt-cluster-plan.md`.
- Baseline evidence: `docs/evidence/mem-compare-d-preprod.{md,jsonl}` (MEM-COMPARE-D, `51884a78`): Ade 6.56 GB vs Haskell 5.50 GB, verdict `ade_heavier`.

## 3. Entry Conditions (prior work guarantees)
- **The measurement substrate exists (A1/A2):** `ade_node::mem_measure::rss_sampler` (the RED `/proc` sampler + nearest-rank percentiles), the closed `memory_measure`/`memory_summary` evidence vocab (`--mode admission` + `--mode node`), the RSS↔replay-verdict pairing, and `ci/ci_check_mem_measure_evidence.sh`. This cluster re-uses them to prove each lever is a *reduction* with replay verdict `agreed`.
- **The comparison + its honesty gate exist (MEM-COMPARE-D):** `ci/ci_check_mem_compare_evidence.sh` (mechanical `ade_heavier`/`ade_below` verdict). S3 extends this into an enforced ceiling.
- **The import is a single chokepoint:** `crates/ade_runtime/src/seed_import/importer.rs` is the SOLE seed-import authority — S2 changes one site.
- **No allocator is configured:** default glibc `System` (verified: zero jemalloc/mimalloc references in-tree).

## 4. What Changes (slices)
- **S1 — ALLOC (GREEN/RED; the "tiny win", rolled in here).** Add a `#[global_allocator]` — **mimalloc** (or `tikv-jemallocator` tuned: `background_thread:true`, short `dirty_decay_ms`/`muzzy_decay_ms`, low `narenas`) — so freed pages (the retained import peak) return to the OS. **Determinism-neutral** (`DC-MEM-06`: allocation addresses are never fingerprinted; a CI grep asserts no allocator type enters a fingerprint). Re-run the preprod measurement → expect owned RSS 6.56 → ~3–4 GB. *(`OP-MEM-02`.)*
- **S2 — IMPORT (RED).** Stream the seed import: `serde_json::from_reader` over the file (not `from_slice` on a 3.8 GB buffer) and build the canonical UTxO `BTreeMap` incrementally, never holding the intermediate `RawUtxoMap` + the JSON buffer simultaneously. Removes the ~6.8 GB peak so RSS never spikes. The imported UTxO state is byte-identical (same final fingerprint) — a replay-equivalence obligation. *(`OP-MEM-02`, preserves `DC-WAL-03`.)*
- **S3 — MEASURE (GREEN/RED).** Extend `rss_sampler` to read the **owned** footprint (`Private_Dirty`/`RssAnon` from `/proc/self/smaps_rollup`) alongside `VmRSS`; add a closed `memory_owned_kib` evidence field; and add `ci/ci_check_mem_rss_ceiling.sh` (vacuous-until-committed) that asserts a committed run's owned RSS ≤ a fixed target and flips the comparison verdict toward `ade_below`. Turns the snapshot into an enforced ceiling + a regression guard. *(`OP-MEM-02`, reuses A1/A2.)*

## 5. Exit Criteria (CE — each CI-verifiable)
- **CE-OPS-1 (`OP-MEM-02`, ALLOC) [S1]: ✅ MET (2026-06-15).** Committed preprod transcript with the allocator swapped shows RSS **strictly below** the MEM-MEASURE-A2 baseline (VmRSS p50/peak 6,874,024/6,874,028 → 4,824,884/4,824,976 kiB, −29.8%), `memory_summary{replay_verdict=agreed}`, 0 diverged. `ci_check_mem_measure_evidence.sh` + `ci_check_mem_opt_s1_reduction.sh` + the determinism-neutral allocator gate green; `cargo test -p ade_node` green. Evidence: `docs/evidence/mem-opt-ops-s1-alloc-preprod-memory.{jsonl,md}`. (Metric is VmRSS — the owned `Private_Dirty` refinement is S3; the comparison-verdict flip is S3.)
- **CE-OPS-2 (`OP-MEM-02`, IMPORT) [S2]: ✅ MET (2026-06-15).** Committed streaming-import run shows a reduced **import peak** — the `seed_import` VmHWM (captured right after `import()`, before the snapshot write) **3,405,288 < whole-buffer 6,874,028 kiB (−50.5%)** — the imported UTxO fingerprint byte-identical (`initial_ledger_fp_hex` == S1's `fb7cb12a…`; hermetic 10-fixture equivalence test), replay verdict `agreed`, 0 diverged. Evidence: `docs/evidence/mem-opt-ops-s2-import-preprod-memory.{jsonl,md}`; gates `ci_check_mem_opt_s2_import_peak.sh` + `ci_check_mem_measure_evidence.sh`. (The *run-end* VmHWM is the separate chain.db snapshot transient — §10.)
- **CE-OPS-3 (owned-RSS measurement + honest comparison) [S3]: ✅ MET (2026-06-15).** `rss_sampler` records the owned footprint (RssAnon + Private_Dirty) alongside gross VmRSS; the S3 transcript carries both; `ci_check_mem_opt_s3_owned.sh` validates the owned schema + the honest verdict; the S2 scenario was re-measured (`initial_ledger_fp` == `fb7cb12a…`, replay `agreed`). **The measurement is the deliverable; the OUTCOME is `ade_heavier` on the owned metric** (admission owned 4.59 GiB > Haskell 2.57 GiB), so the owned-RSS ceiling is NOT cleared and `OP-MEM-02` stays `declared`. Evidence: `docs/evidence/mem-opt-ops-s3-owned-preprod-memory.{jsonl,md}` + `…-owned-compare-preprod.jsonl`. (Originally scoped as a `ci_check_mem_rss_ceiling.sh` ceiling-pass; the honest owned measurement showed the ceiling is not met, so the gate is the schema+honest-verdict check, not a vacuous green ceiling.)
- **CE-OPS-close [/cluster-close]:** `OP-MEM-02` records the new operational standing (declared→partial if clearly-below is demonstrated by the cheap levers, else strengthened with the residual gap pointing at MEM-OPT-UTXO-DISK); grounding docs refreshed.

## 6. Expected Slices
- **S1** ALLOC — CE-OPS-1 — GREEN/RED. **Lands first.**
- **S2** streaming import — CE-OPS-2 — RED (`importer.rs`).
- **S3** owned-RSS measure + ceiling gate — CE-OPS-3 — GREEN/RED.

## 7. TCB Color Map
- **BLUE:** none. The ledger, UTxO semantics, and fingerprints are untouched.
- **GREEN:** the owned-RSS percentile/ceiling logic + the evidence vocab extension.
- **RED:** the global allocator, the streaming importer (`importer.rs`), the `/proc/smaps_rollup` read.
- **Affected gates:** new `ci_check_mem_rss_ceiling.sh` + an allocator-determinism-neutrality grep; reused `ci_check_mem_measure_evidence.sh`, `ci_check_mem_compare_evidence.sh`. Stay green — the replay corpus (`DC-WAL-03`), `ci_check_mempool_ingress_replay.sh`.

## 8. Forbidden During This Cluster
1. **No BLUE change** — no ledger/UTxO/fingerprint semantics touched; this cluster is allocator + import-shell + measurement only.
2. **No allocator type in any authoritative fingerprint** (`DC-MEM-06`) — allocation addresses/sizes never enter a hash.
3. **No semantic feature flag / config switch** altering authority; the allocator + the import are not behavior-selectable per run.
4. **No memory win that changes the imported UTxO bytes** — the streaming import must produce the byte-identical UTxO fingerprint (else it is a consensus change, not a memory win).
5. **RSS magnitude never gates an authoritative output** (the A2 discipline); a lower-memory run that diverges is INVALID evidence.

## 9. Replay Obligations
Each lever is a representation/runtime change with a replay-equivalence obligation: the streaming import (S2) must yield the byte-identical UTxO/ledger fingerprint as the non-streaming import (hermetic test); the allocator (S1) cannot affect any fingerprint (`DC-MEM-06`); every committed measurement run must carry `memory_summary{replay_verdict=agreed}`. No new canonical/persisted type.

## 10. Open Questions
- **OQ-OPS-1 (allocator choice):** mimalloc vs tuned `tikv-jemallocator`. *Lean: benchmark both in S1; pick by owned-RSS + decay behavior; mimalloc is the simpler default.*
- **OQ-OPS-2 (does cheap clear 5.50 GB?):** if ALLOC+IMPORT alone put owned RSS clearly below 5.50 GB, MEM-OPT-UTXO-DISK becomes the "mainnet-scalable floor" rather than a prerequisite to win preprod. **S1 finding:** ALLOC *alone* (no IMPORT yet) already put **VmRSS** at 4.60 GiB — below the 5.50 GiB Haskell reference, and the post-import idle footprint was **2.32 GiB** (the live structures, page-cache-free). Strong signal the cheap levers clear preprod; UTXO-DISK is likely the mainnet-scalable floor, not a preprod-win prerequisite. **Caveat:** this is VmRSS; the *owned* metric (S3) is expected lower still. **Re-measure owned RSS after S2/S3 before scoping UTXO-DISK depth.**
- **OQ-OPS-3 (ceiling target):** the fixed owned-RSS ceiling for `ci_check_mem_rss_ceiling.sh` — set from the post-S2 measurement with margin (e.g. ≤ 3.0 GB), not pre-committed.
- **OQ-OPS-4 (NEW — S2 finding: the snapshot/chain.db transient is now the dominant peak).** With the import peak removed, the largest single memory event is `seed_to_snapshot` serializing the recovered 1.9M-entry UTxO into a ~4 GB `chain.db` (run-end VmHWM ~8 GiB), after which the redb `chain.db` mmaps into gross `VmRSS` (~6.9 GiB observed; clean/reclaimable, file-backed). This is **not** the import and is a **separate next target** — likely folds into **MEM-OPT-UTXO-DISK** (the on-disk UTxO redesigns how state is snapshotted) and is precisely why the *owned* footprint (`Private_Dirty`, S3) — which excludes the mmap'd `chain.db` — is the metric that matters, not gross `VmRSS`. *Lean: measure owned (S3) first; then scope the snapshot/serialization transient as part of, or just ahead of, MEM-OPT-UTXO-DISK.*

## 11. Cluster Close Record
*(Filled at `/cluster-close`, 2026-06-16; close commit sits on top of the review-fix `233644f7`.)*

**Verdict: CLOSED — every CE mechanically green; `OP-MEM-02` stays `declared` (honest no-flip).** MEM-OPT-OPS banked two real reductions (allocator + streaming import) and replaced the misleading gross-VmRSS comparison with an honest owned-footprint measurement that showed the owned posture is NOT cleared. The dominant owned cost is the `seed_to_snapshot`/`chain.db` serialization during admission — that is the next cluster's lever, not this one's.

### CE results (all ✅ green at close)
- **CE-OPS-1 (S1 ALLOC):** mimalloc `#[global_allocator]`; live preprod VmRSS p50 6,874,024 → 4,824,884 kiB (**−29.8%**), `replay_verdict=agreed`, 0 diverged. Gates: `ci_check_mem_opt_s1_reduction.sh`, `ci_check_alloc_determinism_neutral.sh`, `ci_check_mem_measure_evidence.sh`.
- **CE-OPS-2 (S2 IMPORT):** streaming `CanonicalUtxoSink`; `seed_import` VmHWM 6,874,028 → 3,405,288 kiB (**−50.5%**), imported UTxO fingerprint byte-identical (`initial_ledger_fp == fb7cb12a…`, 10-fixture hermetic equivalence), `replay_verdict=agreed`. Gate: `ci_check_mem_opt_s2_import_peak.sh`.
- **CE-OPS-3 (S3 MEASURE):** owned sampler (`RssAnon` + `Private_Dirty`); same S2 scenario re-measured (`initial_ledger_fp == fb7cb12a…`, replay `agreed`). **Outcome `ade_heavier`** — active-admission owned 4.59 GiB p50 > Haskell windowed owned 2.57 GiB; Ade *idle* owned 1.95 GiB IS below target. Gate: `ci_check_mem_opt_s3_owned.sh`.

### Decisive finding (why `OP-MEM-02` does NOT flip)
**Gross `VmRSS` and owned `RssAnon` are two different metrics of the *same* process; the gap between them is *opposite* for the two nodes, and that asymmetry is the finding.** (The familiar "over 5 GiB" Haskell number was always gross `VmRSS` from `/proc/<pid>/status`, per MEM-COMPARE-D — never an owned figure.) After S1+S2 the post-reduction *gross* reading looked like a BA-08 win — Ade gross `VmRSS` p50 4.60 GiB < Haskell gross `VmRSS` **5.50 GiB** — but that is MISLEADING:
- **Haskell:** the 5.50 GiB gross is ~half reclaimable **file-backed mmap** (on-disk UTxO + ImmutableDB), so its **owned** `RssAnon` is only **2.57 GiB** (settled p50; 3.95 GiB peak).
- **Ade:** gross ≈ owned — same-run 4.60 GiB `VmRSS` p50 vs **4.59 GiB `RssAnon` p50** (~7 MiB apart, i.e. ~100% anonymous) — because the `chain.db` cost is **anonymous owned heap** (redb write buffers + the `seed_to_snapshot` serialization of the 1.9M-entry UTxO), NOT the reclaimable mmap that S2's OQ-OPS-4 hypothesized.

So on the honest like-for-like **owned** metric Ade is **heavier, not lighter** (4.59 > 2.57) — the owned view makes Ade look WORSE than gross did, not better, because Ade holds nearly its entire footprint in anonymous heap while Haskell offloads ~half to disk. The cheap levers genuinely cut the import peak and the idle footprint (Ade **idle** owned 1.95 GiB < Haskell 2.57 GiB), but the **active-admission** owned floor (4.59 GiB p50, 4.76 GiB peak) sits above the Haskell node. **MEM-OPT-OPS does NOT clear the preprod owned posture; `OP-MEM-02` stays `declared`, residual gap pointing at MEM-OPT-UTXO-DISK** (the gating lever — not merely the "mainnet-scalable floor" OQ-OPS-2 anticipated; *Ade-holds-UTxO-in-anonymous-heap vs Haskell-offloads-to-disk* is precisely what UTXO-DISK closes).

### Cluster reviews (both findings resolved in `233644f7`, before close)
- **IDD review — BLOCK (fixed):** the replay-byte-identity test masked only `"rss_kib":`; S3 widened the same `memory_measure` line with the owned `rss_anon_kib`/`private_dirty_kib` fields, reintroducing the exact non-determinism S1 had fixed for `rss_kib` (a latent flake — green only because the owned reads coincided on the in-memory run). Generalized the masker to `normalize_memory_kib`, which zeroes EVERY observational `*_kib`; the assertion stays strict on every deterministic field (point/slot/durable_tip_fp/verdict/hashes).
- **Security review — MEDIUM (fixed; an EVIDENCE-SCOPE CORRECTION, NOT a production bug):** the S2 "both paths reject any duplicate" claim was overstated. It holds for the canonical-collision case (distinct strings `#0`/`#00` → same `TxIn`; both paths reject), but not in general. The corrected, pinned facts:
  1. **Production streaming import rejects duplicate TxIn forms fail-closed** (`DuplicateTxIn`).
  2. **The whole-buffer oracle is not authoritative for exact duplicate JSON keys** because serde map ingestion collapses them last-wins (returns `Ok`).
  3. **Equivalence is claimed only for valid unique-key seeds and for malformed cases where both paths are expected to agree.**
  4. **Exact duplicate string keys are pinned as a documented oracle asymmetry** (`streaming_rejects_exact_duplicate_string_key_but_oracle_collapses`).
  cardano-cli emits unique outref keys by construction and production is the fail-closed path, so this is an incompleteness in the equivalence proof plus an overstated claim — corrected in the `DC-MEM-06` registry `evidence_notes` and the S2 slice doc — **not a deployed defect**.

### Grounding docs refreshed (all at `233644f7`)
- **CODEMAP** — 0 gaps; **BLUE delta 0** (462→464 canonical types are GREEN/RED only); `OP-MEM-02` recorded honest `declared`/`ade_heavier`.
- **TRACEABILITY** — **378↔378 1:1 registry↔doc join**; +5 MEM-OPT rows (DC-MEM-05/06/07/08, OP-MEM-02) plus 2 reconciled stale `declared→partial` flips (CN-MEM-01, OP-MEM-01 from the prior MEM-MEASURE work); status 250 enforced / 22 partial / 105 declared / 1 scaffolding.
- **SEAMS** — lockstep with CODEMAP; 4 new closed surfaces registered (`JsonSeedError`+`DuplicateTxIn`, `MemoryMeasure`/`MemorySummary`, `BoundedOutcome`/`ShedReason`, `ReplayVerdict`/`EvidenceDefect`); NO new ingress, NO new extension point, BLUE untouched.
- **HEAD_DELTAS** — span `862cd2cb..233644f7` (MEM-MEASURE + MEM-OPT-OPS); MEM-OPT-OPS captured; 190 rule-referenced CI gates at HEAD. *(The `head_deltas_baseline` bump to the close commit is owed at MEM-OPT-UTXO-DISK start, so this cluster's span stays consistent with its recorded baseline.)*

### Next cluster (gated, NOT started at close)
**MEM-OPT-UTXO-DISK** opens with a diagnostic **S0**: *is the ~4.8 GiB active-admission owned footprint retained-freed serialization memory, or live required working set?* — a measurement, NOT a redesign. The answer scopes whether UTXO-DISK is a serialization-streaming fix or a full on-disk-UTxO representation change (`DC-MEM-05`/`DC-MEM-07`/`DC-MEM-08` carry the gap-by-design cells for it).
