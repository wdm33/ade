// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Transaction-validity closure primitives — Cluster PHASE4-B2.
//!
//! B2-S1 ships the phase-1 vkey-witness + required-signer closure:
//!
//! - [`required_signers`] enumerates, over a CLOSED era-versioned
//!   [`required_signers::SignerSource`] surface, every `Hash28` key
//!   hash a Conway transaction must have a vkey witness for.
//! - [`witness::verify_required_witnesses`] checks that every required
//!   key hash is covered by a witness whose Ed25519 signature over the
//!   PRESERVED tx body hash verifies fail-closed.
//!
//! The enumeration and per-cert-kind / per-voter rules are grounded in
//! the Conway ledger spec (`getConwayWitsVKeyNeeded` and
//! `getVKeyWitnessConwayTxCert`); see `required_signers.rs` for the
//! per-source citations.

pub mod encoding;
pub mod phase1;
pub mod required_signers;
pub mod transition;
pub mod verdict;
pub mod witness;

pub use encoding::{
    decode_tx_verdict_surface, encode_tx_verdict_surface, TxSurfaceDecodeError, TxVerdictSurface,
};
pub use phase1::{decode_tx, tx_phase_one, DecodedTx};
pub use required_signers::{
    required_signers, tx_derived_required_signers, RequiredSignerError, RequiredSigners,
    ResolvedInputs, ResolvedOutput, SignerSource,
};
pub use transition::{tx_validity, TxValidityOutcome};
pub use verdict::{TxRejectClass, TxValidityError, TxValidityVerdict};
pub use witness::{verify_required_witnesses, VKeyWitnessRef, WitnessClosureError, WitnessField};
