# Slice MEM-OPT-UTXO-DISK S2 — on-disk / bounded in-memory UTxO backend (CE-UD-2)

> **Status:** **S2 COMPLETE (2026-06-16) — owned-RSS win achieved via PATH A; formal `/cluster-close` deferred.** Move the UTxO off the anonymous heap (the S0 live-working-set finding: ~2.8 GiB re-established every block). Two phases: **S2a** (overlay representation, BLUE, in-memory — de-risks the clone-model change) ✅ **DONE** → **S2b** ✅ **DONE via PATH A**: because the live admission path runs `track_utxo=false` (it never mutates the UTxO per block), the owned-RSS win came from `StaticUtxoFp` + dropping the retained in-memory static UTxO after snapshot durability (**1.94 GiB < Haskell 2.57 GiB**; `OP-MEM-02` enforced, `c64ccbfa`), **NOT** from activating the on-disk redb LIVE backend. The redb anchor + bounded overlay/cache + pre-resolve infra is **built + proven as preparation for `LIVE-LEDGER-APPLY` / `track_utxo=true` (B), OWED.** See `cluster.md` CE-UD-2 + §11.
> **Cluster:** MEM-OPT-UTXO-DISK (`DC-MEM-05` backend-independent replay + `DC-MEM-07` bounded in-memory) · **Prior:** S1 (owned `utxo_lookup`, `103361c1`), S1.5 (incremental fingerprint + cutover, `aea2eba3`)

## 2. Slice Header
### Cluster Exit Criteria Addressed
- [ ] **CE-UD-2** (on-disk UTxO backend): the replay corpus runs **byte-identically under both backends** (`DC-MEM-05`); the in-memory portion (overlay + cache) is **fixed-bounded** (`DC-MEM-07`); per-block commit is **atomic** (torn-commit rejected); **cache eviction cannot alter an authoritative output**; redb key order is **proven** == canonical `TxIn` order (or a fixed-width key); **owned RSS drops** toward the target.

### Intent
S0 proved the admission footprint is a **live working set** (re-accumulates after a forced collect). S1 made the lookup owned; S1.5 made `post_fp` O(delta)/block. S2 finally moves the UTxO **off the heap**: the authoritative set lives in an on-disk redb table (the **anchor**), with a **bounded in-memory overlay** (the last-k blocks' diffs) + a **bounded read cache** in front. The BLUE ledger interface (the S1 `UtxoStore` seam) is unchanged; validation is a pure function of resolved values and never branches on disk-vs-memory.

## 3. The Design
- **Anchor:** the durable UTxO set. S2a: an in-memory `Arc`-shared immutable map (the BTreeMap, now shared not cloned). S2b: a redb table `TxIn→TxOut` (canonical key bytes), on disk.
- **Overlay:** a **bounded** in-memory changelog — the inserts + deletes since the anchor (the last-k blocks, k = the security parameter). Mutations append here (O(delta), NEVER a full-map clone). A clone shares the `Arc` anchor + clones the small overlay (O(overlay), not O(n)).
- **Read cache:** a **bounded**, non-authoritative read-through LRU over the anchor (S2b — absorbs hot disk reads). Eviction changes NO authoritative output (a miss re-reads disk; the resolved value is identical).
- **Lookup:** overlay → cache → anchor. Resolves to an owned `TxOut` (the S1 interface).
- **Mutation:** `utxo_insert`/`utxo_delete` append to the overlay (a tombstone for a delete). O(1) amortized; no full clone.
- **Compaction:** when the overlay exceeds its bound (> k deep), the oldest diffs fold into the anchor (S2b: a redb write-txn) and drop from the overlay — keeping the in-memory portion bounded (`DC-MEM-07`).
- **Fingerprint:** `post_fp` is the S1.5 `IncrementalUtxoFp` maintained from the per-block overlay delta (O(delta)) — NOT a full-anchor scan. The full recompute (`fingerprint_v2`) is the checkpoint oracle.

## 4. Phases (de-risk the BLUE clone-model change before the disk swap)

### S2a — overlay representation (BLUE; in-memory; the clone-model change)
- Replace `UTxOState { utxos: BTreeMap }` with an overlay-capable representation: an `Arc`-shared anchor + a bounded in-memory overlay; route **ALL** access through the `UtxoStore` seam (extend it with `insert`/`remove`/`iter`/`len`, not just `get`) — the 39 direct `.utxos.` sites (21 insert / 7 get / 4 remove / 4 len) move behind it.
- `utxo_insert`/`utxo_delete` become **overlay-append** (O(1)); clone becomes O(overlay) (share the `Arc` anchor). The rollback's whole-ledger clone (`rollback/in_memory_cache.rs`) becomes cheap.
- **Proof:** byte-identical replay + identical fingerprints + identical verdicts/errors vs the pre-S2a BTreeMap (the same proof discipline as S1). NO disk yet; the anchor is an in-memory `Arc<BTreeMap>`.
- **Bounded overlay** introduced but the anchor is still fully in memory — so this is NOT yet the owned-RSS win (`DC-MEM-07` partial). It de-risks the representation + clone-model change in isolation.

### S2b — on-disk redb anchor (RED; the owned-RSS win)
- The anchor moves to a redb table `TxIn→TxOut` (reuse the proven `PersistentChainDb` machinery). Lookup: overlay → bounded read cache → redb. Compaction flushes the overlay to redb in **one write-txn** per commit (atomic; the durable anchor stays behind the k-deep window so a rollback ≤ k never rewinds the store).
- **Key order — PROVEN, not assumed:** redb's sorted-key iteration must equal the canonical `TxIn` order. A **test-vector gate** pins it; if any CBOR integer-width/array-prefix encoding perturbs ordering, use an explicit **fixed-width key `32-byte txid ++ BE-u32 index`** (safer for storage order).
- **Proof (DC-MEM-05):** the SAME replay sequence under the BTreeMap anchor (S2a) AND the redb anchor (S2b) → identical UTxO fingerprints, WAL/checkpoint fingerprints, replay verdicts, and structured errors. **owned RSS drops** (the anchor is now file-backed/off-heap; only the bounded overlay + cache are anonymous). Re-run the S0/S3 owned measurement → the active-admission owned should fall toward the bounded overlay+cache size.

## 5. Tier Classification
- **true:** replay outputs byte-identical across backends; validation never branches on disk-vs-memory; the cache + eviction never alter an authoritative output.
- **derived (`DC-MEM-05`/`DC-MEM-07`):** backend-independent replay; bounded in-memory portion (fixed, closed constants for k + the cache cap).
- **operational (`OP-MEM-02`):** S2b is where the owned-RSS posture can finally improve; flip `OP-MEM-02` only if the owned metric is **clearly below** the target (the honest S3 discipline).
- **release:** the replay corpus runs under both backends in CI; a CI gate asserts the bounds + the key-order vector.

## 6. Invariants
- **`DC-MEM-05`** (the load-bearing one): same WAL + checkpoint → byte-identical post-state AND fingerprint, regardless of backend. The replay corpus runs under BOTH.
- **`DC-MEM-07`:** the overlay (k-deep) + the read cache are bounded by fixed, closed, non-configurable constants; memory pressure cannot grow them.
- **`DC-MEM-06`** (strengthened): the on-disk store's sorted-key iteration equals canonical `TxIn` order by key construction (proven by vector), never relied on natively.
- **`DC-MEM-10`** (S1.5): `post_fp` is the incremental v2 fingerprint, maintained from the per-block delta — O(delta), so the on-disk anchor is never scanned per block.
- **`DC-WAL-03`:** replay-equivalence preserved.

## 7. Mechanical Acceptance Criteria
- [x] **S2a DONE:** `OverlayUtxo` (Arc anchor + bounded overlay); all 75 `.utxos.` sites (39 prod) routed via the BTreeMap-shaped borrowing API + `UTxOState::from_map`; validation resolves through `&impl UtxoStore` / `UtxoMembership` (test fixtures keep `BTreeMap`); `utxo_insert`/`delete` are overlay-append (clone is O(overlay), no full-map clone — `ci_check_overlay_utxo_s2a.sh`); byte-identical replay + fingerprints + verdicts + errors vs pre-S2a (boundary_fingerprint_agreement + differential_utxo_set_equality + orchestrator_replay_equivalence + s2a_overlay_split_fingerprints_identically_to_direct_build all green).
- [ ] **S2b:** the redb anchor + bounded overlay + bounded read cache; the replay corpus byte-identical under BTreeMap AND redb (`DC-MEM-05`); per-block atomic commit (a torn commit rejected — negative test); cache eviction proven non-authoritative; redb key order == canonical `TxIn` (test vector) or the fixed-width key; the bounds are fixed closed constants (`DC-MEM-07`).
- [ ] **owned RSS** re-measured (the S3 scenario) — active-admission owned falls toward the bounded overlay+cache size; the honest comparison regenerated.
- [ ] `cargo test` green under both backends; `ci_check_*` gates green.

## 8. Hard Prohibitions
- **No BLUE behavioral change** — validation reads resolved owned values; it never branches on disk-vs-memory; the same WAL → the same post-state + fingerprint on both backends.
- **No unbounded in-memory growth** (`DC-MEM-07`) — fixed, closed constants for k + the cache; no config knob.
- **The read cache is NON-AUTHORITATIVE** — a miss re-reads the anchor; eviction never changes an output. Proven.
- **No `OP-MEM-02` flip from gross VmRSS** — only the owned metric clearly below target.
- **The fingerprint is the S1.5 incremental v2** — the on-disk anchor is NEVER scanned per block.
- **Key order PROVEN, not assumed** — the test-vector gate, else the fixed-width key.

## 9. Explicit Non-Goals
- Not compact `TxOut` (MEM-OPT-COMPACT, `DC-MEM-08`).
- Not a Mithril/genesis backend change.
- Not mainnet-scale tuning (preprod first; OQ-UD-2 confirms mainnet scaling separately).

## 10. Completion Checklist
- [x] **S2a DONE** (2026-06-16): overlay representation + seam completion + the clone-model change; replay-equivalence proven; bounded overlay (DC-MEM-07 → `partial`, anchor still in memory). `ci_check_overlay_utxo_s2a.sh` green.
- [ ] **S2b:** redb anchor + bounded cache; DC-MEM-05 both-backends corpus; atomic-commit + torn-commit negative test; cache-non-authoritative proof; key-order vector; owned-RSS re-measure.
- [ ] `DC-MEM-05`/`DC-MEM-07` enforced; `DC-MEM-06` strengthened; CE-UD-2 met → MEM-OPT-UTXO-DISK ready for /cluster-close.
