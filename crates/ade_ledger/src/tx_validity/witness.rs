// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Fail-closed required-signer coverage (DC-VAL-06, CN-LEDGER-09).
//!
//! [`verify_required_witnesses`] checks that EVERY required `Hash28`
//! (from [`super::required_signers`]) is covered by a vkey witness whose
//! Ed25519 signature over the PRESERVED tx body hash verifies. Coverage
//! is by key hash = `Blake2b-224(vkey)`. A witness whose key/sig is the
//! wrong size is a fail-closed [`WitnessClosureError::MalformedWitnessField`]
//! (via `from_bytes`), NOT a skip. An extra irrelevant witness can never
//! substitute for a missing required one — coverage is checked per
//! required key, and only a witness matching that key's hash AND verifying
//! its signature counts.

use std::collections::BTreeSet;

use ade_crypto::{blake2b_224, verify_ed25519, Ed25519Signature, Ed25519VerificationKey};
use ade_types::{Hash28, Hash32};

use super::required_signers::{RequiredSigners, SignerSource};

/// A decoded vkey witness: verification key + signature, exactly as
/// carried in witness-set key 0 (`[vkey, signature]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VKeyWitnessRef {
    pub vkey: Vec<u8>,
    pub signature: Vec<u8>,
}

/// Closed failure taxonomy for witness-coverage verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessClosureError {
    /// A required key hash has no covering, signature-verifying witness.
    /// `source` names which required-signer obligation went uncovered.
    MissingRequiredSigner {
        key_hash: Hash28,
        source: SignerSource,
    },
    /// A witness covered a required key by hash, but its Ed25519 signature
    /// over the preserved body hash did NOT verify (forged / wrong body).
    InvalidWitnessSignature { key_hash: Hash28 },
    /// A witness field (vkey or sig) had the wrong size / encoding.
    /// Fail-closed via `from_bytes`.
    MalformedWitnessField {
        which: WitnessField,
        key_hash: Hash28,
    },
}

/// Which witness field was malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessField {
    VerificationKey,
    Signature,
}

/// Verify that every required signer is covered by a valid witness over
/// the preserved body hash.
///
/// Algorithm (fail-closed):
/// 1. Validate every supplied witness's field sizes via `from_bytes`; a
///    wrong-size key/sig is a fail-closed error (never skipped).
/// 2. For each required key hash, find a witness whose
///    `Blake2b-224(vkey) == key_hash` AND whose signature over `body_hash`
///    verifies. If none, the requirement is unmet:
///      - if a witness matched the hash but failed verification →
///        `InvalidWitnessSignature`,
///      - otherwise → `MissingRequiredSigner` (an extra irrelevant
///        witness cannot substitute).
///
/// Determinism: `required.keys` is a `BTreeSet`, so the first-failing
/// requirement is reported in stable Hash28 order.
pub fn verify_required_witnesses(
    body_hash: &Hash32,
    required: &RequiredSigners,
    witnesses: &[VKeyWitnessRef],
) -> Result<(), WitnessClosureError> {
    // Step 1: parse + size-check every witness up front. Fail-closed.
    struct ParsedWitness {
        key_hash: Hash28,
        vk: Ed25519VerificationKey,
        sig: Ed25519Signature,
    }
    let mut parsed: Vec<ParsedWitness> = Vec::with_capacity(witnesses.len());
    for w in witnesses {
        let key_hash = blake2b_224(&w.vkey);
        let vk = Ed25519VerificationKey::from_bytes(&w.vkey).map_err(|_| {
            WitnessClosureError::MalformedWitnessField {
                which: WitnessField::VerificationKey,
                key_hash: key_hash.clone(),
            }
        })?;
        let sig = Ed25519Signature::from_bytes(&w.signature).map_err(|_| {
            WitnessClosureError::MalformedWitnessField {
                which: WitnessField::Signature,
                key_hash: key_hash.clone(),
            }
        })?;
        parsed.push(ParsedWitness { key_hash, vk, sig });
    }

    // Step 2: coverage. Iterate required keys in deterministic order.
    for key_hash in &required.keys {
        let mut matched_hash = false;
        let mut verified = false;
        for pw in &parsed {
            if &pw.key_hash != key_hash {
                continue;
            }
            matched_hash = true;
            // verify_ed25519 is fail-closed: Err on malformed point; we
            // already validated size, but a non-canonical point can still
            // fail here — treat any non-Ok(true) as not-verified.
            if matches!(verify_ed25519(&pw.vk, &body_hash.0, &pw.sig), Ok(true)) {
                verified = true;
                break;
            }
        }
        if verified {
            continue;
        }
        let source = first_source(required, key_hash);
        if matched_hash {
            // A witness claimed this key but its signature did not verify.
            return Err(WitnessClosureError::InvalidWitnessSignature {
                key_hash: key_hash.clone(),
            });
        }
        return Err(WitnessClosureError::MissingRequiredSigner {
            key_hash: key_hash.clone(),
            source,
        });
    }

    Ok(())
}

/// The reported source for a missing key. A key may be required by more
/// than one source; we report the first in the closed enum's ordering
/// (deterministic) so the rejection is reproducible.
fn first_source(required: &RequiredSigners, key_hash: &Hash28) -> SignerSource {
    let sources: BTreeSet<SignerSource> = required.sources_for(key_hash);
    // `required.keys` only contains keys that were `require`d, so at least
    // one source always exists; default is unreachable but kept total.
    sources
        .into_iter()
        .next()
        .unwrap_or(SignerSource::ExplicitRequiredSigner)
}
