// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Praos header validation.
//!
//! `validate_and_apply_header` is the single point of header admission
//! into `PraosChainDepState`. It composes — but does not re-implement —
//! the substrate transitions. The pipeline branches on the header's VRF
//! protocol:
//!
//!   1. forecast-horizon check (`EraSchedule::check_forecast_horizon`)
//!   2. monotone slot check (`state.last_slot`)
//!   3. monotone block-no check (`state.last_block_no`)
//!   4. op-cert counter monotonicity (pre-check vs `state.op_cert_counters`)
//!   5. VRF keyhash binding (`blake2b_256(vrf_vk) == pool_vrf_keyhash`)
//!   6. VRF verification + leader threshold + nonce derivation:
//!      - **TPraos** (Shelley..Alonzo): two role-tagged proofs; nonce-role
//!        output feeds the nonce, leader-role output feeds the threshold.
//!      - **Praos** (Babbage, Conway): one combined proof; the single output
//!        is range-extended to both a leader value and a nonce value.
//!   7. KES + op-cert verification (Praos headers — fail-closed)
//!   8. apply op-cert observation (`op_cert::apply_op_cert`)
//!   9. apply nonce contribution (`nonce::apply_nonce_input`)
//!  10. advance `last_slot` and `last_block_no`
//!
//! The pipeline is sequential and fail-fast — the first failure is the
//! only failure reported. No partial state is ever returned; on any
//! error the caller's state remains unchanged.
//!
//! `HeaderValidationError::BodyHashMismatch` and `EraMismatch` exist in
//! the closed enum but are not produced here — they belong to body-
//! admission (block-fetch / chain-db) consumers.

use ade_crypto::blake2b::blake2b_256;
use ade_crypto::vrf::VrfOutput;
use ade_types::{Hash32, SlotNo};

use crate::consensus::era_schedule::EraSchedule;
use crate::consensus::errors::{HeaderValidationError, VrfCertError};
use crate::consensus::header_summary::{HeaderInput, HeaderVrf, ValidatedHeaderSummary};
use crate::consensus::kes_check::verify_header_kes;
use crate::consensus::ledger_view::LedgerView;
use crate::consensus::nonce::{apply_nonce_input, NonceInput};
use crate::consensus::op_cert::{apply_op_cert, OpCertObservation};
use crate::consensus::praos_state::PraosChainDepState;
use crate::consensus::vrf_cert::{
    check_leader_claim, praos_leader_value, praos_nonce_value, verify_praos_vrf, verify_vrf_cert,
    StakeFraction, VrfRole,
};

/// The successful result of `validate_and_apply_header`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderApplied {
    pub new_state: PraosChainDepState,
    pub summary: ValidatedHeaderSummary,
}

/// The single point of header admission.
///
/// On success returns the new `PraosChainDepState` together with the
/// `ValidatedHeaderSummary` consumed downstream. On any failure the
/// caller's state is unchanged and a single typed
/// `HeaderValidationError` is returned.
///
/// Pure of wall clock and arrival order — same inputs always produce
/// the same result.
pub fn validate_and_apply_header(
    state: &PraosChainDepState,
    header: &HeaderInput,
    ledger_view: &dyn LedgerView,
    era_schedule: &EraSchedule,
) -> Result<HeaderApplied, HeaderValidationError> {
    // Step 1: forecast horizon.
    era_schedule
        .check_forecast_horizon(header.slot)
        .map_err(HeaderValidationError::OutsideForecastRange)?;

    // Step 2: monotone slot.
    if let Some(last) = state.last_slot {
        if header.slot.0 <= last.0 {
            return Err(HeaderValidationError::SlotBeforeLastApplied {
                last,
                attempted: header.slot,
            });
        }
    }

    // Step 3: monotone block-no.
    if let Some(last) = state.last_block_no {
        if header.block_no.0 <= last.0 {
            return Err(HeaderValidationError::BlockNoOutOfOrder {
                last,
                attempted: header.block_no,
            });
        }
    }

    // Step 4: op-cert counter pre-check. Detects regression before any
    // VRF work so the cheap rejection happens first. Per the Cardano
    // protocol, the op-cert counter is monotonically NON-DECREASING
    // across blocks signed by the same pool within a KES period —
    // strictly-less is a regression; equal is the SAME op-cert being
    // re-used (normal pool operation). PHASE4-N-M-FOLLOW.
    if let Some(existing) = state
        .op_cert_counters
        .get(&header.issuer_pool, header.op_cert_kes_period)
    {
        if header.op_cert_counter < existing {
            return Err(HeaderValidationError::OpCertCounter(
                crate::consensus::errors::OpCertCounterError::Regression {
                    existing,
                    attempted: header.op_cert_counter,
                },
            ));
        }
    }

    // Locate the (epoch) this header belongs to so the ledger view can
    // be consulted for the (sigma, asc) threshold inputs.
    let location = era_schedule
        .locate(header.slot)
        .map_err(HeaderValidationError::HFC)?;
    let epoch = location.epoch;

    // Step 5: VRF keyhash binding. The header carries the VRF vkey; the
    // snapshot carries only its hash. Reject if they disagree — this is what
    // ties the proof to the pool that holds the registered stake.
    if let Some(expected) = ledger_view.pool_vrf_keyhash(epoch, &header.issuer_pool) {
        let actual = Hash32(blake2b_256(&header.vrf_vk.0).0);
        if actual != expected {
            return Err(HeaderValidationError::VrfKeyhashMismatch { expected, actual });
        }
    }

    // Step 6: VRF verification + leader/nonce derivation. The output fed to
    // the leader threshold and the output fed to the nonce contribution differ
    // by protocol — encoded by the `HeaderVrf` variant.
    let (leader_value_output, nonce_output) = match &header.vrf {
        HeaderVrf::Tpraos {
            nonce_proof,
            leader_proof,
        } => {
            let verified_nonce = verify_vrf_cert(
                &header.vrf_vk,
                nonce_proof,
                header.slot,
                &state.epoch_nonce,
                VrfRole::NonceContribution,
            )
            .map_err(HeaderValidationError::VrfCert)?;
            let verified_leader = verify_vrf_cert(
                &header.vrf_vk,
                leader_proof,
                header.slot,
                &state.epoch_nonce,
                VrfRole::LeaderEligibility,
            )
            .map_err(HeaderValidationError::VrfCert)?;
            (verified_leader.output, verified_nonce.output)
        }
        HeaderVrf::Praos { proof, output } => {
            // Verify the single combined proof over the Praos input and bind
            // the recomputed output to the one the header carries.
            let recomputed =
                verify_praos_vrf(&header.vrf_vk, proof, header.slot, &state.epoch_nonce)
                    .map_err(HeaderValidationError::VrfCert)?;
            if &recomputed != output {
                return Err(HeaderValidationError::VrfCert(
                    VrfCertError::VerificationFailed,
                ));
            }
            // Range-extend the single output into a leader value and a nonce
            // value (cardano-base `vrfLeaderValue` / `vrfNonceValue`).
            let leader_value = praos_leader_value(&recomputed);
            let nonce_value = praos_nonce_value(&recomputed);
            // Carry the 32-byte nonce value in the high bytes of a VrfOutput so
            // the existing `HeaderContribution` transition mixes exactly it.
            let mut nonce_out = [0u8; 64];
            nonce_out[0..32].copy_from_slice(nonce_value.as_bytes());
            (leader_value, VrfOutput(nonce_out))
        }
    };

    // Step 7: leader threshold check.
    // Stake fraction and active-slots-coefficient come from the ledger
    // view. Missing pieces are treated as a failed leader claim — the
    // caller cannot prove leadership without ledger data, so we reject
    // structurally via `VerificationFailed`.
    let pool_stake = ledger_view
        .pool_active_stake(epoch, &header.issuer_pool)
        .ok_or(HeaderValidationError::VrfCert(
            VrfCertError::VerificationFailed,
        ))?;
    let total_stake = ledger_view
        .total_active_stake(epoch)
        .ok_or(HeaderValidationError::VrfCert(
            VrfCertError::VerificationFailed,
        ))?;
    if total_stake == 0 {
        return Err(HeaderValidationError::VrfCert(
            VrfCertError::VerificationFailed,
        ));
    }
    let asc = ledger_view
        .active_slots_coeff(epoch)
        .ok_or(HeaderValidationError::VrfCert(
            VrfCertError::VerificationFailed,
        ))?;
    let sigma = StakeFraction {
        numer: pool_stake,
        denom: total_stake,
    };
    check_leader_claim(&leader_value_output, sigma, asc)
        .map_err(HeaderValidationError::VrfCert)?;

    // Step 7b: KES signature + op-cert verification (Praos headers carry the
    // material; TPraos headers were authenticated under the legacy N-B model).
    if let Some(kes) = &header.kes {
        verify_header_kes(
            kes,
            header.slot,
            header.op_cert_counter,
            header.op_cert_kes_period,
        )?;
    }

    // Step 8: apply op-cert observation. Step 4 already gated the
    // counter; this call cannot fail in practice but the same typed
    // error is propagated defensively.
    let after_op_cert = apply_op_cert(
        state,
        &OpCertObservation {
            pool: header.issuer_pool.clone(),
            kes_period: header.op_cert_kes_period,
            counter: header.op_cert_counter,
        },
    )
    .map_err(HeaderValidationError::OpCertCounter)?;

    // Step 9: apply nonce contribution using the nonce-role output (TPraos)
    // or the derived nonce value (Praos). Mixing the leader output here would
    // silently desynchronise the evolving nonce from cardano-node.
    // freeze_boundary = firstSlotNextEpoch − RSW, read from the canonical era
    // geometry (DC-EPOCH-16). RSW (= ceil(4k/f)) is present on the live follow
    // path (derived from the venue genesis k). It is `None` only on a warm-start
    // schedule rebuilt from the durable sidecar (which carries no k): the
    // candidate freeze is then INERT and the boundary tick fails closed until B4
    // persists it. The sentinel is explicitly NOT a correctness value -- it must
    // never silently stand in for a forgotten RSW.
    const CANDIDATE_FREEZE_INERT: SlotNo = SlotNo(u64::MAX);
    let freeze_boundary = match era_schedule
        .eras()
        .get(location.era_index as usize)
        .and_then(|era| {
            era.randomness_stabilisation_window_slots
                .map(|rsw| (era.epoch_length_slots, rsw))
        }) {
        Some((epoch_length_slots, rsw)) => {
            let epoch_start = header
                .slot
                .0
                .saturating_sub(u64::from(location.relative_slot_in_epoch));
            let first_slot_next_epoch = epoch_start.saturating_add(u64::from(epoch_length_slots));
            SlotNo(first_slot_next_epoch.saturating_sub(u64::from(rsw)))
        }
        None => CANDIDATE_FREEZE_INERT,
    };
    let after_nonce = apply_nonce_input(
        &after_op_cert,
        &NonceInput::HeaderContribution {
            slot: header.slot,
            prev_block_hash: header.prev_hash.clone(),
            vrf_nonce_output: nonce_output,
            freeze_boundary,
        },
    )
    .map_err(HeaderValidationError::Nonce)?;

    // Step 10: advance `last_block_no`. `apply_nonce_input` already
    // advanced `last_slot` as part of the HeaderContribution transition.
    let mut new_state = after_nonce;
    new_state.last_block_no = Some(header.block_no);

    let summary = ValidatedHeaderSummary {
        slot: header.slot,
        block_no: header.block_no,
        body_hash: header.body_hash.clone(),
        issuer_pool: header.issuer_pool.clone(),
        op_cert_counter: header.op_cert_counter,
        vrf_leader_output: leader_value_output,
    };
    Ok(HeaderApplied { new_state, summary })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use ade_crypto::blake2b::blake2b_256;
    use ade_crypto::vrf::{VrfProof, VrfVerificationKey};
    use ade_types::{BlockNo, CardanoEra, EpochNo, Hash28, Hash32, SlotNo};
    use cardano_crypto::vrf::VrfDraft03;

    use crate::consensus::era_schedule::{BootstrapAnchorHash, EraSummary};
    use crate::consensus::praos_state::{Nonce, PraosChainDepState};
    use crate::consensus::vrf_cert::{vrf_input, ActiveSlotsCoeff, VrfRole};

    // Minimal inline LedgerView stub. We cannot use
    // `ade_testkit::consensus::ledger_view_stub::LedgerViewStub` here
    // because ade_testkit dev-depends on ade_core, which causes a
    // version-graph mismatch when the testkit is pulled back into a
    // unit test inside ade_core. Integration tests under
    // `ade_core/tests/` do not have this restriction.
    struct StubLedger {
        vk: VrfVerificationKey,
        pool: Hash28,
        asc: ActiveSlotsCoeff,
        sigma: (u64, u64),
    }

    impl crate::consensus::ledger_view::LedgerView for StubLedger {
        fn total_active_stake(&self, _: EpochNo) -> Option<u64> {
            Some(self.sigma.1)
        }
        fn pool_active_stake(&self, _: EpochNo, pool: &Hash28) -> Option<u64> {
            (pool == &self.pool).then_some(self.sigma.0)
        }
        fn pool_vrf_keyhash(&self, _: EpochNo, pool: &Hash28) -> Option<Hash32> {
            (pool == &self.pool).then(|| ade_crypto::blake2b::blake2b_256(&self.vk.0))
        }
        fn active_slots_coeff(&self, _: EpochNo) -> Option<ActiveSlotsCoeff> {
            Some(self.asc)
        }
    }

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
            .expect("schedule is well-formed")
    }

    fn pool() -> Hash28 {
        Hash28([0xAA; 28])
    }

    fn key_material(seed: [u8; 32]) -> ([u8; 64], VrfVerificationKey) {
        let (sk, vk_bytes) = VrfDraft03::keypair_from_seed(&seed);
        (sk, VrfVerificationKey(vk_bytes))
    }

    fn prove_nonce(sk: &[u8; 64], slot: SlotNo, epoch_nonce: &Nonce) -> VrfProof {
        let alpha = vrf_input(slot, epoch_nonce, VrfRole::NonceContribution);
        let proof_bytes = VrfDraft03::prove(sk, &alpha).expect("VRF prove");
        VrfProof(proof_bytes)
    }

    fn prove_leader(sk: &[u8; 64], slot: SlotNo, epoch_nonce: &Nonce) -> VrfProof {
        let alpha = vrf_input(slot, epoch_nonce, VrfRole::LeaderEligibility);
        let proof_bytes = VrfDraft03::prove(sk, &alpha).expect("VRF prove");
        VrfProof(proof_bytes)
    }

    fn ledger(vk: VrfVerificationKey) -> StubLedger {
        // asc = 1/1 + sigma = 1/1 makes the leader threshold trivially
        // pass for every VRF output (see vrf_cert::is_leader boundary
        // handling). Suitable for happy-path composition tests.
        StubLedger {
            vk,
            pool: pool(),
            asc: ActiveSlotsCoeff { numer: 1, denom: 1 },
            sigma: (1, 1),
        }
    }

    fn genesis_state() -> PraosChainDepState {
        let mut s = PraosChainDepState::empty();
        s.epoch_nonce = Nonce(Hash32([0xCD; 32]));
        s.evolving_nonce = Nonce(Hash32([0xEE; 32]));
        s
    }

    fn happy_header(
        sk: &[u8; 64],
        vk: VrfVerificationKey,
        state: &PraosChainDepState,
    ) -> HeaderInput {
        let slot = SlotNo(1);
        HeaderInput {
            prev_hash: Hash32([0u8; 32]),
            slot,
            block_no: BlockNo(1),
            body_hash: Hash32([0x55; 32]),
            issuer_pool: pool(),
            op_cert_kes_period: 0,
            op_cert_counter: 0,
            vrf_vk: vk,
            vrf: HeaderVrf::Tpraos {
                nonce_proof: prove_nonce(sk, slot, &state.epoch_nonce),
                leader_proof: prove_leader(sk, slot, &state.epoch_nonce),
            },
            kes: None,
        }
    }

    #[test]
    fn pipeline_short_circuits_on_first_failure() {
        // Construct a state where last_slot is already past the header's
        // slot — step 2 must fail. The header carries *invalid* VRF
        // proofs which would fail at step 5 if step 2 did not short-
        // circuit. We assert (a) the returned error is the step-2 error,
        // not the step-5 error, and (b) the state is unchanged.
        let (sk, vk) = key_material([7u8; 32]);
        let mut state = genesis_state();
        state.last_slot = Some(SlotNo(10));
        let snapshot = state.clone();

        // Header slot = 5 < last_slot = 10 → step 2 fail.
        // VRF proofs intentionally bogus (wrong slot's alpha) so step 5
        // would also fail if reached.
        let bogus_proof = prove_nonce(&sk, SlotNo(999), &state.epoch_nonce);
        let bogus_leader = prove_leader(&sk, SlotNo(999), &state.epoch_nonce);
        let header = HeaderInput {
            prev_hash: Hash32([0u8; 32]),
            slot: SlotNo(5),
            block_no: BlockNo(1),
            body_hash: Hash32([0u8; 32]),
            issuer_pool: pool(),
            op_cert_kes_period: 0,
            op_cert_counter: 0,
            vrf_vk: vk.clone(),
            vrf: HeaderVrf::Tpraos {
                nonce_proof: bogus_proof,
                leader_proof: bogus_leader,
            },
            kes: None,
        };

        let res = validate_and_apply_header(&state, &header, &ledger(vk), &schedule());
        assert_eq!(
            res,
            Err(HeaderValidationError::SlotBeforeLastApplied {
                last: SlotNo(10),
                attempted: SlotNo(5),
            })
        );
        // State unchanged.
        assert_eq!(state, snapshot);
    }

    #[test]
    fn nonce_contribution_uses_nonce_role_vrf_output_not_leader_role() {
        let (sk, vk) = key_material([3u8; 32]);
        let state = genesis_state();
        let header = happy_header(&sk, vk.clone(), &state);

        // Compute both VRF outputs independently so we can verify which
        // one the evolving nonce mixes in.
        let (nonce_proof, leader_proof) = match &header.vrf {
            HeaderVrf::Tpraos {
                nonce_proof,
                leader_proof,
            } => (nonce_proof, leader_proof),
            HeaderVrf::Praos { .. } => panic!("happy_header builds a TPraos header"),
        };
        let nonce_alpha = vrf_input(header.slot, &state.epoch_nonce, VrfRole::NonceContribution);
        let nonce_output_bytes = VrfDraft03::verify(&vk.0, &nonce_proof.0, &nonce_alpha)
            .expect("nonce proof verifies");
        let leader_alpha = vrf_input(header.slot, &state.epoch_nonce, VrfRole::LeaderEligibility);
        let leader_output_bytes = VrfDraft03::verify(&vk.0, &leader_proof.0, &leader_alpha)
            .expect("leader proof verifies");

        // The two outputs MUST differ — otherwise the test cannot tell
        // which one was used.
        assert_ne!(nonce_output_bytes, leader_output_bytes);

        let applied =
            validate_and_apply_header(&state, &header, &ledger(vk), &schedule()).expect("happy");

        // Expected evolving nonce: blake2b256(prior_evolving ‖ nonce_output[0..32]).
        let mut buf = [0u8; 64];
        buf[0..32].copy_from_slice(state.evolving_nonce.as_bytes());
        buf[32..64].copy_from_slice(&nonce_output_bytes[0..32]);
        let expected = blake2b_256(&buf);
        assert_eq!(applied.new_state.evolving_nonce, Nonce(expected));

        // Sanity: if we had (incorrectly) used the leader-role output,
        // the evolving nonce would be a *different* hash.
        let mut buf_leader = [0u8; 64];
        buf_leader[0..32].copy_from_slice(state.evolving_nonce.as_bytes());
        buf_leader[32..64].copy_from_slice(&leader_output_bytes[0..32]);
        let leader_hash = blake2b_256(&buf_leader);
        assert_ne!(
            applied.new_state.evolving_nonce,
            Nonce(leader_hash),
            "evolving nonce silently used leader-role output instead of nonce-role"
        );
    }
}
