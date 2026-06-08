# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `f87d0056` (PHASE4-N-AF S1 — enforce DC-NODE-18 core (extend-after-cert) + 2 live-surfaced loop fixes, 2026-06-07 23:59)
> HEAD: `5858288e` (PHASE4-N-AH registry close — DC-NODE-20/21/22 enforced; N-AG superseded, 2026-06-08 20:54)
> Span: **the PHASE4-N-AF close tail + the PHASE4-N-AG cluster (single-producer loop-continuation-after-feed-EOF, `DC-NODE-19`; superseded-close) THEN the PHASE4-N-AH cluster (local selected durable chain forge-base authority `DC-NODE-20` + cert evidence-only `DC-NODE-21` + single-producer warm-start re-entry `DC-NODE-22`)** — two pivoting single-theme clusters that retired the N-AF operator-certificate forge mechanism and re-homed the sustained-past-k liveness proof onto a **cert-free local-tip** forge path, live-proven on **C2-LOCAL** (run-4).
> **32 commits** (no merges), **48 files changed, +5155 / −743 lines**. The production change is **RED/GREEN-only on the `ade_node` + `ade_runtime` shells** across **six** source files (`ade_node::{node_sync, run_loop_planner, node_lifecycle, cli, live_log::sched_event, live_log::sched_writer}` + `ade_runtime::bootstrap`) — **ZERO BLUE change, ZERO new BLUE module, ZERO new canonical type** (`git diff f87d0056..HEAD` over the BLUE `core_paths` trees touches no file and adds zero `pub struct/enum` lines; no `Cargo.toml` changed; still 11 crates).

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`f87d0056`**, the
> PHASE4-N-AF S1 close (the prior HEAD_DELTAS HEAD) — and it is **valid**: `git rev-parse f87d0056`
> resolves and `git merge-base f87d0056 HEAD == f87d0056` (it is a strict ancestor of HEAD; `f87d0056`
> carries no tag). HEAD is **`5858288e`** (the PHASE4-N-AH registry-close + close-records commit). The
> config baseline at the start of this regen was already `f87d0056` (the previous close bumped it), so the
> window measures cleanly from the recorded baseline forward. The span has **three parts**: (1) the
> **PHASE4-N-AF close tail** — `600581e8` (the formal `Close PHASE4-N-AF` commit that archived the N-AF
> cluster docs to `docs/clusters/completed/PHASE4-N-AF/`) + `2d99cdf2` (the C2-guide §7b record of the
> DC-NODE-18 core close); these are **docs/archive only**, zero production code, zero registry rule; (2) the
> **PHASE4-N-AG cluster** (`DC-NODE-19`, single-producer loop-continuation-after-feed-EOF) — declared at
> `/invariants` (`7e1e8276`), cluster/slice docs + S1 GREEN planner + S2 RED loop continuation + S3 replay
> tests, **superseded-closed** (its hermetic core CE-AG-1..4 is complete; its live CE re-homed to N-AH;
> `DC-NODE-19` stays `declared`); and (3) the **PHASE4-N-AH cluster** (`DC-NODE-20` / `DC-NODE-21` /
> `DC-NODE-22`) — the live-proven cluster that **retired the operator-certificate forge mechanism** and made
> the post-self-admit forge base the **local durable `ChainDb::tip`**. The closer bumps `head_deltas_baseline`
> `f87d0056 → <close SHA>` after this regen so the next cluster measures from here.

This window is **led by PHASE4-N-AH — local selected durable chain forge-base authority (`DC-NODE-20`),
paired with cert-evidence-only (`DC-NODE-21`) and warm-start re-entry (`DC-NODE-22`).** It is the
**production correction** of the N-AF cert-promotion mechanism, surfaced by a live finding, and the headline
is a **brutally clear, honest boundary** that must be read in full:

> **PHASE4-N-AH made Ade sustain CERT-FREE single-producer block production on C2-LOCAL (`cardano-testnet`
> magic 42) against a real Haskell relay (`cardano-node 11.0.1`): Ade forged on its OWN local durable
> `ChainDb::tip`, crossed a follow-link EOF, settled `> k` blocks immutable in the relay's ImmutableDB, and
> RESUMED forging after a hard restart (run-4, `docs/evidence/phase4-n-ah-ce-ah-6-close.{md,jsonl}`). This is
> NOT preprod. This is NOT bounty completion. `RO-LIVE-01` stays operator-gated / partial.**

The arc to that result is itself load-bearing — **the N-AF operator-adoption certificate had leaked from
evidence into forge-loop authority**, which is why this window retires it:

- **PHASE4-N-AG / `DC-NODE-19` (declared; superseded-close).** The CE-AF-6b follow-on deferred from
  PHASE4-N-AF. In a declared single-producer venue **already in** the DC-NODE-18 extend state, a
  `LoopState::Ending` caused **solely by structural feed EOF** (the Ade→relay follow link draining) must not
  by itself terminate the forge loop — the loop continues forging successors on its own certified durable
  spine, fenced to the certified single-producer run, until explicit shutdown / fatal error / a BLUE
  forge-validity bound / a competing chain. The GREEN `plan_loop_step` gains an explicit **5th content-blind
  `VenuePolicy` input** (`HaltOnFeedEnd | ContinueInSingleProducerExtend`) → a **32-case total table**, with a
  GREEN `(VenueRole, ForgeMode) → VenuePolicy` projection; the RED `run_relay_loop_with_sched` threads the
  policy and replaces the `Idle`-under-dead-feed wait with a cancellation-safe clock-tick / shutdown `select!`
  (OQ-19-1). It relocates the loop's termination authority off feed-liveness onto explicit operator shutdown
  (the same move DC-NODE-09 made for the serve-listener lifetime) while preserving DC-NODE-05 (`pump_block`
  stays the sole durable tip-advance authority; feed work drains via `SyncOnce` before any `ForgeTick`). New
  gate `ci/ci_check_single_producer_loop_continuation.sh`. **N-AG was superseded-closed:** the hermetic core
  (CE-AG-1..4) landed and passes, but the DC-NODE-18 cert-promotion mechanism CE-AG-5 relied on to *enter* the
  extend state was retired by DC-NODE-20, so the live sustained-past-k proof (= CE-AF-6b) re-homes to N-AH
  CE-AH-6. `DC-NODE-19` therefore stays **`declared`** (the planned `declared → enforced` flip was gated on
  the now-superseded live CE) and is **strengthened by N-AH** — the extend state it continues is now *entered*
  via local self-admit, not the cert.
- **PHASE4-N-AH / `DC-NODE-20` (NEW, enforced — the production fix).** The run-4 live finding: Ade forged
  block 11, a real Haskell relay **adopted** it, but Ade returned `no_tip_available` every subsequent tick
  because the `proceed_to_forge` gate required `durable_servable_tip == followed_peer_tip` — which fails the
  instant Ade self-admits a block the non-producing relay does not re-announce — and the only escape (the
  DC-NODE-18 cert-promotion into extend mode) **raced and lost** to the follow-link EOF. The cert had become
  forge authority. `DC-NODE-20` makes the **next forge base, post-self-admit, the local selected durable tip**
  (`ChainDb::tip` — the head of Ade's own admitted ChainDB spine), **not** `followed_peer_tip` and **not** the
  cert. It **retires**, as forge authority, the repeated post-self-admit `durable == followed` re-check **and**
  the DC-NODE-18 cert-promotion mechanism; the `ForgeMode::FirstOwnBlockServed` cert-wait intermediate is
  **folded out** of the enum (now a 3-state machine `InitialCatchupRequired → CaughtUpToPeerTip →
  SingleProducerExtendOwnDurableSpine`), and the transition is **direct** on a real self-admit. It engages
  only under a **6-condition fail-closed fence** (VenueRole::SingleProducer; **no competing block observed**
  on the canonical receive stream since catch-up — an OBSERVED-FEED fence, **not** fork-choice; relay
  non-producing; admitted via `pump_block`; spine contiguous/servable; no fork-choice required — mechanically
  **derived** from the observed-feed fence). `DC-NODE-15` **remains** the *initial* catch-up gate (`durable ==
  followed` before the first own-forge); DC-NODE-20 supersedes **only** the repeated post-self-admit re-check.
  In rung 1 "selected" is **degenerate** (no competing candidate ⇒ local ChainDB head = selected tip); rung 2
  must replace this with real fork-choice (`DC-CONS-03`, **untouched**). New gate
  `ci/ci_check_local_durable_forge_base.sh` + the DC-NODE-20 phase-split half of
  `ci_check_forge_followed_tip_admission.sh`.
- **PHASE4-N-AH / `DC-NODE-21` (NEW, enforced — cert is evidence-only).** Separated from DC-NODE-20 (OQ-20-3)
  because "cert is evidence-only, never authority" is independently enforceable and needs a **hard rung-2
  removal boundary**. The file-based operator adoption certificate is a rung-1 RED **evidence-only** shim: it
  may prove relay adoption for the transcript/bounty bundle, but must **never** control forge-base selection
  or any durable authority, and must never appear in multi-producer/preprod/production forge paths. S2 went
  further than demotion: it **fully deleted** `read_adoption_cert` / `parse_hex32` / `VenueAdoptionCertificate`
  / `--adoption-cert-path` — the cert parser is **gone** from `ade_node` production, not merely fenced. New
  gate `ci/ci_check_cert_evidence_only.sh` + `ci_check_node_path_fidelity.sh` (the flag-set 28→29 reconcile).
- **PHASE4-N-AH / `DC-NODE-22` (NEW, enforced — warm-start re-entry).** Found by the S4 run-2 partial:
  warm-start recovery was clean, but forge-resumption stalled in `NoTipAvailable` forever because warm-start
  re-initialized `forge_mode = InitialCatchupRequired`, which needs a fresh follow-link catch-up — and the
  follow link EOF'd first, re-introducing (through restart) the exact follow-link dependency DC-NODE-20
  retired. `DC-NODE-22` is the **warm-start analog of DC-NODE-20**: if warm-start recovery yields a durable
  local `ChainDb::tip` **above** the recovered bootstrap anchor (proving an own-forged continuation spine),
  forge mode **re-enters** `SingleProducerExtendOwnDurableSpine{current_tip = ChainDb::tip}` under the
  DC-NODE-20 fence **without** a fresh followed-peer catch-up; every other case **fails closed** to
  `InitialCatchupRequired`. The new GREEN `warm_start_forge_mode(venue_role, recovered_tip,
  replayed_anchor_block_no)` keys off `replayed_anchor_block_no` — a **derived recovery summary** on
  `BootstrapState` (`recovered_tip.block_no − admit_count`), **NOT** an independently persisted chain point.
  New gate `ci/ci_check_warm_start_re_entry.sh`.

**The N-AF cert mechanism is superseded — recorded honestly.** N-AH **folded out** the `FirstOwnBlockServed`
cert-wait state (the `ForgeMode` enum is now 3-state) and **removed** the `VenueAdoptionCertificate` type +
cert parser. The N-AF cert-promotion mechanism is gone; the extend state is now entered **directly** on local
self-admit. The N-AF live proof (run c2t7 — block 11 forged WITHOUT echo after an adoption certificate)
**validated the cert-free architecture** that N-AH then made the production path. `DC-NODE-18` is retained
(`strengthened_in += PHASE4-N-AH`), its core authority intact; only the cert-into-extend *mechanism* is
retired.

**+0 BLUE canonical type** (the span touches **no** BLUE `core_paths` file). **No `RO-LIVE` rule flipped**
this span — `RO-LIVE-01` stays operator-gated; the run-4 C2-LOCAL transcript is the rung-1 mechanism, not
operator-witnessed bounty acceptance on preprod.

## 0. Headline

| Count | Baseline (`f87d0056`) | HEAD (`5858288e`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 143 | **148** | **+5** — **five NEW gates** (`--diff-filter=A` over `ci/`): `ci_check_single_producer_loop_continuation.sh` (N-AG, DC-NODE-19), `ci_check_local_durable_forge_base.sh` (N-AH, DC-NODE-20), `ci_check_cert_evidence_only.sh` (N-AH, DC-NODE-21), `ci_check_warm_start_re_entry.sh` (N-AH, DC-NODE-22), `ci_check_live_transcript_forge_base.sh` (N-AH S4a, CN-NODE-04 / DC-NODE-20 evidence). **No gate removed** (`--diff-filter=D` over `ci/` empty). **Three gates modified in place** (`--diff-filter=M`): `ci_check_forge_followed_tip_admission.sh` (phase-split for the DC-NODE-20 call chain), `ci_check_node_path_fidelity.sh` (flag-set 28→29 reconcile), `ci_check_single_producer_extend_own_spine.sh` (DC-NODE-20 folds out `FirstOwnBlockServed` + the cert-promotion arm). |
| Registry rules (`docs/ade-invariant-registry.toml`) | 343 | **347** | **+4** — four NEW rules: **`DC-NODE-19`** (declared, N-AG) + **`DC-NODE-20`** + **`DC-NODE-21`** + **`DC-NODE-22`** (all enforced, N-AH). **Zero removed** (`diff` of the sorted `id =` lists shows exactly the four additions `DC-NODE-19/20/21/22` and no removal). |
| Registry status (enforced / partial / declared) | 210 / 20 / 113 | **213 / 20 / 114** | **+3 enforced** (`DC-NODE-20` / `DC-NODE-21` / `DC-NODE-22`) **+1 declared** (`DC-NODE-19`). Partial count **unchanged** (20). |
| Registry strengthenings | — | **9** | **`strengthened_in += "PHASE4-N-AH"`** on **9** existing rules: `CN-NODE-02`, `CN-NODE-04`, `DC-NODE-05`, `DC-NODE-12`, `DC-NODE-15`, `DC-NODE-18`, `DC-NODE-19`, `T-REC-03`, `T-REC-05`. **No `PHASE4-N-AG` strengthening tag exists** — the planned N-AG strengthenings were **folded into N-AH** (the live-proven cluster) rather than double-credited (`git grep` of `strengthened_in` for `PHASE4-N-AG` = 0). No rule weakened. |
| BLUE canonical types | 458 | **458** | **0** — **BLUE-untouched.** The span touches **no** BLUE `core_paths` file (`git diff f87d0056..HEAD` over the BLUE trees is empty and adds zero `pub struct/enum`). The new GREEN forge-mode machinery (`warm_start_forge_mode`, the `VenuePolicy` projection, the `single_producer_forge_decision` rewire) and the new RED `BootstrapState.replayed_anchor_block_no` derived summary all live **outside** `core_paths`. |
| Grounding docs (CODEMAP / SEAMS / TRACEABILITY) | last regenerated to **`6363683e`** (the AE.F close); carried **DC-NODE-16** but **not** DC-NODE-17/18/19/20/21/22 — the deferred N-AF + N-AG CODEMAP debt | **REGENERATED this close** to HEAD `5858288e` — 458 types / 148 CI / 347 rules; carry DC-NODE-18/19/20/21/22 + all five new gates | This close **pays the deferred N-AF + N-AG CODEMAP debt**: CODEMAP + SEAMS + TRACEABILITY were regenerated to fold the whole `6363683e..5858288e` span. Cross-reference verified: CODEMAP carries `DC-NODE-20` (×26), `DC-NODE-21` (×12), `DC-NODE-22` (×16), `DC-NODE-19` (×16), and all five new gates; TRACEABILITY carries the `DC-NODE-20/21/22 ↔ gate` rows. **No staleness this close.** |

> **No grounding-doc deferral this close (load-bearing).** Unlike the N-AF close (which deferred CODEMAP/SEAMS),
> **PHASE4-N-AH regenerated all four grounding docs**, paying the deferred N-AF + N-AG debt: the CODEMAP is
> pinned at `5858288e` and explicitly folds the `6363683e..5858288e` span (N-AF DC-NODE-18, N-AG DC-NODE-19,
> N-AH DC-NODE-20/21/22). The new GREEN/RED additions are in `ade_node` + `ade_runtime` — the host modules
> (`node_sync` GREEN, `run_loop_planner` GREEN, `node_lifecycle` RED, `cli` RED, `live_log::{sched_event,
> sched_writer}` GREEN, `ade_runtime::bootstrap` RED) are all in CODEMAP, and the span adds **no module, no
> BLUE seam, no new persistence surface** (`replayed_anchor_block_no` is a *derived* recovery summary, never
> independently persisted; the cert is now removed from production; `VenuePolicy` is RED/GREEN scheduling).
> The registry holds DC-NODE-19/20/21/22 + their gate bindings authoritatively at HEAD (347 rules).

The slice↔rule↔gate map for this window:

| Slice | Rule(s) | Gate | What shipped |
|---|---|---|---|
| **AG.S1** (`b9ef6e69`) | **`DC-NODE-19`** (declared) | — | GREEN `plan_loop_step` gains the 5th content-blind `VenuePolicy` input → 32-case total table + the `(VenueRole, ForgeMode) → VenuePolicy` projection; default `HaltOnFeedEnd` reduces to the prior 16-case behavior. |
| **AG.S2** (`46098c8c`) | **`DC-NODE-19`** | **`ci_check_single_producer_loop_continuation.sh`** (NEW) | RED `run_relay_loop_with_sched` threads the policy; continues past a structural feed EOF only in the certified single-producer extend state; default `Unknown` halts verbatim; fatal source failure still `Err`/fail-fast; the `Idle`-under-dead-feed clock-tick wakeup (OQ-19-1). |
| **AG.S3** (`a65e2039`) | strengthen `T-REC-03`, `T-REC-05` | — (tests) | Replay-equivalence over a post-feed-end chain — two-runs + kill/warm-start byte-identical; the feed-end event appends nothing to the WAL. |
| **AH.S1** (`b0fb8817`) | **`DC-NODE-20`** (NEW, enforced) | **`ci_check_local_durable_forge_base.sh`** (NEW) + `ci_check_forge_followed_tip_admission.sh` (phase-split) | Forge base = local durable `ChainDb::tip` post-self-admit; the direct `CaughtUpToPeerTip → SingleProducerExtendOwnDurableSpine` transition (the `FirstOwnBlockServed` cert-wait folded out of `ForgeMode`); the 6-condition fail-closed fence with the observed-feed competing-block predicate. |
| **AH.S2** (`050237e9`) | **`DC-NODE-21`** (NEW, enforced) | **`ci_check_cert_evidence_only.sh`** (NEW) + `ci_check_node_path_fidelity.sh` (28→29 reconcile) | Adoption certificate is evidence-only — `read_adoption_cert` / `parse_hex32` / `VenueAdoptionCertificate` / `--adoption-cert-path` **fully deleted** from `ade_node`; the cert never feeds forge-base selection and never appears in multi-producer/preprod/production. |
| **AH.S3** (`dad29b43`) | strengthen `T-REC-03`, `T-REC-05` | — (tests) | Replay-equivalence over the local-tip-derived post-self-admit chain — removing the RED cert/timing from the authority path makes the forge MORE deterministic / replay-equivalent. |
| **AH.S4a** (`7049d813`) | strengthen **`CN-NODE-04`** | **`ci_check_live_transcript_forge_base.sh`** (NEW) | The closed emit-only `NodeSchedEvent` vocabulary gains a `ForgeBaseSelected` event (`forge_base_source = local_chaindb_tip`, the entered mode, `cert_path_present`) + an enriched `ForgeResult` (`self_admit_via_pump_block`); RED evidence only — serializes the decision already made, never read by the planner. |
| **AH.S4b** (`e7b9be7e`) | **`DC-NODE-22`** (NEW, enforced) | **`ci_check_warm_start_re_entry.sh`** (NEW) | NEW GREEN `warm_start_forge_mode` re-enters `SingleProducerExtendOwnDurableSpine{current_tip = ChainDb::tip}` for a single-producer venue whose recovered tip is above the replay anchor, fenced; the RED `BootstrapState.replayed_anchor_block_no` derived recovery summary. |
| **AH.S4** (run-1/2/3/4 evidence) | **`DC-NODE-20/21/22`** (live re-home of CE-AF-6b) | — (evidence) | Operator-gated live acceptance = CE-AH-6: cert-free local-tip forge, sustained `> k` immutable across a follow-link EOF + a hard restart, a real Haskell relay adopting (run-4, `docs/evidence/phase4-n-ah-ce-ah-6-close.{md,jsonl}`). |

The per-commit shape (selected — the full verbatim log is §1):

| Commit | Kind | What it did | Code / CI / registry effect |
|--------|------|-------------|-----------------------------|
| `600581e8` | close (N-AF) | Close PHASE4-N-AF — archive cluster docs to `docs/clusters/completed/PHASE4-N-AF/` | **0 code / 0 CI / 0 registry rule**; cluster-doc archive renames |
| `2d99cdf2` | docs (c2-guide) | Record PHASE4-N-AF / DC-NODE-18 core close in §7b (boundary preserved) | **0 code / 0 CI / 0 registry** |
| `7e1e8276` | docs (invariants) | DC-NODE-19 single-producer loop-continuation-after-feed-EOF sketch (declared) | **0 code / 0 CI**; registry: **`DC-NODE-19` declared** |
| `b9ef6e69` | feat (AG.S1) | GREEN planner VenuePolicy input (32-case total table) | **GREEN code** (`run_loop_planner.rs`) |
| `46098c8c` | feat (AG.S2) | RED loop continuation past feed-EOF | **RED+GREEN code** (`node_lifecycle.rs` policy threading + clock-tick wait); **+1 CI** (`ci_check_single_producer_loop_continuation.sh`) |
| `a65e2039` | test (AG.S3) | Replay-equivalence over a post-feed-end chain | **test-only** |
| `b261589d` | docs (invariants) | DC-NODE-20/21 invariants + registry + cluster plan | **0 code / 0 CI**; registry: **`DC-NODE-20` / `DC-NODE-21` declared** |
| `b0fb8817` | feat (AH.S1) | Forge base = local durable `ChainDb::tip`; self-admit enters extend (DC-NODE-20) | **GREEN+RED code** (`node_sync.rs` direct transition + 6-condition fence; `node_lifecycle.rs` `proceed_to_forge` rewire + `dc_node_15_refusal`); **+1 CI** (`ci_check_local_durable_forge_base.sh`) + phase-split of `ci_check_forge_followed_tip_admission.sh` + edit of `ci_check_single_producer_extend_own_spine.sh` |
| `050237e9` | feat (AH.S2) | Adoption certificate is evidence-only, never forge authority (DC-NODE-21) | **GREEN+RED code** (full deletion of `read_adoption_cert` / `parse_hex32` / `VenueAdoptionCertificate`; `cli.rs` `--adoption-cert-path` removed); **+1 CI** (`ci_check_cert_evidence_only.sh`) + edit of `ci_check_node_path_fidelity.sh` |
| `dad29b43` | test (AH.S3) | Replay-equivalence over the local-tip-derived chain | **test-only** |
| `7049d813` | feat (AH.S4a) | Live transcript forge-base evidence (RED, CN-NODE-04) | **GREEN code** (`live_log::{sched_event, sched_writer}` `ForgeBaseSelected` + enriched `ForgeResult`); **+1 CI** (`ci_check_live_transcript_forge_base.sh`) |
| `e7b9be7e` | feat (AH.S4b) | Single-producer warm-start re-entry (DC-NODE-22) | **GREEN+RED code** (`node_sync.rs` `warm_start_forge_mode`; `node_lifecycle.rs` warm-start arm; `ade_runtime::bootstrap.rs` `replayed_anchor_block_no`); **+1 CI** (`ci_check_warm_start_re_entry.sh`) |
| `2cc6ce25` | fix (ci) | Repair `ci_check_forge_followed_tip_admission` for the DC-NODE-20 call chain | **CI-only** (the phase-split call-chain verification) |
| `5858288e` | docs (N-AH close) | Registry close + close records — DC-NODE-20/21/22 enforced; N-AG superseded | registry: **`DC-NODE-20/21/22` declared → enforced** + 9 `strengthened_in += PHASE4-N-AH`; N-AH cluster-doc archive; all four grounding docs refreshed |

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `5858288e` | docs | docs(phase4-n-ah): registry close + close records — DC-NODE-20/21/22 enforced; N-AG superseded |
| `2cc6ce25` | fix | fix(ci): repair ci_check_forge_followed_tip_admission for the DC-NODE-20 call chain |
| `1159fba9` | docs | docs(phase4-n-ah): CE-AH-6 MET — full 8+3 live bar (run-4); DC-NODE-20/21/22 proven live |
| `c9183f08` | docs | docs(phase4-n-ah): S4 live run-3 — DC-NODE-22 warm-start re-entry CONFIRMED live (point 8 closed) |
| `e7b9be7e` | feat | feat(phase4-n-ah): S4b — single-producer warm-start re-entry (DC-NODE-22) |
| `83e5d269` | docs | docs(phase4-n-ah): S4b seam resolution — replayed_anchor_block_no (a', derived recovery summary) |
| `16792f58` | docs | docs(phase4-n-ah): S4b authority — DC-NODE-22 single-producer warm-start re-entry (found by run-2) |
| `2bebd730` | docs | docs(phase4-n-ah): S4 live run-2 PARTIAL — forge-base transcript direct; warm-start re-entry gap found |
| `7049d813` | feat | feat(phase4-n-ah): S4a — live transcript forge-base evidence (RED, CN-NODE-04) |
| `838de78b` | docs | docs(phase4-n-ah): S4a slice doc — live transcript forge-base evidence (RED, CN-NODE-04) |
| `f5c5b393` | docs | docs(phase4-n-ah): S4 live run-1 PARTIAL — cert-free DC-NODE-20 architectural validation |
| `5c51ec44` | docs | docs(phase4-n-ah): S4 slice doc — operator-gated live acceptance (CE-AH-6, cert-free local-tip path) |
| `dad29b43` | test | test(phase4-n-ah): S3 — replay-equivalence over the local-tip-derived chain (CE-AH-4, zero production) |
| `1eda02b8` | docs | docs(phase4-n-ah): S3 slice doc — replay-equivalence over the local-tip-derived chain (CE-AH-4, test-only) |
| `dc24aada` | docs | docs(phase4-n-ah): record CN-REHEARSAL-FIDELITY-01 28->29 reconciliation (S2 premise correction) |
| `050237e9` | feat | feat(phase4-n-ah): S2 — adoption certificate is evidence-only, never forge authority (DC-NODE-21) |
| `d04c2a9e` | docs | docs(phase4-n-ah): S2 slice doc — adoption certificate is evidence-only (DC-NODE-21, full removal) |
| `b0fb8817` | feat | feat(phase4-n-ah): S1 — forge base = local durable ChainDb::tip; self-admit enters extend (DC-NODE-20) |
| `4a58755f` | docs | docs(phase4-n-ah): S1 slice doc — forge-base authority rewire (DC-NODE-20, self-admit enters extend) |
| `c72cc9b5` | docs | docs(phase4-n-ah): cluster doc + c2-guide pivot boundary (DC-NODE-20/21 — forge-base = local tip, cert evidence-only) |
| `b261589d` | docs | docs(phase4-n-ah): DC-NODE-20/21 invariants + registry + cluster plan — forge-base = local durable tip, cert evidence-only |
| `267364b9` | docs | docs(phase4-n-ag): S4 slice doc — operator-gated live acceptance / CE-AF-6b (DC-NODE-19) |
| `a65e2039` | test | test(phase4-n-ag): S3 — replay-equivalence over a post-feed-end chain (DC-NODE-19, CE-AG-4) |
| `6790d6e6` | docs | docs(phase4-n-ag): S3 slice doc — replay-equivalence over a post-feed-end chain (DC-NODE-19, CE-AG-4) |
| `46098c8c` | feat | feat(phase4-n-ag): S2 — RED loop continuation past feed-EOF (DC-NODE-19, CE-AG-2/3) |
| `17ec5a57` | docs | docs(phase4-n-ag): S2 slice doc — RED loop continuation past feed-EOF (DC-NODE-19, CE-AG-2/3) |
| `b9ef6e69` | feat | feat(phase4-n-ag): S1 — GREEN planner VenuePolicy input (DC-NODE-19, CE-AG-1) |
| `022fba91` | docs | docs(phase4-n-ag): S1 slice doc — GREEN planner VenuePolicy refinement (DC-NODE-19, CE-AG-1) |
| `c87f7109` | docs | docs(phase4-n-ag): cluster-slice plan + cluster doc — single-producer loop-continuation-after-feed-EOF (DC-NODE-19) |
| `7e1e8276` | docs | docs(dc-node-19): single-producer loop-continuation-after-feed-EOF invariants sketch + DC-NODE-19 (declared) |
| `2d99cdf2` | docs | docs(c2-guide): record PHASE4-N-AF / DC-NODE-18 core close in §7b (boundary preserved) |
| `600581e8` | close | Close PHASE4-N-AF — DC-NODE-18 single-producer extend-after-certificate (scoped core enforced) |

No merge commits in the span. **32 commits, zero unclassified.** All but two carry an explicit
conventional-commits prefix (`docs(...)` / `feat(...)` / `fix(...)` / `test(...)`). The two prefix-less
subjects — `600581e8` (`Close PHASE4-N-AF …`) and `1159fba9` (`docs(phase4-n-ah): CE-AH-6 MET …`, prefixed)
— classify by diff scope: `600581e8` is a cluster-doc archive + registry close (**close**, docs/archive only,
zero production code). The `feat(...)` commits (`b9ef6e69`, `46098c8c`, `b0fb8817`, `050237e9`, `7049d813`,
`e7b9be7e`) are the production GREEN/RED changes; the `test(...)` commits (`a65e2039`, `dad29b43`) are
test-only; `2cc6ce25` is a CI-only repair. The N-AF close tail landed 2026-06-07; all N-AG + N-AH work landed
2026-06-08.

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty
> trailer requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that
> is an Ade-local override of the global no-AI-attribution rule and applies to **commit messages
> only**. It does not affect this doc's content.

## 2. New Modules

**None.** `git diff --diff-filter=A --name-only f87d0056..HEAD -- '*.rs'` shows **no new `.rs` source file**
(not even a test file), no new crate, no new `Cargo.toml`, no new workspace (`git diff --name-only … '**/Cargo.toml'`
is empty; still 11 crates). The whole span is **modification only** — it rewrites the GREEN forge-mode
machinery **inside the existing** `crates/ade_node/src/node_sync.rs` (folding out `FirstOwnBlockServed`,
adding `warm_start_forge_mode` and the `VenuePolicy` projection, removing the cert), the GREEN planner
**inside the existing** `crates/ade_node/src/run_loop_planner.rs` (the 5th `VenuePolicy` input), the RED loop
**inside the existing** `crates/ade_node/src/node_lifecycle.rs` (the `dc_node_15_refusal` helper, the
`proceed_to_forge` rewire, the warm-start arm, the cert-read removal), the closed emit-only sched vocabulary
**inside the existing** `crates/ade_node/src/live_log/{sched_event.rs, sched_writer.rs}` (the `ForgeBaseSelected`
event + enriched `ForgeResult`), the CLI **inside the existing** `crates/ade_node/src/cli.rs` (the
`--adoption-cert-path` flag removed), and one additive derived field **inside the existing**
`crates/ade_runtime/src/bootstrap.rs` (`BootstrapState.replayed_anchor_block_no`). The only added files this
span are **five CI gates** (§5), the N-AG + N-AH **cluster + slice + plan docs**, the N-AG/N-AH **invariants
docs**, and the **N-AH live-run evidence** (`docs/evidence/phase4-n-ah-{live-run-1,live-run-2,live-run-3,ce-ah-6-close}.{md,jsonl}`).

> **Cross-reference (CODEMAP/SEAMS) — no new surface, no new module, no new BLUE seam.** The span adds **no
> module and no BLUE seam** — the new GREEN machinery is in `ade_node::{node_sync, run_loop_planner,
> live_log::*}` and the new behavior is a **RED** loop rewire (`node_lifecycle`) + one additive RED derived
> field (`ade_runtime::bootstrap`). All host modules are already in CODEMAP, which was **regenerated this
> close** to fold the whole span (it carries `DC-NODE-20` ×26, `DC-NODE-21` ×12, `DC-NODE-22` ×16,
> `DC-NODE-19` ×16, `VenuePolicy`, `warm_start_forge_mode`, `replayed_anchor_block_no`, `ForgeBaseSelected`,
> and all five new gates). The span removes a surface (`VenueAdoptionCertificate` + cert parser) rather than
> adding one, and adds **no new authoritative persistence surface** (`replayed_anchor_block_no` is a *derived*
> recovery summary, never independently persisted). No cross-reference warning.

## 3. Modules Modified

Six source modules changed this span — **`ade_node` (GREEN + RED) + `ade_runtime` (RED shell)**, **+0 BLUE
canonical type**:

| Module | Color / scope | Key changes |
|--------|---------------|-------------|
| `ade_node::node_sync` (`crates/ade_node/src/node_sync.rs`) | **GREEN** classifier, +1351 / heavy | **AH.S1 (`b0fb8817`) — DC-NODE-20 forge-base rewire:** `forge_mode_after_admit` now enters `SingleProducerExtendOwnDurableSpine{current_tip = own}` **directly** on a real self-admit from `CaughtUpToPeerTip` — the `FirstOwnBlockServed` cert-wait variant is **folded out** of `ForgeMode` (now a 3-state machine `InitialCatchupRequired → CaughtUpToPeerTip → SingleProducerExtendOwnDurableSpine`). `single_producer_forge_decision` is rewired: **no cert-promotion / await-cert arm** (the cert is evidence-only, DC-NODE-21), it derives the forge base from the local durable spine head under the **6-condition fence** (the observed-feed competing-block predicate; fail-closed `ForgeRefused::SingleProducerFenceViolation` over the closed `SingleProducerFenceReason`). `forge_followed_tip_admission` **remains** the DC-NODE-15 *initial* catch-up classifier. **AG.S1/S2 — DC-NODE-19:** the reused `single_producer_forge_decision` / `SingleProducerFenceReason` continuation fence (unchanged behavior; consumed by the planner-threaded continuation). **AH.S4b (`e7b9be7e`) — DC-NODE-22:** the NEW GREEN `warm_start_forge_mode(venue_role, recovered_tip, replayed_anchor_block_no)` re-enters the extend state for a single-producer venue whose recovered tip is above the replay anchor; fails closed to `InitialCatchupRequired` otherwise. **AH.S2 (`050237e9`):** `VenueAdoptionCertificate` **REMOVED**. **No BLUE type; the GREEN decision never references a chain selector (`DC-CONS-03` untouched).** Tests added: `caughtup_self_admit_enters_extend_directly_no_cert`, `local_spine_sustains_two_successors_no_cert`, `local_spine_two_runs_byte_identical`, `warm_start_reentry_requires_tip_above_recovered_anchor`, `warm_start_single_producer_re_enters_extend_and_forges`. |
| `ade_node::run_loop_planner` (`crates/ade_node/src/run_loop_planner.rs`) | **GREEN** planner, +259 | **AG.S1 (`b9ef6e69`) — DC-NODE-19:** `plan_loop_step` gains an explicit **5th content-blind `VenuePolicy` input** (`HaltOnFeedEnd | ContinueInSingleProducerExtend`) → a **32-case total table** (no wildcard); a GREEN `(VenueRole, ForgeMode) → VenuePolicy` projection yields `ContinueInSingleProducerExtend` **only** when `venue == SingleProducer && mode == SingleProducerExtendOwnDurableSpine`, else `HaltOnFeedEnd` (verbatim prior). The default `HaltOnFeedEnd` path **reduces** to the prior 16-case behavior. The planner stays content-blind (no `SlotNo`) and **emit-only** (it may emit but must not consume CN-NODE-04 events). **No RED `LoopState` re-derivation** (OQ-19-6 — the feed-ended truth is stated plainly). Tests: `plan_loop_step_venue_policy_table_is_total`, `plan_loop_step_halt_policy_reduces_to_prior_16`, `venue_policy_projection_is_continue_only_in_extend`. **No new type; no BLUE change.** |
| `ade_node::node_lifecycle` (`crates/ade_node/src/node_lifecycle.rs`) | **RED** loop, +228 / heavy | **AH.S1 (`b0fb8817`) — DC-NODE-20:** the `proceed_to_forge` gate is rewired — **post-self-admit** the forge base derives from the **local durable tip** (`ChainDb::tip`), **not** `durable == followed`, **not** `read_adoption_cert`; the DC-NODE-15 initial-catch-up refusal moves into the named `dc_node_15_refusal` helper (the call chain `run_relay_loop_with_sched → dc_node_15_refusal → forge_followed_tip_admission`, both links verified by the phase-split gate). **AG.S2 (`46098c8c`) — DC-NODE-19:** `run_relay_loop_with_sched` derives `VenuePolicy` from `(act.venue_role, act.forge_mode)` and threads it to the planner; the `Idle`-under-dead-feed wait is replaced with a cancellation-safe clock-tick / `shutdown.changed()` `select!` (OQ-19-1 — the dead feed is no longer the lifecycle authority, the injected clock is the forge-cadence authority). **AH.S2 (`050237e9`):** `read_adoption_cert` / `parse_hex32` **REMOVED** from the forge path. **AH.S4b (`e7b9be7e`) — DC-NODE-22:** the warm-start arm calls `warm_start_forge_mode` to derive the re-entry forge mode from the recovered own-spine tip. **No new type in this module; no BLUE change.** |
| `ade_node::live_log::sched_event` (`crates/ade_node/src/live_log/sched_event.rs`) | **GREEN** closed vocabulary, +83 | **AH.S4a (`7049d813`) — CN-NODE-04 (strengthened):** the closed emit-only `NodeSchedEvent` enum gains a `ForgeBaseSelected` variant (`forge_base_source`, the entered forge mode, `cert_path_present`) + an **enriched** `ForgeResult` (adds `self_admit_via_pump_block`), plus the mirror enums `ForgeBaseSource` (e.g. `local_chaindb_tip`) and `ForgeModeKind` (re-exported via `live_log::mod`). **Operational/diagnostic tier ONLY** — never a consensus-evidence/acceptance/BA-02 signal; emitting changes no forge scheduling/base/authority, and the planner **must not** consume these events. **No BLUE change.** |
| `ade_node::live_log::sched_writer` (`crates/ade_node/src/live_log/sched_writer.rs`) | **GREEN** emit-only writer, +113 | **AH.S4a (`7049d813`):** the exhaustive JSONL encoder is extended to serialize the new `ForgeBaseSelected` event + the enriched `ForgeResult` to the `--log` sink — emit-only, one-directional (`planner → log`). A new variant is a compile error at the exhaustive encoder until wired + allow-listed (CN-NODE-04 closedness). **No BLUE change.** |
| `ade_node::cli` (`crates/ade_node/src/cli.rs`) | **RED**, −13 | **AH.S2 (`050237e9`) — DC-NODE-21:** the `--adoption-cert-path` flag and its `Cli.adoption_cert_path` field + parse arm are **REMOVED** (the cert is no longer a node input). `--single-producer-venue` (the DC-NODE-18 venue declaration) is **retained**. The `ci_check_node_path_fidelity.sh` pinned flag set reconciles 28→29 (adds the pre-existing legitimate `--single-producer-venue`, never re-adds `--adoption-cert-path`). **No new type; no BLUE change.** |
| `ade_runtime::bootstrap` (`crates/ade_runtime/src/bootstrap.rs`) | **RED** shell, +12 | **AH.S4b (`e7b9be7e`) — DC-NODE-22:** `BootstrapState` gains an **additive** `replayed_anchor_block_no: Option<u64>` — a **derived warm-start recovery summary** (`recovered_tip.block_no − replayed_admit_count`), explicitly **NOT an independently persisted chain point**; `None` on cold-start / first-run / `NotRequired` warm-start (only `warm_start_recovery` populates it). It distinguishes bare-anchor recovery from recovery with a replayed local continuation spine, feeding the DC-NODE-22 re-entry decision. **No new persistence surface; no BLUE change.** |

> **No BLUE change this span (load-bearing).** `git diff f87d0056..HEAD` over the BLUE `core_paths` trees is
> **empty** — the span touches **no** BLUE file and adds **zero** `pub struct/enum` lines there. The fix is
> deliberately **GREEN (the forge-mode machinery, the planner projection, the warm-start decision, the closed
> sched vocabulary) + RED (the loop rewire, the CLI, the derived bootstrap summary)**. The BLUE
> canonical-type count is **458 → 458**. The header / body authorities, the KES verifier, forge eligibility,
> the closed wire grammar, the `pump_block` durable-admit chokepoint (`DC-NODE-05`/`DC-NODE-12`/`DC-NODE-16`),
> `ChainDb::tip`, and chain selection (`DC-CONS-03`) are all **unchanged** — the span only changes *which
> tip* the forge gate reads (the local durable tip) and *when* the loop continues, for a declared
> single-producer venue. **Two test/loopback files** were touched additively (`crates/ade_node/tests/wire_only_loopback.rs`
> −1) — test-only, no production-code change. |

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace `Cargo.toml`, and
**no `Cargo.toml` changed in this window** (`git diff --name-only f87d0056..HEAD -- '**/Cargo.toml' 'Cargo.toml'`
is empty). No `#[cfg(feature = …)]` gate was introduced. The notable CLI-flag delta this span is a
**removal**: `--adoption-cert-path` was **deleted** (DC-NODE-21 — the cert is no longer a node input), while
`--single-producer-venue` (the DC-NODE-18 venue declaration) is **retained**. These are CLI flags parsed into
`Cli`, **not** Cargo feature flags, env vars, or compile-time `cfg`. **Coupling note:** with
`--adoption-cert-path` gone, the forge path has **no cert input at all** — the cert (if used for the
transcript) is parsed by the operator harness **outside** the node; the `single_producer_forge_decision`
fence still **fails closed** (`VenueNotDeclaredSingleProducer`) if the extend machinery is reached without an
explicitly declared single-producer venue. The DC-NODE-21 gate `ci_check_cert_evidence_only.sh` enforces that
no cert token survives in the forge path.

## 5. CI Checks (143 → 148; +5 new gates, 3 gates modified in place, 0 gates removed)

Five new gates this span; three modified in place; no gate removed. `git diff --diff-filter=A f87d0056..HEAD
-- ci/` lists exactly the five gates below; `--diff-filter=M` lists the three modified; `--diff-filter=D`
over `ci/` is **empty**.

### PHASE4-N-AG gate (`46098c8c`)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_single_producer_loop_continuation.sh` | **New** | PHASE4-N-AG (`46098c8c`); `DC-NODE-19` | Fences the loop-continuation-after-feed-EOF surface: the explicit **5th `VenuePolicy` input** to `plan_loop_step` + the **32-case no-wildcard** total table + the default-`HaltCleanly`-preserved reduction + the continuation fence **reused (not re-implemented)** from DC-NODE-18 (`SingleProducerFenceReason`) + **no-BLUE-token** in the changed region + the clock-bounded `Idle`-under-dead-feed wait (no busy-spin, no waiting forever on a dead feed). |

### PHASE4-N-AH gates (`b0fb8817`, `050237e9`, `7049d813`, `e7b9be7e`)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_local_durable_forge_base.sh` | **New** | PHASE4-N-AH S1 (`b0fb8817`); `DC-NODE-20` | The post-self-admit forge base is the local durable `ChainDb::tip` (not `followed_peer_tip`, not a cert); the **direct** `CaughtUpToPeerTip → SingleProducerExtendOwnDurableSpine` transition (`forge_mode_after_admit`); the **6-condition fail-closed fence** (incl. the observed-feed competing-block predicate); no silent fallback to followed/cert. |
| `ci_check_cert_evidence_only.sh` | **New** | PHASE4-N-AH S2 (`050237e9`); `DC-NODE-21` | No cert token (`VenueAdoptionCertificate` / `read_adoption_cert` / `adoption_cert`) survives in the `ade_node` forge path; the cert never feeds forge-base selection and never appears in multi-producer/preprod/production forge paths. (Run-4 transcript: `cert_path_present=false` ×461, `cert_path_present:true` count = 0.) |
| `ci_check_warm_start_re_entry.sh` | **New** | PHASE4-N-AH S4b (`e7b9be7e`); `DC-NODE-22` | `warm_start_forge_mode` re-enters `SingleProducerExtendOwnDurableSpine{current_tip = ChainDb::tip}` **only** for a single-producer venue whose recovered tip is above the replay anchor (`replayed_anchor_block_no`); every other case **fails closed** to `InitialCatchupRequired`; the re-entry reads the recovered tip, advances no tip (pump_block stays sole admit authority). |
| `ci_check_live_transcript_forge_base.sh` | **New** | PHASE4-N-AH S4a (`7049d813`); `CN-NODE-04` / `DC-NODE-20` evidence | The closed emit-only `NodeSchedEvent` vocabulary carries the `ForgeBaseSelected` event (`forge_base_source = local_chaindb_tip`, the entered mode, `cert_path_present`) + the enriched `ForgeResult` (`self_admit_via_pump_block`); **emit-only** (serializes the decision already made, never read by the planner / any authority path). |

### Gates modified in place (no add, no remove)

| Check | Change | Why |
|-------|--------|-----|
| `ci_check_forge_followed_tip_admission.sh` | **Modified** (`b0fb8817` + repaired at `2cc6ce25`) | **Phase-split for the DC-NODE-20 call chain.** The DC-NODE-15 *initial* catch-up gate is now verified via the call chain `run_relay_loop_with_sched → dc_node_15_refusal → forge_followed_tip_admission` (**both links** asserted — the prior loop-body grep was stale once the call moved into the `dc_node_15_refusal` helper), while the **post-self-admit** forge base is asserted as the local `ChainDb::tip` with **no followed re-check** — part (a). This encodes the DC-NODE-15 / DC-NODE-20 phase split: initial catch-up requires `durable == followed`; post-self-admit local-tip mode does not. |
| `ci_check_node_path_fidelity.sh` | **Modified** (`050237e9`) | **Flag-set 28→29 reconcile.** The pinned closed `--mode node` allow-list adds the pre-existing legitimate `--single-producer-venue` (N-AF-introduced, legitimately missing from PINNED since N-AF) and confirms the DC-NODE-21-retired `--adoption-cert-path` is **gone** and was never in the pinned set. CN-REHEARSAL-FIDELITY-01 preserved: no from-genesis/devnet/backdoor flag added. |
| `ci_check_single_producer_extend_own_spine.sh` | **Modified** (`b0fb8817`) | **DC-NODE-20 folds out the cert-wait.** The DC-NODE-18 gate now asserts the 3-state `ForgeMode` enum (`FirstOwnBlockServed` **removed**), that `forge_mode_after_admit` enters the extend state **directly** on self-admit, and that `single_producer_forge_decision` has **no** cert-promotion / await-cert arm (the cert is evidence-only). The DC-NODE-18 *core* (no-bool mode, fail-closed fence, no chain selector, mode-aware loop preserving the DC-NODE-15 default) is preserved. |

> **Cross-reference (TRACEABILITY) — refreshed this close, no removal.** The new rule↔gate bindings
> (`DC-NODE-19 ↔ ci_check_single_producer_loop_continuation.sh`, `DC-NODE-20 ↔ ci_check_local_durable_forge_base.sh`
> + the phase-split half of `ci_check_forge_followed_tip_admission.sh`, `DC-NODE-21 ↔ ci_check_cert_evidence_only.sh`
> + `ci_check_node_path_fidelity.sh`, `DC-NODE-22 ↔ ci_check_warm_start_re_entry.sh`, `CN-NODE-04 ↔ ci_check_live_transcript_forge_base.sh`)
> are recorded **both** in the registry at HEAD **and** in the regenerated TRACEABILITY (which carries
> `DC-NODE-20` ×29, `DC-NODE-21` ×17, `DC-NODE-22` ×10, `DC-NODE-19` ×16). **No rule↔gate binding was
> removed.** None of the five new gates is an orphan — each enforces exactly its named rule. `DC-NODE-19` is
> **declared** with a populated `ci_script` (`ci_check_single_producer_loop_continuation.sh`) but stays
> `declared` because its live CE was superseded (the hermetic gate is green; the status flip was gated on the
> re-homed live CE). |

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. **No BLUE canonical type was
added or removed in this window** — the BLUE count is unchanged (**458 → 458**; `git diff f87d0056..HEAD`
over the BLUE `core_paths` trees is empty and adds zero `pub struct/enum`). This window in fact **removed** a
GREEN type (`VenueAdoptionCertificate`, the N-AF cert carrier) along with the cert parser; the new pub items
that carry a chain point — the GREEN `warm_start_forge_mode` decision and the RED
`BootstrapState.replayed_anchor_block_no` derived summary — are **outside** `core_paths` and are **not** BLUE
canonical types. No `Cargo.toml` changed.

## 7. Normative / Invariant Rule Delta (343 → 347; +4 rules, 9 strengthenings, zero removals)

**Four rule IDs were added; zero removed** (343 → 347; `diff` of the sorted `id =` lists shows exactly the
four additions `DC-NODE-19` / `DC-NODE-20` / `DC-NODE-21` / `DC-NODE-22` and no removal). The status tally
moves **210 → 213 enforced** (`DC-NODE-20` / `DC-NODE-21` / `DC-NODE-22`) and **113 → 114 declared**
(`DC-NODE-19`); the 20 partial **unchanged**.

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs, the
CE-73 reclassification, and `CLAUDE.md` — were **not** changed this span: `git diff --name-only
f87d0056..HEAD` over those paths is empty. The rule-count delta is entirely the invariant-registry change
below.)*

**New rules (`+4`):**

| Rule | Family / Tier · Status | Statement (summary) |
|------|------------------------|---------------------|
| `DC-NODE-20` | DC / `derived` · **enforced** (`introduced_in = "PHASE4-N-AH"`) | **Local selected durable chain forge-base authority (rung-1 single-producer).** After Ade self-admits a valid forged block through `pump_block` (DC-NODE-12) onto its local durable ChainDB spine, the next forge base is Ade's **LOCAL SELECTED DURABLE TIP** (`ChainDb::tip`) — **NOT** `followed_peer_tip`, **NOT** an operator adoption certificate. **Retires** (as forge authority) the repeated post-self-admit `durable == followed` re-check **and** the DC-NODE-18 cert-promotion mechanism; the `FirstOwnBlockServed` cert-wait intermediate is **folded out** (direct `CaughtUpToPeerTip → SingleProducerExtendOwnDurableSpine` on self-admit, no cert read). Engages only under a **6-condition fence**; ANY failure **fails closed** (no silent fallback): VenueRole::SingleProducer; **no competing block observed** on the canonical receive stream (an OBSERVED-FEED fence, **not** fork-choice); relay non-producing; admitted via `pump_block` (DC-NODE-05 stays the sole durable admit authority); spine contiguous/servable; no fork-choice required — mechanically **derived** from the observed-feed fence. `DC-NODE-15` remains the **initial** catch-up gate; DC-NODE-20 supersedes **only** the repeated post-self-admit re-check. In rung 1 "selected" is **degenerate**; rung 2 must replace this with real fork-choice (`DC-CONS-03`, untouched). `ci_script = ci/ci_check_local_durable_forge_base.sh, ci/ci_check_forge_followed_tip_admission.sh`; `cross_ref = [DC-NODE-05, DC-NODE-12, DC-NODE-15, DC-NODE-18, DC-NODE-19, DC-NODE-21, DC-CONS-03, DC-CONS-23, T-REC-03, T-REC-05]`. |
| `DC-NODE-21` | DC / `derived` · **enforced** (`introduced_in = "PHASE4-N-AH"`) | **Adoption certificate is rung-1 evidence-only, never forge authority.** The file-based operator adoption certificate is a rung-1 RED **EVIDENCE-ONLY** shim — it MAY prove relay adoption for the transcript/bounty bundle, but MUST **NEVER** control forge-base selection (DC-NODE-20 derives the base from the local durable ChainDB tip) or any durable authority, and MUST NEVER appear in multi-producer/preprod/production forge paths. **Hard removal boundary:** the shim exists ONLY because Ade lacks full multi-producer fork-choice + peer-state lifecycle; it must be removed/replaced by node-local selected-chain / fork-choice authority (DC-CONS-03) before rung 2 / preprod. **S2 fully DELETED** `read_adoption_cert` / `parse_hex32` / `VenueAdoptionCertificate` / `--adoption-cert-path` (the parser is gone, not just demoted). `ci_script = ci/ci_check_cert_evidence_only.sh, ci/ci_check_node_path_fidelity.sh`; `cross_ref = [DC-NODE-20, DC-NODE-18, DC-NODE-19, DC-CONS-03]`. |
| `DC-NODE-22` | DC / `derived` · **enforced** (`introduced_in = "PHASE4-N-AH"`) | **Single-producer warm-start re-entry derives forge mode from the recovered local durable spine.** If warm-start recovery yields a durable local `ChainDb::tip` **above** the recovered bootstrap anchor (proving an own-forged continuation of Ade's own spine, not the bare imported anchor), forge mode MUST re-enter `SingleProducerExtendOwnDurableSpine{current_tip = ChainDb::tip}` under the DC-NODE-20 fence, **WITHOUT** a fresh followed-peer catch-up. The warm-start analog of DC-NODE-20. Without it, warm-start re-inits `forge_mode = InitialCatchupRequired` and, if the follow link EOFs first, the node stalls in `NoTipAvailable` forever — re-introducing through restart the exact follow-link dependency DC-NODE-20 retired. Engages only under a **9-condition fence**; ANY failure **fails closed** (fall back to `InitialCatchupRequired`). The `replayed_anchor_block_no` (= `recovered_tip.block_no − admit_count`) is a **DERIVED recovery summary** on `BootstrapState`, NOT an independently persisted chain point. `ci_script = ci/ci_check_warm_start_re_entry.sh`; `cross_ref = [DC-NODE-20, DC-NODE-19, DC-NODE-15, DC-NODE-12, DC-NODE-05, T-REC-05, CN-NODE-02, DC-CONS-03]`. |
| `DC-NODE-19` | DC / `derived` · **declared** (`introduced_in = "TBD"`) | **Single-producer forge-loop continuation after follow-link EOF.** In a declared single-producer venue already in the DC-NODE-18 extend state, a `LoopState::Ending` caused **solely by structural feed EOF** MUST NOT by itself terminate the forge loop; the loop continues forging successors on its OWN certified durable spine (each admitted via `pump_block`, DC-NODE-12), **fenced** to the certified single-producer run and **failing closed** (verbatim `HaltCleanly` / typed refusal, never a silent forge) on any of **7** conditions. Relocates the loop's termination authority off feed-liveness onto explicit operator shutdown / fatal error (the move DC-NODE-09 made for the serve-listener lifetime) while preserving DC-NODE-05 (`pump_block` sole tip-advance; feed work drains via `SyncOnce` first). Only a clean structural feed EOF is continued; a fatal source failure exits via `Err`/fail-fast. **Declared** at `/invariants` (N-AG); enforced only when the slice lands with the GREEN 32-case totality + the certified-run continuation fence + replay-equivalence over a post-feed-end chain + a committed sustained-past-k live transcript. **Superseded-close:** the hermetic core (CE-AG-1..4) landed and passes, but the live CE re-homed to N-AH CE-AH-6 (the cert-promotion entry mechanism was retired by DC-NODE-20), so DC-NODE-19 stays `declared` and is **strengthened by N-AH** (the extend state it continues is now entered via local self-admit). `ci_script = ci/ci_check_single_producer_loop_continuation.sh`; `cross_ref = [DC-NODE-18, DC-NODE-05, DC-NODE-09, DC-NODE-12, CN-NODE-02, T-REC-03, T-REC-05, DC-CONS-03, DC-EPOCH-03, DC-CONS-09]`. |

**Strengthenings (`strengthened_in += "PHASE4-N-AH"`) — 9:** `CN-NODE-02` (the run-loop lifecycle owner now
covers the warm-start wiring + the loop's termination authority = explicit operator shutdown / fatal error,
not accidental feed EOF), `CN-NODE-04` (the closed emit-only sched vocabulary now also carries the DC-NODE-20
forge-base evidence — `ForgeBaseSelected` + enriched `ForgeResult`), `DC-NODE-05` (feed work still drains
before forge; the forge advances no durable tip directly — preserved across the local-tip rewire),
`DC-NODE-12` (own-forged durable admit chokepoint — the local-tip forge base only READS the tip `pump_block`
produced), `DC-NODE-15` (the `durable == followed` gate is now the **initial** catch-up gate only,
phase-split from the retired post-self-admit re-check), `DC-NODE-18` (the proven extend state is now entered
via local self-admit, not the cert; the `FirstOwnBlockServed` cert-wait is folded out), `DC-NODE-19` (the
continue-past-EOF extend state now survives warm-start), `T-REC-03` + `T-REC-05` (replay-equivalence now
covers cert-free local-tip-derived successors **and** post-warm-start forge resumption). **`DC-CONS-03`
explicitly untouched** (rung-2 fork-choice successor). **No `PHASE4-N-AG` strengthening tag exists** — the
planned N-AG strengthenings were **folded into N-AH** (the live-proven cluster) to avoid double-crediting.
No rule was weakened.

> **The brutally clear boundary — what PHASE4-N-AH closes (and what it does not).** PHASE4-N-AH enforced
> **`DC-NODE-20` / `DC-NODE-21` / `DC-NODE-22`**: **Ade sustained cert-free single-producer block production
> on C2-LOCAL (`cardano-testnet` magic 42) against a real Haskell relay (`cardano-node 11.0.1`)** — Ade
> forged on its OWN local durable `ChainDb::tip`, **crossed a follow-link EOF**, settled **`> k` blocks
> immutable** in the relay's ImmutableDB, and **RESUMED forging after a hard restart** (run-4,
> `docs/evidence/phase4-n-ah-ce-ah-6-close.{md,jsonl}`; the full 8+3 bar). This is **NOT preprod. This is NOT
> bounty completion.** `RO-LIVE-01` stays **operator-gated / partial** — the run-4 C2-LOCAL transcript is the
> rung-1 mechanism, **not** operator-witnessed bounty acceptance on preprod. Rung-2 fork-choice +
> multi-producer + preprod are explicitly out of scope; the observed-feed competing-block fence **fails
> closed** (never resolves — that is rung 2). **No `RO-LIVE` rule flipped** this span. |

**No rule was removed (expected: 0).** The registry delta is **four new rules (`DC-NODE-20`/`21`/`22`
enforced + `DC-NODE-19` declared), nine `strengthened_in += PHASE4-N-AH` appends, zero removals** —
consistent with append-only registry discipline. **Note on the N-AF cert mechanism:** retiring the
cert-into-extend *mechanism* (DC-NODE-20 folds out `FirstOwnBlockServed`; DC-NODE-21 deletes the parser) is
**not** a rule removal or weakening — `DC-NODE-18` is **retained** (`strengthened_in += PHASE4-N-AH`), its
statement and ID unchanged, its scope-boundary note intact; only the *implementation mechanism* it described
is superseded by a stronger cert-free path. No discipline violation.

## Working tree at HEAD `5858288e`

Clean of tracked changes from this span — the N-AF close tail, the N-AG cluster (invariants → cluster/slice
docs → S1/S2/S3), and the N-AH cluster (invariants → cluster/slice docs → S1/S2/S3/S4a/S4b → close) are all
committed. `git status --short` shows only an untracked `.mithril-scratch/` (operator scratch, ignored).
**This regen runs *after* all 32 span commits** (the N-AH close `5858288e` is HEAD for this window); the
registry records `DC-NODE-19/20/21/22` + their gate bindings authoritatively at HEAD (347 rules), and all
four grounding docs (CODEMAP/SEAMS/TRACEABILITY) were regenerated this close to `5858288e`. The remaining
close-pass action is the baseline bump (`f87d0056 → 5858288e`, performed by the closer).

> **Cluster-context note.** PHASE4-N-AH is **formally closed** — its cluster doc carries the §11 close record
> (CE-AH-7), and PHASE4-N-AG is **superseded-closed** (its §11 records the partial close; its hermetic core
> is complete and its live CE re-homed to N-AH). Both clusters' docs live under `docs/clusters/PHASE4-N-A{G,H}/`;
> whether they are moved to `docs/clusters/completed/` is a close-pass bookkeeping decision separate from this
> HEAD_DELTAS regen (the N-AF docs were archived to `docs/clusters/completed/PHASE4-N-AF/` by `600581e8`).

## Honest residual (window scope)

PHASE4-N-AH **closed the cert-free local-tip single-producer forge core** — Ade sustained block production on
its own local durable spine across a follow-link EOF + a restart, live-proven on C2-LOCAL (run-4). The honest
residual:

- **The headline boundary (verbatim, brutally clear).** Ade sustained **cert-free single-producer block
  production on C2-LOCAL** (`cardano-testnet` magic 42) against a real Haskell relay (`cardano-node 11.0.1`),
  **crossed a follow-link EOF**, settled **`> k` blocks immutable**, and **resumed forging after a hard
  restart** (run-4, `docs/evidence/phase4-n-ah-ce-ah-6-close.{md,jsonl}`). **NOT preprod. NOT bounty
  completion. `RO-LIVE-01` stays operator-gated / partial.**
- **The run-4 live finding redirected the fix.** The N-AF operator-adoption certificate had **leaked from
  evidence into forge-loop authority**: post-self-admit, the `proceed_to_forge` gate required `durable ==
  followed` (which fails the instant the non-producing relay does not re-announce Ade's block), and the only
  escape (cert-promotion) **raced and lost** to the follow-link EOF. `DC-NODE-20` makes the post-self-admit
  forge base the **local durable `ChainDb::tip`** under a 6-condition fence; `DC-NODE-21` **deletes** the cert
  from production (evidence-only); `DC-NODE-22` makes warm-start **re-enter** the extend state on the
  recovered own-spine tip so a restart does not re-introduce the follow-link dependency.
- **PHASE4-N-AG is superseded — its hermetic core stands.** N-AG's CE-AG-1..4 (the 32-case planner totality,
  the 7-condition fail-closed continuation, the post-feed-end replay-equivalence) **landed and pass** as
  hermetic infrastructure (`DC-NODE-19`). Its live sustained CE (CE-AG-5 = CE-AF-6b) was **re-homed** to N-AH
  CE-AH-6 because the cert-promotion entry mechanism it relied on was retired by DC-NODE-20. `DC-NODE-19`
  stays **`declared`** (not overclaimed as independent live architecture — the live forge architecture is
  DC-NODE-20) and is **strengthened by N-AH**.
- **GREEN+RED only, +0 BLUE / +0 BLUE type / no new persistence surface.** The span touches **no** BLUE file;
  the fix is the GREEN forge-mode machinery + planner projection + warm-start decision + closed sched
  vocabulary (`ade_node::{node_sync, run_loop_planner, live_log::*}`) + the RED loop rewire + CLI removal +
  the additive derived bootstrap summary (`ade_node::{node_lifecycle, cli}` + `ade_runtime::bootstrap`). The
  cert is **removed** from production (not just demoted); `replayed_anchor_block_no` is a **derived** recovery
  summary, never independently persisted. BLUE canonical-type count **458 → 458**; `pump_block` durable
  admission (`DC-NODE-05`/`DC-NODE-12`/`DC-NODE-16`), `ChainDb::tip`, ledger/chain_dep/WAL, and chain
  selection (`DC-CONS-03`) are all unchanged.
- **`DC-CONS-03` untouched; the fence fails closed, never resolves.** The local-tip authority is gated behind
  `VenueRole::SingleProducer`; the observed-feed competing-block predicate **fails closed** on any peer-origin
  non-spine block — **no fork resolution** is attempted (that is rung 2). `DC-CONS-03` stays the sole
  follow/fork authority; the continuation/forge-base decision never selects/reorders/prefers chains. The five
  new gates fence: local-tip-forge-base + 6-condition-fence (S1), cert-not-in-forge-path (S2), warm-start
  re-entry fail-closed (S4b), forge-base-evidence emit-only (S4a), and loop-continuation 32-case +
  fence-reused + clock-bounded (N-AG).
- **N-AF close tail is the span head (docs/archive only).** `600581e8` (the formal `Close PHASE4-N-AF`,
  archiving the cluster docs) and `2d99cdf2` (the C2-guide §7b record) are **docs/archive only** (no `.rs` /
  `.sh`); they added **no** rule. The registry was already at 343 from the N-AF impl.
- **Grounding-doc debt paid this close.** Unlike the N-AF close (which deferred CODEMAP/SEAMS), PHASE4-N-AH
  **regenerated all four grounding docs** to `5858288e`, folding the whole `6363683e..5858288e` span (N-AF
  DC-NODE-18, N-AG DC-NODE-19, N-AH DC-NODE-20/21/22). CODEMAP/SEAMS/TRACEABILITY now carry
  DC-NODE-18/19/20/21/22 + all five new gates — **no staleness this close**. The next cluster's
  `/head-deltas` measures from `5858288e`.
- **Carry-forward (rung-1 hardening, not blockers).** **AH-FOLLOW-1:** broaden the DC-NODE-20 competing-block
  fence from the observed-tip `block_no`/hash checks to a RED-computed "peer-origin candidate not in Ade's
  admitted spine / own-served lineage" flag (ChainDb spine-membership) threaded into the GREEN fence —
  classify as rung-1 hardening before multi-producer / rung 2. The independent-anchor-tip persistence
  (DC-NODE-22 option b′) is a deferred storage-hardening slice. **OQ-KA** (follow-link keep-alive / reconnect)
  remains a separate non-blocking cousin.

---

## Historical — PHASE4-N-AF single-producer extend-own-durable-spine (`6363683e → f87d0056`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It was a
> **single-slice cluster lead** narrating the `6363683e → f87d0056` span: the PHASE4-N-AE.F close
> grounding-doc refresh (`d3f52e7c`, the baseline's own docs commit, span head) + a C2-guide doc (`1302417d`)
> followed by the **OQ-1 / DC-NODE-17 investigation** (`bd1a7a73` declared DC-NODE-17 → `dadf4743`
> live-disproved it as the fix) and the **PHASE4-N-AF cluster** (single slice AF.S1 — `DC-NODE-18`,
> single-producer extend-own-durable-spine). Counts here are the figures **at `f87d0056`** (343 rules, 143 CI
> gates, 458 canonical types); the current window measures **forward** from `f87d0056`. The full §§0–7
> narrative is recoverable from this doc's git history at `f87d0056`.

> Baseline: `6363683e` (AE.F receive idempotency — survive the post-adoption echo, 2026-06-07 13:40)
> HEAD: `f87d0056` (PHASE4-N-AF S1 — enforce DC-NODE-18 core (extend-after-cert) + 2 live-surfaced loop fixes, 2026-06-07 23:59)
> Span: **the PHASE4-N-AE.F close grounding refresh + the OQ-1 / DC-NODE-17 investigation + the PHASE4-N-AF cluster (DC-NODE-18)** — 8 commits, 19 files, +2489 / −456.

PHASE4-N-AF enforced **`DC-NODE-18`**: a mode-aware ForgeTick gate (the `ForgeMode` enum — at N-AF a 4-state
`InitialCatchupRequired → CaughtUpToPeerTip → FirstOwnBlockServed → SingleProducerExtendOwnDurableSpine`,
GREEN transition fn, no booleans) let a declared single-producer venue forge on Ade's **OWN durable spine
WITHOUT a relay echo**, **after** an explicit RED venue-adoption certificate matched by **chain-point
identity** (hash + block_no), **never** inferred from self-admit; fail-closed
`ForgeRefused::SingleProducerFenceViolation`; the certificate was **admissibility-only** (never persisted /
replay-visible); the default `VenueRole::Unknown` path was the verbatim prior `DC-NODE-15` gate.
**N-AF-window headline (at `f87d0056`):** Registry **341 → 343** (+DC-NODE-17 declared + DC-NODE-18 enforced;
0 strengthenings; 0 removed; status 209 → 210 enforced). CI gates **142 → 143** (+1
`ci_check_single_producer_extend_own_spine.sh`; 0 modified / 0 removed). **GREEN+RED only — BLUE canonical
types 458 → 458** (no BLUE file touched). Two live-surfaced loop fixes (a `not_leader`-advance bug + a
cert-match-too-strict-on-slot bug) landed in the close commit, both missed by the hermetic suite + the
IDD/security reviews. **No `RO-LIVE` flip.** The N-AF live proof (run c2t7 — block 11 forged WITHOUT echo
after an adoption certificate) **validated the cert-free architecture** that **PHASE4-N-AH then made the
production path** (folding out `FirstOwnBlockServed` + removing the cert — see the current lead).
CODEMAP/SEAMS/TRACEABILITY refresh was **deferred** at the N-AF close and **paid in full at the N-AH close**.

---

## Historical — PHASE4-N-AE.F post-CE-A5 echo-idempotency follow-up (`a76672b9 → 6363683e`)

> Preserved as a pointer. A **single-slice lead** narrating the `a76672b9 → 6363683e` span: the PHASE4-N-AE
> close grounding-doc refresh (`62811a4e`, span head) followed by the **PHASE4-N-AE.F** slice (`DC-NODE-16`
> receive idempotency at the durable-admit chokepoint — a re-announced block Ade already durably holds (same
> hash, same slot) is an idempotent no-op at `pump_block`, so a continuous recover→follow run survives the
> post-adoption echo instead of exiting 43). Counts at `6363683e`: 341 rules, 142 CI gates, 458 canonical
> types. **RED chokepoint only — BLUE 458 → 458.** New gate `ci_check_receive_idempotency.sh`. No `RO-LIVE`
> flip. The full §§0–7 narrative is recoverable from this doc's git history at `6363683e` / `d3f52e7c`.

---

## Historical — PHASE4-N-AD durability proof + C2-LOCAL run + PHASE4-N-AE CE-A5 cluster (`25ddeebd → a76672b9`)

> Preserved as a pointer. A **multi-part lead** narrating the `25ddeebd → a76672b9` span: the PHASE4-N-AC
> grounding-doc-refresh tail (`25ddeebd`), the **test-only PHASE4-N-AD** tip-successor durability cluster, a
> **docs-only C2-LOCAL** preprod-tip / cardano-testnet venue guide-and-finding run, and the closing **CE-A5
> cluster PHASE4-N-AE** — **Recover→Serve Continuity and Forge Admissibility** (the CE-A5 manifest: a real
> `cardano-node 11.0.1` relay `AddedToCurrentChain` an Ade-forged successor block). Counts at `a76672b9`: 340
> rules, 141 CI gates, 458 canonical types. +4 enforced rules (`DC-NODE-14`, `DC-NODE-15`, `DC-CONS-24`,
> `DC-PROTO-10`) + 9 strengthenings; CI 138 → 141 (+3). **BLUE-additive, +0 canonical type.** **CE-A5 is
> recorded as `enforced`-backing evidence, NOT a `RO-LIVE` flip.** The full §§0–7 narrative is recoverable
> from this doc's git history at `a76672b9` / `62811a4e`.

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

### Regen `f87d0056 → 5858288e` (PHASE4-N-AG superseded + PHASE4-N-AH local-tip forge-base authority — current lead)

- **Baseline valid; two pivoting single-theme clusters preceded by the N-AF close tail.** Run against
  `f87d0056` (the PHASE4-N-AF S1 close, the prior HEAD_DELTAS HEAD), which `git rev-parse` resolves and `git
  merge-base f87d0056 HEAD` confirms is a strict ancestor of HEAD `5858288e` (`f87d0056` carries no tag). The
  start-of-regen config baseline was already `f87d0056` (the previous close bumped it). The closer bumps
  `head_deltas_baseline` `f87d0056 → 5858288e` after this regen.
- **Counts are mechanical (git/grep/ls):** commit log + `--shortstat` over `f87d0056..HEAD` (**32** commits,
  no merges / **48** files / **+5155 / −743**); CI gate count via `git ls-tree -r --name-only <ref> ci/ |
  grep -cE 'ci_check_.*\.sh$'` at each ref (**143 → 148**; `--diff-filter=A` over `ci/` = the five new gates;
  `--diff-filter=M` = the three modified gates — `ci_check_forge_followed_tip_admission.sh`,
  `ci_check_node_path_fidelity.sh`, `ci_check_single_producer_extend_own_spine.sh`; `--diff-filter=D` over
  `ci/` **empty**); registry rule count via `grep -cE '^id = '` at each ref (**343 → 347**; `diff` of sorted
  `id =` lists shows the four additions `DC-NODE-19/20/21/22`, zero removals); registry status via `grep -E
  '^status = ' | sort | uniq -c` (**210 → 213 enforced**, **113 → 114 declared**, 20 partial unchanged);
  strengthenings = **9** (a `python` scan of `strengthened_in` lines shows `PHASE4-N-AH` appended to
  CN-NODE-02 / CN-NODE-04 / DC-NODE-05 / DC-NODE-12 / DC-NODE-15 / DC-NODE-18 / DC-NODE-19 / T-REC-03 /
  T-REC-05; **zero** `PHASE4-N-AG` strengthening tag — folded into N-AH); BLUE canonical types via a `git
  diff f87d0056..HEAD` over the BLUE `core_paths` trees (**empty diff, zero `pub struct/enum` → 458 → 458**).
- **Note (brief vs. git — the third modified gate).** The close brief named two in-place gate edits
  (`ci_check_forge_followed_tip_admission.sh`, `ci_check_node_path_fidelity.sh`). `git diff --diff-filter=M`
  over `ci/` shows a **third**: `ci_check_single_producer_extend_own_spine.sh` was edited at AH.S1
  (`b0fb8817`) so the DC-NODE-18 gate asserts the **3-state** `ForgeMode` (the `FirstOwnBlockServed` cert-wait
  **removed**) and the cert-promotion-free `forge_mode_after_admit`. Recorded faithfully (git is authoritative);
  the §0 / §5 counts read **3 gates modified**, not 2.
- **GREEN+RED-only span — no BLUE file, +0 BLUE canonical type, no Cargo.toml change.** `git diff
  --name-status f87d0056..HEAD` shows the only production-code change is six source files in `ade_node` +
  `ade_runtime` (`node_sync.rs`, `run_loop_planner.rs`, `node_lifecycle.rs`, `cli.rs`,
  `live_log/{sched_event,sched_writer}.rs`, `ade_runtime/src/bootstrap.rs`) and `git diff f87d0056..HEAD` over
  the BLUE trees is **empty**. No new `.rs` *source* file. `git diff --name-only … '**/Cargo.toml' 'Cargo.toml'`
  is empty (no feature-flag delta; the notable CLI-flag delta is a **removal**, `--adoption-cert-path`).
  **Classification note:** `node_sync.rs` + `run_loop_planner.rs` + `live_log/{sched_event,sched_writer}.rs`
  are **GREEN** (pure/total/deterministic, no I/O / emit-only); `node_lifecycle.rs` + `cli.rs` are **RED**
  (the loop + CLI); `ade_runtime::bootstrap` is the **RED** shell. `ade_node` is neither a BLUE `core_paths`
  crate nor `ade_runtime` (the RED shell crate); per the project's TCB scoping the new GREEN/RED items are
  non-BLUE.
- **Registry delta is +4 rules (three enforced, one declared) + 9 strengthenings, NOT a removal.**
  `DC-NODE-20` / `DC-NODE-21` are declared (`b261589d`) then flipped declared → enforced at the N-AH close
  `5858288e`; `DC-NODE-22` is declared (S4b authority `16792f58`) then enforced at close; `DC-NODE-19` is
  declared (`7e1e8276`) and **stays declared** (its live CE was superseded — N-AG superseded-close). The
  sorted-id `diff` confirms zero removals. Retiring the N-AF cert mechanism (folding out `FirstOwnBlockServed`,
  deleting the parser) is **not** a rule removal — `DC-NODE-18` is retained + strengthened.
- **Span head is the N-AF close tail (`600581e8` + `2d99cdf2`).** Both are **docs/archive only** (`git show
  --name-only` has no `.rs` / `.sh`); `600581e8` archived the N-AF cluster docs to
  `docs/clusters/completed/PHASE4-N-AF/`. The registry was already at 343 from the N-AF impl; neither added a
  rule.
- **No `RO-LIVE` flip; CE-AH-6 is the live-proven core on the cert-free path.** N-AH's DC-NODE-20/21/22 are
  recorded `enforced` for the rung-1 single-producer scope, backed by hermetic enforcement (the five gates +
  the unit/replay tests) + the run-4 C2-LOCAL live proof (cert-free local-tip forge, sustained `> k`
  immutable across a follow-link EOF + a restart, a real Haskell relay adopting). It does **NOT** claim
  preprod or bounty completion. No `RO-LIVE` registry status changed this span (`RO-LIVE-01` stays
  operator-gated / partial).
- **Normative docs unchanged this span.** `git diff --name-only f87d0056..HEAD` over the configured
  `normative_docs` (CE-79 statement + addendum, the three contract docs, CE-73 reclassification, `CLAUDE.md`)
  is empty — the §7 delta is entirely the invariant-registry change.
- **§1 commit log verbatim from `git log --oneline --no-merges` (newest first).** The per-slice synthesis is
  in §0/§3. All but two subjects carry a `docs(...)` / `feat(...)` / `fix(...)` / `test(...)` prefix; the
  prefix-less `600581e8` (`Close PHASE4-N-AF …`) classifies by diff scope as **close** (docs/archive only).
- **Doc-refresh state — all four grounding docs REGENERATED this close.** Unlike the N-AF close (which
  deferred CODEMAP/SEAMS), PHASE4-N-AH regenerated CODEMAP + SEAMS + TRACEABILITY to `5858288e`, paying the
  deferred N-AF + N-AG debt. Cross-reference verified by `grep -c`: CODEMAP carries `DC-NODE-20` ×26 /
  `DC-NODE-21` ×12 / `DC-NODE-22` ×16 / `DC-NODE-19` ×16 + all five new gates + `VenuePolicy` /
  `warm_start_forge_mode` / `replayed_anchor_block_no` / `ForgeBaseSelected`; TRACEABILITY carries the
  `DC-NODE-20/21/22 ↔ gate` rows (`DC-NODE-20` ×29 / `DC-NODE-21` ×17 / `DC-NODE-22` ×10). **No staleness;
  no cross-reference warning.**
- **Working tree clean.** This regen runs *after* all 32 span commits (the N-AH close `5858288e` is HEAD for
  this window); `git status --short` shows only an untracked `.mithril-scratch/` (operator scratch, ignored).
  The remaining close-pass action is the baseline bump `f87d0056 → 5858288e`.

### Regen `6363683e → f87d0056` (PHASE4-N-AF single-producer extend-own-durable-spine — prior lead)

- **Single-slice cluster lead** preceded by a docs-only investigation arc, measured from `6363683e` (the
  PHASE4-N-AE.F idempotency fix). **8** commits / **19** files / **+2489 / −456** (the AE.F-close grounding
  refresh `d3f52e7c` dominates the doc churn; the AF.S1 impl is the only production change — GREEN
  `node_sync.rs`, RED `node_lifecycle.rs` + `cli.rs`); CI gates **142 → 143** (+1
  `ci_check_single_producer_extend_own_spine.sh`; 0 modified / 0 removed); registry **341 → 343** (+DC-NODE-17
  declared + DC-NODE-18 enforced; 0 strengthenings; 0 removed; status 209 → 210 enforced); BLUE canonical
  types **458 → 458** (GREEN+RED only — no BLUE file touched). `DC-NODE-17` was declared then **live-disproved**
  as the fix (the relay does NOT re-announce Ade's own block) and retained safety/observation-only; the actual
  fix was `DC-NODE-18` (extend the chain Ade is building). Two live-surfaced loop fixes landed in the close
  commit, both missed by the hermetic suite + the IDD/security reviews. CODEMAP/SEAMS/TRACEABILITY refresh was
  **deferred** at the N-AF close and **paid at the N-AH close** (the current lead). No `RO-LIVE` flip. Full
  notes recoverable from this doc's git history at `f87d0056`.
