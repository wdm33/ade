// Core Contract:
// - Operational SLA, NOT a hash-critical invariant.
// - This is the ONLY file in the producer cluster permitted to import
//   `std::time` (whitelisted by `ci/ci_check_scheduler_closure.sh`).
// - Measures wall-clock latency of one full scheduler_step pipeline
//   pass (RED -> GREEN -> BLUE -> BLUE -> RED) on a reference fixture.
//   Asserts median latency < 1000ms (mainnet slot deadline).

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use std::collections::BTreeMap;
use std::time::Instant;

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::leader_schedule::LeaderScheduleAnswer;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_core::consensus::vrf_cert::{ActiveSlotsCoeff, ExpectedVrfInput};
use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
use ade_crypto::ed25519::Ed25519VerificationKey;
use ade_crypto::kes::{KesPeriod, KesSignature, SUM6_KES_SIG_LEN};
use ade_crypto::vrf::VrfProof;
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::mempool::admit::MempoolState;
use ade_ledger::state::LedgerState;
use ade_runtime::producer::scheduler::{scheduler_step, SchedulerInput, SchedulerState};
use ade_runtime::producer::tick_assembler::TickInputs;
use ade_testkit::validity::ConwayValidityCorpus;
use ade_types::primitives::SlotNo;
use ade_types::shelley::block::{OperationalCert, PrevHash, ProtocolVersion};
use ade_types::{BlockNo, CardanoEra, EpochNo, Hash28, Hash32};
use cardano_crypto::vrf::VrfDraft03;
use ed25519_dalek::{Signer, SigningKey as DalekSk};

const EPOCH_576: EpochNo = EpochNo(576);
const EPOCH_577_START: u64 = 163_900_800;
const MAINNET_EPOCH_LENGTH: u64 = 432_000;

fn schedule() -> EraSchedule {
    let start_576 = EPOCH_577_START - MAINNET_EPOCH_LENGTH;
    let eras = vec![EraSummary {
        randomness_stabilisation_window_slots: None,
        era: CardanoEra::Conway,
        start_slot: SlotNo(start_576),
        start_epoch: EPOCH_576,
        slot_length_ms: 1_000,
        epoch_length_slots: MAINNET_EPOCH_LENGTH as u32,
        safe_zone_slots: MAINNET_EPOCH_LENGTH as u32,
    }];
    EraSchedule::new(BootstrapAnchorHash(Hash32([0u8; 32])), 0, eras)
        .expect("schedule well-formed")
}

fn state_with_eta0(eta0: [u8; 32]) -> PraosChainDepState {
    let mut s = PraosChainDepState::empty();
    s.epoch_nonce = Nonce(Hash32(eta0));
    s.evolving_nonce = Nonce(Hash32(eta0));
    s
}

fn ledger_at_576() -> LedgerState {
    let mut l = LedgerState::new(CardanoEra::Conway);
    l.epoch_state.epoch = EPOCH_576;
    l
}

fn view(c: &ConwayValidityCorpus) -> PoolDistrView {
    let total = c.pd_total_active_stake;
    let asc = ActiveSlotsCoeff {
        numer: c.asc.numer as u32,
        denom: c.asc.denom as u32,
    };
    let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
    for (pool_id, p) in &c.pools {
        assert!(p.sigma.denom != 0);
        assert!(total % p.sigma.denom == 0);
        let scale = total / p.sigma.denom;
        let active_stake = p.sigma.numer * scale;
        pools.insert(
            Hash28(*pool_id),
            PoolEntry {
                active_stake,
                vrf_keyhash: Hash32(p.vrf_keyhash),
            },
        );
    }
    PoolDistrView::new(EPOCH_576, total, asc, pools)
}

fn synth_opcert(
    cold_seed: [u8; 32],
    hot_vkey: [u8; 32],
    sequence_number: u64,
    kes_period: u64,
) -> (OperationalCert, Ed25519VerificationKey) {
    let cold = DalekSk::from_bytes(&cold_seed);
    let cold_vk_bytes = *cold.verifying_key().as_bytes();
    let mut signable = Vec::with_capacity(48);
    signable.extend_from_slice(&hot_vkey);
    signable.extend_from_slice(&sequence_number.to_be_bytes());
    signable.extend_from_slice(&kes_period.to_be_bytes());
    let sigma = cold.sign(&signable);
    (
        OperationalCert {
            hot_vkey: hot_vkey.to_vec(),
            sequence_number,
            kes_period,
            sigma: sigma.to_bytes().to_vec(),
        },
        Ed25519VerificationKey::from_bytes(&cold_vk_bytes).unwrap(),
    )
}

fn leader_always() -> LeaderScheduleAnswer {
    LeaderScheduleAnswer {
        slot: SlotNo(100),
        pool: Hash28([0xAA; 28]),
        epoch: EpochNo(0),
        expected_vrf_input: ExpectedVrfInput::Praos([0u8; 32]),
        stake_fraction: (1, 2),
        asc: ActiveSlotsCoeff { numer: 1, denom: 1 },
    }
}

fn base_tick_inputs() -> TickInputs {
    let (opcert, cold_vk) = synth_opcert([0x42; 32], [0x43; 32], 7, 42);
    let (sk, _vk) = VrfDraft03::keypair_from_seed(&[0xA5; 32]);
    let mut alpha = Vec::with_capacity(16);
    alpha.extend_from_slice(&100u64.to_be_bytes());
    alpha.extend_from_slice(b"vrf-input-stub");
    let proof_bytes = VrfDraft03::prove(&sk, &alpha).expect("prove");
    TickInputs {
        vrf_proof: VrfProof(proof_bytes),
        kes_period: KesPeriod(42),
        kes_signature: KesSignature([0u8; SUM6_KES_SIG_LEN]),
        opcert,
        cold_vk,
        vrf_vkey: vec![0u8; 32],
        leader_answer: leader_always(),
        pparams: Default::default(),
        mempool_tx_bytes: Vec::new(),
        prev_opcert_counter: None,
        block_number: BlockNo(1),
        prev_hash: PrevHash::Block(Hash32([0u8; 32])),
        protocol_version: ProtocolVersion { major: 9, minor: 0 },
    }
}

fn build_fixture() -> (SchedulerState, SchedulerInput, PoolDistrView) {
    let corpus = ConwayValidityCorpus::load().expect("corpus loads");
    let v = view(&corpus);
    let state = SchedulerState {
        ledger: ledger_at_576(),
        chain_dep: state_with_eta0(corpus.epoch_nonce),
        mempool: MempoolState::new(ledger_at_576()),
        era_schedule: schedule(),
        last_seen_slot: None,
        prev_opcert_counter: None,
        halted: None,
    };
    let inputs = base_tick_inputs();
    let input = SchedulerInput::SlotTick { slot: 100, inputs };
    (state, input, v)
}

#[test]
fn producer_full_path_under_slot_deadline_on_reference_fixture() {
    // Operational SLA: median scheduler_step latency over N runs is
    // below the 1000ms mainnet slot deadline. The pipeline traversed:
    //   RED scheduler -> GREEN tick assembler -> BLUE forge -> BLUE
    //   self_accept -> RED halt-or-broadcast. Outcome variance does
    //   not affect the timing budget; the budget is generous (1s).
    let (state, input, v) = build_fixture();

    let runs = 10usize;
    let mut durations_us: Vec<u128> = Vec::with_capacity(runs);
    for _ in 0..runs {
        let st = state.clone();
        let inp = input.clone();
        let start = Instant::now();
        let (_next, _effects) = scheduler_step(st, inp, &v);
        durations_us.push(start.elapsed().as_micros());
    }
    durations_us.sort();
    let median_us = durations_us[runs / 2];
    assert!(
        median_us < 1_000_000,
        "median scheduler_step latency {median_us} us exceeds 1s slot deadline; \
         observed durations (us, sorted) = {durations_us:?}"
    );
}
