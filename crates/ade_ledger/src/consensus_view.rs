// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE production `LedgerView` projection.
//!
//! `PoolDistrView` is the leadership-relevant projection of a `LedgerState`'s
//! pool-distribution (`nesPd` / `stakeDistrib.unPoolDistr`). It surfaces the
//! four facts BLUE consensus consumes through the `ade_core::consensus::LedgerView`
//! boundary — total active stake, per-pool active stake, per-pool registered VRF
//! keyhash, and the active-slots coefficient — and nothing else.
//!
//! Pure data: it is constructed once from an already-frozen snapshot (for B1,
//! the committed Conway-576 corpus; later, a parsed `LedgerState`) and never
//! performs I/O, holds a clock, or rederives a stake snapshot. `BTreeMap` only —
//! deterministic iteration is the only acceptable shape in an authority path.

use std::collections::BTreeMap;

use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_types::{EpochNo, Hash28, Hash32};

/// One pool's slice of the leadership projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolEntry {
    /// Active stake (lovelace) frozen at the set snapshot (E−2).
    pub active_stake: u64,
    /// Registered VRF key *hash* (`blake2b-256` of the VRF vkey). The vkey
    /// itself arrives in the block header; header validation binds the two.
    pub vrf_keyhash: Hash32,
}

/// The leadership-relevant projection of a ledger pool-distribution.
///
/// Single-epoch: a `PoolDistrView` answers only for the one `epoch` it was
/// built for. Queries for any other epoch return `None`, so a caller can never
/// silently consume the wrong snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolDistrView {
    epoch: EpochNo,
    total_active_stake: u64,
    asc: ActiveSlotsCoeff,
    pools: BTreeMap<Hash28, PoolEntry>,
}

impl PoolDistrView {
    /// Build a projection for one operating epoch from already-frozen data.
    pub fn new(
        epoch: EpochNo,
        total_active_stake: u64,
        asc: ActiveSlotsCoeff,
        pools: BTreeMap<Hash28, PoolEntry>,
    ) -> Self {
        Self {
            epoch,
            total_active_stake,
            asc,
            pools,
        }
    }

    /// PHASE4-N-F-A A4: project the **recovered** seed-epoch
    /// consensus-input record into the leadership `PoolDistrView`.
    ///
    /// A near-direct field map: A2's merge already zipped per-pool
    /// active stake with the registered VRF keyhash into the single
    /// `BTreeMap<Hash28, PoolEntry>` the view holds, so no second map
    /// and no zero-hash fallback are needed here (unlike the
    /// operator-bundle projection, which zips two maps). `epoch_no`,
    /// `total_active_stake`, and `active_slots_coeff` are carried
    /// verbatim from the recovered record.
    ///
    /// This is the projection half of `DC-CINPUT-02`: it proves the
    /// recovered surface is a drop-in leadership source. The A5
    /// production-wiring slice swaps the bounty-primary call site to
    /// call this instead of the bundle projection (CE-A-4b); A4 only
    /// ships + pins the projection (CE-A-4a).
    pub fn from_seed_epoch_consensus_inputs(
        record: &crate::seed_consensus_inputs::SeedEpochConsensusInputs,
    ) -> Self {
        Self {
            epoch: record.epoch_no,
            total_active_stake: record.total_active_stake,
            asc: record.active_slots_coeff,
            pools: record.pool_distribution.clone(),
        }
    }
}

impl LedgerView for PoolDistrView {
    fn total_active_stake(&self, epoch: EpochNo) -> Option<u64> {
        (epoch == self.epoch).then_some(self.total_active_stake)
    }

    fn pool_active_stake(&self, epoch: EpochNo, pool: &Hash28) -> Option<u64> {
        if epoch != self.epoch {
            return None;
        }
        self.pools.get(pool).map(|p| p.active_stake)
    }

    fn pool_vrf_keyhash(&self, epoch: EpochNo, pool: &Hash28) -> Option<Hash32> {
        if epoch != self.epoch {
            return None;
        }
        self.pools.get(pool).map(|p| p.vrf_keyhash.clone())
    }

    fn active_slots_coeff(&self, epoch: EpochNo) -> Option<ActiveSlotsCoeff> {
        (epoch == self.epoch).then_some(self.asc)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn pool_a() -> Hash28 {
        Hash28([0x01; 28])
    }

    fn view() -> PoolDistrView {
        let mut pools = BTreeMap::new();
        pools.insert(
            pool_a(),
            PoolEntry {
                active_stake: 1_000,
                vrf_keyhash: Hash32([0x07; 32]),
            },
        );
        PoolDistrView::new(
            EpochNo(576),
            10_000,
            ActiveSlotsCoeff { numer: 1, denom: 20 },
            pools,
        )
    }

    #[test]
    fn pool_distr_view_no_hashmap() {
        // Structural: the only associative container in the projection is a
        // BTreeMap, asserted by construction here and grepped in CI.
        let v = view();
        assert_eq!(v.total_active_stake(EpochNo(576)), Some(10_000));
        assert_eq!(v.pool_active_stake(EpochNo(576), &pool_a()), Some(1_000));
    }

    // ===== PHASE4-N-F-A A4: recovered-surface projection =====

    use crate::seed_consensus_inputs::SeedEpochConsensusInputs;

    fn sample_record() -> SeedEpochConsensusInputs {
        let mut pools = BTreeMap::new();
        pools.insert(
            Hash28([0x01; 28]),
            PoolEntry {
                active_stake: 1_000,
                vrf_keyhash: Hash32([0x07; 32]),
            },
        );
        pools.insert(
            Hash28([0x05; 28]),
            PoolEntry {
                active_stake: 2_500,
                vrf_keyhash: Hash32([0x08; 32]),
            },
        );
        SeedEpochConsensusInputs {
            anchor_fp: Hash32([0x5A; 32]),
            epoch_no: EpochNo(576),
            active_slots_coeff: ActiveSlotsCoeff { numer: 5, denom: 100 },
            total_active_stake: 3_500,
            pool_distribution: pools,
        }
    }

    #[test]
    fn projection_maps_recovered_fields_onto_ledgerview_surface() {
        // The recovered record projects onto the full LedgerView surface
        // for its seed epoch: total / per-pool stake / per-pool VRF
        // keyhash / ASC all reflect the record verbatim.
        let r = sample_record();
        let v = PoolDistrView::from_seed_epoch_consensus_inputs(&r);
        assert_eq!(v.total_active_stake(EpochNo(576)), Some(3_500));
        assert_eq!(
            v.pool_active_stake(EpochNo(576), &Hash28([0x01; 28])),
            Some(1_000)
        );
        assert_eq!(
            v.pool_active_stake(EpochNo(576), &Hash28([0x05; 28])),
            Some(2_500)
        );
        assert_eq!(
            v.pool_vrf_keyhash(EpochNo(576), &Hash28([0x01; 28])),
            Some(Hash32([0x07; 32]))
        );
        assert_eq!(
            v.pool_vrf_keyhash(EpochNo(576), &Hash28([0x05; 28])),
            Some(Hash32([0x08; 32]))
        );
        assert_eq!(
            v.active_slots_coeff(EpochNo(576)),
            Some(ActiveSlotsCoeff { numer: 5, denom: 100 })
        );
        // Equivalent to the direct hand-built view (field-map fidelity).
        assert_eq!(v, PoolDistrView::new(
            r.epoch_no,
            r.total_active_stake,
            r.active_slots_coeff,
            r.pool_distribution.clone(),
        ));
    }

    #[test]
    fn projection_two_runs_identical() {
        let r = sample_record();
        assert_eq!(
            PoolDistrView::from_seed_epoch_consensus_inputs(&r),
            PoolDistrView::from_seed_epoch_consensus_inputs(&r)
        );
    }

    #[test]
    fn projection_off_epoch_returns_none() {
        // Single-epoch semantics preserved: every LedgerView query for an
        // epoch other than the recovered seed epoch returns None.
        let v = PoolDistrView::from_seed_epoch_consensus_inputs(&sample_record());
        assert_eq!(v.total_active_stake(EpochNo(577)), None);
        assert_eq!(v.pool_active_stake(EpochNo(577), &Hash28([0x01; 28])), None);
        assert_eq!(v.pool_vrf_keyhash(EpochNo(577), &Hash28([0x01; 28])), None);
        assert_eq!(v.active_slots_coeff(EpochNo(577)), None);
    }
}
