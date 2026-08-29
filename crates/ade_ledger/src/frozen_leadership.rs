// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE self-contained leadership authority (LIVE-LEDGER-EPOCH-TRANSITION S4-pre).
//!
//! `FrozenLeadershipPoolDistr` is cardano's leadership `PoolDistr` (`nesPd`) captured as a self-contained,
//! persistable object: per-pool `(active_stake, vrf_keyhash)` FROZEN at a cardano SNAP boundary. It is the
//! SOLE input to the leadership projection — `to_pool_distr_view` reads stake and VRF DIRECTLY from it, never
//! from the active `cert_state.pool.pools`, the go snapshot, `future_pools`, or the `retiring` map.
//!
//! Why a separate surface (proven, `67890681`): leadership uses SET-derived stake + snapshot-FROZEN pool
//! params/VRF, and includes zero-stake registered pools AND retired/POOLREAP'd pools whose VRF is absent from
//! active state. Deriving leadership from go + active params is a DISPROVEN hypothesis
//! (`consensus_view::from_accumulator_go_active_params_for_test_only`), not an optimization target.

use std::collections::{BTreeMap, BTreeSet};

use ade_codec::cbor::{
    canonical_width, read_array_header, read_bytes, read_map_header, read_uint, write_array_header,
    write_bytes_canonical, write_map_header, write_uint_canonical, ContainerEncoding,
};
use ade_codec::CodecError;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_crypto::blake2b::blake2b_256;
use ade_types::tx::Coin;
use ade_types::{EpochNo, Hash28, Hash32, PoolId, SlotNo};

use crate::consensus_view::{PoolDistrView, PoolEntry};

/// The frozen-leadership canonical schema version. A store carrying a well-formed v6 object is
/// PROMOTION-CERTIFIED (S4-L2: carries `source_checkpoint_commitment`); a v5 / v4 / absent object is not. The
/// accumulator BLOB codec is unchanged (stays v4-decodable) so the non-authority observe-only follow still reads
/// existing stores — only the leadership authority path fails closed when this object is missing/old/malformed.
/// v6 (S4-L2) added `source_checkpoint_commitment` so the promoted candidate authority is FULLY self-contained
/// (leadership + its provenance commitment both from the one frozen object; no window replay, no live-checkpoint
/// lookup at promotion time).
pub const FROZEN_LEADERSHIP_SCHEMA_VERSION: u32 = 7;

/// Outer array: [version, target_leadership_epoch, source_slot, source_hash, source_checkpoint_commitment,
/// total_active_stake, pools-map].
const OUTER_FIELDS: u64 = 7;
/// Per-pool entry array: [active_stake, vrf_keyhash].
const ENTRY_FIELDS: u64 = 2;

/// One pool's frozen leadership slice: the stake and VRF keyhash captured at the leadership freeze point.
/// A pool can leave active state (retire / POOLREAP) yet remain leadership-relevant, so both are captured
/// HERE at freeze time and never re-derived from active params.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeadershipPoolEntry {
    /// Active (leadership) stake in lovelace, frozen at the SNAP boundary.
    pub active_stake: u64,
    /// Registered VRF key *hash*, frozen at the SNAP boundary (survives retirement).
    pub vrf_keyhash: Hash32,
}

/// The self-contained leadership `PoolDistr` authority (cardano `nesPd`). Persisted with the accumulator; the
/// SOLE input to leadership projection. `source_slot` / `source_hash` bind the boundary/point it was frozen
/// at (lineage provenance).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenLeadershipPoolDistr {
    /// The leadership epoch this distribution AUTHORIZES (cardano `nesPd` for this epoch) — i.e. the epoch
    /// whose leader schedule reads it. NOT a source epoch: the point the distribution was frozen AT is carried
    /// separately by `source_slot` / `source_hash`. The store indexes leadership by THIS field
    /// (`leadership_authority_for_epoch(e)` returns the object whose `target_leadership_epoch == e`).
    pub target_leadership_epoch: EpochNo,
    /// The canonical selected point the leadership distribution was frozen at (lineage provenance, NOT the
    /// epoch it authorizes).
    pub source_slot: SlotNo,
    /// Its lineage hash.
    pub source_hash: Hash32,
    /// S4-L2 (v6): the reduced-checkpoint commitment finalized AT `source_slot`/`source_hash` — captured at
    /// freeze time, when the checkpoint is at that exact point. This is the leader schedule's provenance
    /// binding: the promoted candidate authority reads it DIRECTLY (never a live/historical checkpoint lookup or
    /// a window-replay re-materialization), so the frozen object is fully self-contained. It byte-matches the
    /// commitment the retired window-replay path bound for the same source point.
    pub source_checkpoint_commitment: Hash32,
    /// LV-1 (DC-EPOCH-40): the snapshot's TOTAL ACTIVE STAKE — cardano's `pdTotalActiveStake`, folded
    /// over the STAKE (credential) map at freeze time, BEFORE any membership decision.
    ///
    /// `calculatePoolDistr'` computes this from `unStake` (Conway: `sumAllStakeCompact`; master:
    /// `sumAllActiveStake` in `mkSnapShot`) and then merely COPIES it into the `PoolDistr`; its
    /// `includeHash` / `numDelegators > 0` guards filter `unPoolDistr` ONLY, and run after the total
    /// is already fixed. **So which pools appear in `pools` cannot change this number.**
    ///
    /// Ade used to derive it by summing `pools`, which made every pool's sigma a function of the
    /// membership filter. That is wrong in BOTH directions: a pool in the set whose stake cardano
    /// does not count inflates it (sigma low, threshold low, spurious REJECT — the observed preprod
    /// halt), and stake cardano counts behind a pool Ade filtered out deflates it (sigma high,
    /// threshold high, spurious ACCEPT — a silent consensus divergence).
    pub total_active_stake: u64,
    /// Per-pool frozen leadership entry, canonical key order.
    pub pools: BTreeMap<Hash28, LeadershipPoolEntry>,
}

impl FrozenLeadershipPoolDistr {
    /// Project into the leadership [`PoolDistrView`] — stake and VRF read DIRECTLY from this frozen object,
    /// never an active-param / go / `future_pools` / `retiring` lookup. `total_active_stake` = the sum of the
    /// per-pool stakes (zero-stake registered pools contribute 0 but are carried for byte-identity).
    pub fn to_pool_distr_view(&self, asc: ActiveSlotsCoeff) -> PoolDistrView {
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        // LV-1 (DC-EPOCH-40): READ the snapshot total. It is NOT summed from `pools` here and there is
        // deliberately no "sum if it looks unset" fallback -- such a fallback would reintroduce the
        // defect on exactly the objects that need the fix.
        let total_active_stake: u64 = self.total_active_stake;
        for (keyhash, entry) in &self.pools {
            pools.insert(
                keyhash.clone(),
                PoolEntry {
                    active_stake: entry.active_stake,
                    vrf_keyhash: entry.vrf_keyhash.clone(),
                },
            );
        }
        PoolDistrView::new(self.target_leadership_epoch, total_active_stake, asc, pools)
    }

    /// The bootstrap import: build the frozen leadership distribution from the manifest-bound seed consensus
    /// inputs (the named artifact). The seed record's `pool_distribution` IS cardano's leadership `nesPd`
    /// (proven byte-exact vs the reference at bootstrap; the LDAT trace `67890681` confirmed it reproduces
    /// the 659-pool leadership set incl. zero-stake + retired-frozen pools).
    pub fn from_seed_epoch_consensus_inputs(
        record: &crate::seed_consensus_inputs::SeedEpochConsensusInputs,
        source_checkpoint_commitment: Hash32,
    ) -> Self {
        let pools = record
            .pool_distribution
            .iter()
            .map(|(keyhash, entry)| {
                (
                    keyhash.clone(),
                    LeadershipPoolEntry {
                        active_stake: entry.active_stake,
                        vrf_keyhash: entry.vrf_keyhash.clone(),
                    },
                )
            })
            .collect();
        FrozenLeadershipPoolDistr {
            target_leadership_epoch: record.epoch_no,
            source_slot: record.seed_point_slot,
            source_hash: record.seed_point_hash.clone(),
            source_checkpoint_commitment,
            // LV-1 (DC-EPOCH-40): the seed record carries the certified total directly. Never summed.
            total_active_stake: record.total_active_stake,
            pools,
        }
    }

    /// The native boundary freeze (S4-pre-2): build the next epoch's leadership `nesPd` at a self-derived epoch
    /// boundary from the SNAP-frozen inputs — the just-built MARK snapshot's per-pool stake and the
    /// pre-POOLREAP registered pool params' VRF. cardano's `nesPd_{e+1} = calculatePoolDistr(set_{e+1} = M_e)`,
    /// where `M_e`'s frozen params are the active pool params at the boundary into `e`, captured BEFORE POOLREAP
    /// removes retiring pools.
    ///
    /// The leadership pool SET is `numDelegators > 0` — the pools with AT LEAST ONE registered delegator
    /// (`delegated_pools` = the image of the pre-POOLREAP delegation map), intersected with the registered pool
    /// set (`registered_pool_vrfs`). This is the DERIVED `PoolDistr` membership (DC-EPOCH-24), DISTINCT from the
    /// full registered set: a registered pool with NO delegator is NOT in `nesPd` (proven: 703 registered but
    /// 658 in the reference `nesPd`). It INCLUDES zero-stake pools that have a delegator (stake 0) and pools
    /// retiring at this boundary (their VRF still present pre-POOLREAP) — exactly as cardano's `nes[5]` does. A
    /// delegated pool with no registered VRF cannot occur pre-POOLREAP and is excluded (no frozen params). VRF
    /// is read HERE, at capture time — NEVER re-derived from active params at leadership-use time (the disproven
    /// `from_accumulator_go_active_params_for_test_only` hypothesis, DC-EPOCH-25).
    pub fn from_boundary_snapshot(
        epoch: EpochNo,
        source_slot: SlotNo,
        source_hash: Hash32,
        source_checkpoint_commitment: Hash32,
        delegated_pools: &BTreeSet<PoolId>,
        mark_pool_stakes: &BTreeMap<PoolId, Coin>,
        registered_pool_vrfs: &BTreeMap<PoolId, Hash32>,
        // LV-1 (DC-EPOCH-40): the mark's credential-side total, from
        // `StakeSnapshot::total_active_stake` -- captured BEFORE the membership filter below and
        // carried through untouched, so `pools` cannot move it.
        total_active_stake: u64,
    ) -> Self {
        let mut pools: BTreeMap<Hash28, LeadershipPoolEntry> = BTreeMap::new();
        for pool_id in delegated_pools {
            if let Some(vrf_keyhash) = registered_pool_vrfs.get(pool_id) {
                let active_stake = mark_pool_stakes.get(pool_id).map(|c| c.0).unwrap_or(0);
                pools.insert(
                    pool_id.0.clone(),
                    LeadershipPoolEntry { active_stake, vrf_keyhash: vrf_keyhash.clone() },
                );
            }
        }
        FrozenLeadershipPoolDistr {
            target_leadership_epoch: epoch,
            source_slot,
            source_hash,
            source_checkpoint_commitment,
            total_active_stake,
            pools,
        }
    }

    /// The bootstrap import of the seed+1 leadership `nesPd` from the imported MARK snapshot's PoolDistr
    /// (`calculatePoolDistr(ssStakeMark)` = `s1a.mark_pool_distr`). This is the bootstrap-certified initial
    /// condition for `target_leadership_epoch = seed_epoch + 1` — the ONE epoch no native boundary freeze can
    /// produce (the cross into seed+1 freezes `nesPd_{seed+2}`), so it is imported verbatim as the seed window
    /// already serves it (the bridge). `mark_pool_distr` is pool -> (active_stake, VRF keyhash).
    pub fn from_mark_pool_distr(
        target_leadership_epoch: EpochNo,
        source_slot: SlotNo,
        source_hash: Hash32,
        source_checkpoint_commitment: Hash32,
        mark_pool_distr: &BTreeMap<PoolId, (u64, Hash32)>,
        // LV-1 (DC-EPOCH-40): the IMPORTED mark snapshot's credential-side total
        // (`s1a.snapshots.mark.total_active_stake()`). Never summed from `mark_pool_distr`, which is
        // already the filtered PoolDistr.
        total_active_stake: u64,
    ) -> Self {
        let pools = mark_pool_distr
            .iter()
            .map(|(pid, (active_stake, vrf))| {
                (
                    pid.0.clone(),
                    LeadershipPoolEntry { active_stake: *active_stake, vrf_keyhash: vrf.clone() },
                )
            })
            .collect();
        FrozenLeadershipPoolDistr {
            target_leadership_epoch,
            source_slot,
            source_hash,
            source_checkpoint_commitment,
            total_active_stake,
            pools,
        }
    }
}

/// Typed, fail-closed leadership-codec faults — never a silent default or inferred VRF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrozenLeadershipError {
    /// The encoded schema version is not `FROZEN_LEADERSHIP_SCHEMA_VERSION` (an old / foreign object).
    UnknownVersion { expected: u32, found: u32 },
    /// A structural CBOR-shape violation (wrong array/map length, indefinite container, wrong hash width).
    Structural { reason: &'static str },
    /// The same pool key appears twice.
    DuplicatePoolKey,
    /// Pool keys are not in ascending canonical order.
    NonCanonicalMapOrder,
    /// A `u64` field exceeds `u32`.
    FieldOverflow { reason: &'static str },
    /// Bytes remain after the object.
    TrailingBytes { extra: usize },
    /// Structurally valid but not byte-canonical (re-encode != input).
    NonCanonicalBytes,
    /// A lower-level CBOR read failure (short buffer, bad major type).
    Codec,
}

impl From<CodecError> for FrozenLeadershipError {
    fn from(_: CodecError) -> Self {
        FrozenLeadershipError::Codec
    }
}

/// Canonical, versioned, self-describing encoding: `array(6)[version, target_leadership_epoch, source_slot,
/// source_hash, source_checkpoint_commitment, map{ pool_keyhash -> array(2)[active_stake, vrf_keyhash] }]`.
/// `BTreeMap` iteration is ascending canonical key order (the sole acceptable map ordering on an authority
/// path). Zero-stake pools are preserved.
pub fn encode_frozen_leadership(d: &FrozenLeadershipPoolDistr) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(&mut buf, ContainerEncoding::Definite(OUTER_FIELDS, canonical_width(OUTER_FIELDS)));
    write_uint_canonical(&mut buf, FROZEN_LEADERSHIP_SCHEMA_VERSION as u64);
    write_uint_canonical(&mut buf, d.target_leadership_epoch.0);
    write_uint_canonical(&mut buf, d.source_slot.0);
    write_bytes_canonical(&mut buf, &d.source_hash.0);
    write_bytes_canonical(&mut buf, &d.source_checkpoint_commitment.0);
    write_uint_canonical(&mut buf, d.total_active_stake);
    let count = d.pools.len() as u64;
    write_map_header(&mut buf, ContainerEncoding::Definite(count, canonical_width(count)));
    for (keyhash, entry) in &d.pools {
        write_bytes_canonical(&mut buf, &keyhash.0);
        write_array_header(&mut buf, ContainerEncoding::Definite(ENTRY_FIELDS, canonical_width(ENTRY_FIELDS)));
        write_uint_canonical(&mut buf, entry.active_stake);
        write_bytes_canonical(&mut buf, &entry.vrf_keyhash.0);
    }
    buf
}

/// The stable canonical fingerprint (`blake2b-256` of the canonical encoding).
pub fn canonical_hash(d: &FrozenLeadershipPoolDistr) -> Hash32 {
    blake2b_256(&encode_frozen_leadership(d))
}

/// Canonical decode — fail-closed on unknown version, wrong shape, duplicate / unsorted pool keys, wrong hash
/// width, trailing bytes, or any non-byte-canonical encoding (re-encode != input). No inferred VRF, no default.
pub fn decode_frozen_leadership(bytes: &[u8]) -> Result<FrozenLeadershipPoolDistr, FrozenLeadershipError> {
    let mut o = 0usize;
    expect_array(bytes, &mut o, OUTER_FIELDS)?;
    let version = read_u32(bytes, &mut o)?;
    if version != FROZEN_LEADERSHIP_SCHEMA_VERSION {
        return Err(FrozenLeadershipError::UnknownVersion {
            expected: FROZEN_LEADERSHIP_SCHEMA_VERSION,
            found: version,
        });
    }
    let epoch = EpochNo(read_u64(bytes, &mut o)?);
    let source_slot = SlotNo(read_u64(bytes, &mut o)?);
    let source_hash = read_hash32(bytes, &mut o)?;
    let source_checkpoint_commitment = read_hash32(bytes, &mut o)?;
    let total_active_stake = read_u64(bytes, &mut o)?;
    let pools = decode_pools(bytes, &mut o)?;
    if o != bytes.len() {
        return Err(FrozenLeadershipError::TrailingBytes { extra: bytes.len() - o });
    }
    let decoded = FrozenLeadershipPoolDistr {
        target_leadership_epoch: epoch,
        source_slot,
        source_hash,
        source_checkpoint_commitment,
        total_active_stake,
        pools,
    };
    if encode_frozen_leadership(&decoded) != bytes {
        return Err(FrozenLeadershipError::NonCanonicalBytes);
    }
    Ok(decoded)
}

fn decode_pools(
    bytes: &[u8],
    o: &mut usize,
) -> Result<BTreeMap<Hash28, LeadershipPoolEntry>, FrozenLeadershipError> {
    let count = match read_map_header(bytes, o)? {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(FrozenLeadershipError::Structural { reason: "indefinite pools map" })
        }
    };
    let mut pools: BTreeMap<Hash28, LeadershipPoolEntry> = BTreeMap::new();
    let mut prev: Option<Hash28> = None;
    for _ in 0..count {
        let keyhash = read_hash28(bytes, o)?;
        if let Some(p) = &prev {
            match keyhash.0.cmp(&p.0) {
                std::cmp::Ordering::Greater => {}
                std::cmp::Ordering::Equal => return Err(FrozenLeadershipError::DuplicatePoolKey),
                std::cmp::Ordering::Less => return Err(FrozenLeadershipError::NonCanonicalMapOrder),
            }
        }
        expect_array(bytes, o, ENTRY_FIELDS)?;
        let active_stake = read_u64(bytes, o)?;
        let vrf_keyhash = read_hash32(bytes, o)?;
        prev = Some(keyhash.clone());
        pools.insert(keyhash, LeadershipPoolEntry { active_stake, vrf_keyhash });
    }
    Ok(pools)
}

fn expect_array(bytes: &[u8], o: &mut usize, len: u64) -> Result<(), FrozenLeadershipError> {
    match read_array_header(bytes, o)? {
        ContainerEncoding::Definite(n, _) if n == len => Ok(()),
        ContainerEncoding::Definite(_, _) => {
            Err(FrozenLeadershipError::Structural { reason: "wrong array length" })
        }
        ContainerEncoding::Indefinite => {
            Err(FrozenLeadershipError::Structural { reason: "indefinite array" })
        }
    }
}

fn read_u32(bytes: &[u8], o: &mut usize) -> Result<u32, FrozenLeadershipError> {
    let (n, _) = read_uint(bytes, o)?;
    u32::try_from(n).map_err(|_| FrozenLeadershipError::FieldOverflow { reason: "u32 field" })
}

fn read_u64(bytes: &[u8], o: &mut usize) -> Result<u64, FrozenLeadershipError> {
    let (n, _) = read_uint(bytes, o)?;
    Ok(n)
}

fn read_hash32(bytes: &[u8], o: &mut usize) -> Result<Hash32, FrozenLeadershipError> {
    let (h, _) = read_bytes(bytes, o)?;
    let arr: [u8; 32] = h
        .try_into()
        .map_err(|_| FrozenLeadershipError::Structural { reason: "expected 32-byte hash" })?;
    Ok(Hash32(arr))
}

fn read_hash28(bytes: &[u8], o: &mut usize) -> Result<Hash28, FrozenLeadershipError> {
    let (h, _) = read_bytes(bytes, o)?;
    let arr: [u8; 28] = h
        .try_into()
        .map_err(|_| FrozenLeadershipError::Structural { reason: "expected 28-byte hash" })?;
    Ok(Hash28(arr))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_core::consensus::ledger_view::LedgerView;

    #[test]
    fn to_pool_distr_view_reads_stake_and_vrf_directly_incl_zero_stake() {
        let mut pools = BTreeMap::new();
        pools.insert(
            Hash28([0x11; 28]),
            LeadershipPoolEntry { active_stake: 1_000, vrf_keyhash: Hash32([0xA1; 32]) },
        );
        // A zero-stake registered pool: carried (leadership-set membership) but contributes 0 to the total.
        pools.insert(
            Hash28([0x22; 28]),
            LeadershipPoolEntry { active_stake: 0, vrf_keyhash: Hash32([0xB2; 32]) },
        );
        let d = FrozenLeadershipPoolDistr {
            target_leadership_epoch: EpochNo(1341),
            source_slot: SlotNo(115_862_416),
            source_hash: Hash32([0x07; 32]),
            source_checkpoint_commitment: Hash32([0x0C; 32]),
            // LV-1: preserves this test's PRE-EXISTING semantics (it was written against the summed
            // denominator). Production never sums -- see StakeSnapshot::total_active_stake.
            total_active_stake: pools.values().map(|e| e.active_stake).sum(),
            pools,
        };
        let asc = ActiveSlotsCoeff { numer: 1, denom: 20 };
        let v = d.to_pool_distr_view(asc);
        assert_eq!(v.epoch(), EpochNo(1341));
        assert_eq!(v.total_active_stake(EpochNo(1341)), Some(1_000));
        assert_eq!(v.pool_active_stake(EpochNo(1341), &Hash28([0x11; 28])), Some(1_000));
        assert_eq!(v.pool_vrf_keyhash(EpochNo(1341), &Hash28([0x11; 28])), Some(Hash32([0xA1; 32])));
        // The zero-stake pool is present with its VRF (leadership-set membership), stake 0.
        assert_eq!(v.pool_active_stake(EpochNo(1341), &Hash28([0x22; 28])), Some(0));
        assert_eq!(v.pool_vrf_keyhash(EpochNo(1341), &Hash28([0x22; 28])), Some(Hash32([0xB2; 32])));
    }

    #[test]
    fn from_boundary_snapshot_is_the_delegation_image_not_the_full_registered_set() {
        // The pre-POOLREAP REGISTERED pools: a staked pool, a zero-stake pool with a delegator, a retiring
        // pool with stake, AND a registered pool with NO delegator (0x44) — the last must be EXCLUDED (nesPd is
        // `numDelegators > 0`, not the full registered set).
        let mut registered_pool_vrfs: BTreeMap<PoolId, Hash32> = BTreeMap::new();
        registered_pool_vrfs.insert(PoolId(Hash28([0x11; 28])), Hash32([0xA1; 32]));
        registered_pool_vrfs.insert(PoolId(Hash28([0x22; 28])), Hash32([0xB2; 32])); // zero-stake, has delegator
        registered_pool_vrfs.insert(PoolId(Hash28([0x33; 28])), Hash32([0xC3; 32])); // retiring, has stake
        registered_pool_vrfs.insert(PoolId(Hash28([0x44; 28])), Hash32([0xD4; 32])); // registered, NO delegator
        // The pools with >= 1 delegator (the delegation-map image) — 0x44 is registered but has no delegator.
        let delegated_pools: BTreeSet<PoolId> = [0x11u8, 0x22, 0x33]
            .into_iter()
            .map(|b| PoolId(Hash28([b; 28])))
            .collect();
        // The mark's stake: only the non-zero pools (the ssActiveStake NonZero rule omits zero-stake), plus a
        // STRAY pool with stake but no registration + no delegation (must not occur; excluded).
        let mut mark_pool_stakes: BTreeMap<PoolId, Coin> = BTreeMap::new();
        mark_pool_stakes.insert(PoolId(Hash28([0x11; 28])), Coin(1_000));
        mark_pool_stakes.insert(PoolId(Hash28([0x33; 28])), Coin(1_000_000_000_000));
        mark_pool_stakes.insert(PoolId(Hash28([0x99; 28])), Coin(5)); // stray

        let d = FrozenLeadershipPoolDistr::from_boundary_snapshot(
            EpochNo(1341),
            SlotNo(115_862_416),
            Hash32([0x07; 32]),
            Hash32([0x0C; 32]),
            &delegated_pools,
            &mark_pool_stakes,
            &registered_pool_vrfs,
            // LV-1: an explicit snapshot total, distinct from the summed entries so the test cannot
            // pass by accident if the summing loop ever returns.
            7_777_777,
        );
        assert_eq!(d.target_leadership_epoch, EpochNo(1341));
        assert_eq!(d.source_slot, SlotNo(115_862_416));
        // The leadership SET is the delegation image (3 pools); the registered-but-undelegated 0x44 and the
        // stray unregistered 0x99 are BOTH excluded.
        assert_eq!(d.pools.len(), 3);
        assert!(!d.pools.contains_key(&Hash28([0x44; 28])), "a registered pool with no delegator is not leadership");
        assert!(!d.pools.contains_key(&Hash28([0x99; 28])), "a pool with no frozen params is not leadership");
        // Staked pool: stake from the mark, VRF from the frozen params.
        assert_eq!(
            d.pools[&Hash28([0x11; 28])],
            LeadershipPoolEntry { active_stake: 1_000, vrf_keyhash: Hash32([0xA1; 32]) }
        );
        // Zero-stake pool WITH a delegator: present with stake 0 + its frozen VRF (cardano's nes[5] keeps it).
        assert_eq!(
            d.pools[&Hash28([0x22; 28])],
            LeadershipPoolEntry { active_stake: 0, vrf_keyhash: Hash32([0xB2; 32]) }
        );
        // Retiring-but-still-registered pool: its stake + frozen VRF are captured PRE-POOLREAP — the datum a
        // use-time active-param lookup would drop once POOLREAP reaps it.
        assert_eq!(
            d.pools[&Hash28([0x33; 28])],
            LeadershipPoolEntry { active_stake: 1_000_000_000_000, vrf_keyhash: Hash32([0xC3; 32]) }
        );
    }

    fn sample_distr() -> FrozenLeadershipPoolDistr {
        let mut pools = BTreeMap::new();
        pools.insert(
            Hash28([0x11; 28]),
            LeadershipPoolEntry { active_stake: 1_000, vrf_keyhash: Hash32([0xA1; 32]) },
        );
        // A zero-stake registered pool — carried for leadership-set membership / byte-identity.
        pools.insert(
            Hash28([0x22; 28]),
            LeadershipPoolEntry { active_stake: 0, vrf_keyhash: Hash32([0xB2; 32]) },
        );
        pools.insert(
            Hash28([0x33; 28]),
            LeadershipPoolEntry { active_stake: 999_999_999_999, vrf_keyhash: Hash32([0xC3; 32]) },
        );
        FrozenLeadershipPoolDistr {
            target_leadership_epoch: EpochNo(1341),
            source_slot: SlotNo(115_862_416),
            source_hash: Hash32([0x07; 32]),
            source_checkpoint_commitment: Hash32([0x0C; 32]),
            // LV-1: preserves this test's PRE-EXISTING semantics (it was written against the summed
            // denominator). Production never sums -- see StakeSnapshot::total_active_stake.
            total_active_stake: pools.values().map(|e| e.active_stake).sum(),
            pools,
        }
    }

    /// Build the frozen-leadership CBOR for an explicit ordered pool list — allows duplicate / unsorted keys
    /// (which a `BTreeMap` cannot express) so the fail-closed decode paths can be exercised directly.
    fn encode_pool_order(
        epoch: EpochNo,
        slot: SlotNo,
        hash: &Hash32,
        pools: &[(Hash28, u64, Hash32)],
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        write_array_header(&mut buf, ContainerEncoding::Definite(OUTER_FIELDS, canonical_width(OUTER_FIELDS)));
        write_uint_canonical(&mut buf, FROZEN_LEADERSHIP_SCHEMA_VERSION as u64);
        write_uint_canonical(&mut buf, epoch.0);
        write_uint_canonical(&mut buf, slot.0);
        write_bytes_canonical(&mut buf, &hash.0);
        write_bytes_canonical(&mut buf, &[0x0C; 32]); // v6: source_checkpoint_commitment
        write_uint_canonical(&mut buf, 1_000_000); // v7 (LV-1): total_active_stake
        let count = pools.len() as u64;
        write_map_header(&mut buf, ContainerEncoding::Definite(count, canonical_width(count)));
        for (keyhash, stake, vrf) in pools {
            write_bytes_canonical(&mut buf, &keyhash.0);
            write_array_header(&mut buf, ContainerEncoding::Definite(ENTRY_FIELDS, canonical_width(ENTRY_FIELDS)));
            write_uint_canonical(&mut buf, *stake);
            write_bytes_canonical(&mut buf, &vrf.0);
        }
        buf
    }

    #[test]
    fn codec_round_trip_identity() {
        let d = sample_distr();
        let bytes = encode_frozen_leadership(&d);
        let back = decode_frozen_leadership(&bytes).unwrap();
        assert_eq!(back, d);
        // Re-encode is byte-stable (canonical).
        assert_eq!(encode_frozen_leadership(&back), bytes);
    }

    #[test]
    fn canonical_hash_is_stable_and_content_bound() {
        let d = sample_distr();
        let h = canonical_hash(&d);
        assert_eq!(canonical_hash(&d.clone()), h);
        // A changed stake changes the hash.
        let mut d_stake = d.clone();
        d_stake.pools.get_mut(&Hash28([0x11; 28])).unwrap().active_stake = 1_001;
        assert_ne!(canonical_hash(&d_stake), h);
        // A changed VRF changes the hash.
        let mut d_vrf = d.clone();
        d_vrf.pools.get_mut(&Hash28([0x11; 28])).unwrap().vrf_keyhash = Hash32([0xFF; 32]);
        assert_ne!(canonical_hash(&d_vrf), h);
        // A changed source point changes the hash.
        let mut d_src = d;
        d_src.source_slot = SlotNo(115_862_417);
        assert_ne!(canonical_hash(&d_src), h);
    }

    #[test]
    fn codec_preserves_zero_stake_pool() {
        let back = decode_frozen_leadership(&encode_frozen_leadership(&sample_distr())).unwrap();
        let z = back.pools.get(&Hash28([0x22; 28])).unwrap();
        assert_eq!(z.active_stake, 0);
        assert_eq!(z.vrf_keyhash, Hash32([0xB2; 32]));
    }

    #[test]
    fn codec_rejects_unknown_version() {
        let mut bytes = encode_frozen_leadership(&sample_distr());
        // Outer array header then the single-byte version uint at offset 1. The header byte tracks
        // OUTER_FIELDS, so assert it instead of naming a stale literal.
        assert_eq!(bytes[0], 0x80 | (OUTER_FIELDS as u8));
        assert_eq!(bytes[1], FROZEN_LEADERSHIP_SCHEMA_VERSION as u8);
        bytes[1] = 4;
        assert_eq!(
            decode_frozen_leadership(&bytes),
            Err(FrozenLeadershipError::UnknownVersion {
                expected: FROZEN_LEADERSHIP_SCHEMA_VERSION,
                found: 4
            })
        );
    }

    #[test]
    fn codec_rejects_duplicate_pool_key() {
        let k = Hash28([0x44; 28]);
        let bytes = encode_pool_order(
            EpochNo(1341),
            SlotNo(1),
            &Hash32([0x07; 32]),
            &[(k.clone(), 10, Hash32([0x01; 32])), (k, 20, Hash32([0x02; 32]))],
        );
        assert_eq!(decode_frozen_leadership(&bytes), Err(FrozenLeadershipError::DuplicatePoolKey));
    }

    #[test]
    fn codec_rejects_unsorted_pool_keys() {
        // Descending key order (0x55.. before 0x44..) is not ascending canonical.
        let bytes = encode_pool_order(
            EpochNo(1341),
            SlotNo(1),
            &Hash32([0x07; 32]),
            &[(Hash28([0x55; 28]), 10, Hash32([0x01; 32])), (Hash28([0x44; 28]), 20, Hash32([0x02; 32]))],
        );
        assert_eq!(decode_frozen_leadership(&bytes), Err(FrozenLeadershipError::NonCanonicalMapOrder));
    }

    #[test]
    fn codec_rejects_trailing_bytes() {
        let mut bytes = encode_frozen_leadership(&sample_distr());
        bytes.push(0xFF);
        assert_eq!(
            decode_frozen_leadership(&bytes),
            Err(FrozenLeadershipError::TrailingBytes { extra: 1 })
        );
    }
}
