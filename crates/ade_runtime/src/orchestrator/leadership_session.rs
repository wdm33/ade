// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED leadership-session tokio task (PHASE4-N-K S5).
//!
//! Pulls slot ticks from the injected [`Clock`] and forwards them
//! to the orchestrator's inbox as `SlotTick` events. The
//! orchestrator core (S2) decides what happens at each tick;
//! producer leadership-check integration is a separate operator-
//! action concern (CN-CONS-06 live half).
//!
//! Slot arithmetic is pure (`millis_to_slot`); the only
//! nondeterministic input is the `Clock` itself — in tests, a
//! `DeterministicClock` makes the whole loop replayable.

use ade_types::SlotNo;
use tokio::sync::mpsc;

use crate::clock::{millis_to_slot, Clock};

use super::event::OrchestratorEvent;

/// Era anchor for slot arithmetic. Mirrors the
/// `EraSummary.{start_slot, slot_length_ms}` fields without
/// pulling the whole `EraSchedule` into the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotEraAnchor {
    pub start_slot: SlotNo,
    pub start_millis: u64,
    pub slot_length_ms: u32,
}

/// Tokio-driven leadership session. Owns the `Clock` and the
/// orchestrator inbox sender; emits one `SlotTick` per
/// `clock.next_tick()`.
pub struct LeadershipSession<C: Clock> {
    pub clock: C,
    pub events_out: mpsc::Sender<OrchestratorEvent>,
    pub anchor: SlotEraAnchor,
}

impl<C: Clock> LeadershipSession<C> {
    /// Drive the leadership loop. Exits cleanly when the clock is
    /// exhausted (`next_tick → None`) or the orchestrator inbox
    /// drops.
    pub async fn run(mut self) {
        loop {
            let tick_millis = match self.clock.next_tick() {
                Some(t) => t,
                None => return,
            };
            let slot = millis_to_slot(
                tick_millis,
                self.anchor.start_millis,
                self.anchor.start_slot,
                self.anchor.slot_length_ms,
            );
            let event = OrchestratorEvent::SlotTick {
                slot_millis: tick_millis,
                slot,
            };
            if self.events_out.send(event).await.is_err() {
                return;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::clock::DeterministicClock;

    #[tokio::test]
    async fn leadership_session_emits_one_event_per_clock_tick() {
        let (tx, mut rx) = mpsc::channel(16);
        let session = LeadershipSession {
            clock: DeterministicClock::new(0, vec![1000, 2000, 3000, 4000]),
            events_out: tx,
            anchor: SlotEraAnchor {
                start_slot: SlotNo(0),
                start_millis: 0,
                slot_length_ms: 1000,
            },
        };
        let handle = tokio::spawn(session.run());

        let mut ticks = Vec::new();
        while let Some(ev) = rx.recv().await {
            match ev {
                OrchestratorEvent::SlotTick { slot_millis, slot } => ticks.push((slot_millis, slot)),
                other => panic!("expected SlotTick, got {other:?}"),
            }
        }
        handle.await.expect("join");
        assert_eq!(
            ticks,
            vec![
                (1000u64, SlotNo(1)),
                (2000, SlotNo(2)),
                (3000, SlotNo(3)),
                (4000, SlotNo(4)),
            ]
        );
    }

    #[tokio::test]
    async fn leadership_session_slot_arithmetic_is_pure() {
        let run = || async {
            let (tx, mut rx) = mpsc::channel(16);
            let session = LeadershipSession {
                clock: DeterministicClock::new(0, vec![500, 1500, 2500]),
                events_out: tx,
                anchor: SlotEraAnchor {
                    start_slot: SlotNo(100),
                    start_millis: 0,
                    slot_length_ms: 1000,
                },
            };
            tokio::spawn(session.run());
            let mut out = Vec::new();
            while let Some(ev) = rx.recv().await {
                if let OrchestratorEvent::SlotTick { slot_millis, slot } = ev {
                    out.push((slot_millis, slot));
                }
            }
            out
        };
        let a = run().await;
        let b = run().await;
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn leadership_session_terminates_on_clock_exhaustion() {
        let (tx, mut rx) = mpsc::channel(4);
        let session = LeadershipSession {
            clock: DeterministicClock::new(0, vec![10, 20]),
            events_out: tx,
            anchor: SlotEraAnchor {
                start_slot: SlotNo(0),
                start_millis: 0,
                slot_length_ms: 10,
            },
        };
        let handle = tokio::spawn(session.run());
        let mut count = 0;
        while rx.recv().await.is_some() {
            count += 1;
        }
        handle.await.expect("join");
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn leadership_session_exits_when_orchestrator_drops() {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let session = LeadershipSession {
            clock: DeterministicClock::new(0, vec![100, 200, 300]),
            events_out: tx,
            anchor: SlotEraAnchor {
                start_slot: SlotNo(0),
                start_millis: 0,
                slot_length_ms: 100,
            },
        };
        let handle = tokio::spawn(session.run());
        handle.await.expect("join");
    }
}
