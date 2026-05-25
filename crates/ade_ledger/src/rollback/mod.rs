// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE rollback authority (PHASE4-N-I).
//!
//! Closes PHASE4-N-H's deferred DC-CONS-20 rollback-side
//! `open_obligation` via the snapshot + replay-forward strategy:
//! `materialize_rolled_back_state` composes a `SnapshotReader`
//! lookup + `BlockSource` iteration + `apply_block_with_verdicts`
//! to recompute `(LedgerState, PraosChainDepState)` at a target
//! point. `commit_rollback` then performs atomic state replacement
//! across ChainDb + ledger + chain_dep + pending headers.
//!
//! Scope: in-memory variant. `DC-CONS-21` (persistent
//! encode/decode round-trip) stays declared with its
//! `open_obligation` naming the follow-on persistent-encoder
//! cluster.

pub mod error;
pub mod materialize;
pub mod traits;

pub use error::{CommitRollbackError, MaterializeError};
pub use materialize::{materialize_rolled_back_state, TargetPoint};
pub use traits::{BlockSource, SnapshotReader};
