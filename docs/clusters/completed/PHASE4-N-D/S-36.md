# Slice S-36 Entry Obligation Discharge

> **Status:** Discharged. S-36 implementation may begin.
>
> **Authority Level:** Slice-entry proof discharge (per Phase 4 cluster
> plan §"Cluster N-D — Chain DB & persistence").
>
> **Cluster:** N-D, slice 4. Tier 1 for the recovery contract; Tier 5
> for sequencing strategy.
>
> **Predecessors:** S-33 (trait + in-memory) ✓, S-34 (redb-backed
> persistent impl) ✓, S-35 (SnapshotStore) ✓.

This slice ships the recovery primitive: snapshot + forward-replay.
Combines the existing chaindb pieces — `ChainDb::iter_from_slot` and
`SnapshotStore::latest_snapshot` — into a single generic function
that recovers a state-at-tip from a (snapshot, blocks) pair. The
slice deliberately stays decoupled from `ade_ledger`: the recovery
function is generic over a `Recoverable` trait that callers
implement.

---

## Slice scope

**In:**
- `ade_runtime::recovery::Recoverable` trait — declares "I know how
  to decode myself from snapshot bytes and apply a block."
- `ade_runtime::recovery::recover` function — the generic recovery
  entry point.
- `RecoveryReport<R>` — what got recovered (starting state, starting
  slot, blocks replayed, ending state).
- `RecoveryError<E>` — failure modes (no snapshot AND no genesis,
  decoder failure, missing block, applier failure).
- Contract test suite using a fake `Recoverable` impl.

**Out:**
- Any `Recoverable` impl on `ade_ledger::LedgerState` (caller-side;
  added when consensus runtime needs it).
- Concurrent / parallel replay (single-thread only this slice).
- Progress reporting / cancellation (callers can wrap if needed).
- Snapshot rotation triggered by recovery (S-35 already covers
  manual snapshot management; recovery is read-only).
- Bootstrap-from-network (Cluster N-A; recovery here is local-only).

---

## O-36.1 — Where does recovery live? (architectural)

**Obligation:** Recovery uses both chaindb (`ade_runtime`) and the
ledger state machine (`ade_ledger`). Which crate owns the recovery
logic?

### Answer

**`ade_runtime::recovery`, generic over a `Recoverable` trait.**

Rationale:
1. `ade_runtime` is the imperative shell — driving stores and state
   machines is its job.
2. Generic over `Recoverable` means **no `ade_ledger` dependency
   from `ade_runtime`**. Architecture stays clean:
   `ade_node` → `ade_runtime` and `ade_node` → `ade_ledger`, with
   `ade_node` (or test code) implementing `Recoverable`.
3. The pattern composes naturally — anyone with a state that can
   be snapshotted and forward-applied can use the same recovery
   primitive (e.g., a stub for tests, a partial-state harness, a
   future light-client mode).

Anti-pattern rejected: putting recovery inside `ade_ledger`. That
would require `ade_ledger` to depend on `ade_runtime` for the
storage interface, inverting the architecture. The pure ledger
must not know about I/O.

---

## O-36.2 — Recovery function shape

**Obligation:** What signature does the recovery function take? What
does it return?

### Answer

```rust
pub trait Recoverable: Sized {
    /// Error type produced by both decode and apply. One type so
    /// the recovery error stays simple; callers that need to
    /// distinguish wrap / discriminate at the impl level.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Decode snapshot bytes (produced earlier by the caller's own
    /// canonical encoder) back into the state.
    fn decode_snapshot(bytes: &[u8]) -> Result<Self, Self::Error>;

    /// Apply one block to the current state. Consumes self and
    /// returns the new state, mirroring the pure-ledger style.
    fn apply_block(self, block_bytes: &[u8]) -> Result<Self, Self::Error>;
}

pub struct RecoveryReport<R> {
    pub starting_state: StartingState,
    pub blocks_replayed: u64,
    pub ending_state: R,
    pub ending_slot: Option<SlotNo>,
}

pub enum StartingState {
    Snapshot { slot: SlotNo },
    Genesis,
}

#[derive(Debug)]
pub enum RecoveryError<E> {
    /// No snapshot available AND no genesis state was provided.
    NoStartingPoint,
    /// Snapshot decoder rejected the stored bytes.
    SnapshotDecodeFailed(E),
    /// `apply_block` rejected a block during replay.
    ApplyBlockFailed { slot: SlotNo, source: E },
    /// Underlying chaindb/snapshot-store error.
    Storage(ChainDbError),
}

pub fn recover<C, S, R>(
    chaindb: &C,
    snapshots: &S,
    genesis: Option<R>,
) -> Result<RecoveryReport<R>, RecoveryError<R::Error>>
where
    C: ChainDb,
    S: SnapshotStore,
    R: Recoverable;
```

**Why `genesis: Option<R>`** rather than always required:
- Production deployments may have a snapshot and never need the
  genesis fallback. Forcing them to construct one is friction.
- Test deployments may want to validate the no-snapshot path
  without having to fabricate a snapshot. A `None` genesis tests
  the `NoStartingPoint` error path.
- Operator deployments that explicitly don't carry a genesis (e.g.,
  a node that's required to start from a snapshot) get a clear
  `NoStartingPoint` error if their state is missing.

**Why consume + return `R`** instead of `&mut R`:
- Mirrors `ade_ledger::apply_block`'s functional style.
- Avoids interior-mutability pressure on the state type.
- Cost is negligible: states are passed by move, not cloned.

---

## O-36.3 — Snapshot decoder responsibility

**Obligation:** Who decodes the snapshot bytes — chaindb, recovery,
or the `Recoverable` impl?

### Answer

**The `Recoverable` impl.** Same separation as snapshot encoding in
S-35 — bytes are opaque at the storage layer. The state type knows
its own canonical encoding; storage doesn't.

This means `Recoverable::decode_snapshot` is the inverse of whatever
encoder the caller used to produce the snapshot. The recovery
function doesn't care about format — it just hands bytes to the
trait method.

Operational consequence: if the encoder version drifts from the
decoder version (e.g., binary upgrade after snapshot was written),
the trait impl is responsible for catching it. Recovery surfaces
the error as `RecoveryError::SnapshotDecodeFailed(impl_error)`.

---

## O-36.4 — Failure modes & semantics

**Obligation:** What does recovery do when something goes wrong
mid-replay? Roll back? Stop with partial state? Report and continue?

### Answer

**Stop and surface the error. No partial recovery.**

Rationale:
- Mid-replay errors mean either corrupt storage, a binary mismatch,
  or a buggy applier. None of those are conditions where "continue
  with partial state" is safe — the resulting ledger state is
  meaningless.
- Recovery is read-only against the chaindb (no rollbacks, no
  pruning). The chaindb is left as-is on failure; the operator can
  retry after fixing root cause.
- Failure carries enough context to diagnose: slot of the failing
  block + the underlying error.

Cancellation / interruption is **not** in scope for S-36. The
recovery function runs to completion or returns an error. Callers
who want cancellation wrap with their own runtime (tokio task +
abort handle, or similar). The function is sync-only this slice.

---

## O-36.5 — Empty chaindb / no-snapshot semantics

**Obligation:** What happens when:
1. No snapshot exists AND no genesis provided?
2. Snapshot exists but no blocks past it?
3. No snapshot, but genesis provided AND blocks exist starting at
   some slot?

### Answer

| Situation | Behavior |
|---|---|
| 1: no snapshot, no genesis | `Err(NoStartingPoint)`. Operator must either provide genesis or restore a snapshot. |
| 2: snapshot present, no blocks past it | `Ok` with `starting_state = Snapshot { slot }`, `blocks_replayed = 0`, `ending_state = decoded snapshot`, `ending_slot = Some(slot)`. The snapshot itself is the recovered tip. |
| 3: no snapshot, genesis present, blocks exist | `Ok` with `starting_state = Genesis`, replay starts at `iter_from_slot(0)` (full genesis-replay path). The cluster plan flags this as operator-explicit fallback; this slice doesn't reject it but doesn't optimize for it either. |
| 4: snapshot at slot S, blocks at slots ≤ S | Blocks at `≤ S` are skipped. Replay starts at `iter_from_slot(S+1)`. |
| 5: snapshot at slot S, gap in blocks (say S+5 missing) | The gap is benign — Ouroboros allows empty slots. Replay applies whatever blocks the iter yields, in slot order. |

The "iter from S+1" is implemented via `chaindb.iter_from_slot(SlotNo(S+1))`.

---

## O-36.6 — Acceptance gate

1. `cargo build --workspace` clean.
2. `cargo test -p ade_runtime` green:
   - All 8 existing tests still pass.
   - New `recover_from_snapshot_and_replay_forward` ✓
   - New `recover_from_genesis_when_no_snapshot` ✓
   - New `no_starting_point_error` ✓
   - New `decode_failure_surfaces_as_error` ✓
   - New `apply_failure_surfaces_with_slot` ✓
3. `cargo clippy -p ade_runtime --all-targets` clean.
4. Tier isolation: still no `redb` outside `persistent.rs`. **No
   `ade_ledger` import in `ade_runtime`.** Latter check is mechanical:
   `rg "ade_ledger" crates/ade_runtime/` returns nothing.

---

## Forbidden patterns for S-36

- **No `ade_ledger` dependency.** `ade_runtime` stays decoupled.
- **No partial-recovery success.** Mid-replay failure aborts; no
  "best-effort" mode.
- **No async recovery.** Sync only; callers wrap if they need
  cancellation.
- **No automatic snapshot pruning** during recovery. Read-only
  against the snapshot store.
- **No genesis-replay optimization** (precompiled state, parallel
  application). Out of scope; the slice ships correctness, not
  performance.

---

## Out of scope (explicitly)

- 1,000-kill-9 stress harness (S-37).
- `Recoverable` impl on `ade_ledger::LedgerState` (caller-side, added
  when consensus runtime starts using it).
- Concurrent block application.
- Progress reporting / cancellation.
- Bootstrap-from-network (Cluster N-A).
- Light-client / SPV recovery (Phase 6+).

---

## Authority Reminder

This discharge is a planning artifact. Authority for the trait surface
belongs to the published `ade_runtime::recovery::*` API once S-36
ships.
