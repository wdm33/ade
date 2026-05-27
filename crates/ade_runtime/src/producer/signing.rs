// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED producer signing primitives (PHASE4-N-C S1).
//!
//! Wraps `cardano-crypto`'s `VrfDraft03` + `Sum6Kes` (+ Ed25519 cold key)
//! with a closed surface: private-key bytes never cross the module
//! boundary, KES evolution is one-way, and `Debug` impls redact secret
//! material. The only exports are the artifact types (`VrfProof`,
//! `VrfOutput`, `KesSignature`) plus opaque secret wrappers.
//!
//! Constraints enforced mechanically by
//! `ci/ci_check_private_key_custody.sh`:
//! - No `pub struct .*SigningKey` outside `producer/`.
//! - No `cardano_crypto::vrf::VrfDraft03::prove` /
//!   `cardano_crypto::kes::KesAlgorithm::sign_kes` /
//!   `update_kes` call outside `producer/`.
//! - No `pub fn` here returning raw `[u8; N]` / `Vec<u8>` — every
//!   signing output is wrapped in a closed BLUE type.
//! - Custom (non-derived) `Debug` for every secret-bearing struct.

use ade_crypto::error::CryptoError;
use ade_crypto::kes::{KesPeriod, KesSignature, SUM6_KES_SIG_LEN};
use ade_crypto::vrf::{VrfOutput, VrfProof};

use cardano_crypto::kes::{KesAlgorithm, Sum6Kes};
use cardano_crypto::vrf::VrfDraft03;

// =========================================================================
// VRF signing — RED-confined private-key custody
// =========================================================================

/// Libsodium-format VRF secret key (64 bytes: seed || public key).
///
/// Construction goes through `from_bytes_zeroizing`; the raw bytes are
/// never exposed via a public accessor, and the `Debug` impl emits only
/// a length-tagged redaction. `Drop` best-effort overwrites the bytes
/// before deallocation.
pub struct VrfSigningKey([u8; 64]);

impl VrfSigningKey {
    /// Construct from a byte slice (libsodium expanded form, 64 bytes).
    ///
    /// The input slice is copied into the wrapper; callers SHOULD also
    /// zeroize the source on their side. The returned key zeroizes on
    /// drop.
    pub fn from_bytes_zeroizing(b: &[u8]) -> Result<Self, SigningError> {
        if b.len() != 64 {
            return Err(SigningError::MalformedKey {
                algorithm: "vrf_praos_draft03",
                detail: "expected 64-byte libsodium secret key",
            });
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(b);
        Ok(Self(arr))
    }
}

impl Drop for VrfSigningKey {
    fn drop(&mut self) {
        zeroize_bytes(&mut self.0);
    }
}

impl core::fmt::Debug for VrfSigningKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("VrfSigningKey(<redacted>)")
    }
}

/// Produce a Praos-VRF proof + output for the given alpha under `sk`.
///
/// Wraps `cardano_crypto::vrf::VrfDraft03::prove` and `proof_to_hash` and
/// produces closed BLUE types (`VrfProof`, `VrfOutput`). The raw secret
/// never leaves the wrapper.
pub fn vrf_prove(sk: &VrfSigningKey, alpha: &[u8]) -> Result<(VrfProof, VrfOutput), SigningError> {
    let proof_bytes = VrfDraft03::prove(&sk.0, alpha).map_err(SigningError::CardanoCrypto)?;
    let output_bytes =
        VrfDraft03::proof_to_hash(&proof_bytes).map_err(SigningError::CardanoCrypto)?;
    Ok((VrfProof(proof_bytes), VrfOutput(output_bytes)))
}

// =========================================================================
// KES signing — RED-confined private-key custody + evolution discipline
// =========================================================================

/// In-memory Sum6KES signing key, period, and evolutions-remaining
/// counter. Period and remaining count are part of the wrapper, not the
/// inner cardano-crypto signing key, so the wrapper boundary is the
/// single source of evolution truth.
pub struct KesSecret {
    inner: <Sum6Kes as KesAlgorithm>::SigningKey,
    current_period: KesPeriod,
    evolutions_remaining: u32,
}

impl KesSecret {
    /// Construct from raw seed bytes (32 bytes). The seed is run through
    /// `Sum6Kes::gen_key_kes_from_seed_bytes` to expand into the full
    /// signing-key tree; the seed bytes are best-effort zeroized after
    /// expansion.
    ///
    /// The constructed key starts at period 0 with the full
    /// `SUM6_MAX_PERIOD` evolutions remaining (63 future updates from
    /// period 0 to period 63 inclusive).
    pub fn from_bytes_zeroizing(b: &[u8]) -> Result<Self, SigningError> {
        if b.len() != 32 {
            return Err(SigningError::MalformedKey {
                algorithm: "kes_sum6",
                detail: "expected 32-byte seed",
            });
        }
        let mut seed = [0u8; 32];
        seed.copy_from_slice(b);
        let inner =
            Sum6Kes::gen_key_kes_from_seed_bytes(&seed).map_err(SigningError::CardanoCrypto)?;
        zeroize_bytes(&mut seed);
        Ok(Self {
            inner,
            current_period: KesPeriod(0),
            // Sum6KES has 64 periods (0..=63). Starting at period 0,
            // 63 forward updates remain. The wrapper keeps this counter
            // for fail-fast `kes_sign` / `kes_update` range checks.
            evolutions_remaining: ade_crypto::kes::SUM6_MAX_PERIOD,
        })
    }

    /// Construct a `KesSecret` from a 32-byte seed advanced to
    /// `period_idx`. Calls `Sum6Kes::gen_key_kes_from_seed_bytes(seed)`
    /// to materialize the period-0 tree, then steps forward via
    /// `kes_update` exactly `period_idx` times.
    ///
    /// Returns `SigningError::EvolutionExhausted` if `period_idx`
    /// exceeds `SUM6_MAX_PERIOD` (63). The Sum6KES tree has 64 periods
    /// (0..=63); period_idx > 63 has no representable state.
    pub fn from_seed_at_period(
        seed: &[u8; 32],
        period_idx: u32,
    ) -> Result<Self, SigningError> {
        let sk = Self::from_bytes_zeroizing(seed)?;
        if period_idx == 0 {
            return Ok(sk);
        }
        kes_update(sk, KesPeriod(period_idx))
    }

    /// Verification-key fingerprint: 64-char lowercase hex of the
    /// Sum6KES root verification key (Blake2b-256 hash of the
    /// left/right subtree VKs). Non-secret; safe to print on the
    /// `key-gen-KES` success line.
    pub fn verification_key_fingerprint(&self) -> String {
        let vk_bytes = Sum6Kes::raw_serialize_verification_key_kes(
            &Sum6Kes::derive_verification_key(&self.inner)
                .expect("derive_verification_key is total for a constructed KesSecret"),
        );
        let mut out = String::with_capacity(vk_bytes.len() * 2);
        for b in &vk_bytes {
            out.push_str(&format!("{:02x}", b));
        }
        out
    }

    pub fn current_period(&self) -> KesPeriod {
        self.current_period
    }

    pub fn evolutions_remaining(&self) -> u32 {
        self.evolutions_remaining
    }
}

// Note: KesSecret does NOT implement `Drop` explicitly. The inner
// `Sum6Kes::SigningKey` is itself a `cardano-crypto` type that the
// crate hand-rolls to zeroize its allocated buffers on drop (the crate
// uses the `zeroize` family internally). Adding our own `Drop` here
// would block destructuring the wrapper in `kes_update` (the field
// move is what enables the borrow-checker-friendly update); the
// type-level discipline (no public byte accessors, redacted Debug,
// RED-only) carries the load.

impl core::fmt::Debug for KesSecret {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Period and counter are non-secret metadata; the inner tree is
        // redacted.
        f.debug_struct("KesSecret")
            .field("inner", &"<redacted>")
            .field("current_period", &self.current_period)
            .field("evolutions_remaining", &self.evolutions_remaining)
            .finish()
    }
}

/// Sign `msg` at `period` under `sk`. Period bounds are enforced at the
/// wrapper boundary so RED cannot accidentally sign for a past or
/// out-of-range period.
pub fn kes_sign(
    sk: &KesSecret,
    period: KesPeriod,
    msg: &[u8],
) -> Result<KesSignature, SigningError> {
    if period.0 < sk.current_period.0 {
        return Err(SigningError::PeriodBackwards {
            requested: period,
            current: sk.current_period,
        });
    }
    // `period > current + evolutions_remaining` is the signing-exhaustion
    // boundary: we have no way to bring the key forward to that period.
    let max_reachable = sk.current_period.0.saturating_add(sk.evolutions_remaining);
    if period.0 > max_reachable {
        return Err(SigningError::PeriodExhausted {
            requested: period,
            max: KesPeriod(max_reachable),
        });
    }

    let sig_raw = Sum6Kes::sign_kes(&(), period.0 as u64, msg, &sk.inner)
        .map_err(SigningError::CardanoCrypto)?;
    let sig_bytes = Sum6Kes::raw_serialize_signature_kes(&sig_raw);
    debug_assert_eq!(sig_bytes.len(), SUM6_KES_SIG_LEN);
    let mut arr = [0u8; SUM6_KES_SIG_LEN];
    arr.copy_from_slice(&sig_bytes);
    Ok(KesSignature(arr))
}

/// Evolve `sk` forward to `to`. One-way: `to < current` is rejected at
/// the wrapper boundary, and exhausting the underlying tree is
/// translated into `EvolutionExhausted` rather than `None`.
pub fn kes_update(sk: KesSecret, to: KesPeriod) -> Result<KesSecret, SigningError> {
    if to.0 < sk.current_period.0 {
        return Err(SigningError::EvolutionBackwards {
            from: sk.current_period,
            to,
        });
    }
    let max_reachable = sk.current_period.0.saturating_add(sk.evolutions_remaining);
    if to.0 > max_reachable {
        return Err(SigningError::EvolutionExhausted {
            from: sk.current_period,
            to,
            evolutions_remaining: sk.evolutions_remaining,
        });
    }

    let KesSecret {
        mut inner,
        mut current_period,
        mut evolutions_remaining,
    } = sk;

    while current_period.0 < to.0 {
        let next = Sum6Kes::update_kes(&(), inner, current_period.0 as u64)
            .map_err(SigningError::CardanoCrypto)?;
        match next {
            Some(updated) => {
                inner = updated;
                current_period = KesPeriod(current_period.0 + 1);
                // Sum6KES has 64 periods; once we step into period N,
                // (63 - N) updates remain.
                evolutions_remaining = evolutions_remaining.saturating_sub(1);
            }
            None => {
                return Err(SigningError::EvolutionExhausted {
                    from: current_period,
                    to,
                    evolutions_remaining,
                });
            }
        }
    }
    Ok(KesSecret {
        inner,
        current_period,
        evolutions_remaining,
    })
}

// =========================================================================
// Cold (Ed25519) signing key — opcert signing pathway (S2 consumer)
// =========================================================================

/// Ed25519 cold signing key, 32-byte seed. The expanded compound (seed
/// || vk) is held in dalek's `SigningKey`; we keep only the seed and
/// re-derive on demand so the wrapper boundary is simpler.
pub struct ColdSigningKey {
    seed: [u8; 32],
}

impl ColdSigningKey {
    /// Construct from a 32-byte seed.
    pub fn from_bytes_zeroizing(b: &[u8]) -> Result<Self, SigningError> {
        if b.len() != 32 {
            return Err(SigningError::MalformedKey {
                algorithm: "ed25519_cold",
                detail: "expected 32-byte seed",
            });
        }
        let mut seed = [0u8; 32];
        seed.copy_from_slice(b);
        Ok(Self { seed })
    }

    /// Derive the matching Ed25519 verification key (public, 32 bytes).
    /// This is a non-secret derivation; exposing it is the canonical
    /// path for callers that need to write a companion `.vkey`.
    pub fn derive_verification_key_bytes(&self) -> [u8; 32] {
        use ed25519_dalek::SigningKey;
        let sk = SigningKey::from_bytes(&self.seed);
        sk.verifying_key().to_bytes()
    }
}

impl Drop for ColdSigningKey {
    fn drop(&mut self) {
        zeroize_bytes(&mut self.seed);
    }
}

impl core::fmt::Debug for ColdSigningKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("ColdSigningKey(<redacted>)")
    }
}

// =========================================================================
// SigningError — BLUE-safe (no private-key bytes in payload).
// =========================================================================

/// Closed RED-signing error surface. Every variant carries only
/// non-secret metadata (period numbers, counters, algorithm names) —
/// never key bytes or seed material.
#[derive(Debug)]
pub enum SigningError {
    /// kes_sign called with a period earlier than the wrapper's
    /// current period (forward-secrecy violation attempt).
    PeriodBackwards {
        requested: KesPeriod,
        current: KesPeriod,
    },
    /// kes_sign called for a period beyond the tree's reach
    /// (`current + evolutions_remaining`).
    PeriodExhausted {
        requested: KesPeriod,
        max: KesPeriod,
    },
    /// kes_update target is earlier than the current period.
    EvolutionBackwards { from: KesPeriod, to: KesPeriod },
    /// kes_update target exceeds the tree's remaining evolutions.
    EvolutionExhausted {
        from: KesPeriod,
        to: KesPeriod,
        evolutions_remaining: u32,
    },
    /// Constructor rejected the input byte slice as structurally
    /// invalid. Algorithm and detail strings are static literals; no
    /// runtime key material appears in the payload.
    MalformedKey {
        algorithm: &'static str,
        detail: &'static str,
    },
    /// Wrapper-internal error originating from `cardano-crypto`. The
    /// inner `Error` type is the crate's own enum, which does not
    /// embed key bytes.
    CardanoCrypto(cardano_crypto::common::CryptoError),
}

// =========================================================================
// Bridge between SigningError-style malformed-key and CryptoError.
// =========================================================================

impl From<CryptoError> for SigningError {
    fn from(err: CryptoError) -> Self {
        match err {
            CryptoError::MalformedKey { algorithm, detail } => {
                SigningError::MalformedKey { algorithm, detail }
            }
            CryptoError::MalformedSignature { algorithm, detail } => {
                SigningError::MalformedKey { algorithm, detail }
            }
            _ => SigningError::MalformedKey {
                algorithm: "ade_crypto",
                detail: "structural error from ade_crypto",
            },
        }
    }
}

// =========================================================================
// Best-effort zeroize helper. Without `zeroize` as a direct dep and with
// `deny(unsafe_code)` at crate root, we use a black-boxed byte fill: the
// compiler cannot prove the bytes are dead after the loop, so the writes
// survive optimization in practice. A real production deployment should
// upgrade this to the `zeroize` crate; the type-level discipline (custom
// Debug, no public byte accessors, RED-only) is the load-bearing guard.
// =========================================================================

fn zeroize_bytes(bytes: &mut [u8]) {
    for byte in bytes.iter_mut() {
        *byte = 0;
    }
    core::hint::black_box(bytes);
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_crypto::kes::{verify_kes_signature, KesVerificationKey};
    use ade_crypto::vrf::{verify_vrf, VrfVerificationKey};
    use ade_testkit::producer::reference_vectors::{
        kes_reference_set, kes_update_reference_chain, vrf_reference_set,
    };

    fn make_vrf_key(seed: [u8; 32]) -> (VrfSigningKey, VrfVerificationKey) {
        let (sk_bytes, vk_bytes) = VrfDraft03::keypair_from_seed(&seed);
        let sk = VrfSigningKey::from_bytes_zeroizing(&sk_bytes).unwrap();
        (sk, VrfVerificationKey(vk_bytes))
    }

    fn make_kes_secret(seed: [u8; 32]) -> (KesSecret, KesVerificationKey) {
        let sk = KesSecret::from_bytes_zeroizing(&seed).unwrap();
        let raw = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();
        let vk_bytes = Sum6Kes::raw_serialize_verification_key_kes(
            &Sum6Kes::derive_verification_key(&raw).unwrap(),
        );
        let mut vk_arr = [0u8; 32];
        vk_arr.copy_from_slice(&vk_bytes);
        (sk, KesVerificationKey(vk_arr))
    }

    #[test]
    fn vrf_prove_matches_reference_vectors() {
        for v in vrf_reference_set() {
            let (sk, _vk) = make_vrf_key(v.seed);
            let (proof, output) = vrf_prove(&sk, &v.alpha).unwrap();
            assert_eq!(
                proof.0, v.expected_proof,
                "VRF proof mismatch for seed {:?}",
                v.seed
            );
            assert_eq!(
                output.0, v.expected_output,
                "VRF output mismatch for seed {:?}",
                v.seed
            );
        }
    }

    #[test]
    fn kes_sign_matches_reference_vectors() {
        for v in kes_reference_set() {
            let (sk, _vk) = make_kes_secret(v.seed);
            let sk = if v.period > 0 {
                kes_update(sk, KesPeriod(v.period)).unwrap()
            } else {
                sk
            };
            let sig = kes_sign(&sk, KesPeriod(v.period), &v.message).unwrap();
            assert_eq!(
                sig.0, v.expected_signature,
                "KES signature mismatch for seed {:?} period {}",
                v.seed, v.period
            );
        }
    }

    #[test]
    fn kes_update_chain_matches_reference() {
        for (seed, period_after_n_updates, expected_fingerprint) in kes_update_reference_chain() {
            let (sk, _vk) = make_kes_secret(seed);
            let evolved = kes_update(sk, KesPeriod(period_after_n_updates)).unwrap();
            assert_eq!(evolved.current_period().0, period_after_n_updates);

            // Fingerprint: a deterministic signature over a fixed message
            // at the evolved period collapses the whole tree state into
            // 32 bytes via blake2b-256. Any drift in the evolved tree
            // produces a different fingerprint.
            let probe = b"kes-update-chain-probe";
            let sig = kes_sign(&evolved, KesPeriod(period_after_n_updates), probe).unwrap();
            let fp = ade_crypto::blake2b_256(&sig.0).0;
            assert_eq!(
                fp, expected_fingerprint,
                "KES fingerprint mismatch for seed {:?} period {}",
                seed, period_after_n_updates
            );
        }
    }

    #[test]
    fn vrf_prove_then_verify_round_trip() {
        for v in vrf_reference_set() {
            let (sk, vk) = make_vrf_key(v.seed);
            let (proof, _output) = vrf_prove(&sk, &v.alpha).unwrap();
            let extracted = verify_vrf(&vk, &proof, &v.alpha).unwrap();
            assert_eq!(extracted.0, v.expected_output);
        }
    }

    #[test]
    fn kes_sign_then_verify_round_trip() {
        for v in kes_reference_set() {
            let (sk, vk) = make_kes_secret(v.seed);
            let sk = if v.period > 0 {
                kes_update(sk, KesPeriod(v.period)).unwrap()
            } else {
                sk
            };
            let sig = kes_sign(&sk, KesPeriod(v.period), &v.message).unwrap();
            verify_kes_signature(&vk, KesPeriod(v.period), &v.message, &sig).unwrap();
        }
    }

    #[test]
    fn kes_sign_rejects_period_past_evolutions_remaining() {
        let (sk, _vk) = make_kes_secret([0x33; 32]);
        // sk is at period 0 with 63 evolutions remaining => max reachable = 63.
        let too_far = KesPeriod(64);
        let err = kes_sign(&sk, too_far, b"x").unwrap_err();
        match err {
            SigningError::PeriodExhausted { requested, max } => {
                assert_eq!(requested.0, 64);
                assert_eq!(max.0, 63);
            }
            other => panic!("expected PeriodExhausted, got {:?}", other),
        }
    }

    #[test]
    fn kes_update_rejects_backwards_evolution() {
        let (sk, _vk) = make_kes_secret([0x44; 32]);
        let sk = kes_update(sk, KesPeriod(2)).unwrap();
        let err = kes_update(sk, KesPeriod(1)).unwrap_err();
        match err {
            SigningError::EvolutionBackwards { from, to } => {
                assert_eq!(from.0, 2);
                assert_eq!(to.0, 1);
            }
            other => panic!("expected EvolutionBackwards, got {:?}", other),
        }
    }

    #[test]
    fn kes_secret_debug_is_redacted() {
        let seed = [0xCAu8; 32];
        let (sk, _vk) = make_kes_secret(seed);
        let formatted = format!("{:?}", sk);
        assert!(formatted.contains("<redacted>"));
        // None of the seed bytes leak into the formatted output. We
        // check the 32-byte seed in hex (lowercase + uppercase) and as
        // the literal decimal byte sequence.
        let seed_hex = seed
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        assert!(!formatted.contains(&seed_hex));
        // 0xCA repeated is also a recognizable pattern: ensure the
        // raw byte value (202) does not appear as a comma-separated
        // sequence of 32 occurrences.
        assert!(!formatted.contains("202, 202, 202"));
    }

    #[test]
    fn vrf_signing_key_debug_is_redacted() {
        let (sk, _vk) = make_vrf_key([0xBBu8; 32]);
        let formatted = format!("{:?}", sk);
        assert!(formatted.contains("<redacted>"));
        // 0xBB repeated as decimal must not appear contiguously.
        assert!(!formatted.contains("187, 187, 187"));
    }

    #[test]
    fn signing_error_contains_no_key_bytes() {
        // For each variant, construct an error from inputs that include
        // a known seed pattern, then assert the seed bytes do not
        // appear in the formatted error output.
        let seed = [0x9Cu8; 32];
        let seed_hex: String = seed.iter().map(|b| format!("{:02x}", b)).collect();
        let seed_dec_run = "156, 156, 156, 156";

        // PeriodBackwards
        let (sk, _vk) = make_kes_secret(seed);
        let sk = kes_update(sk, KesPeriod(3)).unwrap();
        let err = kes_sign(&sk, KesPeriod(1), &seed).unwrap_err();
        let s = format!("{:?}", err);
        assert!(matches!(err, SigningError::PeriodBackwards { .. }));
        assert!(!s.contains(&seed_hex));
        assert!(!s.contains(seed_dec_run));

        // PeriodExhausted
        let (sk2, _vk2) = make_kes_secret(seed);
        let err = kes_sign(&sk2, KesPeriod(100), &seed).unwrap_err();
        let s = format!("{:?}", err);
        assert!(matches!(err, SigningError::PeriodExhausted { .. }));
        assert!(!s.contains(&seed_hex));
        assert!(!s.contains(seed_dec_run));

        // EvolutionBackwards
        let (sk3, _vk3) = make_kes_secret(seed);
        let sk3 = kes_update(sk3, KesPeriod(2)).unwrap();
        let err = kes_update(sk3, KesPeriod(1)).unwrap_err();
        let s = format!("{:?}", err);
        assert!(matches!(err, SigningError::EvolutionBackwards { .. }));
        assert!(!s.contains(&seed_hex));
        assert!(!s.contains(seed_dec_run));

        // EvolutionExhausted
        let (sk4, _vk4) = make_kes_secret(seed);
        let err = kes_update(sk4, KesPeriod(200)).unwrap_err();
        let s = format!("{:?}", err);
        assert!(matches!(err, SigningError::EvolutionExhausted { .. }));
        assert!(!s.contains(&seed_hex));
        assert!(!s.contains(seed_dec_run));

        // MalformedKey
        let err = KesSecret::from_bytes_zeroizing(&seed[..16]).unwrap_err();
        let s = format!("{:?}", err);
        assert!(matches!(err, SigningError::MalformedKey { .. }));
        // Sub-seed prefix shouldn't appear either.
        assert!(!s.contains(&seed_hex[..32]));

        // CardanoCrypto variant: provoke a zero-seed key gen failure
        // is impractical (gen accepts any 32 bytes). Skip explicit
        // construction — the variant's payload type is the crate's
        // own enum, which does not embed key bytes by design.
    }
}
