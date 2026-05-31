# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `51c9fbf` (PHASE4-N-F-D S1 doc tail of the prior baseline span; `.idd-config.json` `head_deltas_baseline`)
> HEAD: `a02e1f5` (PHASE4-N-F-D S3b — crash-at-boundary recovery equivalence)
> Cluster: **PHASE4-N-F-D — live relay run-loop (hermetic spine)**, closed 2026-05-31.

This window narrates the **PHASE4-N-F-D cluster** — wiring the tested-but-unwired
N-F-C `node_sync` surface into a continuous `--mode node` relay run loop. The cluster
is **relay-only and strictly hermetic**: bootstrap/recovery → continuous sync → durable
tip advance → clean shutdown. No forge, no evidence, no live peer, no slot tick, **no BLUE
crate change**.

## 0. Headline

| Count | Baseline | HEAD | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 108 | **110** | +2 |
| Registry rules | 306 | **309** | +3 |
| Test attributes (workspace) | ~2144 | **2164** | +20 |
| BLUE canonical types | 456 | 456 | 0 (no BLUE change) |

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `a02e1f5` | feat | PHASE4-N-F-D S3b — crash-at-boundary recovery equivalence |
| `f710baa` | docs | add PHASE4-N-F-D S3b slice doc |
| `ead14a7` | feat | PHASE4-N-F-D S3a — clean loop replay-equivalence |
| `c5fdbde` | docs | add PHASE4-N-F-D S3a slice doc |
| `458b67a` | feat | PHASE4-N-F-D S2 — RED relay loop wired into --mode node |
| `d7aa2b7` | docs | add PHASE4-N-F-D S2 slice doc |
| `a299307` | feat | PHASE4-N-F-D S1 — GREEN loop planner |
| `4861e57` | docs | add PHASE4-N-F-D S1 slice doc |
| `b1b5102` | docs | scope PHASE4-N-F-D — live relay run-loop authority docs |

(Plus this close-pass commit: grounding-doc refresh + `.idd-config.json` baseline bump + cluster-doc archive.)

## 2. New Modules

| Module | Color | Purpose |
|--------|-------|---------|
| `ade_node::run_loop_planner` | **GREEN** | Pure lifecycle decision function `plan_loop_step(loop_state, sync_status, shutdown_status) -> LoopStep` over the closed `{ SyncOnce, Idle, HaltCleanly }` vocabulary. Owns no authority; cannot encode a ledger/chain/leadership/forge/evidence decision. (S1) |

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_node::node_lifecycle` | RED | `run_relay_loop` (the relay loop composer); both FirstRun + WarmStart arms converge into it (no more print-and-exit); `first_run_mithril_bootstrap` now returns `BootstrapState`; `run_node_lifecycle_inner` is `async` and threads `shutdown`; new closed `NodeLifecycleError::RelaySync` + `EXIT_NODE_RELAY_SYNC_FAILED = 43`. (S2) |
| `ade_node::node_sync` | RED | `NodeBlockSource::WirePump` becomes a struct variant `{ rx, lookahead, disconnected }` with a **content-blind** availability buffer; `next_block` is now **non-blocking** (drain-available-then-`None`) so `run_node_sync` stays the SOLE block-consumption path; new content-blind readiness signals `has_work_ready` / `is_ended` / `wait_ready`. `run_node_sync` itself is UNMODIFIED. (S2) + the S2/S3a/S3b hermetic tests. |
| `ade_node::lib` | RED | declares `pub mod run_loop_planner;`. (S1) |

## 4. Feature Flags

No feature-flag deltas. No `Cargo.toml` changed.

## 5. CI Checks (108 → 110)

| Check | Status | Backs |
|-------|--------|-------|
| `ci_check_loop_planner_closed.sh` | **New** (S1) | CN-NODE-02 (planner half) — closed `LoopStep`, no `#[non_exhaustive]`, no wildcard arm, no authority/I/O token in the GREEN planner. |
| `ci_check_node_run_loop_containment.sh` | **New** (S2) | CN-NODE-02 / DC-SYNC-02 — `run_relay_loop` calls `run_node_sync(` and reaches NO direct `pump_block` / manual tip advance / forge / evidence / verdict / follower / second-bootstrap token. |

`ci_check_node_sync_via_pump.sh` stays green (`run_node_sync` unmodified).

## 6. Canonical Type Registry Delta

n/a — no BLUE crate changed; the 456 BLUE canonical-type total is unchanged. The new
`LoopStep` / `ShutdownStatus` / `SyncStatus` / `LoopState` and the `NodeBlockSource`
struct-variant fields live in the RED/GREEN `ade_node` crate and are not canonical-counted.

## 7. Normative / Invariant Rule Delta (306 → 309)

### New rules (declared at sketch, promoted this cluster)

| ID | Tier | Status | Summary |
|----|------|--------|---------|
| `CN-NODE-02` | constraint | **enforced** (S2) | `--mode node` is the single live-run lifecycle owner; advances authoritative state only via the existing closed seams; no alternate apply/forge/evidence/tip-advance/second-bootstrap path. |
| `DC-SYNC-02` | derived | **enforced** (S2) | Continuous relay sync: every iteration advances the tip only via `run_node_sync → pump_block`; no verdict/follower/manual-tip path. |
| `T-REC-03` | true | **enforced** (S3a) | Loop-as-replay: same recovered state + same ordered feed + same shutdown schedule ⇒ byte-identical tips, WAL, checkpoints. |

### Strengthenings (append-only `strengthened_in += "PHASE4-N-F-D"`)

- `T-REC-01`, `T-REC-02` — relay loop's advanced tip is replay-/recovery-derivable (S3b).
- `DC-SYNC-01` — durable-before-advance holds every loop iteration (S3b).

## 8. Honest residual (cluster scope)

**Hermetic only — no live peer, no BA-02 claim.** N-F-D wires no live peer source (the
binary enters the loop with an empty source and halts cleanly on the first tick, proving
loop reachability + both-arms convergence); the populated-source behavior (durable sync,
idle/shutdown, fail-closed, replay-equivalence, crash-recovery) is proven hermetically by
the `run_relay_loop` tests. A **live, unbounded WirePump peer's** continuous-batch
operation over a never-closing channel is the RO-LIVE-01 follow-on (the non-blocking
`next_block` drain handles terminating/hermetic sources; live unbounded batching is future
work). RO-LIVE-01 remains partial/operator-gated. The hard line held throughout:
`run_node_sync` is UNMODIFIED, and the readiness lookahead is content-blind.
