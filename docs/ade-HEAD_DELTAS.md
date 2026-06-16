# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `862cd2cb` (PHASE4-N-AO close — flip `CN-CONS-03` enforced on the natural CE-AO-6 transcript, 2026-06-13 17:05)
> HEAD: `1b851c07` (MEM-OPT-UTXO-DISK — reconcile cluster to PATH A, finalize S2b statuses pre-close, 2026-06-16 20:35)
> Span: **`862cd2cb → 1b851c07`** — **63 commits** (no merges), **165 files changed, +15957 / −9209 lines**. The lead is the **MEM-OPT-UTXO-DISK cluster** (`e0c77492..1b851c07`, **the cluster this refresh closes**: the on-disk-UTxO preparation + the owned-footprint memory win). It extends the prior HEAD_DELTAS (which ended at `233644f7`) by the **MEM-OPT-OPS close commit `e0c77492`** + the entire **MEM-OPT-UTXO-DISK cluster** (`a2d22113..1b851c07`). **This cluster TOUCHED BLUE** (≈18 BLUE `src` files, +1537/−72) — the **opposite** of MEM-OPT-OPS (zero-BLUE); it is the home of **+7 canonical types** (`464 → 471`, all BLUE) and the only authoritative-output change in the span: the deliberate, versioned **v1→v2 UTxO-fingerprint cutover** (Ade-internal, never peer-facing). **`OP-MEM-02` is the headline rule and it FLIPPED `declared → enforced`** — but **SCOPED** (owned `RssAnon` 1.94 GiB < Haskell 2.57 GiB on the **LIVE `track_utxo=false` admission path only**; the 10-day BA-08 certification and `track_utxo=true` full live ledger application remain **OWED**). **+1 crate (11 → 12):** `ade_mem_diag`, the RED `unsafe`-`mi_collect` quarantine. **+10 CI gates** (190 → 200). **Registry 378 → 380** (+`DC-MEM-09`/`DC-MEM-10`, both `enforced`; `OP-MEM-02 declared → enforced`; `DC-MEM-07 declared → partial`; zero removals).

> **Baseline note (load-bearing — read before §0).** This refresh's baseline is **`862cd2cb`**, the PHASE4-N-AO
> cluster-close (it flipped `CN-CONS-03 → enforced`), and it is **valid**: `git rev-parse 862cd2cb` resolves and
> `git merge-base 862cd2cb HEAD == 862cd2cb` (it is a strict ancestor of HEAD; `862cd2cb` carries no tag). HEAD is
> **`1b851c07`** (the MEM-OPT-UTXO-DISK cluster's last commit — it reconciles the cluster to PATH A and finalizes the S2b
> statuses pre-close; the cluster-close commit lands on top carrying this regenerated doc). This is a **cluster-close
> refresh for `MEM-OPT-UTXO-DISK`** (the cluster span is `e0c77492..1b851c07`, first cluster commit `a2d22113`, parent
> `e0c77492` = the explicit MEM-OPT-OPS close commit). The prior doc was regenerated to HEAD `233644f7` against this same
> baseline; this refresh extends it by `e0c77492` (the MEM-OPT-OPS close) + the MEM-OPT-UTXO-DISK cluster. The prior
> MEM-OPT-OPS lead is preserved **verbatim** below as the "immediately-prior" section (it still narrates band 1 +
> band 2 + MEM-OPT-OPS); this refresh **leads with MEM-OPT-UTXO-DISK** (§§0–7 below). All counts/§§ in the lead read at
> HEAD `1b851c07`.
>
> **`.idd-config.json` is UNCHANGED by this refresh.** The committed `head_deltas_baseline` still reads `862cd2cb`. The
> baseline bump `862cd2cb → e0c77492` is a **deliberate, separate post-close step** (a distinct commit the user lands
> after this close) — it is **NOT** part of this cluster-close doc regen. This regen's job is only to regenerate the doc
> against the existing baseline.
>
> **Working-tree note.** At this regen the working tree shows `docs/ade-CODEMAP.md` modified (the coordinated
> grounding-doc refresh in progress — all four land together at `1b851c07` in the closure commit) + untracked scratch
> (`.mithril-scratch/`). §1 narrates the committed span `862cd2cb..1b851c07` verbatim from `git log`; §0/§6/§7 read rule
> **status** and canonical-type counts from the registry / BLUE trees at HEAD `1b851c07` (`OP-MEM-02` **enforced**,
> **380** rules, **471** canonical types).

---

# Lead — MEM-OPT-UTXO-DISK cluster (`e0c77492 → 1b851c07`)

**MEM-OPT-UTXO-DISK** is the on-disk-UTxO-preparation cluster, and it is the one that **delivered the owned-footprint
memory win** that MEM-OPT-OPS measured but could not clear. It shipped two distinct mechanisms — a **LIVE** one
(**PATH A**) that delivered the win, and a **BUILT-BUT-DORMANT** one (the on-disk redb backend) that is preparation for
the still-owed "B" (full live ledger application). Unlike MEM-OPT-OPS, **this cluster touched BLUE** (the v2 ECMH
set-commitment primitive, the `OverlayUtxo` representation, the owned `utxo_lookup` seam, the `StaticUtxoFp`/
`IncrementalUtxoFp`/`UtxoFpCache` fingerprint machinery), and it is the home of the entire **+7 canonical-type** delta
this span. Every BLUE change is proven behavior-invariant (identical verdict / fingerprint / failure-shape) **except**
the one deliberate change: the **versioned v1→v2 UTxO-fingerprint cutover**, which is **Ade-internal and never
peer-facing**. The cluster also adds the workspace's first and only `unsafe` surface — a single `mi_collect` diagnostic
probe — quarantined in the dedicated RED crate `ade_mem_diag` so `ade_node` keeps `#![deny(unsafe_code)]` with zero
local allows.

### The slice arc (`a2d22113..1b851c07`)

**S0 diagnostic → S1 owned `utxo_lookup` BLUE interface → S1.5 versioned v2 fingerprint (ECMH) → S2a overlay
representation → S2b PATH A (`StaticUtxoFp` + drop the in-memory UTxO) + the dormant redb B-infra.** No new CLI flag, no
new peer-facing surface.

- **S0 — the owned-footprint diagnostic + the `ade_mem_diag` quarantine** (`a2d22113` scope, `09b0795c` mechanism,
  `a13468d5` verdict; gate `ci_check_mem_diag_quarantine.sh` + `ci_check_mem_opt_utxo_disk_s0.sh`). The S0 transcript
  measured the active-admission owned footprint; its decisive control is a forced `mi_collect` (mimalloc returns
  freed-but-`MADV_FREE`-retained pages to the OS so a retained-freed footprint and a live working set are
  distinguishable). That ONE `unsafe` FFI call is quarantined in the **new RED crate `ade_mem_diag`** (so `ade_node`
  keeps `#![deny(unsafe_code)]`), gated behind the `ADE_MEM_PHASE_DIAGNOSTIC` env toggle (absent on every production
  run). **The probe is diagnostic-only and was NOT used in the BA-08 evidence run** (the owned-RSS measurement contains
  zero forced-collect points) — it is never a dependency for the memory win. Verdict: the admission footprint is a LIVE
  working set, not retained-freed garbage — so the lever must be *not retaining* the UTxO, not GC tuning.
- **S1 — the owned `utxo_lookup` BLUE interface** (`2c8b5a45` scope, `103361c1` build; `DC-MEM-09` **enforced**, gate
  `ci_check_utxo_lookup_owned.sh`). Changed the authoritative `utxo_lookup` from `-> Option<&TxOut>` to
  `-> Option<TxOut>` (**OWNED**) behind a minimal `UtxoStore` / `UtxoMembership` seam (so an on-disk backend can serve a
  resolved output by value without leaking storage lifetimes into the validity rules). The two production borrow sites
  (`phase.rs` apply_phase_2_failure + `tx_validity/phase1.rs` required-signers resolution) route through the owned
  interface. **INTERFACE-PREP — explicitly NOT a memory victory** (the `BTreeMap` is still fully in memory; a single
  bounded TxOut clone per lookup, not a map clone). Proven behavior-invariant.
- **S1.5 — the versioned v2 fingerprint (ECMH)** (`9e2fd58f` scope, `f680753c` primitive+oracle, `2523f3b8` incremental
  == oracle, `aea2eba3` cutover; `DC-MEM-10` **enforced**, gate `ci_check_utxo_fp_v2.sh`). S1.5a introduced
  `ade_crypto::utxo_set_commitment::UtxoSetCommitment` — a **Ristretto255 ECMH** (commutative, add/remove-exact-inverse,
  value-binding, domain-separated, version-tagged, 3 FROZEN golden vectors) — and the `fingerprint_v2` family.
  S1.5b proved the per-block `IncrementalUtxoFp` **== the full-recompute oracle** after every block, then performed the
  **production cutover** (the only authoritative-output change in the span; see the honesty note).
- **S2a — the bounded overlay** (`d63788db` scope, `c31e87c9`/`252580d5` build; `DC-MEM-07` **`declared → partial`**,
  gate `ci_check_overlay_utxo_s2a.sh`). `ade_ledger::utxo_overlay::OverlayUtxo` = an `Arc`-shared immutable anchor + a
  **bounded** in-memory overlay of diffs (`Some` = insert / `None` = delete tombstone), capped by the fixed, closed,
  non-configurable `MAX_OVERLAY_ENTRIES`; exceeding it folds the overlay into a fresh anchor (`compact()`) so the diff
  never grows unboundedly. A clone is `O(overlay)` (anchor `Arc` shared, copy-on-write); a mutation is an overlay append
  (no whole-`BTreeMap` clone). **De-risks the clone-model change before the disk swap — NOT the owned-RSS win** (the
  anchor is still fully in memory). The overlay-bound clause is now mechanically enforced; the read-cache-bound clause
  stays declared pending the on-disk anchor.
- **S2b — PATH A (the live win) + the dormant redb B-infra** (`6dc31213`/`253ee718`/`32e0da41`/`96118302`/`15302ecc`/
  `5eee92f1`/`9cd49b07`/`6748e7f8`/`c73a420f`/`e6b623d5`/`c64ccbfa`/`1b851c07`; `OP-MEM-02` **`declared → enforced`**,
  gate `ci_check_utxo_fp_cache.sh`). See the two sub-mechanisms below.

### PATH A — the live memory win (`OP-MEM-02` SCOPED-enforced)

The live admission path runs `track_utxo=false` (it follows headers/tips; it does **not** mutate the UTxO per block), so
the retained 1.9M-entry in-memory static UTxO was *accidental retention* — re-scanned per block to recompute a
**constant** fingerprint. **A.2 (`c73a420f` / `e6b623d5`) drops it:** bootstrap computes the constant UTxO-component
fingerprint **once** — `ade_ledger::fingerprint::StaticUtxoFp` (fail-closed under `track_utxo=true` or a version
mismatch) — **before** `drop(utxo)`; `post_fp` then uses the static component (`fingerprint_v2_with_utxo` ignores
`state.utxo_state`, so the empty-UTxO live ledger yields a `post_fp` byte-identical to the full-UTxO one — DC-MEM-05
replay-equivalence preserved); the durable copy is the existing on-disk snapshot. **Live re-measure** (preprod docker
peer, epoch 295, fresh 3.8 GB seed): active-admission owned `RssAnon` **1.94 GiB** (p50 == peak, n=33) vs the preprod
Haskell baseline **2.57 GiB** → verdict **`ade_below`** (~25% below; **58% below the 4.59 GiB pre-A.2 baseline**).
Correctness: 36 blocks admitted, 1 agreed + 35 lagging, **0 diverged, 0 hash mismatches, `replay_verdict agreed`**.

### The on-disk redb UTxO backend — BUILT + UNIT-PROVEN but DORMANT

`ade_runtime::chaindb::{utxo_anchor, utxo_key}` is the `TxIn → TxOut` on-disk anchor (`#![allow(dead_code)]`), the
fixed-width `txid[32] || BE-u32(index)` storage key (forcing redb's byte-sorted iteration to equal canonical `TxIn`
order, DC-MEM-06), the `AnchorPosition`/`reconcile` recovery surface, and the GREEN `pre_resolve::{collect_required_txins,
WorkingSet}` pre-resolution view (inputs ∪ collateral ∪ reference). **It is reachable from NO live path** (a CI guardrail
keeps it out of BLUE — it deliberately does **not** implement `UtxoStore`). **It is preparation for "B", NOT the
mechanism that delivered the memory win, and is NOT live-proven for durability under load/crash.**

> **CRITICAL honesty framing (verbatim-correct — an IDD review flagged this).**
>
> 1. **`OP-MEM-02` FLIPPED `declared → enforced`, but the flip is SCOPED — never read it as full BA-08 certification.**
>    The flip is the OWNED `RssAnon` 1.94 GiB < Haskell 2.57 GiB result (`ade_below`, ~25% below; ~58% below the
>    4.59 GiB pre-cluster baseline) on the **LIVE `track_utxo=false` admission path ONLY** (a 36-block evidence run:
>    replay `agreed`, 0 diverged, 0 hash mismatches). **Two obligations remain OWED:** (a) the **10-day sustained BA-08
>    certification run** — the 36-block run *positions* Ade to win the window but does **not** certify it; (b)
>    **`track_utxo=true` full live ledger application** (**LIVE-LEDGER-APPLY, "B"**) — the redb backend is the
>    preparation for it but is dormant. `enforced` here means "the SCOPED owned-footprint bound is mechanically backed on
>    the live path," NOT "BA-08 is certified." (This is the **opposite** of the prior HEAD_DELTAS, which correctly read
>    "OP-MEM-02 STAYS declared" for MEM-OPT-OPS — do not carry that claim forward.)
> 2. **The live win is PATH A** (`StaticUtxoFp` = constant UTxO-component fingerprint computed once at bootstrap,
>    fail-closed; + dropping the in-memory 1.9M-entry static UTxO after the existing snapshot is durable). The **on-disk
>    redb UTxO backend is BUILT + UNIT-PROVEN but DORMANT** (`#![allow(dead_code)]`, not live-wired) — preparation for B,
>    NOT the win mechanism, and NOT live-proven for durability.
> 3. **The only authoritative-output change is the deliberate versioned v1→v2 fingerprint cutover** (`fingerprint()`
>    delegates to `fingerprint_v2` everywhere; `fingerprint_v1` FROZEN; the chaindb META carries `FINGERPRINT_VERSION=2`,
>    fail-closed on opening an old store via `ChainDbError::FingerprintVersionMismatch`). It is **Ade-internal, never
>    peer-facing** — the `AgreementVerdict` (block-hash) path is untouched. All other BLUE changes are proven
>    behavior-invariant (identical verdict / fingerprint / failure-shape).
> 4. **The `ade_mem_diag` unsafe probe is diagnostic-only and was NOT used in the BA-08 evidence run** (zero
>    forced-collect points in the owned-RSS measurement) — it must never read as a dependency for the memory win.

### BLUE-was-touched (load-bearing — the shift from MEM-OPT-OPS)

> **MEM-OPT-UTXO-DISK TOUCHED BLUE — the OPPOSITE of MEM-OPT-OPS (which was zero-BLUE).** `git diff e0c77492..1b851c07`
> over the configured BLUE `core_paths` trees is **18 files, +1537/−72** (verified). The BLUE work is the home of the
> entire **+7 canonical-type** delta (`464 → 471`): `ade_crypto` +1 (`utxo_set_commitment::UtxoSetCommitment`) and
> `ade_ledger` +6 (`utxo_overlay::OverlayUtxo`, `pre_resolve::WorkingSet`, `fingerprint::{StaticUtxoFp, StaticUtxoFpError,
> IncrementalUtxoFp, UtxoFpCache}`). Over the **full span** `862cd2cb..1b851c07` BLUE canonical types move `462 → 471`
> (+9): **+2 in band 1** (`TxSubmissionTxId` + `RedeemerFields`, the prior pre-preprod window) and **+7 in this cluster**.
> **`ade_core` is UNTOUCHED** (`49 → 49`; the consensus authority `select_best_chain` / `validate_and_apply_header` is
> byte-identical) — the UTxO-fingerprint cutover lives in `ade_ledger` + `ade_crypto` only. **Do NOT carry forward a
> blanket "zero BLUE" claim** — that was MEM-OPT-OPS; this cluster is the consensus-adjacent BLUE band.

## 0. Headline (full span `862cd2cb → 1b851c07`; **bold = MEM-OPT-UTXO-DISK delta**)

| Count | Baseline (`862cd2cb`) | Cluster parent (`e0c77492`) | HEAD (`1b851c07`) | Δ (full span / **MEM-OPT-UTXO-DISK**) |
|---|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 173 | 190 | **200** | **+27 new / 4 modified / 0 removed** full span; **MEM-OPT-UTXO-DISK: +10 new** (`ci_check_{mem_diag_quarantine, mem_opt_utxo_disk_s0, utxo_lookup_owned, utxo_fp_v2, overlay_utxo_s2a, utxo_disk_anchor, utxo_disk_key, utxo_admission_seam, utxo_pre_resolve, utxo_fp_cache}.sh`) **+1 modified** (`ci_check_mem_measure_evidence.sh`). |
| Registry rules (`docs/ade-invariant-registry.toml`) | 372 | 378 | **380** | **+8** full span (`DC-PROTO-11` band 1; `DC-MEM-05/06/07/08` + `OP-MEM-02` band 3 / MEM-OPT-OPS; `DC-MEM-09/10` this cluster); **MEM-OPT-UTXO-DISK: +2** (`DC-MEM-09`, `DC-MEM-10`). **Zero removed** (`comm -23` of sorted `id =` lists empty across every band). |
| Registry status (enforced / scaffolding / partial / declared) | 239 / 1 / 19 / 113 | 250 / 1 / 22 / 105 | **253 / 1 / 23 / 103** | **MEM-OPT-UTXO-DISK: enforced 250 → 253 (+3), partial 22 → 23 (+1), declared 105 → 103 (−2).** The +2 new rules land **both `enforced`** (`DC-MEM-09`, `DC-MEM-10`); **`OP-MEM-02 declared → enforced`** (+1 enforced flip); **`DC-MEM-07 declared → partial`** (+1 partial). |
| **`OP-MEM-02` (owned memory below Haskell)** | `declared` | `declared` | **`enforced` (SCOPED)** | **THE headline rule — FLIPPED `declared → enforced`, SCOPED.** Owned `RssAnon` **1.94 GiB** < Haskell **2.57 GiB** (`ade_below`, ~58% below the 4.59 GiB pre-cluster baseline) on the **LIVE `track_utxo=false` admission path ONLY** (36-block run, `agreed`, 0 diverged, 0 hash mismatches). **OWED:** the 10-day sustained BA-08 cert + `track_utxo=true` full live ledger application. Not full certification. |
| `DC-MEM-09` (owned `utxo_lookup` interface) | — | — | **`enforced`** (NEW) | **NEW + `enforced`.** S1 owned `utxo_lookup` (`-> Option<TxOut>`) behind the `UtxoStore` / `UtxoMembership` seam — INTERFACE-PREP for a swappable backend, proven behavior-invariant. Gate `ci_check_utxo_lookup_owned.sh`. |
| `DC-MEM-10` (v2 ECMH fingerprint, versioned) | — | — | **`enforced`** (NEW) | **NEW + `enforced`.** S1.5 v2 UTxO set commitment (Ristretto255 ECMH) + the versioned production cutover (`fingerprint() → fingerprint_v2`; `fingerprint_v1` FROZEN; store `FINGERPRINT_VERSION=2` fail-closed). Internal replay contract only — no peer-facing change. Gate `ci_check_utxo_fp_v2.sh`. |
| `DC-MEM-07` (in-memory UTxO bounded) | `declared` | `declared` | **`partial`** | **`declared → partial`** (S2a: the overlay/changelog bound is mechanically enforced; the read-cache-bound clause stays declared pending the on-disk anchor). Gate `ci_check_overlay_utxo_s2a.sh`. |
| BLUE canonical types | 462 | 464 | **471** | **+9 full span** (`+2` band 1 — `TxSubmissionTxId`, `RedeemerFields`; **`+7` this cluster**). **MEM-OPT-UTXO-DISK: +7** — `ade_crypto +1` (`UtxoSetCommitment`), `ade_ledger +6` (`OverlayUtxo`, `WorkingSet`, `StaticUtxoFp`, `StaticUtxoFpError`, `IncrementalUtxoFp`, `UtxoFpCache`). **`ade_core` untouched (49 → 49).** |
| Crates | 11 | 11 | **12** | **+1 — NEW crate `ade_mem_diag`** (the workspace's SOLE `unsafe`-FFI surface: the quarantined RED `mi_collect` diagnostic probe; `ade_node` keeps `#![deny(unsafe_code)]`). The other manifest changes are `curve25519-dalek = "4"` on `ade_crypto` (the ECMH group) + the `ade_mem_diag` path dep on `ade_node`. |
| Tests (`#[test]` / `#[tokio::test]` attrs) | — | 2595 | **2640** | **MEM-OPT-UTXO-DISK: +45** — the hermetic unit suites (the ECMH commitment + golden-vector suite, the overlay suite, the owned-lookup / static-fp / pre-resolve / disk-anchor / disk-key suites, the `ade_mem_diag` no-op test). Approximate per the attribute-count fallback. |
| Grounding docs (CODEMAP / SEAMS / TRACEABILITY) | regenerated to `862cd2cb` | — | **all four → `1b851c07`** | **COORDINATED REFRESH — all four grounding docs land together at `1b851c07` in the closure commit.** CODEMAP = **12 crates / 471 canonical types / 2640 tests / 200 CI checks**; registry = **380 rules** (253 enforced / 23 partial / 103 declared / 1 enforced_scaffolding). This is no longer the "two windows + a cluster stale" state the prior doc flagged. |

The thread↔rule↔gate map for the MEM-OPT-UTXO-DISK cluster (the full verbatim log is §1):

| Slice | Rule(s) | Gate | What shipped |
|---|---|---|---|
| **Scope + S0** (`a2d22113`, `09b0795c`, `a13468d5`) | `CE-UD-0` (diagnostic) | `ci_check_mem_diag_quarantine.sh` (NEW); `ci_check_mem_opt_utxo_disk_s0.sh` (NEW) | Cluster scope + the owned-footprint diagnostic; the **new RED crate `ade_mem_diag`** (the quarantined `mi_collect` probe). Verdict: admission footprint is a LIVE working set. |
| **S1** (`2c8b5a45`, `103361c1`) | **`DC-MEM-09`** (NEW → **enforced**) | `ci_check_utxo_lookup_owned.sh` (NEW) | Owned `utxo_lookup` (`-> Option<TxOut>`) behind the `UtxoStore` / `UtxoMembership` seam. INTERFACE-PREP, behavior-invariant. |
| **S1.5** (`9e2fd58f`, `f680753c`, `2523f3b8`, `aea2eba3`) | **`DC-MEM-10`** (NEW → **enforced**; `strengthened_in += MEM-OPT-UTXO-DISK`) | `ci_check_utxo_fp_v2.sh` (NEW) | v2 ECMH set commitment + oracle + incremental == oracle + the versioned production cutover (`fingerprint_v1` FROZEN; store `FINGERPRINT_VERSION=2` fail-closed). |
| **S2a** (`d63788db`, `c31e87c9`, `252580d5`) | **`DC-MEM-07`** (`declared → partial`) | `ci_check_overlay_utxo_s2a.sh` (NEW) | `OverlayUtxo` = `Arc`-anchor + bounded overlay + `compact()`. Clone-model de-risk, NOT the win. |
| **S2b redb B-infra** (`6dc31213`, `253ee718`, `32e0da41`, `96118302`, `15302ecc`, `5eee92f1`) | `DC-MEM-05` / `DC-MEM-06` (DORMANT-backed) | `ci_check_utxo_disk_anchor.sh`, `ci_check_utxo_disk_key.sh`, `ci_check_utxo_admission_seam.sh`, `ci_check_utxo_pre_resolve.sh` (all NEW) | On-disk redb anchor + fixed-width key + pre-resolve working-set + admission-seam spec. **BUILT + UNIT-PROVEN but DORMANT** (preparation for "B"). |
| **S2b PATH A** (`9cd49b07`, `6748e7f8`, `c73a420f`, `e6b623d5`, `c64ccbfa`, `1b851c07`) | **`OP-MEM-02`** (`declared → enforced`, **SCOPED**) | `ci_check_utxo_fp_cache.sh` (NEW) | `StaticUtxoFp` (constant fp once at bootstrap, fail-closed) + `drop(utxo)`. Live owned `RssAnon` 1.94 GiB < Haskell 2.57 GiB; the scoped BA-08 flip. |

## 1. Commit Log (newest first, full span `862cd2cb..1b851c07`)

| Hash | Type | Summary |
|------|------|---------|
| `1b851c07` | docs | docs(mem-opt-utxo-disk): reconcile cluster to PATH A -- finalize S2b statuses pre-close |
| `c64ccbfa` | docs | docs(mem-opt): OP-MEM-02 -> enforced (scoped: track_utxo=false owned-RSS below Haskell) |
| `e6b623d5` | docs | docs(mem-opt-utxo-disk): S2b-2c.1b-A.2.2 owned-RSS re-measure -- BA-08 achieved |
| `c73a420f` | feat | feat(mem-opt-utxo-disk): S2b-2c.1b-A.2.2 -- drop the in-memory static UTxO |
| `6748e7f8` | feat | feat(mem-opt-utxo-disk): S2b-2c.1b-A.2.1 -- explicit StaticUtxoFp (fail-closed) |
| `9cd49b07` | feat | feat(mem-opt-utxo-disk): S2b-2c.1b-A.1 -- cache the constant UTxO fingerprint |
| `5eee92f1` | feat | feat(mem-opt-utxo-disk): S2b-2c.1a -- anchor position marker + reconcile primitive |
| `15302ecc` | docs | docs(mem-opt-utxo-disk): S2b-2c.0 -- admission seam spec (atomicity + recovery) |
| `96118302` | feat | feat(mem-opt-utxo-disk): S2b pre-resolve wiring -- resolved-view working-set |
| `32e0da41` | feat | feat(mem-opt-utxo-disk): S2b pre-resolve -- era-aware dependency enumeration |
| `253ee718` | feat | feat(mem-opt-utxo-disk): S2b -- on-disk redb UTxO anchor + backend equivalence |
| `6dc31213` | feat | feat(mem-opt-utxo-disk): S2b foundation -- fixed-width UTxO storage key |
| `252580d5` | feat | feat(mem-opt-utxo-disk): S2a -- overlay UTxO representation (the clone-model change) |
| `c31e87c9` | feat | feat(mem-opt-utxo-disk): S2a foundation -- the overlay UTxO data structure |
| `d63788db` | docs | docs(mem-opt-utxo-disk): scope S2 -- on-disk UTxO backend (S2a overlay / S2b redb) |
| `aea2eba3` | feat | feat(mem-opt-utxo-disk): S1.5b cutover -- production fingerprint is v2, fail-closed (DC-MEM-10 enforced) |
| `2523f3b8` | feat | feat(mem-opt-utxo-disk): S1.5b incremental maintenance proven == oracle |
| `f680753c` | feat | feat(mem-opt-utxo-disk): S1.5a -- v2 UTxO set commitment + oracle (DC-MEM-10 partial) |
| `9e2fd58f` | docs | docs(mem-opt-utxo-disk): scope S1.5 -- versioned incremental UTxO fingerprint (v2) |
| `103361c1` | feat | feat(mem-opt-utxo-disk): S1 -- owned utxo_lookup BLUE interface (DC-MEM-09) |
| `2c8b5a45` | docs | docs(mem-opt-utxo-disk): scope S1 (owned-utxo_lookup BLUE interface) + S2 gate |
| `a13468d5` | feat | feat(mem-opt-utxo-disk): S0 t5 probe + verdict -- admission footprint is a LIVE working set |
| `09b0795c` | feat | feat(mem-opt-utxo-disk): S0 diagnostic mechanism + ade_mem_diag unsafe quarantine |
| `a2d22113` | docs | docs(mem-opt-utxo-disk): scope MEM-OPT-UTXO-DISK + S0 diagnostic slice |
| `e0c77492` | *(none)* | Close MEM-OPT-OPS -- OP-MEM-02 owned-footprint posture (honest no-flip) |
| `233644f7` | fix | fix(mem-opt): MEM-OPT-OPS cluster-review fixes -- normalize all observational *_kib + correct the dup-key equivalence claim |
| `3628ed16` | feat | feat(mem-opt): MEM-OPT-OPS S3 MEASURE -- owned-footprint sampler + honest owned comparison; verdict ade_heavier |
| `54975bb0` | feat | feat(mem-opt): MEM-OPT-OPS S2 IMPORT -- streaming seed import (byte-identical), import peak halved |
| `861757f4` | feat | feat(mem-opt): MEM-OPT-OPS S1 ALLOC live -- CE-OPS-1 met, VmRSS -29.8% on preprod with mimalloc |
| `0f2dcbe6` | feat | feat(mem-opt): MEM-OPT-OPS S1 ALLOC -- mimalloc global allocator + DC-MEM-06 determinism-neutral gate |
| `2a790fad` | docs | docs(mem-opt): scaffold MEM-OPT foundation -- grounding + 3-cluster plan + OPS cluster doc + 5 declared invariants |
| `51884a78` | docs | docs(mem-measure): MEM-COMPARE-D -- committed Haskell-vs-Ade RSS comparison (BA-08); Ade loses by ~1 GB |
| `c54edb93` | feat | feat(mem-measure): MEM-MEASURE-A2 operator pass -- live preprod memory transcript (OP-MEM-01 -> partial) |
| `fbe08b58` | feat | feat(mem-measure): MEM-MEASURE-A2 build -- live memory-evidence instrumentation |
| `a84f9045` | feat | feat(mem-measure): MEM-MEASURE-A1 bounded inbound admission + memory-evidence substrate |
| `02b5c9ad` | fix | fix(admission): fail-closed guard -- bundle source_tip must equal the seed point |
| `6d052db8` | fix | fix(consensus-inputs): source the `set` stake snapshot for leader election (was `go`) |
| `e497add0` | fix | fix(admission): tolerate raw [era,block] block-fetch framing (drop stale tag-24 unwrap) |
| `38bd1943` | docs | docs(evidence): add preview ADE1 pool registration manifest |
| `78fd09d2` | chore | chore(c2): point --network preview at off-repo ~/.cardano-node-preview |
| `bd5e4c23` | feat | feat(c2): venue-parametric live path (C2-VENUE-PARAM) — Preview-first pivot |
| `71b59359` | docs | docs(c2): correct epoch ETA — faucet reaches leader-election `go` at epoch 297 |
| `ef8ac25f` | fix | fix(consensus-inputs): source leader-election `go` stake, not stake-distribution |
| `758ec953` | docs | docs(evidence): refresh preprod ADE1 manifest — faucet delegation + forging keys |
| `92b78ee1` | docs | docs(grounding): regenerate HEAD_DELTAS + TRACEABILITY to 0887b2ad |
| `0887b2ad` | feat | feat(ade_network): flip DC-PROTO-02 enforced -- live tx-submission2 full exchange closes the last surface |
| `92b855c4` | feat | feat(ade_network): tx-submission2 codec on cardano-node's real wire form (era-tagged txid + indefinite arrays) |
| `04a857a3` | docs | docs(planning): DC-PROTO-02 assessment + option-B routing (Stream 1) |
| `717febaa` | feat | feat(ci): Plutus conformance manifest -> flip CN-PLUTUS-01 enforced (Stream 1 / slice A4) |
| `dec0fd22` | feat | feat(ci): host-environment purity gate -> flip CN-PLUTUS-04 enforced (Stream 1 / slice A2) |
| `b25d1594` | test | test(ade_plutus): broaden the adversarial Plutus reject corpus (Stream 1 / slice A3) |
| `ed408410` | fix | fix(ade_plutus): per-script declared ex_units cap -- close a Plutus false-accept (Stream 1 / slice A1) |
| `55a8a7e1` | feat | feat(ci): required-signer closure gate -- lock the witness enumeration (Stream 1 / slice B) |
| `91f63195` | test | test(ade_ledger): double-spend adversarial coverage -- close the CN-LEDGER-08 gap (Stream 1 / slice C) |
| `378ca2ca` | feat | feat(ci): FSM transition-purity gate -> flip DC-PROTO-01/06 enforced; Stream 3 complete |
| `7f13d646` | feat | feat(ci): mini-protocol surface gate -> flip DC-PROTO-03/04 enforced (Stream 3) |
| `37d4c068` | feat | feat(ci): header-body binding gate -> flip CN-CONS-04 enforced (Stream 3) |
| `8e8f8eb0` | feat | feat(ci): closed-codec-message gate -> flip CN-WIRE-07 enforced (Stream 3) |
| `b01d1d38` | docs | docs(planning): Stream-3 classification + conservative routing |
| `c27ee281` | docs | docs(registry): flip DC-CORE-01 enforced (Stream 3) -- BLUE sync-only, gate-complete |
| `86252176` | docs | docs(planning): pre-preprod local-first work streams + strict order (3->1->2) |
| `5532ddf4` | docs | docs(c2-guide): rung 2 fully closed -- CN-CONS-03 enforced (PHASE4-N-AO) |
| `388d8073` | docs | docs(phase4-n-ao): cluster-close housekeeping -- regenerate grounding docs + archive + groom registry |

No merge commits in the span. **63 commits, one unclassified** — `e0c77492` ("**Close** MEM-OPT-OPS …") carries no
conventional-commits prefix (it is the explicit MEM-OPT-OPS cluster-close commit; the scope is unambiguous from its
subject, so it is surfaced as an anomaly, not guessed). The remaining **62 carry an explicit prefix**: **`feat`×33**,
**`docs`×20**, **`fix`×6**, **`test`×2**, **`chore`×1** (= 62; + 1 unclassified = 63). The **MEM-OPT-UTXO-DISK** band
(`a2d22113..1b851c07`, 23 commits) is `feat`×15 + `docs`×8. The **MEM-OPT-OPS close** is the single `e0c77492` commit.
The earlier two bands (band 1 pre-preprod + band 2 MEM-MEASURE/C2 + MEM-OPT-OPS itself, `388d8073..233644f7`) are
preserved below in the "immediately-prior" section.

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty trailer
> requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that is an Ade-local override
> of the global no-AI-attribution rule and applies to **commit messages only**. It does not affect this doc's content.

## 2. New Modules

MEM-OPT-UTXO-DISK adds **1 new crate** + **6 new source files** (3 BLUE, 3 RED). `git diff --diff-filter=A --name-status
e0c77492..1b851c07 -- 'crates/**/*.rs'` (cluster) plus `git diff --diff-filter=A '**/Cargo.toml'` confirm the crate
count moves **11 → 12** (the only new workspace member is `ade_mem_diag`).

| Module | Color | Purpose | Key sub-paths | Added in |
|--------|-------|---------|---------------|----------|
| `ade_mem_diag` (**new crate**) | **RED** | The workspace's SOLE `unsafe`-FFI surface: a quarantined `mi_collect` diagnostic probe (mimalloc returns freed-but-`MADV_FREE`-retained pages to the OS, so a retained-freed footprint is distinguishable from a live working set). It is the quarantine that lets `ade_node` keep `#![deny(unsafe_code)]` with zero local allows. Gated behind the `ADE_MEM_PHASE_DIAGNOSTIC` env toggle (absent on every production run). **Diagnostic-only — NOT used in the BA-08 evidence run; never a dependency for the memory win.** No public type (one fn), so NOT canonical-counted. No BLUE/GREEN crate may depend on it. | `src/lib.rs` (+60 — the single `unsafe { libmimalloc_sys::mi_collect(true) }` + the explicit "deliberately NOT `#![deny(unsafe_code)]`" banner) | `MEM-OPT-UTXO-DISK` S0 (`09b0795c`) |
| `ade_crypto::utxo_set_commitment` (**new file**) | **BLUE** | The v2 UTxO-fingerprint primitive: `UtxoSetCommitment`, a Ristretto255 ECMH (`curve25519-dalek 4`) binding `(canonical TxIn, canonical TxOut)` — commutative, add/remove-exact-inverse, value-binding, domain-separated (`"ade.utxo.fp.v2.entry"` / `"ade/fp/utxo/v2"`), 3 FROZEN golden vectors. **Ade's INTERNAL replay-contract primitive — NOT peer-facing, NOT Cardano consensus.** | `utxo_set_commitment.rs` (+175) | `MEM-OPT-UTXO-DISK` S1.5a (`f680753c`) |
| `ade_ledger::utxo_overlay` (**new file**) | **BLUE** | `OverlayUtxo` — an `Arc`-shared immutable anchor + a bounded in-memory overlay of diffs (`Some` = insert / `None` = tombstone), capped by the fixed `MAX_OVERLAY_ENTRIES`; `compact()` folds the overlay into a fresh anchor. The clone-model change (cheap `Arc` clone, copy-on-write). | `utxo_overlay.rs` (+351) | `MEM-OPT-UTXO-DISK` S2a (`c31e87c9`) |
| `ade_ledger::pre_resolve` (**new file**) | **GREEN-by-content** (in the BLUE crate) | The era-aware pre-resolution working-set: `collect_required_txins` + `WorkingSet` (inputs ∪ collateral ∪ reference) — the resolved-view an on-disk backend would consult. **B-infra** (consumed by the dormant redb path, not the live one). | `pre_resolve.rs` (+264) | `MEM-OPT-UTXO-DISK` S2b (`32e0da41`) |
| `ade_runtime::chaindb::utxo_anchor` (**new file**) | **RED** | The DORMANT on-disk redb `TxIn → TxOut` anchor (`#![allow(dead_code)]`): the `AnchorPosition` / `RecoveryDecision` / `reconcile` recovery surface + the backend-equivalence proof. **BUILT + UNIT-PROVEN but reachable from NO live path** (deliberately does not implement `UtxoStore`). Preparation for "B". | `utxo_anchor.rs` (+468) | `MEM-OPT-UTXO-DISK` S2b (`253ee718`) |
| `ade_runtime::chaindb::utxo_key` (**new file**) | **RED** | The DORMANT fixed-width storage key `txid[32] || BE-u32(index)` — forces redb's byte-sorted iteration to equal canonical `TxIn` order (DC-MEM-06). B-infra. | `utxo_key.rs` (+158) | `MEM-OPT-UTXO-DISK` S2b (`6dc31213`) |

The full-span set also includes **band-2 `ade_node::mem_measure`** (RED+GREEN, MEM-MEASURE-A1) and the **band-1
`ade_network` tx-submission2 capture bin + corpus** (`capture_tx_submission2_server.rs` + two test files) — both narrated
in the "immediately-prior" section, not MEM-OPT-UTXO-DISK.

> **Cross-reference (CODEMAP) — all six new modules ARE registered.** The coordinated CODEMAP refresh at `1b851c07`
> carries `ade_mem_diag` (§RED, with its quarantine banner), `ade_crypto::utxo_set_commitment` (§`ade_crypto`),
> `ade_ledger::{utxo_overlay, pre_resolve}` (§`ade_ledger`), and the dormant `ade_runtime::chaindb::{utxo_anchor,
> utxo_key}` (§RED). No new module is missing from CODEMAP — the "CODEMAP is stale" warning the prior doc carried is
> resolved by this coordinated refresh.

## 3. Modules Modified

MEM-OPT-UTXO-DISK modified the BLUE **`ade_ledger`** (the fingerprint machinery + the owned-lookup routing + the era
rules), the BLUE **`ade_crypto`** (the new ECMH primitive wired into `lib.rs`), the RED **`ade_runtime::chaindb`** (the
dormant backend + the `FINGERPRINT_VERSION` META + the streaming-import sink), and the RED/GREEN **`ade_node`**
admission orchestration (the `StaticUtxoFp` wiring + the test updates). Per-module diffstats are over the cluster
`e0c77492..1b851c07`.

| Module | Color / scope | Key changes (MEM-OPT-UTXO-DISK) |
|--------|---------------|---------------------------------|
| `ade_ledger::fingerprint` (`fingerprint.rs` **+511/−10**) | **BLUE**, the v2 cutover | **S1.5 + A.2.** Adds `fingerprint_v2` / `fingerprint_v2_with_utxo` / `fingerprint_utxo_v2` (the full-recompute oracle) / `IncrementalUtxoFp` / `UtxoFpCache` / `StaticUtxoFp`(+`StaticUtxoFpError`). The production cutover: `fingerprint()` delegates to `fingerprint_v2`; `fingerprint_v1` FROZEN (the v1 goldens + the 12 boundary-snapshot pins repointed at it). Per-block incremental maintenance PROVEN == the full recompute after every block. `StaticUtxoFp` computes the constant UTxO component once, fail-closed under `track_utxo=true` / version mismatch. |
| `ade_ledger::utxo_overlay` (`utxo_overlay.rs` **+351**, NEW) | **BLUE**, S2a | The `OverlayUtxo` data structure + `compact()` + the closed `MAX_OVERLAY_ENTRIES` bound. Proven fingerprint-identical to a direct build; `clone` shares the anchor `Arc`. |
| `ade_ledger::pre_resolve` (`pre_resolve.rs` **+264**, NEW) | **GREEN-by-content**, S2b | Era-aware `collect_required_txins` + `WorkingSet`. B-infra (dormant-path input). |
| `ade_ledger::utxo` (`utxo.rs` **+108/−24**) | **BLUE**, S1 | `utxo_lookup` → `-> Option<TxOut>` (OWNED) behind the `UtxoStore` / `UtxoMembership` seam; `UTXOState`/`BTreeMap` is the sole impl. |
| `ade_ledger` era + apply rules (`phase.rs` +2/−2, `tx_validity/phase1.rs` +2/−1, `alonzo.rs`, `babbage.rs`, `conway.rs`, `hfc.rs`, `late_era_validation.rs`, `plutus_eval.rs`, `rules.rs`, `snapshot/{mod,utxo_state}.rs`, `lib.rs`) | **BLUE**, S1 routing + v2 threading | The two production borrow sites route through the owned interface; the era rules + snapshot encode/decode + bootstrap move to v2 together (one shared entry, no per-site drift). **Behavior-invariant** except the deliberate v1→v2 cutover. |
| `ade_crypto::lib` (`lib.rs`, modified) | **BLUE**, S1.5 | Exposes the new `utxo_set_commitment` module; `curve25519-dalek = "4"` added to `ade_crypto/Cargo.toml` (the Ristretto255 group). |
| `ade_runtime::chaindb` (`utxo_anchor.rs` **+468** NEW, `utxo_key.rs` **+158** NEW, `persistent.rs`, `error.rs`, `mod.rs`, `seed_import/importer.rs`; dir **+700/−2**) | **RED**, S2b + the version META | The DORMANT redb backend (anchor + key + reconcile) + the `FINGERPRINT_VERSION=2` META in `persistent.rs` (checked fail-closed on open, `ChainDbError::FingerprintVersionMismatch` in `error.rs`) + the importer touch for the v2 fingerprint. |
| `ade_node::admission::{bootstrap,runner}` (`bootstrap.rs`, `runner.rs`; dir **+179/−4**) | **RED/GREEN**, PATH A wiring | Threads `StaticUtxoFp` through bootstrap (compute the constant fp once before `drop(utxo)`); `post_fp` uses the cached static component. Observational memory sampling unchanged in decision authority. |
| `ade_node` tests (`admission_adversarial_corpus.rs`, `admission_cross_epoch_guard.rs`, `admission_replay_equivalence.rs`) | **test**, additive | Updated for the owned-lookup + the v2 fingerprint + the `StaticUtxoFp` PATH A; all green (replay-equivalence preserved). |
| `ade_mem_diag` (`src/lib.rs` **+60** NEW, `Cargo.toml` NEW) | **RED**, S0 | The quarantine crate (see §2). |

> **BLUE WAS touched in MEM-OPT-UTXO-DISK (load-bearing — the shift from MEM-OPT-OPS).** `git diff e0c77492..1b851c07`
> over the configured BLUE `core_paths` trees is **18 files, +1537/−72** (verified) — the home of the +7 canonical types.
> This is the **opposite** of the prior cluster (MEM-OPT-OPS was zero-BLUE). The BLUE changes are proven
> behavior-invariant (identical verdict / fingerprint / failure-shape) **except** the deliberate, versioned v1→v2
> UTxO-fingerprint cutover (`DC-MEM-10`), which is **Ade-internal, never peer-facing** (the `AgreementVerdict` block-hash
> path is untouched). **`ade_core` is UNTOUCHED** (`git diff … crates/ade_core/src/` empty; `49 → 49`) — the consensus
> authority `select_best_chain` / `validate_and_apply_header` is byte-identical. Do **not** carry forward a blanket "zero
> BLUE" claim from the prior doc.

## 4. Feature Flags

**No project feature-flag deltas in any band.** Ade declares no `[features]` table in any workspace `Cargo.toml` at any
ref (`git grep '^\[features\]'` is empty at `862cd2cb`, `e0c77492`, and `1b851c07`). **MEM-OPT-UTXO-DISK introduces no
`#[cfg(feature = …)]` gate** (`git diff e0c77492..1b851c07 -- 'crates/**/*.rs' | grep -c '^+.*cfg(feature'` = **0**), **no
`compile_error!` coupling** (grep = **0**), and **no new CLI flag** (`crates/ade_node/src/cli.rs` untouched in the
cluster). The cluster's manifest changes are three `Cargo.toml` edits — none a feature flag: (1) the **new
`ade_mem_diag/Cargo.toml`** (the quarantine crate); (2) **`curve25519-dalek = "4"`** added to `ade_crypto/Cargo.toml`
(the Ristretto255 group for the v2 ECMH commitment); (3) the **`ade_mem_diag = { path = "../ade_mem_diag" }`** path
dependency on `crates/ade_node/Cargo.toml` (so the RED binary can call the quarantined probe under the env toggle).
There is no feature-flag coupling to report.

> The diagnostic probe is **not** a feature flag — it is gated by a **runtime env variable** (`ADE_MEM_PHASE_DIAGNOSTIC`,
> absent on every production run), so it cannot couple the build the way a `cfg(feature)` would. The `#![deny(unsafe_code)]`
> separation (the probe lives only in `ade_mem_diag`, which is the sole crate that omits the deny) is enforced
> mechanically by `ci_check_mem_diag_quarantine.sh`, not by a flag.

## 5. CI Checks (173 → 200 over the full span; +27 new, 4 modified, 0 removed · **MEM-OPT-UTXO-DISK: +10 new, 1 modified**)

Across the full span, **27** CI scripts were added, **4** materially modified, **0** removed (`ls ci/ci_check_*.sh | wc
-l` = **173 → 200**; `--diff-filter=D` over `ci/` empty). The **MEM-OPT-UTXO-DISK** band adds **10** new gates +
**modifies 1** (`ci_check_mem_measure_evidence.sh`). The grouping below isolates the MEM-OPT-UTXO-DISK gates; the
band-2/band-1 + MEM-OPT-OPS gates are summarized after.

### MEM-OPT-UTXO-DISK — on-disk-UTxO preparation + the owned-footprint win (new gates this cluster)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_utxo_fp_cache.sh` | **New** (`OP-MEM-02`) | `StaticUtxoFp` is computed once at bootstrap (constant UTxO-component fingerprint), `drop(utxo)` follows, and the fail-closed guard fires under `track_utxo=true` / version mismatch. The mechanical backing for the SCOPED `OP-MEM-02` flip. |
| `ci_check_utxo_fp_v2.sh` | **New** (`DC-MEM-10`) | The v2 ECMH fingerprint is NAMED / domain-separated / golden-pinned / version-tagged (NOT a naive XOR/sum); commutative + add/remove-exact-inverse + value-binding; the incremental maintenance == the full recompute; a v1/unversioned store is rejected fail-closed. |
| `ci_check_utxo_lookup_owned.sh` | **New** (`DC-MEM-09`) | The authoritative `utxo_lookup` returns OWNED (`-> Option<TxOut>`), routes through the `UtxoStore` / `UtxoMembership` seam at the two production sites, introduces no map-clone-heavy path, and `ade_ledger` carries no redb. |
| `ci_check_overlay_utxo_s2a.sh` | **New** (`DC-MEM-07`) | The `OverlayUtxo` anchor/overlay shape + the closed `MAX_OVERLAY_ENTRIES` bound + `compact()` + the fingerprint-identity (an overlay-split state fingerprints byte-identically to a direct build) + the in-memory-only seam (no redb). |
| `ci_check_utxo_disk_anchor.sh` | **New** (dormant B-infra; conceptually `DC-MEM-05`) | The dormant on-disk redb anchor's pure-storage discipline + the backend-equivalence proof — the on-disk anchor replays byte-identically to the in-memory `BTreeMap`. |
| `ci_check_utxo_disk_key.sh` | **New** (dormant B-infra; conceptually `DC-MEM-06`) | The fixed-width `txid[32] || BE-u32(index)` storage key forces redb's byte-sorted iteration to equal canonical `TxIn` order (proven by a test vector). |
| `ci_check_utxo_admission_seam.sh` | **New** (dormant B-infra) | The admission-seam spec (S2b-2c.0): atomicity + the recovery contract for the dormant on-disk anchor. |
| `ci_check_utxo_pre_resolve.sh` | **New** (B-infra) | The era-aware required-input enumeration (`collect_required_txins`) is complete (inputs ∪ collateral ∪ reference) across eras. |
| `ci_check_mem_opt_utxo_disk_s0.sh` | **New** (diagnostic `CE-UD-0`) | The S0 owned-footprint diagnostic transcript is well-formed and carries the "admission footprint is a LIVE working set" verdict. |
| `ci_check_mem_diag_quarantine.sh` | **New** (quarantine guardrail) | `ade_mem_diag` is the SOLE crate without `#![deny(unsafe_code)]`; its single `unsafe` surface is `mi_collect`; no BLUE/GREEN crate depends on it; `ade_node` retains `#![deny(unsafe_code)]`. |

### MEM-OPT-UTXO-DISK — modified gate

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_mem_measure_evidence.sh` | **Modified** | Extended this cluster so the RSS↔replay evidence record stays test-deterministic across the v2 fingerprint cutover + the PATH A re-measure (the observational `*_kib` fields remain normalized). |

### Earlier bands — pre-preprod / MEM-MEASURE+C2 / MEM-OPT-OPS (new + modified gates, summarized)

Over `862cd2cb..e0c77492` the other **17** new gates + **3** modified landed (band 1: the Stream-3 + Plutus +
tx-submission2 gates — `ci_check_{codec_message_closed, header_body_binding, mini_protocol_surface,
mini_protocol_transition_purity, plutus_budget_cap, plutus_conformance, plutus_eval_purity,
plutus_oracle_no_false_accept, required_signer_closure, tx_submission2_real_capture}.sh`; band 2/MEM-OPT-OPS:
`ci_check_{bounded_inbound_admission, mem_measure_evidence, mem_compare_evidence, alloc_determinism_neutral,
mem_opt_s1_reduction, mem_opt_s2_import_peak, mem_opt_s3_owned}.sh` + `check_ade1_leader_stake_active.sh`; the 3
full-span-modified gates are `ci_check_admission_log_vocabulary_closed.sh` + `ci_check_convergence_evidence_vocabulary_closed.sh`
+ `ci_check_mem_measure_evidence.sh`). The full per-gate catalog is in the "immediately-prior" section's §5.

> **Cross-reference (TRACEABILITY) — 5 of the 10 new gates are bound to a registry rule; 6 enforce dormant/diagnostic
> surfaces with no enforced-rule home yet.** Bound (the gate appears in the rule's `ci_script`/`code_locus`):
> `ci_check_utxo_lookup_owned.sh → DC-MEM-09`, `ci_check_utxo_fp_v2.sh → DC-MEM-10`, `ci_check_overlay_utxo_s2a.sh →
> DC-MEM-07`, `ci_check_utxo_fp_cache.sh → OP-MEM-02` (and `ci_check_mem_measure_evidence.sh` rides `DC-MEM-06`).
> **NOT yet referenced by any registry rule in any field at HEAD:** `ci_check_mem_diag_quarantine.sh`,
> `ci_check_mem_opt_utxo_disk_s0.sh`, `ci_check_utxo_disk_anchor.sh`, `ci_check_utxo_disk_key.sh`,
> `ci_check_utxo_admission_seam.sh`, `ci_check_utxo_pre_resolve.sh`. These guard (a) the `ade_mem_diag` quarantine + the
> S0 diagnostic verdict, and (b) the **dormant** redb B-infra + the pre-resolve enumeration + the admission-seam spec —
> whose home rules `DC-MEM-05` (representation-independence, `code_locus = ""`) and `DC-MEM-06` (store-order clause, names
> only the alloc/import gates) are still `declared`/`partial` and have not yet adopted the disk gates into their
> `ci_script`/`tests` arrays. **This is the expected state for dormant-prep + diagnostic gates** — not an orphan defect:
> the gates protect real surfaces, but they will only appear *rule-bound* in TRACEABILITY once "B" is live-wired and
> `DC-MEM-05`/`DC-MEM-06` adopt them (or flip). TRACEABILITY (reading the registry as primary source) should record these
> 6 as "gate present, not yet bound to an enforced rule." Since all four grounding docs are regenerated together at
> `1b851c07`, TRACEABILITY at HEAD reflects this state directly.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`); canonical-type rules
live inline in the invariant registry under family **T**. **MEM-OPT-UTXO-DISK added 7 BLUE canonical types** — the
structural `struct`/`enum` count over the BLUE `core_paths` trees moves **`464 → 471`** across the cluster. This is the
home of the entire cluster BLUE delta.

The **+7** this cluster:

- **`+UtxoSetCommitment`** — `pub struct` in `crates/ade_crypto/src/utxo_set_commitment.rs` (the Ristretto255 ECMH v2
  set-commitment primitive). `ade_crypto` `21 → 22`.
- **`+OverlayUtxo`** — `pub struct` in `crates/ade_ledger/src/utxo_overlay.rs` (the S2a `Arc`-anchor + bounded overlay).
- **`+WorkingSet`** — `pub struct` in `crates/ade_ledger/src/pre_resolve.rs` (the era-aware resolved-view, B-infra).
- **`+StaticUtxoFp`** + **`+StaticUtxoFpError`** — in `crates/ade_ledger/src/fingerprint.rs` (the constant
  UTxO-component fingerprint computed once at bootstrap + its fail-closed error).
- **`+IncrementalUtxoFp`** + **`+UtxoFpCache`** — in `crates/ade_ledger/src/fingerprint.rs` (the per-block incremental
  maintenance proven == the oracle + the fingerprint cache). `ade_ledger` `181 → 187`.

Over the **full span** `862cd2cb..1b851c07` the BLUE-tree canonical-type metric moves **`462 → 471`** (+9): the **+2 are
band 1** (`+TxSubmissionTxId` in `ade_network::codec::tx_submission`, `+RedeemerFields` in `ade_plutus::tx_eval`) and the
**+7 are this cluster**. **`ade_core` is unchanged (49 → 49)** — the consensus authority is byte-identical.

The dormant redb-anchor types (`AnchorPosition`, `RecoveryDecision`) are `pub(crate)` in the **RED** `ade_runtime` crate
and are **NOT** canonical-counted; the `ade_mem_diag` crate exports a single fn (no public type) and is **NOT** counted.

**Zero BLUE canonical types removed** in any band (append-only within the major version).

## 7. Normative / Invariant Rule Delta (372 → 380 full span; **MEM-OPT-UTXO-DISK: +2, zero removals**)

**MEM-OPT-UTXO-DISK added 2 rule IDs (`DC-MEM-09`, `DC-MEM-10`), both `enforced`; zero removed** (registry **378 → 380**
across the cluster; `comm -23` of the sorted `id =` lists is empty — exactly two additions, no removal). The status
tally over the cluster moves **enforced 250 → 253** (+3), **partial 22 → 23** (+1), **declared 105 → 103** (−2),
`enforced_scaffolding` 1 → 1. The +3 enforced = the 2 new `enforced` rules + the **`OP-MEM-02 declared → enforced`**
flip; the −2 declared = `OP-MEM-02` (left declared) + `DC-MEM-07` (left declared, now partial).

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs, the CE-73
reclassification, and `CLAUDE.md` — were **not** changed anywhere in the full span: `git diff --name-only
862cd2cb..1b851c07` over those paths is empty. The §7 delta is entirely the invariant-registry change.)*

**New MEM-OPT-UTXO-DISK rules (`+2`):**

| Rule | Family / Tier · Status @ HEAD | Statement (summary) |
|------|-------------------------------|---------------------|
| `DC-MEM-09` | DC / `derived` · **`enforced`** · `introduced_in = "MEM-OPT-UTXO-DISK"` | **Owned `utxo_lookup` interface.** The authoritative UTxO lookup returns OWNED values (`Option<TxOut>`), never a borrow into storage — the precondition for a swappable backend (`DC-MEM-05`): a resolved output is materialized BY VALUE so an on-disk backend can serve it without leaking storage lifetimes into the validity rules. Changing the lookup to owned MUST NOT alter any verdict, fingerprint, or failure shape. **ENFORCED** at S1 (`ci_check_utxo_lookup_owned.sh`); proven behavior-invariant (validity + fingerprint + DC-WAL-03 replay + admission corpus all green). INTERFACE-PREP, explicitly NOT a memory victory. |
| `DC-MEM-10` | DC / `derived` · **`enforced`** · `introduced_in = "MEM-OPT-UTXO-DISK"` · `strengthened_in = ["MEM-OPT-UTXO-DISK"]` | **Versioned v2 UTxO fingerprint (ECMH).** The v2 UTxO-fingerprint component is a NAMED commutative set commitment (Ristretto255 ECMH) binding `(TxIn, TxOut)` over the canonical encodings, domain-separated and version-tagged (`fingerprint_version` v1 vs v2 EXPLICIT, never silently mixed); it enables an O(delta)/block `post_fp`. Per-block incremental maintenance MUST equal the full recompute. **Internal replay contract only — no peer-facing / Cardano-consensus change.** NOT a naive XOR/sum. **ENFORCED** at S1.5b (the production cutover: `fingerprint() → fingerprint_v2`; `fingerprint_v1` FROZEN; store `FINGERPRINT_VERSION=2` checked fail-closed on open; `ci_check_utxo_fp_v2.sh`). |

**Status moves this cluster:**

- **`OP-MEM-02` `declared → enforced` (SCOPED)** — the headline. Owned `RssAnon` 1.94 GiB < Haskell 2.57 GiB
  (`ade_below`) on the LIVE `track_utxo=false` admission path; gate `ci_check_utxo_fp_cache.sh` + `ci_check_mem_opt_s3_owned.sh`.
  **The flip is scoped — NOT full BA-08 certification** (see the honesty note below).
- **`DC-MEM-07` `declared → partial`** — the overlay/changelog bound is now mechanically enforced (S2a,
  `ci_check_overlay_utxo_s2a.sh`); the read-cache-bound clause stays declared pending the on-disk anchor.
- **`DC-MEM-05` (storage-backend-independent replay), `DC-MEM-06` (allocator/store-order-neutral fingerprint — already
  `partial` from MEM-OPT-OPS), `DC-MEM-08` (compaction)** are unchanged in status this cluster (DC-MEM-05/08 stay
  `declared`; DC-MEM-06 stays `partial`). The dormant redb backend backs DC-MEM-05/06 mechanically (the disk gates) but
  those rules have not yet adopted the gates into their arrays (see §5 cross-reference).

**No rule was removed (expected: 0).** The MEM-OPT-UTXO-DISK registry delta is **2 new enforced rules, 2 status flips
(`OP-MEM-02 → enforced`, `DC-MEM-07 → partial`), 1 strengthening (`DC-MEM-10`), zero removals** — consistent with
append-only registry discipline.

> **`OP-MEM-02` flipped to `enforced` — but the flip is SCOPED (surfaced, NOT an anomaly, NOT over-claimed).** This is
> the OPPOSITE of the prior HEAD_DELTAS ("OP-MEM-02 STAYS declared"), and the change is correct: the SCOPED flip is the
> OWNED `RssAnon` 1.94 GiB < Haskell 2.57 GiB result (`ade_below`, ~58% below the 4.59 GiB pre-cluster baseline) on the
> **LIVE `track_utxo=false` admission path ONLY** — a 36-block evidence run (`replay_verdict agreed`, 0 diverged, 0 hash
> mismatches). The registry's own `evidence_notes` state the scope verbatim: enforced "for the CURRENT live
> `track_utxo=false` header/tip-following admission path ONLY — NOT full live UTxO application; B (LIVE-LEDGER-APPLY,
> `track_utxo=true`) remains OWED," and the "BOUNTY BA-08 full criterion is a SUSTAINED ~10-day run, CERTIFIED only by
> that sustained measurement (owed) — the current evidence POSITIONS Ade to win the 10-day window but does NOT yet
> certify it." **Never let this flip read as full BA-08 certification.**

## Honest residual (MEM-OPT-UTXO-DISK scope)

This cluster delivered the owned-footprint memory win MEM-OPT-OPS could not, and prepared (but did not live-wire) the
on-disk UTxO backend. The honest residual:

- **`OP-MEM-02` is `enforced` but SCOPED — the 10-day BA-08 cert and `track_utxo=true` are OWED.** The flip is the
  LIVE `track_utxo=false` owned `RssAnon` 1.94 GiB < Haskell 2.57 GiB result on a **36-block** run. The **sustained
  ~10-day certification run** and **full live ledger application** ("B", LIVE-LEDGER-APPLY) remain owed; the current
  evidence positions Ade to win the window but does not certify it.
- **The live win is PATH A; the on-disk redb backend is BUILT + UNIT-PROVEN but DORMANT.** PATH A = `StaticUtxoFp` (a
  constant fingerprint computed once at bootstrap, fail-closed under `track_utxo=true`/version mismatch) + dropping the
  in-memory 1.9M-entry static UTxO after the snapshot is durable. The redb backend (`utxo_anchor` / `utxo_key` /
  `pre_resolve`) is `#![allow(dead_code)]`, reachable from no live path, and **NOT live-proven for durability under
  load/crash** — it is preparation for "B".
- **The only authoritative-output change is the deliberate versioned v1→v2 fingerprint cutover — Ade-internal, never
  peer-facing.** `fingerprint() → fingerprint_v2`; `fingerprint_v1` FROZEN; store `FINGERPRINT_VERSION=2` fail-closed.
  The `AgreementVerdict` (block-hash) path is untouched. All other BLUE changes are proven behavior-invariant.
- **This cluster TOUCHED BLUE (≈18 files, +1537/−72) — the opposite of MEM-OPT-OPS.** It is the home of the +7
  canonical types. `ade_core` (the consensus authority) is untouched.
- **The `ade_mem_diag` unsafe probe is diagnostic-only — NOT used in the BA-08 evidence run.** Zero forced-collect
  points in the owned-RSS measurement; never a dependency for the win. It is the workspace's sole `unsafe` surface,
  quarantined so `ade_node` keeps `#![deny(unsafe_code)]`.
- **Four gates enforce dormant/diagnostic surfaces not yet bound to an enforced registry rule** (`utxo_disk_anchor`,
  `utxo_disk_key`, `utxo_admission_seam`, `utxo_pre_resolve`; plus the S0 `mem_opt_utxo_disk_s0` + the
  `mem_diag_quarantine` guardrail). They guard real surfaces; they bind to a rule once "B" is live-wired and
  `DC-MEM-05`/`DC-MEM-06` adopt them. The registry holds the cluster's enforced bindings authoritatively (380 rules).

## Working tree at HEAD `1b851c07` (coordinated grounding-doc refresh in progress)

At this regen the working tree shows `docs/ade-CODEMAP.md` modified (the coordinated refresh — all four grounding docs
land together at `1b851c07` in the closure commit) + untracked scratch (`.mithril-scratch/`). §1 narrates the committed
span `862cd2cb..1b851c07` verbatim; §0/§6/§7 read rule status + canonical-type counts from the registry / BLUE trees at
HEAD (`OP-MEM-02` enforced-SCOPED, 380 rules, 471 canonical types). **`.idd-config.json` is UNCHANGED by this refresh**
(`head_deltas_baseline` still reads `862cd2cb`). The baseline bump `862cd2cb → e0c77492` is a deliberate, separate
post-close step the user lands as a distinct commit — it is NOT part of this regen.

---

## Immediately-prior — MEM-OPT-OPS cluster + the MEM-MEASURE/C2 + pre-preprod bands (`862cd2cb → 233644f7`)

> The section below is the **previous** HEAD_DELTAS lead, preserved verbatim. It narrated the `862cd2cb → 233644f7` span
> against this same baseline `862cd2cb` — three sequential bands: **(1)** the pre-preprod local-first enforcement-mapping
> window (`388d8073..0887b2ad`: `DC-PROTO-02` flip + Stream 3 + Plutus, **+2 BLUE types**), **(2)** the MEM-MEASURE + C2
> band (`92b78ee1..51884a78`: the bounded-admission + RSS↔replay substrate + the Preview-first C2 pivot — it introduced
> `OP-MEM-01` + the `mem_measure` module), and **(3)** the MEM-OPT-OPS cluster (`2a790fad..233644f7`: mimalloc + streaming
> import + owned sampler). **In that prior lead `OP-MEM-02` correctly STAYED `declared`** — MEM-OPT-OPS found Ade
> *heavier* on the owned metric (`ade_heavier`, owned p50 4.59 GiB vs Haskell 2.57 GiB), which is precisely the finding
> that motivated MEM-OPT-UTXO-DISK's PATH A. **MEM-OPT-OPS touched ZERO BLUE** (band 3); the **+2 BLUE types in that span
> were entirely band 1**. Read the current lead (above) for how MEM-OPT-UTXO-DISK then flipped `OP-MEM-02 → enforced`
> (SCOPED) and **did** touch BLUE.

### Band 3 — MEM-OPT-OPS (`2a790fad..233644f7`)

> **MEM-OPT-OPS is RED/GREEN-only — ZERO BLUE change.** `git diff 2a790fad^..233644f7` over the configured BLUE
> `core_paths` trees is **EMPTY**. BLUE canonical types are **unchanged** across band 3. Every new field is a RED
> measurement (`/proc/self/status` RSS) or a GREEN evidence record — never an authoritative output.

- **`2a790fad` `docs(mem-opt)` — scaffold the MEM-OPT foundation.** Grounding + a **3-cluster plan** (OPS → UTXO-DISK →
  COMPACT) + the MEM-OPT-OPS cluster doc + **5 declared registry invariants** (`OP-MEM-02`, `DC-MEM-05`, `DC-MEM-06`,
  `DC-MEM-07`, `DC-MEM-08`).
- **`0f2dcbe6` + `861757f4` `feat(mem-opt)` — S1 ALLOC.** `#[global_allocator] static GLOBAL: mimalloc::MiMalloc` (a
  binary-only dep) + `ci_check_alloc_determinism_neutral.sh`. **`DC-MEM-06` flips `declared → partial`.** Live: glibc →
  mimalloc dropped `VmRSS` p50 **−29.8%**, agreed, 0 diverged — `CE-OPS-1` met.
- **`54975bb0` `feat(mem-opt)` — S2 IMPORT.** Streaming seed import + `ci_check_mem_opt_s2_import_peak.sh`.
  Byte-identical; import peak **−50.5%**; rejects any duplicate `TxIn` fail-closed. `DC-MEM-06 += strengthened_in`.
- **`3628ed16` `feat(mem-opt)` — S3 MEASURE.** Owned-footprint sampler (`RssAnon`) + `ci_check_mem_opt_s3_owned.sh`.
  Finding: idle/recovered owned **1.95 GiB** (below target) but active-admission owned p50 **4.59 GiB** → verdict
  **`ade_heavier`** vs Haskell 2.57 GiB. **MEM-OPT-OPS alone does NOT clear the owned posture.**
- **`233644f7` `fix(mem-opt)` — cluster-review fixes.** Generalized the `*_kib` test-determinism normalizer + corrected
  the dup-key equivalence claim. Touched `ci_check_mem_measure_evidence.sh`.

### Band 2 — MEM-MEASURE + C2-venue (`92b78ee1..51884a78`)

- **`92b78ee1` `docs(grounding)`** — regenerated HEAD_DELTAS + TRACEABILITY to `0887b2ad`.
- **C2-venue Preview pivot + leader-election stake fixes** (`ef8ac25f`, `71b59359`, `758ec953`, `bd5e4c23`, `78fd09d2`,
  `38bd1943`): source the leader-election **`go`** stake; the venue-parametric live path (`C2-VENUE-PARAM`) +
  `check_ade1_leader_stake_active.sh`.
- **Admission fail-closed + raw-framing fixes** (`e497add0`, `02b5c9ad`): tolerate raw `[era,block]` block-fetch framing
  + the fail-closed `source_tip == seed point` guard.
- **MEM-MEASURE A1/A2 + COMPARE-D** (`a84f9045`, `fbe08b58`, `c54edb93`, `51884a78`): the **new `mem_measure` module**
  (RED RSS sampler + GREEN bounded-admission gate + GREEN evidence record/validator + GREEN/RED runner); the live preprod
  memory transcript (**`OP-MEM-01 → partial`**); the committed **Haskell-vs-Ade RSS comparison** (BA-08, `MEM-COMPARE-D`):
  Ade 6.56 GB vs Haskell 5.50 GB (`ade_heavier`, +19%). Gates added: `ci_check_bounded_inbound_admission.sh`,
  `ci_check_mem_measure_evidence.sh`, `ci_check_mem_compare_evidence.sh`.

### Band 1 — pre-preprod local-first enforcement-mapping window (`388d8073..0887b2ad`)

> A **pre-preprod local-first enforcement-mapping pass** scoped by `86252176`, ordered strictly **3 → 1 → 2**. **18
> commits, 51 files.** Headline flips: **`DC-PROTO-02`** (the last N2N+N2C mini-protocol surface, on a live
> tx-submission2 full-exchange real-capture corpus) + the six **Stream 3** wire-FSM/codec/BLUE-sync flips + the two
> **Plutus** flips (`CN-PLUTUS-01`, `CN-PLUTUS-04`). **+1 new rule `DC-PROTO-11`** (enforced), **10 declared→enforced
> flips**, **+1 strengthening** (`DC-PROTO-02`), **zero removals**. **TOUCHED BLUE — +2 canonical types**
> (`+TxSubmissionTxId` `ade_network::codec::tx_submission`; `+RedeemerFields` `ade_plutus::tx_eval`). **+10 CI gates.**
> One new RED capture bin (`ade_tx_submission2_server_capture`) + a real-capture corpus; no new crate; no `[features]`,
> no `cfg(feature)`, no `compile_error!`, no new CLI flag.

Band 1's headline table (rule↔gate), preserved:

| Thread / slice | Rule(s) | Gate | What shipped |
|---|---|---|---|
| **Stream 3** (`c27ee281`) | `DC-CORE-01` (→ enforced) | `ci_check_no_async_in_blue.sh` (existing) | BLUE sync-only flip (docs/registry only). |
| **Stream 3** (`8e8f8eb0`) | `CN-WIRE-07` (→ enforced) | `ci_check_codec_message_closed.sh` (NEW) | Closed codec-message taxonomy gate. |
| **Stream 3** (`37d4c068`) | `CN-CONS-04` (→ enforced) | `ci_check_header_body_binding.sh` (NEW) | Header/body binding gate. |
| **Stream 3** (`7f13d646`) | `DC-PROTO-03` + `DC-PROTO-04` (→ enforced) | `ci_check_mini_protocol_surface.sh` (NEW) | Mini-protocol surface gate. |
| **Stream 3** (`378ca2ca`) | `DC-PROTO-01` + `DC-PROTO-06` (→ enforced) | `ci_check_mini_protocol_transition_purity.sh` (NEW) | FSM transition-purity gate; Stream 3 complete. |
| **Stream 1 / A1** (`ed408410`) | (Plutus false-accept fix) | — | **BLUE fix** in `ade_plutus::tx_eval` — per-script ex_units cap (+`RedeemerFields`). |
| **Stream 1 / A2** (`dec0fd22`) | `CN-PLUTUS-04` (→ enforced) | `ci_check_plutus_eval_purity.sh` (NEW) | Host-environment purity gate. |
| **Stream 1 / A4** (`717febaa`) | `CN-PLUTUS-01` (→ enforced) | `ci_check_plutus_conformance.sh` (NEW) | Registry-bound IOG conformance manifest gate. |
| **Stream 1 / B** (`55a8a7e1`) | `DC-LEDGER-05` (stays `partial`) | `ci_check_required_signer_closure.sh` (NEW) | Required-signer closure gate. **No flip.** |
| **Stream 1 / C** (`91f63195`) | `CN-LEDGER-08` (stays `declared`) | — | Double-spend adversarial coverage. **No flip.** |
| **TXSUB2** (`92b855c4`) | `DC-PROTO-11` (NEW, → enforced) | `ci_check_tx_submission2_real_capture.sh` (NEW) | tx-submission2 codec on the real wire form (+`TxSubmissionTxId`); new RED capture bin + corpus. |
| **TXSUB2** (`0887b2ad`) | `DC-PROTO-02` (→ enforced) | `ci_check_tx_submission2_real_capture.sh` | **Flip `DC-PROTO-02`** — live full exchange closes the last surface. |

The full §§0–7 narrative for the MEM-OPT-OPS lead (new modules / modules modified / feature flags / CI checks /
canonical-type delta / rule delta, with all cross-reference warnings) is recoverable from this doc's git history at
`233644f7`. Its headline: **MEM-OPT-OPS is zero-BLUE, `OP-MEM-02` STAYS `declared` (`ade_heavier`)**; band 1 is
**`DC-PROTO-02` enforced + 10 flips, BLUE touched +2 types**; band 2 introduced **`mem_measure` + `OP-MEM-01`**.

---

## Historical — PHASE4-N-AO live multi-candidate fork-choice SELECT + `CN-CONS-03` flip (`31efec44 → 862cd2cb`)

> The section below is a **previous** HEAD_DELTAS lead, preserved in condensed form. It narrated the
> `31efec44 → 862cd2cb` span — **the PHASE4-N-AO cluster** (slices S2–S14, with S1 at the baseline tip): the live
> multi-candidate fork-choice SELECT + adopt path. Ade DECIDES a fork-choice win among competing live branches, PROVES
> the replacement branch (fetch → bind → link → validate), COMMITS the adoption, and re-converges — on a NATURAL
> two-producer partition-and-reconverge venue. **This is the cluster that flipped `CN-CONS-03` `declared → enforced`**
> (Cardano post-partition convergence) on a committed, sha256-pinned natural CE-AO-6 transcript
> (`ContinuesSelectedBranch` + `AgreedAtSwitchTip{391}` + 25 descendants + 0 diverged; transcript OUTSIDE-repo per
> competition-secrecy). **42 commits, 50 files, +9797 / −98.** **GREEN+RED ONLY — ZERO new BLUE canonical type and ZERO
> BLUE-tree change** (`462 → 462`; `select_best_chain` + `validate_and_apply_header` reused byte-unchanged). **+6 new
> modules**, all GREEN/RED in `ade_node` (`candidate_aggregator`, `selector_state`, `fork_switch`, `lca_walk` GREEN;
> `fair_merge` RED; `post_switch_continuity` GREEN + bin); **NO new crate (11)**, **NO `Cargo.toml` change**, **NO new
> CLI flag**. **+11 CI gates, 6 modified, 0 removed** (162 → 173). **Registry 365 → 372** (+7 new rules `DC-NODE-38..41`,
> `DC-PUMP-04`, `DC-EVIDENCE-04/05`, all enforced; +5 declared→enforced flips `CN-CONS-03` + `DC-NODE-34..37`; +10
> strengthenings; **zero removals**; status 227/1/19/118 → 239/1/19/113). **NO `RO-LIVE` flip** (`RO-LIVE-01` stays
> operator-gated). The full §§0–7 narrative is recoverable from this doc's git history at `862cd2cb`. *(The N-AO close's
> grounding-doc regen + cluster-doc archive land in the FIRST two commits of band 1 — `388d8073`, `5532ddf4` — so
> the four grounding docs were pinned to `862cd2cb` at the start of band 1.)*

---

## Historical — PHASE4-N-AM keep-alive client + PHASE4-N-AN rollback-materialize eta0 (`e87e8a43 → b8860b16`)

> Preserved as a pointer. It narrated the `e87e8a43 → b8860b16` span: the **PHASE4-N-AL close commit** (`35a851b9`) +
> the **PHASE4-N-AM cluster** (`DC-PUMP-03` — the N2N keep-alive CLIENT, mini-protocol 8) + the **PHASE4-N-AN cluster**
> (`T-REC-06` — `materialize_rolled_back_state` overlays the recovered seed-epoch eta0 before the `block_validity` fold)
> + a stale-gate triage. **12 commits, 32 files, +2288 / −516.** **Touched BLUE but added ZERO new canonical type**
> (462 → 462; a single METHOD + a field). **NO new crate (11), NO new module, NO new CLI flag** (only a `tokio`
> `test-util` dev-dependency). **+2 CI gates** (159 → 161). **Registry 359 → 361** (+2; +1 strengthening; 0 removed).
> **NO `RO-LIVE` flip; `CN-CONS-03` NOT flipped** (single-best-peer rollback-FOLLOW scope — flipped the next window,
> PHASE4-N-AO). The full §§0–7 narrative is recoverable from this doc's git history at `b8860b16`.

---

## Historical — PHASE4-N-AL participant recovered-anchor rollback no-op (`b4c0983d → e87e8a43`)

> Preserved as a pointer. The **N-AK close commit** (`efa2a44e`) + a C2-guide remediation note + the **PHASE4-N-AL
> cluster** (single slice AL-S1, `DC-NODE-33` — the participant mirror of N-AK's `DC-NODE-32` recovered-anchor rollback
> no-op). **4 commits, 14 files, +1792 / −825.** **Did NOT touch BLUE** (462 → 462); **NO new crate/module/type/gate**
> (159 → 159). Registry **358 → 359** (+1; 0 strengthenings; 0 removals). **NO `RO-LIVE` flip.** The full §§0–7 narrative
> is recoverable from this doc's git history at `e87e8a43`.

---

## Historical — PHASE4-N-AK recovered-anchor live-follow start + rollback boundary (`b1bed361 → b4c0983d`)

> Preserved as a pointer. The **N-AJ close commit** (`bbdc3585`) + the **PHASE4-N-AK cluster** (AK-S1 + AK-S2). **7
> commits, 33 files, +2647 / −544.** **Touched BLUE — +2 canonical types** (`RecoveredAnchorPoint` +
> `RecoveredAnchorPointError`, new BLUE module `crates/ade_ledger/src/recovered_anchor_point.rs`; + a new RED module).
> `DC-NODE-31` (persist the bootstrap anchor POINT + resolve the live-follow FindIntersect start) + `DC-NODE-32`
> (single-producer `RollBackward(anchor)` idempotent no-op). Registry **356 → 358** (+2; `T-REC-05` strengthened; 0
> removed); CI **159 → 159**. **NO `RO-LIVE` flip.** The full §§0–7 narrative is recoverable from this doc's git history
> at `b4c0983d`.

---

## Historical — PHASE4-N-AJ Participant-path convergence evidence emission (`e99a86c7 → b1bed361`)

> Preserved as a pointer. The **PHASE4-N-AJ cluster** — Participant-path convergence evidence emission, the CE-AI-6
> bridge. **9 commits, 19 files, +1813 / −35.** **EVIDENCE-ONLY — ZERO BLUE change.** Added a deterministic GREEN
> evidence side-output (the EXISTING closed `AgreementVerdict` vocabulary to a `--convergence-evidence-path` JSONL sink,
> the new GREEN/RED module `ade_node::convergence_evidence`). CI **157 → 159** (+2). Registry **354 → 356** (+2;
> `DC-ADMIT-04` strengthened; **`CN-CONS-03` NOT flipped**; 0 removed). **NO `RO-LIVE` flip.** The full §§0–7 narrative
> is recoverable from this doc's git history at `b1bed361`.

---

## Historical — earlier windows (`8e2c3672 → e99a86c7` and before)

> Preserved as pointers. The **PHASE4-N-AI cluster** (live fork-choice rollback-follow wiring — single-best-peer FOLLOW,
> `DC-NODE-23..29`; +2 BLUE types `RollbackPoint`/`RollbackReason`; 347 → 354 rules / 148 → 157 CI; H-1 found + closed by
> AI-S6; `CN-CONS-03` NOT flipped); the **PHASE4-N-AG/N-AH** (single-producer loop-continuation + local-tip forge-base
> authority, `DC-NODE-19..22`; 343 → 347 rules / 143 → 148 CI; cert-free single-producer block production on C2-LOCAL);
> the **PHASE4-N-AF / N-AE.F / N-AD-N-AE CE-A5 window** (single-producer durable-spine extend + receive idempotency +
> recover→serve continuity + forge-on-followed-tip admissibility — the CE-A5 manifest: a real `cardano-node 11.0.1`
> relay `AddedToCurrentChain` an Ade-forged block; `DC-NODE-14..18`, `DC-CONS-24`, `DC-PROTO-10`); the **PHASE4-N-AC/AB/AA**
> (KES key evolution `DC-CRYPTO-10`, outbound mux segmentation `CN-SESS-05`, bounded serve range `DC-SERVEMEM-01`); the
> **PHASE4-N-U** (forged-block durability); and the **G-K…G-R + C1 multi-cluster catch-up**. The full §§0–7 narrative
> for each is recoverable from this doc's git history at the respective HEADs.

---

## Generation notes

### Regen `862cd2cb → 1b851c07` (MEM-OPT-UTXO-DISK cluster-close refresh — current lead)

- **Baseline valid; cluster-close refresh for MEM-OPT-UTXO-DISK.** Run against `862cd2cb` (the PHASE4-N-AO
  cluster-close), which `git rev-parse` resolves and `git merge-base 862cd2cb HEAD == 862cd2cb` confirms is a strict
  ancestor of HEAD `1b851c07` (`862cd2cb` carries no tag). The cluster span is `e0c77492..1b851c07` (first cluster commit
  `a2d22113`, parent `e0c77492` = the explicit MEM-OPT-OPS close commit, `233644f7`'s child). The prior doc was
  regenerated to `233644f7` against this same baseline; this refresh extends it by `e0c77492` + the cluster and **leads
  with MEM-OPT-UTXO-DISK** (§§0–7). The prior MEM-OPT-OPS lead is preserved verbatim as the "immediately-prior" section.
- **`.idd-config.json` is UNCHANGED by this regen.** `head_deltas_baseline` still reads `862cd2cb`. The baseline bump
  `862cd2cb → e0c77492` is a deliberate separate post-close step (a distinct commit the user lands after this close) —
  NOT part of this doc regen.
- **Counts are mechanical (git/grep/ls).** Commit log + `--shortstat` over `862cd2cb..1b851c07` (**63** commits, no
  merges / **165** files / **+15957 / −9209**); CI gate count via `git ls-tree -r --name-only <ref> ci/ | grep -c
  ci_check_.*\.sh` = **173** baseline → **190** at `e0c77492` → **200** at HEAD (cluster `e0c77492..1b851c07`
  `--diff-filter=A` = 10 new, `=M` = 1 modified, `=D` empty); registry rule count via `grep -c '^id = '` (**372 → 378 →
  380**; cluster +2; `comm -23` of sorted `id =` lists empty in every band — zero removals); registry status via
  `grep '^status = ' | sort | uniq -c` (cluster `e0c77492` 250/1/22/105 → HEAD 253/1/23/103 enforced/scaffolding/partial/
  declared); the 2 new cluster IDs via `comm -13` (`DC-MEM-09`, `DC-MEM-10`); canonical types via `git grep -hE
  "^(pub )?(struct|enum) "` over the 6 BLUE crate `src/` trees + 9 BLUE `ade_network` submodule paths (**462 → 464 →
  471**; cluster +7); tests via the `#[test]`/`#[tokio::test]` attribute grep (**2595 → 2640**, cluster +45, approximate).
- **MEM-OPT-UTXO-DISK TOUCHED BLUE — the load-bearing shift from MEM-OPT-OPS.** `git diff e0c77492..1b851c07` over the
  configured BLUE `core_paths` trees = **18 files, +1537/−72** (verified) — the home of the +7 canonical types
  (`ade_crypto +1` `UtxoSetCommitment`; `ade_ledger +6` `OverlayUtxo`/`WorkingSet`/`StaticUtxoFp`/`StaticUtxoFpError`/
  `IncrementalUtxoFp`/`UtxoFpCache`). `ade_core` UNTOUCHED (49 → 49). The full-span +2 band-1 BLUE types
  (`TxSubmissionTxId`, `RedeemerFields`) bring the full-span metric to `462 → 471`. **Do NOT carry forward a "zero BLUE"
  claim — that was MEM-OPT-OPS.**
- **`OP-MEM-02 declared → enforced`, but SCOPED (honest, not over-claimed).** The flip is owned `RssAnon` 1.94 GiB <
  Haskell 2.57 GiB (`ade_below`) on the LIVE `track_utxo=false` admission path (a 36-block run, `agreed`, 0 diverged, 0
  hash mismatches). The 10-day sustained BA-08 cert + `track_utxo=true` full live ledger application (B/LIVE-LEDGER-APPLY)
  remain OWED — the registry `evidence_notes` state the scope verbatim. Surfaced in §0/§7/residual.
- **The only authoritative-output change is the versioned v1→v2 fingerprint cutover (`DC-MEM-10`) — Ade-internal, never
  peer-facing.** `fingerprint() → fingerprint_v2`; `fingerprint_v1` FROZEN; store `FINGERPRINT_VERSION=2` fail-closed
  (`ChainDbError::FingerprintVersionMismatch`). The `AgreementVerdict` block-hash path is untouched. All other BLUE
  changes proven behavior-invariant.
- **PATH A delivered the win; the redb backend is BUILT + UNIT-PROVEN but DORMANT.** `ade_runtime::chaindb::{utxo_anchor,
  utxo_key}` + `ade_ledger::pre_resolve` are `#![allow(dead_code)]`, reachable from no live path, not live-proven for
  durability — preparation for "B", not the win mechanism.
- **+1 crate `ade_mem_diag` (the unsafe quarantine); diagnostic-only, NOT used in the BA-08 run.** Workspace members
  11 → 12 (`git diff --name-status e0c77492..1b851c07 -- Cargo.toml`). `ade_node` keeps `#![deny(unsafe_code)]` (lines
  14 + 39); `ade_mem_diag` is the sole crate that omits it (one `unsafe { mi_collect }`). No BLUE/GREEN crate depends on
  it. The owned-RSS measurement contains zero forced-collect points.
- **No feature flag, no CLI flag.** No `[features]` table at any ref; 0 `cfg(feature)` and 0 `compile_error!` added in
  the cluster; `cli.rs` untouched. The cluster's manifest changes are `ade_mem_diag/Cargo.toml` (new), `curve25519-dalek
  = "4"` on `ade_crypto`, and the `ade_mem_diag` path dep on `ade_node`.
- **Normative docs unchanged across the full span.** `git diff --name-only 862cd2cb..1b851c07` over the configured
  `normative_docs` is empty — the §7 delta is entirely the invariant-registry change.
- **§1 commit log verbatim from `git log` (newest first).** 62 of 63 carry a conventional prefix (`feat`×33 / `docs`×20
  / `fix`×6 / `test`×2 / `chore`×1); the **1 unclassified** is `e0c77492` ("Close MEM-OPT-OPS …", no prefix — the
  explicit MEM-OPT-OPS close commit; scope unambiguous, surfaced as an anomaly not guessed).
- **Coordinated grounding-doc refresh — all four land at `1b851c07`.** CODEMAP (12 crates / 471 canonical types / 2640
  tests / 200 CI checks) + SEAMS + TRACEABILITY + this doc regenerate together in the closure commit. **Cross-reference:**
  all 6 new modules ARE in the regenerated CODEMAP (§2). **5 of the 10 new gates bind a registry rule** (`utxo_lookup_owned
  → DC-MEM-09`, `utxo_fp_v2 → DC-MEM-10`, `overlay_utxo_s2a → DC-MEM-07`, `utxo_fp_cache → OP-MEM-02`); **6 enforce
  dormant/diagnostic surfaces** (`mem_diag_quarantine`, `mem_opt_utxo_disk_s0`, `utxo_disk_anchor`, `utxo_disk_key`,
  `utxo_admission_seam`, `utxo_pre_resolve`) **not yet referenced by any registry rule** — the expected state for
  dormant-prep + diagnostic gates; they bind once "B" is live-wired and `DC-MEM-05`/`DC-MEM-06` adopt them (§5).
- **Working tree at this regen.** `docs/ade-CODEMAP.md` modified (the coordinated refresh in progress) + untracked
  scratch (`.mithril-scratch/`). The closure commit carries all four refreshed grounding docs.

### Regen `862cd2cb → 233644f7` (MEM-OPT-OPS cluster-close refresh — now the "immediately-prior" lead)

- **Cluster-close refresh for MEM-OPT-OPS** against `862cd2cb`. Three bands: band 1 (pre-preprod, `388d8073..0887b2ad`),
  band 2 (MEM-MEASURE + C2, `92b78ee1..51884a78`, introduced `OP-MEM-01` + `mem_measure`), band 3 (MEM-OPT-OPS,
  `2a790fad..233644f7`). **38 commits, 110 files, +11463 / −9128.** CI **173 → 190**; registry **372 → 378**; BLUE
  canonical types **462 → 464** (both band 1). **MEM-OPT-OPS is zero-BLUE; `OP-MEM-02` STAYS `declared`** (`ade_heavier`).
  The full per-§ detail is recoverable from this doc's git history at `233644f7`.

### Regen `862cd2cb → 0887b2ad` (pre-preprod local-first enforcement-mapping window — band 1)

- **Not a cluster — a local-first enforcement-mapping pass** scoped by `86252176`, ordered strictly 3→1→2. **18 commits,
  51 files.** CI **173 → 183**; registry **372 → 373** (+1 rule `DC-PROTO-11`, 10 declared→enforced flips); BLUE
  canonical types **462 → 464**. The headline flips are `DC-PROTO-02` (the last wire surface) + the six Stream 3 flips +
  the two Plutus flips. The full §§0–7 narrative is recoverable from this doc's git history at `0887b2ad`.
