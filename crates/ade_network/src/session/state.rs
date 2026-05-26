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
use super::event::HandshakeRole;

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
}

impl ConnectedState {
    pub fn new(negotiated_version: u16, negotiated_params: VersionData) -> Self {
        Self {
            negotiated_version,
            negotiated_params,
            buffer: FrameBuffer::new(),
            next_outbound_seq: 0,
        }
    }
}
