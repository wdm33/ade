# Slice S-33 Entry Obligation Discharge

> **Status:** Discharged. S-33 implementation may begin.
>
> **Authority Level:** Slice-entry proof discharge (per Phase 4 cluster
> plan §"Cluster N-D — Chain DB & persistence").
>
> **Cluster:** N-D (Chain DB & persistence). Tier 5 (intentional
> divergence). See `docs/active/phase_4_cluster_plan.md` and
> `docs/active/CE-79_tier5_addendum.md`.
>
> **Version scope:** cardano-node 10.6.2 protocol surface. Storage
> layout is implementation-internal and version-coupled to Ade only.

This is the **first Phase 4 slice**. It establishes the storage
abstraction the rest of cluster N-D (and downstream clusters N-A,
N-B, N-E) will use. The slice is deliberately narrow: define the
trait surface and a minimal in-memory implementation for testing.
No persistent backing store yet — that's S-34.

---

## Slice scope

**In:**
- `ade_runtime::chaindb::ChainDb` trait (the abstract storage surface).
- `ade_runtime::chaindb::ChainDbError` enum (failure taxonomy).
- `ade_runtime::chaindb::InMemoryChainDb` (test-only impl, satisfies
  the trait).
- Contract-level tests against the trait (any impl must pass).
- Tier-5 design rationale doc.

**Out:**
- Persistent backing store (rocksdb / sled / custom) — S-34.
- Snapshot management — S-35.
- Recovery / rollback — S-36.
- Durability stress test (CE-N-D-1) — S-37.
- Network integration — Cluster N-A.

**Tier classification:**
- **Trait signatures** — Tier 1 (callers depend on these; changes
  break API contract).
- **Implementation choice** (in-memory now, persistent later) — Tier 5
  (deliberate divergence opportunity).
- **Storage layout / file format** — Tier 5.
- **Block bytes returned** — Tier 1 (must be byte-identical to what
  was written; what was written is whatever the wire / consensus
  layer canonicalized).

---

## O-33.1 — Trait operations

**Obligation:** What is the minimal set of operations the trait must
expose? Anything we add later is API surface debt; anything we omit
forces awkward workarounds in callers.

### Answer

Three logical surfaces, six operations:

```rust
pub trait ChainDb: Send + Sync {
    // --- Block storage ---
    fn put_block(&self, block: &Block) -> Result<(), ChainDbError>;
    fn get_block_by_hash(&self, hash: &Hash32) -> Result<Option<Block>, ChainDbError>;
    fn get_block_by_slot(&self, slot: SlotNo) -> Result<Option<Block>, ChainDbError>;

    // --- Tip & iteration ---
    fn tip(&self) -> Result<Option<ChainTip>, ChainDbError>;
    fn iter_from_slot(
        &self,
        from: SlotNo,
    ) -> Result<Box<dyn Iterator<Item = Result<Block, ChainDbError>> + '_>, ChainDbError>;

    // --- Rollback ---
    fn rollback_to_slot(&self, slot: SlotNo) -> Result<(), ChainDbError>;
}
```

**Why this set:**

- `put_block` is the only write op; consensus appends, never edits.
- Two read paths because callers use both: chain-sync iterates by
  slot, block-fetch / queries look up by hash.
- `tip` is the bootstrap signal — every consumer needs to know
  "where am I?"
- `iter_from_slot` is the chain-sync workhorse and the snapshot
  replay engine. Iterator-shaped so callers can stream without
  loading everything.
- `rollback_to_slot` is the only operation that mutates non-tip
  history. Required for fork-choice (cluster N-B). Trait-level
  rather than internal because durability semantics differ
  between rollback and put.

**Why not on this trait:**

- Snapshot read/write — different lifecycle, different shapes,
  different durability requirements. S-35 separately.
- Concurrent multi-writer support — explicitly out of scope; one
  writer (the consensus runtime), many readers. Encoded by the
  `Send + Sync` requirement plus call-site discipline; do not
  expose locking.
- Pruning / garbage collection — Tier 5 implementation detail,
  not in trait.
- Backup / restore — Tier 5 operational concern, handled at the
  filesystem level (single-file copy).

### Citation

No external authority; this is Tier 5 design intent. The shape is
informed by:
- cardano-node's `LocalChainSync` mini-protocol API (what callers
  need to be able to express).
- Reth's `BlockProvider` and `BlockchainTreeViewer` traits as
  prior art for Rust-shaped chain abstractions.
- The Phase 4 cluster plan's stated goal: replace the
  ImmutableDB+VolatileDB+LedgerDB three-DB pattern with a single
  abstraction whose internals are free.

---

## O-33.2 — Durability model

**Obligation:** What does "durable" mean for `put_block`? When does
fsync happen? What gets persisted under what crash window?

### Answer

**The trait says nothing about durability. Implementations choose.**

The trait contract is *logical*: after `put_block(b).await?` returns
`Ok`, subsequent calls must observe `b` in lookups by hash and slot.
That is durability *to the caller*; whether the bytes have hit disk
is an implementation choice.

**Mandatory durability obligations** (enforced via contract tests):

1. After successful `put_block`, the block is observable via both
   `get_block_by_hash` and `get_block_by_slot` from the same
   handle and from any other handle to the same store.
2. After `tip()` returns `Some(t)` with `t.slot >= s`, every block
   at slot `≤ s` returned by `iter_from_slot(0)` is `==` to what
   was previously `put_block`'d.
3. After `rollback_to_slot(s)`, no block at slot `> s` is observable.

**Crash-safety obligations** (enforced per implementation, not
trait-level):

1. If a crash occurs mid-`put_block`, on restart either the block is
   fully observable or fully absent — never half-written.
2. If a crash occurs after `put_block` returns `Ok`, on restart the
   block IS observable (caller's contract).
3. If a crash occurs during `rollback_to_slot`, on restart the tip
   is at most where rollback was heading — never beyond.

The persistent impl in S-34 will pick a specific strategy
(write-ahead log, atomic rename, sync-on-commit). The in-memory
impl in this slice trivially satisfies (1)-(3) by virtue of having
no persistence.

**Why trait-silent on fsync timing:**

Different deployment shapes want different latency / durability
trade-offs. A relay node can batch fsync per epoch; a producing
node should fsync per block. Putting policy in the trait collapses
that flexibility. Implementations expose tuning knobs separately.

---

## O-33.3 — Error taxonomy

**Obligation:** What failure modes does the trait surface, and how
do callers distinguish "block not found" from "block exists but I
can't read it"?

### Answer

```rust
#[derive(Debug)]
pub enum ChainDbError {
    /// Storage layer I/O failure (disk full, permissions, etc.).
    /// Not a logic error; retry may succeed.
    Io { source: std::io::Error },

    /// Stored data failed integrity check (checksum mismatch,
    /// truncated record, version tag invalid). Implies storage
    /// corruption; cannot be recovered by retry.
    Corruption { detail: String },

    /// Storage was opened with a schema version this binary doesn't
    /// understand. Caller chooses migration path.
    SchemaMismatch { expected: u32, found: u32 },

    /// Operation is invalid for current state (e.g., rollback to
    /// slot beyond current tip). Caller-side logic error.
    InvalidOperation { detail: String },
}
```

**`Option`-based not-found, error-based failure**:
- `get_block_by_*` returns `Result<Option<Block>, ChainDbError>`.
  `Ok(None)` is "no block at this key" — a normal, expected outcome
  for chain-sync probing. `Err(...)` is "I can't tell you, something
  is wrong."
- This separation is load-bearing: callers must not treat "not
  found" as an error path, and must not silently retry on a real
  error.

**Why no `BlockNotFound` variant:** that's a logic claim about the
chain, not a storage claim. Whether absence is meaningful depends
on caller context (chain-sync expects gaps; queries do not). The
distinction belongs at the caller, not the trait.

**Why no concurrency error variant:** the trait commits to
`Send + Sync` and a single-writer model. Concurrent-write attempts
are caller bugs; the impl may panic, deadlock, or serialize, but
this trait does not specify which. If we later need concurrent
multi-writer support, that's a different trait.

---

## O-33.4 — Tier isolation

**Obligation:** How does the trait keep Tier-1 conformance (block
bytes, hashes, slot numbers) cleanly separated from Tier-5
divergence (storage layout, file format, indexing strategy)?

### Answer

**Trait inputs and outputs use only canonical-domain types:**
- `Block` (from `ade_types`) — wire-byte authoritative; `Block::hash()`
  produces the chain's hash.
- `Hash32` — domain hash type.
- `SlotNo` — domain slot type.
- `ChainTip` — `(SlotNo, Hash32)` pair.

**Trait says nothing about:**
- File paths, directory layouts, key/value schemas.
- Encoding of stored bytes (the impl can store wire-bytes
  unchanged, recompress them, or split body/header — caller can't
  tell).
- Index structures (B-trees, hash maps, LSM forests — impl detail).
- Whether snapshots and blocks share a backing store.

**Mechanical enforcement:**
- The `chaindb` module re-exports nothing from any backing store
  crate (no `pub use rocksdb::...`).
- A CI check (`ci/ci_check_chaindb_tier_isolation.sh`, S-34) greps
  for backing-store crate names in non-impl modules.

**What this enables:**
- The persistent impl in S-34 can be written once and swapped for
  an alternative (different key layout, different backing store,
  different compression) without changes anywhere else.
- Tier-5 improvement targets (CE-N-D-2 warm restart latency,
  CE-N-D-3 on-disk size) are pursued behind the trait, with no
  caller-visible churn.

---

## O-33.5 — Test strategy for a Tier-5 slice

**Obligation:** What does "proof obligations" mean for a slice
whose primary virtue is divergence rather than conformance?

### Answer

Three test layers:

**1. Trait contract tests (Tier-1 obligations).**
A reusable test suite — `trait_contract_test_suite(impl: &dyn ChainDb)`
— that any `ChainDb` implementation must pass. Encodes the logical
durability obligations from O-33.2. Lives in
`crates/ade_runtime/src/chaindb/contract_tests.rs` and runs against
both `InMemoryChainDb` (this slice) and the persistent impl (S-34).

**2. Tier-5 improvement-target benchmarks.**
Latency / footprint / throughput benchmarks under
`crates/ade_runtime/benches/`. These do not gate the slice — they
inform the design rationale and accumulate evidence for CE-N-D-2 /
CE-N-D-3 closure later. No comparison against cardano-node;
self-comparison across iterations.

**3. Crash-safety fault injection (S-37 scope).**
Out of this slice. Listed here for completeness so callers know
what's coming.

**Acceptance gate for S-33:**
- Trait compiles in `ade_runtime`.
- `InMemoryChainDb` passes the trait contract test suite.
- `cargo test -p ade_runtime` green.
- `cargo clippy -p ade_runtime` clean.
- No backing-store crate names appear in ade_runtime
  (`rg "rocksdb|sled|sqlite|redb" crates/ade_runtime/` returns
  nothing).

---

## Summary of decisions locked for S-33

1. **Trait surface** (O-33.1): six methods on `ChainDb` covering put,
   read-by-hash, read-by-slot, tip, iter, rollback. No snapshots,
   no concurrent-write, no pruning. Snapshots split into S-35.

2. **Durability model** (O-33.2): trait-silent on fsync; logical
   durability contract via three observable obligations. Crash-safety
   handled per-impl, not in trait.

3. **Error taxonomy** (O-33.3): four variants — `Io`, `Corruption`,
   `SchemaMismatch`, `InvalidOperation`. Not-found is `Ok(None)`,
   never an error.

4. **Tier isolation** (O-33.4): trait speaks only canonical-domain
   types. Backing-store crates are not re-exported. CI guards
   isolation in S-34.

5. **Test strategy** (O-33.5): trait contract test suite reusable
   across impls; Tier-5 benchmarks are evidence, not gates;
   crash-safety in S-37.

6. **Slice boundary**: this slice ships the trait + in-memory impl
   only. No persistent backing store. No callers in `ade_node` or
   `ade_ledger` yet — those wire in during cluster N-A's
   chain-sync slice.

---

## Forbidden patterns for S-33

- **No "we'll figure out durability later" stubs.** The contract
  tests must pass on the in-memory impl; callers must be able to
  rely on the logical durability obligations regardless of impl.
- **No leaking backing-store types through the trait.** The trait
  signatures contain only `ade_types` / `ade_runtime` types.
- **No reaching into the future.** This slice is not S-34. Don't
  half-implement persistence "to save time later."
- **No concurrent multi-writer accommodation.** Single writer
  is the contract; defending against multiple writers is wasted
  scope.

---

## Out of scope (explicitly)

- Persistent backing store implementation (S-34).
- Snapshot read/write (S-35).
- Rollback semantics beyond "blocks past the slot disappear" (S-36).
- Crash-safety stress testing (S-37, CE-N-D-1).
- Tier-5 footprint / latency benchmarks against cardano-node
  (deferred to operational evidence; no head-to-head gate).
- Integration with consensus runtime / chain-sync mini-protocol
  (cluster N-A).

---

## Authority Reminder

This discharge is a planning artifact. Authority for the trait
surface belongs to the published `ade_runtime::chaindb` API once
S-33 ships. The Tier classifications above derive from
`docs/active/CE-79_tier5_addendum.md` (draft) and the Phase 4
cluster plan (`docs/active/phase_4_cluster_plan.md`).
