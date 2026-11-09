# Invariant Slice — PHASE4-N-J S8

## Slice Header

**Slice Name:** `PersistentSnapshotCache` + close DC-CONS-21
**Cluster:** PHASE4-N-J
**Status:** In Progress
**CEs addressed:** CE-N-J-8
**Registry effects on merge:**
  * DC-CONS-21 → `enforced`, `open_obligation` removed,
    `strengthened_in = ["PHASE4-N-J"]`.
**Dependencies:** S7 (`framing::{encode,decode}_snapshot`) + N-D
  (`chaindb::SnapshotStore`) + N-I (`SnapshotReader` trait).

---

## Intent

The bridge from S7's combined-snapshot framing to any
`SnapshotStore` impl. Closes PHASE4-N-I's carry-forward
DC-CONS-21 open obligation.

* `ade_runtime::rollback::persistent_cache::PersistentSnapshotCache`
  — `SnapshotReader` impl wrapping `&'a S: SnapshotStore + ?Sized`.
  Holds no in-memory state; safe to instantiate per lookup.
* Reader path (`nearest_le(target_slot)`):
  1. `store.list_snapshot_slots()` (ascending; trait contract)
  2. Find largest slot ≤ target via `.iter().rev().find(...)`
  3. `store.get_snapshot(slot)` → bytes
  4. `framing::decode_snapshot(&bytes)` → `(LedgerState,
     PraosChainDepState)`
  5. Decode failures yield `None` — the reader treats a
     corrupt/missing snapshot as "no usable snapshot here",
     never panics. Loud surface (the closed `PersistentCacheError`
     sum) is the writer-side error type.
* Writer path (`capture(slot, ledger, chain_dep)`):
  encode_snapshot → `put_snapshot`. Returns
  `PersistentCacheError::Encode` on pre-Conway era,
  `PersistentCacheError::Store` on storage failure.
* `pub const PERSISTENT_CACHE_SCHEMA_VERSION: u32` mirrors
  `framing::SCHEMA_VERSION` so out-of-crate consumers (e.g.
  ops dashboards) can pin the cache wire version without
  importing the BLUE crate.

### Cross-impl equivalence test

`persistent_cache_matches_in_memory_cache_semantics` probes the
persistent reader and the in-memory reader at 10 query points
across 4 stored snapshots; every (slot, ledger, chain_dep)
triple must agree — including the edge cases (probe < oldest
slot returns None on both, probe > newest slot returns newest
on both, probe == stored slot returns that exact entry on both).

### Registry effect

DC-CONS-21 moves from `declared` + `open_obligation =
"persistent_ledger_snapshot_encoding_follow_on_cluster"` to
`enforced` with that obligation removed. `tests`, `code_locus`,
and `ci_script` (`ci/ci_check_snapshot_encoder_closure.sh` —
same gate that flipped DC-STORE-08/09/CN-STORE-08 in S7)
populated. `strengthened_in = ["PHASE4-N-J"]` records the
closure cluster.

---

## §12 Mechanical Acceptance Criteria

- `persistent_cache_capture_then_nearest_le_round_trips`
  — 3 snapshots @ slots 100/200/300; probes 250 → 200,
  300 → 300, 99999 → 300, 50 → None.
- `persistent_cache_matches_in_memory_cache_semantics`
  — cross-impl reader equivalence at 10 probe points.
- `persistent_cache_empty_store_returns_none`
  — empty store reader returns None for any probe.
- `persistent_cache_rejects_pre_conway_on_capture`
  — Babbage `LedgerState` → `Encode(EraNotSupported)`.
- `persistent_cache_corrupt_bytes_yields_none_from_reader`
  — corrupt-bytes write + reader probe → None.
- `persistent_cache_schema_version_mirrors_framing`
  — pinned `PERSISTENT_CACHE_SCHEMA_VERSION ==
  SCHEMA_VERSION`.

---

## §14 Hard Prohibitions

- No HashMap/HashSet/wall-clock/tokio/rand/float literals in
  `rollback::persistent_cache`.
- No alternate decoder path inside the cache — every byte payload
  flows through `framing::decode_snapshot` (CN-STORE-08).
- No `unwrap()` / `panic!()` on the read path — decode failures
  surface as `None`, encode failures as `PersistentCacheError`.

---

## §15 Explicit Non-Goals

- Snapshot eviction (out of N-J scope per the invariants sketch).
- Cadence integration into the live receive loop —
  `snapshot_writer::maybe_capture_snapshot` (PHASE4-N-I) stays
  in-memory; a separate orchestrator-level slice would wire the
  cadence policy to the persistent writer.
- Cross-impl byte equivalence against a reference cardano-node
  ledger snapshot — Ade's snapshot wire format is project-internal
  (DC-LEDGER-internal-byte-authority); cross-impl agreement at the
  byte level is non-goal.
