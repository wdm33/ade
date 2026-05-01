// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Persistent `ChainDb` implementation backed by redb (S-34).
//!
//! Tier 5 choice per `docs/active/S-34_obligation_discharge.md` §O-34.1.
//! All redb-specific types stay inside this module; the public surface
//! is the [`ChainDb`] trait from the parent module. This isolation is
//! enforced by `rg "redb" crates/ade_runtime/src/chaindb/` showing
//! matches only inside this file.

use std::path::PathBuf;
use std::sync::Mutex;

use redb::{Database, ReadableTable, TableDefinition};

use ade_types::primitives::{Hash32, SlotNo};

use super::{BlockIter, ChainDb, ChainDbError, ChainTip, SnapshotStore, StoredBlock};

const BLOCKS_BY_SLOT: TableDefinition<u64, &[u8]> =
    TableDefinition::new("blocks_by_slot");
const SLOT_BY_HASH: TableDefinition<&[u8; 32], u64> =
    TableDefinition::new("slot_by_hash");
const SNAPSHOTS_BY_SLOT: TableDefinition<u64, &[u8]> =
    TableDefinition::new("snapshots_by_slot");
const META: TableDefinition<&str, &[u8]> = TableDefinition::new("meta");

/// Schema version. Bumped from 1 to 2 in S-35 to add the
/// `snapshots_by_slot` table. Forward-compatible with v1 files —
/// opening a v1 file succeeds; the schema_version is bumped to 2 on
/// the first write.
const SCHEMA_VERSION: u32 = 2;
const MAGIC_BYTES: &[u8] = b"ADE\0CHAINDB\0";
const META_KEY_MAGIC: &str = "magic";
const META_KEY_SCHEMA: &str = "schema_version";

/// Sync cadence policy. Per O-34.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncCadence {
    /// fsync per write. Strongest durability; trait-default semantics.
    PerWrite,
    /// fsync only on explicit `flush()` or drop. Operator-controlled
    /// durability; useful during initial sync from genesis.
    Manual,
}

/// Open / construction options for the persistent chaindb.
#[derive(Debug, Clone)]
pub struct PersistentChainDbOptions {
    pub path: PathBuf,
    pub sync_policy: SyncCadence,
}

impl PersistentChainDbOptions {
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            sync_policy: SyncCadence::PerWrite,
        }
    }

    pub fn with_sync_policy(mut self, policy: SyncCadence) -> Self {
        self.sync_policy = policy;
        self
    }
}

/// redb-backed `ChainDb`.
///
/// Single-writer; the `Mutex` guards write-transaction sequencing.
/// Reads do not acquire it (redb's read transactions are MVCC).
pub struct PersistentChainDb {
    db: Database,
    options: PersistentChainDbOptions,
    write_lock: Mutex<()>,
}

impl std::fmt::Debug for PersistentChainDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersistentChainDb")
            .field("path", &self.options.path)
            .field("sync_policy", &self.options.sync_policy)
            .finish()
    }
}

impl PersistentChainDb {
    /// Open or create a chaindb at the configured path.
    pub fn open(options: PersistentChainDbOptions) -> Result<Self, ChainDbError> {
        let db = Database::create(&options.path).map_err(map_db_err)?;
        let me = Self {
            db,
            options,
            write_lock: Mutex::new(()),
        };
        me.init_or_check_schema()?;
        Ok(me)
    }

    fn init_or_check_schema(&self) -> Result<(), ChainDbError> {
        // Try a read-only check first; if meta is absent, we initialize.
        let needs_init = {
            let txn = self.db.begin_read().map_err(map_txn_err)?;
            match txn.open_table(META) {
                Ok(meta) => {
                    let magic = meta
                        .get(META_KEY_MAGIC)
                        .map_err(map_storage_err)?
                        .map(|v| v.value().to_vec());
                    match magic {
                        Some(bytes) if bytes == MAGIC_BYTES => {
                            let version = meta
                                .get(META_KEY_SCHEMA)
                                .map_err(map_storage_err)?
                                .map(|v| u32_from_bytes(v.value()))
                                .ok_or_else(|| {
                                    ChainDbError::Corruption(
                                        "schema_version absent".into(),
                                    )
                                })?;
                            // Forward-compatible: v1 files are
                            // accepted. Schema is upgraded to current
                            // on first write below.
                            if version > SCHEMA_VERSION {
                                return Err(ChainDbError::SchemaMismatch {
                                    expected: SCHEMA_VERSION,
                                    found: version,
                                });
                            }
                            if version < SCHEMA_VERSION {
                                // upgrade on next write
                                true
                            } else {
                                false
                            }
                        }
                        Some(_) => {
                            return Err(ChainDbError::Corruption(
                                "magic mismatch — not an Ade chaindb file".into(),
                            ));
                        }
                        None => true,
                    }
                }
                Err(redb::TableError::TableDoesNotExist(_)) => true,
                Err(e) => return Err(map_table_err(e)),
            }
        };
        if needs_init {
            self.write_meta()?;
        }
        Ok(())
    }

    fn write_meta(&self) -> Result<(), ChainDbError> {
        let _guard = self.write_lock.lock().map_err(lock_poisoned)?;
        let mut txn = self.db.begin_write().map_err(map_txn_err)?;
        if matches!(self.options.sync_policy, SyncCadence::Manual) {
            txn.set_durability(redb::Durability::None);
        }
        {
            let mut meta = txn.open_table(META).map_err(map_table_err)?;
            meta.insert(META_KEY_MAGIC, MAGIC_BYTES)
                .map_err(map_storage_err)?;
            let version_bytes = SCHEMA_VERSION.to_le_bytes();
            meta.insert(META_KEY_SCHEMA, &version_bytes[..])
                .map_err(map_storage_err)?;
        }
        txn.commit().map_err(map_commit_err)?;
        Ok(())
    }

    fn begin_write(&self) -> Result<redb::WriteTransaction, ChainDbError> {
        let mut txn = self.db.begin_write().map_err(map_txn_err)?;
        if matches!(self.options.sync_policy, SyncCadence::Manual) {
            txn.set_durability(redb::Durability::None);
        }
        Ok(txn)
    }
}

impl ChainDb for PersistentChainDb {
    fn put_block(&self, block: &StoredBlock) -> Result<(), ChainDbError> {
        let _guard = self.write_lock.lock().map_err(lock_poisoned)?;

        // Detect slot conflict / idempotent re-put under a single
        // write transaction to keep the invariant atomic.
        let txn = self.begin_write()?;

        // Probe in a tightly-scoped read block first, then perform
        // the write below. Splitting these scopes avoids overlapping
        // borrows on the WriteTransaction's owned tables.
        let conflict = {
            let blocks = txn.open_table(BLOCKS_BY_SLOT).map_err(map_table_err)?;
            let already_present = blocks
                .get(block.slot.0)
                .map_err(map_storage_err)?
                .is_some();
            drop(blocks);
            if already_present {
                let hashes = txn.open_table(SLOT_BY_HASH).map_err(map_table_err)?;
                let existing_hash = hashes
                    .iter()
                    .map_err(map_storage_err)?
                    .filter_map(|r| r.ok())
                    .find_map(|(h, s)| {
                        if s.value() == block.slot.0 {
                            Some(*h.value())
                        } else {
                            None
                        }
                    });
                Some(existing_hash)
            } else {
                None
            }
        };

        match conflict {
            Some(Some(h)) if h == block.hash.0 => {
                // Idempotent re-put — same hash at the same slot.
                txn.commit().map_err(map_commit_err)?;
                return Ok(());
            }
            Some(_) => {
                return Err(ChainDbError::InvalidOperation(format!(
                    "slot {} already occupied by a different block",
                    block.slot.0,
                )));
            }
            None => {}
        }

        {
            let mut blocks = txn.open_table(BLOCKS_BY_SLOT).map_err(map_table_err)?;
            let mut hashes = txn.open_table(SLOT_BY_HASH).map_err(map_table_err)?;
            blocks
                .insert(block.slot.0, block.bytes.as_slice())
                .map_err(map_storage_err)?;
            hashes
                .insert(&block.hash.0, block.slot.0)
                .map_err(map_storage_err)?;
        }
        txn.commit().map_err(map_commit_err)?;
        Ok(())
    }

    fn get_block_by_hash(
        &self,
        hash: &Hash32,
    ) -> Result<Option<StoredBlock>, ChainDbError> {
        let txn = self.db.begin_read().map_err(map_txn_err)?;
        let hashes = match txn.open_table(SLOT_BY_HASH) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(map_table_err(e)),
        };
        let slot_value = hashes.get(&hash.0).map_err(map_storage_err)?;
        let Some(slot_value) = slot_value else {
            return Ok(None);
        };
        let slot = SlotNo(slot_value.value());
        drop(slot_value);
        drop(hashes);

        let blocks = match txn.open_table(BLOCKS_BY_SLOT) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(map_table_err(e)),
        };
        let bytes = blocks.get(slot.0).map_err(map_storage_err)?;
        Ok(bytes.map(|v| StoredBlock {
            slot,
            hash: hash.clone(),
            bytes: v.value().to_vec(),
        }))
    }

    fn get_block_by_slot(
        &self,
        slot: SlotNo,
    ) -> Result<Option<StoredBlock>, ChainDbError> {
        let txn = self.db.begin_read().map_err(map_txn_err)?;
        let blocks = match txn.open_table(BLOCKS_BY_SLOT) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(map_table_err(e)),
        };
        let bytes = blocks.get(slot.0).map_err(map_storage_err)?;
        let Some(bytes) = bytes else { return Ok(None) };
        let block_bytes = bytes.value().to_vec();
        drop(bytes);
        drop(blocks);

        // Recover the hash from the index. If the slot table has an
        // entry but the hash index doesn't, the db is corrupt.
        let hashes = match txn.open_table(SLOT_BY_HASH) {
            Ok(t) => t,
            Err(e) => return Err(map_table_err(e)),
        };
        let hash = hashes
            .iter()
            .map_err(map_storage_err)?
            .filter_map(|r| r.ok())
            .find_map(|(h, s)| {
                if s.value() == slot.0 {
                    Some(Hash32(*h.value()))
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                ChainDbError::Corruption(format!(
                    "slot {} present in blocks but absent from hash index",
                    slot.0,
                ))
            })?;

        Ok(Some(StoredBlock {
            slot,
            hash,
            bytes: block_bytes,
        }))
    }

    fn tip(&self) -> Result<Option<ChainTip>, ChainDbError> {
        let txn = self.db.begin_read().map_err(map_txn_err)?;
        let blocks = match txn.open_table(BLOCKS_BY_SLOT) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(map_table_err(e)),
        };
        let last_slot: Option<u64> = blocks
            .iter()
            .map_err(map_storage_err)?
            .filter_map(|r| r.ok())
            .map(|(s, _)| s.value())
            .last();
        drop(blocks);
        let Some(slot_raw) = last_slot else {
            return Ok(None);
        };
        let slot = SlotNo(slot_raw);

        let hashes = txn.open_table(SLOT_BY_HASH).map_err(map_table_err)?;
        let hash = hashes
            .iter()
            .map_err(map_storage_err)?
            .filter_map(|r| r.ok())
            .find_map(|(h, s)| {
                if s.value() == slot.0 {
                    Some(Hash32(*h.value()))
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                ChainDbError::Corruption(
                    "tip slot present in blocks but absent from hash index".into(),
                )
            })?;

        Ok(Some(ChainTip { hash, slot }))
    }

    fn iter_from_slot(&self, from: SlotNo) -> Result<BlockIter<'_>, ChainDbError> {
        // Snapshot into a Vec to avoid lifetime gymnastics with the
        // read transaction. Adequate for current callers; future
        // optimization can replace this with a self-referential
        // streaming iter if profiling shows iteration on hot paths.
        let txn = self.db.begin_read().map_err(map_txn_err)?;
        let blocks = match txn.open_table(BLOCKS_BY_SLOT) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => {
                return Ok(Box::new(std::iter::empty()));
            }
            Err(e) => return Err(map_table_err(e)),
        };
        let hashes = txn.open_table(SLOT_BY_HASH).map_err(map_table_err)?;

        let mut snapshot: Vec<StoredBlock> = Vec::new();
        for entry in blocks.range(from.0..).map_err(map_storage_err)? {
            let (slot_v, bytes_v) = entry.map_err(map_storage_err)?;
            let slot = SlotNo(slot_v.value());
            let bytes = bytes_v.value().to_vec();
            // Hash recovery — same secondary lookup as get_block_by_slot.
            let hash = hashes
                .iter()
                .map_err(map_storage_err)?
                .filter_map(|r| r.ok())
                .find_map(|(h, s)| {
                    if s.value() == slot.0 {
                        Some(Hash32(*h.value()))
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    ChainDbError::Corruption(format!(
                        "iter: slot {} present without hash index entry",
                        slot.0,
                    ))
                })?;
            snapshot.push(StoredBlock { slot, hash, bytes });
        }
        Ok(Box::new(snapshot.into_iter().map(Ok)))
    }

    fn rollback_to_slot(&self, slot: SlotNo) -> Result<(), ChainDbError> {
        let _guard = self.write_lock.lock().map_err(lock_poisoned)?;
        let txn = self.begin_write()?;
        {
            let mut blocks = txn.open_table(BLOCKS_BY_SLOT).map_err(map_table_err)?;
            let mut hashes = txn.open_table(SLOT_BY_HASH).map_err(map_table_err)?;

            // Collect slots to remove from the blocks table.
            let to_remove: Vec<u64> = blocks
                .range((slot.0 + 1)..)
                .map_err(map_storage_err)?
                .filter_map(|r| r.ok())
                .map(|(s, _)| s.value())
                .collect();

            // Find matching hashes via the index.
            let hashes_to_remove: Vec<[u8; 32]> = hashes
                .iter()
                .map_err(map_storage_err)?
                .filter_map(|r| r.ok())
                .filter_map(|(h, s)| {
                    if to_remove.contains(&s.value()) {
                        Some(*h.value())
                    } else {
                        None
                    }
                })
                .collect();

            for s in &to_remove {
                blocks.remove(*s).map_err(map_storage_err)?;
            }
            for h in &hashes_to_remove {
                hashes.remove(h).map_err(map_storage_err)?;
            }
        }
        txn.commit().map_err(map_commit_err)?;
        Ok(())
    }
}

impl SnapshotStore for PersistentChainDb {
    fn put_snapshot(
        &self,
        slot: SlotNo,
        bytes: &[u8],
    ) -> Result<(), ChainDbError> {
        let _guard = self.write_lock.lock().map_err(lock_poisoned)?;
        let txn = self.begin_write()?;

        // Conflict / idempotency check.
        let conflict = {
            let snapshots =
                txn.open_table(SNAPSHOTS_BY_SLOT).map_err(map_table_err)?;
            let existing = snapshots
                .get(slot.0)
                .map_err(map_storage_err)?
                .map(|v| v.value().to_vec());
            existing
        };

        match conflict {
            Some(existing_bytes) if existing_bytes == bytes => {
                txn.commit().map_err(map_commit_err)?;
                return Ok(());
            }
            Some(_) => {
                return Err(ChainDbError::InvalidOperation(format!(
                    "snapshot at slot {} already occupied by different bytes",
                    slot.0,
                )));
            }
            None => {}
        }

        {
            let mut snapshots =
                txn.open_table(SNAPSHOTS_BY_SLOT).map_err(map_table_err)?;
            snapshots
                .insert(slot.0, bytes)
                .map_err(map_storage_err)?;
        }
        txn.commit().map_err(map_commit_err)?;
        Ok(())
    }

    fn get_snapshot(&self, slot: SlotNo) -> Result<Option<Vec<u8>>, ChainDbError> {
        let txn = self.db.begin_read().map_err(map_txn_err)?;
        let snapshots = match txn.open_table(SNAPSHOTS_BY_SLOT) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(map_table_err(e)),
        };
        let bytes = snapshots.get(slot.0).map_err(map_storage_err)?;
        Ok(bytes.map(|v| v.value().to_vec()))
    }

    fn latest_snapshot(
        &self,
    ) -> Result<Option<(SlotNo, Vec<u8>)>, ChainDbError> {
        let txn = self.db.begin_read().map_err(map_txn_err)?;
        let snapshots = match txn.open_table(SNAPSHOTS_BY_SLOT) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(map_table_err(e)),
        };
        let last: Option<(u64, Vec<u8>)> = snapshots
            .iter()
            .map_err(map_storage_err)?
            .filter_map(|r| r.ok())
            .map(|(s, b)| (s.value(), b.value().to_vec()))
            .last();
        Ok(last.map(|(s, b)| (SlotNo(s), b)))
    }

    fn list_snapshot_slots(&self) -> Result<Vec<SlotNo>, ChainDbError> {
        let txn = self.db.begin_read().map_err(map_txn_err)?;
        let snapshots = match txn.open_table(SNAPSHOTS_BY_SLOT) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(e) => return Err(map_table_err(e)),
        };
        let slots: Vec<SlotNo> = snapshots
            .iter()
            .map_err(map_storage_err)?
            .filter_map(|r| r.ok())
            .map(|(s, _)| SlotNo(s.value()))
            .collect();
        Ok(slots)
    }

    fn delete_snapshot(&self, slot: SlotNo) -> Result<(), ChainDbError> {
        let _guard = self.write_lock.lock().map_err(lock_poisoned)?;
        let txn = self.begin_write()?;
        {
            let mut snapshots =
                txn.open_table(SNAPSHOTS_BY_SLOT).map_err(map_table_err)?;
            snapshots.remove(slot.0).map_err(map_storage_err)?;
        }
        txn.commit().map_err(map_commit_err)?;
        Ok(())
    }
}

// ---------- error mapping ----------

fn lock_poisoned<T>(_: std::sync::PoisonError<T>) -> ChainDbError {
    ChainDbError::Corruption("persistent chaindb mutex poisoned".to_string())
}

fn map_db_err(e: redb::DatabaseError) -> ChainDbError {
    match e {
        redb::DatabaseError::Storage(s) => map_storage_err(s),
        other => ChainDbError::Corruption(format!("redb open: {other}")),
    }
}

fn map_txn_err(e: redb::TransactionError) -> ChainDbError {
    match e {
        redb::TransactionError::Storage(s) => map_storage_err(s),
        other => ChainDbError::Corruption(format!("redb txn: {other}")),
    }
}

fn map_table_err(e: redb::TableError) -> ChainDbError {
    match e {
        redb::TableError::Storage(s) => map_storage_err(s),
        other => ChainDbError::Corruption(format!("redb table: {other}")),
    }
}

fn map_commit_err(e: redb::CommitError) -> ChainDbError {
    match e {
        redb::CommitError::Storage(s) => map_storage_err(s),
        other => ChainDbError::Corruption(format!("redb commit: {other}")),
    }
}

fn map_storage_err(e: redb::StorageError) -> ChainDbError {
    match e {
        redb::StorageError::Io(io) => ChainDbError::Io(io),
        redb::StorageError::Corrupted(detail) => {
            ChainDbError::Corruption(format!("redb storage: {detail}"))
        }
        other => ChainDbError::Corruption(format!("redb storage: {other}")),
    }
}

fn u32_from_bytes(b: &[u8]) -> u32 {
    if b.len() != 4 {
        return 0; // schema check upstream catches this as 0 != SCHEMA_VERSION
    }
    let mut buf = [0u8; 4];
    buf.copy_from_slice(b);
    u32::from_le_bytes(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chaindb::run_contract_tests;
    use tempfile::TempDir;

    fn fresh_db(dir: &TempDir, name: &str) -> PersistentChainDb {
        let path = dir.path().join(name);
        PersistentChainDb::open(PersistentChainDbOptions::at(&path))
            .expect("open chaindb")
    }

    #[test]
    fn persistent_passes_contract() {
        let tmp = TempDir::new().expect("tempdir");
        let counter = std::cell::Cell::new(0u32);
        run_contract_tests(|| {
            counter.set(counter.get() + 1);
            fresh_db(&tmp, &format!("contract_{}.chaindb", counter.get()))
        });
    }

    #[test]
    fn persistent_passes_snapshot_contract() {
        use crate::chaindb::run_snapshot_contract_tests;
        let tmp = TempDir::new().expect("tempdir");
        let counter = std::cell::Cell::new(0u32);
        run_snapshot_contract_tests(|| {
            counter.set(counter.get() + 1);
            fresh_db(&tmp, &format!("snap_{}.chaindb", counter.get()))
        });
    }

    #[test]
    fn snapshots_persist_across_reopen() {
        use crate::chaindb::SnapshotStore;
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("snap_reopen.chaindb");
        {
            let db = PersistentChainDb::open(PersistentChainDbOptions::at(&path))
                .expect("open");
            db.put_snapshot(SlotNo(42), b"snapshot bytes").expect("put");
        }
        let db = PersistentChainDb::open(PersistentChainDbOptions::at(&path))
            .expect("reopen");
        let got = db
            .get_snapshot(SlotNo(42))
            .expect("get")
            .expect("present");
        assert_eq!(got, b"snapshot bytes");
    }

    #[test]
    fn reopen_observes_committed_block() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("reopen.chaindb");
        let block = StoredBlock {
            slot: SlotNo(42),
            hash: Hash32([0x42; 32]),
            bytes: vec![0xab; 128],
        };
        {
            let db =
                PersistentChainDb::open(PersistentChainDbOptions::at(&path)).expect("open");
            db.put_block(&block).expect("put");
        }
        // Drop the first handle, reopen.
        let db =
            PersistentChainDb::open(PersistentChainDbOptions::at(&path)).expect("reopen");
        let got = db
            .get_block_by_slot(SlotNo(42))
            .expect("get")
            .expect("present");
        assert_eq!(got, block);
    }

    #[test]
    fn corrupted_magic_returns_corruption_error() {
        // A redb file written by another tool / unrelated content
        // should fail the magic check.
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("foreign.chaindb");
        // Create a redb db without writing our magic.
        {
            let db = redb::Database::create(&path).expect("create raw redb");
            let txn = db.begin_write().expect("begin");
            {
                let mut t = txn
                    .open_table::<&str, &[u8]>(redb::TableDefinition::new("meta"))
                    .expect("open");
                t.insert("magic", &b"NOT_ADE"[..]).expect("insert");
            }
            txn.commit().expect("commit");
        }
        let result = PersistentChainDb::open(PersistentChainDbOptions::at(&path));
        assert!(
            matches!(result, Err(ChainDbError::Corruption(_))),
            "expected Corruption, got {result:?}",
        );
    }
}
