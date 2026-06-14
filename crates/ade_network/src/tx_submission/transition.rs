// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Pure tx-submission2 transition function.
//
// Shape (per slice §9):
//   fn tx_submission2_transition(state, agency, version, msg)
//       -> Result<(new_state, output), error>
//
// No async, no I/O, no ambient state. The selected version is an
// explicit input — never read from a session global (DC-PROTO-06).
//
// State graph (only these tuples are legal; everything else is
// IllegalTransition):
//
//   Init               + Client + Init                         -> Idle               + Event(ServerOpened)
//   Idle               + Server + RequestTxIds{blocking:T,..}  -> TxIdsBlocking{req} + Event(IdsRequested{..})
//   Idle               + Server + RequestTxIds{blocking:F,..}  -> TxIdsNonBlocking{req} + Event(IdsRequested{..})
//   TxIdsBlocking{req} + Client + ReplyTxIds(entries)          -> Idle               + Event(IdsDelivered{entries})
//   TxIdsNonBlocking{req} + Client + ReplyTxIds(entries)       -> Idle               + Event(IdsDelivered{entries})
//   Idle               + Server + RequestTxs(ids)              -> TxsRequested{n}    + Event(TxsRequested{ids})
//   TxsRequested{n}    + Client + ReplyTxs(tx_bytes)           -> Idle               + Event(TxsDelivered{tx_bytes})
//   Idle               + Server + Done                         -> Done               + Done
//
// Grammar invariants enforced (per Ouroboros tx-submission2 spec):
//   - Blocking ReplyTxIds must be non-empty (the blocking call promises
//     ≥1 ID).
//   - ReplyTxIds count ≤ req advertised.
//   - RequestTxs must request ≥1 tx (server cannot request zero).
//   - ReplyTxs count ≤ outstanding req_count (subset reply is legal,
//     overfill is not).

use crate::codec::tx_submission::TxSubmission2Message;
use crate::codec::version::TxSubmission2Version;
use crate::tx_submission::agency::TxSubmission2Agency;
use crate::tx_submission::event::InventoryEvent;
use crate::tx_submission::state::{TxSubmission2Error, TxSubmission2Output, TxSubmission2State};

/// Highest tx-submission2 mini-protocol version this state machine accepts.
///
/// Tx-submission2 has shipped a single closed grammar (6 messages, no
/// version-gated variants) for every cardano-node 11.0.1 (10.6.2 forward-compatible) supported
/// version. We pin the upper bound at `MAX_TX_SUBMISSION_VERSION` so a
/// future spec extension cannot silently transit messages whose
/// semantics this state machine has not been updated for — the
/// `InvalidForVersion` error surfaces the mismatch at the protocol
/// boundary instead of letting an unknown future variant through.
const MAX_TX_SUBMISSION_VERSION: u16 = 100;

/// Pure tx-submission2 transition.
///
/// `agency` is the agency the *sender* held when it produced `msg`.
/// Per the inverted client-server semantics of tx-submission2, server
/// originates `RequestTxIds` / `RequestTxs` / `Done`; client originates
/// `Init` / `ReplyTxIds` / `ReplyTxs`. Any other pairing returns
/// `IllegalTransition`.
pub fn tx_submission2_transition(
    state: TxSubmission2State,
    agency: TxSubmission2Agency,
    version: TxSubmission2Version,
    msg: TxSubmission2Message,
) -> Result<(TxSubmission2State, TxSubmission2Output), TxSubmission2Error> {
    if version.get() > MAX_TX_SUBMISSION_VERSION {
        return Err(TxSubmission2Error::InvalidForVersion {
            version,
            message_tag: message_tag(&msg),
        });
    }
    match (state, agency, msg) {
        (TxSubmission2State::Init, TxSubmission2Agency::Client, TxSubmission2Message::Init) => Ok((
            TxSubmission2State::Idle,
            TxSubmission2Output::Event(InventoryEvent::ServerOpened),
        )),
        (
            TxSubmission2State::Idle,
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxIds { blocking, ack, req },
        ) => {
            let next = if blocking {
                TxSubmission2State::TxIdsBlocking { req }
            } else {
                TxSubmission2State::TxIdsNonBlocking { req }
            };
            Ok((
                next,
                TxSubmission2Output::Event(InventoryEvent::IdsRequested { blocking, ack, req }),
            ))
        }
        (
            TxSubmission2State::TxIdsBlocking { req },
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxIds(entries),
        ) => {
            if entries.is_empty() {
                return Err(TxSubmission2Error::MalformedMessage {
                    reason: "blocking ReplyTxIds must be non-empty",
                });
            }
            if entries.len() > req as usize {
                return Err(TxSubmission2Error::MalformedMessage {
                    reason: "ReplyTxIds count exceeds requested req",
                });
            }
            Ok((
                TxSubmission2State::Idle,
                TxSubmission2Output::Event(InventoryEvent::IdsDelivered { entries }),
            ))
        }
        (
            TxSubmission2State::TxIdsNonBlocking { req },
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxIds(entries),
        ) => {
            if entries.len() > req as usize {
                return Err(TxSubmission2Error::MalformedMessage {
                    reason: "ReplyTxIds count exceeds requested req",
                });
            }
            Ok((
                TxSubmission2State::Idle,
                TxSubmission2Output::Event(InventoryEvent::IdsDelivered { entries }),
            ))
        }
        (
            TxSubmission2State::Idle,
            TxSubmission2Agency::Server,
            TxSubmission2Message::RequestTxs(ids),
        ) => {
            if ids.is_empty() {
                return Err(TxSubmission2Error::MalformedMessage {
                    reason: "RequestTxs must request at least one tx",
                });
            }
            let req_count = ids.len();
            Ok((
                TxSubmission2State::TxsRequested { req_count },
                TxSubmission2Output::Event(InventoryEvent::TxsRequested { ids }),
            ))
        }
        (
            TxSubmission2State::TxsRequested { req_count },
            TxSubmission2Agency::Client,
            TxSubmission2Message::ReplyTxs(tx_bytes),
        ) => {
            if tx_bytes.len() > req_count {
                return Err(TxSubmission2Error::MalformedMessage {
                    reason: "ReplyTxs count exceeds requested req_count",
                });
            }
            Ok((
                TxSubmission2State::Idle,
                TxSubmission2Output::Event(InventoryEvent::TxsDelivered { tx_bytes }),
            ))
        }
        (TxSubmission2State::Idle, TxSubmission2Agency::Server, TxSubmission2Message::Done) => {
            Ok((TxSubmission2State::Done, TxSubmission2Output::Done))
        }
        (state, agency, msg) => Err(TxSubmission2Error::IllegalTransition {
            state,
            message_tag: message_tag(&msg),
            agency,
        }),
    }
}

fn message_tag(msg: &TxSubmission2Message) -> &'static str {
    match msg {
        TxSubmission2Message::Init => "Init",
        TxSubmission2Message::RequestTxIds { .. } => "RequestTxIds",
        TxSubmission2Message::ReplyTxIds(_) => "ReplyTxIds",
        TxSubmission2Message::RequestTxs(_) => "RequestTxs",
        TxSubmission2Message::ReplyTxs(_) => "ReplyTxs",
        TxSubmission2Message::Done => "Done",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::tx_submission::{TxIdAndSize, TxSubmissionTxId};
    use ade_types::{Hash32, TxId};

    fn version() -> TxSubmission2Version {
        TxSubmission2Version::new(13)
    }

    fn tx_id(seed: u8) -> TxSubmissionTxId {
        TxSubmissionTxId { era: 6, id: TxId(Hash32([seed; 32])) }
    }

    fn sample_entries(n: usize) -> Vec<TxIdAndSize> {
        (0..n)
            .map(|i| TxIdAndSize {
                tx_id: tx_id(i as u8 + 1),
                size: 100 + i as u32,
            })
            .collect()
    }

    fn sample_tx_bytes(n: usize) -> Vec<Vec<u8>> {
        (0..n)
            .map(|i| {
                let b = (i as u8).wrapping_add(0xA0);
                vec![0x82, b, b.wrapping_add(1), b.wrapping_add(2), 0xFF]
            })
            .collect()
    }

    #[test]
    fn init_handshake_opens_session() {
        let (st, out) = tx_submission2_transition(
            TxSubmission2State::Init,
            TxSubmission2Agency::Client,
            version(),
            TxSubmission2Message::Init,
        )
        .expect("init+client+init");
        assert_eq!(st, TxSubmission2State::Idle);
        match out {
            TxSubmission2Output::Event(InventoryEvent::ServerOpened) => {}
            other => panic!("expected Event(ServerOpened), got {other:?}"),
        }
    }

    #[test]
    fn request_blocking_ids_then_reply_round_trips() {
        let (st1, out1) = tx_submission2_transition(
            TxSubmission2State::Idle,
            TxSubmission2Agency::Server,
            version(),
            TxSubmission2Message::RequestTxIds {
                blocking: true,
                ack: 0,
                req: 10,
            },
        )
        .expect("idle+server+request_blocking");
        assert_eq!(st1, TxSubmission2State::TxIdsBlocking { req: 10 });
        match out1 {
            TxSubmission2Output::Event(InventoryEvent::IdsRequested {
                blocking,
                ack,
                req,
            }) => {
                assert!(blocking);
                assert_eq!(ack, 0);
                assert_eq!(req, 10);
            }
            other => panic!("expected Event(IdsRequested), got {other:?}"),
        }

        let entries = sample_entries(3);
        let (st2, out2) = tx_submission2_transition(
            st1,
            TxSubmission2Agency::Client,
            version(),
            TxSubmission2Message::ReplyTxIds(entries.clone()),
        )
        .expect("blocking+client+reply");
        assert_eq!(st2, TxSubmission2State::Idle);
        match out2 {
            TxSubmission2Output::Event(InventoryEvent::IdsDelivered { entries: got }) => {
                assert_eq!(got, entries);
            }
            other => panic!("expected Event(IdsDelivered), got {other:?}"),
        }
    }

    #[test]
    fn request_non_blocking_ids_with_empty_reply_round_trips() {
        let (st1, _) = tx_submission2_transition(
            TxSubmission2State::Idle,
            TxSubmission2Agency::Server,
            version(),
            TxSubmission2Message::RequestTxIds {
                blocking: false,
                ack: 0,
                req: 5,
            },
        )
        .expect("idle+server+request_nonblocking");
        assert_eq!(st1, TxSubmission2State::TxIdsNonBlocking { req: 5 });

        // Empty reply is legal in non-blocking mode.
        let (st2, out2) = tx_submission2_transition(
            st1,
            TxSubmission2Agency::Client,
            version(),
            TxSubmission2Message::ReplyTxIds(Vec::new()),
        )
        .expect("nonblocking+client+empty_reply");
        assert_eq!(st2, TxSubmission2State::Idle);
        match out2 {
            TxSubmission2Output::Event(InventoryEvent::IdsDelivered { entries }) => {
                assert!(entries.is_empty());
            }
            other => panic!("expected Event(IdsDelivered{{entries: []}}), got {other:?}"),
        }
    }

    #[test]
    fn request_txs_then_reply_delivers_bytes_byte_identical() {
        let ids = vec![tx_id(0x11), tx_id(0x22), tx_id(0x33)];
        let (st1, out1) = tx_submission2_transition(
            TxSubmission2State::Idle,
            TxSubmission2Agency::Server,
            version(),
            TxSubmission2Message::RequestTxs(ids.clone()),
        )
        .expect("idle+server+request_txs");
        assert_eq!(st1, TxSubmission2State::TxsRequested { req_count: 3 });
        match out1 {
            TxSubmission2Output::Event(InventoryEvent::TxsRequested { ids: got }) => {
                assert_eq!(got, ids);
            }
            other => panic!("expected Event(TxsRequested), got {other:?}"),
        }

        let bytes_in = sample_tx_bytes(3);
        let (st2, out2) = tx_submission2_transition(
            st1,
            TxSubmission2Agency::Client,
            version(),
            TxSubmission2Message::ReplyTxs(bytes_in.clone()),
        )
        .expect("txsrequested+client+reply_txs");
        assert_eq!(st2, TxSubmission2State::Idle);
        match out2 {
            TxSubmission2Output::Event(InventoryEvent::TxsDelivered { tx_bytes }) => {
                assert_eq!(tx_bytes, bytes_in);
            }
            other => panic!("expected Event(TxsDelivered), got {other:?}"),
        }
    }

    #[test]
    fn request_txs_with_partial_reply_legal() {
        // Server asks for 5, client only has 3 — subset reply is
        // grammatically legal (some requested txs may be unavailable).
        let ids = (0..5u8).map(tx_id).collect::<Vec<_>>();
        let (st1, _) = tx_submission2_transition(
            TxSubmission2State::Idle,
            TxSubmission2Agency::Server,
            version(),
            TxSubmission2Message::RequestTxs(ids),
        )
        .expect("idle+server+request_5");
        assert_eq!(st1, TxSubmission2State::TxsRequested { req_count: 5 });

        let partial = sample_tx_bytes(3);
        let (st2, out2) = tx_submission2_transition(
            st1,
            TxSubmission2Agency::Client,
            version(),
            TxSubmission2Message::ReplyTxs(partial.clone()),
        )
        .expect("txsrequested+client+partial");
        assert_eq!(st2, TxSubmission2State::Idle);
        match out2 {
            TxSubmission2Output::Event(InventoryEvent::TxsDelivered { tx_bytes }) => {
                assert_eq!(tx_bytes, partial);
            }
            other => panic!("expected Event(TxsDelivered), got {other:?}"),
        }
    }

    #[test]
    fn blocking_reply_empty_is_malformed() {
        // Blocking call promises ≥1 ID; empty reply is grammar-illegal.
        let err = tx_submission2_transition(
            TxSubmission2State::TxIdsBlocking { req: 10 },
            TxSubmission2Agency::Client,
            version(),
            TxSubmission2Message::ReplyTxIds(Vec::new()),
        )
        .expect_err("must reject empty blocking reply");
        match err {
            TxSubmission2Error::MalformedMessage { reason } => {
                assert_eq!(reason, "blocking ReplyTxIds must be non-empty");
            }
            other => panic!("expected MalformedMessage, got {other:?}"),
        }
    }

    #[test]
    fn reply_ids_overfill_is_malformed() {
        // Client cannot return more IDs than the server asked for.
        let too_many = sample_entries(6);
        let err = tx_submission2_transition(
            TxSubmission2State::TxIdsNonBlocking { req: 5 },
            TxSubmission2Agency::Client,
            version(),
            TxSubmission2Message::ReplyTxIds(too_many),
        )
        .expect_err("must reject overfill");
        match err {
            TxSubmission2Error::MalformedMessage { reason } => {
                assert_eq!(reason, "ReplyTxIds count exceeds requested req");
            }
            other => panic!("expected MalformedMessage, got {other:?}"),
        }
    }

    #[test]
    fn request_txs_empty_is_malformed() {
        // Server must request ≥1 tx.
        let err = tx_submission2_transition(
            TxSubmission2State::Idle,
            TxSubmission2Agency::Server,
            version(),
            TxSubmission2Message::RequestTxs(Vec::new()),
        )
        .expect_err("must reject empty RequestTxs");
        match err {
            TxSubmission2Error::MalformedMessage { reason } => {
                assert_eq!(reason, "RequestTxs must request at least one tx");
            }
            other => panic!("expected MalformedMessage, got {other:?}"),
        }
    }

    #[test]
    fn reply_txs_overfill_is_malformed() {
        // Client cannot return more bodies than the server requested IDs.
        let overfill = sample_tx_bytes(5);
        let err = tx_submission2_transition(
            TxSubmission2State::TxsRequested { req_count: 3 },
            TxSubmission2Agency::Client,
            version(),
            TxSubmission2Message::ReplyTxs(overfill),
        )
        .expect_err("must reject overfill");
        match err {
            TxSubmission2Error::MalformedMessage { reason } => {
                assert_eq!(reason, "ReplyTxs count exceeds requested req_count");
            }
            other => panic!("expected MalformedMessage, got {other:?}"),
        }
    }

    #[test]
    fn done_terminates_session() {
        let (st, out) = tx_submission2_transition(
            TxSubmission2State::Idle,
            TxSubmission2Agency::Server,
            version(),
            TxSubmission2Message::Done,
        )
        .expect("idle+server+done");
        assert_eq!(st, TxSubmission2State::Done);
        match out {
            TxSubmission2Output::Done => {}
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn illegal_message_in_init_returns_error() {
        // Only the Init handshake is legal in Init state; any other
        // message is grammar-illegal.
        let err = tx_submission2_transition(
            TxSubmission2State::Init,
            TxSubmission2Agency::Server,
            version(),
            TxSubmission2Message::RequestTxIds {
                blocking: true,
                ack: 0,
                req: 1,
            },
        )
        .expect_err("must reject");
        match err {
            TxSubmission2Error::IllegalTransition {
                state,
                message_tag,
                agency,
            } => {
                assert_eq!(state, TxSubmission2State::Init);
                assert_eq!(message_tag, "RequestTxIds");
                assert_eq!(agency, TxSubmission2Agency::Server);
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn wrong_agency_returns_error() {
        // Init is client-originated; pairing it with Server agency is
        // grammar-illegal.
        let err = tx_submission2_transition(
            TxSubmission2State::Init,
            TxSubmission2Agency::Server,
            version(),
            TxSubmission2Message::Init,
        )
        .expect_err("must reject server+init");
        match err {
            TxSubmission2Error::IllegalTransition {
                message_tag,
                agency,
                ..
            } => {
                assert_eq!(message_tag, "Init");
                assert_eq!(agency, TxSubmission2Agency::Server);
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }

        // RequestTxIds is server-originated; pairing it with Client
        // agency is grammar-illegal.
        let err = tx_submission2_transition(
            TxSubmission2State::Idle,
            TxSubmission2Agency::Client,
            version(),
            TxSubmission2Message::RequestTxIds {
                blocking: true,
                ack: 0,
                req: 1,
            },
        )
        .expect_err("must reject client+request");
        match err {
            TxSubmission2Error::IllegalTransition {
                message_tag,
                agency,
                ..
            } => {
                assert_eq!(message_tag, "RequestTxIds");
                assert_eq!(agency, TxSubmission2Agency::Client);
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn version_gating_rejects_out_of_version_message() {
        // The tx-submission2 wire grammar across cardano-node 11.0.1 (10.6.2 forward-compatible)
        // has shipped a single closed message set for every supported
        // version, so there is no real per-variant version gating yet.
        // The state machine still has to expose the InvalidForVersion
        // error path because the type signature commits to it; the
        // pinned guard rejects future versions above
        // MAX_TX_SUBMISSION_VERSION = 100.
        let bogus_version = TxSubmission2Version::new(MAX_TX_SUBMISSION_VERSION + 1);
        let err = tx_submission2_transition(
            TxSubmission2State::Init,
            TxSubmission2Agency::Client,
            bogus_version,
            TxSubmission2Message::Init,
        )
        .expect_err("must reject");
        match err {
            TxSubmission2Error::InvalidForVersion {
                version,
                message_tag,
            } => {
                assert_eq!(version, bogus_version);
                assert_eq!(message_tag, "Init");
            }
            other => panic!("expected InvalidForVersion, got {other:?}"),
        }
    }
}
