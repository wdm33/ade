# Invariant Slice — PHASE4-N-K S5

## Slice Header

**Slice Name:** RED `orchestrator::leadership_session` — slot-tick
producer pump driven by `Clock::tick_stream()`.
**Cluster:** PHASE4-N-K
**Status:** In Progress
**CEs addressed:** contributes to CE-N-K-4 (DC-NODE-03 — slot
ticks enter the orchestrator only via the `Clock` seam).
**Registry effects on merge:** none yet (DC-NODE-03 flips at S8).
**Dependencies:** S1 (`Clock`), S2 (orchestrator event sum).

---

## Intent

Slot ticks reach the orchestrator core only through `Clock`. The
production wall-clock `SystemClock` and the test
`DeterministicClock` are interchangeable from the orchestrator's
perspective. The leadership session also reads the wall-clock
boundary into the event stream — there is no other `Instant::now`
reachable from `orchestrator::core`.

---

## Scope

- `crates/ade_runtime/src/orchestrator/leadership_session.rs` —
  RED task that loops over `clock.next_tick()` and pushes
  `OrchestratorEvent::SlotTick` onto the orchestrator inbox.
- `crates/ade_runtime/src/orchestrator/mod.rs` — re-export.

This slice does **not** introduce a producer scheduler call. The
N-C `scheduler_step` is invoked by the orchestrator core in
response to `SlotTick` events; integrating that into S2 is
deferred because Tier 1 producer evidence is operator-action
work (CN-CONS-06 live half remains open per the sketch §11).

---

## Execution Boundary

- **BLUE:** none.
- **GREEN:** unchanged.
- **RED:** `orchestrator::leadership_session`.

---

## Invariants Preserved

- DC-NODE-03 — leadership session never reads wall-clock or rand
  except through `Clock`.

## Invariants Strengthened or Introduced

- DC-NODE-03 (partial: leadership-session-side compliance).

---

## Design Summary

```rust
pub struct LeadershipSession<C: Clock> {
    pub clock: C,
    pub events_out: mpsc::Sender<OrchestratorEvent>,
    pub slot_length_ms: u32,
    pub era_anchor: SlotEraAnchor, // start_slot + start_millis pair
}

impl<C: Clock> LeadershipSession<C> {
    pub async fn run(mut self);
}
```

Loop body:
1. `let Some(tick_millis) = self.clock.next_tick() else { break; }`
2. Compute `slot = era_anchor.start_slot + (tick_millis -
   era_anchor.start_millis) / slot_length_ms`.
3. `events_out.send(SlotTick { slot_millis: tick_millis, slot
   }).await`. Send failure → orchestrator dropped, exit.

---

## Replay, Crash, and Epoch Validation

- **Tests:**
  - `leadership_session_emits_one_event_per_clock_tick` — drive
    `DeterministicClock` with 10 ticks; observe exactly 10
    `SlotTick` events in order.
  - `leadership_session_slot_arithmetic_is_pure` — two
    runs over the same `(tick_vector, anchor, slot_length)`
    produce identical `(slot, slot_millis)` pairs.
  - `leadership_session_terminates_on_clock_exhaustion` —
    `DeterministicClock` empty → loop exits cleanly.

## §12 Mechanical Acceptance Criteria

- [ ] `leadership_session_emits_one_event_per_clock_tick`
- [ ] `leadership_session_slot_arithmetic_is_pure`
- [ ] `leadership_session_terminates_on_clock_exhaustion`

---

## §14 Hard Prohibitions

- No `Instant::now()` / `SystemTime::now()` / `tokio::time::Instant`
  in this file (clock is injected).
- No `unwrap()` / `expect()` / `panic!()` in non-test code.
- No production forge/sign call (that's behind the orchestrator
  core's response to `SlotTick`; in S5's scope the production
  routing remains operator-action work).

## §15 Explicit Non-Goals

- No producer forge integration into the core (out of cluster;
  CN-CONS-06 live obligation).
- No mempool tick.
- No epoch boundary computation (consumed by the existing
  producer scheduler).

---

## §16 Completion Checklist

- [ ] All §12 tests added and passing.
- [ ] No new CI gate (DC-NODE-03 grep gate lands at S8 over the
  whole orchestrator dir).
