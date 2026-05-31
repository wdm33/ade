# Cluster PHASE4-N-F-D — Live relay run-loop (hermetic spine)

> **Status: OPEN (code-verified 2026-05-31, `b046b7f`).** Successor to PHASE4-N-F-C
> (closed, `b046b7f`), which shipped `node_sync` (`run_node_sync` / `NodeBlockSource`)
> and `ba02_evidence` as **tested-but-unwired library surfaces** — `--mode node` today
> bootstraps/recovers then prints-and-exits (`run_node_lifecycle` ignores its
> `_shutdown: watch::Receiver<bool>`). This cluster wires the **relay half** into a
> continuous loop. Every "reuse X" below is verified by reading the function body at
> `b046b7f`, not headers/docs.
>
> Companion docs: `../../planning/phase4-n-f-d-live-node-run-loop-invariants.md`
> (invariant sketch — OQ1–OQ5 settled), `../../planning/phase4-n-f-d-cluster-slice-plan.md`
> (the S1→S2→S3a→S3b ordered plan, Shape A locked).
>
> **Hard line:** if implementation discovers that `run_node_sync` must be modified, or that
> readiness requires peeking into block content, **stop and re-scope** rather than smuggling
> authority into the loop.

## Primary invariant
The `--mode node` lifecycle owner, after bootstrap/recovery on **either** arm (FirstRun,
WarmStart), enters a continuous relay loop that advances the durable tip **only** through
the existing `run_node_sync → pump_block` seam, selects each lifecycle step via a pure
GREEN planner over a closed `{ SyncOnce, Idle, HaltCleanly }` vocabulary, halts cleanly on
shutdown at a block boundary, and is byte-identical across clean replays and across a crash
at an iteration boundary — introducing **no new authority, no new persisted state, no new
canonical type, and no BLUE change** (`CN-NODE-02`, `DC-SYNC-02`, `T-REC-03`).

## The one loop

```
ENTRY (both arms converge — no more bootstrap-and-exit):
  run_node_lifecycle → bootstrap (FirstRun) | warm_start_recovery (WarmStart)
    → BootstrapState  →  run_relay_loop(state, source, chaindb, wal, …)

LOOP (RED driver; GREEN plans each step; BLUE authority stays behind the seam):
  loop:
    sync_status   = NodeBlockSource readiness (cheap, NON-consuming, RED scheduling only)
    shutdown      = shutdown_rx.borrow()
    plan_loop_step(loop_state, sync_status, shutdown)  →  LoopStep      (GREEN, pure)
      SyncOnce    → run_node_sync(...) once   (drains available batch via the SINGLE
                    pump_block site; captures its E4 PersistentSnapshotCache checkpoint)
      Idle        → select!(source-ready, shutdown.changed())   (cancellation-safe;
                    only .await is next_block() BETWEEN blocks, never mid-pump_block)
      HaltCleanly → break   (tip == last fully-applied block; on-disk state recoverable)
```

**The tip is a durable-apply fact, never an agreement verdict.** No forge, no evidence, no
live peer, no slot tick, no wall-clock in this cluster — relay only.

## Locked rules (from the OQ1–OQ5 ratification)

- **Relay-only.** Continuous sync + durable tip advance + clean shutdown. No forge tick,
  no `correlate`/BA-02. Forging is a separate fenced successor sub-cluster. (OQ1)
- **Strictly hermetic.** Acceptance is deterministic over an in-memory `NodeBlockSource`
  + injected shutdown points → byte-identical tips / WAL / checkpoints / halt. **No live
  docker/preprod peer** — the live operator pass stays `RO-LIVE-01` / operator-gated. (OQ2)
- **No slot tick.** Relay needs none; if a slot signal is ever required it is an injected
  deterministic `SlotSource` in tests. The wall-clock→`SlotNo` adapter is **deferred to the
  forge sub-cluster**; this cluster only states the RED-boundary prohibition (no `SystemTime`
  / `Instant` crosses a seam input). (OQ3)
- **Crash-replay is a strengthening, not a new law.** No new persistence behavior; rides
  `DC-SYNC-01` / `T-REC-01` / `T-REC-02`. Recovery stays snapshot + forward-replay, NOT
  full-genesis. (OQ4)
- **Shape A locked (S2 entry proof obligation).** Per-iteration planner + shutdown
  granularity is achieved by calling `run_node_sync` **once per `SyncOnce` step** + a cheap
  non-consuming `NodeBlockSource` readiness signal + a cancellation-safe `Idle` select.
  `run_node_sync` stays **unmodified**; the loop is a thin RED composer. **Shape B (planner
  awareness inside `run_node_sync`) is rejected** — it would blur the sync seam. (OQ5)
- **Readiness is RED scheduling information only.** The `NodeBlockSource` readiness signal
  **must not inspect, decode, classify, hash, validate, reorder, or consume block bytes.**
  It may answer **only** whether a subsequent `next_block()` is expected to make progress.
  Otherwise "readiness" can accidentally become a second ingress/classification path. (OQ5)
- **GREEN planner owns lifecycle, not authority.** `LoopStep` is closed; the planner cannot
  encode a ledger / chain-selection / leadership / forge / evidence decision, and its inputs
  expose only lifecycle-level status — not ledger, chain-selection, forge, block-identity, or
  evidence authority objects. (OQ5)

## Verified component inventory (read, not assumed — `b046b7f`)

| Component | Real state at `b046b7f` | Use |
|---|---|---|
| `node_lifecycle::run_node_lifecycle(cli, _shutdown)` | bootstraps/recovers then **prints + exits**; `_shutdown: watch::Receiver<bool>` ignored | S2 makes it enter the loop; `_shutdown` becomes load-bearing |
| `node_sync::run_node_sync(source, state, chaindb, wal, era_schedule, ledger_view) -> Result<Option<PumpTip>, NodeSyncError>` | drain-to-completion over `source`; sole `pump_block` caller; captures E4 `PersistentSnapshotCache` checkpoint at selected tip | S2 calls **unmodified** once per `SyncOnce` (Shape A) |
| `node_sync::NodeBlockSource` | closed `{ WirePump(mpsc::Receiver<AdmissionPeerEvent>) \| InMemory(VecDeque<Vec<u8>>) }`; `next_block()` yields only `Block`, skips `TipUpdate`, ends on `Disconnected`/closed | S2 source; **gains** a non-consuming, content-blind readiness method (RED scheduling only) |
| `forward_sync::pump_block` (+ `ForwardSyncState`, `PumpTip`, `NoCheckpointSink`) | durable validated apply, durable-before-tip (`DC-SYNC-01`); driven only by `run_node_sync` | reused via `run_node_sync`; **never called directly by the loop** |
| `produce_mode::run_produce_mode` | `watch::Receiver<bool>` + `shutdown_rx.changed()` slot-loop shutdown pattern | S2 mirrors the shutdown-watch pattern (NOT the slot ticker) |
| `ba02_evidence` (GREEN-by-content in `ade_node`) | `//! GREEN` banner inside the RED `ade_node` crate | precedent for the new GREEN `run_loop_planner` module |

## Slices (safety order)

### S1 — GREEN loop planner *(hermetic)*
New GREEN-by-content module in `ade_node` (working name `run_loop_planner`) with a
`//! GREEN` banner: `plan_loop_step(loop_state, sync_status, shutdown_status) -> LoopStep`
and the closed `LoopStep { SyncOnce, Idle, HaltCleanly }` + closed input enums. Pure, total,
no I/O, no `#[non_exhaustive]`, no wildcard match. Planner inputs expose only lifecycle-level
status — not ledger, chain-selection, forge, block-identity, or evidence authority objects.
Lands tested-but-unwired (precedented by N-F-C). Introduces `T-REC-03` as `declared`;
introduces the planner half of `CN-NODE-02`.

### S2 — RED relay loop wired into `--mode node` *(hermetic; Shape A locked)*
`run_relay_loop` (RED, in/alongside `node_lifecycle`) drives the planner-selected steps;
both bootstrap/recovery arms converge into it (no more print-and-exit). Tip advances **only**
via `run_node_sync → pump_block`. `NodeBlockSource` gains a cheap **non-consuming,
content-blind** readiness signal feeding `sync_status` — RED scheduling information only; it
must not inspect/decode/classify/hash/validate/reorder/consume block bytes, only answer
whether the next `next_block()` is expected to make progress. Shutdown mirrors the
`produce_mode` watch pattern; `Idle` is a cancellation-safe `select!(source-ready,
shutdown.changed())` — the only `.await` is `next_block()` between blocks, never
mid-`pump_block`, so shutdown can't tear a durable apply. Fail-closed on undecodable /
unvalidatable / cross-epoch blocks and at shutdown (no partial write).

> **Cross-epoch halt is cluster-scope containment, not Cardano semantics.** A block beyond
> the recovered single-epoch consensus view halts the loop fail-closed. This is a
> **cluster-scope containment rule, not a Cardano compatibility claim** — full cross-epoch
> consensus-view rollover is deferred to the successor cluster; N-F-D may halt fail-closed
> rather than silently inventing an off-epoch consensus input.

Flips `CN-NODE-02` + `DC-SYNC-02` → enforced.

### S3a — Clean loop replay-equivalence *(hermetic)*
Two clean runs over identical inputs (same recovered/bootstrapped state + same ordered
in-memory feed + same deterministic shutdown schedule) produce byte-identical tips, WAL, and
checkpoints. Proves deterministic orchestration absent crash interference. Flips `T-REC-03`
→ enforced.

### S3b — Crash-at-boundary recovery equivalence *(hermetic crash/restart)*
Kill at a loop-iteration boundary → warm-start recovery (the existing N-F-C path) → resume
same inputs ⇒ same final tip/state as an uninterrupted run. Proves recovery sequencing +
persisted-state validation (a different proof surface from S3a). Strengthens `T-REC-03`;
appends `strengthened_in += "PHASE4-N-F-D"` to `T-REC-01`, `T-REC-02`, `DC-SYNC-01`.

## Exit criteria (mechanical, CI-verifiable)
New test/check names are **candidate** (created by the owning slice); existing artifacts named as-is.

- **CE-D-1** — `plan_loop_step` is pure/total/deterministic over closed inputs returning the
  closed `LoopStep { SyncOnce, Idle, HaltCleanly }`; no I/O; cannot encode a ledger /
  chain-selection / leadership / forge / evidence decision. Planner inputs may expose only
  lifecycle-level status, not ledger, chain-selection, forge, block-identity, or evidence
  authority objects. Candidate gate `ci_check_loop_planner_closed.sh` (closed enum, no
  `#[non_exhaustive]`/wildcard; forbids `pump_block` / `run_node_sync` / `run_real_forge` /
  `correlate` / `ChainDb` / `LedgerState` / `BlockHash` / `SlotNo` / `ChainTip` / `PumpTip`
  tokens in the planner module — unless one is explicitly required as an opaque status input,
  named in the slice doc) + candidate planner unit tests. *(`CN-NODE-02` planner half)*
- **CE-D-2** — the `--mode node` run-loop body advances the tip **only** via
  `run_node_sync → pump_block`: candidate gate `ci_check_node_run_loop_containment.sh`
  (data-flow-resistant, scoped to the loop body) forbids a second apply path, manual tip
  advance (`put_block`/`AdvanceTip`/`rollback_to_slot`), follower/verdict-as-sync
  (`follow`/`derive_verdict`/`run_admission`), forge
  (`run_real_forge`/`forge_one_from_recovered`), evidence (`correlate`/`Ba02Manifest`), and a
  second bootstrap/recover path. *(`DC-SYNC-02` + `CN-NODE-02`)*
- **CE-D-3** — `--mode node` enters the loop after both arms and runs continuously; a
  shutdown signal halts it at a block boundary with the durable tip == last fully-applied
  block (no partial write); an undecodable/unvalidatable/cross-epoch block halts fail-closed.
  Candidate hermetic integration test `node_run_loop_enters_and_shuts_down_clean` (in-memory
  source + injected shutdown) + `node_run_loop_fails_closed_on_cross_epoch_block`. *(`CN-NODE-02` + `DC-SYNC-02`)*
- **CE-D-4** — clean loop replay-equivalence: candidate test
  `node_run_loop_two_runs_byte_identical` asserts byte-identical tips + WAL + checkpoint bytes
  across two clean runs over identical inputs. Flips `T-REC-03` → enforced. *(`T-REC-03`)*
- **CE-D-5** — crash-at-boundary recovery: candidate test
  `node_run_loop_kill_at_boundary_recovers_same_tip` asserts kill→warm-start→resume lands at
  the same tip/state as an uninterrupted run. *(`T-REC-03`; strengthens `DC-SYNC-01` /
  `T-REC-01` / `T-REC-02`)*

## TCB color map
- **BLUE (none — reuse only):** no BLUE crate is touched. Every authoritative step is an
  existing closed seam (`bootstrap_initial_state`, `forward_sync::pump_block` via
  `run_node_sync`, BLUE validity inside `pump_block`). A BLUE change is a red flag the loop is
  absorbing authority → reject.
- **GREEN:** the new `ade_node::run_loop_planner` module (`plan_loop_step` + closed `LoopStep`
  / input enums), `//! GREEN` banner (precedent: `ade_node::ba02_evidence`).
- **RED:** `run_relay_loop` + the `--mode node` composition + shutdown handling (in/alongside
  `ade_node::node_lifecycle`, the marked `PHASE4-N-F-C-LIFECYCLE-OWNER`); the non-consuming,
  content-blind `NodeBlockSource` readiness signal in `ade_node::node_sync`.
- **CI:** candidate `ci_check_loop_planner_closed.sh` (S1), candidate
  `ci_check_node_run_loop_containment.sh` (S2); the existing
  `ci_check_node_sync_via_pump.sh` / `ci_check_node_mode_closure.sh` /
  `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` continue to hold.

## Forbidden during this cluster *(slice-level hard prohibitions inherit from this list)*
- No BLUE crate changes.
- No new WAL entry type.
- No new checkpoint format.
- No new canonical type.
- No `run_real_forge`, `forge_one_from_recovered`, `correlate`, or `Ba02Manifest` on the loop path.
- No `derive_verdict`, `run_admission`, or follower-as-sync (`ade_core_interop::follow`).
- No direct `put_block`, `AdvanceTip`, or `rollback_to_slot` from the loop — the tip advances
  ONLY through `run_node_sync → pump_block`.
- No wall-clock, slot ticker, or live peer in this cluster.
- No JSONL/event vocabulary unless introduced in a separate fenced evidence/logging cluster.
- **Readiness must not peek into block content** (no inspect/decode/classify/hash/validate/
  reorder/consume) — it is RED scheduling information only.
- Shape B (planner awareness inside `run_node_sync`) is rejected; `run_node_sync` stays unmodified.
- No `HashMap`/clock/float/async-in-BLUE (no BLUE is touched, but the planner stays pure-deterministic).
- **Hard line:** if `run_node_sync` must change, or readiness needs block content, **stop and
  re-scope** — do not smuggle authority into the loop.

## Replay obligations (scoped)
**No new canonical type, no new authoritative transition, no new WAL/checkpoint format, no new
`ade_testkit` corpus entry** — the cluster composes existing seams. `T-REC-03` is discharged by
**tests** (S3a `node_run_loop_two_runs_byte_identical`; S3b
`node_run_loop_kill_at_boundary_recovers_same_tip`), not by adding oracle/corpus. Determinism
guard: no wall-clock/rand/float, `BTreeMap` only, planner pure, shutdown-injection points are
explicit deterministic inputs. Acceptance scoped to touched crates (`ade_node`, `ade_runtime`,
specific `ade_node` integration tests) — **not** the full `ade_testkit` corpus/oracle lane
(times out ~600s on clean HEAD).

## Registry impact (at close)
Three rules already `declared` at sketch time (registry 306 → 309). Promotion/strengthening:
- `CN-NODE-02` (constraint) — `declared` → **enforced** in S2 (`ci_check_node_run_loop_containment.sh`).
- `DC-SYNC-02` (derived) — `declared` → **enforced** in S2.
- `T-REC-03` (true) — `declared` → **enforced** in S3a; strengthened in S3b.
- `T-REC-01`, `T-REC-02`, `DC-SYNC-01` — `strengthened_in += "PHASE4-N-F-D"` in S3b.
- **Not added here (fenced out):** `DC-NODE-05` (forge-slot discipline → forge sub-cluster),
  `RO-LIVE-07` (live evidence → later `RO-LIVE-01` strengthening).

## Non-goals
No forge / no `correlate` / no BA-02 claim. No live peer / no operator pass (stays `RO-LIVE-01`
gated). No wall-clock / slot ticker (forge sub-cluster). **No cross-epoch production** — the
single-epoch recovered view is the cluster boundary; a cross-epoch block halts the loop
(cluster-scope containment, not a permanent Cardano protocol rule; rollover is the successor
cluster's job). No new BLUE authority/type. No new durability subsystem. No JSONL/event
vocabulary. No grounding-doc regeneration (that's `/cluster-close`).
```
