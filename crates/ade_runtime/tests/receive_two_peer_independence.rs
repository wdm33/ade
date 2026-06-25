// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// PHASE4-N-H S4 — Receive-side multi-peer independence.
//
// Two peers share one ChainDb. Each receives the same Conway-576
// corpus block via RollForward + BlockDelivered. Assert:
//   * Both peers see Admitted (the second peer's put_block is
//     idempotent on byte-identity).
//   * Per-peer transcripts equal their solo-run transcripts (no
//     cross-peer state contamination).

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::collections::BTreeMap;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
use ade_ledger::block_validity::decode_block;
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::receive::{ReceiveEffect, ReceiveState};
use ade_ledger::state::LedgerState;
use ade_network::codec::block_fetch::{encode_block_fetch_message, BlockFetchMessage};
use ade_network::codec::chain_sync::{
    encode_chain_sync_message, ChainSyncMessage, Point as CsPoint, Tip as CsTip,
};
use ade_network::codec::version::{BlockFetchVersion, ChainSyncVersion};
use ade_runtime::chaindb::{ChainDb, InMemoryChainDb};
use ade_runtime::receive::{
    dispatch_block_fetch_inbound, dispatch_chain_sync_inbound, ChainDbWriter,
    PerPeerReceiveState,
};
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
            randomness_stabilisation_window_slots: None,
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

fn fresh_state(eta0: [u8; 32]) -> ReceiveState {
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

fn build_per_peer(eta0: [u8; 32]) -> PerPeerReceiveState {
    PerPeerReceiveState::new(
        fresh_state(eta0),
        ChainSyncVersion::new(9),
        BlockFetchVersion::new(9),
    )
}

/// Drive one peer through (RollForward → BlockDelivered) for the
/// given block and return the sequence of effects.
fn drive_peer(
    state: &mut PerPeerReceiveState,
    db: &InMemoryChainDb,
    bytes: &[u8],
) -> Vec<ReceiveEffect> {
    let (_c, view) = corpus_view();
    let schedule = schedule();
    let decoded = decode_block(bytes).expect("decode");
    let mut effects = Vec::new();

    let mut writer = ChainDbWriter::new(db);
    let cs_frame = encode_chain_sync_message(&ChainSyncMessage::RollForward {
        header: bytes.to_vec(),
        tip: CsTip {
            point: CsPoint::Block {
                slot: decoded.header_input.slot,
                hash: decoded.block_hash.clone(),
            },
            block_no: decoded.header_input.block_no.0,
        },
    });
    let e = dispatch_chain_sync_inbound(state, &cs_frame, &mut writer, &schedule, &view)
        .expect("cache")
        .expect("Some");
    effects.push(e);

    let bf_frame = encode_block_fetch_message(&BlockFetchMessage::Block {
        bytes: bytes.to_vec(),
    });
    let e = dispatch_block_fetch_inbound(state, &bf_frame, &mut writer, &schedule, &view)
        .expect("admit")
        .expect("Some");
    effects.push(e);
    effects
}

#[test]
fn two_peers_admit_same_block_into_shared_chaindb() {
    let (c, _view) = corpus_view();
    let bytes = pick_lightest(&c);

    let db = InMemoryChainDb::new();
    let mut peer_a = build_per_peer(c.epoch_nonce);
    let mut peer_b = build_per_peer(c.epoch_nonce);

    let effects_a = drive_peer(&mut peer_a, &db, &bytes);
    let effects_b = drive_peer(&mut peer_b, &db, &bytes);

    // Both peers must see Cached then Admitted.
    for effects in [&effects_a, &effects_b] {
        assert_eq!(effects.len(), 2);
        assert!(matches!(effects[0], ReceiveEffect::Cached { .. }));
        assert!(matches!(effects[1], ReceiveEffect::Admitted { .. }));
    }

    // Shared ChainDb must hold the block at the expected key.
    let decoded = decode_block(&bytes).expect("decode");
    let stored = db
        .get_block_by_hash(&decoded.block_hash)
        .expect("get")
        .expect("present");
    assert_eq!(stored.slot, decoded.header_input.slot);
    assert_eq!(stored.bytes, bytes);
}

#[test]
fn two_peers_per_session_transcripts_match_solo_runs() {
    let (c, _view) = corpus_view();
    let bytes = pick_lightest(&c);

    // Solo runs — establish per-peer reference transcripts.
    let solo = |eta0: [u8; 32]| -> Vec<ReceiveEffect> {
        let db = InMemoryChainDb::new();
        let mut peer = build_per_peer(eta0);
        drive_peer(&mut peer, &db, &bytes)
    };
    let solo_a = solo(c.epoch_nonce);
    let solo_b = solo(c.epoch_nonce);

    // Parallel: shared ChainDb, two peers.
    let db = InMemoryChainDb::new();
    let mut peer_a = build_per_peer(c.epoch_nonce);
    let mut peer_b = build_per_peer(c.epoch_nonce);
    let par_a = drive_peer(&mut peer_a, &db, &bytes);
    let par_b = drive_peer(&mut peer_b, &db, &bytes);

    assert_eq!(solo_a, par_a, "peer A transcript drifts under shared-db drive");
    assert_eq!(solo_b, par_b, "peer B transcript drifts under shared-db drive");
    assert_eq!(solo_a, solo_b, "same inputs → same transcript");
}
