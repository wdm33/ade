//! EPOCH-CONSENSUS-VIEW S3f-4d-2 — derive the activation candidate from a VALIDATED source
//! window.
//!
//! `checkpoint commitment + a validated durable ChainDB window = candidate
//! EpochConsensusView`. The candidate is bound to the window's TARGET-epoch context (the
//! Mark→Set leadership lag, DC-EPOCH-08), and its identity is exactly what the WAL
//! activation record (S3f-4a) commits to and recovery (S3f-4c) reproduces. Candidate
//! binding happens BEFORE WAL activation (constraint 3).
//!
//! RED: it drives the durable redb checkpoint. The candidate contents are a pure function
//! of `(checkpoint, bootstrap cert state, the window's blocks, era, network, nonce)` -- no
//! peer/network read, wall-clock, or async side channel (constraint 6).

use crate::epoch_source_window::ActivationSourceWindow;
use ade_core::consensus::events::Point;
use ade_ledger::reduced_epoch_view::EpochConsensusView;
use ade_ledger::state::LedgerState;
use ade_runtime::chaindb::{
    drive_window_aggregate, ReducedCheckpointError, ReducedUtxoCheckpoint, WindowDriverError,
};
use ade_types::shelley::block::ShelleyBlock;
use ade_types::{CardanoEra, Hash32};

/// Why a candidate could not be derived (fail closed -- no candidate, so the predicate
/// (S3f-4b) never sees a partial one).
#[derive(Debug)]
pub enum CandidateDeriveError {
    /// The window drive (reduce + advance + aggregate) failed.
    Drive(WindowDriverError),
    /// The reduced-UTxO checkpoint commitment could not be read.
    Checkpoint(ReducedCheckpointError),
}

/// Derive the activation candidate from a validated source window: drive the reduced
/// checkpoint + cert state forward over the window's blocks (DC-EVIEW-10) -> per-pool stake,
/// then bind it into an EpochConsensusView with the window's TARGET-epoch context. The
/// caller MUST have validated the window (DC-EPOCH-08) first; this binds the result so the
/// candidate's identity is the durable activation identity.
#[allow(clippy::too_many_arguments)]
pub fn derive_candidate(
    window: &ActivationSourceWindow,
    checkpoint: &ReducedUtxoCheckpoint,
    bootstrap_state: &LedgerState,
    blocks: &[ShelleyBlock],
    era: CardanoEra,
    network_magic: u32,
    nonce: Hash32,
) -> Result<EpochConsensusView, CandidateDeriveError> {
    let stake = drive_window_aggregate(checkpoint, bootstrap_state, blocks, era)
        .map_err(CandidateDeriveError::Drive)?;
    // The window drive applies block deltas, which clear the completeness marker; finalize
    // re-marks the (window-end) checkpoint complete AND returns its commitment -- the source
    // checkpoint commitment the candidate is bound to.
    let checkpoint_commitment = checkpoint
        .finalize()
        .map_err(CandidateDeriveError::Checkpoint)?;
    Ok(EpochConsensusView::bind(
        network_magic,
        era,
        window.target_epoch,
        Point { slot: window.source_window_end, hash: window.lineage_pin.clone() },
        checkpoint_commitment,
        nonce,
        window.snapshot_phase,
        stake.pool_stakes,
        stake.total_active_stake,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epoch_activation::{activation_record_for, recover_active_view, ActiveEpochView};
    use crate::epoch_source_window::target_epoch_for_source;
    use ade_ledger::reduced_snapshot::SnapshotPhase;
    use ade_ledger::reduced_utxo::ReducedStakeRef;
    use ade_types::primitives::SlotNo;
    use ade_types::shelley::cert::StakeCredential;
    use ade_types::tx::{Coin, PoolId, TxIn};
    use ade_types::{EpochNo, Hash28};
    use std::collections::BTreeMap;

    const RAW_CONWAY_BLOCK: &[u8] = include_bytes!("../tests/fixtures/raw_era_block_conway.cbor");

    fn conway_block() -> ShelleyBlock {
        let env = ade_codec::cbor::envelope::decode_block_envelope(RAW_CONWAY_BLOCK).expect("env");
        let inner = &RAW_CONWAY_BLOCK[env.block_start..env.block_end];
        ade_codec::conway::decode_conway_block(inner).expect("decode").decoded().clone()
    }

    fn window() -> ActivationSourceWindow {
        ActivationSourceWindow {
            source_epoch: EpochNo(575),
            source_window_start: SlotNo(0),
            source_window_end: SlotNo(1000),
            snapshot_phase: SnapshotPhase::Set,
            target_epoch: target_epoch_for_source(EpochNo(575), SnapshotPhase::Set).unwrap(),
            source_window_anchor: Hash32([0x00; 32]),
            lineage_pin: Hash32([0xab; 32]),
        }
    }

    // The candidate is bound to the window's TARGET-epoch context, and its identity
    // round-trips through the WAL activation record + recovery -- tying the candidate
    // derivation (S3f-4d-2) to the durable record (S3f-4a) and recovery (S3f-4c).
    #[test]
    fn derive_candidate_binds_target_epoch_and_round_trips_through_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let cp = ReducedUtxoCheckpoint::open(&dir.path().join("rc.redb")).unwrap();
        let mut reduced: BTreeMap<TxIn, (Coin, ReducedStakeRef)> = BTreeMap::new();
        reduced.insert(
            TxIn { tx_hash: Hash32([7; 32]), index: 0 },
            (Coin(5_000_000), ReducedStakeRef::Base(StakeCredential::KeyHash(Hash28([0xc; 28])))),
        );
        cp.build_from(&reduced).unwrap();
        let mut state = LedgerState::new(CardanoEra::Conway);
        state
            .cert_state
            .delegation
            .delegations
            .insert(StakeCredential::KeyHash(Hash28([0xc; 28])), PoolId(Hash28([0x9; 28])));

        let w = window();
        let block = conway_block();
        let candidate = derive_candidate(
            &w,
            &cp,
            &state,
            std::slice::from_ref(&block),
            CardanoEra::Conway,
            2,
            Hash32([0x42; 32]),
        )
        .expect("derive");

        // bound to the TARGET epoch + the window's context.
        assert_eq!(candidate.epoch, w.target_epoch);
        assert_eq!(candidate.snapshot_phase, SnapshotPhase::Set);
        assert_eq!(candidate.source_point.hash, w.lineage_pin);
        assert_eq!(candidate.network_magic, 2);
        assert!(candidate.verify_canonical_hash());

        // the candidate's identity is the durable activation identity (round-trips).
        let rec = activation_record_for(&candidate);
        assert_eq!(
            recover_active_view(Some(&rec), Some(&candidate)),
            Ok(ActiveEpochView::Promoted(candidate.clone()))
        );
    }
}
