# Slice MEM-OPT-OPS S1 — global allocator swap (return the retained import peak to the OS)

> **Status:** In Progress
> **Cluster:** MEM-OPT-OPS (primary invariant `OP-MEM-02`)
> **Cluster doc:** `docs/clusters/MEM-OPT-OPS/cluster.md` · **Grounding:** `docs/planning/mem-opt-grounding.md`

## 2. Slice Header

### Cluster Exit Criteria Addressed
- [ ] **CE-OPS-1** (`OP-MEM-02`, ALLOC): a committed preprod transcript with the allocator swapped shows resident memory **strictly below** the MEM-MEASURE-A2 baseline (6.56 GB), `memory_summary{replay_verdict=agreed}`, 0 diverged; `ci_check_mem_measure_evidence.sh` + the determinism-neutral allocator gate green; `cargo test -p ade_node` green.

Exit criteria CE-OPS-2 (streaming import) and CE-OPS-3 (owned-RSS ceiling) are explicitly **out of scope** for this slice.

### Slice Dependencies
- Entry substrate (already merged): MEM-MEASURE-A1/A2 — `ade_node::mem_measure::rss_sampler` (the RED `/proc` sampler), the closed `memory_measure`/`memory_summary` admission vocabulary, `ci_check_mem_measure_evidence.sh`, and the committed A2 baseline transcript `docs/evidence/mem-measure-a2-preprod-memory.jsonl` (6.56 GB, verdict `agreed`).

---

## 3. Implementation Instruction
Swap the process global allocator and add the determinism-neutrality gate. Nothing else. No BLUE change, no streaming import, no owned-RSS sampler. The allocator is a single `#[global_allocator]` static in the RED binary entry; it changes no authoritative output.

---

## 4. Intent
Make it **impossible** for the node process to retain the transient seed-import peak (~4 GB of freed pages) indefinitely. The default glibc `System` allocator keeps freed pages in its per-thread arenas and rarely returns them to the OS, pinning RSS near the ~6.8 GB import peak (`mem-opt-grounding.md §A`). An allocator that returns freed pages to the OS removes that retention — a **runtime representation change that alters no ledger semantics, chain selection, persisted bytes, or replay output** (`OP-MEM-02`).

## 5. Scope
- **Modules / crates:** `crates/ade_node` only — `Cargo.toml` (one dependency) + `src/main.rs` (one `#[global_allocator]` static). New CI gate `ci/ci_check_alloc_determinism_neutral.sh`.
- **State machines affected:** none.
- **Persistence impact:** none — no WAL/checkpoint/canonical-byte change.
- **Network-visible impact:** none.
- **Out of scope:** streaming import (S2 / `importer.rs`); the owned-footprint `smaps_rollup` sampler + RSS-ceiling gate (S3); on-disk UTxO (MEM-OPT-UTXO-DISK); any allocator *tuning* beyond the swap (the `MIMALLOC_PURGE_DELAY` env knob is a measurement-env lever, not baked into the binary).

## 6. Execution Boundary
- **BLUE:** none. The ledger, UTxO semantics, canonical encoders, and fingerprints are untouched — and the gate proves the allocator type never appears in any BLUE crate.
- **GREEN:** the new CI gate (`ci_check_alloc_determinism_neutral.sh`) — deterministic static analysis of the source tree.
- **RED:** the global allocator (`crates/ade_node/src/main.rs`) — the process binary entry. The allocator manages process memory and influences no authoritative output; allocation addresses/sizes are never fingerprinted.

## 7. Invariants Preserved
- **Replay-equivalence (`DC-WAL-03`):** same WAL + checkpoint → byte-identical post-state and fingerprint. The allocator cannot change any fingerprint (proven by the unchanged `ade_testkit` replay corpus passing byte-identically).
- **Backend independence (`DC-MEM-05`):** trivially — no storage backend changes; the UTxO remains the in-memory `BTreeMap`.
- **Determinism (`T-DET-01`):** no `HashMap`/float/wall-clock/native-endian introduced; the allocator is determinism-neutral.
- **The A2 measurement discipline:** the re-measurement run still emits `memory_summary{replay_verdict=agreed}` — a lower-memory run that diverges is invalid evidence.

## 8. Invariants Strengthened or Introduced
- **`OP-MEM-02`** (toward): the first lever banking the retained import peak — drives resident memory toward clearly-below the reference node.
- **`DC-MEM-06`** (newly mechanically enforced): *the allocator is determinism-neutral; no allocator type enters any authoritative fingerprint.* Enforced by `ci_check_alloc_determinism_neutral.sh` — exactly one `#[global_allocator]`, located in the RED binary entry, and **zero** allocator references in any BLUE crate (where every canonical encoder and fingerprint lives).

## 9. Design Summary
- `crates/ade_node/src/main.rs` gets, at crate root:
  ```rust
  #[global_allocator]
  static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
  ```
  mimalloc returns freed pages to the OS (segment purge/decommit), so the ~4 GB transient import peak no longer pins RSS. The `#[global_allocator]` lives in the binary crate root, so it is process-wide for **every** mode (`node`, `admission`, `produce`, …) — including the `--mode admission` measurement run.
- **Allocator choice (resolves OQ-OPS-1 → mimalloc):** simplest integration (one static; no `malloc_conf` symbol-prefix fragility that can silently no-op a jemalloc decay config); returns memory to the OS by default; the grounding's cited RSS-reduction precedent (rust-analyzer, Meilisearch). Fallback lever if the live purge is insufficient: env `MIMALLOC_PURGE_DELAY=0` (immediate decommit) for the measurement run, then tuned `tikv-jemallocator` (`background_thread:true,dirty_decay_ms:0,muzzy_decay_ms:0`) as the documented alternative — neither needed unless CE-OPS-1 misses.
- **`DC-MEM-06` enforcement** (`ci/ci_check_alloc_determinism_neutral.sh`, with `--self-test`):
  1. exactly one `#[global_allocator]` in the tree, and it is in `crates/ade_node/src/main.rs` (the RED binary entry);
  2. zero allocator references (`mimalloc` / `MiMalloc` / `jemalloc` / `tikv-jemalloc` / `global_allocator` / `GlobalAlloc`) in any BLUE crate source **or** manifest (`ade_ledger`, `ade_codec`, `ade_types`, `ade_crypto`, `ade_plutus`, `ade_core`).

## 10. Changes Introduced
- **Types:** none.
- **State transitions:** none.
- **Persistence:** none.
- **Dependencies:** `mimalloc` added to `crates/ade_node/Cargo.toml` `[dependencies]` (binary crate only; never a BLUE dep).
- **CI:** new `ci/ci_check_alloc_determinism_neutral.sh`.

## 11. Replay, Crash, and Epoch Validation
- **Replay tests:** the existing `ade_testkit` replay corpus (`boundary_replay`, `differential_*_replay`, `ledger_determinism`, `stateful_replay`) must pass **byte-identically** with the allocator swapped — the mechanical proof the allocator changes no fingerprint. No new replay test is needed (the invariant is "no change", best proven by the unchanged corpus).
- **Crash/restart:** unaffected — no persistence change.
- **Epoch boundary:** not applicable.
- **Live (CE-OPS-1):** a committed `--mode admission` preprod re-run (same A2 protocol, same `rss_sampler`/`VmRSS` metric) with the mimalloc binary, showing resident memory strictly below 6.56 GB and `memory_summary{replay_verdict=agreed}`. (The owned-footprint `Private_Dirty`/`RssAnon` refinement is S3; S1 keeps the metric identical to the baseline so the comparison is apples-to-apples — only the allocator changed.)

## 12. Mechanical Acceptance Criteria
- [ ] `cargo build -p ade_node` succeeds with the mimalloc global allocator.
- [ ] `cargo test -p ade_node` green (no behavioral change).
- [ ] `cargo test -p ade_testkit` (replay corpus) passes byte-identically — the allocator changes no fingerprint.
- [ ] `ci/ci_check_alloc_determinism_neutral.sh` green, and `--self-test` green: exactly one `#[global_allocator]` (in `ade_node/src/main.rs`); zero allocator references in BLUE crates.
- [ ] `ci/ci_check_registry_code_locus_exists.sh` green (DC-MEM-06 `code_locus` → the new gate resolves on disk).
- [ ] **CE-OPS-1 (live):** committed preprod `--mode admission` transcript with the mimalloc binary — resident memory strictly below 6.56 GB, `memory_summary{replay_verdict=agreed}`, 0 diverged; `ci/ci_check_mem_measure_evidence.sh` green.

## 13. Failure Modes
- **Build failure** (mimalloc C build): fail-fast at compile; no partial state.
- **Allocator leak into BLUE:** `ci_check_alloc_determinism_neutral.sh` fails closed (non-zero) — a BLUE crate referencing the allocator type is a determinism risk and blocks the slice.
- **Second `#[global_allocator]`:** Rust compile error (only one is permitted) — fail-fast.
- A lower-memory run that **diverges** (`replay_verdict != agreed`, or any `diverged`): INVALID evidence — the A2 discipline rejects it; CE-OPS-1 is not satisfied.

## 14. Hard Prohibitions
Inherits all of the cluster's "Forbidden During This Cluster" (§8). Slice-specific:
- No BLUE change — no ledger/UTxO/fingerprint/canonical-encoder edit.
- No allocator type in any authoritative fingerprint (`DC-MEM-06`).
- No feature flag / config switch selecting the allocator per run — the allocator is fixed at build, not behavior-selectable.
- No second `#[global_allocator]`; the allocator is **not** added to any BLUE crate's dependencies.
- No streaming-import change (that is S2), no owned-RSS sampler (that is S3).

## 15. Explicit Non-Goals
- Not the streaming seed import (S2).
- Not the owned-footprint `smaps_rollup` sampler or the RSS-ceiling regression gate (S3).
- Not the on-disk UTxO (MEM-OPT-UTXO-DISK) or compact TxOut (MEM-OPT-COMPACT).
- No allocator *tuning* baked into the binary beyond the swap; no general performance optimization.

## 16. Completion Checklist
- [ ] No new authoritative state (the allocator is RED runtime only).
- [ ] No new canonical/persisted bytes.
- [ ] Failure modes deterministic (compile-time / gate fail-closed).
- [ ] No TODOs/placeholders in BLUE (no BLUE touched).
- [ ] CI enforces the strengthened invariant (`DC-MEM-06` gate).
- [ ] Replay-equivalence corpus passes byte-identically.

## 17. Review Notes
- **Invariant risk considered:** could the allocator perturb any authoritative output? No — `#[global_allocator]` swaps the byte-provider beneath `alloc`; iteration order, hashing, and canonical encoding are all over *values*, never addresses. The gate makes "allocator type invisible to BLUE" mechanical, and the unchanged replay corpus is the runtime proof.
- **Follow-up implied:** S2 (streaming import — prevent the peak rather than return it) and S3 (owned-footprint measurement + ceiling). Re-measure after S1 before deciding MEM-OPT-UTXO-DISK depth (OQ-OPS-2).
