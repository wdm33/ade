# Invariant Slice AI-S4b-ii — Live rollback-follow routing + forge gate

> Slice of PHASE4-N-AI. AI-S1/S2/S3/S4a/S4b-i closed + pushed (`fbe33112`). **RED; the go-live
> behavior flip** — the first slice where `--mode node` acts on a peer's ChainSync rollback
> live. One whole slice (no further split): receive routing + rollback application + pending
> state + forge gate together.
>
> **Claim scope (honesty):** this proves **live single-best-peer rollback *following*** —
> Ade follows one peer through an explicit ChainSync `RollBackward` and replay-equivalently
> adopts the peer's branch. It does **NOT** call `select_best_chain` / `process_stream_input`,
> and does **NOT** prove full multi-candidate `CN-CONS-03` live selection (that is CE-AI-6/AI-S5
> + a later multi-peer slice).

## 2. Slice Header
- **Slice Name:** Live single-best-peer rollback-follow routing + receive classification +
  DC-NODE-28 forge gate.
- **Cluster:** PHASE4-N-AI. **Status:** Proposed.
- **CE Addressed:** **CE-AI-2 (live)** (DC-NODE-23/24 run on the live path), **CE-AI-3 (live)**
  (DC-NODE-25/26 — durable rollback apply + reconcile on the live path), **CE-AI-4** (DC-NODE-28).
- **Dependencies:** AI-S2 (detector), AI-S3 (`apply_chain_event`), AI-S4a
  (`AdmissionPeerEvent::RollBackward`), AI-S4b-i (`VenueRole::Participant`).

## 4. Intent
Every peer receive event on the live `--mode node` path is **classified or routed
deterministically**, and the producer **cannot forge while a re-selection is unresolved**. For
`VenueRole::Participant`, a peer `RollBackward` is followed by a durable rollback through the
already-enforced apply authority (AI-S3); `SingleProducer`/`Unknown` keep the existing
fail-closed receive + the DC-NODE-20 forge fence byte-unchanged.

## 5. Scope
- **RED `ade_node::node_sync`:** richer `NodeSyncItem { Block(Vec<u8>) | RollBack(Point) }`;
  `NodeBlockSource` carries `VecDeque<NodeSyncItem>` + `next_item()`; `run_node_sync` RollBack-aware
  for SP/Unknown (Block→`pump_block`, RollBack→refuse).
- **RED `ade_node::node_lifecycle`:** the Participant receive routing (detector +
  RollBack→`apply_chain_event`) — *here, not node_sync, because `apply_chain_event` lives here*;
  the pending-reselection state; the ForgeTick DC-NODE-28 gate.
- **Reuses:** AI-S2 `classify_receive`/`resolve_disposition` (GREEN), AI-S3 `apply_chain_event`
  (RED), `materialize_rolled_back_state`/`commit_rollback`/`pump_block` (BLUE/reused), AI-S4a
  `RollBackward`, AI-S4b-i `Participant`.
- **Out of scope:** multi-candidate `select_best_chain` adoption (later multi-peer slice); the
  operator convergence pass (AI-S5).

## 6. Execution Boundary (TCB color)
- **RED:** the loop routing + Participant sync + pending state + forge gate (`node_lifecycle`);
  the richer item + RollBack-aware drain (`node_sync`). **GREEN (reused):**
  `classify_receive`/`resolve_disposition`. **BLUE (reused, unchanged):**
  materialize/commit_rollback/pump_block via `apply_chain_event`. **No new BLUE.**

## 7. Invariants Preserved (registry IDs)
`DC-NODE-20` (SP forge fence byte-unchanged), `DC-NODE-05/12` (pump_block sole roll-forward
admit), `DC-CONS-03` (the loop never calls `select_best_chain`), `DC-CONS-05/06` (within-k via
`apply_chain_event`'s materialize), `DC-NODE-25/26/27` (the apply authority, reused not changed),
`DC-NODE-23/24` (detector/resolver semantics, reused), `CN-WAL-01` (append-only — the RollBack
record via AI-S3).

## 8. Invariants Strengthened / Introduced
- **DC-NODE-23/24 → live** (Participant runs `classify_receive` + `resolve_disposition` on the
  live receive path).
- **DC-NODE-25/26 → live** (durable rollback apply + reconcile now driven by a live peer
  `RollBackward`).
- **Introduces DC-NODE-28** (no forge across an unresolved re-selection). *One family: live
  rollback-follow receive routing + the producer-race fence.*
- **Honesty (registry note):** proves single-best-peer rollback *following* (replay-equivalent
  peer-branch adoption); does **not** claim full `CN-CONS-03` multi-candidate live selection.

## 9. Design Summary (direct `RolledBack`, not `process_stream_input`)
**Richer sync item:** `NodeSyncItem { Block(Vec<u8>), RollBack(Point) }`; the WirePump drain maps
`AdmissionPeerEvent::{Block→Block, RollBackward→RollBack, TipUpdate→observe-skip,
Disconnected→end}`; `in_memory(Vec<Vec<u8>>)` wraps as `Block` (existing tests unaffected) +
`in_memory_items` for rollback tests. `pump_block` stays the sole roll-forward admit.

**Routing, venue-gated:**
- **Participant** (the new path, in `node_lifecycle`):
  - `Block(bytes)` → decode header → `CandidateSummary` + `in_spine` (ChainDb membership) →
    `classify_receive` → `resolve_disposition(Participant)`:
    - `AlreadyHave` → drop.
    - `LinearExtend` → `pump_block`.
    - `NeedsForkChoice` (Competing) → **fail closed** — a bare competing block has no safe fork
      point (single-best-peer; live multi-candidate selection is a later slice).
  - `RollBack(point)` → look up `point` in the durable ChainDb → if absent / beyond-k / crossing
    the immutable tip, **fail closed** (no fabricated block_no); else compute `to_block_no`+`depth`,
    construct `ChainEvent::RolledBack { to_point, depth }`, **set pending-reselection**,
    `apply_chain_event` (AI-S3), **clear pending only after `apply_chain_event` returns and
    reconciliation/failure handling completes**. *(Not `process_stream_input` — its in-memory ring
    is header-arrival-populated, empty here; `apply_chain_event`'s materialize over the durable
    snapshot store is the within-k authority.)*
- **SingleProducer / Unknown** (`run_node_sync`, byte-unchanged receive): `Block` → `pump_block`;
  `RollBack` → refuse (fail closed — they do not follow peer rollbacks).

**DC-NODE-28 forge gate:** a `pending_reselection` flag on the loop's forge activation, set when a
RollBack apply is in flight, cleared **only after** `apply_chain_event` returns + reconciliation
or fail-closed handling completes. The `ForgeTick` arm refuses (typed `ForgeRefused`) while
pending — never forges on the stale pre-resolution tip.

**Reconciliation (DC-NODE-26, reused):** `apply_chain_event` already asserts durable
`ChainDb::tip == to_point` after the rollback.

## 10. Changes Introduced
`NodeSyncItem` + `NodeBlockSource::next_item` + RollBack-aware `run_node_sync`; the Participant
receive-routing in `node_lifecycle` (decode→detector→route; RollBack→durable-lookup→`apply_chain_event`);
`ForgeActivation.pending_reselection` + the ForgeTick gate; a `NodeSyncError`/`ForgeRefused`
variant for the bare-competing + bad-rollback fail-closed. No new BLUE; no `select_best_chain` /
`process_stream_input` call.

## 11. Replay / Crash / Epoch
The live RollBack produces a `WalEntry::RollBack` via `apply_chain_event` (AI-S3) —
replay-equivalent by the AI-S1/S3 machinery. **Hermetic loop test**
(`tests/live_fork_choice_ai_s4bii.rs`): drive an `InMemory` source through the Participant loop
(Block→RollBack→Block), then replay the WAL → recovers the reselected tip, never the abandoned
branch.

## 12. Mechanical Acceptance Criteria (hermetic, `InMemory` NodeBlockSource)
- [ ] `participant_rollback_then_extend_converges` — Participant: Block(s) → RollBack(in-chain
  point) → Block(s) ⇒ durable rollback via `apply_chain_event` + roll-forward; `ChainDb::tip` ==
  reselected tip; replay-equivalent.
- [ ] `participant_rollback_to_unknown_point_fails_closed` — RollBack to a point not in the
  durable chain → fail closed (no fabricated block_no, no apply).
- [ ] `participant_rollback_beyond_k_fails_closed` — RollBack beyond retention/k → fail closed
  (materialize `RollbackTooDeep`).
- [ ] `participant_linear_extend_uses_pump_block`, `participant_already_have_drops`,
  `participant_bare_competing_fails_closed`.
- [ ] `singleproducer_block_path_unchanged` — SP: Block→`pump_block` (byte-identical);
  `singleproducer_rollback_refused`; `unknown_venue_rollback_refused`.
- [ ] `forge_refused_while_reselection_pending` (DC-NODE-28) — a ForgeTick during a pending
  RollBack apply → typed `ForgeRefused`; `forge_resumes_after_reselection_reconciled`.
- [ ] `forge_resumes_after_failed_reselection_only_after_pending_cleared` — pending set →
  RollBack apply **fails closed** → pending cleared → **no block forged during the pending
  window** → a later forge tick uses the **unchanged durable tip** (the failure path, not just
  success).
- [ ] New gate **`ci/ci_check_live_fork_choice_wiring.sh`** (here-strings, not `echo|grep -q`):
  the Participant RollBack path constructs `ChainEvent::RolledBack` + calls `apply_chain_event`
  and does **not** call `process_stream_input`/`select_best_chain`; `pump_block` remains the sole
  roll-forward admit; the SP/Unknown receive path is unchanged; the ForgeTick gate checks
  `pending_reselection`; `pending_reselection` is cleared only after the apply returns. Reused
  gates (`ci_check_live_fork_choice_apply.sh`, `ci_check_participant_venue_inert.sh`,
  `ci_check_single_producer_extend_own_spine.sh`, `ci_check_wire_rollback_signal_preserved.sh`)
  stay green.
- [ ] `cargo test -p ade_node` green.

## 13. Failure Modes (all deterministic, fail-closed)
RollBack to unknown/beyond-k/immutable-crossing point → fail closed; bare Competing block → fail
closed; `apply_chain_event` error → propagated (no partial), pending cleared on the failure path;
ForgeTick while pending → `ForgeRefused`. None silently diverges.

## 14. Hard Prohibitions
No second rollback/admit/materialize path (reuse `apply_chain_event`); no
`select_best_chain`/`process_stream_input`/`chain_selector` call in the live path; `pump_block`
stays the sole roll-forward admit; SP/Unknown receive + the DC-NODE-20 forge fence byte-unchanged;
no header-only adoption; `selected_tip` audit-only; no fabricated rollback block_no; no
multi-candidate adoption (later slice); no new BLUE. **`pending_reselection` is NEVER cleared
before `apply_chain_event` returns and reconciliation/failure handling completes** — otherwise a
producer tick could slip through between rollback start and durable state settlement.

## 15. Explicit Non-Goals
Multi-candidate `select_best_chain` live selection; the operator convergence pass + full
CN-CONS-03 (AI-S5); multi-peer.

## 16. Completion Checklist
- [ ] `NodeSyncItem` + RollBack-aware drain; Participant routing (detector +
  RollBack→`apply_chain_event`); pending-state + DC-NODE-28 forge gate (pending cleared only
  after apply returns).
- [ ] Contract tests (incl. the failure-path forge test) + replay-equivalence + the gate green;
  reused gates stay green; `cargo test -p ade_node` green.
- [ ] SP/Unknown byte-unchanged; no `select_best_chain`/`process_stream_input`; no new BLUE.
