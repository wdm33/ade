# Invariant Slice — PHASE4-N-I S1

## Slice Header

**Slice Name:** `SnapshotReader` + `BlockSource` narrow read-only traits + `MaterializeError` + `CommitRollbackError` closed sums
**Cluster:** PHASE4-N-I
**Status:** In Progress
**CEs addressed:** CE-N-I-1 (foundation for CN-STORE-07; both flip at S2)
**Registry effects on merge:** none directly; sets up trait surface S2 + S3 build on.
**Dependencies:** PHASE4-N-H (ChainDb, ReceiveState).

---

## Intent

Stand up the BLUE skeleton the materialize driver and commit
helper compose against. Both traits are minimal — single method
each — and read-only. Both error sums are closed (no String, no
non_exhaustive). This is the equivalent of N-H S1 for the
rollback path.

---

## The change

### 1. New `crates/ade_ledger/src/rollback/mod.rs` + submodules

```
rollback/
  mod.rs                 // pub use re-exports
  traits.rs              // SnapshotReader + BlockSource
  error.rs               // MaterializeError + CommitRollbackError
```

### 2. Trait shapes

```rust
pub trait SnapshotReader {
    /// Largest snapshot key ≤ target_slot, or None.
    fn nearest_le(&self, target_slot: SlotNo)
        -> Option<(SlotNo, LedgerState, PraosChainDepState)>;
}

pub trait BlockSource {
    /// Ordered iterator of `(slot, block_bytes)` for slots strictly
    /// greater than `from_exclusive` and ≤ `to_inclusive`.
    fn blocks_in_range(
        &self,
        from_exclusive: SlotNo,
        to_inclusive: SlotNo,
    ) -> Vec<(SlotNo, Vec<u8>)>;
}
```

`SnapshotReader::nearest_le` returns owned values (clones from the
in-memory cache) because materialize needs to mutate the state via
`apply_block_with_verdicts`. `BlockSource::blocks_in_range` returns
owned bytes for the same reason — the materialize driver consumes
them. Both methods are pure (no I/O at the trait level; impls may
touch storage).

### 3. Error sums

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum MaterializeError {
    RollbackTooDeep {
        target_slot: SlotNo,
        oldest_snapshot: Option<SlotNo>,
    },
    ReplayFailedAt {
        slot: SlotNo,
        error: BlockValidityError,
    },
    EraNotSupported {
        era: CardanoEra,
        slot: SlotNo,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitRollbackError {
    ChainDb(ChainWriteError),
}
```

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_ledger/src/rollback/traits.rs`:
- `snapshot_reader_trait_is_object_safe` — `&dyn SnapshotReader`
  compiles.
- `block_source_trait_is_object_safe` — `&dyn BlockSource` compiles.

In `crates/ade_ledger/src/rollback/error.rs`:
- `materialize_error_round_trips_through_pattern_match` —
  exhaustive match over all 3 variants.
- `commit_rollback_error_round_trips_through_pattern_match` —
  exhaustive match over all variants.

---

## §14 Hard Prohibitions

- No `HashMap` / `HashSet` / wall-clock / tokio / rand in either
  module.
- No `String` variant in either error sum.
- No `#[non_exhaustive]` on either sum.
- No `pub trait` extension beyond the single read method per
  trait.

---

## §15 Explicit Non-Goals

- Materialize driver (S2).
- Commit helper (S3).
- GREEN cadence + cache (S4).
- Receive orchestrator extension (S5).
- Reducer RollBackward branch update (S6).

---

## Replay obligations

None added at S1; traits + error sums are skeletons.

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
