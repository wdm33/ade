// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED keep-alive session (PHASE4-N-L S8).
//!
//! Drives the keep-alive mini-protocol at a fixed cadence using
//! the PHASE4-N-K `Clock` seam (DC-SESS-05). The cadence is pinned
//! at compile time (`KeepAliveCadence::DEFAULT` = 60 s);
//! operator-tunable cadence is forbidden in this cluster (Tier 5
//! future).
//!
//! Behaviour:
//!   - Loop body pulls one tick from `clock.next_tick()`.
//!   - Emits `OrchestratorEvent::OutboundKeepAlive { peer_id }`
//!     onto the inbox.
//!   - Loop exits when the clock is exhausted (deterministic test
//!     clock) or the orchestrator inbox is dropped (production).

use tokio::sync::mpsc;

use crate::clock::Clock;

use super::event::{OrchestratorEvent, PeerId};

/// Keep-alive cadence parameters. Pinned defaults — operator-
/// tunable cadence is explicitly forbidden in PHASE4-N-L.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeepAliveCadence {
    pub interval_ms: u32,
}

impl KeepAliveCadence {
    pub const DEFAULT: Self = Self {
        interval_ms: 60_000,
    };
}

impl Default for KeepAliveCadence {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Per-peer keep-alive task. Owns the clock + the orchestrator
/// inbox sender; emits one `OutboundKeepAlive` event per tick.
pub struct KeepAliveSession<C: Clock> {
    pub clock: C,
    pub cadence: KeepAliveCadence,
    pub peer_id: PeerId,
    pub events_out: mpsc::Sender<OrchestratorEvent>,
}

impl<C: Clock> KeepAliveSession<C> {
    pub async fn run(mut self) {
        loop {
            let _tick = match self.clock.next_tick() {
                Some(t) => t,
                None => return,
            };
            let event = OrchestratorEvent::OutboundKeepAlive {
                peer_id: self.peer_id,
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
    async fn keep_alive_session_emits_one_event_per_clock_tick() {
        let (tx, mut rx) = mpsc::channel(16);
        let session = KeepAliveSession {
            clock: DeterministicClock::new(0, vec![60_000, 120_000, 180_000, 240_000, 300_000]),
            cadence: KeepAliveCadence::DEFAULT,
            peer_id: PeerId(7),
            events_out: tx,
        };
        let handle = tokio::spawn(session.run());
        let mut count = 0;
        while let Some(ev) = rx.recv().await {
            match ev {
                OrchestratorEvent::OutboundKeepAlive { peer_id } => {
                    assert_eq!(peer_id, PeerId(7));
                    count += 1;
                }
                other => panic!("expected OutboundKeepAlive, got {other:?}"),
            }
        }
        handle.await.expect("join");
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn keep_alive_session_is_pure_under_deterministic_clock() {
        let run = || async {
            let (tx, mut rx) = mpsc::channel(8);
            let session = KeepAliveSession {
                clock: DeterministicClock::new(0, vec![60_000, 120_000, 180_000]),
                cadence: KeepAliveCadence::DEFAULT,
                peer_id: PeerId(1),
                events_out: tx,
            };
            tokio::spawn(session.run());
            let mut out = Vec::new();
            while let Some(ev) = rx.recv().await {
                if let OrchestratorEvent::OutboundKeepAlive { peer_id } = ev {
                    out.push(peer_id);
                }
            }
            out
        };
        let a = run().await;
        let b = run().await;
        assert_eq!(a, b);
        assert_eq!(a.len(), 3);
    }

    #[test]
    fn keep_alive_cadence_default_is_60s() {
        assert_eq!(KeepAliveCadence::DEFAULT.interval_ms, 60_000);
    }

    #[tokio::test]
    async fn keep_alive_session_exits_when_orchestrator_drops() {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let session = KeepAliveSession {
            clock: DeterministicClock::new(0, vec![1, 2, 3]),
            cadence: KeepAliveCadence::DEFAULT,
            peer_id: PeerId(1),
            events_out: tx,
        };
        let handle = tokio::spawn(session.run());
        handle.await.expect("join");
    }
}
