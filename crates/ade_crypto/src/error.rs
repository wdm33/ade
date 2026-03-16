// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

// Foundational error taxonomy for all cryptographic verification in ade_crypto.
//
// Three-class verdict discipline:
//   Malformed  — structurally invalid, cannot attempt verification
//   Invalid    — well-formed inputs, cryptographic check failed
//   Valid      — well-formed inputs, cryptographic check passed
//
// Every CryptoError variant maps to the "malformed" class.
// The "invalid" class is represented by Ok(false) for standard verification
// or Err(VerificationFailed) for extractive verification (VRF).

/// Structured error type for all cryptographic verification failures.
///
/// All variants use `&'static str` fields to ensure errors are deterministic,
/// canonical, and free of runtime-allocated context that could vary across
/// invocations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// Input byte slice has wrong length for the expected type.
    InvalidInputLength {
        expected: usize,
        actual: usize,
        context: &'static str,
    },

    /// Cryptographic verification failed (well-formed inputs, bad proof/signature).
    /// Used by extractive verification (VRF) where Ok(false) is not appropriate.
    VerificationFailed { algorithm: &'static str },

    /// Public key bytes are structurally invalid (wrong length, invalid point encoding).
    MalformedKey {
        algorithm: &'static str,
        detail: &'static str,
    },

    /// Signature bytes are structurally invalid (wrong length, invalid encoding).
    MalformedSignature {
        algorithm: &'static str,
        detail: &'static str,
    },

    /// VRF proof bytes are structurally invalid.
    MalformedProof { detail: &'static str },

    /// VRF output extraction failed after successful verification.
    VrfOutputExtractionFailed { detail: &'static str },

    /// KES period is beyond the maximum allowed for the tree depth.
    KesExpiredPeriod { current: u32, max: u32 },

    /// KES Merkle path is structurally invalid.
    KesMalformedPath { depth: u8, detail: &'static str },

    /// Algorithm is not supported in this build.
    UnsupportedAlgorithm { algorithm: &'static str },
}

impl core::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CryptoError::InvalidInputLength {
                expected,
                actual,
                context,
            } => write!(
                f,
                "invalid input length for {context}: expected {expected}, got {actual}"
            ),
            CryptoError::VerificationFailed { algorithm } => {
                write!(f, "{algorithm} verification failed")
            }
            CryptoError::MalformedKey { algorithm, detail } => {
                write!(f, "malformed {algorithm} key: {detail}")
            }
            CryptoError::MalformedSignature { algorithm, detail } => {
                write!(f, "malformed {algorithm} signature: {detail}")
            }
            CryptoError::MalformedProof { detail } => {
                write!(f, "malformed proof: {detail}")
            }
            CryptoError::VrfOutputExtractionFailed { detail } => {
                write!(f, "VRF output extraction failed: {detail}")
            }
            CryptoError::KesExpiredPeriod { current, max } => {
                write!(f, "KES period {current} exceeds maximum {max}")
            }
            CryptoError::KesMalformedPath { depth, detail } => {
                write!(f, "malformed KES path at depth {depth}: {detail}")
            }
            CryptoError::UnsupportedAlgorithm { algorithm } => {
                write!(f, "unsupported algorithm: {algorithm}")
            }
        }
    }
}

impl std::error::Error for CryptoError {}
