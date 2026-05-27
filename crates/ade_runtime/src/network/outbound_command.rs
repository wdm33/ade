// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED `OutboundCommand` closed enum (PHASE4-N-S-B B2).
//!
//! The sole channel between `produce_mode::dispatch_server_frame_event`
//! and `MuxPump`'s outbound session-aware encoder. Carries
//! typed mini-protocol replies (`chain_sync::ServerReply` /
//! `block_fetch::ServerReply`) plus the target `PeerId` — **not**
//! pre-encoded bytes. The session-aware encoder is the only
//! producer of wire-byte streams.
//!
//! Doctrine: see [[feedback-fail-closed-validation]] —
//! typed-commands-only outbound traversal makes
//! arbitrary-byte tunneling structurally unrepresentable.

use ade_network::block_fetch::server::ServerReply as BlockFetchServerReply;
use ade_network::chain_sync::server::ServerReply as ChainSyncServerReply;

use crate::orchestrator::event::PeerId;

/// Closed outbound-command surface. Each variant carries
/// the target `PeerId` so MuxPump can verify the command is
/// destined for the peer it owns — cross-peer leakage is
/// structurally impossible.
#[derive(Debug, Clone)]
pub enum OutboundCommand {
    /// Send a chain-sync server reply to `peer`.
    ChainSync {
        peer: PeerId,
        reply: ChainSyncServerReply,
    },
    /// Send a block-fetch server reply to `peer`.
    BlockFetch {
        peer: PeerId,
        reply: BlockFetchServerReply,
    },
    /// Cleanly close the per-peer session.
    ClosePeer {
        peer: PeerId,
        reason: CloseReason,
    },
}

impl OutboundCommand {
    /// The peer this command is destined for. MuxPump uses
    /// this to enforce no-cross-peer-leakage at runtime.
    pub fn peer(&self) -> PeerId {
        match self {
            OutboundCommand::ChainSync { peer, .. }
            | OutboundCommand::BlockFetch { peer, .. }
            | OutboundCommand::ClosePeer { peer, .. } => *peer,
        }
    }
}

/// Closed close-reason taxonomy. No `String` payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReason {
    /// Operator-initiated graceful close.
    Graceful,
    /// Detected protocol violation by the peer (the dispatch
    /// reducer returned a fatal error; closing the session is
    /// the fail-closed response).
    ProtocolViolation,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_network::block_fetch::server::ServerReply as BlockFetchReply;
    use ade_network::chain_sync::server::ServerReply as ChainSyncReply;

    #[test]
    fn outbound_command_peer_accessor_returns_target_peer() {
        let p = PeerId(42);
        let c = OutboundCommand::BlockFetch {
            peer: p,
            reply: BlockFetchReply::no_blocks(),
        };
        assert_eq!(c.peer(), p);
    }

    #[test]
    fn outbound_command_carries_typed_reply_not_raw_bytes() {
        // Compile-time check: OutboundCommand variants accept
        // only typed ServerReply values; passing Vec<u8> would
        // fail at the type level. This test exists to document
        // the load-bearing constraint (CN-OUTBOUND-RELAY-01).
        let p = PeerId(1);
        let _cs = OutboundCommand::ChainSync {
            peer: p,
            reply: ChainSyncReply::await_reply(),
        };
        let _bf = OutboundCommand::BlockFetch {
            peer: p,
            reply: BlockFetchReply::batch_done(),
        };
        let _close = OutboundCommand::ClosePeer {
            peer: p,
            reason: CloseReason::Graceful,
        };
    }
}
