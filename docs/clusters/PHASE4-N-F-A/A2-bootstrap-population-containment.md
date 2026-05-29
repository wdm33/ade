# Slice PHASE4-N-F-A / A2 — bootstrap-time population (keyed SnapshotStore) + CI containment gate

## 2. Slice Header
- **Slice Name:** Add the dedicated keyed `SnapshotStore` sidecar surface, populate `SeedEpochConsensusInputs` during verified bootstrap (anchor-bound), and CI-fence the forge-time consensus-inputs path out of it.
- **Cluster:** PHASE4-N-F-A — Seed-Epoch Consensus Input Provenance.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-A-2** — populated ONLY during verified bootstrap and anchor-bound; forge-time path fenced out.
- **Slice Dependencies:** A1 (the `SeedEpochConsensusInputs` BLUE type + sole codec).

## 3. Implementation Instruction (AI)
Implement §10 only. Add the keyed sidecar surface on `SnapshotStore`, populate it at the **bootstrap composition sites**, add the CI containment gate. Do **not** wire `bootstrap_initial_state` warm-start to *read* it (that restore + production replay test is A3), and do **not** add projection (A4). Commit with the model-attribution trailer.

## 4. Intent
Make it impossible for the produce-consumed seed-epoch consensus inputs to originate anywhere except the **verified-bootstrap** extraction: the `SeedEpochConsensusInputs` sidecar is written only at the bootstrap composition sites, through an explicit **anchor-keyed** `SnapshotStore` surface, and the forge-time `--consensus-inputs-path` import path is structurally fenced out of it. *(Introduces candidate `CN-CINPUT-02`.)*

## 5. Scope
- **Modules / crates:** `ade_runtime` (RED) — `chaindb` (the `SnapshotStore` trait + its impls: redb-backed + in-memory) gains a **dedicated keyed sidecar method pair**; `genesis_bootstrap` / `mithril_bootstrap` (the composition sites) gain a sidecar build+persist step; `ci/ci_check_consensus_input_provenance.sh` (new gate). Reuses `ade_ledger::seed_consensus_inputs` (A1).
- **State machines affected:** none.
- **Persistence impact:** new **anchor-keyed** sidecar surface on `SnapshotStore` — `put_seed_epoch_consensus_inputs(anchor_fp, bytes)` / `get_seed_epoch_consensus_inputs(anchor_fp)`, keyed by the anchor fingerprint (`Hash32`), **NOT** a reserved sentinel slot and **separate from the slot-keyed snapshot namespace**. A2 *writes* it at bootstrap (and exercises `get` in tests); A3 *reads* it in warm-start.
- **Network-visible impact:** none.
- **Out of scope:** `bootstrap_initial_state` warm-start restore + production replay (A3), projection (A4), produce wiring, any `BootstrapAnchor`/codec change, any sentinel-slot scheme.

## 6. Execution Boundary (TCB color)
- **BLUE:** reuses `ade_ledger::seed_consensus_inputs` (A1) — type + `encode_seed_epoch_consensus_inputs`. No BLUE change.
- **GREEN:** the deterministic merge transform (`LiveConsensusInputsCanonical` + `anchor_fp` + `epoch` → `SeedEpochConsensusInputs`) may be a GREEN-by-content helper (pure mapping, no I/O), banner `//! GREEN …`; **open color** — if simplest, it stays inline in the RED composer. The *transform* is deterministic (GREEN-eligible); the *store I/O* is RED.
- **RED:** the keyed `SnapshotStore` put/get methods + impls; the bootstrap composition persist step (`ade_runtime::{chaindb, genesis_bootstrap, mithril_bootstrap}`).
- *No ambiguous colors leave implementation:* store I/O = RED; mapping = GREEN-or-inline-RED (pick one, banner it).

## 7. Invariants Preserved
- `CN-CINPUT-01` (A1) — sidecar built via the A1 sole encoder; no second encoder.
- `CN-NODE-01` — bootstrap still routes initial state through the single `bootstrap_initial_state`; A2 adds a sidecar write at the same composition sites, not a parallel bootstrap path.
- `CN-ANCHOR-01` / `DC-ANCHOR-01` — the anchor + its codec untouched (the sidecar is a separate keyed surface, Option 3; no `ANCHOR_SCHEMA_VERSION` bump).
- `CN-MITHRIL-01` / `DC-MITHRIL-02` — Mithril binding + seed-point independence unchanged; the sidecar is populated *after* the binding verifies.
- Existing `SnapshotStore` slot-keyed snapshot semantics — the new keyed methods are a **disjoint** surface (anchor-fp-keyed), not overloaded slot data; `put_snapshot`/`get_snapshot` behavior is unchanged.
- BLUE/GREEN forbidden-pattern set; determinism of the transform.

## 8. Invariants Strengthened or Introduced
- **Introduces** candidate `CN-CINPUT-02` — *the `SeedEpochConsensusInputs` sidecar is populated ONLY on the verified-bootstrap composition path, through the anchor-keyed `SnapshotStore` surface; the forge-time consensus-inputs import path may not populate it.* Strengthens exactly one family (consensus-input provenance/containment). Enforced by the new CI gate.

## 9. Design Summary
- **Keyed sidecar surface (new):** add to the `SnapshotStore` trait `fn put_seed_epoch_consensus_inputs(&self, anchor_fp: &Hash32, bytes: &[u8]) -> Result<(), ChainDbError>` and `fn get_seed_epoch_consensus_inputs(&self, anchor_fp: &Hash32) -> Result<Option<Vec<u8>>, ChainDbError>`, implemented on the redb-backed store (a dedicated table keyed by the 32-byte fingerprint) and the in-memory store (`BTreeMap<Hash32, Vec<u8>>`). Idempotent on identical bytes for the same `anchor_fp`; conflicting bytes for the same key → `InvalidOperation` (mirrors `put_snapshot`). This surface is **disjoint** from the slot-keyed snapshot namespace.
- **Population at the bootstrap composition sites** (`genesis_bootstrap::bootstrap_from_conway_genesis`, `mithril_bootstrap::bootstrap_from_mithril_snapshot`): after the anchor is minted (and, for Mithril, after `verify_mithril_binding` passes), build `SeedEpochConsensusInputs { anchor_fp = anchor.initial_ledger_fingerprint, epoch_no, active_slots_coeff, total_active_stake, pool_distribution }` by **merging** the verified-bootstrap `LiveConsensusInputsCanonical` `pool_distribution` (stake) with `pool_vrf_keyhashes` (vrf keyhash) into the BLUE single `BTreeMap<Hash28, PoolEntry{active_stake, vrf_keyhash}>`. A pool present in one map but missing from the other → **fail-closed** structured error (no defaulting). `encode_seed_epoch_consensus_inputs` (A1) → `put_seed_epoch_consensus_inputs(anchor_fp, bytes)`.
- **CI containment gate `ci_check_consensus_input_provenance.sh`** (N-Z `ci_check_mithril_seed_point_independence.sh` style, data-flow-resistant): (a) `put_seed_epoch_consensus_inputs` is called **only** under the bootstrap composition sites; (b) the forge-time path — `produce_mode`, `import_live_consensus_inputs*`, `pool_distr_view_from_consensus_inputs`, `--consensus-inputs-path` — may not build or `put` the sidecar; (c) strips `#[cfg(test)]` + comments before grepping. (The admission-mode `import_live_consensus_inputs` use is a distinct mode, not the bounty-primary produce surface; the gate scopes to the produce/forge path + the sidecar populator.)

## 10. Changes Introduced
### Types
- No new persisted types (reuses A1). A closed RED merge-error variant for the pool merge mismatch (non-secret primitives).
### State Transitions
- Bootstrap composition gains a sidecar build+persist step (after anchor mint / after Mithril binding verify).
### Persistence
- New keyed `SnapshotStore` methods `put_/get_seed_epoch_consensus_inputs(anchor_fp, …)` + impls. New sidecar write at bootstrap. **No** anchor/WAL/slot-snapshot schema change; **no** sentinel slot.
### Removal / Refactors
- None. The RED `consensus_inputs` importer is untouched (it remains the bootstrap-time extraction source; not rerouted to forge-time).

## 11. Replay, Crash, and Epoch Validation
- **Tests added:** `bootstrap_persists_anchor_keyed_seed_consensus_inputs` (genesis + Mithril composition each `put` a sidecar that `get_seed_epoch_consensus_inputs(anchor_fp)` returns and the A1 codec decodes to the expected merged record; `anchor_fp` == `anchor.initial_ledger_fingerprint`); `bootstrap_seed_inputs_merge_fails_closed_on_missing_vrf_or_stake`; `snapshot_store_keyed_sidecar_is_disjoint_from_slot_snapshots` (a `put_snapshot(slot)` and a `put_seed_epoch_consensus_inputs(fp)` do not collide / overwrite).
- **Crash/restart behavior:** the sidecar is written durably at bootstrap; the **production warm-start** read-back + byte-identity is A3 (CE-A-3). A2 proves write + keyed-get + decodability + namespace disjointness.
- **Epoch boundary:** n/a (single seed epoch).

## 12. Mechanical Acceptance Criteria
- [ ] `ci/ci_check_consensus_input_provenance.sh` exists and passes: sidecar `put` only at bootstrap composition sites; forge-time path does not build/`put` it.
- [ ] `bootstrap_persists_anchor_keyed_seed_consensus_inputs` passes (genesis + Mithril; keyed by `anchor_fp`; decodes via A1 codec).
- [ ] `bootstrap_seed_inputs_merge_fails_closed_on_missing_vrf_or_stake` passes (fail-closed merge).
- [ ] `snapshot_store_keyed_sidecar_is_disjoint_from_slot_snapshots` passes (no collision with the slot namespace).
- [ ] `cargo build --workspace` + `cargo clippy` clean (no new deny violation; store I/O RED; transform pure).
- [ ] `cargo test --workspace` stays green.

## 13. Failure Modes
Fail-closed, typed, no panic: pool in `pool_distribution` without a `pool_vrf_keyhashes` entry (or vice versa) → structured merge error; keyed-put conflicting bytes for the same `anchor_fp` → `InvalidOperation`; store write failure → propagated bootstrap error. A bootstrap that cannot persist the sidecar fails the bootstrap (no silent proceed without provenance). All deterministic.

## 14. Hard Prohibitions
**Inherited (cluster):** no forge/leader-check/KES/VRF signing; no second anchor codec or storage-init authority; no stake computation/rotation; no `HashMap`; no clock/float/async in BLUE/GREEN; no registry promotion; no grounding-doc regeneration.
**Slice-specific:**
- The sidecar may be populated **only** at the verified-bootstrap composition sites — never from `produce_mode` / `import_live_consensus_inputs` / `pool_distr_view_from_consensus_inputs` / `--consensus-inputs-path`.
- **No reserved sentinel slot** and no overloading of the slot-keyed snapshot namespace — the sidecar uses the dedicated anchor-keyed methods.
- No second encoder/decoder for `SeedEpochConsensusInputs` (A1 is sole).
- No `BootstrapAnchor` / `ANCHOR_SCHEMA_VERSION` change.
- No warm-start *read* wiring (A3), no projection (A4), no produce wiring.
- No defaulting on a merge mismatch — fail closed.

## 15. Explicit Non-Goals
No `bootstrap_initial_state` warm-start restore + production replay (A3). No projection (A4). No produce wiring. No anchor schema change. No reroute of the RED importer to forge-time. No `recover_node_state` change (it is not the production path).

## 16. Completion Checklist
- [ ] Keyed sidecar surface added (trait + redb + in-memory impls), disjoint from slot snapshots.
- [ ] Sidecar written durably at bootstrap, anchor-keyed.
- [ ] Forge-time path CI-fenced from populating it.
- [ ] Merge mismatch fails closed (deterministic).
- [ ] §12 tests + the new CI gate pass; `cargo test --workspace` green.
