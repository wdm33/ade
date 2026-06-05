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

use ade_codec::cbor::skip_item;
use ade_codec::error::CodecError;

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

/// PHASE4-N-F-G-E S1 (DC-LIVEMEM-01): the maximum bytes a single
/// per-mini-protocol reassembly buffer may hold for an INCOMPLETE item before
/// the session fails closed (drop the peer). Generous headroom over any
/// legitimate single block/item (a Cardano block is ~tens of KiB), while
/// bounding a peer that streams an endless / oversized incomplete item. A
/// **defensive implementation bound, NOT a Cardano semantic parameter**; it may
/// be tightened by a future hardening slice, but no runtime option (CLI / env /
/// config) may disable it or set it unbounded. Closed constant.
const MAX_REASSEMBLY_TAIL_BYTES: usize = 16 * 1024 * 1024;

/// PHASE4-N-AB S1 (CN-SESS-05): the maximum size of a single OUTBOUND
/// mini-protocol payload `handle_outbound` will segment into mux frames. Above
/// it, the send fails closed (`OutboundPayloadTooLarge`). The outbound
/// counterpart of `MAX_REASSEMBLY_TAIL_BYTES` (16 MiB, symmetric with the
/// inbound DC-LIVEMEM-01 reassembly cap) — generous over any legitimate single
/// block/item, while bounding the outbound buffer. A **defensive implementation
/// bound, NOT a Cardano semantic parameter**; it may be tightened by a future
/// slice, but no runtime option (CLI / env / config) may disable it or set it
/// unbounded. Closed constant.
const MAX_OUTBOUND_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;

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
                // PHASE4-N-M-FRAG: per-mini-protocol payload
                // reassembly. Append this mux frame's payload to
                // the per-protocol buffer; then drain every
                // COMPLETE CBOR item from the buffer head. The
                // peer's block-fetch server splits one MsgBlock
                // CBOR item across multiple mux frames bearing
                // the same protocol id; the consumer must see one
                // DeliverPeerFrame per item, not per frame.
                let buf = connected.proto_buffers.get_mut(proto);
                buf.extend_from_slice(&frame.payload);
                drain_protocol_items(proto, buf, &mut effects)?;
                // PHASE4-N-F-G-E S1 (DC-LIVEMEM-01): bound the reassembly tail.
                // After draining every COMPLETE item, an incomplete tail past
                // the cap is a peer streaming an endless / oversized item — fail
                // closed (drop the peer), never grow unbounded. No silent
                // truncation, no partial decode. Defensive bound, no disable.
                if buf.len() > MAX_REASSEMBLY_TAIL_BYTES {
                    return Err(SessionError::ReassemblyBufferOverflow {
                        protocol: proto.id(),
                        len: buf.len(),
                        cap: MAX_REASSEMBLY_TAIL_BYTES,
                    });
                }
            }
        }
    }
    Ok(effects)
}

/// Drain every COMPLETE CBOR item from the head of `buf`,
/// pushing one `DeliverPeerFrame` effect per item. Stops at the
/// first truncated tail (caller buffers it for the next mux
/// frame). Fails closed if `skip_item` reports a non-EOF
/// `CodecError` — that's a malformed item at a boundary, which
/// is peer-fatal (DC-SESS-06).
fn drain_protocol_items(
    proto: AcceptedMiniProtocol,
    buf: &mut Vec<u8>,
    effects: &mut Vec<SessionEffect>,
) -> Result<(), SessionError> {
    loop {
        if buf.is_empty() {
            return Ok(());
        }
        let mut offset = 0usize;
        match skip_item(buf, &mut offset) {
            Ok(_) => {
                // One complete item at [0..offset).
                let payload: Vec<u8> = buf.drain(..offset).collect();
                effects.push(SessionEffect::DeliverPeerFrame {
                    mini_protocol: proto,
                    payload,
                });
                // Loop to drain any further pipelined items.
            }
            Err(CodecError::UnexpectedEof { .. }) => {
                // Truncated tail — wait for the next mux frame.
                return Ok(());
            }
            Err(e) => {
                return Err(SessionError::ProtocolPayloadMalformed {
                    protocol: proto.id(),
                    detail: codec_error_detail(&e),
                });
            }
        }
    }
}

/// Closed `&'static str` label for a non-EOF `CodecError` at an
/// item boundary. Keeps `SessionError` `&'static str` carrying
/// (no heap strings on the reducer's error path).
fn codec_error_detail(err: &CodecError) -> &'static str {
    match err {
        CodecError::UnexpectedEof { .. } => "unexpected_eof",
        CodecError::UnknownEraTag { .. } => "unknown_era_tag",
        CodecError::UnknownCertTag { .. } => "unknown_cert_tag",
        CodecError::InvalidCborStructure { .. } => "invalid_cbor_structure",
        CodecError::TrailingBytes { .. } => "trailing_bytes",
        CodecError::UnexpectedCborType { .. } => "unexpected_cbor_type",
        CodecError::InvalidLength { .. } => "invalid_length",
        CodecError::DuplicateMapKey { .. } => "duplicate_map_key",
    }
}

fn handle_outbound(
    state: &mut SessionState,
    mini_protocol: AcceptedMiniProtocol,
    payload: Vec<u8>,
    mode: MuxMode,
    timestamp: u32,
) -> Result<Vec<SessionEffect>, SessionError> {
    // CN-SESS-05: fail closed only ABOVE the outbound ceiling; payloads up to it
    // are SEGMENTED below into <= MAX_PAYLOAD frames (so the per-frame guard in
    // encode_inner_frame is always satisfied).
    if payload.len() > MAX_OUTBOUND_PAYLOAD_BYTES {
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
    // CN-SESS-05 outbound segmentation — the inverse of CN-SESS-04 inbound
    // reassembly. A payload larger than MAX_PAYLOAD is split into ordered
    // <= MAX_PAYLOAD chunks, each encoded via the single-frame encode_inner_frame
    // authority; every segment carries the SAME mini-protocol id + mode and the
    // SAME captured `timestamp` (GREEN — no per-segment clock). An empty payload
    // still emits exactly one (empty) frame. The ordered frame bytes are
    // concatenated into one SendBytes; the receiver's demux + CN-SESS-04
    // reassembly reconstructs the original. Byte-preserving + lossless:
    // concat(segment payloads) == payload.
    let mut frames: Vec<u8> = Vec::new();
    if payload.is_empty() {
        frames.extend(encode_inner_frame(mini_protocol, Vec::new(), mode, timestamp)?);
    } else {
        for chunk in payload.chunks(MAX_PAYLOAD) {
            frames.extend(encode_inner_frame(mini_protocol, chunk.to_vec(), mode, timestamp)?);
        }
    }
    Ok(vec![SessionEffect::SendBytes(frames)])
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

    fn conn_vdata() -> VersionData {
        VersionData {
            network_magic: MAINNET_NETWORK_MAGIC,
            initiator_only_diffusion: false,
            peer_sharing: PeerSharingFlag::NoPeerSharing,
            query: false,
            peras_support: false,
        }
    }

    fn wrap_block_fetch_frame(payload: Vec<u8>, mode: MuxMode) -> Vec<u8> {
        let f = MuxFrame {
            header: MuxHeader {
                timestamp: 0,
                mode,
                mini_protocol_id: MiniProtocolId::new(3).expect("3"),
                length: payload.len() as u16,
            },
            payload,
        };
        encode_frame(&f).expect("encode")
    }

    // ===== PHASE4-N-F-G-E S1: reassembly-tail cap (DC-LIVEMEM-01) =====

    #[test]
    fn session_reassembly_tail_over_cap_fails_closed() {
        // A peer streams an endless INCOMPLETE item: a CBOR byte string (0x5B,
        // major 2 / 8-byte length) declaring a length far over the cap, then
        // never delivering it. `skip_item` returns UnexpectedEof on every drain,
        // so the per-protocol reassembly buffer grows. Once it exceeds
        // MAX_REASSEMBLY_TAIL_BYTES (16 MiB) the session MUST fail closed (drop
        // the peer) — never grow unbounded, never partial-accept.
        let mut state = SessionState::Connected(ConnectedState::new(14, conn_vdata()));
        let declared_len: u64 = 100 * 1024 * 1024; // >> the 16 MiB cap
        let mut first = vec![0x5Bu8];
        first.extend_from_slice(&declared_len.to_be_bytes());
        first.extend_from_slice(&vec![0u8; 60 * 1024]);
        let filler = vec![0u8; 60 * 1024];

        let mut got: Option<SessionError> = None;
        for i in 0..400u32 {
            let payload = if i == 0 { first.clone() } else { filler.clone() };
            let frame = wrap_block_fetch_frame(payload, MuxMode::Initiator);
            match step(&mut state, ByteChunkIn::Inbound(frame)) {
                Ok(_) => continue, // still under the cap, item still incomplete
                Err(e) => {
                    got = Some(e);
                    break;
                }
            }
        }
        let err = got.expect("reassembly MUST fail closed past the 16 MiB cap");
        assert!(
            matches!(
                err,
                SessionError::ReassemblyBufferOverflow { cap, len, .. }
                    if cap == 16 * 1024 * 1024 && len > cap
            ),
            "expected ReassemblyBufferOverflow over the 16 MiB cap, got {err:?}"
        );
    }

    #[test]
    fn session_reassembly_tail_under_cap_still_drains_complete_item() {
        // A normal under-cap item reassembles + drains UNCHANGED: a complete
        // 4-byte CBOR byte string (0x44 + 4 bytes) split across two frames →
        // exactly one DeliverPeerFrame carrying the full item, no error.
        let mut state = SessionState::Connected(ConnectedState::new(14, conn_vdata()));
        let f1 = wrap_block_fetch_frame(vec![0x44, 0x01, 0x02], MuxMode::Initiator);
        let e1 = step(&mut state, ByteChunkIn::Inbound(f1)).expect("frame1 ok");
        assert!(
            !e1.iter()
                .any(|e| matches!(e, SessionEffect::DeliverPeerFrame { .. })),
            "no complete item yet (item is split), got {e1:?}"
        );
        let f2 = wrap_block_fetch_frame(vec![0x03, 0x04], MuxMode::Initiator);
        let e2 = step(&mut state, ByteChunkIn::Inbound(f2)).expect("frame2 ok");
        assert!(
            e2.iter().any(|e| matches!(
                e,
                SessionEffect::DeliverPeerFrame { payload, .. }
                    if payload.as_slice() == [0x44, 0x01, 0x02, 0x03, 0x04]
            )),
            "the completed item must be delivered intact, got {e2:?}"
        );
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

    // -------------------------------------------------------------
    // PHASE4-N-M-FRAG: per-mini-protocol payload reassembly tests.
    // -------------------------------------------------------------

    /// Build a raw mux frame (any protocol id) carrying `payload`.
    /// Used by the FRAG tests to construct fragmented byte streams.
    fn wrap_frame(proto_id: u16, payload: Vec<u8>, mode: MuxMode) -> Vec<u8> {
        let f = MuxFrame {
            header: MuxHeader {
                timestamp: 0,
                mode,
                mini_protocol_id: MiniProtocolId::new(proto_id).expect("id"),
                length: payload.len() as u16,
            },
            payload,
        };
        encode_frame(&f).expect("encode")
    }

    /// Construct a `ConnectedState`-anchored `SessionState` with
    /// stock VersionData (mainnet magic, no peer-sharing).
    fn fresh_connected_state() -> SessionState {
        SessionState::Connected(ConnectedState::new(
            14,
            VersionData {
                network_magic: MAINNET_NETWORK_MAGIC,
                initiator_only_diffusion: false,
                peer_sharing: PeerSharingFlag::NoPeerSharing,
                query: false,
                peras_support: false,
            },
        ))
    }

    /// A canonical large CBOR `bytes(...)` value used to stand in
    /// for a Conway block-fetch body that exceeds the mux payload
    /// limit (65535). Returns one complete CBOR item with a
    /// 4-byte length header (CBOR major 2, additional info 26 =
    /// `0x5A`, followed by a big-endian u32 length).
    fn big_cbor_bytes(n: usize) -> Vec<u8> {
        assert!(n > u16::MAX as usize, "test fixture must exceed mux frame");
        assert!(n <= u32::MAX as usize, "test fixture fits in CBOR u32 length");
        let mut buf = Vec::with_capacity(n + 5);
        // major 2 (bytes) with 4-byte length argument.
        buf.push(0x5A);
        buf.extend_from_slice(&(n as u32).to_be_bytes());
        buf.extend(std::iter::repeat(0xAB).take(n));
        buf
    }

    #[test]
    fn fragmented_chain_sync_message_assembles_one_deliver() {
        let mut state = fresh_connected_state();
        // Chain-sync RequestNext = `0x81 0x00` (array(1)[uint(0)]).
        let cs_msg: Vec<u8> = vec![0x81, 0x00];
        // Split the item across two mux frames: first frame has
        // byte [0], second has byte [1]. Both bear protocol id 2.
        let frame_a = wrap_frame(2, vec![cs_msg[0]], MuxMode::Responder);
        let frame_b = wrap_frame(2, vec![cs_msg[1]], MuxMode::Responder);

        let effects_a = step(&mut state, ByteChunkIn::Inbound(frame_a)).expect("step a");
        assert!(
            effects_a.is_empty(),
            "first half-fragment must NOT emit a DeliverPeerFrame"
        );
        let effects_b = step(&mut state, ByteChunkIn::Inbound(frame_b)).expect("step b");
        match effects_b.as_slice() {
            [SessionEffect::DeliverPeerFrame {
                mini_protocol: AcceptedMiniProtocol::ChainSync,
                payload,
            }] => assert_eq!(payload, &cs_msg),
            other => panic!("expected exactly one DeliverPeerFrame, got {other:?}"),
        }
    }

    #[test]
    fn fragmented_block_fetch_block_assembles_one_deliver() {
        let mut state = fresh_connected_state();
        // Build `[4, <70KB bytes>]` outer array. Outer header is
        // `0x82 0x04` (array(2)[uint(4)]); then the 70KB bytes item.
        let bytes_item = big_cbor_bytes(70_000);
        let mut full_msg: Vec<u8> = Vec::with_capacity(2 + bytes_item.len());
        full_msg.push(0x82);
        full_msg.push(0x04);
        full_msg.extend_from_slice(&bytes_item);

        // Split into three mux frames. Each frame must be <= 65535
        // bytes per the mux header's u16 length. Use 25k, 25k, rest.
        let len = full_msg.len();
        assert!(len > u16::MAX as usize);
        let chunks: [&[u8]; 3] = [
            &full_msg[0..25_000],
            &full_msg[25_000..50_000],
            &full_msg[50_000..len],
        ];
        let f1 = wrap_frame(3, chunks[0].to_vec(), MuxMode::Responder);
        let f2 = wrap_frame(3, chunks[1].to_vec(), MuxMode::Responder);
        let f3 = wrap_frame(3, chunks[2].to_vec(), MuxMode::Responder);

        let e1 = step(&mut state, ByteChunkIn::Inbound(f1)).expect("f1");
        assert!(e1.is_empty(), "fragment 1 must not deliver");
        let e2 = step(&mut state, ByteChunkIn::Inbound(f2)).expect("f2");
        assert!(e2.is_empty(), "fragment 2 must not deliver");
        let e3 = step(&mut state, ByteChunkIn::Inbound(f3)).expect("f3");
        match e3.as_slice() {
            [SessionEffect::DeliverPeerFrame {
                mini_protocol: AcceptedMiniProtocol::BlockFetch,
                payload,
            }] => {
                assert_eq!(payload.len(), full_msg.len());
                assert_eq!(payload, &full_msg);
            }
            other => panic!("expected one DeliverPeerFrame on f3, got {other:?}"),
        }
    }

    #[test]
    fn interleaved_chain_sync_and_block_fetch_fragments_stay_isolated() {
        let mut state = fresh_connected_state();
        // CS fragment (first byte of `0x81 0x00`)
        let cs_frame_a = wrap_frame(2, vec![0x81], MuxMode::Responder);
        // BF fragment (first byte of `0x82 0x03` = array(2)[uint(3)]
        // for MsgNoBlocks shape; here we'll do `0x81 0x03` = a
        // 1-element array[uint(3)] = MsgNoBlocks itself).
        // To make the test exercise BOTH protocols' buffers under
        // fragmentation, we deliver each in two halves and
        // interleave.
        let bf_frame_a = wrap_frame(3, vec![0x81], MuxMode::Responder);
        let cs_frame_b = wrap_frame(2, vec![0x00], MuxMode::Responder);
        let bf_frame_b = wrap_frame(3, vec![0x03], MuxMode::Responder);

        let e1 = step(&mut state, ByteChunkIn::Inbound(cs_frame_a)).expect("cs a");
        assert!(e1.is_empty(), "cs fragment a must not deliver");
        let e2 = step(&mut state, ByteChunkIn::Inbound(bf_frame_a)).expect("bf a");
        assert!(e2.is_empty(), "bf fragment a must not deliver");
        let e3 = step(&mut state, ByteChunkIn::Inbound(cs_frame_b)).expect("cs b");
        match e3.as_slice() {
            [SessionEffect::DeliverPeerFrame {
                mini_protocol: AcceptedMiniProtocol::ChainSync,
                payload,
            }] => assert_eq!(payload, &vec![0x81, 0x00]),
            other => panic!("expected one CS DeliverPeerFrame, got {other:?}"),
        }
        let e4 = step(&mut state, ByteChunkIn::Inbound(bf_frame_b)).expect("bf b");
        match e4.as_slice() {
            [SessionEffect::DeliverPeerFrame {
                mini_protocol: AcceptedMiniProtocol::BlockFetch,
                payload,
            }] => assert_eq!(payload, &vec![0x81, 0x03]),
            other => panic!("expected one BF DeliverPeerFrame, got {other:?}"),
        }
    }

    #[test]
    fn pipelined_two_chain_sync_messages_in_one_mux_frame_emit_two_delivers() {
        let mut state = fresh_connected_state();
        // Concatenate two CBOR items inside one mux frame's payload.
        // First = RequestNext (`0x81 0x00`), second = AwaitReply
        // (`0x81 0x01`).
        let payload = vec![0x81, 0x00, 0x81, 0x01];
        let frame_bytes = wrap_frame(2, payload, MuxMode::Responder);
        let effects = step(&mut state, ByteChunkIn::Inbound(frame_bytes)).expect("step");
        match effects.as_slice() {
            [SessionEffect::DeliverPeerFrame {
                mini_protocol: AcceptedMiniProtocol::ChainSync,
                payload: p1,
            }, SessionEffect::DeliverPeerFrame {
                mini_protocol: AcceptedMiniProtocol::ChainSync,
                payload: p2,
            }] => {
                assert_eq!(p1, &vec![0x81, 0x00]);
                assert_eq!(p2, &vec![0x81, 0x01]);
            }
            other => panic!("expected two CS DeliverPeerFrames, got {other:?}"),
        }
    }

    #[test]
    fn malformed_cbor_at_item_boundary_returns_session_error() {
        let mut state = fresh_connected_state();
        // 0x1F = major type 0 (uint) with reserved additional-info
        // value 31, which `skip_item` rejects as
        // InvalidCborStructure rather than UnexpectedEof — i.e.,
        // truly malformed at the boundary.
        let bad_payload = vec![0x1F];
        let frame_bytes = wrap_frame(2, bad_payload, MuxMode::Responder);
        let err = step(&mut state, ByteChunkIn::Inbound(frame_bytes)).expect_err("must err");
        match err {
            SessionError::ProtocolPayloadMalformed { protocol, .. } => {
                assert_eq!(protocol, 2);
            }
            other => panic!("expected ProtocolPayloadMalformed, got {other:?}"),
        }
    }

    #[test]
    fn truncated_then_complete_two_step_drain() {
        let mut state = fresh_connected_state();
        // Item = `0x82 0x18 0x2A 0x18 0x2B` = array(2)[uint(42),
        // uint(43)] = 5 bytes. Deliver bytes [0..3] in frame 1
        // (truncated), then [3..5] in frame 2 (completes).
        let item = vec![0x82, 0x18, 0x2A, 0x18, 0x2B];
        let f1 = wrap_frame(2, item[..3].to_vec(), MuxMode::Responder);
        let f2 = wrap_frame(2, item[3..].to_vec(), MuxMode::Responder);

        let e1 = step(&mut state, ByteChunkIn::Inbound(f1)).expect("f1");
        assert!(e1.is_empty(), "truncated item must NOT deliver");
        let e2 = step(&mut state, ByteChunkIn::Inbound(f2)).expect("f2");
        match e2.as_slice() {
            [SessionEffect::DeliverPeerFrame { payload, .. }] => {
                assert_eq!(payload, &item);
            }
            other => panic!("expected one DeliverPeerFrame on f2, got {other:?}"),
        }
    }

    #[test]
    fn proto_buffers_isolation_across_accepted_protocols() {
        // Send one fragmented item to each non-handshake protocol.
        // First half is a 1-byte truncation; second half completes.
        let item: Vec<u8> = vec![0x81, 0x00];
        // Closed list of non-handshake protocols (id != 0).
        let protos: [(AcceptedMiniProtocol, u16); 9] = [
            (AcceptedMiniProtocol::ChainSync, 2),
            (AcceptedMiniProtocol::BlockFetch, 3),
            (AcceptedMiniProtocol::TxSubmission, 4),
            (AcceptedMiniProtocol::LocalChainSync, 5),
            (AcceptedMiniProtocol::LocalTxSubmission, 6),
            (AcceptedMiniProtocol::LocalStateQuery, 7),
            (AcceptedMiniProtocol::KeepAlive, 8),
            (AcceptedMiniProtocol::LocalTxMonitor, 9),
            (AcceptedMiniProtocol::PeerSharing, 10),
        ];
        for (variant, id) in protos.iter() {
            let mut state = fresh_connected_state();
            let f1 = wrap_frame(*id, item[..1].to_vec(), MuxMode::Responder);
            let f2 = wrap_frame(*id, item[1..].to_vec(), MuxMode::Responder);
            let e1 = step(&mut state, ByteChunkIn::Inbound(f1)).expect("f1");
            assert!(
                e1.is_empty(),
                "{variant:?}: truncated half MUST not deliver"
            );
            let e2 = step(&mut state, ByteChunkIn::Inbound(f2)).expect("f2");
            match e2.as_slice() {
                [SessionEffect::DeliverPeerFrame {
                    mini_protocol,
                    payload,
                }] => {
                    assert_eq!(mini_protocol, variant);
                    assert_eq!(payload, &item);
                }
                other => panic!(
                    "{variant:?}: expected one DeliverPeerFrame after reassembly, got {other:?}"
                ),
            }
        }
    }

    #[test]
    fn fragmented_replay_equivalence_two_runs_byte_identical() {
        // T-DET-01 strengthening: identical fragmented input chunks
        // produce byte-identical DeliverPeerFrame sequences across
        // two reducer runs.
        let item = vec![0x82, 0x18, 0x2A, 0x18, 0x2B];
        let f1_bytes = wrap_frame(2, item[..2].to_vec(), MuxMode::Responder);
        let f2_bytes = wrap_frame(2, item[2..].to_vec(), MuxMode::Responder);

        fn drive(f1: Vec<u8>, f2: Vec<u8>) -> Vec<SessionEffect> {
            let mut state = fresh_connected_state();
            let mut out = Vec::new();
            out.extend(step(&mut state, ByteChunkIn::Inbound(f1)).expect("f1"));
            out.extend(step(&mut state, ByteChunkIn::Inbound(f2)).expect("f2"));
            out
        }
        let a = drive(f1_bytes.clone(), f2_bytes.clone());
        let b = drive(f1_bytes, f2_bytes);
        assert_eq!(a, b, "fragmented inbound → DeliverPeerFrame sequence must be byte-identical across runs");
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
        // PHASE4-N-M-FRAG: the reducer now requires the
        // mini-protocol payload to be valid CBOR (one complete
        // item per DeliverPeerFrame). `0x81 0x00` = `array(1)
        // [uint(0)]` = `ChainSyncMessage::RequestNext` wire bytes.
        let payload = vec![0x81u8, 0x00];
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

    // -------------------------------------------------------------
    // PHASE4-N-AB S1: outbound mux segmentation (CN-SESS-05).
    // -------------------------------------------------------------

    /// Decode every mux frame in a SendBytes blob (what a peer's demux does).
    fn decode_all_frames(mut bytes: &[u8]) -> Vec<MuxFrame> {
        let mut out = Vec::new();
        while !bytes.is_empty() {
            let (frame, rest) = decode_frame(bytes).expect("decode frame");
            out.push(frame);
            bytes = rest;
        }
        out
    }

    /// Drive one outbound BlockFetch message through the reducer and return the
    /// encoded mux frames (Responder mode = the serve direction).
    fn outbound_frames(payload: Vec<u8>) -> Vec<MuxFrame> {
        let mut state = fresh_connected_state();
        let effects = step(
            &mut state,
            ByteChunkIn::OutboundFrame {
                mini_protocol: AcceptedMiniProtocol::BlockFetch,
                payload,
                mode: MuxMode::Responder,
                timestamp: 99,
            },
        )
        .expect("outbound step");
        let bytes = match effects.first() {
            Some(SessionEffect::SendBytes(b)) => b.clone(),
            other => panic!("expected SendBytes, got {other:?}"),
        };
        decode_all_frames(&bytes)
    }

    #[test]
    fn outbound_payload_at_max_payload_is_one_frame() {
        let frames = outbound_frames(vec![0xCD; MAX_PAYLOAD]);
        assert_eq!(frames.len(), 1, "len == MAX_PAYLOAD is a single frame");
        assert_eq!(frames[0].payload.len(), MAX_PAYLOAD);
    }

    #[test]
    fn outbound_payload_over_max_payload_segments_into_two() {
        let frames = outbound_frames(vec![0xCD; MAX_PAYLOAD + 1]);
        assert_eq!(frames.len(), 2, "len == MAX_PAYLOAD + 1 is two frames");
        assert_eq!(frames[0].payload.len(), MAX_PAYLOAD);
        assert_eq!(frames[1].payload.len(), 1);
    }

    #[test]
    fn outbound_segment_order_preserved() {
        let len = MAX_PAYLOAD * 2 + 7;
        let payload: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
        let frames = outbound_frames(payload.clone());
        assert_eq!(frames.len(), 3);
        let reassembled: Vec<u8> = frames.iter().flat_map(|f| f.payload.clone()).collect();
        assert_eq!(reassembled, payload, "concat(segments) == original, in order");
    }

    #[test]
    fn outbound_segments_keep_same_mini_protocol_id_and_mode() {
        let frames = outbound_frames(vec![0x01; MAX_PAYLOAD * 2 + 100]);
        assert_eq!(frames.len(), 3);
        let id0 = frames[0].header.mini_protocol_id.get();
        let mode0 = frames[0].header.mode;
        for f in &frames {
            assert_eq!(f.header.mini_protocol_id.get(), id0, "same mini-protocol id");
            assert_eq!(f.header.mode, mode0, "same mode");
            assert!(f.payload.len() <= MAX_PAYLOAD, "each frame <= MAX_PAYLOAD");
        }
    }

    #[test]
    fn outbound_large_payload_reassembles_byte_identical_via_inbound() {
        // A valid CBOR item [4, bytes(70000)] (block-fetch-shaped) > MAX_PAYLOAD.
        let bytes_item = big_cbor_bytes(70_000);
        let mut full_msg: Vec<u8> = Vec::with_capacity(2 + bytes_item.len());
        full_msg.push(0x82);
        full_msg.push(0x04);
        full_msg.extend_from_slice(&bytes_item);
        // Segment it via the outbound reducer (Responder = serve direction).
        let mut out_state = fresh_connected_state();
        let effects = step(
            &mut out_state,
            ByteChunkIn::OutboundFrame {
                mini_protocol: AcceptedMiniProtocol::BlockFetch,
                payload: full_msg.clone(),
                mode: MuxMode::Responder,
                timestamp: 7,
            },
        )
        .expect("outbound step");
        let wire = match effects.first() {
            Some(SessionEffect::SendBytes(b)) => b.clone(),
            other => panic!("expected SendBytes, got {other:?}"),
        };
        assert!(decode_all_frames(&wire).len() >= 2, "70KB payload must segment");
        // Feed the segmented wire bytes back through Ade's OWN inbound reassembly
        // (CN-SESS-04) — the round-trip must reconstruct the original exactly.
        let mut in_state = fresh_connected_state();
        let delivered = step(&mut in_state, ByteChunkIn::Inbound(wire)).expect("inbound step");
        match delivered.as_slice() {
            [SessionEffect::DeliverPeerFrame {
                mini_protocol: AcceptedMiniProtocol::BlockFetch,
                payload,
            }] => assert_eq!(payload, &full_msg, "segment -> reassemble == identity"),
            other => panic!("expected one DeliverPeerFrame, got {other:?}"),
        }
    }

    #[test]
    fn outbound_payload_at_upper_bound_is_allowed() {
        let mut state = fresh_connected_state();
        let effects = step(
            &mut state,
            ByteChunkIn::OutboundFrame {
                mini_protocol: AcceptedMiniProtocol::BlockFetch,
                payload: vec![0x00; MAX_OUTBOUND_PAYLOAD_BYTES],
                mode: MuxMode::Responder,
                timestamp: 1,
            },
        )
        .expect("payload at the upper bound is allowed (segmented)");
        let bytes = match effects.first() {
            Some(SessionEffect::SendBytes(b)) => b.clone(),
            other => panic!("expected SendBytes, got {other:?}"),
        };
        let frames = decode_all_frames(&bytes);
        let total: usize = frames.iter().map(|f| f.payload.len()).sum();
        assert_eq!(
            total, MAX_OUTBOUND_PAYLOAD_BYTES,
            "every byte segmented, none dropped"
        );
        for f in &frames {
            assert!(f.payload.len() <= MAX_PAYLOAD);
        }
    }

    #[test]
    fn outbound_payload_over_upper_bound_fails_closed() {
        let mut state = fresh_connected_state();
        let err = step(
            &mut state,
            ByteChunkIn::OutboundFrame {
                mini_protocol: AcceptedMiniProtocol::BlockFetch,
                payload: vec![0x00; MAX_OUTBOUND_PAYLOAD_BYTES + 1],
                mode: MuxMode::Responder,
                timestamp: 1,
            },
        )
        .expect_err("payload over the upper bound fails closed");
        assert!(matches!(
            err,
            SessionError::OutboundPayloadTooLarge { len } if len == MAX_OUTBOUND_PAYLOAD_BYTES + 1
        ));
    }
}
