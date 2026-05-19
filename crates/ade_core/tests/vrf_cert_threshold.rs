// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_core::consensus::{
    check_leader_claim, is_leader, verify_vrf_cert, ActiveSlotsCoeff, Nonce, StakeFraction,
    VerifiedVrf, VrfCertError, VrfRole, VRF_INPUT_LEN,
};
use ade_crypto::vrf::{VrfOutput, VrfProof, VrfVerificationKey};
use ade_types::{Hash32, SlotNo};
use cardano_crypto::vrf::VrfDraft03;

fn nonce(byte: u8) -> Nonce {
    Nonce(Hash32([byte; 32]))
}

fn build_alpha(slot: SlotNo, n: &Nonce, role: VrfRole) -> [u8; VRF_INPUT_LEN] {
    let mut out = [0u8; VRF_INPUT_LEN];
    out[0..8].copy_from_slice(&slot.0.to_be_bytes());
    out[8..40].copy_from_slice(n.as_bytes());
    out[40] = role.tag_byte();
    out
}

#[test]
fn vrf_input_layout_is_41_bytes_with_correct_tag() {
    let slot = SlotNo(0xDEAD_BEEF_CAFE_BABE);
    let n = nonce(0x33);
    let alpha_n = build_alpha(slot, &n, VrfRole::NonceContribution);
    let alpha_l = build_alpha(slot, &n, VrfRole::LeaderEligibility);

    assert_eq!(alpha_n.len(), 41);
    assert_eq!(alpha_l.len(), 41);
    assert_eq!(
        &alpha_n[0..8],
        &[0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE]
    );
    assert_eq!(&alpha_n[8..40], n.as_bytes());
    assert_eq!(alpha_n[40], 0x4E);
    assert_eq!(alpha_l[40], 0x4C);
    // First 40 bytes identical between roles — the only difference is the tag byte.
    assert_eq!(&alpha_n[..40], &alpha_l[..40]);
}

#[test]
fn verify_vrf_cert_accepts_valid_proof() {
    let seed = [11u8; 32];
    let (sk, vk_bytes) = VrfDraft03::keypair_from_seed(&seed);
    let slot = SlotNo(12345);
    let en = nonce(0x55);
    let role = VrfRole::NonceContribution;
    let alpha = build_alpha(slot, &en, role);
    let proof_bytes = VrfDraft03::prove(&sk, &alpha).unwrap();

    let vk = VrfVerificationKey(vk_bytes);
    let proof = VrfProof(proof_bytes);

    let result = verify_vrf_cert(&vk, &proof, slot, &en, role).unwrap();
    let VerifiedVrf {
        role: r,
        slot: s,
        output: _,
    } = result;
    assert_eq!(r, VrfRole::NonceContribution);
    assert_eq!(s, slot);
}

#[test]
fn verify_vrf_cert_rejects_wrong_alpha() {
    let seed = [22u8; 32];
    let (sk, vk_bytes) = VrfDraft03::keypair_from_seed(&seed);
    let slot = SlotNo(12345);
    let wrong_slot = SlotNo(12346);
    let en = nonce(0x77);
    let role = VrfRole::NonceContribution;
    let alpha = build_alpha(slot, &en, role);
    let proof_bytes = VrfDraft03::prove(&sk, &alpha).unwrap();

    let vk = VrfVerificationKey(vk_bytes);
    let proof = VrfProof(proof_bytes);

    let err = verify_vrf_cert(&vk, &proof, wrong_slot, &en, role).unwrap_err();
    assert_eq!(err, VrfCertError::VerificationFailed);
}

#[test]
fn verify_vrf_cert_rejects_malformed_proof() {
    let seed = [33u8; 32];
    let (_, vk_bytes) = VrfDraft03::keypair_from_seed(&seed);
    let vk = VrfVerificationKey(vk_bytes);

    // Construct an 80-byte all-zero proof. It is length-correct so the
    // length check passes, but the verifier will reject it. The mapping
    // is to VerificationFailed (the bytes are well-formed-length, but
    // cryptographically invalid).
    let proof = VrfProof([0u8; 80]);
    let slot = SlotNo(1);
    let en = nonce(0xAA);

    let err = verify_vrf_cert(&vk, &proof, slot, &en, VrfRole::LeaderEligibility).unwrap_err();
    assert_eq!(err, VrfCertError::VerificationFailed);
}

#[test]
fn is_leader_zero_stake_never_leads() {
    let output = VrfOutput([0u8; 64]);
    let sigma = StakeFraction { numer: 0, denom: 1 };
    let asc = ActiveSlotsCoeff {
        numer: 1,
        denom: 20,
    };
    for hi in [0u8, 1, 0x7F, 0xFF].iter().copied() {
        let mut bytes = [0u8; 64];
        bytes[0] = hi;
        let out = VrfOutput(bytes);
        assert!(!is_leader(&out, sigma, asc));
        assert!(!is_leader(&output, sigma, asc));
    }
}

#[test]
fn is_leader_full_stake_always_leads() {
    // σ = 1 means the pool owns all stake. The threshold becomes
    // 1 - (1 - f)^1 = f. For any output strictly below `f`, the pool
    // leads. With asc = 1 (which short-circuits to true), it leads
    // regardless of output.
    let sigma = StakeFraction { numer: 1, denom: 1 };
    let asc_full = ActiveSlotsCoeff { numer: 1, denom: 1 };
    for hi in [0u8, 1, 0x7F, 0xFF].iter().copied() {
        let mut bytes = [0u8; 64];
        bytes[0] = hi;
        let out = VrfOutput(bytes);
        assert!(is_leader(&out, sigma, asc_full));
    }
}

#[test]
fn is_leader_determinism() {
    let bytes = [0x12u8; 64];
    let out = VrfOutput(bytes);
    let sigma = StakeFraction {
        numer: 3,
        denom: 100,
    };
    let asc = ActiveSlotsCoeff {
        numer: 1,
        denom: 20,
    };
    let r1 = is_leader(&out, sigma, asc);
    let r2 = is_leader(&out, sigma, asc);
    let r3 = is_leader(&out, sigma, asc);
    assert_eq!(r1, r2);
    assert_eq!(r2, r3);
}

/// Self-consistent synthetic vector pinning the comparison.
///
/// Mainnet `f = 1/20`. For σ = 1/2, the threshold p = 1 - (1 - 1/20)^(1/2)
/// ≈ 1 - sqrt(19/20) ≈ 1 - 0.97468 ≈ 0.02532. So any VRF output whose
/// top byte makes the value < ~0.02532 should lead.
///
/// We pin two outputs:
/// - top byte 0x01 -> value ≈ 1/256 ≈ 0.00391 (well below threshold) -> leads
/// - top byte 0x80 -> value ≈ 0.5 (well above threshold) -> does NOT lead
///
/// S-B10's live-interop pass replaces this with a real cardano-node 10.6.2
/// vector pinned against the oracle.
#[test]
fn is_leader_known_vector_matches_reference() {
    let sigma = StakeFraction { numer: 1, denom: 2 };
    let asc = ActiveSlotsCoeff {
        numer: 1,
        denom: 20,
    };

    let mut low_bytes = [0u8; 64];
    low_bytes[0] = 0x01;
    let low_output = VrfOutput(low_bytes);
    assert!(
        is_leader(&low_output, sigma, asc),
        "value ≈ 1/256 must be below threshold ≈ 0.0253 for σ=1/2, f=1/20"
    );

    let mut hi_bytes = [0u8; 64];
    hi_bytes[0] = 0x80;
    let hi_output = VrfOutput(hi_bytes);
    assert!(
        !is_leader(&hi_output, sigma, asc),
        "value ≈ 0.5 must be above threshold ≈ 0.0253 for σ=1/2, f=1/20"
    );
}

#[test]
fn check_leader_claim_returns_typed_error_on_above_threshold() {
    // Construct a vector where the leader value is well above the
    // threshold. σ = 1/1000 with mainnet f gives a threshold around
    // 1 - (19/20)^(1/1000) ≈ 5.13e-5. A leader_value of ~0.5 is
    // overwhelmingly above this threshold.
    let mut bytes = [0u8; 64];
    bytes[0] = 0x80;
    let out = VrfOutput(bytes);
    let sigma = StakeFraction {
        numer: 1,
        denom: 1000,
    };
    let asc = ActiveSlotsCoeff {
        numer: 1,
        denom: 20,
    };

    let err = check_leader_claim(&out, sigma, asc).unwrap_err();
    match err {
        VrfCertError::LeaderValueAboveThreshold { value, threshold } => {
            assert_eq!(value[0], 0x80);
            // threshold is tiny; the high byte must be 0.
            assert_eq!(threshold[0], 0x00);
        }
        other => panic!("expected LeaderValueAboveThreshold, got {other:?}"),
    }
}
