# Invariant Slice — PHASE4-N-I S5

## Slice Header

**Slice Name:** RED snapshot-write hook `maybe_capture_snapshot`
**Cluster:** PHASE4-N-I
**Status:** In Progress
**CEs addressed:** CE-N-I-5 (integration foundation for S6)
**Registry effects on merge:** none directly; foundational for S6.
**Dependencies:** N-I-S1..S4

---

## Intent

The hook the orchestrator calls after each successful admission to
capture a snapshot at cadence. Pure decision + in-memory write —
no I/O, no clock. Cache is per-caller; caller can use one per peer
or share one across peers.

Signature:

```rust
pub fn maybe_capture_snapshot(
    cache: &mut InMemorySnapshotCache,
    cadence: SnapshotCadence,
    effect: &ReceiveEffect,
    state: &ReceiveState,
) -> bool;
```

Returns `true` iff a snapshot was captured. Skips when:
* `effect` is not `Admitted`.
* `state.chain_dep.last_block_no` is `None` (initial state).
* Cadence policy says off-cadence.
* `cache.most_recent()` ≥ effect slot (idempotency / late event).

Also adds `InMemorySnapshotCache::most_recent` /
`InMemorySnapshotCache::slots` / `iter_for_test` accessors to
support cadence seeding + test inspection.

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_runtime/src/rollback/snapshot_writer.rs`:
- `maybe_capture_snapshot_captures_at_cadence`.
- `maybe_capture_snapshot_skips_off_cadence`.
- `maybe_capture_snapshot_only_on_admitted_effect`.
- `maybe_capture_snapshot_deterministic_over_admission_sequence` —
  50 admissions at cadence 5 → 10 captures; same captures across
  two runs.

---

## §14 Hard Prohibitions

- No I/O, clock, randomness, HashMap.
- No state mutation on non-Admitted effects.

---

## §15 Explicit Non-Goals

- Wire the hook into the actual receive orchestrator (S5
  signature stays standalone; the caller — typically a future
  node binary or the S6 integration test — invokes it after each
  dispatch). The orchestrator's dispatch functions are unchanged.
- Receive reducer RollBackward branch update (S6).

---

## Replay obligations

`maybe_capture_snapshot_deterministic_over_admission_sequence`
proves the hook is replay-deterministic.

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
