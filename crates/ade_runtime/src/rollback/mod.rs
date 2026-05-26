// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN rollback adapter layer (PHASE4-N-I S4).
//!
//! Wires the BLUE rollback driver to runtime infrastructure:
//!   - [`cadence`] — pure cadence policy
//!     (`should_snapshot_after_block`).
//!   - [`in_memory_cache`] — `InMemorySnapshotCache` implementing
//!     `SnapshotReader`.
//!   - [`chaindb_block_source`] — `ChainDbBlockSource` adapting any
//!     `ChainDb` impl into a `BlockSource`.

pub mod cadence;
pub mod chaindb_block_source;
pub mod in_memory_cache;
pub mod persistent_cache;
pub mod persistent_writer;
pub mod snapshot_writer;

pub use cadence::{should_snapshot_after_block, SnapshotCadence};
pub use chaindb_block_source::ChainDbBlockSource;
pub use in_memory_cache::InMemorySnapshotCache;
pub use persistent_cache::{
    PersistentCacheError, PersistentSnapshotCache, PERSISTENT_CACHE_SCHEMA_VERSION,
};
pub use persistent_writer::PersistentSnapshotWriter;
pub use snapshot_writer::maybe_capture_snapshot;
