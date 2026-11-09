# Cluster/Slice Plan — Ade · PHASE4-N-F-D (Live relay run-loop)

> **Status:** Cluster/slice plan (IDD Part IV). Overall plan only — full
> cluster doc is `/cluster-doc`, slice docs are `/slice-doc`.
> Companion to the invariant sketch
> `docs/planning/phase4-n-f-d-live-node-run-loop-invariants.md` (OQ1–OQ5
> settled). Successor to PHASE4-N-F-C (closed, `b046b7f`), which shipped
> `node_sync` / `ba02_evidence` as tested-but-unwired library surfaces.
> Scope decisions ratified by the user 2026-05-31; S3 split into S3a/S3b
> and Shape A locked per that ratification.

## Cluster Index (Dependency Order)

1. **PHASE4-N-F-D — Live relay run-loop (hermetic spine)** — primary
   invariant: `--mode node` runs a continuous, shutdown-clean relay loop
   that advances the durable tip **only** through the existing
   `run_node_sync → pump_block` seam, governed by a pure GREEN planner that
   owns no authority, and is replay-equivalent across clean runs and across
   a crash at an iteration boundary.

A single cluster, four strictly-ordered slices: the GREEN decision function
(S1) must exist and be proven before the RED loop is pinned to it (S2); the
loop must exist before its clean-replay property (S3a) and its
crash-recovery property (S3b) can be proven; S3a (deterministic orchestration
absent crash interference) precedes S3b (recovery sequencing + persisted-state
validation) so a failure localizes to one proof surface.

---

## PHASE4-N-F-D — Live relay run-loop (hermetic spine)

- **Primary invariant:** the `--mode node` lifecycle owner, after
  bootstrap/recovery on *either* arm (FirstRun, WarmStart), enters a
  continuous relay loop that (a) advances the tip only via
  `run_node_sync → pump_block`, (b) selects each lifecycle step via a pure
  GREEN planner over a closed `{ SyncOnce, Idle, HaltCleanly }` vocabulary,
  (c) halts cleanly on shutdown at a block boundary leaving on-disk state
  recoverable, and (d) is byte-identical across clean replays and across a
  crash at an iteration boundary — **introducing no new authority, no new
  persisted state, no new canonical type, and no BLUE change.**

- **TCB partition:**
  - **BLUE:** *none.* Load-bearing: if a BLUE change surfaces, the loop is
    absorbing authority it must not — reject the slice. Every authoritative
    step is an *existing* closed seam.
  - **GREEN:** a new GREEN-by-content module in `ade_node` (working name
    `run_loop_planner`) — `plan_loop_step(loop_state, sync_status,
    shutdown_status) -> LoopStep` + the closed `LoopStep` / input enums.
    (Precedent: `ade_node::ba02_evidence` is GREEN-by-content with its own
    `//! GREEN` banner inside the RED `ade_node` crate.)
  - **RED:** the loop driver `run_relay_loop` + the `--mode node`
    composition + shutdown handling, in `ade_node::node_lifecycle` (THE
    marked `PHASE4-N-F-C-LIFECYCLE-OWNER`) or a sibling `node_run` it calls;
    plus a read-only, non-consuming readiness signal on `NodeBlockSource`
    (`ade_node::node_sync`, RED) feeding the planner's `sync_status`.

- **Cluster Exit Criteria:**
  - **CE-D-1** — `plan_loop_step` is a pure, total, deterministic function
    over closed input types returning the closed
    `LoopStep { SyncOnce, Idle, HaltCleanly }`; it performs no I/O and
    cannot express a ledger / chain-selection / leadership / forge /
    evidence decision (closed vocabulary + exhaustive tests + a CI gate over
    the planner module). *(CN-NODE-02, planner half)*
  - **CE-D-2** — the `--mode node` run-loop body advances the durable tip
    **only** through `run_node_sync → pump_block`: no second apply path, no
    manual tip advance (`put_block` / `AdvanceTip` / `rollback_to_slot`), no
    follower/verdict-as-sync (`follow` / `derive_verdict` / `run_admission`),
    no forge (`run_real_forge` / `forge_one_from_recovered`), no evidence
    (`correlate` / `Ba02Manifest`), no second bootstrap/recover path —
    enforced by a data-flow-resistant containment gate scoped to the loop
    body. *(DC-SYNC-02 + CN-NODE-02)*
  - **CE-D-3** — `--mode node` enters the relay loop after **both** the
    FirstRun and WarmStart arms (no more bootstrap-and-exit) and runs
    continuously; a shutdown signal halts it cleanly at a block boundary with
    the durable tip equal to the last fully-applied block (no partial write);
    an undecodable/unvalidatable block — including a cross-epoch block beyond
    the recovered single-epoch consensus view — halts the loop fail-closed.
    Hermetic integration test over an in-memory source + injected shutdown.
    *(CN-NODE-02 + DC-SYNC-02)*
  - **CE-D-4** — clean loop replay-equivalence: the same recovered/
    bootstrapped state + same ordered in-memory block feed + same
    deterministic shutdown schedule produce byte-identical tips, WAL, and
    checkpoints across two clean runs. *(T-REC-03)*
  - **CE-D-5** — crash-at-boundary recovery equivalence: a kill at a
    loop-iteration boundary, followed by warm-start recovery + resumed loop
    on the same inputs, lands at the same tip/state as an uninterrupted run.
    *(T-REC-03; strengthens DC-SYNC-01 / T-REC-01 / T-REC-02)*

- **Slices:**
  - **S1 — GREEN loop planner** — invariant: lifecycle step-selection is a
    pure total function over canonical lifecycle inputs; `LoopStep` is closed
    (`{ SyncOnce, Idle, HaltCleanly }`, no `#[non_exhaustive]`, no wildcard
    match) and cannot encode an authority decision. — addresses: CE-D-1 —
    TCB: **GREEN** (new `run_loop_planner` module + banner; CI gate
    `ci_check_loop_planner_closed.sh`: closed enum, pure, forbids
    `pump_block` / `run_node_sync` / forge / `correlate` / `ChainDb` /
    `LedgerState` tokens). Lands tested-but-unwired (precedented by N-F-C).
    Introduces **T-REC-03** as `declared`/`partial`; introduces the planner
    half of **CN-NODE-02**.
  - **S2 — RED relay loop wired into `--mode node`** — invariant: the loop is
    the single live-run owner, planner-driven, shutdown-clean, advancing the
    tip only via `run_node_sync → pump_block`; both bootstrap/recovery arms
    converge into it; fail-closed on undecodable / unvalidatable / cross-epoch
    blocks and at shutdown (no partial write). — addresses: CE-D-2, CE-D-3 —
    TCB: **RED** (`run_relay_loop` + `--mode node` wiring + non-consuming
    `NodeBlockSource` readiness signal; new containment gate
    `ci_check_node_run_loop_containment.sh`; hermetic integration test with
    shutdown injection). **Locks Shape A** (entry proof obligation, below).
    Flips **CN-NODE-02** + **DC-SYNC-02** → `enforced`.
  - **S3a — Clean loop replay-equivalence** — invariant: same recovered state
    + same ordered in-memory block feed + same deterministic shutdown schedule
    ⇒ byte-identical tips, WAL, and checkpoints across two clean runs (proves
    deterministic orchestration absent crash interference). — addresses:
    CE-D-4 — TCB: **RED** test+enforcement (two-runs-byte-identical test over
    tips + WAL + checkpoint bytes). Tier: **true**. Flips **T-REC-03** →
    `enforced`.
  - **S3b — Crash-at-boundary recovery equivalence** — invariant: kill at a
    loop-iteration boundary → warm-start recovery → resume same inputs ⇒ same
    final tip/state as an uninterrupted run (proves recovery sequencing +
    persisted-state validation — a different proof surface from clean
    replay). — addresses: CE-D-5 — TCB: **RED** test+enforcement
    (kill→warm-start-recover→resume vs uninterrupted, compared byte-for-byte).
    Tier: **true**, with derived storage implications. **Strengthens**
    T-REC-03, and appends `strengthened_in += "PHASE4-N-F-D"` to **T-REC-01**,
    **T-REC-02**, **DC-SYNC-01**.

- **Replay obligations:** **No new authoritative state, no new canonical
  type, no new WAL entry/snapshot format, no new replay-corpus entry.** This
  is the cluster's defining property — the loop *composes* existing seams,
  which is exactly why crash-replay is a *strengthening* of DC-SYNC-01 /
  T-REC-01 / T-REC-02 rather than a new durability law (sketch OQ4). T-REC-03
  is discharged by **tests** (S3a two-runs-byte-identical; S3b
  kill→recover→resume), not by adding corpus to `ade_testkit`. Recovery stays
  snapshot + forward-replay (NOT full-genesis replay). Determinism guard: no
  wall-clock (hermetic; no slot tick — OQ3), no rand, no float, `BTreeMap`
  only; the planner is pure; shutdown-injection points are explicit
  deterministic inputs in tests.

---

## S2 entry proof obligation — Shape A (LOCKED)

`run_node_sync` is itself a drain-to-completion loop over the source, but the
planner mandates *iteration-level* control. S2 MUST achieve per-iteration
planner + shutdown granularity **without** a second `pump_block` call site
(DC-SYNC-02 forbids it) and **without** turning the planner into a sync
authority. The implementation path is **Shape A** (Shape B is rejected):

- `run_relay_loop` calls `run_node_sync` once per `SyncOnce` step. Each call
  drains the currently-available batch through the single `pump_block` site,
  captures its E4 `PersistentSnapshotCache` checkpoint, and returns.
- `NodeBlockSource` gains a cheap **non-consuming readiness signal** feeding
  the planner's `sync_status` (it does NOT pop a block; it reports whether one
  is available).
- The `Idle` step is a cancellation-safe `select!(source-ready,
  shutdown.changed())`. Cancellation is clean because the only `.await` is
  `next_block()` *between* blocks, never mid-`pump_block` — so a shutdown can
  never tear a durable apply.
- `run_node_sync` stays **unmodified**; the loop is a thin RED composer over
  it. (Mirror the `produce_mode::run_produce_mode` watch-channel +
  `shutdown_rx.changed()` shutdown pattern.)

**Shape B is rejected:** folding planner/shutdown awareness inside
`run_node_sync` would blur the sync seam and make the sync authority harder to
audit. Per-iteration control must not introduce a second `pump_block` call
site or convert the planner into a sync authority.

---

## Scope boundaries (load-bearing — stated, not assumed)

- **Single recovered epoch only.** The recovered consensus view is
  single-epoch (DC-CINPUT-02a returns `None` off-epoch), so a cross-epoch
  block fails closed and halts the loop. The cross-epoch consensus-view roll
  is a separate future cluster (N-U / cross-epoch), explicitly out of scope.
- **No structured/JSONL evidence vocabulary.** Operational reporting stays
  human-readable stderr; the durable tip + WAL + checkpoint are the
  authoritative facts. A node-event log, if ever wanted, is a separate fenced
  evidence/logging cluster with its own closed vocabulary (avoids the
  shell-overstatement trap).
- **Relay only.** No forge, no `correlate`, no live peer — all fenced out by
  the S2 containment gate.

---

## Hard prohibitions (binding on every slice; reject the slice if violated)

- **No BLUE crate changes.**
- **No new WAL entry type.**
- **No new checkpoint format.**
- **No new canonical type.**
- **No `run_real_forge`, `forge_one_from_recovered`, `correlate`, or
  `Ba02Manifest`** on the loop path.
- **No `derive_verdict`, `run_admission`, or follower-as-sync**
  (`ade_core_interop::follow`).
- **No direct `put_block`, `AdvanceTip`, or `rollback_to_slot`** from the
  loop — the tip advances ONLY through `run_node_sync → pump_block`.
- **No wall-clock, slot ticker, or live peer** in this cluster.
- **No JSONL/event vocabulary** unless introduced in a separate fenced
  evidence/logging cluster.

---

## Registry handling

Three rules already declared at sketch time (`status = "declared"`,
`introduced_in = "PHASE4-N-F-D"`; registry 306 → 309). This cluster promotes
them and strengthens carry-forward rules:

| Rule | Tier | At cluster start | Promotion path |
|------|------|------------------|----------------|
| **CN-NODE-02** | constraint | declared | planner half in **S1**; **enforced in S2** (containment gate). |
| **DC-SYNC-02** | derived | declared | **enforced in S2** (`run_node_sync → pump_block`-only containment). |
| **T-REC-03** | true | declared | introduced `declared`/`partial` in **S1/S2**; **enforced in S3a**; **strengthened in S3b**. |
| **T-REC-01** | true | enforced | `strengthened_in += "PHASE4-N-F-D"` in **S3b**. |
| **T-REC-02** | true | enforced | `strengthened_in += "PHASE4-N-F-D"` in **S3b**. |
| **DC-SYNC-01** | derived | enforced | `strengthened_in += "PHASE4-N-F-D"` in **S3b**. |

**Not added here (fenced out):** **DC-NODE-05** (forge-slot discipline →
forge sub-cluster) and **RO-LIVE-07** (live evidence → later RO-LIVE-01
strengthening). N-F-D is relay-only and hermetic.

---

## Authority

Cluster/slice authority belongs to `docs/ade-invariant-registry.toml`
(CN-NODE-02 / DC-SYNC-02 / T-REC-03 + the strengthened carry-forward rules)
and the per-slice docs once written. This plan is the ordered index only.
Per project discipline, the cluster/slice authority doc is committed
standalone **before** implementing against it.
