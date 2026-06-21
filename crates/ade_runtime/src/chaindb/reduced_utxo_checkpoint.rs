// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! EPOCH-CONSENSUS-VIEW S3b-1 (DC-EVIEW-04) — the DURABLE reduced-UTxO checkpoint.
//!
//! A disk-backed redb store of the reduced UTxO `TxIn → (Coin, ReducedStakeRef)`
//! (the BLUE record + reduction live in `ade_ledger::reduced_utxo`). It is the
//! "minimal native state" (Option B): the single ledger authority's own reduced-UTxO
//! projection — a GREEN durable CACHE of a BLUE-derivable projection, reconstructible
//! by replay if lost/corrupt, NEVER authority and NEVER on the live follow/forge path
//! (the live producer stays `track_utxo=false`; this is built/advanced lazily off the
//! per-block path).
//!
//! Crash-safety: `build_from` clears any prior partial build, writes all entries
//! (durable redb commits), computes the checkpoint fingerprint over the entries in
//! `TxIn` order, then writes the completeness marker LAST in a separate durable
//! commit. A SIGKILL before the marker leaves an INCOMPLETE checkpoint
//! (`is_complete() == false`) that the caller rebuilds — a partial build is NEVER
//! mistaken for a complete one.
//!
//! Replay-equivalence: the fingerprint is a hash chain over the canonical records
//! (`encode_reduced_record`) in `TxIn` key order, so two builds from the same reduced
//! UTxO yield a byte-identical checkpoint + fingerprint (DC-WAL-03 lineage).

use std::collections::BTreeMap;
use std::path::Path;

use ade_crypto::blake2b::blake2b_256;
use ade_ledger::reduced_utxo::{encode_reduced_record, ReducedStakeRef};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, TxIn};
use ade_types::{Hash32, SlotNo};
use redb::{Database, ReadableTable, TableDefinition};

const KEY_LEN: usize = 34; // tx_hash(32) ++ index(2 BE)
const REDUCED_TABLE: TableDefinition<&[u8; KEY_LEN], &[u8]> = TableDefinition::new("reduced_utxo");
const META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("reduced_meta");
/// value = fingerprint(32) ++ count(8 BE). Present iff the build completed.
const COMPLETE_KEY: &str = "complete";
/// S3f-4d-mat-2 (DC-EPOCH-11): the slot of the last block the checkpoint advanced over.
/// Durable so the live advancer replays the ChainDB in lockstep and a lagging checkpoint
/// (behind the durable tip) is detectable.
const LAST_SLOT_KEY: &str = "last_advanced_slot";
/// S3f-4d-mat-3 (DC-EPOCH-11): the IMMUTABLE bootstrap reduced records (the seed state),
/// sealed once at build and never advanced. A reorg rollback re-materializes the live table
/// from this baseline (`reset_to_bootstrap`), then the advancer replays the rolled-back chain.
const BOOTSTRAP_TABLE: TableDefinition<&[u8; KEY_LEN], &[u8]> =
    TableDefinition::new("reduced_bootstrap");
/// S3f-4d-mat-3 (DC-EPOCH-11): the IMMUTABLE bootstrap slot (the seed point), distinct from
/// LAST_SLOT (which advances). `reset_to_bootstrap` resets LAST_SLOT back to this.
const SEED_SLOT_KEY: &str = "seed_slot";
const FP_DOMAIN: &[u8] = b"eview-reduced-utxo-checkpoint-v1";

#[derive(Debug)]
pub enum ReducedCheckpointError {
    Redb(String),
    /// The checkpoint has no completeness marker (a crashed / partial build).
    Incomplete,
    /// A stored value could not be decoded (corrupt store).
    Decode,
    /// A per-credential coin sum exceeded u64 (fail-closed; unreachable under the
    /// Cardano max-supply bound, but never silently wrapped).
    Overflow,
}

/// S3f-4d-mat-4 (DC-EPOCH-11): why the live reduced checkpoint is NOT ready to derive an
/// EpochConsensusView at a required slot. EVERY variant fails closed -- a missing / corrupt /
/// lagging / wrong-lineage / overshot checkpoint MUST block view production rather than derive
/// a stake distribution from a stale or wrong-lineage state.
#[derive(Debug, PartialEq, Eq)]
pub enum CheckpointReadinessError {
    /// Reading the checkpoint state failed (redb / decode error -- a corrupt store).
    Read(String),
    /// The checkpoint carries no sealed bootstrap baseline (uninitialised / crashed build).
    Unsealed,
    /// The checkpoint's sealed seed slot does not match the expected bootstrap lineage.
    SeedMismatch { seed: u64, expected: u64 },
    /// The checkpoint has not advanced to the required slot yet (behind the boundary).
    Lagging { advanced: u64, required: u64 },
    /// The checkpoint advanced PAST the required slot (an unhandled rollback / overshoot) --
    /// its state no longer reflects the required slot exactly.
    Ahead { advanced: u64, required: u64 },
}

fn rerr(e: impl std::fmt::Debug) -> ReducedCheckpointError {
    ReducedCheckpointError::Redb(format!("{e:?}"))
}

fn txin_key(txin: &TxIn) -> [u8; KEY_LEN] {
    let mut k = [0u8; KEY_LEN];
    k[..32].copy_from_slice(&txin.tx_hash.0);
    k[32..34].copy_from_slice(&txin.index.to_be_bytes());
    k
}

fn encode_value(coin: Coin, reduced: &ReducedStakeRef) -> Vec<u8> {
    let mut v = Vec::with_capacity(8 + 29);
    v.extend_from_slice(&coin.0.to_be_bytes());
    reduced.encode(&mut v);
    v
}

fn decode_value(bytes: &[u8]) -> Option<(Coin, ReducedStakeRef)> {
    let coin = Coin(u64::from_be_bytes(bytes.get(0..8)?.try_into().ok()?));
    let (reduced, _) = ReducedStakeRef::decode(bytes.get(8..)?)?;
    Some((coin, reduced))
}

/// A durable, disk-backed reduced-UTxO checkpoint.
pub struct ReducedUtxoCheckpoint {
    db: Database,
}

impl ReducedUtxoCheckpoint {
    /// Open (create if absent) the checkpoint at `path`. redb's default `Immediate`
    /// durability (fsync per commit) gives crash-safe commits.
    pub fn open(path: &Path) -> Result<Self, ReducedCheckpointError> {
        let db = Database::create(path).map_err(rerr)?;
        Ok(Self { db })
    }

    /// Build the checkpoint from a reduced UTxO. Idempotent + rebuild-safe: clears any
    /// prior (possibly partial) build first, writes all entries, then writes the
    /// completeness marker LAST. Returns the checkpoint fingerprint.
    pub fn build_from(
        &self,
        reduced: &BTreeMap<TxIn, (Coin, ReducedStakeRef)>,
    ) -> Result<Hash32, ReducedCheckpointError> {
        // (1) fresh start + all entries, in one durable commit. Dropping the marker
        // and the table first guarantees a rebuild discards any prior partial build.
        {
            let txn = self.db.begin_write().map_err(rerr)?;
            {
                let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
                let _ = meta.remove(COMPLETE_KEY).map_err(rerr)?;
            }
            txn.delete_table(REDUCED_TABLE).map_err(rerr)?;
            {
                let mut table = txn.open_table(REDUCED_TABLE).map_err(rerr)?;
                for (txin, (coin, reduced)) in reduced.iter() {
                    table
                        .insert(&txin_key(txin), encode_value(*coin, reduced).as_slice())
                        .map_err(rerr)?;
                }
            }
            txn.commit().map_err(rerr)?;
        }
        // (2) fingerprint + count over the entries in TxIn key order (redb iterates
        // sorted), then (3) the completeness marker LAST in a separate durable commit.
        let (fp, count) = self.compute_fingerprint()?;
        {
            let txn = self.db.begin_write().map_err(rerr)?;
            {
                let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
                let mut marker = Vec::with_capacity(40);
                marker.extend_from_slice(&fp.0);
                marker.extend_from_slice(&count.to_be_bytes());
                meta.insert(COMPLETE_KEY, marker.as_slice()).map_err(rerr)?;
            }
            txn.commit().map_err(rerr)?;
        }
        Ok(fp)
    }

    /// Advance the checkpoint by one block's reduced delta (S3b-2): remove the spent
    /// inputs, insert the produced reduced outputs. This INVALIDATES the completeness
    /// marker (the checkpoint is mid-advance and incomplete) — `finalize` recomputes it
    /// after the whole epoch window. A crash mid-window leaves an INCOMPLETE checkpoint;
    /// because the reduced UTxO is reconstructible by replay (DC-EVIEW-04), recovery is
    /// a rebuild, never a wrong stake snapshot from a partial advance.
    pub fn apply_block_delta(
        &self,
        spent: &[TxIn],
        produced: &[(TxIn, Coin, ReducedStakeRef)],
    ) -> Result<(), ReducedCheckpointError> {
        let txn = self.db.begin_write().map_err(rerr)?;
        {
            let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
            let _ = meta.remove(COMPLETE_KEY).map_err(rerr)?;
        }
        {
            let mut table = txn.open_table(REDUCED_TABLE).map_err(rerr)?;
            for txin in spent {
                let _ = table.remove(&txin_key(txin)).map_err(rerr)?;
            }
            for (txin, coin, reduced) in produced {
                table
                    .insert(&txin_key(txin), encode_value(*coin, reduced).as_slice())
                    .map_err(rerr)?;
            }
        }
        txn.commit().map_err(rerr)?;
        Ok(())
    }

    /// S3f-4d-mat-2 (DC-EPOCH-11): advance the checkpoint by ONE durably-admitted block,
    /// ATOMICALLY applying its reduced delta AND recording `slot` as the last-advanced slot
    /// (a single redb commit, so the checkpoint can never record a slot it did not apply, or
    /// apply a delta whose slot it did not record). Removes the completeness marker (re-
    /// finalize after a window of advances). The caller drives this in strict ChainDB/WAL
    /// order; a missing block leaves a gap the lagging check (DC-EPOCH-11) detects.
    pub fn advance_block(
        &self,
        slot: SlotNo,
        spent: &[TxIn],
        produced: &[(TxIn, Coin, ReducedStakeRef)],
    ) -> Result<(), ReducedCheckpointError> {
        let txn = self.db.begin_write().map_err(rerr)?;
        {
            let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
            let _ = meta.remove(COMPLETE_KEY).map_err(rerr)?;
            let slot_bytes = slot.0.to_be_bytes();
            meta.insert(LAST_SLOT_KEY, slot_bytes.as_slice()).map_err(rerr)?;
        }
        {
            let mut table = txn.open_table(REDUCED_TABLE).map_err(rerr)?;
            for txin in spent {
                let _ = table.remove(&txin_key(txin)).map_err(rerr)?;
            }
            for (txin, coin, reduced) in produced {
                table
                    .insert(&txin_key(txin), encode_value(*coin, reduced).as_slice())
                    .map_err(rerr)?;
            }
        }
        txn.commit().map_err(rerr)?;
        Ok(())
    }

    /// S3f-4d-mat-3 (DC-EPOCH-11): seal the freshly-built checkpoint as the bootstrap baseline.
    /// Copies the seed reduced records into the IMMUTABLE bootstrap table and records the seed
    /// slot as BOTH the resume cursor (LAST_SLOT) and the immutable re-materialization anchor
    /// (SEED_SLOT). Called once at build -- a reorg rollback re-materializes the live table
    /// from this sealed baseline.
    pub fn seal_bootstrap(&self, seed_slot: SlotNo) -> Result<(), ReducedCheckpointError> {
        let txn = self.db.begin_write().map_err(rerr)?;
        {
            let reduced = txn.open_table(REDUCED_TABLE).map_err(rerr)?;
            let mut boot = txn.open_table(BOOTSTRAP_TABLE).map_err(rerr)?;
            // Clear any prior baseline first, so a re-seal cannot accumulate stale rows
            // (idempotent). The cold-bootstrap call graph seals exactly once, but this keeps
            // seal_bootstrap correct independent of that invariant.
            while boot.pop_first().map_err(rerr)?.is_some() {}
            for entry in reduced.iter().map_err(rerr)? {
                let (k, v) = entry.map_err(rerr)?;
                boot.insert(k.value(), v.value()).map_err(rerr)?;
            }
        }
        {
            let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
            let slot_bytes = seed_slot.0.to_be_bytes();
            meta.insert(SEED_SLOT_KEY, slot_bytes.as_slice()).map_err(rerr)?;
            meta.insert(LAST_SLOT_KEY, slot_bytes.as_slice()).map_err(rerr)?;
        }
        txn.commit().map_err(rerr)?;
        Ok(())
    }

    /// S3f-4d-mat-3 (DC-EPOCH-11): re-materialize the live checkpoint to the sealed bootstrap
    /// baseline after a reorg rollback. Clears the live reduced table, copies the immutable
    /// bootstrap records back, and resets LAST_SLOT to SEED_SLOT -- so the advancer then replays
    /// the (rolled-back) ChainDB from seed_slot+1 to the new tip. The reduced delta is NOT
    /// invertible, so re-materializing from the sealed seed state is the only way to restore the
    /// EXACT post-rollback lineage. Fail-closed if the checkpoint was never sealed.
    pub fn reset_to_bootstrap(&self) -> Result<(), ReducedCheckpointError> {
        let seed_slot = self.seed_slot()?.ok_or(ReducedCheckpointError::Incomplete)?;
        let txn = self.db.begin_write().map_err(rerr)?;
        {
            let mut reduced = txn.open_table(REDUCED_TABLE).map_err(rerr)?;
            // clear the live table, then copy the immutable bootstrap records back.
            while reduced.pop_first().map_err(rerr)?.is_some() {}
            let boot = txn.open_table(BOOTSTRAP_TABLE).map_err(rerr)?;
            for entry in boot.iter().map_err(rerr)? {
                let (k, v) = entry.map_err(rerr)?;
                reduced.insert(k.value(), v.value()).map_err(rerr)?;
            }
        }
        {
            let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
            let _ = meta.remove(COMPLETE_KEY).map_err(rerr)?;
            let slot_bytes = seed_slot.0.to_be_bytes();
            meta.insert(LAST_SLOT_KEY, slot_bytes.as_slice()).map_err(rerr)?;
        }
        txn.commit().map_err(rerr)?;
        Ok(())
    }

    /// The immutable bootstrap (seed) slot, or `None` if the checkpoint was never sealed.
    pub fn seed_slot(&self) -> Result<Option<SlotNo>, ReducedCheckpointError> {
        let txn = self.db.begin_read().map_err(rerr)?;
        let meta = match txn.open_table(META_TABLE) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        match meta.get(SEED_SLOT_KEY).map_err(rerr)? {
            Some(v) => {
                let b = v.value();
                if b.len() != 8 {
                    return Err(ReducedCheckpointError::Decode);
                }
                Ok(Some(SlotNo(u64::from_be_bytes(b.try_into().unwrap()))))
            }
            None => Ok(None),
        }
    }

    /// S3f-4d-mat-4 (DC-EPOCH-11): verify the checkpoint is READY to derive an
    /// EpochConsensusView at `required_slot` against the expected bootstrap lineage
    /// (`expected_seed_slot`). The gate the boundary derive consults -- it FAILS CLOSED on any
    /// of: a read error (corrupt store), an unsealed checkpoint, a seed/lineage mismatch, the
    /// checkpoint lagging the required slot, or the checkpoint advanced PAST it (an unhandled
    /// rollback). Ready iff the checkpoint sits EXACTLY at `required_slot` with the matching
    /// seed -- so a missing / corrupt / lagging / wrong-lineage checkpoint can never produce a
    /// view from a stale or wrong state.
    pub fn verify_ready_at(
        &self,
        required_slot: SlotNo,
        expected_seed_slot: SlotNo,
    ) -> Result<(), CheckpointReadinessError> {
        let seed = self
            .seed_slot()
            .map_err(|e| CheckpointReadinessError::Read(format!("{e:?}")))?
            .ok_or(CheckpointReadinessError::Unsealed)?;
        if seed.0 != expected_seed_slot.0 {
            return Err(CheckpointReadinessError::SeedMismatch {
                seed: seed.0,
                expected: expected_seed_slot.0,
            });
        }
        let advanced = self
            .last_advanced_slot()
            .map_err(|e| CheckpointReadinessError::Read(format!("{e:?}")))?
            .ok_or(CheckpointReadinessError::Unsealed)?;
        if advanced.0 < required_slot.0 {
            return Err(CheckpointReadinessError::Lagging {
                advanced: advanced.0,
                required: required_slot.0,
            });
        }
        if advanced.0 > required_slot.0 {
            return Err(CheckpointReadinessError::Ahead {
                advanced: advanced.0,
                required: required_slot.0,
            });
        }
        Ok(())
    }

    /// The slot of the last block the checkpoint advanced over (DC-EPOCH-11), or `None` if it
    /// was only built (never advanced). The live advancer reads this to resume in lockstep.
    pub fn last_advanced_slot(&self) -> Result<Option<SlotNo>, ReducedCheckpointError> {
        let txn = self.db.begin_read().map_err(rerr)?;
        let meta = match txn.open_table(META_TABLE) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        match meta.get(LAST_SLOT_KEY).map_err(rerr)? {
            Some(v) => {
                let b = v.value();
                if b.len() != 8 {
                    return Err(ReducedCheckpointError::Decode);
                }
                Ok(Some(SlotNo(u64::from_be_bytes(b.try_into().unwrap()))))
            }
            None => Ok(None),
        }
    }

    /// Recompute + write the completeness marker after a window of `apply_block_delta`
    /// calls (the durable commit that makes the advanced checkpoint complete). Returns
    /// the new fingerprint.
    pub fn finalize(&self) -> Result<Hash32, ReducedCheckpointError> {
        let (fp, count) = self.compute_fingerprint()?;
        let txn = self.db.begin_write().map_err(rerr)?;
        {
            let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
            let mut marker = Vec::with_capacity(40);
            marker.extend_from_slice(&fp.0);
            marker.extend_from_slice(&count.to_be_bytes());
            meta.insert(COMPLETE_KEY, marker.as_slice()).map_err(rerr)?;
        }
        txn.commit().map_err(rerr)?;
        Ok(fp)
    }

    /// Hash-chain fingerprint over the canonical records in `TxIn` order, + the count.
    fn compute_fingerprint(&self) -> Result<(Hash32, u64), ReducedCheckpointError> {
        let txn = self.db.begin_read().map_err(rerr)?;
        let table = txn.open_table(REDUCED_TABLE).map_err(rerr)?;
        let mut h = blake2b_256(FP_DOMAIN);
        let mut count = 0u64;
        for entry in table.iter().map_err(rerr)? {
            let (k, v) = entry.map_err(rerr)?;
            let key = k.value();
            let mut tx_hash = [0u8; 32];
            tx_hash.copy_from_slice(&key[..32]);
            let index = u16::from_be_bytes([key[32], key[33]]);
            let (coin, reduced) =
                decode_value(v.value()).ok_or(ReducedCheckpointError::Decode)?;
            let txin = TxIn { tx_hash: Hash32(tx_hash), index };
            let record = encode_reduced_record(&txin, coin, &reduced);
            let mut chain = Vec::with_capacity(32 + record.len());
            chain.extend_from_slice(&h.0);
            chain.extend_from_slice(&record);
            h = blake2b_256(&chain);
            count += 1;
        }
        Ok((h, count))
    }

    /// Whether the checkpoint has a completeness marker (a finished build).
    pub fn is_complete(&self) -> Result<bool, ReducedCheckpointError> {
        Ok(self.marker()?.is_some())
    }

    /// The stored checkpoint fingerprint, or `Incomplete` if the build did not finish.
    pub fn fingerprint(&self) -> Result<Hash32, ReducedCheckpointError> {
        let (fp, _) = self.marker()?.ok_or(ReducedCheckpointError::Incomplete)?;
        Ok(fp)
    }

    /// The number of reduced records (from the completeness marker).
    pub fn len(&self) -> Result<u64, ReducedCheckpointError> {
        let (_, count) = self.marker()?.ok_or(ReducedCheckpointError::Incomplete)?;
        Ok(count)
    }

    /// S3c: fold the reduced UTxO into per-base-credential coin sums (only
    /// `Base(cred)` entries contribute at Conway; `NonContributing` is skipped). The
    /// caller (`ade_ledger::reduced_aggregate::aggregate_pool_stake`) groups these by
    /// the delegation map into per-pool stake. Fail-closed on overflow (unreachable
    /// under the max-supply bound) — never a silently wrapped sum.
    pub fn sum_base_credential_stake(
        &self,
    ) -> Result<BTreeMap<StakeCredential, Coin>, ReducedCheckpointError> {
        let txn = self.db.begin_read().map_err(rerr)?;
        let table = txn.open_table(REDUCED_TABLE).map_err(rerr)?;
        let mut sums: BTreeMap<StakeCredential, Coin> = BTreeMap::new();
        for entry in table.iter().map_err(rerr)? {
            let (_, v) = entry.map_err(rerr)?;
            let (coin, reduced) =
                decode_value(v.value()).ok_or(ReducedCheckpointError::Decode)?;
            if let ReducedStakeRef::Base(cred) = reduced {
                let e = sums.entry(cred).or_insert(Coin(0));
                *e = e.checked_add(coin).ok_or(ReducedCheckpointError::Overflow)?;
            }
        }
        Ok(sums)
    }

    /// Resolve a `TxIn` to its `(Coin, ReducedStakeRef)`, or `None` if absent.
    pub fn get(&self, txin: &TxIn) -> Result<Option<(Coin, ReducedStakeRef)>, ReducedCheckpointError> {
        let txn = self.db.begin_read().map_err(rerr)?;
        let table = txn.open_table(REDUCED_TABLE).map_err(rerr)?;
        match table.get(&txin_key(txin)).map_err(rerr)? {
            Some(v) => Ok(Some(decode_value(v.value()).ok_or(ReducedCheckpointError::Decode)?)),
            None => Ok(None),
        }
    }

    /// Test-only: write the entries WITHOUT the completeness marker — exactly the
    /// durable state a crash mid-build leaves (a SIGKILL after the entry commit, before
    /// the marker commit). Used to prove crash-recovery deterministically.
    #[cfg(test)]
    fn write_entries_without_marker_for_test(
        &self,
        reduced: &BTreeMap<TxIn, (Coin, ReducedStakeRef)>,
    ) -> Result<(), ReducedCheckpointError> {
        let txn = self.db.begin_write().map_err(rerr)?;
        {
            let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
            let _ = meta.remove(COMPLETE_KEY).map_err(rerr)?;
        }
        txn.delete_table(REDUCED_TABLE).map_err(rerr)?;
        {
            let mut table = txn.open_table(REDUCED_TABLE).map_err(rerr)?;
            for (txin, (coin, reduced)) in reduced.iter() {
                table
                    .insert(&txin_key(txin), encode_value(*coin, reduced).as_slice())
                    .map_err(rerr)?;
            }
        }
        txn.commit().map_err(rerr)?;
        Ok(())
    }

    fn marker(&self) -> Result<Option<(Hash32, u64)>, ReducedCheckpointError> {
        let txn = self.db.begin_read().map_err(rerr)?;
        let meta = match txn.open_table(META_TABLE) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        match meta.get(COMPLETE_KEY).map_err(rerr)? {
            Some(v) => {
                let b = v.value();
                if b.len() != 40 {
                    return Err(ReducedCheckpointError::Decode);
                }
                let mut fp = [0u8; 32];
                fp.copy_from_slice(&b[..32]);
                let count = u64::from_be_bytes(b[32..40].try_into().unwrap());
                Ok(Some((Hash32(fp), count)))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ade_types::shelley::cert::StakeCredential;
    use ade_types::Hash28;
    use tempfile::TempDir;

    fn txin(h: u8, i: u16) -> TxIn {
        TxIn { tx_hash: Hash32([h; 32]), index: i }
    }
    fn base(fill: u8) -> ReducedStakeRef {
        ReducedStakeRef::Base(StakeCredential::KeyHash(Hash28([fill; 28])))
    }
    fn sample() -> BTreeMap<TxIn, (Coin, ReducedStakeRef)> {
        let mut m = BTreeMap::new();
        m.insert(txin(0x01, 0), (Coin(100), base(0xaa)));
        m.insert(txin(0x01, 1), (Coin(200), ReducedStakeRef::NonContributing));
        m.insert(txin(0x02, 0), (Coin(300), base(0xbb)));
        m
    }

    /// S3f-4d-mat-2 (DC-EPOCH-11): advance_block applies a block's reduced delta AND records
    /// its slot ATOMICALLY (durable across reopen) -- the per-block advance the live ChainDB
    /// replay drives in lockstep.
    #[test]
    fn advance_block_applies_delta_and_records_slot() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("rc.redb");
        let cp = ReducedUtxoCheckpoint::open(&path).unwrap();
        cp.build_from(&sample()).unwrap();
        assert_eq!(cp.last_advanced_slot().unwrap(), None, "built-only -> no advanced slot");
        // advance: spend txin(0x01,0), produce a new base-cred output, at slot 500.
        cp.advance_block(SlotNo(500), &[txin(0x01, 0)], &[(txin(0x05, 0), Coin(999), base(0xcc))])
            .unwrap();
        assert_eq!(cp.last_advanced_slot().unwrap(), Some(SlotNo(500)), "slot recorded");
        assert_eq!(cp.get(&txin(0x01, 0)).unwrap(), None, "spent output removed");
        assert_eq!(
            cp.get(&txin(0x05, 0)).unwrap(),
            Some((Coin(999), base(0xcc))),
            "produced output present"
        );
        // a second advance moves the slot forward.
        cp.advance_block(SlotNo(510), &[], &[]).unwrap();
        assert_eq!(cp.last_advanced_slot().unwrap(), Some(SlotNo(510)));
        // durable across reopen (the slot survives).
        drop(cp);
        let cp2 = ReducedUtxoCheckpoint::open(&path).unwrap();
        assert_eq!(cp2.last_advanced_slot().unwrap(), Some(SlotNo(510)), "slot durable across reopen");
    }

    /// S3f-4d-mat-3 (DC-EPOCH-11): seal the bootstrap baseline, advance the live state, then
    /// reset_to_bootstrap re-materializes EXACTLY the seed state + resets the slot to the seed
    /// slot -- the reorg-rollback recovery (the reduced delta is not invertible, so re-
    /// materialize from the sealed seed is the only faithful restore).
    #[test]
    fn reset_to_bootstrap_re_materializes_seed_state() {
        let tmp = TempDir::new().unwrap();
        let cp = ReducedUtxoCheckpoint::open(&tmp.path().join("rc.redb")).unwrap();
        cp.build_from(&sample()).unwrap();
        cp.seal_bootstrap(SlotNo(279)).unwrap();
        assert_eq!(cp.seed_slot().unwrap(), Some(SlotNo(279)), "seed slot sealed");
        assert_eq!(cp.last_advanced_slot().unwrap(), Some(SlotNo(279)), "resume cursor = seed");
        // advance: mutate the live state (spend a seed output, produce a new one).
        cp.advance_block(SlotNo(300), &[txin(0x01, 0)], &[(txin(0x07, 0), Coin(555), base(0xdd))])
            .unwrap();
        assert_eq!(cp.last_advanced_slot().unwrap(), Some(SlotNo(300)));
        assert_eq!(cp.get(&txin(0x01, 0)).unwrap(), None, "seed output spent");
        assert_eq!(cp.get(&txin(0x07, 0)).unwrap(), Some((Coin(555), base(0xdd))), "new output present");
        // reorg: re-materialize to the sealed seed baseline.
        cp.reset_to_bootstrap().unwrap();
        assert_eq!(cp.last_advanced_slot().unwrap(), Some(SlotNo(279)), "slot reset to seed");
        assert_eq!(
            cp.get(&txin(0x01, 0)).unwrap(),
            Some((Coin(100), base(0xaa))),
            "spent seed output restored"
        );
        assert_eq!(cp.get(&txin(0x07, 0)).unwrap(), None, "post-seed output removed");
        // reset clears the completeness marker (like advance_block); the advancer re-finalizes
        // after replaying. Re-finalize here to confirm the live table is EXACTLY the 3 seed rows.
        cp.finalize().unwrap();
        assert_eq!(cp.len().unwrap(), 3, "live table == the seed state exactly");
    }

    /// S3f-4d-mat-4 (DC-EPOCH-11): verify_ready_at fails closed on every not-ready shape
    /// (unsealed, wrong lineage, lagging, overshot) and admits ONLY an exact-slot, matching-
    /// lineage checkpoint -- the gate that blocks view production from a stale/wrong state.
    #[test]
    fn verify_ready_at_fails_closed_unless_exact_and_lineage_bound() {
        let tmp = TempDir::new().unwrap();
        let cp = ReducedUtxoCheckpoint::open(&tmp.path().join("rc.redb")).unwrap();
        // unsealed (only built) -> fail closed.
        cp.build_from(&sample()).unwrap();
        assert_eq!(cp.verify_ready_at(SlotNo(300), SlotNo(279)), Err(CheckpointReadinessError::Unsealed));
        // seal at seed 279, advance to 300.
        cp.seal_bootstrap(SlotNo(279)).unwrap();
        cp.advance_block(SlotNo(300), &[], &[]).unwrap();
        // exact slot + matching lineage -> READY.
        assert_eq!(cp.verify_ready_at(SlotNo(300), SlotNo(279)), Ok(()));
        // wrong lineage -> SeedMismatch.
        assert_eq!(
            cp.verify_ready_at(SlotNo(300), SlotNo(280)),
            Err(CheckpointReadinessError::SeedMismatch { seed: 279, expected: 280 })
        );
        // required slot ahead of the checkpoint -> Lagging.
        assert_eq!(
            cp.verify_ready_at(SlotNo(310), SlotNo(279)),
            Err(CheckpointReadinessError::Lagging { advanced: 300, required: 310 })
        );
        // checkpoint past the required slot -> Ahead.
        assert_eq!(
            cp.verify_ready_at(SlotNo(290), SlotNo(279)),
            Err(CheckpointReadinessError::Ahead { advanced: 300, required: 290 })
        );
    }

    #[test]
    fn build_then_query_and_complete() {
        let tmp = TempDir::new().unwrap();
        let cp = ReducedUtxoCheckpoint::open(&tmp.path().join("rc.redb")).unwrap();
        let fp = cp.build_from(&sample()).unwrap();
        assert!(cp.is_complete().unwrap());
        assert_eq!(cp.len().unwrap(), 3);
        assert_eq!(cp.fingerprint().unwrap(), fp);
        assert_eq!(cp.get(&txin(0x01, 0)).unwrap(), Some((Coin(100), base(0xaa))));
        assert_eq!(
            cp.get(&txin(0x01, 1)).unwrap(),
            Some((Coin(200), ReducedStakeRef::NonContributing))
        );
        assert_eq!(cp.get(&txin(0x09, 0)).unwrap(), None);
    }

    #[test]
    fn durable_across_reopen() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("rc.redb");
        let fp = {
            let cp = ReducedUtxoCheckpoint::open(&path).unwrap();
            cp.build_from(&sample()).unwrap()
        };
        // reopen: the checkpoint persists, byte-identical fingerprint + entries.
        let cp = ReducedUtxoCheckpoint::open(&path).unwrap();
        assert!(cp.is_complete().unwrap());
        assert_eq!(cp.fingerprint().unwrap(), fp);
        assert_eq!(cp.len().unwrap(), 3);
        assert_eq!(cp.get(&txin(0x02, 0)).unwrap(), Some((Coin(300), base(0xbb))));
    }

    #[test]
    fn replay_equivalent_two_builds_byte_identical() {
        let tmp = TempDir::new().unwrap();
        let cp1 = ReducedUtxoCheckpoint::open(&tmp.path().join("a.redb")).unwrap();
        let cp2 = ReducedUtxoCheckpoint::open(&tmp.path().join("b.redb")).unwrap();
        assert_eq!(
            cp1.build_from(&sample()).unwrap(),
            cp2.build_from(&sample()).unwrap(),
            "two builds from the same reduced UTxO -> identical fingerprint"
        );
    }

    #[test]
    fn fingerprint_changes_with_content() {
        let tmp = TempDir::new().unwrap();
        let cp = ReducedUtxoCheckpoint::open(&tmp.path().join("c.redb")).unwrap();
        let fp_a = cp.build_from(&sample()).unwrap();
        let mut other = sample();
        other.insert(txin(0x03, 0), (Coin(1), base(0xcc)));
        let fp_b = cp.build_from(&other).unwrap();
        assert_ne!(fp_a, fp_b);
        // rebuild discards the prior content (the extra entry is gone).
        let fp_c = cp.build_from(&sample()).unwrap();
        assert_eq!(fp_a, fp_c, "rebuild from the same input reproduces the fingerprint");
        assert_eq!(cp.len().unwrap(), 3);
    }

    // Crash mid-build (entries committed, marker NOT) -> on reopen the partial
    // checkpoint is detected as INCOMPLETE (never mistaken for complete) and a rebuild
    // produces the correct complete checkpoint. The durable redb survives a real
    // SIGKILL mid-commit by its Immediate durability (the same backend DC-EVIEW-01
    // proved under its 1000-kill harness); this proves the completeness-marker
    // recovery semantics deterministically.
    #[test]
    fn crash_mid_build_is_incomplete_then_rebuilds() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("crash.redb");
        let expected = {
            // entries written but the marker NOT (the crash-mid-build state).
            let cp = ReducedUtxoCheckpoint::open(&path).unwrap();
            cp.write_entries_without_marker_for_test(&sample()).unwrap();
            assert!(!cp.is_complete().unwrap(), "partial build must be INCOMPLETE");
            assert!(matches!(cp.fingerprint(), Err(ReducedCheckpointError::Incomplete)));
            drop(cp);
            // a separate clean build gives the expected fingerprint.
            let other = TempDir::new().unwrap();
            ReducedUtxoCheckpoint::open(&other.path().join("e.redb"))
                .unwrap()
                .build_from(&sample())
                .unwrap()
        };
        // reopen the crashed store and rebuild -> complete + the correct fingerprint.
        let cp = ReducedUtxoCheckpoint::open(&path).unwrap();
        assert!(!cp.is_complete().unwrap(), "the persisted partial build is still incomplete after reopen");
        let fp = cp.build_from(&sample()).unwrap();
        assert!(cp.is_complete().unwrap());
        assert_eq!(fp, expected, "rebuild reproduces the canonical fingerprint");
        assert_eq!(cp.len().unwrap(), 3);
    }

    // S3b-2: apply_block_delta removes spent + inserts produced; the checkpoint is
    // INCOMPLETE mid-advance and complete only after finalize().
    #[test]
    fn apply_block_delta_then_finalize() {
        let tmp = TempDir::new().unwrap();
        let cp = ReducedUtxoCheckpoint::open(&tmp.path().join("adv.redb")).unwrap();
        cp.build_from(&sample()).unwrap(); // start: 3 entries (txin 1/0, 1/1, 2/0)
        // a block that spends txin(0x01,0) and produces a new txin(0x05,0).
        cp.apply_block_delta(&[txin(0x01, 0)], &[(txin(0x05, 0), Coin(500), base(0xee))])
            .unwrap();
        assert!(!cp.is_complete().unwrap(), "mid-advance is INCOMPLETE until finalize");
        let fp = cp.finalize().unwrap();
        assert!(cp.is_complete().unwrap());
        assert_eq!(cp.fingerprint().unwrap(), fp);
        assert_eq!(cp.get(&txin(0x01, 0)).unwrap(), None, "spent input removed");
        assert_eq!(cp.get(&txin(0x05, 0)).unwrap(), Some((Coin(500), base(0xee))), "produced output present");
        assert_eq!(cp.len().unwrap(), 3); // -1 spent +1 produced
        // the advanced state equals building from the resulting reduced UTxO directly.
        let mut expected = sample();
        expected.remove(&txin(0x01, 0));
        expected.insert(txin(0x05, 0), (Coin(500), base(0xee)));
        let cp2 = ReducedUtxoCheckpoint::open(&tmp.path().join("exp.redb")).unwrap();
        assert_eq!(fp, cp2.build_from(&expected).unwrap());
    }

    // S3b-2 end-to-end on a REAL Conway block: advancing a fresh checkpoint by the
    // block's reduced_block_delta equals build_from the reduced block UTxO -- proving
    // the reduced_block_delta -> apply_block_delta -> finalize chain on real wire data.
    #[test]
    fn advance_over_real_conway_block_matches_build_from() {
        const RAW: &[u8] =
            include_bytes!("../../../ade_node/tests/fixtures/raw_era_block_conway.cbor");
        let env = ade_codec::cbor::envelope::decode_block_envelope(RAW).unwrap();
        let inner = &RAW[env.block_start..env.block_end];
        let block = ade_codec::conway::decode_conway_block(inner).unwrap().decoded().clone();
        let delta = ade_ledger::reduced_advance::reduced_block_delta(
            &block,
            ade_types::CardanoEra::Conway,
        )
        .unwrap();
        assert!(!delta.produced.is_empty(), "real block produces outputs");

        let tmp = TempDir::new().unwrap();
        let cp = ReducedUtxoCheckpoint::open(&tmp.path().join("adv.redb")).unwrap();
        cp.apply_block_delta(&delta.spent, &delta.produced).unwrap();
        let fp_adv = cp.finalize().unwrap();
        assert!(cp.is_complete().unwrap());

        let mut map = BTreeMap::new();
        for (txin, coin, r) in &delta.produced {
            map.insert(txin.clone(), (*coin, r.clone()));
        }
        let cp2 = ReducedUtxoCheckpoint::open(&tmp.path().join("bld.redb")).unwrap();
        assert_eq!(
            fp_adv,
            cp2.build_from(&map).unwrap(),
            "advance(empty, real block) == build_from(reduced block UTxO)"
        );
    }

    // S3c: sum_base_credential_stake folds only Base(cred) coins (per credential);
    // NonContributing entries (pointer/enterprise/Byron) are skipped.
    #[test]
    fn sum_base_credential_stake_skips_non_contributing() {
        use ade_types::shelley::cert::StakeCredential;
        let tmp = TempDir::new().unwrap();
        let cp = ReducedUtxoCheckpoint::open(&tmp.path().join("sum.redb")).unwrap();
        let cred_a = StakeCredential::KeyHash(Hash28([0xaa; 28]));
        let mut m = BTreeMap::new();
        // two Base entries for the SAME credential (must sum) + one NonContributing.
        m.insert(txin(0x01, 0), (Coin(100), ReducedStakeRef::Base(cred_a.clone())));
        m.insert(txin(0x01, 1), (Coin(40), ReducedStakeRef::Base(cred_a.clone())));
        m.insert(txin(0x02, 0), (Coin(9999), ReducedStakeRef::NonContributing));
        cp.build_from(&m).unwrap();
        let sums = cp.sum_base_credential_stake().unwrap();
        assert_eq!(sums.get(&cred_a), Some(&Coin(140)), "Base coins summed per credential");
        assert_eq!(sums.len(), 1, "NonContributing contributes no credential");
    }

    // A fresh store with no build is INCOMPLETE (not mistaken for an empty-but-complete
    // checkpoint) -- the crash-mid-build recovery signal.
    #[test]
    fn fresh_store_is_incomplete() {
        let tmp = TempDir::new().unwrap();
        let cp = ReducedUtxoCheckpoint::open(&tmp.path().join("d.redb")).unwrap();
        assert!(!cp.is_complete().unwrap());
        assert!(matches!(cp.fingerprint(), Err(ReducedCheckpointError::Incomplete)));
    }
}
