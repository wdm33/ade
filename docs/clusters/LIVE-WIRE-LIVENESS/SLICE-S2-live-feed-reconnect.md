# SLICE S2 — reconnect an established live feed

> A transport-level loss of the live `--peer` feed must be *recovered*, not fatal. Shell-only
> (`ade_node`, RED); **no BLUE edits**, and every authority decision stays where it is.

## Problem (same 2026-08-01 live rehearsal as S1)

After the peer shut the connection down, Ade logged `admission_wire_pump: exit=Eof` and then
`ade_node --mode node: relay run loop exited` — the run ended. S1 stops us being *disconnected for
going silent*; it does nothing about a disconnect that happens anyway (relay restart, network blip,
peer-side resource pressure). For a producer that must hold the tip for hours waiting on a leader
slot, a single transient disconnect still ends the run.

### Why the run ends today

`spawn_live_wire_pump_source` (`node_lifecycle.rs:1274`) spawns, per `--peer`, a **one-shot** task:
dial → `run_admission_wire_pump` → return. When the pump returns, the task ends, `lane_tx` drops,
the lane closes, `fair_merge` ends, `merged_tx` drops, the feed ends.

There is a second, independent trigger: the pump's `finalize` emits
`AdmissionPeerEvent::Disconnected` before returning, and the consumer treats that as a feed end —
`NodeBlockSource::pump_lookahead` sets its `disconnected` flag (`node_sync.rs:240/406`), which is
latched. **Both** must be addressed; reconnecting the transport while still surfacing `Disconnected`
would leave the feed permanently ended.

## Invariants

- **INV-WL-6 (recover the transport, never the authority).** Reconnect re-establishes *only* the
  wire session. Every consensus decision — admission, rollback k-guard, boundary promotion, forge
  fence — stays in the consumer and keeps its existing typed halt. Reconnect can never convert a
  fail-closed authority outcome into a retry.
- **INV-WL-7 (resume without a gap).** The new session resumes from the last block actually
  delivered downstream, so no block is skipped. Events already buffered in the lane are still
  delivered (the lane is never dropped), so the consumer sees a contiguous sequence.
- **INV-WL-8 (startup semantics unchanged).** Reconnect applies only to a session that was
  *established and then lost*. An unparseable `--peer`, or a first dial that fails, behaves exactly
  as before: logged-and-dropped, feed ends, clean halt. An unreachable peer must never become an
  infinite boot spin.
- **INV-WL-9 (transport-only trigger).** Reconnect fires on `Eof` / `TransportRead` /
  `TransportWrite`. A peer **protocol or grammar violation** (`Session`, `ChainSyncDecode`,
  `BlockFetchDecode`, `UnexpectedProtocolMessage`, `UnsupportedRollbackPoint`, `KeepAlive`,
  `DeferredFrameOverflow`) keeps today's fail-closed drop — a systematically bad peer is not retried
  into a livelock. `EventsChannelDropped` means the consumer is gone: exit.
- **INV-WL-10 (bounded, deterministic backoff).** A fixed escalating schedule, no randomness, capped.

## Design

All of it inside `spawn_live_wire_pump_source`; no other signature changes.

### 1. Per-peer supervisor loop

The one-shot task becomes `dial → pump → classify → maybe re-dial`. `established` starts false and
is set on the first successful dial; while false a dial failure returns (INV-WL-8), and only after
it is set does a dial failure back off and retry.

### 2. Interpose between pump and lane

The pump gets an inner channel; the supervisor forwards inner → `lane_tx`. This buys two things:

- **`Disconnected` is swallowed** — it is a per-*session* artifact, and surfacing it would latch the
  consumer's feed-end flag. The lane itself stays open across reconnects, so the feed never ends.
- **The resume point is observable.** The supervisor keeps a clone of the most recent
  `AdmissionPeerEvent::Block` payload and decodes it **only on reconnect** (one decode per
  reconnect, not per block), yielding the `Point` for the next `FindIntersect`.

Forwarding is 1:1 with `send().await`, so the DC-PUMP-04 per-peer self-backpressure is preserved.

### 3. Why "last delivered block" is the correct resume point

The lane is FIFO and is never dropped, so everything the old session forwarded is still delivered.
The new session's events are appended *after* those. So by the time the consumer processes the new
session's `RollBackward` to the intersection, it has already admitted every buffered block —
including the resume point — and the rollback target is present in the `ChainDb`, within k.

Resuming from the *original* boot anchor instead would be wrong, not merely slow: once the node has
advanced more than k past it, the peer's rollback to that anchor exceeds the k-guard and fails
closed.

If no block was delivered in the session, the previous start point is reused.

## Mechanical acceptance criteria

- **CE-WL-5.** A peer that accepts, serves, then closes the connection is re-dialed, and blocks from
  the *second* session reach the consumer — the feed does not end.
- **CE-WL-6.** `Disconnected` from a session that will be retried is not surfaced downstream (the
  consumer's feed-end flag is never latched).
- **CE-WL-7.** A first-dial failure and an unparseable `--peer` still yield an ended feed
  (INV-WL-8) — the existing `spawn_live_wire_pump_source_with_no_usable_peer_yields_ended_feed`
  behaviour is preserved.
- **CE-WL-8 (live).** A live preview run survives a peer restart and resumes following.

## NOT claimed here

Reconnect does not repair a **deep reorg past the resume point**: if the peer cannot find the
resume point, chain-sync behaves exactly as it does for an initial connect today (no retry ladder of
older candidate points). That is unchanged pre-existing behaviour, called out rather than silently
inherited. Multi-peer failover/selection is untouched — this is per-peer session recovery only, and
`select_best_chain` remains arrival-order independent (CN-CONS-01).
