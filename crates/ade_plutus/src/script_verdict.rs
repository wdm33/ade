// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Script verdict integration (Phase 3 Cluster P-D).
//!
//! Slice: S-32 (Verdict integration + mainnet verdict agreement).
//!
//! Entry obligations to discharge before implementation:
//! - O-32.1: exact failure-class mapping — which errors are phase-1
//!   (tx rejected outright, no state change) vs. phase-2 (collateral
//!   consumed, outputs not produced)? Cite Alonzo spec.
//! - O-32.2: multi-script tx — does the tx-level budget cap
//!   accumulate across all scripts, or is each script independent
//!   against a per-script cap?
//! - O-32.3: Conway governance-action script failure — does a failed
//!   voting script fail the whole tx or only invalidate the vote?
//!   Cite Conway spec.
//!
//! Authority invariant: a transaction is accepted iff all ledger
//! rules pass AND every script evaluates successfully within budget.
//! On phase-2 failure, collateral is consumed and outputs are not
//! produced. Closes `ScriptVerdict::Passed` / `Failed` replacing
//! Phase 2's `NotYetEvaluated` placeholder.

// Intentionally empty. No code until entry obligations are discharged.
