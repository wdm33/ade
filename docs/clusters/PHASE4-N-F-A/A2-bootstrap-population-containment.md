# Slice PHASE4-N-F-A / A2 — bootstrap-time population + CI containment gate

## 2. Slice Header
- **Slice Name:** Populate the `SeedEpochConsensusInputs` sidecar during verified bootstrap (anchor-bound), and CI-fence the forge-time consensus-inputs path out of it.
- **Cluster:** PHASE4-N-F-A — Seed-Epoch Consensus Input Provenance.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-A-2** — populated ONLY during verified bootstrap and anchor-bound; forge-time path fenced out.
- **Slice Dependencies:** A1 (the `SeedEpochConsensusInputs` BLUE type + sole codec).

## 3. Implementation Instruction (AI)
Implement §10 only. Build + persist the sidecar at the **bootstrap composition sites**; add the CI containment gate. Do **not** add recovery read (A3) or projection (A4). Commit with the model-attribution trailer.

## 4. Intent
Make it impossible for the recovered/produce-consumed seed-epoch consensus inputs to originate anywhere except the **verified-bootstrap** extraction: the `SeedEpochConsensusInputs` sidecar is written only at the bootstrap composition sites (anchor-bound), and the forge-time `--consensus-inputs-path` import path is structurally fenced out of it. *(Introduces candidate `CN-CINPUT-02`.)*

## 5. Scope
- **Modules / crates:** `ade_runtime` (RED) — `bootstrap_anchor` / `genesis_bootstrap` / `mithril_bootstrap` (the composition sites) + a RED population fn; `ci/ci_check_consensus_input_provenance.sh` (new gate).
- **State machines affected:** none (bootstrap is a one-shot composition).
- **Persistence impact:** writes the `SeedEpochConsensusInputs` sidecar at bootstrap, **anchor-bound (keyed/bound by `anchor_fp`)**, into the persistent store the anchor uses (impl resolves the exact mechanism: a reserved `SnapshotStore` entry, a dedicated sidecar store, or a file alongside the WAL/anchor — constraint: readable at recovery, bound to this anchor). Read-back is A3.
- **Network-visible impact:** none.
- **Out of scope:** recovery restore (A3), projection (A4), produce wiring, any `BootstrapAnchor`/codec change (A1 froze the sidecar codec; the anchor is untouched).

## 6. Execution Boundary (TCB color)
- **BLUE:** reuses `ade_ledger::seed_consensus_inputs` (A1) for the type + `encode_seed_epoch_consensus_inputs`. No BLUE change.
- **GREEN:** the deterministic RED→BLUE construction transform may be a GREEN-by-content helper (pure mapping `LiveConsensusInputsCanonical` + `anchor_fp` + `epoch` → `SeedEpochConsensusInputs`), banner `//! GREEN …`, if it touches no I/O; **open color** — if simplest to keep inline in the RED composer, it stays RED. Resolve at impl: the *transform* is deterministic (GREEN-eligible); the *persist* is RED.
- **RED:** the bootstrap composition sites + the store write (`ade_runtime`).
- *No ambiguous colors leave implementation:* the persist I/O is RED; the mapping is GREEN-or-inline-RED (pick one, banner it).

## 7. Invariants Preserved
- `CN-CINPUT-01` (A1) — the sidecar is built via the A1 sole encoder; no second encoder.
- `CN-NODE-01` — bootstrap still routes initial state through the single `bootstrap_initial_state`; A2 adds a sidecar write at the same composition sites, not a parallel bootstrap path.
- `CN-ANCHOR-01` / `DC-ANCHOR-01` — the anchor + its codec are untouched (the sidecar is separate, Option A).
- `CN-MITHRIL-01` / `DC-MITHRIL-02` — the Mithril binding + seed-point independence are unchanged; the sidecar is populated after the binding verifies.
- BLUE forbidden-pattern set; determinism of the transform.

## 8. Invariants Strengthened or Introduced
- **Introduces** candidate `CN-CINPUT-02` — *the `SeedEpochConsensusInputs` sidecar is populated ONLY on the verified-bootstrap composition path and is anchor-bound; the forge-time consensus-inputs import path may not populate it.* Strengthens exactly one family (consensus-input provenance/containment). Enforced by the new CI gate (the candidate's `ci_scripts` becomes real once §12's gate exists).

## 9. Design Summary
- RED population at the bootstrap composition sites (`genesis_bootstrap::bootstrap_from_conway_genesis`, `mithril_bootstrap::bootstrap_from_mithril_snapshot`): after the anchor is minted (and, for Mithril, after `verify_mithril_binding` passes), build `SeedEpochConsensusInputs { anchor_fp = fingerprint(anchor), epoch_no = seed epoch, active_slots_coeff, total_active_stake, pool_distribution }` by **merging** the operator-extracted `LiveConsensusInputsCanonical` `pool_distribution` (stake) with `pool_vrf_keyhashes` (vrf keyhash) into the BLUE single `BTreeMap<Hash28, PoolEntry{active_stake, vrf_keyhash}>`. A pool present in one map but missing from the other → **fail-closed** structured error (no defaulting). Encode via the A1 BLUE codec; persist the bytes anchor-bound.
- CI containment gate `ci_check_consensus_input_provenance.sh` (N-Z `ci_check_mithril_seed_point_independence.sh` style, data-flow-resistant): (a) the sidecar build/persist call appears **only** under the bootstrap composition sites; (b) the forge-time path — `produce_mode`, `import_live_consensus_inputs*`, `pool_distr_view_from_consensus_inputs`, `--consensus-inputs-path` — may not build or persist `SeedEpochConsensusInputs`; (c) strip `#[cfg(test)]` + comments before grepping. The admission-mode use of `import_live_consensus_inputs` is a separate mode and is not the bounty-primary produce surface — the gate scopes to the produce/forge path + the sidecar populator.

## 10. Changes Introduced
### Types
- No new persisted types (reuses A1). Possibly a closed RED error variant for the merge mismatch (`SeedInputsMergeError::{MissingVrfKeyhash, MissingStake, …}` or reuse an existing bootstrap error sum) — non-secret primitives only.
### State Transitions
- Bootstrap composition gains a sidecar build+persist step (after anchor mint / after Mithril binding verify).
### Persistence
- New sidecar write at bootstrap (anchor-bound). No anchor/WAL/snapshot-schema change.
### Removal / Refactors
- None. The RED `consensus_inputs` importer is untouched (it remains the bootstrap-time extraction source; A2 consumes its output, it is not rerouted to forge-time).

## 11. Replay, Crash, and Epoch Validation
- **Tests added:** `bootstrap_persists_anchor_bound_seed_consensus_inputs` (genesis + Mithril composition each produce a persisted sidecar that decodes via the A1 codec to the expected merged record, `anchor_fp` == the minted anchor's fp); `bootstrap_seed_inputs_merge_fails_closed_on_missing_vrf_or_stake` (a pool in one map but not the other → structured error, no panic, no default).
- **Crash/restart behavior:** the sidecar is written durably at bootstrap; read-back/byte-identity is proven in A3 (CE-A-3). A2 proves the write + decodability.
- **Epoch boundary:** n/a (single seed epoch; `epoch_no` recorded).

## 12. Mechanical Acceptance Criteria
- [ ] `ci/ci_check_consensus_input_provenance.sh` exists and passes: sidecar populated only at bootstrap composition sites; forge-time path (`produce_mode` / `import_live_consensus_inputs*` / `pool_distr_view_from_consensus_inputs` / `--consensus-inputs-path`) does not build/persist the sidecar.
- [ ] `bootstrap_persists_anchor_bound_seed_consensus_inputs` passes (both genesis + Mithril paths; decodes via A1 codec; anchor-bound).
- [ ] `bootstrap_seed_inputs_merge_fails_closed_on_missing_vrf_or_stake` passes (fail-closed merge).
- [ ] `cargo build --workspace` + `cargo clippy` clean (no new deny violation; RED I/O confined; GREEN transform if used is pure).
- [ ] `cargo test --workspace` stays green.

## 13. Failure Modes
Fail-closed, typed, no panic: pool in `pool_distribution` without a `pool_vrf_keyhashes` entry (or vice versa) → structured merge error; store write failure → propagated bootstrap error; (decode is A1's). All deterministic; a bootstrap that cannot persist the sidecar fails the bootstrap (does not silently proceed without provenance).

## 14. Hard Prohibitions
**Inherited (cluster):** no forge/leader-check/KES/VRF signing; no second anchor codec or storage-init authority; no stake computation/rotation; no `HashMap`; no clock/float/async in BLUE/GREEN; no registry promotion; no grounding-doc regeneration.
**Slice-specific:**
- The sidecar may be populated **only** at the verified-bootstrap composition sites — never from `produce_mode` / `import_live_consensus_inputs` / `pool_distr_view_from_consensus_inputs` / `--consensus-inputs-path`.
- No second encoder/decoder for `SeedEpochConsensusInputs` (A1 is sole).
- No `BootstrapAnchor` / `ANCHOR_SCHEMA_VERSION` change.
- No recovery read, no projection, no produce wiring.
- No defaulting on a merge mismatch — fail closed.

## 15. Explicit Non-Goals
No recovery restore (A3). No projection to `PoolDistrView`/`ExpectedVrfInput` (A4). No produce wiring. No anchor schema change. No reroute of the RED importer to forge-time.

## 16. Completion Checklist
- [ ] Sidecar written durably at bootstrap, anchor-bound.
- [ ] Forge-time path CI-fenced from populating it.
- [ ] Merge mismatch fails closed (deterministic).
- [ ] §12 tests + the new CI gate pass; `cargo test --workspace` green.
