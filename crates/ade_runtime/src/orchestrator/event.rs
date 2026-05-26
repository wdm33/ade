// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN orchestrator event + effect sums (PHASE4-N-K S2).
//!
//! These are the canonical input/output vocabulary of the
//! orchestrator core. RED runners (peer sessions, leadership
//! session, server pump) translate sockets and clocks into
//! `OrchestratorEvent`s and effects into socket writes.

use ade_network::codec::version::{BlockFetchVersion, ChainSyncVersion};
use ade_types::{Hash32, SlotNo};

/// Stable, deterministic identifier for one connected peer. The
/// production runner allocates these from a monotonic atomic
/// counter; tests assign them explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeerId(pub u64);

/// Closed event sum consumed by `orchestrator::core::step`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorEvent {
    /// A slot boundary advanced by the `Clock`. `slot_millis` is
    /// the wall-clock millisecond value the clock emitted; `slot`
    /// is its translation through the era schedule.
    SlotTick { slot_millis: u64, slot: SlotNo },

    /// A new peer connected (receive-side or server-side); the
    /// orchestrator installs per-peer state in its map.
    PeerConnected {
        peer_id: PeerId,
        chain_sync_version: ChainSyncVersion,
        block_fetch_version: BlockFetchVersion,
        role: PeerRole,
    },

    /// A peer disconnected; the orchestrator removes that peer's
    /// state. No other peer is affected.
    PeerDisconnected { peer_id: PeerId },

    /// Frame received from a receive-side upstream peer (client of
    /// chain-sync / block-fetch).
    PeerChainSyncFrame { peer_id: PeerId, bytes: Vec<u8> },
    PeerBlockFetchFrame { peer_id: PeerId, bytes: Vec<u8> },

    /// Frame received from a server-side downstream peer (we are
    /// the server; the peer is fetching from us).
    PeerN2nServerChainSyncFrame { peer_id: PeerId, bytes: Vec<u8> },
    PeerN2nServerBlockFetchFrame { peer_id: PeerId, bytes: Vec<u8> },

    /// External shutdown signal (Ctrl-C, SIGTERM); the orchestrator
    /// drains the admit/write/snapshot pipeline and emits
    /// `ShutdownAcknowledged`.
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerRole {
    /// Upstream peer the node is following (receive side).
    UpstreamClient,
    /// Downstream peer the node is serving (server side).
    DownstreamServer,
}

/// Closed effect sum emitted by `orchestrator::core::step`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorEffect {
    /// Encoded bytes to write to a peer socket. The RED runner is
    /// responsible for the socket-side `write_all`.
    SendToPeer { peer_id: PeerId, bytes: Vec<u8> },

    /// A block was admitted into the canonical ledger + chaindb.
    /// The RED runner may emit this to its logging / metrics
    /// surface; the orchestrator state already reflects it.
    AdmittedBlock { slot: SlotNo, hash: Hash32 },

    /// Cadence said "snapshot at this slot." The persistent writer
    /// (S3) consumes this effect and calls
    /// `PersistentSnapshotCache::capture`.
    CaptureSnapshot { slot: SlotNo },

    /// A peer session halted (decode error, validity reject,
    /// rollback-too-deep, ...). The runner drops the socket; per-
    /// peer isolation (DC-NODE-01) guarantees no other peer is
    /// affected.
    PeerSessionHalted { peer_id: PeerId, reason: PeerHaltReason },

    /// Shutdown acknowledged; orchestrator state is now quiescent.
    /// The runner force-captures a final snapshot via the
    /// persistent writer, then exits.
    ShutdownAcknowledged,
}

/// Closed reason-tag for `PeerSessionHalted`. No `String` — every
/// variant maps to a known closed-discriminant condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerHaltReason {
    ChainSyncDecodeError,
    BlockFetchDecodeError,
    ServerChainSyncDecodeError,
    ServerBlockFetchDecodeError,
    ReceiveValidityRejected,
    ReceiveHeaderBodyMismatch,
    ReceiveRollbackOutOfScope,
    ServerChainSyncProtocolError,
    ServerBlockFetchProtocolError,
    PeerUnknown,
}

/// Closed authority-fatal kind. Any of these halts the binary
/// deterministically (DC-NODE-04). Errors that are merely per-
/// peer-fatal map to `PeerSessionHalted`, not this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityFatalKind {
    ChainWriteIo,
    SnapshotDecodeUnknownVersion,
    SnapshotDecodeFingerprintMismatch,
}

/// Top-level orchestrator error. Single variant today; the closed
/// sum stays in place so future authority-fatal kinds slot in
/// without changing the type signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestratorError {
    AuthorityFatal(AuthorityFatalKind),
}
