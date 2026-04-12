// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Cost model parsing (Phase 3 Cluster P-B).
//!
//! Slice: S-30 (Cost models + budget + conformance).
//!
//! Scope decision #5: parse-only. Cost-model coefficients are
//! protocol-parameter inputs; we do not fit or calibrate them.
//!
//! Entry obligations to discharge before implementation:
//! - O-30.1: cost-model CBOR format varies across eras (V1 flat
//!   integer array; V2 keyed map; V3 versioned map). Decode from a
//!   10.6.2 snapshot and compare to cardano-cli `query
//!   protocol-parameters` output.
//! - O-30.2: aiken budget accounting must produce byte-identical
//!   budget consumption on the full IOG conformance suite
//!   (`.uplc.budget.expected` match).
//! - O-30.3: tx-level budget cap — is it enforced per-script or
//!   aggregated across scripts? Cite Alonzo+ spec.
//!
//! Authority invariant: per-PV cost models parse deterministically
//! from pparams; budget accounting matches oracle byte-for-byte on
//! the conformance suite.

// Intentionally empty. No code until entry obligations are discharged.
