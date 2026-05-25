# Invariant Slice — PHASE4-N-I S2

## Slice Header

**Slice Name:** `materialize_rolled_back_state` driver — pure replay-forward fold
**Cluster:** PHASE4-N-I
**Status:** In Progress
**CEs addressed:** CE-N-I-2
**Registry flips on merge:** `CN-STORE-07` + `DC-CONS-22` → `enforced`
**Dependencies:** N-I-S1

---

## Intent

The single canonical materialize authority. Given `(target, &reader,
&source, era_schedule, ledger_view)`:
1. `reader.nearest_le(target.slot)` → snapshot or `RollbackTooDeep`.
2. If snapshot slot == target slot → return snapshot state.
3. Else: iterate `source.blocks_in_range(snapshot_slot,
   target.slot)`; for each block, decode envelope for era +
   detect pre-Conway (→ `EraNotSupported`); else call
   `block_validity` (the same validator the receive admit branch
   uses — CN-CONS-08); on `Invalid` → `ReplayFailedAt`; on
   `Valid` → advance ledger + chain_dep.
4. Return final `(LedgerState, PraosChainDepState)`.

Snapshot is a pure cache for direct-apply; replay-forward yields
the same state direct-apply would have. CN-STORE-07 + DC-CONS-22
close together at this slice.

---

## The change

### 1. New `ade_ledger::rollback::materialize`

```rust
pub struct TargetPoint {
    pub slot: SlotNo,
    pub hash: Hash32,                       // recorded but not enforced at S2
}

pub fn materialize_rolled_back_state(
    target: TargetPoint,
    reader: &dyn SnapshotReader,
    source: &dyn BlockSource,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<(LedgerState, PraosChainDepState), MaterializeError>;
```

### 2. CI gate `ci/ci_check_rollback_materialize_closure.sh`

- No `HashMap`/`HashSet`/wall-clock/tokio/rand in
  `rollback/materialize.rs` production code.
- No other `pub fn` returning `(LedgerState,
  PraosChainDepState)` from `rollback/*` (CN-STORE-07 single-
  authority).
- Positive grep for `block_validity` call site.

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_ledger/src/rollback/materialize.rs`:

- `materialize_returns_rollback_too_deep_when_no_snapshot` —
  empty reader → `RollbackTooDeep { target_slot, oldest_snapshot:
  None }`.
- `materialize_with_snapshot_at_target_returns_snapshot_state` —
  degenerate case: snapshot_slot == target.slot.
- `materialize_with_snapshot_below_target_replays_forward` —
  snapshot at S, target at T > S, replay-forward applies blocks.
- `materialize_fails_closed_on_invalid_block` — synthetic
  corrupted-body block in the source → `ReplayFailedAt`.
- `materialize_replay_forward_equals_direct_apply` — CORE
  PROOF for DC-CONS-22: direct-apply S→T equals snapshot@S +
  replay-forward S→T (by fingerprint).

CI: `ci/ci_check_rollback_materialize_closure.sh` (new).

---

## §14 Hard Prohibitions

- No `HashMap`/`HashSet`/wall-clock/tokio/rand in production code.
- No other `pub fn` returning `(LedgerState, PraosChainDepState)`
  in `rollback/*` (single-authority).
- No skip of validity check on a block — every block in
  the range goes through `block_validity`.

---

## §15 Explicit Non-Goals

- Hash equality enforcement at target (deferred to S6 integration
  test if needed — the materialize driver replays up to slot;
  caller verifies hash if required).
- Commit (S3); GREEN cadence/cache (S4); orchestrator (S5);
  reducer wire-up (S6).

---

## Replay obligations

`materialize_replay_forward_equals_direct_apply` is the
DC-CONS-22 closure: snapshot is a pure cache, not authority.

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
