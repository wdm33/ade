// Core Contract:
// - Deterministic: same inputs => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Pure: no I/O, no clock, no allocation, no await

//! GREEN loop planner (PHASE4-N-F-D S1).
//!
//! The pure lifecycle decision function for the `--mode node` relay run loop.
//! `plan_loop_step` selects each iteration's [`LoopStep`] from three closed,
//! orthogonal, lifecycle-level inputs — operator intent, momentary source
//! readiness, and structural feed liveness. It owns **no authority**: it
//! cannot decide ledger validity, chain selection, leadership, forge
//! eligibility, or evidence, and its closed output vocabulary makes an
//! authority decision unrepresentable as a planner result.
//!
//! This is the mechanical proof of the cluster's central line — RED performs
//! effects, GREEN plans iteration, BLUE authority stays behind the existing
//! closed seams (`bootstrap_initial_state`, `run_node_sync` -> `pump_block`).
//! The RED relay loop (S2) is a thin composer over this function; it advances
//! the durable tip only through the existing sync seam, never here.
//!
//! Honest scope: lands tested-but-unwired. S2 feeds it real status.

/// Operator-intent input. Whether a shutdown signal (SIGINT/SIGTERM) has been
/// observed. Lifecycle-level only — carries no chain or ledger content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownStatus {
    /// No shutdown requested; the loop may keep cycling.
    Running,
    /// A shutdown was requested; the loop must halt at the next boundary.
    ShutdownRequested,
}

/// Momentary source-readiness input. Whether a block is ready to pump *now*
/// — i.e. whether a subsequent `next_block()` is expected to make progress.
/// This is the closed projection of the RED, content-blind `NodeBlockSource`
/// readiness signal: a yes/no, never a block / hash / slot / verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// A block is available to pump on the next sync step.
    WorkAvailable,
    /// No block is ready right now (the feed may yet deliver more).
    NoWorkReady,
}

/// Structural feed-liveness input. Whether the source feed has ended (a clean
/// disconnect, or a closed-and-drained channel). Distinct from [`SyncStatus`]:
/// `NoWorkReady` is momentary ("nothing right now"); `Ending` is structural
/// ("the feed is over"). Lifecycle-level only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopState {
    /// The source feed is still live; the loop should keep cycling.
    Continuing,
    /// The source feed has ended; the loop should wind down to a clean halt.
    Ending,
}

/// The closed set of lifecycle steps the relay loop may take. This is the
/// whole vocabulary — there is deliberately no variant that could encode an
/// authority decision (apply / forge / admit / evidence). Not
/// `#[non_exhaustive]`: a new step is a compile error until wired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopStep {
    /// Run one `run_node_sync` step (drain the currently-available batch
    /// through the single `pump_block` seam, capturing its checkpoint).
    SyncOnce,
    /// No work right now and the feed is still live — wait (in the RED driver,
    /// a cancellation-safe select on source-readiness or shutdown).
    Idle,
    /// Halt the loop cleanly at this boundary, leaving on-disk state
    /// recoverable.
    HaltCleanly,
}

/// Select the next relay-loop step from the three closed lifecycle inputs.
///
/// Pure, total, and deterministic: same inputs => same [`LoopStep`], with no
/// I/O, clock, allocation, or `await`. Precedence:
///
/// 1. a requested shutdown halts promptly at the next boundary (it does not
///    start new work);
/// 2. otherwise, available work drains first (even while the feed is ending);
/// 3. a drained, ended feed halts cleanly;
/// 4. an open feed with no work right now idles.
///
/// Total over all 2x2x2 input combinations via an exhaustive `match` with no
/// wildcard arm.
pub fn plan_loop_step(
    loop_state: LoopState,
    sync_status: SyncStatus,
    shutdown_status: ShutdownStatus,
) -> LoopStep {
    match shutdown_status {
        ShutdownStatus::ShutdownRequested => LoopStep::HaltCleanly,
        ShutdownStatus::Running => match sync_status {
            SyncStatus::WorkAvailable => LoopStep::SyncOnce,
            SyncStatus::NoWorkReady => match loop_state {
                LoopState::Ending => LoopStep::HaltCleanly,
                LoopState::Continuing => LoopStep::Idle,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every input combination, spelled out — the decision table is total.
    const STATES: [LoopState; 2] = [LoopState::Continuing, LoopState::Ending];
    const SYNCS: [SyncStatus; 2] = [SyncStatus::WorkAvailable, SyncStatus::NoWorkReady];
    const SHUTDOWNS: [ShutdownStatus; 2] =
        [ShutdownStatus::Running, ShutdownStatus::ShutdownRequested];

    /// Expected output for every one of the 8 input combinations.
    fn expected(s: LoopState, y: SyncStatus, d: ShutdownStatus) -> LoopStep {
        match d {
            ShutdownStatus::ShutdownRequested => LoopStep::HaltCleanly,
            ShutdownStatus::Running => match y {
                SyncStatus::WorkAvailable => LoopStep::SyncOnce,
                SyncStatus::NoWorkReady => match s {
                    LoopState::Ending => LoopStep::HaltCleanly,
                    LoopState::Continuing => LoopStep::Idle,
                },
            },
        }
    }

    #[test]
    fn plan_loop_step_decision_table_is_total() {
        let mut count = 0;
        for &s in &STATES {
            for &y in &SYNCS {
                for &d in &SHUTDOWNS {
                    assert_eq!(
                        plan_loop_step(s, y, d),
                        expected(s, y, d),
                        "case {s:?},{y:?},{d:?}"
                    );
                    count += 1;
                }
            }
        }
        assert_eq!(count, 8, "all 2x2x2 input combinations must be covered");
    }

    #[test]
    fn plan_loop_step_is_deterministic() {
        for &s in &STATES {
            for &y in &SYNCS {
                for &d in &SHUTDOWNS {
                    let a = plan_loop_step(s, y, d);
                    let b = plan_loop_step(s, y, d);
                    assert_eq!(a, b, "deterministic for {s:?},{y:?},{d:?}");
                }
            }
        }
    }

    #[test]
    fn shutdown_halts_even_with_work_available() {
        // Shutdown takes precedence over available work and a live feed.
        assert_eq!(
            plan_loop_step(
                LoopState::Continuing,
                SyncStatus::WorkAvailable,
                ShutdownStatus::ShutdownRequested,
            ),
            LoopStep::HaltCleanly,
        );
    }

    #[test]
    fn available_work_drains_before_ending_halts() {
        // Work drains even while the feed is ending; only a drained+ended feed
        // halts.
        assert_eq!(
            plan_loop_step(
                LoopState::Ending,
                SyncStatus::WorkAvailable,
                ShutdownStatus::Running
            ),
            LoopStep::SyncOnce,
        );
        assert_eq!(
            plan_loop_step(
                LoopState::Ending,
                SyncStatus::NoWorkReady,
                ShutdownStatus::Running
            ),
            LoopStep::HaltCleanly,
        );
    }

    #[test]
    fn live_feed_with_no_work_idles() {
        assert_eq!(
            plan_loop_step(
                LoopState::Continuing,
                SyncStatus::NoWorkReady,
                ShutdownStatus::Running
            ),
            LoopStep::Idle,
        );
    }
}
