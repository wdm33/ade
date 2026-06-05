# Invariant Slice — PHASE4-N-AA S1: bounded ChainDb read primitives

## §2 Slice Header
- **Slice Name:** bounded ChainDb read primitives
- **Cluster:** PHASE4-N-AA (bounded peer-driven serve range) — primary invariant **DC-SERVEMEM-01**
- **Status:** in progress
- **Cluster Exit Criteria Addressed:** **CE-1** — the bounded, hash-free, slot-ordered read primitives the serve cap (S2) is built on. *(CE-2 / the DC-SERVEMEM-01 flip = S2.)*

## §3 Dependencies
N-D ChainDb (`ChainDb` trait, `StoredBlock`, `PersistentChainDb` redb backing, `InMemoryChainDb`). No dependency on S2.

## §4 Intent (invariant impact)
Give the ChainDb the **storage capability** S2's serve cap needs: read a slot range, or the chain tip, with **bounded memory** and **no per-block `SLOT_BY_HASH` scan**, so the serve never has to call the unbounded `iter_from_slot` (which materializes the full `from..tip` range into a `Vec` and scans the whole hash table per block — O(N²)). This slice is storage capability only; the peer-facing policy (the cap value + fail-closed) is S2. It is NOT a broad storage-optimization pass — the trusted-recovery `iter_from_slot` internals are left unchanged (only doc-fenced).

## §5 Scope / What is built
- **NEW closed value type** `CappedSlotRange { blocks: Vec<(SlotNo, Vec<u8>)>, truncated: bool }` in `ade_runtime::chaindb` (`types.rs`). `truncated` = the requested range contained MORE than `max` blocks (the per-request cap was exceeded) — the structured signal S2 uses to distinguish "cap exceeded" from "genuinely empty" even though both encode to the same wire `NoBlocks`.
- **NEW `ChainDb` trait method `range_bytes_capped(from: SlotNo, to: SlotNo, max: usize) -> Result<CappedSlotRange, ChainDbError>`** — lazily ranges `from..=to` over the slot-ordered store, takes at most `max + 1` entries, returns the first `≤ max` as `(slot, bytes)` with `truncated = (more than max existed)`. **Hash-free** (the serve derives the hash from bytes) → NO `SLOT_BY_HASH` scan. Memory bounded to `≤ max` blocks regardless of chain length.
  - `PersistentChainDb`: `blocks.range(from.0..=to.0)` (lazy redb range) `.take(max + 1)`; collect `(slot, bytes)`; no hash recovery.
  - `InMemoryChainDb`: `by_slot.range(from..=to)` `.take(max + 1)`; `(slot, block.bytes)`.
- **NEW `ChainDb` trait method `last_block_bytes(&self) -> Result<Option<(SlotNo, Vec<u8>)>, ChainDbError>`** — the highest-slot block's `(slot, bytes)`, O(log N), hash-free.
  - `PersistentChainDb`: `blocks.last()` (redb O(log N)); no `blocks.iter()…last()` (O(N)), no hash scan.
  - `InMemoryChainDb`: `by_slot.values().next_back()` → `(slot, bytes)`.
- **Doc-fence** `iter_from_slot` + `tip` (in `ade_runtime::chaindb`): a doc-comment marking them full-materialization / hash-scanning, for **trusted internal callers only (recovery / rollback)** — NOT for the peer-driven serve path (which must use the bounded primitives). No change to their internals.
- **Out of scope:** any `iter_from_slot` / `tip` / `get_block_by_slot` internal rewrite; any schema change; the serve cap (S2).

## §6 Execution Boundary (TCB color)
- **RED (new/changed):** `ade_runtime::chaindb::{contract (trait + CappedSlotRange via types), persistent, in_memory}` — bounded read primitives.
- **RED (reused, unchanged internals):** `iter_from_slot`, `tip`, `get_block_by_*` (doc-comment only).
- **BLUE:** none touched.
- **No new BLUE authority / canonical type / storage schema change.** `CappedSlotRange` is a RED storage value type, not a canonical (hashed/persisted) type.

## §7 Invariants Preserved
The `ChainDb` logical contract (put-then-observable; slot-ascending iteration; `get_block_by_hash` O(1)) — the new primitives are additive reads, no write/mutation. Determinism: the bounded reads are pure functions of (durable store, range, max). No schema change → on-disk format unchanged; existing dbs read identically.

## §8 Invariants Strengthened or Introduced
DC-SERVEMEM-01 stays **declared** at S1 (this slice ships the capability; the rule flips to **enforced** at S2 when the serve actually uses the cap + the gate lands). S1 adds no enforced rule on its own — it is the storage substrate.

## §11 Replay / Crash / Epoch Validation
Read-only additive primitives; no replay/crash/epoch surface. The contract tests (CE-1) prove bounded behavior + cross-impl parity.

## §12 Mechanical Acceptance Criteria
- `cargo test -p ade_runtime` green incl. NEW (run for BOTH `PersistentChainDb` and `InMemoryChainDb` — contract parity):
  - `range_bytes_capped_returns_at_most_max` — a chain longer than `max` → exactly `max` blocks + `truncated == true`.
  - `range_bytes_capped_within_cap_not_truncated` — a range ≤ `max` → all in-range blocks + `truncated == false`.
  - `range_bytes_capped_bytes_byte_identical` — returned bytes == stored bytes (verbatim).
  - `range_bytes_capped_respects_bounds` — only slots in `[from, to]` returned, ascending.
  - `last_block_bytes_returns_highest_slot` — the tip block's `(slot, bytes)`; `None` on empty.
- No `SLOT_BY_HASH` iteration in the new primitives (the gate that asserts this is S2's `ci_check_serve_range_bounded.sh`; S1 keeps the impls scan-free by construction).
- Relevant crate tests green; the existing ChainDb contract/crash-safety tests stay green (no behavior change to `iter_from_slot`/`tip`/`get_block_by_*`).

## §14 Hard Prohibitions
**Inherited (cluster §11):** no per-block `SLOT_BY_HASH` scan in the new primitives; no storage schema migration; no second hash authority (the primitives are hash-FREE — they don't recover the hash at all); no BLUE change; no RO-LIVE flip.
**Slice-specific:** `range_bytes_capped` must bound memory to `≤ max` (take `max + 1`, never collect the full range); `last_block_bytes` must use the O(log N) tip access (`blocks.last()` / `next_back()`), never a full `iter()`; do NOT rewrite `iter_from_slot`/`tip` internals (doc-fence only).

## §15 Explicit Non-Goals
The serve cap + fail-closed policy (S2); any optimization of the trusted-recovery `iter_from_slot` (full-Vec / O(N²)) — a separate non-security perf follow-on; any `HASH_BY_SLOT` index / schema change; `tip()` rewrite.
