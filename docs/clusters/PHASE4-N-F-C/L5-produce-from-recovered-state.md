# Slice PHASE4-N-F-C / L5 — Produce from recovered selected tip + recovered consensus inputs

> The first slice to touch the forge handoff. Wires the single recovered-state forge path:
> recovered `BootstrapState` + selected tip → `PoolDistrView::from_seed_epoch_consensus_inputs`
> → `ForgeRequestContext` → `run_real_forge`. The pool-distribution view that drives leadership
> is projected from the RECOVERED `SeedEpochConsensusInputs` (L3/L4 surface), never from a
> forge-time `--consensus-inputs-path` bundle and never via a shape-swap populator. Authority doc:
> `cluster.md`. Builds on L3 (`warm_start_recovery` → recovered `BootstrapState` with
> `seed_epoch_consensus_inputs: Some(..)`) and L4 (durable apply → recoverable selected tip).

## 2. Slice Header
- **Slice Name:** Build the node-lifecycle forge handoff from RECOVERED state: project the
  leadership `PoolDistrView` from the recovered `SeedEpochConsensusInputs` via
  `PoolDistrView::from_seed_epoch_consensus_inputs`, assemble a `ForgeRequestContext` from the
  recovered `BootstrapState` (base ledger + chain_dep + eta0) and the selected tip
  (block_number + prev_hash), and run a single-shot `run_real_forge`. No bundle / cold /
  shape-swap forge base on the lifecycle path.
- **Cluster:** PHASE4-N-F-C — Build the real Ade node lifecycle.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:** CE-L-5 — *"node-lifecycle forge base derives from recovered
  tip + recovered `SeedEpochConsensusInputs` via the A4 projection; no `InMemoryChainDb`/
  `--consensus-inputs-path` forge base on the lifecycle path; CN-CINPUT-03 + DC-CINPUT-02b enforced."*
- **Slice Dependencies:** L3 (recovered `BootstrapState` carrying `seed_epoch_consensus_inputs:
  Some(..)`), L4 (recoverable selected tip + the `BootstrapState`/tip the forge base is built from).
  Reuses the shipped `run_real_forge` / `ForgeRequestContext` engine verbatim (§9.0 F2).

## 3. Implementation Instruction (AI)
Implement §10 only — a `forge_one_from_recovered(...)` handoff function + its hermetic single-shot
tests + the CI enforcement of DC-CINPUT-02b / CN-CINPUT-03. **L5 does NOT wire a live `--mode node`
produce loop** (F4 = Option A). Do NOT read `--consensus-inputs-path` on the forge path, do NOT build
an `InMemoryChainDb`/cold `LedgerState` forge base, do NOT call `pool_distr_view_from_consensus_inputs`
on the lifecycle path, do NOT shape-swap an operator bundle into a `SeedEpochConsensusInputs`, do NOT
add a genesis fallback, do NOT emit any BA-02 / peer-accept claim, do NOT add a peer-evidence harness,
do NOT produce across more than one epoch, and keep it single-shot (one forge attempt) until
forged-block durability (N-U) lands. `run_real_forge`, `ForgeRequestContext`,
`PoolDistrView::from_seed_epoch_consensus_inputs`, and the leader-check are consumed verbatim —
**add no new BLUE authority**, do not edit `produce_mode`'s cold diagnostic path, and do not extract
the forge engine into a neutral module (that would be a separate refactor slice; F2). Do not change
the forge engine, `bootstrap.rs`, or `replay.rs`. Resolve the entry obligations in §9.0 before
coding. **Sequential edits only: make one edit, confirm it applied, then the next** (this turn's
shell lag caused silent edit failures; the verify→stage→commit guard caught them, but sequential
editing avoids the noise). Commit with the model-attribution trailer.

## 4. Intent
Make the producer's leadership input on the node-lifecycle path a **recovered-provenance** fact: the
`PoolDistrView` (and eta0) that decide who may forge are projected from the recovered
`SeedEpochConsensusInputs` — established at a Mithril-certified bootstrap, WAL-proven, warm-start
restored+verified (L3), atop a durably-applied selected tip (L4) — and the forge-time operator
bundle (`--consensus-inputs-path`) is made unrepresentable as a forge base. Provenance, not shape.

## 5. Scope
- **Modules / crates:**
  - `ade_node::node_sync` (RED) — the recovered-state forge handoff `forge_one_from_recovered(...)`:
    require `recovered.seed_epoch_consensus_inputs.is_some()` (else fail closed), project the
    `PoolDistrView` via `from_seed_epoch_consensus_inputs`, assemble `ForgeRequestContext` from the
    recovered base + selected tip, run the leader-check (§9.0 F3), call `run_real_forge` single-shot,
    return its `CoordinatorEvent` (fail-closed on error). (Module home: extend `node_sync` — it
    already owns the L4 recovered-state→durable-apply driver, so the recovered-state→forge handoff is
    its natural neighbor; no new module.)
  - `crates/ade_node/src/produce_mode.rs` (RED) — **consumed unchanged**; L5 reuses its `pub`
    `run_real_forge` + `ForgeRequestContext` (§9.0 F2). No edit to the cold/bundle path.
  - `crates/ade_ledger/src/consensus_view.rs` (`PoolDistrView::from_seed_epoch_consensus_inputs`) —
    **consumed unchanged**.
  - `ci/ci_check_consensus_input_provenance.sh` — **extended** for CN-CINPUT-03 (consume-side fence
    + "no shape-swap populator anywhere") and DC-CINPUT-02b (the lifecycle forge base's
    `pool_distr_view` comes from `from_seed_epoch_consensus_inputs`, not the bundle helper).
- **State machines affected:** none new. Reuses the existing forge engine + leader-check.
- **Persistence impact:** none in L5 (single-shot forge; forged-block durability is N-U). The forge
  reads recovered state; it persists no new format.
- **Network-visible impact:** none (no peer evidence harness; BA-02 is gated).
- **Out of scope:** L6 (BA-02 evidence + the live node produce run); any peer-accept claim; any
  change to `produce_mode`'s cold path or the forge engine; extraction of the forge engine to a
  neutral module; multi-epoch production; a live produce slot-loop on `--mode node`; new BLUE
  authority; forged-block durability (N-U).

## 6. Execution Boundary (TCB color)
- **BLUE (reuse only — no change):** `PoolDistrView::from_seed_epoch_consensus_inputs` (pure
  projection), the leader-check / VRF authorities, the BLUE forge/`self_accept`/Praos authorities
  that `run_real_forge` composes.
- **GREEN (reuse only):** the recovered `BootstrapState` → forge-base field mapping (pure over
  recovered state + tip).
- **RED:** the `forge_one_from_recovered` handoff driver (loads operator keys via `ProducerShell`,
  runs the single-shot forge); `run_real_forge` (RED composer, reused).
- **CI:** the extended `ci_check_consensus_input_provenance.sh`.

## 7. Invariants Preserved
- `CN-CINPUT-02` (registry) — the sidecar **populate** path stays contained to verified-bootstrap
  composers; L5 only **consumes** (projects) the recovered surface, never constructs/puts/encodes it.
- `DC-CINPUT-01` (registry) — the recovered-state production chain stays intact; L5 consumes its
  output, does not weaken it.
- `DC-CINPUT-02a` (registry) — the `PoolDistrView`/`ExpectedVrfInput` projection determinism is
  reused unchanged.
- `CN-PROD-03` (registry) — `produce_mode`'s cold-start diagnostic forge base is untouched (it stays
  the bundle/cold path; L5 does not convert it).
- `CN-FORGE-01..04`, `DC-FORGE-01` (registry) — the forged block still round-trips the same BLUE
  decode / `self_accept` / Praos-VRF authorities; L5 changes the forge *inputs'* provenance, never
  the forge/validate symmetry.
- `CN-NODE-01` — no second bootstrap/recovery/storage-init authority; L5 consumes the recovered
  `BootstrapState`, it does not re-bootstrap.

## 8. Invariants Strengthened or Introduced
- **Introduces the enforcement for `DC-CINPUT-02b` (producer consumes the recovered surface):** the
  node-lifecycle forge base's `pool_distr_view` is derived ONLY from
  `PoolDistrView::from_seed_epoch_consensus_inputs(recovered.seed_epoch_consensus_inputs)` — proven
  by the hermetic forge-from-recovered test + the extended CI gate.
- **Introduces the enforcement for `CN-CINPUT-03` (consume-side fence + no shape-swap populator):**
  the forge path references the recovered surface only as a consumer; no operator bundle is converted
  into a `SeedEpochConsensusInputs` anywhere; the forge path reads no `--consensus-inputs-path`.
- A slice strengthens one invariant family — here the **consensus-input provenance / consume-side**
  family (DC-CINPUT-02b + CN-CINPUT-03 are the two faces of "recovered surface, not bundle, drives
  the forge"). **No registry edit in-slice** — the registry append (the two new rules
  `declared`→`enforced` with `code_locus`/`tests`/`ci_script` populated, plus
  `strengthened_in += "PHASE4-N-F-C"` on `DC-CINPUT-01`/`DC-CINPUT-02a`/`CN-PROD-03`/`DC-FORGE-01`)
  happens at `/cluster-close`, consistent with L1–L4.

## 9. Design Summary

### 9.0 Entry obligations (resolved)
- **(F1) The recovered surface carries consensus inputs + base, NOT operator keys.** The recovered
  `BootstrapState` gives: `base_state` (recovered ledger), `chain_dep_state` + `eta0`
  (recovered `chain_dep` / its epoch nonce), `block_number` + `prev_hash` (selected tip), and
  `pool_distr_view` (from the recovered sidecar). It does NOT give `vrf_vk`, `pparams`,
  `protocol_version`, `prev_opcert_counter`, or the KES/VRF/opcert signing keys — those are operator
  custody (RED), loaded the `produce_mode` way (operator key inputs). This separation is correct and
  intended: **recovered state decides who may lead + the base; operator keys sign.** L5 sources keys
  from the operator key path, never from the recovered surface or a bundle.
- **(F2 — RESOLVED: reuse, no extraction.)** `run_real_forge` + `ForgeRequestContext` live in
  `produce_mode` and are `pub`. L5 reuses them verbatim
  (`crate::produce_mode::{run_real_forge, ForgeRequestContext}`). This is reuse of a public
  forge-engine surface, NOT a `produce_mode` conversion, and is acceptable iff: produce_mode's cold
  path is untouched; the node lifecycle does NOT call `pool_distr_view_from_consensus_inputs`; the
  node lifecycle does NOT read `--consensus-inputs-path`; and the node lifecycle constructs
  `ForgeRequestContext` from recovered state. L5 does NOT extract the forge engine into a neutral
  module — that would be a separate refactor slice and would add churn before the recovered-state
  handoff is proven.
- **(F3) A faithful single-shot forge runs the leader-check first.** `run_real_forge` needs a
  `leader_schedule_answer`; `produce_mode` obtains it from the leader-check over
  `(slot, eta0, pool_distr_view, vrf key)`. L5 runs the same leader-check, seeded with the RECOVERED
  eta0 (`recovered.chain_dep.epoch_nonce`) + the RECOVERED-surface `pool_distr_view`. Not-leader is
  a valid deterministic outcome (`ForgeNotLeader`), not a failure.
- **(F4 — RESOLVED: Option A.)** L5 implements a `forge_one_from_recovered(...)` handoff function
  plus hermetic single-shot tests. It does NOT wire a live `--mode node` produce loop. The handoff
  takes a recovered `BootstrapState` + selected tip + operator keys/pparams + slot and returns the
  reused `CoordinatorEvent`; the hermetic test proves it end-to-end (recovered fixture → projection
  from the recovered surface → `ForgeRequestContext` → `run_real_forge` →
  `ForgeSucceeded`/`ForgeNotLeader` deterministically), with NO bundle/cold/`--consensus-inputs-path`
  on the path. The live node produce loop is deferred to L6 / operator-run evidence work — it is NOT
  a mechanical CE here, and the live run must not be smuggled into L5.

### 9.1 The recovered forge-base assembly (grounded in the real shapes)
`ForgeRequestContext` (`produce_mode.rs:597`, 12 fields) is filled from recovered state + tip + keys:

| ctx field | L5 source (recovered / tip / operator) |
|---|---|
| `eta0` | `&recovered.chain_dep.epoch_nonce` (RECOVERED) |
| `pool_distr_view` | `&PoolDistrView::from_seed_epoch_consensus_inputs(recovered.seed_epoch_consensus_inputs)` (RECOVERED — the DC-CINPUT-02b point) |
| `base_state` | `&recovered.ledger` (RECOVERED) |
| `chain_dep_state` | `&recovered.chain_dep` (RECOVERED) |
| `block_number` | from the selected `tip` (L4) |
| `prev_hash` | from the selected `tip` (L4) |
| `era_schedule` | the node era schedule (single-epoch, cluster scope) |
| `vrf_vk` / `leader_schedule_answer` | operator VRF key + leader-check result (F1/F3) |
| `pparams` / `protocol_version` / `prev_opcert_counter` | node config / operator (F1) |

`run_real_forge(slot, kes_period, &ctx, shell) -> CoordinatorEvent`; success is
`ForgeSucceeded { slot, artifact }`. Single-shot: one slot, one attempt.

### 9.2 What makes DC-CINPUT-02b true (consume from recovered, not bundle)
The lifecycle forge path calls `PoolDistrView::from_seed_epoch_consensus_inputs` on the recovered
sidecar — NOT `pool_distr_view_from_consensus_inputs(&LiveConsensusInputsCanonical)` (the cold/bundle
helper that stays in `produce_mode` for the diagnostic path). The hermetic test asserts the
`pool_distr_view` the forge consumes equals `from_seed_epoch_consensus_inputs(recovered)`.

### 9.3 What makes CN-CINPUT-03 true (no shape-swap, no bundle on the forge path)
The extended `ci_check_consensus_input_provenance.sh`: the lifecycle forge module references
`from_seed_epoch_consensus_inputs(`, and does NOT reference `import_live_consensus_inputs` /
`pool_distr_view_from_consensus_inputs` / `consensus_inputs_path` / `InMemoryChainDb`; and no
production site anywhere constructs a `SeedEpochConsensusInputs` from a bundle (the populate-side
containment from CN-CINPUT-02 already restricts construction to the verified-bootstrap composers —
CN-CINPUT-03 adds the consume-side assertion that the forge path only projects, never builds).

## 10. Changes Introduced
### Types
- A RED forge-handoff entry `forge_one_from_recovered(...)` returning the reused `CoordinatorEvent`.
  Additional fail-closed error for "recovered state lacks `seed_epoch_consensus_inputs`" (a forge
  attempt on a non-recovered base is unrepresentable). No new BLUE/canonical type; reuses
  `ForgeRequestContext`/`CoordinatorEvent`.
### State Transitions
- None new (reuses the forge engine + leader-check).
### Persistence
- None (single-shot; durability is N-U).
### Removal / Refactors
- None to `produce_mode` cold path, the forge engine, `bootstrap.rs`, `replay.rs`, or L1–L4. No
  forge-engine extraction (F2).

## 11. Replay, Crash, and Epoch Validation
- **Replay (reused, preserved):** `run_real_forge_is_byte_identical_across_two_runs`
  (`crates/ade_node/tests/forge_handler_variants.rs:310`) — L5 keeps it green; the engine is unchanged.
- **Replay (new, this slice):** `forge_from_recovered_uses_recovered_pool_distr` (hermetic):
  given a recovered `BootstrapState` fixture with `seed_epoch_consensus_inputs: Some(..)` + a selected
  tip + test operator keys, assert (a) the forge ctx's `pool_distr_view` equals
  `from_seed_epoch_consensus_inputs(recovered)` and (b) `run_real_forge` returns a deterministic
  `ForgeSucceeded`/`ForgeNotLeader` (two runs byte-identical).
- **Crash/restart:** none new (single-shot, no persist). Forged-block durability is N-U.
- **Epoch:** single seed epoch; the leader-check uses the recovered eta0 for that epoch only.

## 12. Mechanical Acceptance Criteria
- [ ] Extended `ci/ci_check_consensus_input_provenance.sh` passes: the lifecycle forge path
      references `from_seed_epoch_consensus_inputs(` and references NO `import_live_consensus_inputs` /
      `pool_distr_view_from_consensus_inputs` / `consensus_inputs_path` / `InMemoryChainDb`; no
      production shape-swap populator anywhere (CN-CINPUT-03); produce_mode's existing forge-time
      fence stays green.
- [ ] Hermetic `forge_from_recovered_uses_recovered_pool_distr`: forge ctx `pool_distr_view` ==
      `from_seed_epoch_consensus_inputs(recovered)`; `run_real_forge` deterministic across two runs
      (DC-CINPUT-02b).
- [ ] Fail-closed test: a forge attempt where `recovered.seed_epoch_consensus_inputs.is_none()`
      returns a typed error / non-forge outcome — never a bundle/cold fallback.
- [ ] `ci/ci_check_node_sync_via_pump.sh` + the lifecycle-owner / mode-closure / bootstrap-closure
      gates stay green.
- [ ] `cargo build` + scoped `ade_node` tests + named gates pass. Full `ade_testkit` corpus/oracle
      lane is NOT an L5 gate (times out ~600s on clean HEAD).

## 13. Failure Modes (all fail-closed, typed)
Recovered state without `seed_epoch_consensus_inputs` ⇒ typed error (no forge, no fallback). A
`run_real_forge` `ForgeFailed` ⇒ surfaced typed; `ForgeNotLeader` ⇒ deterministic non-forge outcome
(not an error). No bundle / cold / `--consensus-inputs-path` / genesis path is reachable on the
forge path.

## 14. Hard Prohibitions
**Inherited (cluster):** no `--consensus-inputs-path` as a forge input; no cold `InMemoryChainDb`
forge base; no tip-bundle; no genesis fallback; no shape-swap of an operator bundle into
`SeedEpochConsensusInputs`; no second bootstrap/recovery/storage-init authority; no new BLUE
authority/type; no `HashMap`/clock/float in BLUE.
**Slice-specific (from the L5 brief):** no BA-02 claim; no peer-evidence harness; no multi-epoch
production; single-shot only until forged-block durability (N-U); no live `--mode node` produce loop
(deferred to L6 / operator-run evidence work); no change to `produce_mode`'s cold path; no extraction
of the forge engine to a neutral module; no change to the forge engine, `bootstrap.rs`, `replay.rs`,
or the L1–L4 surfaces; no registry status flip; no grounding-doc regeneration. **Process:** sequential
edits only — one edit, confirm, next.

## 15. Explicit Non-Goals
No L6 / BA-02 evidence; no peer-accept; no live `--mode node` produce slot-loop (the live node
produce loop is deferred to L6 / operator-run evidence work); no multi-epoch; no forged-block
durability (N-U); no forge-engine extraction; no registry append; no grounding-doc refresh.

## 16. Completion Checklist
- [ ] §9.0 F1–F4 honored (F4 = Option A: `forge_one_from_recovered` + hermetic single-shot tests, no
      live produce loop; F2 = reuse `produce_mode::run_real_forge`/`ForgeRequestContext`, no extraction).
- [ ] Forge handoff projects `pool_distr_view` from the recovered surface, assembles
      `ForgeRequestContext` from recovered base + selected tip, runs single-shot `run_real_forge`.
- [ ] Fail-closed when `seed_epoch_consensus_inputs` is absent; no bundle/cold/genesis fallback.
- [ ] DC-CINPUT-02b test + CN-CINPUT-03 gate extension green; existing provenance + sync + owner
      gates stay green.
- [ ] `produce_mode` cold path / forge engine / `bootstrap.rs` / `replay.rs` / L1–L4 unchanged.
- [ ] `cargo build` + scoped tests + named gates pass (full corpus lane excluded).

## 17. Review Notes
- **Invariant risk considered:** that the forge silently consumes a bundle-projected `pool_distr_view`
  (the produce_mode cold path) instead of the recovered surface. Fenced by DC-CINPUT-02b (the
  hermetic equality test) + CN-CINPUT-03 (the extended gate: forge path projects, never imports/builds).
- **Assumption challenged (F1):** operator keys are deliberately NOT in the recovered surface — the
  forge needs key custody + pparams the recovered state doesn't (and shouldn't) carry; sourcing them
  separately is correct, not a bundle leak.
- **Assumption challenged (F2):** reusing `produce_mode`'s `pub` forge engine ≠ converting
  produce_mode; the cold path is untouched, and the engine is not extracted in L5.
- **Follow-up slices implied:** L6 (BA-02 peer-accept evidence + the live node produce loop /
  operator-run evidence work); N-U (forged-block durability, lifts the single-shot limit).
