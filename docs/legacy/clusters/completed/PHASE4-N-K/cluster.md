# Cluster PHASE4-N-K — Orchestrator + `ade_node` binary

> **Status:** Planning artifact (non-normative). Introduces
> `CN-NODE-01` + `DC-NODE-01..04` as enforced. Strengthens
> `T-DET-01`, `CN-CONS-08`, `CN-STORE-07`, `CN-STORE-08`,
> `DC-CONS-21`, `DC-STORE-08`. Appends
> `open_obligation = "snapshot_schema_migration_follow_on_cluster"`
> to `DC-STORE-09`.

## Primary invariant

> The `ade_node` binary composes the BLUE authorities shipped by
> PHASE4-N-A..N-J — `admit_via_block_validity` (CN-CONS-08),
> `materialize_rolled_back_state` (CN-STORE-07),
> `framing::{encode,decode}_snapshot` (CN-STORE-08) — into one
> running node via a GREEN orchestrator core plus RED tokio
> runner. Bootstrap is single-authority; per-peer sessions are
> isolated; the persistent-snapshot writer obeys the N-I cadence
> policy with no parallel override; the orchestrator core depends
> on a `Clock` trait that replays byte-deterministically; clean
> shutdown drains the admit/write/snapshot pipeline; restarting
> against the same `(chaindb, snapshot store)` reproduces a
> byte-identical initial `(LedgerState, PraosChainDepState,
> ChainDb tip)`.

## Scope

- **GREEN (new):**
  - `ade_runtime::bootstrap` — single `pub fn` returning
    `(LedgerState, PraosChainDepState, ChainTip)`. Cold-start
    (genesis-only) and warm-start (snapshot-resume +
    replay-forward) are two branches of the same function.
  - `ade_runtime::clock` — `Clock` trait + `DeterministicClock`
    test impl. Production `SystemClock` lives next to it but is
    RED.
  - `ade_runtime::orchestrator::core` — pure `step` reducer over
    `OrchestratorEvent` → `(state', Vec<OrchestratorEffect>)`.
  - `ade_runtime::rollback::persistent_writer` — pure cadence-
    checker + persistent-cache caller glue.
- **RED (new):**
  - `ade_runtime::orchestrator::tokio_runner` — tokio select loop
    + per-peer task spawner + signal handlers.
  - `ade_runtime::clock::SystemClock` — production wall-clock impl.
  - `ade_node::cli` — CLI flag parsing + config loading.
  - `ade_node::main` — `tokio::main` entry + signal handlers +
    bootstrap wiring + shutdown drain.
- **BLUE:** unchanged. No new BLUE in this cluster.

Out-of-scope items are listed explicitly per slice. The cluster
**does not** introduce new wire formats, new ledger transitions,
new VRF/KES code, snapshot eviction, or schema-migration tooling.

## Grounding (verified at HEAD `1946573`)

- **`ade_ledger::receive::admit_via_block_validity`** — sole admit
  authority (CN-CONS-08). Orchestrator composes; never bypasses.
- **`ade_ledger::rollback::materialize_rolled_back_state`** — sole
  materialize authority (CN-STORE-07). Bootstrap warm-start +
  rollback-handling routes through it.
- **`ade_ledger::snapshot::framing::{encode,decode}_snapshot`** —
  sole snapshot byte authority (CN-STORE-08, DC-STORE-08,
  DC-STORE-09).
- **`ade_runtime::rollback::PersistentSnapshotCache`** — N-J S8
  bridge from framing to `SnapshotStore`. Orchestrator uses this
  for writer + bootstrap warm-start read.
- **`ade_runtime::rollback::should_snapshot_after_block` +
  `SnapshotCadence`** — N-I S4 cadence policy. Persistent writer
  consults exactly this; no parallel cadence.
- **`ade_runtime::receive::{dispatch_chain_sync_inbound,
  dispatch_block_fetch_inbound, PerPeerReceiveState}`** — N-H S4
  pure per-peer receive driver. Reused under the tokio peer-session
  task.
- **`ade_runtime::network::n2n_server::{dispatch_chain_sync_frame,
  dispatch_block_fetch_frame, PerPeerN2nServerState}`** — N-G S6
  pure per-peer server driver. Reused under the tokio
  server-pump task.
- **`ade_runtime::producer::{scheduler_step, assemble_tick,
  signing, broadcast, broadcast_to_served, served_chain_lookups}`** —
  N-C / N-G producer surface. Leadership session drives the
  scheduler against `Clock::tick_stream()`.
- **`ade_runtime::chaindb::{ChainDb, SnapshotStore, PersistentChainDb,
  PersistentChainDbOptions}`** — N-D storage. `bootstrap` opens
  both via the persistent variant.
- **`ade_runtime::consensus::genesis_parser`** — cold-start
  `EraSchedule` derivation from operator-supplied genesis bundle.

## Slice index

| Slice | Scope | TCB |
|-------|-------|-----|
| S1 | `Clock` trait + `DeterministicClock` + `SystemClock` + `ade_runtime::bootstrap` (cold-start + warm-start branches); CN-NODE-01 grep gate | GREEN + RED |
| S2 | `OrchestratorEvent` / `OrchestratorEffect` / `OrchestratorState` / `OrchestratorError` + `orchestrator::core::step` reducer | GREEN |
| S3 | `rollback::persistent_writer` — cadence-fidelity glue to `PersistentSnapshotCache::capture`; DC-NODE-02 | GREEN |
| S4 | RED `orchestrator::tokio_runner::peer_session` — per-peer receive task wrapping `dispatch_*_inbound`; DC-NODE-01 isolation test | RED |
| S5 | RED `orchestrator::tokio_runner::leadership_session` — slot-tick producer pump via `Clock::tick_stream()` | RED |
| S6 | RED `orchestrator::tokio_runner::n2n_server_pump` — listening socket + per-peer server task spawner | RED |
| S7 | `ade_node::{cli,main}` — `tokio::main`, signal handlers, bootstrap wiring, shutdown drain; DC-NODE-04 halt + resume identity | RED |
| S8 | Replay-equivalence harness with `DeterministicClock` + recorded `OrchestratorEvent` corpus under `corpus/orchestrator/`; DC-NODE-03 | GREEN + test |

Dependencies form a strict DAG: S2 depends on S1; S3 depends on S2;
S4–S6 depend on S2/S3; S7 depends on S1–S6; S8 depends on S2/S3.

## Exit criteria (CI-verifiable)

- [ ] **CE-N-K-1 (CN-NODE-01)** — `ci_check_bootstrap_closure.sh`
  asserts exactly one `pub fn` in `crates/ade_runtime/src/bootstrap.rs`
  returning `(LedgerState, PraosChainDepState, ChainTip)` (or a
  newtype wrapper). Forbids HashMap / wall-clock / tokio / rand in
  the bootstrap source.
- [ ] **CE-N-K-2 (DC-NODE-01)** — integration test
  `peer_session_isolation_holds_under_failure` proves a peer
  session that hits `ReceiveDispatchError::ChainSyncDecode` /
  `Receive(...)` halts only that peer's task while the
  orchestrator continues processing slot ticks and other peers.
- [ ] **CE-N-K-3 (DC-NODE-02)** —
  `persistent_writer_calls_capture_only_on_cadence` test +
  `ci_check_persistent_writer_no_parallel_cadence.sh` (greps that
  the only consult of cadence in `ade_runtime` is via
  `should_snapshot_after_block`).
- [ ] **CE-N-K-4 (DC-NODE-03)** — replay harness
  `replay_equivalence_under_deterministic_clock_holds` reads a
  recorded `OrchestratorEvent` corpus from
  `corpus/orchestrator/`, drives the orchestrator core twice
  under `DeterministicClock`, and asserts byte-identical
  `(LedgerFingerprint.combined, PraosChainDepState,
  ChainDb tip)` after both runs +
  `ci_check_clock_seam.sh` greps that
  `ade_runtime::orchestrator::core::*` contains no
  `SystemTime::now()`, `Instant::now()`, `tokio::time::*`, or
  raw `std::time::*` reads.
- [ ] **CE-N-K-5 (DC-NODE-04)** — integration test
  `shutdown_then_resume_produces_byte_identical_state` runs
  `bootstrap → drive events → shutdown_drain → bootstrap` and
  asserts the second bootstrap returns byte-identical state to
  the snapshot captured during shutdown +
  `binary_halts_on_authority_fatal_decode_error` asserts
  `SnapshotDecodeError::UnknownVersion` /
  `FingerprintMismatch` at bootstrap exits non-zero deterministically.

> No human review may substitute for these checks.

## TCB color map (FC/IS partition)

- **BLUE (deterministic, authoritative):** unchanged. No new BLUE.
- **GREEN (deterministic glue, non-authoritative):**
  - `crates/ade_runtime/src/bootstrap.rs`
  - `crates/ade_runtime/src/clock.rs` (trait + `DeterministicClock`
    impl)
  - `crates/ade_runtime/src/orchestrator/core.rs`
  - `crates/ade_runtime/src/orchestrator/event.rs`
  - `crates/ade_runtime/src/orchestrator/mod.rs`
  - `crates/ade_runtime/src/rollback/persistent_writer.rs`
- **RED (nondeterministic shell):**
  - `crates/ade_runtime/src/clock.rs::SystemClock` (sub-classified;
    the production wall-clock impl)
  - `crates/ade_runtime/src/orchestrator/tokio_runner.rs`
  - `crates/ade_runtime/src/orchestrator/peer_session.rs`
  - `crates/ade_runtime/src/orchestrator/leadership_session.rs`
  - `crates/ade_runtime/src/orchestrator/n2n_server_pump.rs`
  - `crates/ade_node/src/cli.rs`
  - `crates/ade_node/src/main.rs`

Rules:
- No RED behavior may appear in any GREEN file (no `tokio::*`, no
  `std::time::SystemTime`, no `Instant::now()`).
- GREEN composes BLUE; never reimplements or shortcuts a BLUE
  authority.
- BLUE remains untouched by this cluster.

## Forbidden during this cluster

- No `SystemTime::now()`, `Instant::now()`, or `tokio::time::*` in
  GREEN files. The orchestrator core consumes time only as
  `Clock` outputs.
- No `HashMap` / `HashSet` / `rand::*` / floating-point in any
  GREEN file.
- No second `pub fn` in `ade_runtime::bootstrap` returning the
  initial `(LedgerState, PraosChainDepState, ChainTip)` triple.
  Cold-start and warm-start are two branches of one function.
- No parallel cadence policy: any decision to capture a
  persistent snapshot MUST go through
  `should_snapshot_after_block`.
- No bypass of `admit_via_block_validity`, `commit_rollback`, or
  `framing::{encode,decode}_snapshot` by the orchestrator.
- No cross-peer state mutation: each peer session owns its own
  `PerPeerReceiveState` / `PerPeerN2nServerState`. The shared
  `LedgerState` view is read-only at the peer task boundary; the
  single mutating consumer is the orchestrator core under the
  single authority chain.
- No silent fallback decode of a snapshot that emitted
  `UnknownVersion` / `FingerprintMismatch`. The binary halts.
- No indefinite blocking shutdown: shutdown bounds itself to
  draining the admit/write/snapshot pipeline. Peer sockets may
  be closed mid-stream.

## Replay obligations introduced by this cluster

- New deterministic surface: the `OrchestratorEvent` stream
  consumed by `orchestrator::core::step`. The replay corpus under
  `corpus/orchestrator/` is the canonical evidence.
- `T-DET-01.strengthened_in += "PHASE4-N-K"` — replay equivalence
  now extends across the orchestrator core under clock injection.
- `CN-CONS-08.strengthened_in += "PHASE4-N-K"` — admit path now
  driven by the production orchestrator.
- `CN-STORE-07.strengthened_in += "PHASE4-N-K"` — materialize
  path driven by the production orchestrator (warm-start branch
  of bootstrap).
- `CN-STORE-08.strengthened_in += "PHASE4-N-K"` — encode/decode
  driven by the production orchestrator (bootstrap, persistent
  writer, shutdown drain).
- `DC-CONS-21.strengthened_in += "PHASE4-N-K"` — round-trip
  equivalence now exercised end-to-end at bootstrap +
  shutdown-resume.
- `DC-STORE-08.strengthened_in += "PHASE4-N-K"` — encoder
  canonicality exercised by persistent writer + shutdown.

## Open obligations carried after closure

- `DC-STORE-09` carries
  `open_obligation = "snapshot_schema_migration_follow_on_cluster"`
  (snapshot v1 → v2 upgrade tooling is a snapshot-format
  lifecycle concern, not a node-binary concern).
- `RO-LIVE-01`, `RO-LIVE-02`, `CN-CONS-06` live-evidence halves
  remain `blocked_until_operator_peer_available` — closing them
  is a separate operator-action cluster running `ade_node`
  against a private cardano-node peer.

## Authority reminder

This document is a planning aid only. All correctness rules live
in the project's normative specifications and the invariant
registry. If there is ever a disagreement:

> **Normative documents + CI enforcement win.**
