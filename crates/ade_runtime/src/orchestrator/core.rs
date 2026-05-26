// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN orchestrator core reducer (PHASE4-N-K S2).
//!
//! Pure `step` function: `(state, event) -> Result<Vec<Effect>,
//! Error>`. Composes the receive-side N-H dispatchers, the server-
//! side N-G dispatchers, and the snapshot cadence policy. Never
//! reads wall-clock; never imports tokio; never bypasses an
//! authority path.
//!
//! Authority routing per event variant:
//!
//! | Event                                  | Authority called                                        |
//! |----------------------------------------|---------------------------------------------------------|
//! | `PeerChainSyncFrame`                   | `receive::dispatch_chain_sync_inbound`                  |
//! | `PeerBlockFetchFrame`                  | `receive::dispatch_block_fetch_inbound`                 |
//! | `PeerN2nServerChainSyncFrame`          | `network::n2n_server::dispatch_chain_sync_frame`        |
//! | `PeerN2nServerBlockFetchFrame`         | `network::n2n_server::dispatch_block_fetch_frame`       |
//! | `SlotTick`                             | state-only (records last_observed_slot)                 |
//! | `PeerConnected` / `PeerDisconnected`   | state-only (install/remove per-peer map entry)          |
//! | `Shutdown`                             | sets shutdown_requested + emits ShutdownAcknowledged    |
//!
//! After every receive-side `Admitted` effect, the orchestrator
//! consults `should_snapshot_after_block` and emits
//! `CaptureSnapshot { slot }` if the cadence policy returns true.
//! The persistent writer (S3) consumes that effect.

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_ledger::producer::ServedChainSnapshot;
use ade_ledger::receive::{ChainDbWrite, ReceiveEffect, ReceiveError};

use crate::receive::orchestrator::{
    dispatch_block_fetch_inbound, dispatch_chain_sync_inbound, PerPeerReceiveState,
    ReceiveDispatchError,
};
use crate::network::n2n_server::{
    dispatch_block_fetch_frame, dispatch_chain_sync_frame, DispatchError as ServerDispatchError,
};
use crate::rollback::cadence::should_snapshot_after_block;

use super::event::{
    AuthorityFatalKind, OrchestratorEffect, OrchestratorError, OrchestratorEvent, PeerHaltReason,
    PeerId,
};
use super::state::{OrchestratorState, PerPeerReceiveVersions};

/// One step of the orchestrator. Pure: same inputs → same
/// `Vec<Effect>`; no `tokio::*`, no `SystemTime`, no `rand`.
///
/// Returns `Ok(Vec<Effect>)` on success; `Err(OrchestratorError)`
/// only for authority-fatal conditions that must halt the binary
/// (DC-NODE-04). Per-peer-fatal errors are folded into
/// `OrchestratorEffect::PeerSessionHalted` effects.
pub fn step<W: ChainDbWrite>(
    state: &mut OrchestratorState,
    event: OrchestratorEvent,
    chain_write: &mut W,
    served_snapshot: &ServedChainSnapshot,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<Vec<OrchestratorEffect>, OrchestratorError> {
    if state.shutdown_requested && !matches!(event, OrchestratorEvent::Shutdown) {
        // After Shutdown, refuse to advance — runner is draining.
        return Ok(Vec::new());
    }

    match event {
        OrchestratorEvent::SlotTick { slot, .. } => {
            state.last_observed_slot = Some(slot);
            Ok(Vec::new())
        }

        OrchestratorEvent::PeerConnected {
            peer_id,
            chain_sync_version,
            block_fetch_version,
            role,
        } => {
            match role {
                super::event::PeerRole::UpstreamClient => {
                    state.install_receive_peer(
                        peer_id,
                        PerPeerReceiveVersions {
                            chain_sync_version,
                            block_fetch_version,
                        },
                    );
                }
                super::event::PeerRole::DownstreamServer => {
                    let server = crate::network::n2n_server::PerPeerN2nServerState::new(
                        chain_sync_version,
                        block_fetch_version,
                    );
                    state.install_server_peer(peer_id, server);
                }
            }
            Ok(Vec::new())
        }

        OrchestratorEvent::PeerDisconnected { peer_id } => {
            state.remove_peer(peer_id);
            Ok(Vec::new())
        }

        OrchestratorEvent::PeerChainSyncFrame { peer_id, bytes } => {
            handle_receive_chain_sync(
                state,
                peer_id,
                &bytes,
                chain_write,
                era_schedule,
                ledger_view,
            )
        }

        OrchestratorEvent::PeerBlockFetchFrame { peer_id, bytes } => {
            handle_receive_block_fetch(
                state,
                peer_id,
                &bytes,
                chain_write,
                era_schedule,
                ledger_view,
            )
        }

        OrchestratorEvent::PeerN2nServerChainSyncFrame { peer_id, bytes } => {
            handle_server_chain_sync(state, peer_id, &bytes, served_snapshot)
        }

        OrchestratorEvent::PeerN2nServerBlockFetchFrame { peer_id, bytes } => {
            handle_server_block_fetch(state, peer_id, &bytes, served_snapshot)
        }

        OrchestratorEvent::Shutdown => {
            state.shutdown_requested = true;
            Ok(vec![OrchestratorEffect::ShutdownAcknowledged])
        }

        OrchestratorEvent::OutboundKeepAlive { peer_id: _ } => {
            // Recorded as a no-op effect set at the orchestrator core;
            // the session-side keep-alive frame encoding lives at the
            // session layer (future cluster). The Clock seam end-to-end
            // exercise (DC-SESS-05) is what this event proves.
            Ok(Vec::new())
        }
    }
}

fn handle_receive_chain_sync<W: ChainDbWrite>(
    state: &mut OrchestratorState,
    peer_id: PeerId,
    bytes: &[u8],
    chain_write: &mut W,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<Vec<OrchestratorEffect>, OrchestratorError> {
    let versions = match state.per_peer_receive.get(&peer_id) {
        Some(v) => *v,
        None => {
            return Ok(vec![OrchestratorEffect::PeerSessionHalted {
                peer_id,
                reason: PeerHaltReason::PeerUnknown,
            }]);
        }
    };
    let mut per_peer = PerPeerReceiveState::new(
        state.receive_state.clone(),
        versions.chain_sync_version,
        versions.block_fetch_version,
    );
    let result = dispatch_chain_sync_inbound(
        &mut per_peer,
        bytes,
        chain_write,
        era_schedule,
        ledger_view,
    );
    finish_receive(state, peer_id, per_peer, result)
}

fn handle_receive_block_fetch<W: ChainDbWrite>(
    state: &mut OrchestratorState,
    peer_id: PeerId,
    bytes: &[u8],
    chain_write: &mut W,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<Vec<OrchestratorEffect>, OrchestratorError> {
    let versions = match state.per_peer_receive.get(&peer_id) {
        Some(v) => *v,
        None => {
            return Ok(vec![OrchestratorEffect::PeerSessionHalted {
                peer_id,
                reason: PeerHaltReason::PeerUnknown,
            }]);
        }
    };
    let mut per_peer = PerPeerReceiveState::new(
        state.receive_state.clone(),
        versions.chain_sync_version,
        versions.block_fetch_version,
    );
    let result = dispatch_block_fetch_inbound(
        &mut per_peer,
        bytes,
        chain_write,
        era_schedule,
        ledger_view,
    );
    finish_receive(state, peer_id, per_peer, result)
}

fn finish_receive(
    state: &mut OrchestratorState,
    peer_id: PeerId,
    per_peer: PerPeerReceiveState,
    result: Result<Option<ReceiveEffect>, ReceiveDispatchError>,
) -> Result<Vec<OrchestratorEffect>, OrchestratorError> {
    match result {
        Ok(Some(effect)) => {
            // Successful dispatch: promote the per-peer ReceiveState back
            // to the canonical seat.
            state.receive_state = per_peer.receive_state;
            let mut effects = Vec::new();
            match &effect {
                ReceiveEffect::Admitted { slot, hash } => {
                    effects.push(OrchestratorEffect::AdmittedBlock {
                        slot: *slot,
                        hash: hash.clone(),
                    });
                    // Cadence consult.
                    if let Some(block_no) = state.receive_state.chain_dep.last_block_no {
                        if should_snapshot_after_block(
                            *slot,
                            block_no,
                            state.cadence,
                            state.last_persistent_snapshot_slot,
                        ) {
                            effects.push(OrchestratorEffect::CaptureSnapshot { slot: *slot });
                            // Record the schedule advance immediately so a
                            // burst of same-slot events does not fire two
                            // captures. The writer (S3) is idempotent on
                            // the same slot anyway.
                            state.last_persistent_snapshot_slot = Some(*slot);
                        }
                    }
                }
                _ => {}
            }
            Ok(effects)
        }
        Ok(None) => {
            // Non-state-changing frame (e.g., IntersectFound).
            // Promote the per-peer state anyway in case version
            // bookkeeping advanced.
            state.receive_state = per_peer.receive_state;
            Ok(Vec::new())
        }
        Err(err) => {
            // Per-peer-fatal vs authority-fatal classification.
            match err {
                ReceiveDispatchError::ChainSyncDecode(_) => Ok(vec![
                    OrchestratorEffect::PeerSessionHalted {
                        peer_id,
                        reason: PeerHaltReason::ChainSyncDecodeError,
                    },
                ]),
                ReceiveDispatchError::BlockFetchDecode(_) => Ok(vec![
                    OrchestratorEffect::PeerSessionHalted {
                        peer_id,
                        reason: PeerHaltReason::BlockFetchDecodeError,
                    },
                ]),
                ReceiveDispatchError::Receive(rerr) => match rerr {
                    ReceiveError::ChainDb(cw) => {
                        // ChainDb IO is authority-fatal.
                        if matches!(
                            cw,
                            ade_ledger::receive::ChainWriteError::Underlying(
                                ade_ledger::receive::ChainWriteErrorKind::Io
                            )
                        ) {
                            Err(OrchestratorError::AuthorityFatal(
                                AuthorityFatalKind::ChainWriteIo,
                            ))
                        } else {
                            Ok(vec![OrchestratorEffect::PeerSessionHalted {
                                peer_id,
                                reason: PeerHaltReason::ReceiveValidityRejected,
                            }])
                        }
                    }
                    ReceiveError::HeaderBodyMismatch { .. } => {
                        Ok(vec![OrchestratorEffect::PeerSessionHalted {
                            peer_id,
                            reason: PeerHaltReason::ReceiveHeaderBodyMismatch,
                        }])
                    }
                    ReceiveError::Validity(_) => {
                        Ok(vec![OrchestratorEffect::PeerSessionHalted {
                            peer_id,
                            reason: PeerHaltReason::ReceiveValidityRejected,
                        }])
                    }
                    ReceiveError::RollbackOutOfScope { .. } => {
                        Ok(vec![OrchestratorEffect::PeerSessionHalted {
                            peer_id,
                            reason: PeerHaltReason::ReceiveRollbackOutOfScope,
                        }])
                    }
                },
            }
        }
    }
}

fn handle_server_chain_sync(
    state: &mut OrchestratorState,
    peer_id: PeerId,
    bytes: &[u8],
    served_snapshot: &ServedChainSnapshot,
) -> Result<Vec<OrchestratorEffect>, OrchestratorError> {
    let per_peer = match state.per_peer_server.remove(&peer_id) {
        Some(p) => p,
        None => {
            return Ok(vec![OrchestratorEffect::PeerSessionHalted {
                peer_id,
                reason: PeerHaltReason::PeerUnknown,
            }]);
        }
    };
    match dispatch_chain_sync_frame(per_peer, bytes, served_snapshot) {
        Ok((new_state, reply, done)) => {
            state.install_server_peer(peer_id, new_state);
            let mut effects = Vec::new();
            if let Some(bytes) = reply {
                effects.push(OrchestratorEffect::SendToPeer { peer_id, bytes });
            }
            if done {
                state.remove_peer(peer_id);
            }
            Ok(effects)
        }
        Err(err) => Ok(vec![OrchestratorEffect::PeerSessionHalted {
            peer_id,
            reason: server_err_reason_chain_sync(&err),
        }]),
    }
}

fn handle_server_block_fetch(
    state: &mut OrchestratorState,
    peer_id: PeerId,
    bytes: &[u8],
    served_snapshot: &ServedChainSnapshot,
) -> Result<Vec<OrchestratorEffect>, OrchestratorError> {
    let per_peer = match state.per_peer_server.remove(&peer_id) {
        Some(p) => p,
        None => {
            return Ok(vec![OrchestratorEffect::PeerSessionHalted {
                peer_id,
                reason: PeerHaltReason::PeerUnknown,
            }]);
        }
    };
    match dispatch_block_fetch_frame(per_peer, bytes, served_snapshot) {
        Ok((new_state, replies, done)) => {
            state.install_server_peer(peer_id, new_state);
            let mut effects = Vec::with_capacity(replies.len());
            for bytes in replies {
                effects.push(OrchestratorEffect::SendToPeer { peer_id, bytes });
            }
            if done {
                state.remove_peer(peer_id);
            }
            Ok(effects)
        }
        Err(err) => Ok(vec![OrchestratorEffect::PeerSessionHalted {
            peer_id,
            reason: server_err_reason_block_fetch(&err),
        }]),
    }
}

fn server_err_reason_chain_sync(err: &ServerDispatchError) -> PeerHaltReason {
    match err {
        ServerDispatchError::ChainSyncDecode(_) => PeerHaltReason::ServerChainSyncDecodeError,
        ServerDispatchError::BlockFetchDecode(_) => PeerHaltReason::ServerBlockFetchDecodeError,
        ServerDispatchError::ChainSync(_) => PeerHaltReason::ServerChainSyncProtocolError,
        ServerDispatchError::BlockFetch(_) => PeerHaltReason::ServerBlockFetchProtocolError,
    }
}

fn server_err_reason_block_fetch(err: &ServerDispatchError) -> PeerHaltReason {
    server_err_reason_chain_sync(err)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_core::consensus::praos_state::PraosChainDepState;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
    use ade_ledger::block_validity::decode_block;
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::receive::ReceiveState;
    use ade_ledger::state::LedgerState;
    use ade_network::codec::block_fetch::{encode_block_fetch_message, BlockFetchMessage};
    use ade_network::codec::chain_sync::{
        encode_chain_sync_message, ChainSyncMessage, Point as CsPoint, Tip as CsTip,
    };
    use ade_network::codec::version::{BlockFetchVersion, ChainSyncVersion};
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

    use crate::chaindb::InMemoryChainDb;
    use crate::orchestrator::event::{
        OrchestratorEffect, OrchestratorEvent, PeerId, PeerRole,
    };
    use crate::receive::ChainDbWriter;
    use crate::rollback::cadence::SnapshotCadence;

    const EPOCH_576: EpochNo = EpochNo(576);
    const EPOCH_577_START: u64 = 163_900_800;
    const MAINNET_EPOCH_LENGTH: u64 = 432_000;

    fn schedule() -> EraSchedule {
        let start_576 = EPOCH_577_START - MAINNET_EPOCH_LENGTH;
        EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![EraSummary {
                era: CardanoEra::Conway,
                start_slot: SlotNo(start_576),
                start_epoch: EPOCH_576,
                slot_length_ms: 1_000,
                epoch_length_slots: MAINNET_EPOCH_LENGTH as u32,
                safe_zone_slots: MAINNET_EPOCH_LENGTH as u32,
            }],
        )
        .expect("schedule")
    }

    fn corpus_view() -> (ConwayValidityCorpus, PoolDistrView) {
        let c = ConwayValidityCorpus::load().expect("corpus");
        let total = c.pd_total_active_stake;
        let asc = ActiveSlotsCoeff {
            numer: c.asc.numer as u32,
            denom: c.asc.denom as u32,
        };
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        for (pool_id, p) in &c.pools {
            let scale = total / p.sigma.denom;
            pools.insert(
                Hash28(*pool_id),
                PoolEntry {
                    active_stake: p.sigma.numer * scale,
                    vrf_keyhash: Hash32(p.vrf_keyhash),
                },
            );
        }
        (c, PoolDistrView::new(EPOCH_576, total, asc, pools))
    }

    fn fresh_receive(eta0: [u8; 32]) -> ReceiveState {
        let mut ledger = LedgerState::new(CardanoEra::Conway);
        ledger.epoch_state.epoch = EPOCH_576;
        let mut chain_dep = PraosChainDepState::empty();
        chain_dep.epoch_nonce = Nonce(Hash32(eta0));
        chain_dep.evolving_nonce = Nonce(Hash32(eta0));
        ReceiveState::new(ledger, chain_dep)
    }

    fn pick_lightest(c: &ConwayValidityCorpus) -> Vec<u8> {
        let idx = (0..c.blocks.len())
            .min_by_key(|&i| {
                let env = decode_block_envelope(&c.blocks[i]).expect("env");
                env.block_end - env.block_start
            })
            .expect("non-empty");
        c.blocks[idx].clone()
    }

    fn install_one_receive_peer(state: &mut OrchestratorState, peer: PeerId) {
        state.per_peer_receive.insert(
            peer,
            PerPeerReceiveVersions {
                chain_sync_version: ChainSyncVersion::new(9),
                block_fetch_version: BlockFetchVersion::new(9),
            },
        );
    }

    fn served_empty() -> ServedChainSnapshot {
        ServedChainSnapshot::new()
    }

    #[test]
    fn step_two_runs_produce_byte_identical_effects() {
        let (corpus, view) = corpus_view();
        let bytes = pick_lightest(&corpus);
        let decoded = decode_block(&bytes).expect("decode");
        let cs_frame = encode_chain_sync_message(&ChainSyncMessage::RollForward {
            header: bytes.clone(),
            tip: CsTip {
                point: CsPoint::Block {
                    slot: decoded.header_input.slot,
                    hash: decoded.block_hash.clone(),
                },
                block_no: decoded.header_input.block_no.0,
            },
        });
        let bf_frame = encode_block_fetch_message(&BlockFetchMessage::Block {
            bytes: bytes.clone(),
        });

        let run = || {
            let mut state =
                OrchestratorState::new(fresh_receive(corpus.epoch_nonce), SnapshotCadence::DEFAULT);
            install_one_receive_peer(&mut state, PeerId(1));
            let db = InMemoryChainDb::new();
            let mut writer = ChainDbWriter::new(&db);
            let served = served_empty();
            let mut effects = Vec::new();
            effects.extend(
                step(
                    &mut state,
                    OrchestratorEvent::PeerChainSyncFrame {
                        peer_id: PeerId(1),
                        bytes: cs_frame.clone(),
                    },
                    &mut writer,
                    &served,
                    &schedule(),
                    &view,
                )
                .expect("cs"),
            );
            effects.extend(
                step(
                    &mut state,
                    OrchestratorEvent::PeerBlockFetchFrame {
                        peer_id: PeerId(1),
                        bytes: bf_frame.clone(),
                    },
                    &mut writer,
                    &served,
                    &schedule(),
                    &view,
                )
                .expect("bf"),
            );
            effects
        };
        let a = run();
        let b = run();
        assert_eq!(a, b, "orchestrator step must be deterministic");
    }

    #[test]
    fn step_per_peer_decode_error_isolates() {
        let (corpus, view) = corpus_view();
        let bytes = pick_lightest(&corpus);
        let decoded = decode_block(&bytes).expect("decode");
        let cs_frame = encode_chain_sync_message(&ChainSyncMessage::RollForward {
            header: bytes.clone(),
            tip: CsTip {
                point: CsPoint::Block {
                    slot: decoded.header_input.slot,
                    hash: decoded.block_hash.clone(),
                },
                block_no: decoded.header_input.block_no.0,
            },
        });

        let mut state =
            OrchestratorState::new(fresh_receive(corpus.epoch_nonce), SnapshotCadence::DEFAULT);
        install_one_receive_peer(&mut state, PeerId(1));
        install_one_receive_peer(&mut state, PeerId(2));
        let db = InMemoryChainDb::new();
        let mut writer = ChainDbWriter::new(&db);
        let served = served_empty();

        // Peer 1 sends garbage; peer 2 sends a valid frame.
        let effects_1 = step(
            &mut state,
            OrchestratorEvent::PeerChainSyncFrame {
                peer_id: PeerId(1),
                bytes: vec![0xFFu8; 4],
            },
            &mut writer,
            &served,
            &schedule(),
            &view,
        )
        .expect("step");
        match effects_1.as_slice() {
            [OrchestratorEffect::PeerSessionHalted { peer_id, reason: PeerHaltReason::ChainSyncDecodeError }] => {
                assert_eq!(*peer_id, PeerId(1));
            }
            other => panic!("expected single PeerSessionHalted, got {other:?}"),
        }

        // Peer 1 is no longer in the map structurally; orchestrator
        // would normally have processed a Disconnected event next.
        // For this slice's isolation test, we just verify peer 2's
        // dispatch still succeeds.
        let effects_2 = step(
            &mut state,
            OrchestratorEvent::PeerChainSyncFrame {
                peer_id: PeerId(2),
                bytes: cs_frame.clone(),
            },
            &mut writer,
            &served,
            &schedule(),
            &view,
        )
        .expect("step");
        // Peer 2's frame should produce no halt effect (decode succeeded).
        assert!(
            !effects_2.iter().any(|e| matches!(e, OrchestratorEffect::PeerSessionHalted { .. })),
            "peer 2 must not be halted by peer 1's failure"
        );
    }

    #[test]
    fn step_admit_triggers_capture_snapshot_at_cadence() {
        // Use cadence = every_n_blocks: 1 so every Admitted triggers
        // a CaptureSnapshot effect.
        let (corpus, view) = corpus_view();
        let bytes = pick_lightest(&corpus);
        let decoded = decode_block(&bytes).expect("decode");
        let cs_frame = encode_chain_sync_message(&ChainSyncMessage::RollForward {
            header: bytes.clone(),
            tip: CsTip {
                point: CsPoint::Block {
                    slot: decoded.header_input.slot,
                    hash: decoded.block_hash.clone(),
                },
                block_no: decoded.header_input.block_no.0,
            },
        });
        let bf_frame = encode_block_fetch_message(&BlockFetchMessage::Block {
            bytes: bytes.clone(),
        });

        let mut state = OrchestratorState::new(
            fresh_receive(corpus.epoch_nonce),
            SnapshotCadence { every_n_blocks: 1 },
        );
        install_one_receive_peer(&mut state, PeerId(1));
        let db = InMemoryChainDb::new();
        let mut writer = ChainDbWriter::new(&db);
        let served = served_empty();

        let _ = step(
            &mut state,
            OrchestratorEvent::PeerChainSyncFrame {
                peer_id: PeerId(1),
                bytes: cs_frame.clone(),
            },
            &mut writer,
            &served,
            &schedule(),
            &view,
        )
        .expect("cs");
        let effects = step(
            &mut state,
            OrchestratorEvent::PeerBlockFetchFrame {
                peer_id: PeerId(1),
                bytes: bf_frame.clone(),
            },
            &mut writer,
            &served,
            &schedule(),
            &view,
        )
        .expect("bf");

        let captured = effects
            .iter()
            .find_map(|e| match e {
                OrchestratorEffect::CaptureSnapshot { slot } => Some(*slot),
                _ => None,
            })
            .expect("CaptureSnapshot must fire at cadence=1");
        assert_eq!(captured, decoded.header_input.slot);
    }

    #[test]
    fn step_shutdown_drains_then_halts() {
        let (corpus, view) = corpus_view();
        let mut state =
            OrchestratorState::new(fresh_receive(corpus.epoch_nonce), SnapshotCadence::DEFAULT);
        let db = InMemoryChainDb::new();
        let mut writer = ChainDbWriter::new(&db);
        let served = served_empty();
        let effects = step(
            &mut state,
            OrchestratorEvent::Shutdown,
            &mut writer,
            &served,
            &schedule(),
            &view,
        )
        .expect("shutdown");
        assert_eq!(effects, vec![OrchestratorEffect::ShutdownAcknowledged]);
        assert!(state.is_shutdown_requested());

        // Subsequent SlotTick is suppressed.
        let effects = step(
            &mut state,
            OrchestratorEvent::SlotTick {
                slot_millis: 1,
                slot: SlotNo(42),
            },
            &mut writer,
            &served,
            &schedule(),
            &view,
        )
        .expect("post-shutdown tick");
        assert!(effects.is_empty());
    }

    #[test]
    fn step_peer_connect_disconnect_updates_map() {
        let (corpus, view) = corpus_view();
        let mut state =
            OrchestratorState::new(fresh_receive(corpus.epoch_nonce), SnapshotCadence::DEFAULT);
        let db = InMemoryChainDb::new();
        let mut writer = ChainDbWriter::new(&db);
        let served = served_empty();
        step(
            &mut state,
            OrchestratorEvent::PeerConnected {
                peer_id: PeerId(7),
                chain_sync_version: ChainSyncVersion::new(11),
                block_fetch_version: BlockFetchVersion::new(11),
                role: PeerRole::UpstreamClient,
            },
            &mut writer,
            &served,
            &schedule(),
            &view,
        )
        .expect("connect");
        assert!(state.per_peer_receive.contains_key(&PeerId(7)));
        step(
            &mut state,
            OrchestratorEvent::PeerDisconnected { peer_id: PeerId(7) },
            &mut writer,
            &served,
            &schedule(),
            &view,
        )
        .expect("disconnect");
        assert!(!state.per_peer_receive.contains_key(&PeerId(7)));
    }

    #[test]
    fn step_slot_tick_records_last_observed_slot() {
        let (corpus, view) = corpus_view();
        let mut state =
            OrchestratorState::new(fresh_receive(corpus.epoch_nonce), SnapshotCadence::DEFAULT);
        let db = InMemoryChainDb::new();
        let mut writer = ChainDbWriter::new(&db);
        let served = served_empty();
        let effects = step(
            &mut state,
            OrchestratorEvent::SlotTick {
                slot_millis: 12345,
                slot: SlotNo(999),
            },
            &mut writer,
            &served,
            &schedule(),
            &view,
        )
        .expect("tick");
        assert!(effects.is_empty());
        assert_eq!(state.last_observed_slot, Some(SlotNo(999)));
    }
}
