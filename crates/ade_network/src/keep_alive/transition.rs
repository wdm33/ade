// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Pure keep-alive transition function.
//
// Shape (per slice §9):
//   fn keep_alive_transition(state, agency, version, msg)
//       -> Result<(new_state, output), error>
//
// No async, no I/O, no ambient state, no wall-clock. The selected
// version is an explicit input — never read from a session global
// (DC-PROTO-06). The cookie carried in `ServerHasAgency` is a 16-bit
// nonce, not a timestamp; the session layer (RED) is responsible for
// latency accounting.
//
// State graph (only these tuples are legal; everything else is
// IllegalTransition):
//
//   ClientIdle              + Client + KeepAlive(cookie)            -> ServerHasAgency{cookie} + Event(PingSent{cookie})
//   ClientIdle              + Client + Done                         -> Done                    + Done
//   ServerHasAgency{cookie} + Server + ResponseKeepAlive(c')        -> ClientIdle              + Event(PongReceived{cookie})  [requires c' == cookie; else MalformedMessage]

use crate::codec::keep_alive::KeepAliveMessage;
use crate::codec::version::KeepAliveVersion;
use crate::keep_alive::agency::KeepAliveAgency;
use crate::keep_alive::event::KeepAliveEvent;
use crate::keep_alive::state::{KeepAliveError, KeepAliveOutput, KeepAliveState};

/// Highest keep-alive mini-protocol version this state machine accepts.
///
/// Keep-alive has shipped a single closed grammar (3 messages, no
/// version-gated variants) for every cardano-node 11.0.1 (10.6.2 forward-compatible) supported
/// version. We pin the upper bound at `MAX_KEEP_ALIVE_VERSION` so a
/// future spec extension cannot silently transit messages whose
/// semantics this state machine has not been updated for — the
/// `InvalidForVersion` error surfaces the mismatch at the protocol
/// boundary instead of letting an unknown future variant through.
const MAX_KEEP_ALIVE_VERSION: u16 = 100;

/// Pure keep-alive transition.
///
/// `agency` is the agency the *sender* held when it produced `msg`.
/// Client-originated messages (KeepAlive / Done) are paired with
/// `KeepAliveAgency::Client`; the server-originated reply
/// (ResponseKeepAlive) is paired with `Server`. Any other pairing
/// returns `IllegalTransition`.
pub fn keep_alive_transition(
    state: KeepAliveState,
    agency: KeepAliveAgency,
    version: KeepAliveVersion,
    msg: KeepAliveMessage,
) -> Result<(KeepAliveState, KeepAliveOutput), KeepAliveError> {
    if version.get() > MAX_KEEP_ALIVE_VERSION {
        return Err(KeepAliveError::InvalidForVersion {
            version,
            message_tag: message_tag(&msg),
        });
    }
    match (state, agency, msg) {
        (
            KeepAliveState::ClientIdle,
            KeepAliveAgency::Client,
            KeepAliveMessage::KeepAlive(cookie),
        ) => Ok((
            KeepAliveState::ServerHasAgency { cookie },
            KeepAliveOutput::Event(KeepAliveEvent::PingSent { cookie }),
        )),
        (
            KeepAliveState::ServerHasAgency { cookie },
            KeepAliveAgency::Server,
            KeepAliveMessage::ResponseKeepAlive(resp_cookie),
        ) => {
            if cookie != resp_cookie {
                return Err(KeepAliveError::MalformedMessage {
                    reason: "ResponseKeepAlive cookie does not match request",
                });
            }
            Ok((
                KeepAliveState::ClientIdle,
                KeepAliveOutput::Event(KeepAliveEvent::PongReceived { cookie }),
            ))
        }
        (KeepAliveState::ClientIdle, KeepAliveAgency::Client, KeepAliveMessage::Done) => {
            Ok((KeepAliveState::Done, KeepAliveOutput::Done))
        }
        (state, agency, msg) => Err(KeepAliveError::IllegalTransition {
            state,
            message_tag: message_tag(&msg),
            agency,
        }),
    }
}

fn message_tag(msg: &KeepAliveMessage) -> &'static str {
    match msg {
        KeepAliveMessage::KeepAlive(_) => "KeepAlive",
        KeepAliveMessage::ResponseKeepAlive(_) => "ResponseKeepAlive",
        KeepAliveMessage::Done => "Done",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::keep_alive::KeepAliveCookie;

    fn version() -> KeepAliveVersion {
        KeepAliveVersion::new(9)
    }

    #[test]
    fn client_ping_then_server_pong_round_trips() {
        let cookie = KeepAliveCookie(0xBEEF);
        let (st1, out1) = keep_alive_transition(
            KeepAliveState::ClientIdle,
            KeepAliveAgency::Client,
            version(),
            KeepAliveMessage::KeepAlive(cookie),
        )
        .expect("idle+client+keep_alive");
        assert_eq!(st1, KeepAliveState::ServerHasAgency { cookie });
        match out1 {
            KeepAliveOutput::Event(KeepAliveEvent::PingSent { cookie: c }) => {
                assert_eq!(c, cookie);
            }
            other => panic!("expected Event(PingSent), got {other:?}"),
        }

        let (st2, out2) = keep_alive_transition(
            st1,
            KeepAliveAgency::Server,
            version(),
            KeepAliveMessage::ResponseKeepAlive(cookie),
        )
        .expect("server_has_agency+server+response");
        assert_eq!(st2, KeepAliveState::ClientIdle);
        match out2 {
            KeepAliveOutput::Event(KeepAliveEvent::PongReceived { cookie: c }) => {
                assert_eq!(c, cookie);
            }
            other => panic!("expected Event(PongReceived), got {other:?}"),
        }
    }

    #[test]
    fn cookie_mismatch_returns_malformed() {
        let req = KeepAliveCookie(0x1111);
        let bogus = KeepAliveCookie(0x2222);
        let err = keep_alive_transition(
            KeepAliveState::ServerHasAgency { cookie: req },
            KeepAliveAgency::Server,
            version(),
            KeepAliveMessage::ResponseKeepAlive(bogus),
        )
        .expect_err("must reject mismatched cookie");
        match err {
            KeepAliveError::MalformedMessage { reason } => {
                assert_eq!(reason, "ResponseKeepAlive cookie does not match request");
            }
            other => panic!("expected MalformedMessage, got {other:?}"),
        }
    }

    #[test]
    fn client_done_terminates_session() {
        let (st, out) = keep_alive_transition(
            KeepAliveState::ClientIdle,
            KeepAliveAgency::Client,
            version(),
            KeepAliveMessage::Done,
        )
        .expect("idle+client+done");
        assert_eq!(st, KeepAliveState::Done);
        match out {
            KeepAliveOutput::Done => {}
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn illegal_message_in_idle_returns_error() {
        // ResponseKeepAlive is server-only and arrives while
        // ClientIdle has the agency — grammar-illegal.
        let err = keep_alive_transition(
            KeepAliveState::ClientIdle,
            KeepAliveAgency::Server,
            version(),
            KeepAliveMessage::ResponseKeepAlive(KeepAliveCookie(7)),
        )
        .expect_err("must reject");
        match err {
            KeepAliveError::IllegalTransition {
                state,
                message_tag,
                agency,
            } => {
                assert_eq!(state, KeepAliveState::ClientIdle);
                assert_eq!(message_tag, "ResponseKeepAlive");
                assert_eq!(agency, KeepAliveAgency::Server);
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn wrong_agency_returns_error() {
        // KeepAlive is a client-originated message; pairing it with
        // Server agency is grammar-illegal.
        let err = keep_alive_transition(
            KeepAliveState::ClientIdle,
            KeepAliveAgency::Server,
            version(),
            KeepAliveMessage::KeepAlive(KeepAliveCookie(1)),
        )
        .expect_err("must reject server+keep_alive");
        match err {
            KeepAliveError::IllegalTransition {
                message_tag,
                agency,
                ..
            } => {
                assert_eq!(message_tag, "KeepAlive");
                assert_eq!(agency, KeepAliveAgency::Server);
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn version_gating_rejects_out_of_version_message() {
        // The keep-alive wire grammar across cardano-node 11.0.1 (10.6.2 forward-compatible) has
        // shipped a single closed message set for every supported
        // version, so there is no real per-variant version gating
        // yet. The state machine still has to expose the
        // InvalidForVersion error path because the type signature
        // commits to it; the pinned guard rejects future versions
        // above MAX_KEEP_ALIVE_VERSION = 100.
        let bogus_version = KeepAliveVersion::new(MAX_KEEP_ALIVE_VERSION + 1);
        let err = keep_alive_transition(
            KeepAliveState::ClientIdle,
            KeepAliveAgency::Client,
            bogus_version,
            KeepAliveMessage::KeepAlive(KeepAliveCookie(0)),
        )
        .expect_err("must reject");
        match err {
            KeepAliveError::InvalidForVersion {
                version,
                message_tag,
            } => {
                assert_eq!(version, bogus_version);
                assert_eq!(message_tag, "KeepAlive");
            }
            other => panic!("expected InvalidForVersion, got {other:?}"),
        }
    }
}
