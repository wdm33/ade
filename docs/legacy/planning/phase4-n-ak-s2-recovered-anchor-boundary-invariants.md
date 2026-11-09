# AK-S2 — Recovered Anchor Boundary Is Valid for Live Follow (rollback no-op)

> Invariants sketch (IDD Part I). Cluster **PHASE4-N-AK** (the live regression is not
> remediated until recover→follow completes; **CE-AK-3** is the cluster's live bar).
> **Classification:** *improving reinterpretation that preserves semantics* — external
> Cardano follow/admit behavior is unchanged ("I intersect at this point and continue
> from it"); Ade's internal authority improves (the intersection base is store-derived +
> replayable, never an Origin-resync accident).

## The seam AK-S1 exposed

`--single-producer-venue` dispatches the live feed to `run_node_sync` (`node_sync.rs:471`),
which fails closed on **every** RollBack (`NodeSyncItem::RollBack(_) => Err(UnexpectedRollback)`)
and never receives the recovered anchor point. After AK-S1's FindIntersect-at-the-anchor, the
relay's standard post-`IntersectFound(anchor)` rewind `RollBackward(anchor)` halts the follow
(exit 43, `EXIT_NODE_RELAY_SYNC_FAILED`) before catch-up. The anchor is a recovery **snapshot
boundary**, not a stored servable block.

## Investigation outcome (OQ-AK-S2, resolved before this sketch was finalized)

- **OQ #1 (anchor data-flow) — resolved.** The recovered anchor point lives once in
  `BootstrapState.tip` (AK-S1, the single authority) and today reaches only
  `spawn_live_wire_pump_source` (the wire-pump start). `ForwardSyncState` carries only a
  fingerprint (`prior_fp`), not the anchor point. → AK-S2 threads `BootstrapState.tip` into
  `run_node_sync` as a new parameter; it **must not** re-read the store inside the loop (that
  would create a second anchor authority).
- **OQ #2 (one change or two) — resolved EMPIRICALLY: ONE change.** A temporary probe
  (no-op the non-Origin rollback in `run_node_sync`, keep Origin failing) was run live against
  the frozen `c2-relay` venue. Result: the node **applied blocks 9–13 through the existing
  `pump_block` path** — `forge_base_source=local_chaindb_tip`, `forge_mode=caught_up_to_peer_tip`,
  `forge_base_block_no=13`, WAL grew 76 B → 639 B, **0** errors, **0** `UnexpectedRollback`,
  **0** `UnsupportedRollbackPoint`, ran continuously (killed, not halted). The first forward
  block (9, `prev_hash` = block 8 = the anchor) links via the recovered `chain_dep` (the
  snapshot at the anchor) through `block_validity`'s normal `prev_hash` check. **The
  first-forward parent binding is already enforced by the existing admit path — no new "link"
  code is needed.** (Probe reverted; evidence at `~/.cardano-ceai6/inputs/node-run-ak3probe.*`.)
- **OQ #3 (stored-block rollback parity) — out of scope.** `run_node_sync` follows no
  rollbacks today; a bare anchor only ever receives `RollBackward(anchor)`. AK-S2 adds the
  recovered-anchor case **only**. General stored-block rollback-follow on the single-producer
  path is a separate hardening obligation if/when a continuation-spine recover (`admit_count>0`)
  needs it.
- **OQ #4 (participant-path parity) — flagged, NOT fixed here.** `run_participant_sync`
  (`node_lifecycle.rs:2573`) has the same bare-anchor blocker (`get_block_by_hash(anchor)→None`).
  CE-AK-3 is single-producer, so AK-S2 targets `run_node_sync`; participant parity is an
  explicit separate follow-on.
- **OQ #5 (read-pointer model) — resolved: pure no-op.** `run_node_sync` has no follow cursor
  (it consumes `source.next_item()`; the wire read-pointer is the pump's, already at the anchor).
  Rollback-to-anchor mutates nothing: no WAL, no ChainDb, no ledger, no cursor.

## Pure transformation?
**Yes.** The follow decision is a pure function of
`(peer chain-sync item, persisted recovered anchor point) → {accept-anchor-noop, fail-closed}`
on the rollback surface; the forward surface is the unchanged `pump_block` admit. No new
nondeterminism; the peer item is canonical input, the anchor is store-derived canonical state.

## 1. What must always be true
- **S2-1 (boundary-point authority).** The recovered bootstrap anchor point (AK-S1's persisted
  `RecoveredAnchorPoint` / `BootstrapState.tip`) is an authoritative *local boundary point* for
  live follow: a peer `RollBackward` whose target binds **exactly** (slot AND hash) to the
  persisted anchor is a valid rewind target — even though the anchor is a snapshot, not a stored
  servable ChainDb block.
- **S2-2 (rollback resolution on the single-producer path is total).** On `run_node_sync`, a
  peer `RollBackward(point)` resolves: `point == persisted anchor` (slot ∧ hash) → idempotent
  no-op boundary rewind → else fail closed. `RollBackward(Origin)` still fails closed (AI-S4a,
  unchanged). Every non-anchor, non-Origin rollback still fails closed (the stored-block case is
  out of scope — OQ #3).
- **S2-3 (single anchor authority).** The anchor point consumed by `run_node_sync` is
  `BootstrapState.tip` (DC-NODE-31), threaded in — never re-read from the store inside the loop.
- **S2-4 (peer/persisted binding).** A peer-supplied rollback point is accepted only when it
  binds to the **persisted** anchor on slot AND hash — never on peer slot or hash alone.
- **S2-5 (idempotent no-op).** Rollback-to-recovered-anchor produces **no durable effect** — no
  `commit_rollback`, no `WalEntry::RollBack`, no tip/ledger mutation, no cursor change.
- **Preserved + verified (NOT new code):** the first forward block after the anchor admits
  through the **existing sole `pump_block` path**, whose `block_validity` `prev_hash` check
  already binds it to the recovered `chain_dep` (proven by the OQ #2 probe).
- **Preserved (unchanged):** `RollBackward(Origin)` rejection (AI-S4a); the stored-block
  rollback authority on the participant path (DC-NODE-29/AI-S6); `pump_block` as sole admit;
  `ChainDb::tip()` / serve semantics; DC-CONS-03; T-REC-05; DC-NODE-31 (AK-S1); N-AJ evidence.

## 2. What must never be possible
- Accepting a single-producer rollback to any point other than the persisted recovered anchor
  (Origin and all other points fail closed).
- **Synthesizing** a ChainDb block for the anchor, **marking** it servable, or
  `ChainDb::tip()`/`last_block_bytes`/serve ever returning the anchor as a block.
- Admitting any forward block via a path other than `pump_block`.
- Trusting a peer slot/hash that disagrees with the persisted anchor.
- A rollback-to-anchor producing any durable mutation.

## 3. What must remain identical across executions
- The single-producer rollback-resolution decision: pure `fn(peer_point, persisted_anchor) →
  {AnchorNoop, FailClosed}`. No wall-clock / rand / HashMap / float.

## 4. What must be replay-equivalent
Same recovered store (same persisted anchor) + same ordered peer feed
`[RollBackward(anchor), RollForward(9), …, RollForward(13)]` → **byte-identical** post-state
(ledger, chain_dep, ChainDb tip) and byte-identical admit sequence. Extends AK-S1's start-tip
replay-equivalence (and T-REC-05) to the **single-producer follow itself** — recover→follow is a
deterministic function of (store, feed), never of an Origin-resync accident.

## 5. State transitions in scope (single-producer `run_node_sync`)
```
(follow @ recovered_anchor, RollBackward(Origin))
    → Err(FailClosed)                                          # AI-S4a, unchanged
(follow @ recovered_anchor, RollBackward(p)) where p == persisted_anchor (slot ∧ hash)
    → Ok((follow @ recovered_anchor, NO_EFFECT))               # NEW: idempotent boundary rewind
(follow @ recovered_anchor, RollBackward(p)) otherwise
    → Err(FailClosed)                                          # all other points (incl. would-be stored, OQ#3)
(follow @ recovered_anchor, RollForward(b))                    # block 9..13
    → existing pump_block admit (UNCHANGED — prev_hash binds recovered chain_dep)
```

## 6. TCB color hypothesis
- **BLUE** — the rollback-resolution predicate (pure, total: `(peer_point, persisted_anchor) →
  disposition`). The persisted anchor is already-BLUE canonical state (DC-NODE-31).
- **BLUE (unchanged)** — `pump_block`, `block_validity` (the forward admit, already enforces the
  first-forward link).
- **RED** — `run_node_sync` orchestration: threading the anchor in, routing the RollBack item to
  the BLUE predicate, applying the no-op. No authority here.

## 7. Open questions
All OQ-AK-S2 questions resolved above. None remain blocking `/cluster-doc`.

## Registry (declared after investigation)
DC-NODE-32 (family DC, derived, `introduced_in = PHASE4-N-AK`, `status = declared`) — the
rollback-to-recovered-anchor no-op clause only (the first-forward "link" clause is **not** a new
invariant; it is a preserved, already-enforced property recorded above). Statement in the
registry entry.
