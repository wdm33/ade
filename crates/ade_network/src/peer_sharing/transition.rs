// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Pure peer-sharing transition function.
//
// Shape (per slice §9):
//   fn peer_sharing_transition(state, agency, version, msg)
//       -> Result<(new_state, output), error>
//
// No async, no I/O, no ambient state, no peer-book mutation. The
// selected version is an explicit input — never read from a session
// global (DC-PROTO-06). Population of the peer book is a RED session-
// level concern in a future cluster; the state machine only emits
// `PeersShared { peers }` events.
//
// State graph (only these tuples are legal; everything else is
// IllegalTransition):
//
//   Idle         + Client + ShareRequest{amount} -> Busy{amount}     + Event(SharingRequested{amount})
//   Idle         + Client + Done                 -> Done             + Done
//   Busy{amount} + Server + SharePeers(peers)    -> Idle             + Event(PeersShared{peers})  [requires peers.len() <= amount; else MalformedMessage]
//
// Grammar invariant enforced (per Ouroboros peer-sharing spec):
//   - `SharePeers` reply count <= advertised `amount`. Empty reply is
//     legal (the server may have no peers to share).

use crate::codec::peer_sharing::PeerSharingMessage;
use crate::codec::version::PeerSharingVersion;
use crate::peer_sharing::agency::PeerSharingAgency;
use crate::peer_sharing::event::PeerSharingEvent;
use crate::peer_sharing::state::{PeerSharingError, PeerSharingOutput, PeerSharingState};

/// Highest peer-sharing mini-protocol version this state machine accepts.
///
/// Peer-sharing has shipped a single closed grammar (3 messages, no
/// version-gated variants) for every cardano-node 10.6.2 supported
/// version. We pin the upper bound at `MAX_PEER_SHARING_VERSION` so a
/// future spec extension cannot silently transit messages whose
/// semantics this state machine has not been updated for — the
/// `InvalidForVersion` error surfaces the mismatch at the protocol
/// boundary instead of letting an unknown future variant through.
const MAX_PEER_SHARING_VERSION: u16 = 100;

/// Pure peer-sharing transition.
///
/// `agency` is the agency the *sender* held when it produced `msg`.
/// Client-originated messages (ShareRequest / Done) are paired with
/// `PeerSharingAgency::Client`; the server-originated reply
/// (SharePeers) is paired with `Server`. Any other pairing returns
/// `IllegalTransition`.
pub fn peer_sharing_transition(
    state: PeerSharingState,
    agency: PeerSharingAgency,
    version: PeerSharingVersion,
    msg: PeerSharingMessage,
) -> Result<(PeerSharingState, PeerSharingOutput), PeerSharingError> {
    if version.get() > MAX_PEER_SHARING_VERSION {
        return Err(PeerSharingError::InvalidForVersion {
            version,
            message_tag: message_tag(&msg),
        });
    }
    match (state, agency, msg) {
        (
            PeerSharingState::Idle,
            PeerSharingAgency::Client,
            PeerSharingMessage::ShareRequest { amount },
        ) => Ok((
            PeerSharingState::Busy { amount },
            PeerSharingOutput::Event(PeerSharingEvent::SharingRequested { amount }),
        )),
        (
            PeerSharingState::Busy { amount },
            PeerSharingAgency::Server,
            PeerSharingMessage::SharePeers { peers },
        ) => {
            if peers.len() > amount as usize {
                return Err(PeerSharingError::MalformedMessage {
                    reason: "SharePeers count exceeds requested amount",
                });
            }
            Ok((
                PeerSharingState::Idle,
                PeerSharingOutput::Event(PeerSharingEvent::PeersShared { peers }),
            ))
        }
        (PeerSharingState::Idle, PeerSharingAgency::Client, PeerSharingMessage::Done) => {
            Ok((PeerSharingState::Done, PeerSharingOutput::Done))
        }
        (state, agency, msg) => Err(PeerSharingError::IllegalTransition {
            state,
            message_tag: message_tag(&msg),
            agency,
        }),
    }
}

fn message_tag(msg: &PeerSharingMessage) -> &'static str {
    match msg {
        PeerSharingMessage::ShareRequest { .. } => "ShareRequest",
        PeerSharingMessage::SharePeers { .. } => "SharePeers",
        PeerSharingMessage::Done => "Done",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::peer_sharing::PeerAddress;

    fn version() -> PeerSharingVersion {
        PeerSharingVersion::new(11)
    }

    fn v4(seed: u8, port: u16) -> PeerAddress {
        PeerAddress::V4 {
            addr: 0xC0A80000 | seed as u32,
            port,
        }
    }

    fn v6(seed: u8, port: u16) -> PeerAddress {
        let mut a = [0u8; 16];
        a[0] = 0x20;
        a[1] = 0x01;
        a[2] = 0x0D;
        a[3] = 0xB8;
        a[15] = seed;
        PeerAddress::V6 {
            addr: a,
            port,
            flowinfo: 0,
            scope: 0,
        }
    }

    #[test]
    fn share_request_then_full_reply_round_trips() {
        let amount: u8 = 3;
        let (st1, out1) = peer_sharing_transition(
            PeerSharingState::Idle,
            PeerSharingAgency::Client,
            version(),
            PeerSharingMessage::ShareRequest { amount },
        )
        .expect("idle+client+share_request");
        assert_eq!(st1, PeerSharingState::Busy { amount });
        match out1 {
            PeerSharingOutput::Event(PeerSharingEvent::SharingRequested { amount: a }) => {
                assert_eq!(a, amount);
            }
            other => panic!("expected Event(SharingRequested), got {other:?}"),
        }

        let peers = vec![v4(0x01, 3001), v4(0x02, 3001), v6(0x03, 3001)];
        let (st2, out2) = peer_sharing_transition(
            st1,
            PeerSharingAgency::Server,
            version(),
            PeerSharingMessage::SharePeers {
                peers: peers.clone(),
            },
        )
        .expect("busy+server+share_peers");
        assert_eq!(st2, PeerSharingState::Idle);
        match out2 {
            PeerSharingOutput::Event(PeerSharingEvent::PeersShared { peers: got }) => {
                assert_eq!(got, peers);
            }
            other => panic!("expected Event(PeersShared), got {other:?}"),
        }
    }

    #[test]
    fn share_request_then_empty_reply_is_legal() {
        // The server may have no peers to share — an empty reply is
        // grammatically legal as long as the count is <= amount.
        let (st1, _) = peer_sharing_transition(
            PeerSharingState::Idle,
            PeerSharingAgency::Client,
            version(),
            PeerSharingMessage::ShareRequest { amount: 5 },
        )
        .expect("idle+client+share_request");
        assert_eq!(st1, PeerSharingState::Busy { amount: 5 });

        let (st2, out2) = peer_sharing_transition(
            st1,
            PeerSharingAgency::Server,
            version(),
            PeerSharingMessage::SharePeers { peers: Vec::new() },
        )
        .expect("busy+server+empty_reply");
        assert_eq!(st2, PeerSharingState::Idle);
        match out2 {
            PeerSharingOutput::Event(PeerSharingEvent::PeersShared { peers }) => {
                assert!(peers.is_empty());
            }
            other => panic!("expected Event(PeersShared{{peers: []}}), got {other:?}"),
        }
    }

    #[test]
    fn reply_exceeds_amount_returns_malformed() {
        // Server cannot return more peers than the client asked for.
        let peers = vec![v4(0x01, 3001), v4(0x02, 3001), v4(0x03, 3001)];
        let err = peer_sharing_transition(
            PeerSharingState::Busy { amount: 2 },
            PeerSharingAgency::Server,
            version(),
            PeerSharingMessage::SharePeers { peers },
        )
        .expect_err("must reject overlarge reply");
        match err {
            PeerSharingError::MalformedMessage { reason } => {
                assert_eq!(reason, "SharePeers count exceeds requested amount");
            }
            other => panic!("expected MalformedMessage, got {other:?}"),
        }
    }

    #[test]
    fn client_done_terminates_session() {
        let (st, out) = peer_sharing_transition(
            PeerSharingState::Idle,
            PeerSharingAgency::Client,
            version(),
            PeerSharingMessage::Done,
        )
        .expect("idle+client+done");
        assert_eq!(st, PeerSharingState::Done);
        match out {
            PeerSharingOutput::Done => {}
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn illegal_message_in_idle_returns_error() {
        // SharePeers is server-only and arrives while Idle has the
        // agency — grammar-illegal.
        let err = peer_sharing_transition(
            PeerSharingState::Idle,
            PeerSharingAgency::Server,
            version(),
            PeerSharingMessage::SharePeers { peers: Vec::new() },
        )
        .expect_err("must reject");
        match err {
            PeerSharingError::IllegalTransition {
                state,
                message_tag,
                agency,
            } => {
                assert_eq!(state, PeerSharingState::Idle);
                assert_eq!(message_tag, "SharePeers");
                assert_eq!(agency, PeerSharingAgency::Server);
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn wrong_agency_returns_error() {
        // ShareRequest is a client-originated message; pairing it
        // with Server agency is grammar-illegal.
        let err = peer_sharing_transition(
            PeerSharingState::Idle,
            PeerSharingAgency::Server,
            version(),
            PeerSharingMessage::ShareRequest { amount: 5 },
        )
        .expect_err("must reject server+share_request");
        match err {
            PeerSharingError::IllegalTransition {
                message_tag,
                agency,
                ..
            } => {
                assert_eq!(message_tag, "ShareRequest");
                assert_eq!(agency, PeerSharingAgency::Server);
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn version_gating_rejects_out_of_version_message() {
        // The peer-sharing wire grammar across cardano-node 10.6.2
        // has shipped a single closed message set for every supported
        // version, so there is no real per-variant version gating
        // yet. The state machine still has to expose the
        // InvalidForVersion error path because the type signature
        // commits to it; the pinned guard rejects future versions
        // above MAX_PEER_SHARING_VERSION = 100.
        let bogus_version = PeerSharingVersion::new(MAX_PEER_SHARING_VERSION + 1);
        let err = peer_sharing_transition(
            PeerSharingState::Idle,
            PeerSharingAgency::Client,
            bogus_version,
            PeerSharingMessage::ShareRequest { amount: 5 },
        )
        .expect_err("must reject");
        match err {
            PeerSharingError::InvalidForVersion {
                version,
                message_tag,
            } => {
                assert_eq!(version, bogus_version);
                assert_eq!(message_tag, "ShareRequest");
            }
            other => panic!("expected InvalidForVersion, got {other:?}"),
        }
    }
}
