// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// TCB color partition (cluster PHASE4-N-A):
//   BLUE: codec, handshake, chain_sync, block_fetch, tx_submission,
//         keep_alive, peer_sharing, n2c, mux::frame
//   RED:  mux::transport, session
//   GREEN: lib (this file), mux::mod
// DC-CORE-01: BLUE submodules are sync-only — no async, no tokio,
// no futures. Enforced by ci/ci_check_no_async_in_blue.sh.

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::float_arithmetic)]

pub mod block_fetch;
pub mod chain_sync;
pub mod codec;
pub mod handshake;
pub mod keep_alive;
pub mod mux;
pub mod n2c;
pub mod peer_sharing;
pub mod session;
pub mod tx_submission;
