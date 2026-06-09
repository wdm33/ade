# Invariant Slice AI-S3 — Live fork-choice apply driver + reconciliation

> Slice of PHASE4-N-AI. AI-S1 (`WalEntry::RollBack` + rollback-aware replay) and AI-S2
> (detector + resolver) are closed + pushed (`47c0f487`). RED apply driver; **latent until
> AI-S4 wires it into the loop** (no live behavior change on merge). The heaviest slice.

## 2. Slice Header
- **Slice Name:** Live fork-choice apply driver (durable rollback/extend over `ForwardSyncState`)
  + decision/durable reconciliation.
- **Cluster:** PHASE4-N-AI. **Status:** Proposed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-AI-1** (`DC-NODE-27` rollback replay-equivalence) — **live-production half** (the live
    `WalEntry::RollBack` is now *produced* on durable rollback + proven replay-equivalent
    hermetically; AI-S1 was the mechanism).
  - [ ] **CE-AI-3** (`DC-NODE-25` durable apply authority + `DC-NODE-26` reconciliation).
- **Slice Dependencies:** AI-S1 (`WalEntry::RollBack`), AI-S2 (resolver — not a code dep here).

## 4. Intent
A fork-choice outcome is applied to the durable stores **only** through the existing enforced
authorities, so that a competing chain that wins is durably adopted (rollback + extend)
**replay-equivalently**, and the orchestrator's decision state can never durably diverge from
the persisted ChainDb.

## 5. Scope
- **Modules / crates:** RED `ade_node::node_lifecycle` (the `apply_chain_event` driver); GREEN
  `ade_node::node_sync` (the reconciliation assertion helper). Reuses BLUE
  `ade_ledger::rollback::{materialize_rolled_back_state, commit_rollback}`,
  `ade_ledger::wal::WalEntry::RollBack`, `ade_runtime::forward_sync::pump_block`; the RED
  `ade_runtime::rollback::*` SnapshotReader/BlockSource impls; the live `ForwardSyncState`.
- **Persistence impact:** *produces* `WalEntry::RollBack` (the AI-S1 record) on a rollback;
  otherwise reuses the existing ChainDb/WAL admit + rollback writers.
- **Network-visible impact:** none (latent until AI-S4).
- **Out of scope:** the live receive-loop wiring that produces `ChainEvent`s + block-fetches
  bodies + holds the `OrchestratorState` — **AI-S4**. Production warm-start rollback-aware
  recovery rewire — **AI-S4**. Convergence evidence — AI-S5.

## 6. Execution Boundary (TCB color)
- **RED:** `ade_node::node_lifecycle::apply_chain_event` (the composition driver — owns no
  decision, only sequences the BLUE authorities over the live `fwd`).
- **GREEN:** `ade_node::node_sync` reconciliation assertion (`fwd` durable tip == applied target
  — pure comparison).
- **BLUE (reused, unchanged):** `materialize_rolled_back_state` (CN-STORE-07), `commit_rollback`
  (DC-CONS-20), `pump_block` (DC-NODE-05/12), `WalEntry::RollBack` (AI-S1). **No new BLUE.**

## 7. Invariants Preserved (registry IDs)
`DC-CONS-20` (lockstep — `commit_rollback` applies ChainDb+ledger+chain_dep atomically, reused
not changed), `CN-STORE-07` (single materialize authority — reused), `DC-CONS-05/06` (rollback ≤
k / byte-identical — reused bounds), `DC-NODE-05/12` (pump_block sole durable admit — roll-forward
goes through it), `CN-WAL-01`/`DC-WAL-02` (append-only WAL + fp chain), `T-REC-03/05`/`DC-CONS-22`
(replay-equivalence), `DC-NODE-20` (SingleProducer fail-closed — untouched), `DC-CONS-03`
(fork-choice — untouched; the orchestrator/AI-S4 owns `select_best_chain`; this driver only
*applies* a decision).

## 8. Invariants Strengthened / Introduced
- **Introduces** `DC-NODE-25` (live fork-choice durable application authority) + `DC-NODE-26`
  (decision/durable reconciliation) — enforced hermetically here.
- **Completes the live-production half of `DC-NODE-27`** (CE-AI-1): the live `WalEntry::RollBack`
  is now *produced* during durable rollback and replay-equivalence is proven hermetically (AI-S1
  mechanism + AI-S3 production). **Full live restart/recovery closure of DC-NODE-27 remains
  AI-S4/S5 evidence + cluster close** (the production warm-start rollback-aware recovery rewire is
  AI-S4). One family: durable application of a fork-choice decision.

## 9. Design Summary (grounded; the central questions resolved)
**Live-spine state (resolved):** the live spine holds a `ForwardSyncState { receive: ReceiveState,
prior_fp, cadence, last_checkpoint }` (node_lifecycle `fwd`); `fwd.receive` is the BLUE
`ReceiveState` (`ledger` + `chain_dep` + `pending_headers`) — **the exact type `commit_rollback`
mutates.** ∴ AI-S3 **reuses `commit_rollback`** on `fwd.receive` — *not* a second rollback path;
ChainDb writes go through a `ChainDbWriter` (as `pump_block` does).

**`apply_chain_event(...)`** is applied **per `ChainEvent`** (the orchestrator emits these
separately — `process_stream_input`):

- **`RolledBack { to_point, depth }`** — strict ordering (the WAL record is appended ONLY after
  the durable rollback has succeeded, so a crash can never record a rollback the stores did not
  apply):
  1. `materialize_rolled_back_state(to_point)` (CN-STORE-07; via the live SnapshotReader +
     BlockSource) → `(new_ledger, new_chain_dep)`.
  2. `commit_rollback(&mut fwd.receive, target, new_ledger, new_chain_dep, &mut chain_writer)`
     (DC-CONS-20 lockstep over ChainDb + ledger + chain_dep) — irreversible-step-first; on failure
     `fwd.receive` + ChainDb are unchanged.
  3. re-anchor `fwd.prior_fp = fingerprint(new_ledger).combined`.
  4. **append `WalEntry::RollBack`** (AI-S1) — the durable record that makes the rollback
     replay-equivalent (CE-AI-1 production). **NEVER before step 2 succeeds.**
  5. reconcile: assert the durable `ChainDb::tip` == `to_point`.
- **`ChainSelected { new_tip, .. }`** → roll FORWARD by admitting `new_tip`'s block via
  `pump_block` (DC-NODE-05/12) — **header→body coherent**: `pump_block` validates+applies the
  body; no tip advance without a validated body. Then reconcile (tip == `new_tip`).
- **`Rejected { .. }`** (TiebreakerLossKeepCurrent) → **no durable change.**

**Reconciliation (DC-NODE-26):** after any applied event, the durable `ChainDb::tip` equals the
event's resulting target (`to_point` for `RolledBack`; `new_tip` for `ChainSelected`). A mismatch
is a **fail-fast** `ApplyError::ReconciliationMismatch` (never a silent divergence). *(OQ-2: the
`OrchestratorState`/selector is rebuilt-per-decision by AI-S4 from `fwd`'s durable tip +
chain_dep, so the decision view never diverges going in; this driver guarantees the durable side
matches the decision coming out.)*

**Fail-closed bounds (OQ-4):** the rollback target depth is bounded by the reused authorities —
`materialize_rolled_back_state` returns `RollbackTooDeep` when no snapshot ≤ target within
retention, and the orchestrator's `apply_rollback` returns `ExceededRollback` /
`ForkBeforeImmutableTip` beyond k / below the immutable tip. `apply_chain_event` **propagates
these as `ApplyError`, applying nothing** (no partial). The snapshot cadence (existing
`SnapshotCadence` in `fwd`) provides the ≤-target snapshot.

## 10. Changes Introduced
- **Types (RED):** `ApplyError` (closed: `Materialize`, `CommitRollback`, `Pump`,
  `ReconciliationMismatch { expected, actual }`, `RollForwardIncomplete`); `AppliedTip` (the
  resulting durable tip). No new BLUE; no new canonical type.
- **Function (RED):** `apply_chain_event` in `node_lifecycle`. **GREEN helper:** a pure
  `reconcile(durable_tip, expected) -> bool` in `node_sync`.
- **Persistence:** produces `WalEntry::RollBack` (AI-S1 variant) on `RolledBack`, *after*
  `commit_rollback` succeeds. No new WAL shape.
- **No** loop wiring, no block-fetch, no `OrchestratorState` ownership (AI-S4).

## 11. Replay / Crash / Epoch Validation
- **Replay (reuses AI-S1):** a hermetic test applies `RolledBack` then `ChainSelected` to a live
  `fwd`, then replays the resulting WAL (incl. the produced `WalEntry::RollBack`) via
  `replay_from_anchor` → byte-identical recovered tip/ledger-fp/chain_dep; the abandoned branch
  never resurrects. Tests: `apply_rolledback_then_extend_replays_byte_identical`,
  `apply_rolledback_produces_wal_rollback_entry`.
- **Crash:** the WAL record is appended only after `commit_rollback` succeeds (so a crash never
  records a rollback the stores did not apply); the produced `WalEntry::RollBack` + the
  (AI-S4) rollback-aware warm-start recover the reselected chain. AI-S3 proves the record is
  produced correctly + replay-equivalent; the production recovery wiring is AI-S4.
- **Epoch:** not applicable here (epoch nonce handled by the orchestrator/chain_dep).

## 12. Mechanical Acceptance Criteria
- [ ] `apply_rolledback_rolls_back_durable_state_via_commit_rollback` — ChainDb +
  `fwd.receive.ledger` + `fwd.receive.chain_dep` rolled to `to_point`; `fwd.prior_fp` re-anchored
  to the materialized fp.
- [ ] `apply_rolledback_produces_wal_rollback_entry` — exactly one `WalEntry::RollBack` appended
  (append-only), *after* `commit_rollback`.
- [ ] `apply_rolledback_does_not_append_wal_on_failed_commit_rollback` — `commit_rollback` fails →
  **no** `WalEntry::RollBack` appended, `fwd.receive` unchanged, ChainDb unchanged →
  `ApplyError::CommitRollback`. (Append-only durability never lies about state.)
- [ ] `apply_rolledback_then_extend_replays_byte_identical` — replay via `replay_from_anchor`
  recovers the reselected tip, never the abandoned branch (CE-AI-1 production).
- [ ] `apply_chain_selected_extends_via_pump_block` — `new_tip` admitted through `pump_block` (no
  second admit path).
- [ ] `apply_rejected_makes_no_durable_change` — ChainDb/WAL/ledger unchanged.
- [ ] `apply_reconciliation_mismatch_fails_fast` — a post-apply durable tip ≠ expected →
  `ApplyError::ReconciliationMismatch` (DC-NODE-26).
- [ ] `apply_rollback_beyond_k_fails_closed` — `materialize`/`apply_rollback` bound → `ApplyError`,
  nothing applied (DC-CONS-05/06).
- [ ] `apply_chain_selected_invalid_body_no_tip_advance` — a `pump_block` body-validation failure
  leaves the tip unadvanced (no header-only adoption).
- [ ] New gate **`ci/ci_check_live_fork_choice_apply.sh`**: `apply_chain_event` calls
  `commit_rollback` + `materialize_rolled_back_state` + `pump_block` (reuse, not reimplement);
  appends `WalEntry::RollBack` on rollback; contains no `select_best_chain` (the driver applies,
  never selects); no second `rollback_to_slot`/admit path. `ci_check_rollback_materialize_closure.sh`
  + `ci_check_receive_reducer_closure.sh` + `ci_check_forged_durable_admit_via_pump.sh` stay green.
- [ ] `cargo test -p ade_node` green.

## 13. Failure Modes (all deterministic; fail-fast)
`ApplyError::Materialize` (RollbackTooDeep / replay-failed), `CommitRollback` (ChainDb write
failed — `commit_rollback`'s irreversible-step-first leaves `fwd.receive` + ChainDb unchanged,
**and no WAL record is appended**), `Pump` (roll-forward body invalid → tip unadvanced),
`ReconciliationMismatch` (fail-fast), `RollForwardIncomplete` (durable tip short of `new_tip`).
All halt the apply; none silently diverges.

## 14. Hard Prohibitions
Inherits cluster §8 (all eight). **Slice-specific:** no second rollback / materialize / admit
implementation (compose `commit_rollback` + `materialize_rolled_back_state` + `pump_block` only);
no `select_best_chain` / `fork_choice` / `chain_selector` call in the driver (it *applies* a
decision, never makes one — `DC-CONS-03` is the orchestrator's, AI-S4); `selected_tip` /
`replaced_tip` are not durable authority (the durable tip comes from `commit_rollback` /
`pump_block`); **`WalEntry::RollBack` is appended only after `commit_rollback` succeeds**;
`pump_block` stays the sole roll-forward admit; no header-only tip advance; no `OrchestratorState`
ownership / loop wiring / block-fetch (AI-S4); no new BLUE; SingleProducer (DC-NODE-20) untouched.

## 15. Explicit Non-Goals
No receive-loop wiring / `ChainEvent` production / block-fetch / `OrchestratorState` (AI-S4); no
detector/forge-gate; no production warm-start rollback-aware rewire (AI-S4 — AI-S3 proves the WAL
record is produced + replay-equivalent); no convergence evidence (AI-S5); no multi-peer; no
performance work.

## 16. Completion Checklist
- [ ] `apply_chain_event` composes `materialize_rolled_back_state` + `commit_rollback` +
  `WalEntry::RollBack` (after commit) + `pump_block` over the live `fwd.receive` (ReceiveState) —
  no new rollback/admit path.
- [ ] Per-event: `RolledBack` → durable rollback + WAL record (in the pinned 5-step order);
  `ChainSelected` → `pump_block` extend; `Rejected` → no-op.
- [ ] Failed `commit_rollback` appends no WAL + leaves state unchanged.
- [ ] Reconciliation fail-fast (DC-NODE-26); k-bounds fail-closed (OQ-4); header→body coherence.
- [ ] Replay-equivalent (reuse AI-S1) — abandoned branch never resurrects (CE-AI-1 production half).
- [ ] `ci_check_live_fork_choice_apply.sh` + `cargo test -p ade_node` green; the reused-authority
  gates stay green; no new BLUE; `DC-CONS-03`/`DC-NODE-20` untouched.
