// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Pure LocalChainSync transition function.
//
// Shape (per slice §9):
//   fn local_chain_sync_transition(state, agency, version, msg)
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
//   CanAwait  + Server + RollForward{block, tip}     -> Idle      + Event(RollForward{block_bytes, tip})
//   CanAwait  + Server + RollBackward{point, tip}    -> Idle      + Event(RollBackward{point, tip})
//   CanAwait  + Server + AwaitReply                  -> MustReply + Reply(AwaitReply)
//   MustReply + Server + RollForward{block, tip}     -> Idle      + Event(RollForward{block_bytes, tip})
//   MustReply + Server + RollBackward{point, tip}    -> Idle      + Event(RollBackward{point, tip})
//   Intersect + Server + IntersectFound{point, tip}  -> Idle      + Event(Intersected{point, tip})
//   Intersect + Server + IntersectNotFound{tip}      -> Idle      + Event(NoIntersection{tip})

use crate::codec::local_chain_sync::LocalChainSyncMessage;
use crate::codec::version::LocalChainSyncVersion;
use crate::n2c::local_chain_sync::agency::LocalChainSyncAgency;
use crate::n2c::local_chain_sync::event::LocalChainSyncEvent;
use crate::n2c::local_chain_sync::state::{
    LocalChainSyncError, LocalChainSyncOutput, LocalChainSyncState,
};

/// Highest LocalChainSync mini-protocol version this state machine
/// accepts.
///
/// LocalChainSync has shipped a single closed grammar (8 messages, no
/// version-gated variants) for every cardano-node 11.0.1 (10.6.2 forward-compatible) supported
/// version. We pin the upper bound at `MAX_LOCAL_CHAIN_SYNC_VERSION`
/// so a future spec extension cannot silently transit messages whose
/// semantics this state machine has not been updated for — the
/// `InvalidForVersion` error surfaces the mismatch at the protocol
/// boundary instead of letting an unknown future variant through.
const MAX_LOCAL_CHAIN_SYNC_VERSION: u16 = 100;

/// Pure LocalChainSync transition.
///
/// `agency` is the agency the *sender* held when it produced `msg`.
/// Client-originated messages (RequestNext / FindIntersect / Done) are
/// paired with `LocalChainSyncAgency::Client`; server-originated
/// replies are paired with `Server`. Any other pairing returns
/// `IllegalTransition`.
pub fn local_chain_sync_transition(
    state: LocalChainSyncState,
    agency: LocalChainSyncAgency,
    version: LocalChainSyncVersion,
    msg: LocalChainSyncMessage,
) -> Result<(LocalChainSyncState, LocalChainSyncOutput), LocalChainSyncError> {
    if version.get() > MAX_LOCAL_CHAIN_SYNC_VERSION {
        return Err(LocalChainSyncError::InvalidForVersion {
            version,
            message_tag: message_tag(&msg),
        });
    }
    match (state, agency, msg) {
        (
            LocalChainSyncState::Idle,
            LocalChainSyncAgency::Client,
            LocalChainSyncMessage::RequestNext,
        ) => Ok((
            LocalChainSyncState::CanAwait,
            LocalChainSyncOutput::Reply(LocalChainSyncMessage::RequestNext),
        )),
        (
            LocalChainSyncState::Idle,
            LocalChainSyncAgency::Client,
            LocalChainSyncMessage::FindIntersect { points },
        ) => {
            if points.is_empty() {
                return Err(LocalChainSyncError::MalformedMessage {
                    reason: "FindIntersect carried an empty points list",
                });
            }
            Ok((
                LocalChainSyncState::Intersect,
                LocalChainSyncOutput::Reply(LocalChainSyncMessage::FindIntersect { points }),
            ))
        }
        (LocalChainSyncState::Idle, LocalChainSyncAgency::Client, LocalChainSyncMessage::Done) => {
            Ok((LocalChainSyncState::Done, LocalChainSyncOutput::Done))
        }
        (
            LocalChainSyncState::CanAwait,
            LocalChainSyncAgency::Server,
            LocalChainSyncMessage::RollForward { block, tip },
        ) => Ok((
            LocalChainSyncState::Idle,
            LocalChainSyncOutput::Event(LocalChainSyncEvent::RollForward {
                block_bytes: block,
                tip,
            }),
        )),
        (
            LocalChainSyncState::CanAwait,
            LocalChainSyncAgency::Server,
            LocalChainSyncMessage::RollBackward { point, tip },
        ) => Ok((
            LocalChainSyncState::Idle,
            LocalChainSyncOutput::Event(LocalChainSyncEvent::RollBackward { point, tip }),
        )),
        (
            LocalChainSyncState::CanAwait,
            LocalChainSyncAgency::Server,
            LocalChainSyncMessage::AwaitReply,
        ) => Ok((
            LocalChainSyncState::MustReply,
            LocalChainSyncOutput::Reply(LocalChainSyncMessage::AwaitReply),
        )),
        (
            LocalChainSyncState::MustReply,
            LocalChainSyncAgency::Server,
            LocalChainSyncMessage::RollForward { block, tip },
        ) => Ok((
            LocalChainSyncState::Idle,
            LocalChainSyncOutput::Event(LocalChainSyncEvent::RollForward {
                block_bytes: block,
                tip,
            }),
        )),
        (
            LocalChainSyncState::MustReply,
            LocalChainSyncAgency::Server,
            LocalChainSyncMessage::RollBackward { point, tip },
        ) => Ok((
            LocalChainSyncState::Idle,
            LocalChainSyncOutput::Event(LocalChainSyncEvent::RollBackward { point, tip }),
        )),
        (
            LocalChainSyncState::Intersect,
            LocalChainSyncAgency::Server,
            LocalChainSyncMessage::IntersectFound { point, tip },
        ) => Ok((
            LocalChainSyncState::Idle,
            LocalChainSyncOutput::Event(LocalChainSyncEvent::Intersected { point, tip }),
        )),
        (
            LocalChainSyncState::Intersect,
            LocalChainSyncAgency::Server,
            LocalChainSyncMessage::IntersectNotFound { tip },
        ) => Ok((
            LocalChainSyncState::Idle,
            LocalChainSyncOutput::Event(LocalChainSyncEvent::NoIntersection { tip }),
        )),
        (state, agency, msg) => Err(LocalChainSyncError::IllegalTransition {
            state,
            message_tag: message_tag(&msg),
            agency,
        }),
    }
}

fn message_tag(msg: &LocalChainSyncMessage) -> &'static str {
    match msg {
        LocalChainSyncMessage::RequestNext => "RequestNext",
        LocalChainSyncMessage::AwaitReply => "AwaitReply",
        LocalChainSyncMessage::RollForward { .. } => "RollForward",
        LocalChainSyncMessage::RollBackward { .. } => "RollBackward",
        LocalChainSyncMessage::FindIntersect { .. } => "FindIntersect",
        LocalChainSyncMessage::IntersectFound { .. } => "IntersectFound",
        LocalChainSyncMessage::IntersectNotFound { .. } => "IntersectNotFound",
        LocalChainSyncMessage::Done => "Done",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::local_chain_sync::{Point, Tip};
    use ade_types::{Hash32, SlotNo};

    fn version() -> LocalChainSyncVersion {
        LocalChainSyncVersion::new(16)
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

    fn sample_block_bytes() -> Vec<u8> {
        vec![0x82, 0x01, 0x02, 0x03, 0x04, 0xFF, 0xAA, 0xBB, 0xCC, 0xDD]
    }

    #[test]
    fn local_chain_sync_request_next_then_roll_forward() {
        // Two-step drive: Idle -> CanAwait via client RequestNext,
        // then CanAwait -> Idle + Event via server RollForward.
        let (st1, out1) = local_chain_sync_transition(
            LocalChainSyncState::Idle,
            LocalChainSyncAgency::Client,
            version(),
            LocalChainSyncMessage::RequestNext,
        )
        .expect("idle+request_next");
        assert_eq!(st1, LocalChainSyncState::CanAwait);
        match out1 {
            LocalChainSyncOutput::Reply(LocalChainSyncMessage::RequestNext) => {}
            other => panic!("expected Reply(RequestNext), got {other:?}"),
        }

        let block = sample_block_bytes();
        let tip = sample_tip();
        let (st2, out2) = local_chain_sync_transition(
            st1,
            LocalChainSyncAgency::Server,
            version(),
            LocalChainSyncMessage::RollForward {
                block: block.clone(),
                tip: tip.clone(),
            },
        )
        .expect("can_await+roll_forward");
        assert_eq!(st2, LocalChainSyncState::Idle);
        match out2 {
            LocalChainSyncOutput::Event(LocalChainSyncEvent::RollForward {
                block_bytes,
                tip: ev_tip,
            }) => {
                assert_eq!(block_bytes, block);
                assert_eq!(ev_tip, tip);
            }
            other => panic!("expected Event(RollForward), got {other:?}"),
        }
    }

    #[test]
    fn local_chain_sync_roll_backward_signal() {
        let point_in = Point::Block {
            slot: SlotNo(777),
            hash: Hash32([0xCC; 32]),
        };
        let tip_in = other_tip();
        let (st, out) = local_chain_sync_transition(
            LocalChainSyncState::CanAwait,
            LocalChainSyncAgency::Server,
            version(),
            LocalChainSyncMessage::RollBackward {
                point: point_in.clone(),
                tip: tip_in.clone(),
            },
        )
        .expect("can_await+roll_backward");
        assert_eq!(st, LocalChainSyncState::Idle);
        match out {
            LocalChainSyncOutput::Event(LocalChainSyncEvent::RollBackward {
                point,
                tip: ev_tip,
            }) => {
                assert_eq!(point, point_in);
                assert_eq!(ev_tip, tip_in);
            }
            other => panic!("expected Event(RollBackward), got {other:?}"),
        }
    }

    #[test]
    fn local_chain_sync_find_intersect_known_point() {
        // Drive: Idle -> Intersect (client FindIntersect), then
        // Intersect -> Idle + Event(Intersected) (server IntersectFound).
        let points = vec![
            Point::Origin,
            Point::Block {
                slot: SlotNo(100),
                hash: Hash32([0x11; 32]),
            },
        ];
        let (st1, out1) = local_chain_sync_transition(
            LocalChainSyncState::Idle,
            LocalChainSyncAgency::Client,
            version(),
            LocalChainSyncMessage::FindIntersect {
                points: points.clone(),
            },
        )
        .expect("idle+find_intersect");
        assert_eq!(st1, LocalChainSyncState::Intersect);
        match out1 {
            LocalChainSyncOutput::Reply(LocalChainSyncMessage::FindIntersect {
                points: replied,
            }) => {
                assert_eq!(replied, points);
            }
            other => panic!("expected Reply(FindIntersect), got {other:?}"),
        }

        let found_point = Point::Block {
            slot: SlotNo(100),
            hash: Hash32([0x11; 32]),
        };
        let tip = sample_tip();
        let (st2, out2) = local_chain_sync_transition(
            st1,
            LocalChainSyncAgency::Server,
            version(),
            LocalChainSyncMessage::IntersectFound {
                point: found_point.clone(),
                tip: tip.clone(),
            },
        )
        .expect("intersect+intersect_found");
        assert_eq!(st2, LocalChainSyncState::Idle);
        match out2 {
            LocalChainSyncOutput::Event(LocalChainSyncEvent::Intersected {
                point,
                tip: ev_tip,
            }) => {
                assert_eq!(point, found_point);
                assert_eq!(ev_tip, tip);
            }
            other => panic!("expected Event(Intersected), got {other:?}"),
        }
    }

    #[test]
    fn local_chain_sync_find_intersect_unknown() {
        let points = vec![Point::Block {
            slot: SlotNo(1),
            hash: Hash32([0x00; 32]),
        }];
        let (st1, _) = local_chain_sync_transition(
            LocalChainSyncState::Idle,
            LocalChainSyncAgency::Client,
            version(),
            LocalChainSyncMessage::FindIntersect { points },
        )
        .expect("idle+find_intersect");
        assert_eq!(st1, LocalChainSyncState::Intersect);

        let tip = other_tip();
        let (st2, out2) = local_chain_sync_transition(
            st1,
            LocalChainSyncAgency::Server,
            version(),
            LocalChainSyncMessage::IntersectNotFound { tip: tip.clone() },
        )
        .expect("intersect+intersect_not_found");
        assert_eq!(st2, LocalChainSyncState::Idle);
        match out2 {
            LocalChainSyncOutput::Event(LocalChainSyncEvent::NoIntersection { tip: ev_tip }) => {
                assert_eq!(ev_tip, tip);
            }
            other => panic!("expected Event(NoIntersection), got {other:?}"),
        }
    }

    #[test]
    fn local_chain_sync_client_done_terminates() {
        let (st, out) = local_chain_sync_transition(
            LocalChainSyncState::Idle,
            LocalChainSyncAgency::Client,
            version(),
            LocalChainSyncMessage::Done,
        )
        .expect("idle+done");
        assert_eq!(st, LocalChainSyncState::Done);
        match out {
            LocalChainSyncOutput::Done => {}
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn local_chain_sync_wrong_agency_returns_error() {
        // RequestNext is a client-originated message; passing Server
        // agency for it is grammar-illegal.
        let err = local_chain_sync_transition(
            LocalChainSyncState::Idle,
            LocalChainSyncAgency::Server,
            version(),
            LocalChainSyncMessage::RequestNext,
        )
        .expect_err("must reject");
        match err {
            LocalChainSyncError::IllegalTransition {
                message_tag,
                agency,
                ..
            } => {
                assert_eq!(message_tag, "RequestNext");
                assert_eq!(agency, LocalChainSyncAgency::Server);
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn local_chain_sync_version_gating() {
        let bogus_version = LocalChainSyncVersion::new(MAX_LOCAL_CHAIN_SYNC_VERSION + 1);
        let err = local_chain_sync_transition(
            LocalChainSyncState::Idle,
            LocalChainSyncAgency::Client,
            bogus_version,
            LocalChainSyncMessage::RequestNext,
        )
        .expect_err("must reject");
        match err {
            LocalChainSyncError::InvalidForVersion {
                version,
                message_tag,
            } => {
                assert_eq!(version, bogus_version);
                assert_eq!(message_tag, "RequestNext");
            }
            other => panic!("expected InvalidForVersion, got {other:?}"),
        }
    }
}
