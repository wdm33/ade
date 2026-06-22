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
pub mod ba02_pass;
pub mod bootstrap_export;
pub mod candidate_aggregator;
pub mod cli;
pub mod convergence_evidence;
pub mod epoch_activate;
pub mod epoch_activation;
pub mod epoch_candidate;
pub mod epoch_rebind;
pub mod epoch_source_window;
pub mod epoch_wire;
pub mod fair_merge;
pub mod forge_intent;
pub mod fork_switch;
pub mod key_gen;
pub mod lca_walk;
pub mod live_log;
pub mod mem_measure;
pub mod node;
pub mod node_lifecycle;
pub mod node_sync;
pub mod operator_forge;
pub mod post_switch_continuity;
pub mod produce_mode;
pub mod rehearsal_evidence;
pub mod rehearsal_pass;
pub mod run_loop_planner;
pub mod selector_state;
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
