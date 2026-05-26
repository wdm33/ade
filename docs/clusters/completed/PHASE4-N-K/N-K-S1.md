# Invariant Slice — PHASE4-N-K S1

## Slice Header

**Slice Name:** `Clock` trait + `ade_runtime::bootstrap` single
authority (cold-start + warm-start)
**Cluster:** PHASE4-N-K
**Status:** In Progress
**CEs addressed:** CE-N-K-1 (CN-NODE-01).
**Registry effects on merge:** CN-NODE-01 → `enforced` with
`code_locus = "crates/ade_runtime/src/bootstrap.rs"`,
`tests = [...]`, `ci_script = "ci/ci_check_bootstrap_closure.sh"`,
`strengthened_in = ["PHASE4-N-K"]`.
**Dependencies:** N-D (`SnapshotStore`, `PersistentChainDb`), N-I
(`SnapshotReader`, `BlockSource`), N-J
(`PersistentSnapshotCache::nearest_le`), `materialize_rolled_back_state`
(N-I S2), `genesis_parser`.

---

## Intent

Make the initial node state a single-authority composition: one
`pub fn` in `ade_runtime::bootstrap` returning
`(LedgerState, PraosChainDepState, ChainTip)`. Cold-start and
warm-start are two branches of the same function — no parallel
paths. Adds the `Clock` seam needed by S2+.

---

## Scope

- **New modules:**
  - `crates/ade_runtime/src/clock.rs` — `Clock` trait,
    `DeterministicClock` (BLUE-eligible; GREEN by file location),
    `SystemClock` (RED sub-classified).
  - `crates/ade_runtime/src/bootstrap.rs` — the single `pub fn`
    `bootstrap_initial_state` plus a closed `BootstrapError` sum
    plus a `BootstrapInputs` parameter struct.
- **State machines affected:** none.
- **Persistence impact:** none (reads only).
- **Network-visible impact:** none.

Out-of-scope (S7): mounting bootstrap inside `tokio::main`;
operator config loading.

---

## Execution Boundary

- **BLUE:** none (this slice).
- **GREEN:**
  - `ade_runtime::bootstrap::bootstrap_initial_state`,
    `BootstrapInputs`, `BootstrapError`, helpers.
  - `ade_runtime::clock::{Clock, DeterministicClock}`.
- **RED:** `ade_runtime::clock::SystemClock`.

---

## Invariants Preserved

- CN-CONS-08 (sole admit authority) — not invoked by this slice.
- CN-STORE-07 (sole materialize authority) — invoked by warm-start
  branch via `materialize_rolled_back_state`.
- CN-STORE-08 (sole snapshot byte authority) — invoked indirectly
  via `PersistentSnapshotCache::nearest_le`.
- DC-CONS-21 — round-trip equivalence at bootstrap on warm-start.
- T-DET-01 — bootstrap output is determined by inputs (no clock,
  no rand, no network read).

## Invariants Strengthened or Introduced

- CN-NODE-01 (this slice introduces).
- T-DET-01 strengthened: deterministic initial state under
  type-level constraint.

---

## Design Summary

```rust
pub trait Clock: Send + Sync {
    fn now_millis(&self) -> u64;
    fn next_tick(&mut self) -> Option<u64>; // returns next slot's millis, None on shutdown
}

pub struct DeterministicClock { ticks: Vec<u64>, cursor: usize, anchor_millis: u64 }
pub struct SystemClock; // RED; not callable from BLUE/GREEN.

pub struct BootstrapInputs<'a, D, S>
where
    D: ChainDb,
    S: SnapshotStore,
{
    pub chaindb: &'a D,
    pub snapshot_store: &'a S,
    pub era_schedule: &'a EraSchedule,
    pub ledger_view: &'a dyn LedgerView,
    pub genesis_initial: Option<(LedgerState, PraosChainDepState)>,
}

pub enum BootstrapError {
    SnapshotDecode(SnapshotDecodeError),
    Materialize(MaterializeError),
    ChainDb(ChainDbError),
    GenesisRequiredButAbsent,
    InconsistentTipVsSnapshot { snapshot_slot: SlotNo, chain_tip_slot: SlotNo },
}

pub fn bootstrap_initial_state<D, S>(
    inputs: BootstrapInputs<'_, D, S>,
) -> Result<(LedgerState, PraosChainDepState, Option<ChainTip>), BootstrapError>
```

Branch selection (one function, two cases):

1. `chaindb.tip()` → `None` AND `snapshot_store.list_snapshot_slots()` empty
   → **cold-start**: return `genesis_initial` (must be `Some`).
2. Otherwise → **warm-start**:
   - `PersistentSnapshotCache::nearest_le(chain_tip.slot)` →
     snapshot must exist; if not, error.
   - `materialize_rolled_back_state(target=chain_tip, reader,
     source, era_schedule, ledger_view)` → produces `(ledger,
     chain_dep)`.
   - Return `(ledger, chain_dep, Some(chain_tip))`.

`BlockSource` is a thin adapter over `chaindb.iter_from_slot`.

---

## Replay, Crash, and Epoch Validation

- **Replay tests added:**
  - `bootstrap_cold_start_returns_genesis_when_empty` — empty
    chaindb + empty snapshot_store + `Some(genesis)` →
    deterministic `(genesis_ledger, genesis_chain_dep, None)`.
  - `bootstrap_cold_start_without_genesis_errors` — empty stores
    + `None` → `GenesisRequiredButAbsent`.
  - `bootstrap_warm_start_materializes_from_persistent_snapshot`
    — seed snapshot at slot N + blocks N+1..M into chaindb;
    bootstrap returns same `(ledger, chain_dep)` as a direct
    `materialize_rolled_back_state(target=M)` call (single-
    authority equivalence).
  - `bootstrap_two_runs_produce_byte_identical_state` — fingerprint
    of returned LedgerState is identical across two bootstraps
    over the same `(chaindb, snapshot_store)`.
- **Crash/restart:** the warm-start path is itself the
  restart-resume path; tested by S7's shutdown-resume integration.
- **Epoch boundary:** not relevant; bootstrap is point-in-time.

## §12 Mechanical Acceptance Criteria

- [ ] `bootstrap_cold_start_returns_genesis_when_empty`
- [ ] `bootstrap_cold_start_without_genesis_errors`
- [ ] `bootstrap_warm_start_materializes_from_persistent_snapshot`
- [ ] `bootstrap_warm_start_equals_direct_materialize` (byte-
  identical fingerprint vs direct call to materialize authority)
- [ ] `bootstrap_two_runs_produce_byte_identical_state`
- [ ] `deterministic_clock_is_pure` — `DeterministicClock` yields
  identical tick sequences across two iterations.
- [ ] `ci/ci_check_bootstrap_closure.sh` — single `pub fn`
  returning `(LedgerState, PraosChainDepState, Option<ChainTip>)`
  in the bootstrap source; no HashMap / tokio / wall-clock /
  rand in the file.
- [ ] `ci/ci_check_clock_seam.sh` (partial — full gate in S8) —
  `crates/ade_runtime/src/clock.rs` is the sole site of
  `SystemTime::now()` / `Instant::now()` within `ade_runtime`.

---

## Failure Modes

- `BootstrapError::SnapshotDecode` — surface the underlying
  `SnapshotDecodeError` (UnknownVersion, FingerprintMismatch,
  …); fail-fast at the bootstrap boundary. Authority-fatal.
- `BootstrapError::Materialize(MaterializeError)` — fail-fast.
  Replay-affecting; mismatch between snapshot store and chaindb.
- `BootstrapError::GenesisRequiredButAbsent` — cold-start without
  genesis input.
- `BootstrapError::InconsistentTipVsSnapshot` — chaindb tip
  exists but no snapshot ≤ tip.

All variants are deterministic and fail-fast.

---

## §14 Hard Prohibitions

- No HashMap/HashSet in `bootstrap.rs` or `clock.rs`.
- No `tokio::*` import in `bootstrap.rs`. (Tokio touch lives in
  `SystemClock` and S5+ runner files.)
- No `SystemTime::now()` / `Instant::now()` outside the
  `SystemClock` block in `clock.rs`.
- No `rand::*` anywhere in this slice.
- No second `pub fn` in `bootstrap.rs` returning the initial
  triple.
- No reimplementation of `materialize_rolled_back_state` —
  bootstrap calls it.

## §15 Explicit Non-Goals

- No tokio runtime wiring (S5+).
- No CLI flag parsing or operator config loading (S7).
- No production cardano-node snapshot byte compatibility
  (Tier 5 — Ade's snapshot format is project-internal).
- No snapshot eviction (out of cluster).
- No schema-migration tooling (DC-STORE-09 open obligation).

---

## §16 Completion Checklist

- [ ] All §12 tests added and passing.
- [ ] `ci_check_bootstrap_closure.sh` added and passing.
- [ ] No new warnings under `cargo clippy --workspace`.
- [ ] Registry CN-NODE-01 flipped to `enforced` with locus + test
  list + ci_script populated.
