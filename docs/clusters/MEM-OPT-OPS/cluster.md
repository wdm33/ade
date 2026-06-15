# Cluster MEM-OPT-OPS — operational memory quick-wins (allocator + streaming import + owned-RSS measure)

**Primary invariant:** `OP-MEM-02` (Ade's owned resident memory stays clearly below the reference Haskell node's, with NO change to ledger semantics, chain selection, persisted bytes, or replay-equivalence).
**Status:** In progress — **S1 (ALLOC) MERGED** (hermetic `0f2dcbe6` + live CE-OPS-1 transcript); **S2 (IMPORT) next.** First of the three MEM-OPT clusters; the cheapest, lowest-risk levers, no BLUE change. S1 result: VmRSS p50 6,874,024 → 4,824,884 kiB (−29.8%), now ~0.90 GiB below the 5.50 GiB Haskell reference on VmRSS (the allocator alone banked the retained import peak). Re-measure after this cluster before committing to MEM-OPT-UTXO-DISK.
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
- **CE-OPS-2 (`OP-MEM-02`, IMPORT) [S2]:** a committed run with streaming import shows a reduced peak (owned RSS strictly below S1's), the imported UTxO fingerprint byte-identical to the non-streaming import (a hermetic replay test), replay verdict `agreed`.
- **CE-OPS-3 (owned-RSS ceiling) [S3]:** `ci_check_mem_rss_ceiling.sh --self-test` green; the committed run's owned RSS ≤ the cluster target; the comparison artifact records the new standing.
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

## 11. Cluster Close Record
*(Filled at `/cluster-close`.)*
