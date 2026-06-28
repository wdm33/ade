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
use ade_types::SlotNo;
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

    /// Advance the current accumulator to `slot`. The blob + `LAST_SLOT` are written in ONE redb commit, so
    /// the stored blob always matches its stored slot. Fail-closed if unsealed or non-forward.
    pub fn advance(
        &self,
        acc: &EpochAccumulator,
        slot: SlotNo,
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
        let txn = self.db.begin_write().map_err(rerr)?;
        {
            let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
            meta.insert(CURRENT_BLOB_KEY, blob.as_slice())
                .map_err(rerr)?;
            meta.insert(LAST_SLOT_KEY, slot_bytes.as_slice())
                .map_err(rerr)?;
        }
        txn.commit().map_err(rerr)?;
        Ok(())
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
    fn acc_bootstrap() -> EpochAccumulator {
        EpochAccumulator::new(CardanoEra::Conway)
    }
    fn acc_advanced() -> EpochAccumulator {
        let mut a = EpochAccumulator::new(CardanoEra::Conway);
        a.epoch_state.epoch = EpochNo(9);
        a.epoch_state.reserves = Coin(12_345);
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
        let err = s.advance(&acc_advanced(), SlotNo(10)).unwrap_err();
        assert!(matches!(err, EpochAccumulatorStoreError::NotSealed));
        assert_eq!(
            s.verify_advanced_through(SlotNo(10), SlotNo(0)),
            Err(AccumulatorReadinessError::Unsealed)
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

        s.advance(&adv, SlotNo(200)).unwrap();
        assert_eq!(s.last_advanced_slot().unwrap(), Some(SlotNo(200)));
        assert_eq!(s.load_current().unwrap(), Some((SlotNo(200), adv.clone())));

        // Reorg reset → back to the sealed bootstrap baseline + seed slot (no inverse mutation).
        s.reset_to_bootstrap().unwrap();
        assert_eq!(s.last_advanced_slot().unwrap(), Some(SlotNo(100)));
        assert_eq!(s.load_current().unwrap(), Some((SlotNo(100), boot)));
        // The seed lineage is untouched by the reset.
        assert_eq!(s.seed_slot().unwrap(), Some(SlotNo(100)));
    }

    #[test]
    fn advance_is_strictly_forward() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();
        s.advance(&acc_advanced(), SlotNo(150)).unwrap();
        // Backward / equal advance is fail-closed (a reorg must reset, not advance backward).
        for slot in [SlotNo(150), SlotNo(149), SlotNo(100)] {
            let err = s.advance(&acc_advanced(), slot).unwrap_err();
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
        s.advance(&acc_advanced(), SlotNo(200)).unwrap();

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
            s.advance(&adv, SlotNo(200)).unwrap();
        }
        // A fresh handle on the same path recovers the durable current state (restart).
        let s2 = EpochAccumulatorStore::open(&tmp.path().join("acc.redb")).unwrap();
        assert!(s2.is_complete().unwrap());
        assert_eq!(s2.load_current().unwrap(), Some((SlotNo(200), adv)));
        assert_eq!(s2.seed_slot().unwrap(), Some(SlotNo(100)));
    }
}
