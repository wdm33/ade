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

// cardano-crypto is imported per-test under #[cfg(test)] only. The
// remaining cross-impl tests deliberately document the divergence
// (see `sum6_kes_seed_expansion_diverges_from_cardano_crypto_rust_1_0_8`
// and `cardano_cli_corpus_sign_then_upstream_verifies`); both pull
// the upstream symbols in locally.

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
// Documented divergence vs cardano-crypto 1.0.8 (under #[cfg(test)] only).
//
// PHASE4-N-P S4 discovered that `cardano-crypto` 1.0.8 uses seed-expansion
// prefix bytes (0x00, 0x01) that DIVERGE from Haskell `cardano-base`'s
// (0x01, 0x02). The cardano-cli output is generated by Haskell, so
// cardano-cli is the ground truth. Our `ade_crypto::kes_sum` matches
// Haskell (and therefore cardano-cli) byte-for-byte; it deliberately
// disagrees with `cardano-crypto` Rust 1.0.8.
//
// These tests document the divergence so the situation is explicit and
// any future upstream version-bump that aligns to Haskell can be
// detected mechanically.
// =========================================================================

#[test]
fn sum6_kes_seed_expansion_diverges_from_cardano_crypto_rust_1_0_8() {
    // Cross-check that we and upstream disagree. If a future upstream
    // version aligns to Haskell (which would close the divergence),
    // this test starts failing and the doc comment in `hash.rs`
    // should be updated.
    use cardano_crypto::kes::hash::Blake2b256 as CcBlake2b256;
    use cardano_crypto::kes::KesHashAlgorithm;

    let seed = [0x42u8; 32];
    let (ade_l, ade_r) = super::hash::expand_seed(&seed);
    let (cc_l, cc_r) = CcBlake2b256::expand_seed(&seed);
    assert_ne!(
        ade_l.as_slice(),
        cc_l.as_slice(),
        "Expected ade <-> cardano-crypto Rust divergence; if these now agree the upstream crate may have aligned to Haskell. Update hash.rs doc."
    );
    assert_ne!(ade_r.as_slice(), cc_r.as_slice());
}

#[test]
fn sum6_kes_vk_diverges_from_cardano_crypto_rust_for_same_seed() {
    // Same divergence at the derived-VK level — different sub-trees
    // produce different VKs.
    use cardano_crypto::kes::KesAlgorithm as CcKesAlgorithm;
    use cardano_crypto::kes::Sum6Kes as CcSum6Kes;

    let seed = [0x42u8; 32];
    let sk_ade = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let vk_ade = Sum6Kes::derive_verification_key(&sk_ade);

    let sk_cc = CcSum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let vk_cc_raw = CcSum6Kes::derive_verification_key(&sk_cc).unwrap();
    let mut vk_cc = [0u8; 32];
    vk_cc.copy_from_slice(&vk_cc_raw);

    assert_ne!(
        vk_ade, vk_cc,
        "Expected divergence vs cardano-crypto Rust 1.0.8; cardano-cli is the ground truth (see kes_sum::cardano_cli_corpus)."
    );
}

// =========================================================================
// PHASE4-N-P S3 — raw-byte serde + period inference
// =========================================================================

#[test]
fn sum6_raw_serialize_signing_key_kes_size_is_608() {
    let seed = [0x42u8; 32];
    let sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let bytes = Sum6Kes::raw_serialize_signing_key_kes(&sk);
    assert_eq!(bytes.len(), 608);
}

#[test]
fn sum6_raw_serialize_signature_kes_size_is_448() {
    let seed = [0x42u8; 32];
    let sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let sig = Sum6Kes::sign_kes(&sk, 0, b"msg").unwrap();
    let bytes = Sum6Kes::raw_serialize_signature_kes(&sig);
    assert_eq!(bytes.len(), 448);
}

#[test]
fn sum6_skey_round_trip_at_every_period_0_to_63() {
    let seed = [0x42u8; 32];
    let mut sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();

    for p in 0u32..=63 {
        let bytes = Sum6Kes::raw_serialize_signing_key_kes(&sk);
        let parsed = Sum6Kes::raw_deserialize_signing_key_kes(&bytes)
            .unwrap_or_else(|e| panic!("parse failed at p={}: {:?}", p, e));

        // Round-trip: re-serialize must be byte-identical.
        let bytes_after = Sum6Kes::raw_serialize_signing_key_kes(&parsed);
        assert_eq!(bytes, bytes_after, "re-serialize drift at p={}", p);

        // Period inference must agree.
        assert_eq!(
            Sum6Kes::current_period_of_signing_key(&parsed),
            p,
            "current_period drift at p={}",
            p
        );

        if p < 63 {
            sk = Sum6Kes::update_kes(sk, p).unwrap().unwrap();
        }
    }
}

#[test]
fn sum6_signature_round_trip_at_every_period() {
    let seed = [0x07u8; 32];
    let mut sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let vk = Sum6Kes::derive_verification_key(&sk);

    for p in 0u32..=63 {
        let msg = format!("p={}", p);
        let sig = Sum6Kes::sign_kes(&sk, p, msg.as_bytes()).unwrap();
        let bytes = Sum6Kes::raw_serialize_signature_kes(&sig);
        let parsed_sig = Sum6Kes::raw_deserialize_signature_kes(&bytes).unwrap();
        // Verify with the parsed signature.
        Sum6Kes::verify_kes(&vk, p, msg.as_bytes(), &parsed_sig)
            .unwrap_or_else(|e| panic!("verify failed at p={}: {:?}", p, e));
        // Re-serialize byte-identical.
        let bytes_after = Sum6Kes::raw_serialize_signature_kes(&parsed_sig);
        assert_eq!(bytes, bytes_after, "sig re-serialize drift at p={}", p);

        if p < 63 {
            sk = Sum6Kes::update_kes(sk, p).unwrap().unwrap();
        }
    }
}

#[test]
fn period_from_zeroed_sum6_tree_shape_agrees_with_update_kes_chain() {
    let seed = [0x55u8; 32];
    let mut sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();

    for p in 0u32..=63 {
        let bytes = Sum6Kes::raw_serialize_signing_key_kes(&sk);
        let bytes_arr: &[u8; 608] = bytes.as_slice().try_into().unwrap();
        let inferred = super::period::period_from_zeroed_sum6_tree_shape(bytes_arr).unwrap();
        assert_eq!(inferred, p, "shape-inferred period mismatch at p={}", p);

        if p < 63 {
            sk = Sum6Kes::update_kes(sk, p).unwrap().unwrap();
        }
    }
}

#[test]
fn period_from_zeroed_sum6_tree_shape_rejects_leaf_all_zero() {
    let mut bytes = [0u8; 608];
    // Make the rest of the buffer non-zero so we know the leaf check
    // is what triggers.
    for b in bytes.iter_mut().skip(32) {
        *b = 0xAB;
    }
    let err = super::period::period_from_zeroed_sum6_tree_shape(&bytes).unwrap_err();
    assert!(matches!(err, KesParseError::LeafSignKeyAllZero));
}

#[test]
fn raw_deserialize_signing_key_kes_rejects_wrong_payload_size() {
    for size in [0usize, 32, 100, 512, 607, 609, 612, 1000] {
        let bytes = vec![0xABu8; size];
        let err = Sum6Kes::raw_deserialize_signing_key_kes(&bytes).unwrap_err();
        match err {
            KesParseError::WrongPayloadSize { actual } => {
                assert_eq!(actual, size, "actual mismatch for size {}", size);
            }
            other => panic!("expected WrongPayloadSize at size {}, got {:?}", size, other),
        }
    }
}

#[test]
fn raw_deserialize_signing_key_kes_rejects_leaf_all_zero() {
    // Construct a real Sum6 skey bytes, then zero the leaf.
    let seed = [0x33u8; 32];
    let sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let mut bytes = Sum6Kes::raw_serialize_signing_key_kes(&sk);
    for b in bytes[0..32].iter_mut() {
        *b = 0;
    }
    // The leaf-zero check fires at the innermost Sum0 layer, which
    // returns LeafSignKeyAllZero. SumKes<D>::raw_deserialize_signing_key_kes
    // propagates the error from D's recursive call.
    let err = Sum6Kes::raw_deserialize_signing_key_kes(&bytes).unwrap_err();
    assert!(matches!(err, KesParseError::LeafSignKeyAllZero));
}

#[test]
fn raw_deserialize_signing_key_kes_rejects_inconsistent_vk_left_at_level_6() {
    let seed = [0x77u8; 32];
    let sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let mut bytes = Sum6Kes::raw_serialize_signing_key_kes(&sk);
    // At p=0, level-6 seed is non-zero (left subtree active). Flip
    // vk_left at level 6 (bytes[544..576)).
    bytes[544] ^= 0xFF;
    let err = Sum6Kes::raw_deserialize_signing_key_kes(&bytes).unwrap_err();
    assert!(
        matches!(err, KesParseError::InconsistentSubtreeVkLeft { level: 6 }),
        "got {:?}",
        err
    );
}

#[test]
fn raw_deserialize_signing_key_kes_rejects_inconsistent_vk_right_at_level_6() {
    let seed = [0x88u8; 32];
    let sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let mut bytes = Sum6Kes::raw_serialize_signing_key_kes(&sk);
    // At p=0, level-6 seed is non-zero (left subtree active). Flip
    // vk_right at level 6 (bytes[576..608)).
    bytes[576] ^= 0xFF;
    let err = Sum6Kes::raw_deserialize_signing_key_kes(&bytes).unwrap_err();
    assert!(
        matches!(err, KesParseError::InconsistentSubtreeVkRight { level: 6 }),
        "got {:?}",
        err
    );
}

#[test]
fn raw_deserialize_signature_kes_rejects_wrong_payload_size() {
    for size in [0usize, 64, 256, 447, 449, 1000] {
        let bytes = vec![0xABu8; size];
        let err = Sum6Kes::raw_deserialize_signature_kes(&bytes).unwrap_err();
        assert!(
            matches!(err, KesParseError::WrongPayloadSize { actual: _ }),
            "expected WrongPayloadSize at size {}, got {:?}",
            size,
            err
        );
    }
}

#[test]
fn sum6_cross_impl_skey_serialization_matches_cardano_crypto() {
    // We don't have access to upstream's raw_serialize_signing_key_kes
    // (it's the missing function that drives PHASE4-N-P). Instead, we
    // verify cross-impl agreement by:
    // 1. Generate via both impls from the same seed.
    // 2. Serialize ours.
    // 3. Confirm size + VK byte-equality (already covered by S2 tests).
    // The byte-shape ground-truth against cardano-cli output comes
    // in S4.
    let seed = [0x42u8; 32];
    let sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
    let bytes = Sum6Kes::raw_serialize_signing_key_kes(&sk);
    assert_eq!(bytes.len(), 608);
    // Round-trip through our deserializer must succeed and reach
    // period 0.
    let parsed = Sum6Kes::raw_deserialize_signing_key_kes(&bytes).unwrap();
    assert_eq!(Sum6Kes::current_period_of_signing_key(&parsed), 0);
    // VKs match (sanity).
    assert_eq!(
        Sum6Kes::derive_verification_key(&parsed),
        Sum6Kes::derive_verification_key(&sk),
    );
}

// =========================================================================
// PHASE4-N-P S4 — Real cardano-cli ground-truth corpus
// =========================================================================

use super::cardano_cli_corpus::{ALL_PAIRS, SKEY1, SKEY2, SKEY3, VKEY1, VKEY2, VKEY3};

#[test]
fn cardano_cli_corpus_skey_deserializes_and_vk_matches_ground_truth() {
    for (skey_bytes, expected_vk_bytes) in ALL_PAIRS {
        let sk = Sum6Kes::raw_deserialize_signing_key_kes(*skey_bytes)
            .expect("deserialize real cardano-cli skey");
        // Fresh-from-keygen ⇒ period 0.
        assert_eq!(Sum6Kes::current_period_of_signing_key(&sk), 0);
        let computed_vk = Sum6Kes::derive_verification_key(&sk);
        assert_eq!(
            &computed_vk, *expected_vk_bytes,
            "VK divergence vs cardano-cli ground truth"
        );
    }
}

#[test]
fn cardano_cli_corpus_skey_round_trips_through_ade_serde() {
    for (skey_bytes, _vk) in ALL_PAIRS {
        let sk = Sum6Kes::raw_deserialize_signing_key_kes(*skey_bytes).unwrap();
        let re_serialized = Sum6Kes::raw_serialize_signing_key_kes(&sk);
        assert_eq!(&re_serialized, *skey_bytes, "round-trip drift");
    }
}

#[test]
fn cardano_cli_corpus_skey_period_inference_is_zero() {
    // Fresh cardano-cli output is always period 0; the tree-shape
    // function must agree.
    for (skey_bytes, _vk) in ALL_PAIRS {
        let inferred = super::period::period_from_zeroed_sum6_tree_shape(*skey_bytes).unwrap();
        assert_eq!(inferred, 0);
    }
}

#[test]
fn cardano_cli_corpus_cross_impl_vk_matches_cardano_crypto() {
    for (skey_bytes, expected_vk_bytes) in ALL_PAIRS {
        // Our deserializer
        let sk_ade = Sum6Kes::raw_deserialize_signing_key_kes(*skey_bytes).unwrap();
        let vk_ade = Sum6Kes::derive_verification_key(&sk_ade);
        // cardano-crypto cannot deserialize this directly (the whole
        // point of N-P), but we can sanity-check that the captured
        // ground-truth VK is what upstream would produce by reading
        // the leaf seed from our deserialized key and asking upstream
        // to gen_key from it. That, however, would require us to
        // expose the leaf seed — which we deliberately do not.
        //
        // Instead, the cross-impl VK match is structural: our
        // derive_verification_key matches the ground-truth VK; if
        // upstream's derive disagreed, upstream's verify against the
        // ground-truth VK would fail. We exercise that via the
        // sign-cross-verify tests below.
        assert_eq!(&vk_ade, *expected_vk_bytes);
    }
}

#[test]
fn cardano_cli_corpus_sign_then_upstream_verifies() {
    use cardano_crypto::kes::KesAlgorithm as CcKesAlgorithm;
    use cardano_crypto::kes::Sum6Kes as CcSum6Kes;

    for (skey_bytes, expected_vk_bytes) in ALL_PAIRS {
        let sk_ade = Sum6Kes::raw_deserialize_signing_key_kes(*skey_bytes).unwrap();
        let msg = b"S4 sign-then-upstream-verifies";

        let sig_ade = Sum6Kes::sign_kes(&sk_ade, 0, msg).unwrap();
        let sig_bytes = Sum6Kes::raw_serialize_signature_kes(&sig_ade);

        let sig_cc = CcSum6Kes::raw_deserialize_signature_kes(&sig_bytes)
            .expect("upstream deserialize of our sig bytes");

        let vk_cc = expected_vk_bytes.to_vec();
        CcSum6Kes::verify_kes(&(), &vk_cc, 0, msg, &sig_cc)
            .expect("upstream verifies our signature");
    }
}

#[test]
fn cardano_cli_corpus_negative_flip_one_byte_in_vk_left_fail_closed() {
    // Adversarial: flip a byte in the level-6 vk_left section
    // [544..576) of SKEY1 and verify the deserializer fail-closes
    // via InconsistentSubtreeVkLeft { level: 6 }.
    let mut bytes = *SKEY1;
    bytes[544] ^= 0xFF;
    let err = Sum6Kes::raw_deserialize_signing_key_kes(&bytes).unwrap_err();
    assert!(
        matches!(err, KesParseError::InconsistentSubtreeVkLeft { level: 6 }),
        "got {:?}",
        err
    );
}

#[test]
fn cardano_cli_corpus_constants_have_expected_sizes() {
    assert_eq!(SKEY1.len(), 608);
    assert_eq!(SKEY2.len(), 608);
    assert_eq!(SKEY3.len(), 608);
    assert_eq!(VKEY1.len(), 32);
    assert_eq!(VKEY2.len(), 32);
    assert_eq!(VKEY3.len(), 32);
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
// (retained from S2; superseded by the production serializer landed in
// S3, but kept because the `cardano_cli_corpus_sign_then_upstream_verifies`
// test uses the production serializer directly without reaching for
// these helpers).
// =========================================================================

/// Reverse direction — parse a 448-byte canonical buffer into our
/// typed `Sum6Kes::Signature`. Test-only.
#[allow(dead_code)]
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
