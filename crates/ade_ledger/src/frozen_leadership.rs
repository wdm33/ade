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

use std::collections::BTreeMap;

use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_types::{EpochNo, Hash28, Hash32, SlotNo};

use crate::consensus_view::{PoolDistrView, PoolEntry};

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
    /// The epoch this distribution authorizes leadership for.
    pub epoch: EpochNo,
    /// The canonical selected point the leadership distribution was frozen at.
    pub source_slot: SlotNo,
    /// Its lineage hash.
    pub source_hash: Hash32,
    /// Per-pool frozen leadership entry, canonical key order.
    pub pools: BTreeMap<Hash28, LeadershipPoolEntry>,
}

impl FrozenLeadershipPoolDistr {
    /// Project into the leadership [`PoolDistrView`] — stake and VRF read DIRECTLY from this frozen object,
    /// never an active-param / go / `future_pools` / `retiring` lookup. `total_active_stake` = the sum of the
    /// per-pool stakes (zero-stake registered pools contribute 0 but are carried for byte-identity).
    pub fn to_pool_distr_view(&self, asc: ActiveSlotsCoeff) -> PoolDistrView {
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        let mut total_active_stake: u64 = 0;
        for (keyhash, entry) in &self.pools {
            total_active_stake = total_active_stake.saturating_add(entry.active_stake);
            pools.insert(
                keyhash.clone(),
                PoolEntry {
                    active_stake: entry.active_stake,
                    vrf_keyhash: entry.vrf_keyhash.clone(),
                },
            );
        }
        PoolDistrView::new(self.epoch, total_active_stake, asc, pools)
    }

    /// The bootstrap import: build the frozen leadership distribution from the manifest-bound seed consensus
    /// inputs (the named artifact). The seed record's `pool_distribution` IS cardano's leadership `nesPd`
    /// (proven byte-exact vs the reference at bootstrap; the LDAT trace `67890681` confirmed it reproduces
    /// the 659-pool leadership set incl. zero-stake + retired-frozen pools).
    pub fn from_seed_epoch_consensus_inputs(
        record: &crate::seed_consensus_inputs::SeedEpochConsensusInputs,
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
            epoch: record.epoch_no,
            source_slot: record.seed_point_slot,
            source_hash: record.seed_point_hash.clone(),
            pools,
        }
    }
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
            epoch: EpochNo(1341),
            source_slot: SlotNo(115_862_416),
            source_hash: Hash32([0x07; 32]),
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
}
