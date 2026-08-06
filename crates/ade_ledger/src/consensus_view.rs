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
use ade_types::{EpochNo, Hash28, Hash32, PoolId};

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

    /// LIVE-2 (CE-L2-6): how many pools this leadership view actually carries.
    ///
    /// Exists to tell "our pool is not in a POPULATED leadership set" (a legitimate not-elected
    /// answer) from "the leadership set is EMPTY" (authority that was never established, which can
    /// answer nothing). Both make `leader_schedule_answer` return `UnknownPool`, and collapsing them
    /// is how an unestablished authority comes to report a confident `not_leader`.
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }

    /// The single epoch this view answers for. EPOCH-CONTINUITY-ACTIVATION ECA-3 (DC-EPOCH-14): the
    /// authority's epoch-match guard compares this to the protocol epoch implied by a block/slot
    /// context, so a wrong-epoch (missing / premature / mismatched) promotion is a structured halt.
    pub fn epoch(&self) -> EpochNo {
        self.epoch
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

    /// LIVE-LEDGER-EPOCH-TRANSITION S4 — **FAILED HYPOTHESIS, test-only (do NOT use as production authority).**
    /// Derives a leadership `PoolDistrView` from the go-snapshot stake + the ACTIVE pool-params VRF. The
    /// Leadership Distribution Authority Trace (`ce3d_boundary_differential::ldat_classify_leadership_pools`)
    /// PROVED this does NOT reproduce cardano's leadership `nesPd`: the leadership stake is the SET snapshot
    /// (go is the REWARD snapshot), and a retired/POOLREAP'd pool's VRF is ABSENT from the active params (it
    /// survives only in the snapshot-FROZEN leadership params). So this cannot be the S4 authority — the
    /// frozen-leadership builder (S4-pre) replaces it. Retained ONLY as a negative regression; the
    /// `_for_test_only` suffix blocks accidental production wiring. FAIL-CLOSED `NotLeadershipComplete` on a
    /// staked go pool with no registered params.
    pub fn from_accumulator_go_active_params_for_test_only(
        acc: &crate::epoch_accumulator::EpochAccumulator,
        asc: ActiveSlotsCoeff,
    ) -> Result<Self, AccumulatorAuthorityError> {
        let snaps = acc
            .epoch_state
            .snapshots
            .as_authoritative()
            .ok_or(AccumulatorAuthorityError::SnapshotsNotAuthoritative)?;
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        let mut total_active_stake: u64 = 0;
        for (pool_id, coin) in &snaps.go.0.pool_stakes {
            let params = acc
                .cert_state
                .pool
                .pools
                .get(pool_id)
                .ok_or_else(|| AccumulatorAuthorityError::NotLeadershipComplete(pool_id.clone()))?;
            total_active_stake = total_active_stake
                .checked_add(coin.0)
                .ok_or(AccumulatorAuthorityError::StakeOverflow)?;
            pools.insert(
                pool_id.0.clone(),
                PoolEntry {
                    active_stake: coin.0,
                    vrf_keyhash: params.vrf_hash.clone(),
                },
            );
        }
        Ok(Self {
            epoch: acc.epoch_state.epoch,
            total_active_stake,
            asc,
            pools,
        })
    }
}

/// LIVE-LEDGER-EPOCH-TRANSITION S4: why the accumulator cannot answer as the leadership authority for a
/// prefix. Every variant is a fail-closed terminal — NEVER a silent seed-window fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccumulatorAuthorityError {
    /// The accumulator's snapshots are not in the authoritative (post-seed) phase.
    SnapshotsNotAuthoritative,
    /// A staked go-snapshot pool has no registered params (VRF) in the active pool set — the view is not
    /// leadership-complete, so it cannot be promoted to authority.
    NotLeadershipComplete(PoolId),
    /// The summed active stake overflowed u64 (unreachable under the max-supply bound).
    StakeOverflow,
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

    /// S4 (DC-EPOCH-19): `from_accumulator` joins the go-snapshot stake with the active pool params' VRF into
    /// the leadership view, and FAILS CLOSED (never a zero-hash / seed fallback) on a staked pool with no
    /// registered params. Byte-identity vs the seed view is the flip's own acceptance gate (real fixture).
    #[test]
    fn from_accumulator_joins_go_stake_with_active_pool_vrf_or_fails_closed() {
        use crate::delegation::PoolParams;
        use crate::epoch::{GoSnapshot, StakeSnapshot};
        use crate::epoch_accumulator::EpochAccumulator;
        use ade_types::{CardanoEra, Coin};

        let pid = |b: u8| PoolId(Hash28([b; 28]));
        let pp = |b: u8, vrf: u8| PoolParams {
            pool_id: pid(b),
            vrf_hash: Hash32([vrf; 32]),
            pledge: Coin(0),
            cost: Coin(0),
            margin: (0, 1),
            reward_account: Vec::new(),
            owners: Vec::new(),
        };

        let mut acc = EpochAccumulator::new(CardanoEra::Conway);
        acc.epoch_state.epoch = EpochNo(1341);
        let mut snap = StakeSnapshot::new();
        snap.pool_stakes.insert(pid(0x11), Coin(1_000));
        snap.pool_stakes.insert(pid(0x22), Coin(2_000));
        acc.epoch_state.snapshots.as_authoritative_mut().unwrap().go = GoSnapshot(snap);
        acc.cert_state.pool.pools.insert(pid(0x11), pp(0x11, 0xA1));
        acc.cert_state.pool.pools.insert(pid(0x22), pp(0x22, 0xB2));

        let asc = ActiveSlotsCoeff { numer: 1, denom: 20 };
        let v = PoolDistrView::from_accumulator_go_active_params_for_test_only(&acc, asc).unwrap();
        assert_eq!(v.epoch(), EpochNo(1341));
        assert_eq!(v.total_active_stake(EpochNo(1341)), Some(3_000));
        assert_eq!(v.pool_active_stake(EpochNo(1341), &pid(0x11).0), Some(1_000));
        assert_eq!(v.pool_vrf_keyhash(EpochNo(1341), &pid(0x11).0), Some(Hash32([0xA1; 32])));
        assert_eq!(v.pool_vrf_keyhash(EpochNo(1341), &pid(0x22).0), Some(Hash32([0xB2; 32])));
        assert_eq!(v.active_slots_coeff(EpochNo(1341)), Some(asc));
        // A different epoch never silently answers (single-epoch guard).
        assert_eq!(v.pool_active_stake(EpochNo(1342), &pid(0x11).0), None);

        // FAIL-CLOSED: a staked go pool with no active params -> NotLeadershipComplete.
        acc.cert_state.pool.pools.remove(&pid(0x22));
        assert_eq!(
            PoolDistrView::from_accumulator_go_active_params_for_test_only(&acc, asc),
            Err(AccumulatorAuthorityError::NotLeadershipComplete(pid(0x22)))
        );
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
    use ade_types::SlotNo;

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
            epoch_start_slot: SlotNo(576 * 432_000),
            epoch_length_slots: 432_000,
            security_param: 2160,
            epoch_nonce: ade_core::consensus::praos_state::Nonce(Hash32([0x55; 32])),
            genesis_hash: Hash32([0x9a; 32]),
            protocol_params_hash: Hash32([0x9b; 32]),
            seed_point_slot: SlotNo(576 * 432_000 + 100),
            seed_point_hash: Hash32([0x6c; 32]),
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
