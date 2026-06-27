// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// S-B7 composition test — drives validate_and_apply_header end-to-end
// over synthesised VRF proofs, asserting that each step's failure path
// produces the expected typed HeaderValidationError and that the happy
// path advances every relevant field of PraosChainDepState.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::collections::BTreeMap;

use ade_core::consensus::vrf_cert::vrf_input;
use ade_core::consensus::{
    validate_and_apply_header, ActiveSlotsCoeff, BootstrapAnchorHash, EraSchedule, EraSummary,
    HeaderInput, HeaderValidationError, HeaderVrf, LeaderEligibility, Nonce, OpCertCounterError,
    OutsideForecastRange,
    PraosChainDepState, VrfCertError, VrfRole,
};
use ade_crypto::vrf::{VrfProof, VrfVerificationKey};
use ade_testkit::consensus::ledger_view_stub::{
    EpochStakeFixture, LedgerViewStub, PoolFixture,
};
use ade_types::{BlockNo, CardanoEra, EpochNo, Hash28, Hash32, SlotNo};
use cardano_crypto::vrf::VrfDraft03;

fn schedule() -> EraSchedule {
    let eras = vec![EraSummary {
        randomness_stabilisation_window_slots: None,
        era: CardanoEra::Shelley,
        start_slot: SlotNo(0),
        start_epoch: EpochNo(0),
        slot_length_ms: 1_000,
        epoch_length_slots: 432_000,
        safe_zone_slots: 129_600,
    }];
    EraSchedule::new(BootstrapAnchorHash(Hash32([0u8; 32])), 0, eras)
        .expect("schedule constructs")
}

fn pool() -> Hash28 {
    Hash28([0xAA; 28])
}

fn key_material(seed: [u8; 32]) -> ([u8; 64], VrfVerificationKey) {
    let (sk, vk_bytes) = VrfDraft03::keypair_from_seed(&seed);
    (sk, VrfVerificationKey(vk_bytes))
}

fn prove(sk: &[u8; 64], slot: SlotNo, epoch_nonce: &Nonce, role: VrfRole) -> VrfProof {
    let alpha = vrf_input(slot, epoch_nonce, role);
    let bytes = VrfDraft03::prove(sk, &alpha).expect("VRF prove");
    VrfProof(bytes)
}

fn ledger(vk: VrfVerificationKey) -> LedgerViewStub {
    // asc = 1/1, sigma = 1/1: leader threshold trivially passes for
    // every VRF output (every slot leads).
    let mut pools = BTreeMap::new();
    pools.insert(
        pool(),
        PoolFixture {
            active_stake: 1,
            vrf_keyhash: ade_crypto::blake2b::blake2b_256(&vk.0),
        },
    );
    LedgerViewStub::new().with_epoch(
        EpochNo(0),
        EpochStakeFixture {
            total_active_stake: 1,
            asc: ActiveSlotsCoeff { numer: 1, denom: 1 },
            pools,
        },
    )
}

fn genesis_state() -> PraosChainDepState {
    let mut s = PraosChainDepState::empty();
    s.epoch_nonce = Nonce(Hash32([0xCD; 32]));
    s.evolving_nonce = Nonce(Hash32([0xEE; 32]));
    s
}

fn header_at(
    sk: &[u8; 64],
    vk: &VrfVerificationKey,
    state: &PraosChainDepState,
    slot: SlotNo,
    block_no: BlockNo,
    op_cert_counter: u64,
) -> HeaderInput {
    HeaderInput {
        prev_hash: Hash32([0u8; 32]),
        slot,
        block_no,
        body_hash: Hash32([0x55; 32]),
        issuer_pool: pool(),
        op_cert_kes_period: 0,
        op_cert_counter,
        vrf_vk: vk.clone(),
        vrf: HeaderVrf::Tpraos {
            nonce_proof: prove(sk, slot, &state.epoch_nonce, VrfRole::NonceContribution),
            leader_proof: prove(sk, slot, &state.epoch_nonce, VrfRole::LeaderEligibility),
        },
        kes: None,
    }
}

#[test]
fn valid_header_accepted_advances_state() {
    let (sk, vk) = key_material([1u8; 32]);
    let state = genesis_state();
    let header = header_at(&sk, &vk, &state, SlotNo(1), BlockNo(1), 0);

    let applied = validate_and_apply_header(&state, &header, &ledger(vk), &schedule(), LeaderEligibility::Enforce)
        .expect("valid header accepted");
    assert_eq!(applied.new_state.last_slot, Some(SlotNo(1)));
    assert_eq!(applied.new_state.last_block_no, Some(BlockNo(1)));
    assert_eq!(applied.new_state.op_cert_counters.get(&pool(), 0), Some(0));
    assert_ne!(applied.new_state.evolving_nonce, state.evolving_nonce);
    // Summary mirrors the inputs.
    assert_eq!(applied.summary.slot, SlotNo(1));
    assert_eq!(applied.summary.block_no, BlockNo(1));
    assert_eq!(applied.summary.op_cert_counter, 0);
    assert_eq!(applied.summary.issuer_pool, pool());
    assert_eq!(applied.summary.body_hash, Hash32([0x55; 32]));
}

#[test]
fn header_with_slot_regression_rejected() {
    let (sk, vk) = key_material([2u8; 32]);
    let mut state = genesis_state();
    state.last_slot = Some(SlotNo(100));
    let header = header_at(&sk, &vk, &state, SlotNo(50), BlockNo(1), 0);
    let res = validate_and_apply_header(&state, &header, &ledger(vk), &schedule(), LeaderEligibility::Enforce);
    assert_eq!(
        res,
        Err(HeaderValidationError::SlotBeforeLastApplied {
            last: SlotNo(100),
            attempted: SlotNo(50),
        })
    );
}

#[test]
fn header_with_block_no_regression_rejected() {
    let (sk, vk) = key_material([4u8; 32]);
    let mut state = genesis_state();
    state.last_slot = Some(SlotNo(1));
    state.last_block_no = Some(BlockNo(100));
    let header = header_at(&sk, &vk, &state, SlotNo(2), BlockNo(50), 0);
    let res = validate_and_apply_header(&state, &header, &ledger(vk), &schedule(), LeaderEligibility::Enforce);
    assert_eq!(
        res,
        Err(HeaderValidationError::BlockNoOutOfOrder {
            last: BlockNo(100),
            attempted: BlockNo(50),
        })
    );
}

#[test]
fn header_with_op_cert_regression_rejected() {
    let (sk, vk) = key_material([5u8; 32]);
    let state = genesis_state();

    // First admit a header with counter = 10.
    let h1 = header_at(&sk, &vk, &state, SlotNo(1), BlockNo(1), 10);
    let applied =
        validate_and_apply_header(&state, &h1, &ledger(vk.clone()), &schedule(), LeaderEligibility::Enforce).expect("first ok");

    // Now attempt to admit a second header with a regression (counter
    // = 9 < 10) for the same pool + KES period.
    let s2 = applied.new_state;
    let h2 = header_at(&sk, &vk, &s2, SlotNo(2), BlockNo(2), 9);
    let res = validate_and_apply_header(&s2, &h2, &ledger(vk), &schedule(), LeaderEligibility::Enforce);
    assert_eq!(
        res,
        Err(HeaderValidationError::OpCertCounter(
            OpCertCounterError::Regression {
                existing: 10,
                attempted: 9,
            }
        ))
    );
}

#[test]
fn header_with_invalid_vrf_proof_rejected() {
    let (sk, vk) = key_material([6u8; 32]);
    let state = genesis_state();
    // Use a proof produced for a different slot — verify_vrf_cert will
    // fail because the alpha embedded in the proof doesn't match.
    let bad_proof = prove(&sk, SlotNo(999), &state.epoch_nonce, VrfRole::NonceContribution);
    let leader_proof = prove(&sk, SlotNo(1), &state.epoch_nonce, VrfRole::LeaderEligibility);
    let header = HeaderInput {
        prev_hash: Hash32([0u8; 32]),
        slot: SlotNo(1),
        block_no: BlockNo(1),
        body_hash: Hash32([0x55; 32]),
        issuer_pool: pool(),
        op_cert_kes_period: 0,
        op_cert_counter: 0,
        vrf_vk: vk.clone(),
        vrf: HeaderVrf::Tpraos {
            nonce_proof: bad_proof,
            leader_proof,
        },
        kes: None,
    };
    let res = validate_and_apply_header(&state, &header, &ledger(vk), &schedule(), LeaderEligibility::Enforce);
    assert_eq!(
        res,
        Err(HeaderValidationError::VrfCert(
            VrfCertError::VerificationFailed
        ))
    );
}

#[test]
fn header_beyond_forecast_horizon_rejected() {
    let (sk, vk) = key_material([7u8; 32]);
    let state = genesis_state();
    let beyond = SlotNo(u64::MAX);
    let header = header_at(&sk, &vk, &state, beyond, BlockNo(1), 0);
    let res = validate_and_apply_header(&state, &header, &ledger(vk), &schedule(), LeaderEligibility::Enforce);
    assert_eq!(
        res,
        Err(HeaderValidationError::OutsideForecastRange(
            OutsideForecastRange {
                requested: beyond,
                horizon: SlotNo(129_600),
            }
        ))
    );
}

#[test]
fn validate_replay_is_deterministic() {
    let (sk, vk) = key_material([8u8; 32]);
    let state = genesis_state();
    let header = header_at(&sk, &vk, &state, SlotNo(1), BlockNo(1), 0);

    let a = validate_and_apply_header(&state, &header, &ledger(vk.clone()), &schedule(), LeaderEligibility::Enforce)
        .expect("first apply");
    let b =
        validate_and_apply_header(&state, &header, &ledger(vk), &schedule(), LeaderEligibility::Enforce).expect("second apply");
    assert_eq!(a, b);
}

