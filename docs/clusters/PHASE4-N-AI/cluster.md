# Cluster PHASE4-N-AI — Live fork-choice wiring (rung-2, single-best-peer)

**Primary invariant:** `DC-NODE-25` (anchored by `DC-NODE-23`/`DC-NODE-24` detect+route, `DC-NODE-26` reconcile, `DC-NODE-27` replay-equiv, `DC-NODE-28` forge-race fence).
**Status:** Active — S1 next. **Authority bundle:** committed `d92f9ce8` (rules) / `80862c7f` (OQ-1→A) / `19676365` (plan), all pushed.
**Rung:** rung-2, **single-best-peer only** (no multi-peer candidate comparison).

## 1. Primary Invariant
**`DC-NODE-25`** (registry) — a peer-origin candidate that wins Praos fork-choice (`DC-CONS-03`) is durably adopted on the live `--mode node` spine **only** via the already-enforced authorities (`materialize_rolled_back_state` `CN-STORE-07` + the lockstep `receive::reducer` `DC-CONS-20` + `pump_block` roll-forward `DC-NODE-05/12`) — no second apply path, no header-only adoption. Supporting authority chain: **`DC-NODE-23`** (shared venue-blind detector), **`DC-NODE-24`** (venue-split resolver — `SingleProducer→refuse`, `Participant→fork-choice`), **`DC-NODE-26`** (selector tip == ChainDb tip after every applied decision), **`DC-NODE-27`** (rollback+reselection replay-equivalence via mechanism A), **`DC-NODE-28`** (no forge across unresolved re-selection). Convergence targets flipped at close: **`CN-CONS-01`** (partial→enforced), **`CN-CONS-03`** (declared→enforced). The BLUE fork-choice/rollback core (`select_best_chain`/`apply_rollback`/`materialize_rolled_back_state`) is **reused, not built** — `DC-CONS-03`/`05`/`06`, `CN-STORE-07`, `DC-CONS-20` already enforced.

## 2. Normative Anchors
- Registry rules `DC-NODE-23`…`DC-NODE-28` (canonical statements); reused `DC-CONS-03`/`05`/`06`, `CN-STORE-07`, `DC-CONS-20`, `DC-NODE-05`/`12`/`20`; convergence `CN-CONS-01`/`03`; replay `T-REC-03`/`05`, `DC-CONS-22`; WAL `CN-WAL-01`, `DC-WAL-02/03`.
- Invariants sketch `docs/planning/phase4-n-ai-live-fork-choice-invariants.md` (I-1..I-12; T-A..T-E).
- OQ-1 decision record `docs/planning/phase4-n-ai-oq1-rollback-durability-decision.md` (**mechanism A**, binding).
- Cluster plan `docs/planning/phase4-n-ai-cluster-slice-plan.md`.
- Gap record `docs/planning/c2-local-discovered-gaps.md` (Gap 1 / Slice B — this cluster; Slice A = Gap 2 closed via N-AE/AF/AH).
- `docs/active/c2-preprod-tip-guide.md` (rung ladder + the competing-producer venue for CE-AI-6).

## 3. Entry Conditions (prior clusters guarantee)
- **BLUE fork-choice/rollback core, enforced + reused unchanged:** `select_best_chain` (`DC-CONS-03`; `ci_check_no_density_in_fork_choice.sh`, `ci_check_no_chaindb_in_consensus_blue.sh`, `ci_check_consensus_closed_enums.sh`), `apply_rollback` (`DC-CONS-05/06`), `materialize_rolled_back_state` (`CN-STORE-07`; `ci_check_rollback_materialize_closure.sh`).
- **Lockstep receive reducer, enforced:** `receive::reducer` admit+`roll_backward` atomic over ChainDb+LedgerState+PraosChainDepState (`DC-CONS-20`; `ci_check_receive_reducer_closure.sh`, `ci_check_receive_orchestrator_no_producer_dep.sh`).
- **Orchestrator, tested:** `ade_runtime::consensus::chain_selector::process_stream_input` over `OrchestratorState` (`StreamInput::{HeaderArrival,RollBack,EpochBoundary}`; replay-tested via `ade_testkit::consensus::stream_replay`).
- **Durable admit, enforced:** `pump_block` sole durable tip authority (`DC-NODE-05/12`; `ci_check_forged_durable_admit_via_pump.sh`, `ci_check_node_run_loop_containment.sh`); receive idempotency (`DC-NODE-16`; `ci_check_receive_idempotency.sh`).
- **WAL, enforced linear:** append-only `WalEntry` 2-variant sum + linear `replay_from_anchor` (`CN-WAL-01`, `DC-WAL-02/03`).
- **Rung-1 fence, enforced:** `DC-NODE-15` initial catch-up + `DC-NODE-20` local-tip forge base, observed-feed competing-block fence fails closed (`ci_check_local_durable_forge_base.sh`, `ci_check_forge_followed_tip_admission.sh`).
- Code anchors: live receive admit = `ade_runtime::forward_sync::pump::pump_block` (extend-only, fails closed on non-linear); the DC-NODE-20 forge fence = `ade_node::node_sync::single_producer_forge_decision`; warm-start WAL-tail reconciliation = `node_lifecycle.rs:~1696`.

## 4. What Changes (design)
**The gap (Gap 1 / Slice B):** the live `--mode node` spine admits via extend-only `pump_block` and **fails closed** (`BlockNoOutOfOrder`) on a competing chain; it never reaches the (already-built) fork-choice/rollback authorities. Two fail-closed points: the `DC-NODE-20` forge fence and the `pump_block` receive.

**The correction (wiring, not building):**
- **AI-S1 — rollback durability foundation (the OQ-1 mechanism, BLUE).** Add a version-gated additive `WalEntry::RollBack { to_point, reason, prior_tip, selected_tip }` (tag 1 — the reserved RollBackward slot). `replay_from_anchor` gains a `RollBack` arm that **re-invokes** the existing `materialize_rolled_back_state` (`CN-STORE-07`) + lockstep reducer (`DC-CONS-20`) and re-anchors `prev_post_fp` to the materialized rolled-back fp — replacing today's `ChainBreak` with a faithful linear-with-rollbacks replay. **NOT a second rollback implementation.** Append-only preserved. *(Without this, a live rollback either `ChainBreak`s on restart or resurrects the abandoned branch — see the OQ-1 record.)*
- **AI-S2 — shared detector + venue-split resolver (GREEN).** Pure total classifier `(durable_tip, candidate_header_summary) → ReceiveDisposition { AlreadyHave | LinearExtend | RefuseSingleProducer | NeedsForkChoice }`, venue-blind; an already-known echo is `AlreadyHave`, never a competing candidate. Venue projection: `SingleProducer → refuse` (the `DC-NODE-20` behavior, byte-unchanged), `Participant → NeedsForkChoice`. Venue is an **explicit closed mode** (`SingleProducer | Participant`); unknown/invalid **fails closed before any fork-choice/forge traffic** — no silent inference of `Participant` from network traffic, no silent inference of `SingleProducer` for a configured participant node.
- **AI-S3 — apply driver + reconciliation (RED+GREEN).** Given a `ChainEvent` (`ChainSelected`/`RolledBack`) from the orchestrator, apply durably: `materialize_rolled_back_state` + lockstep `roll_backward` to the fork point + `WalEntry::RollBack` append (S1) + `pump_block` roll-forward to the new tip. **Header→body coherent:** a fork-choice win is provisional until bodies validate and apply through `pump_block` (no header-only tip advance). Reconcile: orchestrator `selector.current_tip == ChainDb::tip` after every applied decision.
- **AI-S4a — wire rollback signal preservation (RED).** The admission wire pump preserves the peer's chain-sync `RollBackward` **point** as a closed `AdmissionPeerEvent` variant (today `point: _` is discarded, emitting `TipUpdate` only); fail-closed on a malformed/unsupported point; `Block`/`TipUpdate`/`Disconnected` unchanged. **Merges latent** — the live loop does not consume it until AI-S4b. This is wire-signal preservation, NOT fork-choice wiring.
- **AI-S4b — live receive-loop fork-choice wiring + forge gate (RED).** The live receive loop consumes the surfaced rollback signal + classifies (S2) and routes `NeedsForkChoice` (Participant) → orchestrator → apply driver (S3); `SingleProducer` keeps the `DC-NODE-20` fail-closed. **Forge gate (`DC-NODE-28`):** while a re-selection is pending, forging refuses (typed `ForgeRefused`) — never forges on the stale pre-resolution tip (the producer race). The rollback point comes from the peer's chain-sync `RollBackward`, surfaced by AI-S4a (single-best-peer).
- **AI-S5 — convergence evidence + operator pass.** Hermetic: `select_best_chain` over a fixed candidate set is arrival-order-independent (`CN-CONS-01`). Operator-gated: Ade + ≥1 Haskell producer on a competing-producer venue converge on the same tip (`CN-CONS-03`) — a closed derived-tier evidence vocabulary that **does not claim full multi-peer ChainSel coverage**.

## 5. Exit Criteria (CE — each CI-verifiable)
- **CE-AI-1 (`DC-NODE-27` rollback replay-equivalence) [S1]:** new hermetic tests — a WAL with a `RollBack` entry replays **byte-identically** (the rolled-back-then-reselected chain recovers the *selected* tip, never the abandoned branch); the `RollBack` replay arm invokes `materialize_rolled_back_state`/the lockstep reducer (not a reimpl); append-only preserved. New gate `ci/ci_check_wal_rollback_replay_equiv.sh` green; `cargo test -p ade_ledger` green. *(Production half completed by S3.)*
- **CE-AI-2 (`DC-NODE-23`+`DC-NODE-24` detector + venue split) [S2]:** hermetic tests — classifier is total over the 4 dispositions; `SingleProducer→refuse`, `Participant→NeedsForkChoice`; `AlreadyHave` for a known echo; the detector never calls `select_best_chain`; an unknown/invalid venue fails closed (no silent inference). New gate `ci/ci_check_receive_detector_venue_split.sh` green.
- **CE-AI-3 (`DC-NODE-25`+`DC-NODE-26` apply + reconcile) [S3+S4b]:** hermetic — a `ChainSelected` requiring rollback applies via materialize+lockstep+`WalEntry::RollBack`+`pump_block`; no header-only tip advance; `selector.current_tip == ChainDb::tip` after apply; the live Participant path adopts a competing chain end-to-end. New gate `ci/ci_check_live_fork_choice_apply.sh` green; `ci_check_rollback_materialize_closure.sh` + `ci_check_receive_reducer_closure.sh` + `ci_check_forged_durable_admit_via_pump.sh` stay green.
- **CE-AI-4 (`DC-NODE-28` no forge across unresolved re-selection) [S4b]:** hermetic — a producer tick during a pending fork-choice decision yields a typed `ForgeRefused` (never forges on the stale tip); forge resumes only after the decision is applied+reconciled or rejected-unchanged. New gate `ci/ci_check_no_forge_across_pending_reselection.sh` green.
- **CE-AI-5 (`CN-CONS-01` deterministic, arrival-order-independent) [S5]:** hermetic — for a fixed competing-candidate set, the converged tip is the fork-choice-maximal chain regardless of arrival order (a permutation test over the orchestrator/replay). New gate `ci/ci_check_chain_selection_arrival_order_independent.sh` green.
- **CE-AI-6 (`CN-CONS-03` live convergence — operator-gated, derived-tier) [S5]:** committed transcript `docs/evidence/phase4-n-ai-convergence-pass.{md,jsonl}` — Ade + ≥1 Haskell producer on a competing-producer venue converge on the same tip, arrival-order-independent; verbatim `--mode node`. **Proves the exercised venue, NOT full multi-peer Cardano ChainSel.** Evidence schema gate `ci/ci_check_convergence_evidence_schema.sh` (closed vocabulary, vacuous-until-committed, sha256-bound).
- **CE-AI-7 (close) [/cluster-close]:** `DC-NODE-23`/`24`/`25`/`26`/`27`/`28` flipped declared→enforced (tests + ci_scripts appended); `CN-CONS-01` partial→enforced + `CN-CONS-03` declared→enforced; strengthen `DC-CONS-03`/`05`/`06`/`20`, `CN-STORE-07`, `DC-NODE-05`/`12`/`20`, `T-REC-03`/`05`; 4 grounding docs refreshed; cluster archived.

## 6. Expected Slices
- **AI-S1** rollback WAL durability foundation — `WalEntry::RollBack` + encode/decode/version-gate + `replay_from_anchor` RollBack arm (re-invokes existing authority) — CE-AI-1. **BLUE** (`ade_ledger::wal`) + hermetic. **Lands + proven FIRST.**
- **AI-S2** shared detector + venue-split resolver — CE-AI-2. **GREEN** (`ade_node::node_sync`).
- **AI-S3** live fork-choice apply driver + reconciliation — CE-AI-1 (production) + CE-AI-3. **RED+GREEN**; reuses BLUE materialize/lockstep + S1 WAL.
- **AI-S4a** wire rollback signal preservation — CE-AI-2 (wire-signal precursor). **RED** (`ade_runtime::admission`); merges latent (loop does not consume until S4b).
- **AI-S4b** live receive-loop fork-choice wiring + forge gate — CE-AI-3 (live) + CE-AI-4 (+ CE-AI-2 live). **RED** (`node_lifecycle`/`node_sync`).
- **AI-S5** convergence evidence + operator pass — CE-AI-5 (hermetic) + CE-AI-6 (operator-gated). **RED** + hermetic.
- **close** — CE-AI-7 via `/cluster-close`.

## 7. TCB Color Map
- **BLUE:** `ade_ledger::wal` — **AI-S1 only**: the additive `WalEntry::RollBack` variant + encode/decode + the `replay_from_anchor` RollBack arm. *(The cluster's only new BLUE authority surface.)* Reused **unchanged**: `ade_core::consensus::{fork_choice, rollback, candidate}`, `ade_ledger::rollback::{materialize_rolled_back_state, commit_rollback}`, `ade_ledger::receive::reducer`, `validate_and_apply_header`, `pump_block`.
- **GREEN:** `ade_node::node_sync` — the detector + venue resolver (S2) + the decision/durable reconciliation projection (S3); `ade_runtime::consensus::chain_selector` (existing orchestrator, reused).
- **RED:** `ade_node::node_lifecycle` — the apply driver (S3) + the live loop wiring + forge gate (S4); `ade_node` live evidence (S5).
- **Affected gates:** new — `ci_check_wal_rollback_replay_equiv.sh`, `ci_check_receive_detector_venue_split.sh`, `ci_check_live_fork_choice_apply.sh`, `ci_check_no_forge_across_pending_reselection.sh`, `ci_check_chain_selection_arrival_order_independent.sh`, `ci_check_convergence_evidence_schema.sh`. Stay green — `ci_check_rollback_materialize_closure.sh`, `ci_check_receive_reducer_closure.sh`, `ci_check_forged_durable_admit_via_pump.sh`, `ci_check_node_run_loop_containment.sh`, `ci_check_local_durable_forge_base.sh`, `ci_check_consensus_closed_enums.sh`, `ci_check_no_density_in_fork_choice.sh`.

## 8. Forbidden During This Cluster (slice-level hard prohibitions inherit) — the eight hard lines
1. **No live fork-choice wiring (S2–S5) before AI-S1 proves rollback replay-equivalence.** S1 lands + is proven first.
2. **`WalEntry::RollBack` is the ONLY new BLUE authority surface.** No other BLUE change.
3. **The rollback WAL entry is NOT a second rollback implementation** — a durable marker only.
4. **Replay of a `RollBack` entry MUST invoke** `materialize_rolled_back_state` (`CN-STORE-07`) + the lockstep reducer (`DC-CONS-20`).
5. **`pump_block` remains the sole roll-forward durable admit authority** (`DC-NODE-05/12`) — no second durable tip-advance path.
6. **`SingleProducer` stays fail-closed** (`DC-NODE-20` byte-unchanged) — a competing candidate is refused, never resolved.
7. **`Participant` is the ONLY path that reaches fork-choice** (`select_best_chain`/`DC-CONS-03`); a raw `followed_peer_tip` signal never reaches `select_best_chain` (only validated header summaries, only in Participant).
8. **CE-AI-6 is operator-gated, derived-tier** — it proves the exercised competing-producer venue, NOT full multi-peer Cardano ChainSel coverage.
- Plus: **no multi-peer candidate comparison** (single-best-peer only); **no header-only durable adoption** (provisional until bodies apply); **venue is explicit + closed** (no silent default either direction); **no `RO-LIVE` flip** beyond the operator-gated CE-AI-6 transcript.

## 9. Replay Obligations
AI-S1 introduces the **one** new canonical type — `WalEntry::RollBack` (additive, version-gated). New replay-corpus obligation: a WAL containing a `RollBack` entry replays byte-identically and recovers the **selected** chain, never the abandoned branch (CE-AI-1). AI-S3 produces these entries live. Strengthens **T-REC-03/05** and **DC-CONS-06/22** (replay-equivalence now spans rollback+reselection). No new durable surface beyond the WAL variant — rollback reuses materialize/lockstep; `pump_block` stays sole admit. CE-AI-5 adds a determinism/arrival-order-permutation corpus for `select_best_chain`.

## 10. Open Questions
- **OQ-1 → RESOLVED → A** (decision record): version-gated `WalEntry::RollBack` marker re-invoking the existing rollback/materialize authority; option B (WAL-tail reconciliation) rejected.
- **OQ-2 (decision-state ownership) [S3]:** rebuild `ChainSelectorState` from the durable stores per decision (guarantees `DC-NODE-26` reconciliation by construction) vs hold `OrchestratorState` in lockstep with an assertion. *Lean: rebuild-per-decision unless cost forces otherwise.*
- **OQ-3 (rollback-point identification) [S4]:** in single-best-peer follow the rollback point is the peer's chain-sync `RollBackward(point)` → `StreamInput::RollBack` (peer-driven), not Ade-derived.
- **OQ-4 (snapshot availability ≤ fork point within k) [S3]:** confirm the existing snapshot cadence guarantees a snapshot ≤ any reachable fork point within k (DC-CONS-05 bound); else the apply driver fails closed (`ExceededRollback`).
- **OQ-5 (venue declaration) [S2/S4]:** venue must be **explicit and closed**. `SingleProducer` and `Participant` are distinct declared modes. An unknown/invalid venue **fails closed before forging/receiving fork-choice traffic**. The cluster MUST NOT silently infer `Participant` from network traffic, and MUST NOT silently infer `SingleProducer` for a configured participant node — silent defaulting would hide operator/config mistakes. (Reuses the existing `--single-producer-venue` declaration; the participant mode is its own explicit declaration.)
- **OQ-6 (convergence evidence shape) [S5]:** closed, derived-tier vocabulary extending the existing live-transcript style; never overstates (no full-ChainSel claim).

## 11. Cluster Close Record
*(Filled at `/cluster-close`.)* Active — AI-S1 next.

## 12. Follow-ons & Notes
- **Multi-peer candidate comparison (full Cardano ChainSel):** explicitly **out of scope** — a later hardening cluster. This cluster proves one live competing-chain resolution path end-to-end (single-best-peer).
- **OQ-2 resolution** may retire the in-memory `OrchestratorState` duplication if rebuild-per-decision is chosen; record in the S3 slice doc.
