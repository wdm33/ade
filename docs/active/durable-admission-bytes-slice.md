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

## Open obligation (live write-half)

The hermetic tests prove the **read** half (warm-start consumes the real
ChainDb bytes / fails closed) and the contract recovers. The **write** half —
the production admission runner actually `put_block`s, end-to-end — is proven by
the C2-PREVIEW-BA02 forge resume: a real preview admission writes the store →
a fresh WarmStart forge recovers it with **no** `BlockBytesMissing`. That run
can only succeed if `run_admission` persisted the bytes.
