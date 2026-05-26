// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

#![deny(unsafe_code)]

pub mod admission;
pub mod bootstrap;
pub mod bootstrap_anchor;
pub mod chaindb;
pub mod clock;
pub mod consensus;
pub mod consensus_inputs;
pub mod network;
pub mod orchestrator;
pub mod producer;
pub mod receive;
pub mod recovery;
pub mod rollback;
pub mod seed_import;
pub mod wal;
