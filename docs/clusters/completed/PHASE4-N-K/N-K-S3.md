# Invariant Slice — PHASE4-N-K S3

## Slice Header

**Slice Name:** GREEN `rollback::persistent_writer` — cadence-
fidelity glue to `PersistentSnapshotCache::capture`.
**Cluster:** PHASE4-N-K
**Status:** In Progress
**CEs addressed:** CE-N-K-3 (DC-NODE-02).
**Registry effects on merge:** DC-NODE-02 → `enforced` with
`code_locus = "crates/ade_runtime/src/rollback/persistent_writer.rs"`,
`tests = [...]`, `ci_script =
"ci/ci_check_persistent_writer_no_parallel_cadence.sh"`,
`strengthened_in = ["PHASE4-N-K"]`.
**Dependencies:** S2 (`OrchestratorEffect::CaptureSnapshot`),
N-I (`should_snapshot_after_block`, `SnapshotCadence`), N-J
(`PersistentSnapshotCache`).

---

## Intent

The orchestrator's only path to capturing a persistent snapshot is
through this writer. The writer's only path to deciding when to
capture is through `should_snapshot_after_block`. No alternate
cadence policy in the codebase.

---

## Scope

- `crates/ade_runtime/src/rollback/persistent_writer.rs` — new
  GREEN module.
- `crates/ade_runtime/src/rollback/mod.rs` — pub-use re-export.

State-machine impact: introduces an opaque writer struct that
holds `last_persistent_snapshot_slot: Option<SlotNo>` (mirrors the
orchestrator-state field for the writer's local trigger).

---

## Execution Boundary

- **BLUE:** none.
- **GREEN:** `rollback::persistent_writer`.
- **RED:** none.

---

## Invariants Preserved

- DC-CONS-21 — round-trip equivalence on every capture; bytes
  go through `framing::encode_snapshot` via the cache.
- CN-STORE-08 — sole encode authority; the writer calls
  `PersistentSnapshotCache::capture` which calls
  `encode_snapshot`.

## Invariants Strengthened or Introduced

- DC-NODE-02 (this slice introduces).

---

## Design Summary

```rust
pub struct PersistentSnapshotWriter<'a, S: SnapshotStore + ?Sized> {
    cache: PersistentSnapshotCache<'a, S>,
    cadence: SnapshotCadence,
    last_capture: Option<SlotNo>,
}

impl<'a, S: SnapshotStore + ?Sized> PersistentSnapshotWriter<'a, S> {
    pub fn new(store: &'a S, cadence: SnapshotCadence) -> Self;

    /// Returns Ok(true) if a snapshot was captured at this slot,
    /// Ok(false) if cadence said no.
    /// Authority-fatal errors (PersistentCacheError::Store with
    /// underlying Io) propagate; encode errors propagate
    /// (Encode(EraNotSupported) for pre-Conway is per-event-fatal
    /// but the orchestrator does not run pre-Conway).
    pub fn on_admitted(
        &mut self,
        slot: SlotNo,
        block_no: BlockNo,
        ledger: &LedgerState,
        chain_dep: &PraosChainDepState,
    ) -> Result<bool, PersistentCacheError>;

    /// Force capture (shutdown drain). Always captures regardless
    /// of cadence; used by DC-NODE-04 shutdown discipline.
    pub fn force_capture(
        &mut self,
        slot: SlotNo,
        ledger: &LedgerState,
        chain_dep: &PraosChainDepState,
    ) -> Result<(), PersistentCacheError>;

    pub fn last_captured_slot(&self) -> Option<SlotNo>;
}
```

Decision flow for `on_admitted`:
1. Consult `should_snapshot_after_block(slot, block_no, cadence,
   self.last_capture)`.
2. If `false`, return `Ok(false)`.
3. If `true`, call `cache.capture(slot, ledger, chain_dep)`. On
   success, update `self.last_capture = Some(slot)` and return
   `Ok(true)`.

`force_capture` skips the cadence check (always writes) but does
update `last_capture`.

---

## Replay, Crash, and Epoch Validation

- **Replay tests:**
  - `persistent_writer_on_admitted_captures_only_on_cadence` —
    drive 200 synthetic admissions; assert
    `(admissions_captured, captured_slots)` matches the cadence
    policy's output exactly (no over-/under-capture).
  - `persistent_writer_round_trips_via_framing` — capture once;
    read back via `PersistentSnapshotCache::nearest_le`; assert
    `(ledger, chain_dep)` byte-identical to inputs.
  - `persistent_writer_force_capture_skips_cadence_but_updates_state`
    — `on_admitted` followed by `force_capture` at the same
    slot ≥ updates `last_capture`.
  - `persistent_writer_two_runs_are_deterministic` — same
    `(seed, admissions)` → same captured-slot vector.

## §12 Mechanical Acceptance Criteria

- [ ] `persistent_writer_on_admitted_captures_only_on_cadence`
- [ ] `persistent_writer_round_trips_via_framing`
- [ ] `persistent_writer_force_capture_skips_cadence_but_updates_state`
- [ ] `persistent_writer_two_runs_are_deterministic`
- [ ] `persistent_writer_propagates_io_error_authority_fatally`
- [ ] `ci_check_persistent_writer_no_parallel_cadence.sh` — the
  only consult of cadence in `crates/ade_runtime/src/rollback/`
  outside `cadence.rs` itself is via
  `should_snapshot_after_block`. No literal modulo on
  `block_no.0` and no parallel "every_n_blocks" constant.

---

## §14 Hard Prohibitions

- No `tokio::*` imports.
- No `HashMap` / `HashSet`.
- No `unwrap()` / `expect()` / `panic!()` in non-test code.
- No alternative cadence policy: every snapshot-capture decision
  routes through `should_snapshot_after_block`.
- No direct `encode_snapshot` call — writer goes through
  `PersistentSnapshotCache::capture`.

## §15 Explicit Non-Goals

- No tokio file I/O wrapping (the store impl provides that).
- No snapshot eviction.
- No producer-side broadcast tie-in.

---

## §16 Completion Checklist

- [ ] All §12 tests added and passing.
- [ ] CI gate passes.
- [ ] Registry DC-NODE-02 flipped to `enforced`.
