// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// PHASE4-N-H S5 — Mechanical cross-impl adapter (CE-N-H-5).
//
// Drives the Conway-576 corpus block-by-block through the full
// receive pipeline (orchestrator dispatch → GREEN adapter → BLUE
// reducer → InMemoryChainDb) and asserts admit + ChainDb-tip
// + byte-identity + ledger-fingerprint-change per block.
//
// Each block is admitted in a fresh state because the corpus is a
// set of independent Conway-576 blocks, not a sequential chain.
// This is the same per-block independence assumption that
// PHASE4-N-G S7's cross_impl_server_pipeline uses on the send side.

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

fn build_per_peer(eta0: [u8; 32]) -> PerPeerReceiveState {
    PerPeerReceiveState::new(
        fresh_state(eta0),
        ChainSyncVersion::new(9),
        BlockFetchVersion::new(9),
    )
}

fn ledger_fp(state: &LedgerState) -> Hash32 {
    ade_ledger::fingerprint::fingerprint(state).combined
}

fn drive_one_block(bytes: &[u8], eta0: [u8; 32]) -> DriveResult {
    let (_c, view) = corpus_view();
    let schedule = schedule();
    let decoded = decode_block(bytes).expect("decode");
    let fresh_fp = ledger_fp(&fresh_state(eta0).ledger);

    let db = InMemoryChainDb::new();
    let mut peer = build_per_peer(eta0);

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
    let bf_frame = encode_block_fetch_message(&BlockFetchMessage::Block {
        bytes: bytes.to_vec(),
    });

    let cache_effect = {
        let mut writer = ChainDbWriter::new(&db);
        dispatch_chain_sync_inbound(&mut peer, &cs_frame, &mut writer, &schedule, &view)
            .expect("cache")
            .expect("Some")
    };
    let admit_effect = {
        let mut writer = ChainDbWriter::new(&db);
        dispatch_block_fetch_inbound(&mut peer, &bf_frame, &mut writer, &schedule, &view)
            .expect("admit")
            .expect("Some")
    };
    let tip = db.tip().expect("tip");
    let stored_bytes = tip.as_ref().and_then(|t| {
        db.get_block_by_hash(&t.hash)
            .expect("get")
            .map(|b| b.bytes)
    });
    let post_fp = ledger_fp(&peer.receive_state.ledger);
    DriveResult {
        cache_effect,
        admit_effect,
        tip_slot: tip.as_ref().map(|t| t.slot),
        tip_hash: tip.map(|t| t.hash),
        stored_bytes,
        fresh_fp,
        post_fp,
        expected_slot: decoded.header_input.slot,
        expected_hash: decoded.block_hash,
    }
}

struct DriveResult {
    cache_effect: ReceiveEffect,
    admit_effect: ReceiveEffect,
    tip_slot: Option<SlotNo>,
    tip_hash: Option<Hash32>,
    stored_bytes: Option<Vec<u8>>,
    fresh_fp: Hash32,
    post_fp: Hash32,
    expected_slot: SlotNo,
    expected_hash: Hash32,
}

#[test]
fn receive_pipeline_corpus_drive_admits_every_block() {
    let (c, _view) = corpus_view();
    for bytes in &c.blocks {
        let r = drive_one_block(bytes, c.epoch_nonce);
        assert!(matches!(r.cache_effect, ReceiveEffect::Cached { .. }));
        assert!(
            matches!(r.admit_effect, ReceiveEffect::Admitted { .. }),
            "every corpus block must admit: got {:?}",
            r.admit_effect
        );
    }
}

#[test]
fn receive_pipeline_corpus_drive_chaindb_tip_matches_expected() {
    let (c, _view) = corpus_view();
    for bytes in &c.blocks {
        let r = drive_one_block(bytes, c.epoch_nonce);
        assert_eq!(r.tip_slot, Some(r.expected_slot));
        assert_eq!(r.tip_hash, Some(r.expected_hash));
    }
}

#[test]
fn receive_pipeline_corpus_drive_admitted_bytes_equal_corpus_bytes() {
    let (c, _view) = corpus_view();
    for bytes in &c.blocks {
        let r = drive_one_block(bytes, c.epoch_nonce);
        assert_eq!(
            r.stored_bytes.as_deref(),
            Some(bytes.as_slice()),
            "ChainDb-stored bytes must equal corpus bytes byte-identically"
        );
    }
}

#[test]
fn receive_pipeline_corpus_drive_ledger_fingerprint_changes_on_admit() {
    let (c, _view) = corpus_view();
    for bytes in &c.blocks {
        let r = drive_one_block(bytes, c.epoch_nonce);
        assert_ne!(
            r.fresh_fp, r.post_fp,
            "ledger fingerprint must change on admission"
        );
        // Smoke: any corpus block decodes — envelope.block_end > start.
        let env = decode_block_envelope(bytes).expect("env");
        assert!(env.block_end > env.block_start);
    }
}
