// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// CE-B1-1 close — builds the production PoolDistrView from the real
// Conway-576 corpus and asserts it surfaces the correct
// (sigma, total_active_stake, vrf_keyhash, asc) for every issuing pool,
// returns None for unknown pool/epoch, and answers identically on repeat
// queries (purity). The view is consumed only through the BLUE
// `ade_core::consensus::LedgerView` trait.
//
// Integration test (compiled separately from the BLUE library crate).

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_testkit::validity::{pool_distr_view_from_corpus, ConwayValidityCorpus};
use ade_types::{EpochNo, Hash28, Hash32};

const EPOCH: EpochNo = EpochNo(576);

fn corpus() -> ConwayValidityCorpus {
    ConwayValidityCorpus::load().expect("corpus loads")
}

#[test]
fn view_returns_corpus_pool_stake_and_vrf_keyhash() {
    let corpus = corpus();
    let total = corpus.pd_total_active_stake;
    let view = pool_distr_view_from_corpus(&corpus, EPOCH).expect("view builds");

    assert_eq!(corpus.pools.len(), 14, "corpus must hold the 14 issuing pools");

    // Total active stake is the shared pdTotalActiveStake denominator.
    assert_eq!(view.total_active_stake(EPOCH), Some(total));

    // ASC = 1/20 (mainnet constant).
    assert_eq!(
        view.active_slots_coeff(EPOCH),
        Some(ActiveSlotsCoeff { numer: 1, denom: 20 })
    );

    for (pool_id, pool) in &corpus.pools {
        let key = Hash28(*pool_id);
        let stake = view
            .pool_active_stake(EPOCH, &key)
            .expect("known pool has stake");
        let total_v = view.total_active_stake(EPOCH).expect("total present");

        // sigma is preserved as the reduced fraction `numer/denom`; the view
        // normalizes to the shared total. The rational must be preserved:
        //   stake / total == sigma.numer / sigma.denom
        // checked by exact cross-multiplication (no float).
        assert_eq!(
            (stake as u128) * (pool.sigma.denom as u128),
            (pool.sigma.numer as u128) * (total_v as u128),
            "sigma not preserved for pool {pool_id:?}"
        );

        // VRF keyhash surfaced verbatim from the corpus.
        assert_eq!(
            view.pool_vrf_keyhash(EPOCH, &key),
            Some(Hash32(pool.vrf_keyhash)),
            "vrf_keyhash mismatch for pool {pool_id:?}"
        );
    }
}

#[test]
fn view_unknown_pool_returns_none() {
    let corpus = corpus();
    let view = pool_distr_view_from_corpus(&corpus, EPOCH).expect("view builds");
    let unknown = Hash28([0xFF; 28]);
    assert!(!corpus.pools.contains_key(&unknown.0));
    assert_eq!(view.pool_active_stake(EPOCH, &unknown), None);
    assert_eq!(view.pool_vrf_keyhash(EPOCH, &unknown), None);
}

#[test]
fn view_unknown_epoch_returns_none() {
    let corpus = corpus();
    let view = pool_distr_view_from_corpus(&corpus, EPOCH).expect("view builds");
    let other = EpochNo(577);
    let any_pool = Hash28(*corpus.pools.keys().next().expect("at least one pool"));

    assert_eq!(view.total_active_stake(other), None);
    assert_eq!(view.pool_active_stake(other, &any_pool), None);
    assert_eq!(view.pool_vrf_keyhash(other, &any_pool), None);
    assert_eq!(view.active_slots_coeff(other), None);
}

#[test]
fn view_is_pure() {
    let corpus = corpus();
    let view = pool_distr_view_from_corpus(&corpus, EPOCH).expect("view builds");
    let any_pool = Hash28(*corpus.pools.keys().next().expect("at least one pool"));

    // Two identical query sequences must yield byte-identical answers.
    let first = (
        view.total_active_stake(EPOCH),
        view.pool_active_stake(EPOCH, &any_pool),
        view.pool_vrf_keyhash(EPOCH, &any_pool),
        view.active_slots_coeff(EPOCH),
    );
    let second = (
        view.total_active_stake(EPOCH),
        view.pool_active_stake(EPOCH, &any_pool),
        view.pool_vrf_keyhash(EPOCH, &any_pool),
        view.active_slots_coeff(EPOCH),
    );
    assert_eq!(first, second);
}
