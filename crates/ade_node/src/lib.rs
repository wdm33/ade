// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED `ade_node` binary library (PHASE4-N-K S7).
//!
//! Exposes the binary's `run` entry point as a function so
//! integration tests can drive shutdown-resume identity
//! (DC-NODE-04) in-process. The `bin/ade_node` is a thin wrapper.

#![deny(unsafe_code)]

pub mod cli;
pub mod node;

pub use cli::{Cli, CliError};
pub use node::{
    NodeRunError, NodeShutdownEvidence, NodeStartupInputs, run_node_until_shutdown,
    EXIT_AUTHORITY_FATAL_DECODE, EXIT_AUTHORITY_FATAL_IO, EXIT_GENERIC_STARTUP,
};
