# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `6363683e` (AE.F receive idempotency — survive the post-adoption echo, 2026-06-07 13:40)
> HEAD: `f87d0056` (PHASE4-N-AF S1 — enforce DC-NODE-18 core (extend-after-cert) + 2 live-surfaced loop fixes, 2026-06-07 23:59)
> Span: **the PHASE4-N-AE.F close grounding refresh + the OQ-1 / DC-NODE-17 investigation + the PHASE4-N-AF cluster (DC-NODE-18, single-producer extend-own-durable-spine)** — a docs-heavy investigation arc terminating in **one** RED/GREEN impl slice (`AF.S1`, two commits), plus the AE.F-close grounding refresh (`d3f52e7c`) and a C2-guide doc (`1302417d`) folded into the span head.
> **8 commits** (no merges), **19 files changed, +2489 / −456 lines** — but the file/line totals are split: `d3f52e7c` (the AE.F-close grounding refresh) rewrote CODEMAP/SEAMS/TRACEABILITY/HEAD_DELTAS (the bulk of the doc churn), and the **AF.S1 impl** is the only production change — **three RED/GREEN files in `ade_node`**: `crates/ade_node/src/node_sync.rs` (+787, the GREEN forge-mode state machine + decision fn + the new types), `crates/ade_node/src/node_lifecycle.rs` (+199 / −23, the RED mode-aware ForgeTick gate + the 2 live-surfaced fixes), and `crates/ade_node/src/cli.rs` (+28, two new flags), plus the new gate `ci/ci_check_single_producer_extend_own_spine.sh` (+164) and the registry (+55).

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`6363683e`**, the
> PHASE4-N-AE.F idempotency fix (the prior HEAD_DELTAS HEAD) — and it is **valid**: `git rev-parse 6363683e`
> resolves and `git merge-base 6363683e HEAD == 6363683e` (it is a strict ancestor of HEAD; `6363683e`
> carries no tag). HEAD is **`f87d0056`** (the PHASE4-N-AF S1 close). The config baseline at the start of
> this regen was already `6363683e` (the previous close bumped it), so the window measures cleanly from the
> recorded baseline forward. The span has **three parts**: (1) the **PHASE4-N-AE.F close grounding refresh**
> `d3f52e7c` — the docs commit that *wrote the previous HEAD_DELTAS lead* and brought CODEMAP/SEAMS/
> TRACEABILITY/HEAD_DELTAS current at `6363683e` (it is the baseline's own docs commit, so it contributes
> doc churn but **zero** production code); plus the **C2-guide** doc `1302417d` (records the CE-A5 manifest +
> adds the §7b robustness ladder); (2) the **OQ-1 / DC-NODE-17 investigation** (`bd1a7a73` invariants sketch
> + DC-NODE-17 declared → `dadf4743` the OQ-1 result that **live-disproved** DC-NODE-17-as-fix) — docs +
> one declared registry rule, **zero code**; and (3) the **PHASE4-N-AF cluster** (cluster/slice docs
> `c58575ec` + `b7b1bb52` → impl `f746084f` → close `f87d0056`) — **DC-NODE-18, single-producer
> extend-own-durable-spine.** The closer bumps `head_deltas_baseline` `6363683e → f87d0056` after this regen
> so the next cluster measures from here.

This window is **led by PHASE4-N-AF — single-producer extend-own-durable-spine (`DC-NODE-18`).** It is the
**rung-1 Finding B fix**, and the headline is an **honest boundary** that must be read in full:

> **PHASE4-N-AF enforced DC-NODE-18: Ade extended an explicitly adopted single-producer durable spine
> WITHOUT relay echo (live-proven, run c2t7 — block 11 adopted → RED adoption certificate → promote →
> block 12 forged via extend, no echo → adopted). It did NOT prove sustained >k, follow-link continuation,
> relay ImmutableDB settlement, epoch transition, or rung-1 completion. Those are deferred to DC-NODE-19 /
> follow-up rung-1 liveness work.**

The arc to that result is itself load-bearing — **DC-NODE-17 was declared and then live-disproved as the
fix**, which is why the cluster lands DC-NODE-18 and not DC-NODE-17:

- **OQ-1 / `DC-NODE-17` (declared, then reclassified to safety/observation-only).** A recovered/following
  `--mode node` Ade forges exactly **one** block per recover, then stalls: the AE.A/`DC-NODE-15` gate
  (forge only when `durable_servable_tip == followed_peer_tip`) waits for the relay to **re-announce** Ade's
  own just-adopted block back over the follow link. `DC-NODE-17` was **declared** (`bd1a7a73`) as the
  candidate fix: *let `followed_peer_tip` advance on the self-adoption echo (the relay re-announcing Ade's
  own block).* **OQ-1 then live-disproved it** (`dadf4743`, run **c2t4**, instrumented + reverted): the real
  `cardano-node 11.0.1` relay **genuinely adopts** Ade's forged block (`AddedToCurrentChain blockNo=12`) but
  **does NOT re-announce it** back over Ade's follow link (`followed_peer_tip` stayed at block 11; no
  `RollForward(12)`; the link then EOF'd). The pump **does** emit `TipUpdate` on every `RollForward`
  (verified live for block 11) — there is simply **no `RollForward(12)`.** So observing the echo cannot
  un-stick the loop. `DC-NODE-17` is therefore **NOT the sustained-forge fix**; it is **RETAINED as a
  SAFETY / OBSERVATION invariant ONLY** (if the peer advertises a tip, the RED signal must reflect it; never
  local inference). It lands `declared` (`introduced_in = "TBD"`), no gate, no test.
- **`DC-NODE-18` (NEW, enforced — the actual fix).** A real sole producer does **not** learn its own tip
  from a relay echo; it **extends the chain it is building.** `DC-NODE-18` makes the ForgeTick gate
  **mode-aware** via an explicit `ForgeMode` enum (GREEN transition fn, RED loop state):
  `InitialCatchupRequired → CaughtUpToPeerTip → FirstOwnBlockServed → SingleProducerExtendOwnDurableSpine`.
  Promotion into the extend state fires **only** on an explicit **RED venue-adoption certificate**
  (operator/harness-supplied evidence that the relay adopted Ade's own tip), **matched by chain-point
  identity (hash + block_no), NEVER inferred from self-admit.** In the extend state each ForgeTick forges on
  the durable spine head **without** the `followed == durable` requirement, behind a **fail-closed
  single-producer fence** (`ForgeRefused::SingleProducerFenceViolation`). It is a gate-**applicability**
  refinement, **NOT** a fork-choice weakening — `DC-CONS-03` stays the sole follow/fork authority, the
  certificate is **admissibility-only** (never persisted / replay-visible), and a non-single-producer venue
  takes the **verbatim prior `DC-NODE-15` path** (default off ⇒ fail-closed). New gate
  `ci/ci_check_single_producer_extend_own_spine.sh`.

**Two live-surfaced loop fixes** landed in the close commit `f87d0056` — **both missed by the hermetic
suite AND the IDD/security reviews** (the live gate's value):

1. **`not_leader`-advanced-mode.** The post-forge mode advancement keyed off the loop's `forged` flag,
   which is set on a `not_leader` tick (the forge ran, VRF said not-elected, **nothing was admitted**) —
   wrongly promoting the mode before any real block. **Fix:** capture `let admitted = handoff.is_some()` and
   advance only on an actual admit via the new pure `forge_mode_after_admit(mode, admitted, own_tip,
   parent_peer_tip)` (regression test `forge_mode_after_admit_only_advances_on_real_admit`).
2. **cert-match-too-strict-on-slot.** The promotion guard used full `TipPoint` equality **including slot**
   (`&c.adopted_tip == own_tip`); the relay-reported certificate slot need not byte-equal the served-tip
   slot. **Fix:** match on **chain-point identity** — `c.adopted_tip.hash == own_tip.hash &&
   c.adopted_tip.block_no == own_tip.block_no` — consistent with the catch-up gate's documented
   slot-ignoring equality.

**+0 BLUE canonical type** (the span touches **no** BLUE `core_paths` file). **No `RO-LIVE` rule flipped**
this span.

## 0. Headline

| Count | Baseline (`6363683e`) | HEAD (`f87d0056`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 142 | **143** | **+1** — **one NEW gate** (`--diff-filter=A` over `ci/`): `ci_check_single_producer_extend_own_spine.sh` (AF.S1). **No gate removed** (`--diff-filter=D` over `ci/` empty), **no gate modified** (`--diff-filter=M` over `ci/` empty). |
| Registry rules (`docs/ade-invariant-registry.toml`) | 341 | **343** | **+2** — two NEW rules: **`DC-NODE-17`** (declared) + **`DC-NODE-18`** (enforced). **Zero removed** (`diff` of the sorted `id =` lists shows exactly the two additions `DC-NODE-17` / `DC-NODE-18` and no removal). |
| Registry status (enforced / partial / declared) | 209 / 20 / 112 | **210 / 20 / 113** | **+1 enforced** (`DC-NODE-18`) **+1 declared** (`DC-NODE-17`). Partial count unchanged. |
| Registry strengthenings | — | **0** | No `strengthened_in` append this span — both new rules carry `strengthened_in = []`; they **cross-reference** existing rules (`DC-NODE-15` / `DC-NODE-16` / `DC-CONS-03` / `DC-NODE-10` / `DC-CONS-24` / `T-REC-05` / `DC-WAL-02` / `DC-NODE-12`, preserved, not strengthened). No rule weakened. |
| BLUE canonical types | 458 | **458** | **0** — **BLUE-untouched.** The span touches **no** BLUE `core_paths` file (`git diff 6363683e..HEAD` over the BLUE trees is empty). The 5 new `pub enum`/`struct` (`ForgeMode`, `VenueRole`, `VenueAdoptionCertificate`, `SingleProducerFenceReason`, `SingleProducerForgeDecision`) all live in the **GREEN** `ade_node::node_sync` — **not** in `core_paths`. |
| Grounding docs (CODEMAP / SEAMS / TRACEABILITY) | refreshed for the **AE.F** close at `6363683e` in `d3f52e7c` (the baseline's docs commit, span head) — 458 types / 142 CI / 341 rules | **NOT regenerated this close** (deferred); they carry **DC-NODE-16** but **not** `DC-NODE-17` / `DC-NODE-18` | The three sibling docs were regenerated to `6363683e` by `d3f52e7c`; the N-AF cluster commits touched **none** of them (`git log 6363683e..HEAD -- docs/ade-{CODEMAP,SEAMS,TRACEABILITY}.md` shows only `d3f52e7c`). DC-NODE-18 / DC-NODE-17 mentions in all three = **0**; the gate `ci_check_single_producer_extend_own_spine.sh` is referenced in **0**. **The registry holds `DC-NODE-17` + `DC-NODE-18` + the gate binding authoritatively at HEAD (343 rules).** See the grounding-doc deferral note below. |

> **Grounding-doc deferral note (load-bearing).** **CODEMAP/SEAMS refresh deferred this close: PHASE4-N-AF
> introduced no new module, no BLUE seam, and no new authoritative persistence surface. The next cluster
> refresh must include baseline f87d0056.** The same applies to TRACEABILITY (the `DC-NODE-18` ↔
> `ci_check_single_producer_extend_own_spine.sh` row refreshes on the next regen) — the registry is
> authoritative in the interim (343 rules; `DC-NODE-18.ci_script = "ci/ci_check_single_producer_extend_own_spine.sh"`,
> `tests = [forge_mode_transitions_are_total_and_deterministic, …]`). The N-AF additions are **GREEN/RED in
> `ade_node`** — the host modules (`ade_node::node_sync` GREEN, `ade_node::node_lifecycle` RED,
> `ade_node::cli` RED) are already in CODEMAP; AF.S1 adds **no module, no BLUE seam, no new persistence
> surface** (the certificate is admissibility-only, never persisted), so the deferral is structurally safe.

This is a **single-slice cluster lead** preceded by a docs-only investigation arc. The slice↔rule↔gate map:

| Slice | Rule(s) | Gate | What shipped |
|---|---|---|---|
| **AF.S1** (`f746084f` impl + `f87d0056` close) | **`DC-NODE-18`** (NEW, enforced) | **`ci_check_single_producer_extend_own_spine.sh`** (NEW) | Single-producer extend-own-durable-spine. A mode-aware ForgeTick gate (`ForgeMode` 4-state enum, GREEN transition fn) lets a declared single-producer venue forge on Ade's own durable spine **without** a relay echo, **after** an explicit chain-point-identity-matched RED venue-adoption certificate. Fail-closed `SingleProducerFenceViolation`; certificate admissibility-only (never persisted); `DC-CONS-03` untouched; default (non-single-producer) venue takes the verbatim prior `DC-NODE-15` path. **No BLUE change.** |
| — (investigation) | **`DC-NODE-17`** (NEW, **declared**) | — | Declared as the OQ-1 candidate fix (let `followed_peer_tip` advance on the self-adoption echo), then **live-disproved** by OQ-1 (the relay does NOT re-announce Ade's own block) and **retained as a safety/observation invariant only** — no gate, no test, `introduced_in = "TBD"`. |

The per-commit shape:

| Commit | Kind | What it did | Code / CI / registry effect |
|--------|------|-------------|-----------------------------|
| `d3f52e7c` | docs (AE.F close refresh) | Grounding-doc refresh for PHASE4-N-AE.F + restore ade_testkit CODEMAP entry | **0 code / 0 CI**; regenerated CODEMAP/SEAMS/TRACEABILITY/HEAD_DELTAS to `6363683e` (458 types / 142 CI / 341 rules); **0 registry rule added/removed** (registry already at 341 from the AE.F impl) |
| `1302417d` | docs (c2-guide) | Record CE-A5 manifest + add the §7b robustness ladder (phased de-risking) | **0 code / 0 CI / 0 registry**; `docs/active/c2-preprod-tip-guide.md` (+120) |
| `bd1a7a73` | docs (invariants sketch) | Sustained-single-producer-forge invariants sketch + `DC-NODE-17` (declared) from rung-1 | **0 code / 0 CI**; `docs/planning/sustained-single-producer-forge-invariants.md` (+174); registry: **`DC-NODE-17` declared** |
| `dadf4743` | docs (OQ-1 result) | OQ-1 result — relay doesn't re-announce Ade's own block; `DC-NODE-17` not the fix | **0 code / 0 CI**; updates the `DC-NODE-17` `evidence_notes` (live-disproved, retained safety/observation-only) + the invariants doc |
| `c58575ec` | docs (invariants) | `DC-NODE-18` invariant — single-producer extend-own-durable-spine (declared) | **0 code / 0 CI**; `docs/planning/single-producer-extend-own-spine-invariants.md` (+104); registry: **`DC-NODE-18` declared** |
| `b7b1bb52` | docs (cluster + slice doc) | PHASE4-N-AF slice + cluster doc — single-producer extend-own-durable-spine | **0 code / 0 CI / 0 registry**; `docs/clusters/PHASE4-N-AF/{cluster,S1-…}.md` (+197) |
| `f746084f` | feat (AF.S1 impl) | PHASE4-N-AF S1 — single-producer extend-own-durable-spine forge mode | **GREEN+RED code** (`node_sync.rs` GREEN state machine + decision fn + 5 new types; `node_lifecycle.rs` RED mode-aware gate + `read_adoption_cert` + `declare_single_producer_venue`; `cli.rs` two flags); **+1 CI** (`ci_check_single_producer_extend_own_spine.sh`) |
| `f87d0056` | fix (AF.S1 close) | PHASE4-N-AF S1 — enforce `DC-NODE-18` core (extend-after-cert) + 2 live-surfaced loop fixes | **RED+GREEN code** (the 2 live fixes: `forge_mode_after_admit` advance-only-on-admit in `node_lifecycle.rs`; chain-point-identity cert match in `node_sync.rs`); registry: **`DC-NODE-18` declared → enforced** (341 → 343 with the two new IDs) |

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `f87d0056` | fix | PHASE4-N-AF S1: enforce DC-NODE-18 core (extend-after-cert) + 2 live-surfaced loop fixes |
| `f746084f` | feat | PHASE4-N-AF S1: single-producer extend-own-durable-spine forge mode (DC-NODE-18) |
| `b7b1bb52` | docs | PHASE4-N-AF slice + cluster doc — single-producer extend-own-durable-spine (DC-NODE-18) |
| `c58575ec` | docs | DC-NODE-18 invariant — single-producer extend-own-durable-spine (declared) |
| `dadf4743` | docs | OQ-1 result — relay doesn't re-announce Ade's own block; DC-NODE-17 not the fix |
| `bd1a7a73` | docs | sustained-single-producer-forge invariants sketch + DC-NODE-17 (declared) from rung-1 |
| `1302417d` | docs | c2-guide: record CE-A5 manifest + add the §7b robustness ladder (phased de-risking) |
| `d3f52e7c` | docs | grounding-doc refresh for PHASE4-N-AE.F + restore ade_testkit CODEMAP entry |

No merge commits in the span. **8 commits, zero unclassified.** Six carry an explicit `docs(...)` / `docs:`
conventional-commits prefix (the AE.F-close refresh, the C2-guide doc, the OQ-1/DC-NODE-17/DC-NODE-18
investigation, and the N-AF cluster+slice docs). The two **AF.S1 impl commits** (`f746084f`, `f87d0056`)
do **not** carry a `feat:`/`fix:` prefix in their subject (they begin `PHASE4-N-AF S1: …`), but their diff
scope is unambiguous production code (`.rs` + `.sh` + registry) — classified **feat** (the new state
machine + mode-aware gate) and **fix** (the two live-surfaced loop fixes), respectively, per diff scope.
All N-AF work landed 2026-06-07.

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty
> trailer requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that
> is an Ade-local override of the global no-AI-attribution rule and applies to **commit messages
> only**. It does not affect this doc's content.

## 2. New Modules

**None.** `git diff --diff-filter=A --name-only 6363683e..HEAD -- '*.rs'` shows **no new `.rs` source file**
(not even a test file), no new crate, no new `Cargo.toml`, no new workspace. AF.S1 is **modification only** —
it adds the GREEN forge-mode state machine + decision fn + five new types **inside the existing** module
`crates/ade_node/src/node_sync.rs`, the RED mode-aware ForgeTick gate **inside the existing**
`crates/ade_node/src/node_lifecycle.rs`, and two CLI flags **inside the existing** `crates/ade_node/src/cli.rs`.
The only added files this span are **one CI gate** (`ci/ci_check_single_producer_extend_own_spine.sh`, §5),
the N-AF **cluster + slice docs** (`docs/clusters/PHASE4-N-AF/{cluster.md, S1-single-producer-extend-own-spine.md}`),
two **planning/invariants docs** (`docs/planning/{sustained-single-producer-forge-invariants.md,
single-producer-extend-own-spine-invariants.md}`), and the **CE-AF-6a evidence pair**
(`docs/evidence/phase4-n-af-extend-own-spine.{md,jsonl}`).

> **Cross-reference (CODEMAP/SEAMS) — no new surface, no new module, no new BLUE seam.** AF.S1 adds **no
> module and no BLUE seam** — the new types are **GREEN** (`ade_node::node_sync`) and the new behavior is a
> **RED** mode-aware gate (`ade_node::node_lifecycle`). All three host modules are already in CODEMAP. The
> certificate is **admissibility-only** (never persisted / replay-visible — `ci_check_single_producer_extend_own_spine.sh`
> clause (c) enforces no cert token co-occurs with a persistence verb), so AF.S1 adds **no new authoritative
> persistence surface.** CODEMAP/SEAMS need **no new module/type/TCB/seam entry** for the *module inventory*;
> they pick up the `DC-NODE-18` rule row + the five GREEN forge-mode types on their next regen, with the
> registry holding it authoritatively in the interim (the `code_locus` field names the exact sites).

## 3. Modules Modified

Three modules changed this span — **all in `ade_node`** (GREEN + RED), **+0 BLUE canonical type**:

| Module | Color / scope | Key changes |
|--------|---------------|-------------|
| `ade_node::node_sync` (`crates/ade_node/src/node_sync.rs`) | **GREEN** classifier, +787 / 0 | **AF.S1 (`f746084f` + `f87d0056`):** the GREEN single-producer forge-mode machinery. Five new types: `pub enum ForgeMode` (the closed 4-state machine `InitialCatchupRequired → CaughtUpToPeerTip → FirstOwnBlockServed → SingleProducerExtendOwnDurableSpine`, **no booleans**), `pub enum VenueRole`, `pub struct VenueAdoptionCertificate` (the RED venue-adoption certificate carrier), `pub enum SingleProducerFenceReason`, `pub enum SingleProducerForgeDecision`. The pure/total transition fns `forge_mode_on_caughtup` / `_on_first_own_block_served` / `_on_extend`, the post-forge `forge_mode_after_admit` (advances ONLY on an actual admit — **live-fix #1**), the decision fn `single_producer_forge_decision` (promotion gated on an explicit certificate matched by **chain-point identity** hash + block_no — **live-fix #2**; `AwaitAdoptionCertificate` without a cert — never self-admit-inferred), and `ForgeRefused::SingleProducerFenceViolation { reason, durable_tip, followed_peer_tip, observed_peer_tip, venue_role }`. **No BLUE type; the GREEN decision never references a chain selector (`DC-CONS-03` untouched).** Six unit tests (`forge_mode_transitions_are_total_and_deterministic`, `extend_own_spine_promotion_requires_adoption_certificate`, `extend_own_spine_forges_on_durable_tip_without_followed_equality`, `single_producer_fence_fails_closed`, `extend_own_spine_two_runs_byte_identical`, `forge_mode_after_admit_only_advances_on_real_admit`). |
| `ade_node::node_lifecycle` (`crates/ade_node/src/node_lifecycle.rs`) | **RED** loop, +199 / −23 | **AF.S1 (`f746084f` + `f87d0056`):** the **mode-aware ForgeTick gate** in `run_relay_loop_with_sched` — behind `VenueRole::SingleProducer` it calls `single_producer_forge_decision`; the **default `VenueRole::Unknown` path is the verbatim prior `DC-NODE-15` gate** (`dc_node_15_refusal` / `forge_followed_tip_admission`). Plus `read_adoption_cert`, `declare_single_producer_venue`, and `ForgeActivation.{forge_mode, venue_role, adoption_cert_path}`. **Both live-surfaced fixes land here (`f87d0056`):** (1) `let admitted = handoff.is_some()` + routing the post-forge transition through `forge_mode_after_admit(..., admitted, ...)` so a `not_leader`/no-op tick (`forged = true`, nothing admitted) no longer advances the mode; (2) the cert-match relaxation flows through the GREEN decision fn (chain-point identity). **No new type in this module; no BLUE change.** |
| `ade_node::cli` (`crates/ade_node/src/cli.rs`) | **RED**, +28 / 0 | **AF.S1 (`f746084f`):** two new flags — `--single-producer-venue` (declare an explicitly single-producer venue, default `false` ⇒ the forge stays pure `DC-NODE-15`) and `--adoption-cert-path PATH` (the RED venue-adoption-certificate file, admissibility-only, only meaningful with `--single-producer-venue`). Parsed into `Cli.{single_producer_venue, adoption_cert_path}`. **No new type; no BLUE change.** |

> **No BLUE change this span (load-bearing).** `git diff 6363683e..HEAD` over the BLUE `core_paths` trees
> is **empty** — AF.S1 touches **no** BLUE file. The fix is deliberately **GREEN (the pure forge-mode state
> machine + decision fn) + RED (the loop gate + CLI)**: the five new types are all in `ade_node::node_sync`
> (GREEN, **not** in `core_paths`), and the certificate is admissibility-only with **no new BLUE reducer
> input**. The BLUE canonical-type count is **458 → 458**. The header / body authorities, the KES verifier,
> forge eligibility, the closed wire grammar, the `pump_block` durable-admit chokepoint (`DC-NODE-16`
> idempotency), and chain selection (`DC-CONS-03`) are all **unchanged** — AF.S1 only refines *when* the
> forge gate fires for a declared single-producer venue. **Two test files** were touched additively
> (`crates/ade_node/tests/phase4_n_ae_recover_serve_continuity_diag.rs` +3, `crates/ade_node/tests/wire_only_loopback.rs`
> +2) — test-only, no production-code change.

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace `Cargo.toml`, and
**no `Cargo.toml` changed in this window** (`git diff --name-only 6363683e..HEAD -- '**/Cargo.toml'
'Cargo.toml'` is empty). No `#[cfg(feature = …)]` gate was introduced. The two new constructs are **CLI
flags** (`--single-producer-venue`, `--adoption-cert-path`), parsed into `Cli` — **not** Cargo feature
flags, env vars, or compile-time `cfg`. **Coupling note:** `--adoption-cert-path` is only meaningful with
`--single-producer-venue`; the default (`--single-producer-venue` absent) leaves the forge on the verbatim
prior `DC-NODE-15` path, and the `single_producer_forge_decision` fence **fails closed**
(`VenueNotDeclaredSingleProducer`) if the extend machinery is reached without an explicitly declared
single-producer venue.

## 5. CI Checks (142 → 143; +1 new gate, 0 gates modified, 0 gates removed)

One new gate this span; no gate modified, no gate removed. `git diff --diff-filter=A 6363683e..HEAD -- ci/`
lists exactly the one gate below; `--diff-filter=D` and `--diff-filter=M` over `ci/` are both **empty**.

### PHASE4-N-AF gate (`f746084f`)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_single_producer_extend_own_spine.sh` | **New** | PHASE4-N-AF (`f746084f`); `DC-NODE-18` | Static-grep over the **production** bodies of `node_sync.rs` + `node_lifecycle.rs` (`#[cfg(test)]` stripped). Asserts: **(a)** the forge mode is an explicit `ForgeMode` enum with the four named states — **never a boolean** (no `forge_mode: bool`); **(b)** promotion into `SingleProducerExtendOwnDurableSpine` requires an explicit certificate (`single_producer_forge_decision` consults `cert` / `adopted_tip`, has a `Promote` path AND an `AwaitAdoptionCertificate` no-cert path) — **never self-admit-inferred**; **(c)** the certificate is **admissibility-only** — no cert token (`VenueAdoptionCertificate` / `adoption_cert` / `read_adoption_cert`) co-occurs with a persistence verb (`wal.` / `append_` / `pump_block` / `WalEntry` / …); **(d)** the fence **fails closed** to a typed structured `SingleProducerFenceViolation { reason, durable_tip, followed_peer_tip, observed_peer_tip, venue_role }`, checking the venue role first (`VenueNotDeclaredSingleProducer`); **(e)** the mode/decision **never references a chain selector** (`select_best_chain` / `chain_selector` / `fork_choice`) — `DC-CONS-03` untouched; **(f)** the loop is mode-aware (`run_relay_loop_with_sched` calls `single_producer_forge_decision` behind `VenueRole::SingleProducer`) **AND preserves the pure `DC-NODE-15` default** (`dc_node_15_refusal` / `forge_followed_tip_admission`). |

> **Cross-reference (TRACEABILITY) — new binding, deferred to next regen, no removal.** The new rule↔gate
> binding (`DC-NODE-18` ↔ `ci_check_single_producer_extend_own_spine.sh`) is recorded **authoritatively in
> the registry** at HEAD (`DC-NODE-18.ci_script = "ci/ci_check_single_producer_extend_own_spine.sh"`,
> `tests = [forge_mode_transitions_are_total_and_deterministic, …]`). TRACEABILITY was regenerated for the
> **AE.F** close at `6363683e` (`d3f52e7c`); it currently does **not** carry the `DC-NODE-18` row
> (mentions = 0) — it picks it up on its **next regen** (the deferral note above), with the registry
> authoritative in the interim. **No rule↔gate binding was removed.** This gate is **not** an orphan — it
> enforces exactly `DC-NODE-18`. `DC-NODE-17` is **declared** with no gate (`ci_script = ""`) — correctly,
> since it is a retained safety/observation invariant, not a mechanically-enforced one this span.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. **No BLUE canonical type was
added or removed in this window** — the BLUE count is unchanged (**458 → 458**; `git diff 6363683e..HEAD`
over the BLUE `core_paths` trees is empty). AF.S1 adds **five** new `pub enum`/`struct` (`ForgeMode`,
`VenueRole`, `VenueAdoptionCertificate`, `SingleProducerFenceReason`, `SingleProducerForgeDecision`) — **all
in the GREEN `ade_node::node_sync`**, **none** in `core_paths`, so they are **not** BLUE canonical types. No
`Cargo.toml` changed.

## 7. Normative / Invariant Rule Delta (341 → 343; +2 rules, 0 strengthenings, zero removals)

**Two rule IDs were added; zero removed** (341 → 343; `diff` of the sorted `id =` lists shows exactly the
two additions `DC-NODE-17` / `DC-NODE-18` and no removal). The status tally moves **209 → 210 enforced**
(`DC-NODE-18`) and **112 → 113 declared** (`DC-NODE-17`); the 20 partial unchanged.

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs, the
CE-73 reclassification, and `CLAUDE.md` — were **not** changed this span: `git diff --name-only
6363683e..HEAD` over those paths is empty. The rule-count delta is entirely the invariant-registry change
below.)*

**New rules (`+2`):**

| Rule | Family / Tier · Status | Statement (summary) |
|------|------------------------|---------------------|
| `DC-NODE-18` | DC / `derived` · **enforced** (`introduced_in = "PHASE4-N-AF"`) | **Successor extension after an explicit adoption certificate (single-producer, single successor).** After initial peer catch-up (`DC-NODE-15`) **and** explicit relay-adoption evidence for Ade's first successor — an operator/harness-supplied **RED venue-adoption certificate** naming the adopted own tip, matched by **chain-point identity (hash + block_no), NEVER inferred from self-admit** — single-producer Ade may forge the next successor on its **OWN durable adopted spine WITHOUT** a relay echo of the adopted block. A successor is adoptable **by induction** (it extends an already-adopted parent; the relay is a pure follower). Valid **ONLY** while the venue is explicitly single-producer; **fails closed** otherwise. A gate-**applicability** refinement, **NOT** a fork-choice weakening — `DC-CONS-03` stays the sole follow/fork authority; the followed-peer-tip signal still may not select/replace/reorder/prefer chains. **SCOPE BOUNDARY (live-proven core only):** asserts ONLY the successor-extension-after-certificate authority; does **NOT** assert sustained production past k, relay ImmutableDB settlement, follow-link liveness, or forge-loop continuation after a follow-link EOF — those are deferred to `DC-NODE-19`. `ci_script = ci/ci_check_single_producer_extend_own_spine.sh`; `cross_ref = [DC-NODE-15, DC-NODE-17, DC-CONS-03, DC-NODE-10, DC-CONS-24, DC-NODE-12, T-REC-05]`. |
| `DC-NODE-17` | DC / `derived` · **declared** (`introduced_in = "TBD"`) | **`followed_peer_tip` advances ONLY from a real observed peer ChainSync advertisement** (including the self-adoption echo case), updating forge **admissibility only** (`DC-NODE-15`) — never mutating the durable tip / WAL / ledger (`DC-NODE-16` idempotency preserved; replay-neutral) and never reaching chain selection (`DC-CONS-03`). **Declared** as the OQ-1 candidate fix, then **live-disproved** by OQ-1 (run c2t4: the relay does **NOT** re-announce Ade's own adopted block — `followed_peer_tip` stayed at block 11, no `RollForward(12)`, then EOF). **Retained as a SAFETY / OBSERVATION invariant ONLY** (if the peer advertises a tip, the RED signal must reflect it; never local inference); **NOT** the sustained-forge fix (that is `DC-NODE-18`). No gate, no test. `cross_ref = [DC-NODE-15, DC-NODE-16, DC-CONS-03, DC-NODE-10, DC-CONS-24, T-REC-05, DC-WAL-02]`. |

**Strengthenings (`strengthened_in +=`) — 0:** none this span. Both new rules carry `strengthened_in = []`;
they **cross-reference** existing rules (`DC-NODE-15`, `DC-NODE-16`, `DC-CONS-03`, `DC-NODE-10`,
`DC-CONS-24`, `T-REC-05`, `DC-WAL-02`, `DC-NODE-12`) — all **preserved** (the certificate appends nothing to
the WAL and never reaches chain selection, so idempotency / replay / fork-choice are unaffected) — but none
is a `strengthened_in` append. No rule was weakened.

> **The honest boundary — what AF.S1 closes (and what it does not).** AF.S1 enforced **`DC-NODE-18`**:
> **Ade extended an explicitly adopted single-producer durable spine WITHOUT relay echo** (live-proven,
> **run c2t7**, C2-LOCAL `cardano-testnet` magic 42 — block 11 forged @ slot 327 → the non-producing
> Haskell relay `AddedToCurrentChain` block 11 → the operator/harness wrote the RED adoption certificate
> `11 327 7e67cd0d…` → Ade promoted on **chain-point identity** → block 12 forged @ slot 406 via **extend**,
> **no echo** of block 11 → the relay `AddedToCurrentChain` block 12). It did **NOT** prove **sustained >k,
> follow-link continuation, relay ImmutableDB settlement, epoch transition, or rung-1 completion** — those
> are **deferred to `DC-NODE-19` / follow-up rung-1 liveness work** (CE-AF-6b). The c2t7 run stopped at 2
> blocks because the Ade→relay follow link **EOF'd** (relay idle timeout, no keep-alive) and the forge loop
> currently treats the follow source as a lifecycle authority — a loop-lifecycle obligation scoped to
> `DC-NODE-19`, **not** a `DC-NODE-18` authority failure. **No `RO-LIVE` rule flipped** this span; `DC-NODE-18`
> is recorded as `enforced` for the **scoped core invariant only**, with hermetic enforcement (6 tests +
> the gate) backing the byte-identity / fail-closed / total-state-machine claims.

**No rule was removed (expected: 0).** The registry delta is **two new rules (`DC-NODE-18` enforced +
`DC-NODE-17` declared), zero `strengthened_in` appends, zero removals** — consistent with append-only
registry discipline. **Note on `DC-NODE-17`:** declaring a rule and then **reclassifying its role**
(candidate-fix → retained safety/observation-only) after a live disproof is a *strengthening of evidence,
not a weakening* — the ID is **retained**, the statement is **unchanged**, and its `evidence_notes` record
the OQ-1 disproof in full. No discipline violation.

## Working tree at HEAD `f87d0056`

Clean of tracked changes from this span — the AE.F-close refresh, the C2-guide doc, the OQ-1/DC-NODE-17
investigation, and the N-AF cluster (invariants → cluster/slice docs → impl → close) are all committed.
`git status --short` shows only an untracked `.mithril-scratch/` (operator scratch, ignored). **This regen
runs *after* all 8 span commits** (the AF.S1 close `f87d0056` is HEAD for this window); the registry records
`DC-NODE-17` + `DC-NODE-18` + the `DC-NODE-18` gate binding authoritatively at HEAD (343 rules). The
remaining close-pass actions are this HEAD_DELTAS and the baseline bump (`6363683e → f87d0056`).

> **Cluster-context note.** AF.S1 is the **single slice of PHASE4-N-AF** (the cluster + slice docs live at
> the active path `docs/clusters/PHASE4-N-AF/`). The AF.S1 close `f87d0056` is a `PHASE4-N-AF S1: …` commit,
> not a formal `chore: close` archive commit; whether N-AF is formally archived (moving
> `docs/clusters/PHASE4-N-AF/` → `docs/clusters/completed/PHASE4-N-AF/`) is a close-pass decision separate
> from this HEAD_DELTAS regen.

## Honest residual (window scope)

PHASE4-N-AF **closed the single-producer extend-after-adoption-certificate core** — Ade extended its own
durable adopted spine without a relay echo, live-proven (run c2t7). The honest boundary:

- **The headline boundary (verbatim).** PHASE4-N-AF enforced `DC-NODE-18`: Ade extended an explicitly
  adopted single-producer durable spine **WITHOUT relay echo** (live-proven, run c2t7 — block 11 adopted →
  RED adoption certificate → promote → block 12 forged via extend, no echo → adopted). It did **NOT** prove
  **sustained >k, follow-link continuation, relay ImmutableDB settlement, epoch transition, or rung-1
  completion.** Those are **deferred to `DC-NODE-19` / follow-up rung-1 liveness work.**
- **OQ-1 redirected the fix.** `DC-NODE-17` (let `followed_peer_tip` advance on the self-adoption echo) was
  **declared, then live-disproved** by OQ-1 (the relay does NOT re-announce Ade's own adopted block) — so
  it is **NOT** the stall fix; it is **retained as a safety/observation invariant only** (no gate, no test,
  `introduced_in = "TBD"`). The actual fix is `DC-NODE-18` (extend the chain Ade is building).
- **Two live-surfaced loop fixes — the live gate's value.** Both landed in the close commit `f87d0056` and
  were **missed by the hermetic suite AND the IDD/security reviews**: (1) the post-forge mode advancement
  fired on a `not_leader` tick (`forged = true`, nothing admitted) — fixed via `forge_mode_after_admit`
  advancing only on an actual admit; (2) the cert promotion used full `TipPoint` equality incl. slot —
  fixed to **chain-point identity** (hash + block_no). The hermetic suite was **extended** with
  `forge_mode_after_admit_only_advances_on_real_admit` to lock fix (1).
- **GREEN+RED only, +0 BLUE / +0 BLUE type / no new persistence surface.** The span touches **no** BLUE
  file; the fix is the GREEN forge-mode state machine + decision fn (`ade_node::node_sync`) + the RED
  mode-aware loop gate (`ade_node::node_lifecycle`) + two RED CLI flags (`ade_node::cli`). The certificate
  is **admissibility-only** (never persisted / replay-visible). BLUE canonical-type count **458 → 458**;
  `pump_block` durable admission (`DC-NODE-16` idempotency), ledger/chain_dep/WAL, and chain selection
  (`DC-CONS-03`) are all unchanged.
- **`DC-CONS-03` untouched; default venue fails safe.** The single-producer machinery is gated behind
  `VenueRole::SingleProducer`; the default `VenueRole::Unknown` path is the **verbatim prior `DC-NODE-15`
  gate**, and the fence **fails closed** (`VenueNotDeclaredSingleProducer`) if the extend machinery is
  reached without an explicitly declared single-producer venue. The gate
  `ci_check_single_producer_extend_own_spine.sh` fences no-boolean-mode + cert-required-promotion +
  cert-not-persisted + fail-closed-fence + no-chain-selector + mode-aware-loop-preserves-default.
- **AE.F-close grounding refresh + C2-guide are the span head (docs-only).** `d3f52e7c` (the AE.F-close
  grounding refresh) and `1302417d` (the C2-guide doc) are **docs/config only** (no `.rs` / `.sh`);
  `d3f52e7c` regenerated CODEMAP/SEAMS/TRACEABILITY/HEAD_DELTAS for the AE.F close at `6363683e` and
  contributes the bulk of the window's doc churn. The registry was already at 341 from the AE.F impl;
  these two commits added **no** rule.
- **CODEMAP/SEAMS/TRACEABILITY refresh deferred this close — next regen must include `f87d0056`.** Per the
  grounding-doc deferral note above: PHASE4-N-AF introduced **no new module, no BLUE seam, and no new
  authoritative persistence surface**, so the three sibling docs (last regenerated to `6363683e` by
  `d3f52e7c`) were **not** regenerated this close — they carry `DC-NODE-16` but not `DC-NODE-17` /
  `DC-NODE-18` (mentions = 0 in all three; the gate referenced in 0). **The next cluster refresh must
  include baseline `f87d0056`.** The registry holds `DC-NODE-17` + `DC-NODE-18` + the `DC-NODE-18` gate
  binding authoritatively at HEAD (343 rules) in the interim.

---

## Historical — PHASE4-N-AE.F post-CE-A5 echo-idempotency follow-up (`a76672b9 → 6363683e`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It was a
> **single-slice lead** narrating the `a76672b9 → 6363683e` span: the PHASE4-N-AE close grounding-doc
> refresh (`62811a4e`, the baseline's own docs commit, span head) followed by the **PHASE4-N-AE.F** slice
> (invariants sketch `d11bdbe8` → slice doc `8049dd43` → impl `6363683e`) — the **post-CE-A5
> echo-idempotency** follow-up. Counts here are the figures **at `6363683e`** (341 rules, 142 CI gates, 458
> canonical types); the current window measures **forward** from `6363683e`. The full §§0–7 narrative is
> recoverable from this doc's git history at `6363683e` / `d3f52e7c`.

> Baseline: `a76672b9` (AE.E chain-sync server FindIntersect cursor — CE-A5 manifest achieved, 2026-06-07 12:26)
> HEAD: `6363683e` (AE.F receive idempotency — survive the post-adoption echo, 2026-06-07 13:40)
> Span: **PHASE4-N-AE.F — the post-CE-A5 echo-idempotency follow-up** — one impl slice (`AE.F`) on the receive-side durable-admit chokepoint, plus the PHASE4-N-AE close grounding-doc refresh (`62811a4e`) folded into the span head — 4 commits, 13 files, +1401 / −473.

PHASE4-N-AE.F was the **post-CE-A5 echo** follow-up: after the real `cardano-node 11.0.1` relay **adopted**
Ade's forged block 17 (`AddedToCurrentChain`, the CE-A5 manifest closed in AE.E), the relay
**re-announced** that block back over Ade's follow link, and the BLUE header authority **correctly** rejected
`SlotBeforeLastApplied{last=421, attempted=421}` — which **terminated the run (exit 43) *after*
`AddedToCurrentChain`.** AE.F closed it in one slice:

- **AE.F — receive idempotency at the durable-admit chokepoint (`6363683e`; `DC-NODE-16` NEW, enforced).**
  `pump_block` (the **RED** durable-admit chokepoint in `ade_runtime::forward_sync::pump`), **immediately
  after `decode_block` and BEFORE the BLUE chokepoint reducer**, queries
  `db.get_block_by_hash(&decoded.block_hash)`; if `Some(stored)` **and** `stored.slot ==
  decoded.header_input.slot`, it returns **`Ok(None)`** — an **idempotent no-op** (no reducer step, no WAL
  append, no tip change). The skip is **hash-keyed** (never slot alone): a **different** block at/before the
  last-applied slot falls through to the **unchanged** BLUE authority and **fails closed**. New gate
  `ci/ci_check_receive_idempotency.sh`.

**N-AE.F headline (at `6363683e`):** Registry **340 → 341** (+1 enforced `DC-NODE-16`; 0 strengthenings; 0
removed; status 208 → 209 enforced). CI gates **141 → 142** (+1 `ci_check_receive_idempotency.sh`; 0
modified / 0 removed). **RED chokepoint only — BLUE canonical types 458 → 458** (no BLUE file touched). **No
`RO-LIVE` flip** — AE.F is a **continuous-run prerequisite**, not a new live claim; the CE-A5 manifest was
closed in AE.E and backs `DC-NODE-14` / `DC-PROTO-10`. All four grounding docs were regenerated to `6363683e`
in `d3f52e7c` (the close grounding refresh).

---

## Historical — PHASE4-N-AD durability proof + C2-LOCAL run + PHASE4-N-AE CE-A5 cluster (`25ddeebd → a76672b9`)

> Preserved in condensed form. A **multi-part lead** narrating the `25ddeebd → a76672b9` span: the
> PHASE4-N-AC grounding-doc-refresh tail (`25ddeebd`), the **test-only PHASE4-N-AD** tip-successor
> durability cluster, a **docs-only C2-LOCAL** preprod-tip / cardano-testnet venue guide-and-finding run,
> and the closing **CE-A5 cluster PHASE4-N-AE** — **Recover→Serve Continuity and Forge Admissibility**.
> Counts here are the figures **at `a76672b9`** (340 rules, 141 CI gates, 458 canonical types). The full
> §§0–7 narrative is recoverable from this doc's git history at `a76672b9` / `62811a4e`.

> Baseline: `25ddeebd` (grounding-doc refresh for PHASE4-N-AC close, 2026-06-06 11:48)
> HEAD: `a76672b9` (AE.E chain-sync server FindIntersect cursor — CE-A5 manifest achieved, 2026-06-07 12:26)
> Span: **the N-AC close-refresh tail + PHASE4-N-AD (test-only) + the C2-LOCAL guide/finding run + PHASE4-N-AE** — 19 commits, 24 files, +3635 / −129.

PHASE4-N-AE was the **CE-A5 cluster** that turned the recover→follow→forge→serve pipeline into a **proven
end-to-end live result**: a **real `cardano-node 11.0.1` relay `AddedToCurrentChain` an Ade-forged
successor block** (block 17 @ slot 421, issuerHash `a1ed4e04… == blake2b-224(pool1` cold VK`)`; relay
forging = 0) — the **CE-A5 manifest**. It closed across **four impl slices** (committed A → C → B → E):
AE.A forge-on-followed-tip admission (`DC-NODE-15` + `DC-CONS-24`); AE.C recover→follow WAL prior-fp
seeding (`DC-WAL-02` + `T-REC-05` strengthened); AE.B recovered/forge-parent FindIntersect-only
intersectability (`DC-NODE-14`); AE.E the chain-sync **server** FindIntersect-cursor fix (`DC-PROTO-10`
NEW — **the CE-A5 closer**). **N-AE-window headline (at `a76672b9`):** Registry **336 → 340** (+4 enforced;
9 strengthenings; 0 removed). CI gates **138 → 141** (+3 gates). **BLUE-additive, +0 canonical type** (458 →
458). **CE-A5 is recorded as `enforced`-backing evidence, NOT a `RO-LIVE` flip.**

---

## Historical — earlier windows (`c6e7fafe → 1d54abb4` and before)

> Preserved as pointers. The **PHASE4-N-AC cluster** (`c6e7fafe → 1d54abb4`, KES signing evolves the
> operator key to the current period before signing — `DC-CRYPTO-10`; 335 → 336 rules / 137 → 138 CI gates
> at `1d54abb4`; RED-only, 458 types unchanged); the **PHASE4-N-AB cluster** (`b0365df0 → c6e7fafe`,
> outbound mux segmentation — `CN-SESS-05`; 334 → 335 rules / 136 → 137 CI gates at `c6e7fafe`; GREEN-only);
> the **PHASE4-N-AA cluster** (`999199f8 → b0365df0`, bounded peer-driven serve range — `DC-SERVEMEM-01`;
> 333 → 334 rules / 135 → 136 CI gates at `b0365df0`; RED-only); the **PHASE4-N-U cluster CLOSE +
> gate-hygiene tail** (`4e358e92 → 999199f8`, 333 rules / 135 CI gates at `999199f8` — 11 gates repaired in
> place, 0 added/removed, 0 invariants weakened); the **PHASE4-N-U cluster** (`65954fa3 → 4e358e92`,
> forged-block durability — `DC-NODE-12`, `DC-CONS-23`, `DC-WAL-04`, `T-REC-05`, `DC-NODE-13`; one new RED
> module `served_chain_projection`; 328 → 333 rules); and the **G-K…G-R + C1 multi-cluster catch-up**
> (`550eec3a → 65954fa3`, eight clusters toward a live genesis-successor follower — 319 → 328 rules, 126 →
> 134 CI gates, the one BLUE canonical type `ArrayHead` 457 → 458). The full §§0–7 narrative for each is
> recoverable from this doc's git history at the respective HEADs.

---

## Generation notes

### Regen `6363683e → f87d0056` (PHASE4-N-AF single-producer extend-own-durable-spine — current lead)

- **Baseline valid; single-slice cluster lead preceded by a docs-only investigation arc.** Run against
  `6363683e` (the PHASE4-N-AE.F idempotency fix, the prior HEAD_DELTAS HEAD), which `git rev-parse` resolves
  and `git merge-base 6363683e HEAD` confirms is a strict ancestor of HEAD `f87d0056` (`6363683e` carries no
  tag). The start-of-regen config baseline was already `6363683e` (the previous close bumped it). The closer
  bumps `head_deltas_baseline` `6363683e → f87d0056` after this regen.
- **Counts are mechanical (git/grep/ls):** commit log + `--shortstat` over `6363683e..HEAD` (**8** commits,
  no merges / **19** files / **+2489 / −456** — the AE.F-close grounding refresh `d3f52e7c` dominates the
  doc churn; the AF.S1 impl is the only production change — GREEN `node_sync.rs` +787, RED `node_lifecycle.rs`
  +199 / −23, RED `cli.rs` +28); CI gate count via `git ls-tree -r --name-only <ref> ci/ | grep -c
  'ci_check_.*\.sh$'` at each ref (**142 → 143**; `--diff-filter=A` over `ci/` = the one new gate
  `ci_check_single_producer_extend_own_spine.sh`; `--diff-filter=D` and `--diff-filter=M` over `ci/` both
  **empty**); registry rule count via `grep -cE '^id = '` at each ref (**341 → 343**; `diff` of sorted `id =`
  lists shows the two additions `DC-NODE-17` / `DC-NODE-18`, zero removals); registry status via `grep -E
  '^status = ' | sort | uniq -c` (**209 → 210 enforced**, **112 → 113 declared**, 20 partial unchanged);
  strengthenings = **0** (the two `strengthened_in` diff lines are the new entries' own empty arrays — no
  append to any existing rule); BLUE canonical types via a `git diff 6363683e..HEAD` over the BLUE
  `core_paths` trees (**empty diff → 458 → 458**; the 5 new `pub enum`/`struct` are all in the GREEN
  `ade_node::node_sync`, not in `core_paths`).
- **GREEN+RED-only span — no BLUE file, +0 BLUE canonical type, no Cargo.toml change.** `git diff
  --name-status 6363683e..HEAD` shows the only production-code change is in `ade_node` —
  `crates/ade_node/src/{node_sync.rs, node_lifecycle.rs, cli.rs}` — and `git diff 6363683e..HEAD` over the
  BLUE trees is **empty**. No new `.rs` *source* file. `git diff --name-only … '**/Cargo.toml' 'Cargo.toml'`
  is empty (no feature-flag delta; the two new constructs are CLI flags, not Cargo features).
  **Classification note:** `node_sync.rs` is **GREEN** (the pure forge-mode state machine + decision fn — no
  I/O, total/deterministic), `node_lifecycle.rs` + `cli.rs` are **RED** (the loop gate + CLI). `ade_node` is
  neither a BLUE `core_paths` crate nor `ade_runtime` (the RED shell crate); per the project's TCB scoping
  the new types are non-BLUE.
- **Registry delta is +2 rules (one enforced, one declared), NOT a strengthening or removal.** `DC-NODE-18`
  is declared (`c58575ec`) then flipped declared → enforced in the AF.S1 close `f87d0056` (it cross-references
  `DC-NODE-15` / `DC-NODE-16` / `DC-CONS-03` / `DC-NODE-10` / `DC-CONS-24` / `DC-NODE-12` / `T-REC-05`, all
  **preserved**). `DC-NODE-17` is declared (`bd1a7a73`) and stays declared (live-disproved as the fix by
  OQ-1 in `dadf4743`, retained as safety/observation-only). The sorted-id `diff` confirms zero removals.
  `DC-NODE-18` carries a populated `ci_script` (`ci/ci_check_single_producer_extend_own_spine.sh`) + six
  `tests`; `DC-NODE-17` carries `ci_script = ""` + `tests = []` (correctly, a declared safety/observation
  invariant).
- **Span head `d3f52e7c` is the AE.F-close grounding refresh (the baseline's docs commit); `1302417d` is a
  C2-guide doc.** Both are **docs/config only** (`git show --name-only` has no `.rs` / `.sh`); `d3f52e7c`
  regenerated CODEMAP/SEAMS/TRACEABILITY/HEAD_DELTAS for the AE.F close at `6363683e` and contributes the
  bulk of the window's doc churn. The registry was already at 341 from the AE.F impl; neither added a rule.
- **The two live-surfaced loop fixes are in the close commit `f87d0056`, not the impl `f746084f`.** `git diff
  f746084f..f87d0056` over `node_lifecycle.rs` + `node_sync.rs` isolates them: (1) `let admitted =
  handoff.is_some()` + the new `forge_mode_after_admit(mode, admitted, own_tip, parent_peer_tip)` (advance
  only on a real admit, not on a `not_leader` tick); (2) the promotion guard `&c.adopted_tip == own_tip`
  (full `TipPoint` equality incl. slot) → chain-point identity `c.adopted_tip.hash == own_tip.hash &&
  c.adopted_tip.block_no == own_tip.block_no`. Both were **missed by the hermetic suite + the IDD/security
  reviews** and surfaced on the C2-LOCAL run c2t7 — recorded in `DC-NODE-18.evidence_notes` and
  `docs/evidence/phase4-n-af-extend-own-spine.md`.
- **No `RO-LIVE` flip; CE-AF-6a is the live-proven core, CE-AF-6b is deferred.** AF.S1 is recorded as
  `enforced` for the **scoped** core invariant (successor extension after an adoption certificate), backed by
  hermetic enforcement (6 tests + the gate) + the c2t7 live proof. It does **NOT** claim sustained >k,
  follow-link continuation, relay ImmutableDB settlement, epoch transition, or rung-1 completion — those are
  `CE-AF-6b`, deferred to `DC-NODE-19`. No `RO-LIVE` registry status changed this span.
- **Normative docs unchanged this span.** `git diff --name-only 6363683e..HEAD` over the configured
  `normative_docs` (CE-79 statement + addendum, the three contract docs, CE-73 reclassification, `CLAUDE.md`)
  is empty — the §7 delta is entirely the invariant-registry change.
- **§1 commit log verbatim from `git log --oneline --no-merges` (newest first).** The per-slice synthesis is
  in §0/§3. The two AF.S1 impl commits begin `PHASE4-N-AF S1: …` (no `feat:`/`fix:` prefix) but classify by
  diff scope as `feat` (impl) / `fix` (close + 2 live fixes); the other six carry `docs(...)` / `docs:`.
- **Doc-refresh state — CODEMAP/SEAMS/TRACEABILITY refresh DEFERRED this close.** `git log 6363683e..HEAD --
  docs/ade-{CODEMAP,SEAMS,TRACEABILITY}.md` shows the three sibling docs were last touched by `d3f52e7c`
  (the AE.F close at `6363683e`); the N-AF cluster commits touched **none** of them. `DC-NODE-18` /
  `DC-NODE-17` mentions in all three = **0**; `ci_check_single_producer_extend_own_spine.sh` referenced in
  **0**. Per the grounding-doc deferral note, the refresh is deferred because PHASE4-N-AF introduced **no new
  module, no BLUE seam, and no new authoritative persistence surface** — the **next cluster refresh must
  include baseline `f87d0056`.** The **registry holds `DC-NODE-17` + `DC-NODE-18` + the `DC-NODE-18` gate
  binding authoritatively at HEAD** (343 rules) in the interim.
- **Working tree clean.** This regen runs *after* all 8 span commits (the AF.S1 close `f87d0056` is HEAD for
  this window); `git status --short` shows only an untracked `.mithril-scratch/` (operator scratch,
  ignored). The remaining close-pass actions are this HEAD_DELTAS and the baseline bump
  `6363683e → f87d0056`.

### Regen `a76672b9 → 6363683e` (PHASE4-N-AE.F post-CE-A5 echo-idempotency follow-up — prior lead)

- **Single-slice lead** (N-AE close refresh span-head → AE.F), measured from `a76672b9` (the
  PHASE4-N-AE.E CE-A5 closer). **4** commits / **13** files / **+1401 / −473** (dominated by `62811a4e`, the
  N-AE close grounding refresh); CI gates **141 → 142** (+1 `ci_check_receive_idempotency.sh`; 0 modified /
  0 removed); registry **340 → 341** (+1 enforced `DC-NODE-16`; 0 strengthenings; 0 removed; status 208 →
  209 enforced); BLUE canonical types **458 → 458** (RED chokepoint only — no BLUE file touched). AE.F is a
  **continuous-run prerequisite**, not a `RO-LIVE` flip; the CE-A5 manifest was closed in AE.E (prior
  window) and backs `DC-NODE-14` / `DC-PROTO-10`. All four grounding docs were regenerated to `6363683e` in
  `d3f52e7c` (the close grounding refresh). Full notes recoverable from this doc's git history at `6363683e`
  / `d3f52e7c`.
