// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::float_arithmetic)]

pub mod blake2b;
pub mod ed25519;
pub mod error;
pub mod kes;
pub mod traits;
pub mod vrf;

pub use blake2b::{
    blake2b_224, blake2b_256, block_header_hash, credential_hash, script_hash, transaction_id,
};
pub use ed25519::{
    verify_byron_bootstrap, verify_ed25519, ByronExtendedVerificationKey, Ed25519Signature,
    Ed25519VerificationKey,
};
pub use error::CryptoError;
pub use kes::{verify_kes, verify_opcert, KesPeriod, KesVerificationKey, OperationalCertData};
pub use traits::HashAlgorithm;
pub use vrf::{verify_vrf, VrfOutput, VrfProof, VrfVerificationKey};
