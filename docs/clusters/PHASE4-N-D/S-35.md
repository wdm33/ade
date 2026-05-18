# Slice S-35 Entry Obligation Discharge

> **Status:** Discharged. S-35 implementation may begin.
>
> **Authority Level:** Slice-entry proof discharge (per Phase 4 cluster
> plan §"Cluster N-D — Chain DB & persistence").
>
> **Cluster:** N-D, slice 3. Tier 5 (intentional divergence) for
> snapshot encoding and storage layout; Tier 1 for the trait surface.
>
> **Predecessors:** S-33 (trait + in-memory) shipped; S-34 (redb-backed
> persistent impl) shipped.

This slice adds the snapshot surface to cluster N-D. Snapshots are
the basis of fast recovery (S-36) and bootstrap-from-checkpoint
flows. The slice is deliberately narrow: a separate `SnapshotStore`
trait, plus impls on `InMemoryChainDb` and `PersistentChainDb`.
No serialization policy for `LedgerState` is implied — the trait
takes opaque bytes.

---

## Slice scope

**In:**
- `ade_runtime::chaindb::SnapshotStore` trait.
- `InMemoryChainDb`: snapshot storage in a sibling `BTreeMap`.
- `PersistentChainDb`: snapshot storage in a new redb table
  `snapshots_by_slot` inside the existing chaindb file.
- Contract test suite for `SnapshotStore` reusable across impls.
- Schema bump: chaindb on-disk version 1 → 2 (new table allowed).

**Out:**
- Recovery / forward-replay using snapshots (S-36).
- Snapshot rotation / pruning policy beyond raw delete (operator
  policy, applied via `delete_snapshot`).
- LedgerState serialization (caller's responsibility; opaque bytes
  at this layer).
- Compression of snapshot bytes (caller may apply; trait is
  byte-transparent).
- Snapshot signing or HMAC (Tier 1 hash already provides integrity
  on the wire; storage integrity is redb's transaction guarantee).

---

## O-35.1 — Snapshot trait shape

**Obligation:** Should snapshots live on `ChainDb` (one fat trait) or
a separate trait? What operations belong on it?

### Answer

**Separate trait: `SnapshotStore`.** Snapshot lifecycle differs from
block storage in three ways that argue against fusion:

1. Write cadence — blocks every ~20s (mainnet); snapshots every
   epoch or every N blocks at operator's choice.
2. Read pattern — blocks are streamed sequentially during sync;
   snapshots are point lookups during recovery.
3. Optionality — a relay node that only forwards blocks doesn't
   need snapshots at all. A read-mostly query node may snapshot
   aggressively.

A separate trait makes the optionality explicit at the type level:
callers that need both types take `D: ChainDb + SnapshotStore`;
callers that only need one take just one.

```rust
pub trait SnapshotStore: Send + Sync {
    /// Insert a snapshot at `slot`. Idempotent if the same bytes
    /// were already stored at the same slot; conflicting bytes at
    /// the same slot return `InvalidOperation`.
    fn put_snapshot(
        &self,
        slot: SlotNo,
        bytes: &[u8],
    ) -> Result<(), ChainDbError>;

    /// Look up a snapshot by slot. `Ok(None)` if absent.
    fn get_snapshot(
        &self,
        slot: SlotNo,
    ) -> Result<Option<Vec<u8>>, ChainDbError>;

    /// Highest-slot snapshot, or `None` if none exist.
    fn latest_snapshot(
        &self,
    ) -> Result<Option<(SlotNo, Vec<u8>)>, ChainDbError>;

    /// All slots with stored snapshots, in ascending order.
    /// Diagnostic surface for operators.
    fn list_snapshot_slots(&self) -> Result<Vec<SlotNo>, ChainDbError>;

    /// Remove a snapshot at `slot`. `Ok(())` whether present or not.
    fn delete_snapshot(&self, slot: SlotNo) -> Result<(), ChainDbError>;
}
```

**Why no `iter_snapshots`:** snapshots are large (typical Cardano
ledger ≈ hundreds of MB serialized). Streaming all of them is
practically nonsensical; operators want one snapshot at a time.
`list_snapshot_slots` + `get_snapshot` covers the use cases.

**Why opaque `&[u8]`:** the trait deliberately knows nothing about
ledger types. The caller (consensus runtime in Phase 4 N-B)
serializes `LedgerState` using `ade_ledger::fingerprint` (or
whatever canonical encoding ships) before calling `put_snapshot`.
Keeps Tier 5 storage divergence cleanly separated from Tier 1
ledger semantics.

---

## O-35.2 — Storage location

**Obligation:** Same file as blocks, separate file, or separate
table?

### Answer

**Same redb file, separate table.** Three reasons:

1. **Atomic correlated commits.** Putting block N and snapshot at
   slot N in one transaction is an obvious operator pattern (e.g.,
   "snapshot at every epoch boundary"). Same-file means one redb
   transaction can do both — no two-phase-commit dance across
   files.
2. **Single backup target.** Aligns with the cluster N-D Tier 5
   design intent: backup/restore is single-file copy + checksum.
   A second file would either need bundled backup tooling or risk
   skew between block file and snapshot file.
3. **Smaller operational surface.** One file = one path = one
   permission set = one fsync target.

The new table name: `snapshots_by_slot`, mapping `u64` (slot) to
`&[u8]` (snapshot bytes). Sits alongside `blocks_by_slot`,
`slot_by_hash`, and `meta`.

### Schema bump

The new table is an additive change. Schema version goes from 1 to
2; old (v1) databases are migrated automatically by opening the
table the first time it's accessed (redb creates the table on
demand within a write transaction). The schema check in
`init_or_check_schema` allows reading v1 files but writes v2 magic
on next write.

**Migration semantics**:
- Open v1 file → succeeds; `schema_version` field is upgraded to 2
  on first write transaction.
- Open v2 file → succeeds.
- Open v0 / unknown → returns `SchemaMismatch`.

This is the simplest forward-compatible policy. A rollback (v2 →
v1 binary opening v2 file) returns `SchemaMismatch`. Operators
who downgrade restore from a v1 backup.

---

## O-35.3 — Snapshot identification

**Obligation:** Snapshots indexed by what? Slot, hash, checkpoint
counter, all of the above?

### Answer

**Slot only.**

Reasons:
- Slots are the natural sync coordinate. Recovery from a snapshot
  needs "the snapshot at slot S, then replay forward from S+1."
- A given slot has at most one snapshot — the operator chose to
  snapshot at that slot. Multiple snapshots-per-slot has no use
  case in this surface.
- A hash-of-snapshot-bytes index would gate features the trait
  doesn't expose (snapshot integrity verification, snapshot
  identity for distribution). Defer until a feature actually needs
  it.

**Conflict semantics:** `put_snapshot(slot, bytes)` when a snapshot
already exists at `slot`:
- Same bytes → idempotent; `Ok(())`.
- Different bytes → `Err(ChainDbError::InvalidOperation)`. Operator
  must `delete_snapshot(slot)` first.

The "same bytes" check is byte-equality, computed cheaply because
snapshot ingestion is rare relative to block ingestion. No hash
shortcut.

---

## O-35.4 — Compaction / pruning

**Obligation:** What happens to old snapshots? Does the chaindb
prune them automatically?

### Answer

**No automatic pruning. Operator decides via `delete_snapshot`.**

Rationale:
- Pruning policy is operator-specific (keep last N, keep one per
  epoch, keep all for audit, etc.). Embedding policy in the
  storage layer locks operators into one choice.
- `list_snapshot_slots` plus `delete_snapshot` is enough surface
  for any policy a caller wants.
- Snapshot rotation tooling (a higher-level utility that calls
  `list_snapshot_slots` and `delete_snapshot`) is operator-side
  scope, not Phase 4 N-D scope.

The trait surface is the storage primitive; policy lives above it.

---

## O-35.5 — Encoding

**Obligation:** What format are snapshot bytes in?

### Answer

**Opaque to the trait. Caller's choice.**

Per O-35.1, the trait sees `&[u8]`. The caller chooses the encoding.
Per the Phase 4 cluster plan: "snapshots as compact CBOR blobs at
chosen intervals, using Ade's canonical fingerprint format" — this
is a *recommendation* to callers, not a constraint imposed by the
storage layer.

This separation matters for two reasons:
- Tier 1 (the trait) stays free of ledger types. Storage works for
  any caller, including non-ledger uses (e.g., snapshot a
  governance state for offline analysis).
- Tier 5 (storage layout) doesn't constrain Tier 1 (encoding
  choice). Future encoder swaps don't bump the chaindb schema.

---

## O-35.6 — Crash safety

**Obligation:** What's the durability semantics for snapshot writes?

### Answer

**Same as block writes (O-34.2).** Snapshots use the same redb write
transaction discipline; `SyncCadence::PerWrite` fsyncs per
`put_snapshot`. The crash-safety contract test suite extends
`run_crash_safety_tests` with snapshot-specific assertions:

- `put_snapshot_then_kill_then_reopen_observes_snapshot`
- `delete_snapshot_persists_across_reopen`

Both use the same `KillStrategy` interface; S-37 wires real
fault injection.

---

## Acceptance gate for S-35

1. `cargo build --workspace` clean.
2. `cargo test -p ade_runtime` green:
   - Existing 5 tests still pass.
   - New `in_memory_passes_snapshot_contract` ✓
   - New `persistent_passes_snapshot_contract` ✓
   - New `persistent_passes_crash_safety_with_snapshots` ✓
3. `cargo clippy -p ade_runtime --all-targets` clean.
4. Tier isolation grep still returns only `persistent.rs`:
   `rg "redb|rocksdb|sled|sqlite" crates/ade_runtime/src/`.
5. Schema migration tested: open a v1-shaped file (no
   `snapshots_by_slot` table); first snapshot write succeeds and
   bumps to v2.

---

## Forbidden patterns for S-35

- **No coupling snapshots to LedgerState.** The trait stays
  byte-opaque. Don't import `ade_ledger` types into `chaindb`.
- **No automatic pruning.** No "keep last N" hidden policy.
  Operator-driven only.
- **No second public storage path** (separate file). Same-file
  rule from O-35.2 holds.
- **No snapshot index by hash without a use case.** Slot-only.

---

## Out of scope (explicitly)

- Recovery / forward-replay using snapshots (S-36).
- 1,000-kill-9 stress harness (S-37).
- Snapshot rotation utility (operator-side scope).
- Snapshot signing / authentication (defer until feature need).
- Compression (caller may apply).
- Schema migration registry beyond the v1 → v2 auto-bump (future
  slice if migration policy needs to be richer).

---

## Authority Reminder

This discharge is a planning artifact. Authority for the trait
surface belongs to the published `ade_runtime::chaindb::SnapshotStore`
API once S-35 ships.
