# Invariant Sketch — PHASE4-N-F-D: Live Relay Run-Loop

> **Status:** Invariant sketch (IDD Part I). Planning artifact — precedes
> `/cluster-plan`. Decisions OQ1–OQ5 resolved by the user 2026-05-31.
> Predecessor: PHASE4-N-F-C (closed, `b046b7f`) shipped `node_sync` /
> `ba02_evidence` as tested-but-unwired library surfaces; N-F-D wires the
> **relay half** of the run loop into the binary.

## Scope decisions (settled — load-bearing)

These bound the cluster. The implementer treats them as hard:

1. **Relay-only continuous sync loop.** N-F-D = bootstrap/recovery →
   continuous sync → durable tip advance → clean shutdown. **No forge tick
   in this cluster.** Forging is a separate, fenced successor sub-cluster
   after the loop spine is enforced. (OQ1)
2. **Strictly hermetic.** Acceptance is deterministic: same recovered state +
   same ordered `NodeBlockSource` feed + same deterministic loop inputs +
   same shutdown-injection points ⇒ byte-identical tips, WAL/checkpoints,
   and halt state. **No live docker/preprod peer in this cluster** — the live
   operator pass stays RO-LIVE-01 / operator-gated. (OQ2)
3. **No slot ticking unless required for housekeeping.** Relay sync does not
   need a wall-clock slot tick. If a slot signal is needed, inject a
   deterministic `SlotSource` in hermetic tests. The wall-clock→`SlotNo`
   adapter (RED observe → GREEN convert from explicit `SystemStart` +
   `EraSchedule` → `SlotNo`; only `SlotNo` reaches a seam) is **deferred to
   the forge sub-cluster**. N-F-D only *states the RED boundary prohibition*
   (N8); it builds no wall-clock path. (OQ3)
4. **Crash-replay equivalence is a strengthening, not a new storage law.**
   N-F-D creates no new persistence behavior; it rides DC-SYNC-01 /
   T-REC-01 / T-REC-02. The loop-specific replay statement is captured as
   `T-REC-03` for traceability only — **no new "continuous durability law."**
   Recovery stays snapshot + forward-replay (not full-genesis replay). (OQ4)
5. **Add a tiny GREEN loop planner.** A pure
   `plan_loop_step(loop_state, sync_status, shutdown_status) -> LoopStep`
   with a closed minimal output vocabulary `{ SyncOnce, Idle, HaltCleanly }`.
   It makes **lifecycle** decisions only from already-canonical inputs — it
   must NOT decide ledger validity, chain selection, leadership, forge
   eligibility, or evidence. This is the mechanical proof that "the RED loop
   owns no authority": RED performs effects; GREEN plans iteration; BLUE
   authority stays behind existing seams. (OQ5)

**Hard line (reject the slice if violated):** if implementation pressure makes
the loop add a second apply path, a second recovery path, manual tip
advancement, self-certifying evidence, or wall-clock-derived authority — reject.

## Framing: `canonical input → canonical output`?

A run loop is not itself a pure transformation — it is the **imperative shell
(RED)**, driving sockets, retries, and shutdown. The IDD-correct expression:
the loop is RED orchestration that **owns no authority**. Every authoritative
output it threads — the synced tip, the durable checkpoint, the halt state —
is produced exclusively by the already-closed N-F-C seams (`bootstrap_initial_state`,
`run_node_sync` → `pump_block`). The cluster's discipline is **containment**:
a thin driver, a tiny GREEN planner, and a proof that no authority leaks in.

**Named nondeterminism boundaries:**
- Socket arrival timing / peer-feed ordering → already canonicalized by
  `NodeBlockSource` (yields an *ordered* block-bytes sequence; that ordered
  sequence is the canonical input).
- Shutdown timing (SIGINT/SIGTERM) → RED; must produce a deterministic halt at
  a state boundary, never a partial authoritative write.
- Wall-clock → **out of scope for N-F-D** (no slot tick); the prohibition (N8)
  is stated so the forge sub-cluster inherits it.

---

## 1. What must always be true

- **A1 — Single run-loop owner.** `--mode node` remains the one lifecycle
  owner (CN-NODE-01). The relay loop lives inside it; no second binary arm
  drives sync.
- **A2 — Authority only through closed seams.** Initial state via
  `bootstrap_initial_state`; tip advance via `run_node_sync` → `pump_block`.
  The loop constructs no new authority and no parallel apply path.
- **A3 — Durable-before-advance carries into the loop.** DC-SYNC-01 ordering
  (`StoreBlockBytes` + `AppendWal` durable *before* `AdvanceTip`) holds on
  every iteration — so a crash at any iteration is recoverable to a real
  durable artifact (the `PersistentSnapshotCache` checkpoint
  `warm_start_recovery` reads back).
- **A4 — Clean shutdown is total.** On shutdown signal the loop halts at a
  state boundary leaving on-disk state recoverable; the watch-channel
  `_shutdown` (currently ignored by `run_node_lifecycle`) becomes
  load-bearing.
- **A5 — GREEN planner owns lifecycle, not authority.** The per-iteration
  decision (`SyncOnce` / `Idle` / `HaltCleanly`) is a pure function of
  already-canonical lifecycle inputs only.

## 2. What must never be possible

- **N1 — No verdict-as-sync / follower-as-sync in the loop** (carries
  DC-SYNC-01 containment: no `derive_verdict` / `run_admission` /
  `ade_core_interop::follow` driving the tip).
- **N2 — No manual tip advance** (`.put_block` / `AdvanceTip` /
  `rollback_to_slot`) outside `pump_block`.
- **N3 — No second bootstrap / recovery path** invoked by the loop
  (CN-NODE-01).
- **N4 — No partial authoritative write on shutdown** — an interrupt
  mid-apply must not leave a half-written tip or an unrecorded sidecar.
- **N5 — The GREEN planner must not decide authority** — no ledger validity,
  chain selection, leadership, forge eligibility, or evidence.
- **N6 — No forge / produce / evidence path in this cluster** — relay only.
  No `forge_one_from_recovered`, no `correlate`, no BA-02 manifest wired into
  the loop. (Deferred; fenced.)
- **N7 — No live peer** drives the loop in acceptance — hermetic only.
- **N8 — No wall-clock in any authoritative path** (stated, inherited by the
  forge sub-cluster): no `SystemTime` / `Instant` crosses a seam input.

## 3. What must remain identical across executions

Given identical canonical inputs — same ordered `NodeBlockSource` feed, same
recovered/bootstrapped `BootstrapState`, same deterministic loop inputs, same
shutdown-injection points — the following are byte-identical run to run:
- the sequence of tips `run_node_sync` advances to;
- the post-apply `(ledger, chain_dep)` captured at each checkpoint;
- the `LoopStep` sequence the GREEN planner emits;
- the final halt state.

The loop's *scheduling* (real wall-clock pacing) is **not** in this set — but
N-F-D has no wall-clock pacing (OQ3), so this is moot for the cluster.

## 4. What must be replay-equivalent

- **R1 — Loop-as-replay.** Replaying the same ordered canonical inputs through
  the relay loop produces byte-identical authoritative outputs (tips,
  checkpoints, halt state). Extends T-REC-01 / T-REC-02 from single-shot to
  the loop. → **T-REC-03**.
- **R2 — Crash-replay equivalence (a strengthening, not a new law).** A kill at
  any loop iteration, followed by warm-start recovery + resumed loop on the
  same inputs, lands at the same tip/state as an uninterrupted run. Rides
  DC-SYNC-01 / DC-WAL-03 / T-REC-01/02 — **no new continuous-durability
  invariant.** (OQ4)

## 5. State transitions in scope

Each as `(prior_state, input) → Result<(new_state, effects), error>`:

1. **Compose loop from recovered/bootstrapped base** *(RED orchestration)*
   `(BootstrapState, RunConfig) → Result<(RunningLoop, []), NodeLifecycleError>`
   — the new wiring: after bootstrap/recovery, *enter the loop* instead of
   print-and-exit.
2. **Plan one step** *(GREEN, pure)*
   `(LoopState, SyncStatus, ShutdownStatus) → LoopStep` where
   `LoopStep ∈ { SyncOnce, Idle, HaltCleanly }`.
3. **One sync step** *(drives the L4 seam, unchanged authority)*
   `(ForwardSyncState, NodeBlockSource) → Result<(ForwardSyncState', Option<PumpTip> + durable effects), NodeSyncError)`
   — exactly `run_node_sync`; the loop calls it, never reimplements it.
4. **Shutdown** *(RED)*
   `(RunningLoop, ShutdownSignal) → Result<(Halted, durable-state-intact), NodeLifecycleError>`
   — total, boundary-aligned, no partial write.

## 6. TCB color hypothesis

- **RED (shell)** — the relay run loop itself: the iteration driver, shutdown
  handling, `NodeBlockSource` wiring, and the composition of the seams. Lands
  in `ade_node` (extends `node_lifecycle` or a new `node_run` module).
- **GREEN** — the `plan_loop_step` planner: pure, closed `LoopStep` vocabulary,
  testable with no I/O. Its existence is the mechanical proof of "RED loop owns
  no authority."
- **BLUE** — **none.** No BLUE crate changes. If a BLUE change surfaces, that
  is a red flag the loop is absorbing authority it shouldn't.

## 7. Closure tiers (per user)

- **true:** no RED nondeterminism crosses into BLUE; replay-equivalent outputs
  for the same canonical feed (T-REC-03).
- **derived:** continuous Cardano sync uses only the closed
  `run_node_sync` → `pump_block` path (DC-SYNC-02).
- **constraint:** single live-run lifecycle owner; no alternate apply / forge /
  evidence / tip-advance path (CN-NODE-02).
- **release / operational:** hermetic loop-replay + crash-at-boundary tests are
  the acceptance surface; the **live operator run remains gated** (RO-LIVE-01,
  unchanged — not strengthened by this hermetic cluster).

---

## Registry entries (appended at sketch time, `status = "declared"`)

Appended to `docs/ade-invariant-registry.toml` per the user's explicit
instruction. `introduced_in = "PHASE4-N-F-D"`; `tests = []` / `ci_script = ""`
until slices populate them (the `ci_check_registry_code_locus_exists.sh` gate
skips `declared` rules, so the tree stays CI-green).

- **CN-NODE-02** — single live-run lifecycle owner; advances authoritative
  state only by invoking existing closed seams; no alternate bootstrap / apply
  / forge / evidence / tip-advance path. *(constraint_network)*
- **DC-SYNC-02** — continuous relay sync: every iteration preserves
  durable-before-advance and advances the tip only through
  `run_node_sync` → `pump_block`; verdict / admission / follower paths cannot
  drive the live tip. *(derived; relay-only this cluster)*
- **T-REC-03** — loop-as-replay: same recovered state + same ordered canonical
  block feed + same deterministic loop inputs + same shutdown schedule ⇒
  byte-identical tips, WAL/checkpoints, and halt state. *(true)*

**Deferred (NOT appended):**
- **DC-NODE-05** — forge-slot discipline. Deferred to the forge sub-cluster
  (N-F-D is relay-only).
- **RO-LIVE-07** — not created. Live operator evidence stays a later
  strengthening of RO-LIVE-01, not a new ID.
