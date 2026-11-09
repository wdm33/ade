# PHASE4-N-K — Orchestrator + `ade_node` binary — invariants sketch

## Framing

PHASE4-N-K builds the shell that composes the BLUE/GREEN primitives
shipped through PHASE4-N-A..N-J into a single runnable Cardano
block-producing node binary.

**PHASE4-N-K introduces no BLUE authority. The orchestrator core
is GREEN: deterministic composition of existing BLUE authorities
over canonical event input. The tokio runner, sockets, wall-clock,
CLI, signal handling, and file/config loading are RED.**

That split preserves the clock-injection replay test. A purely
RED orchestrator would be much harder to prove replay-equivalent.

Every authority path (`admit_via_block_validity`,
`materialize_rolled_back_state`,
`framing::{encode,decode}_snapshot`) is reused unchanged. The
novelty is the orchestration semantics — bootstrap, per-peer
sessions, slot-tick driving, persistent-snapshot writer, shutdown
— and the clock-injection seam that lets the orchestrator core
be tested replay-equivalently against a deterministic event
stream.

Predecessor anchors (HEAD `156198e`): PHASE4-N-C (producer),
N-G (server response paths), N-H (receive admit), N-I
(in-memory rollback), N-J (persistent snapshot encoder).

## 1. What must always be true

- **I-1 Authority preservation end-to-end.** Every block
  admission goes through `CN-CONS-08`'s
  `admit_via_block_validity`; every rolled-back-state
  materialization through `CN-STORE-07`'s
  `materialize_rolled_back_state`; every snapshot byte
  encode/decode through `CN-STORE-08`'s
  `framing::{encode,decode}_snapshot`. Orchestrator composes
  the three; never bypasses, never parallels.
- **I-2 Single bootstrap authority.** Exactly one `pub fn` in
  `ade_runtime::bootstrap` returns the initial
  `(LedgerState, PraosChainDepState, ChainDb tip)` at startup.
  Cold-start (genesis-only) and warm-start (snapshot-resume +
  replay-forward) are two branches of the same function —
  never parallel paths. Type-level + CI grep enforcement.
- **I-3 Per-peer session isolation.** One peer's failure (decode
  error, validity reject, rollback-too-deep, protocol violation)
  halts only that peer's session. The orchestrator continues
  serving other peers and producing blocks. No cross-peer state
  sharing (e.g. one peer's pending-header cache mutating
  another's).
- **I-4 Persistent-writer cadence fidelity.** The orchestrator's
  persistent-snapshot writer calls
  `PersistentSnapshotCache::capture` only on the schedule
  emitted by the N-I `SnapshotCadence`. No orchestrator-side
  cadence override; the policy stays single-source.
- **I-5 Clock-injection seam.** The orchestrator depends on a
  `Clock` trait that yields `now()` and `tick_stream()`.
  Production impl is `tokio::time`; test impl is a deterministic
  tick vector. The seam is mandatory: no
  `SystemTime::now()` or `tokio::time::Instant::now()` reachable
  from the orchestrator core.
- **I-6 Authority-fatal halt.** Errors that affect authoritative
  state (chain_write failure on a committed rollback,
  `SnapshotDecodeError::UnknownVersion` during bootstrap,
  `SnapshotDecodeError::FingerprintMismatch` during bootstrap)
  halt the binary deterministically with a non-zero exit code.
  No silent retry, no fallback decode.
- **I-7 Shutdown → restart-resume identity.**
  Ctrl-C / SIGTERM drains in-flight admissions, writes a final
  snapshot via the persistent writer, then exits cleanly.
  Restarting against the same `(chaindb, snapshot store)`
  produces a byte-identical `(LedgerState, PraosChainDepState,
  ChainDb tip)` to the pre-shutdown state.

  **"Drains" defined narrowly:** no partially admitted block
  may remain between validation, ChainDb write, state
  replacement, and snapshot capture. It does NOT mean waiting
  indefinitely for arbitrary peer sessions. Peer sockets may be
  closed mid-stream; the requirement is that no half-committed
  admission persists across the shutdown boundary.

## 2. What must never be possible

- **¬P-1** Orchestrator construction of `AdmittedBlock` (or any
  bypass of `admit_via_block_validity`).
- **¬P-2** Parallel `encode_snapshot` / `decode_snapshot` /
  `encode_ledger_state` / `decode_ledger_state` /
  `encode_chain_dep` / `decode_chain_dep` fn outside their
  sole-authority site (CN-STORE-08 hard).
- **¬P-3** Wall-clock or randomness reads in BLUE. Orchestrator
  owns the clock and delivers slot ticks as canonical inputs.
- **¬P-4** Half-committed rollback. Orchestrator uses
  `commit_rollback`; never per-field state mutation.
- **¬P-5** Silent re-decode of a snapshot that emitted
  `SnapshotDecodeError::UnknownVersion` — version mismatch is
  fatal at bootstrap.
- **¬P-6** Persistent-snapshot eviction inside this cluster
  (deferred; orchestrator may grow snapshot count unboundedly).
- **¬P-7** Production-binary admission writing to ChainDb
  without first running through `receive::apply_event`.
- **¬P-8** Cross-peer state mutation. Each peer session owns
  its own working memory; shared state is read-only (immutable
  config + the single `LedgerState` behind a synchronizing
  primitive that admits via the single authority).
- **¬P-9** Orchestrator-side reimplementation of
  `SnapshotCadence` (no parallel cadence policy).
- **¬P-10** Shutdown that waits indefinitely for a non-quiescent
  peer session before exiting. Shutdown bounds itself to
  draining the admit/write/snapshot pipeline.

## 3. What must remain identical across executions

- Deterministic replay of the orchestrator core (GREEN reducer
  fed the deterministic `Clock` + a recorded
  `OrchestratorEvent` stream) against a frozen snapshot store +
  chaindb produces byte-identical
  `(LedgerState fingerprint, PraosChainDepState, ChainDb tip)`
  across runs.
- Bootstrap byte-determinism: same `(snapshot store, chaindb)`
  produces byte-identical initial state.
- Snapshot-bytes determinism (already enforced by N-J /
  DC-STORE-08): orchestrator writes byte-identical snapshot
  bytes for byte-identical `(ledger, chain_dep)`.
- Shutdown-write determinism: shutdown snapshot bytes are
  byte-identical to a normal cadence-driven snapshot at the
  same `(ledger, chain_dep)`.

## 4. What must be replay-equivalent

- The orchestrator's `OrchestratorEvent` stream — `SlotTick`,
  `PeerMessageIn`, `PeerMessageOut`, `SnapshotCaptureRequested`,
  `RollBackward`, `Shutdown` — when replayed against the
  deterministic clock + the snapshot store + chaindb in their
  starting state, produces byte-identical events and final
  state.
- Boot sequence: replaying boot against the same
  `(snapshot store, chaindb)` produces an identical initial
  state.
- Restart equivalence:
  `(bootstrap → events E → shutdown → bootstrap)` produces the
  same state as `(bootstrap → events E)`.

## 5. State transitions in scope

All GREEN orchestrator-core + RED tokio runner. BLUE unchanged.

```text
bootstrap(genesis_or_none, snapshot_store, chaindb)
  -> Result<(LedgerState, PraosChainDepState, ChainDbTip), BootstrapError>

orchestrator_core::step(state, OrchestratorEvent)
  -> Result<(state', Vec<OrchestratorEffect>), OrchestratorError>
  // composes receive::apply_event / producer scheduler+forge+broadcast /
  // persistent_writer / commit_rollback per event variant

persistent_writer::on_admitted(state, slot)
  -> Result<(), PersistentWriterError>
  // consults the existing SnapshotCadence; calls
  // PersistentSnapshotCache::capture if scheduled

shutdown::drain_and_snapshot(state)
  -> Result<(), ShutdownError>
  // drains the admit/write/snapshot pipeline to a quiescent
  // point (bounded); flushes a final snapshot via the
  // persistent writer
```

The RED tokio runner wraps `orchestrator_core::step` in a
select loop driven by the `Clock` ticks, the peer sockets, and
the shutdown signal.

## 6. TCB color hypothesis

- **GREEN (new):**
  - `ade_runtime::bootstrap` — pure composition of
    `PersistentSnapshotCache::nearest_le` +
    `materialize_rolled_back_state` + genesis cold-start.
  - `ade_runtime::clock` — `Clock` trait + `DeterministicClock`
    test impl.
  - `ade_runtime::orchestrator::core` — pure `step` reducer
    over `OrchestratorEvent`.
  - `ade_runtime::rollback::persistent_writer` — pure
    cadence-checker + persistent-cache caller glue.
- **RED (new):**
  - `ade_runtime::orchestrator::tokio_runner` — tokio select
    loop + per-peer task spawner.
  - `ade_runtime::clock::SystemClock` — production wall-clock
    impl.
  - `ade_node::cli` — CLI flag parsing + config loading.
  - `ade_node::main` — `tokio::main` entry point + signal
    handlers.
- **BLUE:** unchanged. No new BLUE in this cluster.

## 7. Decisions on framing questions

| # | Question | Decision |
|---|----------|----------|
| 1 | Crate hosting | Keep `ade_node` as the thin binary. Put orchestration in `ade_runtime`. `ade_node` does NOT accumulate semantic orchestration logic. |
| 2 | Existing `live_block_*_session` binaries | Keep them. Targeted live-evidence anchors; deleting weakens evidence locality. |
| 3 | Cold-start from genesis | Ship `--genesis-path` cold-start. Do not require a preloaded snapshot. |
| 4 | Close RO-LIVE-01 / RO-LIVE-02 / CN-CONS-06 live obligations? | No. Binary existence is necessary but not sufficient; those need operator-run testnet evidence. |
| 5 | Snapshot eviction | Out of scope. Do NOT hang the eviction `open_obligation` on DC-NODE-02 — eviction is a storage concern, not node cadence fidelity. Either defer without registry obligation or create a future storage rule later. |
| 6 | Schema-migration tool | Out of scope. Attach `open_obligation = "snapshot_schema_migration_follow_on_cluster"` to DC-STORE-09 (snapshot-format lifecycle), NOT to DC-NODE-04 (shutdown semantics). |
| 7 | Network discrimination | `--genesis-path` mandatory; `--network` is metadata only (logging/tag). Do not bake genesis bundles into this cluster. |
| 8 | Mechanical replay test | Recorded `OrchestratorEvent` corpus including slot ticks, peer messages, rollback, snapshot capture, and shutdown. Shipped under `corpus/orchestrator/`. |

## 8. Registry deltas (planned at /cluster-plan)

### New families
- **DC-NODE** — derived constraints on the node binary /
  orchestrator surface (per-peer isolation, clock injection,
  cadence fidelity, halt discipline, shutdown-resume identity).
- **CN-NODE** — single-authority closures on the node binary
  surface (bootstrap).

### New rules (all `*-01` style; status `declared` until slice-close flips them)

- **CN-NODE-01** — Single bootstrap authority.
- **DC-NODE-01** — Per-peer session isolation.
- **DC-NODE-02** — Persistent-writer cadence fidelity.
  **No `open_obligation`** (eviction is not cadence fidelity).
- **DC-NODE-03** — Clock-injection seam + replay equivalence.
- **DC-NODE-04** — Authority-fatal halt + shutdown-resume
  identity. **No `open_obligation`** (schema migration is not
  shutdown semantics).

### Existing-rule updates
- `DC-STORE-09` — append
  `open_obligation = "snapshot_schema_migration_follow_on_cluster"`
  (lifecycle home for the migration-tool follow-on).

### Strengthenings recorded at cluster close
- `T-DET-01.strengthened_in += ["PHASE4-N-K"]` — replay
  equivalence under clock injection.
- `CN-CONS-08.strengthened_in += ["PHASE4-N-K"]` — admit path
  driven by production orchestrator.
- `CN-STORE-07.strengthened_in += ["PHASE4-N-K"]` —
  materialize path driven by production orchestrator
  (bootstrap warm-start).
- `CN-STORE-08.strengthened_in += ["PHASE4-N-K"]` —
  encode/decode driven by production orchestrator (bootstrap +
  persistent writer + shutdown).
- `DC-CONS-21.strengthened_in += ["PHASE4-N-K"]` — round-trip
  equivalence exercised end-to-end at bootstrap.
- `DC-STORE-08.strengthened_in += ["PHASE4-N-K"]` — encoder
  canonicality exercised by persistent writer + shutdown.

### Live-evidence note
RO-LIVE-01 and RO-LIVE-02 remain
`open_obligation = "blocked_until_operator_peer_available"`
after this cluster. Closing them is a separate operator-action
cluster that runs `ade_node` against a private cardano-node
peer and captures the log.
