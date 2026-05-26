// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN admission-mode JSONL event vocabulary + writer
//! (PHASE4-N-M-B S2).
//!
//! Physically isolated from `crate::live_log` (wire-only mode).
//! CI grep enforces both directions (DC-ADMIT-04 in registry).

pub mod event;
pub mod writer;

pub use event::{
    AdmissionHaltReason, AdmissionLogEvent, AdmissionShutdownReason,
};
pub use writer::AdmissionLogWriter;
