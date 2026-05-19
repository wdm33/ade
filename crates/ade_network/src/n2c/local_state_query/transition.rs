// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Pure LocalStateQuery transition function.
//
// Shape (per slice §9):
//   fn local_state_query_transition(state, agency, version, msg)
//       -> Result<(new_state, output), error>
//
// No async, no I/O, no ambient state. The selected version is an
// explicit input — never read from a session global (DC-PROTO-06).
//
// State graph (only these tuples are legal; everything else is
// IllegalTransition):
//
//   Idle      + Client + Acquire{point}    -> Acquiring + Event(AcquireRequested{point})
//   Idle      + Client + Done              -> Done      + Done
//   Acquiring + Server + Acquired          -> Acquired  + Event(SnapshotAcquired)
//   Acquiring + Server + Failure(reason)   -> Idle      + Event(AcquireFailed{reason})
//   Acquired  + Client + Query(payload)    -> Querying  + Event(QueryRequested{payload})
//   Querying  + Server + Result(payload)   -> Acquired  + Event(QueryReplied{payload})
//   Acquired  + Client + Release           -> Idle      + Event(SnapshotReleased)
//   Acquired  + Client + ReAcquire{point}  -> Acquiring + Event(ReAcquireRequested{point})
//   Acquired  + Client + Done              -> Done      + Done

use crate::codec::local_state_query::LocalStateQueryMessage;
use crate::codec::version::LocalStateQueryVersion;
use crate::n2c::local_state_query::agency::LocalStateQueryAgency;
use crate::n2c::local_state_query::event::LocalStateQueryEvent;
use crate::n2c::local_state_query::state::{
    LocalStateQueryError, LocalStateQueryOutput, LocalStateQueryState,
};

/// Highest LocalStateQuery mini-protocol version this state machine
/// accepts.
///
/// LocalStateQuery has shipped a single closed grammar (8 messages,
/// no version-gated variants) for every cardano-node 11.0.1 (10.6.2 forward-compatible) supported
/// version. We pin the upper bound at `MAX_LOCAL_STATE_QUERY_VERSION`
/// so a future spec extension cannot silently transit messages whose
/// semantics this state machine has not been updated for.
const MAX_LOCAL_STATE_QUERY_VERSION: u16 = 100;

/// Pure LocalStateQuery transition.
///
/// `agency` is the agency the *sender* held when it produced `msg`.
pub fn local_state_query_transition(
    state: LocalStateQueryState,
    agency: LocalStateQueryAgency,
    version: LocalStateQueryVersion,
    msg: LocalStateQueryMessage,
) -> Result<(LocalStateQueryState, LocalStateQueryOutput), LocalStateQueryError> {
    if version.get() > MAX_LOCAL_STATE_QUERY_VERSION {
        return Err(LocalStateQueryError::InvalidForVersion {
            version,
            message_tag: message_tag(&msg),
        });
    }
    match (state, agency, msg) {
        (
            LocalStateQueryState::Idle,
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Acquire { point },
        ) => Ok((
            LocalStateQueryState::Acquiring,
            LocalStateQueryOutput::Event(LocalStateQueryEvent::AcquireRequested { point }),
        )),
        (
            LocalStateQueryState::Idle,
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Done,
        ) => Ok((LocalStateQueryState::Done, LocalStateQueryOutput::Done)),
        (
            LocalStateQueryState::Acquiring,
            LocalStateQueryAgency::Server,
            LocalStateQueryMessage::Acquired,
        ) => Ok((
            LocalStateQueryState::Acquired,
            LocalStateQueryOutput::Event(LocalStateQueryEvent::SnapshotAcquired),
        )),
        (
            LocalStateQueryState::Acquiring,
            LocalStateQueryAgency::Server,
            LocalStateQueryMessage::Failure(reason),
        ) => Ok((
            LocalStateQueryState::Idle,
            LocalStateQueryOutput::Event(LocalStateQueryEvent::AcquireFailed { reason }),
        )),
        (
            LocalStateQueryState::Acquired,
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Query(payload),
        ) => Ok((
            LocalStateQueryState::Querying,
            LocalStateQueryOutput::Event(LocalStateQueryEvent::QueryRequested { payload }),
        )),
        (
            LocalStateQueryState::Querying,
            LocalStateQueryAgency::Server,
            LocalStateQueryMessage::Result(payload),
        ) => Ok((
            LocalStateQueryState::Acquired,
            LocalStateQueryOutput::Event(LocalStateQueryEvent::QueryReplied { payload }),
        )),
        (
            LocalStateQueryState::Acquired,
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Release,
        ) => Ok((
            LocalStateQueryState::Idle,
            LocalStateQueryOutput::Event(LocalStateQueryEvent::SnapshotReleased),
        )),
        (
            LocalStateQueryState::Acquired,
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::ReAcquire { point },
        ) => Ok((
            LocalStateQueryState::Acquiring,
            LocalStateQueryOutput::Event(LocalStateQueryEvent::ReAcquireRequested { point }),
        )),
        (
            LocalStateQueryState::Acquired,
            LocalStateQueryAgency::Client,
            LocalStateQueryMessage::Done,
        ) => Ok((LocalStateQueryState::Done, LocalStateQueryOutput::Done)),
        (state, agency, msg) => Err(LocalStateQueryError::IllegalTransition {
            state,
            message_tag: message_tag(&msg),
            agency,
        }),
    }
}

fn message_tag(msg: &LocalStateQueryMessage) -> &'static str {
    match msg {
        LocalStateQueryMessage::Acquire { .. } => "Acquire",
        LocalStateQueryMessage::Acquired => "Acquired",
        LocalStateQueryMessage::Failure(_) => "Failure",
        LocalStateQueryMessage::Query(_) => "Query",
        LocalStateQueryMessage::Result(_) => "Result",
        LocalStateQueryMessage::Release => "Release",
        LocalStateQueryMessage::ReAcquire { .. } => "ReAcquire",
        LocalStateQueryMessage::Done => "Done",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::local_state_query::{AcquireFailure, Point, QueryPayload, ResultPayload};
    use ade_types::{Hash32, SlotNo};

    fn version() -> LocalStateQueryVersion {
        LocalStateQueryVersion::new(16)
    }

    fn sample_point() -> Point {
        Point::Block {
            slot: SlotNo(4321),
            hash: Hash32([0xAB; 32]),
        }
    }

    #[test]
    fn acquire_then_acquired_transitions_to_acquired() {
        let point_in = Some(sample_point());
        let (st1, out1) = local_state_query_transition(
            LocalStateQueryState::Idle,
            LocalStateQueryAgency::Client,
            version(),
            LocalStateQueryMessage::Acquire {
                point: point_in.clone(),
            },
        )
        .expect("idle+acquire");
        assert_eq!(st1, LocalStateQueryState::Acquiring);
        match out1 {
            LocalStateQueryOutput::Event(LocalStateQueryEvent::AcquireRequested { point }) => {
                assert_eq!(point, point_in);
            }
            other => panic!("expected Event(AcquireRequested), got {other:?}"),
        }

        let (st2, out2) = local_state_query_transition(
            st1,
            LocalStateQueryAgency::Server,
            version(),
            LocalStateQueryMessage::Acquired,
        )
        .expect("acquiring+acquired");
        assert_eq!(st2, LocalStateQueryState::Acquired);
        match out2 {
            LocalStateQueryOutput::Event(LocalStateQueryEvent::SnapshotAcquired) => {}
            other => panic!("expected Event(SnapshotAcquired), got {other:?}"),
        }
    }

    #[test]
    fn acquire_then_failure_returns_to_idle() {
        let (st1, _) = local_state_query_transition(
            LocalStateQueryState::Idle,
            LocalStateQueryAgency::Client,
            version(),
            LocalStateQueryMessage::Acquire { point: None },
        )
        .expect("idle+acquire");
        assert_eq!(st1, LocalStateQueryState::Acquiring);

        let (st2, out2) = local_state_query_transition(
            st1,
            LocalStateQueryAgency::Server,
            version(),
            LocalStateQueryMessage::Failure(AcquireFailure::PointTooOld),
        )
        .expect("acquiring+failure");
        assert_eq!(st2, LocalStateQueryState::Idle);
        match out2 {
            LocalStateQueryOutput::Event(LocalStateQueryEvent::AcquireFailed { reason }) => {
                assert_eq!(reason, AcquireFailure::PointTooOld);
            }
            other => panic!("expected Event(AcquireFailed), got {other:?}"),
        }
    }

    #[test]
    fn query_then_result_round_trips() {
        let query_bytes = vec![0x01, 0x02, 0x03, 0xCA, 0xFE];
        let result_bytes = vec![0xBA, 0xBE, 0xDE, 0xAD];
        let (st1, out1) = local_state_query_transition(
            LocalStateQueryState::Acquired,
            LocalStateQueryAgency::Client,
            version(),
            LocalStateQueryMessage::Query(QueryPayload(query_bytes.clone())),
        )
        .expect("acquired+query");
        assert_eq!(st1, LocalStateQueryState::Querying);
        match out1 {
            LocalStateQueryOutput::Event(LocalStateQueryEvent::QueryRequested { payload }) => {
                assert_eq!(payload, QueryPayload(query_bytes));
            }
            other => panic!("expected Event(QueryRequested), got {other:?}"),
        }

        let (st2, out2) = local_state_query_transition(
            st1,
            LocalStateQueryAgency::Server,
            version(),
            LocalStateQueryMessage::Result(ResultPayload(result_bytes.clone())),
        )
        .expect("querying+result");
        assert_eq!(st2, LocalStateQueryState::Acquired);
        match out2 {
            LocalStateQueryOutput::Event(LocalStateQueryEvent::QueryReplied { payload }) => {
                assert_eq!(payload, ResultPayload(result_bytes));
            }
            other => panic!("expected Event(QueryReplied), got {other:?}"),
        }
    }

    #[test]
    fn release_returns_to_idle() {
        let (st, out) = local_state_query_transition(
            LocalStateQueryState::Acquired,
            LocalStateQueryAgency::Client,
            version(),
            LocalStateQueryMessage::Release,
        )
        .expect("acquired+release");
        assert_eq!(st, LocalStateQueryState::Idle);
        match out {
            LocalStateQueryOutput::Event(LocalStateQueryEvent::SnapshotReleased) => {}
            other => panic!("expected Event(SnapshotReleased), got {other:?}"),
        }
    }

    #[test]
    fn re_acquire_from_acquired_transitions_to_acquiring() {
        let point_in = Some(sample_point());
        let (st, out) = local_state_query_transition(
            LocalStateQueryState::Acquired,
            LocalStateQueryAgency::Client,
            version(),
            LocalStateQueryMessage::ReAcquire {
                point: point_in.clone(),
            },
        )
        .expect("acquired+re_acquire");
        assert_eq!(st, LocalStateQueryState::Acquiring);
        match out {
            LocalStateQueryOutput::Event(LocalStateQueryEvent::ReAcquireRequested { point }) => {
                assert_eq!(point, point_in);
            }
            other => panic!("expected Event(ReAcquireRequested), got {other:?}"),
        }
    }

    #[test]
    fn client_done_from_idle_terminates() {
        let (st, out) = local_state_query_transition(
            LocalStateQueryState::Idle,
            LocalStateQueryAgency::Client,
            version(),
            LocalStateQueryMessage::Done,
        )
        .expect("idle+done");
        assert_eq!(st, LocalStateQueryState::Done);
        match out {
            LocalStateQueryOutput::Done => {}
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn client_done_from_acquired_terminates() {
        let (st, out) = local_state_query_transition(
            LocalStateQueryState::Acquired,
            LocalStateQueryAgency::Client,
            version(),
            LocalStateQueryMessage::Done,
        )
        .expect("acquired+done");
        assert_eq!(st, LocalStateQueryState::Done);
        match out {
            LocalStateQueryOutput::Done => {}
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn wrong_agency_returns_error() {
        // Acquire is a client-originated message; passing Server
        // agency for it is grammar-illegal.
        let err = local_state_query_transition(
            LocalStateQueryState::Idle,
            LocalStateQueryAgency::Server,
            version(),
            LocalStateQueryMessage::Acquire { point: None },
        )
        .expect_err("must reject");
        match err {
            LocalStateQueryError::IllegalTransition {
                message_tag,
                agency,
                ..
            } => {
                assert_eq!(message_tag, "Acquire");
                assert_eq!(agency, LocalStateQueryAgency::Server);
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }
}
