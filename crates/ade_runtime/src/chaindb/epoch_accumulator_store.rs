// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! LIVE-LEDGER-EPOCH-TRANSITION S2 (DC-EPOCH-20) — the DURABLE `EpochAccumulator` store.
//!
//! The small non-UTxO companion authority (`ade_ledger::epoch_accumulator`) persisted beside the
//! disk-backed reduced UTxO checkpoint. Unlike the reduced checkpoint (a per-key UTxO map), the
//! accumulator is a SINGLE canonical value, so this is a single-blob store: the current accumulator blob +
//! `LAST_SLOT` cursor, plus an immutable sealed bootstrap blob + `SEED_SLOT` for reorg-reset.
//!
//! TCB color: RED shell (redb I/O). It is a GREEN durable CACHE of a BLUE-derivable value — the
//! accumulator is reconstructible by folding `apply_selected_block` over the durable selected chain
//! (DC-EPOCH-20 rematerialization), so a lost/corrupt store is rebuilt by replay and is never authority on
//! its own. The canonical blob is `ade_ledger::epoch_accumulator::encode_epoch_accumulator` (no second
//! encoding scheme).
//!
//! DC-EPOCH-20 (no resumed split prefix). The accumulator is one of four derived stores that must reflect
//! the same selected-chain prefix (the WAL tail). This store carries the durable `LAST_SLOT` so a lagging
//! accumulator is DETECTABLE, and `verify_advanced_through` / `verify_ready_at` fail closed so a
//! lagging / wrong-lineage / overshot accumulator can never be read as authority — recovery rematerializes
//! it to the WAL tail first.
//!
//! Crash-safety: `seal_bootstrap` writes the blobs + slots, then the completeness marker LAST in a
//! separate durable commit; a SIGKILL before the marker leaves `is_complete() == false` (the caller
//! re-seals — a partial seal is never mistaken for a complete one). `advance` writes the current blob +
//! `LAST_SLOT` in ONE redb commit (atomic — the stored blob always matches its stored slot, never a torn
//! blob/slot pair). A reorg is `reset_to_bootstrap` + forward replay, never an ad hoc inverse mutation.

use std::path::Path;

use ade_ledger::epoch_accumulator::{
    decode_epoch_accumulator, encode_epoch_accumulator, EpochAccumulator,
};
use ade_types::{BlockNo, CardanoEra, Hash32, SlotNo};
use redb::{Database, ReadableTable, TableDefinition};

const META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("epoch_acc_meta");
/// The current accumulator, canonically encoded (`encode_epoch_accumulator`).
const CURRENT_BLOB_KEY: &str = "current_blob";
/// The slot the current accumulator is applied through (8 BE bytes) — the DC-EPOCH-20 `LAST_SLOT` cursor.
const LAST_SLOT_KEY: &str = "last_advanced_slot";
/// The IMMUTABLE sealed bootstrap accumulator (the seed baseline). A reorg resets the current blob to this.
const BOOTSTRAP_BLOB_KEY: &str = "bootstrap_blob";
/// The IMMUTABLE sealed seed slot. `reset_to_bootstrap` resets `LAST_SLOT` back to this.
const SEED_SLOT_KEY: &str = "seed_slot";
/// Present iff `seal_bootstrap` completed (written LAST). A partial seal has `is_complete() == false`.
const COMPLETE_KEY: &str = "complete";
/// LIVE-LEDGER-EPOCH-TRANSITION S3 (DC-EPOCH-22): the PENDING boundary-mark WITNESS — the canonical boundary
/// point + lineage the co-advancer committed to crossing at, persisted BEFORE the accumulator crosses. 40
/// bytes: `boundary_slot` (8 BE) ++ `boundary_hash` (32). The mark VALUE is NOT stored — it is the
/// deterministic projection of the lineage-matched reduced checkpoint at that point (re-derived on consume,
/// never double-stored; MEM-OPT). The witness is the durable commitment + the reorg lineage key: a reorg
/// that removes/replaces the boundary point yields a different `boundary_hash`, invalidating the binding.
const PENDING_BOUNDARY_MARK_KEY: &str = "pending_boundary_mark";

/// LIVE-LEDGER-EPOCH-TRANSITION S5: the accumulator LINEAGE ANCHOR — the canonical selected point the
/// persisted accumulator last represented. 48 bytes: `slot` (8 BE) ++ `block_no` (8 BE) ++ `header_hash`
/// (32). Written ATOMICALLY with each advance (same redb commit as the blob + `LAST_SLOT`), so a certified
/// store binds its accumulator to a specific selected-chain point, not just a height. PRESENCE = lineage-
/// certified; ABSENCE = uncertified (a legacy pre-anchor store, or the transitional state after
/// `reset_to_bootstrap`) — recovery must reset + re-fold from canonical blocks, never trust height alone.
/// A present-but-malformed anchor fails closed (`CorruptLastAdvancedPoint`).
const LAST_ADVANCED_POINT_KEY: &str = "last_advanced_point";

/// Closed store-failure surface.
#[derive(Debug)]
pub enum EpochAccumulatorStoreError {
    /// A redb error (open / txn / table / commit).
    Redb(String),
    /// A stored value was expected but absent (a corrupt / partially-written store).
    Missing(&'static str),
    /// A stored blob failed to decode (corrupt store).
    Decode(String),
    /// A stored slot value was not 8 bytes (corrupt store).
    CorruptSlot,
    /// `advance` / `reset_to_bootstrap` called before the store was sealed.
    NotSealed,
    /// A non-forward `advance` (slot ≤ the last advanced slot). The accumulator only moves forward; a
    /// reorg uses `reset_to_bootstrap` + replay, never a backward `advance`.
    NonMonotonicAdvance { slot: u64, last: u64 },
    /// The persisted `LastAdvancedPoint` lineage anchor is malformed — wrong length, or its slot disagrees
    /// with `LAST_SLOT` (they are written in one commit, so a mismatch is corruption). Fail closed: recovery
    /// must not read a corrupt anchor as authority.
    CorruptLastAdvancedPoint,
}

/// LIVE-LEDGER-EPOCH-TRANSITION S5: the canonical selected point the persisted accumulator last represented
/// — the lineage anchor recovery admission checks against (S5 `admit_rollback`). `header_hash` is the
/// already-authoritative stored header hash (NOT a re-derived convenience hash); `block_no` is the decoded
/// canonical header's block number (the height cardano's `SecurityParam` k bounds).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastAdvancedPoint {
    pub slot: SlotNo,
    pub block_no: BlockNo,
    pub header_hash: Hash32,
}

impl LastAdvancedPoint {
    /// Fixed-width canonical bytes: `slot` (8 BE) ++ `block_no` (8 BE) ++ `header_hash` (32) = 48.
    fn encode(&self) -> [u8; 48] {
        let mut b = [0u8; 48];
        b[0..8].copy_from_slice(&self.slot.0.to_be_bytes());
        b[8..16].copy_from_slice(&self.block_no.0.to_be_bytes());
        b[16..48].copy_from_slice(&self.header_hash.0);
        b
    }
    /// Decode the fixed-width bytes. Wrong length is `CorruptLastAdvancedPoint`.
    fn decode(bytes: &[u8]) -> Result<Self, EpochAccumulatorStoreError> {
        if bytes.len() != 48 {
            return Err(EpochAccumulatorStoreError::CorruptLastAdvancedPoint);
        }
        let mut slot = [0u8; 8];
        slot.copy_from_slice(&bytes[0..8]);
        let mut block = [0u8; 8];
        block.copy_from_slice(&bytes[8..16]);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes[16..48]);
        Ok(LastAdvancedPoint {
            slot: SlotNo(u64::from_be_bytes(slot)),
            block_no: BlockNo(u64::from_be_bytes(block)),
            header_hash: Hash32(hash),
        })
    }
}

/// Why the accumulator is NOT ready to be read as authority at a required slot (DC-EPOCH-20). Mirrors the
/// reduced checkpoint's readiness gate: every variant FAILS CLOSED, so a missing / corrupt / lagging /
/// wrong-lineage / overshot accumulator can never be read as authority — recovery must rematerialize it to
/// the required (WAL-tail) slot first.
#[derive(Debug, PartialEq, Eq)]
pub enum AccumulatorReadinessError {
    /// Reading the store failed (redb / decode — a corrupt store).
    Read(String),
    /// The store carries no sealed bootstrap baseline (uninitialised / crashed seal).
    Unsealed,
    /// The sealed seed slot does not match the expected bootstrap lineage.
    SeedMismatch { seed: u64, expected: u64 },
    /// The accumulator has not advanced to the required slot yet (behind the WAL tail).
    Lagging { advanced: u64, required: u64 },
    /// The accumulator advanced PAST the required slot (an unhandled rollback / overshoot) — its state no
    /// longer reflects the required slot exactly.
    Ahead { advanced: u64, required: u64 },
    /// CONWAY-PROPOSAL-DEPOSIT-EXPIRY S2 (absent ≠ empty): the sealed bootstrap baseline is a Conway+
    /// store that carries NO imported governance state (`gov_state = None`) — it PREDATES the
    /// governance-proposal import (a pre-v6 bootstrap). It must NEVER load as "zero proposals" (an absent
    /// set is not an empty set); fail closed — re-bootstrap to upgrade. A v6 bootstrap always carries
    /// `gov_state = Some(..)`, even when the pending-proposal set is empty.
    GovernanceImportRequired { era_tag: u64 },
}

fn rerr(e: impl std::fmt::Debug) -> EpochAccumulatorStoreError {
    EpochAccumulatorStoreError::Redb(format!("{e:?}"))
}

fn parse_slot(b: &[u8]) -> Result<SlotNo, EpochAccumulatorStoreError> {
    if b.len() != 8 {
        return Err(EpochAccumulatorStoreError::CorruptSlot);
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(b);
    Ok(SlotNo(u64::from_be_bytes(arr)))
}

/// Parse the 40-byte boundary-mark witness (`boundary_slot` 8 BE ++ `boundary_hash` 32). A wrong length is
/// a corrupt store (`CorruptSlot` — the closed fixed-width-value-malformed surface).
fn parse_boundary_witness(b: &[u8]) -> Result<(SlotNo, Hash32), EpochAccumulatorStoreError> {
    if b.len() != 40 {
        return Err(EpochAccumulatorStoreError::CorruptSlot);
    }
    let mut slot = [0u8; 8];
    slot.copy_from_slice(&b[..8]);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&b[8..]);
    Ok((SlotNo(u64::from_be_bytes(slot)), Hash32(hash)))
}

/// The durable single-value `EpochAccumulator` store (DC-EPOCH-20).
pub struct EpochAccumulatorStore {
    db: Database,
}

impl EpochAccumulatorStore {
    /// Open (create if absent) the store at `path`. redb's default `Immediate` durability (fsync per
    /// commit) gives crash-safe commits.
    pub fn open(path: &Path) -> Result<Self, EpochAccumulatorStoreError> {
        let db = Database::create(path).map_err(rerr)?;
        Ok(Self { db })
    }

    /// Seal the bootstrap baseline: the accumulator at `seed_slot` becomes BOTH the immutable reorg-reset
    /// baseline AND the initial current state. The completeness marker is written LAST in a separate commit
    /// so a crash mid-seal leaves `is_complete() == false`.
    pub fn seal_bootstrap(
        &self,
        acc: &EpochAccumulator,
        seed_slot: SlotNo,
    ) -> Result<(), EpochAccumulatorStoreError> {
        let blob = encode_epoch_accumulator(acc);
        let seed_bytes = seed_slot.0.to_be_bytes();
        {
            let txn = self.db.begin_write().map_err(rerr)?;
            {
                let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
                // Clear any prior completeness marker FIRST so a re-seal is never seen as complete mid-write.
                let _ = meta.remove(COMPLETE_KEY).map_err(rerr)?;
                meta.insert(BOOTSTRAP_BLOB_KEY, blob.as_slice())
                    .map_err(rerr)?;
                meta.insert(SEED_SLOT_KEY, seed_bytes.as_slice())
                    .map_err(rerr)?;
                meta.insert(CURRENT_BLOB_KEY, blob.as_slice())
                    .map_err(rerr)?;
                meta.insert(LAST_SLOT_KEY, seed_bytes.as_slice())
                    .map_err(rerr)?;
            }
            txn.commit().map_err(rerr)?;
        }
        // Completeness marker LAST, in a separate durable commit.
        {
            let txn = self.db.begin_write().map_err(rerr)?;
            {
                let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
                meta.insert(COMPLETE_KEY, [1u8].as_slice()).map_err(rerr)?;
            }
            txn.commit().map_err(rerr)?;
        }
        Ok(())
    }

    /// Advance the current accumulator to the canonical selected point `(slot, block_no, header_hash)`. The
    /// blob + `LAST_SLOT` + the `LastAdvancedPoint` lineage anchor are written in ONE redb commit, so a
    /// certified store always binds its accumulator to the exact point it last represented (not just a
    /// height). `header_hash` MUST be the authoritative stored header hash and `block_no` the decoded
    /// canonical header block number. Fail-closed if unsealed or non-forward.
    pub fn advance(
        &self,
        acc: &EpochAccumulator,
        slot: SlotNo,
        block_no: BlockNo,
        header_hash: Hash32,
    ) -> Result<(), EpochAccumulatorStoreError> {
        let last = self
            .last_advanced_slot()?
            .ok_or(EpochAccumulatorStoreError::NotSealed)?;
        if !self.is_complete()? {
            return Err(EpochAccumulatorStoreError::NotSealed);
        }
        if slot.0 <= last.0 {
            return Err(EpochAccumulatorStoreError::NonMonotonicAdvance {
                slot: slot.0,
                last: last.0,
            });
        }
        let blob = encode_epoch_accumulator(acc);
        let slot_bytes = slot.0.to_be_bytes();
        let anchor = LastAdvancedPoint {
            slot,
            block_no,
            header_hash,
        }
        .encode();
        let txn = self.db.begin_write().map_err(rerr)?;
        {
            let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
            meta.insert(CURRENT_BLOB_KEY, blob.as_slice())
                .map_err(rerr)?;
            meta.insert(LAST_SLOT_KEY, slot_bytes.as_slice())
                .map_err(rerr)?;
            // S5: the lineage anchor rides the SAME atomic commit as the blob + LAST_SLOT.
            meta.insert(LAST_ADVANCED_POINT_KEY, anchor.as_slice())
                .map_err(rerr)?;
        }
        txn.commit().map_err(rerr)?;
        Ok(())
    }

    /// The lineage anchor the persisted accumulator is certified to, or `None` if the store is NOT
    /// lineage-certified — a legacy pre-anchor store, or the transitional state after `reset_to_bootstrap`.
    /// Recovery treats `None` as uncertified (reset + re-fold from canonical blocks), NEVER as trusted height
    /// authority. A present anchor whose slot disagrees with `LAST_SLOT`, or whose bytes are the wrong length,
    /// is corruption (`CorruptLastAdvancedPoint`) — fail closed.
    pub fn last_advanced_point(
        &self,
    ) -> Result<Option<LastAdvancedPoint>, EpochAccumulatorStoreError> {
        let txn = self.db.begin_read().map_err(rerr)?;
        let meta = txn.open_table(META_TABLE).map_err(rerr)?;
        let raw = match meta.get(LAST_ADVANCED_POINT_KEY).map_err(rerr)? {
            None => return Ok(None),
            Some(v) => v.value().to_vec(),
        };
        let point = LastAdvancedPoint::decode(&raw)?;
        // Integrity: the anchor is written in the same commit as LAST_SLOT, so its slot MUST match.
        let last = match meta.get(LAST_SLOT_KEY).map_err(rerr)? {
            Some(s) => parse_slot(s.value())?,
            None => return Err(EpochAccumulatorStoreError::CorruptLastAdvancedPoint),
        };
        if point.slot != last {
            return Err(EpochAccumulatorStoreError::CorruptLastAdvancedPoint);
        }
        Ok(Some(point))
    }

    /// Load the current accumulator + the slot it is applied through. `None` if unsealed.
    pub fn load_current(
        &self,
    ) -> Result<Option<(SlotNo, EpochAccumulator)>, EpochAccumulatorStoreError> {
        if !self.is_complete()? {
            return Ok(None);
        }
        let txn = self.db.begin_read().map_err(rerr)?;
        let meta = txn.open_table(META_TABLE).map_err(rerr)?;
        let slot = match meta.get(LAST_SLOT_KEY).map_err(rerr)? {
            Some(v) => parse_slot(v.value())?,
            None => return Err(EpochAccumulatorStoreError::Missing(LAST_SLOT_KEY)),
        };
        let acc = match meta.get(CURRENT_BLOB_KEY).map_err(rerr)? {
            Some(v) => decode_epoch_accumulator(v.value())
                .map_err(|e| EpochAccumulatorStoreError::Decode(format!("{e:?}")))?,
            None => return Err(EpochAccumulatorStoreError::Missing(CURRENT_BLOB_KEY)),
        };
        Ok(Some((slot, acc)))
    }

    /// Reorg reset: restore the current accumulator to the sealed bootstrap baseline and `LAST_SLOT` back
    /// to the seed slot. The advancer then re-materializes by replaying the rolled-back canonical chain
    /// (the same fold as restart) — never an ad hoc inverse mutation.
    pub fn reset_to_bootstrap(&self) -> Result<(), EpochAccumulatorStoreError> {
        if !self.is_complete()? {
            return Err(EpochAccumulatorStoreError::NotSealed);
        }
        let txn = self.db.begin_write().map_err(rerr)?;
        {
            let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
            let boot = meta
                .get(BOOTSTRAP_BLOB_KEY)
                .map_err(rerr)?
                .map(|v| v.value().to_vec())
                .ok_or(EpochAccumulatorStoreError::Missing(BOOTSTRAP_BLOB_KEY))?;
            let seed = meta
                .get(SEED_SLOT_KEY)
                .map_err(rerr)?
                .map(|v| v.value().to_vec())
                .ok_or(EpochAccumulatorStoreError::Missing(SEED_SLOT_KEY))?;
            meta.insert(CURRENT_BLOB_KEY, boot.as_slice())
                .map_err(rerr)?;
            meta.insert(LAST_SLOT_KEY, seed.as_slice()).map_err(rerr)?;
            // DC-EPOCH-22: a reorg reset invalidates any pending boundary-mark binding (its lineage no longer
            // holds) — drop it so the rematerialized chain re-binds at its OWN boundary point.
            let _ = meta.remove(PENDING_BOUNDARY_MARK_KEY).map_err(rerr)?;
            // S5: a reset leaves the accumulator at the seed baseline but NOT lineage-certified — clear the
            // anchor. Recovery treats the cleared store as uncertified until a successful canonical re-fold
            // re-writes a fresh LastAdvancedPoint; it never trusts a reset store as lineage authority.
            let _ = meta.remove(LAST_ADVANCED_POINT_KEY).map_err(rerr)?;
        }
        txn.commit().map_err(rerr)?;
        Ok(())
    }

    /// DC-EPOCH-22 (BOUNDARY-ALIGNED-MARK-CAPTURE): durably BIND the boundary-mark witness — the canonical
    /// boundary point `(boundary_slot, boundary_hash)` the co-advancer is about to cross at — in ONE redb
    /// commit, BEFORE the accumulator crosses. The mark VALUE is not stored: it is the deterministic
    /// projection of the lineage-matched reduced checkpoint at `boundary_slot` (re-derived on consume, never
    /// double-stored). Fail-closed if unsealed. A later `bind` overwrites (the next boundary).
    pub fn bind_boundary_mark(
        &self,
        boundary_slot: SlotNo,
        boundary_hash: &Hash32,
    ) -> Result<(), EpochAccumulatorStoreError> {
        if !self.is_complete()? {
            return Err(EpochAccumulatorStoreError::NotSealed);
        }
        let mut witness = [0u8; 40];
        witness[..8].copy_from_slice(&boundary_slot.0.to_be_bytes());
        witness[8..].copy_from_slice(&boundary_hash.0);
        let txn = self.db.begin_write().map_err(rerr)?;
        {
            let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
            meta.insert(PENDING_BOUNDARY_MARK_KEY, witness.as_slice())
                .map_err(rerr)?;
        }
        txn.commit().map_err(rerr)?;
        Ok(())
    }

    /// DC-EPOCH-22: the pending boundary-mark witness `(boundary_slot, boundary_hash)`, or `None` if absent
    /// (no boundary pending / cleared / reorg-dropped). The co-advancer validates `boundary_hash` against the
    /// canonical durable block at `boundary_slot` before consuming the mark — a mismatch is a stale (reorged)
    /// binding, never reused on an epoch-number match alone.
    pub fn boundary_mark_binding(
        &self,
    ) -> Result<Option<(SlotNo, Hash32)>, EpochAccumulatorStoreError> {
        let txn = self.db.begin_read().map_err(rerr)?;
        let meta = match txn.open_table(META_TABLE) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        match meta.get(PENDING_BOUNDARY_MARK_KEY).map_err(rerr)? {
            Some(v) => Ok(Some(parse_boundary_witness(v.value())?)),
            None => Ok(None),
        }
    }

    /// DC-EPOCH-22: clear the pending boundary-mark witness once the cross has consumed it (one commit;
    /// idempotent — a no-op if absent). The binding is transient: it lives only between `bind_boundary_mark`
    /// and the cross that consumes it.
    pub fn clear_boundary_mark(&self) -> Result<(), EpochAccumulatorStoreError> {
        let txn = self.db.begin_write().map_err(rerr)?;
        {
            let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
            let _ = meta.remove(PENDING_BOUNDARY_MARK_KEY).map_err(rerr)?;
        }
        txn.commit().map_err(rerr)?;
        Ok(())
    }

    /// Whether the store carries a completeness marker (a sealed, non-partial store).
    pub fn is_complete(&self) -> Result<bool, EpochAccumulatorStoreError> {
        let txn = self.db.begin_read().map_err(rerr)?;
        let meta = match txn.open_table(META_TABLE) {
            Ok(t) => t,
            Err(_) => return Ok(false),
        };
        Ok(meta.get(COMPLETE_KEY).map_err(rerr)?.is_some())
    }

    /// The slot the accumulator is applied through, or `None` if unsealed.
    pub fn last_advanced_slot(&self) -> Result<Option<SlotNo>, EpochAccumulatorStoreError> {
        let txn = self.db.begin_read().map_err(rerr)?;
        let meta = match txn.open_table(META_TABLE) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        match meta.get(LAST_SLOT_KEY).map_err(rerr)? {
            Some(v) => Ok(Some(parse_slot(v.value())?)),
            None => Ok(None),
        }
    }

    /// The immutable sealed seed slot, or `None` if unsealed.
    pub fn seed_slot(&self) -> Result<Option<SlotNo>, EpochAccumulatorStoreError> {
        let txn = self.db.begin_read().map_err(rerr)?;
        let meta = match txn.open_table(META_TABLE) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        match meta.get(SEED_SLOT_KEY).map_err(rerr)? {
            Some(v) => Ok(Some(parse_slot(v.value())?)),
            None => Ok(None),
        }
    }

    /// Readiness witness (DC-EPOCH-20): the accumulator has advanced AT OR BEYOND `required_slot` against
    /// the expected bootstrap lineage. The catch-up gate — fails closed on unsealed / seed mismatch /
    /// lagging; at-or-beyond is acceptable (a recovery fold lands exactly at the WAL tail; an over-advance
    /// is caught by `verify_ready_at`).
    pub fn verify_advanced_through(
        &self,
        required_slot: SlotNo,
        expected_seed_slot: SlotNo,
    ) -> Result<(), AccumulatorReadinessError> {
        let (seed, advanced) = self.readiness_inputs(expected_seed_slot)?;
        if advanced < required_slot.0 {
            return Err(AccumulatorReadinessError::Lagging {
                advanced,
                required: required_slot.0,
            });
        }
        let _ = seed;
        Ok(())
    }

    /// Exact readiness gate (DC-EPOCH-20): the accumulator sits EXACTLY at `required_slot` with the
    /// matching seed. The gate any authoritative read of the accumulator-at-a-slot consults — fails closed
    /// on unsealed / seed mismatch / lagging / advanced-past (an unhandled rollback).
    pub fn verify_ready_at(
        &self,
        required_slot: SlotNo,
        expected_seed_slot: SlotNo,
    ) -> Result<(), AccumulatorReadinessError> {
        let (_seed, advanced) = self.readiness_inputs(expected_seed_slot)?;
        if advanced < required_slot.0 {
            return Err(AccumulatorReadinessError::Lagging {
                advanced,
                required: required_slot.0,
            });
        }
        if advanced > required_slot.0 {
            return Err(AccumulatorReadinessError::Ahead {
                advanced,
                required: required_slot.0,
            });
        }
        Ok(())
    }

    /// CONWAY-PROPOSAL-DEPOSIT-EXPIRY S2 (absent ≠ empty): require that the sealed bootstrap baseline
    /// carries the imported governance state. A Conway+ bootstrap baseline with `gov_state = None`
    /// PREDATES the governance-proposal import (a pre-v6 store); it must NEVER be loaded as "zero
    /// proposals" — a missing imported set is not an empty set. Fail closed; re-bootstrap to upgrade.
    /// The warm-start path consults this BEFORE operating on a recovered store.
    pub fn verify_governance_imported(&self) -> Result<(), AccumulatorReadinessError> {
        let read = |e: EpochAccumulatorStoreError| AccumulatorReadinessError::Read(format!("{e:?}"));
        let txn = self.db.begin_read().map_err(|e| read(rerr(e)))?;
        let meta = txn.open_table(META_TABLE).map_err(|e| read(rerr(e)))?;
        let blob = meta
            .get(BOOTSTRAP_BLOB_KEY)
            .map_err(|e| read(rerr(e)))?
            .map(|v| v.value().to_vec())
            .ok_or(AccumulatorReadinessError::Unsealed)?;
        let acc = decode_epoch_accumulator(&blob)
            .map_err(|e| AccumulatorReadinessError::Read(format!("{e:?}")))?;
        if (acc.era as u8) >= (CardanoEra::Conway as u8) && acc.gov_state.is_none() {
            return Err(AccumulatorReadinessError::GovernanceImportRequired { era_tag: acc.era as u64 });
        }
        Ok(())
    }

    /// Shared readiness prelude: the sealed seed (lineage-checked) + the last advanced slot, fail-closed.
    fn readiness_inputs(
        &self,
        expected_seed_slot: SlotNo,
    ) -> Result<(u64, u64), AccumulatorReadinessError> {
        let seed = self
            .seed_slot()
            .map_err(|e| AccumulatorReadinessError::Read(format!("{e:?}")))?
            .ok_or(AccumulatorReadinessError::Unsealed)?;
        if seed.0 != expected_seed_slot.0 {
            return Err(AccumulatorReadinessError::SeedMismatch {
                seed: seed.0,
                expected: expected_seed_slot.0,
            });
        }
        let advanced = self
            .last_advanced_slot()
            .map_err(|e| AccumulatorReadinessError::Read(format!("{e:?}")))?
            .ok_or(AccumulatorReadinessError::Unsealed)?;
        Ok((seed.0, advanced.0))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_ledger::epoch_accumulator::EpochAccumulator;
    use ade_types::tx::Coin;
    use ade_types::{CardanoEra, EpochNo};
    use tempfile::TempDir;

    fn store(tmp: &TempDir) -> EpochAccumulatorStore {
        EpochAccumulatorStore::open(&tmp.path().join("acc.redb")).unwrap()
    }

    /// A bootstrap accumulator and a clearly-distinct advanced one (different epoch + reserves), so the
    /// round-trip / reset assertions are exact (EpochAccumulator derives PartialEq).
    // v2: a persisted Conway accumulator carries the deposit params (production seeds them from the
    // certified snapshot); the codec fails closed on None, so a round-tripped fixture must set Some.
    // gov_state stays None here so `governance_import_gate_rejects_absent_but_allows_empty` still isolates
    // the gov-import readiness check.
    fn conway_deposits() -> ade_ledger::pparams::ConwayOnlyDepositParams {
        ade_ledger::pparams::ConwayOnlyDepositParams {
            drep_deposit: Coin(500_000_000),
            gov_action_deposit: Coin(100_000_000_000),
            drep_activity: 20,
        }
    }
    fn acc_bootstrap() -> EpochAccumulator {
        let mut a = EpochAccumulator::new(CardanoEra::Conway);
        a.conway_deposit_params = Some(conway_deposits());
        a
    }
    fn acc_advanced() -> EpochAccumulator {
        let mut a = EpochAccumulator::new(CardanoEra::Conway);
        a.epoch_state.epoch = EpochNo(9);
        a.epoch_state.reserves = Coin(12_345);
        a.conway_deposit_params = Some(conway_deposits());
        a
    }

    #[test]
    fn unsealed_store_reads_empty_and_advance_fails_closed() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        assert!(!s.is_complete().unwrap());
        assert!(s.load_current().unwrap().is_none());
        assert!(s.last_advanced_slot().unwrap().is_none());
        assert!(s.seed_slot().unwrap().is_none());
        let err = s.advance(&acc_advanced(), SlotNo(10), BlockNo(1), Hash32([0x0A; 32])).unwrap_err();
        assert!(matches!(err, EpochAccumulatorStoreError::NotSealed));
        assert_eq!(
            s.verify_advanced_through(SlotNo(10), SlotNo(0)),
            Err(AccumulatorReadinessError::Unsealed)
        );
    }

    /// CONWAY-PROPOSAL-DEPOSIT-EXPIRY S2 (absent ≠ empty): a Conway bootstrap baseline that PREDATES the
    /// governance import (`gov_state = None`) is rejected — re-bootstrap required — while a v6 baseline
    /// with an EMPTY-but-present gov_state passes. The missing import must never masquerade as "zero
    /// proposals".
    #[test]
    fn governance_import_gate_rejects_absent_but_allows_empty() {
        use ade_ledger::state::ConwayGovState;

        // Absent: a pre-v6 Conway bootstrap (gov_state = None) -> re-bootstrap required.
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        let boot_none = acc_bootstrap();
        assert!(boot_none.gov_state.is_none());
        s.seal_bootstrap(&boot_none, SlotNo(100)).unwrap();
        assert_eq!(
            s.verify_governance_imported(),
            Err(AccumulatorReadinessError::GovernanceImportRequired {
                era_tag: CardanoEra::Conway as u64
            }),
            "an absent imported gov state predates the import — re-bootstrap required",
        );

        // Empty-but-present: a v6 bootstrap whose imported proposal set is empty -> OK (absent != empty).
        let tmp2 = TempDir::new().unwrap();
        let s2 = store(&tmp2);
        let mut boot_empty = acc_bootstrap();
        boot_empty.gov_state = Some(ConwayGovState {
            prev_pparam_action: ade_ledger::state::PreviousPParamAction::Unversioned,
            proposals: Vec::new(),
            committee: std::collections::BTreeMap::new(),
            committee_quorum: (1, 1),
            drep_expiry: std::collections::BTreeMap::new(),
            gov_action_lifetime: 0,
            vote_delegations: std::collections::BTreeMap::new(),
            pool_voting_thresholds: Vec::new(),
            drep_voting_thresholds: Vec::new(),
            committee_hot_keys: std::collections::BTreeMap::new(),
            num_dormant: ade_ledger::state::DormantEpochs::Unversioned,
        });
        s2.seal_bootstrap(&boot_empty, SlotNo(100)).unwrap();
        assert_eq!(
            s2.verify_governance_imported(),
            Ok(()),
            "an empty-but-PRESENT imported gov state is valid (absent != empty)",
        );
    }

    #[test]
    fn seal_advance_reset_round_trip_is_exact() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        let boot = acc_bootstrap();
        let adv = acc_advanced();

        s.seal_bootstrap(&boot, SlotNo(100)).unwrap();
        assert!(s.is_complete().unwrap());
        assert_eq!(s.seed_slot().unwrap(), Some(SlotNo(100)));
        assert_eq!(s.load_current().unwrap(), Some((SlotNo(100), boot.clone())));

        s.advance(&adv, SlotNo(200), BlockNo(20), Hash32([0xC8; 32])).unwrap();
        assert_eq!(s.last_advanced_slot().unwrap(), Some(SlotNo(200)));
        assert_eq!(s.load_current().unwrap(), Some((SlotNo(200), adv.clone())));

        // Reorg reset → back to the sealed bootstrap baseline + seed slot (no inverse mutation).
        s.reset_to_bootstrap().unwrap();
        assert_eq!(s.last_advanced_slot().unwrap(), Some(SlotNo(100)));
        assert_eq!(s.load_current().unwrap(), Some((SlotNo(100), boot)));
        // The seed lineage is untouched by the reset.
        assert_eq!(s.seed_slot().unwrap(), Some(SlotNo(100)));
    }

    // ----- S5: the LastAdvancedPoint lineage anchor -----

    #[test]
    fn lineage_anchor_absent_until_advance_then_round_trips() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();
        // A freshly sealed (not-yet-advanced) store is NOT lineage-certified — the anchor is absent.
        assert_eq!(s.last_advanced_point().unwrap(), None);
        // advance writes the anchor atomically with the blob + LAST_SLOT.
        s.advance(&acc_advanced(), SlotNo(200), BlockNo(20), Hash32([0xC8; 32]))
            .unwrap();
        assert_eq!(
            s.last_advanced_point().unwrap(),
            Some(LastAdvancedPoint {
                slot: SlotNo(200),
                block_no: BlockNo(20),
                header_hash: Hash32([0xC8; 32]),
            })
        );
    }

    #[test]
    fn reset_to_bootstrap_clears_the_lineage_anchor() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();
        s.advance(&acc_advanced(), SlotNo(200), BlockNo(20), Hash32([0xC8; 32]))
            .unwrap();
        assert!(s.last_advanced_point().unwrap().is_some());
        // A reset leaves the accumulator UNCERTIFIED — the anchor is cleared until a canonical re-fold
        // re-writes it; recovery must never read a reset store as lineage authority.
        s.reset_to_bootstrap().unwrap();
        assert_eq!(s.last_advanced_point().unwrap(), None);
        // The next advance re-certifies at its own point.
        s.advance(&acc_advanced(), SlotNo(150), BlockNo(15), Hash32([0x96; 32]))
            .unwrap();
        assert_eq!(
            s.last_advanced_point().unwrap().map(|p| p.block_no),
            Some(BlockNo(15))
        );
    }

    #[test]
    fn malformed_lineage_anchor_bytes_fail_closed() {
        // The fixed-width decoder rejects a wrong-length anchor (corruption) -> typed failure.
        for bad in [vec![0u8; 47], vec![0u8; 49], Vec::new()] {
            assert!(matches!(
                LastAdvancedPoint::decode(&bad),
                Err(EpochAccumulatorStoreError::CorruptLastAdvancedPoint)
            ));
        }
        // A well-formed 48-byte anchor round-trips exactly (slot ++ block_no ++ header_hash).
        let p = LastAdvancedPoint {
            slot: SlotNo(7),
            block_no: BlockNo(3),
            header_hash: Hash32([0x11; 32]),
        };
        assert_eq!(LastAdvancedPoint::decode(&p.encode()).unwrap(), p);
    }

    #[test]
    fn advance_is_strictly_forward() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();
        s.advance(&acc_advanced(), SlotNo(150), BlockNo(15), Hash32([0x96; 32])).unwrap();
        // Backward / equal advance is fail-closed (a reorg must reset, not advance backward).
        for slot in [SlotNo(150), SlotNo(149), SlotNo(100)] {
            let err = s.advance(&acc_advanced(), slot, BlockNo(1), Hash32([0x01; 32])).unwrap_err();
            assert!(
                matches!(
                    err,
                    EpochAccumulatorStoreError::NonMonotonicAdvance { last: 150, .. }
                ),
                "expected NonMonotonicAdvance for {slot:?}, got {err:?}"
            );
        }
    }

    #[test]
    fn readiness_gate_fails_closed() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();
        s.advance(&acc_advanced(), SlotNo(200), BlockNo(20), Hash32([0xC8; 32])).unwrap();

        // Correct lineage, at-or-beyond.
        assert!(s.verify_advanced_through(SlotNo(200), SlotNo(100)).is_ok());
        assert!(s.verify_advanced_through(SlotNo(150), SlotNo(100)).is_ok());
        assert!(s.verify_ready_at(SlotNo(200), SlotNo(100)).is_ok());

        // Wrong seed lineage.
        assert_eq!(
            s.verify_advanced_through(SlotNo(200), SlotNo(999)),
            Err(AccumulatorReadinessError::SeedMismatch {
                seed: 100,
                expected: 999
            })
        );
        // Lagging (required beyond advanced).
        assert_eq!(
            s.verify_advanced_through(SlotNo(300), SlotNo(100)),
            Err(AccumulatorReadinessError::Lagging {
                advanced: 200,
                required: 300
            })
        );
        // Exact gate rejects an over-advance (unhandled rollback).
        assert_eq!(
            s.verify_ready_at(SlotNo(150), SlotNo(100)),
            Err(AccumulatorReadinessError::Ahead {
                advanced: 200,
                required: 150
            })
        );
    }

    #[test]
    fn reopen_recovers_durable_state() {
        let tmp = TempDir::new().unwrap();
        let adv = acc_advanced();
        {
            let s = store(&tmp);
            s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();
            s.advance(&adv, SlotNo(200), BlockNo(20), Hash32([0xC8; 32])).unwrap();
        }
        // A fresh handle on the same path recovers the durable current state (restart).
        let s2 = EpochAccumulatorStore::open(&tmp.path().join("acc.redb")).unwrap();
        assert!(s2.is_complete().unwrap());
        assert_eq!(s2.load_current().unwrap(), Some((SlotNo(200), adv)));
        assert_eq!(s2.seed_slot().unwrap(), Some(SlotNo(100)));
    }

    /// DC-EPOCH-22 (#2b-ii): the durable boundary-mark witness round-trips — absent → bind → read → rebind
    /// (overwrites) → clear → absent. The witness carries ONLY the point + lineage `(slot, hash)`; the mark
    /// value is re-derived from the lineage-matched checkpoint, never stored here.
    #[test]
    fn boundary_mark_witness_bind_read_clear_round_trip() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();
        assert_eq!(s.boundary_mark_binding().unwrap(), None);

        let h = Hash32([0xAB; 32]);
        s.bind_boundary_mark(SlotNo(500), &h).unwrap();
        assert_eq!(s.boundary_mark_binding().unwrap(), Some((SlotNo(500), h)));

        // A later bind overwrites (the next boundary's point + lineage).
        let h2 = Hash32([0xCD; 32]);
        s.bind_boundary_mark(SlotNo(600), &h2).unwrap();
        assert_eq!(s.boundary_mark_binding().unwrap(), Some((SlotNo(600), h2)));

        // Clear once the cross consumed it — then idempotent.
        s.clear_boundary_mark().unwrap();
        assert_eq!(s.boundary_mark_binding().unwrap(), None);
        s.clear_boundary_mark().unwrap();
        assert_eq!(s.boundary_mark_binding().unwrap(), None);
    }

    /// DC-EPOCH-22 (#2b-ii): binding fails closed on an unsealed store (the seal must precede any binding).
    #[test]
    fn boundary_mark_bind_requires_sealed() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        let err = s
            .bind_boundary_mark(SlotNo(500), &Hash32([1; 32]))
            .unwrap_err();
        assert!(matches!(err, EpochAccumulatorStoreError::NotSealed));
    }

    /// DC-EPOCH-22 (#2b-ii): a reorg reset DROPS the pending binding — its lineage no longer holds, so the
    /// rematerialized chain must re-bind at its own boundary point (never reuse a stale, reorged mark).
    #[test]
    fn reset_to_bootstrap_drops_the_boundary_mark_binding() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();
        s.advance(&acc_advanced(), SlotNo(200), BlockNo(20), Hash32([0xC8; 32])).unwrap();
        s.bind_boundary_mark(SlotNo(199), &Hash32([0xEE; 32]))
            .unwrap();
        assert!(s.boundary_mark_binding().unwrap().is_some());

        s.reset_to_bootstrap().unwrap();
        assert_eq!(s.boundary_mark_binding().unwrap(), None);
    }

    /// DC-EPOCH-22 (#2b-ii): the binding is DURABLE — persisted before the cross, it survives a restart
    /// (crash between bind and cross → the binding is recovered, the cross re-derives + crosses).
    #[test]
    fn boundary_mark_binding_survives_reopen() {
        let tmp = TempDir::new().unwrap();
        {
            let s = store(&tmp);
            s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();
            s.bind_boundary_mark(SlotNo(500), &Hash32([0x77; 32]))
                .unwrap();
        }
        let s2 = EpochAccumulatorStore::open(&tmp.path().join("acc.redb")).unwrap();
        assert_eq!(
            s2.boundary_mark_binding().unwrap(),
            Some((SlotNo(500), Hash32([0x77; 32])))
        );
    }
}
