# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `b4c0983d` (PHASE4-N-AK AK-S2 — recovered-anchor rollback no-op completes live follow, DC-NODE-32, 2026-06-10 17:34)
> HEAD: `e87e8a43` (PHASE4-N-AL AL-S1 — participant recovered-anchor rollback no-op, DC-NODE-33, 2026-06-10 21:43)
> Span: **the PHASE4-N-AL cluster — the PARTICIPANT-path recovered-anchor rollback no-op, the participant MIRROR of N-AK's single-producer `DC-NODE-32`: `run_participant_sync` now accepts a peer `RollBackward` binding EXACTLY (slot AND hash) to the persisted recovered anchor (`state.recovered_anchor`, from `DC-NODE-31`) as an idempotent boundary no-op, evaluated BEFORE the unchanged `DC-NODE-29` stored-block resolution (`DC-NODE-33`)** — preceded by the **N-AK close commit** (`efa2a44e`, which flipped `DC-NODE-31`/`DC-NODE-32` `enforced_scaffolding → enforced` and regenerated all four grounding docs to `b4c0983d`) and a **C2-guide remediation note** (`c3ec7466`, records the N-AK recover→follow regression fix).
> **4 commits** (no merges), **14 files changed, +1792 / −825 lines**. **This span does NOT touch BLUE** — `git diff b4c0983d..HEAD` over the `core_paths` BLUE trees is **empty** (no `^+(pub )?(struct|enum)` line in any BLUE tree; **456 → 456** BLUE `pub struct`/`pub enum`, **462 → 462** by CODEMAP's whole-tree metric). **NO new crate** (`git diff --name-only … '**/Cargo.toml'` empty — still **11 crates**), **NO new module** (`git diff --diff-filter=A --name-only … 'crates/**/*.rs'` empty), **NO new canonical type**, and **NO new CI gate** (`--diff-filter=A/M/D` over `ci/` all **empty**; `ls ci/ci_check_*.sh | wc -l` = **159** at both refs). **Registry 358 → 359** (+1 rule `DC-NODE-33`, enforced at close; `T-REC-05` **NOT** re-strengthened — **zero strengthenings** this window; zero removals). The entire production change is **17 added lines** of RED follow-loop code in `crates/ade_node/src/node_lifecycle.rs` (`run_participant_sync` `RollBack` handler) plus **5** hermetic `participant_*` tests in `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs` (+194). The remaining +1581 / −825 is the in-span **N-AK close** (`efa2a44e`) regenerating the four grounding docs + archiving the N-AK cluster docs + the N-AL cluster/slice/invariants docs (`f8275c55`).

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`b4c0983d`**, the
> PHASE4-N-AK AK-S2 close (the prior HEAD_DELTAS HEAD), and it is **valid**: `git rev-parse b4c0983d` resolves and
> `git merge-base b4c0983d HEAD == b4c0983d` (it is a strict ancestor of HEAD; `b4c0983d` carries no tag). HEAD is
> **`e87e8a43`** (the PHASE4-N-AL AL-S1 impl — the participant-path recovered-anchor rollback no-op). At the start
> of this regen the **working-tree** config baseline was already `b4c0983d` (the previous close's bump
> `b1bed361 → b4c0983d` is itself an uncommitted working-tree step from the N-AK close-pass; the **committed**
> `.idd-config.json` at HEAD still reads `b1bed361`), so the window measures cleanly from the recorded working-tree
> baseline forward. The span has **three parts**: (1) the **PHASE4-N-AK close commit** — `efa2a44e`
> (`Close PHASE4-N-AK …`), which flipped `DC-NODE-31` + `DC-NODE-32` `enforced_scaffolding → enforced`, regenerated
> CODEMAP/SEAMS/TRACEABILITY/HEAD_DELTAS to `b4c0983d`, and archived the N-AK cluster docs — **docs/registry only,
> 0 code, 0 net new rule** (registry stayed 358); (2) a **C2-guide remediation note** — `c3ec7466`
> (`docs(c2-guide): record PHASE4-N-AK recover→follow remediation`), docs-only, records the N-AK fix history, **not
> N-AL work**; and (3) the **PHASE4-N-AL cluster** (`DC-NODE-33`) — cluster authority doc + invariants sketch +
> AL-S1 slice doc declaring `DC-NODE-33` (`f8275c55`) + the AL-S1 impl (`e87e8a43`). **The cluster-close registry
> flip + grounding-doc/config touch + archive are an uncommitted working-tree close-pass at this regen** (see the
> working-tree note below).
>
> **Working-tree note (load-bearing).** At the time of this regen there are **UNCOMMITTED working-tree changes** —
> the N-AL close artifacts (the `DC-NODE-33 declared → enforced` flip + its `tests` array populated in the registry,
> the AL-S1 slice-doc `Merged` flip, the c2-guide/runbook sync, the CODEMAP HEAD-pin touch, the config baseline bump,
> and this HEAD_DELTAS refresh). **§1 narrates the COMMITTED span `b4c0983d..e87e8a43` verbatim from `git log`.** The
> rule **STATUS** in §0/§7 is read from the **CURRENT working-tree** `docs/ade-invariant-registry.toml` so the prose
> reflects the close state (`DC-NODE-33` **enforced**, **359** rules). The operator bumps `head_deltas_baseline`
> `b4c0983d → e87e8a43` as the **post-close step this regen performs** (the prior N-AK bump `b1bed361 → b4c0983d` was
> left uncommitted in the working tree; this regen advances it to `e87e8a43`).

This window is **led by PHASE4-N-AL — the participant-path recovered-anchor rollback boundary.** It is the
**participant MIRROR** of N-AK's `DC-NODE-32` (which fixed the single-producer `run_node_sync` path): after recovery
from a **bare bootstrap anchor** (a recovery snapshot captured at the anchor slot with **no servable post-anchor
block** durable in the `ChainDb`), the relay's standard post-`IntersectFound` `RollBackward(anchor)` would, on the
PARTICIPANT path (`run_participant_sync`), fall through to the `DC-NODE-29` stored-block resolution — where
`get_block_by_hash(anchor)` returns `None` (the anchor is a snapshot boundary, NOT a stored servable block) and the
follow fails closed BEFORE the first forward admit. N-AL closes that participant-side gap with a **single, narrow,
fail-closed RED follow-loop boundary**:

> **PHASE4-N-AL accepts, on the participant live-follow path (`run_participant_sync`), a peer `RollBackward` whose
> target binds EXACTLY (slot AND hash) to the persisted recovered anchor point (`DC-NODE-31` / `BootstrapState.tip`,
> carried in `ForwardSyncState.recovered_anchor`) as an IDEMPOTENT NO-OP boundary rewind — no `commit_rollback`, no
> `WalEntry::RollBack`, no `ChainDb`/ledger/`chain_dep` mutation, no cursor, no `pending_reselection` — evaluated
> BEFORE the unchanged `DC-NODE-29` `get_block_by_hash` stored-block resolution (`DC-NODE-33`). The anchor is a
> recovery snapshot BOUNDARY, **never** synthesized into a servable block (`ChainDb::tip()` / serve never return it);
> `RollBackward(Origin)` still FAILS CLOSED (AI-S4a unchanged); **every** non-anchor, non-Origin rollback still
> resolves through the EXISTING `DC-NODE-29` authority UNCHANGED (a real stored-block rollback still routes through
> `apply_chain_event`, slot-only / hash-only near-misses still fail closed); the accepted point binds to the
> PERSISTED anchor on slot AND hash, never peer-supplied alone. The anchor consumed by `run_participant_sync` is the
> single authority (`state.recovered_anchor`, set once in the forge-ON arm at `node_lifecycle.rs:563` and threaded
> via `run_relay_loop_with_sched`) — **NEVER re-read from the store inside the loop**. The first forward block after
> the anchor no-op admits through the EXISTING sole `pump_block` path — AL adds **NO** forward-link code. Recover→
> follow on the participant path is replay-equivalent (extends `T-REC-05` / `DC-NODE-31` / `DC-NODE-32` to the
> participant follow). `DC-NODE-32` stays scoped to `run_node_sync` (**NOT** broadened — a distinct sibling rule).
> NO BLUE change; NO new module; NO new canonical type; NO new CI gate; NO `RO-LIVE` flip.**

The cluster is a **single slice** — the participant-path boundary mirrors the AK-S2 single-producer boundary,
reusing the EXISTING `ForwardSyncState.recovered_anchor` field (the AK-S2 carrier) unchanged:

- **PHASE4-N-AL / AL-S1 / `DC-NODE-33` (enforced) (participant recovered-anchor rollback no-op — RED follow-loop
  no-op + fail-closed fence, before `DC-NODE-29`).** `crates/ade_node/src/node_lifecycle.rs`'s `run_participant_sync`
  `RollBack` handler gains a **17-line** anchor branch immediately AFTER the existing `RollBackward(Origin)`
  fail-close (AI-S4a, `wire_pump.rs:447`, unchanged) and immediately BEFORE the `DC-NODE-29` `get_block_by_hash`
  durable-membership resolution: `if let Some(anchor) = &state.recovered_anchor { if slot == anchor.slot && hash ==
  anchor.hash { continue; } }`. A `RollBackward` binding EXACTLY (slot AND hash) to the persisted recovered anchor is
  an **idempotent NO-OP** (`continue` — the node is already at the anchor, a recovery snapshot boundary that
  `get_block_by_hash` would otherwise resolve to `None`, failing the follow closed before the first forward admit);
  **every other point still flows through the UNCHANGED `DC-NODE-29` authority** (a real stored-block rollback
  resolves the wire hash against the durable `ChainDb`, requires the peer-supplied slot to equal the stored slot,
  and either `apply_chain_event`s or fails closed). No new field, no new type — the branch reads the EXISTING
  `ForwardSyncState.recovered_anchor` (AK-S2's carrier, `crates/ade_runtime/src/forward_sync/reducer.rs`), which the
  forge-ON arm already sets to `state.tip.clone()` (`BootstrapState.tip`, `node_lifecycle.rs:563`) and threads in via
  `run_relay_loop_with_sched`. The anchor is **NEVER re-read from the store inside the loop**; the first forward block
  after the no-op admits through the EXISTING sole `pump_block` (its `prev_hash` binds the recovered `chain_dep`). The
  test file `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs` gains **5** hermetic `participant_*` CEs
  (`participant_rollback_to_recovered_anchor_is_noop` — CE-AL-1; `participant_rollback_origin_fails_closed` — Origin
  fails closed even with a recovered anchor present, AI-S4a; `participant_rollback_non_anchor_fails_closed` — a
  non-anchor rollback fails closed, slot AND hash bound; `participant_first_forward_after_anchor_noop_admits_via_pump_block`
  — the forward block after the no-op reaches `pump_block` and admits; `participant_stored_block_rollback_still_applies`
  — a real durable stored-block rollback still routes through the UNCHANGED `DC-NODE-29` `apply_chain_event`). **`DC-NODE-33
  → enforced`** at close (5 named tests; **no dedicated CI gate** — `ci_script = ""`, enforced by the unit/integration
  suite, matching the `DC-NODE-31` / `DC-NODE-32` test-enforced precedent). **Honesty:** SCOPE is the **participant
  `run_participant_sync` recovered-anchor rollback-to-intersection ONLY** — it does NOT add general multi-candidate
  fork-choice, does NOT change the N-AJ evidence emission (`DC-NODE-30`), does NOT flip `CN-CONS-03`, and does NOT
  broaden `DC-NODE-32` (which stays scoped to `run_node_sync`).

**NO BLUE change this span** — `git diff b4c0983d..HEAD` over the BLUE `core_paths` trees is empty; **0** new canonical
type (456 → 456 BLUE / 462 → 462 whole-tree). **No `RO-LIVE` rule flipped** — `RO-LIVE-01` stays operator-gated. The
live **CE-AL-3-LIVE** end-to-end pass IS recorded as `enforced`-backing evidence for `DC-NODE-33` (2026-06-10, on a
**FRESH 2-pool `cardano-testnet` venue**, magic 42: fresh **bare-anchor recover @ slot 741** → peer
`RollBackward(741)` idempotent no-op → first forward block **admitted @ slot 777** → converged to
`agreement_verdict{agreed}` **@ slot 801** with `our_hash == peer_hash` (exact match), **0 `UnexpectedRollback` + 0
`UnsupportedRollbackPoint` + 0 diverged**; the live transcript is **OUTSIDE-REPO**, scrubbed in-repo note only). It is
**NOT** preprod, **NOT** bounty completion, and **does NOT prove CE-AI-6 reorg convergence, full ChainSel, or natural
reorg capture** (CE-AI-6 is a SEPARATE induced-reorg operator pass).

## 0. Headline

| Count | Baseline (`b4c0983d`, committed) | HEAD (`e87e8a43` + close working-tree) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 159 | **159** | **±0** — **no gate added, modified, or removed** (`--diff-filter=A` / `--diff-filter=M` / `--diff-filter=D` over `ci/` all **empty**; `ls ci/ci_check_*.sh \| wc -l` = 159 at both refs). `DC-NODE-33` carries **`ci_script = ""`** — it is test-enforced (5 named tests), matching the `DC-NODE-31` / `DC-NODE-32` / `DC-PROTO-10` precedent. |
| Registry rules (`docs/ade-invariant-registry.toml`) | 358 | **359** | **+1** — one NEW rule `DC-NODE-33`. **Zero removed** (`diff` of the sorted `id =` lists shows exactly the single addition `DC-NODE-33` and no removal). |
| Registry status (enforced / enforced_scaffolding / partial / declared) | 222 / 3 / 19 / 114 | **225 / 1 / 19 / 114** | **+3 enforced**, **−2 enforced_scaffolding** (`partial=19` and `declared=114` net unchanged). The reconciliation: the **in-span N-AK close commit `efa2a44e`** flipped `DC-NODE-31` + `DC-NODE-32` `enforced_scaffolding → enforced` (+2 enforced, −2 enforced_scaffolding — at the COMMITTED baseline `b4c0983d` both were still `enforced_scaffolding`), and the **N-AL working-tree close** flips the NEW `DC-NODE-33` `declared → enforced` (+1 enforced) — its declaration by `f8275c55` had moved `declared` 114→115, and the close flip moves it back to 114. Net: +3 enforced, −2 enforced_scaffolding, declared/partial unchanged. |
| Registry strengthenings | — | **0** | **No `strengthened_in += "PHASE4-N-AL"` on any rule.** `DC-NODE-33` *cross-refs* `DC-NODE-32` / `DC-NODE-31` / `DC-NODE-29` / `DC-NODE-23` / `T-REC-05` / `CN-CONS-03` in its statement (its replay-equivalence "extends `T-REC-05`/`DC-NODE-31`/`DC-NODE-32` to the participant follow"), but it does **not** append `PHASE4-N-AL` to any existing rule's `strengthened_in` (`grep 'strengthened_in.*PHASE4-N-AL'` = 0 matches). No rule weakened; no rule removed. |
| BLUE canonical types | 456 | **456** | **±0** — `git diff b4c0983d..HEAD` over the BLUE `core_paths` trees is **empty** (no `^+(pub )?(struct\|enum)` line in any BLUE tree). By CODEMAP's whole-tree metric: **462 → 462**. No `Cargo.toml` changed — still 11 crates. The only production change is **17 RED lines** in `node_lifecycle.rs` (a follow-loop branch reading the EXISTING `ForwardSyncState.recovered_anchor` field) — no new field, no new type. |
| Grounding docs | CODEMAP / SEAMS / TRACEABILITY all regenerated to **`b4c0983d`** by the in-span N-AK close `efa2a44e` (462 canonical types / 159 CI / 358 rules; they carry `DC-NODE-31` / `DC-NODE-32` / `RecoveredAnchorPoint` + the N-AJ `convergence_evidence` + N-AK `recovered_anchor` modules — the prior two-cluster doc-refresh debt was PAID at the N-AK close) | Still pinned at **`b4c0983d`** — now **one cluster stale**: none yet carries `DC-NODE-33` (`grep -c DC-NODE-33` in each = 0). N-AL adds **NO new module and NO new type**, so CODEMAP's module/type inventory (462 types / 11 crates) stays accurate; only **TRACEABILITY** owes the new `DC-NODE-33` four-cell row, and SEAMS/CODEMAP owe at most a HEAD-pin/count refresh. | **CODEMAP + SEAMS + TRACEABILITY are now ONE cluster STALE** (missing `DC-NODE-33` only) — the registry holds `DC-NODE-33` + its bindings authoritatively at HEAD (**359 rules**); the refresh to `e87e8a43` is a follow-on item this close. See the cross-reference warning at the end of §5. |

> **Grounding-doc state this close (load-bearing).** **CODEMAP, SEAMS, and TRACEABILITY were all regenerated to
> `b4c0983d` at the N-AK close `efa2a44e`** (the in-span first commit), which paid the two-cluster debt the prior
> HEAD_DELTAS recorded — they now carry `DC-NODE-31`, `DC-NODE-32`, `RecoveredAnchorPoint`, the N-AJ
> `convergence_evidence` module, and the N-AK `recovered_anchor` module. They are now **one cluster stale** (N-AL):
> `grep -c DC-NODE-33` in all three is **0**. Because N-AL introduces **no new module and no new canonical type**,
> CODEMAP's structural inventory (462 types / 11 crates) is unaffected; the only owed refresh is the `DC-NODE-33`
> four-cell row in TRACEABILITY (plus a HEAD-pin/count bump to `e87e8a43` / 359 rules across all three). The invariant
> registry holds `DC-NODE-33` + its `tests` binding authoritatively at HEAD (**359 rules**); the
> CODEMAP + SEAMS + TRACEABILITY refresh to `e87e8a43` is the follow-on item this close (surfaced in §5).

The slice↔rule↔gate map for this window:

| Slice | Rule(s) | Gate | What shipped |
|---|---|---|---|
| **N-AK close** (`efa2a44e`) | flip `DC-NODE-31 → enforced`; `DC-NODE-32 → enforced` (both `enforced_scaffolding → enforced`); CODEMAP/SEAMS/TRACEABILITY/HEAD_DELTAS regenerated to `b4c0983d` | — (no new gate) | **docs/registry only — 0 code.** Closed the N-AK cluster: flipped the two recovered-anchor rules to `enforced`, regenerated all four grounding docs to `b4c0983d` (paying the two-cluster refresh debt), and archived the N-AK cluster/slice docs to `docs/clusters/completed/PHASE4-N-AK/`. Folded into this span because it sits inside `b4c0983d..HEAD`; it is **not** N-AL work. |
| **c2-guide note** (`c3ec7466`) | — | — | **docs-only — 0 code / 0 rule.** Records the PHASE4-N-AK recover→follow regression remediation history in `docs/active/c2-preprod-tip-guide.md` (the CE-AI-6 pass blocker + the AK-S1/AK-S2 fix + CE-AK-3 live result). **Not N-AL work.** |
| **cluster doc + AL-S1 doc** (`f8275c55`) | `DC-NODE-33` **declared** | — | N-AL cluster authority doc + invariants sketch + AL-S1 slice doc; declares `DC-NODE-33`. 0 code. |
| **AL-S1** (`e87e8a43`) | **`DC-NODE-33`** (NEW, → enforced at close) | (none — `ci_script=""`; test-enforced) | **RED follow-loop no-op + fail-closed fence.** `run_participant_sync` `RollBack` handler in `node_lifecycle.rs` (+17) gains the recovered-anchor exact-(slot AND hash) idempotent no-op, evaluated BEFORE the unchanged `DC-NODE-29` `get_block_by_hash` resolution; reads the EXISTING `ForwardSyncState.recovered_anchor`. +5 hermetic `participant_*` tests in `live_fork_choice_ai_s4bii.rs`. **Last slice — the cluster-close flip/archive/baseline-bump are the in-progress working-tree close.** |

The per-commit shape (the full verbatim log is §1):

| Commit | Kind | What it did | Code / CI / registry effect |
|--------|------|-------------|-----------------------------|
| `efa2a44e` | (close) | Close PHASE4-N-AK — recovered anchor is the live recover→follow authority (DC-NODE-31 + DC-NODE-32 enforced) | **0 code / 0 CI**; docs/registry: flipped `DC-NODE-31` + `DC-NODE-32` `enforced_scaffolding → enforced`, regenerated CODEMAP/SEAMS/TRACEABILITY/HEAD_DELTAS to `b4c0983d`, archived N-AK cluster docs. Registry count unchanged (358). **N-AK, not N-AL** |
| `c3ec7466` | docs (c2-guide) | Record PHASE4-N-AK recover→follow remediation | **0 code / 0 CI / 0 rule**; `docs/active/c2-preprod-tip-guide.md` only. **Not N-AL work** |
| `f8275c55` | docs (phase4-n-al) | Participant recovered-anchor boundary authority; declare `DC-NODE-33` | **0 code / 0 CI**; registry: `DC-NODE-33` declared; + N-AL cluster/slice/invariants docs |
| `e87e8a43` | feat (phase4-n-al) | AL-S1 — participant recovered-anchor rollback no-op (`DC-NODE-33`) | **RED code** (`node_lifecycle.rs` `run_participant_sync` rollback no-op, +17, reading the EXISTING `recovered_anchor` field) + 5 tests (`live_fork_choice_ai_s4bii.rs`, +194); **+0 BLUE type**; **+0 module**; **+0 CI**; registry: `DC-NODE-33` → enforced at close |

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `e87e8a43` | feat | feat(phase4-n-al): AL-S1 -- participant recovered-anchor rollback no-op (DC-NODE-33) |
| `f8275c55` | docs | docs(phase4-n-al): participant recovered-anchor boundary authority; declare DC-NODE-33 |
| `c3ec7466` | docs | docs(c2-guide): record PHASE4-N-AK recover->follow remediation |
| `efa2a44e` | (close) | Close PHASE4-N-AK — recovered anchor is the live recover->follow authority (DC-NODE-31 + DC-NODE-32 enforced) |

No merge commits in the span. **4 commits, zero unclassified.** Three subjects carry an explicit
conventional-commits prefix (`feat(...)` / `docs(...)`); the fourth (`efa2a44e`, `Close PHASE4-N-AK …`) is the
prior-window **close commit** (no prefix — the project's close-commit convention), folded into this span because it
sits inside `b4c0983d..HEAD`. The **only** production code lands in the single `feat(...)` commit (`e87e8a43` AL-S1);
the two `docs(...)` commits are the N-AL cluster/slice docs (`f8275c55`) and a C2-guide remediation note for the
PRIOR cluster (`c3ec7466`). **`efa2a44e` is N-AK close work, not PHASE4-N-AL** (docs/registry only — 0 code), and
**`c3ec7466` documents N-AK, not N-AL.** All commits landed 2026-06-10.

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty
> trailer requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that
> is an Ade-local override of the global no-AI-attribution rule and applies to **commit messages
> only**. It does not affect this doc's content.

## 2. New Modules

**No new modules this window.** `git diff --diff-filter=A --name-only b4c0983d..HEAD -- 'crates/**/*.rs'` is
**empty** — N-AL adds no new library module and no new test file (the 5 new tests are added to the EXISTING
`crates/ade_node/tests/live_fork_choice_ai_s4bii.rs`). There is **no new crate, no new `Cargo.toml`, no new
workspace** (`git diff --name-only … '**/Cargo.toml'` is empty; still **11 crates**). The N-AL change is confined to
**one existing source file** (`node_lifecycle.rs`) plus the one existing test file (§3).

> **Cross-reference (CODEMAP) — no new module to register.** Because N-AL introduces no module and no canonical
> type, CODEMAP's module inventory (11 crates / 462 canonical types, regenerated to `b4c0983d` at the N-AK close
> `efa2a44e`) remains structurally accurate at HEAD. The single new rule `DC-NODE-33` attaches to the EXISTING RED
> `ade_node::node_lifecycle` module (already in CODEMAP §RED) — only its rule↔enforcement binding (TRACEABILITY)
> needs the refresh (§5).

## 3. Modules Modified

Beyond the (zero) new modules (§2), **one existing source file and one existing test file** changed for the N-AL
production work — both in `ade_node`. (The remaining span churn is the in-span N-AK close `efa2a44e` regenerating
the four grounding docs + archiving cluster docs, and `f8275c55`/`c3ec7466` adding N-AL/c2-guide docs — docs only.)
The substantive change is a single, narrow follow-loop branch:

| Module | Color / scope | Key changes |
|--------|---------------|-------------|
| `ade_node::node_lifecycle` (`node_lifecycle.rs` +17) | **RED** participant follow loop, additive | **AL-S1 (`e87e8a43`):** the `run_participant_sync` `RollBack` handler gains a **17-line** recovered-anchor branch immediately AFTER the existing `RollBackward(Origin)` fail-close (AI-S4a, unchanged) and immediately BEFORE the `DC-NODE-29` `get_block_by_hash` stored-block resolution: `if let Some(anchor) = &state.recovered_anchor { if slot == anchor.slot && hash == anchor.hash { continue; } }` — a `RollBackward` binding EXACTLY (slot AND hash) to the persisted recovered anchor is an idempotent NO-OP (`continue` — no `commit_rollback` / `WalEntry::RollBack` / `ChainDb`/ledger/`chain_dep` mutation / cursor / `pending_reselection`); every other point flows through the UNCHANGED `DC-NODE-29` authority (`get_block_by_hash` + stored slot/hash binding → `apply_chain_event` or fail closed). Reads the EXISTING `ForwardSyncState.recovered_anchor` field (AK-S2's carrier), set once in the forge-ON arm at `:563` and threaded via `run_relay_loop_with_sched`; **never re-read from the store inside the loop**. **No new field, no new type. `DC-NODE-32` NOT broadened** (it stays scoped to `run_node_sync`; this is a distinct sibling on the participant path). |
| `ade_node::tests::live_fork_choice_ai_s4bii` (`live_fork_choice_ai_s4bii.rs` +194) | **test**, additive | **AL-S1 (`e87e8a43`):** **5** new hermetic `participant_*` CEs — `participant_rollback_to_recovered_anchor_is_noop`, `participant_rollback_origin_fails_closed`, `participant_rollback_non_anchor_fails_closed`, `participant_first_forward_after_anchor_noop_admits_via_pump_block`, `participant_stored_block_rollback_still_applies` (the registry-named enforcement for `DC-NODE-33`). The 8 pre-existing `participant_*` tests (from N-AI/N-AJ) are unchanged. |

> **No BLUE change this span (load-bearing).** Like the N-AJ window (and unlike N-AI / N-AK), this span is
> **BLUE-empty**: `git diff b4c0983d..HEAD` over the BLUE `core_paths` trees is empty (no file touched, no
> `^+(pub )?(struct\|enum)` line) — BLUE count **456 → 456** (462 → 462 whole-tree). The single production change is
> **17 RED lines** in the `ade_node` shell, reading an EXISTING field; everything BLUE-authoritative
> (`pump_block`, the `DC-NODE-29` durable-membership resolution, `ChainDb::tip()`, the `RecoveredAnchorPoint` codec)
> is untouched.

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace `Cargo.toml`, and **no
`Cargo.toml` changed in this window** (`git diff --name-only b4c0983d..HEAD -- '**/Cargo.toml' 'Cargo.toml'` is
empty). No `#[cfg(feature = …)]` gate was introduced and no `compile_error!` coupling was added. **No new CLI flag
this span either** — N-AL adds no new struct field and no new flag: the new behavior reads the EXISTING
`ForwardSyncState.recovered_anchor` field (AK-S2's carrier, populated by the recover path's forge-ON arm), so a
participant follow over a store with no recovered anchor (`recovered_anchor == None`) reproduces pre-AL behavior
verbatim. The durable restart authority remains the **persisted anchor-point record** (`DC-NODE-31`), explicitly NOT
CLI re-supply.

## 5. CI Checks (159 → 159; no gate added, modified, or removed)

**Zero CI-script changes this span.** `git diff --diff-filter=A b4c0983d..HEAD -- ci/`,
`--diff-filter=M`, and `--diff-filter=D` over `ci/` are **all empty** — no gate was added, modified in place, or
removed; `ls ci/ci_check_*.sh | wc -l` = **159** at both refs. The new rule carries **`ci_script = ""`**: it is
enforced by the **unit/integration test suite** (the 5 `participant_*` tests named below), matching the
`DC-NODE-31` / `DC-NODE-32` / `DC-PROTO-10` / `T-REC-05` test-enforced precedent (a recovered-anchor follow-loop
boundary is a behavioral property of the `run_participant_sync` reducer path, exercised directly by the hermetic CEs
rather than by a textual gate).

### PHASE4-N-AL enforcement (AL-S1) — test-suite-backed, no new gate

| Rule | Enforced by | What it checks |
|------|-------------|----------------|
| `DC-NODE-33` | `ci_script=""`; 5 named tests | `RollBackward(anchor)` (exact slot AND hash) is an idempotent no-op on the participant path (`participant_rollback_to_recovered_anchor_is_noop`); `RollBackward(Origin)` fails closed even with a recovered anchor present (`participant_rollback_origin_fails_closed`, AI-S4a); a non-anchor rollback fails closed, slot AND hash bound (`participant_rollback_non_anchor_fails_closed`); the forward block after the no-op reaches `pump_block` and admits (`participant_first_forward_after_anchor_noop_admits_via_pump_block`); a real durable stored-block rollback still routes through the UNCHANGED `DC-NODE-29` `apply_chain_event` (`participant_stored_block_rollback_still_applies`). |

> **Cross-reference (CODEMAP + SEAMS + TRACEABILITY) — ONE cluster stale this close; refresh owed.** The new
> rule↔enforcement binding (`DC-NODE-33` ↔ its 5 tests) is recorded **in the registry at HEAD**
> (`docs/ade-invariant-registry.toml`, 359 rules). It is **NOT yet in TRACEABILITY, SEAMS, or CODEMAP**, all three of
> which were regenerated to the N-AK close `b4c0983d` (`grep -c DC-NODE-33` in each = 0). **No gate is orphaned** (no
> gate was added). **TRACEABILITY note:** because `DC-NODE-33` carries `ci_script=""`, TRACEABILITY's enforcement
> column for it is the **test suite**, not a `ci_check_*` script — intentional (the participant-path no-op is a
> reducer-path behavioral property). **No new module / no new type**, so CODEMAP's structural inventory needs no
> change — only the HEAD-pin/count bump. **Action:** regenerate CODEMAP + SEAMS + TRACEABILITY to `e87e8a43` as a
> follow-on this close so `DC-NODE-33` appears in TRACEABILITY with its named enforcement and all three docs pin to
> the N-AL HEAD; until then the registry is authoritative for the new binding.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. **This window added ZERO BLUE
canonical types:** the BLUE `pub struct`/`pub enum` count over the `core_paths` trees is **`456 → 456`** (462 → 462
by CODEMAP's whole-tree metric) — `git diff b4c0983d..HEAD` over the BLUE trees is **empty** (no file touched, no
`^+(pub )?(struct|enum)` line). **Zero BLUE canonical types added; zero removed.** No `Cargo.toml` changed (still 11
crates). N-AL introduces **no new type at any tier** — the production change is a 17-line RED follow-loop branch
reading the EXISTING `ForwardSyncState.recovered_anchor` field.

## 7. Normative / Invariant Rule Delta (358 → 359; +1 rule, 0 strengthenings, zero removals)

**One rule ID was added; zero removed** (`358 → 359`; `diff` of the sorted `id =` lists shows exactly the single
addition `DC-NODE-33` and no removal). The status tally moves **222 → 225 enforced** and **3 → 1
enforced_scaffolding** (the `partial = 19` and `declared = 114` net unchanged). The +3-enforced / −2-enforced_scaffolding
reconciles as: the **in-span N-AK close commit `efa2a44e`** (the first commit in this span) flipped `DC-NODE-31` +
`DC-NODE-32` `enforced_scaffolding → enforced` (+2 enforced, −2 enforced_scaffolding — at the committed baseline
`b4c0983d` both were still `enforced_scaffolding`), **and** the N-AL working-tree close flips the NEW `DC-NODE-33`
`declared → enforced` (+1 enforced; its declaration by `f8275c55` had transiently moved `declared` 114→115, the close
flip returns it to 114).

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs, the
CE-73 reclassification, and `CLAUDE.md` — were **not** changed this span: `git diff --name-only b4c0983d..HEAD`
over those paths is empty. The rule-count delta is entirely the invariant-registry change.)*

**New rule (`+1`, `introduced_in = "PHASE4-N-AL"`, enforced):**

| Rule | Family / Tier · Status | Statement (summary) |
|------|------------------------|---------------------|
| `DC-NODE-33` | DC / `derived` · **enforced** | **Participant-path recovered-anchor rollback boundary (the participant MIRROR of `DC-NODE-32`).** On the participant live-follow path (`run_participant_sync`), a peer `RollBackward` whose target binds EXACTLY (slot AND hash) to the persisted recovered anchor point (`DC-NODE-31` / `BootstrapState.tip`, carried in `ForwardSyncState.recovered_anchor`) is accepted as an IDEMPOTENT NO-OP boundary rewind: no `commit_rollback`, no `WalEntry::RollBack`, no `ChainDb`/ledger/`chain_dep` mutation, no cursor, no `pending_reselection`. The anchor branch is evaluated **BEFORE** the existing `DC-NODE-29` stored-block resolution. Recover→follow on the participant path is replay-equivalent (extends `T-REC-05` / `DC-NODE-31` / `DC-NODE-32` to the participant follow). **MUST NOT:** the anchor is a recovery snapshot boundary, **NOT** a stored servable block, and is **NEVER** synthesized into one (`ChainDb::tip()`/serve never return it); `RollBackward(Origin)` still fails closed (AI-S4a unchanged); every non-anchor, non-Origin rollback still resolves through the EXISTING `DC-NODE-29` authority UNCHANGED; the accepted point binds to the PERSISTED anchor on slot AND hash, **never peer-supplied alone**; the anchor consumed by the loop is the single authority (`state.recovered_anchor`, set in the forge-ON arm at `node_lifecycle.rs:563`), threaded in — **NEVER re-read from the store inside the loop**; the first forward block after the anchor no-op admits through the EXISTING sole `pump_block` (AL adds **no** forward-link code); `DC-NODE-32` stays scoped to `run_node_sync` (**NOT broadened**). **SCOPE:** the recovered-anchor rollback-to-intersection case ONLY; does NOT add general multi-candidate fork-choice, does NOT change N-AJ evidence emission (`DC-NODE-30`), does NOT flip `CN-CONS-03`. |

**Strengthenings (`strengthened_in += "PHASE4-N-AL"`) — 0:** no existing rule's `strengthened_in` gained
`PHASE4-N-AL` (`grep 'strengthened_in.*PHASE4-N-AL'` = 0). `DC-NODE-33` *cross-refs* `T-REC-05` / `DC-NODE-31` /
`DC-NODE-32` / `DC-NODE-29` / `DC-NODE-23` / `CN-CONS-03` in its statement, but those are `cross_ref` pointers, not
`strengthened_in` appends. **No rule was weakened.**

**No rule was removed (expected: 0).** The registry delta is **one new rule (`DC-NODE-33`, enforced), zero
strengthenings, zero removals** — consistent with append-only registry discipline. **No anomaly.** (The
+3-enforced / −2-enforced_scaffolding tally includes the `DC-NODE-31` / `DC-NODE-32` `enforced_scaffolding → enforced`
flips carried by the in-span N-AK close commit `efa2a44e`, accounted above.)

## Honest residual (window scope)

PHASE4-N-AL **closed the participant-side half of the recovered-anchor rollback boundary** — the participant MIRROR
of N-AK's single-producer `DC-NODE-32`. The honest residual:

- **The headline boundary (verbatim).** On the participant live-follow path (`run_participant_sync`), the peer's
  post-intersection `RollBackward(anchor)` (exact slot AND hash to the persisted recovered anchor) is now an
  idempotent boundary no-op, evaluated BEFORE the unchanged `DC-NODE-29` stored-block resolution, so the participant
  follow catches up instead of failing closed on `get_block_by_hash(anchor) → None` (`DC-NODE-33`). **The anchor is a
  recovery BOUNDARY, never a servable block** — `ChainDb::tip()` / serve never return it; `pump_block` stays the sole
  roll-forward admit; every non-anchor rollback still flows through the EXISTING `DC-NODE-29` authority.
- **CE-AL-3-LIVE is `enforced`-backing evidence, NOT a `RO-LIVE` flip.** The live end-to-end pass (2026-06-10, FRESH
  2-pool `cardano-testnet` venue, magic 42: bare-anchor recover @ slot 741 → `RollBackward(741)` idempotent no-op →
  first admit @ slot 777 → `agreement_verdict{agreed}` @ slot 801 with `our_hash == peer_hash` exact match; **0
  `UnexpectedRollback` + 0 `UnsupportedRollbackPoint` + 0 diverged**; transcript OUTSIDE-REPO) backs `DC-NODE-33` as
  enforced. It is **NOT** preprod, **NOT** bounty completion. `RO-LIVE-01` stays operator-gated / partial; no
  `RO-LIVE` registry status changed this span.
- **Does NOT prove CE-AI-6 / full ChainSel (load-bearing).** `DC-NODE-33` covers the **participant
  `run_participant_sync` recovered-anchor rollback-to-intersection ONLY**. It does **not** add general multi-candidate
  fork-choice, does **not** prove **CE-AI-6** reorg convergence or natural reorg capture (CE-AI-6 is a SEPARATE
  induced-reorg operator pass), does **not** change the N-AJ convergence-evidence emission (`DC-NODE-30`), does
  **not** flip `CN-CONS-03`, and does **not** broaden `DC-NODE-32` (which stays scoped to the single-producer
  `run_node_sync`). The recovered-anchor rollback boundary is now closed on **both** the single-producer (`DC-NODE-32`)
  and participant (`DC-NODE-33`) follow paths; full ChainSel convergence remains the named follow-on.
- **NO BLUE change — a BLUE-empty window.** `git diff b4c0983d..HEAD` over the BLUE `core_paths` trees is empty
  (456 → 456 / 462 → 462). The single production change is 17 RED lines in `node_lifecycle.rs` reading the EXISTING
  `ForwardSyncState.recovered_anchor` field; everything BLUE-authoritative (`pump_block`, the `DC-NODE-29` resolution,
  `ChainDb::tip()`, the `RecoveredAnchorPoint` codec) is untouched.
- **No new module, no new type, no new CLI flag, no new field.** N-AL reuses AK-S2's `ForwardSyncState.recovered_anchor`
  carrier unchanged; a participant follow over a store with no recovered anchor reproduces pre-AL behavior verbatim.
- **No new CI gate; enforced by the test suite.** `DC-NODE-33` carries `ci_script=""`; the 5 named `participant_*`
  tests are the mechanical enforcement (matching the `DC-NODE-31` / `DC-NODE-32` precedent).
- **CODEMAP + SEAMS + TRACEABILITY refresh owed this close — now ONE cluster behind.** All three were regenerated to
  the N-AK close `b4c0983d` (the prior two-cluster debt was PAID at `efa2a44e`) and now lack only `DC-NODE-33`
  (`grep -c` in each = 0). Because N-AL adds no module and no type, only TRACEABILITY's `DC-NODE-33` row (plus a
  HEAD-pin/count bump to `e87e8a43` / 359 rules) is owed. The registry holds `DC-NODE-33` + its binding
  authoritatively at HEAD (359 rules) in the interim. Regenerating CODEMAP + SEAMS + TRACEABILITY to `e87e8a43` is the
  named follow-on (surfaced in §5).
- **Two in-span commits are prior-window / cross-cluster.** `efa2a44e` (`Close PHASE4-N-AK …`) is docs/registry only
  (0 code) — it flipped `DC-NODE-31`/`DC-NODE-32` to enforced and regenerated all four grounding docs to `b4c0983d`;
  `c3ec7466` (`docs(c2-guide): record PHASE4-N-AK …`) is a docs-only remediation note for the PRIOR cluster. Both sit
  inside `b4c0983d..HEAD` and are recorded in §1/§0 for completeness, but neither is PHASE4-N-AL work.

## Working tree at HEAD `e87e8a43` (close in progress)

**There are UNCOMMITTED working-tree changes at this regen** — the N-AL close artifacts: the registry flip
(`DC-NODE-33 → enforced` + its `tests` array populated), the AL-S1 slice-doc `Merged` flip, the c2-guide / N-AI
convergence-runbook sync, the CODEMAP HEAD-pin touch, the `.idd-config.json` baseline bump, and this HEAD_DELTAS
refresh. §1 narrates the **committed** span `b4c0983d..e87e8a43` verbatim; §0/§7 read rule **status** from the
**current working-tree** registry (so the prose reflects `DC-NODE-33` enforced / 359 rules). The remaining close-pass
actions are (1) committing the close artifacts, and (2) the CODEMAP + SEAMS + TRACEABILITY refresh to `e87e8a43`
(surfaced in §5). **This regen DOES perform the baseline bump** (`b4c0983d → e87e8a43` in `.idd-config.json`
`head_deltas_baseline`, with the `_head_deltas_baseline_doc` lead prepended for N-AL and the N-AK paragraph demoted
to "PRIOR baseline"), per the task's post-close step.

> **Cluster-context note.** PHASE4-N-AL closes with AL-S1 (`e87e8a43`) as the single/last slice — the final rule
> flip (`DC-NODE-33 → enforced`) is carried by the in-progress working-tree close, alongside moving the cluster docs
> to `docs/clusters/completed/PHASE4-N-AL/`.

---

## Historical — PHASE4-N-AK recovered-anchor live-follow start + rollback boundary (`b1bed361 → b4c0983d`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It narrated the
> `b1bed361 → b4c0983d` span (measured from the PHASE4-N-AJ close `b1bed361`): the **N-AJ close commit**
> (`bbdc3585`, docs/registry/config only — registry 354→356, `DC-NODE-30 → enforced` + `DC-EVIDENCE-03 →
> enforced_scaffolding`, baseline bump `e99a86c7 → b1bed361`) + the **PHASE4-N-AK cluster** (two slices AK-S1 +
> AK-S2) — a post-N-AH/N-AI/N-AJ live recover→follow regression remediation. **7 commits, 33 files, +2647 / −544.**
> **This span TOUCHED BLUE — +2 canonical types** (`456 → 458`): one NEW BLUE module
> `crates/ade_ledger/src/recovered_anchor_point.rs` shipping `RecoveredAnchorPoint` (the closed, version-gated,
> byte-canonical anchor-point record) + `RecoveredAnchorPointError` + the sole canonical CBOR codec
> (`RECOVERED_ANCHOR_POINT_SCHEMA_VERSION = 1`). It also added one NEW RED module
> `crates/ade_runtime/src/recovered_anchor.rs` (`load_recovered_anchor_point` — kept OUT of `bootstrap.rs` to
> preserve the `CN-NODE-01` single-`pub fn` closure). **AK-S1 / `DC-NODE-31`** (enforced): persist the bootstrap
> anchor POINT as fingerprint-bound recovery provenance + resolve the live-follow FindIntersect start from it
> (`resolve_live_follow_start`: servable `ChainDb` tip → persisted non-Origin anchor → Origin/None) so a bare-anchor
> recovery starts AT the anchor, not Origin; new `SnapshotStore::{put,get}_recovered_anchor_point` (redb
> `recovered_anchor_point_by_anchor_fp`); 3 new fail-closed `BootstrapError` variants. **AK-S2 / `DC-NODE-32`**
> (enforced): the single-producer `run_node_sync` `RollBack` handler accepts `RollBackward(anchor)` (exact slot AND
> hash) as an idempotent no-op, all else `UnexpectedRollback`; new `ForwardSyncState.recovered_anchor` field; forward
> block admits via the EXISTING `pump_block`. CI gates **159 → 159** (both rules `ci_script=""`, backstopped by
> `ci_check_bootstrap_closure.sh`). Registry **356 → 358** (+2: `DC-NODE-31` + `DC-NODE-32` enforced; `T-REC-05`
> strengthened `+= PHASE4-N-AK`; 0 removed). **At this close, the N-AK close commit `efa2a44e` regenerated all four
> grounding docs to `b4c0983d`** (paying the two-cluster CODEMAP/SEAMS/TRACEABILITY refresh debt the N-AK HEAD_DELTAS
> lead had recorded). Live **CE-AK-3** (2026-06-10, frozen c2-relay) PASSED end-to-end: re-recover → FindIntersect at
> the persisted anchor → `RollBackward(anchor)` idempotent no-op → caught up to `forge_base_block_no=13` == the frozen
> relay tip, **0 `UnsupportedRollbackPoint` + 0 `UnexpectedRollback`**. **NO `RO-LIVE` flip.** SCOPE was the
> single-producer `run_node_sync` path ONLY — the participant path was the named follow-on, closed by **PHASE4-N-AL /
> `DC-NODE-33`** (this doc's current lead). The full §§0–7 narrative is recoverable from this doc's git history at
> `b4c0983d`.

---

## Historical — PHASE4-N-AJ Participant-path convergence evidence emission (`e99a86c7 → b1bed361`)

> Preserved as a pointer. It narrated the `e99a86c7 → b1bed361` span (measured from the PHASE4-N-AI close
> `e99a86c7`): the **N-AI baseline-bump chore** (`c1f4c876`) + **one unrelated docs commit** (`c95e2592`, a C2-guide
> sync) + the **PHASE4-N-AJ cluster** — Participant-path convergence evidence emission, the CE-AI-6 bridge.
> **9 commits, 19 files, +1813 / −35.** **EVIDENCE-ONLY — ZERO BLUE change, 460 canonical types unchanged** (the
> first window since G-N not to touch BLUE; old whole-tree metric). It took the EXISTING N-AI single-best-peer
> rollback-follow receive path and added a **deterministic GREEN evidence side-output** — emitting the EXISTING
> closed `AgreementVerdict` vocabulary (`block_received` / `block_admitted` / `agreement_verdict` via
> `verdict::derive`) to a dedicated `--convergence-evidence-path` JSONL sink (the new GREEN/RED module
> `ade_node::convergence_evidence`). CI gates **157 → 159** (+2). Registry **354 → 356** (+2: `DC-NODE-30` enforced +
> `DC-EVIDENCE-03` enforced_scaffolding; `DC-ADMIT-04` strengthened; **`CN-CONS-03` NOT flipped**; 0 removed).
> Headline (honest boundary): the live `--mode node --participant-venue` rollback-follow path now emits convergence
> EVIDENCE — **NOT authority**. **NO `RO-LIVE` flip.** *(The N-AJ close artifacts were committed by `bbdc3585`, the
> first commit of the SUCCEEDING N-AK window.)* The full §§0–7 narrative is recoverable from this doc's git history at
> `b1bed361`.

---

## Historical — PHASE4-N-AI live fork-choice rollback-follow wiring (`8e2c3672 → 5ec841c8` / close `e99a86c7`)

> Preserved as a pointer. It narrated the `8e2c3672 → 5ec841c8` span: the **N-AH baseline-bump chore**
> (`c66fa9a9`) + the **PHASE4-N-AI cluster** (live fork-choice rollback-follow wiring of the EXISTING
> `chain_selector` → BLUE `select_best_chain` into the live `--mode node` receive path — single-best-peer FOLLOW,
> NOT full ChainSel; `DC-NODE-23`…`DC-NODE-29`; close `5ec841c8`, docs/baseline `e99a86c7`) + one unrelated docs
> commit (`cbad2ae3`). **26 commits, 46 files, +5350 / −53.** **FIRST BLUE delta since G-N: +2 canonical types**
> (`458 → 460` by the old whole-tree metric — the `ade_ledger::wal::event::{RollbackPoint, RollbackReason}` payload
> types of the new closed-sum `WalEntry::RollBack` durable MARKER). CI gates **148 → 157** (+9; 0 modified, 0
> removed). Registry **347 → 354** (+7: `DC-NODE-23..29` enforced; `CN-CONS-01` flipped partial→enforced; 13
> strengthenings; 0 removed). Headline (honest boundary): Ade follows ONE peer's chain-sync `RollBackward` reorg
> end-to-end on a declared Participant venue — replay-equivalently and fail-closed. **Single-best-peer
> rollback-FOLLOW, NOT full multi-peer Cardano ChainSel.** **`CN-CONS-03` was NOT flipped.** The per-cluster
> security review found **H-1** (mixed peer/local rollback target → durable-chain truncation) → remediated by
> **AI-S6 / `DC-NODE-29`** (durable stored point as sole authority, validated pre-mutation, fail-closed) →
> re-review **H-1 CLOSED.** **NO `RO-LIVE` flip.** The full §§0–7 narrative is recoverable from this doc's git
> history at `5ec841c8` / `e99a86c7`.

---

## Historical — PHASE4-N-AG superseded + PHASE4-N-AH local-tip forge-base authority (`f87d0056 → 5858288e`)

> Preserved as a pointer. It narrated the `f87d0056 → 5858288e` span: the **PHASE4-N-AF close tail** (`600581e8`
> + `2d99cdf2`) + the **PHASE4-N-AG cluster** (single-producer loop-continuation-after-feed-EOF, `DC-NODE-19`;
> **superseded-close**) + the **PHASE4-N-AH cluster** (local selected durable chain forge-base authority
> `DC-NODE-20` + cert evidence-only `DC-NODE-21` + single-producer warm-start re-entry `DC-NODE-22`). **32 commits,
> 48 files, +5155 / −743.** **RED/GREEN-only — ZERO BLUE change, 458 → 458 (old metric).** CI gates **143 → 148**
> (+5; 3 modified in place; 0 removed). Registry **343 → 347** (+4; 9 strengthenings; 0 removed). Headline (honest
> boundary): Ade sustained **cert-free single-producer block production on C2-LOCAL** (`cardano-testnet` magic 42)
> against a real Haskell relay (`cardano-node 11.0.1`) — forged on its OWN local durable `ChainDb::tip`, crossed a
> follow-link EOF, settled `> k` immutable, and resumed forging after a hard restart (run-4). NOT preprod. NOT
> bounty completion. No `RO-LIVE` flip. The full §§0–7 narrative is recoverable from this doc's git history at
> `5858288e`.

---

## Historical — PHASE4-N-AF single-producer extend-own-durable-spine (`6363683e → f87d0056`)

> Preserved as a pointer. A **single-slice cluster lead** narrating the `6363683e → f87d0056` span: the
> PHASE4-N-AE.F close grounding-doc refresh (`d3f52e7c`) + a C2-guide doc (`1302417d`) followed by the **OQ-1 /
> DC-NODE-17 investigation** (`bd1a7a73` declared DC-NODE-17 → `dadf4743` live-disproved it as the fix) and the
> **PHASE4-N-AF cluster** (single slice AF.S1 — `DC-NODE-18`, single-producer extend-own-durable-spine). Counts
> at `f87d0056`: 343 rules, 143 CI gates, 458 canonical types (old metric). **GREEN+RED only — BLUE 458 → 458.**
> New gate `ci_check_single_producer_extend_own_spine.sh`. No `RO-LIVE` flip. The full §§0–7 narrative is
> recoverable from this doc's git history at `f87d0056`.

---

## Historical — PHASE4-N-AE.F post-CE-A5 echo-idempotency follow-up (`a76672b9 → 6363683e`)

> Preserved as a pointer. A **single-slice lead** narrating the `a76672b9 → 6363683e` span: the PHASE4-N-AE
> close grounding-doc refresh (`62811a4e`) followed by the **PHASE4-N-AE.F** slice (`DC-NODE-16` receive
> idempotency at the durable-admit chokepoint — a re-announced block Ade already durably holds (same hash, same
> slot) is an idempotent no-op at `pump_block`). Counts at `6363683e`: 341 rules, 142 CI gates, 458 canonical types
> (old metric). **RED chokepoint only — BLUE 458 → 458.** New gate `ci_check_receive_idempotency.sh`. No `RO-LIVE`
> flip. The full §§0–7 narrative is recoverable from this doc's git history at `6363683e`.

---

## Historical — earlier windows (`25ddeebd → a76672b9` and before)

> Preserved as pointers. The **PHASE4-N-AD/N-AE CE-A5 window** (`25ddeebd → a76672b9`, recover→serve continuity +
> forge-on-followed-tip admissibility — the CE-A5 manifest: a real `cardano-node 11.0.1` relay
> `AddedToCurrentChain` an Ade-forged successor block; `DC-NODE-14`/`DC-NODE-15`/`DC-CONS-24`/`DC-PROTO-10`; 336 →
> 340 rules at `a76672b9`); the **PHASE4-N-AC cluster** (KES signing evolves the operator key to the current period
> — `DC-CRYPTO-10`; 335 → 336 rules); the **PHASE4-N-AB cluster** (outbound mux segmentation — `CN-SESS-05`; 334 →
> 335 rules); the **PHASE4-N-AA cluster** (bounded peer-driven serve range — `DC-SERVEMEM-01`; 333 → 334 rules);
> the **PHASE4-N-U cluster + gate-hygiene tail** (forged-block durability — `DC-NODE-12`/`DC-CONS-23`/`DC-WAL-04`/
> `T-REC-05`/`DC-NODE-13`; 328 → 333 rules); and the **G-K…G-R + C1 multi-cluster catch-up** (`550eec3a →
> 65954fa3`, 319 → 328 rules, 126 → 134 CI gates). The full §§0–7 narrative for each is recoverable from this
> doc's git history at the respective HEADs.

---

## Generation notes

### Regen `b4c0983d → e87e8a43` (PHASE4-N-AL participant recovered-anchor rollback no-op — current lead)

- **Baseline valid; one single-slice cluster + the prior-window close commit + one cross-cluster docs note.** Run
  against `b4c0983d` (the PHASE4-N-AK AK-S2 close, the prior HEAD_DELTAS HEAD), which `git rev-parse` resolves and
  `git merge-base b4c0983d HEAD` confirms is a strict ancestor of HEAD `e87e8a43` (`b4c0983d` carries no tag). The
  start-of-regen **working-tree** config baseline was already `b4c0983d` (the previous N-AK close's bump
  `b1bed361 → b4c0983d` is itself an uncommitted working-tree step; the **committed** `.idd-config.json` at HEAD still
  reads `b1bed361`). This regen **performs the baseline bump** `b4c0983d → e87e8a43` as the task's post-close step.
- **Counts are mechanical (git/grep/ls):** commit log + `--shortstat` over `b4c0983d..HEAD` (**4** commits, no
  merges / **14** files / **+1792 / −825**); CI gate count via `ls ci/ci_check_*.sh | wc -l` = **159** at HEAD and
  `git ls-tree -r --name-only b4c0983d ci/ | grep -c ci_check_.*\.sh` = **159** at baseline (`--diff-filter=A/M/D`
  over `ci/` all **empty**); registry rule count via `grep -c '^id = '` at each ref (**358 → 359**; `diff` of the
  sorted `id =` lists shows exactly the single addition `DC-NODE-33`, zero removals); registry status via
  `grep '^status = ' | sort | uniq -c` (committed baseline `b4c0983d` = **222 enforced / 3 enforced_scaffolding / 19
  partial / 114 declared**; working-tree close = **225 / 1 / 19 / 114**); strengthening = **0** (`grep
  'strengthened_in.*PHASE4-N-AL'` = 0 matches); BLUE canonical types **456 → 456** (`git diff b4c0983d..HEAD` over
  the BLUE `core_paths` trees is **empty**).
- **NO BLUE change this span.** `git diff b4c0983d..HEAD` over the configured BLUE `core_paths` trees (`ade_ledger` /
  `ade_codec` / `ade_types` / `ade_crypto` / `ade_plutus` / `ade_core` / the BLUE `ade_network` submodules) is empty
  — no file touched, no `^+(pub )?(struct|enum)` line. BLUE count **456 → 456** (462 → 462 whole-tree). `git diff
  --name-only … '**/Cargo.toml' 'Cargo.toml'` is empty (no manifest/feature-flag delta; no new CLI flag — N-AL adds
  no struct field, reading the EXISTING `ForwardSyncState.recovered_anchor`).
- **No new module.** `git diff --diff-filter=A --name-only b4c0983d..HEAD -- 'crates/**/*.rs'` is empty — no new
  library module and no new test file (the 5 tests are added to the EXISTING `live_fork_choice_ai_s4bii.rs`). No new
  crate / `Cargo.toml` / workspace — still 11 crates.
- **Production change is 17 RED lines + 5 tests.** The only `crates/**/*.rs` files in the span are
  `crates/ade_node/src/node_lifecycle.rs` (+17 — the `run_participant_sync` recovered-anchor no-op branch, evaluated
  BEFORE the `DC-NODE-29` `get_block_by_hash` resolution) and `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs`
  (+194 — 5 new `participant_*` CEs). Everything else in the +1792/−825 is the in-span N-AK close `efa2a44e`
  regenerating the four grounding docs + archiving cluster docs, and `f8275c55`/`c3ec7466` docs.
- **Registry delta is +1 rule, 0 strengthenings, NOT a removal.** `DC-NODE-33` declared at the cluster/slice doc
  (`f8275c55`) then flipped to enforced at close (working tree). The sorted-id `diff` confirms zero removals. No
  existing rule's `strengthened_in` gained `PHASE4-N-AL`. The +3-enforced / −2-enforced_scaffolding status tally
  additionally reflects the `DC-NODE-31` + `DC-NODE-32` `enforced_scaffolding → enforced` flips carried by the in-span
  **N-AK close commit** (`efa2a44e`); at the COMMITTED baseline `b4c0983d` both were still `enforced_scaffolding`.
- **No new CI gate — enforced by tests.** `DC-NODE-33` carries `ci_script=""`; enforced by the 5 named
  `participant_*` tests (matching the `DC-NODE-31` / `DC-NODE-32` / `DC-PROTO-10` test-enforced precedent — a
  reducer-path behavioral property exercised by hermetic CEs, not a textual gate). No `--diff-filter=A/M/D` change
  over `ci/`.
- **STATUS read from the CURRENT working tree (load-bearing).** There are **uncommitted** N-AL close artifacts at
  this regen (the `DC-NODE-33` flip + `tests` array, the AL-S1 slice-doc `Merged` flip, the c2-guide/runbook sync,
  the CODEMAP HEAD-pin touch, the config baseline bump, this HEAD_DELTAS refresh). §1 narrates the **committed** span
  `b4c0983d..e87e8a43` verbatim; §0/§7 read rule **status** from the **current working-tree**
  `docs/ade-invariant-registry.toml` (359 rules, `DC-NODE-33` enforced). The baseline-side counts via `git show
  b4c0983d:docs/ade-invariant-registry.toml` (358 rules, committed status 222/3/19/114).
- **No `RO-LIVE` flip; CE-AL-3-LIVE is enforced-backing evidence.** `DC-NODE-33` is recorded `enforced` (hermetic CEs
  + the live CE-AL-3-LIVE end-to-end pass on a FRESH 2-pool `cardano-testnet` venue: bare-anchor recover @ slot 741 →
  `RollBackward(741)` no-op → admit @ 777 → agreed @ 801 exact-hash-match, 0 `UnexpectedRollback` + 0
  `UnsupportedRollbackPoint` + 0 diverged; transcript OUTSIDE-REPO). This is **NOT** a bounty/preprod claim and does
  **NOT** prove CE-AI-6 reorg / full ChainSel. No `RO-LIVE` registry status changed (`RO-LIVE-01` stays
  operator-gated / partial).
- **Normative docs unchanged this span.** `git diff --name-only b4c0983d..HEAD` over the configured `normative_docs`
  (CE-79 statement + addendum, the three contract docs, CE-73 reclassification, `CLAUDE.md`) is empty — the §7 delta
  is entirely the invariant-registry change.
- **§1 commit log verbatim from `git log` (newest first).** The per-slice synthesis is in §0/§3. Three subjects carry
  a conventional-commits prefix (`feat(...)` / two `docs(...)`); the fourth (`efa2a44e`, `Close PHASE4-N-AK …`) is the
  prior-window **close commit** (no prefix, per the project's close-commit convention) and is **N-AK work, not N-AL**
  (docs/registry only, 0 code); `c3ec7466` is a docs-only remediation note for the PRIOR cluster (also not N-AL).
- **Doc-refresh state — CODEMAP + SEAMS + TRACEABILITY now ONE cluster STALE (refresh owed).** All three were
  regenerated to the N-AK close `b4c0983d` by the in-span `efa2a44e` (paying the prior two-cluster debt) and carry
  `DC-NODE-31` / `DC-NODE-32` / `RecoveredAnchorPoint` + the N-AJ/N-AK modules; they lack only `DC-NODE-33`
  (`grep -c DC-NODE-33` in each = 0). N-AL adds **no module and no type**, so CODEMAP's structural inventory is
  unaffected — only the `DC-NODE-33` four-cell row in TRACEABILITY (plus a HEAD-pin/count bump to `e87e8a43` / 359
  rules across all three) is owed. **Cross-reference warning surfaced in §5.** Regenerate CODEMAP + SEAMS +
  TRACEABILITY to `e87e8a43` as a follow-on this close; the registry holds `DC-NODE-33` + its binding authoritatively
  in the interim (359 rules). No orphan gate (no gate was added).
- **Working tree NOT clean.** This regen runs with the N-AL close artifacts **uncommitted** (registry flip + `tests`
  array, slice-doc `Merged` flip, c2-guide/runbook sync, CODEMAP HEAD-pin touch, config baseline bump, this
  HEAD_DELTAS refresh). The remaining close-pass actions are committing the close artifacts and the CODEMAP + SEAMS +
  TRACEABILITY refresh to `e87e8a43`. **This regen DOES perform the `.idd-config.json` baseline bump** `b4c0983d →
  e87e8a43` (the prior N-AK bump `b1bed361 → b4c0983d` was left uncommitted; this regen advances it to `e87e8a43`),
  per the task's post-close step.
