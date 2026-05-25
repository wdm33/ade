// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE producer authority for PHASE4-N-C.
//!
//! Hosts the canonical [`state::ProducerTick`] input value and the pure
//! [`forge::forge_block`] transition. Relocated from
//! `ade_core::consensus::*` because `ade_ledger` already depends on
//! `ade_core` and the forge body needs `LedgerState` +
//! `mempool::admit` — the inverse import would close a Cargo cycle.
//! Classification is unchanged: BLUE, deterministic, fail-closed.

pub mod forge;
pub mod self_accept;
pub mod served_chain;
pub mod state;

pub use self_accept::{self_accept, AcceptedBlock, SelfAcceptError};
pub use served_chain::{served_chain_admit, ServedChainAdmitError, ServedChainSnapshot};
