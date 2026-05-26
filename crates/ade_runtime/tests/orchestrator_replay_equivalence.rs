// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration test — PHASE4-N-K S8 (DC-NODE-03).
//!
//! Replay-equivalence under deterministic clock injection. The
//! orchestrator core, driven twice over the same
//! `OrchestratorEvent` corpus, produces byte-identical
//! `(LedgerFingerprint.combined, PraosChainDepState, ChainDb tip)`
//! across runs.
//!
//! The "corpus" is constructed in-test from the ConwayValidityCorpus
//! anchor block — checked into ade_testkit. This keeps the corpus
//! frozen alongside the existing replay fixtures; the test asserts
//! the deterministic property without requiring a separate
//! `corpus/orchestrator/` directory.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_ledger::block_validity::decode_block;
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::producer::ServedChainSnapshot;
use ade_ledger::receive::ReceiveState;
use ade_ledger::state::LedgerState;
use ade_network::codec::block_fetch::{encode_block_fetch_message, BlockFetchMessage};
use ade_network::codec::chain_sync::{
    encode_chain_sync_message, ChainSyncMessage, Point as CsPoint, Tip as CsTip,
};
use ade_network::codec::version::{BlockFetchVersion, ChainSyncVersion};
use ade_runtime::chaindb::{ChainDb, ChainTip, InMemoryChainDb};
use ade_runtime::clock::DeterministicClock;
use ade_runtime::clock::{millis_to_slot, Clock};
use ade_runtime::orchestrator::event::{
    OrchestratorEffect, OrchestratorEvent, PeerId, PeerRole,
};
use ade_runtime::orchestrator::state::OrchestratorState;
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

/// One concrete replay run; returns a fingerprint of the final
/// orchestrator-driven `(ledger, chain_dep, chain_tip)`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReplayFingerprint {
    pub ledger_combined: Hash32,
    pub chain_dep: PraosChainDepState,
    pub chain_tip: Option<ChainTip>,
    pub effects_count: usize,
    pub admitted_slot: Option<SlotNo>,
}

fn run_orchestrator_corpus(events: &[OrchestratorEvent]) -> ReplayFingerprint {
    let (corpus, view) = corpus_view();
    let mut state =
        OrchestratorState::new(fresh_receive(corpus.epoch_nonce), SnapshotCadence::DEFAULT);
    let db = InMemoryChainDb::new();
    let mut writer = ChainDbWriter::new(&db);
    let served = ServedChainSnapshot::new();
    let sched = schedule();
    let mut all_effects = Vec::new();
    let mut admitted_slot: Option<SlotNo> = None;
    for event in events {
        let effects = step(
            &mut state,
            event.clone(),
            &mut writer,
            &served,
            &sched,
            &view,
        )
        .expect("step");
        for e in &effects {
            if let OrchestratorEffect::AdmittedBlock { slot, .. } = e {
                admitted_slot = Some(*slot);
            }
        }
        all_effects.extend(effects);
    }
    ReplayFingerprint {
        ledger_combined: fingerprint(&state.receive_state.ledger).combined,
        chain_dep: state.receive_state.chain_dep.clone(),
        chain_tip: db.tip().expect("tip"),
        effects_count: all_effects.len(),
        admitted_slot,
    }
}

fn build_events(corpus: &ConwayValidityCorpus) -> Vec<OrchestratorEvent> {
    let bytes = pick_lightest(corpus);
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

    // Use a deterministic clock to derive slot ticks; anchor at
    // start slot 0, slot_length_ms 1000, four ticks spaced 1s apart.
    let mut clock = DeterministicClock::new(0, vec![1000, 2000, 3000, 4000]);
    let mut events = vec![OrchestratorEvent::PeerConnected {
        peer_id: PeerId(1),
        chain_sync_version: ChainSyncVersion::new(9),
        block_fetch_version: BlockFetchVersion::new(9),
        role: PeerRole::UpstreamClient,
    }];
    while let Some(t) = clock.next_tick() {
        let slot = millis_to_slot(t, 0, SlotNo(EPOCH_577_START - MAINNET_EPOCH_LENGTH), 1000);
        events.push(OrchestratorEvent::SlotTick {
            slot_millis: t,
            slot,
        });
    }
    events.push(OrchestratorEvent::PeerChainSyncFrame {
        peer_id: PeerId(1),
        bytes: cs_frame,
    });
    events.push(OrchestratorEvent::PeerBlockFetchFrame {
        peer_id: PeerId(1),
        bytes: bf_frame,
    });
    events.push(OrchestratorEvent::Shutdown);
    events
}

#[test]
fn replay_equivalence_under_deterministic_clock_holds() {
    let (corpus, _view) = corpus_view();
    let events = build_events(&corpus);

    let fp_a = run_orchestrator_corpus(&events);
    let fp_b = run_orchestrator_corpus(&events);
    assert_eq!(
        fp_a, fp_b,
        "orchestrator replay must be byte-identical under DeterministicClock"
    );
    assert!(
        fp_a.admitted_slot.is_some(),
        "corpus must produce at least one admitted block"
    );
}

#[test]
fn replay_corpus_is_present_and_decodable() {
    // The "corpus" is the in-test construction in build_events;
    // re-build it and assert non-empty deterministic decode.
    let (corpus, _view) = corpus_view();
    let events_a = build_events(&corpus);
    let events_b = build_events(&corpus);
    assert!(!events_a.is_empty());
    assert_eq!(events_a, events_b, "corpus builder must be deterministic");
}
