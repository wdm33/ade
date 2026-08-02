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
use ade_ledger::frozen_leadership::{
    decode_frozen_leadership, encode_frozen_leadership, FrozenLeadershipError,
    FrozenLeadershipPoolDistr, FROZEN_LEADERSHIP_SCHEMA_VERSION,
};
use ade_types::{BlockNo, CardanoEra, EpochNo, Hash32, SlotNo};
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

/// LIVE-LEDGER-EPOCH-TRANSITION S4-pre-1b: the store-level LEADERSHIP CERTIFICATION marker — 4 BE bytes
/// holding `FROZEN_LEADERSHIP_SCHEMA_VERSION`. PRESENT-and-`== 5` ⇒ the store is leadership-certified; ABSENT
/// or `!= 5` ⇒ a legacy (v4 / pre-S4-pre) store that was never leadership-certified. The accumulator BLOB
/// codec is UNCHANGED (still v4-decodable), so the non-authority observe-only follow still reads existing
/// stores; ONLY the leadership authority read (`leadership_authority_for_epoch`) gates on this marker + object.
const LEADERSHIP_SCHEMA_KEY: &str = "leadership_schema_version";

/// ACCUMULATOR-REFOLD-BOUND S1: the SETTLED rewind point — a rolling snapshot of the accumulator at
/// a point the chain can no longer retract (older than `k`). A reorg rewinds HERE instead of to the
/// bootstrap baseline, bounding the refold to ~`k` slots rather than "everything since bootstrap"
/// (which grew without bound with node uptime: measured 26.6 min at 85,690 slots out, and rising).
///
/// Three keys move together in one commit and are only ever read together. Absent on an older store
/// => the rewind falls back to `reset_to_bootstrap`, i.e. exactly the pre-slice behaviour, so this
/// addition cannot regress an existing deployment.
/// A DOUBLE BUFFER is required, not a single snapshot. The accumulator's current state tracks the
/// tip, so it is never itself settled; a single snapshot refreshed from `current` would be unusable
/// at steady state, and one refreshed only while catching up would age without bound. So the current
/// state is STAGED as `pending`, PROMOTED to `settled` once the tip has advanced `k` blocks past it,
/// and a fresh `pending` staged. `settled` is then always between `k` and `2k` blocks old: always
/// usable, and the refold it implies is bounded by `2k` regardless of uptime.
const SETTLED_BLOB_KEY: &str = "settled_blob";
const SETTLED_POINT_KEY: &str = "settled_point";
const SETTLED_LEADERSHIP_KEY: &str = "settled_leadership";
const PENDING_BLOB_KEY: &str = "pending_settled_blob";
const PENDING_POINT_KEY: &str = "pending_settled_point";
const PENDING_LEADERSHIP_KEY: &str = "pending_settled_leadership";

/// Deterministic length-prefixed encoding of the epoch-indexed leadership table:
/// `count(4 BE) ++ [epoch(8 BE) ++ len(4 BE) ++ blob]*`. Entries are written in redb's ascending
/// key order, so the encoding is canonical for a given table.
fn encode_leadership_entries(entries: &[(u64, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + entries.len() * 48);
    out.extend_from_slice(&(entries.len() as u32).to_be_bytes());
    for (epoch, blob) in entries {
        out.extend_from_slice(&epoch.to_be_bytes());
        out.extend_from_slice(&(blob.len() as u32).to_be_bytes());
        out.extend_from_slice(blob);
    }
    out
}

fn decode_leadership_entries(
    raw: &[u8],
) -> Result<Vec<(u64, Vec<u8>)>, EpochAccumulatorStoreError> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let bad = || EpochAccumulatorStoreError::Decode("settled leadership table".to_string());
    if raw.len() < 4 {
        return Err(bad());
    }
    let count = u32::from_be_bytes(raw[0..4].try_into().map_err(|_| bad())?) as usize;
    let mut out = Vec::with_capacity(count);
    let mut off = 4usize;
    for _ in 0..count {
        if off + 12 > raw.len() {
            return Err(bad());
        }
        let epoch = u64::from_be_bytes(raw[off..off + 8].try_into().map_err(|_| bad())?);
        let len = u32::from_be_bytes(raw[off + 8..off + 12].try_into().map_err(|_| bad())?) as usize;
        off += 12;
        if off + len > raw.len() {
            return Err(bad());
        }
        out.push((epoch, raw[off..off + len].to_vec()));
        off += len;
    }
    if off != raw.len() {
        return Err(bad());
    }
    Ok(out)
}
/// LIVE-LEDGER-EPOCH-TRANSITION S4-0: the CURRENT leadership authority, EPOCH-INDEXED — `target_leadership_epoch`
/// (u64) -> canonically-encoded `FrozenLeadershipPoolDistr`. Production reads leadership for an EXACT epoch
/// (`leadership_authority_for_epoch`), NEVER "the current object": a boundary freeze produces `nesPd_{target+1}`,
/// so while operating in epoch E the store may already hold `nesPd_{E+1}`. Seeded at bootstrap with the
/// bootstrap-certified epochs; a boundary freeze inserts by `target_leadership_epoch`; a reset restores this
/// table from the BOOTSTRAP table.
const CURRENT_LEADERSHIP_BY_EPOCH: TableDefinition<u64, &[u8]> =
    TableDefinition::new("current_leadership_by_epoch");
/// LIVE-LEDGER-EPOCH-TRANSITION S4-0: the IMMUTABLE BOOTSTRAP leadership authority, EPOCH-INDEXED — the
/// bootstrap-certified initial condition (`nesPd_1338` from the seed record, `nesPd_1339` from the imported
/// MARK snapshot), never overwritten by a boundary freeze. `reset_to_bootstrap` restores CURRENT := BOOTSTRAP
/// (this table), so a reset rewinding the accumulator can never leave a stale post-boundary leadership object.
const BOOTSTRAP_LEADERSHIP_BY_EPOCH: TableDefinition<u64, &[u8]> =
    TableDefinition::new("bootstrap_leadership_by_epoch");

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

/// LIVE-LEDGER-EPOCH-TRANSITION S4-pre-1b — why a store cannot be read as leadership authority (or seeded as
/// one). Every variant FAILS CLOSED: a legacy / torn / corrupt / wrong-lineage leadership certification can
/// never load as authority. This is DISTINCT from the accumulator-blob readiness gate: the non-authority
/// observe-only follow still decodes an existing v4 accumulator blob; only the leadership authority path is
/// gated here.
#[derive(Debug)]
pub enum LeadershipAuthorityError {
    /// The store carries no leadership-schema marker, or one that is not `FROZEN_LEADERSHIP_SCHEMA_VERSION`:
    /// a legacy v4 / pre-S4-pre store that was never leadership-certified (or a future schema this binary
    /// does not certify). Re-bootstrap to leadership-certify. `found` is the marker version if present.
    OldAccumulatorSchemaNotLeadershipCertified { found: Option<u32> },
    /// The leadership-schema marker claims certification but the frozen-leadership object blob is absent (a
    /// torn / corrupt certified store — the two are written in one commit, so this is corruption).
    MissingFrozenLeadershipDistr,
    /// The persisted frozen-leadership object failed canonical decode (a corrupt store).
    MalformedFrozenLeadershipDistr(FrozenLeadershipError),
    /// Bootstrap seeding: the seed record's frozen source point does not match the expected bootstrap point —
    /// a wrong-lineage seed record. Leadership is NEVER seeded from a foreign point.
    FrozenLeadershipSourceMismatch {
        expected_slot: u64,
        record_slot: u64,
        expected_hash: Hash32,
        record_hash: Hash32,
    },
    /// Bootstrap seeding: the freshly built frozen-leadership object failed its encode→decode self-check
    /// before sealing (a canonical-encoding invariant violation). A non-canonical authority object is NEVER
    /// persisted.
    FrozenLeadershipCanonicalDecodeFailed(FrozenLeadershipError),
    /// LIVE-LEDGER-EPOCH-TRANSITION S4-0: an EXACT epoch-indexed read found no leadership object sealed for the
    /// requested `target_leadership_epoch`. Fail closed — production reads leadership for an EXACT epoch, never
    /// "the latest / current object" or a nearest neighbour.
    LeadershipEpochNotSealed { requested: u64 },
    /// S4-0: the object sealed under the requested epoch key carries a DIFFERENT `target_leadership_epoch` — a
    /// corrupt / mis-keyed store. Fail closed (an exact read must return exactly the requested epoch).
    LeadershipEpochMismatch { requested: u64, found: u64 },
    /// S4-0 bootstrap seeding: two bootstrap leadership objects claim the SAME `target_leadership_epoch`. The
    /// bootstrap-certified initial condition must have one object per epoch.
    DuplicateBootstrapLeadershipEpoch { epoch: u64 },
    /// S4-L2: a PROMOTION read found the epoch's leadership object present in `current` but ALSO in `bootstrap`
    /// — a bootstrap-IMPORTED object (seed / seed+1), which is L1-initial-view-only and NOT promotion-certified.
    /// The promotion path (candidate epochs beyond the bootstrap bridge) requires a NATIVE boundary freeze
    /// (`current` present, `bootstrap` absent). Fail closed — a bootstrap import is never promoted.
    NotPromotionCertified { epoch: u64 },
    /// An underlying store (redb) failure while reading or sealing the leadership object.
    Store(EpochAccumulatorStoreError),
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

    /// LIVE-LEDGER-EPOCH-TRANSITION S4-pre-2: a BOUNDARY advance — the accumulator blob + `LAST_SLOT` + the
    /// `LastAdvancedPoint` anchor + the boundary-frozen CURRENT leadership object + the v5 marker are written
    /// in ONE redb commit. This is the ATOMIC authoritative advance unit: the store NEVER durably exposes a
    /// new accumulator epoch without its matching frozen leadership, or a frozen leadership without the
    /// accumulator boundary transition that produced it (no torn split-authority state — no pending/complete
    /// marker needed because it is one commit). The within-epoch path keeps using `advance` (no leadership
    /// change); only a boundary cross uses this. Fail-closed if unsealed or non-forward.
    pub fn advance_with_current_leadership(
        &self,
        acc: &EpochAccumulator,
        slot: SlotNo,
        block_no: BlockNo,
        header_hash: Hash32,
        leadership: &FrozenLeadershipPoolDistr,
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
        let lead_blob = encode_frozen_leadership(leadership);
        let version = FROZEN_LEADERSHIP_SCHEMA_VERSION.to_be_bytes();
        let txn = self.db.begin_write().map_err(rerr)?;
        {
            let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
            meta.insert(CURRENT_BLOB_KEY, blob.as_slice())
                .map_err(rerr)?;
            meta.insert(LAST_SLOT_KEY, slot_bytes.as_slice())
                .map_err(rerr)?;
            meta.insert(LAST_ADVANCED_POINT_KEY, anchor.as_slice())
                .map_err(rerr)?;
            meta.insert(LEADERSHIP_SCHEMA_KEY, version.as_slice())
                .map_err(rerr)?;
        }
        {
            // The boundary-frozen CURRENT leadership rides the SAME commit as the accumulator advance, keyed by
            // its target epoch (never a torn accumulator/leadership pair).
            let mut cur = txn.open_table(CURRENT_LEADERSHIP_BY_EPOCH).map_err(rerr)?;
            cur.insert(leadership.target_leadership_epoch.0, lead_blob.as_slice())
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
            // ACCUMULATOR-REFOLD-BOUND S1: a bootstrap reset discards BOTH rewind buffers -- the
            // staged one (abandoned chain) and the settled one (we are going further back than it
            // anyway), so nothing survives a bootstrap reset as a rewind target.
            let _ = meta.remove(PENDING_BLOB_KEY).map_err(rerr)?;
            let _ = meta.remove(PENDING_POINT_KEY).map_err(rerr)?;
            let _ = meta.remove(PENDING_LEADERSHIP_KEY).map_err(rerr)?;
            let _ = meta.remove(SETTLED_BLOB_KEY).map_err(rerr)?;
            let _ = meta.remove(SETTLED_POINT_KEY).map_err(rerr)?;
            let _ = meta.remove(SETTLED_LEADERSHIP_KEY).map_err(rerr)?;
            // S5: a reset leaves the accumulator at the seed baseline but NOT lineage-certified — clear the
            // anchor. Recovery treats the cleared store as uncertified until a successful canonical re-fold
            // re-writes a fresh LastAdvancedPoint; it never trusts a reset store as lineage authority.
            let _ = meta.remove(LAST_ADVANCED_POINT_KEY).map_err(rerr)?;
            // S4-0: leadership is EPOCH-INDEXED + RECURRENT (a boundary freeze inserts nesPd_{target+1} keyed by
            // epoch). A reset that rewinds the accumulator to the bootstrap baseline MUST restore CURRENT :=
            // BOOTSTRAP (the immutable bootstrap-certified epochs), NEVER preserve stale post-boundary objects
            // (which would outrun the refolded accumulator, violating replay equivalence). Copy the BOOTSTRAP
            // leadership table over CURRENT; if there is NO bootstrap leadership (an uncertified store) clear
            // CURRENT + the marker so no stray object survives a reset as authority.
            let boot_entries: Vec<(u64, Vec<u8>)> = {
                let boot = txn.open_table(BOOTSTRAP_LEADERSHIP_BY_EPOCH).map_err(rerr)?;
                let mut v = Vec::new();
                for r in boot.iter().map_err(rerr)? {
                    let (k, val) = r.map_err(rerr)?;
                    v.push((k.value(), val.value().to_vec()));
                }
                v
            };
            {
                let mut cur = txn.open_table(CURRENT_LEADERSHIP_BY_EPOCH).map_err(rerr)?;
                let cur_keys: Vec<u64> = {
                    let mut ks = Vec::new();
                    for r in cur.iter().map_err(rerr)? {
                        ks.push(r.map_err(rerr)?.0.value());
                    }
                    ks
                };
                for k in cur_keys {
                    let _ = cur.remove(k).map_err(rerr)?;
                }
                for (e, blob) in &boot_entries {
                    cur.insert(*e, blob.as_slice()).map_err(rerr)?;
                }
            }
            if boot_entries.is_empty() {
                let _ = meta.remove(LEADERSHIP_SCHEMA_KEY).map_err(rerr)?;
            }
        }
        txn.commit().map_err(rerr)?;
        Ok(())
    }

    /// ACCUMULATOR-REFOLD-BOUND S1: snapshot the CURRENT accumulator as the SETTLED rewind point.
    ///
    /// The caller refreshes this only once the current state is at least `k` behind the durable tip
    /// (INV-AR-1) — this method does not know the tip and does not police settledness; it records
    /// what the caller certifies as settled.
    ///
    /// Only a lineage-CERTIFIED current state may become a rewind target: an uncertified store is
    /// exactly what a prior reset leaves behind, and promoting one would launder an unverified state
    /// into a trusted baseline. Returns `false` (a no-op) when unsealed or uncertified.
    ///
    /// The leadership table is snapshotted WITH the blob so a later rewind can restore the exact
    /// pair (INV-AR-3). Leadership only ever changes at a boundary crossing
    /// (`advance_with_current_leadership`, the single recurrent writer), so this snapshot is simply
    /// whatever the last boundary at-or-before the point left.
    /// `tip_block_no` is the durable tip's height and `security_param` is `k` in BLOCKS — both in
    /// block units, so this needs no active-slot-coefficient assumption (comparing slots would).
    pub fn roll_settled_rewind_point(
        &self,
        tip_block_no: BlockNo,
        security_param: u64,
    ) -> Result<bool, EpochAccumulatorStoreError> {
        if !self.is_complete()? {
            return Ok(false);
        }
        let Some(point) = self.last_advanced_point()? else {
            return Ok(false);
        };
        // PROMOTE first: a staged point that the tip has now outrun by >= k is settled — no
        // admissible reorg can reach it — so it becomes the rewind target. Then stage the current
        // state in its place.
        let staged = {
            let txn = self.db.begin_read().map_err(rerr)?;
            let meta = txn.open_table(META_TABLE).map_err(rerr)?;
            match meta.get(PENDING_POINT_KEY).map_err(rerr)? {
                Some(v) => Some(LastAdvancedPoint::decode(&v.value().to_vec())?),
                None => None,
            }
        };
        let promote = staged
            .as_ref()
            .is_some_and(|p| p.block_no.0.saturating_add(security_param) <= tip_block_no.0);
        // Nothing to do until the staged point has aged past k (it is re-staged only on promotion,
        // so `settled` ends up between k and 2k old).
        if staged.is_some() && !promote {
            return Ok(false);
        }
        let txn = self.db.begin_write().map_err(rerr)?;
        if promote {
            let (b, p, l) = {
                let meta = txn.open_table(META_TABLE).map_err(rerr)?;
                let b = meta
                    .get(PENDING_BLOB_KEY)
                    .map_err(rerr)?
                    .map(|v| v.value().to_vec());
                let p = meta
                    .get(PENDING_POINT_KEY)
                    .map_err(rerr)?
                    .map(|v| v.value().to_vec());
                let l = meta
                    .get(PENDING_LEADERSHIP_KEY)
                    .map_err(rerr)?
                    .map(|v| v.value().to_vec())
                    .unwrap_or_default();
                (b, p, l)
            };
            if let (Some(b), Some(p)) = (b, p) {
                let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
                meta.insert(SETTLED_BLOB_KEY, b.as_slice()).map_err(rerr)?;
                meta.insert(SETTLED_POINT_KEY, p.as_slice()).map_err(rerr)?;
                meta.insert(SETTLED_LEADERSHIP_KEY, l.as_slice())
                    .map_err(rerr)?;
            }
        }
        {
            let cur_blob = {
                let meta = txn.open_table(META_TABLE).map_err(rerr)?;
                let got = meta
                    .get(CURRENT_BLOB_KEY)
                    .map_err(rerr)?
                    .map(|v| v.value().to_vec());
                got.ok_or(EpochAccumulatorStoreError::Missing(CURRENT_BLOB_KEY))?
            };
            let lead: Vec<(u64, Vec<u8>)> = {
                let cur = txn.open_table(CURRENT_LEADERSHIP_BY_EPOCH).map_err(rerr)?;
                let mut v = Vec::new();
                for r in cur.iter().map_err(rerr)? {
                    let (k, val) = r.map_err(rerr)?;
                    v.push((k.value(), val.value().to_vec()));
                }
                v
            };
            let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
            meta.insert(PENDING_BLOB_KEY, cur_blob.as_slice())
                .map_err(rerr)?;
            meta.insert(PENDING_POINT_KEY, point.encode().as_slice())
                .map_err(rerr)?;
            meta.insert(
                PENDING_LEADERSHIP_KEY,
                encode_leadership_entries(&lead).as_slice(),
            )
            .map_err(rerr)?;
        }
        txn.commit().map_err(rerr)?;
        Ok(promote)
    }

    /// ACCUMULATOR-REFOLD-BOUND S1: the persisted SETTLED rewind point, if any. The caller checks
    /// settledness (`slot + k <= tip`) and lineage (the hash still resolves canonically) BEFORE
    /// calling [`Self::reset_to_settled`] — INV-AR-1 / INV-AR-2.
    pub fn settled_rewind_point(
        &self,
    ) -> Result<Option<LastAdvancedPoint>, EpochAccumulatorStoreError> {
        let txn = self.db.begin_read().map_err(rerr)?;
        let meta = txn.open_table(META_TABLE).map_err(rerr)?;
        let raw = match meta.get(SETTLED_POINT_KEY).map_err(rerr)? {
            None => return Ok(None),
            Some(v) => v.value().to_vec(),
        };
        Ok(Some(LastAdvancedPoint::decode(&raw)?))
    }

    /// ACCUMULATOR-REFOLD-BOUND S1: reorg reset to the SETTLED rewind point instead of the bootstrap
    /// baseline, bounding the post-rollback refold to ~`k` slots instead of "everything since
    /// bootstrap" (INV-AR-5).
    ///
    /// Identical in kind to [`Self::reset_to_bootstrap`] — same three guarantees, different baseline:
    ///   * `CURRENT := SETTLED` (blob + slot),
    ///   * the pending boundary-mark binding is dropped (its lineage no longer holds, DC-EPOCH-22),
    ///   * `LAST_ADVANCED_POINT` is cleared, so the store is UNCERTIFIED until a canonical re-fold
    ///     rewrites it (INV-AR-4 — a rewound store is never lineage authority),
    ///   * `CURRENT_LEADERSHIP := SETTLED_LEADERSHIP`, so no sealed leadership object can outrun the
    ///     refolded accumulator (INV-AR-3 — the `reset_to_bootstrap` guarantee, generalised).
    ///
    /// Returns `false` (store untouched) when no settled point is recorded, so the caller falls back
    /// to `reset_to_bootstrap` and an older store simply behaves as it did pre-slice.
    pub fn reset_to_settled(&self) -> Result<bool, EpochAccumulatorStoreError> {
        if !self.is_complete()? {
            return Err(EpochAccumulatorStoreError::NotSealed);
        }
        // Read the settled triple first: absent => no-op, so the caller can fall back cleanly.
        let (blob, point, lead) = {
            let txn = self.db.begin_read().map_err(rerr)?;
            let meta = txn.open_table(META_TABLE).map_err(rerr)?;
            let Some(blob) = meta
                .get(SETTLED_BLOB_KEY)
                .map_err(rerr)?
                .map(|v| v.value().to_vec())
            else {
                return Ok(false);
            };
            let Some(praw) = meta
                .get(SETTLED_POINT_KEY)
                .map_err(rerr)?
                .map(|v| v.value().to_vec())
            else {
                return Ok(false);
            };
            let lraw = meta
                .get(SETTLED_LEADERSHIP_KEY)
                .map_err(rerr)?
                .map(|v| v.value().to_vec())
                .unwrap_or_default();
            (
                blob,
                LastAdvancedPoint::decode(&praw)?,
                decode_leadership_entries(&lraw)?,
            )
        };

        let txn = self.db.begin_write().map_err(rerr)?;
        {
            let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
            meta.insert(CURRENT_BLOB_KEY, blob.as_slice())
                .map_err(rerr)?;
            meta.insert(LAST_SLOT_KEY, point.slot.0.to_be_bytes().as_slice())
                .map_err(rerr)?;
            let _ = meta.remove(PENDING_BOUNDARY_MARK_KEY).map_err(rerr)?;
            let _ = meta.remove(LAST_ADVANCED_POINT_KEY).map_err(rerr)?;
            // ACCUMULATOR-REFOLD-BOUND S1: the STAGED point was taken on the chain we are
            // abandoning -- drop it so it can never be promoted into a rewind target. The
            // already-SETTLED point is >= k old and is deliberately kept (INV-AR-1: no admissible
            // reorg reaches it), so a second rollback still has a bounded target.
            let _ = meta.remove(PENDING_BLOB_KEY).map_err(rerr)?;
            let _ = meta.remove(PENDING_POINT_KEY).map_err(rerr)?;
            let _ = meta.remove(PENDING_LEADERSHIP_KEY).map_err(rerr)?;
            {
                let mut cur = txn.open_table(CURRENT_LEADERSHIP_BY_EPOCH).map_err(rerr)?;
                let cur_keys: Vec<u64> = {
                    let mut ks = Vec::new();
                    for r in cur.iter().map_err(rerr)? {
                        ks.push(r.map_err(rerr)?.0.value());
                    }
                    ks
                };
                for k in cur_keys {
                    let _ = cur.remove(k).map_err(rerr)?;
                }
                for (e, b) in &lead {
                    cur.insert(*e, b.as_slice()).map_err(rerr)?;
                }
            }
            if lead.is_empty() {
                let _ = meta.remove(LEADERSHIP_SCHEMA_KEY).map_err(rerr)?;
            }
        }
        txn.commit().map_err(rerr)?;
        Ok(true)
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

    // ----- LIVE-LEDGER-EPOCH-TRANSITION S4-0: the EPOCH-INDEXED leadership authority -----

    /// Read the store-level leadership marker, fail-closed. `Ok(())` iff a v5 marker is present. Shared prelude
    /// of every epoch-indexed authority read + seal — a legacy non-indexed store (no marker) is refused.
    fn require_leadership_certified(&self, txn: &redb::ReadTransaction) -> Result<(), LeadershipAuthorityError> {
        // A store that never wrote the meta table (a fresh / never-sealed store) is not leadership-certified —
        // fail closed as such, NOT as a redb error.
        let meta = match txn.open_table(META_TABLE) {
            Ok(t) => t,
            Err(_) => {
                return Err(LeadershipAuthorityError::OldAccumulatorSchemaNotLeadershipCertified {
                    found: None,
                })
            }
        };
        let version = match meta
            .get(LEADERSHIP_SCHEMA_KEY)
            .map_err(|e| LeadershipAuthorityError::Store(rerr(e)))?
        {
            Some(v) if v.value().len() == 4 => {
                let mut b = [0u8; 4];
                b.copy_from_slice(v.value());
                u32::from_be_bytes(b)
            }
            _ => {
                return Err(LeadershipAuthorityError::OldAccumulatorSchemaNotLeadershipCertified {
                    found: None,
                })
            }
        };
        if version != FROZEN_LEADERSHIP_SCHEMA_VERSION {
            return Err(LeadershipAuthorityError::OldAccumulatorSchemaNotLeadershipCertified {
                found: Some(version),
            });
        }
        Ok(())
    }

    /// The FAIL-CLOSED, EXACT epoch-indexed leadership authority read (S4-0). Returns the certified
    /// `FrozenLeadershipPoolDistr` whose `target_leadership_epoch == epoch`, and NOTHING else — there is NO
    /// "latest / current object" or nearest-neighbour behaviour (a boundary freeze produces `nesPd_{target+1}`,
    /// so the store may hold epochs ahead of the one the node is in; production must ask for the EXACT epoch it
    /// is validating/forging). Typed refusals: `OldAccumulatorSchemaNotLeadershipCertified` (legacy / no v5
    /// marker), `LeadershipEpochNotSealed` (no object for this epoch), `MalformedFrozenLeadershipDistr` (corrupt
    /// blob), `LeadershipEpochMismatch` (mis-keyed store).
    pub fn leadership_authority_for_epoch(
        &self,
        epoch: EpochNo,
    ) -> Result<FrozenLeadershipPoolDistr, LeadershipAuthorityError> {
        let txn = self
            .db
            .begin_read()
            .map_err(|e| LeadershipAuthorityError::Store(rerr(e)))?;
        self.require_leadership_certified(&txn)?;
        let cur = txn
            .open_table(CURRENT_LEADERSHIP_BY_EPOCH)
            .map_err(|e| LeadershipAuthorityError::Store(rerr(e)))?;
        let blob = match cur
            .get(epoch.0)
            .map_err(|e| LeadershipAuthorityError::Store(rerr(e)))?
        {
            Some(v) => v.value().to_vec(),
            None => return Err(LeadershipAuthorityError::LeadershipEpochNotSealed { requested: epoch.0 }),
        };
        let distr = decode_frozen_leadership(&blob)
            .map_err(LeadershipAuthorityError::MalformedFrozenLeadershipDistr)?;
        if distr.target_leadership_epoch != epoch {
            return Err(LeadershipAuthorityError::LeadershipEpochMismatch {
                requested: epoch.0,
                found: distr.target_leadership_epoch.0,
            });
        }
        Ok(distr)
    }

    /// LIVE-LEDGER-EPOCH-TRANSITION S4-L2: the SOLE reader for PROMOTION (candidate epochs BEYOND the bootstrap
    /// bridge). Returns the frozen leadership authority for `epoch` ONLY if it is PROMOTION-CERTIFIED — present in
    /// `current` AND ABSENT from `bootstrap` (a NATIVE boundary freeze, never a bootstrap import), with
    /// `target_leadership_epoch == epoch`. Bootstrap-imported epochs (seed / seed+1, which live in BOTH tables)
    /// fail closed `NotPromotionCertified`: they are L1-initial/warm-view only and must never be promoted. The S4
    /// promotion path uses THIS reader; L1's initial/warm view uses the general `leadership_authority_for_epoch`.
    pub fn promotion_leadership_authority_for_epoch(
        &self,
        epoch: EpochNo,
    ) -> Result<FrozenLeadershipPoolDistr, LeadershipAuthorityError> {
        let distr = self.leadership_authority_for_epoch(epoch)?;
        if self
            .bootstrap_frozen_leadership_for_epoch(epoch)
            .map_err(LeadershipAuthorityError::Store)?
            .is_some()
        {
            return Err(LeadershipAuthorityError::NotPromotionCertified { epoch: epoch.0 });
        }
        Ok(distr)
    }

    /// Durably SEAL the CURRENT leadership object for its target epoch (the S4-pre-2 BOUNDARY FREEZE primitive):
    /// `current_leadership_by_epoch[distr.target_leadership_epoch]` + the v5 marker, in ONE redb commit. Keys by
    /// `target_leadership_epoch` and overwrites only that epoch's CURRENT entry; the BOOTSTRAP table is
    /// untouched. The accumulator BLOB is untouched (v4-decodable).
    pub fn seal_current_leadership(
        &self,
        distr: &FrozenLeadershipPoolDistr,
    ) -> Result<(), EpochAccumulatorStoreError> {
        let blob = encode_frozen_leadership(distr);
        let version = FROZEN_LEADERSHIP_SCHEMA_VERSION.to_be_bytes();
        let txn = self.db.begin_write().map_err(rerr)?;
        {
            let mut cur = txn.open_table(CURRENT_LEADERSHIP_BY_EPOCH).map_err(rerr)?;
            cur.insert(distr.target_leadership_epoch.0, blob.as_slice())
                .map_err(rerr)?;
        }
        {
            let mut meta = txn.open_table(META_TABLE).map_err(rerr)?;
            meta.insert(LEADERSHIP_SCHEMA_KEY, version.as_slice())
                .map_err(rerr)?;
        }
        txn.commit().map_err(rerr)?;
        Ok(())
    }

    /// Durably SEAL the BOOTSTRAP leadership epochs (S4-0 bootstrap-certified initial condition): each distr is
    /// written to BOTH `bootstrap_leadership_by_epoch[e]` AND `current_leadership_by_epoch[e]`, plus the v5
    /// marker, in ONE redb commit. Fail-closed BEFORE any write: NO duplicate `target_leadership_epoch`, and
    /// each distr passes an encode→decode canonical self-check. The BOOTSTRAP entries are the stable reset
    /// target (`reset_to_bootstrap` restores CURRENT := BOOTSTRAP); a boundary freeze touches only CURRENT. The
    /// caller (native bootstrap) is responsible for the SOURCE binding of each object it builds.
    pub fn seal_bootstrap_leadership_epochs(
        &self,
        distrs: &[FrozenLeadershipPoolDistr],
    ) -> Result<(), LeadershipAuthorityError> {
        let mut seen: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
        for distr in distrs {
            let e = distr.target_leadership_epoch.0;
            if !seen.insert(e) {
                return Err(LeadershipAuthorityError::DuplicateBootstrapLeadershipEpoch { epoch: e });
            }
            let bytes = encode_frozen_leadership(distr);
            let back = decode_frozen_leadership(&bytes)
                .map_err(LeadershipAuthorityError::FrozenLeadershipCanonicalDecodeFailed)?;
            if &back != distr {
                return Err(LeadershipAuthorityError::FrozenLeadershipCanonicalDecodeFailed(
                    FrozenLeadershipError::NonCanonicalBytes,
                ));
            }
        }
        let version = FROZEN_LEADERSHIP_SCHEMA_VERSION.to_be_bytes();
        let txn = self
            .db
            .begin_write()
            .map_err(|e| LeadershipAuthorityError::Store(rerr(e)))?;
        {
            let mut boot = txn
                .open_table(BOOTSTRAP_LEADERSHIP_BY_EPOCH)
                .map_err(|e| LeadershipAuthorityError::Store(rerr(e)))?;
            let mut cur = txn
                .open_table(CURRENT_LEADERSHIP_BY_EPOCH)
                .map_err(|e| LeadershipAuthorityError::Store(rerr(e)))?;
            for distr in distrs {
                let blob = encode_frozen_leadership(distr);
                let e = distr.target_leadership_epoch.0;
                boot.insert(e, blob.as_slice())
                    .map_err(|e| LeadershipAuthorityError::Store(rerr(e)))?;
                cur.insert(e, blob.as_slice())
                    .map_err(|e| LeadershipAuthorityError::Store(rerr(e)))?;
            }
        }
        {
            let mut meta = txn
                .open_table(META_TABLE)
                .map_err(|e| LeadershipAuthorityError::Store(rerr(e)))?;
            meta.insert(LEADERSHIP_SCHEMA_KEY, version.as_slice())
                .map_err(|e| LeadershipAuthorityError::Store(rerr(e)))?;
        }
        txn.commit()
            .map_err(|e| LeadershipAuthorityError::Store(rerr(e)))?;
        Ok(())
    }

    /// Raw diagnostic accessor: the CURRENT leadership object sealed for `epoch`, or `None`. NOT the fail-closed
    /// authority read (`leadership_authority_for_epoch`). Test / evidence only.
    pub fn frozen_leadership_for_epoch(
        &self,
        epoch: EpochNo,
    ) -> Result<Option<FrozenLeadershipPoolDistr>, EpochAccumulatorStoreError> {
        self.read_leadership_entry(CURRENT_LEADERSHIP_BY_EPOCH, epoch)
    }

    /// Raw diagnostic accessor: the immutable BOOTSTRAP leadership object sealed for `epoch`, or `None`.
    pub fn bootstrap_frozen_leadership_for_epoch(
        &self,
        epoch: EpochNo,
    ) -> Result<Option<FrozenLeadershipPoolDistr>, EpochAccumulatorStoreError> {
        self.read_leadership_entry(BOOTSTRAP_LEADERSHIP_BY_EPOCH, epoch)
    }

    fn read_leadership_entry(
        &self,
        table: TableDefinition<u64, &[u8]>,
        epoch: EpochNo,
    ) -> Result<Option<FrozenLeadershipPoolDistr>, EpochAccumulatorStoreError> {
        let txn = self.db.begin_read().map_err(rerr)?;
        let t = match txn.open_table(table) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        match t.get(epoch.0).map_err(rerr)? {
            Some(v) => decode_frozen_leadership(v.value())
                .map(Some)
                .map_err(|e| EpochAccumulatorStoreError::Decode(format!("{e:?}"))),
            None => Ok(None),
        }
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
    use ade_ledger::frozen_leadership::LeadershipPoolEntry;
    use ade_types::tx::Coin;
    use ade_types::{CardanoEra, EpochNo, Hash28};
    use std::collections::BTreeMap;
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

    // ----- S4-pre-1b: the durable leadership authority -----

    fn frozen_distr() -> FrozenLeadershipPoolDistr {
        let mut pools = BTreeMap::new();
        pools.insert(
            Hash28([0x01; 28]),
            LeadershipPoolEntry { active_stake: 1_000, vrf_keyhash: Hash32([0x07; 32]) },
        );
        // A zero-stake registered pool — carried for leadership-set membership.
        pools.insert(
            Hash28([0x05; 28]),
            LeadershipPoolEntry { active_stake: 0, vrf_keyhash: Hash32([0x08; 32]) },
        );
        pools.insert(
            Hash28([0xAA; 28]),
            LeadershipPoolEntry { active_stake: 999_999, vrf_keyhash: Hash32([0x09; 32]) },
        );
        FrozenLeadershipPoolDistr {
            target_leadership_epoch: EpochNo(576),
            source_slot: SlotNo(576 * 432_000 + 12_345),
            source_hash: Hash32([0x66; 32]),
            source_checkpoint_commitment: Hash32([0x0C; 32]),
            pools,
        }
    }

    /// `frozen_distr` re-labelled for a given target epoch (distinct source slot so equal-content distrs at
    /// different epochs are still distinguishable).
    fn distr_at(epoch: u64) -> FrozenLeadershipPoolDistr {
        let mut d = frozen_distr();
        d.target_leadership_epoch = EpochNo(epoch);
        d.source_slot = SlotNo(epoch * 1_000);
        d
    }

    /// Inject a raw v5 marker (optional) + raw CURRENT-leadership-table entries at `path` (a fresh redb), then
    /// drop the handle. Used to exercise the fail-closed corruption branches the atomic public API cannot
    /// produce (wrong-version marker, torn / malformed / mis-keyed object).
    fn write_raw_leadership(path: &Path, marker: Option<u32>, entries: &[(u64, Vec<u8>)]) {
        let db = Database::create(path).unwrap();
        let txn = db.begin_write().unwrap();
        if let Some(m) = marker {
            let mut meta = txn.open_table(META_TABLE).unwrap();
            meta.insert(LEADERSHIP_SCHEMA_KEY, m.to_be_bytes().as_slice()).unwrap();
        }
        {
            let mut cur = txn.open_table(CURRENT_LEADERSHIP_BY_EPOCH).unwrap();
            for (e, blob) in entries {
                cur.insert(*e, blob.as_slice()).unwrap();
            }
        }
        txn.commit().unwrap();
    }

    // ACCUMULATOR-REFOLD-BOUND S1 — the bounded settled rewind point.

    /// CE-AR-2 / INV-AR-1: a staged point is NOT a rewind target until the tip has outrun it by
    /// `k` blocks. Promoting earlier would expose a point an admissible reorg could still reach.
    #[test]
    fn settled_point_is_only_promoted_once_k_blocks_settled() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        const K: u64 = 10;
        s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();
        s.advance_with_current_leadership(
            &acc_advanced(),
            SlotNo(200),
            BlockNo(20),
            Hash32([0xA1; 32]),
            &distr_at(576),
        )
        .unwrap();

        // First roll STAGES the current point; nothing is settled yet.
        assert!(!s.roll_settled_rewind_point(BlockNo(20), K).unwrap());
        assert!(s.settled_rewind_point().unwrap().is_none());

        // Tip only 5 blocks past the staged point -> still not settled.
        assert!(!s.roll_settled_rewind_point(BlockNo(25), K).unwrap());
        assert!(s.settled_rewind_point().unwrap().is_none());

        // Tip k blocks past -> promote. The rewind target is the STAGED point (block 20).
        assert!(s.roll_settled_rewind_point(BlockNo(30), K).unwrap());
        let sp = s.settled_rewind_point().unwrap().expect("promoted");
        assert_eq!(sp.slot, SlotNo(200));
        assert_eq!(sp.block_no, BlockNo(20));
        assert_eq!(sp.header_hash, Hash32([0xA1; 32]));
    }

    /// INV-AR-5 (bounded refold): the settled point never falls further than `2k` behind the tip,
    /// however long the node runs. That bound IS the slice — the pre-slice behaviour rewound to the
    /// bootstrap anchor, whose distance grows without limit with uptime (measured 26.6 min of
    /// refold at 85,690 slots out, and still rising).
    ///
    /// The mechanism: `pending` is re-staged only on promotion, so `settled` is at least `k` old
    /// (never reachable by an admissible reorg) and at most `2k` old (the next promotion fires as
    /// soon as the newly-staged point itself reaches `k`).
    #[test]
    fn settled_point_never_falls_further_than_2k_behind_the_tip() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        const K: u64 = 10;
        s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();

        let mut worst_age = 0u64;
        // Walk a tip forward far past any single buffer window; the accumulator advances with it.
        for h in 1..=200u64 {
            s.advance_with_current_leadership(
                &acc_advanced(),
                SlotNo(1_000 + h * 10),
                BlockNo(h),
                Hash32([(h & 0xff) as u8; 32]),
                &distr_at(576),
            )
            .unwrap();
            let _ = s.roll_settled_rewind_point(BlockNo(h), K).unwrap();
            if let Some(sp) = s.settled_rewind_point().unwrap() {
                let age = h.saturating_sub(sp.block_no.0);
                worst_age = worst_age.max(age);
                assert!(
                    age >= K,
                    "a settled point must be at least k={K} old (reorg-unreachable), was {age}"
                );
                assert!(
                    age <= 2 * K,
                    "a settled point must never exceed 2k={} old -- the refold bound; was {age}",
                    2 * K
                );
            }
        }
        // The bound is actually exercised, not vacuously satisfied by never promoting.
        assert!(
            worst_age >= K,
            "the walk must have promoted at least once (worst age {worst_age})"
        );
    }

    /// CE-AR-1 / INV-AR-3 / INV-AR-4: rewinding to the settled point restores the accumulator AND
    /// its leadership pair, and leaves the store UNCERTIFIED (no lineage anchor) exactly as a
    /// bootstrap reset does.
    #[test]
    fn reset_to_settled_restores_pair_and_leaves_store_uncertified() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        const K: u64 = 10;
        s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();
        s.advance_with_current_leadership(
            &acc_advanced(),
            SlotNo(200),
            BlockNo(20),
            Hash32([0xA1; 32]),
            &distr_at(576),
        )
        .unwrap();
        assert!(!s.roll_settled_rewind_point(BlockNo(20), K).unwrap());
        assert!(s.roll_settled_rewind_point(BlockNo(30), K).unwrap());

        // Move on: a LATER boundary seals leadership for a further epoch.
        s.advance_with_current_leadership(
            &acc_advanced(),
            SlotNo(400),
            BlockNo(40),
            Hash32([0xB2; 32]),
            &distr_at(577),
        )
        .unwrap();
        assert!(s.frozen_leadership_for_epoch(EpochNo(577)).unwrap().is_some());

        assert!(s.reset_to_settled().unwrap());

        // Accumulator is back at the settled slot...
        let (slot, _) = s.load_current().unwrap().expect("sealed");
        assert_eq!(slot, SlotNo(200));
        // ...the store is UNCERTIFIED (INV-AR-4)...
        assert!(s.last_advanced_point().unwrap().is_none());
        // ...and no leadership object outruns the rewound accumulator (INV-AR-3): epoch 577 was
        // sealed AFTER the settled point and must not survive.
        assert!(s.frozen_leadership_for_epoch(EpochNo(576)).unwrap().is_some());
        assert!(s.frozen_leadership_for_epoch(EpochNo(577)).unwrap().is_none());
    }

    /// A bootstrap reset discards BOTH rewind buffers -- nothing survives it as a rewind target.
    #[test]
    fn bootstrap_reset_discards_the_settled_rewind_point() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        const K: u64 = 10;
        s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();
        s.advance_with_current_leadership(
            &acc_advanced(),
            SlotNo(200),
            BlockNo(20),
            Hash32([0xA1; 32]),
            &distr_at(576),
        )
        .unwrap();
        assert!(!s.roll_settled_rewind_point(BlockNo(20), K).unwrap());
        assert!(s.roll_settled_rewind_point(BlockNo(30), K).unwrap());
        assert!(s.settled_rewind_point().unwrap().is_some());

        s.reset_to_bootstrap().unwrap();
        assert!(s.settled_rewind_point().unwrap().is_none());
        // And with no settled point, a settled rewind is a no-op -> the caller falls back.
        assert!(!s.reset_to_settled().unwrap());
    }

    /// The leadership snapshot rides the rewind buffer verbatim; a torn/short blob fails closed
    /// rather than silently yielding a partial leadership table.
    #[test]
    fn settled_leadership_encoding_roundtrips_and_fails_closed_when_torn() {
        let entries = vec![
            (576u64, vec![1u8, 2, 3]),
            (577u64, vec![]),
            (578u64, vec![9u8; 40]),
        ];
        let raw = encode_leadership_entries(&entries);
        assert_eq!(decode_leadership_entries(&raw).unwrap(), entries);
        assert_eq!(decode_leadership_entries(&[]).unwrap(), Vec::new());
        // Truncated mid-entry -> Decode fault, never a partial table.
        assert!(decode_leadership_entries(&raw[..raw.len() - 3]).is_err());
        // Trailing garbage -> also refused (the encoding is exact).
        let mut extra = raw.clone();
        extra.push(0xFF);
        assert!(decode_leadership_entries(&extra).is_err());
    }

    #[test]
    fn seal_current_and_read_exact_by_epoch() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        let d = frozen_distr(); // target_leadership_epoch = 576
        s.seal_current_leadership(&d).unwrap();
        // The EXACT epoch read returns the object; a DIFFERENT epoch fails closed (no latest / nearest).
        assert_eq!(s.leadership_authority_for_epoch(EpochNo(576)).unwrap(), d);
        assert_eq!(s.frozen_leadership_for_epoch(EpochNo(576)).unwrap(), Some(d.clone()));
        assert!(matches!(
            s.leadership_authority_for_epoch(EpochNo(999)),
            Err(LeadershipAuthorityError::LeadershipEpochNotSealed { requested: 999 })
        ));
    }

    #[test]
    fn leadership_authority_fails_closed_on_legacy_store() {
        // A store sealed as a bootstrap accumulator but NEVER leadership-certified (an existing v4 store).
        // Non-authority follow still decodes its accumulator blob; the leadership authority path fails closed.
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();
        assert!(s.frozen_leadership_for_epoch(EpochNo(576)).unwrap().is_none());
        assert!(matches!(
            s.leadership_authority_for_epoch(EpochNo(576)),
            Err(LeadershipAuthorityError::OldAccumulatorSchemaNotLeadershipCertified { found: None })
        ));
        // The accumulator blob itself still loads (non-authority observe-only follow is unaffected).
        assert!(s.load_current().unwrap().is_some());
    }

    #[test]
    fn epoch_indexed_leadership_survives_reopen() {
        let tmp = TempDir::new().unwrap();
        let d = frozen_distr();
        {
            let s = store(&tmp);
            s.seal_current_leadership(&d).unwrap();
        }
        // A fresh handle on the same path recovers the durable epoch-indexed leadership authority (restart).
        let s2 = EpochAccumulatorStore::open(&tmp.path().join("acc.redb")).unwrap();
        assert_eq!(s2.leadership_authority_for_epoch(EpochNo(576)).unwrap(), d);
    }

    #[test]
    fn reset_to_bootstrap_restores_only_bootstrap_indexed_leadership() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();
        // Bootstrap seals the certified initial epochs into BOTH bootstrap + current (here 1338 + 1339).
        let boot_1338 = distr_at(1338);
        let boot_1339 = distr_at(1339);
        s.seal_bootstrap_leadership_epochs(&[boot_1338.clone(), boot_1339.clone()]).unwrap();
        assert_eq!(s.leadership_authority_for_epoch(EpochNo(1338)).unwrap(), boot_1338);
        assert_eq!(s.leadership_authority_for_epoch(EpochNo(1339)).unwrap(), boot_1339);

        // Native boundary freezes insert LATER epochs into CURRENT only (1340, 1341); bootstrap stays put.
        let cur_1340 = distr_at(1340);
        let cur_1341 = distr_at(1341);
        s.seal_current_leadership(&cur_1340).unwrap();
        s.advance_with_current_leadership(&acc_advanced(), SlotNo(200), BlockNo(20), Hash32([0xC8; 32]), &cur_1341)
            .unwrap();
        assert_eq!(s.leadership_authority_for_epoch(EpochNo(1340)).unwrap(), cur_1340);
        assert_eq!(s.leadership_authority_for_epoch(EpochNo(1341)).unwrap(), cur_1341);
        assert!(s.bootstrap_frozen_leadership_for_epoch(EpochNo(1340)).unwrap().is_none(), "bootstrap has no native epoch");

        // A reorg reset restores CURRENT := BOOTSTRAP — the native epochs 1340/1341 are DROPPED (they would
        // outrun the refolded accumulator); only the bootstrap-indexed 1338/1339 remain. Bootstrap unchanged.
        s.reset_to_bootstrap().unwrap();
        assert_eq!(s.last_advanced_point().unwrap(), None);
        assert_eq!(s.leadership_authority_for_epoch(EpochNo(1338)).unwrap(), boot_1338);
        assert_eq!(s.leadership_authority_for_epoch(EpochNo(1339)).unwrap(), boot_1339);
        assert!(matches!(
            s.leadership_authority_for_epoch(EpochNo(1340)),
            Err(LeadershipAuthorityError::LeadershipEpochNotSealed { requested: 1340 }),
        ), "the native post-boundary epoch must NOT survive a reset");
        assert!(matches!(
            s.leadership_authority_for_epoch(EpochNo(1341)),
            Err(LeadershipAuthorityError::LeadershipEpochNotSealed { requested: 1341 }),
        ));
    }

    /// The uncertified edge: a store with a CURRENT object but NO bootstrap object (a boundary freeze that
    /// somehow preceded bootstrap — never happens in production) must NOT preserve the stray current across a
    /// reset; the reset clears it + the marker so it can never survive as authority.
    #[test]
    fn reset_clears_current_leadership_when_no_bootstrap_object() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        s.seal_bootstrap(&acc_bootstrap(), SlotNo(100)).unwrap();
        s.seal_current_leadership(&frozen_distr()).unwrap(); // current only, NO bootstrap epochs
        assert!(s.frozen_leadership_for_epoch(EpochNo(576)).unwrap().is_some());
        s.reset_to_bootstrap().unwrap();
        assert!(
            s.frozen_leadership_for_epoch(EpochNo(576)).unwrap().is_none(),
            "a bootstrap-less current object is cleared, not kept"
        );
        assert!(matches!(
            s.leadership_authority_for_epoch(EpochNo(576)),
            Err(LeadershipAuthorityError::OldAccumulatorSchemaNotLeadershipCertified { .. })
        ));
    }

    #[test]
    fn seal_bootstrap_leadership_epochs_rejects_duplicate_epoch() {
        let tmp = TempDir::new().unwrap();
        let s = store(&tmp);
        // Two bootstrap objects claiming the SAME target epoch — fail closed, nothing sealed.
        let a = distr_at(1338);
        let mut b = distr_at(1338);
        b.source_slot = SlotNo(7); // same epoch, different content
        assert!(matches!(
            s.seal_bootstrap_leadership_epochs(&[a, b]),
            Err(LeadershipAuthorityError::DuplicateBootstrapLeadershipEpoch { epoch: 1338 })
        ));
        assert!(s.frozen_leadership_for_epoch(EpochNo(1338)).unwrap().is_none());
        assert!(matches!(
            s.leadership_authority_for_epoch(EpochNo(1338)),
            Err(LeadershipAuthorityError::OldAccumulatorSchemaNotLeadershipCertified { .. }),
        ), "a rejected duplicate seal leaves the store uncertified");
    }

    #[test]
    fn leadership_authority_rejects_wrong_version_marker() {
        // A store whose leadership marker is an OLD schema (4) — fail closed, not silently accepted.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("acc.redb");
        let blob = encode_frozen_leadership(&distr_at(1338));
        write_raw_leadership(&path, Some(4), &[(1338, blob)]);
        let s = EpochAccumulatorStore::open(&path).unwrap();
        assert!(matches!(
            s.leadership_authority_for_epoch(EpochNo(1338)),
            Err(LeadershipAuthorityError::OldAccumulatorSchemaNotLeadershipCertified { found: Some(4) })
        ));
    }

    #[test]
    fn leadership_authority_rejects_missing_epoch_under_valid_marker() {
        // A v5 marker + an object for 1338, but NO object for 1339 — the exact read for 1339 fails closed.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("acc.redb");
        let blob = encode_frozen_leadership(&distr_at(1338));
        write_raw_leadership(&path, Some(FROZEN_LEADERSHIP_SCHEMA_VERSION), &[(1338, blob)]);
        let s = EpochAccumulatorStore::open(&path).unwrap();
        assert!(s.leadership_authority_for_epoch(EpochNo(1338)).is_ok());
        assert!(matches!(
            s.leadership_authority_for_epoch(EpochNo(1339)),
            Err(LeadershipAuthorityError::LeadershipEpochNotSealed { requested: 1339 })
        ));
    }

    #[test]
    fn leadership_authority_rejects_wrong_epoch_object() {
        // A v5 marker + an object whose target epoch (1338) does NOT match the key it is stored under (1339) —
        // a mis-keyed / corrupt store fails closed (the exact read must return exactly the requested epoch).
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("acc.redb");
        let blob = encode_frozen_leadership(&distr_at(1338)); // target_leadership_epoch = 1338
        write_raw_leadership(&path, Some(FROZEN_LEADERSHIP_SCHEMA_VERSION), &[(1339, blob)]);
        let s = EpochAccumulatorStore::open(&path).unwrap();
        assert!(matches!(
            s.leadership_authority_for_epoch(EpochNo(1339)),
            Err(LeadershipAuthorityError::LeadershipEpochMismatch { requested: 1339, found: 1338 })
        ));
    }

    #[test]
    fn leadership_authority_rejects_malformed_object() {
        // A v5 marker over a corrupt object blob — fail closed (canonical decode rejects it).
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("acc.redb");
        write_raw_leadership(
            &path,
            Some(FROZEN_LEADERSHIP_SCHEMA_VERSION),
            &[(1338, vec![0xFF, 0xFF, 0xFF])],
        );
        let s = EpochAccumulatorStore::open(&path).unwrap();
        assert!(matches!(
            s.leadership_authority_for_epoch(EpochNo(1338)),
            Err(LeadershipAuthorityError::MalformedFrozenLeadershipDistr(_))
        ));
        // S4-L2: the promotion reader propagates the decode failure as the SAME typed terminal (never a
        // fabricated/empty object) -- the frozen-promotion authority path fails closed on corruption.
        assert!(matches!(
            s.promotion_leadership_authority_for_epoch(EpochNo(1338)),
            Err(LeadershipAuthorityError::MalformedFrozenLeadershipDistr(_))
        ));
    }
}
