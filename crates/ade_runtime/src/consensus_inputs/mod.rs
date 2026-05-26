// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN operator-extracted `LiveConsensusInputs` importer
//! (PHASE4-N-M-C, sub-cluster C).
//!
//! Path (b) per sketch §0: an operator runs `cardano-cli` against
//! the live preprod node and bundles the answers into one JSON
//! envelope; this module decodes, validates, and canonicalizes
//! that envelope into the closed `LiveConsensusInputsCanonical`
//! that BLUE admission consumes through `LiveLedgerView`.
//!
//! Doctrine load:
//! - `[[feedback-oracle-seed-then-ade-owns]]` — the operator
//!   bundle is bootstrap-time evidence at a named point P; after
//!   import the canonical form is the authority.
//! - `[[feedback-shell-must-not-overstate-semantic-truth]]` — the
//!   importer fails fast on every missing / malformed field; no
//!   `Option` field gets a runtime default.
//!
//! Slice progression:
//!   - **C1a (this slice)** — `json` + `importer` ship the
//!     closed JSON shape, the closed error sum, and the
//!     `LiveConsensusInputsRaw` typed-validated form.
//!     CN-CONS-IN-01 / DC-CONS-IN-01.
//!   - C1b — `canonical` adds the canonical CBOR encoding +
//!     Blake2b-256 fingerprint, producing
//!     `LiveConsensusInputsCanonical`. DC-CONS-IN-02.
//!   - C2  — `view` adds `LiveLedgerView`. DC-VIEW-01.

pub mod canonical;
pub mod importer;
pub mod json;
pub mod view;

pub use canonical::{
    canonical_from_raw, import_live_consensus_inputs, import_live_consensus_inputs_from_bytes,
    LiveConsensusInputsCanonical,
};
pub use importer::{
    import_live_consensus_inputs_raw, import_live_consensus_inputs_raw_from_bytes,
    LiveConsensusInputsImportError, LiveConsensusInputsRaw, PoolEntry,
};
pub use json::{parse_consensus_inputs_json, RawConsensusInputs};
pub use view::LiveLedgerView;
