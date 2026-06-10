// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration test — PHASE4-N-K S7 (DC-NODE-04).
//!
//! `bootstrap → drive events → shutdown → bootstrap` must
//! produce a byte-identical initial `(LedgerState fingerprint,
//! PraosChainDepState, ChainTip)`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_ledger::block_validity::decode_block;
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::state::LedgerState;
use ade_node::node::{run_node_until_shutdown, NodeStartupInputs};
use ade_runtime::chaindb::{ChainDb, InMemoryChainDb, StoredBlock};
use ade_runtime::clock::DeterministicClock;
use ade_runtime::orchestrator::event::OrchestratorEvent;
use ade_runtime::orchestrator::leadership_session::SlotEraAnchor;
use ade_runtime::rollback::cadence::SnapshotCadence;
use ade_runtime::rollback::PersistentSnapshotCache;
use ade_runtime::bootstrap::{
    bootstrap_initial_state, BootstrapInputs, BootstrapState, SeedEpochConsensusSource,
};
use ade_testkit::validity::ConwayValidityCorpus;
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};
use tokio::sync::mpsc;

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

fn fresh_genesis(eta0: [u8; 32]) -> (LedgerState, PraosChainDepState) {
    let mut ledger = LedgerState::new(CardanoEra::Conway);
    ledger.epoch_state.epoch = EPOCH_576;
    let mut chain_dep = PraosChainDepState::empty();
    chain_dep.epoch_nonce = Nonce(Hash32(eta0));
    chain_dep.evolving_nonce = Nonce(Hash32(eta0));
    (ledger, chain_dep)
}

fn pick_lightest_block(c: &ConwayValidityCorpus) -> (Vec<u8>, SlotNo, Hash32) {
    let idx = (0..c.blocks.len())
        .min_by_key(|&i| {
            let env = decode_block_envelope(&c.blocks[i]).expect("env");
            env.block_end - env.block_start
        })
        .expect("non-empty");
    let bytes = c.blocks[idx].clone();
    let decoded = decode_block(&bytes).expect("decode");
    (bytes, decoded.header_input.slot, decoded.block_hash)
}

#[tokio::test(flavor = "current_thread")]
async fn shutdown_then_resume_produces_byte_identical_state() {
    let (corpus, view) = corpus_view();
    let sched = schedule();

    // Seed: take a block, apply it via block_validity, snapshot the
    // resulting ledger at the block's slot. This simulates an
    // operator-warm-started chain_db + snapshot_store.
    let (block_bytes, snapshot_slot, block_hash) = pick_lightest_block(&corpus);
    let (mut seed_ledger, mut seed_chain_dep) = fresh_genesis(corpus.epoch_nonce);
    use ade_ledger::block_validity::transition::{block_validity, BlockValidityOutcome};
    use ade_ledger::block_validity::verdict::BlockValidityVerdict;
    let BlockValidityOutcome {
        verdict,
        ledger: new_l,
        chain_dep: new_cd,
    } = block_validity(&seed_ledger, &seed_chain_dep, &sched, &view, &block_bytes);
    match verdict {
        BlockValidityVerdict::Valid { .. } => {
            seed_ledger = new_l;
            seed_chain_dep = new_cd;
        }
        BlockValidityVerdict::Invalid { error, .. } => {
            panic!("seed block must be valid: {error:?}")
        }
    }

    let db = InMemoryChainDb::new();
    db.put_block(&StoredBlock {
        slot: snapshot_slot,
        hash: block_hash.clone(),
        bytes: block_bytes.clone(),
    })
    .expect("put block");
    let cache = PersistentSnapshotCache::new(&db);
    cache
        .capture(snapshot_slot, &seed_ledger, &seed_chain_dep)
        .expect("seed snapshot");

    // First run: bootstrap → Shutdown → force final snapshot.
    let pre_fp = fingerprint(&seed_ledger).combined;
    let view_arc: Arc<dyn ade_core::consensus::ledger_view::LedgerView + Send + Sync> =
        Arc::new(view.clone());
    let (inbox_tx, inbox_rx) = mpsc::channel::<OrchestratorEvent>(16);
    let inputs = NodeStartupInputs {
        chaindb: &db,
        snapshot_store: &db,
        era_schedule: &sched,
        ledger_view: view_arc.clone(),
        cadence: SnapshotCadence { every_n_blocks: 1 },
        leadership_clock: DeterministicClock::new(0, vec![]),
        leadership_anchor: SlotEraAnchor {
            start_slot: SlotNo(0),
            start_millis: 0,
            slot_length_ms: 1000,
        },
        genesis_initial: None,
    };
    // Queue the shutdown event so the run loop exits after bootstrap.
    inbox_tx
        .send(OrchestratorEvent::Shutdown)
        .await
        .expect("queue shutdown");
    let evidence_1 = run_node_until_shutdown(inputs, inbox_tx, inbox_rx)
        .await
        .expect("first run");

    // Second run over the same on-disk state: bootstrap → Shutdown.
    let (inbox_tx2, inbox_rx2) = mpsc::channel::<OrchestratorEvent>(16);
    let inputs_2 = NodeStartupInputs {
        chaindb: &db,
        snapshot_store: &db,
        era_schedule: &sched,
        ledger_view: view_arc.clone(),
        cadence: SnapshotCadence { every_n_blocks: 1 },
        leadership_clock: DeterministicClock::new(0, vec![]),
        leadership_anchor: SlotEraAnchor {
            start_slot: SlotNo(0),
            start_millis: 0,
            slot_length_ms: 1000,
        },
        genesis_initial: None,
    };
    inbox_tx2
        .send(OrchestratorEvent::Shutdown)
        .await
        .expect("queue shutdown 2");
    let evidence_2 = run_node_until_shutdown(inputs_2, inbox_tx2, inbox_rx2)
        .await
        .expect("second run");

    assert_eq!(
        evidence_1.final_chain_tip, evidence_2.final_chain_tip,
        "chain tip must match across shutdown/resume"
    );

    // Direct fingerprint check: bootstrap again to extract the ledger.
    let BootstrapState {
        ledger: l_again,
        chain_dep: cd_again,
        tip: tip_again,
        ..
    } = bootstrap_initial_state(BootstrapInputs {
        chaindb: &db,
        snapshot_store: &db,
        era_schedule: &sched,
        ledger_view: view_arc.as_ref(),
        genesis_initial: None,
        seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired,
        recovered_anchor: None,
    })
    .expect("bootstrap again");
    assert_eq!(
        fingerprint(&l_again).combined,
        pre_fp,
        "ledger fingerprint must equal seed"
    );
    assert_eq!(cd_again, seed_chain_dep);
    assert_eq!(
        tip_again.as_ref().map(|t| t.slot),
        Some(snapshot_slot),
        "tip slot preserved"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn shutdown_clean_exits_with_evidence() {
    let (corpus, view) = corpus_view();
    let sched = schedule();
    let (block_bytes, snapshot_slot, block_hash) = pick_lightest_block(&corpus);
    let (mut seed_ledger, mut seed_chain_dep) = fresh_genesis(corpus.epoch_nonce);
    use ade_ledger::block_validity::transition::{block_validity, BlockValidityOutcome};
    use ade_ledger::block_validity::verdict::BlockValidityVerdict;
    let BlockValidityOutcome {
        verdict,
        ledger: new_l,
        chain_dep: new_cd,
    } = block_validity(&seed_ledger, &seed_chain_dep, &sched, &view, &block_bytes);
    match verdict {
        BlockValidityVerdict::Valid { .. } => {
            seed_ledger = new_l;
            seed_chain_dep = new_cd;
        }
        BlockValidityVerdict::Invalid { error, .. } => {
            panic!("seed block must be valid: {error:?}")
        }
    }
    let db = InMemoryChainDb::new();
    db.put_block(&StoredBlock {
        slot: snapshot_slot,
        hash: block_hash,
        bytes: block_bytes,
    })
    .expect("put");
    let cache = PersistentSnapshotCache::new(&db);
    cache
        .capture(snapshot_slot, &seed_ledger, &seed_chain_dep)
        .expect("seed");

    let view_arc: Arc<dyn ade_core::consensus::ledger_view::LedgerView + Send + Sync> =
        Arc::new(view);
    let (inbox_tx, inbox_rx) = mpsc::channel(8);
    let inputs = NodeStartupInputs {
        chaindb: &db,
        snapshot_store: &db,
        era_schedule: &sched,
        ledger_view: view_arc.clone(),
        cadence: SnapshotCadence::DEFAULT,
        leadership_clock: DeterministicClock::new(0, vec![]),
        leadership_anchor: SlotEraAnchor {
            start_slot: SlotNo(0),
            start_millis: 0,
            slot_length_ms: 1000,
        },
        genesis_initial: None,
    };
    inbox_tx
        .send(OrchestratorEvent::Shutdown)
        .await
        .expect("queue shutdown");
    let evidence = run_node_until_shutdown(inputs, inbox_tx, inbox_rx)
        .await
        .expect("run");
    // Force snapshot was written.
    assert!(evidence.final_persistent_snapshot_slot.is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn cold_start_without_genesis_fails_with_generic_startup_code() {
    let (_, view) = corpus_view();
    let sched = schedule();
    let db = InMemoryChainDb::new(); // empty
    let view_arc: Arc<dyn ade_core::consensus::ledger_view::LedgerView + Send + Sync> =
        Arc::new(view);
    let (inbox_tx, inbox_rx) = mpsc::channel::<OrchestratorEvent>(4);
    let inputs = NodeStartupInputs {
        chaindb: &db,
        snapshot_store: &db,
        era_schedule: &sched,
        ledger_view: view_arc,
        cadence: SnapshotCadence::DEFAULT,
        leadership_clock: DeterministicClock::new(0, vec![]),
        leadership_anchor: SlotEraAnchor {
            start_slot: SlotNo(0),
            start_millis: 0,
            slot_length_ms: 1000,
        },
        genesis_initial: None,
    };
    let err = run_node_until_shutdown(inputs, inbox_tx, inbox_rx)
        .await
        .expect_err("must fail");
    assert_eq!(err.exit_code(), ade_node::EXIT_GENERIC_STARTUP);
}
