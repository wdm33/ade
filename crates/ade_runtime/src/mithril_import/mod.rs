// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED mithril-client import shell (PHASE4-N-Y S1).
//!
//! Consumes a **mithril-client-verified** snapshot manifest and maps
//! its verified output into the closed `SeedProvenance::Mithril`
//! provenance + observed anchor field-set. Performs no semantic
//! decision (the BLUE `verify_mithril_binding` predicate decides) and
//! never re-verifies the STM multisig — that is the mithril-client's
//! job. See [[feedback-mithril-is-peer-infra-not-ade-authority]].

pub mod importer;
pub mod json;

pub use importer::{
    import_mithril_manifest, import_mithril_manifest_from_bytes, MithrilManifestError,
    MithrilProvenanceImport,
};
