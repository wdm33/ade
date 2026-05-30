// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Chain database abstraction (Phase 4 cluster N-D, slice S-33).
//!
//! This module defines the storage abstraction the rest of the runtime
//! and consensus layers use to persist blocks and locate them by hash
//! or slot. The trait surface is Tier 1 — callers depend on it. The
//! choice of backing store and on-disk layout is Tier 5 — deliberate
//! divergence from cardano-node's three-DB pattern. See
//! `docs/clusters/PHASE4-N-D/S-33.md`.
//!
//! The sole implementation in this slice is [`InMemoryChainDb`], used
//! to validate the trait contract. A persistent backing store is the
//! subject of slice S-34.

mod contract;
mod crash_safety;
mod error;
mod in_memory;
mod persistent;
mod snapshot_contract;
mod types;

pub use contract::run_contract_tests;
pub use crash_safety::{run_crash_safety_tests, KillStrategy, NoKill};
pub use error::ChainDbError;
pub use in_memory::InMemoryChainDb;
pub use persistent::{PersistentChainDb, PersistentChainDbOptions, SyncCadence};
pub use snapshot_contract::run_snapshot_contract_tests;
pub use types::{ChainTip, StoredBlock};

use ade_types::primitives::{Hash32, SlotNo};

/// Iterator yielded by [`ChainDb::iter_from_slot`].
///
/// Boxed so the trait stays object-safe and impls are free to back the
/// iterator with arbitrary state (file handles, db cursors, etc.).
pub type BlockIter<'a> =
    Box<dyn Iterator<Item = Result<StoredBlock, ChainDbError>> + 'a>;

/// Logical chain database surface.
///
/// Single-writer, multi-reader. The trait is silent on fsync timing
/// and on-disk layout — implementations choose. The contract is
/// logical: after `put_block(b)?` returns, `b` is observable via
/// every read operation that can locate it (by hash, by slot, by
/// iteration). After a crash followed by reopen, the same property
/// holds for blocks whose `put_block` returned before the crash.
///
/// See [`run_contract_tests`] for the executable form of the contract.
pub trait ChainDb: Send + Sync {
    /// Insert a block. Subsequent reads must observe it.
    fn put_block(&self, block: &StoredBlock) -> Result<(), ChainDbError>;

    /// Look up a block by its content hash. `Ok(None)` is "no block
    /// at this hash" — a normal outcome, not an error.
    fn get_block_by_hash(
        &self,
        hash: &Hash32,
    ) -> Result<Option<StoredBlock>, ChainDbError>;

    /// Look up a block by slot. `Ok(None)` is "no block at this
    /// slot" — slots without blocks are a normal feature of
    /// Ouroboros.
    fn get_block_by_slot(
        &self,
        slot: SlotNo,
    ) -> Result<Option<StoredBlock>, ChainDbError>;

    /// Highest slot with a stored block, or `None` for an empty DB.
    fn tip(&self) -> Result<Option<ChainTip>, ChainDbError>;

    /// Stream blocks in slot order, starting at `from` (inclusive).
    /// Implementations may yield items lazily.
    fn iter_from_slot(&self, from: SlotNo) -> Result<BlockIter<'_>, ChainDbError>;

    /// Discard all blocks at slots strictly greater than `slot`.
    /// After return, no read operation observes such a block.
    /// Rolling back beyond the empty tip is `Ok(())` (no-op).
    fn rollback_to_slot(&self, slot: SlotNo) -> Result<(), ChainDbError>;
}

/// Snapshot storage surface (S-35).
///
/// Separate from [`ChainDb`] because snapshot lifecycle differs from
/// block storage (write cadence, read pattern, optionality). Callers
/// that need both take `D: ChainDb + SnapshotStore`. Bytes are opaque
/// at this layer — caller chooses the encoding (typically Ade's
/// canonical fingerprint format per the Phase 4 cluster plan).
pub trait SnapshotStore: Send + Sync {
    /// Insert a snapshot at `slot`. Idempotent if the same bytes
    /// were already stored at the same slot; conflicting bytes at
    /// the same slot return `InvalidOperation`.
    fn put_snapshot(
        &self,
        slot: SlotNo,
        bytes: &[u8],
    ) -> Result<(), ChainDbError>;

    /// Look up a snapshot by slot. `Ok(None)` if absent.
    fn get_snapshot(&self, slot: SlotNo) -> Result<Option<Vec<u8>>, ChainDbError>;

    /// Highest-slot snapshot, or `None` if none exist.
    fn latest_snapshot(&self)
        -> Result<Option<(SlotNo, Vec<u8>)>, ChainDbError>;

    /// All slots with stored snapshots, in ascending order.
    fn list_snapshot_slots(&self) -> Result<Vec<SlotNo>, ChainDbError>;

    /// Remove a snapshot at `slot`. `Ok(())` whether present or not.
    fn delete_snapshot(&self, slot: SlotNo) -> Result<(), ChainDbError>;

    /// Persist the seed-epoch consensus-inputs sidecar, keyed by the
    /// anchor fingerprint (`anchor_fp`). This is a surface **disjoint**
    /// from the slot-keyed snapshot namespace above — it lives in its
    /// own anchor-fp-keyed space, never a reserved sentinel slot, so a
    /// `put_seed_epoch_consensus_inputs(fp, …)` can never collide with
    /// or overwrite any `put_snapshot(slot, …)`. Idempotent if the same
    /// bytes were already stored for the same `anchor_fp`; conflicting
    /// bytes for the same `anchor_fp` return `InvalidOperation`
    /// (mirrors `put_snapshot`). PHASE4-N-F-A A2.
    fn put_seed_epoch_consensus_inputs(
        &self,
        anchor_fp: &Hash32,
        bytes: &[u8],
    ) -> Result<(), ChainDbError>;

    /// Look up the seed-epoch consensus-inputs sidecar by `anchor_fp`.
    /// `Ok(None)` if absent. Disjoint from the slot-keyed namespace.
    fn get_seed_epoch_consensus_inputs(
        &self,
        anchor_fp: &Hash32,
    ) -> Result<Option<Vec<u8>>, ChainDbError>;

    /// All anchor fingerprints with a stored seed-epoch consensus-inputs
    /// sidecar, ascending. PHASE4-N-F-C L3 (W2): a read-only **discovery**
    /// surface for warm-start recovery — it locates the persisted bootstrap
    /// anchor lineage(s) from the sidecar table key, a source independent of
    /// the WAL provenance entry (reading the anchor_fp back from the very WAL
    /// entry the warm-start then validates would be circular). Discovery ONLY:
    /// finding an `anchor_fp` here is NOT proof the sidecar is valid — the
    /// warm-start verify chain (WAL-provenance match → sidecar hash → anchor/
    /// epoch binding) still applies and is the actual authority. Empty when no
    /// sidecar has been persisted.
    fn list_seed_epoch_consensus_anchor_fps(&self) -> Result<Vec<Hash32>, ChainDbError>;
}
