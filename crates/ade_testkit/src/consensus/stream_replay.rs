// GREEN — corpus-driven replay driver for the chain-selector
// orchestrator. Consumed by `crates/ade_testkit/tests/
// consensus_stream_replay.rs` to close CE-N-B-5.
//
// The driver is a thin wrapper around `ade_runtime::consensus::
// chain_selector::process_stream_input`. Test-only; no authoritative
// decision lives here.

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::events::ChainEvent;
use ade_core::consensus::ledger_view::LedgerView;
use ade_runtime::consensus::chain_selector::{
    process_stream_input, OrchestratorError, OrchestratorState, StreamInput,
};

/// One step of the replay transcript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayStep {
    pub event: Option<ChainEvent>,
}

/// Replay a full ordered sequence of `StreamInput`s through the
/// orchestrator, returning the resulting final state and the list of
/// emitted events (with `None` filtered out).
///
/// Halts on the first orchestrator error and returns the error
/// together with the events emitted up to that point.
pub fn replay_stream(
    initial_state: OrchestratorState,
    inputs: &[StreamInput],
    ledger_view: &dyn LedgerView,
    era_schedule: &EraSchedule,
) -> ReplayResult {
    let mut state = initial_state;
    let mut events: Vec<ChainEvent> = Vec::new();
    let mut steps: Vec<ReplayStep> = Vec::new();
    for input in inputs {
        match process_stream_input(&mut state, input, ledger_view, era_schedule) {
            Ok(maybe_evt) => {
                if let Some(e) = maybe_evt.clone() {
                    events.push(e);
                }
                steps.push(ReplayStep { event: maybe_evt });
            }
            Err(err) => {
                return ReplayResult {
                    final_state: state,
                    events,
                    steps,
                    error: Some(err),
                };
            }
        }
    }
    ReplayResult {
        final_state: state,
        events,
        steps,
        error: None,
    }
}

/// The aggregate output of `replay_stream`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayResult {
    pub final_state: OrchestratorState,
    /// Events with `None` (epoch-boundary) filtered out — the
    /// CE-N-B-5 contract is that two runs produce identical event
    /// lists in this filtered shape.
    pub events: Vec<ChainEvent>,
    /// Per-step transcript (in lockstep with `inputs`), including
    /// `None`-emitting epoch-boundary steps.
    pub steps: Vec<ReplayStep>,
    /// First orchestrator error if any. Replay halts on first error.
    pub error: Option<OrchestratorError>,
}
