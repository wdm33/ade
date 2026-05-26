// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN session reducer (PHASE4-N-L S2).
//!
//! `step` is the SOLE pub fn reducing `(SessionState, ByteChunkIn)
//! -> Result<(SessionState', Vec<SessionEffect>), SessionError>`
//! (CN-SESS-03). Composes:
//!   - `mux::frame::{encode,decode}_frame` (CN-SESS-01)
//!   - `handshake::n2n_transition` (CN-SESS-02)
//!   - `session::demux::FrameBuffer` (S3)
//!   - the closed `AcceptedMiniProtocol` registry (DC-SESS-02)
//!
//! Type-state (DC-SESS-01): `Handshaking` cannot deliver any
//! mini-protocol frame to the orchestrator inbox — only id=0
//! (handshake) frames are legal there. Once the handshake reaches
//! `Done`, the state transitions to `Connected` and mini-protocol
//! dispatch becomes live.

use crate::codec::handshake::{decode_handshake_message, encode_handshake_message, HandshakeMessage};
use crate::handshake::agency::HandshakeAgency;
use crate::handshake::state::{HandshakeState, N2nHandshakeOutput};
use crate::handshake::transition::n2n_transition;
use crate::handshake::version_table::N2N_SUPPORTED;
use crate::mux::frame::{
    encode_frame, MiniProtocolId, MuxFrame, MuxHeader, MuxMode, MAX_PAYLOAD,
};

use super::event::{
    AcceptedMiniProtocol, ByteChunkIn, HandshakeRole, SessionEffect, SessionError,
};
use super::state::{ConnectedState, SessionState};

/// One step of the session reducer. Pure: same `(state, event)`
/// inputs → same `Vec<SessionEffect>` output across runs.
pub fn step(
    state: &mut SessionState,
    event: ByteChunkIn,
) -> Result<Vec<SessionEffect>, SessionError> {
    match event {
        ByteChunkIn::Inbound(bytes) => handle_inbound(state, &bytes),
        ByteChunkIn::OutboundFrame {
            mini_protocol,
            payload,
            mode,
            timestamp,
        } => handle_outbound(state, mini_protocol, payload, mode, timestamp),
        ByteChunkIn::HandshakeStartInitiator { proposal } => {
            handle_handshake_start_initiator(state, proposal)
        }
    }
}

fn handle_inbound(
    state: &mut SessionState,
    bytes: &[u8],
) -> Result<Vec<SessionEffect>, SessionError> {
    match state {
        SessionState::Handshaking(progress) => {
            progress.buffer.append(bytes);
            drain_handshake_frames(state)
        }
        SessionState::Connected(connected) => {
            connected.buffer.append(bytes);
            drain_connected_frames(connected)
        }
    }
}

fn drain_handshake_frames(
    state: &mut SessionState,
) -> Result<Vec<SessionEffect>, SessionError> {
    let mut effects = Vec::new();
    loop {
        let frame = {
            let SessionState::Handshaking(p) = state else {
                // We transitioned to Connected mid-loop; stop here.
                break;
            };
            match p
                .buffer
                .pull_one_frame()
                .map_err(SessionError::Mux)?
            {
                Some(f) => f,
                None => break,
            }
        };
        let id = frame.header.mini_protocol_id.get();
        if id != AcceptedMiniProtocol::HANDSHAKE_ID {
            // Closed-registry check happens before pre-handshake check:
            // an unknown id is peer-fatal regardless of state.
            return Err(if AcceptedMiniProtocol::from_id(id).is_none() {
                SessionError::UnknownMiniProtocolId { id }
            } else {
                SessionError::PreHandshakeMiniProtocolFrame { id }
            });
        }
        let msg = decode_handshake_message(&frame.payload).map_err(|_| {
            SessionError::Handshake(
                crate::handshake::state::HandshakeError::MalformedMessage {
                    reason: "handshake frame failed decode",
                },
            )
        })?;
        let (role, hs_state) = {
            let SessionState::Handshaking(p) = state else {
                unreachable!()
            };
            (p.role, p.inner)
        };
        // Agency selection mirrors the existing transition function's
        // server/client view.
        let agency = match (role, hs_state) {
            (HandshakeRole::Responder, HandshakeState::Idle) => {
                HandshakeAgency::ClientHasAgency
            }
            (HandshakeRole::Initiator, HandshakeState::Proposed) => {
                HandshakeAgency::ServerHasAgency
            }
            _ => HandshakeAgency::NobodyHasAgency,
        };
        let (new_state, output) = n2n_transition(hs_state, agency, N2N_SUPPORTED, msg.clone())
            .map_err(SessionError::Handshake)?;
        // Pin the state-machine advance.
        {
            let SessionState::Handshaking(p) = state else {
                unreachable!()
            };
            p.inner = new_state;
        }
        match output {
            N2nHandshakeOutput::Reply(reply) => {
                let reply_bytes = encode_handshake_message(&reply);
                effects.push(SessionEffect::SendBytes(encode_inner_frame(
                    AcceptedMiniProtocol::Handshake,
                    reply_bytes,
                    MuxMode::Responder,
                    0,
                )?));
            }
            N2nHandshakeOutput::Selected(version, params) => {
                // Responder side: send AcceptVersion before transitioning.
                if matches!(role, HandshakeRole::Responder) {
                    let accept = HandshakeMessage::AcceptVersion(
                        version,
                        crate::codec::handshake::VersionParams(vec![0x01]),
                    );
                    let accept_bytes = encode_handshake_message(&accept);
                    effects.push(SessionEffect::SendBytes(encode_inner_frame(
                        AcceptedMiniProtocol::Handshake,
                        accept_bytes,
                        MuxMode::Responder,
                        0,
                    )?));
                }
                effects.push(SessionEffect::HandshakeComplete {
                    version: version.get(),
                    params,
                });
                // Transition to Connected; this is a one-shot replace.
                *state = SessionState::Connected(ConnectedState::new(version.get(), params));
                // We must drop the rest of this iteration since the
                // outer loop's `let SessionState::Handshaking` guard
                // would now fail; `break` cleanly.
                break;
            }
            N2nHandshakeOutput::Refused(_)
            | N2nHandshakeOutput::Done => {
                // Handshake terminated without a usable version; this is
                // peer-fatal at the session layer.
                return Err(SessionError::Handshake(
                    crate::handshake::state::HandshakeError::MalformedMessage {
                        reason: "handshake refused",
                    },
                ));
            }
        }
    }
    Ok(effects)
}

fn drain_connected_frames(
    connected: &mut ConnectedState,
) -> Result<Vec<SessionEffect>, SessionError> {
    let mut effects = Vec::new();
    loop {
        let frame = match connected
            .buffer
            .pull_one_frame()
            .map_err(SessionError::Mux)?
        {
            Some(f) => f,
            None => break,
        };
        let id = frame.header.mini_protocol_id.get();
        // Closed dispatch — DC-SESS-02.
        let proto = match AcceptedMiniProtocol::from_id(id) {
            Some(p) => p,
            None => return Err(SessionError::UnknownMiniProtocolId { id }),
        };
        match proto {
            AcceptedMiniProtocol::Handshake => {
                // Post-handshake handshake frame is peer-fatal.
                return Err(SessionError::PostHandshakeHandshakeFrame);
            }
            AcceptedMiniProtocol::ChainSync
            | AcceptedMiniProtocol::BlockFetch
            | AcceptedMiniProtocol::TxSubmission
            | AcceptedMiniProtocol::LocalChainSync
            | AcceptedMiniProtocol::LocalTxSubmission
            | AcceptedMiniProtocol::LocalStateQuery
            | AcceptedMiniProtocol::KeepAlive
            | AcceptedMiniProtocol::LocalTxMonitor
            | AcceptedMiniProtocol::PeerSharing => {
                effects.push(SessionEffect::DeliverPeerFrame {
                    mini_protocol: proto,
                    payload: frame.payload,
                });
            }
        }
    }
    Ok(effects)
}

fn handle_outbound(
    state: &mut SessionState,
    mini_protocol: AcceptedMiniProtocol,
    payload: Vec<u8>,
    mode: MuxMode,
    timestamp: u32,
) -> Result<Vec<SessionEffect>, SessionError> {
    if payload.len() > MAX_PAYLOAD {
        return Err(SessionError::OutboundPayloadTooLarge { len: payload.len() });
    }
    if let SessionState::Connected(c) = state {
        c.next_outbound_seq = c.next_outbound_seq.wrapping_add(1);
    }
    // Handshake frames are legal in both states.
    if !state.is_connected() && !matches!(mini_protocol, AcceptedMiniProtocol::Handshake) {
        return Err(SessionError::PreHandshakeMiniProtocolFrame {
            id: mini_protocol.id(),
        });
    }
    let bytes = encode_inner_frame(mini_protocol, payload, mode, timestamp)?;
    Ok(vec![SessionEffect::SendBytes(bytes)])
}

fn handle_handshake_start_initiator(
    state: &mut SessionState,
    proposal: HandshakeMessage,
) -> Result<Vec<SessionEffect>, SessionError> {
    let SessionState::Handshaking(p) = state else {
        return Err(SessionError::PostHandshakeHandshakeFrame);
    };
    if !matches!(p.role, HandshakeRole::Initiator) {
        return Err(SessionError::Handshake(
            crate::handshake::state::HandshakeError::MalformedMessage {
                reason: "responder cannot start initiator handshake",
            },
        ));
    }
    if p.proposal_sent {
        return Err(SessionError::Handshake(
            crate::handshake::state::HandshakeError::MalformedMessage {
                reason: "initiator already sent proposal",
            },
        ));
    }
    let proposal_bytes = encode_handshake_message(&proposal);
    let frame_bytes = encode_inner_frame(
        AcceptedMiniProtocol::Handshake,
        proposal_bytes,
        MuxMode::Initiator,
        0,
    )?;
    p.inner = HandshakeState::Proposed;
    p.proposal_sent = true;
    Ok(vec![SessionEffect::SendBytes(frame_bytes)])
}

fn encode_inner_frame(
    mini_protocol: AcceptedMiniProtocol,
    payload: Vec<u8>,
    mode: MuxMode,
    timestamp: u32,
) -> Result<Vec<u8>, SessionError> {
    if payload.len() > MAX_PAYLOAD {
        return Err(SessionError::OutboundPayloadTooLarge { len: payload.len() });
    }
    let length = payload.len() as u16;
    let frame = MuxFrame {
        header: MuxHeader {
            timestamp,
            mode,
            mini_protocol_id: MiniProtocolId::new(mini_protocol.id()).expect("closed registry id"),
            length,
        },
        payload,
    };
    encode_frame(&frame).map_err(SessionError::Mux)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use crate::codec::handshake::{
        encode_handshake_message, HandshakeMessage, VersionParams,
    };
    use crate::codec::N2NVersion;
    use crate::handshake::state::{PeerSharingFlag, VersionData};
    use crate::handshake::version_table::{MAINNET_NETWORK_MAGIC, N2N_SUPPORTED};
    use crate::mux::frame::{decode_frame, MiniProtocolId, MuxFrame, MuxHeader};

    fn wrap_handshake_frame(msg: &HandshakeMessage, mode: MuxMode) -> Vec<u8> {
        let payload = encode_handshake_message(msg);
        let f = MuxFrame {
            header: MuxHeader {
                timestamp: 0,
                mode,
                mini_protocol_id: MiniProtocolId::new(0).expect("0"),
                length: payload.len() as u16,
            },
            payload,
        };
        encode_frame(&f).expect("encode")
    }

    fn wrap_chain_sync_frame(payload: Vec<u8>, mode: MuxMode) -> Vec<u8> {
        let f = MuxFrame {
            header: MuxHeader {
                timestamp: 0,
                mode,
                mini_protocol_id: MiniProtocolId::new(2).expect("2"),
                length: payload.len() as u16,
            },
            payload,
        };
        encode_frame(&f).expect("encode")
    }

    fn propose_v14_only() -> HandshakeMessage {
        use crate::codec::handshake::VersionTable;
        HandshakeMessage::ProposeVersions(VersionTable(vec![(
            N2NVersion::new(14),
            VersionParams(vec![0x01]),
        )]))
    }

    #[test]
    fn session_blocks_frames_before_handshake() {
        let mut state = SessionState::new_responder();
        // Send a chain-sync frame (id=2) before handshake → must be
        // peer-fatal PreHandshakeMiniProtocolFrame.
        let frame_bytes = wrap_chain_sync_frame(vec![0x00], MuxMode::Initiator);
        let err = step(&mut state, ByteChunkIn::Inbound(frame_bytes))
            .expect_err("must be pre-handshake fatal");
        assert!(
            matches!(err, SessionError::PreHandshakeMiniProtocolFrame { id: 2 }),
            "got {err:?}"
        );
    }

    #[test]
    fn session_unknown_mini_protocol_id_is_peer_fatal() {
        let mut state = SessionState::new_responder();
        // id = 11 is not in the closed registry.
        let f = MuxFrame {
            header: MuxHeader {
                timestamp: 0,
                mode: MuxMode::Initiator,
                mini_protocol_id: MiniProtocolId::new(11).expect("11"),
                length: 0,
            },
            payload: Vec::new(),
        };
        let bytes = encode_frame(&f).expect("encode");
        let err = step(&mut state, ByteChunkIn::Inbound(bytes))
            .expect_err("must be unknown-id fatal");
        assert!(
            matches!(err, SessionError::UnknownMiniProtocolId { id: 11 }),
            "got {err:?}"
        );
    }

    #[test]
    fn session_handshake_completion_transitions_state() {
        // Server-side: receive a ProposeVersions for v14 (we support it).
        let mut state = SessionState::new_responder();
        let frame_bytes = wrap_handshake_frame(&propose_v14_only(), MuxMode::Initiator);
        let effects = step(&mut state, ByteChunkIn::Inbound(frame_bytes))
            .expect("handshake step");
        // We expect: AcceptVersion frame on the wire + HandshakeComplete.
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, SessionEffect::HandshakeComplete { version: 14, .. })),
            "must emit HandshakeComplete, got {effects:?}"
        );
        assert!(state.is_connected(), "must transition to Connected");
    }

    #[test]
    fn session_outbound_frame_encodes_via_encode_frame() {
        let connected_state = ConnectedState::new(
            14,
            VersionData {
                network_magic: MAINNET_NETWORK_MAGIC,
                initiator_only_diffusion: false,
                peer_sharing: PeerSharingFlag::NoPeerSharing,
                query: false,
                peras_support: false,
            },
        );
        let mut state = SessionState::Connected(connected_state);
        let payload = vec![0xAA, 0xBB, 0xCC];
        let effects = step(
            &mut state,
            ByteChunkIn::OutboundFrame {
                mini_protocol: AcceptedMiniProtocol::ChainSync,
                payload: payload.clone(),
                mode: MuxMode::Initiator,
                timestamp: 12345,
            },
        )
        .expect("outbound step");
        let bytes = match effects.first() {
            Some(SessionEffect::SendBytes(b)) => b.clone(),
            other => panic!("expected SendBytes, got {other:?}"),
        };
        let (frame, _rest) = decode_frame(&bytes).expect("decode");
        assert_eq!(frame.header.mini_protocol_id.get(), 2);
        assert_eq!(frame.payload, payload);
        assert_eq!(frame.header.timestamp, 12345);
    }

    #[test]
    fn session_step_two_runs_byte_identical() {
        let payload = vec![0x01, 0x02, 0x03, 0x04];
        let run = || -> Vec<SessionEffect> {
            let mut state = SessionState::Connected(ConnectedState::new(
                14,
                VersionData {
                    network_magic: MAINNET_NETWORK_MAGIC,
                    initiator_only_diffusion: false,
                    peer_sharing: PeerSharingFlag::NoPeerSharing,
                    query: false,
                    peras_support: false,
                },
            ));
            step(
                &mut state,
                ByteChunkIn::OutboundFrame {
                    mini_protocol: AcceptedMiniProtocol::BlockFetch,
                    payload: payload.clone(),
                    mode: MuxMode::Initiator,
                    timestamp: 7,
                },
            )
            .expect("step")
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn session_post_handshake_handshake_frame_is_peer_fatal() {
        let mut state = SessionState::Connected(ConnectedState::new(
            14,
            VersionData {
                network_magic: MAINNET_NETWORK_MAGIC,
                initiator_only_diffusion: false,
                peer_sharing: PeerSharingFlag::NoPeerSharing,
                query: false,
                peras_support: false,
            },
        ));
        let frame_bytes = wrap_handshake_frame(&propose_v14_only(), MuxMode::Initiator);
        let err = step(&mut state, ByteChunkIn::Inbound(frame_bytes))
            .expect_err("must reject post-handshake handshake frame");
        assert!(matches!(err, SessionError::PostHandshakeHandshakeFrame));
    }

    #[test]
    fn session_connected_delivers_chain_sync_frame_as_effect() {
        let mut state = SessionState::Connected(ConnectedState::new(
            14,
            VersionData {
                network_magic: MAINNET_NETWORK_MAGIC,
                initiator_only_diffusion: false,
                peer_sharing: PeerSharingFlag::NoPeerSharing,
                query: false,
                peras_support: false,
            },
        ));
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let frame_bytes = wrap_chain_sync_frame(payload.clone(), MuxMode::Responder);
        let effects = step(&mut state, ByteChunkIn::Inbound(frame_bytes))
            .expect("step");
        match effects.as_slice() {
            [SessionEffect::DeliverPeerFrame {
                mini_protocol: AcceptedMiniProtocol::ChainSync,
                payload: p,
            }] => assert_eq!(p, &payload),
            other => panic!("expected one DeliverPeerFrame, got {other:?}"),
        }
        // N2N_SUPPORTED is loaded but unused outside the handshake driver;
        // touch it here so the import lint stays clean for now.
        let _ = N2N_SUPPORTED.len();
    }
}
