// Core Contract:
// - Deterministic: same inputs => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Pure: no I/O, no clock, no allocation, no await

//! GREEN loop planner (PHASE4-N-F-D S1; forge step PHASE4-N-F-E S1).
//!
//! The pure lifecycle decision function for the `--mode node` relay run loop.
//! `plan_loop_step` selects each iteration's [`LoopStep`] from four closed,
//! orthogonal, lifecycle-level inputs — operator intent, momentary source
//! readiness, forge-slot scheduling, and structural feed liveness. It owns
//! **no authority**: it cannot decide ledger validity, chain selection,
//! leadership, forge eligibility, or evidence, and its closed output
//! vocabulary makes an authority decision unrepresentable as a planner result.
//!
//! This is the mechanical proof of the cluster's central line — RED performs
//! effects, GREEN plans iteration, BLUE authority stays behind the existing
//! closed seams (`bootstrap_initial_state`, `run_node_sync` -> `pump_block`,
//! and — when forge is wired in S2 — `forge_one_from_recovered`). The RED relay
//! loop is a thin composer over this function; it advances the durable tip only
//! through the existing sync seam, never here.
//!
//! **Forge stays subordinate (PHASE4-N-F-E).** The planner only learns whether
//! a forge slot is *due* via the content-blind [`ForgeSlotStatus`] — never who
//! is a leader. Leadership eligibility is decided BLUE inside
//! `forge_one_from_recovered` (reached only from the RED driver, S2). The
//! forge-slot *monotonicity* (at most once per `SlotNo`, never a past slot) is
//! the pure [`forge_slot_status`] guard below — the ONLY function here that
//! observes a `SlotNo`.
//!
//! Honest scope: S1 lands tested-but-unwired for the forge path — the existing
//! `run_relay_loop` caller passes [`ForgeSlotStatus::NotDue`] (forge off), so
//! the planner never returns [`LoopStep::ForgeTick`] there. S2 feeds it a real
//! forge-slot status.

use ade_types::SlotNo;

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

/// Forge-slot scheduling input (PHASE4-N-F-E S1). The closed, **content-blind**
/// projection of the forge-slot monotonic guard: a yes/no, never a slot / hash
/// / tip / verdict / leader status / KES validity / forge eligibility. Keeps
/// `plan_loop_step` unable to observe or encode an authority decision —
/// leadership stays BLUE inside `forge_one_from_recovered` (wired in S2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeSlotStatus {
    /// A forge slot is due now (the current slot strictly exceeds the last
    /// forged slot). The RED caller activates this only with producer material.
    Due,
    /// No forge slot is due now — already forged this slot, a past slot, or
    /// forge is inactive (no producer material). The forge-off path reduces
    /// here, collapsing the planner to its N-F-D relay behavior.
    NotDue,
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
/// authority decision (apply / admit / evidence). `ForgeTick` schedules a forge
/// *attempt*; the leadership decision lives BLUE behind the forge seam, never
/// here. Not `#[non_exhaustive]`: a new step is a compile error until wired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopStep {
    /// Run one `run_node_sync` step (drain the currently-available batch
    /// through the single `pump_block` seam, capturing its checkpoint).
    SyncOnce,
    /// Attempt one forge tick (PHASE4-N-F-E): a due forge slot on a live feed.
    /// The RED driver (S2) calls the recovered-surface forge handoff
    /// `forge_one_from_recovered` — leadership eligibility is decided there
    /// (BLUE), never by the planner. Self-accept-only: advances no durable tip.
    ForgeTick,
    /// No work right now and the feed is still live — wait (in the RED driver,
    /// a cancellation-safe select on source-readiness or shutdown).
    Idle,
    /// Halt the loop cleanly at this boundary, leaving on-disk state
    /// recoverable.
    HaltCleanly,
}

/// Select the next relay-loop step from the four closed lifecycle inputs.
///
/// Pure, total, and deterministic: same inputs => same [`LoopStep`], with no
/// I/O, clock, allocation, or `await`. Precedence
/// (shutdown -> sync -> terminal feed-end -> forge -> idle):
///
/// 1. a requested shutdown halts promptly at the next boundary (it does not
///    start new work);
/// 2. otherwise, available relay work drains first (even while the feed is
///    ending) — produce is subordinate to the sync spine;
/// 3. a drained, **ended** feed halts cleanly **even if a forge slot is due** —
///    the loop must not forge after the input feed is exhausted, so its
///    terminal behavior never depends on a producer branch N-F-D did not have;
/// 4. on an open (continuing) feed, a due forge slot fires a forge tick;
/// 5. otherwise an open feed with no work and nothing due idles.
///
/// Reduction property: with [`ForgeSlotStatus::NotDue`] the table collapses
/// **exactly** to the N-F-D 3-input relay mapping. Total over all 2x2x2x2
/// input combinations via an exhaustive `match` with no wildcard arm.
pub fn plan_loop_step(
    loop_state: LoopState,
    sync_status: SyncStatus,
    forge_slot_status: ForgeSlotStatus,
    shutdown_status: ShutdownStatus,
) -> LoopStep {
    match shutdown_status {
        ShutdownStatus::ShutdownRequested => LoopStep::HaltCleanly,
        ShutdownStatus::Running => match sync_status {
            SyncStatus::WorkAvailable => LoopStep::SyncOnce,
            SyncStatus::NoWorkReady => match loop_state {
                // Terminal feed-end halts cleanly even when a forge slot is due:
                // produce stays subordinate; the loop never forges past the feed.
                LoopState::Ending => LoopStep::HaltCleanly,
                LoopState::Continuing => match forge_slot_status {
                    ForgeSlotStatus::Due => LoopStep::ForgeTick,
                    ForgeSlotStatus::NotDue => LoopStep::Idle,
                },
            },
        },
    }
}

/// Pure forge-slot monotonic guard (PHASE4-N-F-E S1). Decides whether the
/// current slot is *due* a forge: `Due` iff it strictly exceeds the last forged
/// slot (or none has been forged yet). At most once per `SlotNo`, never a past
/// or already-forged slot. This is the ONLY function in the module that
/// observes a `SlotNo`; the resulting closed [`ForgeSlotStatus`] is what crosses
/// into `plan_loop_step`, keeping step selection content-blind. Producer-active
/// gating (return `NotDue` when no producer material) is the RED caller's job.
pub fn forge_slot_status(
    last_forged_slot: Option<SlotNo>,
    current_slot: SlotNo,
) -> ForgeSlotStatus {
    match last_forged_slot {
        None => ForgeSlotStatus::Due,
        Some(last) => {
            if current_slot.0 > last.0 {
                ForgeSlotStatus::Due
            } else {
                ForgeSlotStatus::NotDue
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every input combination, spelled out — the decision table is total.
    const STATES: [LoopState; 2] = [LoopState::Continuing, LoopState::Ending];
    const SYNCS: [SyncStatus; 2] = [SyncStatus::WorkAvailable, SyncStatus::NoWorkReady];
    const FORGES: [ForgeSlotStatus; 2] = [ForgeSlotStatus::Due, ForgeSlotStatus::NotDue];
    const SHUTDOWNS: [ShutdownStatus; 2] =
        [ShutdownStatus::Running, ShutdownStatus::ShutdownRequested];

    /// Expected output for every one of the 16 input combinations
    /// (precedence: shutdown -> sync -> terminal feed-end -> forge -> idle).
    fn expected(s: LoopState, y: SyncStatus, f: ForgeSlotStatus, d: ShutdownStatus) -> LoopStep {
        match d {
            ShutdownStatus::ShutdownRequested => LoopStep::HaltCleanly,
            ShutdownStatus::Running => match y {
                SyncStatus::WorkAvailable => LoopStep::SyncOnce,
                SyncStatus::NoWorkReady => match s {
                    LoopState::Ending => LoopStep::HaltCleanly,
                    LoopState::Continuing => match f {
                        ForgeSlotStatus::Due => LoopStep::ForgeTick,
                        ForgeSlotStatus::NotDue => LoopStep::Idle,
                    },
                },
            },
        }
    }

    /// The N-F-D 3-input relay mapping (no forge). The `NotDue` reduction must
    /// reproduce this exactly.
    fn relay_expected(s: LoopState, y: SyncStatus, d: ShutdownStatus) -> LoopStep {
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
    fn plan_loop_step_forge_precedence_table_is_total() {
        let mut count = 0;
        for &s in &STATES {
            for &y in &SYNCS {
                for &f in &FORGES {
                    for &d in &SHUTDOWNS {
                        assert_eq!(
                            plan_loop_step(s, y, f, d),
                            expected(s, y, f, d),
                            "case {s:?},{y:?},{f:?},{d:?}"
                        );
                        count += 1;
                    }
                }
            }
        }
        assert_eq!(count, 16, "all 2x2x2x2 input combinations must be covered");
    }

    #[test]
    fn plan_loop_step_reduces_to_relay_table_when_forge_notdue() {
        // With forge inactive (NotDue), the planner must reproduce the N-F-D
        // relay mapping exactly over the remaining 8 combinations.
        for &s in &STATES {
            for &y in &SYNCS {
                for &d in &SHUTDOWNS {
                    assert_eq!(
                        plan_loop_step(s, y, ForgeSlotStatus::NotDue, d),
                        relay_expected(s, y, d),
                        "NotDue reduction case {s:?},{y:?},{d:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn plan_loop_step_notdue_never_returns_forge_tick() {
        // S1 caller (run_relay_loop) always passes NotDue, so ForgeTick is
        // structurally unreachable there. Prove it across every combination.
        for &s in &STATES {
            for &y in &SYNCS {
                for &d in &SHUTDOWNS {
                    assert_ne!(
                        plan_loop_step(s, y, ForgeSlotStatus::NotDue, d),
                        LoopStep::ForgeTick,
                        "NotDue must never yield ForgeTick: {s:?},{y:?},{d:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn forge_suppressed_when_feed_ending() {
        // A due forge slot is suppressed once the feed has ended — the loop
        // halts cleanly rather than forge past an exhausted input feed.
        assert_eq!(
            plan_loop_step(
                LoopState::Ending,
                SyncStatus::NoWorkReady,
                ForgeSlotStatus::Due,
                ShutdownStatus::Running,
            ),
            LoopStep::HaltCleanly,
        );
        // ForgeTick fires only on a live (continuing) feed with a due slot.
        assert_eq!(
            plan_loop_step(
                LoopState::Continuing,
                SyncStatus::NoWorkReady,
                ForgeSlotStatus::Due,
                ShutdownStatus::Running,
            ),
            LoopStep::ForgeTick,
        );
    }

    #[test]
    fn plan_loop_step_is_deterministic() {
        for &s in &STATES {
            for &y in &SYNCS {
                for &f in &FORGES {
                    for &d in &SHUTDOWNS {
                        let a = plan_loop_step(s, y, f, d);
                        let b = plan_loop_step(s, y, f, d);
                        assert_eq!(a, b, "deterministic for {s:?},{y:?},{f:?},{d:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn shutdown_halts_even_with_work_available() {
        // Shutdown takes precedence over available work, a due forge, and a
        // live feed.
        assert_eq!(
            plan_loop_step(
                LoopState::Continuing,
                SyncStatus::WorkAvailable,
                ForgeSlotStatus::Due,
                ShutdownStatus::ShutdownRequested,
            ),
            LoopStep::HaltCleanly,
        );
    }

    #[test]
    fn available_work_drains_before_forge_and_ending() {
        // Work drains even while the feed is ending and a forge slot is due.
        assert_eq!(
            plan_loop_step(
                LoopState::Ending,
                SyncStatus::WorkAvailable,
                ForgeSlotStatus::Due,
                ShutdownStatus::Running,
            ),
            LoopStep::SyncOnce,
        );
    }

    #[test]
    fn live_feed_with_no_work_and_nothing_due_idles() {
        assert_eq!(
            plan_loop_step(
                LoopState::Continuing,
                SyncStatus::NoWorkReady,
                ForgeSlotStatus::NotDue,
                ShutdownStatus::Running,
            ),
            LoopStep::Idle,
        );
    }

    #[test]
    fn forge_slot_guard_none_is_due() {
        assert_eq!(forge_slot_status(None, SlotNo(0)), ForgeSlotStatus::Due);
        assert_eq!(forge_slot_status(None, SlotNo(42)), ForgeSlotStatus::Due);
    }

    #[test]
    fn forge_slot_guard_at_most_once_per_slot() {
        // Equal slot — already forged — is not due; a strictly greater slot is.
        assert_eq!(
            forge_slot_status(Some(SlotNo(100)), SlotNo(100)),
            ForgeSlotStatus::NotDue,
        );
        assert_eq!(
            forge_slot_status(Some(SlotNo(100)), SlotNo(101)),
            ForgeSlotStatus::Due,
        );
    }

    #[test]
    fn forge_slot_guard_rejects_past_slot() {
        assert_eq!(
            forge_slot_status(Some(SlotNo(100)), SlotNo(99)),
            ForgeSlotStatus::NotDue,
        );
    }
}
