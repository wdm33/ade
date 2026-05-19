// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Pure LocalTxMonitor transition function.
//
// Shape (per slice §9):
//   fn local_tx_monitor_transition(state, agency, version, msg)
//       -> Result<(new_state, output), error>
//
// No async, no I/O, no ambient state. The selected version is an
// explicit input — never read from a session global (DC-PROTO-06).
//
// State graph (only these tuples are legal; everything else is
// IllegalTransition):
//
//   Idle      + Client + Acquire             -> Acquiring + Event(AcquireRequested)
//   Idle      + Client + Done                -> Done      + Done
//   Acquiring + Server + AwaitAcquire        -> Acquiring + Event(AwaitingAcquisition)
//   Acquiring + Server + Acquired{slot}      -> Acquired  + Event(MempoolAcquired{slot})
//   Acquired  + Client + Query(payload)      -> Querying  + Event(QueryRequested{payload})
//   Querying  + Server + Reply(payload)      -> Acquired  + Event(QueryReplied{payload})
//   Acquired  + Client + Release             -> Idle      + Event(MempoolReleased)
//   Acquired  + Client + Done                -> Done      + Done

use crate::codec::local_tx_monitor::LocalTxMonitorMessage;
use crate::codec::version::LocalTxMonitorVersion;
use crate::n2c::local_tx_monitor::agency::LocalTxMonitorAgency;
use crate::n2c::local_tx_monitor::event::LocalTxMonitorEvent;
use crate::n2c::local_tx_monitor::state::{
    LocalTxMonitorError, LocalTxMonitorOutput, LocalTxMonitorState,
};

/// Highest LocalTxMonitor mini-protocol version this state machine
/// accepts.
///
/// LocalTxMonitor has shipped a single closed grammar (7 messages,
/// no version-gated variants) for every cardano-node 10.6.2 supported
/// version. We pin the upper bound at `MAX_LOCAL_TX_MONITOR_VERSION`
/// so a future spec extension cannot silently transit messages whose
/// semantics this state machine has not been updated for.
const MAX_LOCAL_TX_MONITOR_VERSION: u16 = 100;

/// Pure LocalTxMonitor transition.
///
/// `agency` is the agency the *sender* held when it produced `msg`.
pub fn local_tx_monitor_transition(
    state: LocalTxMonitorState,
    agency: LocalTxMonitorAgency,
    version: LocalTxMonitorVersion,
    msg: LocalTxMonitorMessage,
) -> Result<(LocalTxMonitorState, LocalTxMonitorOutput), LocalTxMonitorError> {
    if version.get() > MAX_LOCAL_TX_MONITOR_VERSION {
        return Err(LocalTxMonitorError::InvalidForVersion {
            version,
            message_tag: message_tag(&msg),
        });
    }
    match (state, agency, msg) {
        (
            LocalTxMonitorState::Idle,
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Acquire,
        ) => Ok((
            LocalTxMonitorState::Acquiring,
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::AcquireRequested),
        )),
        (LocalTxMonitorState::Idle, LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Done) => {
            Ok((LocalTxMonitorState::Done, LocalTxMonitorOutput::Done))
        }
        (
            LocalTxMonitorState::Acquiring,
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::AwaitAcquire,
        ) => Ok((
            LocalTxMonitorState::Acquiring,
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::AwaitingAcquisition),
        )),
        (
            LocalTxMonitorState::Acquiring,
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot },
        ) => Ok((
            LocalTxMonitorState::Acquired,
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::MempoolAcquired { slot }),
        )),
        (
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Query(payload),
        ) => Ok((
            LocalTxMonitorState::Querying,
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::QueryRequested { payload }),
        )),
        (
            LocalTxMonitorState::Querying,
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Reply(payload),
        ) => Ok((
            LocalTxMonitorState::Acquired,
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::QueryReplied { payload }),
        )),
        (
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Release,
        ) => Ok((
            LocalTxMonitorState::Idle,
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::MempoolReleased),
        )),
        (
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Done,
        ) => Ok((LocalTxMonitorState::Done, LocalTxMonitorOutput::Done)),
        (state, agency, msg) => Err(LocalTxMonitorError::IllegalTransition {
            state,
            message_tag: message_tag(&msg),
            agency,
        }),
    }
}

fn message_tag(msg: &LocalTxMonitorMessage) -> &'static str {
    match msg {
        LocalTxMonitorMessage::Done => "Done",
        LocalTxMonitorMessage::Acquire => "Acquire",
        LocalTxMonitorMessage::Acquired { .. } => "Acquired",
        LocalTxMonitorMessage::AwaitAcquire => "AwaitAcquire",
        LocalTxMonitorMessage::Release => "Release",
        LocalTxMonitorMessage::Query(_) => "Query",
        LocalTxMonitorMessage::Reply(_) => "Reply",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::local_tx_monitor::{LocalTxMonitorQuery, LocalTxMonitorReply};
    use ade_types::SlotNo;

    fn version() -> LocalTxMonitorVersion {
        LocalTxMonitorVersion::new(16)
    }

    #[test]
    fn acquire_then_acquired_with_slot() {
        let (st1, out1) = local_tx_monitor_transition(
            LocalTxMonitorState::Idle,
            LocalTxMonitorAgency::Client,
            version(),
            LocalTxMonitorMessage::Acquire,
        )
        .expect("idle+acquire");
        assert_eq!(st1, LocalTxMonitorState::Acquiring);
        match out1 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::AcquireRequested) => {}
            other => panic!("expected Event(AcquireRequested), got {other:?}"),
        }

        let slot_in = SlotNo(987654);
        let (st2, out2) = local_tx_monitor_transition(
            st1,
            LocalTxMonitorAgency::Server,
            version(),
            LocalTxMonitorMessage::Acquired { slot: slot_in },
        )
        .expect("acquiring+acquired");
        assert_eq!(st2, LocalTxMonitorState::Acquired);
        match out2 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::MempoolAcquired { slot }) => {
                assert_eq!(slot, slot_in);
            }
            other => panic!("expected Event(MempoolAcquired), got {other:?}"),
        }
    }

    #[test]
    fn acquire_then_await_then_acquired() {
        let (st1, _) = local_tx_monitor_transition(
            LocalTxMonitorState::Idle,
            LocalTxMonitorAgency::Client,
            version(),
            LocalTxMonitorMessage::Acquire,
        )
        .expect("idle+acquire");
        assert_eq!(st1, LocalTxMonitorState::Acquiring);

        let (st2, out2) = local_tx_monitor_transition(
            st1,
            LocalTxMonitorAgency::Server,
            version(),
            LocalTxMonitorMessage::AwaitAcquire,
        )
        .expect("acquiring+await_acquire");
        assert_eq!(st2, LocalTxMonitorState::Acquiring);
        match out2 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::AwaitingAcquisition) => {}
            other => panic!("expected Event(AwaitingAcquisition), got {other:?}"),
        }

        let slot_in = SlotNo(42);
        let (st3, out3) = local_tx_monitor_transition(
            st2,
            LocalTxMonitorAgency::Server,
            version(),
            LocalTxMonitorMessage::Acquired { slot: slot_in },
        )
        .expect("acquiring+acquired");
        assert_eq!(st3, LocalTxMonitorState::Acquired);
        match out3 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::MempoolAcquired { slot }) => {
                assert_eq!(slot, slot_in);
            }
            other => panic!("expected Event(MempoolAcquired), got {other:?}"),
        }
    }

    #[test]
    fn query_then_reply_round_trips() {
        let query_bytes = vec![0x01, 0x02, 0x03];
        let reply_bytes = vec![0xCA, 0xFE, 0xBA, 0xBE];
        let (st1, out1) = local_tx_monitor_transition(
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            version(),
            LocalTxMonitorMessage::Query(LocalTxMonitorQuery(query_bytes.clone())),
        )
        .expect("acquired+query");
        assert_eq!(st1, LocalTxMonitorState::Querying);
        match out1 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::QueryRequested { payload }) => {
                assert_eq!(payload, LocalTxMonitorQuery(query_bytes));
            }
            other => panic!("expected Event(QueryRequested), got {other:?}"),
        }

        let (st2, out2) = local_tx_monitor_transition(
            st1,
            LocalTxMonitorAgency::Server,
            version(),
            LocalTxMonitorMessage::Reply(LocalTxMonitorReply(reply_bytes.clone())),
        )
        .expect("querying+reply");
        assert_eq!(st2, LocalTxMonitorState::Acquired);
        match out2 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::QueryReplied { payload }) => {
                assert_eq!(payload, LocalTxMonitorReply(reply_bytes));
            }
            other => panic!("expected Event(QueryReplied), got {other:?}"),
        }
    }

    #[test]
    fn release_returns_to_idle() {
        let (st, out) = local_tx_monitor_transition(
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            version(),
            LocalTxMonitorMessage::Release,
        )
        .expect("acquired+release");
        assert_eq!(st, LocalTxMonitorState::Idle);
        match out {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::MempoolReleased) => {}
            other => panic!("expected Event(MempoolReleased), got {other:?}"),
        }
    }

    #[test]
    fn client_done_from_idle_terminates() {
        let (st, out) = local_tx_monitor_transition(
            LocalTxMonitorState::Idle,
            LocalTxMonitorAgency::Client,
            version(),
            LocalTxMonitorMessage::Done,
        )
        .expect("idle+done");
        assert_eq!(st, LocalTxMonitorState::Done);
        match out {
            LocalTxMonitorOutput::Done => {}
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn wrong_agency_returns_error() {
        // Acquire is a client-originated message; passing Server
        // agency for it is grammar-illegal.
        let err = local_tx_monitor_transition(
            LocalTxMonitorState::Idle,
            LocalTxMonitorAgency::Server,
            version(),
            LocalTxMonitorMessage::Acquire,
        )
        .expect_err("must reject");
        match err {
            LocalTxMonitorError::IllegalTransition {
                message_tag,
                agency,
                ..
            } => {
                assert_eq!(message_tag, "Acquire");
                assert_eq!(agency, LocalTxMonitorAgency::Server);
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn version_gating() {
        let bogus_version = LocalTxMonitorVersion::new(MAX_LOCAL_TX_MONITOR_VERSION + 1);
        let err = local_tx_monitor_transition(
            LocalTxMonitorState::Idle,
            LocalTxMonitorAgency::Client,
            bogus_version,
            LocalTxMonitorMessage::Acquire,
        )
        .expect_err("must reject");
        match err {
            LocalTxMonitorError::InvalidForVersion {
                version,
                message_tag,
            } => {
                assert_eq!(version, bogus_version);
                assert_eq!(message_tag, "Acquire");
            }
            other => panic!("expected InvalidForVersion, got {other:?}"),
        }
    }
}
