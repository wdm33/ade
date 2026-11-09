# Invariant Slice — Forward-Sync Static UTxO Fingerprint Authority (DC-MEM-12)

> **Status: Step 0 DONE — fix REJECTED as framed.** The per-block fingerprint is NOT
> the bottleneck (DC-MEM-11's cache works; steady-state `post_fp` ~10 µs). DC-MEM-12 as
> a per-block StaticUtxoFp does not proceed to code. Rescope pending around the real hot
> path (startup recovery materialization + wire pump) — see Step 0 results below.
> No forge launch, no topology change, no push. Not combined with KES / FindIntersect /
> pipelining / demo work.

## Cluster

LIVE-FOLLOW-THROUGHPUT — **this slice is a correction to DC-MEM-11**, not a new win.

## 4. Intent

Under `track_utxo=false`, the UTxO-component fingerprint on the **live forward-sync /
forge** path MUST be the imported/recovered **static authority** (`StaticUtxoFp`), the
same authority the admission path already uses — never a per-block recompute over the
materialized UTxO. This aligns the live-follow UTxO-component authority with the
proven-correct admission path. The ~24 s/block full Ristretto scan that prevents the
forge from closing a catch-up gap before a leader slot is the **observable symptom**;
the **invariant is the authority boundary**.

> Framing discipline: DC-MEM-12 is a **live-path authority alignment**, NOT a "pure
> optimization." Performance is the symptom; the authority boundary is the invariant.

## Background — why this is a DC-MEM-11 correction

DC-MEM-11 added a generation-keyed `UtxoFpCache` to the forward-sync reducer. It is
**hermetically true** (byte-identical to the full `fingerprint()`) but **live-path
insufficient**: on the real `--mode node` forge the per-block UTxO fingerprint still
runs at **99.8 % CPU (~24 s/block, ~0.05 b/s)**, so the forge keeps pace with the chain
at the frontier but can **never close a backlog** to reach a leader slot. The live venue
caught an incomplete invariant — this is the venue doing its job, not regression.

## The authority boundary (the invariant this slice enforces)

| Mode | UTxO fingerprint-component authority |
|---|---|
| `track_utxo=false` | imported/recovered **static** UTxO component (`StaticUtxoFp`) |
| `track_utxo=true`  | **materialized mutable** UTxO state (generation recompute; `StaticUtxoFp` **fails closed**) |

This is **not replay fraud** iff **(a)** the static component is replay-derivable and
byte-identical from the same anchor/seed/snapshot, AND **(b)** the selector cannot be
used when UTxO mutation is authoritative (`StaticUtxoFp::utxo_component` returns `Err`
under `track_utxo=true`). Both proven below. Matches the project replay law: same
anchor, inputs, WAL, checkpoints ⇒ byte-identical outputs.

## Pre-code proof (8 points — COMPLETE, read-only)

| # | Question | Answer | Evidence |
|---|---|---|---|
| 1 | `track_utxo` on the `--mode node` forge path | **`false`** | `state.rs:115` default; never set true on this path; snapshot round-trips it (`snapshot/ledger.rs:87`). **STOP-GATE #1 PASS** |
| 2 | StaticUtxoFp created where | FirstRun **admission only** (`admission/bootstrap.rs:296`) | never on WarmStart / forward-sync |
| 3 | StaticUtxoFp replay-derivable | **YES** — snapshot unconditionally persists the full UTxO (`snapshot/ledger.rs:62`), so the component re-derives byte-identically on replay. **STOP-GATE #3 PASS** |
| 4 | Forward-sync materializes the UTxO | **YES** — WarmStart decodes the full ~1.9M-entry UTxO from the snapshot (`bootstrap.rs:271`, `node_lifecycle.rs:602`) |
| 5 | Why materialized vs admission-empty | Asymmetry: admission **drops** the UTxO after computing StaticUtxoFp (`bootstrap.rs:294`); WarmStart **re-materializes** it from the snapshot for replay (`bootstrap.rs:271`) |
| 6 | Hot-path UTxO recompute site | `forward_sync/reducer.rs:262` (`utxo_fp_cache.utxo_fingerprint` → `fingerprint_utxo_v2`) |
| 7 | 2nd recompute (evidence) | **NO** — reuses `state.prior_fp` (`node_lifecycle.rs:3027`) |
| 8 | static-vs-materialized selector | admission: `match static_utxo_fp { Some => utxo_component(track_utxo), None => cache }` (`runner.rs:476`); forward-sync: **none** |

**Authority resolution:** under `track_utxo=false` the block apply is **skipped** —
`rules.rs:263-264` does `current_state.utxo_state.clone()` (unchanged), `phase1.rs:198`
confirms. So the UTxO content is **invariant per block** ⇒ the scanned component is
**constant** and **equals** both the imported UTxO fp and the admission `StaticUtxoFp`.
Therefore DC-MEM-12 produces **byte-identical** `post_fp` (replay-safe) and is not a
correction-of-value; it is an authority alignment. `StaticUtxoFp::utxo_component`
**fails closed under `track_utxo=true`** (`fingerprint.rs:266`).

## Step 0 — bottleneck proof (THE ONLY APPROVED STEP)

**One question:** *where is the per-block CPU actually going on the real `--mode node`
forward-sync path?* The 8-point proof established the authority but NOT that the UTxO
scan is the live bottleneck. The cache *should* hit (the per-block clone preserves the
`OverlayUtxo` generation) yet CPU is pinned at 99.8 % — so the bottleneck is unproven.
Threading StaticUtxoFp before proving this would risk repeating the DC-MEM-11 mistake
(fixing the wrong cost).

**Method:** a temporary, reverted diagnostic in `forward_sync/reducer.rs` (+ a temp
read-only accessor on `UtxoFpCache`) measuring, per admitted block:

1. `fingerprint_utxo_v2` / UTxO-component time
2. non-UTxO fingerprint-component time (`fingerprint_v2_with_utxo` minus the component)
3. total `post_fp` time
4. cache hit/miss
5. `OverlayUtxo` generation before/after the block
6. `track_utxo` value
7. UTxO entry count

Run on the documented `--mode node` path (fresh near-tip seed → WarmStart forge follow).
Diagnostic reverted clean afterward; tree clean; no production / topology / forge side
effects.

**Decision rule (after Step 0):**

- **UTxO scan dominates + cache misses unexpectedly** → fix the cache-miss cause OR use
  StaticUtxoFp (only if the byte-authority proof holds — it does).
- **UTxO scan dominates + cache hits** → the instrumentation is wrong or there is another
  UTxO scan site; locate it before any fix.
- **non-UTxO fingerprint dominates** → **reject StaticUtxoFp as insufficient and rescope**
  around the actual component.
- **total `post_fp` is not the bottleneck** → stop and profile the real hot path.

DC-MEM-12 proceeds to code **only if the UTxO component is proven to dominate.**

## 5. Scope (fix — NOT approved until Step 0)

- **Step 1:** build `StaticUtxoFp` on the WarmStart path from the materialized snapshot
  UTxO (the value admission computes at `bootstrap.rs:296`), thread it into
  `ForwardSyncState`.
- **Step 2:** `reducer.rs:262` selects `Some(sfp) => sfp.utxo_component(track_utxo)` under
  `track_utxo=false`; `None` / `track_utxo=true` → the generation cache / recompute. Extract
  a single shared selector helper so admission (`runner.rs:476`) and forward-sync share one
  authority path (no duplicated condition).
- **Modules:** `ade_runtime/forward_sync/reducer.rs` (GREEN selector), `ade_node/node_lifecycle.rs`
  (RED: build + thread StaticUtxoFp on WarmStart), `ade_ledger/fingerprint.rs` (shared helper).
- **Persistence impact:** none (StaticUtxoFp is recomputed/derived at recovery; not a new
  persisted field). **Network-visible impact:** none.

## 6. Execution boundary

- **BLUE:** none (the fingerprint is GREEN evidence-of-state, not the ledger transition).
- **GREEN:** `forward_sync/reducer.rs` selector; `fingerprint.rs` shared helper.
- **RED:** `node_lifecycle.rs` WarmStart recovery (builds + threads StaticUtxoFp).

## 7. Invariants preserved

- **Replay equivalence:** same anchor / inputs / WAL / checkpoints ⇒ byte-identical
  `post_fp` (the supreme rule).
- WAL `post_fp` chain (DC-WAL-*).
- `track_utxo=true` authority: the materialized UTxO remains the fingerprint authority.

## 8. Invariants strengthened / introduced

- **DC-MEM-12:** on the live forward-sync/forge path, the UTxO-component fingerprint under
  `track_utxo=false` is the imported **static** authority; the materialized UTxO is never
  scanned per block. Mechanically enforced by a CI gate forbidding `fingerprint_utxo_v2`
  on the `track_utxo=false` forward-sync path and by `StaticUtxoFp::utxo_component`'s
  fail-closed guard under `track_utxo=true`.

## 12. Mechanical Acceptance Criteria

- [ ] `track_utxo=false` forward-sync uses `StaticUtxoFp` for the UTxO fingerprint component.
- [ ] `track_utxo=true` still recomputes from the materialized UTxO and invalidates on generation change.
- [ ] `post_fp` bytes identical to the previous authoritative value for the same imported seed/anchor.
- [ ] Two-run replay from the same seed/WAL/checkpoints ⇒ byte-identical WAL `post_fp`.
- [ ] No per-block `fingerprint_utxo_v2` on the `track_utxo=false` forward-sync path.
- [ ] CI grep/static gate prevents `fingerprint_utxo_v2` on the live `track_utxo=false` path.
- [ ] Staged/live catch-up closes a known gap before the target slot (the symptom, proven).
- [ ] RSS within MEM-OPT expectations.
- [ ] Registry distinguishes DC-MEM-11 (hermetic cache / live-gap) from DC-MEM-12 (live static authority, enforced).

## 14. Hard Prohibitions / 15. Non-Goals

- No chain-selection changes. No admission-mode changes except shared-helper extraction.
- No pipelining. No topology changes. No block-production claim. No forge launch. No push.
- StaticUtxoFp MUST NOT be a blanket shortcut: under `track_utxo=true` it MUST fail closed
  and the materialized UTxO MUST remain authoritative. Ignoring real UTxO mutations while
  claiming to track them is **replay fraud**.

## Registry plan

- **DC-MEM-11:** annotate as hermetic-cache / live-path-gap (byte-correct, but the cache does
  not take effect on the live forward-sync path; superseded for the live path by DC-MEM-12).
- **DC-MEM-12:** `enforced` once Steps 1–2 land with the MAC green — the live static-authority
  alignment for `track_utxo=false`.

## Step 0 results (2026-06-18, real `--mode node` forward-sync, preview, fresh agreed=1 seed)

Per-block split (temporary diagnostic in `reducer.rs`, reverted after; tree clean):

| block | cache_hit | t_utxo | t_nonutxo | t_total |
|---|---|---|---|---|
| 1 (cache fill) | **false** | **23.8 s** | 5 µs | 23.8 s |
| 2–13 (steady)  | **true**  | **1–2 µs** | 3–22 µs | **4–24 µs** |

All blocks: `track=false`, `count=3,066,900`, `gen_before=0 gen_after=0` (generation
**stable** — under `track_utxo=false` the apply is skipped (`rules.rs:263`), the UTxO never
mutates, so the cache hits every block after the fill).

**Decision-rule outcome — "total post_fp is not the bottleneck → stop, profile the real hot path":**
- **DC-MEM-11's cache WORKS.** Steady-state per-block `post_fp` is **~10 µs** (UTxO component
  ~2 µs, cached). The full Ristretto scan runs **once** (first-block cache fill, 23.8 s), not per block.
- **DC-MEM-12 as framed (per-block StaticUtxoFp) is REJECTED** — it would save only the one-time
  first-block scan, not a per-block cost. Coding it would repeat DC-MEM-11's "fix the wrong cost" error.
- The earlier "99.8 % CPU" was a **misread**: it was the **recovery** (WarmStart materializing the
  3.07M-entry UTxO from the snapshot, ~150 s) + the first-block cache fill (~23.8 s) = ~175 s of
  one-time **startup** CPU, during which the WAL was flat — NOT the steady-state follow.

**Real hot path to profile next (NOT this slice, NOT yet approved):** the ~175 s startup — the
recovery materializes the full 3.07M UTxO (the asymmetry: admission keeps it empty + StaticUtxoFp),
and block 1 fills the cache (23.8 s). Plus the steady-state wire/follow rate (one-at-a-time pump).
A rescoped fix would target the **recovery materialization** (don't materialize on the forward-sync
path; reuse the StaticUtxoFp authority) and/or the **wire pump** — only after profiling which one
actually blocks CaughtUp before a leader slot. This slice does not proceed to code.

Diagnostic reverted; tree clean (only the §5 doc fix + this slice doc).
