// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE leader-check evaluator (PHASE4-N-R-A S2 / A2).
//!
//! `verify_and_evaluate_leader` is the closed BLUE function that:
//!
//! 1. Verifies a VRF proof against canonical inputs (slot, epoch
//!    nonce, leader role).
//! 2. Cross-checks that the caller's `LeaderScheduleAnswer` was
//!    derived for the same `(slot, eta0)`.
//! 3. Evaluates leader eligibility from the verified VRF output +
//!    threshold context.
//! 4. Returns a closed two-variant `LeaderCheckVerdict` whose
//!    `Eligible` variant carries forge-capable material and whose
//!    `NotEligible` variant carries only bounded `vrf_output_fingerprint`
//!    evidence.
//!
//! Color: BLUE. No dependency on `LedgerView`, `EraSchedule`,
//! `PraosChainDepState`, wall-clock, storage, or RED crates.
//! The caller derives `LeaderScheduleAnswer` via the authority
//! path (`query_leader_schedule`) and passes it in by reference.
//!
//! Key-custody: this module never imports `KesSecret`,
//! `VrfSigningKey`, or `ColdSigningKey`. CI guard
//! `ci/ci_check_leader_check_no_red_imports.sh` enforces this
//! mechanically.
//!
//! Doctrine: see [[feedback-shell-must-not-overstate-semantic-truth]] —
//! BLUE produces structurally-bounded verdicts, never RED tokens.
//! `NotEligible` MUST NOT carry the verified `vrf_output`; only
//! a non-forge-capable fingerprint.

use ade_crypto::vrf::{verify_vrf, VrfOutput, VrfProof, VrfVerificationKey};
use ade_types::SlotNo;

use crate::consensus::leader_schedule::LeaderScheduleAnswer;
use crate::consensus::praos_state::Nonce;
use crate::consensus::vrf_cert::{is_leader, vrf_input, StakeFraction, VrfRole, VRF_INPUT_LEN};

/// Compose the final per-VRF-output leadership decision from a
/// `LeaderScheduleAnswer`'s threshold context.
///
/// Delegates to `vrf_cert::is_leader` using the answer's
/// `stake_fraction` + `asc`. Pure — no I/O, no allocation.
///
/// **Canonical authority for leader eligibility.** Relocated from
/// `leader_schedule.rs` in PHASE4-N-R-A S2 so the BLUE
/// leader-check module owns the eligibility rule. The function is
/// re-exported by `crate::consensus::mod` for backward compatibility
/// with the `ade_ledger::producer::forge` defense-in-depth pin
/// (NC-VRF-3: single source of leader truth). New external callers
/// MUST use [`verify_and_evaluate_leader`] — the CI gate
/// `ci/ci_check_leader_check_authority.sh` enforces the allow-list
/// (only `leader_check.rs` definition + `forge.rs` pin/defense are
/// permitted).
pub fn is_leader_for_vrf_output(answer: &LeaderScheduleAnswer, output: &VrfOutput) -> bool {
    let sigma = StakeFraction {
        numer: answer.stake_fraction.0,
        denom: answer.stake_fraction.1,
    };
    is_leader(output, sigma, answer.asc)
}

/// First-8-bytes fingerprint of a VRF output. Non-forge-capable:
/// callers cannot reconstruct the full `VrfOutput` from the
/// fingerprint, so `NotEligible` carriers cannot derive leader
/// material from this evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VrfOutputFingerprint(pub [u8; 8]);

impl VrfOutputFingerprint {
    /// Compute the fingerprint from a verified VRF output.
    pub fn of(output: &VrfOutput) -> Self {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&output.0[..8]);
        Self(buf)
    }
}

/// Non-forge-capable cryptographic binding of the leader-check
/// verdict to the specific VRF proof that produced it.
/// Modelled as the first 8 bytes of the verified VRF output —
/// uniquely identifies the proof under VRF determinism, but
/// cannot be used as input to any forge primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LeaderProofFingerprint(pub [u8; 8]);

impl LeaderProofFingerprint {
    pub fn of(output: &VrfOutput) -> Self {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&output.0[..8]);
        Self(buf)
    }
}

/// Closed two-variant leader-check verdict.
///
/// `Eligible` carries the full `vrf_output` (forge-capable
/// material the RED forge handler consumes) plus a
/// `leader_proof` fingerprint (audit binding).
///
/// `NotEligible` carries ONLY `vrf_output_fingerprint`
/// (non-forge-capable evidence). The full `vrf_output` is
/// **not** exposed in this variant — the closed enum shape
/// makes it structurally impossible for a `NotEligible`
/// caller to observe forge-capable material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaderCheckVerdict {
    Eligible {
        slot: SlotNo,
        vrf_output: VrfOutput,
        leader_proof: LeaderProofFingerprint,
    },
    NotEligible {
        slot: SlotNo,
        vrf_output_fingerprint: VrfOutputFingerprint,
    },
}

/// Closed error sum. Each variant is fail-fast at the
/// BLUE / RED boundary; replay equivalence requires
/// byte-identical error verdicts across runs, so no
/// `String` payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaderCheckError {
    /// The supplied `LeaderScheduleAnswer.expected_vrf_input`
    /// does not match the canonical input derived from
    /// `(slot, eta0)`. Defensive coherence check — caller
    /// passed a mismatching answer.
    VrfInputMismatch {
        expected_first_8: [u8; 8],
        answer_first_8: [u8; 8],
    },
    /// The supplied `LeaderScheduleAnswer.slot` does not
    /// match the `slot` argument. Defensive coherence check.
    AnswerSlotMismatch {
        function_slot: u64,
        answer_slot: u64,
    },
    /// `verify_vrf` rejected the proof against
    /// `(vrf_vk, expected_vrf_input)`.
    VrfVerificationFailed,
    /// `LeaderScheduleAnswer.stake_fraction.denom == 0`.
    /// Should not occur if the caller used
    /// `query_leader_schedule` to build the answer;
    /// variant exists for strict totality.
    ZeroStakeDenominator,
}

/// Verify a VRF proof and evaluate leader eligibility.
///
/// Pure BLUE function. Same inputs → byte-identical verdict.
///
/// Pipeline (every step total and deterministic):
///
/// 1. **Coherence check.** `answer.slot == slot` and
///    `answer.expected_vrf_input == vrf_input(slot, eta0, LeaderEligibility)`.
///    Fail-closed with structured errors on mismatch.
/// 2. **Zero-denominator guard.** `answer.stake_fraction.1 > 0`.
/// 3. **VRF proof verification.** `verify_vrf(vrf_vk, vrf_proof,
///    answer.expected_vrf_input)` returns the verified
///    `VrfOutput` or `VrfVerificationFailed`.
/// 4. **Threshold evaluation.**
///    `is_leader_for_vrf_output(answer, &vrf_output)` returns
///    the boolean.
/// 5. **Verdict construction.** `Eligible { vrf_output,
///    leader_proof }` on true; `NotEligible {
///    vrf_output_fingerprint }` on false.
pub fn verify_and_evaluate_leader(
    slot: SlotNo,
    eta0: &Nonce,
    vrf_vk: &VrfVerificationKey,
    vrf_proof: &VrfProof,
    answer: &LeaderScheduleAnswer,
) -> Result<LeaderCheckVerdict, LeaderCheckError> {
    if answer.slot != slot {
        return Err(LeaderCheckError::AnswerSlotMismatch {
            function_slot: slot.0,
            answer_slot: answer.slot.0,
        });
    }

    let expected: [u8; VRF_INPUT_LEN] = vrf_input(slot, eta0, VrfRole::LeaderEligibility);
    if expected != answer.expected_vrf_input {
        let mut expected_first_8 = [0u8; 8];
        expected_first_8.copy_from_slice(&expected[..8]);
        let mut answer_first_8 = [0u8; 8];
        answer_first_8.copy_from_slice(&answer.expected_vrf_input[..8]);
        return Err(LeaderCheckError::VrfInputMismatch {
            expected_first_8,
            answer_first_8,
        });
    }

    if answer.stake_fraction.1 == 0 {
        return Err(LeaderCheckError::ZeroStakeDenominator);
    }

    let vrf_output = verify_vrf(vrf_vk, vrf_proof, &expected)
        .map_err(|_| LeaderCheckError::VrfVerificationFailed)?;

    let fingerprint = VrfOutputFingerprint::of(&vrf_output);

    if is_leader_for_vrf_output(answer, &vrf_output) {
        let leader_proof = LeaderProofFingerprint::of(&vrf_output);
        Ok(LeaderCheckVerdict::Eligible {
            slot,
            vrf_output,
            leader_proof,
        })
    } else {
        Ok(LeaderCheckVerdict::NotEligible {
            slot,
            vrf_output_fingerprint: fingerprint,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use ade_types::{EpochNo, Hash28, Hash32};
    use cardano_crypto::vrf::VrfDraft03;

    use crate::consensus::vrf_cert::ActiveSlotsCoeff;

    fn build_answer(
        slot: SlotNo,
        eta0: &Nonce,
        stake_numer: u64,
        stake_denom: u64,
        asc_numer: u32,
        asc_denom: u32,
    ) -> LeaderScheduleAnswer {
        LeaderScheduleAnswer {
            slot,
            pool: Hash28([0xAA; 28]),
            epoch: EpochNo(0),
            expected_vrf_input: vrf_input(slot, eta0, VrfRole::LeaderEligibility),
            stake_fraction: (stake_numer, stake_denom),
            asc: ActiveSlotsCoeff {
                numer: asc_numer,
                denom: asc_denom,
            },
        }
    }

    fn make_keypair(seed_byte: u8) -> (VrfProof, VrfVerificationKey, [u8; VRF_INPUT_LEN]) {
        let seed = [seed_byte; 32];
        let (sk, vk_bytes) = VrfDraft03::keypair_from_seed(&seed);
        let eta0 = Nonce(Hash32([0xCD; 32]));
        let alpha = vrf_input(SlotNo(42), &eta0, VrfRole::LeaderEligibility);
        let proof_bytes = VrfDraft03::prove(&sk, &alpha).unwrap();
        (VrfProof(proof_bytes), VrfVerificationKey(vk_bytes), alpha)
    }

    #[test]
    fn eligible_on_threshold_with_high_stake_emits_eligible_verdict() {
        let eta0 = Nonce(Hash32([0xCD; 32]));
        let (proof, vk, _alpha) = make_keypair(0x11);
        // 100% stake, ASC = 100% → every slot is a leader slot.
        let answer = build_answer(SlotNo(42), &eta0, 1, 1, 1, 1);
        let verdict =
            verify_and_evaluate_leader(SlotNo(42), &eta0, &vk, &proof, &answer).unwrap();
        match verdict {
            LeaderCheckVerdict::Eligible {
                slot,
                vrf_output: _,
                leader_proof: _,
            } => {
                assert_eq!(slot, SlotNo(42));
            }
            LeaderCheckVerdict::NotEligible { .. } => panic!("expected Eligible"),
        }
    }

    #[test]
    fn not_eligible_with_zero_stake_emits_not_eligible_verdict() {
        let eta0 = Nonce(Hash32([0xCD; 32]));
        let (proof, vk, _alpha) = make_keypair(0x22);
        // 0% stake → impossible to be a leader.
        let answer = build_answer(SlotNo(42), &eta0, 0, 1, 1, 1);
        let verdict =
            verify_and_evaluate_leader(SlotNo(42), &eta0, &vk, &proof, &answer).unwrap();
        match verdict {
            LeaderCheckVerdict::NotEligible {
                slot,
                vrf_output_fingerprint: _,
            } => {
                assert_eq!(slot, SlotNo(42));
            }
            LeaderCheckVerdict::Eligible { .. } => panic!("expected NotEligible"),
        }
    }

    #[test]
    fn malformed_proof_emits_verification_failed() {
        let eta0 = Nonce(Hash32([0xCD; 32]));
        let (_real_proof, vk, _alpha) = make_keypair(0x33);
        // Replace the proof with all-zero bytes — verification must fail.
        let bad_proof = VrfProof([0u8; 80]);
        let answer = build_answer(SlotNo(42), &eta0, 1, 1, 1, 1);
        let err = verify_and_evaluate_leader(SlotNo(42), &eta0, &vk, &bad_proof, &answer)
            .unwrap_err();
        assert_eq!(err, LeaderCheckError::VrfVerificationFailed);
    }

    #[test]
    fn wrong_vk_emits_verification_failed() {
        let eta0 = Nonce(Hash32([0xCD; 32]));
        let (proof, _vk, _alpha) = make_keypair(0x44);
        // Use a different VK — verification must fail.
        let wrong_vk = VrfVerificationKey([0xEE; 32]);
        let answer = build_answer(SlotNo(42), &eta0, 1, 1, 1, 1);
        let err = verify_and_evaluate_leader(SlotNo(42), &eta0, &wrong_vk, &proof, &answer)
            .unwrap_err();
        assert_eq!(err, LeaderCheckError::VrfVerificationFailed);
    }

    #[test]
    fn answer_slot_mismatch_emits_structured_error() {
        let eta0 = Nonce(Hash32([0xCD; 32]));
        let (proof, vk, _alpha) = make_keypair(0x55);
        let answer = build_answer(SlotNo(99), &eta0, 1, 1, 1, 1); // wrong slot in answer
        let err = verify_and_evaluate_leader(SlotNo(42), &eta0, &vk, &proof, &answer)
            .unwrap_err();
        assert_eq!(
            err,
            LeaderCheckError::AnswerSlotMismatch {
                function_slot: 42,
                answer_slot: 99,
            }
        );
    }

    #[test]
    fn vrf_input_mismatch_emits_structured_error() {
        let eta0 = Nonce(Hash32([0xCD; 32]));
        let other_eta0 = Nonce(Hash32([0xAB; 32]));
        let (proof, vk, _alpha) = make_keypair(0x66);
        // answer derived for other_eta0; call argument is eta0.
        let answer = build_answer(SlotNo(42), &other_eta0, 1, 1, 1, 1);
        let err = verify_and_evaluate_leader(SlotNo(42), &eta0, &vk, &proof, &answer)
            .unwrap_err();
        // The first 8 bytes of expected_vrf_input encode the slot, so
        // they're equal in this fixture (same slot, different eta0).
        // The mismatch is in the epoch_nonce portion (bytes 8..40) —
        // the structured error variant existing is the gate, not the
        // diagnostic bytes.
        assert!(matches!(err, LeaderCheckError::VrfInputMismatch { .. }));
    }

    #[test]
    fn zero_stake_denominator_emits_structured_error() {
        let eta0 = Nonce(Hash32([0xCD; 32]));
        let (proof, vk, _alpha) = make_keypair(0x77);
        let answer = build_answer(SlotNo(42), &eta0, 1, 0, 1, 1); // denom = 0
        let err = verify_and_evaluate_leader(SlotNo(42), &eta0, &vk, &proof, &answer)
            .unwrap_err();
        assert_eq!(err, LeaderCheckError::ZeroStakeDenominator);
    }

    #[test]
    fn verdict_is_byte_identical_across_two_runs() {
        let eta0 = Nonce(Hash32([0xCD; 32]));
        let (proof, vk, _alpha) = make_keypair(0x88);
        let answer = build_answer(SlotNo(42), &eta0, 1, 1, 1, 1);
        let v1 = verify_and_evaluate_leader(SlotNo(42), &eta0, &vk, &proof, &answer).unwrap();
        let v2 = verify_and_evaluate_leader(SlotNo(42), &eta0, &vk, &proof, &answer).unwrap();
        assert_eq!(v1, v2);
    }

    #[test]
    fn vrf_output_fingerprint_is_first_8_bytes_of_output() {
        let output = VrfOutput([0x11; 64]);
        let fp = VrfOutputFingerprint::of(&output);
        assert_eq!(fp.0, [0x11; 8]);
    }
}
