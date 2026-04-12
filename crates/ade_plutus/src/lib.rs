// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Cluster plan: docs/active/phase_3_cluster_plan.md
//
// This crate is the quarantine boundary between the Ade-canonical
// ledger and the ported UPLC evaluator (aiken-lang/aiken/crates/uplc
// at a commit pinned in slice S-29 per proof obligation O-29.1).
// Ade-canonical types live at the public surface; pallas-originated
// and aiken-originated types stay internal. No other BLUE crate may
// import an evaluator entry point directly.

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::float_arithmetic)]

pub mod cost_model;
pub mod evaluator;
pub mod script_context;
pub mod script_verdict;

pub use cost_model::{CostModels, DecoderMode};
pub use evaluator::{EvalOutput, PlutusError, PlutusLanguage, PlutusScript};
