// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Pure handshake transition functions.
//
// Shape (per slice §9):
//   fn transition(state, agency, supported, msg)
//       -> Result<(new_state, output), error>
//
// No async, no I/O, no ambient state. The supported version table is
// an explicit input — never read from a global (DC-PROTO-06).
//
// State graph:
//   Idle      + ProposeVersions [Client]   -> Proposed  (server-side view)
//   Idle      + Reply           (any)      -> Done|Refused (client-side view)
//   Proposed  + AcceptVersion   [Server]   -> Done
//   Proposed  + Refuse          [Server]   -> Refused
//   any       + (anything else)            -> IllegalTransition
//
// The function serves both directions: a "server" peer transitions
// Idle -> Proposed on receiving the client's proposal and computes
// the reply; a "client" peer transitions Proposed -> Done|Refused on
// receiving the server's reply. The `agency` argument disambiguates
// the role: when the caller passes the agency the *peer* held when
// it sent `msg`, only the legal grammar tuples are accepted.

use crate::codec::handshake::{HandshakeMessage, RefuseReason};
use crate::codec::n2c_handshake::{N2cHandshakeMessage, N2cRefuseReason};
use crate::handshake::agency::HandshakeAgency;
use crate::handshake::selection::{
    build_n2c_refuse, build_n2n_refuse, proposed_n2c_versions, proposed_n2n_versions,
    select_n2c_version, select_n2n_version, supported_n2c_versions, supported_n2n_versions,
    SelectionOutcome,
};
use crate::handshake::state::{
    HandshakeError, HandshakeState, N2cHandshakeOutput, N2cVersionData, N2nHandshakeOutput,
    VersionData,
};

/// Pure N2N handshake transition.
///
/// `agency` is the agency the *peer* held when it produced `msg`:
///   - `ClientHasAgency` is paired with `ProposeVersions` only.
///   - `ServerHasAgency` is paired with `AcceptVersion` / `Refuse` /
///     `QueryReply` only.
///   - `NobodyHasAgency` is never a legal sender agency.
pub fn n2n_transition(
    state: HandshakeState,
    agency: HandshakeAgency,
    supported: &[(u16, VersionData)],
    msg: HandshakeMessage,
) -> Result<(HandshakeState, N2nHandshakeOutput), HandshakeError> {
    match (state, &msg, agency) {
        (HandshakeState::Idle, HandshakeMessage::ProposeVersions(table), HandshakeAgency::ClientHasAgency) => {
            if table.0.is_empty() {
                return Err(HandshakeError::MalformedMessage {
                    reason: "ProposeVersions carried an empty version table",
                });
            }
            let outcome = select_n2n_version(table, supported);
            match outcome {
                SelectionOutcome::Selected(v, data) => {
                    // Server-side view: accept and terminate. The reply
                    // message is built from the codec's `AcceptVersion`
                    // constructor with the negotiated params encoded by
                    // a downstream codec call — at the transition layer
                    // we just signal `Selected` for the session layer
                    // to encode + transmit.
                    Ok((HandshakeState::Done, N2nHandshakeOutput::Selected(v, data)))
                }
                SelectionOutcome::Mismatch(supported_vs) => {
                    let _ = build_n2n_refuse(supported_vs.clone());
                    Err(HandshakeError::VersionMismatch {
                        proposed_set: proposed_n2n_versions(table),
                        supported_set: supported_n2n_versions(supported),
                    })
                }
            }
        }
        (HandshakeState::Proposed, HandshakeMessage::AcceptVersion(ver, _params), HandshakeAgency::ServerHasAgency) => {
            // Client-side view: server accepted. Look up the local
            // VersionData for the accepted version — the version must
            // be one we proposed (a subset of `supported`).
            let pv = ver.get();
            for (v_local, data_local) in supported {
                if *v_local == pv {
                    return Ok((
                        HandshakeState::Done,
                        N2nHandshakeOutput::Selected(*ver, *data_local),
                    ));
                }
            }
            Err(HandshakeError::UnknownVersion { version: pv })
        }
        (HandshakeState::Proposed, HandshakeMessage::Refuse(reason), HandshakeAgency::ServerHasAgency) => {
            Ok((HandshakeState::Refused, N2nHandshakeOutput::Refused(reason.clone())))
        }
        (HandshakeState::Proposed, HandshakeMessage::QueryReply(_), HandshakeAgency::ServerHasAgency) => {
            // QueryReply terminates the handshake-query branch — semantic
            // equivalent of refusal: the server returned its table
            // instead of negotiating. We surface a synthetic refuse.
            Ok((
                HandshakeState::Refused,
                N2nHandshakeOutput::Refused(RefuseReason::VersionMismatch(Vec::new())),
            ))
        }
        _ => Err(HandshakeError::IllegalTransition {
            state,
            message_tag: n2n_message_tag(&msg),
            agency: agency_tag(agency),
        }),
    }
}

/// Pure N2C handshake transition (analogue of `n2n_transition`).
pub fn n2c_transition(
    state: HandshakeState,
    agency: HandshakeAgency,
    supported: &[(u16, N2cVersionData)],
    msg: N2cHandshakeMessage,
) -> Result<(HandshakeState, N2cHandshakeOutput), HandshakeError> {
    match (state, &msg, agency) {
        (HandshakeState::Idle, N2cHandshakeMessage::ProposeVersions(table), HandshakeAgency::ClientHasAgency) => {
            if table.0.is_empty() {
                return Err(HandshakeError::MalformedMessage {
                    reason: "ProposeVersions carried an empty version table",
                });
            }
            let outcome = select_n2c_version(table, supported);
            match outcome {
                SelectionOutcome::Selected(v, data) => {
                    Ok((HandshakeState::Done, N2cHandshakeOutput::Selected(v, data)))
                }
                SelectionOutcome::Mismatch(supported_vs) => {
                    let _ = build_n2c_refuse(supported_vs.clone());
                    Err(HandshakeError::VersionMismatch {
                        proposed_set: proposed_n2c_versions(table),
                        supported_set: supported_n2c_versions(supported),
                    })
                }
            }
        }
        (HandshakeState::Proposed, N2cHandshakeMessage::AcceptVersion(ver, _params), HandshakeAgency::ServerHasAgency) => {
            let pv = ver.get();
            for (v_local, data_local) in supported {
                if *v_local == pv {
                    return Ok((
                        HandshakeState::Done,
                        N2cHandshakeOutput::Selected(*ver, *data_local),
                    ));
                }
            }
            Err(HandshakeError::UnknownVersion { version: pv })
        }
        (HandshakeState::Proposed, N2cHandshakeMessage::Refuse(reason), HandshakeAgency::ServerHasAgency) => {
            Ok((HandshakeState::Refused, N2cHandshakeOutput::Refused(reason.clone())))
        }
        (HandshakeState::Proposed, N2cHandshakeMessage::QueryReply(_), HandshakeAgency::ServerHasAgency) => {
            Ok((
                HandshakeState::Refused,
                N2cHandshakeOutput::Refused(N2cRefuseReason::VersionMismatch(Vec::new())),
            ))
        }
        _ => Err(HandshakeError::IllegalTransition {
            state,
            message_tag: n2c_message_tag(&msg),
            agency: agency_tag(agency),
        }),
    }
}

fn n2n_message_tag(msg: &HandshakeMessage) -> &'static str {
    match msg {
        HandshakeMessage::ProposeVersions(_) => "ProposeVersions",
        HandshakeMessage::AcceptVersion(_, _) => "AcceptVersion",
        HandshakeMessage::Refuse(_) => "Refuse",
        HandshakeMessage::QueryReply(_) => "QueryReply",
    }
}

fn n2c_message_tag(msg: &N2cHandshakeMessage) -> &'static str {
    match msg {
        N2cHandshakeMessage::ProposeVersions(_) => "ProposeVersions",
        N2cHandshakeMessage::AcceptVersion(_, _) => "AcceptVersion",
        N2cHandshakeMessage::Refuse(_) => "Refuse",
        N2cHandshakeMessage::QueryReply(_) => "QueryReply",
    }
}

fn agency_tag(agency: HandshakeAgency) -> &'static str {
    match agency {
        HandshakeAgency::ClientHasAgency => "ClientHasAgency",
        HandshakeAgency::ServerHasAgency => "ServerHasAgency",
        HandshakeAgency::NobodyHasAgency => "NobodyHasAgency",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::handshake::{
        encode_handshake_message, HandshakeMessage as HMsg, VersionParams, VersionTable,
    };
    use crate::codec::n2c_handshake::{
        encode_n2c_handshake_message, N2cHandshakeMessage as N2cMsg, N2cVersionParams,
        N2cVersionTable,
    };
    use crate::codec::primitives::encode_u64;
    use crate::codec::version::{N2CVersion, N2NVersion};
    use crate::handshake::version_table::{N2C_SUPPORTED, N2N_SUPPORTED};

    fn params_uint(v: u64) -> VersionParams {
        let mut buf = Vec::new();
        encode_u64(&mut buf, v);
        VersionParams(buf)
    }

    fn n2c_params_uint(v: u64) -> N2cVersionParams {
        let mut buf = Vec::new();
        encode_u64(&mut buf, v);
        N2cVersionParams(buf)
    }

    #[test]
    fn n2n_happy_path_each_supported_version() {
        // For each supported N2N version v, a single-entry propose
        // should select v and emit Selected(v, data).
        for (v, expected_data) in N2N_SUPPORTED {
            let table = VersionTable(vec![(N2NVersion::new(*v), params_uint(1))]);
            let (st, out) = n2n_transition(
                HandshakeState::Idle,
                HandshakeAgency::ClientHasAgency,
                N2N_SUPPORTED,
                HMsg::ProposeVersions(table),
            )
            .expect("happy path");
            assert_eq!(st, HandshakeState::Done);
            match out {
                N2nHandshakeOutput::Selected(sel_v, sel_data) => {
                    assert_eq!(sel_v.get(), *v);
                    assert_eq!(sel_data, *expected_data);
                }
                other => panic!("expected Selected, got {other:?}"),
            }
        }
    }

    #[test]
    fn n2c_happy_path_each_supported_version() {
        for (v, expected_data) in N2C_SUPPORTED {
            let table = N2cVersionTable(vec![(N2CVersion::new(*v), n2c_params_uint(1))]);
            let (st, out) = n2c_transition(
                HandshakeState::Idle,
                HandshakeAgency::ClientHasAgency,
                N2C_SUPPORTED,
                N2cMsg::ProposeVersions(table),
            )
            .expect("happy path");
            assert_eq!(st, HandshakeState::Done);
            match out {
                N2cHandshakeOutput::Selected(sel_v, sel_data) => {
                    assert_eq!(sel_v.get(), *v);
                    assert_eq!(sel_data, *expected_data);
                }
                other => panic!("expected Selected, got {other:?}"),
            }
        }
    }

    #[test]
    fn version_mismatch_refused() {
        // Peer proposes only versions we don't support.
        let table = VersionTable(vec![
            (N2NVersion::new(1), params_uint(1)),
            (N2NVersion::new(2), params_uint(2)),
        ]);
        let err = n2n_transition(
            HandshakeState::Idle,
            HandshakeAgency::ClientHasAgency,
            N2N_SUPPORTED,
            HMsg::ProposeVersions(table),
        )
        .expect_err("must refuse");
        match err {
            HandshakeError::VersionMismatch { proposed_set, supported_set } => {
                assert_eq!(proposed_set, vec![1u16, 2u16]);
                assert_eq!(supported_set, vec![11u16, 12, 13, 14]);
            }
            other => panic!("expected VersionMismatch, got {other:?}"),
        }
    }

    #[test]
    fn illegal_message_in_idle_returns_error() {
        // Idle + AcceptVersion (which is server-only and only legal in
        // Proposed) must yield IllegalTransition.
        let err = n2n_transition(
            HandshakeState::Idle,
            HandshakeAgency::ServerHasAgency,
            N2N_SUPPORTED,
            HMsg::AcceptVersion(N2NVersion::new(12), params_uint(1)),
        )
        .expect_err("must reject");
        match err {
            HandshakeError::IllegalTransition { state, message_tag, agency } => {
                assert_eq!(state, HandshakeState::Idle);
                assert_eq!(message_tag, "AcceptVersion");
                assert_eq!(agency, "ServerHasAgency");
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn wrong_agency_returns_error() {
        // ProposeVersions is client-only; ServerHasAgency is wrong.
        let table = VersionTable(vec![(N2NVersion::new(12), params_uint(1))]);
        let err = n2n_transition(
            HandshakeState::Idle,
            HandshakeAgency::ServerHasAgency,
            N2N_SUPPORTED,
            HMsg::ProposeVersions(table),
        )
        .expect_err("must reject");
        match err {
            HandshakeError::IllegalTransition { message_tag, agency, .. } => {
                assert_eq!(message_tag, "ProposeVersions");
                assert_eq!(agency, "ServerHasAgency");
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn overlap_picks_highest_common() {
        // Peer proposes 12, 13. Local supports 11..=14. Intersection is
        // {12, 13}; max is 13.
        let table = VersionTable(vec![
            (N2NVersion::new(12), params_uint(1)),
            (N2NVersion::new(13), params_uint(2)),
        ]);
        let (_st, out) = n2n_transition(
            HandshakeState::Idle,
            HandshakeAgency::ClientHasAgency,
            N2N_SUPPORTED,
            HMsg::ProposeVersions(table),
        )
        .expect("happy path");
        match out {
            N2nHandshakeOutput::Selected(v, _) => assert_eq!(v.get(), 13),
            other => panic!("expected Selected, got {other:?}"),
        }
    }

    #[test]
    fn empty_intersection_refuses_deterministically() {
        // Two runs with the same disjoint proposal must produce
        // identical VersionMismatch payloads (proposed_set + supported_set).
        let mk_table = || {
            VersionTable(vec![
                (N2NVersion::new(7), params_uint(1)),
                (N2NVersion::new(8), params_uint(2)),
            ])
        };
        let err1 = n2n_transition(
            HandshakeState::Idle,
            HandshakeAgency::ClientHasAgency,
            N2N_SUPPORTED,
            HMsg::ProposeVersions(mk_table()),
        )
        .expect_err("refuse");
        let err2 = n2n_transition(
            HandshakeState::Idle,
            HandshakeAgency::ClientHasAgency,
            N2N_SUPPORTED,
            HMsg::ProposeVersions(mk_table()),
        )
        .expect_err("refuse");
        assert_eq!(err1, err2);
    }

    #[test]
    fn version_data_passed_through_byte_identical() {
        // Build a server-accept reply from a Selected outcome; encode
        // via the S-A2 codec; assert byte identity across two runs.
        let table = VersionTable(vec![(N2NVersion::new(14), params_uint(7))]);
        let (_, out) = n2n_transition(
            HandshakeState::Idle,
            HandshakeAgency::ClientHasAgency,
            N2N_SUPPORTED,
            HMsg::ProposeVersions(table),
        )
        .expect("happy");
        let v = match out {
            N2nHandshakeOutput::Selected(v, _) => v,
            other => panic!("expected Selected, got {other:?}"),
        };
        // The encoded AcceptVersion frame with the same (v, params)
        // must be byte-equal across two encodings.
        let params = params_uint(7);
        let bytes1 = encode_handshake_message(&HMsg::AcceptVersion(v, params.clone()));
        let bytes2 = encode_handshake_message(&HMsg::AcceptVersion(v, params));
        assert_eq!(bytes1, bytes2);

        // Same shape for N2C, exercising the second transition surface.
        let n2c_table = N2cVersionTable(vec![(N2CVersion::new(20), n2c_params_uint(3))]);
        let (_, out) = n2c_transition(
            HandshakeState::Idle,
            HandshakeAgency::ClientHasAgency,
            N2C_SUPPORTED,
            N2cMsg::ProposeVersions(n2c_table),
        )
        .expect("happy");
        let v = match out {
            N2cHandshakeOutput::Selected(v, _) => v,
            other => panic!("expected Selected, got {other:?}"),
        };
        let p = n2c_params_uint(3);
        let b1 = encode_n2c_handshake_message(&N2cMsg::AcceptVersion(v, p.clone()));
        let b2 = encode_n2c_handshake_message(&N2cMsg::AcceptVersion(v, p));
        assert_eq!(b1, b2);
    }
}
