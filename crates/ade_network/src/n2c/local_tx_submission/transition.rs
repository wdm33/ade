// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Pure LocalTxSubmission transition function.
//
// Shape (per slice §9):
//   fn local_tx_submission_transition(state, agency, version, msg)
//       -> Result<(new_state, output), error>
//
// No async, no I/O, no ambient state. The selected version is an
// explicit input — never read from a session global (DC-PROTO-06).
//
// State graph (only these tuples are legal; everything else is
// IllegalTransition):
//
//   Idle + Client + SubmitTx{tx_bytes} -> Busy + Event(TxSubmitted{tx_bytes})
//   Idle + Client + Done               -> Done + Done
//   Busy + Server + AcceptTx(_)        -> Idle + Event(TxAccepted)
//   Busy + Server + RejectTx(reason)   -> Idle + Event(TxRejected{rejection: reason})

use crate::codec::local_tx_submission::LocalTxSubmissionMessage;
use crate::codec::version::LocalTxSubmissionVersion;
use crate::n2c::local_tx_submission::agency::LocalTxSubmissionAgency;
use crate::n2c::local_tx_submission::event::LocalTxSubmissionEvent;
use crate::n2c::local_tx_submission::state::{
    LocalTxSubmissionError, LocalTxSubmissionOutput, LocalTxSubmissionState,
};

/// Highest LocalTxSubmission mini-protocol version this state machine
/// accepts.
///
/// LocalTxSubmission has shipped a single closed grammar (4 messages,
/// no version-gated variants) for every cardano-node 11.0.1 (10.6.2 forward-compatible) supported
/// version. We pin the upper bound at `MAX_LOCAL_TX_SUBMISSION_VERSION`
/// so a future spec extension cannot silently transit messages whose
/// semantics this state machine has not been updated for.
const MAX_LOCAL_TX_SUBMISSION_VERSION: u16 = 100;

/// Pure LocalTxSubmission transition.
///
/// `agency` is the agency the *sender* held when it produced `msg`.
/// Client-originated messages (SubmitTx / Done) are paired with
/// `LocalTxSubmissionAgency::Client`; server-originated replies are
/// paired with `Server`. Any other pairing returns `IllegalTransition`.
pub fn local_tx_submission_transition(
    state: LocalTxSubmissionState,
    agency: LocalTxSubmissionAgency,
    version: LocalTxSubmissionVersion,
    msg: LocalTxSubmissionMessage,
) -> Result<(LocalTxSubmissionState, LocalTxSubmissionOutput), LocalTxSubmissionError> {
    if version.get() > MAX_LOCAL_TX_SUBMISSION_VERSION {
        return Err(LocalTxSubmissionError::InvalidForVersion {
            version,
            message_tag: message_tag(&msg),
        });
    }
    match (state, agency, msg) {
        (
            LocalTxSubmissionState::Idle,
            LocalTxSubmissionAgency::Client,
            LocalTxSubmissionMessage::SubmitTx { tx_bytes },
        ) => Ok((
            LocalTxSubmissionState::Busy,
            LocalTxSubmissionOutput::Event(LocalTxSubmissionEvent::TxSubmitted { tx_bytes }),
        )),
        (
            LocalTxSubmissionState::Idle,
            LocalTxSubmissionAgency::Client,
            LocalTxSubmissionMessage::Done,
        ) => Ok((LocalTxSubmissionState::Done, LocalTxSubmissionOutput::Done)),
        (
            LocalTxSubmissionState::Busy,
            LocalTxSubmissionAgency::Server,
            LocalTxSubmissionMessage::AcceptTx(_),
        ) => Ok((
            LocalTxSubmissionState::Idle,
            LocalTxSubmissionOutput::Event(LocalTxSubmissionEvent::TxAccepted),
        )),
        (
            LocalTxSubmissionState::Busy,
            LocalTxSubmissionAgency::Server,
            LocalTxSubmissionMessage::RejectTx(rejection),
        ) => Ok((
            LocalTxSubmissionState::Idle,
            LocalTxSubmissionOutput::Event(LocalTxSubmissionEvent::TxRejected { rejection }),
        )),
        (state, agency, msg) => Err(LocalTxSubmissionError::IllegalTransition {
            state,
            message_tag: message_tag(&msg),
            agency,
        }),
    }
}

fn message_tag(msg: &LocalTxSubmissionMessage) -> &'static str {
    match msg {
        LocalTxSubmissionMessage::SubmitTx { .. } => "SubmitTx",
        LocalTxSubmissionMessage::AcceptTx(_) => "AcceptTx",
        LocalTxSubmissionMessage::RejectTx(_) => "RejectTx",
        LocalTxSubmissionMessage::Done => "Done",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::local_tx_submission::{TxAcceptance, TxRejection};

    fn version() -> LocalTxSubmissionVersion {
        LocalTxSubmissionVersion::new(16)
    }

    fn sample_tx_bytes() -> Vec<u8> {
        vec![0x84, 0xA1, 0x02, 0x03, 0x04, 0xFF, 0xCA, 0xFE, 0xBA, 0xBE]
    }

    #[test]
    fn submit_tx_then_accept_round_trips() {
        let tx_in = sample_tx_bytes();
        let (st1, out1) = local_tx_submission_transition(
            LocalTxSubmissionState::Idle,
            LocalTxSubmissionAgency::Client,
            version(),
            LocalTxSubmissionMessage::SubmitTx {
                tx_bytes: tx_in.clone(),
            },
        )
        .expect("idle+submit_tx");
        assert_eq!(st1, LocalTxSubmissionState::Busy);
        match out1 {
            LocalTxSubmissionOutput::Event(LocalTxSubmissionEvent::TxSubmitted { tx_bytes }) => {
                assert_eq!(tx_bytes, tx_in);
            }
            other => panic!("expected Event(TxSubmitted), got {other:?}"),
        }

        let (st2, out2) = local_tx_submission_transition(
            st1,
            LocalTxSubmissionAgency::Server,
            version(),
            LocalTxSubmissionMessage::AcceptTx(TxAcceptance),
        )
        .expect("busy+accept_tx");
        assert_eq!(st2, LocalTxSubmissionState::Idle);
        match out2 {
            LocalTxSubmissionOutput::Event(LocalTxSubmissionEvent::TxAccepted) => {}
            other => panic!("expected Event(TxAccepted), got {other:?}"),
        }
    }

    #[test]
    fn submit_tx_then_reject_carries_reason_bytes() {
        let tx_in = sample_tx_bytes();
        let reject_bytes = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03];
        let (st1, _) = local_tx_submission_transition(
            LocalTxSubmissionState::Idle,
            LocalTxSubmissionAgency::Client,
            version(),
            LocalTxSubmissionMessage::SubmitTx { tx_bytes: tx_in },
        )
        .expect("idle+submit_tx");
        assert_eq!(st1, LocalTxSubmissionState::Busy);

        let (st2, out2) = local_tx_submission_transition(
            st1,
            LocalTxSubmissionAgency::Server,
            version(),
            LocalTxSubmissionMessage::RejectTx(TxRejection(reject_bytes.clone())),
        )
        .expect("busy+reject_tx");
        assert_eq!(st2, LocalTxSubmissionState::Idle);
        match out2 {
            LocalTxSubmissionOutput::Event(LocalTxSubmissionEvent::TxRejected { rejection }) => {
                assert_eq!(rejection, TxRejection(reject_bytes));
            }
            other => panic!("expected Event(TxRejected), got {other:?}"),
        }
    }

    #[test]
    fn client_done_terminates() {
        let (st, out) = local_tx_submission_transition(
            LocalTxSubmissionState::Idle,
            LocalTxSubmissionAgency::Client,
            version(),
            LocalTxSubmissionMessage::Done,
        )
        .expect("idle+done");
        assert_eq!(st, LocalTxSubmissionState::Done);
        match out {
            LocalTxSubmissionOutput::Done => {}
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn wrong_agency_returns_error() {
        // SubmitTx is a client-originated message; passing Server
        // agency for it is grammar-illegal.
        let err = local_tx_submission_transition(
            LocalTxSubmissionState::Idle,
            LocalTxSubmissionAgency::Server,
            version(),
            LocalTxSubmissionMessage::SubmitTx {
                tx_bytes: sample_tx_bytes(),
            },
        )
        .expect_err("must reject");
        match err {
            LocalTxSubmissionError::IllegalTransition {
                message_tag,
                agency,
                ..
            } => {
                assert_eq!(message_tag, "SubmitTx");
                assert_eq!(agency, LocalTxSubmissionAgency::Server);
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn version_gating() {
        let bogus_version = LocalTxSubmissionVersion::new(MAX_LOCAL_TX_SUBMISSION_VERSION + 1);
        let err = local_tx_submission_transition(
            LocalTxSubmissionState::Idle,
            LocalTxSubmissionAgency::Client,
            bogus_version,
            LocalTxSubmissionMessage::SubmitTx {
                tx_bytes: sample_tx_bytes(),
            },
        )
        .expect_err("must reject");
        match err {
            LocalTxSubmissionError::InvalidForVersion {
                version,
                message_tag,
            } => {
                assert_eq!(version, bogus_version);
                assert_eq!(message_tag, "SubmitTx");
            }
            other => panic!("expected InvalidForVersion, got {other:?}"),
        }
    }
}
