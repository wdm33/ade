# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `65954fa3` (run-2 genesis-rehearsal reproduction + runbook flag fixes + gate now covers c1 manifests — G-K…G-R + C1 catch-up close, 2026-06-04 23:32)
> HEAD: `4e358e92` (refresh stale G-R serve-handoff comment in containment gate (post-N-U-S3), 2026-06-05 17:17)
> Span: **PHASE4-N-U — forged-block durability** (own-forged durable admit → forged-tip crash recovery + replay-equivalence → serve-as-durable-chain projection), plus the G-K…G-R grounding-doc catch-up tail and the cluster-close-in-progress working tree.
> 14 commits (no merges), 28 files changed, +3726 / -1802 lines.

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`65954fa3`**, the
> `.idd-config.json` `head_deltas_baseline` set by the *previous* (G-K…G-R + C1 catch-up) regen — and
> it is **valid**: `git rev-parse 65954fa3` resolves and `git merge-base 65954fa3 HEAD == 65954fa3`
> (it is a strict ancestor of HEAD). The span is the **single cluster PHASE4-N-U** plus its bracketing
> housekeeping: it opens with `08b64ffc` (the G-K…G-R grounding-doc refresh that landed *after* the
> prior baseline commit `65954fa3`), runs the N-U sketch→plan→doc→three-slice arc, and ends at the
> close-in-progress NIT-hygiene commit `4e358e92` with the **cluster close-record + slice-status edits
> still in the working tree** (uncommitted; see §"Close-in-progress working tree"). The closer bumps
> `head_deltas_baseline` `65954fa3 → 4e358e92` after this regen so the next cluster measures from the
> N-U close HEAD.

This window is a **single-cluster lead: PHASE4-N-U — forged-block durability.** It answers one
structural question that every prior forge/serve cluster left open: *once Ade forges its own block,
does that block become part of the **durable** chain — survive a crash, replay byte-identically, and
get served to a follower as durable history — through the SAME gate received blocks use, with NO
second tip-advance path?* Before N-U a forged block was a **local self-accept artifact only**
(DC-NODE-05): the forge tick advanced no durable tip, and the served view was an in-memory
`ServedChainSnapshot` accumulator that did **not** survive restart. N-U closes that gap across three
slices, each peeling one layer:

- **S1 — own-forged durable admit through the pump (`DC-NODE-12`).** The self-accepted forged block
  is now submitted to the **same durable admit chokepoint received blocks use** (`forward_sync::pump_block`:
  `StoreBlockBytes → AppendWal → AdvanceTip`, durable-before-tip, behind the BLUE admit authority).
  The forge gains **no** second tip-advance path; it feeds an admit *input*. The durable admit is
  **extend-only** — a stale-tip re-forge fails closed (`DC-CONS-23`) — and the bytes admitted durably
  are **byte-identical** to the bytes `self_accept` validated, no re-encode (`DC-WAL-04` prior-fp
  clause + I-10).
- **S2 — forged-tip crash recovery + replay-equivalence (`T-REC-05`, `DC-WAL-04` no-orphan clause).**
  Production `warm_start_recovery` now **forward-replays from the nearest snapshot ≤ tip and
  reconciles the WAL tail**, so a forge-then-kill recovers the same durable tip *byte-identically*;
  an un-WAL'd forged orphan above the WAL tail is **dropped** on recovery.
- **S3 — serve-as-durable-chain projection (`DC-NODE-13`; strengthens `CN-CONS-07`, `DC-NODE-11`).**
  The `--mode node` served view is now a deterministic **read-only projection of the durable
  ChainDb** (`ChainDbServedSource`), not the in-memory accumulator. The G-R monotone serve-gate
  workaround (`DC-NODE-11`) is **superseded by structure**: the durable chain is extend-only, so it
  holds exactly one block 0, the projection serves it stably, **and serving survives restart** —
  whereas the accumulator did not. A follower fetches **coherent history A→B** (never B without A).

Each slice's claim is **NARROW and durability-scoped**. N-U makes a forged block *durable, recoverable,
and coherently servable* — it **flips no `RO-LIVE` rule**, makes no preview/preprod bounty-accept
claim, and demonstrates no operator-witnessed peer acceptance. `RO-LIVE-01` stays operator-gated. The
honest residual (two tracked follow-ons + the serve-availability scope line) is carried in the closing
"Honest residual" section.

## 0. Headline

| Count | Baseline (`65954fa3`) | HEAD (`4e358e92`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 134 | **135** | **+2 new** (`forged_durable_admit_via_pump` S1, `served_chain_projection` S3), **−1 removed** (`served_chain_stability` — S3 mechanism supersession), **+3 modified in place** (`node_run_loop_containment`, `node_serve_lifetime`, `feed_tag24_unwrap`) → **net +1** |
| Registry rules (`docs/ade-invariant-registry.toml`) | 328 | **333** | **+5 new** (`DC-NODE-12`, `DC-WAL-04`, `T-REC-05`, `DC-CONS-23`, `DC-NODE-13`); **+2 strengthenings** (`CN-CONS-07`, `DC-NODE-11` each `strengthened_in += "PHASE4-N-U"`); **0 removed** |
| Registry status (enforced / partial / declared) | 196 / 20 / 112 | **201 / 20 / 112** | **+5 enforced** — all five new rules committed `enforced` in-span (no `declared → enforced` close-flip owed) |
| Test attributes (`#[test]`/`#[tokio::test]`, workspace, broad grep) | 2324 | **2334** | **+10** (broad `git grep -hE '#\[(tokio::)?test'`; CODEMAP's strict line-anchored matcher reads lower → it sat at 2301 at baseline and the catch-up CODEMAP header still reads 2301, so its narrower count tracks separately). Concentrated in S2 (`forge_succeeds.rs` / `node_sync.rs` recovery tests) and S3 (`node_spine_serve_loopback.rs` / `forge_succeeds.rs`) |
| BLUE canonical types | 458 | **458** | **0** — the one BLUE touch (`ade_ledger::block_validity::header_input`) factors `accepted_block_header_bytes` into a new public `block_header_bytes(&[u8]) -> Result<&[u8], …>` **function** so the serve projection can read `StoredBlock.bytes` directly; the wire-byte recipe (`DC-CONS-18`) is byte-identical, no new `struct`/`enum` |

The **+1 net CI gate / +5 rules / +10 tests** are the net of one cluster's three slices. The
slice↔rule↔gate map:

| Slice | New CI gate | Retired CI gate | Rule(s) introduced (`enforced`) | Rule(s) strengthened |
|---|---|---|---|---|
| S1 | `ci_check_forged_durable_admit_via_pump.sh` | — | `DC-NODE-12`, `DC-CONS-23`, `DC-WAL-04` (prior-fp clause) | — |
| S2 | — (T-REC-05 test-enforced; see §5 drift note) | — | `T-REC-05`, `DC-WAL-04` (no-orphan clause) | — |
| S3 | `ci_check_served_chain_projection.sh` | `ci_check_served_chain_stability.sh` | `DC-NODE-13` | `CN-CONS-07` (serve clause), `DC-NODE-11` (mechanism superseded; preserved + strengthened) |

> **Cross-reference (other grounding docs) — EXPECTED catch-up state, not drift.** At this baseline
> `65954fa3` the four grounding docs were all pinned at the **G-K…G-R + C1 catch-up close** (their
> headers read **328 rules / 134 CI / 458 types**, and `git grep` confirms **none** of them mention
> `served_chain_projection`, `ServedChainSource`/`ChainDbServedSource`, or any of the five new N-U
> rules `DC-NODE-12` / `DC-WAL-04` / `T-REC-05` / `DC-CONS-23` / `DC-NODE-13`). **This HEAD_DELTAS is
> the FIRST of the four docs to advance to the N-U HEAD.** The matching CODEMAP / SEAMS / TRACEABILITY
> refresh for N-U is part of *this same close pass* — they are not yet committed at the moment this
> doc is written, so at HEAD `4e358e92` a sibling doc reading 328/134/458 is **mid-close, not stale**.
> The closer must regenerate all three to **333 rules / 135 CI checks** and add the new module +
> rules; that is the expected close sequencing, not a discipline failure.

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `4e358e92` | chore(ci) | refresh stale G-R serve-handoff comment in containment gate (post-N-U-S3) |
| `8e0dbe99` | feat | serve-as-durable-chain projection — enforce DC-NODE-13 (PHASE4-N-U S3) |
| `a49563bc` | docs | slice doc PHASE4-N-U S3 serve-as-durable-chain projection + restate CN-CONS-07 |
| `f7e38712` | test | forged-tip forward-replay recovery — enforce T-REC-05 (PHASE4-N-U S2) |
| `232071f7` | feat | forward-replay + WAL-tail reconciliation in warm_start_recovery (PHASE4-N-U S2, DC-WAL-04 enforced) |
| `985bf966` | docs | slice doc PHASE4-N-U S2 forged-tip crash recovery + replay-equivalence |
| `3fedabea` | test | PHASE4-N-U S1 forged durable admit — 5 tests + enforce DC-NODE-12/DC-CONS-23 |
| `f35451f5` | feat | own-forged durable admit through the pump (PHASE4-N-U S1, DC-NODE-12) |
| `71e789db` | fix(ci) | repair stale node run-loop containment gate target |
| `77d0b4a6` | docs | slice doc PHASE4-N-U S1 forged durable admit |
| `f152b025` | docs | cluster doc PHASE4-N-U forged-block durability |
| `15cebb90` | docs | plan PHASE4-N-U forged-block durability |
| `f3ca7dbd` | docs | sketch PHASE4-N-U forged-block durability invariants |
| `08b64ffc` | docs | refresh grounding docs to HEAD 65954fa3 (G-K…G-R + C1 catch-up) |

No merge commits in the span. **14 commits, zero unclassified** — every commit carries a conventional
prefix (`feat:` / `fix(ci):` / `test:` / `docs:` / `chore(ci):`). The shape is regular: the
**G-K…G-R grounding-doc catch-up** (`08b64ffc`, docs only — the refresh that followed the prior
baseline commit), then the **N-U sketch → plan → cluster doc** (`f3ca7dbd` / `15cebb90` / `f152b025`),
then a containment-gate target repair (`71e789db`), then three slices each **doc → impl → test**
(S1 `77d0b4a6`/`f35451f5`/`3fedabea`; S2 `985bf966`/`232071f7`/`f7e38712`; S3 `a49563bc`/`8e0dbe99`),
then the close-pass NIT-hygiene (`4e358e92`). The cluster **close-record + slice-status flips are
still uncommitted** (working tree — see §"Close-in-progress working tree").

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty
> trailer requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer;
> that is an Ade-local override of the global no-AI-attribution rule and applies to **commit messages
> only**. It does not affect this doc's content.

## 2. New Modules

One new source module landed in this window.

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_runtime::network::served_chain_projection` | **RED** (shell, `crates/ade_runtime/`; the file's `//! RED` header confirms) | Projects the **durable ChainDb** through the BLUE serve reducers' read seams so the `--mode node` serve path serves the durable adopted chain instead of an in-memory accumulator (`DC-NODE-13`; serve-as-projection that supersedes the G-R monotone serve-gate workaround). Read-only: advances no tip, admits nothing, derives no verdict; on a `ChainDbError` yields `None`/empty (serve nothing this round — availability, never wrong/partial bytes). | `served_chain_projection.rs` (+255): `pub struct ChainDbServedSource<'a>` holding `&'a dyn ChainDb`; `impl ServedHeaderLookup` (`next_after` — smallest durable key strictly past the cursor) + `impl ServedRangeLookup` over the durable store; reuses the single BLUE `block_header_bytes` header-projection authority (`DC-CONS-18`) and `decode_block`, serving `stored.bytes` **verbatim** (`DC-CONS-17`) — no parallel splitter, no `AcceptedBlock` reconstruction. | `PHASE4-N-U` S3 (`8e0dbe99`) |

> **Cross-reference (CODEMAP) — EXPECTED, sequencing not staleness.** `git grep served_chain_projection
> docs/ade-CODEMAP.md` returns **0** at HEAD — the new module is **not yet in CODEMAP**. This is the
> expected close sequencing (see §0 cross-reference note): CODEMAP is pinned at the prior G-K…G-R
> catch-up close and the N-U refresh is part of *this* close pass. The closer must add
> `ade_runtime::network::served_chain_projection` to CODEMAP §RED. It is **not** a "CODEMAP is silently
> stale" anomaly — it would only become one if CODEMAP were left unrefreshed after this close.

The rest of the code work in this window is **modification of existing modules** (§3), **new CI
gates** (§5), and the **registry delta** (§7). No new crate, no new workspace, no new `Cargo.toml`.

> **Diff-glob footgun (generation note).** `git diff --diff-filter=A --name-only 65954fa3..HEAD --
> 'crates/*/src/'` returns **empty** and would wrongly suggest "no new module" — the glob `crates/*/src/`
> matches one path level only and misses `crates/ade_runtime/src/network/served_chain_projection.rs`
> (two levels deep). The module's newness was confirmed by `git diff --name-status` (`A`) + `git
> cat-file -e 65954fa3:…` (absent at baseline). Verify new modules with `--name-status` over the full
> changed-file set, not a one-level glob.

## 3. Modules Modified

Grouped by slice. Each row names the slice's BLUE/GREEN/RED touch and the rule it backs. Per-file
line counts are `git show --numstat <commit> -- <file>` for the slice's impl/test commits.

### S1 — own-forged durable admit through the pump (`f35451f5` impl, `3fedabea` tests)

`DC-NODE-12` + `DC-CONS-23` + `DC-WAL-04` (prior-fp clause) `enforced` · +1 gate

| Module | Color | Key changes |
|--------|-------|-------------|
| `ade_node::node_sync` | **GREEN/RED** (`crates/ade_node/`) | **S1 (`f35451f5`, +53).** New `pub fn admit_forged_block_durably` — the **fenced driver** that takes a `&SelfAcceptedHandoff` and feeds its bytes (`.accepted().as_bytes()`, **no re-encode**, I-10) into the existing `forward_sync::pump_block` chokepoint (`StoreBlockBytes → AppendWal → AdvanceTip`, durable-before-tip, behind the BLUE admit authority). Adds **no** admit-time fork-choice; a stale-tip re-forge fails closed inside `pump_block` via `block_validity` / prior-fp (`DC-CONS-23`). |
| `ade_node::node_lifecycle` | **RED** (relay-loop home) | **S1 (`f35451f5`, +33 / −5).** The `--mode node` `ForgeTick` arm now calls `admit_forged_block_durably` **before** the G-R serve handoff, so `pump_block` advances `state.receive` + the durable ChainDb together and the next forge builds N+1. The forge advances **no** durable tip directly — `DC-NODE-05` preserved (the loop body still reaches for no direct apply / manual-tip / rollback). |

New gate `ci_check_forged_durable_admit_via_pump.sh` fences the driver body: routes through
`pump_block(`, feeds `.accepted().as_bytes()` (no re-encode), adds no fork-choice, no manual tip
advance. The containment gate `ci_check_node_run_loop_containment.sh` was **extended in place** to
allow-list the 2nd fenced tip-advancer (`admit_forged_block_durably`) while still forbidding direct
`pump_block`/`put_block`/`AdvanceTip`/`rollback` in the loop body. Tests (`3fedabea`):
`forge_tick_durable_admit_advances_tip`, `forge_successor_builds_block_1_from_durable_tip`,
`forged_admit_bytes_byte_identical_to_self_accept`, `stale_tip_forge_fails_closed`,
`forged_admit_wal_prior_fp_chains`.

### S2 — forged-tip crash recovery + replay-equivalence (`232071f7` impl, `f7e38712` test)

`T-REC-05` + `DC-WAL-04` (no-orphan clause) `enforced`

| Module | Color | Key changes |
|--------|-------|-------------|
| `ade_node::node_lifecycle` | **RED** (recovery home, `crates/ade_node/`) | **S2 (`232071f7`, +130 / −67).** Production `warm_start_recovery` (the `bootstrap_initial_state` warm-start branch) now **forward-replays from the nearest snapshot ≤ tip and reconciles the WAL tail** (`recover_node_state`-style), replacing the prior snapshot-at-tip-only placeholders. It reconstructs the seed-epoch sidecar (`from_seed_epoch_consensus_inputs`) so the recovered surface is realistic, and **drops a forged orphan above the WAL tail** (an un-WAL'd block constructed but not yet durably appended) so recovery converges to the durable tip (`DC-WAL-04` no-orphan clause). |
| `ade_node::node_sync` | test (`crates/ade_node/`) | **S2 (`f7e38712`, +154).** In-crate recovery test `forge_kill_then_warm_start_recovers_same_tip_via_forward_replay`: forge a genesis-successor block, kill, then assert `warm_start_recovery` recovers the **byte-identical** durable tip via forward replay from the genesis slot-0 snapshot (`T-REC-05`). In-crate to avoid `make_node_schedule(0,0)` reconstruction churn. |

`T-REC-05` is **test-enforced** (`ci_script = ""` in the registry — see §5 drift note). The S2 work
backs the replay-equivalence half of the cluster: same checkpoint + same WAL → same post-state.

### S3 — serve-as-durable-chain projection (`8e0dbe99` impl)

`DC-NODE-13` `enforced`; `CN-CONS-07` + `DC-NODE-11` `strengthened_in += "PHASE4-N-U"` · +1 gate, −1 gate

| Module | Color | Key changes |
|--------|-------|-------------|
| `ade_runtime::network::served_chain_projection` | **RED** (**NEW** — §2) | **S3 (`8e0dbe99`, +255).** The new `ChainDbServedSource` projection adapter (see §2). |
| `ade_runtime::network::serve_dispatch` | **RED** (`crates/ade_runtime/`) | **S3 (`8e0dbe99`, +167 / churn).** The single serve-dispatch authority (`DC-NODE-07`) gains a **closed source-selector enum** `ServedChainSource<'a>` with two arms — `Snapshot(view)` (the existing in-memory accumulator, still used by `--mode produce`) and **`DurableChainDb(&'a dyn ChainDb)`** (the new `--mode node` durable projection, constructed per dispatched frame as a `ChainDbServedSource`). The enum only selects **where** the one dispatch authority reads; the dispatch logic is otherwise unchanged. |
| `ade_ledger::block_validity::header_input` | **BLUE** (`core_paths` `crates/ade_ledger/`) | **S3 (`8e0dbe99`, +36 / churn).** `accepted_block_header_bytes` is refactored to delegate to a **new public `block_header_bytes(&[u8]) -> Result<&[u8], BlockValidityError>`** — the **same** `header_cbor_slice`/`decode_block_envelope` recipe (`DC-CONS-18`), factored out for callers that hold raw canonical block bytes (`StoredBlock.bytes`) rather than an `AcceptedBlock` token. **Byte-identical** projection, **no** parallel splitter, **no** new type (one new fn; re-exported from `block_validity::mod`). |
| `ade_node::{node_lifecycle, node_sync}` | **RED/GREEN** (`crates/ade_node/`) | **S3 (`8e0dbe99`, lifecycle +88 / −155; node_sync +16 / −31).** The `--mode node` serve task now takes the **durable ChainDb as a READ source** (`ServedChainSource::DurableChainDb`) instead of the in-memory `ServedChainView` accumulator; the net negative line count is the **retirement** of the G-R `serve_gate_admits` accumulator + `serve_gate_*` tests (mechanism superseded). |
| `ade_node::produce_mode` | **RED/GREEN** (`crates/ade_node/`) | **S3 (`8e0dbe99`, +8 — near-trivial).** Call-site adjustment for the new source enum: `--mode produce` continues to serve the `self_accept`'d accumulator via the explicit `ServedChainSource::Snapshot(view)` arm (the original `CN-CONS-07` token-proof; no durable admit path in produce mode). Not a behavior change — the enum just made the source explicit. |

New gate `ci_check_served_chain_projection.sh` backs `DC-NODE-13`: pins that `ChainDbServedSource`
exists, implements **both** BLUE serve seams over the durable ChainDb, reuses the single
`block_header_bytes` authority, serves `stored.bytes` verbatim (no envelope re-walk), and is
read-only. The G-R gate `ci_check_served_chain_stability.sh` is **retired** (mechanism superseded —
the monotone-serve property is now a structural consequence of serve-as-projection over the
extend-only durable chain). Tests: `served_view_projects_durable_chain`,
`follower_fetches_coherent_history_incl_ingested_predecessor`, `served_view_retires_accumulator`.

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace `Cargo.toml`
(`git grep -l '^\[features\]'` over all `Cargo.toml` is empty), and **no `Cargo.toml` changed at all
in this window** (`git diff --name-only 65954fa3..HEAD -- '**/Cargo.toml' 'Cargo.toml'` is empty). No
`#[cfg(feature = …)]` gate was introduced (`git diff … | grep -c 'cfg(feature'` over `crates/` is
**0**); no coupling, no `compile_error!` guard. The closed `ServedChainSource` source-selector enum is
a **runtime** dispatch selector inside the RED shell, not a compile-time feature flag.

## 5. CI Checks (134 → 135; +2 new, +3 modified in place, −1 removed)

Two new gates, three in-place extensions, and **one removal** (a justified mechanism supersession),
repo-root-relative, mirroring the `ci/ci_check_*.sh` convention. `git diff --diff-filter=A
65954fa3..HEAD -- ci/` lists exactly the two new gates; `--diff-filter=D` lists exactly
`ci_check_served_chain_stability.sh`; `--diff-filter=M` lists exactly the three extended gates.

### New gates

| Check | Status | Slice origin | What it checks |
|-------|--------|--------------|----------------|
| `ci_check_forged_durable_admit_via_pump.sh` | **New** | S1 (`f35451f5`) | Backs **`DC-NODE-12` + `DC-CONS-23` + `DC-WAL-04`**. Fences the `admit_forged_block_durably` driver body (in `node_sync.rs`, production code only): (pos) routes through `pump_block(` — the single durable apply engine; (pos) feeds the self-accepted bytes via `.accepted()` + `.as_bytes()` (I-10: no re-encode / reserialize between `self_accept` and durable admit); (neg) no admit-time fork-choice, no manual tip advance (a stale-tip forge fails closed inside `pump_block` via `block_validity` / prior-fp, never an own-block override). |
| `ci_check_served_chain_projection.sh` | **New** | S3 (`8e0dbe99`) | Backs **`DC-NODE-13`**. Pins that the projection adapter `ChainDbServedSource` exists and implements **both** BLUE serve seams (`ServedHeaderLookup` + `ServedRangeLookup`) over the durable ChainDb (`iter_from_slot` / `get_block_by_hash` / `tip`); that it reuses the single `DC-CONS-18` header authority (`block_header_bytes`) and serves `stored.bytes` **verbatim** (no parallel splitter / envelope re-walk); and that the serve path is read-only (advances no tip, admits nothing). |

### Modified gates (extended in place)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_node_run_loop_containment.sh` | **Modified in place** | PHASE4-N-F-D S2 origin; **N-U S1 + S3 extension** | The relay run-loop containment gate, extended so the loop body may advance the durable tip through **two** fenced drivers — `run_node_sync` (received blocks, `DC-SYNC-02`) and `admit_forged_block_durably` (own-forged, `DC-NODE-12`) — each routing through `pump_block`, while still forbidding direct apply / manual-tip / `pump_block` / `put_block` / `AdvanceTip` / rollback in the loop body. (`71e789db` separately repaired a stale target path on this gate before S1.) |
| `ci_check_node_serve_lifetime.sh` | **Modified in place** | PHASE4-N-F-G-K S1 origin (`DC-NODE-09`); **N-U S3 extension** | The serve-lifetime gate (serve task outlives feed-end, owned by the operator shutdown watch), extended so it now pins that the serve source migrated from the in-memory `ServedChainView` accumulator to the **durable ChainDb projection** (`ServedChainSource::DurableChainDb`): the serve task takes the durable ChainDb as a **READ** source (`Arc<dyn ChainDb>`), still read-only (no WAL/forge handle, no durable-write call). |
| `ci_check_feed_tag24_unwrap.sh` | **Modified in place** | PHASE4-N-F-G-O origin (`CN-WIRE-12`); **N-U S3 touch** | The feed-side BlockFetch tag-24 unwrap gate (`CN-WIRE-12`), touched in the S3 commit to keep its asserted test/source anchors (`forge_succeeds.rs`, `node_spine_serve_loopback.rs`) aligned after the S3 serve refactor moved/renamed serve-path test scaffolding. Closed shape unchanged. |

### Retired gate (mechanism supersession — justified removal)

| Check | Status | Origin / removal | Why removed |
|-------|--------|------------------|-------------|
| `ci_check_served_chain_stability.sh` | **Removed** | PHASE4-N-F-G-R S1 origin (`DC-NODE-11`); **retired by N-U S3** | The G-R monotone serve-gate gate. Its stability property — the served chain head advances monotonically, so a follower sees a stable block 0 — is now a **structural consequence** of serve-as-projection over the **extend-only** durable chain (`DC-NODE-13` + `DC-CONS-23`): own-forged blocks are durably admitted (`DC-NODE-12`), a re-mint block 0 fails closed at the extend-only admit (`DC-CONS-23`), so the durable chain holds exactly one block 0 and the projection serves it stably **and survives restart** (`T-REC-05`). `DC-NODE-11` is **preserved + strengthened** (`strengthened_in += "PHASE4-N-U"`; its `ci_script` repointed to `ci_check_served_chain_projection.sh`) — the rule is NOT removed, only its now-redundant accumulator-gate enforcement mechanism. The retired `serve_gate_*` tests go with it.

> **Removal honesty (load-bearing).** This is a **mechanism supersession, not a discipline violation**:
> no rule was removed (registry is 328 → 333, all additive + 2 strengthenings), and the property the
> retired gate enforced is now enforced *more strongly* (survives restart) by the projection gate +
> the extend-only durable admit. The retired gate's name still appears once in the registry — in
> `DC-NODE-12`'s `evidence_notes`, documenting the supersession as historical context, not as a live
> reference (`git grep` confirms no other live consumer at HEAD).

> **CI/test drift on S2 (recorded honestly in the close record).** The cluster doc §8 originally named
> a CE-5 gate `ci_check_forged_tip_recovery.sh` and a CE-6 test `forge_two_clean_runs_byte_identical`;
> **neither was created literally.** S2 instead enforced replay-equivalence via the kill-recover
> fingerprint-equality test (recovered fp == WAL-tail post_fp) — `T-REC-05` is recorded **test-enforced**
> (`ci_script = ""`, with rationale) and `DC-WAL-04`'s no-orphan clause via
> `warm_start_drops_orphan_block_above_wal_tail`. The invariants are enforced; the §8 CE *artifact
> names* drifted during S2. This is surfaced in the §"Close-in-progress working tree" close record, not
> hidden. (TRACEABILITY should record `T-REC-05` as test-only — no CI cell — when it refreshes.)

> **Cross-reference (TRACEABILITY).** At HEAD the registry binds: `DC-NODE-12` →
> `ci_check_forged_durable_admit_via_pump.sh` + `ci_check_node_run_loop_containment.sh`; `DC-CONS-23` +
> `DC-WAL-04` → `ci_check_forged_durable_admit_via_pump.sh`; `T-REC-05` → `""` (test-enforced);
> `DC-NODE-13` + `DC-NODE-11` → `ci_check_served_chain_projection.sh`; `CN-CONS-07` →
> (its existing four gates, unchanged). The TRACEABILITY refresh for this window should cite the two
> new gates, record `T-REC-05` as test-only, and drop the retired `ci_check_served_chain_stability.sh`
> row.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. **No new canonical
type was introduced in this window** (BLUE count unchanged, 458 → 458). The single BLUE touch
(`ade_ledger::block_validity::header_input`, S3) adds **one function** — `pub fn block_header_bytes(&[u8])
-> Result<&[u8], BlockValidityError>`, factored out of `accepted_block_header_bytes` so the serve
projection can read `StoredBlock.bytes` directly — and **no `struct`/`enum`**; the header-projection
recipe (`DC-CONS-18`) is byte-identical. The closed `ServedChainSource<'a>` enum added in S3 lives in
the **RED** shell (`ade_runtime::network::serve_dispatch`), not a BLUE `core_paths` module, so it is
not a BLUE canonical type; it is a runtime serve-source selector.

## 7. Normative / Invariant Rule Delta (328 → 333)

**Five rule IDs added, two strengthenings recorded in-span, zero removed** (328 → 333). All five new
rules are committed **`enforced`** in this span (verified by reading the registry at HEAD: each has
`status = "enforced"`, `introduced_in = "PHASE4-N-U"`, a bound `tests` array, and — except the
test-enforced `T-REC-05` — a bound `ci_script`). **No `declared → enforced` close-flip is owed** —
each rule was committed enforced at its slice.

| Rule | Family / Tier | Introduced in | What it pins |
|------|---------------|---------------|--------------|
| `DC-NODE-12` | DC / `derived` | PHASE4-N-U (S1) | **Own-forged durable admit chokepoint.** A self-accepted forged block may become durable ONLY via the **same** durable admit chokepoint received blocks use (`admit_forged_block_durably` → `forward_sync::pump_block`: `StoreBlockBytes → AppendWal → AdvanceTip`, durable-before-tip, behind the BLUE admit authority, extend-only). The forge has **no** second tip-advance path; bytes admitted durably are byte-identical to bytes `self_accept` validated. **Supersedes** the DC-NODE-05 "forged block is a local artifact only" consequence while **preserving** DC-NODE-05's deeper invariant (forge advances no durable tip directly; `pump_block` remains sole durable tip-advance authority). `ci_script = ci_check_forged_durable_admit_via_pump.sh` + `ci_check_node_run_loop_containment.sh`. |
| `DC-CONS-23` | DC / `derived` | PHASE4-N-U (S1) | **Extend-only durable admit.** The durable admit is extend-only — a **stale-tip forge fails closed** inside `pump_block` via `block_validity` / prior-fp; there is no own-block override of the extend-only rule. `ci_script = ci_check_forged_durable_admit_via_pump.sh`; test `stale_tip_forge_fails_closed`. |
| `DC-WAL-04` | DC / `derived` | PHASE4-N-U (S1 prior-fp clause, S2 no-orphan clause) | **WAL chaining + recovery reconciliation.** The forged-admit WAL entry **chains its prior fingerprint** (no re-encode break); and on `WarmStart` the WAL-tail reconciliation **drops a forged orphan above the WAL tail**. `ci_script = ci_check_forged_durable_admit_via_pump.sh`; tests `forged_admit_wal_prior_fp_chains`, `warm_start_drops_orphan_block_above_wal_tail`. |
| `T-REC-05` | T / `true` | PHASE4-N-U (S2) | **Forged-tip crash-recovery replay-equivalence.** A forge-then-kill recovers the **byte-identical** durable tip via forward replay from the nearest snapshot ≤ tip + WAL-tail reconciliation (same checkpoint + same WAL → same post-state). **Test-enforced** (`ci_script = ""`); test `forge_kill_then_warm_start_recovers_same_tip_via_forward_replay`. |
| `DC-NODE-13` | DC / `derived` | PHASE4-N-U (S3) | **Serve-as-durable-chain projection.** The `--mode node` served view (ChainSync header advertisement + BlockFetch body) is a deterministic **read-only projection of the durable ChainDb** (`ChainDbServedSource`), whose sole production writers are `pump_block` (`DC-NODE-12`) + the validated warm-start / genesis replay `bootstrap_initial_state` — serving cannot leak a byte that did not clear `block_validity`; a follower fetches coherent history A→B; **not** the retired in-memory `ServedChainSnapshot` accumulator. `ci_script = ci_check_served_chain_projection.sh`; tests `served_view_projects_durable_chain`, `follower_fetches_coherent_history_incl_ingested_predecessor`, `served_view_retires_accumulator`. |

**Strengthenings (`strengthened_in += "PHASE4-N-U"`):**

| Rule | Family / Tier | Strengthening |
|------|---------------|---------------|
| `CN-CONS-07` | CN / `release` | **Serve-provenance clause.** The forge-self-accept gate's serve clause is **generalized from an in-memory-token proof to a durable-provenance proof**: when `--mode node` serves its adopted chain (forged AND received), the bytes are a read-only projection of the durable ChainDb whose sole production writers are `pump_block` + validated warm-start replay — a strict generalization of the N-G token-proof (the durable ChainDb is precisely where the `AcceptedBlock` + `AdmittedBlock` gate outputs land, durable-before-tip), **not** a relaxation. Prior `strengthened_in`: N-G, N-H, N-F-G-B. |
| `DC-NODE-11` | DC / `derived` | **Mechanism superseded, invariant preserved + strengthened.** The G-R monotone serve-gate **mechanism** is superseded by serve-as-projection over the extend-only durable chain; the stability property is now a structural consequence (durable chain holds exactly one block 0, projection serves it stably) **and survives restart** (`T-REC-05`) — strictly stronger than the in-memory gate. `ci_script` repointed `ci_check_served_chain_stability.sh` → `ci_check_served_chain_projection.sh`; the `serve_gate_*` tests retired. The rule is **preserved + strengthened, not weakened**. |

**No rule was removed (expected: 0).** The 328 → 333 delta is five purely-additive `enforced` IDs
plus two `strengthened_in` appends. Family spread of the new rules: **3 DC** (`DC-NODE-12/13`,
`DC-CONS-23`, `DC-WAL-04` — four DC IDs touched, one of which spans two slice clauses), **1 T**
(`T-REC-05`). Registry status tally moves **196 → 201 enforced** (the five new IDs), partial **20**
and declared **112** unchanged.

## Close-in-progress working tree (uncommitted at HEAD `4e358e92`)

The cluster **close record + slice-status flips are staged in the working tree, not yet committed** —
this regen is the grounding-doc step of the close pass and runs *before* the close commit. `git status
--short` at HEAD shows three modified docs (plus an untracked `.mithril-scratch/` scratch dir, ignored):

- **`docs/clusters/PHASE4-N-U/cluster.md`** (+18 / −1) — the §13 **Close record**: marks **CLOSED
  2026-06-05**, 3 slices merged, registry **328 → 333** (the five new rules `enforced` + the two
  strengthenings), the per-CE mechanical results (incl. the honest S2 artifact-name drift note), the
  IDD-reviewer **PASS** (one NIT, fixed in `4e358e92`) + cross-slice security **PASS** (no HIGH/CRITICAL),
  and the two tracked follow-ons (below). The cluster-wide gate sweep records **0 S3-introduced
  regressions** and **`cargo test --workspace --exclude ade_testkit` → 0 failed** (ade_testkit excluded
  for the pre-existing ~600s corpus-suite timeout, environmental).
- **`docs/clusters/PHASE4-N-U/S2-forged-tip-recovery.md`** (+1 / −1) — status `in progress → done`.
- **`docs/clusters/PHASE4-N-U/S3-serve-as-durable-chain-projection.md`** (+1 / −1) — status `in
  progress → done`.

These are docs-only and complete the close; no source/CI/registry change remains in the working tree.

## Honest residual (window scope)

PHASE4-N-U made a forged block **durable, crash-recoverable, and coherently servable** through the
same gate received blocks use. The honest boundary:

- **Durability + coherent serve ≠ peer acceptance — NO `RO-LIVE` flip.** No `RO-LIVE` rule was flipped
  in this window. `RO-LIVE-01` stays operator-gated. N-U enforces the forged-block durability
  MECHANISM + recovery + serve-as-projection; it makes **no** preview/preprod bounty-accept claim and
  demonstrates **no** operator-witnessed peer acceptance. The live C1/preprod leg stays a separate,
  still-owed capture.
- **Hermetic / in-crate evidence.** The recovery and serve tests are hermetic (in-crate kill-recover,
  loopback serve); the C1 genesis-rehearsal mechanical regression is preserved (a follower still
  adopts the served block 0, now via the durable projection — `served_view_projects_durable_chain`),
  but the **live** C1 rerun stays operator-gated.
- **No durable long-chain progression demonstrated.** N-U proves N+1 builds from the durable tip and a
  forged block survives one kill-recover cycle byte-identically; it does **not** demonstrate a
  sustained many-block forged chain over a real peer. That is downstream of an operator-witnessed run.
- **Two tracked follow-ons (non-blocking now; before any large-chain live serve).** Recorded in the
  close record, both reinforce the no-live-serve claim:
  - **[MEDIUM]** `ChainDb::iter_from_slot` (pre-existing, `chaindb/persistent.rs`) materializes the
    full range + O(N²) hash recovery, and the serve path has **no per-request range cap** →
    per-request availability amplification on a long chain. Needs a streaming iterator +
    max-blocks-per-range bound before any large-chain live serve.
  - **[LOW]** > 64 KB block bodies cannot be served (session encoder does not segment payloads >
    `MAX_PAYLOAD` 65 535 B → drops the peer, fail-closed); unbounded inbound accept in
    `run_node_serve_task` (pre-existing shared infra).
- **No BLUE-authority weakening.** The one BLUE touch (`block_header_bytes` extraction) is a
  byte-identical refactor of the `DC-CONS-18` header-projection recipe — same wire bytes, one new fn,
  no new type. The retired `ci_check_served_chain_stability.sh` is a **mechanism supersession**, not a
  rule removal — the stability property is now enforced more strongly (survives restart) by the
  projection gate over the extend-only durable admit. **0 canonical-type delta, 0 rule removals, +2
  in-span strengthenings, +5 new enforced rules.**

---

## Historical — PHASE4-N-F-G-K … G-R + C1 window (`550eec3a → 65954fa3`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It was a
> **multi-cluster catch-up** narrating the `550eec3a..65954fa3` span — the PHASE4-N-F-G-J close-pass +
> eight clusters (G-K through G-R) + the C1 genesis-successor rehearsal reproduction evidence. Counts in
> this Historical section are the figures **at `65954fa3`** (328 rules, 134 CI gates, 458 canonical
> types); the current window measures **forward** from `65954fa3`. The full G-K…C1 §§0–7 narrative (and
> the G-J window before it) is recoverable from this doc's git history at `65954fa3`.

> Baseline: `550eec3a` (PHASE4-N-F-G-J close — last state the grounding docs reflected, 2026-06-03 22:02)
> HEAD: `65954fa3` (run-2 genesis-rehearsal reproduction + runbook flag fixes + gate now covers c1 manifests, 2026-06-04 23:32)
> Span: **G-J close-pass → G-K, G-L, G-M, G-N, G-O, G-P, G-Q, G-R → C1 genesis-successor rehearsal evidence** — 28 commits, 73 files, +4967 / −243.

This window was a **multi-cluster catch-up**. Ade closed **eight clusters** (G-K through G-R) plus a
G-J **close-pass** and a C1 **genesis-successor rehearsal evidence** pass, each peeling off the next
concrete blocker on the path to a live C1 genesis-successor follower adopting an Ade-forged block 0
over a real `cardano-node` peer. The chain: serve-listener lifetime (G-K, `DC-NODE-09`) → real-node
**handshake** compat (G-L, `CN-WIRE-10`) → real-node **ChainSync FindIntersect** compat (G-M,
`CN-WIRE-11`, + the closed BLUE enum `ArrayHead = Definite(u64) | Indefinite`, the window's only +1
canonical type, 457 → 458) → recovered-eta0 **WarmStart** so the follower's leader check stops failing
(G-N, `T-REC-04` + `DC-CINPUT-03`) → feed-side **tag-24 unwrap** so block-fetch payloads decode (G-O,
`CN-WIRE-12`) → feed-side **leader-threshold view** so the follower validates + ingests block 0 (G-P,
`DC-CINPUT-04`) → **forge-successor position** from the evolved admitted spine so the node survives
past block 0 (G-Q, `DC-NODE-10`) → **stable served block 0** via a monotone serve gate so the follower
adopts it (G-R, `DC-NODE-11`, gate `ci_check_served_chain_stability.sh`) → and finally the C1
**reproduction evidence** (two recorded runs).

Each of G-L…G-R made a **NARROW, live-confirmed** structural claim — "live-confirmed" meaning *the
specific failure that gated the follower is gone against the real preprod/C1 peer* — but **no**
bounty/preprod-accept claim and **no** `RO-LIVE` flip; `RO-LIVE-01` / `RO-LIVE-06` stayed
operator-gated, and the C1 genesis rehearsal was banked under `CN-REHEARSAL-FIDELITY-01`
non-promotability (`is_rehearsal = true`, `not_bounty_evidence = true`), **not** bounty evidence.

**G-K…C1 headline (at `65954fa3`):** CI gates **126 → 134** (+8 new, one per cluster G-K…G-R, +
`ci_check_rehearsal_manifest_schema.sh` modified in place for C1, 0 removed); registry **319 → 328**
(+9 new — `DC-NODE-09`, `CN-WIRE-10`, `CN-WIRE-11`, `T-REC-04`, `DC-CINPUT-03`, `CN-WIRE-12`,
`DC-CINPUT-04`, `DC-NODE-10`, `DC-NODE-11`, all `enforced`; 0 strengthenings; 0 removed); BLUE
canonical types **457 → 458** (+1 `ArrayHead`); tests **2305 → 2324** (broad grep). **No new module**
in that window (every code change extended an existing module; new `.rs` files were test
fixtures/captures). **Note:** the G-R gate `ci_check_served_chain_stability.sh` introduced in that
window is **retired in the current PHASE4-N-U window** (mechanism superseded by serve-as-projection —
see §5), and `DC-NODE-11` is strengthened there.

> *(The G-E…G-I leads were never re-led in HEAD_DELTAS — each was closed with its own grounding-doc
> refresh and lives in its own close-pass commit + the registry; they are not reconstructed here. The
> G-J lead before that is recoverable from this doc's git history at `65954fa3`.)*

---

## Generation notes

### Regen `65954fa3 → 4e358e92` (PHASE4-N-U — current lead)

- **Baseline valid; single-cluster lead.** Run against the config baseline `65954fa3` (the prior
  G-K…G-R + C1 catch-up close), which `git rev-parse` resolves and `git merge-base 65954fa3 HEAD`
  confirms is a strict ancestor of HEAD. The span is the single cluster **PHASE4-N-U** plus the
  G-K…G-R grounding-doc catch-up tail (`08b64ffc`, the docs refresh that followed the prior baseline
  commit) and the close-in-progress working tree. The closer bumps `head_deltas_baseline`
  `65954fa3 → 4e358e92` after this regen.
- **Counts are mechanical (git/grep/ls only, no cargo):** commit log + `--shortstat` over
  `65954fa3..HEAD` (**14** commits, no merges / **28** files / **+3726 / −1802**); CI gate count via
  `git ls-tree -r --name-only <ref> ci/ | grep -c 'ci_check_.*\.sh'` at each ref (**134 → 135**;
  `--diff-filter=A` over `ci/` = the two new gates, `--diff-filter=D` = exactly
  `ci_check_served_chain_stability.sh`, `--diff-filter=M` = the three extended gates); registry rule
  count via `grep -cE '^\s*id\s*='` at each ref (**328 → 333**; `comm` of sorted id lists shows
  exactly five adds — `DC-NODE-12`, `DC-WAL-04`, `T-REC-05`, `DC-CONS-23`, `DC-NODE-13` — and **zero**
  removals); registry status via `grep -E '^status = ' | sort | uniq -c` at HEAD (**201 enforced / 20
  partial / 112 declared**); workspace test attributes via `git grep -hE '#\[(tokio::)?test'` over
  `crates/**/*.rs` + top-level `*.rs` (**2324 → 2334**, +10 broad-grep).
- **All five new rules committed `enforced` in-span (NO close-flip owed); 2 strengthenings.** Verified
  by reading the registry at HEAD: each of the five IDs is `status = "enforced"`, `introduced_in =
  "PHASE4-N-U"`, with bound `tests` (and a bound `ci_script` except the test-enforced `T-REC-05`,
  `ci_script = ""`). Two strengthenings recorded — `CN-CONS-07` (`strengthened_in` now
  `[N-G, N-H, N-F-G-B, PHASE4-N-U]`) and `DC-NODE-11` (`strengthened_in = [PHASE4-N-U]`, `ci_script`
  repointed to the new projection gate).
- **One new module, +0 canonical type, no Cargo.toml change.** `git diff --name-status 65954fa3..HEAD`
  shows **`A crates/ade_runtime/src/network/served_chain_projection.rs`** (RED; absent at baseline per
  `git cat-file -e`). **Caution:** `git diff --diff-filter=A --name-only … 'crates/*/src/'` returns
  empty (one-level glob misses the two-level `…/network/served_chain_projection.rs`) — verify new
  modules with `--name-status` over the full changed set, not a one-level glob. The BLUE touch
  (`block_header_bytes` extraction) adds **one function, no type** (458 → 458 unchanged); the new
  `ServedChainSource` enum is RED (`serve_dispatch.rs`), not a BLUE canonical type. `git diff
  --name-only … '**/Cargo.toml' 'Cargo.toml'` is empty (no feature-flag delta; no `[features]` table
  exists workspace-wide).
- **CI removal is a mechanism supersession, NOT a discipline violation.** `ci_check_served_chain_stability.sh`
  is retired because its stability property is now a structural consequence of serve-as-projection over
  the extend-only durable chain (`DC-NODE-13` + `DC-CONS-23` + `T-REC-05`); `DC-NODE-11` is preserved +
  strengthened (its `ci_script` repointed to `ci_check_served_chain_projection.sh`). The `serve_gate_*`
  tests retired with it. No rule was removed.
- **S2 CE-artifact-name drift recorded, not hidden.** The §8-named CE-5 gate
  `ci_check_forged_tip_recovery.sh` + CE-6 test `forge_two_clean_runs_byte_identical` were **not created
  literally**; `T-REC-05` is **test-enforced** (`ci_script = ""`) via the kill-recover fingerprint test,
  `DC-WAL-04` no-orphan via `warm_start_drops_orphan_block_above_wal_tail`. The invariants are enforced;
  the CE *artifact names* drifted. Surfaced in the §"Close-in-progress working tree" close record.
- **Sibling-doc coherence — EXPECTED catch-up, not staleness.** At HEAD `4e358e92` CODEMAP / SEAMS /
  TRACEABILITY are still pinned at the prior G-K…G-R catch-up close (328 rules / 134 CI / 458 types; none
  mention the new module or the five new rules). **This HEAD_DELTAS is the first of the four docs to
  advance to the N-U HEAD** — the matching CODEMAP/SEAMS/TRACEABILITY refresh is part of this same close
  pass and must bring them to **333 rules / 135 CI checks** (+ the new module in CODEMAP §RED, the five
  new rules + two strengthenings in TRACEABILITY, the retired gate dropped). A sibling reading 328/134
  at this instant is mid-close, not silently stale.
- **Close-in-progress working tree.** This regen ran *before* the close commit: the cluster.md §13 close
  record + the S2/S3 status flips are staged in the working tree (docs-only; see the dedicated section).
  The untracked `.mithril-scratch/` is operator scratch, ignored.
