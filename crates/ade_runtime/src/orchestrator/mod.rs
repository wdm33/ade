// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN orchestrator core + RED tokio runner pieces (PHASE4-N-K).
//!
//! `core` is the pure reducer (S2). `event` + `state` host the
//! closed event/effect/state vocabulary. The RED runner sub-modules
//! (`peer_session`, `leadership_session`, `n2n_server_pump`) land
//! in slices S4–S6 and are tokio-driven; the core itself is
//! tokio-free.

pub mod core;
pub mod event;
pub mod leadership_session;
pub mod n2n_server_pump;
pub mod peer_session;
pub mod state;

pub use core::step;
pub use event::{
    AuthorityFatalKind, OrchestratorEffect, OrchestratorError, OrchestratorEvent, PeerHaltReason,
    PeerId, PeerRole,
};
pub use state::{OrchestratorState, PerPeerReceiveVersions};
