// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Clock seam (PHASE4-N-K S1).
//!
//! The orchestrator core (PHASE4-N-K S2) consumes time exclusively
//! through the [`Clock`] trait. Production uses [`SystemClock`]
//! (RED sub-classified); tests and replay harnesses use
//! [`DeterministicClock`] (pure — no wall-clock read).
//!
//! `crates/ade_runtime/src/clock.rs` is the sole site of
//! `SystemTime::now()` / `Instant::now()` within `ade_runtime` —
//! enforced by `ci/ci_check_clock_seam.sh`.

use ade_types::SlotNo;

/// Time source for the orchestrator. `now_millis` returns
/// monotonic-ish milliseconds since the era anchor; `next_tick`
/// blocks/yields the next slot-boundary tick (or `None` if the
/// clock has been exhausted, e.g. a deterministic clock at end
/// of its tick vector).
pub trait Clock: Send + Sync {
    fn now_millis(&self) -> u64;
    fn next_tick(&mut self) -> Option<u64>;
}

/// Deterministic clock for replay + integration tests. Yields a
/// pre-computed tick vector; `now_millis` returns the most-recently-
/// produced tick value (or the anchor if no tick has been produced).
///
/// Pure: no wall-clock read, no rand. Two instances built from the
/// same `(anchor_millis, ticks)` produce identical outputs.
pub struct DeterministicClock {
    anchor_millis: u64,
    ticks: Vec<u64>,
    cursor: usize,
}

impl DeterministicClock {
    pub fn new(anchor_millis: u64, ticks: Vec<u64>) -> Self {
        Self {
            anchor_millis,
            ticks,
            cursor: 0,
        }
    }

    pub fn anchor_millis(&self) -> u64 {
        self.anchor_millis
    }

    pub fn remaining_ticks(&self) -> usize {
        self.ticks.len().saturating_sub(self.cursor)
    }
}

impl Clock for DeterministicClock {
    fn now_millis(&self) -> u64 {
        if self.cursor == 0 {
            self.anchor_millis
        } else {
            self.ticks[self.cursor - 1]
        }
    }

    fn next_tick(&mut self) -> Option<u64> {
        let t = *self.ticks.get(self.cursor)?;
        self.cursor += 1;
        Some(t)
    }
}

/// Translate a wall-clock millisecond value into a slot number,
/// given a `(start_slot, start_millis, slot_length_ms)` anchor.
///
/// Pure arithmetic; used by the leadership session to lift
/// `Clock::next_tick()` outputs into `SlotNo` values.
pub fn millis_to_slot(
    tick_millis: u64,
    start_millis: u64,
    start_slot: SlotNo,
    slot_length_ms: u32,
) -> SlotNo {
    if slot_length_ms == 0 {
        return start_slot;
    }
    let delta_ms = tick_millis.saturating_sub(start_millis);
    let slots_since_anchor = delta_ms / slot_length_ms as u64;
    SlotNo(start_slot.0.saturating_add(slots_since_anchor))
}

/// Production wall-clock impl. RED sub-classified — never
/// reachable from BLUE or GREEN code paths. Lives in this file
/// only to keep the seam single-sited.
///
/// The `now_millis` impl reads `std::time::SystemTime`. The
/// `next_tick` impl blocks until the next slot boundary by
/// computing the residual milliseconds and sleeping via
/// `std::thread::sleep`. The orchestrator's tokio runner
/// (S5) drives this via `spawn_blocking` or replaces it with a
/// tokio-aware variant — but the trait surface stays the same.
#[allow(clippy::disallowed_methods)]
pub struct SystemClock {
    slot_length_ms: u32,
    next_boundary_millis: u64,
}

impl SystemClock {
    pub fn new(slot_length_ms: u32) -> Self {
        let now = wall_clock_millis();
        let next = match slot_length_ms {
            0 => now,
            n => {
                let r = now % n as u64;
                if r == 0 {
                    now
                } else {
                    now + (n as u64 - r)
                }
            }
        };
        Self {
            slot_length_ms,
            next_boundary_millis: next,
        }
    }
}

impl Clock for SystemClock {
    fn now_millis(&self) -> u64 {
        wall_clock_millis()
    }

    fn next_tick(&mut self) -> Option<u64> {
        let now = wall_clock_millis();
        if now < self.next_boundary_millis {
            let gap = self.next_boundary_millis - now;
            std::thread::sleep(std::time::Duration::from_millis(gap));
        }
        let fired = self.next_boundary_millis;
        let step = self.slot_length_ms.max(1) as u64;
        self.next_boundary_millis = fired.saturating_add(step);
        Some(fired)
    }
}

fn wall_clock_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_clock_is_pure() {
        let ticks = vec![1000u64, 2000, 3000, 4000, 5000];
        let mut a = DeterministicClock::new(0, ticks.clone());
        let mut b = DeterministicClock::new(0, ticks);
        let drain = |c: &mut DeterministicClock| -> Vec<u64> {
            let mut out = Vec::new();
            while let Some(t) = c.next_tick() {
                out.push(t);
            }
            out
        };
        assert_eq!(drain(&mut a), drain(&mut b));
    }

    #[test]
    fn deterministic_clock_emits_in_order() {
        let mut c = DeterministicClock::new(0, vec![10, 20, 30]);
        assert_eq!(c.next_tick(), Some(10));
        assert_eq!(c.next_tick(), Some(20));
        assert_eq!(c.next_tick(), Some(30));
        assert_eq!(c.next_tick(), None);
        // now_millis returns the most-recent tick value.
        assert_eq!(c.now_millis(), 30);
    }

    #[test]
    fn deterministic_clock_now_millis_is_anchor_before_first_tick() {
        let c = DeterministicClock::new(12345, vec![1, 2]);
        assert_eq!(c.now_millis(), 12345);
    }

    #[test]
    fn millis_to_slot_anchored_arithmetic() {
        // anchor: slot 0 at millis 0, 1000ms/slot.
        assert_eq!(millis_to_slot(0, 0, SlotNo(0), 1000), SlotNo(0));
        assert_eq!(millis_to_slot(999, 0, SlotNo(0), 1000), SlotNo(0));
        assert_eq!(millis_to_slot(1000, 0, SlotNo(0), 1000), SlotNo(1));
        assert_eq!(millis_to_slot(1500, 0, SlotNo(0), 1000), SlotNo(1));
        assert_eq!(millis_to_slot(2000, 0, SlotNo(0), 1000), SlotNo(2));
    }

    #[test]
    fn millis_to_slot_handles_pre_anchor() {
        // tick before anchor → slot floor at start_slot.
        assert_eq!(
            millis_to_slot(500, 1000, SlotNo(10), 1000),
            SlotNo(10),
            "pre-anchor saturates"
        );
    }

    #[test]
    fn millis_to_slot_handles_zero_slot_length() {
        // Degenerate; returns start_slot rather than dividing by 0.
        assert_eq!(millis_to_slot(99999, 0, SlotNo(42), 0), SlotNo(42));
    }
}
