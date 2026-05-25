# Invariant Slice — PHASE4-N-I S6

## Slice Header

**Slice Name:** Wire RollBackward branch → close DC-CONS-20
**Cluster:** PHASE4-N-I
**Status:** In Progress
**CEs addressed:** CE-N-I-6
**Registry flips on merge:** `DC-CONS-20` → `enforced` (removes
`open_obligation = "rollback_side_blocked_until_ledger_snapshot_cluster"`).
**Dependencies:** N-I-S1..S5

---

## Intent

The cluster's closure slice. Extends `receive_apply`'s signature to
accept `rollback_ctx: Option<&RollbackContext>`. The RollBackward
arm:
* If `Some(ctx)` → materialize via `materialize_rolled_back_state`
  (S2) + commit via `commit_rollback` (S3); return
  `Ok(ReceiveEffect::RolledBack { to_slot })`.
* If `None` → return legacy `Err(RollbackOutOfScope)` for backward
  compatibility with N-H callers that haven't wired the ctx.

`RollbackContext { snapshot_reader: &dyn SnapshotReader,
block_source: &dyn BlockSource }` lives next to `receive_apply` and
bundles the two read-only trait references the materialize driver
needs.

All N-H test call sites updated to pass `None` (10 sites; backward
compatible — same legacy behavior).

DC-CONS-20 closes: the receive bridge now atomically commits
ChainDb rollback + ledger replacement + chain_dep replacement +
pending-header reset on RollBackward.

---

## The change

### 1. Reducer signature extension

```rust
pub struct RollbackContext<'a> {
    pub snapshot_reader: &'a dyn SnapshotReader,
    pub block_source: &'a dyn BlockSource,
}

pub fn receive_apply<W: ChainDbWrite>(
    state: &mut ReceiveState,
    event: ReceiveEvent,
    chain_write: &mut W,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    rollback_ctx: Option<&RollbackContext>,
) -> Result<ReceiveEffect, ReceiveError>;
```

### 2. RollBackward arm logic (BLUE — new helper `roll_backward`)

```rust
fn roll_backward<W: ChainDbWrite>(
    state: &mut ReceiveState,
    target_point: TargetPoint,
    chain_write: &mut W,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    ctx: &RollbackContext,
) -> Result<ReceiveEffect, ReceiveError> {
    let mat_target = crate::rollback::TargetPoint { slot, hash };
    let (new_ledger, new_chain_dep) =
        materialize_rolled_back_state(mat_target, reader, source, era, view)
            .map_err(map_materialize_err)?;
    commit_rollback(state, mat_target, new_ledger, new_chain_dep, chain_write)
        .map_err(map_commit_err)?;
    Ok(ReceiveEffect::RolledBack { to_slot: target_point.slot })
}
```

`MaterializeError::RollbackTooDeep` / `EraNotSupported` map to
`ReceiveError::RollbackOutOfScope` (preserves the receive surface;
extending `ReceiveError` with rollback-specific variants is a
future refinement).
`MaterializeError::ReplayFailedAt` maps to
`ReceiveError::Validity(_)`.
`CommitRollbackError::ChainDb(_)` maps to `ReceiveError::ChainDb(_)`.

### 3. Integration test
`crates/ade_runtime/tests/receive_rollback_integration.rs`

End-to-end scenarios proving DC-CONS-20 closure.

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_runtime/tests/receive_rollback_integration.rs`:
- `rollback_branch_returns_rolled_back_on_in_memory_snapshot` —
  admit + snapshot + rollback returns `RolledBack`.
- `rollback_branch_returns_rollback_too_deep_when_no_snapshot` —
  empty cache → `RollbackOutOfScope`; state unchanged.
- `rollback_branch_state_unchanged_on_materialize_failure` —
  materialize failure leaves ledger fingerprint + chain_dep +
  pending_headers unchanged.
- `rollback_branch_without_ctx_returns_legacy_rollback_out_of_scope`
  — None ctx = N-H behavior preserved.
- `rollback_then_continue_admit_equals_straight_line_admit` —
  CORE DC-CONS-22 end-to-end proof (snapshot is a pure cache).

---

## §14 Hard Prohibitions

- The RollBackward arm with `Some(ctx)` must not return
  `Ok(_)` if `materialize_rolled_back_state` returns `Err(_)`.
- The RollBackward arm with `Some(ctx)` must not mutate state if
  `commit_rollback` returns `Err(_)`.
- `RollbackContext` must hold only `&dyn SnapshotReader` +
  `&dyn BlockSource` (read-only traits). No write-shaped traits.

---

## §15 Explicit Non-Goals

- Persistent on-disk snapshot encoding (DC-CONS-21 stays declared
  with `open_obligation` per cluster scope decision).
- Snapshot eviction policy.
- Wiring the receive orchestrator's dispatch functions to take
  `RollbackContext` (the orchestrator currently passes `None`;
  future cluster integrates the rollback ctx into the dispatch
  signature when a real node binary needs it).

---

## Replay obligations

`rollback_then_continue_admit_equals_straight_line_admit` is the
end-to-end DC-CONS-22 + DC-CONS-20 closure: snapshot+rollback
produces the same fingerprint as straight-line admit.

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
