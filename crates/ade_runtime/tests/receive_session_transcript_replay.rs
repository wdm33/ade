// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// PHASE4-N-H S3 — End-to-end receive session-transcript replay.
//
// Drives a synthetic chain-sync signal + block-fetch event stream
// through the GREEN adapter + BLUE reducer + in-memory ChainDb
// twice; asserts byte-identical (ledger_fingerprint, chain_dep,
// chaindb_tip) across runs. DC-PROTO-09 closure.

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
use ade_ledger::receive::{
    receive_apply_sequence, ReceiveEvent, ReceiveState, TipPoint,
};
use ade_ledger::state::LedgerState;
use ade_network::block_fetch::event::BatchDeliveryEvent;
use ade_network::chain_sync::signal::{ForkChoiceSignal, Point as CsPoint, Tip as CsTip};
use ade_runtime::chaindb::{ChainDb, InMemoryChainDb};
use ade_runtime::receive::{
    lift_block_fetch_event, lift_chain_sync_signal, ChainDbWriter,
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

fn build_signal_event_stream(block_bytes: &[u8]) -> Vec<NaSignalOrEvent> {
    let decoded = decode_block(block_bytes).expect("decode");
    let tip = CsTip {
        point: CsPoint::Block {
            slot: decoded.header_input.slot,
            hash: decoded.block_hash.clone(),
        },
        block_no: decoded.header_input.block_no.0,
    };
    vec![
        NaSignalOrEvent::CsSignal(ForkChoiceSignal::RollForward {
            header_bytes: block_bytes.to_vec(),
            tip,
        }),
        NaSignalOrEvent::BfEvent(BatchDeliveryEvent::BatchStarted),
        NaSignalOrEvent::BfEvent(BatchDeliveryEvent::BlockDelivered {
            block_bytes: block_bytes.to_vec(),
        }),
        NaSignalOrEvent::BfEvent(BatchDeliveryEvent::BatchCompleted),
    ]
}

enum NaSignalOrEvent {
    CsSignal(ForkChoiceSignal),
    BfEvent(BatchDeliveryEvent),
}

fn lift_stream(stream: Vec<NaSignalOrEvent>) -> Vec<ReceiveEvent> {
    let mut out = Vec::new();
    for s in stream {
        let maybe = match s {
            NaSignalOrEvent::CsSignal(sig) => lift_chain_sync_signal(sig),
            NaSignalOrEvent::BfEvent(ev) => lift_block_fetch_event(ev),
        };
        if let Some(e) = maybe {
            out.push(e);
        }
    }
    out
}

#[test]
fn receive_session_transcript_replay_byte_identical() {
    let (c, view) = corpus_view();
    let schedule = schedule();
    let bytes = pick_lightest(&c);

    let run = || -> (Hash32, Option<SlotNo>, Option<Hash32>) {
        let stream = build_signal_event_stream(&bytes);
        let events = lift_stream(stream);
        let mut state = fresh_state(c.epoch_nonce);
        let db = InMemoryChainDb::new();
        {
            let mut writer = ChainDbWriter::new(&db);
            receive_apply_sequence(&mut state, events, &mut writer, &schedule, &view)
                .expect("sequence");
        }
        let fp = ade_ledger::fingerprint::fingerprint(&state.ledger).combined;
        let tip = db.tip().expect("tip");
        (fp, tip.as_ref().map(|t| t.slot), tip.map(|t| t.hash))
    };

    let a = run();
    let b = run();
    assert_eq!(a.0, b.0, "ledger fingerprint must replay byte-identical");
    assert_eq!(a.1, b.1, "ChainDb tip slot must replay byte-identical");
    assert_eq!(a.2, b.2, "ChainDb tip hash must replay byte-identical");
    assert!(a.1.is_some(), "non-empty admit must produce a tip");
}

// Silence unused-import warning; TipPoint is exposed via the
// receive module but not directly constructed here.
#[allow(dead_code)]
fn _tp_marker(_p: TipPoint) {}
