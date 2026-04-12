// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! UPLC evaluator surface (Phase 3 Cluster P-B).
//!
//! Slice: S-29 (`ade_plutus` scaffold + UPLC port).
//!
//! Entry obligations to discharge before implementation:
//! - O-29.1: pin aiken commit whose conformance-test output matches
//!   plutus version used by cardano-node 10.6.2; verify zero
//!   divergence on IOG conformance suite at that commit.
//! - O-29.2: review pallas-* transitive dependencies via `cargo tree`;
//!   confirm no conflicts with existing Ade deps.
//! - O-29.3: probe aiken's Flat decoder against mainnet Plutus txs
//!   including BLS12-381 element parsing edge cases.
//!
//! Authority invariant: given `(uplc_term, args, cost_model,
//! builtin_set)`, the evaluation result and budget consumption are
//! identical to the IOG reference implementation at the plutus
//! version pinned to cardano-node 10.6.2.

// Intentionally empty. No code until entry obligations are discharged.
