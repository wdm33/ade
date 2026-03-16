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

pub mod byron;
pub mod cbor;
pub mod error;
pub mod preserved;
pub mod primitives;
pub mod shelley;
pub mod traits;

pub use error::CodecError;
pub use preserved::{PreservedCbor, RawCbor};
pub use traits::{AdeDecode, AdeEncode, CodecContext};
