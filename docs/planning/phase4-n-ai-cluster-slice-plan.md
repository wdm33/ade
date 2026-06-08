# Cluster/Slice Plan — Ade · PHASE4-N-AI

> IDD cluster-planning artifact (overall plan only; full cluster doc is `/cluster-doc`).
> Produced 2026-06-09 from the confirmed invariants sketch
> (`phase4-n-ai-live-fork-choice-invariants.md`) + the OQ-1 decision record
> (`phase4-n-ai-oq1-rollback-durability-decision.md`, mechanism A). Per-cluster plan —
> does NOT overwrite the global `docs/active/phase_4_cluster_plan.md`.

## Cluster Index (Dependency Order)

1. **PHASE4-N-AI** — Live fork-choice wiring (rung-2, single-best-peer follow + rollback) —
   primary invariant: *a peer-origin candidate that wins Praos fork-choice (DC-CONS-03) is
   durably adopted on the live `--mode node` spine via rollback+rollforward through the
   existing enforced authorities — replay-equivalently — while SingleProducer venues stay
   fail-closed.*

A single cluster. The user's hard prohibition (no live fork-choice wiring before the rollback
durability foundation is mechanically proven) is encoded as **slice ordering**: AI-S1 lands
and is proven first; AI-S2..S5 may not merge before it. This is the constitutional guard —
without S1, live fork-choice can appear to work and still fail after restart (the abandoned
branch resurrects; see the OQ-1 decision record).

## PHASE4-N-AI — Live fork-choice wiring (single-best-peer)

- **Primary invariant:** above.

- **TCB partition:**
  - **BLUE** — `ade_ledger::wal` (AI-S1 only: `WalEntry::RollBack` + encode/decode + replay
    arm). *Reused unchanged:* `ade_core::consensus::{fork_choice::select_best_chain,
    rollback::apply_rollback, candidate}`, `ade_ledger::rollback::{materialize_rolled_back_state,
    commit_rollback}`, `ade_ledger::receive::reducer` (lockstep), `validate_and_apply_header`.
  - **GREEN** — `ade_node::node_sync` (AI-S2 detector + venue resolver; AI-S3 reconciliation
    projection); `ade_runtime::consensus::chain_selector` (existing orchestrator, reused).
  - **RED** — `ade_node::node_lifecycle` (AI-S3 apply driver; AI-S4 loop wiring + forge gate);
    `ade_node` live evidence (AI-S5).

- **Cluster Exit Criteria:**
  - **CE-AI-1** — Rollback replay-equivalence: a durable chain that underwent a live
    rollback+reselection replays **byte-identically** from anchor+WAL (the abandoned branch
    never resurrects on restart); a `WalEntry::RollBack` re-invokes the *existing*
    materialize/lockstep authority on replay. *(DC-NODE-27)*
  - **CE-AI-2** — Shared detector + venue split: the receive detector is **total + venue-blind**;
    `SingleProducer → refuse` (DC-NODE-20 byte-unchanged), `Participant → NeedsForkChoice →
    orchestrator`; `AlreadyHave`/`LinearExtend` unchanged. *(DC-NODE-23 + DC-NODE-24)*
  - **CE-AI-3** — Live fork-choice durable application + reconciliation: a competing peer chain
    that wins `select_best_chain` is durably adopted via materialize + lockstep rollback +
    `WalEntry::RollBack` + `pump_block` roll-forward; **provisional until bodies apply** (no
    header-only tip advance); orchestrator selector tip **== ChainDb tip** after every applied
    decision. *(DC-NODE-25 + DC-NODE-26)*
  - **CE-AI-4** — No forge across unresolved re-selection: forge refuses (typed `ForgeRefused`)
    while a decision is pending; never forges on the stale pre-resolution tip. *(DC-NODE-28)*
  - **CE-AI-5** — Deterministic, arrival-order-independent selection (**hermetic**): for a fixed
    set of competing candidates, the converged tip is the fork-choice-maximal chain regardless
    of arrival order. *(CN-CONS-01)*
  - **CE-AI-6** — Live convergence (**operator-gated, derived-tier evidence**): Ade + ≥1 Haskell
    producer on a competing-producer venue converge on the same tip, arrival-order-independent.
    It proves the **selected live behavior for the exercised competing-producer venue**; it does
    **NOT** claim full multi-peer Cardano ChainSel coverage. *(CN-CONS-03)*

- **Slices:**
  - **AI-S1 — Rollback WAL durability foundation** — invariant: a live rollback is recorded as a
    canonical, append-only `WalEntry::RollBack` whose replay re-invokes the existing
    materialize/lockstep authority and re-anchors the fingerprint chain — addresses CE-AI-1
    (mechanism) — **TCB: BLUE** (`ade_ledger::wal`). *Must land + be proven FIRST — the hard
    prohibition gate.*
  - **AI-S2 — Shared detector + venue-split resolver** — invariant: a peer-origin candidate ∉
    admitted spine is classified once (venue-blind, total) and routed `SingleProducer → refuse` /
    `Participant → NeedsForkChoice` — addresses CE-AI-2 (classifier) — **TCB: GREEN**
    (`ade_node::node_sync`).
  - **AI-S3 — Live fork-choice apply driver + reconciliation** — invariant: a `ChainEvent`
    (ChainSelected/RolledBack) is applied to durable stores only via materialize + lockstep +
    `WalEntry::RollBack` + `pump_block` roll-forward, header→body coherent, selector tip ==
    ChainDb tip after apply — addresses CE-AI-1 (production) + CE-AI-3 — **TCB: RED + GREEN
    (reused BLUE)**. *Resolves OQ-2 (decision-state ownership), OQ-4 (snapshot ≤ fork point
    within k).*
  - **AI-S4 — Live receive-loop wiring + forge gate** — invariant: the live receive loop drives
    detector → orchestrator → apply driver on the Participant path (SingleProducer fail-closed
    unchanged), and forging refuses while a decision is pending — addresses CE-AI-2 (live) +
    CE-AI-3 (live) + CE-AI-4 — **TCB: RED** (`ade_node::node_lifecycle`/`node_sync`). *Resolves
    OQ-3 (peer-driven rollback point), OQ-5 (venue declaration + fail-safe default).*
  - **AI-S5 — Convergence evidence + operator pass** — invariant: chain selection is
    deterministic + arrival-order-independent (hermetic), and Ade converges with a Haskell
    producer on the same tip (operator-gated), via a closed derived-tier evidence vocabulary
    that never overstates — addresses CE-AI-5 (hermetic) + CE-AI-6 (operator-gated) — **TCB:
    RED + hermetic test**. *Resolves OQ-6 (evidence shape).*

- **Replay obligations:**
  - **New canonical type:** `WalEntry::RollBack` (additive, version-gated — the *one* new BLUE
    type). **New replay-corpus obligation:** a WAL containing a `RollBack` entry replays
    byte-identically (CE-AI-1). Strengthens **T-REC-03/05, DC-CONS-06/22**.
  - **No new authoritative durable surface** beyond the WAL variant — rollback reuses
    materialize/lockstep; `pump_block` stays the sole durable admit.
  - **Determinism corpus:** arrival-order-independence of `select_best_chain` over a fixed
    candidate set (CE-AI-5).

- **Strengthenings (applied at cluster close):** CN-CONS-01/03, DC-CONS-03/05/06/20,
  CN-STORE-07, DC-NODE-05/12/20, T-REC-03/05.

## Discipline checks

- **Complete-work-only / no carry-forward:** every CE is reachable within this cluster. CE-AI-6
  is operator-gated, consistent with the project's live-CE tier doctrine (as CE-AH-6 was) — the
  slice ships the harness + the closed evidence vocabulary + the hermetic arrival-order proof
  (CE-AI-5); the operator executes the live convergence pass.
- **Mergeable units:** each slice independently leaves the system fully correct. AI-S1/S2/S3 add
  tested capability that is latent until AI-S4 wires it; AI-S4 is the go-live behavior change
  (SingleProducer unchanged, Participant resolves, forge-safe via DC-NODE-28); AI-S5 closes with
  evidence + the operator pass. No slice temporarily weakens an invariant (the forge-race fence
  DC-NODE-28 lands in the same slice that introduces the pending-decision state).
- **The one BLUE touch:** AI-S1's `WalEntry::RollBack` + replay arm, sanctioned by the existing
  WAL-is-additively-evolvable seam doctrine (SEAMS §7 candidate #9). Everything else reuses the
  already-enforced fork-choice / rollback / materialize / lockstep authorities.

## Open questions threaded to slices

- **OQ-2** (decision-state ownership: rebuild `ChainSelectorState` per decision vs hold
  `OrchestratorState` in lockstep) → AI-S3.
- **OQ-3** (rollback-point identification: peer chain-sync `RollBackward` vs Ade-derived fork
  point) → AI-S4.
- **OQ-4** (snapshot availability ≤ the fork point within k; DC-CONS-05 bound) → AI-S3.
- **OQ-5** (venue declaration: reuse `--single-producer-venue`; default fail-safe to the
  conservative SingleProducer arm) → AI-S2/AI-S4.
- **OQ-6** (convergence evidence shape: closed, derived-tier, non-overstating) → AI-S5.
- **OQ-1** — RESOLVED → A (decision record). The mechanism is fixed; AI-S1 implements it.
