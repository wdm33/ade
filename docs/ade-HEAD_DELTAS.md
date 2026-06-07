# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `a76672b9` (AE.E chain-sync server FindIntersect cursor — CE-A5 manifest achieved, 2026-06-07 12:26)
> HEAD: `6363683e` (AE.F receive idempotency — survive the post-adoption echo, 2026-06-07 13:40)
> Span: **PHASE4-N-AE.F — the post-CE-A5 echo-idempotency follow-up** — one impl slice (`AE.F`) on the receive-side durable-admit chokepoint, plus the PHASE4-N-AE close grounding-doc refresh (`62811a4e`, the *baseline's* docs commit) folded into the span head.
> **4 commits** (no merges), **13 files changed, +1401 / −473 lines** — but the file/line totals are dominated by `62811a4e`, the N-AE close grounding-doc refresh (it rewrote CODEMAP/SEAMS/TRACEABILITY/HEAD_DELTAS: +947 / −473 across 7 docs/config files). The **AE.F impl** (`6363683e`) is **+228 / −3 across 5 files** — and the *production* change is **two RED files**: `crates/ade_runtime/src/forward_sync/pump.rs` (+114, the gate + 2 tests) and `crates/ade_node/src/node_sync.rs` (+41, 1 live-shape test), plus the new gate `ci/ci_check_receive_idempotency.sh` (+43) and the registry (+24).

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`a76672b9`**, the
> PHASE4-N-AE.E CE-A5 closer (the prior HEAD_DELTAS HEAD) — and it is **valid**: `git rev-parse a76672b9`
> resolves and `git merge-base a76672b9 HEAD == a76672b9` (it is a strict ancestor of HEAD; `a76672b9`
> carries no tag). HEAD is **`6363683e`** (the AE.F idempotency fix). The config baseline at the start of
> this regen was already `a76672b9` (the previous close bumped it), so the window measures cleanly from the
> recorded baseline forward. The span has **two parts**: (1) the **PHASE4-N-AE close grounding refresh**
> `62811a4e` — the docs/config commit that *wrote the previous HEAD_DELTAS lead* and brought all four
> grounding docs + `.idd-config.json` current at `a76672b9` (it is the baseline's own docs commit, so it
> contributes the bulk of the file/line churn but **zero** production code); and (2) the **PHASE4-N-AE.F
> slice** (3 commits: invariants sketch `d11bdbe8` → slice doc `8049dd43` → impl `6363683e`) — the
> **post-CE-A5 echo-idempotency** follow-up. The closer bumps `head_deltas_baseline` `a76672b9 → 6363683e`
> after this regen so the next cluster measures from here.

This window is **led by PHASE4-N-AE.F — receive idempotency at the durable-admit chokepoint.** It is the
**post-CE-A5 echo** follow-up: after the real `cardano-node 11.0.1` relay **adopted** Ade's forged block 17
(`AddedToCurrentChain`, the CE-A5 manifest closed in AE.E), the relay **re-announced that block back over
Ade's follow link**, and the BLUE header authority **correctly** rejected it as
`SlotBeforeLastApplied{last=421, attempted=421}` — which **terminated the run (exit 43) *after*
`AddedToCurrentChain`.** That is **not** a manifest blocker (the adoption already happened and is recorded),
but it **ends a continuous run** — so it is a **prerequisite for long-running C2-LOCAL / preprod relay
operation.** AE.F closes it in one slice:

- **AE.F — receive idempotency at the durable-admit chokepoint (`6363683e`; `DC-NODE-16` NEW, enforced).**
  `pump_block` (the **RED** durable-admit chokepoint in `ade_runtime::forward_sync::pump`), **immediately
  after `decode_block` and BEFORE the BLUE chokepoint reducer**, queries
  `db.get_block_by_hash(&decoded.block_hash)`; if `Some(stored)` **and** `stored.slot ==
  decoded.header_input.slot`, it returns **`Ok(None)`** — an **idempotent no-op** (no reducer step, no WAL
  append, no tip change). The skip is **hash-keyed** (`get_block_by_hash` is hash-exact), **never slot
  alone**: a **different** block (different hash) at/before the last-applied slot returns `None` here, falls
  through to the **unchanged** BLUE authority, and **fails closed** (`SlotBeforeLastApplied` /
  `BlockNoOutOfOrder`). The no-op is **replay-equivalent** — the WAL never records the re-announce. New gate
  `ci/ci_check_receive_idempotency.sh`.

**The headline:** a **re-announced block Ade already durably holds** (the relay echoing Ade's own adopted
tip back) is now an **idempotent no-op** at the receive-side chokepoint, so a continuous recover→follow run
**survives the post-adoption echo** instead of exiting 43. The fix is a **RED chokepoint gate only — NO BLUE
change, no new BLUE type, no new reducer input.** It is a **refinement of the AE.F invariants sketch's
proposed BLUE `ReceiveOutcome::AlreadyHave`:** because `get_block_by_hash` is a **deterministic** durable-store
query (not nondeterminism), the gate correctly lives at the **RED** chokepoint, and the BLUE header authority
(`validate_and_apply_header` / `block_validity`) stays **untouched** and **still fail-closes every block that
reaches it** (`T-REC-05` / `DC-WAL-02` replay preserved). **+0 BLUE canonical type** (the span touches **no**
BLUE `core_paths` file). **No `RO-LIVE` rule flipped** this span.

## 0. Headline

| Count | Baseline (`a76672b9`) | HEAD (`6363683e`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 141 | **142** | **+1** — **one NEW gate** (`--diff-filter=A` over `ci/`): `ci_check_receive_idempotency.sh` (AE.F). **No gate removed** (`--diff-filter=D` over `ci/` empty), **no gate modified** (`--diff-filter=M` over `ci/` empty). |
| Registry rules (`docs/ade-invariant-registry.toml`) | 340 | **341** | **+1** — one NEW rule **`DC-NODE-16`** (PHASE4-N-AE.F). **Zero removed** (`diff` of the sorted `id =` lists shows exactly the single addition `DC-NODE-16` and no removal). |
| Registry status (enforced / partial / declared) | 208 / 20 / 112 | **209 / 20 / 112** | **+1 enforced** — `DC-NODE-16` lands `enforced` at HEAD. Partial / declared counts unchanged. |
| Registry strengthenings | — | **0** | No `strengthened_in` append this span — AE.F is a **net-new rule** (`DC-NODE-16`); it cross-references `T-REC-05` / `DC-WAL-02` / `DC-CONS-03` (preserved, not strengthened). No rule weakened. |
| BLUE canonical types | 458 | **458** | **0** — **BLUE-untouched.** The span touches **no** BLUE `core_paths` file (`git diff a76672b9..HEAD` over the BLUE trees is empty; `^+(pub )?(struct\|enum)` over the BLUE trees = 0). The only production-code change is **RED**: `ade_runtime::forward_sync::pump` (+114) and the `ade_node::node_sync` test module (+41). |
| Grounding docs | refreshed for the **N-AE** close in `62811a4e` (the baseline's docs commit, span head) | **this HEAD_DELTAS adds the AE.F lead; CODEMAP/SEAMS/TRACEABILITY carry AE.F via the registry** | The span-head commit `62811a4e` regenerated all four grounding docs **for the N-AE close at `a76672b9`** (CODEMAP/SEAMS/TRACEABILITY/HEAD_DELTAS to 458 types / 141 CI / 340 rules). AE.F adds **one rule + one gate**; the **registry records `DC-NODE-16` + its `ci_check_receive_idempotency.sh` binding authoritatively at HEAD** (341 rules). This HEAD_DELTAS prepends the AE.F lead; CODEMAP/SEAMS/TRACEABILITY remain accurate on module inventory (AE.F adds **no module, no type** — only a RED chokepoint step inside an existing module) and pick up the `DC-NODE-16` row on their next regen, with the registry authoritative in the interim. |

This is a **single-slice lead** — the PHASE4-N-AE close grounding refresh (span head, the baseline's docs
commit) followed by the **PHASE4-N-AE.F** idempotency slice. The slice↔rule↔gate map:

| Slice | Rule(s) | Gate | What shipped |
|---|---|---|---|
| **AE.F** (`6363683e`) | **`DC-NODE-16`** (NEW, enforced) | **`ci_check_receive_idempotency.sh`** (NEW) | Receive idempotency at the RED durable-admit chokepoint: `pump_block`, after `decode_block` and **before** the BLUE reducer, returns `Ok(None)` for an already-have block (`get_block_by_hash` hit at the same slot). Hash-keyed (never slot-alone); a different block at/before the last-applied slot still reaches the unchanged BLUE authority and fails closed. No-op is replay-equivalent (no WAL append). **No BLUE change.** |

The per-commit shape:

| Commit | Kind | What it did | Code / CI / registry effect |
|--------|------|-------------|-----------------------------|
| `d11bdbe8` | docs (invariants sketch) | AE.F echo-idempotency invariants sketch (post-adoption follow echo) | **0 code / 0 CI / 0 registry**; `docs/planning/phase4-n-ae-f-echo-idempotency-invariants.md` (+132) |
| `62811a4e` | docs (N-AE close refresh) | Grounding-doc refresh for the PHASE4-N-AE close (CE-A5 manifest) | **0 code / 0 CI**; regenerated CODEMAP/SEAMS/TRACEABILITY/HEAD_DELTAS + `.idd-config.json` (+947 / −473 across 7 files); **0 registry rule added/removed** (the registry was already at 340 from the N-AE slice impls) |
| `8049dd43` | docs (slice doc) | AE.F slice doc — receive idempotency at the durable-admit chokepoint | **0 code / 0 CI / 0 registry**; `docs/clusters/PHASE4-N-AE/slices/AE.F.md` (+97) |
| `6363683e` | fix (AE.F impl) | AE.F — receive idempotency; survive the post-adoption echo | **RED code** (`pump.rs` +114 = gate + CE-F1/CE-F2; `node_sync.rs` +41 = CE-F4); **+1 CI** (`ci_check_receive_idempotency.sh`); registry: `DC-NODE-16` NEW `enforced` (340 → 341) |

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `6363683e` | fix | AE.F receive idempotency — survive the post-adoption echo (PHASE4-N-AE.F, DC-NODE-16) |
| `8049dd43` | docs | AE.F slice doc — receive idempotency at the durable-admit chokepoint |
| `62811a4e` | docs | grounding-doc refresh for PHASE4-N-AE close (CE-A5 manifest) |
| `d11bdbe8` | docs | AE.F echo-idempotency invariants sketch (post-adoption follow echo) |

No merge commits in the span. **4 commits, zero unclassified** — `6363683e` carries an explicit
`fix(phase4-n-ae)` conventional-commits prefix; the other three are `docs(...)` / `docs:` (the AE.F
invariants sketch + slice doc, and the N-AE close grounding refresh, whose diff scope is exclusively
`docs/` + `.idd-config.json`). The shape is **N-AE close refresh (`62811a4e`, baseline's docs commit) +
AE.F (invariants sketch → slice doc → impl)** — note the invariants sketch `d11bdbe8` lands *before* the
close refresh `62811a4e` in commit time (the sketch was written while the N-AE close pass was still
underway), then the slice doc + impl follow. All AE.F work landed 2026-06-07.

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty
> trailer requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that
> is an Ade-local override of the global no-AI-attribution rule and applies to **commit messages
> only**. It does not affect this doc's content.

## 2. New Modules

**None.** `git diff --diff-filter=A --name-only a76672b9..HEAD -- '*.rs'` shows **no new `.rs` source file**
(not even a test file), no new crate, no new `Cargo.toml`, no new workspace. AE.F is **modification only** —
it adds a step inside the **existing** RED chokepoint `pump_block` (in the existing module
`crates/ade_runtime/src/forward_sync/pump.rs`) and three tests inside **existing** test modules
(`pump.rs` and `crates/ade_node/src/node_sync.rs`). The only added files this span are **one CI gate**
(`ci/ci_check_receive_idempotency.sh`, §5), the AE.F **invariants sketch + slice doc**
(`docs/planning/phase4-n-ae-f-echo-idempotency-invariants.md`,
`docs/clusters/PHASE4-N-AE/slices/AE.F.md`), and the regenerated grounding docs (`62811a4e`).

> **Cross-reference (CODEMAP/SEAMS) — no new surface, no new module.** AE.F adds **no module and no type**
> — only a **deterministic durable-store read + early `Ok(None)`** inside the existing RED chokepoint
> `pump_block`. The host module (`ade_runtime::forward_sync::pump`) is already in CODEMAP as RED; the
> `get_block_by_hash` / `tip` ChainDb primitives the gate uses already exist. CODEMAP/SEAMS need **no new
> module/type/TCB entry** for this span; they pick up the `DC-NODE-16` rule row on their next regen, with
> the registry holding it authoritatively in the interim (the `code_locus` field names the exact site).

## 3. Modules Modified

Two modules changed this span — **both RED**, **+0 canonical type**:

| Module | Color / scope | Key changes |
|--------|---------------|-------------|
| `ade_runtime::forward_sync::pump` (`crates/ade_runtime/src/forward_sync/pump.rs`) | RED shell, +114 / 0 | **AE.F (`6363683e`):** the receive-idempotency gate in `pump_block` — immediately after `decode_block` and **before** the BLUE chokepoint reducer (`RollForward` / `BlockDelivered`), `pump_block` queries `db.get_block_by_hash(&decoded.block_hash)`; on `Some(stored)` with `stored.slot == decoded.header_input.slot` it returns **`Ok(None)`** (no reducer step, no WAL append, no tip change). Hash-exact (a different block returns `None` and falls through to the unchanged BLUE authority → fail-closed). Plus the two unit tests `pump_block_reannounced_block_is_idempotent_noop` (CE-F1) and `pump_block_different_block_at_or_before_tip_still_fails_closed` (CE-F2). **No new type; the gate reuses existing ChainDb primitives (`get_block_by_hash`); the BLUE reducer call is unchanged.** |
| `ade_node::node_sync` (`crates/ade_node/src/node_sync.rs`) | RED, +41 / 0 | **AE.F (`6363683e`):** the live-shape hermetic regression test `run_node_sync_survives_reannounced_block_in_feed` (CE-F4) — drives `run_node_sync` over a feed containing a duplicate (`[block, same-block]`), asserts the loop **completes**, admits the block **exactly once**, and does **not** exit 43. **Test-only addition; no production-code change to this module this span.** |

> **No BLUE change this span (load-bearing).** `git diff a76672b9..HEAD` over the BLUE `core_paths` trees
> is **empty** — AE.F touches **no** BLUE file. The fix is deliberately a **RED chokepoint** gate: the
> AE.F invariants sketch initially proposed a BLUE `ReceiveOutcome::AlreadyHave`, but because
> `get_block_by_hash` is a **deterministic** durable-store query (not nondeterminism), the gate belongs at
> the RED chokepoint with **no new BLUE reducer input** and the BLUE authority untouched. The BLUE
> canonical-type count is **458 → 458** (`^+(pub )?(struct\|enum)` over the BLUE trees = 0). The header /
> body authorities, the KES verifier, forge eligibility, and the closed wire grammar are unchanged.

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace `Cargo.toml`, and
**no `Cargo.toml` changed in this window** (`git diff --name-only a76672b9..HEAD -- '**/Cargo.toml'
'Cargo.toml'` is empty). No `#[cfg(feature = …)]` gate was introduced. The AE.F behavior is governed by a
**fixed, typed** construct (the hash-exact `get_block_by_hash` already-have gate returning `Ok(None)` before
the BLUE reducer) — not a feature flag, CLI flag, env var, or config knob.

## 5. CI Checks (141 → 142; +1 new gate, 0 gates modified, 0 gates removed)

One new gate this span; no gate modified, no gate removed. `git diff --diff-filter=A a76672b9..HEAD -- ci/`
lists exactly the one gate below; `--diff-filter=D` and `--diff-filter=M` over `ci/` are both **empty**.

### PHASE4-N-AE.F gate (`6363683e`)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_receive_idempotency.sh` | **New** | PHASE4-N-AE.F (`6363683e`); `DC-NODE-16` | The receive-side already-have skip in `pump_block` (CE-F3) MUST be **(a) hash-keyed** — `get_block_by_hash` on the decoded block hash, never slot-only; **(b) placed BEFORE the BLUE chokepoint reducer** (`forward_sync_step` / the `RollForward`+`BlockDelivered` steps), so the no-op runs no reducer step and appends nothing to the WAL; **(c) slot-consistent** — the skip requires `stored.slot == decoded.slot`. (Static-grep over the production region of `crates/ade_runtime/src/forward_sync/pump.rs`, excluding the `#[cfg(test)]` module.) |

> **Cross-reference (TRACEABILITY) — new binding, no removal.** The new rule↔gate binding
> (`DC-NODE-16` ↔ `ci_check_receive_idempotency.sh`) is recorded **authoritatively in the registry** at
> HEAD (`DC-NODE-16.ci_script = "ci/ci_check_receive_idempotency.sh"`, `tests = [CE-F1, CE-F2, CE-F4]`).
> TRACEABILITY was regenerated for the N-AE close at `a76672b9` (`62811a4e`); it picks up the `DC-NODE-16`
> row on its next regen, with the registry authoritative in the interim. **No rule↔gate binding was
> removed.** This gate is **not** an orphan — it enforces exactly `DC-NODE-16`. |

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. **No canonical type was
added or removed in this window** — the BLUE count is unchanged (**458 → 458**). AE.F adds **no `struct`/`enum`**
anywhere (it is a RED chokepoint step reusing existing primitives). No `Cargo.toml` changed.

## 7. Normative / Invariant Rule Delta (340 → 341; +1 enforced rule, 0 strengthenings, zero removals)

**One rule ID was added; zero removed** (340 → 341; `diff` of the sorted `id =` lists shows exactly the
single addition `DC-NODE-16` and no removal). The status tally moves **208 → 209 enforced** (20 partial /
112 declared unchanged) — the new rule is `enforced` at HEAD.

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs, the
CE-73 reclassification, and `CLAUDE.md` — were **not** changed this span: `git diff --name-only
a76672b9..HEAD` over those paths is empty. The rule-count delta is entirely the invariant-registry change
below.)*

**New rule (`+1`, enforced):**

| Rule | Family / Tier | Statement (summary) |
|------|---------------|---------------------|
| `DC-NODE-16` | DC / `derived` (enforced; `introduced_in = "PHASE4-N-AE"`) | **Receive idempotency at the durable-admit chokepoint.** A peer-delivered block already durably present **byte-identically** in the ChainDb (**same slot, same hash**) is an **idempotent no-op** at `pump_block` — no validation step, no WAL append, no tip change; the post-state is identical and replay-equivalent. A **different** block (different hash) at/before the last-applied slot is **NOT** short-circuited: it reaches the unchanged BLUE header authority and **fails closed** (`SlotBeforeLastApplied` / `BlockNoOutOfOrder`). The skip is gated on **hash equality** vs the durable store, **never slot alone**; **no skip-past, no fork-choice** (`DC-CONS-03` untouched). `ci_script = ci/ci_check_receive_idempotency.sh`; `cross_ref = [DC-NODE-12, DC-PROTO-09, DC-PROTO-10, DC-CONS-03, T-REC-05, DC-WAL-02]`. |

**Strengthenings (`strengthened_in +=`) — 0:** none this span. `DC-NODE-16` is a net-new rule; it
**cross-references** `T-REC-05` (warm-start replay-equivalence) and `DC-WAL-02` (WAL chain-link) — both
**preserved** (the no-op appends nothing to the WAL, so replay is unaffected) — but neither is a
`strengthened_in` append. No rule was weakened.

> **Post-CE-A5 echo — what AE.F closes (and what it does not).** AE.F is the **follow-up** to the CE-A5
> manifest (the real `cardano-node 11.0.1` relay `AddedToCurrentChain` Ade's forged block 17 @ slot 421,
> closed in AE.E). After adoption the relay **re-announced** that block over Ade's follow link; the BLUE
> header authority **correctly** rejected `SlotBeforeLastApplied{last=421, attempted=421}`, which ended the
> run (**exit 43**) *after* the adoption. AE.F makes that echo an **idempotent no-op** so a continuous run
> survives it. The CE-A5 manifest itself is **already recorded** (it backs `DC-NODE-14` / `DC-PROTO-10`,
> AE.E) — AE.F neither re-claims it nor flips any `RO-LIVE` rule; it is a **continuous-run prerequisite**
> for long-running C2-LOCAL / preprod relay operation.

**No rule was removed (expected: 0).** The registry delta is **one new enforced rule, zero
`strengthened_in` appends, zero removals** — consistent with append-only registry discipline.

## Working tree at HEAD `6363683e`

Clean of tracked changes from this span — the N-AE close grounding refresh and the AE.F slice (invariants
sketch → slice doc → impl) are all committed. `git status --short` shows only an untracked
`.mithril-scratch/` (operator scratch, ignored). **This regen runs *after* all 4 span commits** (the AE.F
impl `6363683e` is HEAD for this window); the registry records `DC-NODE-16` + its gate binding
authoritatively at HEAD (341 rules). The remaining close-pass actions are this HEAD_DELTAS and the
baseline bump (`a76672b9 → 6363683e`).

> **Cluster-context note.** AE.F is a **slice of PHASE4-N-AE** (the cluster doc + slice docs live at the
> active path `docs/clusters/PHASE4-N-AE/`). The AE.F impl `6363683e` is a `fix(...)` commit, not a formal
> `chore: close` archive commit; whether N-AE is formally archived (moving `docs/clusters/PHASE4-N-AE/` →
> `docs/clusters/completed/PHASE4-N-AE/`) is a close-pass decision separate from this HEAD_DELTAS regen.

## Honest residual (window scope)

PHASE4-N-AE.F **closed the post-adoption echo** — a re-announced already-have block is now an idempotent
no-op, so a continuous recover→follow run survives the relay echoing Ade's own adopted tip. The honest
boundary:

- **AE.F is a continuous-run prerequisite, NOT a new live claim.** The CE-A5 manifest (real relay adopting
  an Ade-forged block) was closed in AE.E and is **already recorded** as `enforced`-backing evidence on
  `DC-NODE-14` / `DC-PROTO-10`. AE.F neither re-claims it nor flips any `RO-LIVE` rule; it removes the
  exit-43 that ended an otherwise-successful run *after* adoption. `RO-LIVE-01` remains as scoped.
- **RED chokepoint only, +0 BLUE / +0 type.** The span touches **no** BLUE file; the fix is a deterministic
  ChainDb read + early `Ok(None)` inside the existing RED `pump_block`. The BLUE header authority is
  **unchanged** and still fail-closes every block that reaches it. BLUE canonical-type count **458 → 458**.
- **The skip is HASH-keyed and gated before the reducer.** A **different** block (different hash) at/before
  the last-applied slot is **not** short-circuited — it reaches the unchanged BLUE authority and fails
  closed (`SlotBeforeLastApplied` / `BlockNoOutOfOrder`, CE-F2). The skip requires `stored.slot ==
  decoded.slot` (slot-consistency) and never bypasses fork-choice (`DC-CONS-03` untouched). The gate
  `ci_check_receive_idempotency.sh` fences hash-keyed + before-reducer + slot-consistent (CE-F3).
- **Replay-equivalence preserved.** The no-op runs no reducer step and appends **nothing** to the WAL, so
  warm-start forward-replay is unaffected (`T-REC-05` / `DC-WAL-02` preserved — cross-referenced, not
  weakened, by `DC-NODE-16`). CE-F1 asserts chain-dep `last_slot` / WAL length / ChainDb tip unchanged by
  the no-op.
- **N-AE close grounding refresh is the span head (the baseline's docs commit).** `62811a4e` is
  **docs/config only** (no `.rs` / `.sh`); it regenerated the four grounding docs + `.idd-config.json` for
  the N-AE close at `a76672b9` and contributes the bulk of the window's file/line churn. The registry was
  already at 340 from the N-AE slice impls; `62811a4e` added **no** rule.
- **CODEMAP/SEAMS/TRACEABILITY pick up `DC-NODE-16` on next regen.** They were regenerated for the N-AE
  close at `a76672b9` (`62811a4e`) and remain accurate on module inventory (AE.F adds **no module, no
  type**). The registry records `DC-NODE-16` + its `ci_check_receive_idempotency.sh` binding authoritatively
  at HEAD (341 rules); the per-doc rows refresh on the next regen.

---

## Historical — PHASE4-N-AD durability proof + C2-LOCAL run + PHASE4-N-AE CE-A5 cluster (`25ddeebd → a76672b9`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It was a
> **multi-part lead** narrating the `25ddeebd → a76672b9` span: the PHASE4-N-AC grounding-doc-refresh tail
> (`25ddeebd`, the span-opening commit), the **test-only PHASE4-N-AD** tip-successor durability cluster, a
> **docs-only C2-LOCAL** preprod-tip / cardano-testnet venue guide-and-finding run, and the closing
> **CE-A5 cluster PHASE4-N-AE** — **Recover→Serve Continuity and Forge Admissibility**. Counts here are the
> figures **at `a76672b9`** (340 rules, 141 CI gates, 458 canonical types); the current window measures
> **forward** from `a76672b9`. The full §§0–7 narrative is recoverable from this doc's git history at
> `a76672b9` / `62811a4e`.

> Baseline: `25ddeebd` (grounding-doc refresh for PHASE4-N-AC close, 2026-06-06 11:48)
> HEAD: `a76672b9` (AE.E chain-sync server FindIntersect cursor — CE-A5 manifest achieved, 2026-06-07 12:26)
> Span: **the N-AC close-refresh tail + PHASE4-N-AD (test-only) + the C2-LOCAL guide/finding run + PHASE4-N-AE** — 19 commits, 24 files, +3635 / −129.

PHASE4-N-AE was the **CE-A5 cluster** that turned the recover→follow→forge→serve pipeline into a **proven
end-to-end live result**: a **real `cardano-node 11.0.1` relay `AddedToCurrentChain` an Ade-forged
successor block** (block 17 @ slot 421, hash `db3b5675…`, issuerHash `a1ed4e04… == blake2b-224(pool1` cold
VK`)`; relay forging = 0; Ade forge `succeeded = 1`) — the **CE-A5 manifest** (`docs/evidence/
phase4-n-ae-ce-a5-relay-adoption.{md,jsonl}`). It closed across **four impl slices** (committed A → C → B → E):

- **AE.A — forge-on-followed-tip admission gate (`5f2afc2a`; `DC-NODE-15` + `DC-CONS-24` enforced,
  `DC-NODE-14` partial).** Removed the recovered-tip forge-base fallback; a forge is admissible only when
  `durable_servable_tip == followed_peer_tip` (hash + `block_no`). GREEN `forge_followed_tip_admission`
  classifier; typed `ForgeRefused::NotCaughtUp`; followed-peer-tip is an admissibility input only (never
  reaches chain selection). Gate `ci_check_forge_followed_tip_admission.sh`.
- **AE.C — recover→follow WAL prior-fp seeding (`5425b23c`; `DC-WAL-02` + `T-REC-05` strengthened).** Seeds
  the follow `prior_fp = fingerprint(&state.ledger).combined` (was all-zero `Hash32`) at both lifecycle
  sites; fixed the `ChainBreak@1` warm-start break. Gate `ci_check_recover_follow_wal_lineage.sh`.
- **AE.B — recovered/forge-parent intersectability, Option B (`450c6992`; `DC-NODE-14` enforced; `CN-CONS-07`
  + `DC-CONS-23` strengthened).** `ChainDbServedSource::intersect` projects the forge parent's `prev_hash`
  as a **FindIntersect-only, proof-gated** point iff a real successor exists; never serves bytes for it.
  Additive BLUE `DecodedBlock.prev_hash` exposure. Gate `ci_check_recovered_anchor_intersectable.sh`.
- **AE.E — chain-sync SERVER FindIntersect cursor fix (`a76672b9`; `DC-PROTO-10` NEW enforced; `CN-CONS-06`
  + `DC-NODE-14` strengthened).** **The CE-A5 closer.** After `IntersectFound(point)` the producer
  chain-sync server sets its read cursor (`last_announced`) to that point, so the next `RequestNext` serves
  `next_after(point)`, not block 0 — making the real relay roll forward onto the forged successor.
  Regression-test enforced (no dedicated gate).

**N-AE-window headline (at `a76672b9`):** Registry **336 → 340** (+4 enforced `DC-CONS-24` / `DC-NODE-14` /
`DC-NODE-15` / `DC-PROTO-10`; 9 strengthenings across 8 rules; 0 removed). CI gates **138 → 141** (+3 gates;
+1 non-gate operator script `build_consensus_inputs_bundle.sh` modified). **BLUE-additive, +0 canonical
type** (458 → 458 — two BLUE files touched: `header_input` additive `DecodedBlock.prev_hash` field +
`chain_sync/server` additive FindIntersect-cursor logic). **PHASE4-N-AD** was a **test-only** durability
proof (one RED test, +214; `DC-WAL-04` + `T-REC-05` strengthened). The **C2-LOCAL run** was docs-only except
the venue-general `build_consensus_inputs_bundle.sh` change. **CE-A5 is recorded as `enforced`-backing
evidence, NOT a `RO-LIVE` flip.**

---

## Historical — PHASE4-N-AC close + cluster window (`c6e7fafe → 1d54abb4`)

> Preserved in condensed form. A **grounding-doc refresh + the PHASE4-N-AC cluster** (KES signing evolves
> the operator KES key to the current period before signing), narrating the `c6e7fafe → 1d54abb4` span.
> Counts here are the figures **at `1d54abb4`** (336 rules, 138 CI gates, 458 canonical types). The full
> §§0–7 narrative is recoverable from this doc's git history at `1d54abb4` / `25ddeebd`.

> Baseline: `c6e7fafe` (Close PHASE4-N-AB — outbound mux segmentation (CN-SESS-05), 2026-06-06 03:48)
> HEAD: `1d54abb4` (Close PHASE4-N-AC — KES signing evolves key to current period (DC-CRYPTO-10), 2026-06-06 11:08)
> Span: **a grounding-doc refresh + the PHASE4-N-AC cluster** — 5 commits, 12 files, +1029 / −340.

PHASE4-N-AC was a **RED-only live-readiness fix surfaced by the item-4 C1 re-run**: the forge's only real
KES sign required `kes.current_period() == kes_period`, and nothing evolved the minted-at-period-0 operator
key forward, so once the chain aged past one KES period the forge returned `KesPeriodNotCurrent` on every
leader slot. N-AC closed it in one slice:

- **S1 — evolve KES key to current period before signing (`7d4a4a72`; `DC-CRYPTO-10 → enforced`).** A new
  **RED** producer-shell method `ProducerShell::kes_sign_header_advancing(period, pre_image)` =
  `kes_advance_to(period)` then `kes_sign_header(period, pre_image)` — it evolves the operator KES key
  forward via the **existing deterministic `Sum6KES` update** (idempotent at the current period), then
  signs; **fails closed** `EvolutionBackwards` (before key start) / `EvolutionExhausted` (beyond
  `SUM6_MAX_PERIOD = 63`). The forge's single real KES sign is rewired to it (period passed verbatim).
  Signing stays RED. New gate `ci_check_kes_evolution_before_sign.sh`.

**N-AC headline (at `1d54abb4`):** Registry **335 → 336** (+1 enforced `DC-CRYPTO-10`; +1 strengthening
`CN-KES-HEADER-01`; 0 removed). CI gates **137 → 138** (+1 `ci_check_kes_evolution_before_sign.sh`).
**RED-only — BLUE canonical types 458 → 458.** The item-4 C1 re-run proved it live (Ade forged 3 period-1
blocks; the real cardano-node downloaded the period-1 header with no KES rejection). **No `RO-LIVE` flip.**
A genesis-window finding was recorded honestly (`slotsPerKESPeriod = 129600 == 3k/f`, so a from-genesis
rehearsal cannot show forge-at-period-1 **and** follower-adopt simultaneously — the period-1 follower
rejection is `CandidateTooSparse`, KES-independent).

---

## Historical — PHASE4-N-AB close + cluster window (`b0365df0 → c6e7fafe`)

> Preserved in condensed form. A **grounding-doc refresh + the PHASE4-N-AB cluster**, narrating the
> `b0365df0 → c6e7fafe` span. Counts here are the figures **at `c6e7fafe`** (335 rules, 137 CI gates, 458
> canonical types). The full §§0–7 narrative is recoverable from this doc's git history at `c6e7fafe`.

> Baseline: `b0365df0` (Close PHASE4-N-AA — bounded peer-driven serve range (DC-SERVEMEM-01), 2026-06-06 01:43)
> HEAD: `c6e7fafe` (Close PHASE4-N-AB — outbound mux segmentation (CN-SESS-05), 2026-06-06 03:48)
> Span: **a grounding-doc refresh + the PHASE4-N-AB cluster** — 5 commits, 10 files, +1130 / −406.

PHASE4-N-AB was **pre-RO-LIVE hardening item 2** and closed a **receive/send asymmetry**: Ade could
*receive* a block fragmented across multiple mux frames (CN-SESS-04 inbound reassembly) but could **not
transmit one** (`OutboundPayloadTooLarge` above `MAX_PAYLOAD = 65535`). N-AB closed that in one slice:

- **S1 — outbound mux segmentation (`02e6e557`; `CN-SESS-05 → enforced`).** The **GREEN** session
  reducer's `handle_outbound` now **segments** a payload in `MAX_PAYLOAD < len <= MAX_OUTBOUND_PAYLOAD_BYTES`
  into ordered `<= MAX_PAYLOAD` mux frames (each via the single `encode_inner_frame` authority) and **fails
  closed above** the new fixed `MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB`. New gate `ci_check_outbound_segmentation.sh`.

**N-AB headline (at `c6e7fafe`):** Registry **334 → 335** (+1 enforced `CN-SESS-05`; +2 strengthenings
`CN-SESS-04` + `DC-SERVEMEM-01`; 0 removed). CI gates **136 → 137** (+1 `ci_check_outbound_segmentation.sh`).
**GREEN-only — BLUE canonical types 458 → 458.** Outbound inverse of CN-SESS-04 inbound reassembly. **No
`RO-LIVE` flip.**

---

## Historical — PHASE4-N-AA close + cluster window (`999199f8 → b0365df0`)

> Preserved in condensed form. A **focused grounding refresh + the PHASE4-N-AA cluster**, narrating the
> `999199f8 → b0365df0` span. Counts here are the figures **at `b0365df0`** (334 rules, 136 CI gates, 458
> canonical types). The full §§0–7 narrative is recoverable from this doc's git history at `b0365df0`.

> Baseline: `999199f8` (repair 10 pre-existing gate-vs-code drifts (gate hygiene), 2026-06-05 19:28)
> HEAD: `b0365df0` (Close PHASE4-N-AA — bounded peer-driven serve range (DC-SERVEMEM-01), 2026-06-06 01:43)
> Span: **a focused grounding refresh + the PHASE4-N-AA cluster** — 8 commits, 15 files, +1254 / −492.

PHASE4-N-AA was **pre-RO-LIVE hardening item 1** and closed the **MEDIUM** the PHASE4-N-U cross-slice
review left open: *the `--mode node` serve path could be driven by a peer into unbounded memory + O(N²)
CPU.* N-AA closed it across two slices + an in-cluster security fix:

- **S1 — bounded hash-free ChainDb read primitives (`6b8f1779`; CE-1).** Two new bounded, slot-ordered,
  hash-free `ChainDb` primitives `range_bytes_capped` / `last_block_bytes`; new RED type `CappedSlotRange`.
- **S2 — serve projection cap + fail-closed (`3d853ec0`; `DC-SERVEMEM-01 → enforced`).** `ChainDbServedSource`
  switched onto the bounded primitives behind `MAX_SERVE_RANGE_BLOCKS = 256`; new RED enum `ServeRangeOutcome`.
  New gate `ci_check_serve_range_bounded.sh`.
- **In-cluster security-review MEDIUM (`5c9f6cf6`).** An inverted-range (`from > to`) panic fixed in-cluster.

**N-AA headline (at `b0365df0`):** Registry **333 → 334** (+1 enforced `DC-SERVEMEM-01`; +2 strengthenings
`DC-NODE-13` + `DC-LIVEMEM-01`; 0 removed). CI gates **135 → 136** (+1 `ci_check_serve_range_bounded.sh`).
**RED-only — BLUE canonical types 458 → 458.** Serve-side analog of `DC-LIVEMEM-01`. **No `RO-LIVE` flip.**

---

## Historical — earlier windows (`4e358e92 → 999199f8` and before)

> Preserved as pointers. The **PHASE4-N-U cluster CLOSE + gate-hygiene tail** (`4e358e92 → 999199f8`, 333
> rules / 135 CI gates at `999199f8` — 11 gates repaired in place, 0 added/removed, 0 invariants weakened);
> the **PHASE4-N-U cluster** (`65954fa3 → 4e358e92`, forged-block durability — `DC-NODE-12`, `DC-CONS-23`,
> `DC-WAL-04`, `T-REC-05`, `DC-NODE-13`; one new RED module `served_chain_projection`; 328 → 333 rules);
> and the **G-K…G-R + C1 multi-cluster catch-up** (`550eec3a → 65954fa3`, eight clusters G-K through G-R
> toward a live genesis-successor follower — 319 → 328 rules, 126 → 134 CI gates, the one BLUE canonical
> type `ArrayHead` 457 → 458). The full §§0–7 narrative for each is recoverable from this doc's git history
> at `999199f8` / `4e358e92` / `65954fa3`.

> *(The G-E…G-I and earlier leads were each closed with their own grounding-doc refresh and are recoverable
> from this doc's git history.)*

---

## Generation notes

### Regen `a76672b9 → 6363683e` (PHASE4-N-AE.F post-CE-A5 echo-idempotency follow-up — current lead)

- **Baseline valid; single-slice lead (N-AE close refresh span-head → AE.F).** Run against `a76672b9` (the
  PHASE4-N-AE.E CE-A5 closer, the prior HEAD_DELTAS HEAD), which `git rev-parse` resolves and
  `git merge-base a76672b9 HEAD` confirms is a strict ancestor of HEAD `6363683e` (`a76672b9` carries no
  tag). The start-of-regen config baseline was already `a76672b9` (the previous close bumped it). The
  closer bumps `head_deltas_baseline` `a76672b9 → 6363683e` after this regen.
- **Counts are mechanical (git/grep/ls):** commit log + `--shortstat` over `a76672b9..HEAD` (**4** commits,
  no merges / **13** files / **+1401 / −473** — dominated by `62811a4e`, the N-AE close grounding refresh
  at +947 / −473; the AE.F impl `6363683e` is +228 / −3 across 5 files, of which the production change is
  RED `pump.rs` +114 + RED test `node_sync.rs` +41); CI gate count via
  `git ls-tree -r --name-only <ref> ci/ | grep -c 'ci_check_.*\.sh$'` at each ref (**141 → 142**;
  `--diff-filter=A` over `ci/` = the one new gate `ci_check_receive_idempotency.sh`; `--diff-filter=D` and
  `--diff-filter=M` over `ci/` both **empty**); registry rule count via `grep -cE '^id = '` at each ref
  (**340 → 341**; `diff` of sorted `id =` lists shows the single addition `DC-NODE-16`, zero removals);
  registry status via `grep -E '^status = ' | sort | uniq -c` (**208 → 209 enforced**, 20 partial / 112
  declared unchanged); strengthenings = **0** (no `strengthened_in` append this span — `DC-NODE-16` is
  net-new); BLUE canonical types via a `git diff a76672b9..HEAD` over the BLUE `core_paths` trees (**empty
  diff → 458 → 458**, `^+(pub )?(struct\|enum)` over the BLUE trees = 0).
- **RED-only span — no BLUE file, +0 canonical type, no Cargo.toml change.** `git diff --name-status
  a76672b9..HEAD` shows the only production-code change is RED — `crates/ade_runtime/src/forward_sync/pump.rs`
  (the gate + CE-F1/CE-F2) and `crates/ade_node/src/node_sync.rs` (the CE-F4 live-shape test) — and
  `git diff a76672b9..HEAD` over the BLUE trees is **empty**. No new `.rs` *source* file (the three tests
  live in existing `#[cfg(test)]` modules). `git diff --name-only … '**/Cargo.toml' 'Cargo.toml'` is empty
  (no feature-flag delta). **Classification note:** `pump.rs` is **RED** (under `crates/ade_runtime/`, the
  shell crate) — the fix deliberately lives at the RED chokepoint because `get_block_by_hash` is a
  deterministic durable-store query (not nondeterminism), so the BLUE authority stays untouched and there
  is **no new BLUE reducer input** (a refinement of the AE.F invariants sketch's proposed BLUE
  `ReceiveOutcome::AlreadyHave`).
- **Registry delta is +1 enforced rule, NOT a strengthening or removal.** `DC-NODE-16` is declared +
  enforced in the AE.F impl `6363683e` (it cross-references `T-REC-05` / `DC-WAL-02` / `DC-CONS-03`, which
  are **preserved**, not strengthened). The sorted-id `diff` confirms zero removals. `DC-NODE-16` carries a
  populated `ci_script` (`ci/ci_check_receive_idempotency.sh`) + three `tests` (CE-F1 / CE-F2 / CE-F4).
- **Span head `62811a4e` is the N-AE close grounding refresh (the baseline's docs commit).** It is
  **docs/config only** (`git show --name-only 62811a4e` has no `.rs` / `.sh`); it regenerated CODEMAP/SEAMS/
  TRACEABILITY/HEAD_DELTAS + `.idd-config.json` for the N-AE close at `a76672b9` and contributes the bulk of
  the window's file/line churn. The registry was already at 340 from the N-AE slice impls; `62811a4e` added
  **no** rule. It is folded into this span head because it post-dates the recorded baseline `a76672b9`.
- **CE-A5 is a prior-window manifest, NOT re-claimed and NOT a `RO-LIVE` flip this span.** AE.F is the
  **follow-up** that closes the post-adoption echo (exit-43 after `AddedToCurrentChain`); the CE-A5 manifest
  itself was closed in AE.E (prior window) and backs `DC-NODE-14` / `DC-PROTO-10`. No `RO-LIVE` registry
  status changed this span.
- **Normative docs unchanged this span.** `git diff --name-only a76672b9..HEAD` over the configured
  `normative_docs` (CE-79 statement + addendum, the three contract docs, CE-73 reclassification, `CLAUDE.md`)
  is empty — the §7 delta is entirely the invariant-registry change.
- **§1 commit log verbatim from `git log --oneline --no-merges` (newest first).** The per-slice synthesis is
  in §0/§3. Note the AE.F invariants sketch `d11bdbe8` lands *before* the N-AE close refresh `62811a4e` in
  commit time (sketch written during the N-AE close pass).
- **Doc-refresh state — CODEMAP/SEAMS/TRACEABILITY current at `a76672b9`; this HEAD_DELTAS adds AE.F.** The
  three sibling docs were regenerated for the N-AE close at `a76672b9` (`62811a4e`) and remain accurate on
  module inventory (AE.F adds **no module, no type**). They pick up the `DC-NODE-16` row on their next
  regen; the **registry holds `DC-NODE-16` + its gate binding authoritatively at HEAD** (341 rules) in the
  interim.
- **Working tree clean.** This regen runs *after* all 4 span commits (the AE.F impl `6363683e` is HEAD for
  this window); `git status --short` shows only an untracked `.mithril-scratch/` (operator scratch,
  ignored). The remaining close-pass actions are this HEAD_DELTAS and the baseline bump
  `a76672b9 → 6363683e`.

### Regen `25ddeebd → a76672b9` (PHASE4-N-AD durability proof + C2-LOCAL guide/finding run + PHASE4-N-AE CE-A5 cluster — prior lead)

- **Multi-part lead** (N-AC refresh tail → N-AD test-only → C2-LOCAL docs → N-AE CE-A5), measured from
  `25ddeebd` (the N-AC grounding-refresh commit). **19** commits / **24** files / **+3635 / −129**; CI gates
  **138 → 141** (+3 gates; +1 non-gate operator script modified); registry **336 → 340** (+4 enforced
  `DC-CONS-24` / `DC-NODE-14` / `DC-NODE-15` / `DC-PROTO-10`; 9 strengthenings across 8 rules; 0 removed);
  BLUE canonical types **458 → 458** (BLUE-additive — two BLUE files touched, +0 type). The CE-A5 manifest
  (real `cardano-node 11.0.1` relay `AddedToCurrentChain` Ade's forged block 17) is recorded as
  `enforced`-backing evidence on `DC-NODE-14` / `DC-PROTO-10`, **not** a `RO-LIVE` flip. Full notes
  recoverable from this doc's git history at `a76672b9` / `62811a4e`.
