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

pub mod admission;
pub mod admission_log;
pub mod ba02_evidence;
pub mod cli;
pub mod forge_intent;
pub mod key_gen;
pub mod live_log;
pub mod node;
pub mod node_lifecycle;
pub mod node_sync;
pub mod produce_mode;
pub mod run_loop_planner;
pub mod wire_only;

pub use cli::{Cli, CliError, KeyGenKesCli, Mode, ProduceCli};
pub use key_gen::{run_key_gen_kes, EXIT_KEY_GEN_FAILURE};
pub use live_log::{
    LiveLogEvent, LiveLogWriter, ModeTag, PeerDialFailureKind, WireOnlyShutdownReason,
};
pub use node::{
    run_node_until_shutdown, NodeRunError, NodeShutdownEvidence, NodeStartupInputs,
    EXIT_AUTHORITY_FATAL_DECODE, EXIT_AUTHORITY_FATAL_IO, EXIT_GENERIC_STARTUP,
};
pub use node_lifecycle::{run_node_lifecycle, EXIT_NODE_LIFECYCLE_UNWIRED};
pub use produce_mode::{run_produce_mode, EXIT_PRODUCE_FAILURE};
pub use wire_only::{run_wire_only, PeerOutcome, EXIT_LIVE_PASS_PEER_FAILURE};
