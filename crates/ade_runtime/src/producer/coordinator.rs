// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN pure-state-machine coordinator for live producer-mode
//! (PHASE4-N-Q S2).
//!
//! The coordinator orchestrates the slot loop + forge requests + peer
//! lifecycle. It is **GREEN** by construction:
//!
//! - No I/O, no wall clock, no RNG, no `HashMap`/`HashSet`, no floats.
//! - **No secret-key material** in `CoordinatorState`. Forge is modeled
//!   as a closed `Effect::RequestForge` emitted to the RED producer
//!   shell; the shell signs and returns
//!   `Event::ForgeSucceeded`/`ForgeFailed`/`ForgeNotLeader`. This
//!   preserves the project's true-tier key-custody boundary
//!   (DC-CRYPTO-04 / 05 carry; T-KEY-01).
//!
//! See `docs/clusters/PHASE4-N-Q/cluster.md` §1 for the primary
//! invariant and §5 for the hard prohibitions:
//! - **N9.** No `KesSecret` / `VrfSigningKey` / `ColdSigningKey` field
//!   on `CoordinatorState`. Mechanically grep-asserted by a new CI
//!   guard.
//! - **N15.** No socket addresses in `ProducerLogEvent` (the replayable
//!   stream). `PeerId` is opaque `u64`.
//! - **N16.** No real-time replay claim. Replay-equivalence is over
//!   canonical slot-tick + forge-result event streams (DC-PROD-02).
//!
//! S2 scope: slot loop + forge orchestration + peer lifecycle tracking
//! (lifecycle only — per-peer protocol driving lands in S4).

use std::collections::BTreeMap;

use crate::producer::producer_log::{
    ForgeFailureReason, PeerDisconnectReason, PeerId, ProducerLogEvent, ShutdownReason,
    SlotMissedReason,
};

// =========================================================================
// Public configuration types
// =========================================================================

/// Genesis-derived constants. All non-secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenesisAnchor {
    pub network_magic: u32,
    pub slot_zero_time_unix_ms: u64,
    pub slot_length_ms: u64,
    pub slots_per_kes_period: u64,
    /// Slot number where KES period 0 begins; usually 0.
    pub kes_anchor_slot: u64,
    /// Maximum KES period this op-cert covers (Sum6KES → 64 periods
    /// from `kes_start_period`; final period = start + 63).
    pub kes_max_period: u32,
}

/// **Public** opcert metadata that crosses the GREEN/RED boundary.
/// Carries no secret material; the cold-key signature and KES vkey
/// are public on-chain values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpCertPublicMetadata {
    pub kes_vkey: [u8; 32],
    pub kes_start_period: u32,
    pub sequence_number: u64,
    pub cold_vkey_hash: [u8; 28],
}

/// Opaque ledger-snapshot reference. The RED shell maintains a map
/// `LedgerSnapshotRef → actual LedgerState`. The coordinator never
/// sees the snapshot bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LedgerSnapshotRef(pub u64);

/// Chain tip — slot + 32-byte block hash + block height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainTip {
    pub slot: u64,
    pub block_hash: [u8; 32],
    pub block_number: u64,
}

/// Initial-state config passed to `coordinator_init`.
///
/// **Hard prohibition (N9):** this struct contains no signing-key
/// fields. Adding one would couple GREEN state to RED custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoordinatorConfig {
    pub genesis_anchor: GenesisAnchor,
    pub opcert_meta: OpCertPublicMetadata,
    pub initial_chain_tip: Option<ChainTip>,
    pub initial_ledger_snapshot_ref: LedgerSnapshotRef,
    pub broadcast_queue_limit: usize,
    pub peer_limit: usize,
}

// =========================================================================
// CoordinatorState — the pure data shape
// =========================================================================

/// Per-peer lifecycle record. S4 will extend this with per-peer N2N
/// protocol state; S2 tracks only lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerLifecycle {
    pub connected_at_slot: u64,
    pub chain_sync_version: u32,
    pub block_fetch_version: u32,
}

/// The pure coordinator state.
///
/// **Hard prohibition (N9):** no `KesSecret` / `VrfSigningKey` /
/// `ColdSigningKey` field. CI guard
/// (`ci/ci_check_producer_coordinator_no_secrets.sh`, added in this
/// slice) grep-asserts the absence of these types in this file's
/// production scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinatorState {
    // Genesis + opcert public metadata
    pub genesis_anchor: GenesisAnchor,
    pub opcert_meta: OpCertPublicMetadata,

    // Slot-loop state
    /// The highest slot for which a `SlotTick` event has been
    /// processed. Monotonic by invariant — `coordinator_step` errors
    /// on backwards ticks.
    pub last_slot_tick: Option<u64>,
    /// Slot for which a `RequestForge` effect has been emitted but
    /// no matching `ForgeSucceeded`/`Failed`/`NotLeader` has been
    /// returned yet. `None` means no forge is in flight.
    pub pending_forge_slot: Option<u64>,

    // Chain state
    pub chain_tip: Option<ChainTip>,
    pub ledger_snapshot_ref: LedgerSnapshotRef,

    // Peer lifecycle (S4 extends with per-peer protocol state)
    pub peers: BTreeMap<PeerId, PeerLifecycle>,
    pub peer_id_counter: u64,

    // Broadcast queue tracking (RED maintains the actual queue;
    // coordinator tracks the count as a defense-in-depth limit)
    pub broadcast_queue_size: usize,
    pub broadcast_queue_limit: usize,
    pub peer_limit: usize,

    // Shutdown flag
    pub shutdown_in_progress: bool,
}

impl CoordinatorState {
    /// Compute the KES evolution index to sign a block at `slot` with.
    ///
    /// The slot's ABSOLUTE KES period is `(slot - kes_anchor_slot) /
    /// slots_per_kes_period`. The op-cert anchors the KES key's evolution 0 at its
    /// `kes_start_period` (also absolute), so the value RETURNED is the RELATIVE
    /// evolution index `absolute_period - kes_start_period` -- exactly what
    /// `ProducerShell::init`'s `current_period` and the `Sum6KES` `sign_kes` /
    /// header-period field consume (OP-OPS-04: the raw key evolution index is NEVER
    /// the absolute period). Returns `None` if the slot is under the KES anchor,
    /// below the op-cert's start period (key not yet valid), or beyond the key's
    /// covered window (`> kes_max_period` evolutions -> key exhausted). A
    /// from-genesis op-cert has `kes_start_period == 0`, so this is behaviour-
    /// identical to the prior absolute `[0, kes_max_period]` bound there; a
    /// real-chain op-cert (e.g. `kes_start_period = 885` on a chain at absolute
    /// period 888) yields the small relative index (3) the signer actually needs --
    /// previously this returned `None` (888 > kes_max_period), silently blocking
    /// ALL block production on any non-from-genesis chain.
    pub fn kes_period_for_slot(&self, slot: u64) -> Option<u32> {
        if slot < self.genesis_anchor.kes_anchor_slot {
            return None;
        }
        let offset = slot - self.genesis_anchor.kes_anchor_slot;
        let absolute_period = offset / self.genesis_anchor.slots_per_kes_period;
        let start = self.opcert_meta.kes_start_period as u64;
        if absolute_period < start {
            return None;
        }
        let evolution = absolute_period - start;
        if evolution > self.genesis_anchor.kes_max_period as u64 {
            return None;
        }
        Some(evolution as u32)
    }
}

// =========================================================================
// CoordinatorEvent / CoordinatorEffect / CoordinatorError
// =========================================================================

/// Forged-block public projection. Crosses the GREEN/RED boundary in
/// place of `ade_ledger::producer::self_accept::AcceptedBlock`. The
/// RED shell constructs this from a self-accepted `AcceptedBlock`
/// (the only constructor path); the coordinator passes it through to
/// the `BroadcastBlock` effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgedBlockArtifact {
    pub slot: u64,
    pub hash: [u8; 32],
    pub bytes: Vec<u8>,
}

/// Inputs the coordinator processes. Closed enum; closed payload
/// fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinatorEvent {
    /// External slot ticker advanced.
    SlotTick { slot: u64 },
    /// RED shell handled a `RequestForge` and produced an
    /// `AcceptedBlock`.
    ForgeSucceeded {
        slot: u64,
        artifact: ForgedBlockArtifact,
    },
    /// RED shell ran the leader check and we are not the leader
    /// for this slot. No block to broadcast; not an error.
    ForgeNotLeader {
        slot: u64,
        vrf_output_fingerprint: [u8; 8],
    },
    /// RED shell could not produce a block. Structured reason.
    ForgeFailed {
        slot: u64,
        reason: ForgeFailureReason,
    },
    /// A peer's N2N handshake completed.
    PeerConnected {
        peer_id: PeerId,
        chain_sync_version: u32,
        block_fetch_version: u32,
    },
    /// A peer's connection terminated.
    PeerDisconnected {
        peer_id: PeerId,
        reason: PeerDisconnectReason,
    },
    /// The RED shell installed a new ledger snapshot; subsequent
    /// `RequestForge` effects should reference this ref.
    LedgerSnapshotUpdated { ref_: LedgerSnapshotRef },
    /// RED shell drained the broadcast queue by `count` entries.
    BroadcastDrained { count: usize },
    /// Coordinator should begin shutdown.
    Shutdown { reason: ShutdownReason },
}

/// Outputs the RED shell consumes. Closed enum; closed payload
/// fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinatorEffect {
    /// Ask the RED shell to forge a block at this slot. The shell
    /// uses its custodied keys; coordinator never sees them.
    RequestForge {
        slot: u64,
        kes_period: u32,
        ledger_snapshot_ref: LedgerSnapshotRef,
        chain_tip: Option<ChainTip>,
    },
    /// Hand a forged block to the broadcast queue.
    BroadcastBlock { artifact: ForgedBlockArtifact },
    /// Append a closed-vocabulary event to the producer evidence log.
    LogEvidence { event: ProducerLogEvent },
}

/// Closed error surface for `coordinator_step`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinatorError {
    /// `SlotTick { slot }` arrived with `slot < last_slot_tick`.
    SlotDrift { from: u64, to: u64 },
    /// Broadcast queue at limit before a new emit; fail-closed per
    /// N6 / CN-PROD-02.
    BroadcastFull {
        size: usize,
        limit: usize,
    },
    /// `PeerDisconnected` / per-peer effect referenced an unknown
    /// `PeerId`.
    UnknownPeer { peer_id: PeerId },
    /// KES period for the requested slot is out of range
    /// (below anchor or past key max).
    KesPeriodOutOfRange {
        slot: u64,
        max_period: u32,
    },
    /// `ForgeSucceeded`/`Failed`/`NotLeader` arrived for a slot that
    /// is not currently in flight, or while no forge was pending.
    UnexpectedForgeResult {
        slot: u64,
        pending: Option<u64>,
    },
    /// `PeerConnected` would exceed `peer_limit`.
    PeerLimitExceeded { peer_limit: usize },
    /// Coordinator is in shutdown; only `Shutdown` events are
    /// idempotently accepted.
    ShutdownInProgress,
}

// =========================================================================
// Constructors + transitions
// =========================================================================

/// Construct the initial coordinator state from config. Pure.
pub fn coordinator_init(cfg: CoordinatorConfig) -> (CoordinatorState, Vec<CoordinatorEffect>) {
    let state = CoordinatorState {
        genesis_anchor: cfg.genesis_anchor,
        opcert_meta: cfg.opcert_meta,
        last_slot_tick: None,
        pending_forge_slot: None,
        chain_tip: cfg.initial_chain_tip,
        ledger_snapshot_ref: cfg.initial_ledger_snapshot_ref,
        peers: BTreeMap::new(),
        peer_id_counter: 0,
        broadcast_queue_size: 0,
        broadcast_queue_limit: cfg.broadcast_queue_limit,
        peer_limit: cfg.peer_limit,
        shutdown_in_progress: false,
    };
    let effects = vec![CoordinatorEffect::LogEvidence {
        event: ProducerLogEvent::CoordinatorStarted {
            network_magic: cfg.genesis_anchor.network_magic,
            kes_anchor_slot: cfg.genesis_anchor.kes_anchor_slot,
            slots_per_kes_period: cfg.genesis_anchor.slots_per_kes_period,
        },
    }];
    (state, effects)
}

/// Process one coordinator event. Pure; no I/O; no panics.
///
/// Returns the new state + the ordered effects RED should apply.
/// Failure surfaces a closed `CoordinatorError` variant.
pub fn coordinator_step(
    state: CoordinatorState,
    event: CoordinatorEvent,
) -> Result<(CoordinatorState, Vec<CoordinatorEffect>), CoordinatorError> {
    if state.shutdown_in_progress {
        match event {
            CoordinatorEvent::Shutdown { .. } => {
                // Idempotent — already shutting down.
                return Ok((state, Vec::new()));
            }
            _ => return Err(CoordinatorError::ShutdownInProgress),
        }
    }

    let mut state = state;
    let mut effects = Vec::new();

    match event {
        CoordinatorEvent::SlotTick { slot } => {
            handle_slot_tick(&mut state, slot, &mut effects)?;
        }
        CoordinatorEvent::ForgeSucceeded { slot, artifact } => {
            handle_forge_succeeded(&mut state, slot, artifact, &mut effects)?;
        }
        CoordinatorEvent::ForgeNotLeader {
            slot,
            vrf_output_fingerprint,
        } => {
            handle_forge_not_leader(&mut state, slot, vrf_output_fingerprint, &mut effects)?;
        }
        CoordinatorEvent::ForgeFailed { slot, reason } => {
            handle_forge_failed(&mut state, slot, reason, &mut effects)?;
        }
        CoordinatorEvent::PeerConnected {
            peer_id,
            chain_sync_version,
            block_fetch_version,
        } => {
            handle_peer_connected(
                &mut state,
                peer_id,
                chain_sync_version,
                block_fetch_version,
                &mut effects,
            )?;
        }
        CoordinatorEvent::PeerDisconnected { peer_id, reason } => {
            handle_peer_disconnected(&mut state, peer_id, reason, &mut effects)?;
        }
        CoordinatorEvent::LedgerSnapshotUpdated { ref_ } => {
            state.ledger_snapshot_ref = ref_;
        }
        CoordinatorEvent::BroadcastDrained { count } => {
            state.broadcast_queue_size = state.broadcast_queue_size.saturating_sub(count);
        }
        CoordinatorEvent::Shutdown { reason } => {
            state.shutdown_in_progress = true;
            effects.push(CoordinatorEffect::LogEvidence {
                event: ProducerLogEvent::CoordinatorShutdown { reason },
            });
        }
    }

    Ok((state, effects))
}

// =========================================================================
// Internal handlers
// =========================================================================

fn handle_slot_tick(
    state: &mut CoordinatorState,
    slot: u64,
    effects: &mut Vec<CoordinatorEffect>,
) -> Result<(), CoordinatorError> {
    // Backwards-tick is a hard error.
    if let Some(prev) = state.last_slot_tick {
        if slot <= prev {
            return Err(CoordinatorError::SlotDrift {
                from: prev,
                to: slot,
            });
        }
    }

    // KES period for this slot.
    let kes_period = state
        .kes_period_for_slot(slot)
        .ok_or(CoordinatorError::KesPeriodOutOfRange {
            slot,
            max_period: state.genesis_anchor.kes_max_period,
        })?;

    // If a forge is still pending for an older slot when the new tick
    // arrives, that result is stale — emit SlotMissed and discard the
    // pending marker. The RED shell may still return its result
    // later, but `handle_forge_*` will treat it as UnexpectedForgeResult.
    if let Some(pending) = state.pending_forge_slot {
        if pending < slot {
            effects.push(CoordinatorEffect::LogEvidence {
                event: ProducerLogEvent::SlotMissed {
                    from_slot: pending,
                    to_slot: slot,
                    reason: SlotMissedReason::ForgeResultStaleAtNewTick,
                },
            });
            state.pending_forge_slot = None;
        }
    }

    // Emit SlotTick log event.
    effects.push(CoordinatorEffect::LogEvidence {
        event: ProducerLogEvent::SlotTick { slot, kes_period },
    });

    // Emit RequestForge effect (RED shell decides leader-check + forge).
    effects.push(CoordinatorEffect::RequestForge {
        slot,
        kes_period,
        ledger_snapshot_ref: state.ledger_snapshot_ref,
        chain_tip: state.chain_tip,
    });

    state.last_slot_tick = Some(slot);
    state.pending_forge_slot = Some(slot);

    Ok(())
}

fn handle_forge_succeeded(
    state: &mut CoordinatorState,
    slot: u64,
    artifact: ForgedBlockArtifact,
    effects: &mut Vec<CoordinatorEffect>,
) -> Result<(), CoordinatorError> {
    // Must match the pending slot.
    match state.pending_forge_slot {
        Some(p) if p == slot => {}
        other => {
            return Err(CoordinatorError::UnexpectedForgeResult {
                slot,
                pending: other,
            })
        }
    }

    // Stale-at-arrival check: if slot tick has advanced past this slot,
    // discard the result (the broadcast would now be against an out-of-
    // date chain tip). Emit SlotMissed evidence.
    if let Some(last_tick) = state.last_slot_tick {
        if last_tick > slot {
            effects.push(CoordinatorEffect::LogEvidence {
                event: ProducerLogEvent::SlotMissed {
                    from_slot: slot,
                    to_slot: last_tick,
                    reason: SlotMissedReason::ForgeResultStaleAtArrival,
                },
            });
            state.pending_forge_slot = None;
            return Ok(());
        }
    }

    // Broadcast-queue fail-closed gate.
    if state.broadcast_queue_size >= state.broadcast_queue_limit {
        return Err(CoordinatorError::BroadcastFull {
            size: state.broadcast_queue_size,
            limit: state.broadcast_queue_limit,
        });
    }

    // Update chain tip.
    state.chain_tip = Some(ChainTip {
        slot: artifact.slot,
        block_hash: artifact.hash,
        block_number: state
            .chain_tip
            .map(|t| t.block_number + 1)
            .unwrap_or(0),
    });

    // Emit BlockForged log event.
    effects.push(CoordinatorEffect::LogEvidence {
        event: ProducerLogEvent::BlockForged {
            slot: artifact.slot,
            hash: artifact.hash,
            bytes_len: artifact.bytes.len() as u32,
        },
    });

    // Emit BroadcastBlock effect.
    effects.push(CoordinatorEffect::BroadcastBlock { artifact });
    state.broadcast_queue_size = state.broadcast_queue_size.saturating_add(1);

    // Clear pending.
    state.pending_forge_slot = None;

    Ok(())
}

fn handle_forge_not_leader(
    state: &mut CoordinatorState,
    slot: u64,
    vrf_output_fingerprint: [u8; 8],
    effects: &mut Vec<CoordinatorEffect>,
) -> Result<(), CoordinatorError> {
    match state.pending_forge_slot {
        Some(p) if p == slot => {}
        other => {
            return Err(CoordinatorError::UnexpectedForgeResult {
                slot,
                pending: other,
            })
        }
    }
    effects.push(CoordinatorEffect::LogEvidence {
        event: ProducerLogEvent::LeaderCheckOutcome {
            slot,
            is_leader: false,
            vrf_output_fingerprint,
        },
    });
    state.pending_forge_slot = None;
    Ok(())
}

fn handle_forge_failed(
    state: &mut CoordinatorState,
    slot: u64,
    reason: ForgeFailureReason,
    effects: &mut Vec<CoordinatorEffect>,
) -> Result<(), CoordinatorError> {
    match state.pending_forge_slot {
        Some(p) if p == slot => {}
        other => {
            return Err(CoordinatorError::UnexpectedForgeResult {
                slot,
                pending: other,
            })
        }
    }
    let missed_reason = match reason {
        ForgeFailureReason::KesPeriodMismatch => SlotMissedReason::ForgeKeyPeriodMismatch,
        ForgeFailureReason::KeyExhausted => SlotMissedReason::ForgeKeyExhausted,
        ForgeFailureReason::SelfAcceptRejected
        | ForgeFailureReason::EmptyMempool
        | ForgeFailureReason::UnsupportedProducerEra
        | ForgeFailureReason::Other => SlotMissedReason::ForgeFailedRejected,
    };
    let last_tick = state.last_slot_tick.unwrap_or(slot);
    effects.push(CoordinatorEffect::LogEvidence {
        event: ProducerLogEvent::SlotMissed {
            from_slot: slot,
            to_slot: last_tick,
            reason: missed_reason,
        },
    });
    state.pending_forge_slot = None;
    Ok(())
}

fn handle_peer_connected(
    state: &mut CoordinatorState,
    peer_id: PeerId,
    chain_sync_version: u32,
    block_fetch_version: u32,
    effects: &mut Vec<CoordinatorEffect>,
) -> Result<(), CoordinatorError> {
    if state.peers.len() >= state.peer_limit {
        return Err(CoordinatorError::PeerLimitExceeded {
            peer_limit: state.peer_limit,
        });
    }
    let connected_at_slot = state.last_slot_tick.unwrap_or(0);
    state.peers.insert(
        peer_id,
        PeerLifecycle {
            connected_at_slot,
            chain_sync_version,
            block_fetch_version,
        },
    );
    state.peer_id_counter = state.peer_id_counter.max(peer_id.0 + 1);
    effects.push(CoordinatorEffect::LogEvidence {
        event: ProducerLogEvent::HandshakeOk {
            peer_id,
            chain_sync_version,
            block_fetch_version,
            connected_at_slot,
        },
    });
    Ok(())
}

fn handle_peer_disconnected(
    state: &mut CoordinatorState,
    peer_id: PeerId,
    reason: PeerDisconnectReason,
    effects: &mut Vec<CoordinatorEffect>,
) -> Result<(), CoordinatorError> {
    if state.peers.remove(&peer_id).is_none() {
        return Err(CoordinatorError::UnknownPeer { peer_id });
    }
    effects.push(CoordinatorEffect::LogEvidence {
        event: ProducerLogEvent::PeerDisconnect { peer_id, reason },
    });
    Ok(())
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn test_cfg() -> CoordinatorConfig {
        CoordinatorConfig {
            genesis_anchor: GenesisAnchor {
                network_magic: 1,
                slot_zero_time_unix_ms: 1_000_000,
                slot_length_ms: 1000,
                slots_per_kes_period: 129_600,
                kes_anchor_slot: 0,
                kes_max_period: 63,
            },
            opcert_meta: OpCertPublicMetadata {
                kes_vkey: [0x42; 32],
                kes_start_period: 0,
                sequence_number: 0,
                cold_vkey_hash: [0x11; 28],
            },
            initial_chain_tip: None,
            initial_ledger_snapshot_ref: LedgerSnapshotRef(1),
            broadcast_queue_limit: 4,
            peer_limit: 2,
        }
    }

    fn synthetic_artifact(slot: u64, hash_byte: u8, len: usize) -> ForgedBlockArtifact {
        ForgedBlockArtifact {
            slot,
            hash: [hash_byte; 32],
            bytes: vec![hash_byte; len],
        }
    }

    #[test]
    fn kes_period_for_slot_anchors_relative_to_opcert_start_period() {
        // Regression for the BA02 block-production blocker: on a real chain the
        // op-cert anchors the KES key at an ABSOLUTE start period (e.g. 885), and the
        // gate must return the RELATIVE evolution index (absolute - start), bounded by
        // the key's covered window -- NOT the raw absolute period (which exceeds
        // kes_max_period and previously returned None for EVERY real-chain slot,
        // silently blocking all forging).
        const SPK: u64 = 129_600;
        let mut cfg = test_cfg();
        cfg.genesis_anchor.kes_max_period = 62; // preview maxKESEvolutions
        cfg.opcert_meta.kes_start_period = 885; // a real op-cert's start period
        let (state, _) = coordinator_init(cfg);

        // A live epoch-1332 slot: absolute KES period 888 -> relative evolution 3
        // (this returned None before the fix -> forge never KES-signed).
        assert_eq!(state.kes_period_for_slot(115_092_757), Some(3));
        // The op-cert's start period -> evolution 0.
        assert_eq!(state.kes_period_for_slot(885 * SPK), Some(0));
        // One period before the start -> key not yet valid.
        assert_eq!(state.kes_period_for_slot(884 * SPK), None);
        // The last covered period (start + kes_max_period) -> the final evolution.
        assert_eq!(state.kes_period_for_slot((885 + 62) * SPK), Some(62));
        // One period past the covered window -> key exhausted.
        assert_eq!(state.kes_period_for_slot((885 + 63) * SPK), None);

        // From-genesis behaviour (kes_start_period == 0) is preserved exactly.
        let (gs, _) = coordinator_init(test_cfg());
        assert_eq!(gs.kes_period_for_slot(0), Some(0));
        assert_eq!(gs.kes_period_for_slot(3 * SPK), Some(3));
    }

    #[test]
    fn init_emits_started_event_and_zero_other_effects() {
        let (state, effects) = coordinator_init(test_cfg());
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            CoordinatorEffect::LogEvidence {
                event: ProducerLogEvent::CoordinatorStarted { .. },
            }
        ));
        assert_eq!(state.last_slot_tick, None);
        assert_eq!(state.pending_forge_slot, None);
        assert_eq!(state.peers.len(), 0);
        assert_eq!(state.broadcast_queue_size, 0);
        assert!(!state.shutdown_in_progress);
    }

    #[test]
    fn slot_tick_emits_request_forge_and_log() {
        let (state, _) = coordinator_init(test_cfg());
        let (state2, effects) =
            coordinator_step(state, CoordinatorEvent::SlotTick { slot: 5 }).unwrap();
        assert_eq!(state2.last_slot_tick, Some(5));
        assert_eq!(state2.pending_forge_slot, Some(5));
        // Effects: LogEvidence(SlotTick) + RequestForge
        assert_eq!(effects.len(), 2);
        assert!(matches!(
            effects[0],
            CoordinatorEffect::LogEvidence {
                event: ProducerLogEvent::SlotTick {
                    slot: 5,
                    kes_period: 0
                },
            }
        ));
        assert!(matches!(
            effects[1],
            CoordinatorEffect::RequestForge {
                slot: 5,
                kes_period: 0,
                ..
            }
        ));
    }

    #[test]
    fn forge_succeeded_emits_broadcast_and_log() {
        let (state, _) = coordinator_init(test_cfg());
        let (state, _) = coordinator_step(state, CoordinatorEvent::SlotTick { slot: 5 }).unwrap();
        let artifact = synthetic_artifact(5, 0xAB, 200);
        let (state, effects) = coordinator_step(
            state,
            CoordinatorEvent::ForgeSucceeded {
                slot: 5,
                artifact: artifact.clone(),
            },
        )
        .unwrap();
        assert_eq!(state.pending_forge_slot, None);
        assert_eq!(state.broadcast_queue_size, 1);
        assert_eq!(state.chain_tip.unwrap().slot, 5);
        // Effects: LogEvidence(BlockForged) + BroadcastBlock
        assert_eq!(effects.len(), 2);
        assert!(matches!(
            effects[1],
            CoordinatorEffect::BroadcastBlock { ref artifact }
                if artifact.slot == 5 && artifact.bytes.len() == 200
        ));
    }

    #[test]
    fn forge_not_leader_emits_log_and_clears_pending() {
        let (state, _) = coordinator_init(test_cfg());
        let (state, _) = coordinator_step(state, CoordinatorEvent::SlotTick { slot: 7 }).unwrap();
        let (state, effects) = coordinator_step(
            state,
            CoordinatorEvent::ForgeNotLeader {
                slot: 7,
                vrf_output_fingerprint: [0xCD; 8],
            },
        )
        .unwrap();
        assert_eq!(state.pending_forge_slot, None);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            CoordinatorEffect::LogEvidence {
                event: ProducerLogEvent::LeaderCheckOutcome {
                    slot: 7,
                    is_leader: false,
                    ..
                },
            }
        ));
    }

    #[test]
    fn forge_failed_emits_slot_missed_with_mapped_reason() {
        let (state, _) = coordinator_init(test_cfg());
        let (state, _) = coordinator_step(state, CoordinatorEvent::SlotTick { slot: 3 }).unwrap();
        let (state, effects) = coordinator_step(
            state,
            CoordinatorEvent::ForgeFailed {
                slot: 3,
                reason: ForgeFailureReason::SelfAcceptRejected,
            },
        )
        .unwrap();
        assert_eq!(state.pending_forge_slot, None);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            CoordinatorEffect::LogEvidence {
                event: ProducerLogEvent::SlotMissed {
                    reason: SlotMissedReason::ForgeFailedRejected,
                    ..
                },
            }
        ));
    }

    #[test]
    fn backwards_slot_tick_errors_slot_drift() {
        let (state, _) = coordinator_init(test_cfg());
        let (state, _) = coordinator_step(state, CoordinatorEvent::SlotTick { slot: 10 }).unwrap();
        let err =
            coordinator_step(state, CoordinatorEvent::SlotTick { slot: 5 }).unwrap_err();
        assert!(matches!(
            err,
            CoordinatorError::SlotDrift { from: 10, to: 5 }
        ));
    }

    #[test]
    fn forge_result_for_unknown_slot_errors_unexpected() {
        let (state, _) = coordinator_init(test_cfg());
        let err = coordinator_step(
            state,
            CoordinatorEvent::ForgeSucceeded {
                slot: 5,
                artifact: synthetic_artifact(5, 0, 0),
            },
        )
        .unwrap_err();
        assert!(matches!(
            err,
            CoordinatorError::UnexpectedForgeResult {
                slot: 5,
                pending: None
            }
        ));
    }

    #[test]
    fn stale_forge_result_after_new_tick_drops_with_slot_missed() {
        let (state, _) = coordinator_init(test_cfg());
        let (state, _) = coordinator_step(state, CoordinatorEvent::SlotTick { slot: 1 }).unwrap();
        // Now a NEW SlotTick at 2 arrives while forge for slot 1 was
        // still pending — coordinator emits SlotMissed
        // (ForgeResultStaleAtNewTick) and proceeds with slot 2.
        let (state, _) = coordinator_step(state, CoordinatorEvent::SlotTick { slot: 2 }).unwrap();
        assert_eq!(state.pending_forge_slot, Some(2));
        // RED shell finally returns the slot=1 result; coordinator
        // errors UnexpectedForgeResult (pending is now slot=2).
        let err = coordinator_step(
            state,
            CoordinatorEvent::ForgeSucceeded {
                slot: 1,
                artifact: synthetic_artifact(1, 0, 0),
            },
        )
        .unwrap_err();
        assert!(matches!(err, CoordinatorError::UnexpectedForgeResult { slot: 1, .. }));
    }

    #[test]
    fn kes_period_out_of_range_errors() {
        let (state, _) = coordinator_init(test_cfg());
        // kes_max_period = 63; slots_per_kes_period = 129_600 →
        // total slots = 64 * 129_600 = 8_294_400. Slot >= that is
        // out of range.
        let err = coordinator_step(
            state,
            CoordinatorEvent::SlotTick { slot: 8_294_400 },
        )
        .unwrap_err();
        assert!(matches!(
            err,
            CoordinatorError::KesPeriodOutOfRange { .. }
        ));
    }

    #[test]
    fn peer_lifecycle_tracks_connect_and_disconnect() {
        let (state, _) = coordinator_init(test_cfg());
        let (state, effects) = coordinator_step(
            state,
            CoordinatorEvent::PeerConnected {
                peer_id: PeerId(1),
                chain_sync_version: 9,
                block_fetch_version: 9,
            },
        )
        .unwrap();
        assert_eq!(state.peers.len(), 1);
        assert!(matches!(
            effects[0],
            CoordinatorEffect::LogEvidence {
                event: ProducerLogEvent::HandshakeOk { peer_id: PeerId(1), .. },
            }
        ));
        let (state, effects) = coordinator_step(
            state,
            CoordinatorEvent::PeerDisconnected {
                peer_id: PeerId(1),
                reason: PeerDisconnectReason::Graceful,
            },
        )
        .unwrap();
        assert_eq!(state.peers.len(), 0);
        assert!(matches!(
            effects[0],
            CoordinatorEffect::LogEvidence {
                event: ProducerLogEvent::PeerDisconnect { peer_id: PeerId(1), .. },
            }
        ));
    }

    #[test]
    fn unknown_peer_disconnect_errors() {
        let (state, _) = coordinator_init(test_cfg());
        let err = coordinator_step(
            state,
            CoordinatorEvent::PeerDisconnected {
                peer_id: PeerId(99),
                reason: PeerDisconnectReason::Graceful,
            },
        )
        .unwrap_err();
        assert!(matches!(
            err,
            CoordinatorError::UnknownPeer { peer_id: PeerId(99) }
        ));
    }

    #[test]
    fn peer_limit_exceeded_errors() {
        let (state, _) = coordinator_init(test_cfg());
        let (state, _) = coordinator_step(
            state,
            CoordinatorEvent::PeerConnected {
                peer_id: PeerId(1),
                chain_sync_version: 9,
                block_fetch_version: 9,
            },
        )
        .unwrap();
        let (state, _) = coordinator_step(
            state,
            CoordinatorEvent::PeerConnected {
                peer_id: PeerId(2),
                chain_sync_version: 9,
                block_fetch_version: 9,
            },
        )
        .unwrap();
        // peer_limit = 2; third connect fails.
        let err = coordinator_step(
            state,
            CoordinatorEvent::PeerConnected {
                peer_id: PeerId(3),
                chain_sync_version: 9,
                block_fetch_version: 9,
            },
        )
        .unwrap_err();
        assert!(matches!(
            err,
            CoordinatorError::PeerLimitExceeded { .. }
        ));
    }

    #[test]
    fn broadcast_full_errors_when_queue_at_limit() {
        let mut cfg = test_cfg();
        cfg.broadcast_queue_limit = 1;
        let (state, _) = coordinator_init(cfg);
        // First forge succeeds; queue fills to 1.
        let (state, _) = coordinator_step(state, CoordinatorEvent::SlotTick { slot: 1 }).unwrap();
        let (state, _) = coordinator_step(
            state,
            CoordinatorEvent::ForgeSucceeded {
                slot: 1,
                artifact: synthetic_artifact(1, 1, 100),
            },
        )
        .unwrap();
        assert_eq!(state.broadcast_queue_size, 1);
        // Second forge succeeds in principle, but the broadcast gate
        // returns BroadcastFull.
        let (state, _) = coordinator_step(state, CoordinatorEvent::SlotTick { slot: 2 }).unwrap();
        let err = coordinator_step(
            state,
            CoordinatorEvent::ForgeSucceeded {
                slot: 2,
                artifact: synthetic_artifact(2, 2, 100),
            },
        )
        .unwrap_err();
        assert!(matches!(err, CoordinatorError::BroadcastFull { .. }));
    }

    #[test]
    fn broadcast_drained_event_decrements_queue() {
        let mut cfg = test_cfg();
        cfg.broadcast_queue_limit = 2;
        let (state, _) = coordinator_init(cfg);
        let (state, _) = coordinator_step(state, CoordinatorEvent::SlotTick { slot: 1 }).unwrap();
        let (state, _) = coordinator_step(
            state,
            CoordinatorEvent::ForgeSucceeded {
                slot: 1,
                artifact: synthetic_artifact(1, 0, 100),
            },
        )
        .unwrap();
        assert_eq!(state.broadcast_queue_size, 1);
        let (state, _) =
            coordinator_step(state, CoordinatorEvent::BroadcastDrained { count: 1 }).unwrap();
        assert_eq!(state.broadcast_queue_size, 0);
    }

    #[test]
    fn shutdown_event_sets_flag_and_emits_log() {
        let (state, _) = coordinator_init(test_cfg());
        let (state, effects) = coordinator_step(
            state,
            CoordinatorEvent::Shutdown {
                reason: ShutdownReason::SignalReceived,
            },
        )
        .unwrap();
        assert!(state.shutdown_in_progress);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            CoordinatorEffect::LogEvidence {
                event: ProducerLogEvent::CoordinatorShutdown {
                    reason: ShutdownReason::SignalReceived,
                },
            }
        ));
    }

    #[test]
    fn post_shutdown_events_error_except_repeated_shutdown() {
        let (state, _) = coordinator_init(test_cfg());
        let (state, _) = coordinator_step(
            state,
            CoordinatorEvent::Shutdown {
                reason: ShutdownReason::ScheduleEnded,
            },
        )
        .unwrap();
        // Repeated shutdown is idempotent.
        let (state, effects) = coordinator_step(
            state,
            CoordinatorEvent::Shutdown {
                reason: ShutdownReason::SignalReceived,
            },
        )
        .unwrap();
        assert_eq!(effects.len(), 0);
        // SlotTick fails.
        let err =
            coordinator_step(state, CoordinatorEvent::SlotTick { slot: 1 }).unwrap_err();
        assert!(matches!(err, CoordinatorError::ShutdownInProgress));
    }

    #[test]
    fn replay_byte_identity_across_two_runs() {
        // DC-PROD-02: fixed event stream → byte-identical
        // (effects, log events) across runs.
        let cfg = test_cfg();
        let events = vec![
            CoordinatorEvent::PeerConnected {
                peer_id: PeerId(1),
                chain_sync_version: 9,
                block_fetch_version: 9,
            },
            CoordinatorEvent::SlotTick { slot: 1 },
            CoordinatorEvent::ForgeNotLeader {
                slot: 1,
                vrf_output_fingerprint: [0xAA; 8],
            },
            CoordinatorEvent::SlotTick { slot: 2 },
            CoordinatorEvent::ForgeSucceeded {
                slot: 2,
                artifact: synthetic_artifact(2, 0x11, 50),
            },
            CoordinatorEvent::BroadcastDrained { count: 1 },
            CoordinatorEvent::SlotTick { slot: 3 },
            CoordinatorEvent::ForgeFailed {
                slot: 3,
                reason: ForgeFailureReason::EmptyMempool,
            },
            CoordinatorEvent::PeerDisconnected {
                peer_id: PeerId(1),
                reason: PeerDisconnectReason::Graceful,
            },
            CoordinatorEvent::Shutdown {
                reason: ShutdownReason::ScheduleEnded,
            },
        ];

        let run = |events: &[CoordinatorEvent]| -> (CoordinatorState, Vec<CoordinatorEffect>) {
            let (mut state, mut all_effects) = coordinator_init(cfg);
            for e in events {
                let (s, fx) = coordinator_step(state, e.clone()).unwrap();
                state = s;
                all_effects.extend(fx);
            }
            (state, all_effects)
        };

        let (s_a, fx_a) = run(&events);
        let (s_b, fx_b) = run(&events);
        // Coordinator state equal.
        assert_eq!(s_a, s_b, "replay state diverged");
        // Effects equal.
        assert_eq!(fx_a.len(), fx_b.len(), "effect count diverged");
        for (a, b) in fx_a.iter().zip(fx_b.iter()) {
            assert_eq!(a, b, "effect divergence");
        }
        // Log-event JSON serialization byte-identical across runs
        // (DC-PROD-02 closes-of-the-loop assertion).
        let log_json_a = serialize_log_events(&fx_a);
        let log_json_b = serialize_log_events(&fx_b);
        assert_eq!(log_json_a, log_json_b);
    }

    fn serialize_log_events(effects: &[CoordinatorEffect]) -> String {
        let mut out = String::new();
        for e in effects {
            if let CoordinatorEffect::LogEvidence { event } = e {
                out.push_str(&serde_json::to_string(event).unwrap());
                out.push('\n');
            }
        }
        out
    }
}
