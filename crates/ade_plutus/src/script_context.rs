// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! ScriptContext derivation (Phase 3 Cluster P-C).
//!
//! Slice: S-31 (ScriptContext derivation for V1/V2/V3).
//!
//! Entry obligations to discharge before implementation:
//! - O-31.1: document exact structural differences between
//!   ScriptContext V1, V2, and V3 (reference Plutus.V{1,2,3}.Ledger.Api).
//! - O-31.2: reference-input representation in V2 ScriptContext vs
//!   V3 — are they indistinguishable from regular inputs or a
//!   separate field?
//! - O-31.3: Conway ScriptInfo variants — `Voting`, `Proposing`,
//!   `Certifying` — enumerate the per-variant context contents.
//! - O-31.4: datum resolution — when an input refers to a datum by
//!   hash, does ScriptContext include the datum body or only its
//!   hash? Per-version answer required.
//!
//! Authority invariant: given `(tx, resolved_utxo, script_purpose,
//! protocol_version)`, the constructed ScriptContext serializes
//! identically to oracle at every PV where that purpose is
//! available, for every mainnet Plutus transaction in the corpus.

// Intentionally empty. No code until entry obligations are discharged.
