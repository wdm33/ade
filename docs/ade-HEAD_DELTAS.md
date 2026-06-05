# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `999199f8` (repair 10 pre-existing gate-vs-code drifts (gate hygiene; 0 invariants weakened), 2026-06-05 19:28)
> HEAD: `b0365df0` (Close PHASE4-N-AA — bounded peer-driven serve range (DC-SERVEMEM-01), 2026-06-06 01:43)
> Span: **a focused grounding refresh + the PHASE4-N-AA cluster** — the post-N-U-close gate-hygiene refresh commit (`610d666a`, which set this baseline) followed by the single closed cluster **PHASE4-N-AA — bounded peer-driven serve range** (the first pre-RO-LIVE hardening item).
> 8 commits (no merges), 15 files changed, +1254 / −492 lines.

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`999199f8`**, the
> `.idd-config.json` `head_deltas_baseline` set by the *previous* (PHASE4-N-U close + gate-hygiene)
> regen — and it is **valid**: `git rev-parse 999199f8` resolves and `git merge-base 999199f8 HEAD ==
> 999199f8` (it is a strict ancestor of HEAD; `999199f8` carries no tag). HEAD is **`b0365df0`** (the
> real working HEAD, the PHASE4-N-AA close). The span has **two parts**: (1) the span-opening commit
> `610d666a` — the *focused grounding refresh* that narrated the post-N-U gate-hygiene span and **bumped
> the baseline to `999199f8`** (it refreshed SEAMS + TRACEABILITY + HEAD_DELTAS for the hygiene window;
> it is the tail of the *prior* lead, included here because it sits inside this span); and (2) the
> **PHASE4-N-AA cluster** (7 commits) — a **RED-only** hardening cluster that bounds the `--mode node`
> serve path against peer-driven resource amplification. The closer bumps `head_deltas_baseline`
> `999199f8 → b0365df0` after this regen so the next cluster measures from here.

This window is **led by a single closed cluster: PHASE4-N-AA — bounded peer-driven serve range.** It is
**pre-RO-LIVE hardening item 1** and it closes the **MEDIUM finding** the PHASE4-N-U cross-slice
security review left open: *the `--mode node` serve path could be driven by a peer into unbounded
memory + O(N²) CPU work.* Before N-AA, the serve projection (`ChainDbServedSource`, shipped by N-U S3)
fulfilled a peer's BlockFetch RequestRange via `ChainDb::iter_from_slot` — which **materializes the
full chain range into a `Vec`** and performs a **per-block full hash-index scan** — and read the tip
via the **O(N)** `chaindb.tip()`. A peer that requested a wide range could therefore amplify a single
small request into unbounded server-side storage + CPU. N-AA closes that gap across two slices, plus an
in-cluster security-review fix:

- **S1 — bounded hash-free ChainDb read primitives (`f34a2229` doc, `6b8f1779` impl; CE-1).** Two new
  **bounded, slot-ordered, hash-free** `ChainDb` trait primitives: `range_bytes_capped(from, to, max)`
  (returns at most `max` blocks, with a `truncated` flag, and performs **no** hash-index scan — the
  serve derives each hash from the bytes) and `last_block_bytes()` (the highest-slot block's bytes
  without an O(N) tip walk). A new **RED type** `CappedSlotRange { blocks: Vec<(SlotNo, Vec<u8>)>,
  truncated: bool }` carries the bounded result. Five contract tests run against **both**
  `PersistentChainDb` and `InMemoryChainDb`. The unbounded `iter_from_slot` / `tip` are **doc-fenced**
  as **TRUSTED-CALLER reads** (node startup, recovery, rollback) — their internals are **unchanged**;
  the fence only declares that the **peer-driven serve path** must use the bounded primitives instead.
- **S2 — serve projection cap + fail-closed (`1bed02e3` doc, `3d853ec0` impl; `DC-SERVEMEM-01 →
  enforced`).** `ChainDbServedSource`'s `range_bytes` / `next_after` / `tip` are switched onto the S1
  bounded primitives, with a **fixed, non-configurable cap** `const MAX_SERVE_RANGE_BLOCKS: usize =
  256` (symmetric with the receive-side `MAX_WIRE_PUMP_LOOKAHEAD = 256` / DC-LIVEMEM-01). A new **RED
  enum** `ServeRangeOutcome { Served(..) | Empty | CapExceeded | ReadError }` distinguishes outcomes
  internally; **every non-`Served` outcome maps to the wire `NoBlocks`** (oversized ranges **fail
  closed** — `CapExceeded → empty → reducer NoBlocks` — before any unbounded work). Each served block's
  hash is derived from its bytes via the **single BLUE `decode_block` authority** (no second hash
  authority, no `SLOT_BY_HASH` reference on the serve path). New gate
  `ci_check_serve_range_bounded.sh`.
- **In-cluster security-review fix (`5c9f6cf6`; MEDIUM).** The cluster's own security review found a
  **MEDIUM inverted-range panic**: a peer controls both range endpoints, and an inverted range
  (`from > to`) reached `BTreeMap::range` in the `InMemoryChainDb` primitive (and the same exposure on
  `PersistentChainDb`). Fixed **in-cluster**: a `from > to → empty` guard on **both** impls, plus a
  contract test `range_bytes_capped_inverted_range_is_empty` (runs against both) and a serve-path test
  `serve_range_inverted_range_fails_closed`.

**The headline:** the `--mode node` serve is now **bounded against peer-driven resource amplification**
— it never materializes an unbounded chain range, never per-block-scans the hash index on the serve
path, and caps each request at a fixed 256-block bound that **cannot be disabled at runtime**;
oversized / malformed (inverted) ranges **fail closed**. This **closes the PHASE4-N-U cross-slice
security MEDIUM** (peer-driven serve resource amplification) and is the **serve-side analog of
DC-LIVEMEM-01** (receive-side bounded memory). **Both gating reviews PASS** — the per-cluster security
review *found* the inverted-range MEDIUM, and the cluster *fixed it in-cluster*. The window is
**RED-only**: **0 BLUE canonical-type change** (458 unchanged), no `RO-LIVE` flip, no behavior change to
the authoritative core.

## 0. Headline

| Count | Baseline (`999199f8`) | HEAD (`b0365df0`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 135 | **136** | **+1** — **one NEW gate**, `ci_check_serve_range_bounded.sh` (S2, `DC-SERVEMEM-01`; added — `--diff-filter=A`). **No gate modified, no gate removed** in `ci/` this span (`--diff-filter=M` and `--diff-filter=D` over `ci/` are both empty). The full sweep at HEAD includes the new gate; the new gate passes (exit 0). |
| Registry rules (`docs/ade-invariant-registry.toml`) | 333 | **334** | **+1** — one NEW rule **`DC-SERVEMEM-01`** (`tier = derived`, `introduced_in = "PHASE4-N-AA"`, `status = enforced`). **Zero removed** (`comm` of the sorted id lists shows no removal). |
| Registry status (enforced / partial / declared) | 201 / 20 / 112 | **202 / 20 / 112** | **+1 enforced** — `DC-SERVEMEM-01` lands `enforced` at the S2 close (declared at scoping → enforced at close). |
| Registry strengthenings | — | **2** | `strengthened_in += "PHASE4-N-AA"` on exactly two rules: **`DC-NODE-13`** (serve-as-durable-chain projection — now bounded) and **`DC-LIVEMEM-01`** (receive-side bounded memory — N-AA adds the symmetric serve-side bound). Both are strengthenings, **not** new rules. |
| BLUE canonical types | 458 | **458** | **0** — **RED-only span.** No `ade_core` / `ade_codec` / `ade_types` / `ade_crypto` / `ade_plutus` / `ade_ledger` / `ade_network`-BLUE source change at all; every code touch is in the RED shell (`ade_runtime::chaindb` + `ade_runtime::network::served_chain_projection`). |
| Grounding docs | CODEMAP/SEAMS/TRACEABILITY pinned at N-U HEAD `4e358e92` | **SEAMS + TRACEABILITY refreshed in-span (`610d666a`); CODEMAP NOT touched** | The span-opening commit `610d666a` refreshed SEAMS + TRACEABILITY + HEAD_DELTAS (the post-N-U hygiene refresh). The N-AA **close commit `b0365df0`** touched **only** the registry + archived the cluster/slice docs — it did **not** re-touch CODEMAP / SEAMS / TRACEABILITY. **CODEMAP is now slightly stale** on the N-AA RED additions (see the cross-reference warning in §2). |

This is a **single-cluster lead** (PHASE4-N-AA) preceded by the prior-lead's hygiene-refresh tail. The
slice↔rule↔gate map for the cluster:

| Slice | Rule | Gate | What shipped |
|---|---|---|---|
| **S1** (`6b8f1779`) | CE-1 (cluster CE; enforced under `DC-SERVEMEM-01`) | (contract tests; covered by S2's gate) | Bounded hash-free `ChainDb` read primitives `range_bytes_capped` + `last_block_bytes`; new RED type `CappedSlotRange`; `iter_from_slot`/`tip` doc-fenced as trusted-caller-only (internals unchanged). |
| **S2** (`3d853ec0`) | **`DC-SERVEMEM-01`** (NEW, enforced) | **`ci_check_serve_range_bounded.sh`** (NEW) | Serve cap `MAX_SERVE_RANGE_BLOCKS = 256` + fail-closed; new RED enum `ServeRangeOutcome`; hash derived via the BLUE `decode_block` authority. |
| **(security-review MEDIUM)** (`5c9f6cf6`) | `DC-SERVEMEM-01` (reinforced) | (covered by the same gate; + tests) | `from > to → empty` inverted-range guard on both `ChainDb` impls + parity/serve regression tests. |

The per-commit shape:

| Commit | Kind | What it did | Code / CI / registry effect |
|---|---|---|---|
| `610d666a` | docs (prior-lead tail) | Focused grounding refresh for the post-N-U-close gate-hygiene span; refresh SEAMS + TRACEABILITY + HEAD_DELTAS; bump baseline `4e358e92 → 999199f8` | **0 code / 0 CI / 0 registry** (only the 3 docs + `.idd-config.json`) |
| `a6e184e8` | docs (cluster doc) | PHASE4-N-AA cluster doc; **declare `DC-SERVEMEM-01`** | **0 code / 0 CI**; registry: `DC-SERVEMEM-01` added `declared` |
| `f34a2229` | docs (slice doc) | S1 slice doc (bounded ChainDb read primitives) | **0 code / 0 CI / 0 registry** |
| `6b8f1779` | feat(chaindb) | S1 impl — bounded hash-free read primitives (`range_bytes_capped` + `last_block_bytes`); RED type `CappedSlotRange`; doc-fence `iter_from_slot`/`tip` | **RED code** (chaindb contract + both impls + types); 0 CI; 0 registry |
| `1bed02e3` | docs (slice doc) | S2 slice doc (serve projection cap + fail-closed) | **0 code / 0 CI / 0 registry** |
| `3d853ec0` | feat(serve) | S2 impl — serve cap `MAX_SERVE_RANGE_BLOCKS = 256` + fail-closed; RED enum `ServeRangeOutcome`; hash via `decode_block` | **RED code** (`served_chain_projection.rs`); **+1 CI** (`ci_check_serve_range_bounded.sh`); registry: `DC-SERVEMEM-01 → enforced` |
| `5c9f6cf6` | fix(chaindb) | Security-review MEDIUM — inverted-range (`from > to`) guard on both `ChainDb` impls + parity/serve tests | **RED code** (chaindb both impls + contract test + serve test); 0 CI; 0 registry |
| `b0365df0` | chore (close) | Close PHASE4-N-AA — archive cluster/slice docs; `strengthened_in += "PHASE4-N-AA"` on `DC-NODE-13` + `DC-LIVEMEM-01` | **0 code / 0 CI**; registry: 2 strengthenings (no new rule); 3 doc renames to `docs/clusters/completed/PHASE4-N-AA/` |

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `b0365df0` | chore (close) | Close PHASE4-N-AA — bounded peer-driven serve range (DC-SERVEMEM-01) |
| `5c9f6cf6` | fix | guard range_bytes_capped against inverted range (PHASE4-N-AA security-review MEDIUM) |
| `3d853ec0` | feat | bounded peer-driven serve range + fail-closed (PHASE4-N-AA S2, DC-SERVEMEM-01) |
| `1bed02e3` | docs | slice doc PHASE4-N-AA S2 serve projection cap + fail-closed |
| `6b8f1779` | feat | bounded hash-free serve read primitives (PHASE4-N-AA S1, CE-1) |
| `f34a2229` | docs | slice doc PHASE4-N-AA S1 bounded ChainDb read primitives |
| `a6e184e8` | docs | cluster doc PHASE4-N-AA bounded peer-driven serve range + declare DC-SERVEMEM-01 |
| `610d666a` | docs | focused grounding refresh for post-N-U-close gate-hygiene (green-means-green 135/0) |

No merge commits in the span. **8 commits, zero unclassified** — five carry an explicit
conventional-commits prefix (`feat(serve):`, `feat(chaindb):`, `fix(chaindb):`, three `docs:`); the
close commit `b0365df0` is a `/cluster-close`-style record (its diff scope is exclusively `docs/` +
`docs/ade-invariant-registry.toml`, so it classifies `chore`/`docs`). The shape is **refresh →
declare → S1 → S2 → security-fix → close**: the prior-lead hygiene refresh (`610d666a`), the cluster
doc declaring `DC-SERVEMEM-01` (`a6e184e8`), the two slices (S1 `6b8f1779`, S2 `3d853ec0`), the
in-cluster security-review MEDIUM fix (`5c9f6cf6`), and the close (`b0365df0`). The cluster work landed
2026-06-06 (00:35 → 01:43); the refresh tail landed 2026-06-05 19:49.

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty
> trailer requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that
> is an Ade-local override of the global no-AI-attribution rule and applies to **commit messages
> only**. It does not affect this doc's content.

## 2. New Modules

**None.** `git diff --diff-filter=A --name-only 999199f8..b0365df0` shows **no new `.rs` source file**,
no new crate, no new `Cargo.toml`, no new workspace. The only added files this span are **one CI gate**
(`ci/ci_check_serve_range_bounded.sh`, §5) and **three cluster/slice docs**
(`docs/clusters/completed/PHASE4-N-AA/{cluster,S1-…,S2-…}.md`). The span is **modification only** in
code — all changes land in **existing** RED modules (`ade_runtime::chaindb::{contract,in_memory,mod,
persistent,types}` and `ade_runtime::network::served_chain_projection`).

> **Cross-reference (CODEMAP) — STALE on the N-AA RED additions; refresh on next `/codemap`.** This
> span adds **no new module**, but it adds **new RED surface to existing modules** that CODEMAP does not
> yet list: the new RED type `CappedSlotRange` and the new `ChainDb` primitives `range_bytes_capped` /
> `last_block_bytes` (in `ade_runtime::chaindb`), and the new RED enum `ServeRangeOutcome` + the cap
> `MAX_SERVE_RANGE_BLOCKS` (in `ade_runtime::network::served_chain_projection`). The
> `served_chain_projection` module itself is already in CODEMAP (added by N-U S3), but **CODEMAP was not
> regenerated this span** (the close commit `b0365df0` left it pinned at the N-U HEAD `4e358e92`), so
> `grep` for `CappedSlotRange` / `ServeRangeOutcome` / `range_bytes_capped` / `last_block_bytes` in
> CODEMAP returns **0**. CODEMAP is **slightly stale** on these additions — a `/codemap` refresh on the
> next cluster close (or now) would fold them into the `ade_runtime` §RED rows.

## 3. Modules Modified

Two RED modules changed this span (all six touched `.rs` files are RED-shell; **zero BLUE**). Grouped
by sub-system:

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_runtime::chaindb` (`crates/ade_runtime/src/chaindb/{contract,in_memory,mod,persistent,types}.rs`) | +239 / −3 across 5 files (RED shell) | **S1 (`6b8f1779`) + security fix (`5c9f6cf6`).** Adds two **bounded, hash-free, slot-ordered** `ChainDb` trait primitives — `range_bytes_capped(from, to, max) -> CappedSlotRange` (at most `max` blocks, `truncated` flag, **no** hash-index scan) and `last_block_bytes()` (highest-slot block's bytes, no O(N) tip walk) — declared on the trait (`mod.rs`), implemented on `PersistentChainDb` (`persistent.rs`) and `InMemoryChainDb` (`in_memory.rs`), with **6 contract tests** in `contract.rs` running against **both** impls. New RED type **`CappedSlotRange { blocks: Vec<(SlotNo, Vec<u8>)>, truncated: bool }`** (`types.rs`). The unbounded `iter_from_slot` / `tip` are **doc-fenced** (`mod.rs`/`contract.rs`) as **TRUSTED-CALLER** reads (node startup / recovery / rollback) — internals **unchanged**; the fence only declares that the peer-driven serve path MUST use the bounded primitives. **Security fix:** a `from > to → empty` inverted-range guard on **both** impls (`from > to` on `InMemoryChainDb`'s `BTreeMap::range`; `from.0 > to.0` on `PersistentChainDb`) + the contract test `range_bytes_capped_inverted_range_is_empty`. **No new BLUE type, no signature change to any BLUE surface.** |
| `ade_runtime::network::served_chain_projection` (`crates/ade_runtime/src/network/served_chain_projection.rs`) | +197 / −76 (RED shell) | **S2 (`3d853ec0`) + security fix (`5c9f6cf6`).** Switches `ChainDbServedSource`'s `range_bytes` / `next_after` / `tip` off the unbounded `iter_from_slot` / `chaindb.tip()` and onto the S1 bounded primitives, behind a **fixed, non-configurable** cap `const MAX_SERVE_RANGE_BLOCKS: usize = 256` (symmetric with the receive-side `MAX_WIRE_PUMP_LOOKAHEAD = 256`). New RED enum **`ServeRangeOutcome { Served(..) | Empty | CapExceeded | ReadError }`** — every **non-`Served`** outcome maps to wire `NoBlocks` (oversized ranges **fail closed** before unbounded work). Each served block's hash is derived from its bytes via the **single BLUE `decode_block` authority** (no `SLOT_BY_HASH` on the serve path, no second hash authority). **Security fix:** the serve-path regression test `serve_range_inverted_range_fails_closed` (peer controls both endpoints; inverted range → `Empty`, no panic). |

> **No BLUE-authority change (load-bearing).** This span touches **no BLUE source file at all** — every
> code change is in the **RED shell** (`ade_runtime::chaindb` + `ade_runtime::network`). The BLUE
> canonical-type count is **458 → 458**. The serve projection reads raw block bytes and derives hashes
> via the **existing** single BLUE `decode_block` authority; it introduces no parallel hash authority
> and no new BLUE type. The new types (`CappedSlotRange`, `ServeRangeOutcome`) and the cap constant are
> all **RED**.

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace `Cargo.toml`,
and **no `Cargo.toml` changed in this window** (`git diff --name-only 999199f8..b0365df0 --
'**/Cargo.toml' 'Cargo.toml'` is empty). No `#[cfg(feature = …)]` gate was introduced. The cap
`MAX_SERVE_RANGE_BLOCKS = 256` is a **compile-time `const`** — deliberately **not** a feature flag, CLI
flag, env var, or config knob (it **cannot be disabled at runtime**, per `DC-SERVEMEM-01`); the gate
`ci_check_serve_range_bounded.sh` enforces that it stays a fixed non-configurable literal.

## 5. CI Checks (135 → 136; +1 new, 0 modified, 0 removed)

One new gate this span; no gate modified, no gate removed. `git diff --diff-filter=A 999199f8..b0365df0
-- ci/` lists exactly the one gate below; `--diff-filter=M` and `--diff-filter=D` over `ci/` are both
**empty**. The new gate passes at HEAD (exit 0).

### PHASE4-N-AA serve-bound gate (`3d853ec0`)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_serve_range_bounded.sh` | **New** | PHASE4-N-AA S2 (`3d853ec0`); `DC-SERVEMEM-01` | The `--mode node` serve projection must **bound per-request work**: it reads via the S1 hash-free bounded primitives (`range_bytes_capped` / `last_block_bytes`), **never** the unbounded `iter_from_slot` (full-range `Vec` + per-block hash-index scan) or the O(N) `chaindb.tip()` on the serve path; it caps each request at a **fixed, non-configurable** `MAX_SERVE_RANGE_BLOCKS` literal (no CLI/env/config escape, no unbounded mode); and it derives each block's hash from the bytes via the **single BLUE `decode_block` authority** (no `SLOT_BY_HASH` reference on the serve path). Oversized ranges **fail closed** (`CapExceeded → empty → reducer NoBlocks`). The gate is **non-vacuous**: pre-S2 the serve path had 2 `iter_from_slot` + 1 `chaindb.tip()` serve calls (Guards 2/3 would fire); at S2 it has 0. |

> **Cross-reference (TRACEABILITY) — current for the new rule's gate binding, but refreshed by the
> span-opening commit, NOT the close.** TRACEABILITY was refreshed in `610d666a` (the span-opening
> hygiene refresh), **before** `DC-SERVEMEM-01` was enforced (S2 is `3d853ec0`, later in the span) — so
> a TRACEABILITY refresh on the next cluster close should add the `DC-SERVEMEM-01 ↔
> ci_check_serve_range_bounded.sh` row. The registry itself records the binding at HEAD
> (`DC-SERVEMEM-01.ci_script = "ci/ci_check_serve_range_bounded.sh"`), so the rule↔gate link is
> authoritative in the registry; the TRACEABILITY doc is the lagging view. **No rule↔gate binding was
> removed.** The new gate enforces a named, enforced invariant (`DC-SERVEMEM-01`), so it is **not** an
> orphan gate.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. **No canonical type was
added or removed in this window** — this is a **RED-only** span (BLUE count unchanged, **458 → 458**,
per the CODEMAP header). The new types `CappedSlotRange` and `ServeRangeOutcome` and the cap
`MAX_SERVE_RANGE_BLOCKS` are **RED-shell** declarations, not BLUE canonical types. No `Cargo.toml`
changed.

## 7. Normative / Invariant Rule Delta (333 → 334; +1 enforced rule, 2 strengthenings, zero removals)

**One rule ID was added; zero removed** (333 → 334; `comm` of the sorted id lists shows the single
addition `DC-SERVEMEM-01` and no removal). The status tally moves **201 → 202 enforced** (20 partial /
112 declared unchanged) — the new rule lands `enforced` at the S2 close.

**New rule (`+1`, enforced):**

| Rule | Family / Tier | Statement (summary) |
|------|---------------|---------------------|
| `DC-SERVEMEM-01` | DC / `derived` (`enforced`; `introduced_in = "PHASE4-N-AA"`) | **Peer-driven serve range work is bounded.** The `--mode node` serve path must not materialize an unbounded chain range, perform per-block full-index scans, or read more than `MAX_SERVE_RANGE_BLOCKS` blocks for a single peer request. Oversized ranges **fail closed** before unbounded storage/CPU work. The cap is a **defensive implementation bound**, not a Cardano semantic parameter, and **cannot be disabled at runtime**. `ci_script = ci/ci_check_serve_range_bounded.sh`; `cross_ref = [DC-LIVEMEM-01, DC-NODE-13, DC-NODE-07, CN-CONS-07, DC-CONS-17]`. Closes the PHASE4-N-U cross-slice security-review MEDIUM (peer-driven serve resource amplification); the serve-side analog of `DC-LIVEMEM-01`. |

**Strengthenings (`strengthened_in += "PHASE4-N-AA"`) — exactly two, no rule weakened:**

| Rule | Family / Tier | Strengthening |
|------|---------------|---------------|
| `DC-NODE-13` | DC / `derived` (`enforced`, unchanged) | **Serve-as-durable-chain projection — now bounded.** N-U S3 made the `--mode node` served view a read-only projection of the durable ChainDb (`ChainDbServedSource`). N-AA bounds that projection: it now serves via the S1 bounded primitives behind the fixed `MAX_SERVE_RANGE_BLOCKS = 256` cap, fail-closed on oversized ranges. The projection invariant is preserved + strengthened (coherent durable history A→B, now **bounded** per peer request). |
| `DC-LIVEMEM-01` | DC / `derived` (`enforced`, unchanged) | **Receive-side bounded memory — symmetric serve-side bound added.** DC-LIVEMEM-01 bounds the **receive** path (the WirePump lookahead cap `MAX_WIRE_PUMP_LOOKAHEAD = 256`, G-E). N-AA adds the **symmetric serve-side** bound (`MAX_SERVE_RANGE_BLOCKS = 256`, `DC-SERVEMEM-01`), closing the matching peer-driven memory exposure on the **serve** path. The bounded-memory discipline is preserved + extended to both directions. |

> **In-cluster security-review fix (load-bearing, NOT a discipline gap).** The PHASE4-N-AA per-cluster
> security review **found** a MEDIUM **inverted-range panic** (a peer controls both BlockFetch range
> endpoints; an inverted `from > to` range reached `BTreeMap::range` in the `InMemoryChainDb`
> primitive), and the cluster **fixed it in-cluster** (`5c9f6cf6`): a `from > to → empty` guard on
> **both** `ChainDb` impls, plus a contract test (`range_bytes_capped_inverted_range_is_empty`, run
> against both impls) and a serve-path regression test (`serve_range_inverted_range_fails_closed`).
> Both gating reviews **PASS**: the per-slice reviews on S1/S2 and the cross-slice review found + closed
> the finding before close. This is surfaced here and in the cluster doc, **not hidden**.

**No rule was removed (expected: 0).** The registry delta is **one new enforced rule + two
`strengthened_in` appends** — purely additive / strengthening, consistent with append-only registry
discipline.

## Working tree at HEAD `b0365df0`

Clean of tracked changes from this span — the cluster + close are all committed. `git status --short`
shows only an untracked `.mithril-scratch/` (operator scratch, ignored). **This regen runs *after* all
eight span commits** (the close commit `b0365df0` is HEAD), so there is no close-in-progress working
tree; the baseline bump (`999199f8 → b0365df0`) is the only follow-on action.

## Honest residual (window scope)

PHASE4-N-AA **bounds the serve path** — and that is the entire claim. The honest boundary:

- **Pre-RO-LIVE hardening, NOT a capability flip.** This is hardening **item 1** (peer-driven serve
  bound). **No `RO-LIVE` rule was flipped** — `RO-LIVE-01` stays operator-gated. No authoritative
  behavior changed; the span is **RED-only** (0 BLUE change, 458 canonical types unchanged). It closes
  a known **MEDIUM** security exposure; it does not advance the bounty.
- **The cap is defensive, not semantic.** `MAX_SERVE_RANGE_BLOCKS = 256` is a **fixed implementation
  bound** (symmetric with the receive-side `MAX_WIRE_PUMP_LOOKAHEAD`), not a Cardano protocol
  parameter. A peer requesting more than 256 blocks in one range gets `NoBlocks` (fail-closed), not a
  partial-then-continue stream — within-cap serving is unaffected; multi-batch fetch across the cap is
  the peer's responsibility (standard BlockFetch behavior).
- **Trusted-caller reads are out of scope (by design).** The unbounded `iter_from_slot` / `tip`
  internals are **unchanged** — they remain for **trusted, full-range internal callers** (node startup,
  recovery, rollback) and are **doc-fenced** as such. N-AA bounds only the **peer-driven serve path**;
  the recovery/rollback paths are not peer-controllable and are deliberately left full-range.
- **The N-U LOW residual is partially addressed; one item remains.** The PHASE4-N-U honest residual
  named two follow-ons: **[MEDIUM]** `iter_from_slot` full-range materialization + O(N²) hash recovery
  with no per-request serve cap — **closed by this cluster** (`DC-SERVEMEM-01`); and **[LOW]** > 64 KB
  block bodies not served (fail-closed) + unbounded inbound serve accept — **still open** (a separate
  pre-RO-LIVE item).
- **CODEMAP refresh owed on next cluster.** The close commit `b0365df0` did **not** regenerate CODEMAP,
  so CODEMAP is slightly stale on the N-AA RED additions (`CappedSlotRange`, `ServeRangeOutcome`,
  `range_bytes_capped`, `last_block_bytes`) — a `/codemap` refresh item, not a discipline gap (the rule
  count is current at 334, and the registry records the rule↔gate binding authoritatively).

---

## Historical — PHASE4-N-U close + gate-hygiene window (`4e358e92 → 999199f8`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It was the
> **PHASE4-N-U cluster CLOSE + a gate-hygiene / close-correction tail**, narrating the `4e358e92 →
> 999199f8` span. Counts in this Historical section are the figures **at `999199f8`** (333 rules, 135
> CI gates, 458 canonical types); the current window measures **forward** from `999199f8`. The full
> §§0–7 narrative is recoverable from this doc's git history at `999199f8`.

> Baseline: `4e358e92` (refresh stale G-R serve-handoff comment in containment gate (post-N-U-S3), 2026-06-05 17:17)
> HEAD: `999199f8` (repair 10 pre-existing gate-vs-code drifts (gate hygiene; 0 invariants weakened), 2026-06-05 19:28)
> Span: **PHASE4-N-U cluster CLOSE + a gate-hygiene / close-correction tail** — 4 commits, 23 files, +1063 / −658.

The window was **not a feature cluster** — it was the **PHASE4-N-U close pass** (commit `7f00e75d`,
docs-only: archive + 4-grounding-doc refresh + baseline bump `65954fa3 → 4e358e92`) plus a
**gate-hygiene / close-correction tail** of three CI-only commits (`60deecf3`, `e92b40b7`, `999199f8`).
It answered one operational question the N-U close left open: *is the `ci/ci_check_*.sh` sweep
trustworthy as release evidence — does GREEN actually mean GREEN?* At the N-U close the answer was
**no** (the close record named "12 pre-existing gate failures"). The window repaired **every failing
gate in place** — adding no gate, removing no gate, weakening no invariant:

- **The close itself (`7f00e75d`, docs-only).** Archived the N-U cluster doc, refreshed all four
  grounding docs to **458 BLUE types / 135 CI checks / 333 rules**, flipped S2/S3 slice status, bumped
  the baseline. **0 code / 0 CI / 0 registry.**
- **The N-U-stranded gate, reconciled (`60deecf3`).** N-U S3 retired the `--mode node` spine
  `SelfAcceptedHandoff → push_atomic` accumulator (`DC-NODE-13`), so the DC-NODE-06 handoff-fence gate
  `ci_check_served_chain_handoff_fence.sh` **inverted** (the masked stranding). Repointed (not retired —
  count stays 135) to fence the **evolved durable-provenance serve** (node-spine serve sources only
  `ServedChainSource::DurableChainDb`); `DC-NODE-06 strengthened_in += "PHASE4-N-U"`. **No code.**
- **The silent secret-scan, made to actually run (`e92b40b7`).** `ci_check_no_secrets.sh` was exiting
  **126** ("Argument list too long") and **silently not scanning** (a security gate with zero
  protection — env-packed `ls-files` list hit `ARG_MAX`). Fixed by passing the file list via a temp
  file; the scan now runs (**6756 files, 0 secrets**), with IPv4 false-positive tuning (real AKIA / PEM
  / hostname / routable-IP patterns still fail closed).
- **Ten pre-existing gate-vs-code drifts repaired (`999199f8`).** Nine gate scripts with stale grep
  patterns / allow-lists / paths + two comment-only source edits. Each fix only stops a false positive
  or repoints a stale path; the protected invariant holds in code in every case.

**N-U-close-window headline (at `999199f8`):** CI gates **135 → 135** (0 net — **11 gates repaired in
place**, count held at 135 to avoid churn); registry **333 → 333** (identical ID set — the lone edit was
the `DC-NODE-06` strengthening, not a new rule); status **201 / 20 / 112** unchanged; BLUE types **458 →
458**. The headline claim: the full `ci/ci_check_*.sh` sweep was **135 passed / 0 failed** at HEAD
(verified by running it). Triage of the N-U close's 12 gate failures: **0 genuine code-invariant
regressions** — 11 stale-gate drift + 1 N-U-stranded `DC-NODE-06` gate. **No `RO-LIVE` flip, no behavior
change** — pure enforcement-trustworthiness work. The two source edits were comment-only
(`block_validity/mod.rs` Core-Contract header; `seed_import/importer.rs` stale reference-script doc
line).

> **Connecting note to the current window.** The N-U *close + gate-hygiene* window above is the prior
> lead; its tail commit was the focused refresh `610d666a`, which sits at the **head** of the *current*
> window (`999199f8 → b0365df0`) — it refreshed SEAMS + TRACEABILITY + HEAD_DELTAS for the hygiene span
> and bumped the baseline to `999199f8`, then the **PHASE4-N-AA cluster** (bounded peer-driven serve
> range) landed on top. See §§0–7 above.

---

## Historical — PHASE4-N-U cluster window (`65954fa3 → 4e358e92`)

> Preserved in condensed form. The single-cluster lead **PHASE4-N-U — forged-block durability**,
> narrating the `65954fa3 → 4e358e92` span. Counts here are the figures **at `4e358e92`** (333 rules,
> 135 CI gates, 458 canonical types). The full N-U §§0–7 narrative is recoverable from this doc's git
> history at `4e358e92` / `999199f8`.

> Baseline: `65954fa3` (G-K…G-R + C1 catch-up close, 2026-06-04 23:32)
> HEAD: `4e358e92` (refresh stale G-R serve-handoff comment in containment gate (post-N-U-S3), 2026-06-05 17:17)
> Span: **PHASE4-N-U — forged-block durability** (own-forged durable admit → forged-tip crash recovery + replay-equivalence → serve-as-durable-chain projection) — 14 commits, 28 files, +3726 / −1802.

PHASE4-N-U answered: *once Ade forges its own block, does it become part of the **durable** chain —
survive a crash, replay byte-identically, and get served to a follower — through the SAME gate received
blocks use, with NO second tip-advance path?* Before N-U a forged block was a **local self-accept
artifact only** (`DC-NODE-05`). N-U closed that across three slices:

- **S1 — own-forged durable admit through the pump (`DC-NODE-12` + `DC-CONS-23` + `DC-WAL-04` prior-fp
  clause).** A fenced RED driver `ade_node::node_sync::admit_forged_block_durably` feeds the
  self-accepted bytes (`accepted.into_bytes()`, no re-encode) into the **same**
  `forward_sync::pump_block` chokepoint received blocks use (durable-before-tip, extend-only). New gate
  `ci_check_forged_durable_admit_via_pump.sh`.
- **S2 — forged-tip crash recovery + replay-equivalence (`T-REC-05`, `DC-WAL-04` no-orphan clause).**
  Production `warm_start_recovery` forward-replays from the nearest snapshot ≤ tip and reconciles the
  WAL tail; an un-WAL'd forged orphan is dropped. `T-REC-05` is **test-enforced** (`ci_script = ""`).
- **S3 — serve-as-durable-chain projection (`DC-NODE-13`; strengthens `CN-CONS-07`, `DC-NODE-11`).**
  The `--mode node` served view became a deterministic read-only **projection of the durable ChainDb**
  (the NEW RED module `ade_runtime::network::served_chain_projection` / `ChainDbServedSource`). New gate
  `ci_check_served_chain_projection.sh`; retired gate `ci_check_served_chain_stability.sh` (mechanism
  superseded).

**N-U headline (at `4e358e92`):** Registry **328 → 333** (+5 enforced: `DC-NODE-12`, `DC-CONS-23`,
`DC-WAL-04`, `T-REC-05`, `DC-NODE-13`; +2 strengthenings: `CN-CONS-07`, `DC-NODE-11`; 0 removed). CI
gates **134 → 135** (+1 net: +2 new, −1 retired). **One new RED module**
(`served_chain_projection`). **BLUE canonical types 458 → 458.** **No `RO-LIVE` flip** — durability +
coherent serve ≠ operator-witnessed peer acceptance. *(N-AA, the current lead, bounds this S3 serve
projection — `DC-NODE-13 strengthened_in += "PHASE4-N-AA"`.)*

---

## Historical — PHASE4-N-F-G-K … G-R + C1 window (`550eec3a → 65954fa3`)

> Preserved in condensed form. A **multi-cluster catch-up** narrating the `550eec3a..65954fa3` span —
> the PHASE4-N-F-G-J close-pass + eight clusters (G-K through G-R) + the C1 genesis-successor rehearsal
> reproduction evidence. Counts here are the figures **at `65954fa3`** (328 rules, 134 CI gates, 458
> canonical types). The full G-K…C1 §§0–7 narrative (and the G-J window before it) is recoverable from
> this doc's git history at `65954fa3` / `4e358e92` / `999199f8`.

> Baseline: `550eec3a` (PHASE4-N-F-G-J close, 2026-06-03 22:02)
> HEAD: `65954fa3` (run-2 genesis-rehearsal reproduction + runbook flag fixes + gate now covers c1 manifests, 2026-06-04 23:32)
> Span: **G-J close-pass → G-K, G-L, G-M, G-N, G-O, G-P, G-Q, G-R → C1 genesis-successor rehearsal evidence** — 28 commits, 73 files, +4967 / −243.

Ade closed **eight clusters** (G-K through G-R) plus a G-J close-pass and a C1 genesis-successor
rehearsal evidence pass, each peeling off the next blocker toward a live C1 genesis-successor follower
adopting an Ade-forged block 0: serve-listener lifetime (G-K, `DC-NODE-09`) → real-node handshake
compat (G-L, `CN-WIRE-10`) → real-node ChainSync FindIntersect compat (G-M, `CN-WIRE-11`, + the closed
BLUE enum `ArrayHead = Definite(u64) | Indefinite`, the window's only +1 canonical type, 457 → 458) →
recovered-eta0 WarmStart (G-N, `T-REC-04` + `DC-CINPUT-03`) → feed-side tag-24 unwrap (G-O, `CN-WIRE-12`)
→ feed-side leader-threshold view (G-P, `DC-CINPUT-04`) → forge-successor position (G-Q, `DC-NODE-10`) →
stable served block 0 via a monotone serve gate (G-R, `DC-NODE-11`) → and the C1 reproduction evidence.

**G-K…C1 headline (at `65954fa3`):** CI gates **126 → 134** (+8, one per cluster); registry **319 →
328** (+9, all `enforced`); BLUE canonical types **457 → 458** (+1 `ArrayHead`); no new module. **Note:**
the G-R gate `ci_check_served_chain_stability.sh` was **retired in PHASE4-N-U** (mechanism superseded by
serve-as-projection), and `DC-NODE-11` was strengthened there; `DC-NODE-11`'s stranded sibling
`DC-NODE-06` was reconciled in the N-U close window (`60deecf3`).

> *(The G-E…G-I leads were never re-led in HEAD_DELTAS — each was closed with its own grounding-doc
> refresh. The G-J lead before that is recoverable from this doc's git history at `65954fa3`.)*

---

## Generation notes

### Regen `999199f8 → b0365df0` (PHASE4-N-AA — bounded peer-driven serve range — current lead)

- **Baseline valid; single-cluster lead (RED-only) preceded by the prior-lead's refresh tail.** Run
  against the config baseline `999199f8` (the PHASE4-N-U-close-window HEAD), which `git rev-parse`
  resolves and `git merge-base 999199f8 HEAD` confirms is a strict ancestor of HEAD `b0365df0`
  (`999199f8` carries no tag). The span is the **focused grounding refresh** `610d666a` (the prior
  lead's tail — refreshed SEAMS + TRACEABILITY + HEAD_DELTAS, bumped baseline to `999199f8`) **plus the
  PHASE4-N-AA cluster** (7 commits: cluster doc + S1 + S2 + security-review MEDIUM fix + close). The
  closer bumps `head_deltas_baseline` `999199f8 → b0365df0` after this regen.
- **Counts are mechanical (git/grep/ls + one gate run):** commit log + `--shortstat` over
  `999199f8..b0365df0` (**8** commits, no merges / **15** files / **+1254 / −492**); CI gate count via
  `git ls-tree -r --name-only <ref> ci/ | grep -c 'ci_check_.*\.sh$'` at each ref (**135 → 136**;
  `--diff-filter=A` over `ci/` = exactly `ci_check_serve_range_bounded.sh`; `--diff-filter=M` and
  `--diff-filter=D` over `ci/` both **empty**); registry rule count via `grep -cE '^\s*id\s*='` at each
  ref (**333 → 334**; `comm` of sorted id lists shows the single addition `DC-SERVEMEM-01`, zero
  removals); registry status via `grep -E '^status = ' | sort | uniq -c` (**201 → 202 enforced**, 20
  partial / 112 declared unchanged); strengthenings via `grep -c 'strengthened_in = \["PHASE4-N-AA"\]'`
  (**2**: `DC-NODE-13` + `DC-LIVEMEM-01`); BLUE canonical types via the CODEMAP header (**458 → 458**).
- **RED-only span — no BLUE touch, +0 canonical type, no Cargo.toml change.** `git diff --name-status
  999199f8..b0365df0` shows **no new `.rs` source file** (only `A` for one CI gate + three cluster/slice
  docs, `M` for six RED `.rs` files + four docs + the registry + `.idd-config.json`, `R` for the three
  cluster-doc archive renames). All six touched `.rs` files are RED (`ade_runtime::chaindb` +
  `ade_runtime::network::served_chain_projection`). `git diff --name-only … '**/Cargo.toml' 'Cargo.toml'`
  is empty (no feature-flag delta).
- **New gate verified by running it.** `bash ci/ci_check_serve_range_bounded.sh` at HEAD exits **0**
  ("serve range bounded — S1 bounded primitives only … fixed non-configurable MAX_SERVE_RANGE_BLOCKS
  cap, hash via decode_block").
- **Registry delta is +1 enforced rule + 2 strengthenings, NOT a removal.** `DC-SERVEMEM-01` is the new
  rule (declared at `a6e184e8`, enforced at the S2 close); `DC-NODE-13` + `DC-LIVEMEM-01` gained
  `strengthened_in += "PHASE4-N-AA"` at the close (`b0365df0`). `comm` confirms zero removals.
- **Doc-refresh split (important for cross-references).** SEAMS + TRACEABILITY + HEAD_DELTAS were
  refreshed in the **span-opening** commit `610d666a` (the prior hygiene lead's tail), **before**
  `DC-SERVEMEM-01` was enforced. The N-AA **close commit `b0365df0`** touched **only** the registry +
  the three cluster-doc archive renames — it did **not** re-touch CODEMAP / SEAMS / TRACEABILITY.
  Consequence: **CODEMAP is slightly stale** on the N-AA RED additions (`CappedSlotRange` /
  `ServeRangeOutcome` / `range_bytes_capped` / `last_block_bytes` — `grep` count 0 in CODEMAP), and
  **TRACEABILITY** lacks the `DC-SERVEMEM-01 ↔ ci_check_serve_range_bounded.sh` row (the registry holds
  it authoritatively). Both are refresh-on-next-cluster items, not discipline gaps.
- **In-cluster security-review MEDIUM, fixed in-cluster.** The per-cluster security review found an
  inverted-range (`from > to`) panic exposure (peer controls both BlockFetch endpoints); `5c9f6cf6`
  added a `from > to → empty` guard on both `ChainDb` impls + a contract test
  (`range_bytes_capped_inverted_range_is_empty`, both impls) + a serve test
  (`serve_range_inverted_range_fails_closed`). Both gating reviews PASS.
- **Working tree clean.** This regen runs *after* all eight span commits (the close `b0365df0` is HEAD);
  `git status --short` shows only an untracked `.mithril-scratch/` (operator scratch, ignored). The only
  follow-on action is the baseline bump `999199f8 → b0365df0`.
