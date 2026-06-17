# DURABLE-ADMISSION-BYTES (slice)

**Status:** code + hermetic tests enforced (DC-WAL-05); live write-half proof =
the C2-PREVIEW-BA02 forge resume.
**Date:** 2026-06-17
**TCB color:** the runner durable-admit + warm-start reads are BLUE-adjacent
durable-authority code (deterministic, no I/O nondeterminism in the decision);
`ChainDb::put_block` itself is the RED storage shell.

## The bug (split admission authority)

Two code paths admit a block durably:

- `pump_block` (DC-NODE-12): `ChainDb::put_block(bytes)` **then**
  `wal.append(AdmitBlock{hash})`. Correct — bytes-first.
- the live admission runner (`run_admission`, `--mode admission`): its
  `ProcessedBlock::Admitted` arm appended `WalEntry::AdmitBlock` **without ever
  calling `put_block`**.

So an admission store carried `AdmitBlock` WAL entries whose block bytes were
never persisted. `warm_start_recovery` then **silently skipped** the missing
bytes (the pre-fix replay-map loop only inserted *present* bytes), so the gap
was invisible — until a fresh keyed forge (`--mode node`, WarmStart) tried to
replay the WAL and failed `BlockBytesMissing`. Reproduced systematically: the
same failing hash across two fresh stores, both **gracefully** stopped — not
unclean-shutdown corruption, a real durability gap.

## The invariant (DC-WAL-05)

Received/followed-block durable-admit is **bytes-first**, symmetric to the
forged path (DC-WAL-04):

1. compute/verify the block hash (`process_block` → `block_hash`)
2. `ChainDb::put_block(StoredBlock{hash, slot, bytes})` — **before** the WAL
3. append `WalEntry::AdmitBlock`
4. a `put_block` failure halts fail-closed `DurableBlockStoreIo` (exit 36)
   **before** step 3 — a WAL admission record can never outlive its bytes
5. `warm_start_recovery` fails closed `DurableBlockBytesMissing{block_hash,
   entry_index, source}` when an `AdmitBlock`'s bytes are absent — corrupted
   durable state, **not** block absence. The prior silent skip is forbidden.

`bytes-without-WAL` stays a tolerable orphan (DC-WAL-04 reconciliation drops
it); `WAL-without-bytes` now halts fail-closed at **both** the write and the
read.

## Memory guardrail (no BA-08 / OP-MEM-02 regression)

Persistence is to the **disk-backed** ChainDb, not a heap map. The live runner
holds **at most one** block's bytes at a time: received → **moved** (not
cloned) into `StoredBlock` → `put_block` → dropped at the admission-step end.
No `Vec<Vec<u8>>` / `<_, Vec<u8>>` map is built in `run_admission`. The
`StaticUtxoFp` / `track_utxo=false` path and the post-bootstrap static-UTxO
drop are **unchanged**. The warm-start replay map is a bootstrap-only recovery
surface, never live admission.

Acceptance bar (all hold):
- no `Vec<Vec<u8>>` or `BTreeMap<_, Vec<u8>>` retained in the live runner
- no WarmStart byte map created during live admission
- `StoredBlock` bytes written to disk, not held
- static UTxO remains dropped after bootstrap; `StaticUtxoFp` path unchanged
- OP-MEM-02 scoped evidence remains valid

## Enforcement

- **Code:** `crates/ade_node/src/admission/runner.rs` (put_block-before-WAL,
  `DurableBlockStoreIo`, move-not-clone); `crates/ade_node/src/node_lifecycle.rs`
  (`warm_start_recovery` per-`AdmitBlock` `get_block_by_hash` →
  `DurableBlockBytesMissing`).
- **Tests (hermetic, real PersistentChainDb + FileWalStore, drop+reopen, no
  injected byte map):**
  - `warmstart_from_real_admission_store_uses_persisted_bytes_no_mock` —
    durable-admit contract → fresh open → recovers, byte-identical preserved
    block.
  - `warmstart_fails_closed_when_wal_admitblock_missing_bytes` — `AdmitBlock`
    without `put_block` → `DurableBlockBytesMissing`.
- **CI:** `ci/ci_check_admission_runner_no_block_byte_map.sh` (memory guardrail).

## Open obligation (live write-half) — DISCHARGED 2026-06-17

The hermetic tests prove the **read** half (warm-start consumes the real
ChainDb bytes / fails closed) and the contract recovers. The **write** half —
the production admission runner actually `put_block`s, end-to-end — was proven by
the C2-PREVIEW-BA02 forge resume:

- Fresh preview admission (fixed binary) at seed-point slot 115030332 reached
  `agreed`, gracefully stopped (store6 = 1.1 GB).
- The fresh WarmStart forge got **past** the durable-bytes stage with **no**
  `BlockBytesMissing` — it built the replay block-bytes map from the ChainDb and
  reached **forward-replay of the followed block at slot 115030409** (~77 slots
  past the seed). The old binary died at `BlockBytesMissing(eb814250…)` on this
  exact path; the fixed binary found the bytes. **`run_admission` persisted them
  → write half proven.**

The same run surfaced a **separate, preview-specific** bug (NOT durable-bytes):
warm-start era-schedule reconstruction hardcodes the **preprod** epoch length
(432000) — `make_node_schedule` (`epoch_length_slots: 432_000`) + the
`epoch_no * 432_000` start in `warm_start_recovery`. On preview (epoch length
86400) the warm-start schedule mismatches the venue-correct schedule the
admission used, so forward-replay rejects slot 115030409 as
`SlotBeforeSystemStart{first_era_start: 574992000 = 1331*432000}`. The import
path (`make_node_schedule(canonical.epoch_start_slot, …)`) is venue-correct;
the warm-start path must match. Tracked as the next slice
(WARMSTART-ERA-SCHEDULE-VENUE); does not affect preprod.
