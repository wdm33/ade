# Invariant Slice — PHASE4-N-AA S2: serve projection cap + fail-closed

## §2 Slice Header
- **Slice Name:** serve projection cap + fail-closed
- **Cluster:** PHASE4-N-AA (bounded peer-driven serve range) — primary invariant **DC-SERVEMEM-01**
- **Status:** in progress
- **Cluster Exit Criteria Addressed:** **CE-2** — the DC-SERVEMEM-01 flip (the peer-facing policy: cap + fail-closed + derive-hash-at-serve + the gate).

## §3 Dependencies
S1 (the bounded ChainDb read primitives `range_bytes_capped` + `last_block_bytes`). N-U S3 (`ChainDbServedSource`, DC-NODE-13 — the serve projection being bounded). BLUE `ade_ledger::block_validity::{decode_block, block_header_bytes}` (the single hash/header authority; `DecodedBlock.block_hash` = `blake2b_256(header_cbor)` = the stored hash).

## §4 Intent (invariant impact)
Flip the serve path from the unbounded `iter_from_slot` (full-range Vec + per-block O(N) `SLOT_BY_HASH` scan) to S1's bounded, hash-free primitives, and apply the per-request cap fail-closed. After this slice an untrusted peer's `RequestRange` reads at most `MAX_SERVE_RANGE_BLOCKS` blocks; an oversized range fails closed (reducer → `NoBlocks`) before any unbounded storage/CPU work. The serve derives each block's hash from its own bytes via the single BLUE decode authority — no second hash authority, no `SLOT_BY_HASH` scan on the serve path.

## §5 Scope / What is built (`ade_runtime::network::served_chain_projection`)
- **`const MAX_SERVE_RANGE_BLOCKS: usize = 256`** — the per-request serve cap. Fixed, closed, non-configurable (no CLI/env/config read); symmetric with the receive-side `MAX_WIRE_PUMP_LOOKAHEAD = 256` (DC-LIVEMEM-01). A defensive implementation bound, not a Cardano semantic parameter; may be tightened later.
- **NEW closed enum `ServeRangeOutcome`** = `Served(Vec<(SlotNo, Hash32, Vec<u8>)>)` | `Empty` | `CapExceeded` | `ReadError`. The structured internal reason (all non-`Served` encode to the same wire `NoBlocks`, but the reason is distinct for diagnostics + tests).
- **`fn serve_range(from, to) -> ServeRangeOutcome`** — reads `range_bytes_capped(from.0, to.0, MAX_SERVE_RANGE_BLOCKS)`; **`truncated` → `CapExceeded`** (fail closed BEFORE deriving/serving); otherwise derive each block's hash via `decode_block(bytes).block_hash`, filter to the `[from, to]` `(slot,hash)` key window, and return `Served` / `Empty`. A read/decode error → `ReadError`.
- **`ServedRangeLookup::range_bytes`** — now maps `serve_range` → `Vec` (`Served(v) → v`; every other outcome → empty). Same in-range, byte-verbatim semantics as before for within-cap ranges (DC-CONS-17 preserved); oversized ranges fail closed.
- **`ServedHeaderLookup::next_after`** — bounded read: `range_bytes_capped(from, u64::MAX, 2)` (the answer on the linear extend-only durable chain is provably within the first 2 blocks from the cursor slot), derive hash, return the first block whose `(slot,hash)` key is past the cursor.
- **`ServedHeaderLookup::tip`** — `last_block_bytes()` (O(log N)) + `decode_block` to derive the tip hash + block_no (no `chaindb.tip()` O(N) hash scan, no `get_block_by_hash`).
- **`intersect`** — UNCHANGED (already uses `get_block_by_hash`, which is O(1) — no scan).
- **NEW gate `ci/ci_check_serve_range_bounded.sh`** (CE-2).

## §6 Execution Boundary (TCB color)
- **RED (changed):** `ade_runtime::network::served_chain_projection` (serve cap + derive-hash + fail-closed).
- **BLUE (reused, NOT edited):** `decode_block` / `block_header_bytes` (hash/header authority). No BLUE change.
- **No new canonical type, no schema change, no second hash authority.** `MAX_SERVE_RANGE_BLOCKS` + `ServeRangeOutcome` are RED serve-policy constructs.

## §7 Invariants Preserved
CN-CONS-07 serve provenance (bytes still read verbatim from the durable ChainDb — the cap only bounds HOW MANY, never WHICH); DC-CONS-17 (within-cap serving byte-identical); DC-CONS-18 (single header-projection authority — reused, not duplicated); DC-NODE-13 (serve-as-projection — now bounded). The `intersect` provenance-exact match is unchanged.

## §8 Invariants Strengthened or Introduced
**DC-SERVEMEM-01 → enforced** (tests + `ci_check_serve_range_bounded.sh`). At close: `strengthened_in += "PHASE4-N-AA"` on DC-NODE-13 (serve now bounded) + DC-LIVEMEM-01 (cross-ref: the symmetric serve-side bound).

## §11 Replay / Crash / Epoch Validation
Serve is read-only (advances no tip, admits nothing). Within-cap serving is a deterministic function of (durable chain, request, fixed cap) → byte-identical to the pre-slice behavior. Oversized ranges deterministically `CapExceeded` → `NoBlocks`. No WAL/checkpoint/schema change.

## §12 Mechanical Acceptance Criteria
- `cargo test -p ade_runtime` green incl. NEW unit tests on `ChainDbServedSource`:
  - `serve_range_within_cap_is_served_byte_identical` — a range ≤ cap → `Served`, bytes verbatim, same window as before.
  - `serve_range_over_cap_fails_closed` — a range spanning > `MAX_SERVE_RANGE_BLOCKS` blocks → `serve_range` is `CapExceeded` AND `range_bytes` is empty (distinguished from `Empty`).
  - `serve_range_empty_is_empty_not_capexceeded` — an out-of-chain range → `Empty` (the reason distinction holds).
  - `serve_derives_hash_from_bytes_matches_stored` — the derived `(slot,hash)` keys equal the stored hashes (decode-derived == stored).
  - `serve_tip_via_last_block_bounded` + `next_after_reads_bounded` — tip + next_after return the same answers as the pre-slice (iter-based) projection over a small chain.
- The existing N-U serve unit tests (`empty_chaindb_yields_no_tip_no_next_no_range`, `range_bytes_collects_inclusive_window_in_slot_order`, `range_bytes_excludes_out_of_window_and_stops_past_to`, `intersect_matches_only_a_durable_key`) stay green (within-cap parity). NOTE: the two synthetic-byte range tests use raw non-decodable bytes; since the serve now derives the hash via `decode_block`, they are updated to use real decodable corpus bytes (or are re-pinned to `serve_range`'s pre-decode behavior) — documented in the slice.
- NEW gate `ci/ci_check_serve_range_bounded.sh` green: (a) `served_chain_projection.rs` calls `range_bytes_capped` / `last_block_bytes`, NOT `iter_from_slot`; (b) no `tip()` (the O(N) `chaindb.tip()`) on the serve range/tip path; (c) a fixed `MAX_SERVE_RANGE_BLOCKS` literal is present and NOT sourced from CLI/env/config; (d) hash derived via `decode_block` (no second hash authority).
- Cluster-wide: `cargo test -p ade_runtime -p ade_node` green; full `ci/ci_check_*.sh` sweep 135 + 1 = **136 / 0**.

## §14 Hard Prohibitions
**Inherited (cluster §11):** no unbounded Vec of peer-requested blocks; no per-request full-chain materialization; no per-block `SLOT_BY_HASH` scan in the serve path; no runtime-configurable/unbounded cap; no schema migration; no duplicate block-hash authority (derive via `decode_block`); no BLUE change; no RO-LIVE flip.
**Slice-specific:** the serve range/tip/next_after paths must NOT call `iter_from_slot` or `chaindb.tip()`; the cap must be a compile-time constant; `CapExceeded` must fail closed (empty Vec) BEFORE deriving/serving any block.

## §15 Explicit Non-Goals
Any change to the trusted-recovery `iter_from_slot` (still used by recovery/rollback — out of scope, doc-fenced in S1); per-connection-count / peer-fairness limits (a separate hardening surface, like DC-LIVEMEM-01's note); BlockFetch >64KB segmentation (hardening item 2); the C1 re-run (item 4); RO-LIVE.
