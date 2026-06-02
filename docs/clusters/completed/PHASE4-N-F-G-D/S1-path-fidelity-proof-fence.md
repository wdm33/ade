# Invariant Slice — PHASE4-N-F-G-D S1: Path-fidelity proof + fence

> **Status:** Planning Artifact (Non-Normative). Normative authority is the registry + CI.

## 2. Slice Header

### Slice Name
Path-fidelity proof + fence (prove the `--mode node` accepted-block path is input-driven and venue-agnostic; fence against private-only divergence).

### Cluster
**PHASE4-N-F-G-D** — Private-testnet accepted-block bounty dry-run.

### Status
Merged (PHASE4-N-F-G-D close — impl `d4d0f456`; CE-G-D-1 green). First and load-bearing slice — the bounty-alignment proof.

### Cluster Exit Criteria Addressed
- [ ] **CE-G-D-1 (path fidelity — MECHANICAL, closeable)** — a test proves `import_live_consensus_inputs` consumes an early/private-net extraction through the **same** path used for a synced preprod-tip extraction (OQ1 proof obligation; if it cannot, the slice fixes the **shared** path, never a private-only workaround); a path-fidelity CI fence proves G-D adds **no new `--mode node` argv flag** and **no from-genesis consensus-inputs constructor** (the `--mode node` accepted-block path's consensus inputs are populated only via `import_live_consensus_inputs`).

### Slice Dependencies
- PHASE4-N-F-G-C (live WirePump feed + `--mode node` consume of `import_live_consensus_inputs`) — merged (`main` `da205bff`).
- N-M-C (`CN-CINPUT-03` / `DC-CINPUT-02b`: the shared `import_live_consensus_inputs` extraction/consume path) — merged.

## 3. Implementation Instruction (AI)
**Slice-entry proof obligation first (OQ1):** confirm in code that the single shared importer `import_live_consensus_inputs_from_bytes` (`crates/ade_runtime/src/consensus_inputs/canonical.rs:120` → `import_live_consensus_inputs_raw_from_bytes` → `validate_and_lift`, `importer.rs:161`) accepts an **early/private-net-shaped** bundle (`epoch_no = 0`, `epoch_start_slot = 0`, `source_tip_slot = 0` at origin, single pool ~all stake, ASC e.g. `1/2`) **and** a **synced-preprod-tip-shaped** bundle (current epoch, in-window tip, real-ish stake, ASC `1/20`) — both through the **same** function, with no venue parameter or branch. *(Doc-time read: `validate_and_lift` is purely structural — era=Conway, `epoch_end_slot ≥ epoch_start_slot`, `source_tip_slot ∈ [start,end]` (inclusive), ASC denom≠0, hash widths, key-set parity — and the inclusive window check passes an epoch-0 shape (`0 < 0 || 0 > end` is false). So the expected outcome is **PASS, no fix needed**.)* **If a shape trips a check, fix the SHARED importer (preprod benefits too) — never add a private-only workaround (N0).** Then add the path-fidelity CI fence `ci/ci_check_node_path_fidelity.sh`. Touch **no** BLUE crate; add **no** `--mode node` argv flag; add **no** from-genesis constructor; do **not** relax the containment / handoff / memory-bound gates. Commit carries the project attribution trailer (CLAUDE.md), no other AI references.

## 4. Intent
Make it **mechanically impossible** for G-D to introduce a private-only accepted-block path: the C1 dry-run's consensus-inputs ingestion is the **same** `import_live_consensus_inputs` the preprod pass uses (venue-agnostic — proven for both an epoch-0 and a tip shape), and a CI fence pins that G-D adds no `--mode node` flag and no from-genesis constructor. (Begins enforcing clause (1) — *path fidelity* — of `CN-REHEARSAL-FIDELITY-01`; preserves the shared importer's structural validation, the single-live-run-owner contract, and the containment/handoff/memory fences.)

## 5. Scope
- **Modules / crates:**
  - `ci/ci_check_node_path_fidelity.sh` (NEW gate, RED) — **guard (a)** the `--mode node` argv flag-literal set in `crates/ade_node/src/cli.rs` equals the pinned closed allow-list (G-D adds none); **guard (b)** negative-grep: no from-genesis consensus-inputs constructor exists (no `fn *from_genesis*consensus*` / `*consensus_inputs_from_genesis*` / `synthesize*consensus*`), and `import_live_consensus_inputs` is the sole node-path populator of the forge base's consensus inputs (positive grep at `node_lifecycle.rs`).
  - The transfer-fidelity test (GREEN-by-content / test) — exercises the shared `import_live_consensus_inputs_from_bytes` over both bundle shapes.
  - *(Contingency only, expected unused)* `crates/ade_runtime/src/consensus_inputs/importer.rs` — **only if** OQ1 surfaces a check that wrongly rejects a valid epoch-0/private shape, the fix lands here in the **shared** importer (benefiting preprod), never a private-only branch.
- **State machines affected:** none (proof + fence; the importer's `validate_and_lift` transition is unchanged unless the OQ1 contingency fires).
- **Persistence impact:** none.
- **Network-visible impact:** none.
- **Out of scope:** the rehearsal-evidence surface (S2); the runbook + operator execution (S3); any serve/forge/containment/memory change; any live-evidence / BA-02 / RO-LIVE flip.

## 6. Execution Boundary
- **BLUE (none — unchanged):** no BLUE crate touched. A BLUE change → reject.
- **GREEN:** `ade_runtime::consensus_inputs::{canonical, importer, json}` (`//! GREEN` by content within RED `ade_runtime`) — **reused unchanged** (exercised by the transfer-fidelity test; modified only under the OQ1 contingency, and then only to *widen* acceptance in the shared path, never branch).
- **RED:** `ci/ci_check_node_path_fidelity.sh` (new CI fence); the binding it asserts is over RED `ade_node` (`cli.rs` flag set + the `node_lifecycle.rs:1081` `import_live_consensus_inputs` consume site).
- **Color resolved:** no ambiguity — the importer is GREEN-by-content (`//! GREEN`), the fence is RED tooling, no BLUE.

## 7. Invariants Preserved
- `CN-CINPUT-03` / `DC-CINPUT-02b` — the recovered/consumed consensus-inputs surface. S1 **proves** the shared importer transfers to a private/early shape; it does not change the consume contract.
- `CN-NODE-02` — single live-run lifecycle owner. S1 adds no second bootstrap/apply/forge/tip-advance path, no new mode, no venue branch.
- `DC-NODE-06` / `CN-NODE-02` containment + `ci_check_served_chain_handoff_fence.sh` / `ci_check_node_run_loop_containment.sh` — byte-unchanged.
- `DC-LIVEMEM-01` / `ci_check_live_feed_memory_bounds.sh` — byte-unchanged.
- `DC-EPOCH-03` — single-epoch forge fail-closed — unchanged.
- The shared importer's structural validation (`validate_and_lift`: era / window / ASC / parity / hash-width) — preserved; if the OQ1 contingency widens it, the widening keeps every existing rejection (no weakening) and applies equally to all venues.

## 8. Invariants Strengthened or Introduced
- **`CN-REHEARSAL-FIDELITY-01` — clause (1) path fidelity (strengthened: enforcement begun).** S1 records its gate (`ci/ci_check_node_path_fidelity.sh`) + test (`node_accepted_block_consensus_inputs_via_shared_import`) in the rule's `evidence_notes` now; the **final `tests`/`ci_script` binding + the `declared → enforced` flip happen at G-D close** (unless the project registry convention requires per-slice binding — earlier clusters deferred final enforcement to close). The gate + test are green in CI from S1 onward, so the path-fidelity half is mechanically enforced even while the registry status word remains `declared`.

> Single invariant family: "the C1 dry-run uses the same accepted-block path as preprod — no private-only divergence." S1 covers exactly the path-fidelity half; evidence non-promotability is S2.

## 9. Design Summary
- **Transfer-fidelity test** (`node_accepted_block_consensus_inputs_via_shared_import`): build two in-memory `RawConsensusInputs` JSON bundles — one private/epoch-0-shaped, one preprod-tip-shaped — and call the **single** `import_live_consensus_inputs_from_bytes` (the function `node_lifecycle.rs:1081` uses) on each. Assert **both** return `Ok(LiveConsensusInputsCanonical)` and that the only differences are the data fields (epoch_no, slots, stake, nonce), proving venue-agnosticism (one function, no branch). This is the OQ1 proof.
- **Path-fidelity fence** (`ci_check_node_path_fidelity.sh`): **guard (a)** extracts the `"--…" =>` flag literals from the `cli.rs` arg-parse match and asserts the set equals the pinned closed allow-list (`--network, --chain-db, --snapshot-store, --listen, --peer, --mode, --log, --tip-read-timeout-secs, --json-seed, --seed-point-slot, --seed-block-hash, --wal-dir, --snapshot-dir, --network-magic, --genesis-hash, --consensus-inputs-path, --mithril-manifest-path, --out-file, --period-idx, --seed-file, --cold-skey, --kes-skey, --vrf-skey, --opcert, --genesis-file, --evidence-log, --max-slots`) — G-D adds none; **guard (b)** negative-greps the workspace for a from-genesis consensus-inputs constructor and positive-greps that `node_lifecycle.rs` populates the forge base's consensus inputs only via `import_live_consensus_inputs`. Hermetic (grep only; no Docker / cardano-cli / live node).
- **No private-only path:** the C1 dry-run differs from preprod only in operator **inputs** (private genesis stake → fast slots) and the evidence **label** (S2) — never in code.

## 10. Changes Introduced
### Types
- None. No new canonical type, no new `Mode`, no CLI field, no new enum variant.
### State Transitions
- None (unless the OQ1 contingency widens the shared `validate_and_lift` acceptance — and then identically for all venues, preserving every existing rejection).
### Persistence
- None.
### Removal / Refactors
- None expected.

## 11. Replay, Crash, and Epoch Validation
- **Replay:** no new authoritative state → replay unaffected. The transfer-fidelity test is deterministic (pure import of fixed bytes, both shapes). `R2` (forge replay, `DC-NODE-05`/`T-REC-03`) is carried, untouched.
- **Crash/restart:** unchanged — S1 adds no durable state.
- **Epoch boundary:** the epoch-0 shape is *within-epoch* by construction; `DC-EPOCH-03` (off-epoch fail-closed) is preserved, not exercised here.

## 12. Mechanical Acceptance Criteria
This slice is complete only when **all** of the following exist and pass in CI (hermetic):

- [ ] `node_accepted_block_consensus_inputs_via_shared_import` (`ade_node`, exercising `ade_runtime::consensus_inputs::import_live_consensus_inputs_from_bytes`) — a private/epoch-0-shaped bundle **and** a preprod-tip-shaped bundle both import `Ok` through the **same** function (venue-agnostic; no branch). *(If either fails, the shared importer is fixed and this test plus the existing importer tests still pass — never a private-only path.)*
- [ ] `ci_check_node_path_fidelity.sh` — NEW gate, green: guard (a) the `cli.rs` flag-literal set equals the pinned closed allow-list (no G-D-added flag); guard (b) no from-genesis consensus-inputs constructor + `import_live_consensus_inputs` is the sole node-path consensus-inputs populator; smoke-tested fail-closed on an injected `--private-net` flag and an injected `fn build_consensus_inputs_from_genesis`.
- [ ] `ci_check_node_run_loop_containment.sh` — **byte-unchanged + green** (verified `git diff` vs `main`).
- [ ] `ci_check_served_chain_handoff_fence.sh` — **byte-unchanged + green** (verified `git diff` vs `main`).
- [ ] `ci_check_live_feed_memory_bounds.sh` — **byte-unchanged + green** (verified `git diff` vs `main`).
- [ ] `cargo test -p ade_node` + `cargo test -p ade_runtime` green (no regression; the existing `importer.rs` validation tests still pass — proving any OQ1 widening preserved every rejection).

## 13. Failure Modes
- A bundle shape the shared importer rejects (OQ1 trips) → **fix the shared importer** so the rejection is removed for *all* venues (preprod gains the same acceptance); the existing importer rejection tests must still pass (no weakening of era/parity/hash/ASC). Never a private-only branch.
- A future G-D slice attempts to add a `--mode node` flag or a from-genesis constructor → `ci_check_node_path_fidelity.sh` **fails closed** (the fence trips), forcing re-scope.

## 14. Hard Prohibitions
### Inherited Cluster-Level Prohibitions
All PHASE4-N-F-G-D "Forbidden During This Cluster" prohibitions apply (N0 no private-only shortcut; no containment/handoff/memory-bound relaxation; no synthetic manifest; no RO-LIVE flip; no new BLUE authority/canonical type/`--mode node` flag/from-genesis constructor).
### Slice-Specific Prohibitions
- **No private-only workaround for OQ1** — a rejected shape is fixed in the **shared** importer or the dry-run is re-scoped; never special-cased for the private net.
- **No new `--mode node` argv flag** — the C1 dry-run uses the same flags as preprod.
- **No from-genesis / offline-eta0 consensus-inputs constructor** — `import_live_consensus_inputs` stays the sole node-path populator.
- **No weakening of the shared importer's existing rejections** — any OQ1 widening must keep every era/window/ASC/parity/hash check that currently fails closed.
- **No BLUE change; no serve/forge/containment/memory change** (all three fences byte-unchanged).
- **No evidence surface here** — the rehearsal envelope is S2; S1 writes no manifest and makes no live-evidence / BA-02 / RO-LIVE claim.

## 15. Explicit Non-Goals
This slice MUST NOT: build the rehearsal-evidence envelope/gate (S2); write the dry-run runbook or wire operator execution (S3); add any `--mode node` flag or config; add a from-genesis constructor; modify any BLUE crate; relax any containment/handoff/memory fence; claim live evidence / BA-02 / rehearsal; flip RO-LIVE.

## 16. Completion Checklist
- [ ] OQ1 proven: both bundle shapes import via the single shared function (or the shared importer was widened, with all prior rejections preserved).
- [ ] No private-only path / flag / constructor introduced (fence green; fail-closed smoke verified).
- [ ] CI enforces clause (1) path fidelity (the transfer-fidelity test + `ci_check_node_path_fidelity.sh`); the three existing fences byte-unchanged.
- [ ] No BLUE change.
- [ ] `cargo test -p ade_node` + `-p ade_runtime` green.
- [ ] `CN-REHEARSAL-FIDELITY-01` `evidence_notes` record the S1 tests/gate now; final `tests`/`ci_script` binding and status flip happen at G-D close unless the project registry convention requires per-slice binding.

## 17. Review Notes
- **OQ1 doc-time finding (the load-bearing call):** `validate_and_lift` (`importer.rs:161-241`) is purely structural and venue-agnostic; an epoch-0 shape passes the inclusive window check (`source_tip_slot=0 ∈ [0, epoch_end]`). So the expected outcome is **no shared-path fix** — the transfer-fidelity test should pass against today's importer. The contingency (fix the shared path, never a private-only branch) is retained per N0 in case implementation surfaces an edge.
- **Why a fence, not just a test:** the test proves *today's* path transfers; the fence prevents a *future* G-D slice (S2/S3) from sneaking in a private-only flag or constructor — it makes the bounty-alignment invariant durable, not a point-in-time check.
- **Bounty alignment:** S1 is the cheapest possible proof that the C1 dry-run cannot drift from the preprod path — exactly the thing that makes a private rehearsal worth running ([[feedback_produce_subordinate_to_sync_spine]], [[feedback_bounded_smoke_slices]]).
