// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration test — PHASE4-N-K S4 (DC-NODE-01).
//!
//! A peer session that hits a decode error halts only its own
//! session; the orchestrator continues processing slot ticks and
//! a second peer's frames.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_ledger::block_validity::decode_block;
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::producer::ServedChainSnapshot;
use ade_ledger::receive::ReceiveState;
use ade_ledger::state::LedgerState;
use ade_network::codec::block_fetch::{encode_block_fetch_message, BlockFetchMessage};
use ade_network::codec::chain_sync::{
    encode_chain_sync_message, ChainSyncMessage, Point as CsPoint, Tip as CsTip,
};
use ade_network::codec::version::{BlockFetchVersion, ChainSyncVersion};
use ade_runtime::chaindb::InMemoryChainDb;
use ade_runtime::orchestrator::event::{
    OrchestratorEffect, OrchestratorEvent, PeerHaltReason, PeerId, PeerRole,
};
use ade_runtime::orchestrator::state::{OrchestratorState, PerPeerReceiveVersions};
use ade_runtime::orchestrator::step;
use ade_runtime::receive::ChainDbWriter;
use ade_runtime::rollback::cadence::SnapshotCadence;
use ade_testkit::validity::ConwayValidityCorpus;
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

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

#[test]
fn peer_session_isolation_holds_under_failure() {
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

    let mut state =
        OrchestratorState::new(fresh_receive(corpus.epoch_nonce), SnapshotCadence::DEFAULT);
    // Install two peers.
    for pid in [PeerId(1), PeerId(2)] {
        state.per_peer_receive.insert(
            pid,
            PerPeerReceiveVersions {
                chain_sync_version: ChainSyncVersion::new(9),
                block_fetch_version: BlockFetchVersion::new(9),
            },
        );
    }
    let db = InMemoryChainDb::new();
    let mut writer = ChainDbWriter::new(&db);
    let served = ServedChainSnapshot::new();
    let sched = schedule();

    // Peer 1 sends a malformed frame → must be halted.
    let effects_1 = step(
        &mut state,
        OrchestratorEvent::PeerChainSyncFrame {
            peer_id: PeerId(1),
            bytes: vec![0xFFu8; 8],
        },
        &mut writer,
        &served,
        &sched,
        &view,
    )
    .expect("step1");
    assert!(
        effects_1.iter().any(|e| matches!(
            e,
            OrchestratorEffect::PeerSessionHalted {
                peer_id: PeerId(1),
                reason: PeerHaltReason::ChainSyncDecodeError,
            }
        )),
        "peer 1 must be halted on decode error"
    );

    // Slot tick: orchestrator MUST advance normally.
    let tick_effects = step(
        &mut state,
        OrchestratorEvent::SlotTick {
            slot_millis: 1000,
            slot: SlotNo(EPOCH_577_START - MAINNET_EPOCH_LENGTH),
        },
        &mut writer,
        &served,
        &sched,
        &view,
    )
    .expect("tick");
    assert!(tick_effects.is_empty());
    assert!(state.last_observed_slot.is_some());

    // Peer 2 still able to dispatch valid frames.
    let cs_effects = step(
        &mut state,
        OrchestratorEvent::PeerChainSyncFrame {
            peer_id: PeerId(2),
            bytes: cs_frame.clone(),
        },
        &mut writer,
        &served,
        &sched,
        &view,
    )
    .expect("cs2");
    assert!(
        !cs_effects.iter().any(|e| matches!(e, OrchestratorEffect::PeerSessionHalted { .. })),
        "peer 2 must succeed despite peer 1's failure"
    );
    let bf_effects = step(
        &mut state,
        OrchestratorEvent::PeerBlockFetchFrame {
            peer_id: PeerId(2),
            bytes: bf_frame.clone(),
        },
        &mut writer,
        &served,
        &sched,
        &view,
    )
    .expect("bf2");
    assert!(
        bf_effects.iter().any(|e| matches!(e, OrchestratorEffect::AdmittedBlock { .. })),
        "peer 2 must admit the block"
    );
}

#[test]
fn peer_session_per_peer_state_does_not_cross() {
    // Two peers see the same RollForward; the orchestrator's
    // pending-header cache is in the shared canonical ReceiveState.
    // A subsequent BlockFetch from either peer should admit (single
    // shared admission ledger).
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

    let mut state =
        OrchestratorState::new(fresh_receive(corpus.epoch_nonce), SnapshotCadence::DEFAULT);
    for pid in [PeerId(1), PeerId(2)] {
        state.per_peer_receive.insert(
            pid,
            PerPeerReceiveVersions {
                chain_sync_version: ChainSyncVersion::new(9),
                block_fetch_version: BlockFetchVersion::new(9),
            },
        );
    }
    let db = InMemoryChainDb::new();
    let mut writer = ChainDbWriter::new(&db);
    let served = ServedChainSnapshot::new();
    let sched = schedule();

    // Peer 1 caches the header.
    step(
        &mut state,
        OrchestratorEvent::PeerChainSyncFrame {
            peer_id: PeerId(1),
            bytes: cs_frame.clone(),
        },
        &mut writer,
        &served,
        &sched,
        &view,
    )
    .expect("peer1 cs");

    // Peer 2 delivers the body. Single shared admission.
    let effects = step(
        &mut state,
        OrchestratorEvent::PeerBlockFetchFrame {
            peer_id: PeerId(2),
            bytes: bf_frame.clone(),
        },
        &mut writer,
        &served,
        &sched,
        &view,
    )
    .expect("peer2 bf");
    assert!(
        effects.iter().any(|e| matches!(e, OrchestratorEffect::AdmittedBlock { .. })),
        "block must admit when one peer sent the header and another sent the body"
    );
}

#[test]
fn peer_disconnect_removes_only_that_peer() {
    let (corpus, view) = corpus_view();
    let mut state =
        OrchestratorState::new(fresh_receive(corpus.epoch_nonce), SnapshotCadence::DEFAULT);
    let db = InMemoryChainDb::new();
    let mut writer = ChainDbWriter::new(&db);
    let served = ServedChainSnapshot::new();
    let sched = schedule();

    for pid in [PeerId(1), PeerId(2), PeerId(3)] {
        step(
            &mut state,
            OrchestratorEvent::PeerConnected {
                peer_id: pid,
                chain_sync_version: ChainSyncVersion::new(11),
                block_fetch_version: BlockFetchVersion::new(11),
                role: PeerRole::UpstreamClient,
            },
            &mut writer,
            &served,
            &sched,
            &view,
        )
        .expect("connect");
    }
    assert_eq!(state.per_peer_receive.len(), 3);

    step(
        &mut state,
        OrchestratorEvent::PeerDisconnected { peer_id: PeerId(2) },
        &mut writer,
        &served,
        &sched,
        &view,
    )
    .expect("disc");
    assert_eq!(state.per_peer_receive.len(), 2);
    assert!(state.per_peer_receive.contains_key(&PeerId(1)));
    assert!(!state.per_peer_receive.contains_key(&PeerId(2)));
    assert!(state.per_peer_receive.contains_key(&PeerId(3)));
}
