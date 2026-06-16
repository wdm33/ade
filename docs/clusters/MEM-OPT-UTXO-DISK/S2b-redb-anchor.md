# Slice MEM-OPT-UTXO-DISK S2b — on-disk redb UTxO anchor (pre-resolve architecture)

> **Status:** **DONE — built + proven as INFRASTRUCTURE FOR B (`LIVE-LEDGER-APPLY` / `track_utxo=true`), NOT the live backend for the current `track_utxo=false` path.** The on-disk **redb** `UtxoAnchor` (CoW `TxIn→TxOut`) + backend-equivalence (`253ee718`, gated by `ci_check_utxo_disk_anchor.sh`), behind a **pre-resolve** boundary, is complete. It is **NOT** the mechanism that delivered the BA-08 owned-RSS win — the live path is `track_utxo=false`, so that win came from PATH A (`StaticUtxoFp` + dropping the in-memory static UTxO; see `S2b-2cA-static-utxo.md`). Activating this anchor as the LIVE UTxO backend is **B's scope (OWED).** **Prior:** S2a (overlay representation, `252580d5`), S2b key codec (`6dc31213`).
> **Cluster:** MEM-OPT-UTXO-DISK · DC-MEM-05 (backend-independent replay) + DC-MEM-06 (key order) + DC-MEM-07 (bounded in-memory).

## Intent
Move the UTxO anchor out of owned heap while preserving **replay-equivalent validation** and WAL/checkpoint behavior. The S0 finding: the active-admission UTxO is a live working set (~2.8 GiB on-heap); offloading the anchor to disk is the lever.

## The decision: PRE-RESOLVE (lazy redb reads from BLUE are REJECTED)
Lazy disk reads inside the validation `get` path are tempting (simpler) but **violate the FC/IS boundary**: validation would become capable of causing filesystem I/O. That is not an internal optimization — it changes the authority model. Instead:

1. **RED shell** pre-resolves the required TxIns from redb (+ the unflushed overlay).
2. **GREEN/BLUE boundary** passes the resolved working-set INTO validation.
3. **BLUE validation** reads only resolved in-memory state.
4. **RED shell** commits accepted deltas atomically to redb.

### Authority shape
- **redb anchor = RED persistent storage authority.**
- **resolved overlay/cache = deterministic validation input.**
- **BLUE validation = pure consumer of already-resolved UTxO facts.** BLUE may ask *does this TxIn exist?* / *what is its TxOut?* but MUST NOT decide how to fetch it from disk.

## Boundary (TCB colors)
- **BLUE:** the validation interface consumes a **resolved UTxO view only** (the S2a `UtxoStore`/`UtxoMembership` seam, backed by an in-memory `BTreeMap`/overlay — never the storage backend).
- **GREEN:** the deterministic pre-resolve plan + the backend-equivalence checks.
- **RED:** redb reads/writes, cache fill/eviction, compaction, atomic commit.

## Guardrail (load-bearing)
Do **not** call the redb adapter from ledger validation, even indirectly through a trait object. The trait exposed to BLUE represents a **resolved view**, not a storage backend. The redb anchor type does NOT impl `UtxoStore`.

## Pre-resolve completeness
Pre-resolve must include **every UTxO class validation can need**, or a missing dependency creates hidden lazy-fetch pressure to reintroduce disk reads in BLUE:
- spending inputs
- collateral inputs
- reference inputs
- script/context UTxO dependencies

## Cache rule
The read cache is allowed only as **RED/GREEN performance state** — NOT authoritative. Eviction must not affect validation outputs. Proof: a test where cache capacity changes **or the cache is cleared** and the verdict + fingerprint remain identical.

## Commit rule
Per-block commit is a **single redb write transaction**: delete spent inputs → insert produced outputs → advance anchor metadata / block point / fp version → commit. On crash or torn commit: **old anchor valid OR new anchor valid, never half-applied** authoritative state.

## Proof obligations (DC-MEM-05 backend equivalence)
For the same seed + block sequence, `BTreeMap anchor + overlay` == `redb anchor + pre-resolved overlay/cache`:
- same input-present verdicts
- same resolved TxOuts
- same ledger validity verdicts
- same UTxO fingerprint
- same incremental fingerprint
- same WAL post_fp
- same replay verdict
- same structured failure for missing/corrupt anchor entries

Plus DC-MEM-06: redb iterates the fixed-width key (`6dc31213`) in canonical TxIn order; DC-MEM-07: the overlay + read cache are fixed-bounded. Then re-measure owned RSS (S0/S3 scenario) — the active-admission owned should fall toward the bounded overlay+cache size (the bounty win).

## Build order
1. **redb UTxO anchor adapter (RED)** — table `[u8;36] -> canonical TxOut bytes`; `read` / ordered `iter` / atomic `commit_block(spent, produced)`; wires the S2b key codec. **+ backend-equivalence test** (BTreeMap anchor == redb anchor on the same deltas → identical resolved values + iteration + UTxO fingerprint).
2. **pre-resolve plan (GREEN)** — enumerate every required TxIn class from a block; resolve into an in-memory `BTreeMap` working-set (a `UtxoStore`).
3. **bounded read cache (RED/GREEN)** — non-authoritative; eviction/clear/capacity-change proven output-invariant.
4. **live integration + atomic commit** — wire into admission; per-block single write-txn; torn-commit negative test.
5. **owned-RSS re-measure** — the S0/S3 scenario; honest comparison regenerated.
