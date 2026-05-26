// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration test — PHASE4-N-L S9 (DC-NET-03).
//!
//! Replaying the same byte-chunk transcript through
//! `session::core::step` produces byte-identical effects across two
//! runs. The transcript is constructed in-test (mirrors PHASE4-N-K
//! S8's replay-corpus pattern) — a deterministic builder is itself
//! the corpus contract.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ade_network::codec::handshake::{
    encode_handshake_message, HandshakeMessage, VersionParams, VersionTable,
};
use ade_network::codec::N2NVersion;
use ade_network::mux::frame::{encode_frame, MiniProtocolId, MuxFrame, MuxHeader, MuxMode};
use ade_network::session::{
    step, ByteChunkIn, ConnectedState, SessionEffect, SessionState,
};
use ade_network::handshake::state::{PeerSharingFlag, VersionData};
use ade_network::handshake::version_table::MAINNET_NETWORK_MAGIC;

fn handshake_propose_frame() -> Vec<u8> {
    let proposal = HandshakeMessage::ProposeVersions(VersionTable(vec![(
        N2NVersion::new(14),
        VersionParams(vec![0x01]),
    )]));
    let payload = encode_handshake_message(&proposal);
    encode_frame(&MuxFrame {
        header: MuxHeader {
            timestamp: 0,
            mode: MuxMode::Initiator,
            mini_protocol_id: MiniProtocolId::new(0).expect("0"),
            length: payload.len() as u16,
        },
        payload,
    })
    .expect("encode")
}

fn chain_sync_frame(payload: Vec<u8>) -> Vec<u8> {
    encode_frame(&MuxFrame {
        header: MuxHeader {
            timestamp: 0,
            mode: MuxMode::Responder,
            mini_protocol_id: MiniProtocolId::new(2).expect("2"),
            length: payload.len() as u16,
        },
        payload,
    })
    .expect("encode")
}

fn build_canonical_byte_chunk_transcript() -> Vec<ByteChunkIn> {
    let mut out = Vec::new();
    // Phase 1: handshake — responder receives the proposal.
    out.push(ByteChunkIn::Inbound(handshake_propose_frame()));
    // Phase 2: post-handshake mini-protocol traffic. We can't actually
    // do this through the responder path since the responder transitions
    // to Connected after Accept; instead, the harness uses a directly-
    // constructed Connected state for phase 2 to keep the test
    // hermetic (no two-party drive needed).
    out
}

fn run_handshake_phase(chunks: &[ByteChunkIn]) -> Vec<SessionEffect> {
    let mut state = SessionState::new_responder();
    let mut effects = Vec::new();
    for chunk in chunks {
        effects.extend(step(&mut state, chunk.clone()).expect("step"));
    }
    effects
}

fn run_connected_phase(chunks: &[ByteChunkIn]) -> Vec<SessionEffect> {
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
    let mut effects = Vec::new();
    for chunk in chunks {
        effects.extend(step(&mut state, chunk.clone()).expect("step"));
    }
    effects
}

#[test]
fn session_replay_equivalence_holds() {
    // Handshake-phase corpus.
    let hs_chunks = build_canonical_byte_chunk_transcript();
    let a = run_handshake_phase(&hs_chunks);
    let b = run_handshake_phase(&hs_chunks);
    assert_eq!(a, b, "handshake phase must be replay-equivalent");

    // Connected-phase corpus: three chain-sync frames interleaved
    // with a block-fetch frame and an outbound request.
    let cs1 = chain_sync_frame(vec![0x01, 0x02, 0x03]);
    let cs2 = chain_sync_frame(vec![0x04, 0x05]);
    let connected_chunks = vec![
        ByteChunkIn::Inbound(cs1),
        ByteChunkIn::Inbound(cs2),
        ByteChunkIn::OutboundFrame {
            mini_protocol: ade_network::session::AcceptedMiniProtocol::ChainSync,
            payload: vec![0xAA, 0xBB],
            mode: MuxMode::Initiator,
            timestamp: 42,
        },
    ];
    let c = run_connected_phase(&connected_chunks);
    let d = run_connected_phase(&connected_chunks);
    assert_eq!(c, d, "connected phase must be replay-equivalent");
    assert!(
        c.iter().any(|e| matches!(e, SessionEffect::DeliverPeerFrame { .. })),
        "must deliver at least one peer frame"
    );
    assert!(
        c.iter().any(|e| matches!(e, SessionEffect::SendBytes(_))),
        "must emit at least one SendBytes"
    );
}

#[test]
fn session_replay_corpus_builds_deterministically() {
    let a = build_canonical_byte_chunk_transcript();
    let b = build_canonical_byte_chunk_transcript();
    assert_eq!(a, b);
    assert!(!a.is_empty());
}
