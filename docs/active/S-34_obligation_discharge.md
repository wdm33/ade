# Slice S-34 Entry Obligation Discharge

> **Status:** Discharged. Backing-store choice locked to **redb**
> (O-34.1). S-34 shipped: `PersistentChainDb` + `PersistentChainDbOptions`
> + `SyncCadence::{PerWrite, Manual}` + crash-safety harness stubs.
> All five contract+persistence tests green; clippy clean; Tier
> isolation grep returns only `persistent.rs`.
>
> **Authority Level:** Slice-entry proof discharge (per Phase 4 cluster
> plan §"Cluster N-D — Chain DB & persistence").
>
> **Cluster:** N-D (Chain DB & persistence). Tier 5 (intentional
> divergence) for layout / backing store choice; Tier 1 for durability
> and ChainDb trait obligations carried forward from S-33.
>
> **Predecessor:** S-33 (`docs/active/S-33_obligation_discharge.md`)
> shipped the trait surface and an in-memory impl. S-34 adds the
> first persistent impl behind the same trait.

This slice ships a persistent `ChainDb` implementation that satisfies
the existing contract test suite plus a new crash-safety extension.
No callers change — `ade_runtime::chaindb` remains the only public
surface. The slice is deliberately narrow: persistence only. Snapshots
(S-35), recovery (S-36), and durability stress harness (S-37) follow.

---

## Slice scope

**In:**
- `ade_runtime::chaindb::Persistent*` impl of the `ChainDb` trait.
- On-disk layout: single backing store, namespaced keyspaces.
- Durability strategy with documented fsync semantics.
- Schema versioning (magic bytes + version int + migration policy).
- Crash-safety contract extension (the existing `run_contract_tests`
  plus a new `run_crash_safety_tests` that can be invoked under fault
  injection).
- Tier-5 target stakes for CE-N-D-2 (warm-restart latency) and
  CE-N-D-3 (on-disk size).

**Out:**
- Chain snapshots (S-35).
- Forward-replay recovery from snapshots (S-36).
- 1,000-kill-9 stress harness (S-37 — closes CE-N-D-1).
- Any caller changes outside `ade_runtime`.

---

## O-34.1 — Backing store choice (Tier 5)

**Obligation:** Pick the storage backend. This is the load-bearing
Tier 5 decision for cluster N-D — every CE-N-D-2 / CE-N-D-3 metric
flows from it. Wrong choice locks us into either an unmaintained
dependency or a giant rewrite.

### Candidates evaluated

| Backend | Pure Rust? | ACID | Sequential write throughput | Ecosystem | Risk |
|---|---|---|---|---|---|
| **rocksdb** (rust-rocksdb crate) | No (C++ via FFI) | Yes (WAL+manifest) | Excellent — proven on databases, blockchain DBs | Massive; LinkedIn/Facebook/many crypto | Low; mature, but C++ FFI build complexity |
| **redb** | Yes | Yes (MVCC, single-file) | Good for moderate workloads; B-tree-based | Growing; used by some Rust projects | Medium; younger but actively maintained |
| **sled** | Yes | Partial (was unstable historically) | Good | Once-popular, current maintenance status uncertain | High; known data-loss bugs in past releases |
| **fjall** | Yes | Yes (LSM, like rocksdb) | Good — LSM optimized for writes | Very new | Medium-high; least mature |
| **Custom** (append log + B-tree index) | Yes | We define it | Excellent (tailored to append-heavy workload) | N/A | Highest; we own all the bugs |

### Recommendation: **redb**

Rationale:
1. **Pure Rust.** Eliminates the C++ toolchain dependency rocksdb
   imposes. Aligns with the "single static binary, no GHC/external
   toolchain sprawl" Tier 5 goal in
   `docs/active/phase_4_cluster_plan.md` cluster N-F.
2. **ACID guarantees with single-file storage.** Maps cleanly onto
   the Tier 5 design intent of "backup/restore is single-file copy
   + checksum" from the cluster plan.
3. **B-tree underlying, MVCC transactions.** Predictable read
   latency; supports the chain-sync iter pattern naturally.
4. **Active maintenance, MIT/Apache license, no telemetry.**
5. **Tradeoffs accepted:** moderate-to-good write throughput vs.
   rocksdb's excellent. Cardano's block cadence (1 block / ~20s) is
   nowhere near the throughput floor of any of these candidates;
   write performance is not the binding constraint.

### Alternative: rocksdb

Pick rocksdb if:
- C++ build complexity is acceptable in exchange for proven
  blockchain-scale operation history.
- Streaming writes during initial sync from genesis (~10M blocks)
  benefit measurably from rocksdb's WAL+SST tuning vs. redb's B-tree
  inserts. (Worth benchmarking before committing either way.)

### Decision

- **[x] redb** (chosen 2026-05-01)
- [ ] rocksdb
- [ ] other / custom

Resolved per the recommendation rationale above. Pure Rust, ACID,
single-file, MIT/Apache. Aligns with the cluster N-F single-static-binary
goal in `docs/active/phase_4_cluster_plan.md`.

---

## O-34.2 — Durability strategy

**Obligation:** When does a write commit to disk? What's the crash
window between `put_block(b)?` returning `Ok` and the bytes actually
being durable?

### Answer

**Per-put fsync. Configurable via `PersistentChainDbOptions`.**

```rust
pub struct PersistentChainDbOptions {
    /// Path to the single-file backing store.
    pub path: PathBuf,
    /// Sync policy. Default is `SyncCadence::PerWrite`.
    pub sync_policy: SyncCadence,
}

pub enum SyncCadence {
    /// Every put_block / rollback fsyncs before returning.
    /// Strongest durability; lowest throughput.
    PerWrite,
    /// fsync only on commit's redb-default schedule (skips fsync
    /// per write). Operator-controlled durability; useful for relay
    /// nodes during initial sync from genesis.
    Manual,
}
```

`SyncCadence::Batched { window: N }` was scoped out of S-34 in favor
of `Manual` plus operator-side batching policy. The window-based
variant adds state tracking inside `PersistentChainDb` (counter,
flush threshold) without buying anything `Manual` + caller-side
periodic flush doesn't already provide. Re-examine if a future caller
needs in-DB batching.

The default (`PerWrite`) honors the trait's logical durability
contract from S-33 with no caller surprise: after `put_block(b)?`
returns, the block survives a crash. Operators who knowingly accept a
weaker durability window (e.g., during initial sync from genesis)
opt in via `Batched` or `Manual`.

(impl-specific) redb supports per-transaction commit; durability is
guaranteed at transaction boundary. Each `put_block` opens a write
transaction and commits within the call; `Batched` accumulates
puts in one transaction and commits at the window boundary.

### Crash-window obligations (extend S-33 contract)

The new contract test suite `run_crash_safety_tests` codifies:
1. After `put_block(b)?` returns under `PerWrite`, simulated power
   loss at any subsequent point yields a reopened db where `b` is
   observable.
2. Mid-`put_block` crash leaves no half-written block; reopen
   observes either the full block or nothing.
3. Mid-`rollback_to_slot` crash leaves the tip ≤ the rollback target;
   never beyond.

These gate S-37, not S-34, but the test interfaces ship in S-34 so
S-37 has something to call.

---

## O-34.3 — On-disk layout (impl-specific)

**Obligation:** What's the schema? How are blocks, slot index, and
hash index laid out? Why doesn't this layout box us into one set of
access patterns?

### Answer (redb)

Three tables in a single redb database file:

| Table | Key | Value | Role |
|---|---|---|---|
| `blocks_by_slot` | `u64` (slot) | `Vec<u8>` (block bytes) | Canonical store. Iter by slot is a B-tree range scan. |
| `slot_by_hash` | `[u8; 32]` (hash) | `u64` (slot) | Lookup index. `get_block_by_hash` does two reads. |
| `meta` | `&'static str` (key name) | `Vec<u8>` (value) | Schema version, magic bytes, schema revision int. |

**Why store hash in the index, not the block table:**
- Block bytes are large (~64 KB+). Storing them under the hash key
  too would double the on-disk footprint.
- Two reads on hash lookup is acceptable: hash → slot → block.
  Slot index is small and warm.

**Why not embed slot inside the block bytes' canonical form:**
- It's already there (the block header carries the slot). Storing
  the slot redundantly as a key avoids re-decoding on iter.

**(impl-specific) Footprint estimate:** for ~10M blocks at average
block size ~50 KB, total ≈ 500 GB raw block bytes + ~360 MB index.
Compared to cardano-node's ImmutableDB+VolatileDB+LedgerDB raw blocks
+ multiple index files at ~similar block size, the projection is
≤50% on-disk size (CE-N-D-3 stake, see O-34.6).

---

## O-34.4 — Schema versioning

**Obligation:** What happens when a future binary opens an old
database, or vice versa?

### Answer

`meta` table carries:
- `magic`: literal bytes `ADE\0CHAINDB\0` (12 bytes). Distinguishes
  Ade chaindb files from arbitrary redb files.
- `schema_version`: u32. Bumped on incompatible layout changes.
- `binary_version`: string ("0.1.0"). Diagnostic only.

Open path:
1. If file doesn't exist → create with current schema, write magic
   + version.
2. If file exists, magic absent → return
   `ChainDbError::Corruption("not an Ade chaindb file")`.
3. Magic present, schema_version > current → return
   `ChainDbError::SchemaMismatch { expected, found }`. Caller's
   choice (typically: refuse to start; operator runs migration tool).
4. Magic present, schema_version < current → run
   in-place migration if registered for that version pair, else
   return `SchemaMismatch`.

Migrations are not in scope for S-34; the slice ships schema
version 1 only. A future slice may add the migration registry.

---

## O-34.5 — Crash-safety contract extension

**Obligation:** S-33 only validated logical correctness. Persistent
storage requires extending the contract test suite to cover crash
windows.

### Answer

Add a sibling test runner:

```rust
pub fn run_crash_safety_tests<D, F, K>(make_db: F, kill_at: K)
where
    D: ChainDb,
    F: Fn() -> D,
    K: Fn(&D);  // simulated kill — release-without-flush in tests
```

`kill_at` is a fault-injection callback; in unit tests it's a no-op
(fsync semantics are validated by reopen-and-check). In S-37 it's
replaced by a real `kill -9` against a child process.

S-34 ships this interface plus the no-op variant. S-37 wires the
real fault injection.

The test list:
1. `put_then_kill_then_reopen_observes_block`
2. `mid_rollback_kill_keeps_invariant`
3. `repeated_put_same_block_idempotent_across_reopens`
4. `magic_bytes_corrupted_returns_error`
5. `schema_version_mismatch_returns_error`

---

## O-34.6 — Tier 5 target stakes

**Obligation:** This slice carries CE-N-D-2 (warm-restart latency)
and CE-N-D-3 (on-disk size). Without quantitative stakes, "Tier 5
improvement" is just rhetoric.

### Stakes

**CE-N-D-2: warm-restart latency ≤ 30s for state at chain tip.**

Measurement: time from process start to "first `tip()` call returns"
on a chaindb populated with 10K most-recent blocks. Gate is wall-clock
on a baseline reference machine (to be specified in the bench harness).

**CE-N-D-3: on-disk size ≤ 50% of cardano-node's chain DB at the
same slot.**

Measurement: sum of file sizes under chaindb's storage path vs.
cardano-node's `db/{immutable,volatile,ledger}/` at a curated
checkpoint slot. Reported by a benchmark in `crates/ade_runtime/benches/`.

Both gates close in S-34's bench output, **as evidence**, not as
release-blocking gates. Tier 5 improvement targets accumulate
across the cluster; they don't gate slice acceptance individually.

### Anti-stake (what we're NOT chasing)

- Maximum write throughput. Cardano produces 1 block / ~20s on
  mainnet; 10ms-per-put is fine. Don't sacrifice durability or
  simplicity for throughput we don't need.
- Compatibility with cardano-node's chain DB format. Explicit
  Tier 4 non-goal — same family as CE-73-bytes.

---

## Acceptance gate for S-34

Sequencing assumes O-34.1 has been answered.

1. `cargo build -p ade_runtime` clean.
2. `cargo test -p ade_runtime` green:
   - `run_contract_tests` against Persistent impl ✓
   - `run_crash_safety_tests` no-op variant ✓
   - All S-33 tests still pass.
3. `cargo clippy -p ade_runtime --all-targets` clean.
4. `rg "rocksdb|sled|sqlite" crates/ade_runtime/src/chaindb/`
   returns nothing IF redb chosen; matches only the chosen backend
   inside the impl module. Tier-isolation rule from S-33 holds.
5. CE-N-D-2 / CE-N-D-3 bench numbers reported (not gated).

---

## Forbidden patterns for S-34

- **No "we'll fsync later" stubs.** The default `PerWrite` policy
  must actually fsync per put.
- **No leaking redb (or rocksdb) types through `chaindb::*`.** The
  trait surface remains the published API; backing-store types stay
  inside `chaindb::persistent` (or wherever the impl module lives).
- **No second public chaindb path.** The trait surface is the only
  public surface; persistent vs in-memory is an impl choice.
- **No "match cardano-node's layout" temptation.** Tier 5 explicitly
  diverges; matching layouts is conformance for its own sake.

---

## Out of scope (explicitly)

- Snapshots (S-35).
- Recovery / forward-replay (S-36).
- 1,000-kill-9 stress harness (S-37).
- Migration tooling for schema bumps (future slice).
- Cardano-node-format export tooling (future slice if ever needed).
- Concurrent multi-writer support (still out of scope per S-33).

---

## Authority Reminder

This discharge is a planning artifact. The Tier 5 backing-store choice
in O-34.1 is the only blocking decision; the rest of the document is
recommendations grounded in the cluster plan. Once O-34.1 is locked,
implementation proceeds.
