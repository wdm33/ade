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
//   (Idle, Client, Done)                          -> (Done, Done)
//   (Idle, Client, Acquire)                       -> (Acquiring, Event(AcquireRequested))
//   (Acquiring, Server, Acquired{slot})           -> (Acquired,  Event(MempoolAcquired{slot}))
//   (Acquired, Client, Acquire)                   -> (Acquiring, Event(ReAcquireRequested))    ; MsgAwaitAcquire on wire
//   (Acquired, Client, Release)                   -> (Idle,      Event(MempoolReleased))
//   (Acquired, Client, NextTx)                    -> (Busy{NextTx},      Event(NextTxRequested))
//   (Acquired, Client, HasTx{tx_id})              -> (Busy{HasTx},       Event(HasTxRequested{tx_id}))
//   (Acquired, Client, GetSizes)                  -> (Busy{GetSizes},    Event(SizesRequested))
//   (Acquired, Client, GetMeasures)               -> (Busy{GetMeasures}, Event(MeasuresRequested)) [v >= 2]
//   (Busy{NextTx},   Server, ReplyNextTx)         -> (Acquired,  Event(NextTxReplied{tx_bytes}))
//   (Busy{HasTx},    Server, ReplyHasTx)          -> (Acquired,  Event(HasTxReplied{present}))
//   (Busy{GetSizes}, Server, ReplyGetSizes)       -> (Acquired,  Event(SizesReplied(sizes)))
//   (Busy{GetMeasures}, Server, ReplyGetMeasures) -> (Acquired,  Event(MeasuresReplied(measures))) [v >= 2]

use crate::codec::local_tx_monitor::LocalTxMonitorMessage;
use crate::codec::version::LocalTxMonitorVersion;
use crate::n2c::local_tx_monitor::agency::LocalTxMonitorAgency;
use crate::n2c::local_tx_monitor::event::LocalTxMonitorEvent;
use crate::n2c::local_tx_monitor::state::{
    BusyKind, LocalTxMonitorError, LocalTxMonitorOutput, LocalTxMonitorState,
};

/// Highest LocalTxMonitor mini-protocol version this state machine
/// accepts. Cardano-node 11.0.1 currently tops out at V2 (the
/// Measures variant); we pin a generous upper bound so a future spec
/// extension cannot silently transit messages this state machine has
/// not been updated for.
const MAX_LOCAL_TX_MONITOR_VERSION: u16 = 100;

/// Lowest LocalTxMonitor version that allows the `GetMeasures` /
/// `ReplyGetMeasures` message pair (LocalTxMonitor_V2).
const MIN_MEASURES_VERSION: u16 = 2;

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
        // Idle
        (LocalTxMonitorState::Idle, LocalTxMonitorAgency::Client, LocalTxMonitorMessage::Done) => {
            Ok((LocalTxMonitorState::Done, LocalTxMonitorOutput::Done))
        }
        (
            LocalTxMonitorState::Idle,
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Acquire,
        ) => Ok((
            LocalTxMonitorState::Acquiring,
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::AcquireRequested),
        )),
        // Acquiring
        (
            LocalTxMonitorState::Acquiring,
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::Acquired { slot },
        ) => Ok((
            LocalTxMonitorState::Acquired,
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::MempoolAcquired { slot }),
        )),
        // Acquired — client-agency control flow
        //
        // `Acquire` from `Acquired` is the wire-level `MsgAwaitAcquire`
        // (same tag-1 encoding); the codec emits `Acquire` and the
        // state machine reinterprets here.
        (
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Acquire,
        ) => Ok((
            LocalTxMonitorState::Acquiring,
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::ReAcquireRequested),
        )),
        (
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::Release,
        ) => Ok((
            LocalTxMonitorState::Idle,
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::MempoolReleased),
        )),
        // Acquired — client-agency query requests
        (
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::NextTx,
        ) => Ok((
            LocalTxMonitorState::Busy { kind: BusyKind::NextTx },
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::NextTxRequested),
        )),
        (
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::HasTx { tx_id },
        ) => Ok((
            LocalTxMonitorState::Busy { kind: BusyKind::HasTx },
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::HasTxRequested { tx_id }),
        )),
        (
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::GetSizes,
        ) => Ok((
            LocalTxMonitorState::Busy { kind: BusyKind::GetSizes },
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::SizesRequested),
        )),
        (
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            LocalTxMonitorMessage::GetMeasures,
        ) => {
            if version.get() < MIN_MEASURES_VERSION {
                return Err(LocalTxMonitorError::InvalidForVersion {
                    version,
                    message_tag: "GetMeasures",
                });
            }
            Ok((
                LocalTxMonitorState::Busy {
                    kind: BusyKind::GetMeasures,
                },
                LocalTxMonitorOutput::Event(LocalTxMonitorEvent::MeasuresRequested),
            ))
        }
        // Busy — server-agency reply messages.
        (
            LocalTxMonitorState::Busy { kind: BusyKind::NextTx },
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyNextTx { tx_bytes },
        ) => Ok((
            LocalTxMonitorState::Acquired,
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::NextTxReplied { tx_bytes }),
        )),
        (
            LocalTxMonitorState::Busy { kind: BusyKind::HasTx },
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyHasTx { present },
        ) => Ok((
            LocalTxMonitorState::Acquired,
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::HasTxReplied { present }),
        )),
        (
            LocalTxMonitorState::Busy { kind: BusyKind::GetSizes },
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyGetSizes(sizes),
        ) => Ok((
            LocalTxMonitorState::Acquired,
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::SizesReplied(sizes)),
        )),
        (
            LocalTxMonitorState::Busy { kind: BusyKind::GetMeasures },
            LocalTxMonitorAgency::Server,
            LocalTxMonitorMessage::ReplyGetMeasures(measures),
        ) => {
            if version.get() < MIN_MEASURES_VERSION {
                return Err(LocalTxMonitorError::InvalidForVersion {
                    version,
                    message_tag: "ReplyGetMeasures",
                });
            }
            Ok((
                LocalTxMonitorState::Acquired,
                LocalTxMonitorOutput::Event(LocalTxMonitorEvent::MeasuresReplied(measures)),
            ))
        }
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
        LocalTxMonitorMessage::Release => "Release",
        LocalTxMonitorMessage::NextTx => "NextTx",
        LocalTxMonitorMessage::ReplyNextTx { .. } => "ReplyNextTx",
        LocalTxMonitorMessage::HasTx { .. } => "HasTx",
        LocalTxMonitorMessage::ReplyHasTx { .. } => "ReplyHasTx",
        LocalTxMonitorMessage::GetSizes => "GetSizes",
        LocalTxMonitorMessage::ReplyGetSizes(_) => "ReplyGetSizes",
        LocalTxMonitorMessage::GetMeasures => "GetMeasures",
        LocalTxMonitorMessage::ReplyGetMeasures(_) => "ReplyGetMeasures",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::local_tx_monitor::{
        MeasureName, MeasureSizeAndCapacity, MempoolMeasures, MempoolSizeAndCapacity,
    };
    use ade_types::{Hash32, SlotNo, TxId};
    use std::collections::BTreeMap;

    fn v(ver: u16) -> LocalTxMonitorVersion {
        LocalTxMonitorVersion::new(ver)
    }

    fn sample_tx_id() -> TxId {
        TxId(Hash32([0x42; 32]))
    }

    fn sample_sizes() -> MempoolSizeAndCapacity {
        MempoolSizeAndCapacity {
            capacity_bytes: 1_048_576,
            size_bytes: 4096,
            tx_count: 17,
        }
    }

    fn sample_measures() -> MempoolMeasures {
        let mut measures = BTreeMap::new();
        measures.insert(
            MeasureName::new("bytes"),
            MeasureSizeAndCapacity {
                size: 1024,
                capacity: 65536,
            },
        );
        MempoolMeasures {
            tx_count: 3,
            measures,
        }
    }

    #[test]
    fn idle_acquire_then_acquired() {
        let (st1, out1) = local_tx_monitor_transition(
            LocalTxMonitorState::Idle,
            LocalTxMonitorAgency::Client,
            v(2),
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
            v(2),
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
    fn acquired_release_returns_to_idle() {
        let (st, out) = local_tx_monitor_transition(
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            v(2),
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
    fn acquired_await_acquire_re_acquires() {
        let (st, out) = local_tx_monitor_transition(
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            v(2),
            LocalTxMonitorMessage::Acquire,
        )
        .expect("acquired+acquire (await_acquire)");
        assert_eq!(st, LocalTxMonitorState::Acquiring);
        match out {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::ReAcquireRequested) => {}
            other => panic!("expected Event(ReAcquireRequested), got {other:?}"),
        }
    }

    #[test]
    fn acquired_next_tx_then_reply_with_tx() {
        let (st1, out1) = local_tx_monitor_transition(
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            v(2),
            LocalTxMonitorMessage::NextTx,
        )
        .expect("acquired+next_tx");
        assert_eq!(
            st1,
            LocalTxMonitorState::Busy { kind: BusyKind::NextTx }
        );
        match out1 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::NextTxRequested) => {}
            other => panic!("expected Event(NextTxRequested), got {other:?}"),
        }

        let tx = vec![0xCA, 0xFE, 0xBA, 0xBE];
        let (st2, out2) = local_tx_monitor_transition(
            st1,
            LocalTxMonitorAgency::Server,
            v(2),
            LocalTxMonitorMessage::ReplyNextTx {
                tx_bytes: Some(tx.clone()),
            },
        )
        .expect("busy_next_tx+reply");
        assert_eq!(st2, LocalTxMonitorState::Acquired);
        match out2 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::NextTxReplied { tx_bytes }) => {
                assert_eq!(tx_bytes, Some(tx));
            }
            other => panic!("expected Event(NextTxReplied), got {other:?}"),
        }
    }

    #[test]
    fn acquired_next_tx_then_reply_empty_mempool() {
        let (st1, _) = local_tx_monitor_transition(
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            v(2),
            LocalTxMonitorMessage::NextTx,
        )
        .expect("acquired+next_tx");

        let (st2, out2) = local_tx_monitor_transition(
            st1,
            LocalTxMonitorAgency::Server,
            v(2),
            LocalTxMonitorMessage::ReplyNextTx { tx_bytes: None },
        )
        .expect("busy_next_tx+reply(empty)");
        assert_eq!(st2, LocalTxMonitorState::Acquired);
        match out2 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::NextTxReplied { tx_bytes: None }) => {}
            other => panic!("expected Event(NextTxReplied None), got {other:?}"),
        }
    }

    #[test]
    fn acquired_has_tx_then_reply_present() {
        let id = sample_tx_id();
        let (st1, out1) = local_tx_monitor_transition(
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            v(2),
            LocalTxMonitorMessage::HasTx { tx_id: id.clone() },
        )
        .expect("acquired+has_tx");
        assert_eq!(
            st1,
            LocalTxMonitorState::Busy { kind: BusyKind::HasTx }
        );
        match out1 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::HasTxRequested { tx_id }) => {
                assert_eq!(tx_id, id);
            }
            other => panic!("expected Event(HasTxRequested), got {other:?}"),
        }

        let (st2, out2) = local_tx_monitor_transition(
            st1,
            LocalTxMonitorAgency::Server,
            v(2),
            LocalTxMonitorMessage::ReplyHasTx { present: true },
        )
        .expect("busy_has_tx+reply(true)");
        assert_eq!(st2, LocalTxMonitorState::Acquired);
        match out2 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::HasTxReplied { present: true }) => {}
            other => panic!("expected Event(HasTxReplied true), got {other:?}"),
        }
    }

    #[test]
    fn acquired_has_tx_then_reply_absent() {
        let (st1, _) = local_tx_monitor_transition(
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            v(2),
            LocalTxMonitorMessage::HasTx { tx_id: sample_tx_id() },
        )
        .expect("acquired+has_tx");

        let (st2, out2) = local_tx_monitor_transition(
            st1,
            LocalTxMonitorAgency::Server,
            v(2),
            LocalTxMonitorMessage::ReplyHasTx { present: false },
        )
        .expect("busy_has_tx+reply(false)");
        assert_eq!(st2, LocalTxMonitorState::Acquired);
        match out2 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::HasTxReplied { present: false }) => {}
            other => panic!("expected Event(HasTxReplied false), got {other:?}"),
        }
    }

    #[test]
    fn acquired_get_sizes_then_reply() {
        let (st1, out1) = local_tx_monitor_transition(
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            v(2),
            LocalTxMonitorMessage::GetSizes,
        )
        .expect("acquired+get_sizes");
        assert_eq!(
            st1,
            LocalTxMonitorState::Busy { kind: BusyKind::GetSizes }
        );
        match out1 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::SizesRequested) => {}
            other => panic!("expected Event(SizesRequested), got {other:?}"),
        }

        let sizes_in = sample_sizes();
        let (st2, out2) = local_tx_monitor_transition(
            st1,
            LocalTxMonitorAgency::Server,
            v(2),
            LocalTxMonitorMessage::ReplyGetSizes(sizes_in),
        )
        .expect("busy_get_sizes+reply");
        assert_eq!(st2, LocalTxMonitorState::Acquired);
        match out2 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::SizesReplied(sizes)) => {
                assert_eq!(sizes, sizes_in);
            }
            other => panic!("expected Event(SizesReplied), got {other:?}"),
        }
    }

    #[test]
    fn acquired_get_measures_then_reply() {
        let (st1, out1) = local_tx_monitor_transition(
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            v(2),
            LocalTxMonitorMessage::GetMeasures,
        )
        .expect("acquired+get_measures");
        assert_eq!(
            st1,
            LocalTxMonitorState::Busy {
                kind: BusyKind::GetMeasures
            }
        );
        match out1 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::MeasuresRequested) => {}
            other => panic!("expected Event(MeasuresRequested), got {other:?}"),
        }

        let measures_in = sample_measures();
        let (st2, out2) = local_tx_monitor_transition(
            st1,
            LocalTxMonitorAgency::Server,
            v(2),
            LocalTxMonitorMessage::ReplyGetMeasures(measures_in.clone()),
        )
        .expect("busy_get_measures+reply");
        assert_eq!(st2, LocalTxMonitorState::Acquired);
        match out2 {
            LocalTxMonitorOutput::Event(LocalTxMonitorEvent::MeasuresReplied(measures)) => {
                assert_eq!(measures, measures_in);
            }
            other => panic!("expected Event(MeasuresReplied), got {other:?}"),
        }
    }

    #[test]
    fn get_measures_rejected_below_v2() {
        let err = local_tx_monitor_transition(
            LocalTxMonitorState::Acquired,
            LocalTxMonitorAgency::Client,
            v(1),
            LocalTxMonitorMessage::GetMeasures,
        )
        .expect_err("must reject below V2");
        match err {
            LocalTxMonitorError::InvalidForVersion {
                version,
                message_tag,
            } => {
                assert_eq!(version, v(1));
                assert_eq!(message_tag, "GetMeasures");
            }
            other => panic!("expected InvalidForVersion, got {other:?}"),
        }
    }

    #[test]
    fn client_done_terminates_from_idle() {
        let (st, out) = local_tx_monitor_transition(
            LocalTxMonitorState::Idle,
            LocalTxMonitorAgency::Client,
            v(2),
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
            v(2),
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
    fn version_gating_rejects_out_of_version_message() {
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
