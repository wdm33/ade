// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN session state (PHASE4-N-L S2).
//!
//! Type-state split: `Handshaking` cannot deliver mini-protocol
//! frames to the orchestrator inbox; only `Connected` can.
//! DC-SESS-01 is the runtime asserter; this type makes it harder to
//! construct an inconsistent state in the first place.

use crate::handshake::state::{HandshakeState, VersionData};

use super::demux::FrameBuffer;
use super::event::{AcceptedMiniProtocol, HandshakeRole};

/// Top-level session state — closed sum.
pub enum SessionState {
    Handshaking(HandshakeProgress),
    Connected(ConnectedState),
}

impl SessionState {
    pub fn new_initiator() -> Self {
        Self::Handshaking(HandshakeProgress {
            role: HandshakeRole::Initiator,
            inner: HandshakeState::Idle,
            buffer: FrameBuffer::new(),
            proposal_sent: false,
        })
    }

    pub fn new_responder() -> Self {
        Self::Handshaking(HandshakeProgress {
            role: HandshakeRole::Responder,
            inner: HandshakeState::Idle,
            buffer: FrameBuffer::new(),
            proposal_sent: false,
        })
    }

    pub fn is_handshaking(&self) -> bool {
        matches!(self, Self::Handshaking(_))
    }

    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected(_))
    }
}

/// Mid-handshake state. The buffer accumulates bytes; only
/// handshake (id=0) frames are legal in this state.
pub struct HandshakeProgress {
    pub role: HandshakeRole,
    pub inner: HandshakeState,
    pub buffer: FrameBuffer,
    /// Initiator-only: have we encoded + emitted our proposal yet?
    /// Stays false until `HandshakeStartInitiator` is handled.
    pub proposal_sent: bool,
}

/// Post-handshake state. The negotiated version is pinned; all
/// inbound frames go through the closed mini-protocol dispatch.
pub struct ConnectedState {
    pub negotiated_version: u16,
    pub negotiated_params: VersionData,
    pub buffer: FrameBuffer,
    /// Monotonic outbound sequence number — purely for replay
    /// traceability; not on the wire.
    pub next_outbound_seq: u64,
    /// Per-mini-protocol payload accumulators. Cardano N2N mux
    /// frames carry a u16 payload length (max 65535 bytes); the
    /// peer routinely splits large messages (e.g. Conway blocks
    /// in `MsgBlock`) across multiple frames bearing the same
    /// protocol id. The session reducer assembles bytes here per
    /// protocol and emits one `DeliverPeerFrame` per COMPLETE
    /// CBOR item — never per mux frame. PHASE4-N-M-FRAG.
    pub proto_buffers: ProtoBuffers,
}

impl ConnectedState {
    pub fn new(negotiated_version: u16, negotiated_params: VersionData) -> Self {
        Self {
            negotiated_version,
            negotiated_params,
            buffer: FrameBuffer::new(),
            next_outbound_seq: 0,
            proto_buffers: ProtoBuffers::new(),
        }
    }
}

/// Per-mini-protocol payload accumulators. CLOSED registry —
/// one `Vec<u8>` per `AcceptedMiniProtocol` variant; iteration
/// order is the declaration order (no `HashMap`).
///
/// Each buffer accumulates raw payload bytes lifted off
/// consecutive same-protocol mux frames. The session reducer
/// drains COMPLETE CBOR items off the head of each buffer via
/// `ade_codec::cbor::skip_item`; truncated tails wait for the
/// next mux frame to arrive.
pub struct ProtoBuffers {
    pub handshake: Vec<u8>,
    pub chain_sync: Vec<u8>,
    pub block_fetch: Vec<u8>,
    pub tx_submission: Vec<u8>,
    pub local_chain_sync: Vec<u8>,
    pub local_tx_submission: Vec<u8>,
    pub local_state_query: Vec<u8>,
    pub keep_alive: Vec<u8>,
    pub local_tx_monitor: Vec<u8>,
    pub peer_sharing: Vec<u8>,
}

impl ProtoBuffers {
    pub fn new() -> Self {
        Self {
            handshake: Vec::new(),
            chain_sync: Vec::new(),
            block_fetch: Vec::new(),
            tx_submission: Vec::new(),
            local_chain_sync: Vec::new(),
            local_tx_submission: Vec::new(),
            local_state_query: Vec::new(),
            keep_alive: Vec::new(),
            local_tx_monitor: Vec::new(),
            peer_sharing: Vec::new(),
        }
    }

    /// Closed dispatch: returns the accumulating buffer for the
    /// given protocol. The match is exhaustive on the closed sum.
    pub fn get_mut(&mut self, proto: AcceptedMiniProtocol) -> &mut Vec<u8> {
        match proto {
            AcceptedMiniProtocol::Handshake => &mut self.handshake,
            AcceptedMiniProtocol::ChainSync => &mut self.chain_sync,
            AcceptedMiniProtocol::BlockFetch => &mut self.block_fetch,
            AcceptedMiniProtocol::TxSubmission => &mut self.tx_submission,
            AcceptedMiniProtocol::LocalChainSync => &mut self.local_chain_sync,
            AcceptedMiniProtocol::LocalTxSubmission => &mut self.local_tx_submission,
            AcceptedMiniProtocol::LocalStateQuery => &mut self.local_state_query,
            AcceptedMiniProtocol::KeepAlive => &mut self.keep_alive,
            AcceptedMiniProtocol::LocalTxMonitor => &mut self.local_tx_monitor,
            AcceptedMiniProtocol::PeerSharing => &mut self.peer_sharing,
        }
    }
}

impl Default for ProtoBuffers {
    fn default() -> Self {
        Self::new()
    }
}
