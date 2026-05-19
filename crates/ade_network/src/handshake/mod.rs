// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Handshake mini-protocol state machine (BLUE) — S-A3.
//
// Pure version-negotiation state machine for the Ouroboros N2N and
// N2C handshakes. Consumes/produces the codec types defined in S-A2:
// the state machine never re-encodes messages — it produces
// `HandshakeMessage` / `N2cHandshakeMessage` values and the caller
// hands them to the S-A2 codec for byte emission.
//
// Per-protocol agency type per locked §7 #7. No ambient state
// (DC-PROTO-06): the supported version table is an explicit input.

pub mod agency;
pub mod selection;
pub mod state;
pub mod transition;
pub mod version_table;

pub use agency::HandshakeAgency;
pub use selection::{select_n2c_version, select_n2n_version};
pub use state::{
    HandshakeError, HandshakeState, N2cHandshakeOutput, N2cVersionData, N2nHandshakeOutput,
    PeerSharingFlag, VersionData,
};
pub use transition::{n2c_transition, n2n_transition};
pub use version_table::{N2C_SUPPORTED, N2N_SUPPORTED};
