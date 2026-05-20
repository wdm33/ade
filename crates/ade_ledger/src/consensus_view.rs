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
}
