// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN closed JSONL log vocabulary for the wire-only live
//! pass (PHASE4-N-L-LIVE S1).

pub mod event;
pub mod writer;

pub use event::{
    LiveLogEvent, ModeTag, PeerDialFailureKind, WireOnlyShutdownReason,
};
pub use writer::LiveLogWriter;
