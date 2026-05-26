// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED admission-mode wire layer (PHASE4-N-M-C, sub-cluster C).
//!
//! Hosts the per-peer wire pump that owns a post-handshake
//! `MuxTransportHandle`, drives the chain-sync + block-fetch
//! state machines, and emits a CLOSED stream of
//! `AdmissionPeerEvent` values into the admission runner's
//! `peer_events` channel.
//!
//! TCB color: RED. Owns no new authority. The pump moves bytes
//! and lifts them into typed events; the GREEN reducer
//! (`admission::verdict::derive`) and the BLUE authority
//! (`admit_via_block_validity`) are downstream consumers.

pub mod wire_pump;

pub use wire_pump::{
    dial_for_admission, run_admission_wire_pump, AdmissionDialError, AdmissionPeerEvent,
    AdmissionWirePumpError, AdmissionWirePumpResult,
};
