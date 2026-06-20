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
mod reduced_utxo_checkpoint;
mod reduced_window_driver;
mod snapshot_contract;
mod transient_epoch_view;
mod types;
mod utxo_anchor;
mod utxo_key;

pub use contract::run_contract_tests;
pub use crash_safety::{run_crash_safety_tests, KillStrategy, NoKill};
pub use error::ChainDbError;
pub use reduced_utxo_checkpoint::{ReducedCheckpointError, ReducedUtxoCheckpoint};
pub use reduced_window_driver::{drive_window_aggregate, WindowDriverError};
pub use transient_epoch_view::{
    is_valid_window_key, purge_transient_root, transient_root, window_key,
    TransientEpochViewStore, TransientViewError, TRANSIENT_SUBTREE,
};
pub use utxo_anchor::AnchorPosition;
pub use in_memory::InMemoryChainDb;
pub use persistent::{PersistentChainDb, PersistentChainDbOptions, SyncCadence};
pub use snapshot_contract::run_snapshot_contract_tests;
pub use types::{CappedSlotRange, ChainTip, StoredBlock};

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
    ///
    /// NOTE (PHASE4-N-AA / DC-SERVEMEM-01): some impls recover the tip hash via a
    /// full hash-index scan. This is a TRUSTED-CALLER read (node startup, etc.) —
    /// the peer-driven serve path MUST use [`ChainDb::last_block_bytes`] instead.
    fn tip(&self) -> Result<Option<ChainTip>, ChainDbError>;

    /// Stream blocks in slot order, starting at `from` (inclusive).
    /// Implementations may yield items lazily.
    ///
    /// NOTE (PHASE4-N-AA / DC-SERVEMEM-01): some impls MATERIALIZE the full
    /// `from..tip` range and recover each block's hash via a per-block hash-index
    /// scan (O(N²)). This is for TRUSTED, full-range internal callers only
    /// (recovery / rollback). The peer-driven `--mode node` serve path MUST use
    /// the bounded [`ChainDb::range_bytes_capped`] / [`ChainDb::last_block_bytes`]
    /// primitives instead — never this method.
    fn iter_from_slot(&self, from: SlotNo) -> Result<BlockIter<'_>, ChainDbError>;

    /// Bounded, hash-free, slot-ordered read of blocks in `[from, to]`
    /// (inclusive) for the peer-driven serve path (PHASE4-N-AA, DC-SERVEMEM-01).
    ///
    /// Returns at most `max` blocks as `(slot, bytes)` in ascending slot order,
    /// plus [`CappedSlotRange::truncated`] = `true` when the range contained MORE
    /// than `max` blocks (the per-request serve cap was exceeded). Memory is
    /// bounded to `<= max` blocks regardless of chain length. Does NOT recover the
    /// block hash (the serve derives it from the bytes via the BLUE decode
    /// authority) — so this performs NO hash-index scan. Use this on the serve
    /// path, NOT [`ChainDb::iter_from_slot`].
    fn range_bytes_capped(
        &self,
        from: SlotNo,
        to: SlotNo,
        max: usize,
    ) -> Result<CappedSlotRange, ChainDbError>;

    /// The highest-slot stored block's `(slot, bytes)`, or `None` for an empty
    /// DB. O(log N) tip access (no full iteration, no hash scan); the serve
    /// derives the tip hash + block_no from the bytes (PHASE4-N-AA,
    /// DC-SERVEMEM-01).
    fn last_block_bytes(&self) -> Result<Option<(SlotNo, Vec<u8>)>, ChainDbError>;

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

    /// Persist the recovered anchor-point provenance record, keyed by the
    /// anchor fingerprint (`anchor_fp`). PHASE4-N-AK AK-S1 (DC-NODE-31). This
    /// is a surface **disjoint** from both the slot-keyed snapshot namespace
    /// and the seed-epoch consensus-inputs sidecar — its own anchor-fp-keyed
    /// space, so a `put_recovered_anchor_point(fp, …)` can never collide with
    /// any `put_snapshot(slot, …)` or `put_seed_epoch_consensus_inputs(fp, …)`.
    /// The record's bytes are the canonical [`ade_ledger::recovered_anchor_point`]
    /// encoding of `(slot, hash)` bound to `anchor_fp`. Idempotent if the same
    /// bytes were already stored for the same `anchor_fp`; conflicting bytes for
    /// the same `anchor_fp` return `InvalidOperation` (mirrors `put_snapshot`).
    fn put_recovered_anchor_point(
        &self,
        anchor_fp: &Hash32,
        bytes: &[u8],
    ) -> Result<(), ChainDbError>;

    /// Look up the recovered anchor-point provenance record by `anchor_fp`.
    /// `Ok(None)` if absent (a pre-AK store, or a torn write before the WAL
    /// commit point) — the warm-start load fails closed on absence for a
    /// non-Origin recovered store (DC-NODE-31). Disjoint from the slot-keyed
    /// and seed-epoch sidecar namespaces.
    fn get_recovered_anchor_point(
        &self,
        anchor_fp: &Hash32,
    ) -> Result<Option<Vec<u8>>, ChainDbError>;
}
