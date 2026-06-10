# Slice AK-S2 — Recovered Anchor Boundary Is Valid for Live-Follow Rollback

## 1. Title
On the single-producer live-follow path (`run_node_sync`), accept a peer `RollBackward` that binds
EXACTLY (slot AND hash) to the persisted recovered anchor point as an idempotent no-op boundary
rewind, so the recover→follow completes through the existing `pump_block`. The second of two slices
of PHASE4-N-AK.

## 2. Slice Header
- **Cluster:** PHASE4-N-AK. **Status:** Proposed.
- **Cluster Exit Criteria Addressed:** CE-AK-S2-1..5 (hermetic) + CE-AK-3 (live, end-to-end — spans
  AK-S1 + AK-S2).
- **Primary registry rule:** DC-NODE-32 (`declared` → targeted `enforced` at AK close).

## 4. Intent (invariant impact)
Strengthen **DC-NODE-32** `declared → enforced`: the recovered bootstrap anchor point (DC-NODE-31 /
`BootstrapState.tip`) is an authoritative **local boundary point** for live follow. The relay's
standard post-`IntersectFound(anchor)` rewind `RollBackward(anchor)` (every chain-sync server sends it
to set the read pointer to the intersection) is **accepted as an idempotent no-op** on the
single-producer path — the node is already at the anchor — after which `pump_block` resumes catch-up.
This closes the CE-AK-3 follow blocker (`run_node_sync` `UnexpectedRollback` at `node_sync.rs:471`,
because a bare anchor is a recovery snapshot, not a stored servable block) **without** synthesizing a
block, weakening `RollBackward(Origin)` rejection, or touching `pump_block`.

## 6. Execution Boundary (TCB color)
- **BLUE** — the single-producer rollback-resolution predicate in `run_node_sync`
  (`crates/ade_node/src/node_sync.rs`): pure `(peer_point, recovered_anchor) → {AnchorNoop,
  FailClosed}`. The authoritative decision "is this rollback target the recovered boundary?".
- **Canonical input** — the recovered anchor point `BootstrapState.tip` (AK-S1 / DC-NODE-31).
- **RED (wiring)** — threading `BootstrapState.tip` from `run_node_lifecycle` through
  `run_relay_loop_with_sched` into `run_node_sync` (`crates/ade_node/src/node_lifecycle.rs`).
- **RED / UNCHANGED** — `pump_block`, `block_validity` (the forward admit — already enforces the
  first-forward link); `run_participant_sync` (participant path — separate follow-on);
  `spawn_live_wire_pump_source` / wire pump; AI-S4a (`wire_pump.rs:447`); `ChainDb::tip()` / serve.

## 7. Invariants Preserved
- **AI-S4a / DC-NODE-23..29** — `RollBackward(Origin)` stays fail-closed; the participant
  rollback-follow + stored-block authority are untouched.
- **CN-PROD / pump_block sole admit** — the first (and every) forward block admits ONLY through
  `pump_block`; AK-S2 adds no forward-link code.
- **DC-NODE-31 (AK-S1)** — the persisted anchor + start-tip resolution; AK-S2 consumes its output.
- **T-REC-05**, **CN-CONS-03**, the `ChainDb::tip()` storage contract, **N-AJ** evidence — untouched.

## 8. Invariants Strengthened
- **DC-NODE-32** `declared → enforced` — AK-S2 populates its `tests` with the named tests below.
- **T-REC-05** strengthened at AK close — recover→follow on the single-producer path is now
  replay-equivalent (same store + same ordered peer feed ⇒ byte-identical post-state + admit seq).

## 9. Design Summary
- **Single authority + threading (OQ #1 resolved).** The recovered anchor point lives once in
  `BootstrapState.tip` (AK-S1). Thread it (an `Option<ChainTip>`) from `run_node_lifecycle` (both
  forge-OFF and forge-ON arms) through `run_relay_loop_with_sched` into `run_node_sync` as a new
  parameter. **NEVER re-read the store inside `run_node_sync`** (that would create a second anchor
  authority). `None` ⇒ the pre-AK-S2 behavior (any rollback fails closed) — cold-start / no-recovered-
  anchor callers are unchanged.
- **Guarded no-op (the only behavioral change).** In `run_node_sync`'s item loop, replace
  `NodeSyncItem::RollBack(_) => Err(UnexpectedRollback)` with: if the rollback point is a
  `Point::Block { slot, hash }` and `(slot, hash) == recovered_anchor` (slot **AND** hash), `continue`
  (idempotent no-op — no `commit_rollback`, no `WalEntry::RollBack`, no ChainDb / ledger / chain_dep /
  cursor mutation); `Point::Origin` ⇒ `Err(UnexpectedRollback)` (AI-S4a); every other point ⇒
  `Err(UnexpectedRollback)` (fail closed).
- **First forward unchanged (OQ #2 resolved EMPIRICALLY — ONE change).** After the anchor no-op, the
  next `Block` item (block 9, `prev_hash` = the anchor) admits through the EXISTING `pump_block` path
  — its `block_validity` `prev_hash` check already binds it to the recovered `chain_dep` (the snapshot
  at the anchor). The OQ-AK-S2-2 live probe proved it: blocks 9–13 admitted via `local_chaindb_tip`,
  caught up to the relay tip, 0 errors. **No forward-link code is added** (a non-goal).
- **Scope (OQ #3/#4).** Single-producer `run_node_sync` + recovered-anchor case ONLY. No general
  stored-block rollback-follow on this path; `run_participant_sync` (the participant twin at
  `node_lifecycle.rs:2573`) is a separate follow-on, NOT touched.

## 11. Replay / Crash / Epoch Validation
Same recovered store (same persisted anchor) + same ordered peer feed `[RollBackward(anchor),
RollForward(9), …, RollForward(13)]` ⇒ byte-identical post-state (ledger, chain_dep, ChainDb tip) and
byte-identical admit sequence (extends T-REC-05 / DC-NODE-31 to the single-producer follow). The
rollback-to-anchor no-op mutates nothing, so a crash mid-follow recovers identically to AK-S1 (the
durable state is whatever `pump_block` admitted, unchanged).

## 12. Mechanical Acceptance Criteria
- **CE-AK-S2-1** (`ade_node`, hermetic): `run_node_sync_accepts_rollback_to_recovered_anchor_noop` —
  feed `[RollBack(anchor slot+hash)]` with `recovered_anchor = Some(anchor)`; `run_node_sync` returns
  `Ok` (no error), and the store is byte-identical (no WAL append, no ChainDb/tip mutation).
- **CE-AK-S2-2** (hermetic): `run_node_sync_rollback_origin_fails_closed` — `RollBack(Origin)` ⇒
  `Err(UnexpectedRollback)` even with `recovered_anchor = Some(..)`.
- **CE-AK-S2-3** (hermetic): `run_node_sync_rollback_non_anchor_fails_closed` — a non-anchor
  `Point::Block` (different slot, different hash, **slot-only match**, and **hash-only match**) ⇒
  `Err(UnexpectedRollback)` (binds BOTH slot and hash; never slot-alone or hash-alone).
- **CE-AK-S2-4** (hermetic): `run_node_sync_first_forward_after_anchor_noop_admits_via_pump_block` —
  feed `[RollBack(anchor), Block(valid_successor)]`; the anchor rollback no-ops and the successor
  admits through `pump_block` (the tip advances). Proves AK-S2 does not block the forward path.
- **CE-AK-S2-5** (hermetic): `run_node_sync_rollforward_wrong_parent_fails_closed` — a `Block` whose
  `prev_hash` does not link still fails closed through the EXISTING `pump_block` / `block_validity`
  (AK-S2 added no forward-admit logic; existing validation is authoritative).
- **CE-AK-S2-6** (no collateral): `cargo test -p ade_node` green; the existing `run_node_sync` /
  `node_sync` tests (incl. the non-anchor `UnexpectedRollback` cases) stay green; `recovered_anchor =
  None` callers are behaviorally unchanged.
- **CE-AK-3** (live, operator-run at close — spans AK-S1 + AK-S2): the fixed binary re-recovers, then
  `--mode node --single-producer-venue` on the frozen venue ⇒ FindIntersect at the anchor ⇒ the
  relay's `RollBackward(anchor)` is no-op'd ⇒ catch-up reaches the frozen relay tip ⇒
  `forge_base_block_no == frozen relay tip block_no` ⇒ **0** `UnsupportedRollbackPoint` AND **0**
  `UnexpectedRollback`.

## 14. Hard Prohibitions (inherit cluster Forbidden verbatim)
- NO blanket rollback no-op — the ONLY accepted rollback is the recovered anchor.
- NO accepting a rollback by slot ALONE or hash ALONE — bind BOTH to the persisted anchor.
- Do NOT synthesize a ChainDb block for the anchor or mark it servable.
- Do NOT re-read the anchor from the store inside `run_node_sync` — consume the already-loaded
  `BootstrapState.tip`.
- Do NOT change `pump_block` / `block_validity` (the forward admit).
- Do NOT change the participant path (`run_participant_sync`).
- Do NOT add general stored-block rollback-follow on the single-producer path.
- Do NOT weaken AI-S4a — `RollBackward(Origin)` stays fail-closed.
- Do NOT touch N-AJ evidence emission. Do NOT flip CN-CONS-03.
