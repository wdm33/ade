# Invariant Slice — PHASE4-N-I S4

## Slice Header

**Slice Name:** GREEN snapshot cadence + `InMemorySnapshotCache` + `ChainDbBlockSource`
**Cluster:** PHASE4-N-I
**Status:** In Progress
**CEs addressed:** CE-N-I-4
**Registry flips on merge:** `DC-STORE-07` → `enforced`
**Dependencies:** N-I-S1, N-I-S2

---

## Intent

Three GREEN adapters wire the BLUE rollback driver to runtime
infrastructure:

* `cadence::should_snapshot_after_block(slot, block_no, cadence,
  last_snapshot)` — pure decision function.
  `SnapshotCadence { every_n_blocks: u32 }` is BLUE-structural;
  default 100. No operator-tunable runtime input.
* `in_memory_cache::InMemorySnapshotCache` — `BTreeMap<SlotNo,
  (LedgerState, PraosChainDepState)>` impl of `SnapshotReader`.
* `chaindb_block_source::ChainDbBlockSource<'a, D: ChainDb>` —
  impl of `BlockSource` over any ChainDb.

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_runtime/src/rollback/cadence.rs`:
- `should_snapshot_after_block_every_n_returns_true_at_cadence`
- `should_snapshot_after_block_returns_false_off_cadence`
- `should_snapshot_after_block_returns_false_when_already_at_or_after_slot`
- `should_snapshot_after_block_is_pure`
- `snapshot_cadence_default_is_100_blocks`

In `crates/ade_runtime/src/rollback/in_memory_cache.rs`:
- `in_memory_snapshot_cache_nearest_le_returns_largest_key`
- `in_memory_snapshot_cache_iteration_is_btreemap_ordered`
- `in_memory_snapshot_cache_empty_returns_none`
- `in_memory_snapshot_cache_oldest_returns_smallest_slot`

In `crates/ade_runtime/src/rollback/chaindb_block_source.rs`:
- `chaindb_block_source_inclusive_upper_exclusive_lower`
- `chaindb_block_source_empty_when_no_blocks`
- `chaindb_block_source_returns_bytes_byte_identical`

CI: `ci/ci_check_snapshot_cadence_purity.sh` (new).

---

## §14 Hard Prohibitions

- `SnapshotCadence` may have exactly one field
  (`every_n_blocks`). No operator-tunable runtime input.
- No HashMap/HashSet/wall-clock/tokio/rand in any of the three
  modules.

---

## §15 Explicit Non-Goals

- RED orchestrator snapshot-write hook (S5).
- Receive reducer RollBackward branch update (S6).
- Persistent on-disk snapshot encoding (follow-on cluster
  per DC-CONS-21).

---

## Replay obligations

`should_snapshot_after_block_is_pure` proves DC-STORE-07's pure-
function shape over replayed inputs.

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
