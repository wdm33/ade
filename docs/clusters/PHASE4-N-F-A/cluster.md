# Cluster PHASE4-N-F-A — Seed-Epoch Consensus Input Provenance

> **Status:** planned (cluster doc). Predecessor of PHASE4-N-F-B. No code/registry promotion yet.
> **Sources:** `docs/planning/phase4-n-f-cluster-slice-plan.md` (N-F-A) · `docs/planning/phase4-n-f-invariants.md` · guardrail memory `feedback_produce_subordinate_to_sync_spine`.

## Primary invariant
The seed-epoch consensus inputs (eta0, PoolDistr, ASC, pool_vrf_keyhashes) established **during verified bootstrap** become an **anchor-bound, canonical, version-gated, persisted, recovered** Ade state surface, such that `bootstrap → persist → recover` restores them **byte-identically** — and the bounty-primary leader/forge path consumes them **only** from that recovered surface, never from a forge-time bundle. *(Candidate `CN-CINPUT-01`; strengthens `CN-ANCHOR-01`/`DC-ANCHOR-01`, `CN-NODE-01`, `T-REC-01`/`T-REC-02`.)*

This cluster **does not forge**. It establishes and proves the recovered state that makes BA-02 production safe.

## Normative anchors
- `docs/planning/phase4-n-f-invariants.md` — NF-1 (consensus inputs from recovered state), RD-OQ2 (operator bundle diagnostic-only).
- Registry: `CN-ANCHOR-01` / `DC-ANCHOR-01` (sole anchor codec, version-gated byte-canonical round-trip), `CN-MITHRIL-01` / `DC-MITHRIL-02` (seed-provenance binding + data-flow-resistant containment), `CN-NODE-01` (single bootstrap authority), `T-REC-01` / `T-REC-02` (recovery replay-equivalence), `DC-SYNC-01` (durable-before-tip).

## Entry conditions (what shipped clusters guarantee)
- **N-M-A / N-Z:** `BootstrapAnchor` + sole codec `encode/decode_bootstrap_anchor` (`array(8)`, `ANCHOR_SCHEMA_VERSION = 2`, version-gated, byte-canonical), `SeedProvenance` closed, `verify_mithril_binding`, the documented-interface seed extraction.
- **N-M-C:** `LiveConsensusInputsCanonical { active_slots_coeff, epoch_nonce, pool_distribution: BTreeMap<Hash28,PoolEntry>, pool_vrf_keyhashes: BTreeMap<Hash28,Hash32> }` (`consensus_inputs/canonical.rs:54-64`) — the *bootstrap-time* extraction shape (today re-read each run; this cluster persists it).
- **N-Y:** `recover_node_state(chaindb, snapshot_store, wal, anchor_fp, era_schedule, ledger_view, genesis_initial) -> Result<RecoveredNode{ledger, chain_dep, tip}>` (`recovery/restart.rs:111`) — already receives `anchor_fp`; takes `ledger_view: &dyn LedgerView` as the **external input this cluster closes**. eta0 is already recovered via `RecoveredNode.chain_dep.epoch_nonce`.
- **Projection targets exist:** `PoolDistrView::new(epoch, total_active_stake, asc, pools)` impl `LedgerView { total_active_stake, pool_active_stake, pool_vrf_keyhash, active_slots_coeff }` (`ade_ledger::consensus_view`); `ExpectedVrfInput` via `leader_vrf_input(era, slot, epoch_nonce)` (`ade_core::consensus::vrf_cert`).

## 1. Authority surface
N-F-A owns exactly one new authority: the **persisted seed-epoch consensus-input record** + its canonical codec + its anchor binding + the projection from the *recovered* record to `PoolDistrView`/`ExpectedVrfInput`. It owns no forge, no wire, no stake *computation* (it persists already-extracted seed values; it never derives stake by ledger projection or rotation).

## 2. Inputs and origins
- **Origin (bootstrap-time only):** the documented seed extraction (`cardano-cli query protocol-state`/`stake-snapshot` + genesis ASC, per the N-Z runbook), already canonicalized into `LiveConsensusInputsCanonical`. This is part of verified bootstrap, alongside the UTxO seed + `verify_mithril_binding`.
- **Forbidden origin:** a forge-time `--consensus-inputs-path`. After this cluster, that flag may populate **only** a diagnostic fixture, never the bounty-primary recovered surface.
- eta0 origin is unchanged (already in `chain_dep`); N-F-A binds PoolDistr/ASC/vrf_keyhashes to the **same** anchor + `epoch_no` as the recovered `chain_dep`.

## 3. Persisted canonical type
A new BLUE closed record `SeedEpochConsensusInputs { epoch_no, active_slots_coeff, total_active_stake, pool_distribution: BTreeMap<Hash28,PoolEntry>, pool_vrf_keyhashes: BTreeMap<Hash28,Hash32> }` (eta0 stays in `chain_dep`; `epoch_no` ties the two together for the single seed epoch). Sole encoder/decoder pair, deterministic CBOR, `BTreeMap` (no `HashMap`), version-gated, byte-canonical round-trip — mirroring the `CN-ANCHOR-01`/`DC-ANCHOR-01` discipline. No `Default`, no `#[non_exhaustive]`, all fields required at construction.

## 4. Anchor binding

**A1 decides the storage shape.**

**Preferred shape unless size analysis proves otherwise:** a separate canonical `SeedEpochConsensusInputs` record keyed by `anchor_fp` and referenced from `BootstrapAnchor` by hash/fingerprint.

The invariant is **not** "PoolDistr lives inside `BootstrapAnchor`." The invariant is:
- the record is **anchor-bound**;
- **version-gated**;
- **byte-canonical**;
- **recovered byte-identically**;
- **unavailable to produce unless restored from Ade persistence**.

A full anchor schema bump (2 → 3, the record as a 9th outer field) is **allowed only if A1 proves the serialized record is bounded enough** that the anchor remains an identity/provenance object rather than a large payload container. (`pool_distribution` can be large — preprod/mainnet pool counts — so embedding it risks turning the anchor from an identity object into a payload container; the fingerprint-bound sidecar keeps the single-authority principle without that bloat.)

Either shape uses a single sole codec, version-gated decode (rejects unknown version fail-fast), and a byte-canonical round-trip — and is bound to *this* anchor (so a record cannot be silently paired with a different anchor).

## 5. Recovery contract
Recovery is **production-path-real**: node startup runs `bootstrap_initial_state` (`ade_node::node.rs:145`), whose **warm-start** branch is the real restart path (reads persisted state via `SnapshotStore`). N-F-A persists the `SeedEpochConsensusInputs` sidecar through a **dedicated keyed `SnapshotStore` method** (`put_/get_seed_epoch_consensus_inputs(anchor_fp, …)`, keyed by `anchor_fp` / the anchor fingerprint type — **NOT** a reserved sentinel slot; the sidecar is anchor-bound metadata, not a block/slot snapshot) and makes `bootstrap_initial_state` warm-start **restore it byte-identically**, replacing the external `ledger_view: &dyn LedgerView` parameter on the bounty-primary warm-start path with the restored record (the external `LedgerView` remains only for the diagnostic path, fenced). Extends `T-REC-01`/`T-REC-02`. Fail-fast (typed error) on version mismatch, anchor-fingerprint mismatch, or decode divergence — never a silent fallback to a bundle.

> **`recover_node_state` (the WAL-replay recovery helper) is currently test/capability-only — not wired into `ade_node` startup (all callers are `#[cfg(test)]`), so it is NOT the production proof path.** It may carry optional *secondary* coverage only; CE-A-3 is proven on the production warm-start path, not on this helper. Wiring `recover_node_state` into startup is a separate concern (a later cluster), out of N-F-A scope.

## 6. Projection API → PoolDistrView / ExpectedVrfInput
A BLUE projection `SeedEpochConsensusInputs → PoolDistrView` via `PoolDistrView::new(epoch_no, total_active_stake, asc, pool_distribution)` (supplying the full `LedgerView` surface: `total_active_stake`, `pool_active_stake`, `pool_vrf_keyhash`, `active_slots_coeff`), and eta0 (from recovered `chain_dep`) → `ExpectedVrfInput` via `leader_vrf_input(era, slot, epoch_nonce)`. This projection is what the bounty-primary path calls, **replacing** `pool_distr_view_from_consensus_inputs` (`produce_mode.rs:401`) which sourced it from the operator bundle.

## 7. CI containment gate (forge-time bundle fenced out)
A data-flow-resistant containment gate (N-Z `ci_check_mithril_seed_point_independence.sh` style), candidate `ci_check_consensus_input_provenance.sh`: the persisted `SeedEpochConsensusInputs` surface may be populated **only** on the bootstrap path; the bounty-primary leader/forge/recovery path may reference the recovered surface, never `--consensus-inputs-path` / `import_live_consensus_inputs` / `pool_distr_view_from_consensus_inputs` at forge time; and the projection consumes the *recovered* record, not a freshly-imported bundle. Enforces the KEY REVIEW POINT: **no hidden second operator import path.**

## 8. Replay fixtures
- `corpus/.../seed_epoch_consensus_inputs/bootstrap_persist_warmstart.{...}` — a `bootstrap + keyed-SnapshotStore persist + bootstrap_initial_state warm-start` fixture proving production byte-identity (CE-A-3).
- A pinning fixture: the projected `PoolDistrView`/`ExpectedVrfInput` from the recovered surface equals the seed-epoch peer-equivalent (the slice-entry proof obligation).

## 9. Failure modes (all fail-closed, typed)
Unknown schema version on decode; anchor-fingerprint mismatch (surface not bound to *this* anchor); recovery byte-divergence; an epoch other than the seed epoch queried (the single-epoch `PoolDistrView` already returns `None`); a forge-time bundle attempting to populate the surface (CI-blocked). None silently degrade to the operator bundle.

## 10. Non-goals
No forge (that is N-F-B). No stake **computation** or epoch-boundary **rotation** (cross-epoch is a separate predecessor; this persists already-extracted seed-epoch constants). No new wire protocol. No second operator import path. No grounding-doc regeneration.

## Exit criteria (mechanical)
- **CE-A-1** — `SeedEpochConsensusInputs` closed type + sole codec; round-trip test `seed_epoch_consensus_inputs_round_trips_byte_identical` + version-gated `decode_rejects_unknown_version` pass.
- **CE-A-2** — bootstrap populates it; `ci_check_consensus_input_provenance.sh` passes (bootstrap-only population + forge-time bundle fenced + anchor-bound).
- **CE-A-3** — **production warm-start** byte-identity: test `warm_start_restores_seed_epoch_consensus_inputs_byte_identical` proves `production bootstrap + keyed-SnapshotStore sidecar persist + bootstrap_initial_state warm-start = the same eta0/PoolDistr/ASC/pool_vrf_keyhashes` — NOT a test-only `recover_node_state` capability. (recover_node_state may carry optional secondary coverage only.)
- **CE-A-4** — projection test `recovered_surface_projects_pooldistrview_and_expected_vrf_input` shows the projection equals the prior `pool_distr_view_from_consensus_inputs` output for the seed epoch, and the bounty-primary call site consumes the recovered surface.

## Expected slice types
- **A1** — define `SeedEpochConsensusInputs` + sole codec + **decide the anchor-binding storage shape** (prefer a fingerprint-bound sidecar keyed by `anchor_fp`; a schema bump 2→3 only if A1 proves the serialized record bounded) — BLUE.
- **A2** — bootstrap-time population + `ci_check_consensus_input_provenance.sh` containment — RED/GREEN + CI.
- **A3** — `bootstrap_initial_state` warm-start restores the sidecar (dedicated keyed `SnapshotStore` get, by `anchor_fp`) byte-identically + production-path replay test; replaces the external `ledger_view` on the bounty-primary warm-start; `recover_node_state` optional secondary coverage only — RED restore + BLUE replay.
- **A4** — projection API → `PoolDistrView`/`ExpectedVrfInput`, replacing `pool_distr_view_from_consensus_inputs` on the bounty-primary path — BLUE.

## TCB color map
- **BLUE** — `SeedEpochConsensusInputs` + codec + anchor-binding (in `ade_ledger::bootstrap_anchor`), the projection (in `ade_ledger::consensus_view`). *(Anchor codec discipline already BLUE.)*
- **GREEN** — the recovery carry/restore reducer glue (`ade_runtime` GREEN-by-content).
- **RED** — the dedicated keyed `SnapshotStore` put/get (`ade_runtime::chaindb` trait + impls) + `bootstrap_initial_state` warm-start restore wiring (`ade_runtime::bootstrap`) + the bootstrap-time population shell. (`recover_node_state` read = optional secondary only.)
- **Open color** — whether the persisted record's *population from the canonical bootstrap extraction* is BLUE (deterministic transform) vs GREEN; A1/A2 resolve (lean BLUE for the transform, RED for the file/store I/O — the established loader→deserializer split).

## Forbidden during this cluster
No forge / leader-check / KES / VRF signing. No second anchor codec or storage-init authority. No `--consensus-inputs-path` populating the bounty-primary surface. No stake computation/rotation. No `HashMap` (use `BTreeMap`), no clock/float/async in BLUE. No registry promotion. No grounding-doc regeneration.

## Invariants strengthened / candidate new rules (proposed, NOT promoted)
- **Candidates:** `CN-CINPUT-01` (closed persisted surface + sole codec + anchor binding), `CN-CINPUT-02` (bootstrap-time-only population + forge-time fence), `DC-CINPUT-01` (recovery byte-identity), `DC-CINPUT-02` (projection API authority). *(Family letter/numbers confirmed at slice-doc/implement time.)*
- **Strengthenings:** `CN-ANCHOR-01`/`DC-ANCHOR-01` (binding the new record; schema bump only if A1 proves bounded), `CN-NODE-01`, `T-REC-01`/`T-REC-02`, `DC-MITHRIL-02` (containment lineage).

## Replay obligations
New persisted authoritative surface → the **production warm-start** (`bootstrap_initial_state`) restores it byte-identically (CE-A-3, the core safety line). New BLUE canonical type (version-gated, anchor-bound) → extends the canonical-type count. New keyed `SnapshotStore` method (put/get by `anchor_fp`). New corpus: `bootstrap_persist_warmstart` + the projection pinning fixture.
