# SLICE S1 — keep-alive liveness under downstream backpressure

> The wire pump must service the N2N keep-alive mini-protocol on cadence **regardless of how long
> the downstream consumer stalls**. Shell-only (`ade_runtime` RED); **no BLUE edits** — the BLUE
> `keep_alive_transition` state machine and the session reducer are reused unchanged.

## Problem (from the 2026-08-01 live preview rehearsal)

A live preview run (`--mode node`, fresh Mithril bootstrap, forge-ON) was **shut down by the peer**
2 min 52 s after handshake. The Haskell relay's own log states the cause exactly:

```
03:41:19  HandshakeSuccess NodeToNodeV_15 (magic 2) → PromotedToHotRemote
03:44:11  MiniProtocolNum 8 (KeepAlive) ResponderDir terminated with exception
          ExceededTimeLimit (KeepAlive) ClientHasAgency (SingClient)
          → State: Dead → ShutdownPeer
```

Ade then observed `admission_wire_pump: exit=Eof` and the relay run loop exited. Reconstructed
timeline: `ping cookie=2` went out at ~+60–75 s, **no `pong cookie=2` was ever validated**, and Ade
sent nothing further — the peer killed us ~97 s after our last action.

### Root cause

`run_admission_wire_pump` (`crates/ade_runtime/src/admission/wire_pump.rs:215`) is structured as:

```rust
loop {
    while let Some(out) = outbox_payloads.pop_front() { flush_outbound(…).await }   // 1 flush
    let chunk = tokio::select! { transport.inbound.recv(), keep_alive_timer.tick() }; // 2 select
    …step(&mut state, chunk) → handle_chain_sync / handle_block_fetch → emit(…).await  // 3 dispatch
}
```

`emit` is `events_out.send(ev).await` on a **bounded(64)** channel
(`node_lifecycle.rs:5816`). When the consumer (`run_node_sync`: block application, epoch-boundary
fold, recovery-checkpoint capture) stalls, the channel fills and the loop **parks at step 3 —
outside the `select!`**. `keep_alive_timer.tick()` therefore cannot fire, no `MsgKeepAlive` is sent,
and the peer times us out.

The `KEEP_ALIVE_CADENCE` of 20 s against the peer's ~97 s limit (`wire_pump.rs:144-151`) was
designed correctly with ~3 missed-tick margin; **the cadence is simply unreachable while the loop is
blocked on the bounded send.** This is a liveness defect in the pump's control structure, not a
cadence-tuning problem.

### Why a send-only fix is insufficient

Servicing only the *timer* while blocked would emit one `MsgKeepAlive`, moving the BLUE state to
`ClientAwaiting`. The echoed `MsgResponseKeepAlive` arrives on `transport.inbound`, which the pump
is not reading while blocked, so the state never returns to `ClientIdle` and the cadence guard
(`wire_pump.rs:242`) suppresses every later tick. That buys ~117 s total tolerance and leaves a
latent time-bomb: the observed stall already exceeded 97 s, and its duration is unbounded (it scales
with ledger size). **The keep-alive must stay live in BOTH directions for an arbitrarily long
stall.**

## Invariants

- **INV-WL-1 (liveness).** No downstream consumer stall, of any duration or cause, prevents the pump
  from sending `MsgKeepAlive` on cadence and validating the echoed response.
- **INV-WL-2 (replay equivalence).** The sequence of `AdmissionPeerEvent`s delivered to `events_out`
  is byte-identical to the pre-slice pump for the same peer input. Keep-alive remains wire-only and
  emits no event (DC-PUMP-01/03 unchanged).
- **INV-WL-3 (bounded deferral).** Frames deferred while backpressured are held in a queue with a
  fixed cap; exceeding it **fails closed** with a typed error. No unbounded buffering
  (DC-LIVEMEM-01 / DC-SESS-04 discipline).
- **INV-WL-4 (no new demand while backpressured).** No chain-sync `RequestNext` / block-fetch
  `RequestRange` is issued while the pump is waiting for `events_out` capacity, so the peer's
  in-flight bytes are what bound the deferral.
- **INV-WL-5 (no BLUE edit).** `keep_alive_transition`, the session reducer `step`, and every typed
  halt are reused unchanged.

## Design

### 1. Hoist emission into the main loop

`handle_chain_sync` / `handle_block_fetch` currently call `emit(…).await` internally, where
`state`, `transport` and the keep-alive machinery are out of scope. Change them to **push events
into a `&mut VecDeque<AdmissionPeerEvent>`** instead. Both become synchronous and lose their
`events_out` parameter; all sequencing (`chain_sync_in_flight` / `block_fetch_in_flight`, outbox
queuing) is untouched. The main loop drains that queue through the cooperative emit below, so
ordering is preserved by construction (INV-WL-2).

### 2. `flush_outbound` takes `&Sender` instead of `&mut MuxTransportHandle`

It only ever uses `transport.outbound.send`. Narrowing the parameter lets the cooperative emit hold
`&mut transport.inbound` and `&transport.outbound` simultaneously as disjoint field borrows.

### 3. Cooperative emit

```rust
async fn emit_cooperative(
    events_out: &mpsc::Sender<AdmissionPeerEvent>,
    ev: AdmissionPeerEvent,
    state: &mut SessionState,
    inbound: &mut mpsc::Receiver<Vec<u8>>,
    outbound: &mpsc::Sender<Vec<u8>>,
    ka: &mut KeepAliveLane,                 // state, next_cookie, version, timer
    deferred: &mut VecDeque<(AcceptedMiniProtocol, Vec<u8>)>,
    peer_addr: &str,
) -> Result<(), AdmissionWirePumpResult>
```

Loop over `tokio::select!` until a permit is obtained:

- `events_out.reserve()` → `break` with the permit, then `permit.send(ev)`.
- `ka.timer.tick()` → if `ClientIdle`, run the BLUE transition and **flush the frame immediately**
  via `flush_outbound` (not via `outbox_payloads`, which is only drained by the outer loop and would
  never be sent while blocked).
- `inbound.recv()` → `step(state, Inbound(chunk))`; dispatch effects:
  `SendBytes` → send; `DeliverPeerFrame{KeepAlive}` → `handle_keep_alive` (returns the BLUE state to
  `ClientIdle`, satisfying INV-WL-1); every other `DeliverPeerFrame` → `deferred.push_back(…)` with
  the INV-WL-3 cap check.

All three arms are cancel-safe (`reserve`, `Interval::tick`, `Receiver::recv`), so the `select!` is
sound.

### 4. Deferred drain

At the top of the outer loop, **before** reading new inbound, drain `deferred` in order through the
same dispatch path. Combined with INV-WL-4 (the outbox is not flushed while blocked, so no new
requests go out), the peer quiesces after its in-flight batch and only keep-alive traffic continues.

### 5. Bound + typed halt

`MAX_DEFERRED_PEER_FRAMES` with a new `AdmissionWirePumpError::DeferredFrameOverflow` variant.
`AdmissionWirePumpError` is not matched exhaustively outside `wire_pump.rs`, so the addition is
contained.

## Mechanical acceptance criteria

- **CE-WL-1 (liveness under stall).** A test consumer that holds `events_out` full for > 3×
  `KEEP_ALIVE_CADENCE` still sees `MsgKeepAlive` frames emitted on cadence and their responses
  validated; the pump does not exit and no error is returned.
- **CE-WL-2 (ordering).** Under induced backpressure the `AdmissionPeerEvent` sequence is identical
  to the unstalled run over the same scripted peer input.
- **CE-WL-3 (bounded).** Exceeding `MAX_DEFERRED_PEER_FRAMES` returns
  `Error(DeferredFrameOverflow)` — fail closed, no unbounded growth.
- **CE-WL-4 (live).** A live preview `--mode node` run survives the consumer stall that produced the
  03:44:11Z `ExceededTimeLimit (KeepAlive)` shutdown, holding the peer connection across it.

## NOT claimed here

Reconnect-after-EOF (`DC-NODE-19`, still *declared*) is a **separate** defect and a separate slice:
this slice keeps the connection alive, it does not re-establish a lost one. The *cause* of the
>97 s consumer stall is a throughput question owned by LIVE-FOLLOW-THROUGHPUT and is deliberately
not addressed here — INV-WL-1 is designed to hold regardless of it.
