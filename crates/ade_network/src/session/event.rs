// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN session event / effect / error sums (PHASE4-N-L S1+S2).
//!
//! `AcceptedMiniProtocol` is the SOLE closed registry of mini-protocol
//! ids the session demuxer dispatches over (DC-SESS-02). Adding a new
//! mini-protocol means adding a discriminant here — never a wildcard
//! match. The dispatch site at `session::core::step` is a closed
//! `match` over this enum.

use crate::codec::handshake::HandshakeMessage;
use crate::handshake::state::{HandshakeError, VersionData};
use crate::mux::frame::{MuxError, MuxMode};

/// Closed registry of Ouroboros mini-protocol ids the session
/// demuxer accepts. Matches the cardano-node 11.0.1 wire mapping.
///
/// Unknown ids are peer-fatal at the dispatch site
/// (`SessionError::UnknownMiniProtocolId`). Adding a new protocol
/// is an explicit discriminant addition here + a corresponding match
/// arm at `session::core::step`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AcceptedMiniProtocol {
    /// id = 0 — handshake (N2N + N2C).
    Handshake,
    /// id = 2 — chain-sync (N2N).
    ChainSync,
    /// id = 3 — block-fetch (N2N).
    BlockFetch,
    /// id = 4 — tx-submission2 (N2N).
    TxSubmission,
    /// id = 5 — local-chain-sync (N2C).
    LocalChainSync,
    /// id = 6 — local-tx-submission (N2C).
    LocalTxSubmission,
    /// id = 7 — local-state-query (N2C).
    LocalStateQuery,
    /// id = 8 — keep-alive (N2N).
    KeepAlive,
    /// id = 9 — local-tx-monitor (N2C).
    LocalTxMonitor,
    /// id = 10 — peer-sharing (N2N).
    PeerSharing,
}

impl AcceptedMiniProtocol {
    pub const HANDSHAKE_ID: u16 = 0;
    pub const CHAIN_SYNC_ID: u16 = 2;
    pub const BLOCK_FETCH_ID: u16 = 3;
    pub const TX_SUBMISSION_ID: u16 = 4;
    pub const LOCAL_CHAIN_SYNC_ID: u16 = 5;
    pub const LOCAL_TX_SUBMISSION_ID: u16 = 6;
    pub const LOCAL_STATE_QUERY_ID: u16 = 7;
    pub const KEEP_ALIVE_ID: u16 = 8;
    pub const LOCAL_TX_MONITOR_ID: u16 = 9;
    pub const PEER_SHARING_ID: u16 = 10;

    /// Total registry of accepted ids — used by the dispatch site for
    /// a sanity round-trip and by tests for completeness.
    pub const ALL: &'static [AcceptedMiniProtocol] = &[
        Self::Handshake,
        Self::ChainSync,
        Self::BlockFetch,
        Self::TxSubmission,
        Self::LocalChainSync,
        Self::LocalTxSubmission,
        Self::LocalStateQuery,
        Self::KeepAlive,
        Self::LocalTxMonitor,
        Self::PeerSharing,
    ];

    pub const fn id(self) -> u16 {
        match self {
            Self::Handshake => Self::HANDSHAKE_ID,
            Self::ChainSync => Self::CHAIN_SYNC_ID,
            Self::BlockFetch => Self::BLOCK_FETCH_ID,
            Self::TxSubmission => Self::TX_SUBMISSION_ID,
            Self::LocalChainSync => Self::LOCAL_CHAIN_SYNC_ID,
            Self::LocalTxSubmission => Self::LOCAL_TX_SUBMISSION_ID,
            Self::LocalStateQuery => Self::LOCAL_STATE_QUERY_ID,
            Self::KeepAlive => Self::KEEP_ALIVE_ID,
            Self::LocalTxMonitor => Self::LOCAL_TX_MONITOR_ID,
            Self::PeerSharing => Self::PEER_SHARING_ID,
        }
    }

    /// Closed-registry lookup. Unknown ids return `None`; the session
    /// core treats `None` as `SessionError::UnknownMiniProtocolId`.
    pub fn from_id(id: u16) -> Option<Self> {
        match id {
            Self::HANDSHAKE_ID => Some(Self::Handshake),
            Self::CHAIN_SYNC_ID => Some(Self::ChainSync),
            Self::BLOCK_FETCH_ID => Some(Self::BlockFetch),
            Self::TX_SUBMISSION_ID => Some(Self::TxSubmission),
            Self::LOCAL_CHAIN_SYNC_ID => Some(Self::LocalChainSync),
            Self::LOCAL_TX_SUBMISSION_ID => Some(Self::LocalTxSubmission),
            Self::LOCAL_STATE_QUERY_ID => Some(Self::LocalStateQuery),
            Self::KEEP_ALIVE_ID => Some(Self::KeepAlive),
            Self::LOCAL_TX_MONITOR_ID => Some(Self::LocalTxMonitor),
            Self::PEER_SHARING_ID => Some(Self::PeerSharing),
            _ => None,
        }
    }
}

/// Inbound session event — what the RED mux pump (S6) hands the
/// pure reducer. The reducer is sync; the pump is async.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ByteChunkIn {
    /// Raw bytes from the socket; the reducer accumulates and demuxes
    /// into complete frames.
    Inbound(Vec<u8>),
    /// A request from the orchestrator to send a mini-protocol frame.
    /// The session reducer encodes via `mux::frame::encode_frame` and
    /// emits `SessionEffect::SendBytes`.
    OutboundFrame {
        mini_protocol: AcceptedMiniProtocol,
        payload: Vec<u8>,
        mode: MuxMode,
        timestamp: u32,
    },
    /// Begin the N2N handshake by encoding our proposal and emitting
    /// SendBytes. Only valid in `SessionState::Handshaking`.
    HandshakeStartInitiator {
        proposal: HandshakeMessage,
    },
}

/// Closed effect sum the session reducer emits. The RED mux pump
/// translates each effect into a socket write or a forward onto
/// the orchestrator inbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEffect {
    /// Bytes to write to the peer socket.
    SendBytes(Vec<u8>),
    /// A complete mini-protocol frame to lift into an
    /// `OrchestratorEvent` at the pump layer. The pump constructs
    /// the matching `PeerChainSyncFrame` / `PeerBlockFetchFrame` /
    /// ... event from this and the bound `PeerId`.
    DeliverPeerFrame {
        mini_protocol: AcceptedMiniProtocol,
        payload: Vec<u8>,
    },
    /// Handshake completed with a negotiated version + params. The
    /// pump turns this into `PeerConnected { ... }` on the
    /// orchestrator inbox.
    HandshakeComplete {
        version: u16,
        params: VersionData,
    },
}

/// Closed session-error sum. Peer-fatal at the pump layer; the
/// pump emits `OrchestratorEvent::PeerDisconnected` and exits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    /// Inbound frame carried a mini-protocol id not in the closed
    /// `AcceptedMiniProtocol` registry. Peer-fatal (DC-SESS-02).
    UnknownMiniProtocolId { id: u16 },
    /// Inbound mini-protocol frame arrived before the handshake
    /// completed. Peer-fatal (DC-SESS-01).
    PreHandshakeMiniProtocolFrame { id: u16 },
    /// A handshake frame arrived after the handshake completed.
    /// Peer-fatal.
    PostHandshakeHandshakeFrame,
    /// Mux frame decode failed.
    Mux(MuxError),
    /// Handshake state-machine error.
    Handshake(HandshakeError),
    /// Outbound payload exceeded the 16-bit mux length field.
    OutboundPayloadTooLarge { len: usize },
}

/// `HandshakeRole` — initiator (we dial) vs responder (we accept).
/// The session reducer chooses the agency parity from this when
/// driving `handshake::n2n_transition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeRole {
    Initiator,
    Responder,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::mux::frame::MiniProtocolId;

    #[test]
    fn accepted_mini_protocol_round_trips_all_ids() {
        for p in AcceptedMiniProtocol::ALL {
            assert_eq!(AcceptedMiniProtocol::from_id(p.id()), Some(*p));
        }
    }

    #[test]
    fn accepted_mini_protocol_unknown_id_returns_none() {
        for id in [1u16, 11, 100, 0x7FFF] {
            assert_eq!(AcceptedMiniProtocol::from_id(id), None);
        }
    }

    #[test]
    fn accepted_mini_protocol_all_is_complete() {
        assert_eq!(AcceptedMiniProtocol::ALL.len(), 10);
    }

    // Compile-time exhaustiveness probe: if a discriminant is added,
    // this match will fail to compile until the new arm is added.
    #[test]
    fn accepted_mini_protocol_match_is_exhaustive() {
        for p in AcceptedMiniProtocol::ALL {
            let id = match p {
                AcceptedMiniProtocol::Handshake => AcceptedMiniProtocol::HANDSHAKE_ID,
                AcceptedMiniProtocol::ChainSync => AcceptedMiniProtocol::CHAIN_SYNC_ID,
                AcceptedMiniProtocol::BlockFetch => AcceptedMiniProtocol::BLOCK_FETCH_ID,
                AcceptedMiniProtocol::TxSubmission => AcceptedMiniProtocol::TX_SUBMISSION_ID,
                AcceptedMiniProtocol::LocalChainSync => AcceptedMiniProtocol::LOCAL_CHAIN_SYNC_ID,
                AcceptedMiniProtocol::LocalTxSubmission => {
                    AcceptedMiniProtocol::LOCAL_TX_SUBMISSION_ID
                }
                AcceptedMiniProtocol::LocalStateQuery => AcceptedMiniProtocol::LOCAL_STATE_QUERY_ID,
                AcceptedMiniProtocol::KeepAlive => AcceptedMiniProtocol::KEEP_ALIVE_ID,
                AcceptedMiniProtocol::LocalTxMonitor => AcceptedMiniProtocol::LOCAL_TX_MONITOR_ID,
                AcceptedMiniProtocol::PeerSharing => AcceptedMiniProtocol::PEER_SHARING_ID,
            };
            assert_eq!(id, p.id());
        }
    }

    #[test]
    fn mini_protocol_id_lookup_round_trips_via_mux_type() {
        for p in AcceptedMiniProtocol::ALL {
            let mpi = MiniProtocolId::new(p.id()).expect("valid id");
            assert_eq!(mpi.get(), p.id());
        }
    }
}
