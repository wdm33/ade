# PHASE4-N-AE.E — chain-sync server FindIntersect cursor (the CE-A5 closer)

## §1 Slice ID
PHASE4-N-AE.E

## §2 Cluster Exit Criteria addressed
- **CE-A5** — a real Haskell relay adopts an Ade-forged block (the manifest). AE.A/B/C
  were each necessary; AE.E is the final closer.

## §3 Implementation instruction (AI)
In `crates/ade_network/src/chain_sync/server.rs`, the `producer_chain_sync_serve`
`FindIntersect` handler resolves the intersect point and replies `IntersectFound` but does
not update the per-session read cursor (`last_announced`). Set `last_announced` to the
matched intersect point (`Point::Block` → `Some((slot, hash))`, `Point::Origin` → `None`)
so the subsequent `RequestNext` serves `next_after(intersect)`. Nothing else changes.

## §4 Intent (invariant impact)
A client that `FindIntersect`s at a **non-Origin** point (its own tip) must, on the next
`RequestNext`, receive the **successor** of that point — not the chain start. Before the
fix the server served `next_after(None)` = block 0, and the client rejected
`UnexpectedBlockNo(tip_block_no + 1)(0)`. This is the exact failure that blocked the prior
four CE-A5 reruns.

## §5 Design
The chain-sync server is a pure reducer over `(state, message, served)`. The
`FindIntersect` handler already calls `ServedHeaderLookup::intersect(points)` (which is
correct — verified on the live store) and replies `IntersectFound(point)`. The single
missing step: the resolved `point` IS the new read position, so
`state.last_announced` must be set to it. `producer_chain_sync_advance_tip` /
`RequestNext` then project `next_after(last_announced)` = the successor. Origin keeps the
cursor `None` (serve from the chain start — correct, and why Origin-sync clients in
earlier clusters were unaffected).

## §6 Execution boundary (TCB color)
- `crates/ade_network/src/chain_sync/server.rs` — **GREEN** (pure protocol reducer; no I/O,
  no clock, no rand). Unchanged color; the edit is a state assignment inside the existing
  pure transition.

## §7 Invariants preserved
- DC-PROTO-07, DC-PROTO-08 (chain-sync server agency / grammar closure) — unchanged.
- DC-PROTO-09 (receive transcript determinism) — untouched.
- The Origin-intersect behavior (`None` cursor → serve from the chain start) is preserved
  exactly (the `Point::Origin => None` arm).

## §8 Invariants strengthened
- **DC-PROTO-10** (introduced, enforced) — the chain-sync server FindIntersect cursor
  invariant.
- **DC-NODE-14** (strengthened) — the recover→follow→forge→serve→adopt path is now proven
  end-to-end live (the CE-A5 manifest).
- **CN-CONS-06** (strengthened) — the first live cross-impl acceptance (a real cardano-node
  adopts an Ade-forged block), via the `--mode node` AE spine.

## §10 Changes introduced
- `crates/ade_network/src/chain_sync/server.rs` — set `last_announced` in the
  `FindIntersect` handler + the regression test.
- `docs/ade-invariant-registry.toml` — DC-PROTO-10 (new); CN-CONS-06 + DC-NODE-14
  strengthenings.
- `docs/evidence/phase4-n-ae-ce-a5-relay-adoption.{md,jsonl}` — the manifest transcript.

## §11 Replay / crash / epoch validation
The server reducer is pure; the new regression test exercises a two-step
FindIntersect→RequestNext transition and asserts the cursor/RollForward target. No replay
corpus change.

## §12 Mechanical acceptance criteria
- **CE-E1** — `producer_chain_sync_serve_find_intersect_sets_cursor_then_rolls_forward_past_it`
  passes: FindIntersect at a non-tip block sets `last_announced` to that point, and the
  next `RequestNext` RollForwards the **successor** (not block 0).
- **CE-E2** — the existing 15 chain-sync server tests still pass (no regression; Origin
  intersect + RequestNext-from-empty + IntersectNotFound unchanged).
- **CE-A5** — the committed manifest `docs/evidence/phase4-n-ae-ce-a5-relay-adoption.md`:
  a real cardano-node 11.0.1 relay `AddedToCurrentChain` Ade's forged block 17, issuer =
  pool1 (`a1ed4e04` = blake2b-224(pool1 cold VK)), relay forging = 0.

## §14 Hard prohibitions
- No skip-past / no fallback in the serve path (the cursor is set to the resolved
  intersect, never widened).
- No change to `ChainDbServedSource` (it was already correct).
- No determinism tripwire in the GREEN reducer.

## §15 Explicit non-goals
- The post-adoption `SlotBeforeLastApplied` echo (a peer re-announcing Ade's own tip over
  the follow link) — a follow-up idempotency refinement, not this slice.
- The `--mode produce` operator-pass + sustained-window CE-N-C-LIVE capture (CN-CONS-06
  open_obligation) — unchanged.
