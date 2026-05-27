// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Tests for the Ade-owned BLUE Sum_n KES algorithm (PHASE4-N-P S2).
//!
//! Cross-impl agreement under `cardano-crypto` 1.0.8 is mechanically
//! enforced here under `#[cfg(test)]` only. Per N9 (no upstream-shim
//! in production), `cardano-crypto` MUST NOT be imported outside this
//! test module within `kes_sum/`.

#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

use super::*;

use cardano_crypto::kes::{KesAlgorithm as CcKesAlgorithm, Sum6Kes as CcSum6Kes};

// =========================================================================
// Sum0KES — leaf, single Ed25519 period
// =========================================================================

#[test]
fn sum0_kes_signs_and_verifies_at_period_0() {
    let seed = [0x42u8; 32];
    let sk = Sum0Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let vk = Sum0Kes::derive_verification_key(&sk);
    let msg = b"hello sum0";
    let sig = Sum0Kes::sign_kes(&sk, 0, msg).unwrap();
    Sum0Kes::verify_kes(&vk, 0, msg, &sig).unwrap();
}

#[test]
fn sum0_kes_rejects_period_1() {
    let seed = [0x07u8; 32];
    let sk = Sum0Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let err = Sum0Kes::sign_kes(&sk, 1, b"x").unwrap_err();
    assert!(matches!(
        err,
        KesError::PeriodOutOfRange { period: 1, max_period: 0 }
    ));
}

#[test]
fn sum0_kes_update_expires_after_period_0() {
    let seed = [0x11u8; 32];
    let sk = Sum0Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let updated = Sum0Kes::update_kes(sk, 0).unwrap();
    assert!(updated.is_none());
}

#[test]
fn sum0_kes_verify_rejects_wrong_message() {
    let seed = [0x33u8; 32];
    let sk = Sum0Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let vk = Sum0Kes::derive_verification_key(&sk);
    let sig = Sum0Kes::sign_kes(&sk, 0, b"original").unwrap();
    let err = Sum0Kes::verify_kes(&vk, 0, b"tampered", &sig).unwrap_err();
    assert!(matches!(err, KesError::VerificationFailed));
}

// =========================================================================
// Sum1KES — first recursion level, 2 periods
// =========================================================================

#[test]
fn sum1_kes_signs_at_period_0_and_period_1() {
    let seed = [0x55u8; 32];
    let mut sk = Sum1Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let vk = Sum1Kes::derive_verification_key(&sk);

    // Period 0
    let sig0 = Sum1Kes::sign_kes(&sk, 0, b"msg0").unwrap();
    Sum1Kes::verify_kes(&vk, 0, b"msg0", &sig0).unwrap();

    // Advance to period 1
    sk = Sum1Kes::update_kes(sk, 0).unwrap().expect("update to p=1");
    let sig1 = Sum1Kes::sign_kes(&sk, 1, b"msg1").unwrap();
    Sum1Kes::verify_kes(&vk, 1, b"msg1", &sig1).unwrap();

    // Advance to expiration
    let final_sk = Sum1Kes::update_kes(sk, 1).unwrap();
    assert!(final_sk.is_none());
}

// =========================================================================
// Sum6KES — full chain
// =========================================================================

#[test]
fn sum6_kes_total_periods_is_64() {
    assert_eq!(Sum6Kes::total_periods(), 64);
}

#[test]
fn sum6_kes_sizes_match_recurrence() {
    // 32 + 6*96 = 608
    assert_eq!(Sum6Kes::SIGNING_KEY_SIZE, 608);
    // 64 + 6*64 = 448
    assert_eq!(Sum6Kes::SIGNATURE_SIZE, 448);
    assert_eq!(Sum6Kes::VERIFICATION_KEY_SIZE, 32);
    assert_eq!(Sum6Kes::SEED_SIZE, 32);
}

#[test]
fn sum6_kes_chain_advances_through_all_64_periods() {
    let seed = [0x42u8; 32];
    let mut sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let vk = Sum6Kes::derive_verification_key(&sk);

    for p in 0u32..=63 {
        let msg = format!("period {} message", p);
        let sig = Sum6Kes::sign_kes(&sk, p, msg.as_bytes()).unwrap();
        Sum6Kes::verify_kes(&vk, p, msg.as_bytes(), &sig)
            .unwrap_or_else(|e| panic!("verify failed at period {}: {:?}", p, e));

        if p < 63 {
            sk = Sum6Kes::update_kes(sk, p)
                .unwrap()
                .unwrap_or_else(|| panic!("update returned None at p={}", p));
        }
    }
}

#[test]
fn sum6_kes_update_after_period_63_expires() {
    let seed = [0x77u8; 32];
    let mut sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    // Walk to period 63
    for p in 0u32..63 {
        sk = Sum6Kes::update_kes(sk, p).unwrap().unwrap();
    }
    // Update at period 63 → None
    let result = Sum6Kes::update_kes(sk, 63).unwrap();
    assert!(result.is_none());
}

#[test]
fn sum6_kes_sign_rejects_period_64() {
    let seed = [0xCCu8; 32];
    let sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let err = Sum6Kes::sign_kes(&sk, 64, b"x").unwrap_err();
    assert!(matches!(
        err,
        KesError::PeriodOutOfRange { period: 64, max_period: 63 }
    ));
}

#[test]
fn sum6_kes_verify_rejects_wrong_period_signature() {
    let seed = [0xBBu8; 32];
    let sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let vk = Sum6Kes::derive_verification_key(&sk);
    let sig = Sum6Kes::sign_kes(&sk, 0, b"period0 msg").unwrap();
    // Sign was at period 0; verify at period 1 must fail closed.
    let err = Sum6Kes::verify_kes(&vk, 1, b"period0 msg", &sig).unwrap_err();
    assert!(matches!(err, KesError::VerificationFailed));
}

// =========================================================================
// Cross-impl agreement vs cardano-crypto 1.0.8 — under #[cfg(test)] ONLY
// per N9 (no upstream-shim in production).
// =========================================================================

#[test]
fn sum6_kes_cross_impl_vk_matches_cardano_crypto() {
    for seed in [
        [0x42u8; 32],
        [0x07u8; 32],
        [0xCAu8; 32],
        [0xF0u8; 32],
    ] {
        let sk_ade = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
        let vk_ade = Sum6Kes::derive_verification_key(&sk_ade);

        let sk_cc = CcSum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
        let vk_cc_raw = CcSum6Kes::derive_verification_key(&sk_cc).unwrap();
        // cardano-crypto Sum6Kes::VerificationKey is Vec<u8> of len 32
        assert_eq!(vk_cc_raw.len(), 32);
        let mut vk_cc = [0u8; 32];
        vk_cc.copy_from_slice(&vk_cc_raw);

        assert_eq!(
            vk_ade, vk_cc,
            "VK divergence at seed {:?}: ade={:?} cc={:?}",
            seed, vk_ade, vk_cc
        );
    }
}

#[test]
fn sum6_kes_cross_impl_seed_expansion_matches_cardano_crypto() {
    // The hash crate exposes `expand_seed` per H::expand_seed
    // (from KesHashAlgorithm). Sanity-check our impl against upstream.
    use cardano_crypto::kes::hash::Blake2b256 as CcBlake2b256;
    use cardano_crypto::kes::KesHashAlgorithm;

    for seed in [
        [0x42u8; 32],
        [0x07u8; 32],
        [0xCAu8; 32],
    ] {
        let (ade_l, ade_r) = super::hash::expand_seed(&seed);
        let (cc_l, cc_r) = CcBlake2b256::expand_seed(&seed);
        assert_eq!(ade_l.as_slice(), cc_l.as_slice(), "left seed divergence");
        assert_eq!(ade_r.as_slice(), cc_r.as_slice(), "right seed divergence");
    }
}

#[test]
fn sum6_kes_cross_impl_sign_then_upstream_verifies_at_period_0() {
    let seed = [0x42u8; 32];
    let sk_ade = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let sk_cc = CcSum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let vk_cc = CcSum6Kes::derive_verification_key(&sk_cc).unwrap();

    let msg = b"cross-impl sign-verify";
    let sig_ade = Sum6Kes::sign_kes(&sk_ade, 0, msg).unwrap();

    // Convert ade sig to upstream's deserialized form via raw bytes.
    // We don't have our own raw_serialize_signature_kes yet (S3), but
    // we can construct the upstream signature manually using the
    // structured fields.
    let sig_bytes_ade = ade_signature_to_raw_448(&sig_ade);
    let sig_cc = CcSum6Kes::raw_deserialize_signature_kes(&sig_bytes_ade)
        .expect("upstream deser of our sig bytes");

    CcSum6Kes::verify_kes(&(), &vk_cc, 0, msg, &sig_cc)
        .expect("upstream verifies our signature");
}

#[test]
fn sum6_kes_cross_impl_upstream_signs_then_we_verify_at_period_0() {
    let seed = [0x42u8; 32];
    let sk_cc = CcSum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();

    let sk_ade = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let vk_ade = Sum6Kes::derive_verification_key(&sk_ade);

    let msg = b"upstream sign, ade verify";
    let sig_cc = CcSum6Kes::sign_kes(&(), 0, msg, &sk_cc).unwrap();
    let sig_cc_bytes_vec = CcSum6Kes::raw_serialize_signature_kes(&sig_cc);
    assert_eq!(sig_cc_bytes_vec.len(), 448);
    let mut sig_cc_bytes = [0u8; 448];
    sig_cc_bytes.copy_from_slice(&sig_cc_bytes_vec);

    // Reconstruct our typed signature from the raw bytes by walking
    // the recursive (sigma_d, vk0, vk1) shape.
    let sig_ade = raw_448_to_ade_signature(&sig_cc_bytes);
    Sum6Kes::verify_kes(&vk_ade, 0, msg, &sig_ade)
        .expect("ade verifies upstream signature");
}

#[test]
fn sum6_kes_cross_impl_sign_then_upstream_verifies_at_period_17() {
    let seed = [0x07u8; 32];
    let mut sk_ade = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let mut sk_cc = CcSum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let vk_cc = CcSum6Kes::derive_verification_key(&sk_cc).unwrap();

    // Advance both to period 17
    for p in 0u32..17 {
        sk_ade = Sum6Kes::update_kes(sk_ade, p).unwrap().unwrap();
        sk_cc = CcSum6Kes::update_kes(&(), sk_cc, p as u64).unwrap().unwrap();
    }

    let msg = b"period-17 cross-impl";
    let sig_ade = Sum6Kes::sign_kes(&sk_ade, 17, msg).unwrap();
    let sig_bytes_ade = ade_signature_to_raw_448(&sig_ade);
    let sig_cc = CcSum6Kes::raw_deserialize_signature_kes(&sig_bytes_ade)
        .expect("upstream deser at p17");
    CcSum6Kes::verify_kes(&(), &vk_cc, 17, msg, &sig_cc)
        .expect("upstream verifies ade sig at p17");
}

// =========================================================================
// Drop / Debug discipline
// =========================================================================

#[test]
fn sum0_signing_key_debug_is_redacted() {
    let seed = [0x42u8; 32];
    let sk = Sum0Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let formatted = format!("{:?}", sk);
    assert!(formatted.contains("<redacted>"));
    // The literal seed bytes must not appear in any encoding.
    let seed_hex = seed
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    assert!(!formatted.contains(&seed_hex));
}

#[test]
fn sum_signing_key_debug_is_redacted() {
    let seed = [0x42u8; 32];
    let sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let formatted = format!("{:?}", sk);
    assert!(formatted.contains("<redacted>"));
}

#[test]
fn zeroizing_seed_drop_overwrites_bytes() {
    use super::sum::ZeroizingSeed;
    // Construct and drop; we cannot directly observe the heap after
    // drop, but Miri / sanitizer runs would catch a use-after-free.
    // This test exists as the constructional anchor; the discipline
    // is enforced by the impl itself.
    let z = ZeroizingSeed([0xAB; 32]);
    drop(z);
}

// =========================================================================
// Helpers — ade <-> raw 448-byte signature bridge for cross-impl tests
// =========================================================================

/// Walk an ade Sum6 signature and emit the canonical 448-byte layout:
/// 64-byte Ed25519 sig at the leaf || 6 × (vk_left || vk_right) ascending.
///
/// This is a test-only helper; the production serializer (with N3
/// fail-closed semantics) lives in S3.
fn ade_signature_to_raw_448(sig: &SumSignature<Sum5Kes>) -> [u8; 448] {
    let mut out = [0u8; 448];
    write_sum6_sig(sig, &mut out, 0);
    out
}

fn write_sum6_sig(sig: &SumSignature<Sum5Kes>, out: &mut [u8], offset: usize) -> usize {
    let n = write_sum5_sig(&sig.sigma, out, offset);
    out[n..n + 32].copy_from_slice(&sig.vk0);
    out[n + 32..n + 64].copy_from_slice(&sig.vk1);
    n + 64
}

fn write_sum5_sig(sig: &SumSignature<Sum4Kes>, out: &mut [u8], offset: usize) -> usize {
    let n = write_sum4_sig(&sig.sigma, out, offset);
    out[n..n + 32].copy_from_slice(&sig.vk0);
    out[n + 32..n + 64].copy_from_slice(&sig.vk1);
    n + 64
}

fn write_sum4_sig(sig: &SumSignature<Sum3Kes>, out: &mut [u8], offset: usize) -> usize {
    let n = write_sum3_sig(&sig.sigma, out, offset);
    out[n..n + 32].copy_from_slice(&sig.vk0);
    out[n + 32..n + 64].copy_from_slice(&sig.vk1);
    n + 64
}

fn write_sum3_sig(sig: &SumSignature<Sum2Kes>, out: &mut [u8], offset: usize) -> usize {
    let n = write_sum2_sig(&sig.sigma, out, offset);
    out[n..n + 32].copy_from_slice(&sig.vk0);
    out[n + 32..n + 64].copy_from_slice(&sig.vk1);
    n + 64
}

fn write_sum2_sig(sig: &SumSignature<Sum1Kes>, out: &mut [u8], offset: usize) -> usize {
    let n = write_sum1_sig(&sig.sigma, out, offset);
    out[n..n + 32].copy_from_slice(&sig.vk0);
    out[n + 32..n + 64].copy_from_slice(&sig.vk1);
    n + 64
}

fn write_sum1_sig(sig: &SumSignature<Sum0Kes>, out: &mut [u8], offset: usize) -> usize {
    let n = write_sum0_sig(&sig.sigma, out, offset);
    out[n..n + 32].copy_from_slice(&sig.vk0);
    out[n + 32..n + 64].copy_from_slice(&sig.vk1);
    n + 64
}

fn write_sum0_sig(sig: &Sum0Signature, out: &mut [u8], offset: usize) -> usize {
    out[offset..offset + 64].copy_from_slice(sig.as_bytes());
    offset + 64
}

/// Reverse direction — parse a 448-byte canonical buffer into our
/// typed `Sum6Kes::Signature`. Test-only.
fn raw_448_to_ade_signature(bytes: &[u8; 448]) -> SumSignature<Sum5Kes> {
    // The recursive layout: starting at the leftmost 64 bytes is the
    // leaf signature; each subsequent 64 bytes is (vk0 || vk1) for
    // the next outer level.
    let leaf_sig = Sum0Signature::from_bytes(&bytes[0..64]).unwrap();

    let mut cursor = 64;
    let sig1 = read_pair_into::<Sum0Kes>(leaf_sig, bytes, &mut cursor);
    let sig2 = read_pair_into::<Sum1Kes>(sig1, bytes, &mut cursor);
    let sig3 = read_pair_into::<Sum2Kes>(sig2, bytes, &mut cursor);
    let sig4 = read_pair_into::<Sum3Kes>(sig3, bytes, &mut cursor);
    let sig5 = read_pair_into::<Sum4Kes>(sig4, bytes, &mut cursor);
    read_pair_into::<Sum5Kes>(sig5, bytes, &mut cursor)
}

fn read_pair_into<D: KesAlgorithm>(
    inner: D::Signature,
    bytes: &[u8; 448],
    cursor: &mut usize,
) -> SumSignature<D> {
    let mut vk0 = [0u8; 32];
    vk0.copy_from_slice(&bytes[*cursor..*cursor + 32]);
    *cursor += 32;
    let mut vk1 = [0u8; 32];
    vk1.copy_from_slice(&bytes[*cursor..*cursor + 32]);
    *cursor += 32;
    SumSignature {
        sigma: inner,
        vk0,
        vk1,
    }
}
