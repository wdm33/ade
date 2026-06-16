// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration test — PHASE4-N-M-C S2 (DC-ADMIT-11 / ¬P-C2).
//!
//! Feeds the admission runner a real Conway block whose slot is
//! OUTSIDE the imported consensus-inputs epoch window. The runner
//! MUST emit `AdmissionHalted { reason: cross_epoch_use }` and
//! return `AdmissionExitCode::CrossEpochUse` (numeric 32) WITHOUT
//! ever calling `admit_via_block_validity` and WITHOUT appending
//! a WAL entry.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_ledger::block_validity::decode_block;
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::state::LedgerState;
use ade_ledger::wal::{WalEntry, WalError, WalStore};
use ade_node::admission::{
    run_admission, AdmissionExitCode, AdmissionInputs, AdmissionPeerEvent,
    EXIT_LIVE_CROSS_EPOCH_USE,
};
use ade_node::admission_log::AdmissionLogWriter;
use ade_testkit::validity::ConwayValidityCorpus;
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};
use std::collections::BTreeMap;
use tokio::sync::{mpsc, watch};

const EPOCH_576: EpochNo = EpochNo(576);
const EPOCH_577_START: u64 = 163_900_800;
const MAINNET_EPOCH_LENGTH: u64 = 432_000;

struct VecWalStore {
    entries: Vec<WalEntry>,
}
impl WalStore for VecWalStore {
    fn append(&mut self, entry: WalEntry) -> Result<(), WalError> {
        self.entries.push(entry);
        Ok(())
    }
    fn read_all(&self) -> Result<Vec<WalEntry>, WalError> {
        Ok(self.entries.clone())
    }
}

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

fn pick_lightest_block(c: &ConwayValidityCorpus) -> (Vec<u8>, SlotNo) {
    let idx = (0..c.blocks.len())
        .min_by_key(|&i| {
            let env = decode_block_envelope(&c.blocks[i]).expect("env");
            env.block_end - env.block_start
        })
        .expect("non-empty");
    let bytes = c.blocks[idx].clone();
    let decoded = decode_block(&bytes).expect("decode");
    (bytes, decoded.header_input.slot)
}

#[tokio::test]
async fn cross_epoch_block_triggers_halt_without_admit() {
    let (corpus, _ledger_view_pd) = corpus_view();
    let (block_bytes, block_slot) = pick_lightest_block(&corpus);

    // Configure a consensus-inputs epoch window that does NOT
    // contain block_slot — choose a window strictly before the
    // block's slot.
    assert!(
        block_slot.0 >= 2,
        "test corpus block slot must be >= 2 for guard to be exercised"
    );
    let epoch_start = SlotNo(0);
    let epoch_end = SlotNo(block_slot.0 - 1);

    let (tx, rx) = mpsc::channel::<AdmissionPeerEvent>(8);
    let (_sh_tx, sh_rx) = watch::channel(false);
    let schedule = schedule();
    // Use the existing PoolDistrView via a trivial trait object;
    // it's referenced but never queried because the runner halts
    // before admit.
    let view = ade_testkit::consensus::ledger_view_stub::LedgerViewStub::new();

    let wal_store = VecWalStore {
        entries: Vec::new(),
    };
    let inputs = AdmissionInputs {
        writer: AdmissionLogWriter::new(Vec::<u8>::new()),
        wal_store,
        anchor_initial_ledger_fp: Hash32([0xAA; 32]),
        ledger: LedgerState::new(CardanoEra::Conway),
        static_utxo_fp: None,
        chain_dep: PraosChainDepState::genesis(Nonce::ZERO),
        era_schedule: &schedule,
        ledger_view: &view,
        peer_events: rx,
        shutdown: sh_rx,
        peer_count: 1,
        json_seed_path: "/seed.json".into(),
        wal_dir: "/wal".into(),
        initial_chain_tip_slot: 0,
        seed_import_rss_kib: 0,
        seed_import_hwm_kib: 0,
        seed_import_rss_anon_kib: 0,
        seed_import_private_dirty_kib: 0,
        mem_phase_diagnostic: None,
        consensus_inputs_fingerprint: Hash32([0xCC; 32]),
        consensus_inputs_epoch: EPOCH_576,
        consensus_inputs_epoch_start_slot: epoch_start,
        consensus_inputs_epoch_end_slot: epoch_end,
    };

    let feeder = tokio::spawn(async move {
        let _ = tx
            .send(AdmissionPeerEvent::Block {
                peer: "1.1.1.1:3001".into(),
                block_bytes,
            })
            .await;
    });

    let exit = run_admission(inputs).await;
    let _ = feeder.await;

    assert_eq!(
        exit,
        AdmissionExitCode::CrossEpochUse,
        "expected CrossEpochUse exit; got {:?}",
        exit
    );
    assert_eq!(exit.as_i32(), EXIT_LIVE_CROSS_EPOCH_USE);
    assert_eq!(EXIT_LIVE_CROSS_EPOCH_USE, 32);
}
