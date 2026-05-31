# PHASE4-N-F-D — Slice S2: RED relay loop wired into `--mode node`

> **Status:** slice doc (IDD Part IV). Companion to `cluster.md` (S2 row) and
> the cluster/slice plan. Builds on S1 (`run_loop_planner`, committed
> `a299307`). Code-verified against the S1 HEAD at authoring. **Shape A is
> LOCKED** by the cluster doc; this slice implements it, Shape B is rejected.

> **Slice S2 in one line:** wire the RED `run_relay_loop` composer into the
> `--mode node` lifecycle so both bootstrap/recovery arms converge into a
> continuous, planner-driven, shutdown-clean relay loop that advances the
> durable tip **only** via `run_node_sync → pump_block`.

## 1. Slice identity
- **Cluster:** PHASE4-N-F-D. **Slice:** S2.
- **Touches (RED):** `ade_node::node_lifecycle` (the marked
  `PHASE4-N-F-C-LIFECYCLE-OWNER` — `run_relay_loop` + arm convergence),
  `ade_node::node_sync` (the content-blind readiness signal on
  `NodeBlockSource`), `ade_node::main` (route `Mode::Node` unchanged — it
  already calls `run_node_lifecycle`).
- **Cluster Exit Criteria addressed:** CE-D-2, CE-D-3.

## 2. Invariant scope
- **CN-NODE-02 → enforced:** the loop is the single live-run owner; it
  advances authoritative state ONLY via the existing closed seams; no
  alternate apply / forge / evidence / tip-advance / second-bootstrap path.
- **DC-SYNC-02 → enforced:** every loop iteration advances the tip ONLY
  through `run_node_sync → pump_block`; no verdict/follower/manual-tip path.

## 3. Shape A (LOCKED — entry proof obligation)
- `run_relay_loop` calls `run_node_sync` **once per `SyncOnce` step**.
  `run_node_sync` stays **UNMODIFIED**.
- `NodeBlockSource` gains a **content-blind, non-consuming readiness signal**:
  - **InMemory:** `has_work_ready` = queue non-empty; `is_ended` = queue
    empty (trivial; pure length checks, never inspects bytes).
  - **WirePump (mpsc):** a one-slot, order-preserving lookahead buffer filled
    by non-blocking `try_recv()`. `try_recv` learns only `Ok(event)` /
    `Empty` / `Disconnected` and (per the existing `next_block` filter) skips
    `TipUpdate`, buffers a `Block`'s opaque bytes, treats `Disconnected` as
    end. It **never** inspects/decodes/classifies/hashes/validates/reorders
    block content; the buffered block is still delivered next, in order. This
    is RED scheduling information only.
- `Idle` = cancellation-safe `select!` on (source-readiness, `shutdown.changed()`).
  The only `.await` between iterations is `next_block()` / the readiness wait —
  never mid-`pump_block` — so a shutdown can never tear a durable apply.
- **Hard line (inherited):** if this had required modifying `run_node_sync`,
  or readiness needing block *content*, the slice would STOP and re-scope.
  Neither is needed — confirmed.

## 4. Arm convergence (no more print-and-exit)
- `first_run_mithril_bootstrap` changes its return type to
  `Result<BootstrapState, NodeLifecycleError>` (it already builds the state to
  persist; it now also returns it). `warm_start_recovery` already returns
  `BootstrapState`.
- `run_node_lifecycle_inner`: `let state = match start { FirstRun =>
  first_run_mithril_bootstrap(..)?, WarmStart => warm_start_recovery(..)? };`
  then **both** arms call `run_relay_loop(state, source, &chaindb, &mut wal,
  &era_schedule, &ledger_view, shutdown)`.
- **Sync inputs from recovered state (single-epoch):** `era_schedule` from the
  existing `make_node_schedule(epoch_start_slot, epoch_no)`; `ledger_view` =
  `PoolDistrView::from_seed_epoch_consensus_inputs(recovered)` — the recovered
  single-epoch view. A header outside that epoch makes the view return `None`
  → `pump_block` header-validate fails → `run_node_sync` returns
  `NodeSyncError` → the loop fails closed. **This is the cross-epoch
  containment halt (cluster-scope, not a Cardano compat claim).**

## 5. Binary source — honest hermetic scope
- N-F-D wires **no live peer** (forbidden; the live WirePump source is the
  RO-LIVE-01 follow-on). So the `--mode node` binary, lacking a configured
  live peer source in this cluster, enters `run_relay_loop` with an **empty
  source** → the planner sees `NoWorkReady + Ending` → `HaltCleanly` → clean
  exit, with an honest report. The loop is genuinely **entered + driven** by
  the binary arm (flipping `node_sync`/planner from "reached by no binary arm"
  to "reached"); its real continuous-sync, Idle, and shutdown behavior is
  proven **hermetically** by the `run_relay_loop` tests over populated
  InMemory + in-process-mpsc WirePump sources. This mirrors N-F-C L2 (the live
  leg is wired-capable but proven hermetically; the live pass is gated).

## 6. `run_relay_loop` (RED) shape
```
pub async fn run_relay_loop(
    mut state: ForwardSyncState, source: &mut NodeBlockSource,
    chaindb: &PersistentChainDb, wal: &mut FileWalStore,
    era_schedule: &EraSchedule, ledger_view: &dyn LedgerView,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<(), NodeLifecycleError> {
    loop {
        let shutdown_status = if *shutdown.borrow() { ShutdownRequested } else { Running };
        let sync_status = if source.has_work_ready() { WorkAvailable } else { NoWorkReady };
        let loop_state  = if source.is_ended() { Ending } else { Continuing };
        match plan_loop_step(loop_state, sync_status, shutdown_status) {
            SyncOnce => { run_node_sync(source, &mut state, chaindb, wal,
                                        era_schedule, ledger_view).await
                            .map_err(NodeLifecycleError::RelaySync)?; }
            Idle     => { tokio::select! {
                            _ = source.wait_ready() => {}
                            _ = shutdown.changed()  => {} } }
            HaltCleanly => break,
        }
    }
    Ok(())
}
```
- `ForwardSyncState` is threaded across `SyncOnce` calls (one state, `&mut`).
- A new closed `NodeLifecycleError::RelaySync(String)` wraps a `NodeSyncError`
  fail-closed (no skip-past, no fallback).
- `wait_ready` is a `&mut self` async on `NodeBlockSource` that resolves when
  the next `next_block()` is expected to make progress (or the feed ends) —
  content-blind; it fills the lookahead via `try_recv` / awaits one `recv`.

## 7. Proof obligations (exit criteria)
- [ ] **CE-D-2** — new gate `ci/ci_check_node_run_loop_containment.sh` isolates
      the `run_relay_loop` body (signature → next top-level `^}`), strips
      comments + `#[cfg(test)]`, and asserts: (pos) it calls `run_node_sync(`;
      (neg) it does NOT call `pump_block(` directly, `.put_block(`,
      `AdvanceTip`, `rollback_to_slot(`, `run_real_forge`,
      `forge_one_from_recovered`, `correlate`, `Ba02Manifest`,
      `derive_verdict`, `run_admission(`, `ade_core_interop` / `follow(`,
      `bootstrap_initial_state(` / `bootstrap_from_mithril` /
      `bootstrap_from_conway`. (Bootstrap calls live in the dispatcher, NOT
      the loop body — the gate scopes to the loop body so they don't trip it.)
- [ ] **CE-D-3** — hermetic integration tests:
      - `relay_loop_syncs_then_halts_clean_on_source_end` (InMemory batch →
        SyncOnce drains, durable tip = last block, WAL + checkpoint present,
        HaltCleanly on ended).
      - `relay_loop_halts_clean_on_shutdown_no_partial_write` (shutdown pre-set
        → HaltCleanly with no tip advance / no partial write).
      - `relay_loop_idles_then_syncs_on_incremental_feed` (in-process mpsc
        WirePump: idle before first send → readiness wakes → SyncOnce → close
        → HaltCleanly; tests the Idle + cancellation path hermetically, no
        live peer).
      - `relay_loop_fails_closed_on_cross_epoch_block` (a block outside the
        recovered epoch → `RelaySync` fail-closed, tip not advanced past it).
- [ ] readiness method unit tests: `has_work_ready` / `is_ended` for InMemory
      (empty/non-empty) and WirePump (buffered / empty-open / disconnected).
- [ ] both new + existing gates green; `cargo build -p ade_node`; scoped
      `cargo test -p ade_node` green; touched files `rustfmt`-clean.
- [ ] registry: `CN-NODE-02` + `DC-SYNC-02` status `declared` → `enforced`
      (populate `ci_script` + `tests`); `ci_check_registry_code_locus_exists.sh`
      stays green.

## 8. TCB color
- **RED:** `run_relay_loop` + arm convergence (`node_lifecycle`), the
  `NodeBlockSource` readiness signal (`node_sync`). No BLUE change; the GREEN
  planner (S1) is consumed unchanged.

## 9. Forbidden (inherits the cluster Forbidden list)
- No modification of `run_node_sync`. No second `pump_block` call site.
- No manual tip advance / forge / evidence / verdict / follower / second
  bootstrap on the loop path.
- Readiness must not inspect block content.
- No live peer / wall-clock / slot ticker. No new WAL/checkpoint/canonical type.
- No `cargo fmt -p ade_node` (the crate is not fmt-clean at HEAD; it churns
  ~25 unrelated files) — format ONLY touched files with `rustfmt`.

## 10. Replay / determinism
- No new persisted state; rides DC-SYNC-01 durable-before-tip inside
  `pump_block`. Loop replay-equivalence (S3a) + crash-at-boundary (S3b) build
  on this slice. Determinism: planner pure (S1); the loop's only
  nondeterminism is RED scheduling (which the hermetic tests pin via
  deterministic sources + injected shutdown).

## Authority
Registry IDs `CN-NODE-02`, `DC-SYNC-02` (both → `enforced` at this slice).
`cluster.md` + the registry are authoritative; this doc refines.
