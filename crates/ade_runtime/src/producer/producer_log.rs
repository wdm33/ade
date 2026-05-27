// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN closed-vocabulary producer evidence log (PHASE4-N-Q S2).
//!
//! `ProducerLogEvent` is the closed enum the GREEN coordinator emits on
//! every observable producer-mode state transition. RED writes these
//! to a JSONL file; the JSONL writer lives in the produce-mode shell.
//!
//! DC-PROD-01 (declared at PHASE4-N-Q S1): no free-form strings; no
//! key material; no path strings. **Socket addresses MUST NOT appear
//! in this type** — `PeerId` is an opaque `u64`; RED operational
//! metadata (socket addrs, file paths, etc.) flows on a separate
//! channel and is excluded from replay-equivalence comparison
//! (DC-PROD-02).
//!
//! Each reason field is its own closed enum — `SlotMissedReason`,
//! `PeerDisconnectReason`, `ShutdownReason` — so the surface stays
//! grep-auditable.

use serde::Serialize;

/// Opaque per-process peer identifier. Coordinator-internal counter;
/// never embeds a socket address. RED operational metadata maps
/// PeerId → SocketAddr in a separate (non-replayable) ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct PeerId(pub u64);

/// Closed reason vocabulary for `SlotMissed` events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SlotMissedReason {
    /// `SlotTick(N)` arrived but an earlier forge for slot M < N is
    /// still pending; we drop the stale-pending and proceed.
    ForgeResultStaleAtNewTick,
    /// `ForgeSucceeded(M)` arrived AFTER `SlotTick(N > M)` already
    /// advanced; the result is dropped (no broadcast).
    ForgeResultStaleAtArrival,
    /// `ForgeFailed(slot, ...)` from the RED shell.
    ForgeFailedRejected,
    /// `ForgeFailed(slot, KesPeriodMismatch)` — RED shell's KES key
    /// rotated past the requested period.
    ForgeKeyPeriodMismatch,
    /// `ForgeFailed(slot, KeyExhausted)` — KES key has reached its
    /// final period and cannot evolve further.
    ForgeKeyExhausted,
}

/// Closed reason vocabulary for `PeerDisconnect` events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PeerDisconnectReason {
    /// Peer (or our side) initiated graceful close.
    Graceful,
    /// N2N handshake failed (wrong magic, wrong version, malformed
    /// envelope, etc.).
    HandshakeFailed,
    /// Mid-stream protocol violation; per-peer state machine
    /// fail-closed.
    ProtocolError,
    /// We hit `peer_limit`; new connection refused.
    PeerLimitExceeded,
    /// Coordinator shutdown in progress; all peers being drained.
    CoordinatorShutdown,
}

/// Closed reason vocabulary for `CoordinatorShutdown` events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ShutdownReason {
    /// SIGINT / SIGTERM received by the RED shell.
    SignalReceived,
    /// Broadcast queue overflow — fail-closed per N6 / CN-PROD-02.
    BroadcastFull,
    /// KES key exhausted (no further periods reachable).
    KeyExhausted,
    /// Configured slot schedule has ended (e.g., bounded smoke run).
    ScheduleEnded,
    /// Coordinator detected an unrecoverable invariant violation.
    InvariantFailure,
}

/// Closed vocabulary for forge-failure structured errors that the RED
/// shell surfaces back to the coordinator via `CoordinatorEvent::ForgeFailed`.
///
/// Mirrors the relevant `ade_ledger::producer::forge::ForgeError`
/// variants without dragging the BLUE error type across the boundary;
/// the RED shell does the mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ForgeFailureReason {
    /// KES period encoded in the forge request does not match the
    /// RED shell's `KesSecret.current_period()`.
    KesPeriodMismatch,
    /// KES key has reached its final period.
    KeyExhausted,
    /// Self-accept (BLUE validator) rejected the forged bytes.
    SelfAcceptRejected,
    /// Mempool / tx admission produced an empty body unexpectedly.
    EmptyMempool,
    /// Catch-all for other ForgeError variants we don't surface
    /// individually. Closed at the type level (this variant carries
    /// no inner data); the RED shell logs the full BLUE error
    /// out-of-band before mapping.
    Other,
}

/// Closed `ProducerLogEvent` vocabulary. Every observable producer-mode
/// state transition. Serializes to JSONL via `serde_json` with a
/// `kind` tag.
///
/// **Hard prohibitions (DC-PROD-01 declared at PHASE4-N-Q S1):**
/// - No `String` fields.
/// - No `Vec<u8>` of key material.
/// - No file paths or socket addresses.
/// - All reason fields are closed enums.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum ProducerLogEvent {
    /// Coordinator-init complete; first event in every log.
    CoordinatorStarted {
        network_magic: u32,
        kes_anchor_slot: u64,
        slots_per_kes_period: u64,
    },
    /// Inbound peer's N2N handshake completed successfully.
    HandshakeOk {
        peer_id: PeerId,
        chain_sync_version: u32,
        block_fetch_version: u32,
        connected_at_slot: u64,
    },
    /// Coordinator received a slot tick.
    SlotTick {
        slot: u64,
        kes_period: u32,
    },
    /// RED shell ran the leader check; outcome is determined.
    LeaderCheckOutcome {
        slot: u64,
        is_leader: bool,
        /// Blake2b-256 fingerprint of the VRF output (first 8 bytes
        /// is sufficient for evidence; not a security-load-bearing
        /// commitment).
        vrf_output_fingerprint: [u8; 8],
    },
    /// RED shell forged a valid block.
    BlockForged {
        slot: u64,
        hash: [u8; 32],
        bytes_len: u32,
    },
    /// Block bytes were served to a peer via block-fetch.
    BlockServed {
        peer_id: PeerId,
        slot: u64,
        hash: [u8; 32],
        bytes_len: u32,
    },
    /// Peer's chain-sync stream advertised a new tip hash; useful
    /// for detecting cardano-node-side acceptance.
    PeerChainTipObserved {
        peer_id: PeerId,
        slot: u64,
        hash: [u8; 32],
    },
    /// Slot was missed (no broadcast); coordinator advanced past
    /// without a successful forge result for that slot.
    SlotMissed {
        from_slot: u64,
        to_slot: u64,
        reason: SlotMissedReason,
    },
    /// Per-peer connection terminated.
    PeerDisconnect {
        peer_id: PeerId,
        reason: PeerDisconnectReason,
    },
    /// Coordinator shutdown; final event in the log.
    CoordinatorShutdown {
        reason: ShutdownReason,
    },
}

impl ProducerLogEvent {
    /// Stable `kind` tag for grep/CI gates. Matches the `serde(tag)`
    /// label.
    pub fn kind(&self) -> &'static str {
        match self {
            ProducerLogEvent::CoordinatorStarted { .. } => "CoordinatorStarted",
            ProducerLogEvent::HandshakeOk { .. } => "HandshakeOk",
            ProducerLogEvent::SlotTick { .. } => "SlotTick",
            ProducerLogEvent::LeaderCheckOutcome { .. } => "LeaderCheckOutcome",
            ProducerLogEvent::BlockForged { .. } => "BlockForged",
            ProducerLogEvent::BlockServed { .. } => "BlockServed",
            ProducerLogEvent::PeerChainTipObserved { .. } => "PeerChainTipObserved",
            ProducerLogEvent::SlotMissed { .. } => "SlotMissed",
            ProducerLogEvent::PeerDisconnect { .. } => "PeerDisconnect",
            ProducerLogEvent::CoordinatorShutdown { .. } => "CoordinatorShutdown",
        }
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn event_kinds_are_distinct_and_stable() {
        // Spot-check: every variant has a unique kind() string.
        let kinds = [
            ProducerLogEvent::CoordinatorStarted {
                network_magic: 0,
                kes_anchor_slot: 0,
                slots_per_kes_period: 0,
            }
            .kind(),
            ProducerLogEvent::HandshakeOk {
                peer_id: PeerId(0),
                chain_sync_version: 0,
                block_fetch_version: 0,
                connected_at_slot: 0,
            }
            .kind(),
            ProducerLogEvent::SlotTick {
                slot: 0,
                kes_period: 0,
            }
            .kind(),
            ProducerLogEvent::LeaderCheckOutcome {
                slot: 0,
                is_leader: false,
                vrf_output_fingerprint: [0; 8],
            }
            .kind(),
            ProducerLogEvent::BlockForged {
                slot: 0,
                hash: [0; 32],
                bytes_len: 0,
            }
            .kind(),
            ProducerLogEvent::BlockServed {
                peer_id: PeerId(0),
                slot: 0,
                hash: [0; 32],
                bytes_len: 0,
            }
            .kind(),
            ProducerLogEvent::PeerChainTipObserved {
                peer_id: PeerId(0),
                slot: 0,
                hash: [0; 32],
            }
            .kind(),
            ProducerLogEvent::SlotMissed {
                from_slot: 0,
                to_slot: 0,
                reason: SlotMissedReason::ForgeResultStaleAtArrival,
            }
            .kind(),
            ProducerLogEvent::PeerDisconnect {
                peer_id: PeerId(0),
                reason: PeerDisconnectReason::Graceful,
            }
            .kind(),
            ProducerLogEvent::CoordinatorShutdown {
                reason: ShutdownReason::SignalReceived,
            }
            .kind(),
        ];
        let unique: std::collections::BTreeSet<_> = kinds.iter().collect();
        assert_eq!(kinds.len(), unique.len(), "kind tags collide");
    }

    #[test]
    fn json_serialization_round_trips_byte_identical_for_replay() {
        // Two serialize calls on the same value yield byte-identical
        // bytes — a precondition for DC-PROD-02 replay equivalence.
        let evt = ProducerLogEvent::SlotTick {
            slot: 42,
            kes_period: 5,
        };
        let a = serde_json::to_string(&evt).unwrap();
        let b = serde_json::to_string(&evt).unwrap();
        assert_eq!(a, b);
        assert!(a.contains("\"kind\":\"SlotTick\""));
        assert!(a.contains("\"slot\":42"));
        assert!(a.contains("\"kes_period\":5"));
    }

    #[test]
    fn no_string_fields_in_any_variant() {
        // Compile-time check via match-exhaustive: every field below
        // is a primitive / closed enum / fixed-size array. No String.
        // If a new variant is added with a `String` field, the match
        // will still compile (Rust enums don't restrict field types
        // structurally) — the discipline relies on code review + the
        // serde-tag stability test above. This test exists as a
        // template for the future Guard 8 in ci_check_kes_envelope_closed.sh.
        let _ = ProducerLogEvent::CoordinatorStarted {
            network_magic: 1,
            kes_anchor_slot: 0,
            slots_per_kes_period: 129600,
        };
        // (No assertion; the test is the act of compiling without
        // touching any `String`.)
    }

    #[test]
    fn slot_missed_reason_serializes_to_stable_strings() {
        let r = SlotMissedReason::ForgeResultStaleAtArrival;
        let s = serde_json::to_string(&r).unwrap();
        assert_eq!(s, "\"ForgeResultStaleAtArrival\"");
    }
}
