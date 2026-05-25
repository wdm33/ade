// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Producer-side opcert acceptance authority (BLUE).
//!
//! `opcert_validate` is the single sanctioned RED -> BLUE entry point for
//! accepting an `OperationalCert` value at the `ProducerTick -> forge_block`
//! boundary. It rejects every forbidden state with a closed structured
//! error:
//!
//! - cold-signature failure under the operator's cold verification key
//! - KES-period mismatch against the expected period at the forged slot
//! - counter repetition or regression against the per-(cold-key, node)
//!   prev_counter, when one is supplied
//! - shape failures (wrong `hot_vkey` / `sigma` length, malformed cold key)
//!
//! Counter discipline (DC-CONS-12): the operator's first opcert may carry
//! any starting counter (`prev_counter == None`); every subsequent opcert
//! MUST be strictly greater than the previous accepted counter. Repetition
//! is its own variant because it is a distinct operational fault from
//! regression (a node may legitimately regress its visible counter on
//! restart from a stale snapshot; repetition is unambiguous nonce reuse).

use ade_crypto::ed25519::{Ed25519Signature, Ed25519VerificationKey};
use ade_crypto::kes::{verify_opcert, KesVerificationKey, OperationalCertData};
use ade_types::shelley::block::OperationalCert;

const HOT_VKEY_LEN: usize = 32;
const SIGMA_LEN: usize = 64;

/// Closed error sum for opcert acceptance.
///
/// Every forbidden state has a dedicated variant. No `#[non_exhaustive]`
/// — closure is the point: producers downstream of this surface match
/// exhaustively, and the closure is enforced by
/// `ci/ci_check_opcert_closed.sh` guard 3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpCertError {
    /// Cold-key signature over the canonical signable did not verify
    /// under `cold_vk`.
    BadColdSignature,
    /// `opcert.kes_period != expected_period` at the forged slot.
    PeriodMismatch { found: u64, expected: u64 },
    /// `opcert.sequence_number == prev_counter` — verbatim reuse.
    CounterRepeat { counter: u64 },
    /// `opcert.sequence_number < prev_counter` — regression below the
    /// last accepted counter.
    CounterRegression { found: u64, prev: u64 },
    /// `hot_vkey.len() != 32`.
    BadHotVkeyLength { found: usize },
    /// `sigma.len() != 64`.
    BadSigmaLength { found: usize },
    /// `hot_vkey` bytes did not produce a valid Ed25519 point under the
    /// underlying verifier. Defense-in-depth: the codec's shape check
    /// validates length, this catches non-point bytes.
    MalformedColdVk,
}

/// Validate a producer-supplied opcert at the RED -> BLUE boundary.
///
/// Returns `Ok(())` only when:
///   1. `opcert.hot_vkey.len() == 32` and `opcert.sigma.len() == 64`
///      (shape checks first — cheap, deterministic);
///   2. the cold key `cold_vk` signed the canonical signable
///      `hot_vkey || sequence_number_be8 || kes_period_be8`;
///   3. `opcert.kes_period == expected_period`;
///   4. either `prev_counter == None`, or
///      `opcert.sequence_number > prev_counter.unwrap()`.
///
/// `prev_counter == None` is the operator's first opcert: any starting
/// counter is accepted. Subsequent opcerts MUST be strictly greater.
pub fn opcert_validate(
    opcert: &OperationalCert,
    cold_vk: &Ed25519VerificationKey,
    expected_period: u64,
    prev_counter: Option<u64>,
) -> Result<(), OpCertError> {
    if opcert.hot_vkey.len() != HOT_VKEY_LEN {
        return Err(OpCertError::BadHotVkeyLength {
            found: opcert.hot_vkey.len(),
        });
    }
    if opcert.sigma.len() != SIGMA_LEN {
        return Err(OpCertError::BadSigmaLength {
            found: opcert.sigma.len(),
        });
    }

    let mut hot_arr = [0u8; HOT_VKEY_LEN];
    hot_arr.copy_from_slice(&opcert.hot_vkey);
    let mut sig_arr = [0u8; SIGMA_LEN];
    sig_arr.copy_from_slice(&opcert.sigma);

    let opcert_data = OperationalCertData {
        hot_vkey: KesVerificationKey(hot_arr),
        sequence_number: opcert.sequence_number,
        kes_period: opcert.kes_period,
        cold_signature: Ed25519Signature(sig_arr),
    };

    match verify_opcert(cold_vk, &opcert_data) {
        Ok(true) => {}
        Ok(false) => return Err(OpCertError::BadColdSignature),
        Err(_) => return Err(OpCertError::MalformedColdVk),
    }

    if opcert.kes_period != expected_period {
        return Err(OpCertError::PeriodMismatch {
            found: opcert.kes_period,
            expected: expected_period,
        });
    }

    if let Some(prev) = prev_counter {
        if opcert.sequence_number == prev {
            return Err(OpCertError::CounterRepeat { counter: prev });
        }
        if opcert.sequence_number < prev {
            return Err(OpCertError::CounterRegression {
                found: opcert.sequence_number,
                prev,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey as DalekSk};

    /// Build a canonical opcert signed by the cold key derived from
    /// `cold_seed`. The cold-key signature is over the raw byte
    /// concatenation `hot_vkey || seq# BE || kes_period BE` — the same
    /// signable representation `ade_crypto::kes::verify_opcert` consumes.
    fn synth_canonical_opcert(
        cold_seed: [u8; 32],
        hot_vkey_bytes: [u8; 32],
        sequence_number: u64,
        kes_period: u64,
    ) -> (OperationalCert, Ed25519VerificationKey) {
        let cold = DalekSk::from_bytes(&cold_seed);
        let cold_vk_bytes = *cold.verifying_key().as_bytes();

        let mut signable = Vec::with_capacity(48);
        signable.extend_from_slice(&hot_vkey_bytes);
        signable.extend_from_slice(&sequence_number.to_be_bytes());
        signable.extend_from_slice(&kes_period.to_be_bytes());

        let sigma = cold.sign(&signable);

        let opcert = OperationalCert {
            hot_vkey: hot_vkey_bytes.to_vec(),
            sequence_number,
            kes_period,
            sigma: sigma.to_bytes().to_vec(),
        };
        let cold_vk = Ed25519VerificationKey::from_bytes(&cold_vk_bytes).unwrap();
        (opcert, cold_vk)
    }

    fn fixture_inputs() -> (OperationalCert, Ed25519VerificationKey) {
        // Cold seed [0x42; 32], hot vkey [0x43; 32], seq 7, period 42.
        synth_canonical_opcert([0x42; 32], [0x43; 32], 7, 42)
    }

    #[test]
    fn opcert_validate_accepts_canonical_fixture() {
        let (opcert, cold_vk) = fixture_inputs();
        assert_eq!(opcert_validate(&opcert, &cold_vk, 42, None), Ok(()));
    }

    #[test]
    fn opcert_validate_rejects_counter_regression() {
        let (opcert, cold_vk) = fixture_inputs();
        let err = opcert_validate(&opcert, &cold_vk, 42, Some(8));
        assert_eq!(
            err,
            Err(OpCertError::CounterRegression { found: 7, prev: 8 })
        );
    }

    #[test]
    fn opcert_validate_rejects_counter_repeat() {
        let (opcert, cold_vk) = fixture_inputs();
        let err = opcert_validate(&opcert, &cold_vk, 42, Some(7));
        assert_eq!(err, Err(OpCertError::CounterRepeat { counter: 7 }));
    }

    #[test]
    fn opcert_validate_rejects_period_mismatch() {
        let (opcert, cold_vk) = fixture_inputs();
        let err = opcert_validate(&opcert, &cold_vk, 43, None);
        assert_eq!(
            err,
            Err(OpCertError::PeriodMismatch {
                found: 42,
                expected: 43,
            })
        );
    }

    #[test]
    fn opcert_validate_rejects_bad_signature_over_cold_key() {
        let (mut opcert, cold_vk) = fixture_inputs();
        // Flip a single byte of sigma.
        opcert.sigma[0] ^= 0x01;
        let err = opcert_validate(&opcert, &cold_vk, 42, None);
        assert_eq!(err, Err(OpCertError::BadColdSignature));
    }

    #[test]
    fn opcert_validate_rejects_short_hot_vkey() {
        let (mut opcert, cold_vk) = fixture_inputs();
        opcert.hot_vkey.truncate(31);
        let err = opcert_validate(&opcert, &cold_vk, 42, None);
        assert_eq!(err, Err(OpCertError::BadHotVkeyLength { found: 31 }));
    }

    #[test]
    fn opcert_validate_first_opcert_no_prev_counter() {
        // Operator's initial opcert: any starting counter is permitted.
        let (opcert, cold_vk) = synth_canonical_opcert([0x42; 32], [0x43; 32], 9999, 42);
        assert_eq!(opcert_validate(&opcert, &cold_vk, 42, None), Ok(()));
    }
}
