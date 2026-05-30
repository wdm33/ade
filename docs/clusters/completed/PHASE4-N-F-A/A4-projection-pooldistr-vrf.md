# Slice PHASE4-N-F-A / A4 — BLUE projection: recovered surface → PoolDistrView / ExpectedVrfInput

> **Scope honesty (pre-implementation, 2026-05-30).** A4 is the **BLUE
> projection capability + its pinning proof**, NOT the bounty-primary
> call-site swap. Grounding against code: the producer
> (`produce_mode.rs:188-197`) still imports the operator bundle
> (`import_live_consensus_inputs(--consensus-inputs-path)`), builds its
> `PoolDistrView` from that bundle via
> `pool_distr_view_from_consensus_inputs`, and **cold-starts**
> `bootstrap_initial_state` (`InMemoryChainDb` + `genesis_initial`,
> `NotRequired`). The recovered `SeedEpochConsensusInputs` surface does
> **not** reach the producer yet — that wiring is the deferred A5 slice
> (A3b doc §16). So A4 builds + pins the projection; A4 does **not**
> rewire the call site. Same capability-vs-wiring split that kept A3a/A3b
> honest.

## 2. Slice Header
- **Slice Name:** A BLUE pure projection from the recovered `SeedEpochConsensusInputs` (+ recovered eta0) to the leadership-consumed `PoolDistrView` / `ExpectedVrfInput`, with a pinning test proving it equals the prior bundle-path output for the seed epoch.
- **Cluster:** PHASE4-N-F-A. **Status:** Merged (`8b60524`); CE-A-4a closed, CE-A-4b deferred to PHASE4-N-F-C.
- **Cluster Exit Criteria Addressed — CE-A-4 is SPLIT:**
  - [ ] **CE-A-4a** (this slice) — projection test proves the recovered `SeedEpochConsensusInputs` projects to a `PoolDistrView` / `ExpectedVrfInput` **equivalent to the prior `pool_distr_view_from_consensus_inputs` output** for the same seed-epoch fixture.
  - [ ] **CE-A-4b** (assigned to **A5 production wiring**, NOT A4) — the bounty-primary produce call site consumes the recovered surface. A4 does not claim this.
- **Slice Dependencies:** A1 (`SeedEpochConsensusInputs` type), A3b (the recovered surface exists as `BootstrapState.seed_epoch_consensus_inputs`; eta0 recovered in `chain_dep`).

## 3. Implementation Instruction (AI)
Implement §9/§10 only — the BLUE projection fns + the pinning test. Do **not** rewire `produce_mode`, do **not** replace the bounty-primary call site, do **not** touch `--consensus-inputs-path`, do **not** make produce consume the recovered surface, do **not** claim CE-A-4b. Commit with the trailer.

## 4. Intent
Prove that the **recovered** seed-epoch consensus inputs carry exactly the leadership semantics the forge path needs: projecting `SeedEpochConsensusInputs` to `PoolDistrView` (the `LedgerView` the BLUE leader check consumes) and recovered eta0 to `ExpectedVrfInput` (via `leader_vrf_input`) yields output **equivalent** to today's operator-bundle projection for the seed epoch. This is the last piece that makes the recovered surface a drop-in source for leadership — so the A5 wiring can swap the source with a mechanical equivalence already proven. *(Completes the projection half of `DC-CINPUT-02`.)*

## 5. Scope
- **Modules:** `ade_ledger::consensus_view` (BLUE) — add the projection fn(s). Reuses `PoolDistrView::new` (same module) and `ade_core::consensus::vrf_cert::leader_vrf_input` (already an `ade_ledger` dep).
- **State machines:** none.
- **Persistence:** none (pure transform).
- **NOT in scope (A5 production wiring — explicitly deferred):** rewiring `produce_mode`; replacing `pool_distr_view_from_consensus_inputs` at the bounty-primary call site; making produce consume the recovered `SeedEpochConsensusInputs`; any `--consensus-inputs-path` change.
- **Out of scope:** produce/forge logic; the operator-bundle importer (untouched).

## 6. Execution Boundary (TCB)
- **BLUE:** the projection fn(s) — pure, deterministic, `BTreeMap`, no I/O / clock / float. Lives in `ade_ledger::consensus_view` (already BLUE).
- **GREEN / RED:** none.

## 7. Invariants Preserved
- `CN-CINPUT-01` (A1) — the projection reads the A1 record's fields; it does not re-decode or re-encode the sidecar.
- `PoolDistrView` single-epoch semantics — the projection sets `PoolDistrView`'s epoch to the recovered `epoch_no`; queries for any other epoch still return `None` (no widening).
- BLUE forbidden-pattern set; determinism; `BTreeMap` ordering (carried from the A1 record).
- The bundle path (`pool_distr_view_from_consensus_inputs`, `import_live_consensus_inputs`) is **unchanged** — A4 adds a second projection source, it does not reroute the existing one.

## 8. Invariants Strengthened or Introduced
- **Introduces** the projection half of candidate `DC-CINPUT-02` — *the recovered `SeedEpochConsensusInputs` projects deterministically to the same `PoolDistrView` / `ExpectedVrfInput` semantics the leader check consumes, equivalent to the prior bundle path for the seed epoch.* (A5 completes `DC-CINPUT-02`'s authority half by making the bounty-primary path consume it.)

## 9. Design Summary
- **`PoolDistrView` projection** — a BLUE fn (e.g. `PoolDistrView::from_seed_epoch_consensus_inputs(&SeedEpochConsensusInputs) -> PoolDistrView`, or a free fn in `consensus_view`). Near-direct field map, because A2 already merged stake + VRF keyhash into the single `BTreeMap<Hash28, PoolEntry>`:
  - `epoch ← record.epoch_no`
  - `total_active_stake ← record.total_active_stake`
  - `asc ← record.active_slots_coeff`
  - `pools ← record.pool_distribution.clone()`
  No zip, no zero-hash fallback (the A1 `PoolEntry` already carries `vrf_keyhash`). *(Fidelity note: A2's merge fails closed on a pool present in only one map, so `total_active_stake` equals `sum(pool active_stake)`; the pinning test asserts this against the bundle projection rather than assuming it.)*
- **`ExpectedVrfInput` projection** — eta0 is **not** in `SeedEpochConsensusInputs`; it is recovered in `chain_dep.epoch_nonce`. So the VRF-input projection is the existing `leader_vrf_input(era, slot, &epoch_nonce)` called with the recovered nonce. A4 does **not** introduce a new VRF fn; it documents (and tests) that the recovered eta0 drives `leader_vrf_input` identically to the bundle's eta0. *(If a thin wrapper aids the pinning test it stays BLUE + trivial; prefer calling `leader_vrf_input` directly.)*
- **Equivalence is the deliverable:** the pinning test builds one `LiveConsensusInputsCanonical` bundle fixture and the `SeedEpochConsensusInputs` that A2's merge would produce from it, then asserts the two `PoolDistrView`s are equal (`LedgerView` surface: `total_active_stake`, `pool_active_stake`, `pool_vrf_keyhash`, `active_slots_coeff` agree for the seed epoch and return `None` off-epoch), and that `leader_vrf_input` on the bundle eta0 vs the recovered eta0 are equal.

## 10. Changes Introduced
- **Types:** none new (reuses `PoolDistrView`, `ExpectedVrfInput`, `SeedEpochConsensusInputs`).
- **Fns:** one BLUE projection `SeedEpochConsensusInputs → PoolDistrView` in `ade_ledger::consensus_view`. No new VRF fn (reuse `leader_vrf_input`).
- **Transitions / persistence / callers:** none. `produce_mode` untouched. No call-site swap.

## 11. Replay, Crash, and Epoch Validation
- **Tests:** `recovered_surface_projects_pooldistrview_and_expected_vrf_input` (CE-A-4a) — projection equals the prior `pool_distr_view_from_consensus_inputs` output for the same seed-epoch fixture, on the full `LedgerView` surface + off-epoch `None`; plus the eta0 → `leader_vrf_input` equivalence. `projection_two_runs_identical` (determinism). `projection_off_epoch_returns_none` (single-epoch semantics preserved).
- **Crash / epoch boundary:** n/a (pure transform, single seed epoch).

## 12. Mechanical Acceptance Criteria
- [ ] **CE-A-4a:** `recovered_surface_projects_pooldistrview_and_expected_vrf_input` passes (projection equivalence to the bundle path for the seed epoch).
- [ ] `projection_two_runs_identical` + `projection_off_epoch_returns_none` pass.
- [ ] `cargo build` + `cargo clippy` clean for the changed crate (`ade_ledger`); affected-crate tests green (`cargo test -p ade_ledger`, plus `ade_runtime`/`ade_node` if anything references the new fn) — NOT `cargo test --workspace` (`ade_testkit` corpus/oracle suite times out on clean HEAD; pre-existing/environmental, per the A3a closure + memory `reference_ade_testkit_corpus_suite_times_out`).
- [ ] `ci_check_consensus_input_provenance.sh` still passes (A4 adds no populator/append/forge-path reference).
- [ ] **CE-A-4b is NOT claimed by A4** (assigned to A5).

## 13. Failure Modes
A4 is a pure projection with no fallible path of its own — it reads already-validated A1 fields. (The A1 record was fail-closed verified at construction/recovery in A2/A3b; A4 does not re-validate.) The single-epoch `PoolDistrView` returns `None` off-epoch by construction. No panics, no `String`/`anyhow`.

## 14. Hard Prohibitions
**Inherited (cluster):** no forge/leader-check/KES/VRF signing; no second anchor codec; no stake computation/rotation; no `HashMap`; no clock/float/async in BLUE; no registry promotion; no grounding-doc regeneration.
**Slice-specific:** **no `produce_mode` rewire / no bounty-primary call-site swap** (that is A5 / CE-A-4b); no `--consensus-inputs-path` change; no rerouting of the existing bundle projection; no new VRF fn (reuse `leader_vrf_input`); no claim of CE-A-4b or production-path consumption; the projection is pure (no I/O).

## 15. Explicit Non-Goals
No production wiring of the recovered surface into produce (A5 / CE-A-4b). No produce/forge change. No bundle-path removal (it stays until A5 swaps it). No META pointer.

## 16. Completion Checklist
- [ ] BLUE projection fn added in `ade_ledger::consensus_view`, pure + deterministic.
- [ ] Pinning test proves projection equivalence to the bundle path for the seed epoch (CE-A-4a).
- [ ] Determinism + off-epoch tests pass.
- [ ] `produce_mode` / bounty-primary call site untouched; CE-A-4b explicitly deferred to A5.
- [ ] §12 tests + CI gate pass; `cargo test -p ade_ledger` green.
