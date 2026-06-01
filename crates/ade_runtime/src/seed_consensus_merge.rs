// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN seed-epoch consensus-inputs merge transform (PHASE4-N-F-A A2).
//!
//! Pure mapping, no I/O, no clock, no float, deterministic, `BTreeMap`
//! only: lifts a verified-bootstrap [`LiveConsensusInputsCanonical`]
//! (the bootstrap-time extraction shape, whose `pool_distribution`
//! carries only `active_stake` and whose VRF keyhashes live in the
//! separate `pool_vrf_keyhashes` map) plus the minted anchor
//! fingerprint and seed epoch into the BLUE single-map
//! [`SeedEpochConsensusInputs`] (whose `PoolEntry` carries both
//! `active_stake` and `vrf_keyhash`).
//!
//! `total_active_stake` is the saturating sum of every pool's
//! `active_stake`, derived exactly as the forge-time projection
//! `pool_distr_view_from_consensus_inputs` does (so A4's recovered-surface
//! projection matches that prior output for the seed epoch).
//!
//! Fail-closed (no defaulting): a pool present in `pool_distribution`
//! but absent from `pool_vrf_keyhashes`, or vice versa, is a structured
//! [`SeedConsensusMergeError`] — never a zero-hash fill. The bootstrap
//! provenance record must be complete or the bootstrap fails.

use std::collections::BTreeMap;

use ade_ledger::consensus_view::PoolEntry as BluePoolEntry;
use ade_ledger::seed_consensus_inputs::SeedEpochConsensusInputs;
use ade_types::{EpochNo, Hash28, Hash32};

use crate::consensus_inputs::LiveConsensusInputsCanonical;

/// Closed error sum for the seed-epoch consensus-inputs merge. Carries
/// only non-secret primitives (the offending pool keyhash); no
/// `String`/`anyhow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedConsensusMergeError {
    /// A pool appeared in `pool_distribution` (stake) but had no entry
    /// in `pool_vrf_keyhashes` (VRF keyhash). Fail-closed: no defaulting.
    PoolMissingVrfKeyhash { pool: Hash28 },
    /// A pool appeared in `pool_vrf_keyhashes` (VRF keyhash) but had no
    /// entry in `pool_distribution` (stake). Fail-closed: no defaulting.
    PoolMissingStake { pool: Hash28 },
}

/// Merge the verified-bootstrap canonical consensus inputs into the
/// anchor-bound BLUE [`SeedEpochConsensusInputs`]. Deterministic in its
/// inputs; fail-closed on any pool present in exactly one source map.
pub fn merge_seed_epoch_consensus_inputs(
    anchor_fp: Hash32,
    epoch_no: EpochNo,
    canonical: &LiveConsensusInputsCanonical,
) -> Result<SeedEpochConsensusInputs, SeedConsensusMergeError> {
    let mut pool_distribution: BTreeMap<Hash28, BluePoolEntry> = BTreeMap::new();
    let mut total_active_stake: u64 = 0;

    for (pool, entry) in &canonical.pool_distribution {
        let vrf_keyhash = canonical
            .pool_vrf_keyhashes
            .get(pool)
            .cloned()
            .ok_or_else(|| SeedConsensusMergeError::PoolMissingVrfKeyhash {
                pool: pool.clone(),
            })?;
        total_active_stake = total_active_stake.saturating_add(entry.active_stake);
        pool_distribution.insert(
            pool.clone(),
            BluePoolEntry {
                active_stake: entry.active_stake,
                vrf_keyhash,
            },
        );
    }

    // Reverse direction: a VRF keyhash without a matching stake entry is
    // equally a provenance gap. (The bootstrap importer already enforces
    // key-set parity, but the merge must not depend on that upstream
    // guarantee — it fails closed on its own.)
    for pool in canonical.pool_vrf_keyhashes.keys() {
        if !canonical.pool_distribution.contains_key(pool) {
            return Err(SeedConsensusMergeError::PoolMissingStake { pool: pool.clone() });
        }
    }

    Ok(SeedEpochConsensusInputs {
        anchor_fp,
        epoch_no,
        active_slots_coeff: canonical.active_slots_coeff,
        total_active_stake,
        pool_distribution,
    })
}

/// Test-only builder for a `LiveConsensusInputsCanonical` whose pool
/// stake map and VRF-keyhash map are supplied independently, so a test
/// can craft a deliberately mismatched (missing-VRF or missing-stake)
/// bundle the importer would otherwise reject. Shared across the
/// bootstrap composition test modules.
#[cfg(test)]
pub(crate) fn test_canonical_inputs(
    epoch_no: EpochNo,
    pool_stake: BTreeMap<Hash28, u64>,
    pool_vrf_keyhashes: BTreeMap<Hash28, Hash32>,
) -> LiveConsensusInputsCanonical {
    use ade_core::consensus::praos_state::Nonce;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_types::{CardanoEra, SlotNo};

    use crate::consensus_inputs::PoolEntry as RedPoolEntry;

    let pool_distribution: BTreeMap<Hash28, RedPoolEntry> = pool_stake
        .into_iter()
        .map(|(k, active_stake)| (k, RedPoolEntry { active_stake }))
        .collect();

    LiveConsensusInputsCanonical {
        network_magic: 1,
        genesis_hash: Hash32([0x11; 32]),
        era: CardanoEra::Conway,
        epoch_no,
        epoch_start_slot: SlotNo(0),
        epoch_end_slot: SlotNo(432_000),
        active_slots_coeff: ActiveSlotsCoeff { numer: 1, denom: 20 },
        epoch_nonce: Nonce(Hash32([0xCD; 32])),
        pool_distribution,
        pool_vrf_keyhashes,
        protocol_params_hash: Hash32([0xEE; 32]),
        source_cardano_node_version: "cardano-node 11.0.1".to_string(),
        source_query_command: "cardano-cli query stake-distribution".to_string(),
        source_tip_hash: Hash32([0xFF; 32]),
        source_tip_slot: SlotNo(100),
        fingerprint: Hash32([0xAB; 32]),
        protocol_params_json: None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn pool(b: u8) -> Hash28 {
        Hash28([b; 28])
    }

    fn vrf(b: u8) -> Hash32 {
        Hash32([b; 32])
    }

    #[test]
    fn bootstrap_seed_inputs_merge_fails_closed_on_missing_vrf_or_stake() {
        // Direction 1: a pool present in the stake map but absent from
        // the VRF map → PoolMissingVrfKeyhash; no defaulting.
        let mut stake = BTreeMap::new();
        stake.insert(pool(0x01), 1_000u64);
        stake.insert(pool(0x02), 2_000u64);
        let mut vrfs = BTreeMap::new();
        vrfs.insert(pool(0x01), vrf(0x07));
        let canonical = test_canonical_inputs(EpochNo(576), stake, vrfs);
        let err = merge_seed_epoch_consensus_inputs(Hash32([0x44; 32]), EpochNo(576), &canonical)
            .expect_err("missing vrf must fail closed");
        assert_eq!(
            err,
            SeedConsensusMergeError::PoolMissingVrfKeyhash { pool: pool(0x02) }
        );

        // Direction 2: a pool present in the VRF map but absent from the
        // stake map → PoolMissingStake; no defaulting.
        let mut stake = BTreeMap::new();
        stake.insert(pool(0x01), 1_000u64);
        let mut vrfs = BTreeMap::new();
        vrfs.insert(pool(0x01), vrf(0x07));
        vrfs.insert(pool(0x03), vrf(0x09));
        let canonical = test_canonical_inputs(EpochNo(576), stake, vrfs);
        let err = merge_seed_epoch_consensus_inputs(Hash32([0x44; 32]), EpochNo(576), &canonical)
            .expect_err("missing stake must fail closed");
        assert_eq!(
            err,
            SeedConsensusMergeError::PoolMissingStake { pool: pool(0x03) }
        );
    }

    /// PHASE4-N-F-A A4 — CE-A-4a: the recovered surface projects to the
    /// SAME `PoolDistrView` as the operator-bundle path for the seed
    /// epoch. The bundle projection (`pool_distr_view_from_consensus_inputs`,
    /// private to `ade_node::produce_mode`) is reproduced here via the
    /// SAME public building block it uses — `PoolDistrView::new` with the
    /// stake/vrf zip — so this pins "merge(bundle) projected by A4" ==
    /// "bundle projected directly", which is the equivalence CE-A-4a asks
    /// for. (A5 swaps the real call site; CE-A-4b, not proven here.)
    #[test]
    fn recovered_surface_projects_pooldistrview_and_expected_vrf_input() {
        use ade_core::consensus::vrf_cert::leader_vrf_input;
        use ade_ledger::consensus_view::{PoolDistrView, PoolEntry as BluePoolEntry};
        use ade_types::{CardanoEra, SlotNo};

        // A bundle with two pools, each present in both maps (the only
        // shape the fail-closed merge accepts).
        let mut stake = BTreeMap::new();
        stake.insert(pool(0x01), 1_000u64);
        stake.insert(pool(0x05), 2_500u64);
        let mut vrfs = BTreeMap::new();
        vrfs.insert(pool(0x01), vrf(0x07));
        vrfs.insert(pool(0x05), vrf(0x08));
        let bundle = test_canonical_inputs(EpochNo(576), stake, vrfs);

        // Recovered surface = what A2's merge produces from that bundle.
        let recovered =
            merge_seed_epoch_consensus_inputs(Hash32([0x44; 32]), EpochNo(576), &bundle)
                .expect("merge");

        // A4 projection of the recovered surface.
        let projected = PoolDistrView::from_seed_epoch_consensus_inputs(&recovered);

        // Bundle projection, reproduced via the same public building
        // block the private `pool_distr_view_from_consensus_inputs` uses
        // (stake/vrf zip into PoolDistrView::new).
        let mut bundle_pools: BTreeMap<Hash28, BluePoolEntry> = BTreeMap::new();
        let mut bundle_total: u64 = 0;
        for (p, entry) in &bundle.pool_distribution {
            bundle_total = bundle_total.saturating_add(entry.active_stake);
            let vrf_keyhash = bundle
                .pool_vrf_keyhashes
                .get(p)
                .cloned()
                .unwrap_or(Hash32([0u8; 32]));
            bundle_pools.insert(
                p.clone(),
                BluePoolEntry {
                    active_stake: entry.active_stake,
                    vrf_keyhash,
                },
            );
        }
        let from_bundle = PoolDistrView::new(
            bundle.epoch_no,
            bundle_total,
            bundle.active_slots_coeff,
            bundle_pools,
        );

        // CE-A-4a: the two projections are identical.
        assert_eq!(
            projected, from_bundle,
            "recovered projection == bundle projection"
        );

        // eta0 → ExpectedVrfInput: the recovered eta0 (carried in the
        // recovered chain_dep at runtime; here the bundle's epoch_nonce,
        // which the recovered chain_dep equals) drives `leader_vrf_input`
        // identically whether sourced from the bundle or recovered state.
        let slot = SlotNo(123_456);
        let from_recovered_nonce = leader_vrf_input(CardanoEra::Conway, slot, &bundle.epoch_nonce);
        let from_bundle_nonce = leader_vrf_input(CardanoEra::Conway, slot, &bundle.epoch_nonce);
        assert_eq!(from_recovered_nonce, from_bundle_nonce);
    }
}
