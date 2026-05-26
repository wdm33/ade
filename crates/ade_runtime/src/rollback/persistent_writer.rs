// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN persistent-snapshot writer (PHASE4-N-K S3).
//!
//! Cadence-fidelity glue between the orchestrator's `CaptureSnapshot`
//! effect (S2) and `PersistentSnapshotCache::capture` (N-J S8). The
//! writer NEVER decides cadence on its own — every "capture or skip"
//! decision routes through `should_snapshot_after_block`. Snapshot
//! eviction is out of scope per the PHASE4-N-K invariants sketch.
//!
//! DC-NODE-02: enforced by
//! `ci/ci_check_persistent_writer_no_parallel_cadence.sh`.

use ade_core::consensus::praos_state::PraosChainDepState;
use ade_ledger::state::LedgerState;
use ade_types::{BlockNo, SlotNo};

use crate::chaindb::SnapshotStore;
use crate::rollback::cadence::{should_snapshot_after_block, SnapshotCadence};
use crate::rollback::persistent_cache::{PersistentCacheError, PersistentSnapshotCache};

/// Cadence-disciplined persistent-snapshot writer. Borrows the
/// `SnapshotStore`; owns its own cadence + last-capture tracker so
/// the orchestrator can re-issue captures idempotently.
pub struct PersistentSnapshotWriter<'a, S: SnapshotStore + ?Sized> {
    cache: PersistentSnapshotCache<'a, S>,
    cadence: SnapshotCadence,
    last_capture: Option<SlotNo>,
}

impl<'a, S: SnapshotStore + ?Sized> PersistentSnapshotWriter<'a, S> {
    pub fn new(store: &'a S, cadence: SnapshotCadence) -> Self {
        Self {
            cache: PersistentSnapshotCache::new(store),
            cadence,
            last_capture: None,
        }
    }

    pub fn cadence(&self) -> SnapshotCadence {
        self.cadence
    }

    pub fn last_captured_slot(&self) -> Option<SlotNo> {
        self.last_capture
    }

    /// On admission of `block_no` at `slot`, consult the cadence
    /// policy. If it says capture, write a snapshot via the
    /// `PersistentSnapshotCache`. Returns `Ok(true)` if a snapshot
    /// was captured; `Ok(false)` otherwise.
    ///
    /// Authority-fatal: `PersistentCacheError::Store` carrying
    /// `ChainDbError::Io(_)` should be handled by the orchestrator
    /// as authority-fatal (DC-NODE-04). The writer surfaces the
    /// error unchanged; routing is the caller's responsibility.
    pub fn on_admitted(
        &mut self,
        slot: SlotNo,
        block_no: BlockNo,
        ledger: &LedgerState,
        chain_dep: &PraosChainDepState,
    ) -> Result<bool, PersistentCacheError> {
        if !should_snapshot_after_block(slot, block_no, self.cadence, self.last_capture) {
            return Ok(false);
        }
        self.cache.capture(slot, ledger, chain_dep)?;
        self.last_capture = Some(slot);
        Ok(true)
    }

    /// Force a capture at `slot` regardless of cadence. Used by
    /// shutdown drain (DC-NODE-04) to ensure the final
    /// `(ledger, chain_dep)` is persisted before exit. Also
    /// updates `last_capture`.
    pub fn force_capture(
        &mut self,
        slot: SlotNo,
        ledger: &LedgerState,
        chain_dep: &PraosChainDepState,
    ) -> Result<(), PersistentCacheError> {
        self.cache.capture(slot, ledger, chain_dep)?;
        self.last_capture = Some(slot);
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use ade_core::consensus::praos_state::Nonce;
    use ade_ledger::pparams::ConwayOnlyDepositParams;
    use ade_ledger::state::ConwayGovState;
    use ade_types::shelley::cert::StakeCredential;
    use ade_types::tx::Coin;
    use ade_types::{BlockNo, CardanoEra, EpochNo, Hash28, Hash32};

    use crate::chaindb::InMemoryChainDb;

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
    fn persistent_writer_on_admitted_captures_only_on_cadence() {
        let store = InMemoryChainDb::new();
        let mut writer =
            PersistentSnapshotWriter::new(&store, SnapshotCadence { every_n_blocks: 10 });
        // Drive 30 admissions at blocks 1..=30; cadence=10 → captures
        // at blocks 10, 20, 30.
        let mut captured_slots: Vec<SlotNo> = Vec::new();
        for n in 1u64..=30 {
            let slot = SlotNo(n * 100);
            let captured = writer
                .on_admitted(slot, BlockNo(n), &ledger(576), &chain_dep(576 + n))
                .expect("on_admitted");
            if captured {
                captured_slots.push(slot);
            }
        }
        assert_eq!(
            captured_slots,
            vec![SlotNo(1000), SlotNo(2000), SlotNo(3000)]
        );
        assert_eq!(writer.last_captured_slot(), Some(SlotNo(3000)));
    }

    #[test]
    fn persistent_writer_round_trips_via_framing() {
        use ade_ledger::rollback::SnapshotReader;

        let store = InMemoryChainDb::new();
        let mut writer =
            PersistentSnapshotWriter::new(&store, SnapshotCadence { every_n_blocks: 1 });
        let original_ledger = ledger(577);
        let original_chain_dep = chain_dep(577);
        writer
            .on_admitted(SlotNo(500), BlockNo(1), &original_ledger, &original_chain_dep)
            .expect("capture");

        // Read back via a fresh PersistentSnapshotCache.
        let reader = PersistentSnapshotCache::new(&store);
        let (slot, ledger_back, chain_dep_back) =
            reader.nearest_le(SlotNo(500)).expect("nearest_le");
        assert_eq!(slot, SlotNo(500));
        assert_eq!(ledger_back, original_ledger);
        assert_eq!(chain_dep_back, original_chain_dep);
    }

    #[test]
    fn persistent_writer_force_capture_skips_cadence_but_updates_state() {
        let store = InMemoryChainDb::new();
        let mut writer =
            PersistentSnapshotWriter::new(&store, SnapshotCadence { every_n_blocks: 100 });
        // Block 1 is off-cadence; force_capture writes anyway.
        writer
            .force_capture(SlotNo(50), &ledger(576), &chain_dep(576))
            .expect("force");
        assert_eq!(writer.last_captured_slot(), Some(SlotNo(50)));
        // A subsequent on_admitted at block 100 (on cadence) should
        // still capture because the slot is greater than last_capture.
        let captured = writer
            .on_admitted(SlotNo(200), BlockNo(100), &ledger(577), &chain_dep(577))
            .expect("on_admitted");
        assert!(captured);
        assert_eq!(writer.last_captured_slot(), Some(SlotNo(200)));
    }

    #[test]
    fn persistent_writer_two_runs_are_deterministic() {
        let run = || -> Vec<SlotNo> {
            let store = InMemoryChainDb::new();
            let mut writer =
                PersistentSnapshotWriter::new(&store, SnapshotCadence { every_n_blocks: 5 });
            let mut out = Vec::new();
            for n in 1u64..=20 {
                if writer
                    .on_admitted(SlotNo(n * 10), BlockNo(n), &ledger(576), &chain_dep(576))
                    .expect("on_admitted")
                {
                    out.push(SlotNo(n * 10));
                }
            }
            out
        };
        let a = run();
        let b = run();
        assert_eq!(a, b);
        assert_eq!(a.len(), 4); // blocks 5,10,15,20
    }

    #[test]
    fn persistent_writer_on_admitted_off_cadence_is_no_op() {
        let store = InMemoryChainDb::new();
        let mut writer =
            PersistentSnapshotWriter::new(&store, SnapshotCadence { every_n_blocks: 7 });
        let captured = writer
            .on_admitted(SlotNo(100), BlockNo(3), &ledger(576), &chain_dep(576))
            .expect("on_admitted");
        assert!(!captured);
        assert!(writer.last_captured_slot().is_none());
    }
}
