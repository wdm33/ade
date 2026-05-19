// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Pure chain-sync transition function.
//
// Shape (per slice §9):
//   fn chain_sync_transition(state, agency, version, msg)
//       -> Result<(new_state, output), error>
//
// No async, no I/O, no ambient state. The selected version is an
// explicit input — never read from a session global (DC-PROTO-06).
//
// State graph (only these tuples are legal; everything else is
// IllegalTransition):
//
//   Idle      + Client + RequestNext                 -> CanAwait  + Reply(RequestNext)
//   Idle      + Client + FindIntersect{points}       -> Intersect + Reply(FindIntersect{points})
//   Idle      + Client + Done                        -> Done      + Done
//   CanAwait  + Server + RollForward{h, t}           -> Idle      + Signal(RollForward{header_bytes:h, tip:t})
//   CanAwait  + Server + RollBackward{p, t}          -> Idle      + Signal(RollBackward{point:p, tip:t})
//   CanAwait  + Server + AwaitReply                  -> MustReply + Reply(AwaitReply)
//   MustReply + Server + RollForward{h, t}           -> Idle      + Signal(RollForward{header_bytes:h, tip:t})
//   MustReply + Server + RollBackward{p, t}          -> Idle      + Signal(RollBackward{point:p, tip:t})
//   Intersect + Server + IntersectFound{p, t}        -> Idle      + Signal(Intersected{point:p, tip:t})
//   Intersect + Server + IntersectNotFound{t}        -> Idle      + Signal(NoIntersection{tip:t})
//
// Idle + Client + RequestNext unconditionally transitions to CanAwait;
// the move to MustReply happens only when the server explicitly sends
// AwaitReply from CanAwait. The state machine cannot know whether the
// server has data ready — that's an out-of-band server decision.

use crate::chain_sync::agency::ChainSyncAgency;
use crate::chain_sync::signal::ForkChoiceSignal;
use crate::chain_sync::state::{ChainSyncError, ChainSyncOutput, ChainSyncState};
use crate::codec::chain_sync::ChainSyncMessage;
use crate::codec::version::ChainSyncVersion;

/// Highest chain-sync mini-protocol version this state machine accepts.
///
/// Chain-sync has shipped a single closed grammar (8 messages, no
/// version-gated variants) for every cardano-node 10.6.2 supported
/// version. We pin the upper bound at `MAX_CHAIN_SYNC_VERSION` so a
/// future spec extension cannot silently transit messages whose
/// semantics this state machine has not been updated for — the
/// `InvalidForVersion` error surfaces the mismatch at the protocol
/// boundary instead of letting an unknown future variant through.
const MAX_CHAIN_SYNC_VERSION: u16 = 100;

/// Pure chain-sync transition.
///
/// `agency` is the agency the *sender* held when it produced `msg`.
/// Client-originated messages (RequestNext / FindIntersect / Done) are
/// paired with `ChainSyncAgency::Client`; server-originated replies are
/// paired with `Server`. Any other pairing returns `IllegalTransition`.
pub fn chain_sync_transition(
    state: ChainSyncState,
    agency: ChainSyncAgency,
    version: ChainSyncVersion,
    msg: ChainSyncMessage,
) -> Result<(ChainSyncState, ChainSyncOutput), ChainSyncError> {
    if version.get() > MAX_CHAIN_SYNC_VERSION {
        return Err(ChainSyncError::InvalidForVersion {
            version,
            message_tag: message_tag(&msg),
        });
    }
    match (state, agency, msg) {
        (ChainSyncState::Idle, ChainSyncAgency::Client, ChainSyncMessage::RequestNext) => Ok((
            ChainSyncState::CanAwait,
            ChainSyncOutput::Reply(ChainSyncMessage::RequestNext),
        )),
        (
            ChainSyncState::Idle,
            ChainSyncAgency::Client,
            ChainSyncMessage::FindIntersect { points },
        ) => {
            if points.is_empty() {
                return Err(ChainSyncError::MalformedMessage {
                    reason: "FindIntersect carried an empty points list",
                });
            }
            Ok((
                ChainSyncState::Intersect,
                ChainSyncOutput::Reply(ChainSyncMessage::FindIntersect { points }),
            ))
        }
        (ChainSyncState::Idle, ChainSyncAgency::Client, ChainSyncMessage::Done) => {
            Ok((ChainSyncState::Done, ChainSyncOutput::Done))
        }
        (
            ChainSyncState::CanAwait,
            ChainSyncAgency::Server,
            ChainSyncMessage::RollForward { header, tip },
        ) => Ok((
            ChainSyncState::Idle,
            ChainSyncOutput::Signal(ForkChoiceSignal::RollForward {
                header_bytes: header,
                tip,
            }),
        )),
        (
            ChainSyncState::CanAwait,
            ChainSyncAgency::Server,
            ChainSyncMessage::RollBackward { point, tip },
        ) => Ok((
            ChainSyncState::Idle,
            ChainSyncOutput::Signal(ForkChoiceSignal::RollBackward { point, tip }),
        )),
        (ChainSyncState::CanAwait, ChainSyncAgency::Server, ChainSyncMessage::AwaitReply) => Ok((
            ChainSyncState::MustReply,
            ChainSyncOutput::Reply(ChainSyncMessage::AwaitReply),
        )),
        (
            ChainSyncState::MustReply,
            ChainSyncAgency::Server,
            ChainSyncMessage::RollForward { header, tip },
        ) => Ok((
            ChainSyncState::Idle,
            ChainSyncOutput::Signal(ForkChoiceSignal::RollForward {
                header_bytes: header,
                tip,
            }),
        )),
        (
            ChainSyncState::MustReply,
            ChainSyncAgency::Server,
            ChainSyncMessage::RollBackward { point, tip },
        ) => Ok((
            ChainSyncState::Idle,
            ChainSyncOutput::Signal(ForkChoiceSignal::RollBackward { point, tip }),
        )),
        (
            ChainSyncState::Intersect,
            ChainSyncAgency::Server,
            ChainSyncMessage::IntersectFound { point, tip },
        ) => Ok((
            ChainSyncState::Idle,
            ChainSyncOutput::Signal(ForkChoiceSignal::Intersected { point, tip }),
        )),
        (
            ChainSyncState::Intersect,
            ChainSyncAgency::Server,
            ChainSyncMessage::IntersectNotFound { tip },
        ) => Ok((
            ChainSyncState::Idle,
            ChainSyncOutput::Signal(ForkChoiceSignal::NoIntersection { tip }),
        )),
        (state, agency, msg) => Err(ChainSyncError::IllegalTransition {
            state,
            message_tag: message_tag(&msg),
            agency: agency_tag(agency),
        }),
    }
}

fn message_tag(msg: &ChainSyncMessage) -> &'static str {
    match msg {
        ChainSyncMessage::RequestNext => "RequestNext",
        ChainSyncMessage::AwaitReply => "AwaitReply",
        ChainSyncMessage::RollForward { .. } => "RollForward",
        ChainSyncMessage::RollBackward { .. } => "RollBackward",
        ChainSyncMessage::FindIntersect { .. } => "FindIntersect",
        ChainSyncMessage::IntersectFound { .. } => "IntersectFound",
        ChainSyncMessage::IntersectNotFound { .. } => "IntersectNotFound",
        ChainSyncMessage::Done => "Done",
    }
}

fn agency_tag(agency: ChainSyncAgency) -> &'static str {
    match agency {
        ChainSyncAgency::Client => "Client",
        ChainSyncAgency::Server => "Server",
        ChainSyncAgency::Neither => "Neither",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::chain_sync::{Point, Tip};
    use ade_types::{Hash32, SlotNo};

    fn version() -> ChainSyncVersion {
        ChainSyncVersion::new(9)
    }

    fn sample_tip() -> Tip {
        Tip {
            point: Point::Block {
                slot: SlotNo(1234),
                hash: Hash32([0xAA; 32]),
            },
            block_no: 5678,
        }
    }

    fn other_tip() -> Tip {
        Tip {
            point: Point::Block {
                slot: SlotNo(9999),
                hash: Hash32([0xBB; 32]),
            },
            block_no: 42,
        }
    }

    fn sample_header_bytes() -> Vec<u8> {
        vec![0x82, 0x01, 0x02, 0x03, 0x04, 0xFF, 0xAA, 0xBB, 0xCC, 0xDD]
    }

    #[test]
    fn idle_request_next_with_immediate_data_yields_can_await_then_roll_forward() {
        // Two-step drive: Idle -> CanAwait via client RequestNext,
        // then CanAwait -> Idle + Signal via server RollForward (the
        // "server had data ready" branch).
        let (st1, out1) = chain_sync_transition(
            ChainSyncState::Idle,
            ChainSyncAgency::Client,
            version(),
            ChainSyncMessage::RequestNext,
        )
        .expect("idle+request_next");
        assert_eq!(st1, ChainSyncState::CanAwait);
        match out1 {
            ChainSyncOutput::Reply(ChainSyncMessage::RequestNext) => {}
            other => panic!("expected Reply(RequestNext), got {other:?}"),
        }

        let header = sample_header_bytes();
        let tip = sample_tip();
        let (st2, out2) = chain_sync_transition(
            st1,
            ChainSyncAgency::Server,
            version(),
            ChainSyncMessage::RollForward {
                header: header.clone(),
                tip: tip.clone(),
            },
        )
        .expect("can_await+roll_forward");
        assert_eq!(st2, ChainSyncState::Idle);
        match out2 {
            ChainSyncOutput::Signal(ForkChoiceSignal::RollForward {
                header_bytes,
                tip: sig_tip,
            }) => {
                assert_eq!(header_bytes, header);
                assert_eq!(sig_tip, tip);
            }
            other => panic!("expected Signal(RollForward), got {other:?}"),
        }
    }

    #[test]
    fn idle_request_next_with_no_data_yields_must_reply_via_await() {
        // Three-step drive: Idle -> CanAwait (RequestNext),
        // CanAwait -> MustReply (AwaitReply), MustReply -> Idle
        // (RollForward).
        let (st1, _) = chain_sync_transition(
            ChainSyncState::Idle,
            ChainSyncAgency::Client,
            version(),
            ChainSyncMessage::RequestNext,
        )
        .expect("idle+request_next");
        assert_eq!(st1, ChainSyncState::CanAwait);

        let (st2, out2) = chain_sync_transition(
            st1,
            ChainSyncAgency::Server,
            version(),
            ChainSyncMessage::AwaitReply,
        )
        .expect("can_await+await_reply");
        assert_eq!(st2, ChainSyncState::MustReply);
        match out2 {
            ChainSyncOutput::Reply(ChainSyncMessage::AwaitReply) => {}
            other => panic!("expected Reply(AwaitReply), got {other:?}"),
        }

        let header = sample_header_bytes();
        let tip = sample_tip();
        let (st3, out3) = chain_sync_transition(
            st2,
            ChainSyncAgency::Server,
            version(),
            ChainSyncMessage::RollForward {
                header: header.clone(),
                tip: tip.clone(),
            },
        )
        .expect("must_reply+roll_forward");
        assert_eq!(st3, ChainSyncState::Idle);
        match out3 {
            ChainSyncOutput::Signal(ForkChoiceSignal::RollForward {
                header_bytes,
                tip: sig_tip,
            }) => {
                assert_eq!(header_bytes, header);
                assert_eq!(sig_tip, tip);
            }
            other => panic!("expected Signal(RollForward), got {other:?}"),
        }
    }

    #[test]
    fn roll_forward_signal_carries_header_and_tip_byte_identical() {
        // The header bytes must be passed through verbatim; the tip
        // must round-trip its (point, block_no) shape unchanged. This
        // is the contract N-B relies on: no header decoding in N-A.
        let header_in: Vec<u8> = (0u8..=200).collect();
        let tip_in = sample_tip();
        let (_, out) = chain_sync_transition(
            ChainSyncState::CanAwait,
            ChainSyncAgency::Server,
            version(),
            ChainSyncMessage::RollForward {
                header: header_in.clone(),
                tip: tip_in.clone(),
            },
        )
        .expect("ok");
        match out {
            ChainSyncOutput::Signal(ForkChoiceSignal::RollForward { header_bytes, tip }) => {
                assert_eq!(header_bytes, header_in);
                assert_eq!(tip, tip_in);
            }
            other => panic!("expected Signal(RollForward), got {other:?}"),
        }
    }

    #[test]
    fn roll_backward_signal_carries_point_and_tip_byte_identical() {
        let point_in = Point::Block {
            slot: SlotNo(777),
            hash: Hash32([0xCC; 32]),
        };
        let tip_in = other_tip();
        let (_, out) = chain_sync_transition(
            ChainSyncState::CanAwait,
            ChainSyncAgency::Server,
            version(),
            ChainSyncMessage::RollBackward {
                point: point_in.clone(),
                tip: tip_in.clone(),
            },
        )
        .expect("ok");
        match out {
            ChainSyncOutput::Signal(ForkChoiceSignal::RollBackward { point, tip }) => {
                assert_eq!(point, point_in);
                assert_eq!(tip, tip_in);
            }
            other => panic!("expected Signal(RollBackward), got {other:?}"),
        }
    }

    #[test]
    fn find_intersect_with_known_point_yields_intersected_signal() {
        // Drive: Idle -> Intersect (client FindIntersect), then
        // Intersect -> Idle + Signal(Intersected) (server IntersectFound).
        let points = vec![
            Point::Origin,
            Point::Block {
                slot: SlotNo(100),
                hash: Hash32([0x11; 32]),
            },
        ];
        let (st1, out1) = chain_sync_transition(
            ChainSyncState::Idle,
            ChainSyncAgency::Client,
            version(),
            ChainSyncMessage::FindIntersect {
                points: points.clone(),
            },
        )
        .expect("idle+find_intersect");
        assert_eq!(st1, ChainSyncState::Intersect);
        match out1 {
            ChainSyncOutput::Reply(ChainSyncMessage::FindIntersect { points: replied }) => {
                assert_eq!(replied, points);
            }
            other => panic!("expected Reply(FindIntersect), got {other:?}"),
        }

        let found_point = Point::Block {
            slot: SlotNo(100),
            hash: Hash32([0x11; 32]),
        };
        let tip = sample_tip();
        let (st2, out2) = chain_sync_transition(
            st1,
            ChainSyncAgency::Server,
            version(),
            ChainSyncMessage::IntersectFound {
                point: found_point.clone(),
                tip: tip.clone(),
            },
        )
        .expect("intersect+intersect_found");
        assert_eq!(st2, ChainSyncState::Idle);
        match out2 {
            ChainSyncOutput::Signal(ForkChoiceSignal::Intersected {
                point,
                tip: sig_tip,
            }) => {
                assert_eq!(point, found_point);
                assert_eq!(sig_tip, tip);
            }
            other => panic!("expected Signal(Intersected), got {other:?}"),
        }
    }

    #[test]
    fn find_intersect_with_unknown_points_yields_no_intersection() {
        let points = vec![Point::Block {
            slot: SlotNo(1),
            hash: Hash32([0x00; 32]),
        }];
        let (st1, _) = chain_sync_transition(
            ChainSyncState::Idle,
            ChainSyncAgency::Client,
            version(),
            ChainSyncMessage::FindIntersect { points },
        )
        .expect("idle+find_intersect");
        assert_eq!(st1, ChainSyncState::Intersect);

        let tip = other_tip();
        let (st2, out2) = chain_sync_transition(
            st1,
            ChainSyncAgency::Server,
            version(),
            ChainSyncMessage::IntersectNotFound { tip: tip.clone() },
        )
        .expect("intersect+intersect_not_found");
        assert_eq!(st2, ChainSyncState::Idle);
        match out2 {
            ChainSyncOutput::Signal(ForkChoiceSignal::NoIntersection { tip: sig_tip }) => {
                assert_eq!(sig_tip, tip);
            }
            other => panic!("expected Signal(NoIntersection), got {other:?}"),
        }
    }

    #[test]
    fn client_done_terminates_session() {
        let (st, out) = chain_sync_transition(
            ChainSyncState::Idle,
            ChainSyncAgency::Client,
            version(),
            ChainSyncMessage::Done,
        )
        .expect("ok");
        assert_eq!(st, ChainSyncState::Done);
        match out {
            ChainSyncOutput::Done => {}
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn illegal_message_in_idle_returns_error() {
        // Server-only message arriving while the state machine is
        // Idle is grammar-illegal.
        let err = chain_sync_transition(
            ChainSyncState::Idle,
            ChainSyncAgency::Server,
            version(),
            ChainSyncMessage::RollForward {
                header: sample_header_bytes(),
                tip: sample_tip(),
            },
        )
        .expect_err("must reject");
        match err {
            ChainSyncError::IllegalTransition {
                state,
                message_tag,
                agency,
            } => {
                assert_eq!(state, ChainSyncState::Idle);
                assert_eq!(message_tag, "RollForward");
                assert_eq!(agency, "Server");
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn wrong_agency_returns_error() {
        // RequestNext is a client-originated message; passing Server
        // agency for it is grammar-illegal.
        let err = chain_sync_transition(
            ChainSyncState::Idle,
            ChainSyncAgency::Server,
            version(),
            ChainSyncMessage::RequestNext,
        )
        .expect_err("must reject");
        match err {
            ChainSyncError::IllegalTransition {
                message_tag,
                agency,
                ..
            } => {
                assert_eq!(message_tag, "RequestNext");
                assert_eq!(agency, "Server");
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn version_gating_rejects_out_of_version_message() {
        // The chain-sync wire grammar across cardano-node 10.6.2 has
        // shipped a single closed message set for every supported
        // version, so there is no real per-variant version gating yet
        // (the codec already accepts all 8 variants on the wire).
        // The state machine still has to expose the InvalidForVersion
        // error path because the type signature commits to it; the
        // pinned guard rejects future versions above
        // MAX_CHAIN_SYNC_VERSION = 100. When IOG ships a chain-sync
        // grammar extension, the gate moves to per-variant checks and
        // this test gets per-variant siblings.
        let bogus_version = ChainSyncVersion::new(MAX_CHAIN_SYNC_VERSION + 1);
        let err = chain_sync_transition(
            ChainSyncState::Idle,
            ChainSyncAgency::Client,
            bogus_version,
            ChainSyncMessage::RequestNext,
        )
        .expect_err("must reject");
        match err {
            ChainSyncError::InvalidForVersion {
                version,
                message_tag,
            } => {
                assert_eq!(version, bogus_version);
                assert_eq!(message_tag, "RequestNext");
            }
            other => panic!("expected InvalidForVersion, got {other:?}"),
        }
    }

    #[test]
    fn empty_find_intersect_points_returns_malformed() {
        // FindIntersect with no points is a wire-grammar violation: the
        // protocol requires at least one candidate point. The codec
        // accepts an empty list at the byte layer; the state machine is
        // the enforcement point for the grammar invariant. Covers the
        // ChainSyncError::MalformedMessage path.
        let err = chain_sync_transition(
            ChainSyncState::Idle,
            ChainSyncAgency::Client,
            ChainSyncVersion::new(14),
            ChainSyncMessage::FindIntersect { points: vec![] },
        )
        .expect_err("must reject");
        match err {
            ChainSyncError::MalformedMessage { reason } => {
                assert_eq!(reason, "FindIntersect carried an empty points list");
            }
            other => panic!("expected MalformedMessage, got {other:?}"),
        }
    }
}
