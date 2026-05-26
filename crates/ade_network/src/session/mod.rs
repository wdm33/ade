// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN session driver (PHASE4-N-L).
//!
//! Composes mux frame encode/decode + handshake transition + per-
//! mini-protocol fanout into a pure reducer that turns inbound byte
//! chunks into `OrchestratorEvent`s and outbound encoded frames.
//! The async tokio layer lives in `ade_runtime::network::mux_pump`
//! (S6) and `ade_runtime::network::n2n_dialer` (S7).
//!
//! Despite the upstream `.idd-config.json` classifying
//! `ade_network::session` as RED (placeholder before this cluster),
//! every file under this module is GREEN by content — pure,
//! deterministic, no tokio / SystemTime / Instant / rand. The RED
//! layer for this cluster lives in `ade_runtime::network`.

pub mod core;
pub mod demux;
pub mod event;
pub mod handshake_driver;
pub mod state;

pub use core::step;
pub use demux::FrameBuffer;
pub use event::{
    AcceptedMiniProtocol, ByteChunkIn, HandshakeRole, SessionEffect, SessionError,
};
pub use handshake_driver::{
    run_n2n_handshake_initiator, run_n2n_handshake_responder, NegotiatedN2n, Transport,
    TransportError,
};
pub use state::{ConnectedState, HandshakeProgress, SessionState};
