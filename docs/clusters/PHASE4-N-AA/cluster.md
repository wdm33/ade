# PHASE4-N-AA — Bounded peer-driven serve range (DC-SERVEMEM-01)

> **Pre-RO-LIVE hardening, item 1.** Closes the MEDIUM cross-slice security finding from the PHASE4-N-U close: the `--mode node` serve path can be driven by an untrusted peer into unbounded storage/CPU work. The serve-side analog of **DC-LIVEMEM-01** (the PHASE4-N-F-G-E receive-side bounded-memory work). User-confirmed scope + invariant wording 2026-06-05; tracked in `project_pre_rolive_hardening_queue.md` item 1.

## §1 Primary invariant (DC-SERVEMEM-01)
Peer-driven serve range work is bounded. The `--mode node` serve path must not materialize an unbounded chain range, perform per-block full-index scans, or read more than `MAX_SERVE_RANGE_BLOCKS` blocks for a single peer request. Oversized ranges fail closed before unbounded storage/CPU work. The cap is a defensive implementation bound, not a Cardano semantic parameter, and cannot be disabled at runtime.

`MAX_SERVE_RANGE_BLOCKS = 256` — symmetric with the receive-side `MAX_WIRE_PUMP_LOOKAHEAD = 256` (DC-LIVEMEM-01); a fixed, closed, non-configurable defensive bound that may be tightened later.

## §2 The problem (proven from code, not hypothesis)
`crates/ade_runtime/src/chaindb/persistent.rs`:
- `iter_from_slot(from)` **materializes the full `from..tip` range into a `Vec<StoredBlock>`** before returning, and for **each** block scans the entire `SLOT_BY_HASH` table (hash-keyed; no slot→hash index) via `hashes.iter().find_map(...)` to recover the hash — **O(N²) time + O(chain-bytes) memory** per call.
- `tip()` does `blocks.iter()…last()` (O(N)) + the same O(N) hash scan; the serve calls `tip()` on **every** chain-sync request.

`crates/ade_runtime/src/network/served_chain_projection.rs` (the `--mode node` serve, DC-NODE-13):
- `range_bytes` and `next_after` call `iter_from_slot` — so a peer `RequestRange(block0, tip)` forces the full-chain materialization + O(N²) scan; **no per-request range cap exists**.

This is a peer-driven resource-amplification path: an untrusted `RequestRange` → unbounded storage reads + CPU.

## §3 The design — bounded serve reads + derive-hash-at-serve (NO schema change)
**Consumer map** (grounds the design): `recovery`/`rollback` consume `iter_from_slot` using only `slot`+`bytes` (never `.hash`); the serve uses `.hash` but already calls `decode_block(&bytes)`, and `decode_block(bytes).block_hash` **is** the stored hash (both = `blake2b_256(header_cbor)`). So the hash is **derivable at the serve from the block's own bytes** — the storage layer never needs an efficient slot→hash lookup, and no `HASH_BY_SLOT` index / schema migration is required.

Authority split (kept clean):
- **ChainDb (Tier-5 storage):** opaque bytes + **bounded, slot-ordered, hash-free** read primitives. No block decode in storage; no second hash authority.
- **Serve projection (RED):** decode bytes through the existing BLUE authority (`decode_block` / `block_header_bytes`), derive the hash, answer ChainSync / BlockFetch, and cap the per-request range fail-closed.

S1 adds the bounded read primitives **for the serve** and fences accidental reuse of the unbounded path — it is **not** a broad storage-optimization pass (the trusted-recovery `iter_from_slot` internals are left alone; only doc-fenced).

## §4 Normative anchors
- Invariant registry `docs/ade-invariant-registry.toml` — adds **DC-SERVEMEM-01** (declared → enforced at S2).
- Cross-ref: **DC-LIVEMEM-01** (receive-side bounded memory, the analog), **DC-NODE-13** (serve-as-projection — the surface being bounded), **DC-NODE-07** (single serve dispatch), **CN-CONS-07** (serve provenance — unchanged), **DC-CONS-17** (block-fetch byte-identity — unchanged).
- Source: PHASE4-N-U cross-slice security review (MEDIUM finding); `project_pre_rolive_hardening_queue.md` item 1.

## §5 Entry conditions (what prior clusters guarantee)
- **N-U S3 (DC-NODE-13):** the `--mode node` serve is a read-only projection of the durable ChainDb via `ChainDbServedSource` reading `iter_from_slot`/`get_block_by_hash`/`tip` (the path this cluster bounds).
- **N-D (ChainDb):** `BlockIter` / `iter_from_slot` / `get_block_by_slot` / `tip` / redb-backed `PersistentChainDb` + `InMemoryChainDb`.
- **N-F-G-E (DC-LIVEMEM-01):** the receive-side bounded-memory precedent (`MAX_WIRE_PUMP_LOOKAHEAD = 256`, the 16 MiB reassembly cap) — the symmetric pattern.
- BLUE `ade_ledger::block_validity::{decode_block, block_header_bytes}` — the single hash/header authority the serve reuses (unchanged).

## §6 TCB color map (FC/IS partition)
- **RED (new/changed):** `ade_runtime::chaindb::{contract (trait), persistent, in_memory}` (new bounded read primitives — storage), `ade_runtime::network::served_chain_projection` (serve cap + derive-hash + fail-closed).
- **RED (reused, unchanged):** `iter_from_slot`/`tip`/`get_block_by_*` (trusted recovery/rollback paths — doc-fenced, internals untouched), `serve_dispatch`.
- **BLUE (reused, NOT edited):** `ade_ledger::block_validity::{decode_block, block_header_bytes}` (hash/header authority), the N-G serve reducers.
- **No new BLUE authority, no new canonical type, no storage schema change.**

## §7 Slices
| Slice | Scope | CE | Registry → enforced | TCB |
|---|---|---|---|---|
| **S1** | Bounded ChainDb read primitives: add hash-free `range_bytes_capped(from, to, max)` (lazy redb range, take ≤ max, returns the in-range bytes + a `truncated` signal; NO `SLOT_BY_HASH` scan) + `last_block_bytes()` (redb `.last()`, O(log N)) to the `ChainDb` trait + `PersistentChainDb` + `InMemoryChainDb`. Doc-fence `iter_from_slot`/`tip` as full/trusted-only (not for peer-driven serve). NO `iter_from_slot` internal rewrite. | CE-1 | (capability for DC-SERVEMEM-01) | RED |
| **S2** | Serve projection cap + fail-closed: `ChainDbServedSource::range_bytes` uses `range_bytes_capped(from.slot, to.slot, MAX_SERVE_RANGE_BLOCKS)` + derives each hash via `decode_block(bytes).block_hash`; `next_after` uses a bounded read (cap 2); `tip` uses `last_block_bytes` + derive. A range exceeding the cap is distinguished internally (closed reason: empty vs cap-exceeded) and fails closed (reducer → `NoBlocks`). | CE-2 | DC-SERVEMEM-01 | RED |

## §8 Cluster Exit Criteria
All mechanical/CI-verifiable.
- **CE-1 (S1):** `cargo test -p ade_runtime` green incl. tests proving `range_bytes_capped` returns at most `max` blocks (never the full chain), signals `truncated` when the range exceeds `max`, performs no `SLOT_BY_HASH` scan, and is byte-identical to the stored bytes; `last_block_bytes` returns the highest-slot block via redb `.last()`. The two primitives match between `PersistentChainDb` and `InMemoryChainDb` (contract parity).
- **CE-2 (S2, DC-SERVEMEM-01):** new gate `ci/ci_check_serve_range_bounded.sh` — the serve projection (a) calls only the bounded primitives (`range_bytes_capped` / `last_block_bytes`), NOT `iter_from_slot`; (b) does no `SLOT_BY_HASH` / full-index scan; (c) passes a fixed `MAX_SERVE_RANGE_BLOCKS` literal (no CLI/env/config read; no "unbounded" path); (d) derives the hash via `decode_block`/`block_header_bytes` (no second hash authority); DC-SERVEMEM-01 present-and-enforced. Tests: `serve_range_within_cap_byte_identical`, `serve_range_over_cap_fails_closed` (a `[block0, tip]`-style range beyond the cap → `NoBlocks`, distinguished from empty by the closed internal reason), `serve_derives_hash_from_bytes_matches_stored`, `serve_tip_via_last_block_bounded`, `next_after_reads_bounded`.
- **Cluster-wide:** `cargo test --workspace --exclude ade_testkit` green (ade_testkit excluded — pre-existing ~600s corpus timeout); full `ci/ci_check_*.sh` sweep stays **135 + 1 = 136 / 0** (the one new gate); the N-U serve loopback + C1 genesis-rehearsal regressions intact (serving small ranges is byte-identical).

## §9 Replay obligations
Serve is read-only (advances no tip, admits nothing). The bounded reads are a deterministic function of the durable chain + the request range + the fixed cap (same chain + same request → byte-identical served frames, capped identically). No new canonical type, no WAL/checkpoint change, no schema change. Within-cap serving is byte-identical to the pre-cluster behavior (DC-CONS-17 preserved). Command: `cargo test -p ade_runtime` + the `ade_node` serve loopback tests.

## §10 Invariants
- **Adds:** DC-SERVEMEM-01 (declared → enforced at S2).
- **Strengthens** (`strengthened_in += "PHASE4-N-AA"` at close): DC-NODE-13 (the serve projection is now bounded), DC-LIVEMEM-01 (extended from receive-side to a symmetric serve-side bound — cross-ref, not a code change to G-E).
- **Preserves / cross-ref (NOT changed):** DC-NODE-07 (single dispatch — unchanged), CN-CONS-07 (serve provenance — unchanged; bytes still trace to the durable admit), DC-CONS-17/18 (block-fetch byte-identity + header authority — within-cap serving byte-identical), DC-NODE-06 (durable-provenance serve — unchanged).

## §11 Forbidden during this cluster (hard boundaries — user-set)
- No unbounded `Vec` of peer-requested blocks.
- No per-request full-chain materialization.
- No per-block scan of `SLOT_BY_HASH` in the serve path.
- No runtime-configurable / unbounded cap (no CLI/env/config escape hatch; no "unbounded" mode).
- No storage schema migration in this cluster.
- No duplicate block-hash authority (the serve derives via the existing BLUE `decode_block`/`block_header_bytes`).
- No BLUE semantic change.
- No RO-LIVE flip.

## §12 Open questions
- **S1 trusted-iteration scope (resolved):** `iter_from_slot`'s own O(N²)/full-Vec on the trusted recovery/rollback path is OUT OF SCOPE — it is doc-fenced (not for peer-driven serve), internals untouched. A streaming/efficient trusted-recovery rewrite is a separate, non-security perf follow-on (not this cluster).
- **`tip()` (resolved):** the serve stops calling `chaindb.tip()` (uses `last_block_bytes` + derive); `chaindb.tip()`'s O(N) remains for its trusted callers (not peer-driven) and is left alone.

## §13 Close record
*(Open — filled at `/cluster-close` once CE-1…CE-2 are green.)*
