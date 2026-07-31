# SLICE S1 — rollback-following in the `--mode node` forge path

> Route legal live rollbacks in `run_node_sync` through the **already-enforced** participant
> rollback machinery instead of failing closed. Shell-only (`ade_node`); **no BLUE edits**.

## Problem (from the 2026-07-31 live forge attempt)

`run_node_sync` (`crates/ade_node/src/node_sync.rs:595`) is the single-producer forge driver. Its
item loop has two arms: `Block → pump_block`, and `RollBack →` a match (`node_sync.rs:634-641`) that
accepts ONLY the recovered-anchor rewind and returns `NodeSyncError::UnexpectedRollback` on the
`_ =>` arm. A routine live fork therefore kills the forge. The participant path
(`run_participant_sync`, `node_lifecycle.rs:5108`) already follows rollbacks correctly (DC-NODE-23..29).

## Design — extract a shared helper, route both loops through it

### 1. New `pub(crate)` helper in `node_lifecycle.rs`

Extract the participant `RollBack` arm body (`node_lifecycle.rs:5257-5344`) verbatim:

```rust
pub(crate) fn resolve_and_apply_peer_rollback<D>(
    state: &mut ForwardSyncState,
    chaindb: &D,
    wal: &mut dyn WalStore,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    epoch_accumulator: Option<&EpochAccumulatorStore>,
    security_param: SecurityParam,
    wire_point: WirePoint,             // ade_network::codec::chain_sync::Point
    pending_reselection: &mut bool,
) -> Result<(), NodeSyncError>
where D: ChainDb + SnapshotStore
```

Body = the participant arm's logic, with `continue` → `return Ok(())` (anchor no-op) and the applied
tail → `Ok(())`. Steps (unchanged, all typed halts preserved):
1. `Origin` → `Err(UnexpectedRollback)`.
2. Recovered-anchor (slot==anchor.slot && hash==anchor.hash) → `Ok(())` no-op (DC-NODE-33).
3. `chaindb.get_block_by_hash(&hash)`: `None` → `Err(UnexpectedRollback)`; `stored.slot != slot` →
   `Err(RollbackPointSlotMismatch{…})`. `target = Point{ slot: stored.slot, hash }` (DC-NODE-29).
4. `accumulator_admit_and_clear_for_rollback(epoch_accumulator, chaindb, &target,
   &RecoveryAdmissionPolicy{ security_param })` (k-guard via `admit_rollback`).
5. `*pending_reselection = true;` build `ChainEvent::RolledBack{ to_point: target, depth: BlockDistance(0) }`;
   `apply_chain_event(…)`; `*pending_reselection = false;` then `?` the applied result (fence cleared on
   both success and failure — DC-NODE-28).

`run_participant_sync`'s arm collapses to a single call (it already owns `security_param`,
`pending_reselection`, `ledger_view` — zero new state), keeping its existing behavior byte-for-byte.

### 2. `run_node_sync` — thread two args + route the rollback arm

Add to the signature (`node_sync.rs:595`): `security_param: SecurityParam` and
`pending_reselection: Option<&mut bool>` (Option: a keyless follower has no forge to fence).
Add `use ade_core::consensus::events::SecurityParam;`.

Replace the arm at `node_sync.rs:634-641` with:

```rust
NodeSyncItem::RollBack { point, .. } => {
    let view: &dyn LedgerView = match authority.as_deref() {
        Some(auth) => auth.ledger_view(),
        None => ledger_view.ok_or_else(|| NodeSyncError::Pump("rollback: no ledger view".into()))?,
    };
    // INV-FH-4: follow only within-current-epoch rollbacks; a target below the promoted
    // authority's epoch-start slot fails closed (cross-boundary authority-rewind deferred).
    if let (NodeSyncItem::RollBack { point: Point::Block { slot, .. }, .. }, Some(auth)) = (&item, authority.as_deref()) {
        let epoch_start = era_schedule.epoch_start_slot(auth.epoch());   // helper per era_schedule API
        if slot.0 < epoch_start { return Err(NodeSyncError::UnexpectedRollback); }
    }
    let mut scratch = false;
    let fence = pending_reselection.as_deref_mut().unwrap_or(&mut scratch);
    crate::node_lifecycle::resolve_and_apply_peer_rollback(
        state, chaindb, wal, &*era_schedule, view,
        epoch_accumulator, security_param, point, fence,
    )?;
    continue;
}
```

(The recovered-anchor no-op currently at `node_sync.rs:635-638` is subsumed by the helper's identical
DC-NODE-33 step — delete it.) Borrows are clean: `authority` is only *immutably* borrowed for the view
and the epoch guard; `state`/`wal` are the distinct `&mut`s the helper needs.

### 3. Callers (2 sites)

- **Test shim** `run_node_sync_no_eview` (`node_sync.rs:485-519`): pass `SecurityParam(2160)`, `None`.
- **Production** — the forge branch of the SyncOnce dispatch (`node_lifecycle.rs:2832-2846`): read
  `act = forge.as_deref_mut()`; pass `act.security_param` (`ForgeActivation.security_param`,
  `node_lifecycle.rs:1553`) and `Some(&mut act.pending_reselection)` (`node_lifecycle.rs:1545`); when
  `forge` is `None`, pass `RecoveryAdmissionPolicy::cardano().security_param` and `None`.

## Acceptance (CE-FH-1..3)

Mirror the 4 participant tests (`crates/ade_node/tests/live_fork_choice_ai_s4bii.rs:138-260`, helpers
`db_with_fork_and_snapshot` / `rollback_item` / `fwd_at`) against `run_node_sync`:
1. `forge_path_rollback_applies_durably` — within-`k` stored target → durable tip back, `WalEntry::RollBack`
   written, `pending` cleared.
2. `forge_path_rollback_to_unknown_point_fails_closed` — unknown hash → `UnexpectedRollback`, no mutation.
3. `forge_path_rollback_beyond_k_fails_closed_clears_pending` — no snapshot → `Pump(_)`, `pending` cleared.
4. `forge_path_rollback_slot_hash_mismatch_fails_before_mutation` — DC-NODE-29 `RollbackPointSlotMismatch`.
5. `forge_path_rollback_across_epoch_start_fails_closed` — INV-FH-4 within-epoch guard.

Replay-equivalence (CE-FH-2): identical `WalEntry::RollBack` via identical `apply_chain_event` ⇒ holds by
construction; add a forge-path assertion mirroring `reselection_replay_s5`. Plus `cargo test --workspace`.

## Risk

Shell-only (`node_sync.rs` + `node_lifecycle.rs`, same crate). No BLUE edit; BLUE authority called
unchanged. Fail-closed strictly widened-correct (only the routine in-store within-`k` within-epoch fork
flips to a durable follow). Biggest residual: cross-epoch-boundary rollback vs. the promoted authority —
excluded by INV-FH-4 for this cut.
