// GREEN — test-only `LedgerView` implementation backed by an in-memory
// `BTreeMap`. Used by `ade_core/tests/leader_schedule_corpus.rs` and any
// future N-B replay tests that need a deterministic, corpus-driven
// stake snapshot without depending on `ade_ledger`.
//
// Non-authoritative: this stub never affects production consensus
// outputs. It exists purely so BLUE consensus has a concrete `&dyn
// LedgerView` to consume in integration tests.
//
// BTreeMap — never HashMap — because the slice-spec hard-prohibition on
// HashMap applies to the testkit stub too: deterministic iteration
// order is the only acceptable shape, even in tests.

use std::collections::BTreeMap;

use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_crypto::vrf::VrfVerificationKey;
use ade_types::{EpochNo, Hash28};

/// Per-epoch fixture: the four facts a `LedgerView` must surface.
#[derive(Debug, Clone)]
pub struct EpochStakeFixture {
    pub total_active_stake: u64,
    pub asc: ActiveSlotsCoeff,
    pub pools: BTreeMap<Hash28, PoolFixture>,
}

/// One pool's slice of an epoch fixture.
#[derive(Debug, Clone)]
pub struct PoolFixture {
    pub active_stake: u64,
    pub vrf_key: VrfVerificationKey,
}

/// In-memory `LedgerView`. Construct via `LedgerViewStub::new()` and
/// `with_epoch` to load fixtures.
#[derive(Debug, Clone, Default)]
pub struct LedgerViewStub {
    epochs: BTreeMap<EpochNo, EpochStakeFixture>,
}

impl LedgerViewStub {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a complete epoch fixture. Last call wins for a given epoch.
    pub fn with_epoch(mut self, epoch: EpochNo, fixture: EpochStakeFixture) -> Self {
        self.epochs.insert(epoch, fixture);
        self
    }
}

impl LedgerView for LedgerViewStub {
    fn total_active_stake(&self, epoch: EpochNo) -> Option<u64> {
        self.epochs.get(&epoch).map(|f| f.total_active_stake)
    }

    fn pool_active_stake(&self, epoch: EpochNo, pool: &Hash28) -> Option<u64> {
        self.epochs
            .get(&epoch)
            .and_then(|f| f.pools.get(pool))
            .map(|p| p.active_stake)
    }

    fn pool_vrf_key(&self, epoch: EpochNo, pool: &Hash28) -> Option<VrfVerificationKey> {
        self.epochs
            .get(&epoch)
            .and_then(|f| f.pools.get(pool))
            .map(|p| p.vrf_key.clone())
    }

    fn active_slots_coeff(&self, epoch: EpochNo) -> Option<ActiveSlotsCoeff> {
        self.epochs.get(&epoch).map(|f| f.asc)
    }
}
