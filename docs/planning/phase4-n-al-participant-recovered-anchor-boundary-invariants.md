# AL-S1 — Participant Recovered-Anchor Boundary Is Valid for Live Follow (rollback no-op)

> Invariants sketch (IDD Part I). Cluster **PHASE4-N-AL** — the participant-path MIRROR of
> PHASE4-N-AK's `DC-NODE-32`. **CE-AL-3-LIVE** (the participant bare-anchor recover→follow reaching
> the first forward block) is this cluster's live bar; the full CE-AI-6 reorg/convergence pass is the
> downstream CONSUMER, gated on it, NOT this cluster's bar.
> **Classification:** *improving reinterpretation that preserves semantics* — external Cardano follow
> behavior is unchanged ("I intersect at this point and continue from it"); Ade's participant loop
> learns the same recovered-anchor-boundary lesson `run_node_sync` already learned in N-AK.

## The seam this exposes

CE-AI-6 dispatches the live feed to `run_participant_sync` (`node_lifecycle.rs:1313`, when
`venue_role == Participant`). Its `RollBack` handler (`node_lifecycle.rs:2560`) resolves the rollback
point against the durable ChainDb via `get_block_by_hash`; the recovered anchor is a snapshot
boundary, **not** a stored servable block (DC-NODE-31/32), so `get_block_by_hash(anchor) → None →
UnexpectedRollback`. Chain-sync ALWAYS sends `RollBackward(intersection)` as the first message after
`IntersectFound`, so a bare-anchor participant recover halts at message one — **before** admitting any
forward block. This is the participant twin of the AK-S2 seam (`node_sync.rs:471`); DC-NODE-32's own
sketch flagged it as *"OQ #4 (participant-path parity) — flagged, NOT fixed here … an explicit
separate follow-on."*

## Investigation outcome (resolved before this sketch was finalized)

- **OQ-AL-1 (anchor data-flow to the participant path) — RESOLVED, no new wiring.** The recovered
  anchor already lives in `ForwardSyncState.recovered_anchor` (the AK-S2 field, `reducer.rs:148`,
  default `None`), set once from `BootstrapState.tip` in the forge-ON arm (`node_lifecycle.rs:563`),
  which runs for any forge-activated live-feed venue — and a participant venue IS forge-activated
  (`is_participant ⇒ forge.is_some()`, `node_lifecycle.rs:1297-1300`). `run_relay_loop_with_sched`
  passes the SAME `ForwardSyncState` (`&mut fwd`) to both `run_participant_sync` (1313) and
  `run_node_sync` (1326); `run_participant_sync` already takes `state: &mut ForwardSyncState` (2483).
  → `state.recovered_anchor` is **already populated** on the participant path. AL-S1 only adds the
  USE in the `RollBack` handler. **NO new param, NO field add, NO `:563` change.**
- **OQ-AL-2 (one change or two) — ONE change.** Mirror `node_sync.rs:479-486`: add the exact
  slot+hash anchor-match no-op branch at the TOP of the `RollBack` handler, BEFORE the DC-NODE-29
  `get_block_by_hash` resolution. The first forward block then admits through the EXISTING participant
  `LinearExtend → pump_block` arm (`node_lifecycle.rs:2542-2549`, unchanged) — its `block_validity`
  `prev_hash` check already binds it to the recovered `chain_dep` (the same property AK-S2's OQ #2
  live probe proved for the single-producer path). **No forward-link code.**
- **OQ-AL-3 (stored-block rollback parity) — UNCHANGED / preserved.** The participant path ALREADY
  follows stored-block rollbacks via `apply_chain_event` (DC-NODE-23..29 / AI-S6). AL-S1 adds the
  recovered-anchor case ONLY, sitting BEFORE that resolution; the stored-block path is untouched.
- **OQ-AL-4 (read-pointer / durable effect) — pure no-op.** The anchor rollback mutates nothing: no
  `commit_rollback`, no `WalEntry::RollBack`, no ChainDb/ledger/chain_dep, no cursor, **and
  `pending_reselection` is NOT set** (distinct from the stored-block arm, which sets it around
  `apply_chain_event`; the anchor no-op returns BEFORE that).
- **OQ-AL-5 (evidence emission) — UNCHANGED.** N-AJ convergence evidence (DC-NODE-30) is emitted on
  the `Block` path (`block_received` / `block_admitted` / verdict). The anchor no-op is a rollback,
  not a `Block`, so it emits nothing — consistent with DC-NODE-30's emit-only-on-the-Block-path shape.

## Pure transformation?
**Yes.** The participant rollback decision is a pure function of `(peer chain-sync RollBack point,
persisted recovered anchor) → {accept-anchor-noop, fall through to the existing stored-block
resolution / fail-closed}`. No new nondeterminism; the peer item is canonical input, the anchor is
store-derived canonical state (DC-NODE-31).

## 1. What must always be true
- **AL-1 (participant boundary-point authority).** The recovered bootstrap anchor point (DC-NODE-31 /
  `BootstrapState.tip` / `state.recovered_anchor`) is an authoritative *local boundary point* for the
  participant live-follow: a peer `RollBackward` binding **exactly** (slot AND hash) to the persisted
  anchor is a valid rewind target on `run_participant_sync` — even though the anchor is a snapshot,
  not a stored servable ChainDb block.
- **AL-2 (participant rollback resolution stays total).** On `run_participant_sync`, a peer
  `RollBackward(point)` resolves: `point == persisted anchor` (slot ∧ hash) → idempotent no-op; else
  → the EXISTING DC-NODE-29 stored-block resolution (`get_block_by_hash` + stored slot/hash binding)
  UNCHANGED, which fails closed on unknown / Origin / slot-mismatch. The anchor branch is added
  BEFORE the existing resolution; nothing else moves.
- **AL-3 (single anchor authority).** The anchor consumed by `run_participant_sync` is
  `state.recovered_anchor` (DC-NODE-31, set at `node_lifecycle.rs:563`) — never re-read from the store
  inside the loop.
- **AL-4 (peer/persisted binding).** A peer-supplied rollback point is accepted as the anchor no-op
  only when it binds to the **persisted** anchor on slot AND hash — never slot-alone, never hash-alone.
- **AL-5 (idempotent no-op).** Participant rollback-to-recovered-anchor produces **no durable
  effect** — no `commit_rollback`, no `WalEntry::RollBack`, no tip/ledger/chain_dep mutation, no
  cursor, and `pending_reselection` is NOT set.
- **Preserved + verified (NOT new code):** the first forward block after the anchor admits through the
  EXISTING participant `LinearExtend → pump_block` path, whose `block_validity` `prev_hash` check
  binds it to the recovered `chain_dep` (the property AK-S2's OQ #2 probe proved).
- **Preserved (unchanged):** `RollBackward(Origin)` rejection (AI-S4a); the stored-block rollback
  authority (DC-NODE-29 / AI-S6); `pump_block` as sole admit; `ChainDb::tip()` / serve;
  **DC-NODE-32 (single-producer scope, NOT broadened)**; DC-CONS-03; T-REC-05; DC-NODE-31; N-AJ
  evidence (DC-NODE-30).

## 2. What must never be possible
- Accepting a participant rollback to any point other than the persisted recovered anchor via the new
  branch (Origin and every other point still go to the existing fail-closed / stored-block resolution).
- **Synthesizing** a ChainDb block for the anchor, **marking** it servable, or `ChainDb::tip()` /
  serve returning the anchor as a block.
- Admitting any forward block via a path other than `pump_block`.
- Trusting a peer slot/hash that disagrees with the persisted anchor.
- A participant rollback-to-anchor producing any durable mutation or setting `pending_reselection`.
- **Broadening DC-NODE-32's scope** (it stays single-producer; AL is the distinct sibling DC-NODE-33).

## 3. What must remain identical across executions
The participant rollback-resolution decision: pure `fn(peer_point, persisted_anchor) → {AnchorNoop,
ExistingResolution}`. No wall-clock / rand / HashMap / float.

## 4. What must be replay-equivalent
Same recovered store (same persisted anchor) + same ordered participant feed `[RollBackward(anchor),
RollForward(N+1), …]` → **byte-identical** post-state (ledger, chain_dep, ChainDb tip) and admit
sequence. Extends DC-NODE-31 / DC-NODE-32 / T-REC-05 replay-equivalence to the participant follow.

## 5. State transitions in scope (participant `run_participant_sync` RollBack handler)
```
(participant follow, RollBackward(Origin))
    → existing fail-closed (UnexpectedRollback)                # AI-S4a, unchanged
(participant follow, RollBackward(p)) where p == persisted_anchor (slot ∧ hash)
    → Ok(no-op)                                                # NEW: idempotent rewind (before get_block_by_hash)
(participant follow, RollBackward(p)) otherwise
    → existing DC-NODE-29 stored-block resolution (UNCHANGED)  # get_block_by_hash → apply_chain_event or fail closed
(participant follow, RollForward(b))                           # block N+1..
    → existing LinearExtend → pump_block admit (UNCHANGED)
```

## 6. TCB color hypothesis
- **BLUE** — the participant rollback-resolution predicate (pure, total): `(peer_point,
  persisted_anchor) → {AnchorNoop, FallThrough}`. The persisted anchor is already-BLUE canonical
  state (DC-NODE-31).
- **BLUE (unchanged)** — `pump_block`, `block_validity` (the forward admit), `apply_chain_event` (the
  stored-block rollback authority).
- **RED** — `run_participant_sync` orchestration: routing the `RollBack` item to the predicate,
  applying the no-op. No authority here.

## 7. Open questions
All OQ-AL questions resolved above (data-flow confirmed at HEAD `c3ec7466`:
`ForwardSyncState.recovered_anchor` populated for the participant path via `node_lifecycle.rs:563`;
`run_relay_loop_with_sched` passes the same `&mut fwd` to both dispatches). None remain blocking
`/cluster-doc`.

## Registry (declared after investigation)
**DC-NODE-33** (family DC, derived, `introduced_in = PHASE4-N-AL`, `status = declared`) — the
participant-path recovered-anchor rollback no-op, the MIRROR of DC-NODE-32 for `run_participant_sync`
(DC-NODE-32 stays scoped to `run_node_sync` — a distinct sibling, NOT a re-scoping). Statement in the
registry entry. `tests = []` (AL-S1 populates the named tests); `ci_script = ""` (Rust-test-enforced;
CE-AL-3-LIVE is the operator-run live preflight that gates CE-AI-6).
