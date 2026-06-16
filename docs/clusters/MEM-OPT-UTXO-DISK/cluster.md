# Cluster MEM-OPT-UTXO-DISK — storage-backed UTxO memory authority surface

**Primary invariant:** `DC-MEM-05` (the UTxO/ledger fingerprint + post-state are **independent of the storage backend** — an in-memory and an on-disk UTxO produce byte-identical replay; a storage change is NEVER a consensus/replay change) + `DC-MEM-07` (the in-memory portion is bounded by fixed, closed constants). The cluster will also **strengthen `DC-MEM-06`** (close the store-iteration-order clause that MEM-OPT-OPS S1 left `declared`) once a backing store exists.

**Scope framing (honest — not a foregone redesign):** **MEM-OPT-UTXO-DISK owns the storage-backed UTxO memory authority surface. Its first slice is diagnostic (S0) and decides whether the next mergeable work is a contained snapshot-streaming fix or the full bounded in-memory UTxO backend.** The cluster does NOT pre-commit to an on-disk-UTxO redesign; S0's classification gates the structural slices.

**Status:** **S0 (DIAGNOSTIC) DONE — verdict `bootstrap_transient_but_admission_live_working_set`.** The bootstrap serialization is fully reclaimable (the t3 forced collect returns it to ~idle — overturns the original snapshot hypothesis); the active-admission footprint (4.59 GiB owned) is a **LIVE working set** (it re-accumulates the very next block after a forced collect *during* admission). **The structural direction is decided: the on-disk / bounded in-memory UTxO backend (`DC-MEM-05` + `DC-MEM-07`), NOT the snapshot-streaming fix.** No BLUE change in S0.

**Prior:** MEM-OPT-OPS closed at `e0c77492`. Its S3 owned measurement is the motivating finding: active-admission owned `RssAnon` **4.59 GiB p50 / 4.76 GiB peak** vs Ade idle owned **1.95 GiB** vs Haskell owned **2.57 GiB** → `ade_heavier`; `OP-MEM-02` stays `declared`. The residual is the `seed_to_snapshot`/`chain.db` serialization during admission. **This cluster owns that surface.**

## 1. Primary Invariant
**`DC-MEM-05`** — same WAL + checkpoint ⇒ byte-identical post-state AND fingerprint, regardless of the UTxO backend. **`DC-MEM-07`** — the in-memory portion (read cache + last-k changelog) is bounded by fixed, closed, non-configurable constants; memory pressure cannot grow it unboundedly and the bound never changes an authoritative output. Every lever is a *representation/storage* change behind the **unchanged BLUE ledger interface** (`utxo_lookup`/`utxo_insert` signatures unchanged; rules see identical resolved values), each proven replay-equivalent and measured with the A2 RSS↔replay-verdict pairing.

## 2. Normative Anchors
- Registry: `DC-MEM-05` (primary — backend-independent replay), `DC-MEM-07` (bounded in-memory portion), `DC-MEM-06` (to be strengthened — store-iteration-order clause), `DC-WAL-03` (replay-equivalence), `OP-MEM-02` (the BA-08 owned target — stays `declared` until a lever clears it).
- Plan: `docs/planning/mem-opt-cluster-plan.md` §3 (three-cluster split), §5 (on-disk-UTxO determinism guards). Grounding: `docs/planning/mem-opt-grounding.md` §A.
- Prior evidence: MEM-OPT-OPS S3 — `docs/evidence/mem-opt-ops-s3-owned-{preprod-memory,compare-preprod}.*` (the owned measurement + the `ade_heavier` verdict that motivates this cluster).

## 3. Entry Conditions (prior work guarantees)
- **The owned sampler exists (MEM-OPT-OPS S3):** `ade_node::mem_measure::rss_sampler` (`RssAnon` + `Private_Dirty`), the closed `memory_measure`/`memory_summary` evidence vocab, `ci/ci_check_mem_opt_s3_owned.sh`. S0 reuses them.
- **mimalloc is the global allocator (S1):** it returns freed pages to the OS (short decay), so a forced reclaim/decay probe is *meaningful* — if owned drops after a collect, the memory was freed-but-retained.
- **The scenario is committed + reproducible (S2/S3):** same seed, recovered anchor, `initial_ledger_fp == fb7cb12a…`, replay `agreed`. S0 reproduces it exactly.
- **`redb` already drives the persistent ChainDb** (CoW B+tree, MVCC, crash-safe, proven in-tree): IF S0 classifies `live_working_set`, the on-disk backend is half-built (a new table on proven machinery).

## 4. What Changes (slices)
- **S0 — DIAGNOSTIC (GREEN/RED). Lands first.** A phase-resolved owned-RSS timeline (t1 post-import → t2 snapshot-serializing → t3 post-snapshot-after-reclaim → t4 steady-follow) + a **RED-only forced-reclaim/decay probe** at t3, classifying the ~4.6 GiB active-admission owned footprint as **`serialization_transient` | `live_working_set` | `mixed`**. NO storage change, NO BLUE. **Decides the cluster's structural direction.** *(`CE-UD-0`.)*
- **S1 — INTERFACE (BLUE interface-semantics; high-risk, proof-heavy). Lands next.** Introduce a `UtxoStore` abstraction and change the authoritative lookup `utxo_lookup` to return an **owned `Option<TxOut>`** (an on-disk backend cannot hand out a borrow into storage). The BTreeMap stays the ONLY backend — NO redb, NO on-disk state yet. A **BLUE interface change**, isolated and **proven replay-equivalent** (identical verdicts, fingerprints, failure shapes) BEFORE storage is swapped underneath it. Interface-prep, NOT a memory victory — **NO `DC-MEM-07` flip in S1**. *(`CE-UD-1`.)*
- **S2 — ON-DISK STORAGE (RED, behind the S1 interface; the owned-RSS lever). Deferred + GATED.** The copy-on-write **anchor** (redb `TxIn→TxOut`, on disk) + a **bounded in-memory k-deep changelog overlay** + a **bounded read-through cache** (non-authoritative); lookup overlay→cache→disk; mutation appends a delta (no full-map clone). `DC-MEM-05` (backend-independent replay) + `DC-MEM-07` (bounded in-memory). **S2 CANNOT START until OQ-UD-3 is answered:** block admission today computes `post_fp` by FULL-UTxO iteration per block (`runner.rs:437` → `fingerprint_utxo`), so an on-disk backend ALONE would replace heap pressure with catastrophic per-block disk iteration — S2 needs an **incremental-fingerprint** plan first. *(`CE-UD-2`.)*

## 5. Exit Criteria (CE — each CI-verifiable)
- **CE-UD-0 (S0 DIAGNOSTIC) [S0]: ✅ MET.** Committed live run (same seed/anchor, `initial_ledger_fp fb7cb12a…`, replay `agreed`, 0 diverged, 34 admits) + the t1–t5 phase timeline + the honest verdict `bootstrap_transient_but_admission_live_working_set` + the next-slice recommendation. `ci/ci_check_mem_opt_utxo_disk_s0.sh` + `ci/ci_check_mem_diag_quarantine.sh` green.
- **CE-UD-1 (S1 INTERFACE) [S1]:** `utxo_lookup` returns owned `Option<TxOut>`; the BTreeMap is the ONLY backend (no redb); ALL ledger-validity tests green; the replay corpus byte-identical; the UTxO fingerprint identical before/after for the same state; structured errors unchanged; no new clone-heavy path in block admission; the registry marks this **interface-prep, not a memory victory** (**no `DC-MEM-07` flip**). A proof-heavy BLUE-interface slice.
- **CE-UD-2 (S2 ON-DISK STORAGE) [S2]: GATED on OQ-UD-3.** `DC-MEM-05` (same replay sequence under BTreeMap AND redb → identical UTxO fingerprints, WAL/checkpoint fingerprints, replay verdicts, structured errors), `DC-MEM-07` (bounded overlay + cache, fixed closed constants), redb key order **proven** equal to the canonical `TxIn` order (a test-vector gate — else an explicit fixed-width key `txid ++ BE-u32 index`), per-block atomic commit (torn-commit rejected — negative test), cache eviction proven NOT to alter authoritative outputs, owned RSS drops to the target band.
- **CE-UD-close [/cluster-close]:** `OP-MEM-02` records the new operational standing; if a lever clears the owned target, the comparison verdict flips `ade_heavier` → `ade_below` (the BA-08 win, honestly + mechanically gated); grounding docs refreshed.

## 6. Expected Slices
- **S0** DIAGNOSTIC — CE-UD-0 — GREEN/RED. **DONE** (verdict: admission footprint is a live working set).
- **S1** INTERFACE — CE-UD-1 — **BLUE** (the owned-`utxo_lookup` change, proven replay-equivalent). **Lands next.**
- **S2** ON-DISK STORAGE — CE-UD-2 — RED behind the S1 interface (redb anchor + bounded overlay + cache). **Deferred + GATED on OQ-UD-3** (incremental fingerprint).

## 7. TCB Color Map
- **BLUE:** none in S0. **S1 is a BLUE interface-semantics change** (`utxo_lookup` → owned `Option<TxOut>`) — proven verdict/fingerprint/failure-shape-equivalent. S2's UTxO STORE is **RED behind the S1 interface** (FC/IS) — validation is a pure function of resolved UTxO values and never branches on disk-vs-memory.
- **GREEN:** the phase-marker / classification logic + the evidence schema.
- **RED:** the owned sampler, the phase taps, the **forced-reclaim probe (diagnostic only)** — quarantined in the dedicated **`ade_mem_diag`** crate (the workspace's sole unsafe-FFI surface) so `ade_node` keeps `#![deny(unsafe_code)]` with zero local allows — and (later) the storage backend.
- **Affected gates:** new `ci/ci_check_mem_opt_utxo_disk_s0.sh` (S0 timeline + honest classification + replay `agreed`) + `ci/ci_check_mem_diag_quarantine.sh` (the unsafe quarantine enforcement); the 2 phase points added to the closed POINTS vocabulary in `ci_check_mem_measure_evidence.sh`; reused `ci_check_mem_opt_s3_owned.sh`, the replay corpus (`DC-WAL-03`).

## 8. Forbidden During This Cluster
1. **No BLUE *behavioral* change** — S1 changes the `utxo_lookup` SIGNATURE (→ owned) but MUST NOT alter any verdict, fingerprint, or failure shape (proven). S2's storage swap never branches validation on disk-vs-memory. The authoritative outputs are invariant across both.
2. **The forced-reclaim probe is RED-only DIAGNOSTIC.** It MUST NOT become authoritative behavior or a hidden dependency for passing BA-08. If a future production path needs memory-release behavior, that is scoped **separately as operational/runtime policy, never BLUE semantics.** *(Standing user guardrail.)*
3. **No fingerprint from store iteration order** (`DC-MEM-06`) — canonical encoder over fixed-width big-endian keys only; the on-disk store's sorted-key iteration must equal RFC-8949 canonical order by key construction, never relied on natively.
4. **No unbounded in-memory growth** (`DC-MEM-07`) — fixed, closed, non-configurable constants for the cache + the k-deep changelog.
5. **No `OP-MEM-02` flip from gross `VmRSS`** — only the owned metric clearly below the written target.
6. **The A2 discipline** — every measured run still emits `memory_summary{replay_verdict=agreed}`; a lower-memory run that perturbs an authoritative output is invalid evidence.

## 9. Replay Obligations
`DC-MEM-05`: same WAL + checkpoint ⇒ byte-identical post-state AND fingerprint, **regardless of the UTxO backend**. S0 does NOT change the backend, so it trivially preserves this (it reproduces the S3 fingerprint exactly). Every structural slice carries the **backend-independent replay corpus** (`replay_from_anchor` / the boundary+stateful corpus run under both backends) as a hard obligation.

## 10. Open Questions
- **OQ-UD-0 (the S0 question): ANSWERED** — the active-admission footprint is a **live working set** (re-accumulates after a forced collect during admission). The bootstrap serialization is fully reclaimable (not the active cost).
- **OQ-UD-1 (structural shape): RESOLVED** — the **on-disk / bounded in-memory UTxO backend** (S2), not the snapshot-streaming fix.
- **OQ-UD-3 (per-block `post_fp` — S2 ENTRY GATE, OPEN):** block admission computes `post_fp` by **full-UTxO iteration per block** (`runner.rs:437` → `fingerprint(&next_ledger).combined` → `fingerprint_utxo` iterates the whole map). An on-disk backend ALONE would make that a per-block disk scan. **S2 cannot start until an incremental-fingerprint plan exists** (compute `post_fp` from the per-block delta, not a full scan). S1 carries this as an explicit open obligation.
- **OQ-UD-2 (mainnet scale):** confirm the on-disk design scales to mainnet ~10–15M UTxO without an owned blow-up (the point of UTxO-HD).

## 11. Cluster Close Record
*(Filled at `/cluster-close`.)*
