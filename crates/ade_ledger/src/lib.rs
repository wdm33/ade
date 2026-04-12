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

pub mod alonzo;
pub mod babbage;
pub mod byron;
pub mod conway;
pub mod delegation;
pub mod epoch;
pub mod error;
pub mod fingerprint;
pub mod governance;
pub mod hfc;
pub mod late_era_validation;
pub mod mary;
pub mod phase;
pub mod pparams;
pub mod rational;
pub mod rules;
pub mod scripts;
pub mod shelley;
pub mod state;
pub mod utxo;
pub mod value;
pub mod witness;
