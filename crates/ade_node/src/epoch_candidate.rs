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

use std::collections::BTreeMap;

use crate::epoch_source_window::ActivationSourceWindow;
use ade_core::consensus::events::Point;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_ledger::reduced_epoch_view::{consensus_profile_commitment, EpochConsensusView};
use ade_ledger::state::LedgerState;
use ade_runtime::chaindb::{
    drive_window_consensus_inputs, ReducedCheckpointError, ReducedUtxoCheckpoint, WindowDriverError,
};
use ade_types::shelley::block::ShelleyBlock;
use ade_types::tx::{Coin, PoolId};
use ade_types::{CardanoEra, EpochNo, Hash32};

/// Why a candidate could not be derived (fail closed -- no candidate, so the predicate
/// (S3f-4b) never sees a partial one).
#[derive(Debug)]
pub enum CandidateDeriveError {
    /// The window drive (reduce + advance + aggregate) failed.
    Drive(WindowDriverError),
    /// The reduced-UTxO checkpoint commitment could not be read.
    Checkpoint(ReducedCheckpointError),
    /// The recomputed kept-pool total overflowed u64 (unreachable under the Cardano supply bound;
    /// never silently wrapped) — a structured failure, no partial candidate.
    Overflow,
    /// Option B (B3c / DC-EPOCH-18): the seed+2 authority window requires the snapshot-bound bootstrap
    /// reward update, but it was absent (a legacy / non-native-bootstrap store). Fail closed — no
    /// native-bootstrap route derives the seed+2 authority without the reward distribution.
    BootstrapRewardUpdateAbsent { seed: EpochNo },
    /// Option B (B3c / DC-EPOCH-18): the bootstrap reward update is present but bound to a different
    /// seed epoch than this window's (a provenance mismatch). Fail closed.
    BootstrapRewardUpdateEpochMismatch { bound: EpochNo, seed: EpochNo },
}

/// The canonical leadership consensus profile bound into the candidate (ECA-0b): the source-epoch
/// length (for boundary detection in the window replay) + the genesis / protocol-params / ASC the
/// `protocol_params_commitment` is computed ONCE from. All values are canonical + already bound
/// (carried in `EviewActivationInputs`); candidate derivation performs NO filesystem/config/network
/// read.
pub struct CandidateProfile {
    pub slots_per_epoch: u64,
    pub genesis_hash: Hash32,
    pub protocol_params_hash: Hash32,
    pub asc: ActiveSlotsCoeff,
    /// Option B (B3c): the snapshot-bound bootstrap reward update, carried on the profile so BOTH the
    /// live `activate_at_boundary` and the warm-start `recover_at_boundary` apply it IDENTICALLY at the
    /// window-end of the seed+2 authority replay (replay-equivalence). `None` for every non-native /
    /// non-seed-window path; `derive_candidate` additionally gates the apply on
    /// `target_epoch == window.source_epoch`, so a carried update is a strict no-op off its bound window.
    pub bootstrap_reward_update:
        Option<ade_ledger::bootstrap_reward_update::BootstrapRewardUpdate>,
    /// Option B (B3c / DC-EPOCH-18): the seed epoch. The seed+2 authority window is the one whose
    /// `source_epoch == seed_epoch` (`compute_first_window_bounds(seed_epoch)`), so `derive_candidate`
    /// uses this to decide — MECHANICALLY, at the single derivation site — whether the bootstrap reward
    /// update is REQUIRED (seed+2 window: absent/wrong-epoch is terminal) or irrelevant (any other
    /// window: a carried update is a strict no-op).
    pub seed_epoch: EpochNo,
}

/// Derive the LEADERSHIP-COMPLETE activation candidate from a validated source window (ECA-0b):
/// drive the reduced checkpoint + cert state forward over the window's blocks (DC-EVIEW-10/DC-EVIEW-13)
/// -> the window-end `{stake, pool_params}`, then build the candidate's pool set by the cardano-faithful
/// intersection `delegated ∩ registered` (a delegated-but-unregistered pool is DROPPED, matching
/// cardano's snapshot-build), attaching each kept pool's effective VRF keyhash and recomputing the
/// total over the kept set. Binds it into an EpochConsensusView with the window's TARGET-epoch context
/// + the consensus-profile commitment (computed ONCE from the canonical profile). Returns a
/// leadership-complete view (stake key set == VRF key set) or a structured failure — never a partial /
/// best-effort view. The caller MUST have validated the window (DC-EPOCH-08) first.
#[allow(clippy::too_many_arguments)]
pub fn derive_candidate(
    window: &ActivationSourceWindow,
    checkpoint: &ReducedUtxoCheckpoint,
    bootstrap_state: &LedgerState,
    blocks: &[ShelleyBlock],
    era: CardanoEra,
    network_magic: u32,
    nonce: Hash32,
    profile: &CandidateProfile,
) -> Result<EpochConsensusView, CandidateDeriveError> {
    // Option B (B3c, DC-EPOCH-18): the SINGLE mechanical fail-closed site for the bootstrap reward
    // update. The seed+2 authority window (source epoch == the seed epoch) MUST apply the snapshot-bound
    // update at the window-end; absent or wrong-epoch-bound is TERMINAL here -- no native-bootstrap route
    // derives this authority via the rejected accidental-correctness path. Because EVERY caller that
    // derives the seed+2 authority (the activate seam, prepare_authority, and the warm-start recover)
    // routes through this one function, the enforcement is local to the derivation, never replicated in
    // (and never able to drift across) the callers. A non-seed+2 window never applies it -- a carried
    // update is a strict no-op. activate + recover share the profile, so they derive byte-identically.
    let is_seed2 = window.source_epoch == profile.seed_epoch;
    let bootstrap_reward_delta = match (profile.bootstrap_reward_update.as_ref(), is_seed2) {
        (Some(rupd), true) if rupd.target_epoch == profile.seed_epoch => Some(&rupd.reward_delta),
        (Some(rupd), true) => {
            return Err(CandidateDeriveError::BootstrapRewardUpdateEpochMismatch {
                bound: rupd.target_epoch,
                seed: profile.seed_epoch,
            })
        }
        (None, true) => {
            return Err(CandidateDeriveError::BootstrapRewardUpdateAbsent {
                seed: profile.seed_epoch,
            })
        }
        (_, false) => None,
    };
    let inputs = drive_window_consensus_inputs(
        checkpoint,
        bootstrap_state,
        blocks,
        era,
        profile.slots_per_epoch,
        bootstrap_reward_delta,
    )
    .map_err(CandidateDeriveError::Drive)?;
    // The window drive applies block deltas, which clear the completeness marker; finalize
    // re-marks the (window-end) checkpoint complete AND returns its commitment -- the source
    // checkpoint commitment the candidate is bound to.
    let checkpoint_commitment = checkpoint
        .finalize()
        .map_err(CandidateDeriveError::Checkpoint)?;
    // kept = delegated (stake) ∩ registered (pool_params). A delegated-but-unregistered pool is
    // dropped (cardano silently drops stake delegated to a pool absent from the snapshot's params).
    // Every kept pool carries its effective VRF keyhash -> the view is leadership-complete by
    // construction (DC-EVIEW-12). The total is recomputed over the kept set.
    let mut stake_by_pool: BTreeMap<PoolId, Coin> = BTreeMap::new();
    let mut pool_vrf_keyhashes: BTreeMap<PoolId, Hash32> = BTreeMap::new();
    let mut total: u64 = 0;
    for (pool, coin) in &inputs.stake.pool_stakes {
        if let Some(params) = inputs.pool_params.get(pool) {
            stake_by_pool.insert(pool.clone(), *coin);
            pool_vrf_keyhashes.insert(pool.clone(), params.vrf_hash.clone());
            total = total
                .checked_add(coin.0)
                .ok_or(CandidateDeriveError::Overflow)?;
        }
    }
    let protocol_params_commitment = consensus_profile_commitment(
        &profile.genesis_hash,
        &profile.protocol_params_hash,
        profile.asc,
    );
    Ok(EpochConsensusView::bind(
        network_magic,
        era,
        window.target_epoch,
        Point { slot: window.source_window_end, hash: window.lineage_pin.clone() },
        checkpoint_commitment,
        nonce,
        window.snapshot_phase,
        stake_by_pool,
        pool_vrf_keyhashes,
        Coin(total),
        protocol_params_commitment,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epoch_activation::{activation_record_for, recover_active_view, ActiveEpochView};
    use crate::epoch_source_window::target_epoch_for_source;
    use ade_ledger::delegation::PoolParams;
    use ade_ledger::reduced_snapshot::SnapshotPhase;
    use ade_ledger::bootstrap_reward_update::BootstrapRewardUpdate as BootstrapRewardUpdateT;
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
        // register pool 0x9 so the delegated cred's stake is KEPT by the delegated ∩ registered
        // intersection, and its VRF is frozen into the candidate (leadership-complete).
        state.cert_state.pool.pools.insert(
            PoolId(Hash28([0x9; 28])),
            PoolParams {
                pool_id: PoolId(Hash28([0x9; 28])),
                vrf_hash: Hash32([0x9e; 32]),
                pledge: Coin(0),
                cost: Coin(0),
                margin: (0, 1),
                reward_account: vec![],
                owners: vec![],
            },
        );

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
            &CandidateProfile {
                slots_per_epoch: 432_000,
                genesis_hash: Hash32([0x91; 32]),
                protocol_params_hash: Hash32([0x92; 32]),
                asc: ActiveSlotsCoeff { numer: 1, denom: 20 },
                bootstrap_reward_update: None,
                // not the seed+2 window (the window source is 575) -> the rupd gate is a no-op.
                seed_epoch: EpochNo(0),
            },
        )
        .expect("derive");

        // bound to the TARGET epoch + the window's context.
        assert_eq!(candidate.epoch, w.target_epoch);
        assert_eq!(candidate.snapshot_phase, SnapshotPhase::Set);
        assert_eq!(candidate.source_point.hash, w.lineage_pin);
        assert_eq!(candidate.network_magic, 2);
        assert!(candidate.verify_canonical_hash());
        assert!(candidate.is_leadership_complete(), "every staked pool has a VRF keyhash");
        assert_eq!(
            candidate.pool_vrf_keyhashes.get(&PoolId(Hash28([0x9; 28]))),
            Some(&Hash32([0x9e; 32])),
            "the registered+delegated pool's effective VRF is frozen into the candidate"
        );

        // the candidate's identity is the durable activation identity (round-trips).
        let rec = activation_record_for(&candidate);
        assert_eq!(
            recover_active_view(Some(&rec), Some(&candidate)),
            Ok(ActiveEpochView::Promoted(candidate.clone()))
        );
    }

    fn test_profile() -> CandidateProfile {
        CandidateProfile {
            slots_per_epoch: 432_000,
            genesis_hash: Hash32([0x91; 32]),
            protocol_params_hash: Hash32([0x92; 32]),
            asc: ActiveSlotsCoeff { numer: 1, denom: 20 },
            bootstrap_reward_update: None,
            // not the seed+2 window -> the rupd gate is a strict no-op for this generic derive.
            seed_epoch: EpochNo(0),
        }
    }

    fn derive_with(profile: &CandidateProfile) -> EpochConsensusView {
        derive_with_result(profile).expect("derive")
    }

    // Build a FRESH checkpoint + bootstrap state (a registered + delegated pool) and derive the
    // candidate — a fresh checkpoint per call, so the drive is over equivalent durable inputs.
    fn derive_with_result(
        profile: &CandidateProfile,
    ) -> Result<EpochConsensusView, CandidateDeriveError> {
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
        state.cert_state.pool.pools.insert(
            PoolId(Hash28([0x9; 28])),
            PoolParams {
                pool_id: PoolId(Hash28([0x9; 28])),
                vrf_hash: Hash32([0x9e; 32]),
                pledge: Coin(0),
                cost: Coin(0),
                margin: (0, 1),
                reward_account: vec![],
                owners: vec![],
            },
        );
        let block = conway_block();
        derive_candidate(
            &window(),
            &cp,
            &state,
            std::slice::from_ref(&block),
            CardanoEra::Conway,
            2,
            Hash32([0x42; 32]),
            profile,
        )
    }

    // ---- Option B (B3c / DC-EPOCH-18) -- the mechanical seed+2 rupd fail-closed gate ----
    // `window().source_epoch == 575`, so a profile with `seed_epoch == 575` IS the seed+2 window.
    fn seed2_profile(rupd: Option<BootstrapRewardUpdateT>) -> CandidateProfile {
        CandidateProfile {
            seed_epoch: window().source_epoch,
            bootstrap_reward_update: rupd,
            ..test_profile()
        }
    }
    fn rupd_bound_to(target_epoch: EpochNo) -> BootstrapRewardUpdateT {
        let manifest_commitment = Hash32([0x44; 32]);
        let source_point_slot = SlotNo(1);
        let source_point_hash = Hash32([0x66; 32]);
        let delta_treasury = ade_types::Coin(0);
        let delta_reserves = ade_types::Coin(0);
        let reward_delta = BTreeMap::new();
        let canonical_commitment = ade_ledger::bootstrap_reward_update::bootstrap_rupd_commitment(
            &manifest_commitment,
            source_point_slot,
            &source_point_hash,
            target_epoch,
            delta_treasury,
            delta_reserves,
            &reward_delta,
        );
        BootstrapRewardUpdateT {
            manifest_commitment,
            source_point_slot,
            source_point_hash,
            target_epoch,
            delta_treasury,
            delta_reserves,
            reward_delta,
            canonical_commitment,
        }
    }

    #[test]
    fn seed2_window_without_rupd_fails_closed() {
        match derive_with_result(&seed2_profile(None)) {
            Err(CandidateDeriveError::BootstrapRewardUpdateAbsent { seed }) => {
                assert_eq!(seed, window().source_epoch)
            }
            other => panic!("expected BootstrapRewardUpdateAbsent, got {other:?}"),
        }
    }

    #[test]
    fn seed2_window_with_wrong_epoch_rupd_fails_closed() {
        let wrong = rupd_bound_to(EpochNo(999)); // != window().source_epoch (575)
        match derive_with_result(&seed2_profile(Some(wrong))) {
            Err(CandidateDeriveError::BootstrapRewardUpdateEpochMismatch { bound, seed }) => {
                assert_eq!(bound, EpochNo(999));
                assert_eq!(seed, window().source_epoch);
            }
            other => panic!("expected BootstrapRewardUpdateEpochMismatch, got {other:?}"),
        }
    }

    #[test]
    fn seed2_window_with_correct_rupd_derives() {
        let ok = rupd_bound_to(window().source_epoch);
        assert!(derive_with_result(&seed2_profile(Some(ok))).is_ok());
    }

    #[test]
    fn non_seed2_window_without_rupd_is_a_strict_noop() {
        // test_profile().seed_epoch (0) != window().source_epoch (575): not the seed+2 window, so an
        // absent rupd is a strict no-op, NOT a fail-close.
        assert!(derive_with_result(&test_profile()).is_ok());
    }

    // Report req 2: the candidate's canonical hash is byte-identical across an equivalent replay
    // (the derive is a pure function of the durable inputs + the bound profile).
    #[test]
    fn derive_candidate_canonical_hash_is_replay_equivalent() {
        let a = derive_with(&test_profile());
        let b = derive_with(&test_profile());
        assert_eq!(
            a.canonical_hash(),
            b.canonical_hash(),
            "equivalent replay -> byte-identical candidate identity"
        );
    }

    // Report req 3: a candidate derived under one consensus profile is REJECTED when projected with
    // a different genesis / protocol-params / ASC — no unbound protocol-parameter read (through the
    // real derive path, where the commitment was computed once at derivation).
    #[test]
    fn projection_rejects_wrong_profile_through_the_real_derive_path() {
        use ade_ledger::reduced_epoch_view::ProjectionError;
        let candidate = derive_with(&test_profile());
        // the bound profile projects.
        assert!(candidate
            .to_pool_distr_view(
                &Hash32([0x91; 32]),
                &Hash32([0x92; 32]),
                ActiveSlotsCoeff { numer: 1, denom: 20 }
            )
            .is_ok());
        // a wrong genesis -> commitment mismatch -> fail-closed.
        assert_eq!(
            candidate.to_pool_distr_view(
                &Hash32([0x00; 32]),
                &Hash32([0x92; 32]),
                ActiveSlotsCoeff { numer: 1, denom: 20 }
            ),
            Err(ProjectionError::ParamsCommitmentMismatch)
        );
        // a wrong ASC -> commitment mismatch.
        assert_eq!(
            candidate.to_pool_distr_view(
                &Hash32([0x91; 32]),
                &Hash32([0x92; 32]),
                ActiveSlotsCoeff { numer: 1, denom: 21 }
            ),
            Err(ProjectionError::ParamsCommitmentMismatch)
        );
    }
}
