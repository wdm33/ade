# PHASE4-N-AI — Live Fork-Choice Wiring (rung-2, single-best-peer) — Invariant Sketch

> IDD Part I planning artifact. Names invariants before any cluster/slice/code work.
> Produced 2026-06-09. Scope confirmed by the user (Fork 1 = A, Fork 2 = keep-as-mode).

## Concept

Connect the **already-built, already-enforced** fork-choice authority to the live
`--mode node` receive path: when a peer-origin candidate is **not** a linear extension
of Ade's spine, a **Participant** venue resolves it through `select_best_chain` +
rollback (instead of failing closed), and Ade converges on the same tip as the Haskell
peer — **without** weakening the rung-1 single-producer proof.

This is a **wiring** cluster, not a fork-choice **build**. The Praos chain-selection,
rollback, materialize, and lockstep-apply authorities all exist and are enforced
(DC-CONS-03/05/06, CN-STORE-07, DC-CONS-20); a complete single-stream orchestrator
(`ade_runtime::consensus::chain_selector::process_stream_input`) exists and is unit+replay
tested. None of it is reached from the live spine, which admits via extend-only
`forward_sync::pump_block` and fails closed (`BlockNoOutOfOrder`) on a competing chain —
the Gap-1 / Slice-B gap recorded in `docs/planning/c2-local-discovered-gaps.md`
(Slice A = Gap 2 forge-on-followed-tip + serve-continuity is done via N-AE/AF/AH).

### Scope fence (binding)

- **PHASE4-N-AI = live fork-choice wiring for single-best-peer follow.**
- **No** new BLUE fork-choice (reuse `select_best_chain` / `apply_rollback` / `materialize_rolled_back_state`).
- **No** multi-peer candidate comparison (later hardening slice).
- **No** weakening of the SingleProducer fail-closed mode (DC-NODE-20 preserved byte-unchanged).
- **No** durable header-only adoption (a fork-choice win is provisional until bodies apply).
- **No** rollback without replay-equivalent durability (OQ-1 must be resolved first).

### Pure-transformation check (IDD hard rule)

The decision **is** a pure `canonical input → canonical output` — it already is, in BLUE
(`select_best_chain`, `apply_rollback`). The only nondeterminism is **network arrival
order**, which is RED and must enter as a *canonical ordered receive-event sequence*; the
fork-choice **outcome must be arrival-order-independent**. The concept is well-formed; the
risk is entirely in the wiring, not the semantics.

---

## 1. What must always be true

- **I-1 — single fork-choice authority.** Every caught-up chain-selection decision on the
  live `--mode node` spine is made by exactly one authority — BLUE `select_best_chain`
  (DC-CONS-03), reached through the existing `chain_selector` orchestrator. No second,
  parallel, or hand-rolled selection.
- **I-2 — single durable-admit authority preserved.** Roll-*forward* onto the selected
  chain still goes through `pump_block` (DC-NODE-05/12). Rung-2 adds no second durable
  tip-advance path.
- **I-3 — single rollback-materialize authority preserved.** Any durable rollback
  materializes `(LedgerState, PraosChainDepState)` only via `materialize_rolled_back_state`
  (CN-STORE-07). No parallel rolled-back-state computation.
- **I-4 — lockstep apply.** A reselection updates ChainDb + LedgerState + PraosChainDepState
  as one structural transition; a rollback rolls all three back to the same point
  (DC-CONS-20). No partial admission, no partial rollback.
- **I-5 — shared detector.** "Peer-origin candidate ∉ Ade's admitted spine / own-served
  lineage" is computed once, venue-blind, as a pure predicate over
  `(durable_tip, candidate_summary)`.
- **I-6 — venue-split resolver.** The detector's consequent is venue-gated and total:
  `SingleProducer → refuse` (DC-NODE-20, byte-unchanged); `Participant → NeedsForkChoice →
  orchestrator`. Every venue has exactly one defined resolver.
- **I-7 — decision↔durable reconciliation.** After any applied receive decision, the
  orchestrator's `selector.current_tip` equals the durable `ChainDb::tip`. The in-memory
  decision state never diverges from the persisted authority.
- **I-8 — rollback safety bounds.** A live rollback never exceeds `k` (DC-CONS-05) and never
  crosses the immutable tip (DC-CONS-06). The immutable (k-deep) tip is never rewritten.
- **I-9 — forge base = fork-choice winner.** In Participant mode, the forge base is the
  fork-choice-selected durable tip. DC-NODE-20's "selected tip" stops being degenerate
  (local head) and becomes the genuine `select_best_chain` winner.
- **I-10 — convergence (derived tier).** Given the same set of competing chains, Ade and the
  Haskell peer select the same tip, using only protocol-defined observables (CN-CONS-01 →
  enforced, CN-CONS-03 → enforced).
- **I-11 — header→body coherence.** A chain is durably adopted only when its **bodies**
  validate and apply through `pump_block`. A header-only fork-choice win is provisional; no
  tip advances on headers alone.
- **I-12 — no forge across unresolved re-selection.** Once a peer-origin candidate is
  classified `NeedsForkChoice`, Participant-mode forging is **disabled** until the
  fork-choice outcome is either (a) durably applied and reconciled (I-7) **or** (b) rejected
  with durable state unchanged. The forge base is never selected from a stale
  pre-resolution `ChainDb::tip` while a decision is pending. **Tier: derived, with a
  true-tier authority consequence** — it prevents stale local authority leaking into block
  production (an authority race: candidate arrives → fork-choice pending → producer tick
  fires → Ade forges on the old local tip, even if the orchestrator later chooses
  correctly).

## 2. What must never be possible

- A competing peer candidate **adopted via fork-choice in a SingleProducer venue** (must
  fail closed — DC-NODE-20 preserved).
- The orchestrator's selector tip **diverging** from the durable ChainDb tip after an
  applied decision (I-7 violated).
- A **second** chain-selection path, a **second** durable tip-advance path, or a **second**
  rollback-materialize path (I-1/I-2/I-3).
- A rollback that **crosses the immutable tip** or **exceeds k** (must return
  `ForkBeforeImmutableTip` / `ExceededRollback`, state unchanged).
- A **header-only tip advance** — adopting a chain whose bodies were never
  fetched/validated/applied (I-11).
- **Forging on a stale tip while a fork-choice decision is pending** (I-12).
- A **raw `followed_peer_tip` signal** reaching `select_best_chain` — only *validated*
  header summaries become candidates, and only in Participant mode (rung-1's "tip signal is
  admissibility-only" survives in SingleProducer mode).
- A **rolled-back block re-served or re-applied** — the durable-chain serve projection
  (DC-NODE-13) must reflect the reselected chain, not the abandoned branch.
- **WAL mutation other than append** (CN-WAL-01) — the rollback representation must respect
  append-only durability (see OQ-1).
- Fork-choice **outcome depending on block arrival order, wall-clock, or scheduler**.

## 3. What must remain identical across executions (deterministic surface)

- The detector classification: `(durable_tip, candidate_summary, venue) → ReceiveDisposition`
  — pure, total.
- The fork-choice outcome: `select_best_chain(selector_state, candidates)` — already pure
  (block-no then TiebreakerView).
- The rollback target + the materialized `(LedgerState, PraosChainDepState)` at that target.
- **Arrival-order independence:** for a fixed *set* of observed competing chains, the
  converged tip is the fork-choice-maximal chain regardless of the order Ade observed them.

## 4. What must be replay-equivalent

- The ordered live receive-event sequence (RollForward headers, RollBackward points, body
  deliveries), replayed against the same bootstrap anchor + durable log, must produce a
  **byte-identical** durable tip + ledger fingerprint + chain_dep — **including any
  rollback+reselection** (strengthens T-REC-03/05, DC-CONS-06/22).
- This is the binding constraint behind **OQ-1**: a rollback that happened live must be
  reproducible on recovery.

## 5. State transitions in scope

All as `(prior, input) → Result<(new, effects), error>`. The *decision* transitions are
BLUE/GREEN and already exist; the *application* transition is the new RED wiring.

- **T-A (detect):** `(durable_tip, candidate_header_summary, venue) → ReceiveDisposition`
  where `ReceiveDisposition ∈ { AlreadyHave, LinearExtend, RefuseSingleProducer,
  NeedsForkChoice }`. Pure, total. *(GREEN; new)*
- **T-B (resolve):** `(OrchestratorState, StreamInput, ledger_view, era_schedule) →
  Result<(OrchestratorState', Option<ChainEvent>), OrchestratorError>` = the existing
  `process_stream_input`. *(BLUE decides via select_best_chain/apply_rollback; GREEN
  sequences; reused)*
- **T-C (apply):** `(durable_stores, ChainEvent) → Result<(durable_stores', effects),
  ApplyError>`:
  - `ChainSelected{new_tip, replaced_tip}` requiring rollback → `materialize_rolled_back_state`
    + lockstep `roll_backward` to the fork point, then roll-forward (block-fetch bodies +
    `pump_block`) to `new_tip`.
  - `RolledBack{to_point, depth}` → lockstep `roll_backward` to `to_point`.
  - `Rejected{TiebreakerLossKeepCurrent}` → no durable change.
  *(RED driver over BLUE lockstep reducer + materialize; new wiring, no new authority)*
- **T-D (reconcile):** `durable_stores → ChainSelectorState` — project the orchestrator's
  decision state from the persisted authority so I-7 holds. *(GREEN; new — or RED if it must
  hold state; see OQ-2)*
- **T-E (forge base + gate):** `(durable_tip, venue, fork_choice_state) → ForgeBase | ForgeRefused`
  — Participant forge base = reselected durable tip; **refuse while a fork-choice decision is
  pending** (I-12). *(GREEN; strengthens DC-NODE-20 / DC-NODE-05)*

## 6. TCB color hypothesis

- **BLUE — reused, must NOT be rebuilt:** `select_best_chain`, `apply_rollback`,
  `materialize_rolled_back_state`, `validate_and_apply_header`, `receive::reducer` lockstep
  transitions, `apply_block`. **Expected new BLUE: zero** (the N-AE…N-AH pattern). Possible
  exception: a version-gated `WalEntry::RollBack` variant if OQ-1 lands on the explicit
  marker (additive, closed, canonical — BLUE codec surface).
- **GREEN — new glue:** the T-A detector predicate; the T-D reconciliation projection; the
  venue→resolver mapping; the I-12 forge gate (the orchestrator sequencing is already GREEN).
- **RED — new shell:** the live wiring in `node_lifecycle`/`node_sync` — drive the chain-sync
  stream into detect→resolve→apply; block-fetch the selected chain's bodies; the rollback
  application driver; convergence-evidence emission.
- **OPEN colors:** (a) where the orchestrator's in-memory `OrchestratorState` lives relative
  to the durable stores (GREEN projection rebuilt per decision vs. RED-held state kept in
  lockstep) — OQ-2; (b) whether replay-equivalent rollback needs a BLUE `WalEntry` variant
  or a RED WAL-tail reconciliation — OQ-1.

## 7. Open questions (must resolve before `/cluster-plan`)

### OQ-1 — THE crux: WAL/durability across rollback (replay-equivalence)

The enforced rollback path (DC-CONS-20 / CN-STORE-07) is **snapshot-based**
(`receive::reducer` + `materialize` + `ade_runtime::rollback::*`). The live `--mode node`
durability is **WAL-based** (`forward_sync::pump_block`, append-only — CN-WAL-01). Rung-2
must reconcile these so a live rollback is **replay-equivalent**. If live rollback is not
replay-equivalent, rung-2 can appear to work live and still fail the constitution after
restart.

**Steer (user, 2026-06-09): prefer an explicit rollback WAL event** unless existing recovery
already has a mechanically-proven equivalent. Candidate shape:

```
WalEntry::RollBack { to_point, reason, prior_tip, selected_tip }
```

with **three hard constraints**:
1. **append-only only** (no WAL mutation other than append — CN-WAL-01);
2. **canonical bytes** (version-gated additive variant; deterministic CBOR);
3. **replay re-applies the same rollback through the same materialize/reducer authority** —
   the WAL event is **not** a second rollback implementation. It is only the durable record
   that says: *during replay, invoke the existing rollback/materialize authority at this
   point.*

The alternative — **WAL-tail reconciliation** — is viable **only if** the current recovery
path already has an auditable, tested way to drop the orphaned tail after a rollback and
reproduce the same ledger/chain_dep state byte-identically. If that proof is not already
clean, do **not** lean on it. **No implicit live-only rollback.**

**Pre-slice proof obligation (read before deciding A vs B):**
- `crates/ade_ledger/src/receive/reducer.rs` (the rollback / `roll_backward` path)
- `crates/ade_runtime/src/rollback/*` (cadence, in_memory_cache, chaindb_block_source, snapshot_writer)
- the WAL recovery path (`crates/ade_runtime/src/recovery/restart.rs`, `wal/`)
- the N-Y / T-REC-05 orphan-drop behavior (`warm_start_recovery` + `rollback_to_slot`)
- `pump_block` append semantics (`crates/ade_runtime/src/forward_sync/pump.rs`)

Then decide: **A.** explicit rollback WAL marker, or **B.** existing tail reconciliation —
**only with proof**.

### OQ-2 — decision-state ownership

Does the live spine hold a single long-lived `OrchestratorState`, or rebuild
`ChainSelectorState` from the durable stores per decision (T-D)? Decides I-7's enforcement
shape and the GREEN/RED split.

### OQ-3 — rollback-point identification on the live wire

In single-best-peer follow, does the rollback point come from the peer's chain-sync
`RollBackward` directly, or must Ade compute the fork point between its own forged branch and
the peer's chain? (Affects whether `StreamInput::RollBack` is peer-driven or Ade-derived.)

### OQ-4 — snapshot availability for the rollback target

`apply_rollback` and the orchestrator both need a snapshot/materialization *at the fork
point*. Does the live spine's snapshot cadence guarantee a snapshot within `k` of any
reachable fork point? (Ties to DC-CONS-05 and the cadence infra.)

### OQ-5 — venue declaration

How is `Participant` vs `SingleProducer` declared at the live node (CLI flag, reusing the
existing `--single-producer-venue`)? The detector is shared; the venue input must be explicit
and fail-safe (default to the more conservative resolver).

### OQ-6 — convergence evidence shape

What is the closed, committed evidence artifact for CN-CONS-03 (Ade + Haskell converge on the
same tip, arrival-order-independent) — extending the existing live-transcript vocabulary,
derived-tier, never overstating?

---

## 8. Registry impact (proposed — declared at this phase, enforced as slices land)

### Strengthenings (`strengthened_in += "PHASE4-N-AI"`)

`CN-CONS-01` (partial→enforced), `CN-CONS-03` (declared→enforced), `DC-CONS-03`,
`DC-CONS-20`, `CN-STORE-07`, `DC-CONS-05`, `DC-CONS-06`, `DC-NODE-05`, `DC-NODE-12`,
`DC-NODE-20`, `T-REC-03`, `T-REC-05`.

### New rules (status = declared; `introduced_in = "PHASE4-N-AI"`)

| ID | Statement (one line) |
|---|---|
| `DC-NODE-23` | Shared receive detector: a peer-origin candidate ∉ Ade's admitted spine is classified once, venue-blind, as a pure predicate. |
| `DC-NODE-24` | Venue-split resolver: `SingleProducer → refuse` (DC-NODE-20 preserved); `Participant → NeedsForkChoice → select_best_chain` via the existing orchestrator. Total over venues. |
| `DC-NODE-25` | Live fork-choice durable application authority: a `ChainSelected`/`RolledBack` outcome is applied to the durable stores **only** via the lockstep reducer (DC-CONS-20) + materialize (CN-STORE-07) + `pump_block` roll-forward — no second apply path; no header-only adoption. |
| `DC-NODE-26` | Decision↔durable reconciliation: the orchestrator selector tip equals the durable `ChainDb::tip` after every applied decision. |
| `DC-NODE-27` | Rollback+reselection replay-equivalence: the replayed receive-event sequence reproduces a byte-identical durable tip + ledger fp + chain_dep including rollbacks; the durable rollback record re-invokes the same materialize/reducer authority on replay (OQ-1). |
| `DC-NODE-28` *(proposed)* | No forge across unresolved re-selection (I-12): Participant-mode forging is disabled while a fork-choice decision is pending; the forge base is never the stale pre-resolution tip. *(Alternative: fold into DC-NODE-05 / DC-NODE-24 strengthening — user's call.)* |

DC-NODE-26 and DC-NODE-27 are kept **separate** (per user steer): reconciliation and
replay-equivalence are related but fail differently and need different evidence.
