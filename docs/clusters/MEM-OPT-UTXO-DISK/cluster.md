# Cluster MEM-OPT-UTXO-DISK — storage-backed UTxO memory authority surface

**Primary invariant:** `DC-MEM-05` (the UTxO/ledger fingerprint + post-state are **independent of the storage backend** — an in-memory and an on-disk UTxO produce byte-identical replay; a storage change is NEVER a consensus/replay change) + `DC-MEM-07` (the in-memory portion is bounded by fixed, closed constants). The cluster will also **strengthen `DC-MEM-06`** (close the store-iteration-order clause that MEM-OPT-OPS S1 left `declared`) once a backing store exists.

**Scope framing (honest — not a foregone redesign):** **MEM-OPT-UTXO-DISK owns the storage-backed UTxO memory authority surface. Its first slice is diagnostic (S0) and decides whether the next mergeable work is a contained snapshot-streaming fix or the full bounded in-memory UTxO backend.** The cluster does NOT pre-commit to an on-disk-UTxO redesign; S0's classification gates the structural slices.

**Status:** **S0 (DIAGNOSTIC) scoped — structural slices deferred (S0-gated).** No BLUE change in S0.

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
- **S1+ — STRUCTURAL (DEFERRED, S0-gated; shape TBD after S0).** One of, per S0's classification:
  - `serialization_transient` → a **contained `seed_to_snapshot` serialization-streaming fix** (don't materialize the full serialized image in heap). Steady-state owned is already below Haskell's 2.57 GiB → this alone could clear BA-08; the on-disk UTxO becomes the mainnet-scalability lever, not a preprod-win prerequisite.
  - `live_working_set` → the **full bounded in-memory UTxO backend** (storage-backed `TxIn→TxOut` over `redb` + a bounded read cache + a k-deep changelog overlay; `DC-MEM-05`/`DC-MEM-07`), behind the unchanged BLUE ledger interface.
  - `mixed` → scope BOTH and sequence the cheaper (serialization-streaming) first.
  These are **not designed until S0 classifies.**

## 5. Exit Criteria (CE — each CI-verifiable)
- **CE-UD-0 (S0 DIAGNOSTIC) [S0]:** a committed run that reproduces the S3 scenario (**same seed, same recovered anchor, same `initial_ledger_fp` `fb7cb12a…`, same replay verdict `agreed`**) AND carries a **phase-resolved owned-memory artifact** (t1–t4 `RssAnon`/`Private_Dirty` + the t3 pre/post-reclaim delta) AND records an **explicit classification ∈ {`serialization_transient`, `live_working_set`, `mixed`}** AND a **next-slice recommendation derived from that classification**. The forced-reclaim probe is RED-only diagnostic. `ci/ci_check_mem_opt_utxo_disk_s0.sh` green (+ `--self-test`).
- **CE-UD-1+ (STRUCTURAL) [S1+]: TBD after S0.** Any structural slice MUST satisfy: `DC-MEM-05` (the replay corpus runs **byte-identically under both backends**), `DC-MEM-07` (the in-memory portion is fixed-bounded), the `DC-MEM-06` store-iteration-order closure (canonical encoder over fixed-width big-endian keys; fingerprint never from native iteration), per-block atomic commit (a torn commit is rejected — negative test), and the A2 discipline (`memory_summary{replay_verdict=agreed}`). Not pre-committed.
- **CE-UD-close [/cluster-close]:** `OP-MEM-02` records the new operational standing; if a lever clears the owned target, the comparison verdict flips `ade_heavier` → `ade_below` (the BA-08 win, honestly + mechanically gated); grounding docs refreshed.

## 6. Expected Slices
- **S0** DIAGNOSTIC — CE-UD-0 — GREEN/RED. **Lands first; gates the rest.**
- **S1+** STRUCTURAL — CE-UD-1+ — TBD (S0-gated). RED storage behind the unchanged BLUE ledger interface (snapshot-streaming and/or on-disk UTxO backend).

## 7. TCB Color Map
- **BLUE:** none in S0. The structural slices keep the ledger interface UNCHANGED (`utxo_lookup`/`utxo_insert` signatures); the UTxO store is **RED behind the unchanged BLUE authority** (FC/IS) — validation is a pure function of resolved UTxO values and never branches on disk-vs-memory.
- **GREEN:** the phase-marker / classification logic + the evidence schema.
- **RED:** the owned sampler, the phase taps, the **forced-reclaim probe (diagnostic only)**, and (later) the storage backend.
- **Affected gates:** new `ci/ci_check_mem_opt_utxo_disk_s0.sh` (S0 schema + classification + replay pairing); reused `ci_check_mem_opt_s3_owned.sh`, the replay corpus (`DC-WAL-03`).

## 8. Forbidden During This Cluster
1. **No BLUE semantic change** — the UTxO storage is a representation/storage change behind the unchanged ledger interface; validation never branches on disk-vs-memory.
2. **The forced-reclaim probe is RED-only DIAGNOSTIC.** It MUST NOT become authoritative behavior or a hidden dependency for passing BA-08. If a future production path needs memory-release behavior, that is scoped **separately as operational/runtime policy, never BLUE semantics.** *(Standing user guardrail.)*
3. **No fingerprint from store iteration order** (`DC-MEM-06`) — canonical encoder over fixed-width big-endian keys only; the on-disk store's sorted-key iteration must equal RFC-8949 canonical order by key construction, never relied on natively.
4. **No unbounded in-memory growth** (`DC-MEM-07`) — fixed, closed, non-configurable constants for the cache + the k-deep changelog.
5. **No `OP-MEM-02` flip from gross `VmRSS`** — only the owned metric clearly below the written target.
6. **The A2 discipline** — every measured run still emits `memory_summary{replay_verdict=agreed}`; a lower-memory run that perturbs an authoritative output is invalid evidence.

## 9. Replay Obligations
`DC-MEM-05`: same WAL + checkpoint ⇒ byte-identical post-state AND fingerprint, **regardless of the UTxO backend**. S0 does NOT change the backend, so it trivially preserves this (it reproduces the S3 fingerprint exactly). Every structural slice carries the **backend-independent replay corpus** (`replay_from_anchor` / the boundary+stateful corpus run under both backends) as a hard obligation.

## 10. Open Questions
- **OQ-UD-0 (the S0 question):** is the ~4.6 GiB active-admission owned footprint `serialization_transient`, `live_working_set`, or `mixed`? **Answered by S0.**
- **OQ-UD-1 (structural shape, S0-gated):** snapshot-serialization streaming fix vs the full bounded in-memory UTxO backend (redb `TxIn→TxOut` + bounded cache + k-deep changelog) vs both.
- **OQ-UD-2 (mainnet scale):** if the path is on-disk, confirm it scales to mainnet ~10–15M UTxO without an owned-footprint blow-up (the point of UTxO-HD).

## 11. Cluster Close Record
*(Filled at `/cluster-close`.)*
