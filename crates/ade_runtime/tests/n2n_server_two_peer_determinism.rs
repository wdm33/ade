// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// PHASE4-N-G S6 — Multi-peer determinism test.
//
// OQ-4 resolution: per-peer state is independent; cross-peer
// coordination is only through the shared &ServedChainSnapshot.
// Driving two synthetic peers in parallel against one orchestrator
// MUST yield per-session transcripts identical to running each peer
// alone.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::collections::BTreeMap;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::producer::{self_accept, AcceptedBlock, ServedChainSnapshot};
use ade_ledger::state::LedgerState;
use ade_network::codec::chain_sync::{encode_chain_sync_message, ChainSyncMessage};
use ade_network::codec::version::{BlockFetchVersion, ChainSyncVersion};
use ade_runtime::network::n2n_server::{dispatch_chain_sync_frame, PerPeerN2nServerState};
use ade_runtime::producer::broadcast::BroadcastQueue;
use ade_runtime::producer::broadcast_to_served::drain_and_admit;
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

fn build_accepted_one() -> AcceptedBlock {
    let (c, view) = corpus_view();
    let mut idxs: Vec<usize> = (0..c.blocks.len()).collect();
    idxs.sort_by_key(|&i| {
        let env = decode_block_envelope(&c.blocks[i]).expect("env");
        env.block_end - env.block_start
    });
    let bytes = c.blocks[idxs[0]].clone();
    let schedule = schedule();
    let mut ledger = LedgerState::new(CardanoEra::Conway);
    ledger.epoch_state.epoch = EPOCH_576;
    let mut chain_dep = PraosChainDepState::empty();
    chain_dep.epoch_nonce = Nonce(Hash32(c.epoch_nonce));
    chain_dep.evolving_nonce = Nonce(Hash32(c.epoch_nonce));
    self_accept(&bytes, &ledger, &chain_dep, &schedule, &view).expect("self_accept")
}

fn build_snapshot_with_one_block() -> ServedChainSnapshot {
    let accepted = build_accepted_one();
    let mut queue = BroadcastQueue::new(2);
    queue.enqueue(accepted).expect("enqueue");
    let (snap, _q, _d) = drain_and_admit(ServedChainSnapshot::new(), queue).expect("admit");
    snap
}

/// Drive one peer through a given sequence of inbound chain-sync
/// messages, returning its outgoing frame sequence.
fn run_peer(
    cs_v: ChainSyncVersion,
    bf_v: BlockFetchVersion,
    snap: &ServedChainSnapshot,
    inputs: &[ChainSyncMessage],
) -> Vec<Vec<u8>> {
    let mut state = PerPeerN2nServerState::new(cs_v, bf_v);
    let mut frames = Vec::new();
    for m in inputs {
        let frame = encode_chain_sync_message(m);
        let (s2, out, done) = dispatch_chain_sync_frame(state, &frame, snap).expect("dispatch");
        state = s2;
        if let Some(o) = out {
            frames.push(o);
        }
        if done {
            break;
        }
    }
    frames
}

#[test]
fn two_synthetic_peers_preserve_per_session_transcripts() {
    let snap = build_snapshot_with_one_block();
    let cs_v = ChainSyncVersion::new(9);
    let bf_v = BlockFetchVersion::new(9);
    let inputs = vec![ChainSyncMessage::RequestNext, ChainSyncMessage::Done];

    // Solo runs — establish per-peer reference transcripts.
    let solo_a = run_peer(cs_v, bf_v, &snap, &inputs);
    let solo_b = run_peer(cs_v, bf_v, &snap, &inputs);

    // Parallel drive: interleave peer-A and peer-B steps against the
    // SAME orchestrator instance (here represented as two independent
    // PerPeerN2nServerState values sharing &snap — which is exactly
    // the orchestrator's coordination model).
    let mut state_a = PerPeerN2nServerState::new(cs_v, bf_v);
    let mut state_b = PerPeerN2nServerState::new(cs_v, bf_v);
    let mut parallel_a = Vec::new();
    let mut parallel_b = Vec::new();
    for m in &inputs {
        let frame = encode_chain_sync_message(m);
        // Interleave: A, then B, both consuming the same input.
        let (sa, out_a, _da) = dispatch_chain_sync_frame(state_a, &frame, &snap).unwrap();
        let (sb, out_b, _db) = dispatch_chain_sync_frame(state_b, &frame, &snap).unwrap();
        state_a = sa;
        state_b = sb;
        if let Some(o) = out_a {
            parallel_a.push(o);
        }
        if let Some(o) = out_b {
            parallel_b.push(o);
        }
    }

    // Per-session transcripts must match solo runs.
    assert_eq!(solo_a, parallel_a, "peer A transcript drifts under parallel drive");
    assert_eq!(solo_b, parallel_b, "peer B transcript drifts under parallel drive");
    assert_eq!(solo_a, solo_b, "two peers running the same inputs must produce the same transcript");
}
