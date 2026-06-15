# MEM-OPT — Memory-optimization cluster plan

> **Status:** Planning artifact (non-normative). Goal: get Ade's resident memory
> **clearly below** the Haskell cardano-node's on the same preprod chain.

## 0. The gap + the target (measured)

| | RSS (preprod, epoch 295) | source |
|---|---|---|
| Haskell `cardano-node-preprod` (full node) | **5.50 GB** | MEM-COMPARE-D (`51884a78`) |
| Ade `--mode admission` (follow+admit) | **6.56 GB** | MEM-MEASURE-A2 (`c54edb93`) |

Ade is **+1.06 GB (+19%)** while doing *less* work. **Target: ≤ 3.0 GB owned RSS
(aim 2.0–2.5 GB)** — a ~45–55% cut, *clearly* below 5.50 GB. Measured as the
**owned footprint** (`Private_Dirty` / `RssAnon`), not gross `VmRSS`, because the
biggest lever (on-disk UTxO) moves bytes into the reclaimable OS page cache.

## 1. Diagnosis — where the 6.56 GB goes (and why it's beatable)

- **Steady-state structures account for only ~2 GB:** UTxO map `BTreeMap<TxIn,TxOut>` ~0.9–1.3 GB (~2.2M entries; `ade_ledger/src/utxo.rs:62`); 3× stake snapshots ~0.36 GB (`epoch.rs:64`); delegation/rewards ~0.22 GB (`delegation.rs`); pools/gov/pparams/chain_dep < 1 MB each; ChainDb/WAL on-disk (redb/RocksDB), < 50 MB resident; net buffers ~40–80 MB.
- **The ~4 GB gap is almost certainly RETAINED TRANSIENT memory.** The seed import peaks at **~6.8 GB** (3.8 GB JSON buffer + ~3 GB intermediate parsed `RawUtxoMap`, held simultaneously — `ade_runtime/src/seed_import/importer.rs:84`), then drops the parsed map to ~950 MB. But Ade runs on **default glibc malloc**, which keeps freed pages in its per-thread arenas and rarely returns them to the OS → RSS stays pinned near the import peak. This is the dominant, cheapest-to-fix cause.
- **Haskell's 5.50 GB is mostly GC + in-heap UTxO** — a copying collector reserves ~2–3.4× live heap, and the UTxO sits fully in the GHC heap. A Rust, no-GC, on-disk design removes BOTH. IOG's own numbers: mainnet RAM requirement **24 GB (UTxO in-memory) → 8 GB (UTxO on-disk)** — a ~16 GB swing from moving *just the UTxO* to disk (UTxO-HD; cardano-node ≥10.4.1). Notably IOG ships on-disk UTxO but does **not** default it for producers — an opening for a from-scratch Rust node to beat the reference on the same data.

## 2. Primary invariant

**Ade's owned resident memory under a representative preprod follow stays clearly
below the Haskell node's (≤ 3 GB vs 5.5 GB), with ZERO change to ledger semantics,
chain selection, persisted bytes, or replay-equivalence.** Every lever is a
*representation/storage* change behind the **unchanged BLUE authority**, each
proven replay-equivalent and measured with the A2 RSS↔replay-verdict pairing.

## 3. Slices, organized into three clusters (by invariant authority)

The six levers below split into **three clusters** (per "organize by invariant
authority, not feature accumulation"); per-cluster cluster docs live under
`docs/clusters/`. Grounding + sources: `docs/planning/mem-opt-grounding.md`.
- **MEM-OPT-OPS** (authority `OP-MEM-02`): S1 ALLOC · S2 IMPORT · S3 MEASURE —
  operational quick-wins, no BLUE. **ALLOC is S1 here, not a standalone run-ahead.**
- **MEM-OPT-UTXO-DISK** (authority `DC-MEM-05`/`06`/`07`): the on-disk UTxO — a
  new storage authority; the structural lever, highest risk.
- **MEM-OPT-COMPACT** (authority `DC-MEM-08`): compact TxOut + ledger
  sub-structures — the canonical-type/BLUE authority.

The numbered levers (cheapest-bankable first, structural lever in the middle):

1. **MEM-OPT-ALLOC** — swap the global allocator to **mimalloc** (or tuned `tikv-jemallocator`: `background_thread:true`, short `dirty_decay_ms`/`muzzy_decay_ms`, low `narenas`). GREEN/RED (one `#[global_allocator]`; no semantic change — allocation addresses are never fingerprinted). **Banks the retained import peak** → likely the single biggest *quick* RSS drop (6.56 → ~3–4 GB candidate). Do FIRST.
2. **MEM-OPT-IMPORT** — streaming + direct seed import: `serde_json::from_reader` (not `from_slice` on a 3.8 GB buffer), build the canonical UTxO form incrementally, never hold the 6.8 GB peak. RED (the single `importer.rs` chokepoint). Removes the peak so RSS never spikes — complements ALLOC (ALLOC returns a spike that already happened; IMPORT prevents it).
3. **MEM-OPT-UTXO-DISK** — **on-disk UTxO via `redb`** (already a dep, already driving the persistent ChainDb). The structural lever: UTxO bytes live in the reclaimable page cache; Ade's *owned* footprint becomes indices + a bounded in-memory cache + the last-k changelog. Mirrors UTxO-HD (anchor + k-deep in-memory diffs). **BLUE-adjacent + the centerpiece + the highest risk** — gated by §5.
4. **MEM-OPT-TXOUT** — compact `TxOut`: keep the canonical CBOR slice as the single source of truth, drop the duplicated `address`/`coin` (lazy-decode views), store `ShelleyMary` `Value` as raw bytes (parse on access), intern repeated 28-byte policy-ids/credentials, `bytes::Bytes` (≤31 B inline) + `smallvec` for short asset bundles. BLUE canonical type. Shrinks the resident cache.
5. **MEM-OPT-LEDGER** — compact the non-UTxO ledger state: the 3× stake snapshots (~0.36 GB) and delegation/reward maps (interning credentials, compact `Coin`, possibly on-disk for the `go`/`set` snapshots). BLUE.
6. **MEM-OPT-MEASURE** — extend the A1 `rss_sampler` to the **owned** footprint (`Private_Dirty`/`RssAnon`), and add a **CI RSS-regression gate** that asserts Ade ≤ target on a representative run (turns MEM-COMPARE-D from a snapshot into an enforced ceiling + prevents regressions). GREEN/RED; reuses A1/A2.

## 4. Ade-style implementations to exploit (the unfair advantages)

- **`redb` is already a dependency and already proven in-tree** (the persistent ChainDb: CoW B+tree, MVCC, crash-safe). MEM-OPT-UTXO-DISK *reuses proven machinery on a new table* — the biggest lever is half-built.
- **Oracle-seed-then-Ade-owns** ([[feedback_oracle_seed_then_ade_owns]]) — the on-disk UTxO is a **cache of (immutable chain + replay)**, not a new trust root. The WAL + checkpoints stay the authority; the store is replay-reconstructable. Fits Ade's model exactly.
- **Functional Core / Imperative Shell** — the UTxO store is **RED behind the unchanged BLUE ledger interface**: `utxo_lookup`/`utxo_insert` keep their signatures; rules see identical values. The FC/IS partition is what makes an on-disk backend safe.
- **The canonical CBOR encoder** (used for fingerprints/WAL) — encode UTxO keys as **fixed-width big-endian `TxIn` (txid ++ BE index)** so the store's sorted-key iteration *equals* RFC-8949 canonical order; the fingerprint is computed by the canonical encoder, never the store's native iteration.
- **The replay-equivalence harness** (`DC-WAL-03`, `replay_from_anchor`, the boundary/stateful replay corpus) — proves each lever replay-equivalent: same WAL+checkpoint → same tail fingerprint, *backend-independent*.
- **The A1/A2 measurement substrate** (`rss_sampler`, the closed evidence vocab, the RSS↔replay-verdict pairing) — the regression gate is a small extension, not new infra.
- **`TxOut::AlonzoPlus` already keeps `raw` CBOR** (for Aiken script context) — compact `TxOut` is a *narrowing* (drop the duplicated fields), not a rewrite.
- **No GC** — Rust's fundamental edge: no copying-collector 2–3× reserve, free-on-drop, tight `#[repr]`/niche-packed enums. RSS tracks the live working set, not a GC cycle.
- **The closed fixed-bound pattern** (`MAX_SERVE_RANGE_BLOCKS`, `MAX_WIRE_PUMP_LOOKAHEAD`, the A1 admission budgets) — the bounded in-memory UTxO cache reuses the established "fixed, closed, non-configurable bound" idiom.

## 5. Hard invariants preserved + the on-disk-UTxO determinism guards

- **Replay-equivalence (`DC-WAL-03`):** same WAL+checkpoint → same post-state AND same fingerprint, **regardless of UTxO backend**. Every slice carries this as a replay-corpus obligation.
- **Canonical bytes:** the UTxO/ledger fingerprint is computed by the canonical encoder over **canonically-encoded keys**, NEVER from store iteration order.
- **Determinism (BLUE):** no `HashMap`/`HashSet`/float/native-endian key encoding in any authoritative path; ordered iteration only via canonical big-endian keys. (mimalloc/jemalloc affect only allocation addresses — never fingerprinted — so they are determinism-neutral; a CI gate asserts no allocator type leaks into a fingerprint.)
- **Crash-consistency:** per-block, commit `{inputs deleted, outputs inserted, anchor (slot,hash) advanced}` in ONE redb write-txn; keep the durable UTxO anchor behind the k-deep volatile window so rollback ≤ k never rewinds the backing store; the ChainDb's `Durability::None` must NOT be reused for the UTxO commit point (or the store must be reconstructable from the WAL).
- **Ledger semantics:** validation is a pure function of resolved UTxO values; never branches on disk-vs-memory; read-your-writes within a block served by the in-memory changelog overlay.
- **The A2 measurement discipline:** a memory-optimized run must still produce `memory_summary{replay_verdict=agreed}` — low memory that perturbs an authoritative output is invalid.

## 6. Exit criteria (CI-verifiable)
- **CE-OPT-ALLOC/IMPORT:** a committed preprod run with the lever shows owned RSS strictly below the prior committed run, replay verdict `agreed`, 0 diverged.
- **CE-OPT-UTXO-DISK:** the on-disk UTxO passes the full replay corpus byte-identically (`DC-WAL-03` extended); the fingerprint is backend-independent; per-block commit is atomic; a negative test proves a torn commit is rejected. Owned RSS drops to the target band.
- **CE-OPT-MEASURE:** `ci_check_mem_rss_ceiling.sh` (or extend `ci_check_mem_compare_evidence.sh`) asserts the committed run's owned RSS ≤ target and `verdict=ade_below` — the BA-08 *win* recorded honestly, mechanically gated.
- **CE-OPT-close:** the comparison artifact flips `verdict` from `ade_heavier` to `ade_below`; registry records the new operational standing.

## 7. Open questions
- **OQ-OPT-1 (sequencing):** land ALLOC + IMPORT first (cheap, possibly enough to clear 5.50 GB) and *re-measure* before committing to the big UTXO-DISK slice? *Lean: yes — measure after the cheap levers; UTXO-DISK is the floor, not necessarily needed to merely beat 5.50 GB, but is the "clearly below + scales to mainnet" lever.*
- **OQ-OPT-2 (UTxO-DISK shape):** redb table `TxIn→TxOut` with the full set on disk + a k-deep in-memory changelog (UTxO-HD style) vs a simpler write-through bounded LRU cache over redb. *Lean: changelog overlay — it's the replay-equivalent, rollback-cheap design.*
- **OQ-OPT-3 (owned-RSS metric):** standardize on `Private_Dirty` (from `/proc/self/smaps_rollup`) as the gated number; report gross `VmRSS` alongside for transparency.
- **OQ-OPT-4 (mainnet):** the cluster targets preprod; confirm the on-disk design scales to mainnet's ~10–15M UTxO without an owned-footprint blow-up (the whole point of UTxO-HD).
