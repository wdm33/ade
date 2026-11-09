# MEM-OPT grounding — Ade memory map + Cardano memory research

> Load-bearing reference for the memory-optimization clusters. Captures the
> diagnosis the cluster/slice docs cite (so they don't re-derive it). Measured
> baseline: Ade 6.56 GB vs Haskell `cardano-node-preprod` 5.50 GB on the same
> preprod chain (MEM-COMPARE-D, `docs/evidence/mem-compare-d-preprod.{jsonl,md}`).

## A. Ade's in-memory consumers (where the 6.56 GB goes)

| Consumer | Type · file:line | Steady-state estimate | Lever |
|---|---|---|---|
| **UTxO set** | `UTxOState{ utxos: BTreeMap<TxIn,TxOut> }` · `crates/ade_ledger/src/utxo.rs:62` (~2.2M preprod entries) | ~0.9–1.3 GB | on-disk (redb) · compact TxOut · packed TxIn |
| `TxOut` enum | `utxo.rs:16-34` — `AlonzoPlus{ raw:Vec<u8>, address:Vec<u8>, coin }` (raw is full output CBOR; address+coin **duplicated** out of raw for O(1) — `utxo.rs:21-33`); `ShelleyMary{ address:Vec<u8>, value:Value }` (`Value`/`MultiAsset` = nested `BTreeMap`, `crates/ade_ledger/src/value.rs:22,50`) | (in the row above) | drop duplicated fields + lazy-decode; store `Value` as raw bytes |
| `TxIn` key | `crates/ade_types/src/tx.rs:32-37` — Hash32[32]+u16 = 34 B fixed | (in the row above) | fixed-width big-endian key encoding |
| **3× stake snapshots** | `SnapshotState` · `crates/ade_ledger/src/epoch.rs:64-68` (mark/set/go; `BTreeMap<Hash28,(PoolId,Coin)>` ×3, ~1.5M creds) | ~0.36 GB | intern credentials; compact `Coin`; on-disk for go/set |
| Delegation/rewards | `DelegationState`/`CertState` · `crates/ade_ledger/src/delegation.rs:24-46` (registrations/delegations/rewards `BTreeMap`s) | ~0.22 GB | intern; compact |
| Pools / gov / pparams / chain_dep | `delegation.rs:73`, `state.rs:82`, `pparams.rs:21`, `crates/ade_core/src/consensus/praos_state.rs:115` | < 1 MB each | — |
| ChainDb / WAL / snapshots | `crates/ade_runtime/src/chaindb/{mod.rs:56,persistent.rs}` (redb/RocksDB, **on-disk**); `FileWalStore` append-only | < 50 MB resident | already on-disk |
| Network buffers | `crates/ade_network/src/mux/transport.rs:63,70,102` (per-peer 16 KB + reassembly) | ~40–80 MB | already bounded |
| Allocator | **default glibc `System`** (no jemalloc/mimalloc anywhere) | fragmentation + **retention** | swap allocator |

**Steady-state structures sum to ~2 GB. The observed 6.56 GB ⇒ ~4 GB is retained
transient memory.** The seed import (`crates/ade_runtime/src/seed_import/importer.rs:84-107`,
the SOLE import authority) peaks at **~6.8 GB**: `fs::read` the 3.8 GB JSON + `serde_json`
parse into an intermediate `RawUtxoMap` (~3 GB, String keys) held **simultaneously**, then
iterate into the canonical `BTreeMap` and drop the intermediate (final ~950 MB). **glibc
malloc keeps the freed pages in its per-thread arenas and rarely returns them to the OS**,
so RSS stays pinned near the import peak. This — not the live structures — is the dominant
cost, and it is the cheapest to fix (allocator decay + streaming import).

**UPDATE (MEM-OPT-OPS S1+S2 landed).** S1 (mimalloc) returns the retained peak; S2
(streaming import) prevents it — the streaming import peak measured **3.25 GB** (vs
~6.8 GB whole-buffer; `seed_import` VmHWM tap, byte-identical UTxO). With the import
peak gone, **the dominant remaining memory event is `seed_to_snapshot`** (`importer`/
bootstrap step 4): serializing the recovered ~1.9M-entry UTxO into a **~4 GB `chain.db`**
(run VmHWM ~8 GB transient), after which the redb `chain.db` **mmaps into gross `VmRSS`**
(~6.9 GB observed; clean, file-backed, reclaimable). This is the next target — likely
folds into MEM-OPT-UTXO-DISK — and is why the **owned** footprint (`Private_Dirty`, S3),
which excludes the reclaimable mmap, is the metric that matters, not gross `VmRSS`.

**UPDATE (MEM-OPT-OPS S3 — the decisive owned measurement).** `RssAnon ≈ VmRSS` at
every point — Ade's resident memory is almost entirely **owned anonymous heap**, so
the S2 "chain.db mmap pollutes gross VmRSS" hypothesis was **wrong**: redb's admission
cost is **anonymous write buffers** + the `seed_to_snapshot` serialization, counted in
`RssAnon` (not a reclaimable mmap). Owned footprint: **idle/recovered 1.95 GiB** (below
the ≤3 GB target — S1+S2 import-side wins are real) but **active-admission 4.59 GiB**
(p50). Honest owned comparison: Ade 4.59 GiB vs the Haskell node's windowed owned
**2.57 GiB** (GC-variable 2.57–3.95) → **`ade_heavier`** — the OPPOSITE of the gross
signal. **MEM-OPT-OPS (allocator + streaming import) does NOT clear the preprod owned
posture; the `seed_to_snapshot`/`chain.db` serialization is the gating lever → MEM-OPT-UTXO-DISK.**

## B. Cardano (Haskell) memory research — how 5.50 GB happens + UTxO-HD

- **The Haskell 5.50 GB is mostly GC + in-heap UTxO.** GHC's copying collector reserves
  ~2–3.4× live heap (one SPO datapoint: 5.9 GB RSS vs 1.74 GB live ⇒ RSS ≈ 3.4× live;
  `--nonmoving-gc` cut a relay 6 GB→4 GB). The UTxO set sits fully in the GHC heap.
  Sources: <https://forum.cardano.org/t/solving-the-cardano-node-huge-memory-usage-done/67032> · <https://github.com/IntersectMBO/cardano-node/issues/3216>
- **UTxO-HD (the on-disk UTxO precedent).** ouroboros-consensus pulls the UTxO out of the
  in-RAM `NewEpochState` into a `LedgerDB` with `LedgerTables (TxIn→TxOut)`: the full set in a
  **backing store** (LMDB, mmap'd) at/below the immutable tip; the last-`k` blocks' diffs held
  **in memory** (`DbChangelog`, FingerTree of `DiffMK`); periodic **flush** pushes diffs ≤ the
  immutable tip into the store; rollback ≤ `k` is changelog-trimming and never rewinds the store;
  reads forward backing-store values through the changelog so validation sees the tip.
  Sources: <https://ouroboros-consensus.cardano.intersectmbo.org/docs/references/miscellaneous/utxo-hd/> (+ `/utxo-hd_in_depth`, `/migrating`)
- **The RSS swing:** mainnet RAM requirement **24 GB (UTxO in-memory) → 8 GB (UTxO on-disk)** —
  ~16 GB from moving *just the UTxO* to disk. The in-memory UTxO-HD backend is ~memory-neutral
  vs legacy (savings come ONLY from the on-disk backend). LSM-backend mainnet replay reportedly
  needed only 4 GB. **Default since cardano-node 10.4.1 — but the default backend is in-memory;
  producers are told to use only the in-memory backend.** ⇒ a from-scratch Rust node can make
  on-disk-UTxO the default and beat the Haskell *producer* on the same data.
  Sources: <https://developers.cardano.org/docs/operate-a-stake-pool/basics/hardware-requirements/> · <https://github.com/IntersectMBO/cardano-node/releases/tag/10.4.1> · <https://updates.cardano.intersectmbo.org/reports/2025-05-performance-10.4.1/> · <https://iohk.io/en/blog/posts/2025/10/29/strengthening-cardanos-foundations-q3-2025-progress-report/>
- **Compact TxOut (Haskell ledger).** `Compactible`/`CompactForm`: `Coin`=`Word64`, `CompactAddr`=
  one inlined `ShortByteString`, compact `Value` packs the multi-asset bundle into one
  `ShortByteString`; the ada-only-to-key-hash case skips the bytestring entirely. Parsed views
  reconstructed lazily, never written back. `MemPack` flat-buffer serialization (~2× faster).
  Sources: cardano-ledger `Cardano/Ledger/{Compactible,Address}.hs`, Mary `Value.hs`, Babbage `TxOut.hs`; PR <https://github.com/IntersectMBO/cardano-ledger/pull/4811>
- **UTxO sizing:** mainnet ~10–15M entries (design target 100M); preprod < 1M; ~94 B/entry
  serialized (34 B key + 60 B value) in the compact/LSM form, materially larger in-heap.

## C. Rust / Ade-native levers + the determinism guards

- **On-disk UTxO is the single biggest lever** and **`redb` is already a dependency** driving
  the persistent ChainDb (`crates/ade_runtime/src/chaindb/persistent.rs` — CoW B+tree, MVCC,
  crash-safe). An mmap/page-cache-backed store puts UTxO bytes in the **reclaimable OS page
  cache** (shared, clean), so Ade's *owned* footprint becomes indices + a bounded cache +
  changelog. (A 1.7 GB LMDB DB showed "0 Dirty" pages, reclaimed to ~11 KB under pressure —
  <https://blogs.kolabnow.com/2018/02/13/using-and-abusing-memory-with-lmdb-in-kube>.) Prefer
  redb/`heed`(LMDB) for ordered iteration; avoid sled (high RSS) + hash-partitioned layouts
  (no total order). <https://github.com/cberner/redb>
- **Allocator (cheapest lever):** mimalloc (~50% lower RSS in its own bench; Meilisearch/
  rust-analyzer migrated) or `tikv-jemallocator` with `background_thread:true` + short
  `dirty_decay_ms`/`muzzy_decay_ms` + low `narenas` → returns freed memory to the OS.
  `bumpalo` for per-block transient scratch (reset each block; plain-data only, skips `Drop`).
  <https://docs.rs/crate/jemalloc-sys/latest/source/jemalloc/TUNING.md> · <https://www.meilisearch.com/blog/memory-leak-investigation>
- **Compact in-memory:** `bytes::Bytes` (≤31 B inline, no heap), `smallvec`/`tinyvec` for short
  asset bundles + signer lists, intern repeated 28-byte policy-ids/credentials.
- **No GC** — the fundamental Rust edge: no copying-collector reserve, free-on-drop (RSS tracks
  the live set), niche-packed `#[repr]`. This is *why* a Rust node can target far below the RTS.
- **Determinism guards (BLUE on-disk path) — the de-risking decisions:**
  - **Fingerprint from the canonical CBOR encoder over canonical keys, NEVER store iteration.**
  - **Fixed-width big-endian `TxIn` keys** (txid ++ BE index) so store sorted-key order == RFC-8949
    canonical order (<https://www.rfc-editor.org/rfc/rfc8949.html> §4.2.1). Native-endian keys
    (LMDB `MDB_INTEGERKEY`) are a footgun; ledger snapshots are already non-portable across arches.
  - **Per-block atomic commit** `{inputs deleted, outputs inserted, anchor (slot,hash) advanced}`
    in ONE redb write-txn; durable UTxO anchor kept **behind** the k-deep volatile window so
    rollback ≤ k never touches the store. (The ChainDb's `Durability::None` must NOT be reused for
    the UTxO commit point — or the store must be reconstructable from the WAL.)
  - **No `HashMap`/`HashSet`/float** in any authoritative path; allocator type never fingerprinted.
  - **The on-disk UTxO is a cache of (immutable chain + replay)** — fits `oracle-seed-then-Ade-owns`;
    the authority is the chain (WAL + checkpoints), proven by `DC-WAL-03`/`replay_from_anchor`.

## D. Target + ranked levers

**Target: ≤ 3.0 GB owned RSS (`Private_Dirty`/`RssAnon` via `/proc/self/smaps_rollup`), aim
2.0–2.5 GB**, vs Haskell 5.50 GB — clearly below (~45–55%). Lean Rust owned footprint with
on-disk UTxO + bounded cache + compact encoding + tuned allocator: 0.5–1.5 GB (gross VmRSS
1–3 GB incl. reclaimable page cache).

Ranked (biggest first): **(1) on-disk UTxO (redb)** — wins the comparison + scales to mainnet ·
**(2) allocator swap** — cheapest, banks the retained import peak (do first) · **(3) streaming
import** — removes the peak · **(4) compact TxOut** · **(5) compact ledger sub-structures** ·
**(6) bounded caches + no-GC structural wins**.
