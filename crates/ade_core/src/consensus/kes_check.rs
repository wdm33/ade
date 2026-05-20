// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Fail-closed wiring of `ade_crypto::kes` into Praos header validation.
//!
//! Two checks, both fail-closed:
//!
//! 1. **KES signature** — the hot KES key signs the header **body** CBOR
//!    bytes. The KES evolution count is `currentKESPeriod - opcertKESPeriod`,
//!    where `currentKESPeriod = slot / slots_per_kes_period` (mainnet
//!    `slots_per_kes_period = 129_600`). Verified against the 14 Conway-576
//!    corpus blocks.
//! 2. **Operational certificate** — the issuer cold key signs the
//!    `OCertSignable` representation (`kesVkey ‖ counter_be8 ‖ period_be8`).
//!
//! Every fixed-size field flows through `expect_size`: a wrong length is a
//! `FieldError`, never a skipped check.

use ade_crypto::ed25519::{Ed25519Signature, Ed25519VerificationKey};
use ade_crypto::kes::{verify_kes, verify_opcert, KesPeriod, KesVerificationKey, OperationalCertData};
use ade_types::SlotNo;

use crate::consensus::errors::{FieldError, FieldKind, HeaderValidationError};
use crate::consensus::header_summary::HeaderKes;

/// Mainnet slots-per-KES-period (genesis constant since Shelley).
pub const SLOTS_PER_KES_PERIOD: u64 = 129_600;

/// Fail-closed fixed-size field guard.
///
/// Returns `Ok(())` only when `actual == expected`; otherwise a typed
/// `FieldError`. Never silently skips — the forbidden
/// `if len == K { check } else { skip }` shape cannot arise from this helper.
pub fn expect_size(kind: FieldKind, actual: usize, expected: usize) -> Result<(), FieldError> {
    if actual == expected {
        Ok(())
    } else {
        Err(FieldError {
            field: kind,
            expected,
            actual,
        })
    }
}

/// Verify the KES signature and the operational certificate for a Praos
/// header. Fail-closed throughout; the first failure is the only failure.
///
/// `op_cert_counter` / `op_cert_kes_period` come from the `HeaderInput` (the
/// op-cert counter monotonicity is gated separately by the caller).
pub fn verify_header_kes(
    kes: &HeaderKes,
    slot: SlotNo,
    op_cert_counter: u64,
    op_cert_kes_period: u64,
) -> Result<(), HeaderValidationError> {
    // Fixed-size field guards.
    expect_size(FieldKind::IssuerVkey, kes.issuer_vkey.len(), 32)
        .map_err(HeaderValidationError::MalformedField)?;
    expect_size(FieldKind::KesVkey, kes.kes_vkey.len(), 32)
        .map_err(HeaderValidationError::MalformedField)?;
    expect_size(FieldKind::KesSignature, kes.kes_signature.len(), 448)
        .map_err(HeaderValidationError::MalformedField)?;
    expect_size(FieldKind::OpCertSignature, kes.op_cert_signature.len(), 64)
        .map_err(HeaderValidationError::MalformedField)?;

    let mut kes_vk_arr = [0u8; 32];
    kes_vk_arr.copy_from_slice(&kes.kes_vkey);
    let kes_vk = KesVerificationKey(kes_vk_arr);

    let mut issuer_arr = [0u8; 32];
    issuer_arr.copy_from_slice(&kes.issuer_vkey);
    let cold_vk = Ed25519VerificationKey(issuer_arr);

    let mut oc_sig_arr = [0u8; 64];
    oc_sig_arr.copy_from_slice(&kes.op_cert_signature);
    let cold_signature = Ed25519Signature(oc_sig_arr);

    // Operational certificate: cold key signs (hot_vkey ‖ counter ‖ period).
    let opcert = OperationalCertData {
        hot_vkey: kes_vk.clone(),
        sequence_number: op_cert_counter,
        kes_period: op_cert_kes_period,
        cold_signature,
    };
    match verify_opcert(&cold_vk, &opcert) {
        Ok(true) => {}
        Ok(false) => return Err(HeaderValidationError::OpCertInvalid),
        // A malformed cold key is structurally a bad op-cert.
        Err(_) => return Err(HeaderValidationError::OpCertInvalid),
    }

    // KES signature over the header body. The evolution count is the number
    // of KES periods elapsed since the op-cert's start period.
    let current_kes_period = slot.0 / SLOTS_PER_KES_PERIOD;
    let evolution = current_kes_period.saturating_sub(op_cert_kes_period);
    let period = match KesPeriod::new(evolution as u32) {
        Ok(p) => p,
        // Period beyond the Sum6KES horizon: cannot have signed this header.
        Err(_) => return Err(HeaderValidationError::KesInvalid),
    };
    match verify_kes(&kes_vk, period, &kes.kes_signature, &kes.header_body_bytes) {
        Ok(true) => Ok(()),
        Ok(false) => Err(HeaderValidationError::KesInvalid),
        Err(_) => Err(HeaderValidationError::KesInvalid),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn expect_size_rejects_wrong_length() {
        assert_eq!(expect_size(FieldKind::VrfProof, 79, 80), Err(FieldError {
            field: FieldKind::VrfProof,
            expected: 80,
            actual: 79,
        }));
        assert_eq!(expect_size(FieldKind::VrfProof, 80, 80), Ok(()));
    }
}
