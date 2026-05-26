// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! PHASE4-N-M-B — Admission orchestrator (GREEN + RED).
//!
//! Mode dispatch entry for `ade_node --mode admission`. This module
//! composes the N-M-A storage stack (BootstrapAnchor + WAL +
//! seed importer) with the N-L wire stack (N2nDialer + chain-sync
//! pump) into an admission loop that:
//!   - admits peer-supplied blocks via BLUE
//!     `admit_via_block_validity` (CN-CONS-08, unchanged),
//!   - appends a WAL entry per successful admit (DC-WAL-01),
//!   - derives a closed [`verdict::AgreementVerdict`] per admit
//!     comparing our authoritative output against the peer tip
//!     (GREEN evidence, not authority — see
//!     `[[feedback-evidence-reducers-are-green-not-authority]]`),
//!   - emits a closed `AdmissionLogEvent` JSONL transcript
//!     (B2).
//!
//! This module owns NO new BLUE authority. All authority remains
//! in `ade_ledger` / `ade_core`.

pub mod bootstrap;
pub mod runner;
pub mod seed_to_snapshot;
pub mod verdict;

pub use bootstrap::{dispatch_admission, AdmissionBootstrapError};
pub use runner::{
    run_admission, AdmissionExitCode, AdmissionInputs, AdmissionPeerEvent,
    EXIT_LIVE_AGREEMENT_DIVERGED, EXIT_LIVE_INPUT_NOT_FOUND, EXIT_LIVE_WAL_APPEND_IO,
};
pub use seed_to_snapshot::{seed_to_snapshot, SeedToSnapshotError};
pub use verdict::{
    derive as derive_verdict, verdict_kind, AgreementVerdict, BlockAdmitOutcome,
    InvalidAdmitReason,
};
