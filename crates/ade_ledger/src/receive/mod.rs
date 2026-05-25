// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE receive-side bridge authority (PHASE4-N-H).
//!
//! The receive bridge admits peer-supplied header+body bytes into
//! ChainDb + LedgerState + PraosChainDepState **only** via
//! `block_validity` (B1) producing an [`AdmittedBlock`] token whose
//! private constructor lives in [`admitted`].
//!
//! Distinct from `ade_ledger::producer::AcceptedBlock` (broadcast-
//! eligibility token for locally forged blocks) — the two tokens are
//! deliberately separate so cross-use is mechanically impossible
//! (see `docs/planning/receive-side-bridge-invariants.md` §1 ¬P-6).
//!
//! Scope: Path A — admit-only. `RollBackward` returns
//! [`ReceiveError::RollbackOutOfScope`]; full rollback authority is a
//! follow-on cluster.

pub mod admitted;
pub mod chain_write;
pub mod events;
pub mod pending_header_cache;
pub mod reducer;

pub use admitted::{admit_via_block_validity, AdmittedBlock, AdmittedOutcome};
pub use chain_write::{ChainDbWrite, ChainWriteError};
pub use events::{
    NoOpReason, ReceiveEffect, ReceiveError, ReceiveEvent, TargetPoint, TipPoint,
};
pub use pending_header_cache::PendingHeaderCache;
pub use reducer::{receive_apply, receive_apply_sequence, ReceiveState};
