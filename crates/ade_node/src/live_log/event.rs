// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN closed wire-only JSONL event vocabulary (PHASE4-N-L-LIVE S1).
//!
//! The wire-only mode's emitted log is a CLOSED sum of seven
//! variants. Adding a new variant requires a code change here +
//! a corresponding allow-list update in
//! `ci/ci_check_wire_only_event_vocabulary_closed.sh`. The
//! enum's design is the type-level half of ¬P-1 in the cluster
//! invariants sketch: the binary cannot emit
//! `agreement_verdict` / `admitted_block` / `ledger_applied` /
//! `projection_updated` because no variant carries that
//! discriminator. The CI grep is the file-tree half of the same
//! property.
//!
//! Doctrine reference (memory): "Transport success does not imply
//! ledger agreement. Handshake success does not imply block
//! admission. Tip visibility does not imply canonical state
//! transition." The wire-only event vocabulary is the load-bearing
//! example of that doctrine in code.

/// Closed sum of wire-only log event kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveLogEvent {
    /// Binary started; mode + peer count recorded.
    NodeStarted {
        mode: ModeTag,
        peer_count: u32,
    },
    /// A per-peer dial task is starting.
    PeerDialStarted {
        peer: String,
    },
    /// N2N handshake completed with this peer; negotiated
    /// protocol version recorded.
    HandshakeOk {
        peer: String,
        negotiated_version: u16,
    },
    /// Peer's announced tip read from a chain-sync
    /// IntersectFound/IntersectNotFound reply.
    PeerTipRead {
        peer: String,
        slot: u64,
        hash_hex: String,
        block_no: u64,
    },
    /// Per-peer dial / handshake / tip-read failed; the per-peer
    /// task exits. Sibling peers continue.
    PeerDialFailed {
        peer: String,
        kind: PeerDialFailureKind,
        detail: String,
    },
    /// All per-peer tasks complete; aggregate counts recorded.
    /// `admission_enabled` is ALWAYS false in this cluster — the
    /// admission path is RO-LIVE-05 / PHASE4-N-M-LEDGER-SEED.
    WireSmokeComplete {
        admission_enabled: bool,
        peer_count_ok: u32,
        peer_count_failed: u32,
    },
    /// Binary is shutting down; reason recorded.
    NodeShutdown {
        reason: WireOnlyShutdownReason,
    },
}

/// Closed mode tag. The wire-only mode is the only mode this
/// cluster ships. The `Admission` discriminant is the placeholder
/// for the future RO-LIVE-05 cluster — when the binary is invoked
/// in `--mode admission` without a ledger seed, it emits
/// `NodeStarted { mode: WireOnly, ... }` (we DO NOT claim
/// admission mode if the binary fails-closed before entering it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeTag {
    WireOnly,
}

/// Closed shutdown-reason sum. Every variant maps to a known
/// closed-discriminant condition; no `String`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireOnlyShutdownReason {
    /// All peers completed their tip-read; clean exit.
    TipReadComplete,
    /// SIGINT / SIGTERM received during the run.
    SignalReceived,
    /// At least one peer failed; exit non-zero.
    PeerDialFailure,
    /// Admission mode requested but ledger seed prerequisite
    /// missing; binary fail-closed before any dial.
    LedgerSeedUnavailable,
}

/// Closed per-peer failure-kind sum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerDialFailureKind {
    TcpConnectFailed,
    HandshakeRejected,
    TipReadTimeout,
    TipReadProtocolError,
    OrchestratorDropped,
}

impl LiveLogEvent {
    /// Stable discriminator string emitted as the JSON `event`
    /// field. The set is closed — adding a variant means adding
    /// a discriminator + updating the CI gate.
    pub fn discriminator(&self) -> &'static str {
        match self {
            Self::NodeStarted { .. } => "node_started",
            Self::PeerDialStarted { .. } => "peer_dial_started",
            Self::HandshakeOk { .. } => "handshake_ok",
            Self::PeerTipRead { .. } => "peer_tip_read",
            Self::PeerDialFailed { .. } => "peer_dial_failed",
            Self::WireSmokeComplete { .. } => "wire_smoke_complete",
            Self::NodeShutdown { .. } => "node_shutdown",
        }
    }
}

impl ModeTag {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WireOnly => "wire_only",
        }
    }
}

impl WireOnlyShutdownReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TipReadComplete => "tip_read_complete",
            Self::SignalReceived => "signal_received",
            Self::PeerDialFailure => "peer_dial_failure",
            Self::LedgerSeedUnavailable => "ledger_seed_unavailable",
        }
    }
}

impl PeerDialFailureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TcpConnectFailed => "tcp_connect_failed",
            Self::HandshakeRejected => "handshake_rejected",
            Self::TipReadTimeout => "tip_read_timeout",
            Self::TipReadProtocolError => "tip_read_protocol_error",
            Self::OrchestratorDropped => "orchestrator_dropped",
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn discriminator_round_trips_for_each_variant() {
        let cases: Vec<(LiveLogEvent, &'static str)> = vec![
            (
                LiveLogEvent::NodeStarted {
                    mode: ModeTag::WireOnly,
                    peer_count: 0,
                },
                "node_started",
            ),
            (
                LiveLogEvent::PeerDialStarted {
                    peer: "127.0.0.1:0".to_string(),
                },
                "peer_dial_started",
            ),
            (
                LiveLogEvent::HandshakeOk {
                    peer: "p".to_string(),
                    negotiated_version: 14,
                },
                "handshake_ok",
            ),
            (
                LiveLogEvent::PeerTipRead {
                    peer: "p".to_string(),
                    slot: 1,
                    hash_hex: "00".to_string(),
                    block_no: 2,
                },
                "peer_tip_read",
            ),
            (
                LiveLogEvent::PeerDialFailed {
                    peer: "p".to_string(),
                    kind: PeerDialFailureKind::TcpConnectFailed,
                    detail: "x".to_string(),
                },
                "peer_dial_failed",
            ),
            (
                LiveLogEvent::WireSmokeComplete {
                    admission_enabled: false,
                    peer_count_ok: 0,
                    peer_count_failed: 0,
                },
                "wire_smoke_complete",
            ),
            (
                LiveLogEvent::NodeShutdown {
                    reason: WireOnlyShutdownReason::TipReadComplete,
                },
                "node_shutdown",
            ),
        ];
        for (e, expected) in cases {
            assert_eq!(e.discriminator(), expected);
        }
    }

    /// Compile-time exhaustiveness: if a variant is added, this
    /// match fails to compile until updated.
    #[test]
    fn live_log_event_match_is_exhaustive() {
        let e = LiveLogEvent::NodeShutdown {
            reason: WireOnlyShutdownReason::TipReadComplete,
        };
        let _: &str = match &e {
            LiveLogEvent::NodeStarted { .. } => "node_started",
            LiveLogEvent::PeerDialStarted { .. } => "peer_dial_started",
            LiveLogEvent::HandshakeOk { .. } => "handshake_ok",
            LiveLogEvent::PeerTipRead { .. } => "peer_tip_read",
            LiveLogEvent::PeerDialFailed { .. } => "peer_dial_failed",
            LiveLogEvent::WireSmokeComplete { .. } => "wire_smoke_complete",
            LiveLogEvent::NodeShutdown { .. } => "node_shutdown",
        };
    }

    #[test]
    fn mode_tag_only_carries_wire_only_in_this_cluster() {
        // Compile-time confirmation: ModeTag has exactly one
        // variant. Future admission cluster extends additively.
        let m = ModeTag::WireOnly;
        match m {
            ModeTag::WireOnly => {}
        }
        assert_eq!(m.as_str(), "wire_only");
    }
}
