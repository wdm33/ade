# Slice AL-S1 — Participant Recovered-Anchor Boundary Is Valid for Live-Follow Rollback

## 1. Title
On the participant live-follow path (`run_participant_sync`), accept a peer `RollBackward` that binds
EXACTLY (slot AND hash) to the persisted recovered anchor point as an idempotent no-op boundary
rewind, so a bare-anchor participant recover→follow completes through the existing `pump_block`. The
single slice of PHASE4-N-AL — the participant MIRROR of AK-S2 / DC-NODE-32.

## 2. Slice Header
- **Cluster:** PHASE4-N-AL. **Status:** Planned.
- **Cluster Exit Criteria Addressed:** CE-AL-1..6 (hermetic) + CE-AL-3-LIVE (live preflight — the
  CE-AI-6 gate).
- **Primary registry rule:** DC-NODE-33 (`declared` → targeted `enforced` at AL close).

## 4. Intent (invariant impact)
Strengthen **DC-NODE-33** `declared → enforced`: the recovered bootstrap anchor point (DC-NODE-31 /
`BootstrapState.tip` / `state.recovered_anchor`) is an authoritative **local boundary point** for the
PARTICIPANT live-follow. The relay's standard post-`IntersectFound(anchor)` rewind
`RollBackward(anchor)` (every chain-sync server sends it to set the read pointer to the intersection)
is **accepted as an idempotent no-op** on `run_participant_sync` — the node is already at the anchor —
after which the existing `LinearExtend → pump_block` resumes catch-up. This closes the CE-AI-6 start
blocker (`run_participant_sync` `get_block_by_hash(anchor)→None→UnexpectedRollback` at
`node_lifecycle.rs:2581`, because a bare anchor is a recovery snapshot, not a stored servable block)
**without** synthesizing a block, weakening `RollBackward(Origin)` rejection, touching `pump_block`,
or broadening DC-NODE-32.

## 6. Execution Boundary (TCB color)
- **BLUE** — the participant rollback-resolution predicate in `run_participant_sync`
  (`crates/ade_node/src/node_lifecycle.rs`): pure `(peer_point, recovered_anchor) → {AnchorNoop,
  FallThrough}`. The authoritative decision "is this rollback target the recovered boundary?".
- **Canonical input** — `state.recovered_anchor` (DC-NODE-31 / `BootstrapState.tip`, already populated
  via `node_lifecycle.rs:563`).
- **RED (wiring)** — routing the `RollBack` item to the predicate (`node_lifecycle.rs`). NO new
  field/param — reuses the AK-S2 `ForwardSyncState.recovered_anchor`
  (`crates/ade_runtime/src/forward_sync/reducer.rs`).
- **RED / UNCHANGED** — `pump_block`, `block_validity` (the forward admit), `apply_chain_event` / the
  DC-NODE-29 stored-block resolution; `run_node_sync` (DC-NODE-32); `spawn_live_wire_pump_source` /
  the wire pump; AI-S4a (`wire_pump.rs:447`); `ChainDb::tip()` / serve; N-AJ evidence emission.

## 7. Invariants Preserved
- **AI-S4a / DC-NODE-23..29** — `RollBackward(Origin)` stays fail-closed; the stored-block rollback
  authority (`get_block_by_hash` + stored slot/hash binding + `apply_chain_event`) is untouched (the
  anchor branch sits BEFORE it).
- **DC-NODE-32** — single-producer `run_node_sync` unchanged; **NOT** broadened (AL is the sibling
  DC-NODE-33).
- **CN-PROD / `pump_block` sole admit** — every forward block admits ONLY through `pump_block`; AL
  adds no forward-link code.
- **DC-NODE-31 (AK-S1)** — the persisted anchor + `BootstrapState.tip`; AL consumes its output via
  `state.recovered_anchor`.
- **DC-NODE-30 (N-AJ)** — convergence evidence on the `Block` path; the anchor no-op is a rollback,
  emits nothing.
- **T-REC-05**, **CN-CONS-03**, `ChainDb::tip()` / serve — untouched.

## 8. Invariants Strengthened
- **DC-NODE-33** `declared → enforced` — AL-S1 populates its `tests` with the named tests below.
- **T-REC-05** (optional) strengthened at AL close — recover→follow on the participant path is now
  replay-equivalent.

## 9. Design Summary
- **Single authority, no new wiring (OQ-AL-1 resolved).** `state.recovered_anchor`
  (`ForwardSyncState.recovered_anchor`, `reducer.rs:148`) is ALREADY populated for the participant
  path: set once from `BootstrapState.tip` in the forge-ON arm (`node_lifecycle.rs:563`), and
  `run_relay_loop_with_sched(&mut fwd, …)` passes the SAME `ForwardSyncState` to both
  `run_participant_sync` (1313) and `run_node_sync` (1326). `run_participant_sync` already takes
  `state: &mut ForwardSyncState` (2483). So **NO** new param, **NO** field add, **NO** `:563` change —
  AL-S1 only reads `state.recovered_anchor` in the handler. **NEVER re-read the store.** `None` ⇒ the
  pre-AL behavior (the existing resolution) — cold-start / no-recovered-anchor callers unchanged.
- **Guarded no-op (the only behavioral change).** In `run_participant_sync`'s `RollBack` handler
  (`node_lifecycle.rs:2560`), BEFORE the `get_block_by_hash` resolution, add: if the rollback point is
  `Point::Block { slot, hash }` and `Some(anchor) = &state.recovered_anchor` and `slot == anchor.slot
  && hash == anchor.hash` (slot **AND** hash), `continue` (idempotent no-op — no `commit_rollback`, no
  `WalEntry::RollBack`, no ChainDb/ledger/chain_dep/cursor mutation, `pending_reselection` NOT set).
  Otherwise fall through to the EXISTING resolution unchanged (`Origin` ⇒ `UnexpectedRollback`;
  unknown/mismatch ⇒ `get_block_by_hash` → fail closed or `apply_chain_event` for stored blocks).
  Mirror of `node_sync.rs:479-486`, adapted to sit BEFORE (not replace) the participant stored-block
  resolution.
- **First forward unchanged.** After the anchor no-op, the next `Block` item admits through the
  EXISTING participant `LinearExtend → pump_block` arm (`node_lifecycle.rs:2542-2549`); its
  `block_validity` `prev_hash` check binds it to the recovered `chain_dep`. (Same property AK-S2's
  OQ #2 live probe proved for `run_node_sync`; the `LinearExtend` arm is unchanged.) **No forward-link
  code.**
- **Scope (OQ-AL-3).** Participant `run_participant_sync` + recovered-anchor case ONLY. No change to
  the stored-block rollback-follow (DC-NODE-29 untouched); `run_node_sync` (DC-NODE-32) untouched.

## 11. Replay / Crash / Epoch Validation
Same recovered store (same persisted anchor) + same ordered participant feed `[RollBackward(anchor),
RollForward(N+1), …]` ⇒ byte-identical post-state (ledger, chain_dep, ChainDb tip) and admit sequence
(extends T-REC-05 / DC-NODE-31 / DC-NODE-32 to the participant follow). The anchor no-op mutates
nothing, so a crash mid-follow recovers identically (durable state is whatever `pump_block` admitted,
unchanged).

## 12. Mechanical Acceptance Criteria
- **CE-AL-1** (`ade_node`, hermetic): `participant_rollback_to_recovered_anchor_is_noop` — feed
  `[RollBack(anchor slot+hash)]` with `state.recovered_anchor = Some(anchor)`; `run_participant_sync`
  returns `Ok`, store byte-identical (no WAL append, no ChainDb/tip/chain_dep mutation,
  `pending_reselection` stays `false`).
- **CE-AL-2** (hermetic): `participant_rollback_origin_fails_closed` — `RollBack(Origin)` ⇒
  `UnexpectedRollback` even with `recovered_anchor = Some(..)`.
- **CE-AL-3** (hermetic): `participant_rollback_non_anchor_fails_closed` — a non-anchor `Point::Block`
  (different slot+hash; **slot-only match**; **hash-only match**) ⇒ the existing resolution
  (`UnexpectedRollback` for an unknown hash), never the no-op (binds BOTH slot and hash).
- **CE-AL-4** (hermetic): `participant_first_forward_after_anchor_noop_admits_via_pump_block` — feed
  `[RollBack(anchor), Block(valid_successor)]`; the anchor rollback no-ops and the successor admits
  through `pump_block` (the tip advances). Proves AL-S1 does not block the forward path.
- **CE-AL-5** (hermetic): `participant_stored_block_rollback_still_applies` — a `RollBack` to an
  actually-stored block (`get_block_by_hash = Some`) still routes through `apply_chain_event`
  (DC-NODE-29) unchanged — the anchor branch did not capture it (proves DC-NODE-29 preserved).
- **CE-AL-6** (no collateral): `cargo test -p ade_node` green; the existing `run_participant_sync` /
  fork-choice tests (incl. the stored-block + Origin cases) stay green; `recovered_anchor = None`
  callers behaviorally unchanged.
- **CE-AL-3-LIVE** (live preflight, operator-run at close — the **bright-red CE-AI-6 gate**): a FRESH
  recover at the current rung-2 venue tip, then `--mode node --participant-venue` ⇒ FindIntersect at
  the anchor ⇒ the relay's `RollBackward(anchor)` no-op'd ⇒ the FIRST forward block admits through
  `pump_block` ⇒ **0** `UnsupportedRollbackPoint` AND **0** `UnexpectedRollback` before the first admit.

## 14. Hard Prohibitions (inherit cluster Forbidden verbatim)
- NO broadening DC-NODE-32 (AL is the distinct sibling DC-NODE-33).
- NO blanket rollback no-op — the ONLY accepted rollback is the recovered anchor.
- NO accepting a rollback by slot ALONE or hash ALONE — bind BOTH to the persisted anchor.
- Do NOT synthesize a ChainDb block for the anchor or mark it servable.
- Do NOT re-read the anchor from the store inside `run_participant_sync` — consume
  `state.recovered_anchor`.
- Do NOT change `pump_block` / `block_validity` (the forward admit) or `apply_chain_event` / the
  DC-NODE-29 stored-block resolution.
- Do NOT change `run_node_sync`.
- Do NOT weaken AI-S4a — `RollBackward(Origin)` stays fail-closed.
- Do NOT touch N-AJ evidence emission. Do NOT flip CN-CONS-03.
- Do NOT run the CE-AI-6 reorg/convergence pass before CE-AL-3-LIVE proves the first clean forward
  admit.
