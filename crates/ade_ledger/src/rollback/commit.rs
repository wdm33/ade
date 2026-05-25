// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE atomic rollback commit (PHASE4-N-I S3).
//!
//! Sequence (staged-then-committed):
//!   1. `chain_write.rollback_to_slot(target.slot)` — irreversible
//!      step; failure leaves receive state unchanged.
//!   2. `state.ledger = new_ledger` (infallible).
//!   3. `state.chain_dep = new_chain_dep` (infallible).
//!   4. `state.pending_headers = PendingHeaderCache::new()` —
//!      post-rollback cached headers are stale.

use ade_core::consensus::praos_state::PraosChainDepState;

use crate::receive::{ChainDbWrite, PendingHeaderCache, ReceiveState};
use crate::state::LedgerState;

use super::error::CommitRollbackError;
use super::materialize::TargetPoint;

/// Commit a materialized rollback. Pure logic + one trait call.
/// Returns `Ok(())` on success; on failure the state is unchanged.
pub fn commit_rollback<W: ChainDbWrite>(
    state: &mut ReceiveState,
    target: TargetPoint,
    new_ledger: LedgerState,
    new_chain_dep: PraosChainDepState,
    chain_write: &mut W,
) -> Result<(), CommitRollbackError> {
    // 1. Irreversible step first.
    chain_write
        .rollback_to_slot(target.slot)
        .map_err(CommitRollbackError::ChainDb)?;
    // 2-4. Infallible commits.
    state.ledger = new_ledger;
    state.chain_dep = new_chain_dep;
    state.pending_headers = PendingHeaderCache::new();
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use ade_types::{CardanoEra, EpochNo, Hash32, SlotNo};

    use crate::receive::{AdmittedBlock, ChainWriteError, ChainWriteErrorKind};

    fn fresh_state() -> ReceiveState {
        let mut ledger = LedgerState::new(CardanoEra::Conway);
        ledger.epoch_state.epoch = EpochNo(576);
        let chain_dep = PraosChainDepState::empty();
        ReceiveState::new(ledger, chain_dep)
    }

    fn ledger_fp(state: &LedgerState) -> Hash32 {
        crate::fingerprint::fingerprint(state).combined
    }

    /// Mock chain_write that records calls and can be set to fail.
    struct RecordingChainWrite {
        rollbacks: Vec<SlotNo>,
        fail_next_rollback: bool,
    }

    impl ChainDbWrite for RecordingChainWrite {
        fn write_admitted(&mut self, _block: AdmittedBlock) -> Result<(), ChainWriteError> {
            Ok(())
        }
        fn rollback_to_slot(&mut self, slot: SlotNo) -> Result<(), ChainWriteError> {
            if self.fail_next_rollback {
                self.fail_next_rollback = false;
                return Err(ChainWriteError::Underlying(ChainWriteErrorKind::Io));
            }
            self.rollbacks.push(slot);
            Ok(())
        }
    }

    fn make_new_ledger() -> LedgerState {
        // A ledger that differs from fresh by changing the epoch.
        let mut l = LedgerState::new(CardanoEra::Conway);
        l.epoch_state.epoch = EpochNo(577);
        l
    }

    fn make_new_chain_dep() -> PraosChainDepState {
        let mut s = PraosChainDepState::empty();
        s.epoch_nonce = ade_core::consensus::Nonce(Hash32([0xAB; 32]));
        s
    }

    #[test]
    fn commit_rollback_advances_chaindb_and_ledger_atomically() {
        let mut state = fresh_state();
        let pre_fp = ledger_fp(&state.ledger);
        // Seed a pending header so we can verify the reset.
        state
            .pending_headers
            .insert(SlotNo(99), Hash32([0xCC; 32]), vec![0xFF])
            .expect("insert");
        assert_eq!(state.pending_headers.len(), 1);

        let mut writer = RecordingChainWrite {
            rollbacks: Vec::new(),
            fail_next_rollback: false,
        };
        let target = TargetPoint {
            slot: SlotNo(50),
            hash: Hash32([0; 32]),
        };
        commit_rollback(
            &mut state,
            target.clone(),
            make_new_ledger(),
            make_new_chain_dep(),
            &mut writer,
        )
        .expect("commit");
        assert_eq!(writer.rollbacks, vec![SlotNo(50)]);
        assert_ne!(ledger_fp(&state.ledger), pre_fp);
        assert_eq!(state.chain_dep, make_new_chain_dep());
        assert!(state.pending_headers.is_empty(), "pending cache must reset");
    }

    #[test]
    fn commit_rollback_chain_write_failure_leaves_state_unchanged() {
        let mut state = fresh_state();
        let pre_fp = ledger_fp(&state.ledger);
        let pre_chain_dep = state.chain_dep.clone();
        state
            .pending_headers
            .insert(SlotNo(99), Hash32([0xCC; 32]), vec![0xFF])
            .expect("insert");
        let pre_pending_len = state.pending_headers.len();

        let mut writer = RecordingChainWrite {
            rollbacks: Vec::new(),
            fail_next_rollback: true,
        };
        let target = TargetPoint {
            slot: SlotNo(50),
            hash: Hash32([0; 32]),
        };
        let err = commit_rollback(
            &mut state,
            target,
            make_new_ledger(),
            make_new_chain_dep(),
            &mut writer,
        )
        .expect_err("must fail");
        match err {
            CommitRollbackError::ChainDb(_) => {}
        }
        // State unchanged.
        assert_eq!(ledger_fp(&state.ledger), pre_fp);
        assert_eq!(state.chain_dep, pre_chain_dep);
        assert_eq!(state.pending_headers.len(), pre_pending_len);
        assert!(writer.rollbacks.is_empty());
    }

    #[test]
    fn commit_rollback_resets_pending_headers() {
        let mut state = fresh_state();
        for i in 0..5 {
            state
                .pending_headers
                .insert(SlotNo(i), Hash32([i as u8; 32]), vec![i as u8])
                .expect("insert");
        }
        assert_eq!(state.pending_headers.len(), 5);

        let mut writer = RecordingChainWrite {
            rollbacks: Vec::new(),
            fail_next_rollback: false,
        };
        let target = TargetPoint {
            slot: SlotNo(2),
            hash: Hash32([0; 32]),
        };
        commit_rollback(
            &mut state,
            target,
            make_new_ledger(),
            make_new_chain_dep(),
            &mut writer,
        )
        .expect("commit");
        assert_eq!(state.pending_headers.len(), 0);
    }
}
