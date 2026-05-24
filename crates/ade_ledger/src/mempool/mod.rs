// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Mempool admission — Cluster PHASE4-B2, slice B2-S5.
//!
//! Two layers, strictly separated by TCB color:
//!
//! - [`admit`] (BLUE, Tier-1) — the admission gate. A tx is admitted iff
//!   [`crate::tx_validity::tx_validity`] is `Valid` against the mempool's
//!   accumulating state. No false accept; on Invalid the mempool is unchanged.
//! - [`policy`] (GREEN, Tier-5) — deterministic eviction/ordering over the
//!   already-admitted tx ids. It never calls `tx_validity` and cannot change an
//!   admit verdict.
//!
//! Tier-5 is provably below Tier-1: [`policy::order`] reads only the admitted-id
//! list, so no choice of policy can alter which txs `admit` accepts.

pub mod admit;
pub mod canonicalize;
pub mod ingress;
pub mod policy;

pub use admit::{admit, AdmitOutcome, MempoolState};
pub use canonicalize::{canonicalize_peer_streams, PeerId, PeerSubmissionQueue};
pub use ingress::{mempool_ingress, IngressEvent, IngressSource};
pub use policy::{order, OrderPolicy};
