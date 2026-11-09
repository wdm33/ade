# Invariant Slice — PHASE4-N-H S3

## Slice Header

**Slice Name:** GREEN `events_to_state` adapter + `in_memory_chain_write` adapter + session-transcript replay
**Cluster:** PHASE4-N-H
**Status:** In Progress
**CEs addressed:** CE-N-H-3
**Registry flips on merge:** `DC-PROTO-09` → `enforced`
**Dependencies:** N-H-S1, N-H-S2

---

## Intent

Wire the BLUE reducer (S2) into a pipeline an orchestrator can drive:

* `ade_runtime::receive::events_to_state` — pure adapter lifting
  N-A's `ForkChoiceSignal` (chain-sync) and `BatchDeliveryEvent`
  (block-fetch) into the unified `ReceiveEvent` stream. Variants
  that aren't state-changing (BatchStarted, NoBlocks, BatchCompleted,
  Intersected, NoIntersection) return `None` and are filtered out
  by the orchestrator. No I/O, no clock.

* `ade_runtime::receive::in_memory_chain_write` — `ChainDbWrite`
  impl backed by `ade_runtime::chaindb::ChainDb`. Decodes the
  `AdmittedBlock` bytes to extract `(slot, block_hash)` and calls
  `ChainDb::put_block(&StoredBlock { slot, hash, bytes })`.
  Maps `ChainDbError` into the BLUE `ChainWriteError`.

* End-to-end session-transcript replay test: drive a synthetic
  signal+event interleaving through `events_to_state` then
  `receive_apply_sequence`; assert byte-identical
  `(ledger_fingerprint', chain_dep', chaindb_tip')` across two runs.

---

## The change

### 1. New `crates/ade_runtime/src/receive/mod.rs` + submodules

```
receive/
  mod.rs
  events_to_state.rs    // ForkChoiceSignal | BatchDeliveryEvent -> ReceiveEvent
  in_memory_chain_write.rs   // ChainDbWrite impl over ChainDb
```

Adapter shape:

```rust
pub fn lift_chain_sync_signal(sig: ForkChoiceSignal) -> Option<ReceiveEvent>;
pub fn lift_block_fetch_event(ev: BatchDeliveryEvent) -> Option<ReceiveEvent>;
```

In-memory chain write:

```rust
pub struct ChainDbWriter<'a, D: ChainDb> { db: &'a D }
impl<'a, D: ChainDb> ChainDbWrite for ChainDbWriter<'a, D> { ... }
```

### 2. End-to-end replay test
`crates/ade_runtime/tests/receive_session_transcript_replay.rs`

### 3. CI gate `ci/ci_check_receive_replay_purity.sh`

* No `wall-clock`/`tokio`/`rand`/`HashMap` in `events_to_state.rs`
  or `in_memory_chain_write.rs` production code.
* Positive grep: `lift_chain_sync_signal` + `lift_block_fetch_event`
  exist; `ChainDbWriter` impls `ChainDbWrite`.

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_runtime/src/receive/events_to_state.rs`:
- `lift_chain_sync_signal_roll_forward_yields_receive_event` —
  carries tip's (slot, hash) into ReceiveEvent::RollForward.
- `lift_chain_sync_signal_roll_backward_yields_receive_event`.
- `lift_chain_sync_signal_intersected_yields_none`.
- `lift_chain_sync_signal_no_intersection_yields_none`.
- `lift_block_fetch_event_block_delivered_yields_receive_event`.
- `lift_block_fetch_event_batch_started_yields_none`.
- `lift_block_fetch_event_no_blocks_yields_none`.
- `lift_block_fetch_event_batch_completed_yields_none`.

In `crates/ade_runtime/src/receive/in_memory_chain_write.rs`:
- `in_memory_chain_write_admits_via_admitted_block_to_chaindb`.
- `in_memory_chain_write_recovers_slot_hash_from_bytes`.

In `crates/ade_runtime/tests/receive_session_transcript_replay.rs`:
- `receive_session_transcript_replay_byte_identical` — drives
  RollForward → BlockDelivered for a Conway-576 corpus block twice;
  asserts identical ledger fingerprint + identical ChainDb tip.

CI: `ci/ci_check_receive_replay_purity.sh` (new).

---

## §14 Hard Prohibitions

- No I/O, clock, or random in the adapter modules.
- `lift_chain_sync_signal` MUST NOT decode `header_bytes` (opaque
  pass-through; the cache key is derived from the tip).
- `lift_block_fetch_event` MUST NOT decode `block_bytes` (the
  reducer decodes on the BlockDelivered branch).
- `ChainDbWriter` MUST decode the block bytes to extract
  `(slot, hash)` for the `StoredBlock` key.

---

## §15 Explicit Non-Goals

- RED orchestrator (S4); mechanical adapter (S5); live evidence (S6).

---

## Replay obligations

`receive_session_transcript_replay_byte_identical` closes
DC-PROTO-09 at the pipeline level (adapter + reducer +
in-memory ChainDb).

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
