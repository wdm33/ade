# Cluster/Slice Plan — Ade · PHASE4-N-T

> Built from `docs/planning/phase4-n-t-invariants.md`.
> Scope-locked to **Problem 1** (real in-memory production state). Durable
> restart (WAL append / ChainDB store / snapshot cadence / warm-start
> recovery) is **deferred to a later cluster (provisionally N-U)** and is
> explicitly **not** claimed here.
> Baseline HEAD: `dbee4d5`. Cluster-ID format: `named` (cluster
> `PHASE4-N-T`, slices `S1`–`S5`).

## Cluster Index (Dependency Order)

1. **PHASE4-N-T — produce_mode real-bootstrap composition** — primary
   invariant: *produce_mode forges a coherent, chain-extending block
   sequence from real `bootstrap_initial_state`-derived state (never
   synthetic), and every self-accepted forged block reaches the served
   snapshot — in one continuous run.*

Single cluster.

---

## PHASE4-N-T — produce_mode real-bootstrap composition

- **Primary invariant:** produce_mode's forge inputs are derived from the
  real bootstrap state (cold-start from the operator seed via the sole
  `bootstrap_initial_state` authority) and threaded forward across forges
  by a linear GREEN `ChainEvolution` typestate; every self-accepted forged
  block is admitted to the served snapshot via the single `push_atomic`
  authority. No synthetic forge state; no fork off a stale base; no
  silently-dropped broadcast; only self-accepted blocks are served.

- **TCB partition:**
  - **BLUE** (reused, no new authority):
    `ade_ledger::consensus_view::PoolDistrView` (stake projection),
    `ade_core::consensus::leader_schedule` (schedule query),
    `ade_core::consensus::leader_check`,
    `ade_ledger::producer::{forge, self_accept, served_chain_admit}`,
    `ade_ledger::block_validity` (post-state application).
  - **GREEN**: `ade_runtime::bootstrap::bootstrap_initial_state` (reused);
    **NEW `ade_runtime::producer::chain_evolution`** (the `ChainEvolution`
    linear typestate); `ade_runtime::producer::{coordinator,
    producer_log, broadcast_to_served}` (reused).
  - **RED**: `ade_node::produce_mode` (slot loop, absolute-slot ticker,
    `push_atomic` call, evidence I/O); `ade_runtime::{seed_import,
    consensus_inputs}` importers (reused RED); `ade_node::cli`
    `ProduceCli` extension.

- **Cluster Exit Criteria:**
  - **CE-1** — produce_mode obtains its initial forge state via
    `bootstrap_initial_state` (cold-start branch, fed the operator-seeded
    ledger); `SyntheticForgeInputs` / `build_synthetic_forge_context` are
    deleted; a CI gate forbids any synthetic forge-state bypass.
    *(CN-PROD-03; strengthens CN-NODE-01, CN-PROD-02)*
  - **CE-2** — per-slot `ForgeRequestContext` is derived from a linear
    GREEN `ChainEvolution` typestate (real eta0 / `PoolDistrView` stake /
    prev-hash / block-number / absolute slot from the bootstrap tip),
    advanced on each `ForgeSucceeded`; forging against a stale base is
    structurally unrepresentable; two runs over the same (seed,
    slot-sequence, keys) yield a byte-identical chain-evolution series +
    forged bytes. *(DC-PROD-03; strengthens CN-FORGE-01, DC-CONS-18)*
  - **CE-3** — every `BroadcastBlock` effect reconstructs the
    `AcceptedBlock` from `artifact.bytes` through the BLUE `self_accept`
    authority (against the pre-forge base) and admits it to the served
    snapshot via `ServedChainHandle::push_atomic`; if the `self_accept`
    replay rejects, `push_atomic` is not called and the loop emits
    structured `BroadcastPushError::SelfAcceptReplayRejected`;
    `ProducerLogEvent::BlockServed` is emitted only for blocks present in
    the served snapshot. *(CN-PROD-04; strengthens CN-SNAPSHOT-01)*
  - **CE-4** — a hermetic loopback integration test proves forge →
    served → block-fetch readback in one continuous run; the new CI gate
    enforces "produce uses `bootstrap_initial_state`, no synthetic
    bypass"; all new registry rules flip to `enforced`; cluster closes.

- **Slices:**
  - **S1 — Real bootstrap-state startup (behavior-preserving)** —
    invariant: produce_mode constructs the real `(ledger, chain_dep,
    tip)` from the operator `--json-seed` + `--consensus-inputs` bundle
    (mirroring admission's *construction* path) and routes it through
    `bootstrap_initial_state`'s cold-start branch (empty in-memory store);
    derives `EraSchedule` (`make_schedule_for_imported_window`),
    `LiveLedgerView`, and `PoolDistrView`. Forge path still uses synthetic
    (deleted in S3), so observable behavior is unchanged. Extends
    `ProduceCli` with the seed / consensus-inputs flags. — addresses: CE-1
    — TCB: RED (`produce_mode`, `cli`) consuming GREEN `bootstrap` + RED
    importers.
  - **S2 — GREEN `ChainEvolution` typestate (standalone)** — invariant: a
    pure linear typestate —
    `seed(bootstrap_triple) -> ChainEvolution`,
    `derive_forge_context(&self, slot) -> ForgeRequestContext`,
    `advance(self, forged_bytes) -> Result<(ChainEvolution, AcceptedBlock), ChainEvolutionError>`
    **by invoking the existing BLUE `self_accept` authority against the
    consumed pre-forge base — `ChainEvolution` never constructs
    `AcceptedBlock` directly** (preserves OQ1: authority is reconstructed
    only through `self_accept`, never minted by GREEN code). Illegal
    "advance against a stale base" is structurally unrepresentable
    (consumes `self`). Unit tests prove chain-forward determinism +
    two-run byte identity + the closed `ChainEvolutionError` rejection
    path. Not yet wired. — addresses: CE-2 — TCB: GREEN
    (`ade_runtime::producer::chain_evolution`).
  - **S3 — Wire `ChainEvolution` into produce_mode + delete synthetic** —
    invariant: seed `ChainEvolution` from S1's bootstrap state; derive the
    per-slot `ForgeRequestContext` from it (real stake / eta0 / prev-hash
    / block-number); the slot ticker uses the **absolute** start slot from
    `tip.slot` (cold-start: consensus-inputs epoch window); on
    `ForgeSucceeded`, `advance` threads the chain forward (the single
    `self_accept` replay lives here), and an `advance` failure fail-closes
    with a structured error; the returned `AcceptedBlock` is not yet
    served (broadcast remains no-op until S4). **Delete
    `SyntheticForgeInputs` + `build_synthetic_forge_context`.** Forging
    goes directly never→coherent-chain-extending (no fork-prone
    intermediate). — addresses: CE-1, CE-2 — TCB: RED (`produce_mode`)
    consuming GREEN.
  - **S4 — BroadcastBlock → served (forged ⇒ servable)** — invariant: the
    `BroadcastBlock` arm routes the `AcceptedBlock` produced by `advance`
    to `ServedChainHandle::push_atomic` on the handle currently discarded
    at `produce_mode.rs:209`; **if the `self_accept` replay rejects
    `artifact.bytes`, `push_atomic` is not called and the loop emits
    structured `BroadcastPushError::SelfAcceptReplayRejected`** — so only
    self-accepted forged blocks are served; `BlockServed` is emitted on
    the block-fetch serve path only for blocks present in the served
    snapshot. `drain_and_admit` is **not** used (it is the N-G queue-batch
    path; the coordinator emits one `BroadcastBlock` per forge, so
    per-artifact `push_atomic` is the fit — **OQ2 resolved**). —
    addresses: CE-3 — TCB: RED (`produce_mode`) consuming BLUE
    `self_accept` / `served_chain_admit` + RED `push_atomic`.
  - **S5 — Loopback test + CI gate + cluster close** — invariant: a
    hermetic loopback integration test (produce forges against a real
    seed → served → loopback block-fetch reads the bytes back, asserting
    forge↔served↔fetched byte identity and two-run replay equality); new
    CI gate `ci_check_produce_mode_uses_bootstrap_initial_state.sh` —
    **positive** grep (a `bootstrap_initial_state(` call in
    `produce_mode`) + **negative** grep (no `SyntheticForgeInputs` /
    `build_synthetic_forge_context` / inline `LedgerState::new(`
    forge-base) — strengthening **both** CN-NODE-01 and CN-PROD-02
    (**OQ6 resolved**); flip CN-PROD-03 / CN-PROD-04 / DC-PROD-03 to
    `enforced`; cluster close. — addresses: CE-4 — TCB: RED (test, CI).

- **Replay obligations:** One new replay-equivalence obligation —
  **DC-PROD-03** (chain-forward continuity): same `(seed, slot-sequence,
  keys)` → byte-identical chain-evolution series (`block_number`,
  `prev_hash`, post-ledger fingerprint, post-`chain_dep`) and
  byte-identical forged bytes; plus served-snapshot fingerprint stability
  (extends `drain_and_admit`'s proven property to the `push_atomic`
  path). These are **in-memory** two-run tests (S2 unit + S5 integration)
  — **no new on-disk replay corpus** (durability deferred to N-U). No new
  BLUE canonical types (the `ChainEvolution` types are GREEN). No new
  authoritative serialization surface — forged-block bytes are already
  governed by the existing forge authority (DC-CONS-18 / DC-FORGE-01).

- **Persistence non-claim (documented, not a CE):** N-T does not claim
  crash recovery of forged blocks. After process restart, any
  forged-but-not-persisted in-memory chain is not recovered. This is
  acceptable for the block-acceptance artifact and is **not** a
  substitute for DC-STORE / DC-WAL enforcement.

---

## Registry entries (finalized — to append at `/cluster-doc` time)

**NEW** (`tier="derived"`, `status="declared"`, `introduced_in="PHASE4-N-T"`, `cluster="PHASE4-N-T"`):
- **CN-PROD-03** — Bootstrap-derived forge state. produce_mode's forge
  base state is the `bootstrap_initial_state` cold-start triple +
  ledger/bundle-derived `PoolDistrView` + absolute slot from the
  bootstrap tip; `SyntheticForgeInputs` removed.
  `cross_ref = [CN-NODE-01, CN-PROD-02, CN-FORGE-02, DC-NODE-03]`.
  `ci_script = ci/ci_check_produce_mode_uses_bootstrap_initial_state.sh`.
- **CN-PROD-04** — Broadcast reaches served chain. Every `BroadcastBlock`
  reconstructs the `AcceptedBlock` through the BLUE `self_accept`
  authority and admits it via the single `push_atomic` authority before
  the next tick; a `self_accept` replay rejection fail-closes with
  `BroadcastPushError::SelfAcceptReplayRejected` (no push); `BlockServed`
  emitted only for served blocks present in the snapshot; no no-op
  broadcast. `cross_ref = [CN-SNAPSHOT-01, DC-PROD-01]`.
- **DC-PROD-03** — Chain-forward continuity + replay. Each forge linearly
  extends the prior post-state via the GREEN `ChainEvolution` typestate
  (`AcceptedBlock` reconstructed only through BLUE `self_accept`); same
  `(seed, slot-sequence, keys)` → byte-identical chain-evolution series +
  forged bytes (in-memory two-run).
  `cross_ref = [T-DET-01, DC-CONS-18, DC-PROD-02, CN-FORGE-01]`.

**STRENGTHENINGS** (`strengthened_in += "PHASE4-N-T"`):
- `CN-NODE-01` — sole bootstrap (cold-start branch) now *used* by
  `--mode produce`, not bypassed *(single-chokepoint, no parallel
  synthetic path — not a warm-start claim)*.
- `CN-PROD-02` — synthetic forge-state shortcut removed.
- `CN-FORGE-01` — `ForgeSucceeded` path reachable end-to-end against real
  state.
- `CN-SNAPSHOT-01` — `push_atomic` driven by the broadcast arm
  *(in-memory served-chain watch snapshot — not durable)*.
- `DC-CONS-18` — forge transcript equivalence over real bootstrap state.

**Explicitly NOT strengthened** (durability = N-U): no `DC-WAL-*`,
`DC-STORE-*`, `CN-WAL-*`, `CN-STORE-*`, `CN-ANCHOR-*`, `CN-SEED-*`.
