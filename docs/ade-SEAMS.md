# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, **61 CI checks** at HEAD (`1946573`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml` — **214 entries**) for
> rule IDs; reads the Phase 4 cluster plan
> (`docs/active/phase_4_cluster_plan.md`), the closed N-D / N-A / N-B /
> N-E / N-C / N-G / N-H / N-I / N-J / B1 / B2 / B3 / B4 / B5 cluster
> docs, the OQ5 / COMMITTEE / DREP / ENACTMENT-COMMITTEE-FIDELITY /
> ENACTMENT-COMMITTEE-WRITEBACK / PROPOSAL-PROCEDURES-DECODE cluster
> docs, and the **just-closed PHASE4-N-K cluster doc + CLOSURE
> record** (`docs/clusters/completed/PHASE4-N-K/{cluster,CLOSURE}.md`).
>
> **This is the PHASE4-N-K FULL CLOSE refresh (HEAD `1946573`).** The
> previous SEAMS (HEAD `f15102f`) pinned the PHASE4-N-J full-close
> state and surfaced "orchestrator-side persistent-capture wiring" as
> a Tier-5 candidate. PHASE4-N-K supersedes that candidate by shipping
> the entire orchestrator + node binary: `ade_runtime::bootstrap`,
> `ade_runtime::clock`, `ade_runtime::orchestrator::{event, state,
> core}`, `ade_runtime::rollback::persistent_writer`, the RED
> tokio-runner trio (`peer_session`, `leadership_session`,
> `n2n_server_pump`), and the `ade_node` binary
> (`cli`, `lib`, `node`, `main`).
>
> **THE KEY FULL-CLOSE DELTAS.** PHASE4-N-K introduces **five new
> seams** — all non-BLUE — that bind the existing BLUE authority
> surface into a runnable node:
>
> 1. **Clock seam (DC-NODE-03).** `ade_runtime::clock::Clock` trait at
>    `crates/ade_runtime/src/clock.rs`. Production impl `SystemClock`
>    is the SOLE wall-clock-reading site in `ade_runtime`;
>    test/replay impl `DeterministicClock` drives the replay harness.
>    CI gate `ci/ci_check_clock_seam.sh` greps the orchestrator core
>    forbidding `SystemTime::now()`/`Instant::now()`/`tokio::time::*`.
> 2. **Orchestrator event/effect seam (DC-NODE-01, DC-NODE-03).**
>    `ade_runtime::orchestrator::event::{OrchestratorEvent,
>    OrchestratorEffect, OrchestratorError, PeerHaltReason,
>    AuthorityFatalKind, PeerId, PeerRole}` are closed sum types —
>    trait-less, data-only. The seam IS the closed event vocabulary;
>    new event variants require a code change.
> 3. **Bootstrap seam (CN-NODE-01).**
>    `ade_runtime::bootstrap::bootstrap_initial_state` — single
>    `pub fn` returning `(LedgerState, PraosChainDepState,
>    Option<ChainTip>)`. Cold-start (genesis-only) and warm-start
>    (snapshot-resume + replay-forward) are TWO BRANCHES OF THE SAME
>    FUNCTION, never a parallel entry point. CI gate
>    `ci/ci_check_bootstrap_closure.sh`.
> 4. **Persistent writer cadence seam (DC-NODE-02).**
>    `ade_runtime::rollback::persistent_writer::PersistentSnapshotWriter`
>    (`on_admitted` + `force_capture`) is the SOLE caller of
>    `PersistentSnapshotCache::capture` from cadence-driven decisions;
>    the cadence policy itself remains in
>    `ade_runtime::rollback::cadence::should_snapshot_after_block`
>    (single source). CI gate
>    `ci/ci_check_persistent_writer_no_parallel_cadence.sh`.
> 5. **Authority-fatal exit code seam (DC-NODE-04).**
>    `ade_node::node::{EXIT_AUTHORITY_FATAL_IO = 10,
>    EXIT_AUTHORITY_FATAL_DECODE = 12, EXIT_GENERIC_STARTUP = 1}` is
>    the closed exit-code surface. New fatal kinds slot in via
>    additions to `AuthorityFatalKind` (closed sum) + mapping in
>    `NodeRunError::exit_code`. CI gate
>    `ci/ci_check_node_binary_uses_single_bootstrap.sh`.
>
> **No new BLUE seams. No new registry extension points beyond the
> closed sums above. The cluster does NOT introduce any "plugin"-style
> registry — all orchestrator routing is closed-sum dispatch.** The
> previous SEAMS revision's highest-priority candidate
> ("orchestrator-side persistent-capture wiring") is **subsumed and
> closed** by N-K: `PersistentSnapshotWriter` is the production
> cadence-driven persistent-capture caller; the older
> `maybe_capture_snapshot`/`InMemorySnapshotCache` hook still exists
> in `snapshot_writer.rs` for the receive-side hot path, and the
> persistent path now runs in parallel inside the orchestrator core
> via `PersistentSnapshotWriter::on_admitted`.
>
> Counts at this refresh: **+6 CI scripts** (55 → 61:
> `ci_check_bootstrap_closure.sh`, `ci_check_clock_seam.sh`,
> `ci_check_orchestrator_core_purity.sh`,
> `ci_check_persistent_writer_no_parallel_cadence.sh`,
> `ci_check_peer_session_isolation.sh`,
> `ci_check_node_binary_uses_single_bootstrap.sh`); **+5 registry
> rules** introduced (`CN-NODE-01`, `DC-NODE-01`, `DC-NODE-02`,
> `DC-NODE-03`, `DC-NODE-04` — all `enforced`); **6 carried rules
> strengthened** (`T-DET-01`, `CN-CONS-08`, `CN-STORE-07`,
> `CN-STORE-08`, `DC-CONS-21`, `DC-STORE-08` each gain
> `strengthened_in += PHASE4-N-K`); **1 carried rule gains a new
> open_obligation** (`DC-STORE-09 +=
> "snapshot_schema_migration_follow_on_cluster"`); **+8 new GREEN
> files** (`bootstrap.rs`, `clock.rs`, `orchestrator/{mod, event,
> state, core}.rs`, `rollback/persistent_writer.rs`,
> `ade_node/{cli, lib, node}.rs`); **+3 new RED files**
> (`orchestrator/{peer_session, leadership_session,
> n2n_server_pump}.rs`); **+4 new integration test files / 7 tests**
> (`orchestrator_peer_isolation.rs`,
> `orchestrator_replay_equivalence.rs`,
> `shutdown_resume_identity.rs`, `authority_fatal_decode.rs`);
> **0 new operator-action probe binaries**; **0 new BLUE
> chokepoints**. Total invariant registry: **214 entries** (209 →
> 214). **One new explicit carried-forward open obligation surfaced
> by N-K** — `snapshot_schema_migration_follow_on_cluster` on
> `DC-STORE-09` (snapshot v1→v2 migration tooling; out of node-binary
> scope).

Ade is a Cardano block-producing node. Its closure surface is dominated
by two facts:

1. The Cardano protocol fixes wire bytes and hashes for hash-critical
   paths (Tier 1 — must-conform). New work that touches those bytes
   has essentially no degrees of freedom.
2. Everything operator-facing — storage layout, query API, telemetry,
   packaging — is Tier 5: deliberate divergence "in our own image"
   (per `docs/active/CE-79_tier5_addendum.md`).

This document names where the system opens and where it stays closed.

**PHASE4-N-K is fully closed at this HEAD.** The Ade workspace can
now compose itself into a running node: `bootstrap_initial_state`
opens persistent storage and returns a byte-identical initial
`(LedgerState, PraosChainDepState, Option<ChainTip>)` triple from
either genesis or a persisted snapshot; the pure orchestrator core
`step(state, event)` dispatches `OrchestratorEvent`s to closed-sum
effects with per-peer isolation; the RED tokio runner trio drives
the chain-sync / block-fetch / leadership pumps; `ade_node::main`
ties it all together with signal handling, shutdown drain, and
authority-fatal exit-code mapping. The cluster **does not** add new
BLUE; it composes the BLUE shipped by N-A..N-J.

**PHASE4-N-J remains fully closed** (carried). **PHASE4-N-I remains
fully closed** (carried). **PHASE4-N-H remains fully closed**
(carried). **PHASE4-N-G remains fully closed** (carried).
**PHASE4-N-C remains fully closed** (carried).
**PHASE4-N-E remains fully closed** (carried).
**PROPOSAL-PROCEDURES-DECODE remains fully closed** (carried).
**PHASE4-B3..B5, OQ5 / COMMITTEE / DREP /
ENACTMENT-COMMITTEE-WRITEBACK** all remain closed (carried).

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative
> pipelines. At HEAD there remain **eight** fully-wired *external*
> ingress surfaces (block bytes, Plutus script bytes, snapshot bytes,
> Ouroboros mux frames, genesis JSON bundles, chain-selector stream
> inputs, the N-E wire-level mempool ingress, and the N-H receive-side
> N2N peer ingress). **PHASE4-N-K adds no new external ingress
> surface in the wire-format sense** — its surfaces are
> **process-boundary internal**: the OS signal stream (SIGINT/SIGTERM),
> the CLI argv vector, and the `OrchestratorEvent` stream consumed by
> the pure orchestrator core. The chain-sync / block-fetch mux pumps
> are runner glue that defers to the N-G / N-H BLUE chokepoints
> shipped earlier.
>
> **N-K, however, formalises THE INTERNAL EVENT SEAM** between the RED
> tokio runner and the GREEN orchestrator core. The seam is the closed
> sum type `OrchestratorEvent` (variants include peer-source RX
> events, slot-tick from `Clock`, server-pump events, persistent
> writer notifications, and shutdown signal). The runner translates
> RED-side asynchrony into events; the core reduces events
> deterministically. Replay equivalence under `DeterministicClock`
> is the binding contract (DC-NODE-03).

### Surface: Process-boundary node entry (NEW in PHASE4-N-K — CN-NODE-01 + DC-NODE-04)

```
Surface: argv + ENV + on-disk genesis bundle + on-disk ChainDb +
         on-disk SnapshotStore + OS signal stream
Reduces to: bootstrap_initial_state(...) →
            (LedgerState, PraosChainDepState, Option<ChainTip>)
         followed by run_node_until_shutdown(...) → Result<(), NodeRunError>
         where NodeRunError::exit_code() maps deterministically to
         EXIT_AUTHORITY_FATAL_IO (10) | EXIT_AUTHORITY_FATAL_DECODE (12)
         | EXIT_GENERIC_STARTUP (1)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. RED ade_node::cli — parse argv (clap-style), load config bundle.
  2. RED ade_node::main — install signal handlers (Ctrl-C / SIGTERM),
     init tokio runtime.
  3. GREEN bootstrap_initial_state — single-authority init.
     Cold-start: parse genesis, derive initial LedgerState +
                 PraosChainDepState, ChainTip = None.
     Warm-start: open SnapshotStore, find nearest snapshot via
                 PersistentSnapshotCache, decode bytes via
                 framing::decode_snapshot, ChainTip from ChainDb
                 head. Restart-safety: re-running bootstrap against
                 the same (chaindb, snapshot store) returns a
                 byte-identical triple.
  4. RED tokio_runner::* — spawn per-peer session + leadership
     session + n2n_server_pump tasks; wire each to a shared
     OrchestratorEvent mpsc channel.
  5. GREEN orchestrator::core::step — pure (state, event) →
     (state', Vec<OrchestratorEffect>).
  6. RED runner dispatches OrchestratorEffects (socket writes,
     ChainDb commits, PersistentSnapshotWriter::on_admitted,
     PeerSessionHalted task cancellation).
  7. RED shutdown drain — on signal, drive the admit/write/snapshot
     pipeline to a quiescent state, then force_capture a final
     snapshot, then close peer sockets.
  8. GREEN exit-code mapping — NodeRunError::exit_code maps
     authority-fatal categories to the closed exit-code surface;
     SnapshotDecodeError::UnknownVersion / FingerprintMismatch at
     bootstrap exits non-zero deterministically.
Cross-surface state sharing: per-peer state stays fully independent
  (PeerSessionHalted closed reason discriminant; sibling peers
  + producer continue). Bootstrap and writer share the persistent
  SnapshotStore via PersistentSnapshotCache (single production
  consumer; the in-memory cache is the receive-side hot path
  carried from N-I unchanged).
```

**Rule (NEW in N-K).** The node binary surface has a SINGLE
composition root — `bootstrap_initial_state` for init and
`run_node_until_shutdown` for the drive loop. No parallel entry
point. New runtime features attach by:

- Adding a new `OrchestratorEvent` variant + matching reducer arm
  in `orchestrator::core::step` (closed-sum extension — version-gated).
- Adding a new `OrchestratorEffect` variant + matching runner
  dispatch arm (closed-sum extension).
- Adding a new `PeerHaltReason` / `AuthorityFatalKind` variant for
  newly-discriminated failure modes (closed-sum extension).
- Adding a new `Clock` impl (today: `DeterministicClock`,
  `SystemClock`; new impls remain deliberate registry-tracked
  closed additions — not runtime plug-ins).

— **not** by adding a parallel `pub fn` returning the initial
state triple anywhere outside `bootstrap.rs`, **not** by adding a
second wall-clock-reading site outside `SystemClock`, **not** by
adding a parallel cadence consultation that bypasses
`should_snapshot_after_block`, **not** by emitting authority-fatal
exits with new ad-hoc exit codes outside the closed `EXIT_*`
constants.

### Surface: Receive-side N2N peer ingress (carried from N-H + N-I + N-J; **per-peer isolation now mechanically tested in N-K**)

Carried structurally. **N-K effect on this arm:** per-peer dispatch
is now wrapped by the RED `orchestrator::tokio_runner::peer_session`
task. Decode/validity errors emit `OrchestratorEffect::PeerSessionHalted
{ peer_id, reason: PeerHaltReason }` and remove only that peer's
state from `OrchestratorState::per_peer_{receive,server}`; sibling
peers and the producer continue. The reducer arm is unchanged at
the BLUE level — N-K wraps it, never bypasses it. CI gate
`ci_check_peer_session_isolation.sh` + integration test
`peer_session_isolation_holds_under_failure` defend DC-NODE-01.

### Surfaces carried unchanged from prior revisions

- **Producer-side chain-sync server-role ingress** (N-G): carried.
- **Producer-side block-fetch server-role ingress** (N-G): carried.
- **Forge-block transition** (N-C): carried.
- **Self-accept broadcast gate** (N-C): carried.
- **Scheduler input ingress** (N-C): carried. **N-K note:** now
  driven by the `orchestrator::tokio_runner::leadership_session`
  RED pump via `Clock::tick_stream()`; the scheduler chokepoint
  is unchanged.
- **Mempool ingress** (Tier-1 wire-level — N-E): carried.
- **Conway tx-body `proposal_procedures` sub-grammar** (PP): carried.
- **Single-tx validity** (B2): carried.
- **Mempool admission** (Tier-1 gate — B2): carried.
- **Full block validity** (B1): carried.
- **Persistent ledger snapshot encoding** (N-J): carried. **N-K
  effect:** `PersistentSnapshotWriter` is now the production
  cadence-driven caller of `PersistentSnapshotCache::capture`;
  `force_capture` is called once during shutdown drain.
- **Block bytes, Plutus script bytes, Snapshot bytes (N-D layer
  unchanged), Consensus-input extraction, Ouroboros mux frames,
  Genesis JSON bundles, Chain-selector stream inputs**: all carried.

### Receive-side rollback authority (carried from N-I; persistent reader from N-J; **now driven by warm-start bootstrap branch in N-K**)

The BLUE chokepoint set (`materialize_rolled_back_state`,
`commit_rollback`, `RollbackContext`, `ChainDbWrite::rollback_to_slot`)
is structurally unchanged. **N-K effect:** the warm-start branch of
`bootstrap_initial_state` is now a production caller of
`materialize_rolled_back_state` (via the persistent
`SnapshotReader` impl from N-J); this strengthens `CN-STORE-07`
without changing the chokepoint.

### Candidates — surfaces not yet wired

- **N-K SUBSUMED AND CLOSED the prior revision's
  "orchestrator-side persistent-capture wiring" candidate** —
  `PersistentSnapshotWriter` is the production cadence-driven
  capture caller; `force_capture` runs on shutdown drain.
- **NEW CANDIDATE (flagged by N-K close): snapshot schema migration
  v1 → v2** *(promoted from N-J seam-only flag to a `DC-STORE-09`
  `open_obligation` at N-K close)* — `framing::SCHEMA_VERSION:
  u32 = 1` is the explicit anchor; first new field appended bumps
  to 2. Operator-facing migration tooling (read v1 + emit v2; or
  on-restart auto-upgrade) is the named follow-on cluster's
  deliverable. Out of node-binary scope; tracked on `DC-STORE-09`.
- **NEW CANDIDATE (flagged by N-K close): live Ouroboros mux +
  handshake driver above `ade_network::mux::MuxTransport`.** N-K
  ships honest-scope RED runner files (`peer_session.rs`,
  `leadership_session.rs`, `n2n_server_pump.rs`) plus the
  `ade_node` binary that bootstraps + prints a readiness line; the
  actual chain-sync / block-fetch socket pump above
  `MuxTransport` is the follow-on cluster's deliverable. Tracked
  by `RO-LIVE-01` / `RO-LIVE-02` (live-evidence halves;
  `blocked_until_operator_peer_available`).
- **NEW CANDIDATE (flagged by N-K close): metrics + observability
  surface.** The orchestrator core consumes `OrchestratorEvent`
  and produces `OrchestratorEffect`; a future cluster threads a
  closed `MetricEffect` arm + a RED Prometheus exporter through
  the runner. Tier-5; no BLUE invariants change.
- **CANDIDATE (carried): snapshot eviction policy** — Tier-5
  operational concern; carried unchanged from N-J.
- **CANDIDATE (carried from N-I; now restart-safe via N-J): multi-peer
  fork choice.** Praos longest-chain across competing
  `PerPeerReceiveState[]`; now further enabled by N-K's stable
  per-peer task model.
- **CANDIDATE (carried): N2C local-chain-sync receive surface.** Unchanged.
- **CANDIDATE (carried): pre-Conway snapshot encoder.** Carried; no
  current operational need.
- **CE-N-H-6 live-evidence — still
  `blocked_until_operator_peer_available`** (carried).
- **CE-N-G-8 / CE-N-C-8 live-evidence — still
  `blocked_until_operator_*_available`** (carried).
- **PROPOSAL-PROCEDURES-DECODE remains closed** (carried).
- **PHASE4-N-E remains closed** (carried).

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| **PHASE4-N-K** *(FULLY CLOSED at this HEAD — mechanical close; honest-scope RED runner — the mux/handshake driver above `MuxTransport` is RO-LIVE-01/02)* | **Process-boundary node entry: argv + ENV + genesis + ChainDb + SnapshotStore + signals → running node** | `(LedgerState, PraosChainDepState, Option<ChainTip>)` (bootstrap); `Vec<OrchestratorEffect>` per step (orchestrator core); `i32` exit code (process exit) | **DONE:** `ade_runtime::bootstrap::bootstrap_initial_state` (SOLE init `pub fn` — CN-NODE-01); `ade_runtime::clock::{Clock, DeterministicClock, SystemClock}` (DC-NODE-03 seam); `ade_runtime::orchestrator::{event, state, core, mod}` (closed-sum dispatch); `ade_runtime::rollback::persistent_writer::PersistentSnapshotWriter` (DC-NODE-02 single cadence-driven persistent-capture caller); `ade_node::{cli, lib, node, main}` (DC-NODE-04 closed exit-code surface). 6 CI scripts. 5 registry rules `enforced`. 6 carried rules `strengthened_in += PHASE4-N-K`. | **wired & closed in PHASE4-N-K (mechanical half; the live mux driver above `MuxTransport` is RO-LIVE-01/02 — `blocked_until_operator_peer_available`)** |
| **NEW CANDIDATE — Live Ouroboros mux + handshake driver above `MuxTransport`** *(flagged by N-K close — honest-scope follow-on)* | **Real cardano-node peer drives `ade_node` via N2N socket** | Live `OrchestratorEvent::PeerRx*` events fed by a tokio mux pump bound to `MuxTransport` | Likely a new RED submodule `ade_runtime::network::mux_pump` plus integration with `peer_session`/`n2n_server_pump`. Tracked on `RO-LIVE-01`/`RO-LIVE-02`. | **candidate (operator-action follow-on; `blocked_until_operator_peer_available`)** |
| **NEW CANDIDATE — Snapshot schema migration v1 → v2 tooling** *(NEW `DC-STORE-09` `open_obligation` at N-K close)* | **Operator-facing upgrade of persisted snapshot bytes** | A separate tool/binary that reads v1 bytes via `framing::decode_snapshot` and re-emits v2 bytes (when v2 lands); or auto-upgrade on restart | Tier-5 operator tooling; closed-version-gated. Out of node-binary scope. | **candidate (next-cluster seam; tracked on `DC-STORE-09`)** |
| **NEW CANDIDATE — Metrics + observability surface** *(flagged by N-K close)* | **Prometheus-style counters / gauges / histograms emitted from the running node** | Closed `MetricEffect` arm added to `OrchestratorEffect`; RED exporter mapped from runner | Tier-5 wire / Tier-5 semantics; closed-sum extension on the effect type. | **candidate (next-cluster seam; surface)** |
| **CANDIDATE — Snapshot eviction policy** *(carried from N-J)* | Carried. | Carried. | **candidate (next-cluster seam; surface)** |
| **CANDIDATE — Multi-peer fork choice (Praos longest-chain across competing peers)** *(carried; now further enabled by N-K stable per-peer task model)* | Carried. | Carried. | **candidate (next-cluster seam; surface)** |
| **CANDIDATE — N2C local-chain-sync receive surface** *(carried)* | Carried. | Carried. | **candidate (next-cluster seam; surface)** |
| **CANDIDATE — Pre-Conway snapshot encoder** *(carried from N-J)* | Carried. | Carried. | **candidate (low-priority next-cluster seam; surface)** |
| **CE-N-H-6 (cross-cluster obligation carried)** | **Live N2N follow-mode admission** | Carried. | Carried. | **carried (`blocked_until_operator_peer_available`)** |
| **CE-N-G-8 (cross-cluster obligation carried)** | **Live N2N block-fetch acceptance (Ade serving)** | Carried. | Carried. | **carried (`blocked_until_operator_peer_available`)** |
| **CE-N-C-8 (cross-cluster obligation carried)** | **Live N2N block-fetch acceptance (Ade forging)** | Carried. | Carried. | **carried (`blocked_until_operator_stake_available`)** |
| **N-C+ (declared non-goal in N-C cluster doc; OQ-4 lock)** | **TPraos producer (Shelley..Alonzo full-block production)** | Carried. | Carried. | candidate (declared non-goal) |
| **CE-NODE-N2C-LTX (cross-cluster obligation carried from N-E)** | **Live N2C UDS server + N2N bulk-tx inbound listener** | Carried. | Carried. | **deferred cross-cluster obligation** |
| **PP OQ-1..OQ-4 (separable seams)** | various | Carried. | Carried. | candidate (carried) |
| B+ (full tx UTxO scope) | Full-scope single-tx validity over real resolved UTxO | Carried. | Carried. | candidate |
| B+ (Conway body witness depth) | Conway block-body vkey-witness closure | Carried. | Carried. | candidate (B2-carried) |
| B+ (pre-Conway tx) | Pre-Conway single-tx validity | Carried. | Carried. | candidate |
| B1+ (pre-Babbage block) | TPraos full-block validity (Shelley..Alonzo) | Carried. | Carried. | candidate |
| N-F | LSQ semantic dispatch (LocalStateQuery payloads) | Internal Query enum | Single dispatch fn over opaque-bytes payloads | candidate |
| N-F | LocalTxMonitor semantic dispatch | Mempool-snapshot Query/Reply enums | Single dispatch fn over opaque-bytes payloads | candidate |
| N-B+ | Live cardano-node session driver | `StreamInput` translated from `ChainSyncMessage` + `BlockFetchMessage` | Composition layer in `ade_core_interop` | candidate |

### Operator-action evidence (live-wire artifacts — not BLUE seams)

The Ade workspace closes Tier-1 wire-level seams in two halves: a
mechanical / GREEN half (code + harness + CI gates that the workspace
itself can certify on every push) and a **live-wire operator-action
half** (a real peer / client at the other end of a real socket
producing bytes Ade has never seen).

**At this HEAD two live-evidence logs remain committed**, three
cross-cluster obligations remain `blocked_until_operator_*_available`,
and one cross-cluster obligation is carried from N-E. **N-K added
no new operator-action obligation in the wire-format sense** — the
node binary's external surfaces (genesis bundle, ChainDb, snapshot
store, signals) are operator-controlled but not peer-driven; the
peer-driven live cluster is tracked by `RO-LIVE-01`/`RO-LIVE-02`
(the live mux pump above `MuxTransport`).

| Procedure | Evidence-log artifact | Status at HEAD | What it asserts | TCB |
|-----------|----------------------|----------------|------------------|-----|
| `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_<date>.log` | **CAPTURED** (carried) | Real cardano-node N-B follow-mode tip agreement | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_2026-05-25.log` | **CAPTURED** (carried) | Outbound-client probe against a real preprod N2N relay | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-7_PROCEDURE.md` | (deferred) `CE-NODE-N2C-LTX_<date>.log` | **DEFERRED to CE-NODE-N2C-LTX** | Real `cardano-cli transaction submit` to Ade over the N2C UDS | RED operator action (deferred) |
| `docs/clusters/completed/PHASE4-N-C/CE-N-C-8_PROCEDURE.md` | (pending) `CE-N-C-LIVE_<date>.log` | **`blocked_until_operator_stake_available`** (carried) | Cardano-node accepts an Ade-forged block as the next chain head | RED operator action |
| `docs/clusters/completed/PHASE4-N-G/CE-N-G-8_PROCEDURE.md` | (pending) `CE-N-G-LIVE_<date>.log` | **`blocked_until_operator_peer_available`** (carried) | A real cardano-node peer issuing `RequestRange` accepts Ade-served bytes | RED operator action |
| `docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md` | (pending) `CE-N-H-LIVE_<date>.log` | **`blocked_until_operator_peer_available`** (carried) | Ade follower fed RollForward + BlockDelivered from a real cardano-node peer produces a matching ChainDb tip | RED operator action |

**Operator-action probe binaries (RED — `ade_core_interop::bin::*`).**
At this HEAD there are still **five** such binaries (no N-K addition):

| Binary | Slice | Live-evidence target | Status |
|--------|-------|----------------------|--------|
| `live_consensus_session` (PHASE4-N-B) | N-B | CE-N-B-6 | captured |
| `live_tx_submission_session` (PHASE4-N-E S6) | N-E S6 | CE-N-E-6 | captured |
| `live_block_production_session` (PHASE4-N-C S7) | N-C S7 | CE-N-C-8 | blocked_until_operator_stake_available |
| `live_block_fetch_session` (PHASE4-N-G S7) | N-G S7 | CE-N-G-8 | blocked_until_operator_peer_available |
| `live_block_follow_session` (PHASE4-N-H S6) | N-H S6 | CE-N-H-6 | blocked_until_operator_peer_available |

**Pattern carried.** Hermetic default + `--connect <peer>` live pass.
**N-K has no new entry in this family** — the `ade_node` binary itself
is not a probe binary; it is the production node entry. The
peer-driven follow-on (live mux pump) is tracked on
`RO-LIVE-01`/`RO-LIVE-02`.

**These are evidence-log patterns, not BLUE seams.**

User confirmation needed for each candidate at cluster entry. **The
most load-bearing remaining candidates for the bounty** are now
**RO-LIVE-01/02** (the live mux pump above `MuxTransport` — the
next post-N-K cluster to write), **CE-N-C-8** (live cardano-node
forge acceptance), **CE-N-G-8** (live cardano-node block-fetch
acceptance), **CE-N-H-6** (live cardano-node follow-mode admission),
**multi-peer fork choice** (now further enabled by N-K stable
per-peer tasks), **CE-NODE-N2C-LTX**, and the four
**PROPOSAL-PROCEDURES-DECODE open obligations**.

---

## 2. Data-Only vs. Authoritative Layers

Ade has **twenty** authoritative domains. **PHASE4-N-K added one
new compositional domain — node orchestration authority** — at the
GREEN+RED level. No new BLUE chokepoint is introduced; the
orchestrator composes the BLUE shipped by N-A..N-J. Prior cluster
narratives are preserved unchanged below.

### Node orchestration authority (NEW in PHASE4-N-K)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **GREEN bootstrap chokepoint** | `ade_runtime::bootstrap::bootstrap_initial_state` | GREEN | The **SOLE `pub fn` returning the initial `(LedgerState, PraosChainDepState, Option<ChainTip>)` triple** (CN-NODE-01 — CI-defended). Cold-start (genesis-only) and warm-start (snapshot-resume via `PersistentSnapshotCache::nearest_le` → `materialize_rolled_back_state`) are TWO BRANCHES OF THE SAME FUNCTION. Restart-safety contract: re-running against the same `(chaindb, snapshot store)` produces a byte-identical triple. No `HashMap`/wall-clock/`tokio`/`rand` in this file. |
| **GREEN `Clock` trait + `DeterministicClock` impl** | `ade_runtime::clock::{Clock, DeterministicClock}` | GREEN | The `Clock` seam (DC-NODE-03). Orchestrator core consumes time **only** via `Clock` outputs — never `SystemTime::now()` / `Instant::now()` / `tokio::time::*` directly. `DeterministicClock` drives the replay-equivalence harness; given the same tick vector, two runs produce byte-identical orchestrator outcomes. |
| **RED `SystemClock` impl** | `ade_runtime::clock::SystemClock` | RED (sub-classified in a GREEN file) | The **SOLE wall-clock-reading site in `ade_runtime`** (DC-NODE-03 — CI-defended). Production-only; reads `SystemTime::now()` once per tick. `ci_check_clock_seam.sh` greps for any other `SystemTime`/`Instant`/`tokio::time` site in `ade_runtime` and fails. |
| **GREEN closed orchestrator event vocabulary** | `ade_runtime::orchestrator::event::{OrchestratorEvent, OrchestratorEffect, OrchestratorError, PeerHaltReason, AuthorityFatalKind, PeerId, PeerRole}` | GREEN | Closed sum types. Trait-less; data-only. The seam IS the closed vocabulary; new event variants require a code change (no plug-in registry). `PeerHaltReason` is a closed reason-tag (no `String`) — every peer-fatal failure maps to a discriminant. `AuthorityFatalKind` is a closed sum naming exactly the categories that halt the binary (`ChainWriteIo`, `SnapshotDecodeUnknownVersion`, `SnapshotDecodeFingerprintMismatch`, ...). |
| **GREEN `OrchestratorState` + per-peer collections** | `ade_runtime::orchestrator::state::{OrchestratorState, PerPeerReceiveVersions}` | GREEN | Per-peer state in BTreeMaps keyed by `PeerId` (`per_peer_receive`, `per_peer_server`). Each peer's state is isolated; decode/validity errors emit `OrchestratorEffect::PeerSessionHalted { peer_id, reason }` and remove only that peer's entry. |
| **GREEN pure reducer** | `ade_runtime::orchestrator::core::step` | GREEN | Pure `(state, event) → (state', Vec<OrchestratorEffect>)`. No `tokio`, no clock reads, no I/O. Replay-equivalent under `DeterministicClock`. |
| **GREEN persistent writer cadence glue** | `ade_runtime::rollback::persistent_writer::PersistentSnapshotWriter` | GREEN | The **SOLE production caller of `PersistentSnapshotCache::capture` from cadence-driven decisions** (DC-NODE-02 — CI-defended). `on_admitted(slot, &ledger, &chain_dep)` consults `should_snapshot_after_block` exclusively; `force_capture(slot, &ledger, &chain_dep)` runs during shutdown drain. No parallel cadence policy. |
| **RED tokio runner — per-peer task** | `ade_runtime::orchestrator::peer_session` | RED | Per-peer tokio task wrapping `dispatch_chain_sync_inbound` / `dispatch_block_fetch_inbound` (N-H S4). One task per peer; failure halts that task only. Structural mirror of DC-NODE-01. |
| **RED tokio runner — leadership session** | `ade_runtime::orchestrator::leadership_session` | RED | Slot-tick pump driven by `Clock::tick_stream()`. Feeds `OrchestratorEvent::SlotTick` events into the orchestrator core. |
| **RED tokio runner — N2N server pump** | `ade_runtime::orchestrator::n2n_server_pump` | RED | Listening-socket per-connection spawner. Spawns a per-connection tokio task that wraps `dispatch_chain_sync_frame` / `dispatch_block_fetch_frame` (N-G S6). |
| **RED node binary** | `ade_node::{cli, lib, node, main}` | RED | `cli` parses argv; `node::run_node_until_shutdown` is the SOLE main loop (CN-NODE-01 + DC-NODE-04 — CI-defended); `main` installs signal handlers and drives `run_node_until_shutdown`. Authority-fatal exit codes: `EXIT_AUTHORITY_FATAL_IO = 10`, `EXIT_AUTHORITY_FATAL_DECODE = 12`, `EXIT_GENERIC_STARTUP = 1` (closed surface). `NodeRunError::exit_code` maps each error category deterministically. Shutdown drain force-captures a final snapshot via `PersistentSnapshotWriter::force_capture`. |
| **CI gates (6 new)** | `ci/ci_check_{bootstrap_closure,clock_seam,orchestrator_core_purity,persistent_writer_no_parallel_cadence,peer_session_isolation,node_binary_uses_single_bootstrap}.sh` | CI | Mechanical defence of the orchestration authority surface. (1) `bootstrap_closure` — exactly one `pub fn` returning the initial triple in `bootstrap.rs`; forbids `HashMap`/wall-clock/`tokio`/`rand` in that file. (2) `clock_seam` — no `SystemTime`/`Instant`/`tokio::time` in orchestrator core; `SystemClock` is the sole wall-clock site. (3) `orchestrator_core_purity` — no `tokio::*` in GREEN orchestrator files. (4) `persistent_writer_no_parallel_cadence` — the only cadence consult in `ade_runtime` is via `should_snapshot_after_block`. (5) `peer_session_isolation` — per-peer halt routes to `PeerSessionHalted` (closed reason discriminant), not a panic. (6) `node_binary_uses_single_bootstrap` — `ade_node` calls `bootstrap_initial_state`, not a parallel init. Total CI count: 55 → 61. |

**Rule.** This domain has:
- **One GREEN bootstrap chokepoint** (`bootstrap_initial_state` —
  CN-NODE-01 single-authority).
- **One Clock seam** (`Clock` trait + `DeterministicClock` GREEN
  impl + `SystemClock` RED-sub-classified impl — DC-NODE-03).
- **One closed orchestrator event/effect vocabulary** (closed sums —
  trait-less, data-only).
- **One pure reducer** (`orchestrator::core::step`).
- **One persistent-writer cadence chokepoint**
  (`PersistentSnapshotWriter` — DC-NODE-02 single-authority).
- **Three RED tokio runner files** (`peer_session`,
  `leadership_session`, `n2n_server_pump`).
- **One RED node binary** with a closed exit-code surface
  (DC-NODE-04).
- **Six CI gates** defending the above.

**THE KEY SEAMS:**

1. **`bootstrap_initial_state` is the SOLE `pub fn` returning the
   initial `(LedgerState, PraosChainDepState, Option<ChainTip>)`
   triple** (CN-NODE-01). CI-defended via repo-wide grep. Cold-start
   and warm-start are two branches of one function.
2. **The `Clock` trait is the SOLE time seam in `ade_runtime`**
   (DC-NODE-03). All time-dependent code (orchestrator core,
   leadership session, persistent writer cadence) consumes `Clock`
   outputs; only `SystemClock` reads the wall clock. CI-defended.
3. **`OrchestratorEvent` / `OrchestratorEffect` is a closed sum
   pair** (no extension via trait, no plug-in registry). New
   variants are closed-sum extensions requiring a code change.
4. **Per-peer state is isolated by `PeerId` in BTreeMaps**
   (DC-NODE-01). Decode/validity errors map to closed
   `PeerHaltReason` discriminants and remove only the halted peer.
   Sibling peers + producer continue. CI-defended.
5. **`PersistentSnapshotWriter` is the SOLE cadence-driven caller of
   `PersistentSnapshotCache::capture`** (DC-NODE-02). Cadence
   policy itself remains in `should_snapshot_after_block` (single
   source). CI-defended.
6. **Authority-fatal exit codes are closed**: `EXIT_AUTHORITY_FATAL_IO
   = 10`, `EXIT_AUTHORITY_FATAL_DECODE = 12`,
   `EXIT_GENERIC_STARTUP = 1`. `AuthorityFatalKind` is the closed
   sum of fatal categories; `NodeRunError::exit_code` is the closed
   mapping. New fatal kinds slot in via additions to the sum (no
   ad-hoc exit codes).
7. **Replay-equivalence holds under `DeterministicClock`**
   (DC-NODE-03). Given a recorded `OrchestratorEvent` corpus, two
   replays produce byte-identical
   `(LedgerFingerprint.combined, PraosChainDepState, ChainDb tip)`.
8. **Shutdown-then-resume is byte-identical** (DC-NODE-04). After
   `shutdown_drain` force-captures a final snapshot, the next
   `bootstrap_initial_state` against the same `(chaindb, snapshot
   store)` returns the same triple.

**New work** that adds a node-orchestration feature attaches by:
- Adding an `OrchestratorEvent`/`OrchestratorEffect` variant +
  matching reducer arm (closed-sum extension).
- Adding a `PeerHaltReason`/`AuthorityFatalKind` discriminant
  (closed-sum extension; if a new `AuthorityFatalKind` warrants a
  new exit code, append a new `EXIT_*` constant and extend
  `NodeRunError::exit_code`).
- Adding a new `Clock` impl (deliberate registry-tracked closed
  addition — not a runtime plug-in).
- Adding a new RED runner file under `ade_runtime::orchestrator::*`
  (e.g. a future `mux_pump.rs` for the live mux driver — see
  candidate above).

— **not** by adding a parallel `pub fn` returning the initial state
triple outside `bootstrap.rs`, **not** by adding a second wall-clock
site outside `SystemClock`, **not** by adding a parallel cadence
consultation that bypasses `should_snapshot_after_block`, **not** by
bypassing `admit_via_block_validity` / `commit_rollback` /
`framing::{encode,decode}_snapshot` in the orchestrator, **not** by
emitting authority-fatal exits with new ad-hoc exit codes, **not** by
mutating cross-peer state from a peer-session task.

**Declared non-goals carried from the cluster doc:**
- Snapshot schema migration v1 → v2 tooling — tracked on
  `DC-STORE-09.open_obligation =
  "snapshot_schema_migration_follow_on_cluster"`.
- Live Ouroboros mux + handshake driver above `MuxTransport` —
  honest-scope follow-on (`RO-LIVE-01`/`RO-LIVE-02`).
- Snapshot eviction policy — Tier-5 follow-on, carried from N-J.
- Metrics/observability surface — Tier-5 follow-on, flagged.

### Persistent ledger snapshot encoding authority (carried unchanged from PHASE4-N-J)

Carried. **N-K usage:** `bootstrap_initial_state` warm-start branch
is now a production caller of `framing::decode_snapshot` (via
`PersistentSnapshotCache::nearest_le`); `PersistentSnapshotWriter`
is a production caller of `framing::encode_snapshot` (via
`PersistentSnapshotCache::capture`). `CN-STORE-08`,
`DC-STORE-08`, `DC-CONS-21` each gain
`strengthened_in += PHASE4-N-K`.

### Receive-side rollback authority (carried unchanged from PHASE4-N-I; N-J persistent reader carried; **bootstrap warm-start branch now drives it**)

Carried. **N-K note:** `materialize_rolled_back_state` is now a
production caller from the bootstrap warm-start branch (in addition
to the existing receive-side caller). `CN-STORE-07` gains
`strengthened_in += PHASE4-N-K`.

### Receive-side admission authority (carried unchanged from PHASE4-N-H)

Carried. **N-K note:** the admit path is now driven end-to-end by
the production orchestrator core via `OrchestratorEvent::PeerRx*`
→ `step` → `OrchestratorEffect::Admit*`. `CN-CONS-08` gains
`strengthened_in += PHASE4-N-K`.

### Producer-side server response authority (carried unchanged from N-G)

Carried.

### Block production authority (carried unchanged from N-C)

Carried. **N-K note:** the producer scheduler is now driven by
the `orchestrator::tokio_runner::leadership_session` RED pump
via `Clock::tick_stream()`. Scheduler chokepoint unchanged.

### Mempool ingress (carried unchanged from N-E)

Carried.

### Conway tx-body `proposal_procedures` sub-grammar authority (carried unchanged from PROPOSAL-PROCEDURES-DECODE)

Carried.

### Conway value-conservation accounting / Conway certificate-state accumulation / Credential discriminant fidelity / Conway governance-cert accumulation / Single-tx validity / Mempool admission / Full block validity / Ledger application / Stake-snapshot projection for consensus / Plutus phase-2 evaluation / Governance ratification & enactment / Mini-protocol wire conformance / Praos consensus runtime

All carried unchanged from the prior revision. **N-K-specific
strengthening:** `T-DET-01` (replay equivalence) now extends across
the orchestrator core under clock injection.

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on
  RED. N-K added new edges within `ade_runtime` (a RED crate with
  per-file color carve-outs): the GREEN orchestrator-core /
  bootstrap / clock / persistent-writer files import BLUE
  (`ade_ledger::*`, `ade_core::*`, `ade_codec::*`) but never
  RED siblings; the RED runner files import the GREEN orchestrator
  surface but never bypass it. Same direction (RED/GREEN → BLUE)
  as existing N-C / N-G / N-H / N-I / N-J edges; allowed.
- `ci_check_no_async_in_blue.sh` — async forbidden in BLUE. N-K
  added no new BLUE; the orchestrator core + bootstrap are GREEN
  and explicitly non-async (no `tokio::*`, no `async fn`).
- **`ci_check_bootstrap_closure.sh`** *(N-K — CN-NODE-01 enforcement)* —
  asserts exactly one `pub fn bootstrap_initial_state` in
  `crates/ade_runtime/src/bootstrap.rs` returning the initial
  triple (or a newtype wrapper). Forbids
  `HashMap` / wall-clock / `tokio` / `rand` in the bootstrap source.
- **`ci_check_clock_seam.sh`** *(N-K — DC-NODE-03 enforcement)* —
  greps that `ade_runtime::orchestrator::core::*` contains no
  `SystemTime::now()`, `Instant::now()`, `tokio::time::*`, or raw
  `std::time::*` reads; `SystemClock` is the SOLE wall-clock site.
- **`ci_check_orchestrator_core_purity.sh`** *(N-K — DC-NODE-03 + general purity)* —
  greps that GREEN orchestrator files (`core.rs`, `event.rs`,
  `state.rs`, `mod.rs`) contain no `tokio::*` imports.
- **`ci_check_persistent_writer_no_parallel_cadence.sh`** *(N-K — DC-NODE-02)* —
  greps that the only consult of cadence in `ade_runtime` is via
  `should_snapshot_after_block`.
- **`ci_check_peer_session_isolation.sh`** *(N-K — DC-NODE-01)* —
  greps that per-peer halts route to
  `OrchestratorEffect::PeerSessionHalted` (closed reason
  discriminant); no `panic!` / `unwrap` in peer-session paths.
- **`ci_check_node_binary_uses_single_bootstrap.sh`** *(N-K — CN-NODE-01 + DC-NODE-04)* —
  greps that `ade_node` calls `bootstrap_initial_state` (single
  init authority) and maps fatal errors via
  `NodeRunError::exit_code` to the closed exit-code surface.
- *N-J carried CI gates:* `ci_check_snapshot_encoder_closure.sh`.
- *N-I carried CI gates:* `ci_check_rollback_materialize_closure.sh`,
  `ci_check_snapshot_cadence_purity.sh`.
- *N-H carried CI gates:* `ci_check_admitted_block_closure.sh`,
  `ci_check_receive_reducer_closure.sh`,
  `ci_check_receive_replay_purity.sh`,
  `ci_check_receive_orchestrator_no_producer_dep.sh`,
  `ci_check_receive_paths_corpus_present.sh`.
- *N-G carried CI gates:* `ci_check_no_parallel_header_splitter.sh`,
  `ci_check_served_chain_closure.sh`,
  `ci_check_chain_sync_server_closure.sh`,
  `ci_check_block_fetch_server_closure.sh`,
  `ci_check_broadcast_to_served_purity.sh`,
  `ci_check_n2n_server_no_signing_dep.sh`,
  `ci_check_server_paths_corpus_present.sh`.
- *N-C carried CI gates:* `ci_check_private_key_custody.sh`,
  `ci_check_opcert_closed.sh`, `ci_check_forge_purity.sh`,
  `ci_check_no_producer_body_encoder.sh`,
  `ci_check_self_accept_gate.sh`, `ci_check_scheduler_closure.sh`,
  `ci_check_producer_corpus_present.sh`.
- `ci_check_constitution_coverage.sh` — carried.
- `ci_check_proposal_procedures_closed.sh` *(PP — DC-LEDGER-11)* — carried.
- `ci_check_mempool_ingress_closure.sh` /
  `ci_check_mempool_ingress_replay.sh` *(N-E)* — carried.
- `ci_check_credential_discriminant_closed.sh` *(OQ5 / COMMITTEE /
  DREP / ENACTMENT)* — carried.
- `ci_check_gov_cert_accumulation_closed.sh` *(B5)* — carried.
- `ci_check_deposit_param_authority.sh` *(B3)* — carried.
- `ci_check_conway_cert_classification_closed.sh` *(B3F)* — carried.
- `ci_check_no_chaindb_in_consensus_blue.sh` /
  `ci_check_no_float_in_consensus.sh` /
  `ci_check_no_density_in_fork_choice.sh` /
  `ci_check_consensus_closed_enums.sh` — carried.
- `ci_check_pallas_quarantine.sh`, `ci_check_no_signing_in_blue.sh`,
  `ci_check_ingress_chokepoints.sh`, `ci_check_ce_n_a_5_proof.sh` —
  carried.

---

## 3. Closed vs. Extensible Registries

Ade's authority surface is **almost entirely closed.** **PHASE4-N-K
added seven closed surfaces** at the orchestration layer — the
`Clock` trait (with closed impl set: `DeterministicClock`,
`SystemClock`); the closed `OrchestratorEvent` /
`OrchestratorEffect` / `OrchestratorError` / `PeerHaltReason` /
`AuthorityFatalKind` sum quintet (with closed supporting types
`PeerId`, `PeerRole`); the `OrchestratorState` struct (closed field
set, BTreeMap-keyed per-peer collections); the `bootstrap_initial_state`
chokepoint (SOLE init `pub fn`); the `PersistentSnapshotWriter`
chokepoint (SOLE cadence-driven persistent-capture caller); the
`NodeRunError` closed sum + the closed `EXIT_*` constant trio; and
the closed per-file TCB color carve-out for `ade_runtime` (GREEN
orchestrator files explicitly forbid `tokio::*`). Plus **six new
CI gates** (CI count 55 → 61) and **five newly-introduced + six
strengthened + one new open_obligation** registry rules (registry
total 209 → 214).

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `CardanoEra` | `ade_types::era` | 8 variants | New variant = new hard fork. |
| `Certificate` | `ade_types::shelley::cert` | 7 variants | Shelley-era frozen. |
| `StakeCredential` *(OQ5)* | `ade_types::shelley::cert` | 2 variants | DC-LEDGER-10. |
| Credential-decode chokepoints *(OQ5 + PP)* | `ade_codec::{shelley,conway}::cert::decode_stake_credential` + `ade_codec::conway::governance::decode_stake_credential` | 3 functions | Closed 2-variant mapping. |
| `ConwayCert` *(B3/B4)* | `ade_types::conway::cert` | 19 variants | DC-LEDGER-08. |
| `GovAction` *(PP/ENACTMENT)* | `ade_types::conway::governance` | 7 variants | DC-LEDGER-11. |
| `ProposalProcedure` *(PP)* | `ade_types::conway::governance` | closed 4-field struct | DC-LEDGER-11. |
| `decode_proposal_procedures` / `encode_proposal_procedures` *(PP)* | `ade_codec::conway::governance` | 2 functions | DC-LEDGER-11. |
| `MIRPot` | `ade_types::shelley::cert` | 2 variants | Frozen. |
| `DRep` | `ade_types::conway::cert` | 4 variants | CIP-1694 fixed. |
| `CertDisposition` / `DepositEffect` / `CoinSource` *(B3)* | `ade_types::conway::cert` | 3 / 2 / 3 variants | Closed. |
| `ConwayCertAction` *(B4)* | `ade_ledger::delegation` | closed | No `Neutral`. |
| `GovernanceCertEffect` / `OwnerTaggedEffect` / etc. *(B4)* | `ade_ledger::delegation` | closed | B4 plumbing. |
| `GovCertEnv` *(B5)* | `ade_ledger::state` | closed struct | Fail-fast. |
| `apply_conway_gov_cert` dispatch *(B5)* | `ade_ledger::gov_cert` | 1 function | DC-LEDGER-09. |
| `apply_committee_enactment` *(ENACTMENT)* | `ade_ledger::governance` | 1 pure transition | Closed. |
| `IngressSource` *(N-E)* | `ade_ledger::mempool::ingress` | 2 variants | Closed source discriminant. |
| `IngressEvent` *(N-E)* | `ade_ledger::mempool::ingress` | closed struct | Closed flat-data envelope. |
| `mempool_ingress` chokepoint *(N-E)* | `ade_ledger::mempool::ingress` | 1 function | DC-MEM-03. |
| `ProducerTick` *(N-C-S3)* | `ade_ledger::producer::state` | closed 14-field struct | Carried. |
| `forge_block` chokepoint *(N-C-S3)* | `ade_ledger::producer::forge` | 1 function | Carried. |
| `ForgeError` / `ForgeEffects` / `ForgedBlock` *(N-C-S3)* | `ade_ledger::producer::forge` | 7 / 1 / closed struct | Carried. |
| `encode_opcert` / `decode_opcert` chokepoint pair *(N-C-S2)* | `ade_codec::shelley::opcert` | 2 functions | Carried. |
| `OpCertCodecError` *(N-C-S2)* | `ade_codec::shelley::opcert` | 7 variants | Carried. |
| `opcert_validate` chokepoint *(N-C-S2)* | `ade_core::consensus::opcert_validate` | 1 function | Carried. |
| `OpCertError` *(N-C-S2)* | `ade_core::consensus::opcert_validate` | closed | Carried. |
| `block_body_hash_from_buckets` chokepoint *(N-C-S4)* | `ade_ledger::block_body_hash` | 1 function | Carried. |
| `AcceptedBlock` token *(N-C-S5)* | `ade_ledger::producer::self_accept` | 1 newtype | Carried. |
| `self_accept` chokepoint *(N-C-S5)* | `ade_ledger::producer::self_accept` | 1 function | Carried. |
| `SelfAcceptError` *(N-C-S5)* | `ade_ledger::producer::self_accept` | 1 variant | Carried. |
| `SchedulerInput` / `SchedulerEffect` / `SchedulerHaltReason` / `SchedulerState` *(N-C-S6)* | `ade_runtime::producer::scheduler` | closed sums | Carried. **N-K note:** scheduler is now driven by `leadership_session` RED pump via `Clock::tick_stream()`; types unchanged. |
| `TickInputs` / `TickAssemblyError` / `assemble_tick` *(N-C-S6)* | `ade_runtime::producer::tick_assembler` | closed | Carried. |
| `BroadcastError` *(N-C-S6)* | `ade_runtime::producer::broadcast` | 2 variants | Carried. |
| RED signing primitives + key types *(N-C-S1)* | `ade_runtime::producer::signing::*` | closed | Carried. |
| RED key loader *(N-C-S1)* | `ade_runtime::producer::keys` | closed | Carried. |
| `accepted_block_header_bytes` accessor *(N-G-S1)* | `ade_ledger::block_validity::header_input` | 1 function | Carried. |
| `ServerReply` (chain-sync + block-fetch) *(N-G-S1)* | `ade_network::{chain_sync, block_fetch}::server` | 2 closed wrappers | Carried. |
| `HeaderProjection` *(N-G-S3)* | `ade_network::chain_sync::server` | closed struct | Carried. |
| `ServedHeaderLookup` / `ServedRangeLookup` traits *(N-G-S3/S4)* | `ade_network::{chain_sync, block_fetch}::server` | 2 closed traits | Carried. |
| `producer_chain_sync_serve` / `producer_chain_sync_advance_tip` *(N-G-S3)* | `ade_network::chain_sync::server` | 2 functions | Carried. |
| `producer_block_fetch_serve` *(N-G-S4)* | `ade_network::block_fetch::server` | 1 function | Carried. |
| `Producer*ServerState` / `ProducerServerError` / `ServerStep` / etc. *(N-G-S3/S4)* | `ade_network::{chain_sync, block_fetch}::server` | closed | Carried. |
| `ServedChainSnapshot` / `served_chain_admit` *(N-G-S2)* | `ade_ledger::producer::served_chain` | closed | Carried. |
| `PerPeerN2nServerState` / `DispatchError` *(N-G-S6)* | `ade_runtime::network::n2n_server` | closed | Carried. **N-K note:** consumed by `n2n_server_pump` RED runner. |
| `AdmittedBlock` token *(N-H-S1)* | `ade_ledger::receive::admitted` | closed struct | Carried. |
| `AdmittedOutcome` *(N-H-S1)* | `ade_ledger::receive::admitted` | closed struct | Carried. |
| `admit_via_block_validity` chokepoint *(N-H-S1)* | `ade_ledger::receive::admitted` | 1 function | Carried. **N-K note:** now driven end-to-end by the production orchestrator core; `CN-CONS-08.strengthened_in += PHASE4-N-K`. |
| `ReceiveEvent` *(N-H-S1)* | `ade_ledger::receive::events` | 3 variants | Carried. |
| `ReceiveEffect` *(N-H-S1)* | `ade_ledger::receive::events` | 4 variants | Carried. |
| `NoOpReason` *(N-H-S1)* | `ade_ledger::receive::events` | 1 variant | Carried. |
| `ReceiveError` *(N-H-S1)* | `ade_ledger::receive::events` | 4 variants | Carried. |
| `TargetPoint` / `TipPoint` *(N-H-S1 — receive)* | `ade_ledger::receive::events` | 2 closed structs | Carried. |
| `PendingHeaderCache` *(N-H-S1)* | `ade_ledger::receive::pending_header_cache` | closed struct | Carried. |
| `PendingHeaderCacheError` *(N-H-S1)* | `ade_ledger::receive::pending_header_cache` | 1 variant | Carried. |
| `ChainDbWrite` trait *(N-H-S1; N-I-S3 extended)* | `ade_ledger::receive::chain_write` | 2 methods | Carried. |
| `ChainWriteError` / `ChainWriteErrorKind` *(N-H-S1)* | `ade_ledger::receive::chain_write` | 2 / 3 variants | Carried. |
| `ReceiveState` *(N-H-S2)* | `ade_ledger::receive::reducer` | closed struct | Carried. |
| `receive_apply` chokepoint *(N-H-S2; N-I-S6 extended)* | `ade_ledger::receive::reducer` | 1 function | Carried. |
| `receive_apply_sequence` driver *(N-H-S2)* | `ade_ledger::receive::reducer` | 1 function | Carried. |
| `PerPeerReceiveState` *(N-H-S4)* | `ade_runtime::receive::orchestrator` | closed RED struct | Carried. **N-K note:** held by `OrchestratorState::per_peer_receive` (BTreeMap keyed by `PeerId`); per-peer isolation per DC-NODE-01. |
| `ReceiveDispatchError` *(N-H-S4)* | `ade_runtime::receive::orchestrator` | 3 variants | Carried. |
| `SnapshotReader` trait *(N-I-S1; N-J extended)* | `ade_ledger::rollback::traits` | 1 trait with 1 method | Carried. Two production impls (`InMemorySnapshotCache`, `PersistentSnapshotCache`); N-K adds no third. |
| `BlockSource` trait *(N-I-S1)* | `ade_ledger::rollback::traits` | 1 trait with 1 method | Carried. |
| `MaterializeError` *(N-I-S1)* | `ade_ledger::rollback::error` | 3 variants | Carried. |
| `CommitRollbackError` *(N-I-S1)* | `ade_ledger::rollback::error` | 1 variant | Carried. |
| `TargetPoint` *(N-I-S2 — rollback flavor)* | `ade_ledger::rollback::materialize` | closed struct | Carried. |
| `materialize_rolled_back_state` chokepoint *(N-I-S2 — CN-STORE-07)* | `ade_ledger::rollback::materialize` | 1 function | Carried. **N-K note:** now a production caller from the bootstrap warm-start branch; `CN-STORE-07.strengthened_in += PHASE4-N-K`. |
| `commit_rollback` chokepoint *(N-I-S3)* | `ade_ledger::rollback::commit` | 1 function | Carried. |
| `ChainDbWrite::rollback_to_slot` trait method *(N-I-S3)* | `ade_ledger::receive::chain_write` | 1 method | Carried. |
| `RollbackContext<'a>` *(N-I-S6)* | `ade_ledger::receive::reducer` | closed BLUE struct | Carried. |
| `SnapshotCadence` *(N-I-S4 — DC-STORE-07)* | `ade_runtime::rollback::cadence` | closed BLUE-structural struct (exactly 1 field) | Carried. **N-K note:** consulted by `PersistentSnapshotWriter::on_admitted` via `should_snapshot_after_block` (DC-NODE-02 single-source). |
| `SnapshotEncodeError` *(N-J-S1)* | `ade_ledger::snapshot::error` | 1 variant | Carried. |
| `SnapshotDecodeError` *(N-J-S1)* | `ade_ledger::snapshot::error` | 5 variants | Carried. **N-K note:** `UnknownVersion` / `FingerprintMismatch` at bootstrap map to `AuthorityFatalKind::SnapshotDecode*` → `EXIT_AUTHORITY_FATAL_DECODE = 12`. |
| `StructuralReason` *(N-J-S1)* | `ade_ledger::snapshot::error` | 9 variants | Carried. |
| `encode_chain_dep` / `decode_chain_dep` chokepoint pair *(N-J-S1 — CN-STORE-08)* | `ade_ledger::snapshot::chain_dep` | 2 functions | Carried. `CN-STORE-08.strengthened_in += PHASE4-N-K`. |
| `encode_utxo_state` / `decode_utxo_state` / `encode_cert_state` / `decode_cert_state` / `encode_epoch_state` / `decode_epoch_state` / `encode_pparams` / `decode_pparams` / `encode_gov_state` / `decode_gov_state` / `encode_conway_deposit_params` / `decode_conway_deposit_params` chokepoint pairs *(N-J-S2..S5)* | `ade_ledger::snapshot::{utxo_state, cert_state, epoch_state, gov_state}` | 12 functions | Carried. |
| `encode_ledger_state` / `decode_ledger_state` chokepoint pair *(N-J-S6 — CN-STORE-08)* | `ade_ledger::snapshot::ledger` | 2 functions | Carried. |
| `encode_snapshot` / `decode_snapshot` chokepoint pair *(N-J-S7 — CN-STORE-08)* | `ade_ledger::snapshot::framing` | 2 functions | Carried. **N-K note:** `decode_snapshot` is now a production caller from `bootstrap_initial_state` warm-start; `encode_snapshot` from `PersistentSnapshotWriter::{on_admitted, force_capture}`. |
| `SCHEMA_VERSION: u32 = 1` *(N-J-S7 — DC-STORE-09)* | `ade_ledger::snapshot::framing` | 1 `pub const` | Carried. **N-K note:** `DC-STORE-09` now carries `open_obligation = "snapshot_schema_migration_follow_on_cluster"` — the v1→v2 migration tooling is operator-facing and out of node-binary scope. |
| `PersistentSnapshotCache<'a, S: SnapshotStore + ?Sized>` *(N-J-S8)* | `ade_runtime::rollback::persistent_cache` | closed GREEN struct | Carried. **N-K note:** consumed by both `bootstrap_initial_state` (warm-start read) and `PersistentSnapshotWriter` (cadence-driven writes); two production consumer paths. |
| `PersistentCacheError` *(N-J-S8)* | `ade_runtime::rollback::persistent_cache` | 3 variants | Carried. **N-K note:** `Encode`/`Decode` arms surfaced through `NodeRunError::PersistentWriterIo`/`Bootstrap` → exit-code mapping. |
| `PERSISTENT_CACHE_SCHEMA_VERSION: u32` *(N-J-S8)* | `ade_runtime::rollback::persistent_cache` | 1 `pub const` | Carried. |
| **`bootstrap_initial_state` chokepoint** *(NEW in N-K — CN-NODE-01)* | `ade_runtime::bootstrap` | 1 function — **THE SOLE `pub fn` returning the initial `(LedgerState, PraosChainDepState, Option<ChainTip>)` triple in the workspace** | Single-authority init. Cold-start + warm-start = two branches of one function. CI-defended via `ci_check_bootstrap_closure.sh` (workspace-wide grep + forbidden-patterns check). New init shape = strengthening (CI fail). |
| **`BootstrapInputs<'a, D, S>` / `BootstrapError`** *(NEW in N-K)* | `ade_runtime::bootstrap` | closed struct + closed sum | Inputs are borrowed (lifetime `'a`); generic over `D: ChainDb` + `S: SnapshotStore`. `BootstrapError` carries `Materialize(MaterializeError)`, `ChainDb(ChainDbError)`, `GenesisRequiredButAbsent`, `PersistentCacheDecode(SnapshotDecodeError)`. Closed sums. |
| **`Clock` trait + `DeterministicClock` + `SystemClock`** *(NEW in N-K — DC-NODE-03)* | `ade_runtime::clock` | 1 trait + 2 impls — **THE SOLE wall-clock seam in `ade_runtime`** | Closed seam. New impls remain deliberate registry-tracked closed additions — not runtime plug-ins. CI-defended: `SystemClock` is the only file in `ade_runtime` that may read `SystemTime`/`Instant`/`tokio::time`. |
| **`OrchestratorEvent`** *(NEW in N-K — DC-NODE-01 + DC-NODE-03)* | `ade_runtime::orchestrator::event` | closed sum | The closed event vocabulary. Variants cover peer RX events, slot ticks, server-pump events, persistent-writer notifications, shutdown signals. No `#[non_exhaustive]`; no plug-in registry. |
| **`OrchestratorEffect`** *(NEW in N-K — DC-NODE-01 + DC-NODE-04)* | `ade_runtime::orchestrator::event` | closed sum | The closed effect vocabulary. Variants include `PeerSessionHalted { peer_id, reason: PeerHaltReason }`, `AuthorityFatal(AuthorityFatalKind)`, admit/write/snapshot dispatch. No `#[non_exhaustive]`. |
| **`OrchestratorError` / `PeerHaltReason` / `AuthorityFatalKind`** *(NEW in N-K — DC-NODE-01 + DC-NODE-04)* | `ade_runtime::orchestrator::event` | 3 closed sums | `PeerHaltReason` is the closed reason-tag for `PeerSessionHalted` (no `String`). `AuthorityFatalKind` is the closed sum naming exactly the categories that halt the binary (`ChainWriteIo`, `SnapshotDecodeUnknownVersion`, `SnapshotDecodeFingerprintMismatch`, ...). New variant = closed-sum extension; if it warrants a new exit code, append a new `EXIT_*` constant + extend `NodeRunError::exit_code`. |
| **`PeerId` / `PeerRole`** *(NEW in N-K)* | `ade_runtime::orchestrator::event` | 1 newtype + 1 closed enum | `PeerId` keys per-peer BTreeMaps in `OrchestratorState`. `PeerRole` discriminates inbound vs outbound peer / server-side connection. |
| **`OrchestratorState` + `PerPeerReceiveVersions`** *(NEW in N-K)* | `ade_runtime::orchestrator::state` | closed structs | Field set closed. `per_peer_receive` + `per_peer_server` are `BTreeMap<PeerId, _>` (no `HashMap`). |
| **`orchestrator::core::step` reducer** *(NEW in N-K)* | `ade_runtime::orchestrator::core` | 1 function | Pure `(state, event) → (state', Vec<OrchestratorEffect>)`. No `tokio`; no clock; no I/O. CI-defended via `ci_check_orchestrator_core_purity.sh` + `ci_check_clock_seam.sh`. |
| **`PersistentSnapshotWriter` + `on_admitted` + `force_capture`** *(NEW in N-K — DC-NODE-02)* | `ade_runtime::rollback::persistent_writer` | closed struct + 2 methods — **THE SOLE production cadence-driven caller of `PersistentSnapshotCache::capture` in the workspace** | Single-authority. `on_admitted` consults `should_snapshot_after_block` exclusively; `force_capture` runs unconditionally during shutdown drain. CI-defended via `ci_check_persistent_writer_no_parallel_cadence.sh`. |
| **`NodeRunError`** *(NEW in N-K — DC-NODE-04)* | `ade_node::node` | closed sum | Variants include `Bootstrap(BootstrapError)`, `PersistentWriterIo(...)`. `exit_code(&self) -> i32` is the closed deterministic mapping. |
| **`EXIT_AUTHORITY_FATAL_IO = 10` / `EXIT_AUTHORITY_FATAL_DECODE = 12` / `EXIT_GENERIC_STARTUP = 1`** *(NEW in N-K — DC-NODE-04)* | `ade_node::node` | 3 `pub const i32` — **THE closed exit-code surface for `ade_node`** | Closed constants. New fatal kind = append new `EXIT_*` constant + extend `AuthorityFatalKind` + extend `NodeRunError::exit_code`. No ad-hoc exit codes. |
| `PlutusLanguage` | `ade_plutus::evaluator` | 3 variants | |
| Named ingress chokepoints (block CBOR) | `ade_codec::*` | 10 | |
| Conway cert/withdrawals sub-grammar decoders *(B3 / B4)* | `ade_codec::conway::{cert, withdrawals}` + `ade_codec::shelley::cert::read_pool_registration_cert` | 5 functions | Closed. |
| Named ingress chokepoint (Plutus script CBOR) | `ade_plutus::evaluator::PlutusScript::from_cbor` | 1 | |
| `PreservedCbor::new` constructor | `ade_codec::preserved` | 1 chokepoint, `pub(crate)` | |
| `CodecError` variants *(B3-extended)* | `ade_codec::error` | + `UnknownCertTag`, `DuplicateMapKey` | |
| Mini-protocol message enums | `ade_network::codec::*` | 11 closed enums | |
| Mini-protocol encode/decode chokepoints | `ade_network::codec::*::{encode_*, decode_*}` | 22 functions | |
| Mux frame chokepoints | `ade_network::mux::frame::{encode_frame, decode_frame}` | 2 free functions | |
| Mini-protocol transition functions | `ade_network::*::transition` + `n2c::local_*::transition` | 8 modules | |
| Mini-protocol version enums | `ade_network::codec::version::*` | 11 closed enums | |
| `ChainDb` / `SnapshotStore` / `Recoverable` trait surfaces | `ade_runtime::chaindb` + `ade_runtime::recovery` | closed | **N-K note:** `ChainDb` + `SnapshotStore` are now opened by `bootstrap_initial_state` (production single-authority opener). Trait surfaces unchanged. |
| Hash domain functions | `ade_crypto::blake2b::*` | 4 named domains | |
| `ChainEvent` / `ChainSelectionReject` *(N-B)* | `ade_core::consensus::events` | 5 / 4 variants | |
| Consensus error families *(N-B)* | `ade_core::consensus::errors` | 8 closed error enums | |
| `StreamInput` / `OrchestratorError` (N-B) / `DecodeError` / `GenesisParseError` / `GenesisBlob` / `NetworkMagic` *(N-B)* | various | closed | **Naming note:** the N-K `OrchestratorError` is in `ade_runtime::orchestrator::event` and is structurally distinct from N-B's `ade_core::consensus::orchestrator::OrchestratorError` (different namespace, different domain). |
| `LedgerView` trait *(N-B; B1-refined)* | `ade_core::consensus::ledger_view` | 4 methods | |
| `HeaderVrf` *(N-B; B1)* | `ade_core::consensus::header_summary` | 2 variants | |
| `BlockValidityVerdict` / `BlockValidityError` etc. *(B1)* | `ade_ledger::block_validity::verdict` | closed | |
| `block_validity` chokepoint *(B1)* | `ade_ledger::block_validity::transition` | 1 function | Single chokepoint. Five public consumers: B1 validator, `self_accept` (N-C), `served_chain_admit` (N-G), `admit_via_block_validity` (N-H), `materialize_rolled_back_state` (N-I/N-K-driven). N-K adds no new consumer. |
| `TxValidityVerdict` / `TxRejectClass` / `TxValidityError` / `SignerSource` / `WitnessClosureError` etc. *(B2)* | `ade_ledger::tx_validity::*` | closed | |
| `AdmitOutcome` / `MempoolState` / `OrderPolicy` *(B2)* | `ade_ledger::mempool::*` | closed | |
| `LeaderScheduleAnswer` / `is_leader_for_vrf_output` *(N-B)* | `ade_core::consensus::leader_schedule` | closed | |
| `PraosNonces` / `NonceScanError` *(B1)* | `ade_ledger::consensus_input_extract` | | |
| `PraosChainDepState` / `ChainEvent` canonical encodings *(N-B)* | `ade_core::consensus::encoding` | 4 chokepoints | |
| `LedgerFingerprint` fold *(B3/B5)* | `ade_ledger::fingerprint` | | **N-K note:** used by the replay-equivalence test to compare two runs' `combined` fingerprints byte-identically. |
| **CI check set** | `ci/ci_check_*.sh` | **61 scripts (55 → 61 in PHASE4-N-K)** | Existing checks may be tightened, never relaxed. |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; **N-K added 5 rules** (`CN-NODE-01`, `DC-NODE-01..04` all `enforced`); strengthened 6 carried rules (`T-DET-01`, `CN-CONS-08`, `CN-STORE-07`, `CN-STORE-08`, `DC-CONS-21`, `DC-STORE-08`); added 1 new `open_obligation` (`DC-STORE-09`). Total: **214 entries** (209 → 214). | Append-only IDs. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| `CostModels` map (Plutus V1/V2/V3 cost tables) | `ade_plutus::cost_model::CostModels` | Decoder-driven; constrained by closed `PlutusLanguage`. |
| `ProtocolParameters` / `ProtocolParameterUpdate` field set | `ade_ledger::pparams` | Era-versioned. |
| Pool / DRep / Stake registrations | `ade_ledger::state::{DelegationState, CertState}` | Shape closed; set open. |
| Governance proposal / committee / DRep registration set | `ade_ledger::state::ConwayGovState` | Shape closed; instance set open. |
| Tx-body `proposal_procedures` instance set *(PP)* | `ade_types::conway::tx::ConwayTxBody.proposal_procedures` | `Option<Vec<ProposalProcedure>>`. Shape closed; instance set open. |
| `OpCertCounterMap` *(N-B)* | `ade_core::consensus::praos_state` | BTreeMap; inserts strictly increasing per `(pool, kes_period)`. |
| `PoolDistrView` pool table *(B1)* | `ade_ledger::consensus_view::PoolDistrView::pools` | `BTreeMap<Hash28, PoolEntry>`. |
| Withdrawals map *(B3)* | `ade_codec::conway::withdrawals::decode_withdrawals` → `BTreeMap<RewardAccount, Coin>` | Never last-wins. |
| Mempool admitted set *(B2)* | `ade_ledger::mempool::admit::MempoolState::accepted` | `Vec<Hash32>`; shape closed; monotonic. |
| `SignerSource` provenance set *(B2)* | `ade_ledger::tx_validity::required_signers::RequiredSigners::{keys, provenance}` | Per-tx open. |
| `RollbackSnapshot` ring *(N-B)* | `ade_runtime::consensus::chain_selector::OrchestratorState::recent_snapshots` | Bounded ≤ 2160. Distinct from N-I `InMemorySnapshotCache` + N-J `PersistentSnapshotCache`. |
| `ServedChainSnapshot.blocks` admitted set *(N-G-S2)* | `ade_ledger::producer::served_chain::ServedChainSnapshot` | Shape closed; instance set open. |
| `PerPeerN2nServerState` instance set *(N-G-S6)* | `ade_runtime::network::n2n_server` | One instance per connected peer. |
| `PendingHeaderCache.entries` *(N-H-S1)* | `ade_ledger::receive::pending_header_cache::PendingHeaderCache` | `BTreeMap<(SlotNo, Hash32), Vec<u8>>`. |
| `PerPeerReceiveState` instance set *(N-H-S4)* | `ade_runtime::receive::orchestrator` | One instance per upstream peer. |
| `InMemorySnapshotCache.entries` *(N-I-S4)* | `ade_runtime::rollback::in_memory_cache::InMemorySnapshotCache` | `BTreeMap<SlotNo, (LedgerState, PraosChainDepState)>`. Shape closed; instance set open. No eviction (carried OQ-5). |
| Persistent snapshot store contents *(N-J-S8)* | the `SnapshotStore` instance backing `PersistentSnapshotCache` | `BTreeSet<SlotNo>` per `list_snapshot_slots`. Shape closed; instance set open. **N-K note:** the set now grows from TWO production write callers — `PersistentSnapshotWriter::on_admitted` (cadence-driven) + `PersistentSnapshotWriter::force_capture` (shutdown drain); each routes through `PersistentSnapshotCache::capture` (single cache-level entry). |
| **Per-peer collections in `OrchestratorState`** *(NEW in N-K)* | `ade_runtime::orchestrator::state::OrchestratorState::{per_peer_receive, per_peer_server}` | `BTreeMap<PeerId, _>` (no `HashMap`). One entry per connected peer. Set grows on `OrchestratorEvent::PeerConnected`; shrinks on `OrchestratorEffect::PeerSessionHalted`. Shape closed; instance set open. |
| **Orchestrator-event corpus** *(NEW in N-K — tooling-only)* | `corpus/orchestrator/` | Test corpus for the replay-equivalence harness. Read by `replay_equivalence_under_deterministic_clock_holds`. Tooling-only. |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::*` | Tooling-only. |
| Network corpus / Consensus corpus / Block-validity corpus / Tx-validity corpus / Mempool ingress corpus / PP canonical corpus / Producer corpus / Server-paths corpus / Receive-paths corpus | various | Tooling-only. |
| Receive-rollback integration test *(N-I-S6)* | `crates/ade_runtime/tests/receive_rollback_integration.rs` | Tooling-only. |
| Persistent-cache inline test set *(N-J-S8)* | `crates/ade_runtime/src/rollback/persistent_cache.rs` (inline) | Tooling-only. |
| **Orchestrator integration tests** *(NEW in N-K — tooling-only)* | `crates/ade_runtime/tests/orchestrator_{peer_isolation,replay_equivalence}.rs` + `crates/ade_node/tests/{shutdown_resume_identity,authority_fatal_decode}.rs` | Tooling-only. 4 files / 7 tests. Defend DC-NODE-01, DC-NODE-03, DC-NODE-04. |
| Operator-action probe binaries *(N-B + N-E S6 + N-C S7 + N-G S7 + N-H S6)* | `ade_core_interop::bin::*` | RED operator-action; `#[ignore]`-gated. **N-K added no new binary.** |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. |
| Recovery state types | callers of `Recoverable` | Open: any state with canonical encode + apply-block step. |
| Pinned external crates | `crates/*/Cargo.toml` | Tier-5 rationale doc required. **N-K added:** `tokio` is now an explicit `ade_runtime` dep (was already in scope for `ade_runtime::receive::orchestrator` via the wider RED surface; now gated per-file by `ci_check_orchestrator_core_purity.sh` + `ci_check_clock_seam.sh`). |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| **N-K SUBSUMED the prior revision's "orchestrator-side persistent-capture wiring"** — `PersistentSnapshotWriter` is the production cadence-driven persistent-capture caller. (Removed.) | | |
| **NEW candidate — Live mux + handshake driver cluster** *(NEW candidate flagged by N-K close — `RO-LIVE-01`/`RO-LIVE-02`)* | **A RED submodule `ade_runtime::network::mux_pump` (or similar) bound to `MuxTransport` driving real Ouroboros frames into `peer_session` / `n2n_server_pump`** | Operator-action follow-on; honest-scope half of N-K. Tracked on `RO-LIVE-01`/`RO-LIVE-02`. |
| **NEW candidate — Snapshot schema migration v1 → v2 tooling cluster** *(NEW `DC-STORE-09` `open_obligation` at N-K close)* | **Operator-facing tool reading v1 bytes via `framing::decode_snapshot` and emitting v2 bytes** | Tier-5 operator tooling; out of node-binary scope; named follow-on. |
| **NEW candidate — Metrics + observability cluster** *(NEW candidate flagged by N-K close)* | **Closed `MetricEffect` arm on `OrchestratorEffect` + RED Prometheus exporter** | Tier-5; closed-sum extension on the effect type. |
| **Snapshot eviction policy cluster** *(carried from N-J)* | **Bounded ring + persistent retention policy** | Tier-5 operational concern. Applies to both `InMemorySnapshotCache` and the persistent `SnapshotStore`. |
| **Pre-Conway snapshot encoder cluster** *(carried from N-J)* | **Widen encoders to Babbage and earlier eras** | No current operational need; rollback target windows are bounded. |
| **Multi-peer fork choice cluster** *(carried; now further enabled by N-K stable per-peer task model)* | **Praos longest-chain across competing peers** | Re-uses N-I `RollbackContext`; now restart-safe via N-J; now isolated per-peer via N-K. |
| **N2C local-chain-sync receive surface cluster** *(carried)* | Carried. | |
| **CE-N-H-6 / CE-N-G-8 / CE-N-C-8 operator-action live evidence** *(carried)* | Carried. | |
| **N-I+ Tier-5 — operator-tunable rollback policy** *(carried)* | Carried. | |
| **N-G+ Tier-5 — operator-tunable server policy** *(carried)* | Carried. | |
| **N-C+ Tier-5 — operator-tunable producer policy** *(carried)* | Carried. | |
| **CE-NODE-N2C-LTX** *(carried from N-E)* | Carried. | |
| **PP OQ-1..OQ-4** *(carried)* | Carried. | |
| N-A (deferred) | Peer address book | Runtime mutable. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected (links to metrics cluster). |

### Closed-grammar audit (PHASE4-N-K full close)

This sweep was performed after PHASE4-N-K full close.

1. **`OrchestratorEvent` / `OrchestratorEffect` / `OrchestratorError` /
   `PeerHaltReason` / `AuthorityFatalKind` closed sums** — **closed
   by intent.** Trait-less; data-only; no `#[non_exhaustive]`; no
   `String`. New variant = closed-sum extension.
2. **`bootstrap_initial_state` chokepoint** — **closed by intent and
   CI-defended.** Sole `pub fn` returning the initial triple in the
   workspace (CN-NODE-01 grep + forbidden-patterns check).
3. **`Clock` trait + closed impl set** — **closed by intent and
   CI-defended.** `DeterministicClock` + `SystemClock`; new impl = a
   deliberate registry-tracked closed addition. `SystemClock` is the
   SOLE wall-clock-reading site in `ade_runtime` (DC-NODE-03 grep).
4. **`PersistentSnapshotWriter` cadence chokepoint** — **closed by
   intent and CI-defended.** Sole production cadence-driven caller
   of `PersistentSnapshotCache::capture` (DC-NODE-02 grep against
   `should_snapshot_after_block`).
5. **`OrchestratorState` per-peer collections** — **closed by
   intent.** `BTreeMap<PeerId, _>` shape; no `HashMap`. Per-peer
   isolation enforced by `PeerSessionHalted` discriminant
   (DC-NODE-01 grep).
6. **`NodeRunError::exit_code` + closed `EXIT_*` constants** —
   **closed by intent and CI-defended.** `EXIT_AUTHORITY_FATAL_IO =
   10`, `EXIT_AUTHORITY_FATAL_DECODE = 12`, `EXIT_GENERIC_STARTUP =
   1`. New fatal kind = append `EXIT_*` + extend
   `AuthorityFatalKind` + extend mapping. CI-defended via
   `ci_check_node_binary_uses_single_bootstrap.sh`.

**Gap note — live mux pump above `MuxTransport`.** The honest-scope
RED runner files in this cluster (`peer_session.rs`,
`leadership_session.rs`, `n2n_server_pump.rs`, `ade_node::main`)
defer the actual Ouroboros mux + handshake driver to the
operator-action follow-on tracked by `RO-LIVE-01` / `RO-LIVE-02`.
The binary at this HEAD bootstraps + prints a readiness line.

**Gap note — snapshot schema migration v1 → v2 tooling.** Promoted
from "no current cluster" (N-J) to an explicit `DC-STORE-09`
`open_obligation` at N-K close. `SCHEMA_VERSION = 1`; v1→v2 tooling
is the named follow-on cluster's deliverable.

### Closed-grammar audit (carried — PHASE4-N-J / N-I / N-H / N-G / N-C / PROPOSAL-PROCEDURES-DECODE / N-E / B3 / B4 / B5)

All carried unchanged from prior revision.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Cardano-canonical CBOR wire format**.
- **Block envelope shape**: `[era_tag:u8, era_block:CBOR]`; era tags 0..=7.
- **`PreservedCbor<T>` invariant**.
- **Hash algorithms**: Blake2b-224 / 256, Ed25519, Byron-bootstrap,
  KES-sum, VRF-draft-03.
- **Era-correct block body hash** *(B1; strengthened in N-C, N-G, N-H)*.
- **Single canonical body-hash authority** *(N-C-S4 — DC-CONS-16)*: carried.
- **Single canonical header/body splitter** *(N-G-S1 — DC-CONS-18)*: carried.
- **Server-agency closure for outgoing mini-protocol messages** *(N-G-S1 — CN-PROTO-06)*: carried.
- **Receive-event closure for incoming peer signals** *(N-H-S1 — CN-PROTO-07)*: carried.
- **Type-level receive admission gate** *(N-H-S1 — CN-CONS-07 strengthening)*: carried.
- **Receive-side admission state-isolation discipline (Invariant I-6)** *(N-H-S2 — CN-CONS-08 / DC-CONS-19)*: carried. **N-K note:** `CN-CONS-08.strengthened_in += PHASE4-N-K` — admit path now driven by the production orchestrator.
- **Single canonical receive-side rollback materialization authority** *(N-I-S2 — CN-STORE-07)*: carried. `CN-STORE-07.strengthened_in += PHASE4-N-K` — materialize now driven by the bootstrap warm-start branch.
- **Replay-forward correctness** *(N-I-S2 — DC-CONS-22)*: carried.
- **Atomic rollback commit discipline** *(N-I-S3)*: carried.
- **Receive-side atomic admit + rollback over ChainDb + LedgerState + PraosChainDepState** *(N-H-S2 + N-I-S6 — DC-CONS-20)*: carried.
- **Receive-reducer rollback-context discipline** *(N-I-S6)*: carried.
- **Snapshot cadence determinism** *(N-I-S4 — DC-STORE-07)*: carried. **N-K note:** consulted by `PersistentSnapshotWriter::on_admitted` as the single source.
- **`ChainDbWrite::rollback_to_slot` trait method semantics** *(N-I-S3)*: carried.
- **Single canonical snapshot encoder authority** *(N-J-S7 — CN-STORE-08)*: carried. `CN-STORE-08.strengthened_in += PHASE4-N-K` — encode/decode now driven end-to-end by the production orchestrator.
- **Snapshot encoder canonicality** *(N-J — DC-STORE-08)*: carried. `DC-STORE-08.strengthened_in += PHASE4-N-K` — encoder canonicality exercised by persistent writer + shutdown drain.
- **Snapshot bytes version-tag + fingerprint discipline** *(N-J — DC-STORE-09)*: carried. **N-K note:** `DC-STORE-09` gains
  `open_obligation = "snapshot_schema_migration_follow_on_cluster"` — the v1→v2 migration tooling is the named follow-on.
- **Snapshot encoder Conway-only scope** *(N-J)*: carried.
- **Persistent snapshot reader contract** *(N-J-S8)*: carried. `DC-CONS-21.strengthened_in += PHASE4-N-K` — round-trip equivalence now exercised end-to-end at bootstrap warm-start + shutdown-resume.
- **Receive-side replay determinism** *(N-H-S3 — DC-PROTO-09)*: carried.
- **Per-peer receive-state independence across peers** *(N-H-S4)*: carried. **N-K note:** further strengthened — per-peer isolation now extends to the tokio task layer (DC-NODE-01); a peer-session task failure halts only that task.
- **Key-boundary for receive paths** *(N-H-S4)*: carried.
- **Handshake-negotiated version threading through the receive reducer call site** *(N-H-S4 — DC-PROTO-06 strengthening)*: carried.
- **Served-bytes parity** *(N-G-S4 — DC-CONS-17)*: carried.
- **Header-body wire coherence** *(N-G-S5 — DC-CONS-18)*: carried.
- **Producer-side server-role transcript determinism** *(N-G-S5 — DC-PROTO-07)*: carried.
- **Deterministic-resolution discipline for server-agency waits** *(N-G-S3 — DC-PROTO-08)*: carried.
- **Type-level broadcast and serve gate** *(N-C-S5 — CN-CONS-07)*: carried.
- **Tx id over preserved body bytes** *(B2)*.
- **Conway certificate CDDL grammar** *(B3/B3F/B4)*.
- **Conway `DRep` decode grammar** *(B4)*.
- **Owner-tagged Conway cert-state apply contract** *(B4)*: DC-LEDGER-08.
- **Closed total gov-cert dispatch contract** *(B5)*: DC-LEDGER-09.
- **Fail-fast gov-cert environment** *(B5)*.
- **Checked DRep-expiry arithmetic** *(B5)*.
- **`ConwayGovState` deterministic-fold accumulation** *(B5)*.
- **Conway withdrawals map grammar** *(B3)*: never last-wins.
- **Closed deposit-effect sum types** *(B3)*.
- **Canonical deposit-param authority** *(B3)*: DC-TXV-07.
- **Full Conway value-conservation equation** *(B3)*.
- **`LedgerFingerprint` Conway deposit-param fold** *(B3)*.
- **Closed `proposal_procedures` wire grammar at Conway tx-body key 20** *(PP — DC-LEDGER-11)*.
- **Plutus script ingress chokepoint**: `PlutusScript::from_cbor`.
- **Plutus language set**: V1, V2, V3.
- **Aiken UPLC quarantine pin**: `aiken_uplc` at tag `v1.1.21`.
- **Ouroboros mux frame layout**: 8-byte big-endian header.
- **11 closed mini-protocol message enums** + **8 closed state graphs**.
- **`BootstrapAnchorHash` v1 preimage** *(N-B)*.
- **`EraSchedule` invariants** *(N-B)*.
- **`PraosChainDepState` / `ChainEvent` CBOR encodings** *(N-B)*.
- **Consensus error taxonomies** *(N-B)*.
- **`StreamInput` 3-variant taxonomy** *(N-B)*. **`HeaderVrf` era model**.
- **`block_validity` composition contract** *(B1; N-I strengthened)*: carried.
- **`VerdictSurface` CBOR encoding** *(B1)*.
- **`LedgerView` trait shape** *(N-B; B1-refined)*.
- **`tx_validity` composition contract** *(B2)*.
- **`SignerSource` enumeration** *(B2)*.
- **Witness-closure contract** *(B2)*.
- **`TxVerdictSurface` CBOR encoding** *(B2)*.
- **Mempool admission contract** *(B2)*.
- **`mempool_ingress` chokepoint contract** *(N-E)*.
- **`IngressSource` source-invariance contract** *(N-E)*.
- **Verbatim tx-bytes flow through ingress** *(N-E; N-H mirror)*: carried.
- **GREEN single-step replay fold contract** *(N-E — DC-MEM-04)*.
- **Cross-cluster obligation pattern** *(N-E; carried)*.
- **Operator-action evidence pattern** *(N-B / N-E / N-C / N-G / N-H)*: carried. **N-K adds no new instance** in the probe-binary family; the live mux pump is the follow-on (`RO-LIVE-01`/`RO-LIVE-02`).
- **Closed credential discriminant contract** *(OQ5 / COMMITTEE / DREP / ENACTMENT / PP)*.
- **Committee-enactment write-back contract** *(ENACTMENT)*.
- **All canonical types**: shapes frozen at the era / version they entered.
- **Handshake-negotiated version threading** *(N-A; strengthened in N-G + N-H)*: carried.
- **TCB color assignments**: per `.idd-config.json` `core_paths`. **N-K additions:** `ade_runtime::bootstrap`, `ade_runtime::clock` (trait + `DeterministicClock`), `ade_runtime::orchestrator::{mod, event, state, core}`, `ade_runtime::rollback::persistent_writer`, `ade_node::{cli, lib, node}` are GREEN-inside-RED-crate (single-file GREEN classification — pure, no `tokio::*`, no clock-read). `ade_runtime::clock::SystemClock` and `ade_runtime::orchestrator::{peer_session, leadership_session, n2n_server_pump}` and `ade_node::main` are RED.
- **`ChainDb` / `SnapshotStore` / `Recoverable` trait shapes** (N-D). **N-K note:** `ChainDb` + `SnapshotStore` are now opened by `bootstrap_initial_state` as the production single-authority opener; trait surfaces unchanged.
- **`AcceptedBlock` type-level broadcast gate** *(N-C-S5)*: carried.
- **`AdmittedBlock` type-level admission gate** *(N-H-S1)*: carried.
- **`RollbackContext` BLUE struct seam** *(N-I-S6)*: carried.
- **`SnapshotCadence` BLUE-structural single-field discipline** *(N-I-S4 — DC-STORE-07)*: carried.
- **`forge_block` pure-transition contract** *(N-C-S3)*: carried.
- **Single source of leader truth** *(N-C-S3)*: carried.
- **Tx-admissibility prefix property** *(N-C-S3)*: carried.
- **Private-key custody RED-confinement** *(N-C-S1)*: carried.
- **Closed-grammar opcert byte authority** *(N-C-S2)*: carried.
- **OpCert serial counter strict monotonicity** *(N-C-S2)*: carried.
- **Single bootstrap composition root** *(NEW in N-K — CN-NODE-01)*: `bootstrap_initial_state` is the SOLE `pub fn` returning the initial `(LedgerState, PraosChainDepState, Option<ChainTip>)` triple in the workspace. Cold-start and warm-start are two branches of one function. CI-defended.
- **Per-peer task isolation discipline** *(NEW in N-K — DC-NODE-01)*: each peer session owns its own `PerPeerReceiveState` / `PerPeerN2nServerState`; the shared `LedgerState` view is read-only at the peer task boundary; the single mutating consumer is the orchestrator core under the single authority chain. Decode/validity errors emit `OrchestratorEffect::PeerSessionHalted { peer_id, reason: PeerHaltReason }` (closed reason discriminant) and remove only that peer's entry from the per-peer BTreeMaps.
- **Single cadence-driven persistent-capture authority** *(NEW in N-K — DC-NODE-02)*: `PersistentSnapshotWriter::on_admitted` consults `should_snapshot_after_block` exclusively; the orchestrator core also routes through it. No parallel cadence consultation.
- **Single wall-clock-reading site discipline** *(NEW in N-K — DC-NODE-03)*: `ade_runtime::clock::SystemClock` is the SOLE wall-clock-reading site in `ade_runtime`. All other code consumes `Clock` outputs. Replay equivalence holds under `DeterministicClock`. CI-defended.
- **Closed authority-fatal exit-code surface** *(NEW in N-K — DC-NODE-04)*: `EXIT_AUTHORITY_FATAL_IO = 10`, `EXIT_AUTHORITY_FATAL_DECODE = 12`, `EXIT_GENERIC_STARTUP = 1` are the closed `pub const i32` surface for `ade_node`. `AuthorityFatalKind` is the closed sum of fatal categories; `NodeRunError::exit_code` is the closed mapping. New fatal kind = closed-sum extension + new `EXIT_*` constant + new mapping arm.
- **Shutdown-then-resume byte-identical state contract** *(NEW in N-K — DC-NODE-04)*: after `shutdown_drain` force-captures a final snapshot, the next `bootstrap_initial_state` against the same `(chaindb, snapshot store)` returns a byte-identical triple. Defended by `shutdown_then_resume_produces_byte_identical_state` integration test.

### Version-gated (can evolve across major versions)

- **New `CardanoEra` variant**: full coordinated change.
- **New Conway certificate tag** *(B3 / B4 / B5)*.
- **New `CoinSource` deposit-provenance** *(B3)*.
- **Pre-Conway single-tx validity** *(B2 extension point)*.
- **Full-scope `track_utxo=true` tx corpus** *(B2 extension point)*.
- **Conway block-body vkey-witness closure** *(B2-carried)*.
- **Conway governance certificate accumulation** *(B5)*.
- **Credential discriminant extension** *(declared non-goal)*.
- **Committee-enactment write-back** *(ENACTMENT)*.
- **Conway tx-body `proposal_procedures` decode** *(PP — wired)*.
- **TPraos full-block validity** *(B1 extension point)*.
- **TPraos producer** *(N-C declared non-goal — OQ-4 lock)*.
- **New `GovAction` / Plutus version variant**.
- **New `SignerSource` / `TxRejectClass` / `BlockRejectClass` / `OrderPolicy` variant**.
- **New protocol parameter field**.
- **New `ProducerTick` field** *(N-C extension point)*.
- **New `ForgeError` / `SchedulerInput` / `SchedulerEffect` variant**.
- **New `SelfAcceptError` variant** *(N-C extension point)*.
- **New `ServerStep` / `BlockFetchServerStep` / `ServerReply` etc.** *(N-G extension points)*: carried.
- **New `ReceiveEvent` variant** *(N-H — CN-PROTO-07 extension point)*: carried.
- **New `ReceiveEffect` variant** *(N-H)*: carried.
- **New `ReceiveError` variant** *(N-H)*: carried.
- **New `ChainDbWrite` impl** *(N-H; N-I extended to 2 methods)*: carried.
- **New `ChainDbWrite` trait method** *(N-H extension point)*: carried.
- **New `ReceiveDispatchError` variant** *(N-H)*: carried.
- **New `SnapshotReader` impl** *(N-I; N-J extended)*: carried. At this HEAD there are two production impls (`InMemorySnapshotCache`, `PersistentSnapshotCache`); N-K adds none.
- **New `BlockSource` impl** *(N-I)*: carried.
- **New `MaterializeError` / `CommitRollbackError` variant** *(N-I)*: carried.
- **New `RollbackContext` field** *(N-I)*: carried.
- **New `SnapshotCadence` field** *(N-I — WITH MANDATORY CLUSTER RATIFICATION)*: carried.
- **New `SnapshotEncodeError` / `SnapshotDecodeError` / `StructuralReason` variant** *(N-J — extension point)*: carried.
- **New snapshot sub-state encoder/decoder pair** *(N-J — extension point)*: carried.
- **`SCHEMA_VERSION` bump (v1 → v2)** *(N-J — extension point; tracked on `DC-STORE-09` `open_obligation` at N-K)*: closed versioned-schema seam. First field appended to the framing wire format triggers the bump; the named follow-on cluster ratifies the v2 layout + decoder dispatch table + operator-facing migration tooling.
- **New `PersistentSnapshotCache` field** *(N-J — extension point)*: carried.
- **New `PersistentCacheError` variant** *(N-J — extension point)*: carried.
- **New CI check**: additive. (N-K added six —
  `ci_check_bootstrap_closure.sh`, `ci_check_clock_seam.sh`,
  `ci_check_orchestrator_core_purity.sh`,
  `ci_check_persistent_writer_no_parallel_cadence.sh`,
  `ci_check_peer_session_isolation.sh`,
  `ci_check_node_binary_uses_single_bootstrap.sh`.)
- **Pinned external crate bump**: Tier-5 rationale doc required.
- **New mini-protocol** / **Mini-protocol version-table bump**.
- **New `ChainEvent` / `ChainSelectionReject` / `StreamInput` variant**.
- **New `NetworkMagic`** *(N-B)*.
- **New `LedgerView` impl / LedgerState-backed `PoolDistrView` constructor**.
- **`BootstrapAnchorHash` preimage v2** *(N-B)*: hard version-gated.
- **N2N/N2C tx-submission → `mempool_ingress` ingress** *(N-E)*.
- **Live cardano-node N2N block-fetch acceptance / live N2N follow-mode admission** *(N-C / N-G / N-H)*: each reopens on operator availability.
- **Phase-4 cluster surface additions** (N-F): each cluster's wire surface gates additions via its own cluster doc.
- **New `OrchestratorEvent` variant** *(NEW in N-K — extension point)*: closed sum extension. New variant + new reducer arm in `step` + new translation site in the runner.
- **New `OrchestratorEffect` variant** *(NEW in N-K — extension point)*: closed sum extension. New variant + new dispatch arm in the runner.
- **New `PeerHaltReason` discriminant** *(NEW in N-K — extension point)*: closed sum extension. New discriminant when a new peer-fatal failure category emerges.
- **New `AuthorityFatalKind` discriminant** *(NEW in N-K — extension point)*: closed sum extension. If a new fatal kind warrants a distinct exit code, append a new `EXIT_*` constant + extend `NodeRunError::exit_code`.
- **New `Clock` impl** *(NEW in N-K — extension point)*: deliberate registry-tracked closed addition. New impl ≠ runtime plug-in.
- **New `BootstrapError` variant** *(NEW in N-K — extension point)*: closed sum extension.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. **PHASE4-N-K added
eight new GREEN files** (`ade_runtime::bootstrap`,
`ade_runtime::clock` [Clock trait + DeterministicClock],
`ade_runtime::orchestrator::{mod, event, state, core}`,
`ade_runtime::rollback::persistent_writer`,
`ade_node::{cli, lib, node}`) **and three new RED files**
(`ade_runtime::orchestrator::{peer_session, leadership_session,
n2n_server_pump}` plus `ade_node::main` which co-lives with `lib`
and `node`). **Six new CI gates**. **Five new registry rules** (all
`enforced`). **Six carried rules strengthened**
(`strengthened_in += PHASE4-N-K`). **One new `open_obligation`**
on `DC-STORE-09`. N-K added **no new BLUE**, **no new external
ingress wire-format frozen contract**, **no new operator-action
probe binary**.

**N-K also strengthened the `ade_runtime → ade_ledger` and
`ade_node → ade_runtime` cross-color dependency edges**:

1. `ade_runtime → ade_ledger` (already established;
   **further strengthened in N-K**) — the GREEN
   `bootstrap` reads `ade_ledger::state::LedgerState` +
   `ade_ledger::rollback::{materialize_rolled_back_state, RollbackContext}`;
   the GREEN `persistent_writer` reads
   `ade_ledger::state::LedgerState` + calls
   `PersistentSnapshotCache::capture`; the GREEN `orchestrator::core`
   composes the receive + producer + server BLUE chokepoints. Same
   direction (GREEN → BLUE); allowed.
2. `ade_node → ade_runtime` (NEW edge in N-K) — the RED node binary
   composes the GREEN orchestrator + bootstrap + writer surface.
   Same direction (RED → GREEN → BLUE); allowed.

**The module-addition rule N-K sets for future node-binary-side
work:**

1. **A new orchestration-side GREEN primitive attaches inside
   `ade_runtime::orchestrator::*` or `ade_runtime::rollback::*`** as
   a pure function over closed-sum events/effects. No `tokio::*`,
   no clock, no `HashMap`/`HashSet`/`rand`/float. New canonical
   types MUST be closed sums or closed structs; no
   `#[non_exhaustive]`; no `String`-bearing variants.
2. **A new orchestrator event/effect variant attaches as a closed-sum
   extension on `OrchestratorEvent` / `OrchestratorEffect`** plus
   matching reducer/dispatch arm. Code change required; no plug-in
   registry.
3. **A new RED runner attaches inside `ade_runtime::orchestrator::*`**
   (sibling to `peer_session`, `leadership_session`,
   `n2n_server_pump`). The runner translates RED-side asynchrony
   into `OrchestratorEvent`s and dispatches `OrchestratorEffect`s;
   it never bypasses the GREEN core.
4. **A new `Clock` impl attaches inside `ade_runtime::clock`** as a
   deliberate registry-tracked closed addition. New impls extend
   the closed impl set; the single wall-clock-reading site
   discipline (DC-NODE-03) MUST be preserved.
5. **A new `AuthorityFatalKind` discriminant + matching exit code**
   attaches by: append `EXIT_*` constant in `ade_node::node` +
   extend `AuthorityFatalKind` sum in `ade_runtime::orchestrator::event` +
   extend `NodeRunError::exit_code` mapping. No ad-hoc exit codes.
6. **A new node-binary registry rule attaches as a derived `DC-NODE-*` /
   `CN-NODE-*` family entry** with `code_locus`, `ci_script`,
   `tests`, `cross_ref`. Bidirectional cross-refs to consumed rules
   (`CN-CONS-08`, `CN-STORE-07`, `CN-STORE-08`, `DC-STORE-07`,
   `DC-CONS-20`, `DC-CONS-21`, `T-DET-01`).

### Cross-cluster obligation pattern (carried — `RO-LIVE-01`/`RO-LIVE-02` flagged by N-K close)

**N-K adds no new cross-cluster obligation in the wire-format
sense** but ships the honest-scope half of the live-evidence story:
`peer_session.rs`, `leadership_session.rs`, `n2n_server_pump.rs`,
and `ade_node::main` are real and mechanically evidenced; the
actual Ouroboros mux + handshake driver above `MuxTransport` is
the operator-action follow-on tracked by `RO-LIVE-01` /
`RO-LIVE-02` (live-evidence halves; still
`blocked_until_operator_peer_available`).

### Operator-action evidence pattern (carried — no N-K addition to the probe-binary family)

**N-K adds no new operator-action probe binary** — the family
remains at five. The `ade_node` binary is the production node
entry, not a probe binary. Live evidence will come from running
`ade_node` against a private cardano-node peer in the follow-on
operator-action cluster.

### Cluster scope-edge pattern (carried — strengthened in N-K close)

**N-K applies the scope-edge pattern to the honest-scope split**:
the cluster ships the orchestrator core, bootstrap, persistent
writer, and the binary; the live mux pump is the deliberate
out-of-scope follow-on tracked by `RO-LIVE-01`/`RO-LIVE-02`. The
scope edge is documented in the cluster doc's "Honest-scope note"
section + here.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` | First line of every `.rs` is the contract banner. `lib.rs` carries `#![deny(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_arithmetic)]`. No `#[cfg(feature = ...)]`. No async. **N-K:** no BLUE additions. | Other BLUE crates / submodules only. | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention. **N-K:** GREEN orchestrator files (`bootstrap.rs`, `clock.rs` [trait + `DeterministicClock`], `orchestrator/{mod,event,state,core}.rs`, `rollback/persistent_writer.rs`, `ade_node/{cli,lib,node}.rs`) are pure: no `tokio::*`, no `SystemTime`/`Instant`, no `HashMap` (CI-defended by `ci_check_orchestrator_core_purity.sh` + `ci_check_clock_seam.sh` + `ci_check_bootstrap_closure.sh`). | BLUE crates + standard library + ecosystem crates. **N-K:** the GREEN orchestrator + bootstrap + writer live inside `ade_runtime` (RED crate) — color is per-module per the cluster TCB Color Map. | `ade_runtime` for `ade_testkit`; RED submodules in non-test paths. Results must never feed back into a BLUE authoritative decision. |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys. **N-K:** `SystemClock` + the tokio runner trio + `ade_node::main` are RED. | Any BLUE / GREEN crate or submodule (one-way). **N-K strengthened the `ade_node → ade_runtime` edge** (new) and the `ade_runtime → ade_ledger` edge (carried, further strengthened). | Cannot be depended on by BLUE. |

### New module checklist

1. **Add to `Cargo.toml` workspace members** (if a new crate).
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if BLUE.
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts; for node-binary-side sub-modules, model the new CI gate
   on `ci_check_bootstrap_closure.sh` / `ci_check_clock_seam.sh`
   shape (workspace-wide single-authority grep + forbidden-patterns
   check).
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** add a `[[rules]]` block under family `T`
   in the invariant registry, plus a round-trip test. For new
   node-binary authority rules, append `DC-NODE-0X` / `CN-NODE-0X`
   with bidirectional cross-ref to consumed rules.
7. **New operator-action probe binary:** (not applicable for the
   node-binary domain — `ade_node` is the production entry; live
   evidence comes from running it against a real peer).
8. **Cross-cluster obligation:** (the live mux pump is the named
   follow-on; tracked on `RO-LIVE-01`/`RO-LIVE-02`).
9. **Cluster scope-edge:** if the cluster deliberately scopes down a
   derived constraint, document the carve-out in CODEMAP + the
   cluster doc. N-K's "honest-scope RED runner" is the canonical
   example.
10. **Run `cargo test --workspace` and the full CI script suite.**

### Phase 4 anticipated additions

- **PHASE4-N-K — FULLY CLOSED at this HEAD** (mechanical close;
  honest-scope RED runner): orchestrator core + bootstrap + clock
  seam + persistent writer + RED tokio runner + `ade_node` binary +
  5 new registry rules (`enforced`) + 6 new CI scripts + 6 carried
  rules strengthened + 1 new open_obligation on DC-STORE-09. Live
  mux pump is the operator-action follow-on (`RO-LIVE-01`/`RO-LIVE-02`).
- **PHASE4-N-J — FULLY CLOSED** (carried).
- **PHASE4-N-I — FULLY CLOSED** (carried).
- **PHASE4-N-H — FULLY CLOSED** (carried). CE-N-H-6 live-evidence is
  `blocked_until_operator_peer_available`.
- **PHASE4-N-G — FULLY CLOSED** (carried). CE-N-G-8 live-evidence
  is `blocked_until_operator_peer_available`.
- **PHASE4-N-C — FULLY CLOSED** (carried). CE-N-C-8 live-evidence
  is `blocked_until_operator_stake_available`.
- **PROPOSAL-PROCEDURES-DECODE — FULLY CLOSED** (carried).
- **PHASE4-N-E — FULLY CLOSED** (carried).
- **NEW future cluster — Live mux + handshake driver above `MuxTransport`**
  *(NEW candidate flagged by N-K close — `RO-LIVE-01`/`RO-LIVE-02`)*:
  RED submodule binding the closed `MuxTransport` to the per-peer +
  server-pump tokio tasks. Operator-action evidence at the other end.
  Surface for the next planner.
- **NEW future cluster — Snapshot schema migration v1 → v2 tooling**
  *(NEW `DC-STORE-09` `open_obligation` at N-K close)*: operator-facing
  v1→v2 upgrade tooling. Out of node-binary scope; the seam is set by
  `SCHEMA_VERSION = 1`.
- **NEW future cluster — Metrics + observability** *(NEW candidate
  flagged by N-K close)*: closed `MetricEffect` arm + Prometheus
  exporter. Tier-5.
- **NEW future cluster — Snapshot eviction policy** *(carried from
  N-J)*: Tier-5 operational concern applying to both caches.
- **NEW future cluster — Multi-peer fork choice** *(carried; now
  further enabled by N-K stable per-peer tasks)*.
- **NEW future cluster — N2C local-chain-sync receive surface**
  *(carried)*.
- **NEW low-priority future cluster — Pre-Conway snapshot encoder**
  *(carried)*.
- **Future cluster — `CE-N-H-6` / `CE-N-G-8` / `CE-N-C-8` live
  evidence re-open triggers** (carried).
- **Future node-binary cluster (`CE-NODE-N2C-LTX`)** (carried).
- **Tx-validity completeness follow-ups** (carried).
- **PP OQ-1..OQ-4 follow-ups** (carried).
- **N-F (operator API)**: thin RED layer mapping a closed Query
  enum to gRPC/HTTP. **N-K note:** the orchestrator's closed
  event/effect vocabulary is the natural target for a future
  N-F effect arm.

**These placements are candidates** — user confirmation needed at
cluster entry.

---

## 6. Forbidden Patterns (per color)

### BLUE (universal IDD prohibitions; enforced by CI where marked)

- No `HashMap`, `HashSet`, `IndexMap`, `IndexSet`.
- No `SystemTime`, `Instant`, `std::time::*` clocks.
- No `rand::thread_rng`, `thread::spawn`.
- No `f32`, `f64`, floating-point arithmetic.
- No `std::fs`, `std::net`, `tokio`, `async fn`.
- No `anyhow`; `unwrap`/`expect`/`panic` denied at the lint level.
- No `unsafe` outside an explicit allowlist.
- No `#[cfg(feature = ...)]` semantic gating.
- No signing patterns in BLUE.
- No re-hashing of `canonical_bytes` or re-encoded bytes — wire bytes only.
- No construction of `PreservedCbor` outside `ade_codec`.
- No raw CBOR decoding in any BLUE crate except `ade_codec` and the
  single allowlisted file `crates/ade_plutus/src/evaluator.rs`. **N-J
  carve-out (carried):** `ade_ledger::snapshot::*` uses
  `ade_codec::cbor::*` read/write primitives directly — structured
  composition over the BLUE codec layer.
- No `pallas_*` reference outside `ade_plutus`.
- **(N-A specific)** Carried.
- **(N-B specific)** Carried.
- **(B1 specific)** Carried.
- **(B2 specific)** Carried.
- **(B3 / B4 / B5 specific)** Carried.
- **(OQ5 / COMMITTEE / DREP / ENACTMENT-COMMITTEE-WRITEBACK)** Carried.
- **(N-E specific — closed BLUE chokepoint `mempool_ingress`)** Carried.
- **(PP specific — closed BLUE sub-grammar `decode_proposal_procedures`)** Carried.
- **(N-C-S1..S7 specific)** All carried.
- **(N-G-S1..S4 specific)** All carried.
- **(N-H-S1..S6 specific)** All carried.
- **(N-I-S1..S6 specific)** All carried.
- **(N-J-S1..S8 specific)** All carried.
- **(N-K specific)** No new BLUE in this cluster.

### GREEN (`ade_testkit` incl. all corpora; `ade_runtime::consensus::{candidate_fragment, chain_selector}`; `ade_ledger::mempool::{policy, canonicalize}`; the two `ade_core_interop` N-E bridges; `ade_runtime::producer::{tick_assembler, broadcast_to_served, served_chain_lookups}`; `ade_runtime::receive::{events_to_state, in_memory_chain_write}`; `ade_runtime::rollback::{cadence, in_memory_cache, chaindb_block_source, persistent_cache}`; **`ade_runtime::bootstrap` — NEW in N-K**; **`ade_runtime::clock` [trait + `DeterministicClock`] — NEW in N-K**; **`ade_runtime::orchestrator::{mod, event, state, core}` — NEW in N-K**; **`ade_runtime::rollback::persistent_writer` — NEW in N-K**; **`ade_node::{cli, lib, node}` — NEW in N-K**)

- No nondeterminism that leaks into stored fixtures — fixtures must
  be byte-reproducible.
- No participation in authoritative outputs.
- No `HashMap` even in test helpers — `BTreeMap` only.
- No import of `ade_runtime` from `ade_testkit`.
- (carried bullets per prior revision)
- **(`ade_runtime::bootstrap`, NEW in N-K)** Single `pub fn`
  returning `(LedgerState, PraosChainDepState, Option<ChainTip>)`
  (CN-NODE-01). Cold-start and warm-start are TWO BRANCHES OF ONE
  FUNCTION; no parallel entry point. MUST NOT use `HashMap`/
  `HashSet`/`tokio::*`/`rand`/wall-clock (CI-defended by
  `ci_check_bootstrap_closure.sh`). Re-running against the same
  `(chaindb, snapshot store)` MUST produce a byte-identical triple
  (defended by `shutdown_then_resume_produces_byte_identical_state`).
- **(`ade_runtime::clock`, NEW in N-K — trait + `DeterministicClock`
  GREEN side)** `Clock` trait is the SOLE time seam in
  `ade_runtime`. GREEN side MUST NOT use `SystemTime::now()` /
  `Instant::now()` / `tokio::time::*`. `DeterministicClock` MUST be
  pure (given the same tick vector, two clocks produce the same
  outputs).
- **(`ade_runtime::orchestrator::{mod, event, state, core}`, NEW in
  N-K)** Closed-sum event/effect vocabulary; closed `OrchestratorState`
  with `BTreeMap<PeerId, _>` per-peer collections; pure `step`
  reducer. MUST NOT import `tokio::*` (CI-defended by
  `ci_check_orchestrator_core_purity.sh`). MUST NOT read wall clock
  (CI-defended by `ci_check_clock_seam.sh`). Per-peer halts MUST
  emit `OrchestratorEffect::PeerSessionHalted` (closed reason
  discriminant), never `panic!` (CI-defended by
  `ci_check_peer_session_isolation.sh`).
- **(`ade_runtime::rollback::persistent_writer`, NEW in N-K)**
  `PersistentSnapshotWriter` is the SOLE production cadence-driven
  caller of `PersistentSnapshotCache::capture` (DC-NODE-02).
  `on_admitted` MUST consult `should_snapshot_after_block`
  exclusively; `force_capture` is shutdown-only. MUST NOT introduce
  a parallel cadence policy (CI-defended by
  `ci_check_persistent_writer_no_parallel_cadence.sh`).
- **(`ade_node::{cli, lib, node}`, NEW in N-K — GREEN side)** `node`
  hosts the closed `EXIT_*` constant trio and `NodeRunError::exit_code`
  mapping. MUST map authority-fatal kinds deterministically; new
  fatal kinds slot in via additions to `AuthorityFatalKind` +
  mapping arm + new `EXIT_*` constant if warranted (no ad-hoc exit
  codes). `cli` is closed argv parsing.

### RED (`ade_runtime`, `ade_node`, `ade_network::mux::transport`, `ade_network::session`, `ade_network::bin::capture_*`, `ade_runtime::consensus::genesis_parser`, `ade_core_interop` (incl. five live-session probe binaries), the RED-behavior `ade_ledger::consensus_input_extract` scan; `ade_runtime::producer::{signing, keys, scheduler, broadcast}` (N-C); `ade_runtime::network::n2n_server` (N-G-S6); `ade_runtime::receive::orchestrator` (N-H-S4); `ade_runtime::rollback::snapshot_writer` (N-I-S5); **`ade_runtime::clock::SystemClock` — NEW in N-K (RED sub-classified)**; **`ade_runtime::orchestrator::{peer_session, leadership_session, n2n_server_pump}` — NEW in N-K**; **`ade_node::main` — NEW in N-K**)

- No direct mutation of `ade_ledger` state — all transitions go
  through the established BLUE chokepoints. **(N-K carve-out)** The
  RED runner trio (`peer_session`, `leadership_session`,
  `n2n_server_pump`) translates RED-side asynchrony into
  `OrchestratorEvent`s and dispatches `OrchestratorEffect`s; the
  GREEN core is the SOLE mutator of `OrchestratorState`. The runner
  never bypasses the core; the core never bypasses the BLUE
  chokepoints (`admit_via_block_validity`, `materialize_rolled_back_state`,
  `framing::{encode,decode}_snapshot`).
- No bypassing `ade_codec` to construct semantic types from raw bytes.
- (`ade_runtime` specifically) Existing `ade_runtime → ade_ledger`
  edge is **further strengthened in N-K** — the bootstrap +
  persistent writer + orchestrator core consume the BLUE chokepoint
  surface end-to-end. New edge: `ade_node → ade_runtime` (also
  GREEN → BLUE direction at the transitive level). Passes
  `ci_check_dependency_boundary.sh`.
- (`ade_runtime::clock::SystemClock`, NEW in N-K — RED sub-classified)
  SOLE wall-clock-reading site in `ade_runtime` (DC-NODE-03 grep).
  No other file in `ade_runtime` may read `SystemTime`/`Instant`/
  `tokio::time`. Production-only; replaced by `DeterministicClock`
  in tests + the replay harness.
- (`ade_runtime::orchestrator::{peer_session, leadership_session,
  n2n_server_pump}`, NEW in N-K) RED tokio runner trio. May use
  `tokio::*`, `tokio::time::*`, and OS I/O. MUST translate
  RED-side asynchrony into closed-sum `OrchestratorEvent`s; MUST
  dispatch `OrchestratorEffect`s via the runner's dispatch table.
  MUST NOT mutate `OrchestratorState` directly — only the GREEN
  `step` reducer mutates state. Per-peer failure MUST halt only
  the failing peer's task (DC-NODE-01).
- (`ade_node::main`, NEW in N-K) `tokio::main` entry. Installs
  signal handlers (Ctrl-C / SIGTERM). Drives
  `run_node_until_shutdown`. Maps `NodeRunError` to process exit
  code via `NodeRunError::exit_code`. Shutdown drain force-captures
  a final snapshot via `PersistentSnapshotWriter::force_capture`
  before peer sockets close.
- (`ade_network::mux::transport`) No protocol logic.
- (`ade_network::session`) Composition glue only.
- (`ade_network::bin::capture_*`) Live-interop tools only.
- (`ade_runtime::consensus::genesis_parser`) No re-derivation of the
  bootstrap anchor outside `compute_anchor_hash`.
- (`ade_ledger::consensus_input_extract`) Pure-over-bytes.
- (N-E live N2N operator-action session) Carried.
- (Deferred RED operator-action surfaces — CE-NODE-N2C-LTX) Carried.
- (`ade_core_interop`) Live-interop driver only; library tests
  `#[ignore]`-gated. **N-K added no new binary.**
- **(N-C-S1 / S6 specific — `ade_runtime::producer::*`)** All carried.
  **N-K note:** scheduler is now driven by `leadership_session` via
  `Clock::tick_stream()`; behaviour unchanged.
- **(N-G-S6 specific — `ade_runtime::network::n2n_server`)** Carried.
  **N-K note:** now driven by `n2n_server_pump` RED runner.
- **(N-H-S4 specific — `ade_runtime::receive::orchestrator`)** Carried.
  **N-K note:** now driven per-peer by `peer_session` RED tasks
  under DC-NODE-01 isolation.
- **(N-I-S5 specific — `ade_runtime::rollback::snapshot_writer`)**
  Carried. **N-K note:** still the receive-side in-memory hot path;
  the persistent path now runs in parallel via `PersistentSnapshotWriter`
  in the orchestrator core (DC-NODE-02).

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** —
  enforced by `ci_check_no_secrets.sh`. **N-K:** the node binary's
  CLI accepts paths to operator-supplied genesis + key files; it
  never embeds keys in source.
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces must
  be exercised against real cardano-node peers. **N-K:** the binary
  bootstraps + prints a readiness line; live evidence comes from
  the operator-action follow-on (`RO-LIVE-01`/`RO-LIVE-02`) running
  `ade_node` against a private cardano-node peer.
- **No collapsing wire and canonical bytes** — dual-authority rule.
- **No Tier 5 surface without a stated rationale**.
- **No "we'll match it later" stubs on Tier 1 surfaces** — Tier 1
  closure is hard-gated. **N-K:** the honest-scope split (orchestrator
  core + bootstrap + writer + binary shipped; live mux pump deferred)
  is documented in the cluster doc's "Honest-scope note" and tracked
  by `RO-LIVE-01`/`RO-LIVE-02` — not a Tier-1 stub.

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document. **Cross-reference check at this HEAD:**
  CODEMAP is being regenerated in parallel; pending the regen,
  CODEMAP may pin pre-N-K HEAD `f15102f`. The new GREEN files
  (`bootstrap.rs`, `clock.rs`, `orchestrator/{mod,event,state,core}.rs`,
  `rollback/persistent_writer.rs`, `ade_node/{cli,lib,node}.rs`)
  and the new RED files (`orchestrator/{peer_session,
  leadership_session, n2n_server_pump}.rs`, `ade_node/main.rs`) are
  not yet in the prior CODEMAP. The next CODEMAP regen picks these
  up mechanically. CI count moves from 55 → 61.
- Invariant registry: `docs/ade-invariant-registry.toml` — rule
  families incl. T / CN / DC / OP / RO. **N-K added:**
  `CN-NODE-01` (`enforced`, `ci_script =
  ci/ci_check_bootstrap_closure.sh`); `DC-NODE-01` (`enforced`,
  `ci_script = ci/ci_check_peer_session_isolation.sh`);
  `DC-NODE-02` (`enforced`, `ci_script =
  ci/ci_check_persistent_writer_no_parallel_cadence.sh`);
  `DC-NODE-03` (`enforced`, `ci_script =
  ci/ci_check_clock_seam.sh`); `DC-NODE-04` (`enforced`, `ci_script =
  ci/ci_check_node_binary_uses_single_bootstrap.sh`). **Strengthened:**
  `T-DET-01`, `CN-CONS-08`, `CN-STORE-07`, `CN-STORE-08`,
  `DC-CONS-21`, `DC-STORE-08` each gain `strengthened_in +=
  PHASE4-N-K`. **New open_obligation:** `DC-STORE-09 +=
  "snapshot_schema_migration_follow_on_cluster"`. Total: 209 → 214
  entries.
- Phase 4 cluster plan: `docs/active/phase_4_cluster_plan.md`.
- Tier doctrine: `docs/active/CE-79_gate_statement.md` and
  `docs/active/CE-79_tier5_addendum.md`.
- Cluster N-D / N-A / N-B / N-H / N-I / N-J / B1 / B2 / B3 / B4 / B5 /
  OQ5-CREDENTIAL-FIDELITY / COMMITTEE-CRED-FIDELITY /
  DREP-VOTE-FIDELITY / ENACTMENT-COMMITTEE-FIDELITY /
  ENACTMENT-COMMITTEE-WRITEBACK / PHASE4-N-E /
  PROPOSAL-PROCEDURES-DECODE / PHASE4-N-C / PHASE4-N-G: all closed;
  cluster docs carried.
- **Cluster PHASE4-N-K (CLOSED at this HEAD; mechanical half;
  honest-scope RED runner)**: the cluster doc + closure record at
  `docs/clusters/completed/PHASE4-N-K/{cluster,CLOSURE}.md`.
  SHIPS the orchestrator core + bootstrap + clock seam + persistent
  writer + RED tokio runner trio + `ade_node` binary; closes
  CN-NODE-01 + DC-NODE-01..04 (all `enforced`); strengthens 6
  carried rules; adds `open_obligation =
  "snapshot_schema_migration_follow_on_cluster"` to DC-STORE-09;
  added six CI scripts (count 55 → 61); added five derived registry
  rules (total 209 → 214). Five operator-action probe binaries
  remain in the family (no N-K addition); the live mux pump is the
  follow-on tracked on `RO-LIVE-01`/`RO-LIVE-02`.
- **Future obligation: live mux + handshake driver above `MuxTransport`**
  (`RO-LIVE-01`/`RO-LIVE-02`) — operator-action follow-on.
- **Future obligation: snapshot schema migration v1 → v2 tooling**
  (`DC-STORE-09.open_obligation`) — Tier-5 operator tooling.
- **Future obligation: snapshot eviction policy cluster** — carried
  from N-J.
- **Future obligation: metrics + observability cluster** — NEW
  candidate flagged by N-K close.
- **Future obligation: `CE-N-H-6`** — carried.
- **Future obligation: `CE-N-G-8`** — carried.
- **Future obligation: `CE-N-C-8`** — carried.
- **Future obligation: `CE-NODE-N2C-LTX`** — carried from N-E.
- **Future seam candidates (flagged by N-K close)**: live mux pump
  (highest-priority operator-action follow-on); snapshot schema
  migration v1 → v2 tooling (`DC-STORE-09` open_obligation);
  metrics + observability cluster; snapshot eviction policy cluster
  (carried from N-J); pre-Conway snapshot encoder (low-priority,
  carried); multi-peer fork choice cluster (now further enabled by
  N-K stable per-peer tasks); N2C local-chain-sync receive surface
  cluster.
