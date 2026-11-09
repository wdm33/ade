# Invariant Slice — PHASE4-N-L S8

**Slice Name:** RED `keep_alive_session` — Clock-driven periodic ping pump.
**Cluster:** PHASE4-N-L
**Status:** In Progress
**CEs addressed:** part of CE-N-L-7 (DC-SESS-05 keep-alive side).
**Dependencies:** S6 + PHASE4-N-K Clock.

## Intent

Drive the keep-alive mini-protocol at a fixed cadence using `ade_runtime::clock::Clock`. The session core (S2) already routes keep-alive frames; this slice is the periodic-tick producer that emits `OrchestratorEvent::OutboundKeepAlive` events into the inbox at the cadence interval.

The cadence is pinned at compile time (`KeepAliveCadence::DEFAULT` = 60s); operator-tunable cadence is forbidden (Tier 5 future).

## Scope

- `crates/ade_runtime/src/orchestrator/keep_alive_session.rs` — new RED file.
- Extend `OrchestratorEvent` with `OutboundKeepAlive { peer_id }` variant (additive).

```rust
pub struct KeepAliveCadence {
    pub interval_ms: u32,
}
impl KeepAliveCadence { pub const DEFAULT: Self = Self { interval_ms: 60_000 }; }

pub struct KeepAliveSession<C: Clock> {
    pub clock: C,
    pub cadence: KeepAliveCadence,
    pub peer_id: PeerId,
    pub events_out: mpsc::Sender<OrchestratorEvent>,
}

impl<C: Clock> KeepAliveSession<C> {
    pub async fn run(mut self);
}
```

Loop body: `clock.next_tick()` → emit `OutboundKeepAlive { peer_id }`. Sleep handled by the clock.

## §12 Mechanical Acceptance Criteria

- [ ] `keep_alive_session_emits_one_event_per_clock_tick` — deterministic clock with 5 ticks → 5 `OutboundKeepAlive` events.
- [ ] `keep_alive_session_is_pure_under_deterministic_clock` — two runs over the same tick vector → identical event sequence.
- [ ] `keep_alive_cadence_default_is_60s` — pinned constant check.

## §14 Hard Prohibitions

- No `Instant::now()` / `SystemTime::now()` / `tokio::time::*`.
- No operator-tunable cadence.
- No `mpsc::unbounded_channel`.

## §15 Non-Goals

- No keep-alive timeout enforcement (peer-side liveness checking is operator-evidence territory).
- No exponential backoff.
