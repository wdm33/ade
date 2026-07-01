// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN persistent snapshot cache (PHASE4-N-J S8).
//!
//! Bridges `ade_ledger::snapshot::framing::{encode_snapshot,
//! decode_snapshot}` to the `SnapshotStore` trait so the rollback
//! driver can serve `nearest_le` lookups from restart-safe storage.
//! Closes [`ade_ledger::rollback`]'s DC-CONS-21 open obligation.
//!
//! Reader: `nearest_le(target_slot)` walks
//! `SnapshotStore::list_snapshot_slots()` (already ascending), takes
//! the largest ≤ target, reads its bytes via `get_snapshot`, and
//! decodes via `framing::decode_snapshot`. The decoder's embedded
//! fingerprint cross-check + version-tag check guarantee
//! restart-safety on top of any opaque-bytes store.
//!
//! Writer: `capture_persistent(store, slot, ledger, chain_dep)`
//! encodes via `framing::encode_snapshot` and puts the bytes at
//! `slot`. Decode failures or store errors are surfaced as
//! `PersistentCacheError` — the caller (orchestrator) decides
//! whether a missing/corrupt snapshot is fatal or skippable.

use ade_core::consensus::praos_state::PraosChainDepState;
use ade_ledger::rollback::SnapshotReader;
use ade_ledger::snapshot::framing::{decode_snapshot, encode_snapshot, SCHEMA_VERSION};
use ade_ledger::snapshot::{SnapshotDecodeError, SnapshotEncodeError};
use ade_ledger::state::LedgerState;
use ade_types::SlotNo;

use crate::chaindb::{ChainDbError, SnapshotStore};

/// Persistent snapshot cache — pure adapter over any `SnapshotStore`.
///
/// Borrows the store so we never own its lifecycle. Cheap to
/// re-create per lookup; the cache itself holds no in-memory state.
pub struct PersistentSnapshotCache<'a, S: SnapshotStore + ?Sized> {
    store: &'a S,
}

impl<'a, S: SnapshotStore + ?Sized> PersistentSnapshotCache<'a, S> {
    pub fn new(store: &'a S) -> Self {
        Self { store }
    }

    /// Persist `(ledger, chain_dep)` at `slot` via `encode_snapshot`
    /// + `SnapshotStore::put_snapshot`. Conway-only at the encoder
    /// boundary (pre-Conway → `PersistentCacheError::Encode`).
    pub fn capture(
        &self,
        slot: SlotNo,
        ledger: &LedgerState,
        chain_dep: &PraosChainDepState,
    ) -> Result<(), PersistentCacheError> {
        let bytes = encode_snapshot(ledger, chain_dep).map_err(PersistentCacheError::Encode)?;
        self.store
            .put_snapshot(slot, &bytes)
            .map_err(PersistentCacheError::Store)
    }
}

impl<'a, S: SnapshotStore + ?Sized> SnapshotReader for PersistentSnapshotCache<'a, S> {
    fn nearest_le(
        &self,
        target_slot: SlotNo,
    ) -> Option<(SlotNo, LedgerState, PraosChainDepState)> {
        let slots = self.store.list_snapshot_slots().ok()?;
        // BTreeSet via Vec is already ascending per trait contract;
        // pick the largest ≤ target.
        let candidate = slots.iter().rev().find(|s| **s <= target_slot).copied()?;
        let bytes = self.store.get_snapshot(candidate).ok()??;
        let (ledger, chain_dep) = decode_snapshot(&bytes).ok()?;
        Some((candidate, ledger, chain_dep))
    }
}

/// Closed error sum surfaced by the persistent cache. Encode/Decode
/// carry the upstream snapshot errors verbatim; Store carries the
/// upstream `SnapshotStore` error.
#[derive(Debug)]
pub enum PersistentCacheError {
    Encode(SnapshotEncodeError),
    Decode(SnapshotDecodeError),
    Store(ChainDbError),
}

/// Pinned constant: the schema version the persistent cache writes
/// into every snapshot. Mirrors `framing::SCHEMA_VERSION` for
/// out-of-crate consumers that want to assert the cache's wire
/// version without depending on `ade_ledger::snapshot::framing`.
pub const PERSISTENT_CACHE_SCHEMA_VERSION: u32 = SCHEMA_VERSION;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use ade_core::consensus::praos_state::Nonce;
    use ade_ledger::pparams::ConwayOnlyDepositParams;
    use ade_ledger::state::ConwayGovState;
    use ade_types::shelley::cert::StakeCredential;
    use ade_types::tx::Coin;
    use ade_types::{BlockNo, CardanoEra, EpochNo, Hash28, Hash32};
    use std::collections::BTreeMap;

    use crate::chaindb::InMemoryChainDb;
    use crate::rollback::in_memory_cache::InMemorySnapshotCache;

    fn ledger(epoch: u64) -> LedgerState {
        let mut l = LedgerState::new(CardanoEra::Conway);
        l.epoch_state.epoch = EpochNo(epoch);
        l.max_lovelace_supply = 45_000_000_000_000_000;
        l.cert_state.delegation.registrations.insert(
            StakeCredential::KeyHash(Hash28([epoch as u8; 28])),
            Coin(2_000_000),
        );
        l.gov_state = Some(ConwayGovState {
            proposals: Vec::new(),
            committee: BTreeMap::new(),
            committee_quorum: (2, 3),
            drep_expiry: BTreeMap::new(),
            gov_action_lifetime: 6,
            vote_delegations: BTreeMap::new(),
            pool_voting_thresholds: vec![(1, 2)],
            drep_voting_thresholds: vec![(67, 100)],
            committee_hot_keys: BTreeMap::new(),
            num_dormant: ade_ledger::state::DormantEpochs::Unversioned,
        });
        l.conway_deposit_params = Some(ConwayOnlyDepositParams {
            drep_deposit: Coin(500_000_000),
            gov_action_deposit: Coin(100_000_000_000),
            drep_activity: 20,
        });
        l
    }

    fn chain_dep(epoch: u64) -> PraosChainDepState {
        let mut cd = PraosChainDepState::empty();
        cd.epoch_nonce = Nonce(Hash32([epoch as u8; 32]));
        cd.last_epoch_block = Some(EpochNo(epoch));
        cd.last_block_no = Some(BlockNo(epoch * 10));
        cd
    }

    #[test]
    fn persistent_cache_capture_then_nearest_le_round_trips() {
        let store = InMemoryChainDb::new();
        let cache = PersistentSnapshotCache::new(&store);
        cache
            .capture(SlotNo(100), &ledger(576), &chain_dep(576))
            .expect("capture 100");
        cache
            .capture(SlotNo(200), &ledger(577), &chain_dep(577))
            .expect("capture 200");
        cache
            .capture(SlotNo(300), &ledger(578), &chain_dep(578))
            .expect("capture 300");

        let (s, l, cd) = cache.nearest_le(SlotNo(250)).expect("found");
        assert_eq!(s, SlotNo(200));
        assert_eq!(l.epoch_state.epoch, EpochNo(577));
        assert_eq!(cd.epoch_nonce.0 .0[0], 577u16 as u8);

        let (s, l, _) = cache.nearest_le(SlotNo(300)).expect("found");
        assert_eq!(s, SlotNo(300));
        assert_eq!(l.epoch_state.epoch, EpochNo(578));

        let (s, l, _) = cache.nearest_le(SlotNo(99999)).expect("found");
        assert_eq!(s, SlotNo(300));
        assert_eq!(l.epoch_state.epoch, EpochNo(578));

        assert!(cache.nearest_le(SlotNo(50)).is_none());
    }

    #[test]
    fn persistent_cache_matches_in_memory_cache_semantics() {
        let store = InMemoryChainDb::new();
        let persistent = PersistentSnapshotCache::new(&store);
        let mut in_memory = InMemorySnapshotCache::new();

        let cases = [(100u64, 576u64), (200, 577), (300, 578), (450, 579)];
        for (slot, epoch) in cases {
            persistent
                .capture(SlotNo(slot), &ledger(epoch), &chain_dep(epoch))
                .expect("persistent capture");
            in_memory.admit(SlotNo(slot), ledger(epoch), chain_dep(epoch));
        }

        for probe in [50u64, 100, 150, 200, 250, 300, 449, 450, 451, 9999] {
            let p = persistent.nearest_le(SlotNo(probe));
            let m = in_memory.nearest_le(SlotNo(probe));
            match (p, m) {
                (None, None) => {}
                (Some((s1, l1, cd1)), Some((s2, l2, cd2))) => {
                    assert_eq!(s1, s2, "slot disagreement at probe={probe}");
                    assert_eq!(l1, l2, "ledger disagreement at probe={probe}");
                    assert_eq!(cd1, cd2, "chain_dep disagreement at probe={probe}");
                }
                (p, m) => panic!("presence disagreement at probe={probe}: persistent={p:?} in_memory={m:?}"),
            }
        }
    }

    #[test]
    fn persistent_cache_empty_store_returns_none() {
        let store = InMemoryChainDb::new();
        let cache = PersistentSnapshotCache::new(&store);
        assert!(cache.nearest_le(SlotNo(0)).is_none());
        assert!(cache.nearest_le(SlotNo(9999)).is_none());
    }

    #[test]
    fn persistent_cache_rejects_pre_conway_on_capture() {
        let store = InMemoryChainDb::new();
        let cache = PersistentSnapshotCache::new(&store);
        let mut babbage = LedgerState::new(CardanoEra::Babbage);
        babbage.max_lovelace_supply = 1;
        match cache.capture(SlotNo(100), &babbage, &chain_dep(576)) {
            Err(PersistentCacheError::Encode(SnapshotEncodeError::EraNotSupported { era })) => {
                assert_eq!(era, CardanoEra::Babbage);
            }
            other => panic!("expected Encode/EraNotSupported, got {other:?}"),
        }
    }

    #[test]
    fn persistent_cache_corrupt_bytes_yields_none_from_reader() {
        let store = InMemoryChainDb::new();
        let cache = PersistentSnapshotCache::new(&store);
        // Inject corrupt bytes directly.
        store
            .put_snapshot(SlotNo(200), &[0xFF, 0xFE, 0xFD])
            .expect("put corrupt");
        // Reader treats decode failure as "no usable snapshot here" —
        // surfaces None rather than panicking.
        assert!(cache.nearest_le(SlotNo(300)).is_none());
    }

    #[test]
    fn persistent_cache_schema_version_mirrors_framing() {
        assert_eq!(
            PERSISTENT_CACHE_SCHEMA_VERSION,
            ade_ledger::snapshot::framing::SCHEMA_VERSION
        );
    }
}
