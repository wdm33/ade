//! EPOCH-CONSENSUS-VIEW S3f-4d-3a — the boundary activation ORCHESTRATION (the sequenced
//! durable-before-visible flip).
//!
//! ONE atomic path, in the user-mandated order: validate the durable source window
//! (DC-EPOCH-08) → derive the candidate (DC-EPOCH-09) → verify the activation predicate
//! (DC-EPOCH-05/07, BEFORE the WAL) → write the durable WAL activation record (DC-EPOCH-06)
//! → publish the active view ONLY if the write is durable → atomically promote. A failure
//! after the predicate passes is a TERMINAL [`EpochViewActivationError`] (halt — the caller
//! stops admit/forge/follow, NEVER falls back to the seed view). A predicate decline (e.g.
//! the transition is not yet eligible) is `NotYet` (the seed stays authoritative; retry the
//! next boundary).
//!
//! This is the orchestration helper. The LIVE call at the relay-loop boundary apply site
//! (extracting the real durable ChainDB window, feeding the published view to leadership,
//! warm-start recovery) is S3f-4d-3b, gated on the two live cardano-node proofs.

use crate::epoch_activation::{
    activate_durable_before_visible, activation_predicate, activation_record_for, ActivationOutcome,
    ActivationReject, ActiveEpochView, EpochViewActivationError,
};
use crate::epoch_candidate::{derive_candidate, CandidateProfile};
use crate::epoch_source_window::{validate_source_window, ActivationSourceWindow, SourceWindowBlock};
use ade_core::consensus::events::Point;
use ade_ledger::reduced_epoch_view::{EpochConsensusView, ViewBindings};
use ade_ledger::state::LedgerState;
use ade_ledger::wal::WalEntry;
use ade_runtime::chaindb::ReducedUtxoCheckpoint;
use ade_types::shelley::block::ShelleyBlock;
use ade_types::{CardanoEra, Hash32};

/// The result of a boundary activation attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundaryActivationOutcome {
    /// The active view was atomically published (the promoted N+1 view is now authoritative).
    Promoted,
    /// The predicate declined (NOT terminal) — the seed view stays authoritative; retry the
    /// next boundary. Carries the reject reason for evidence.
    NotYet(ActivationReject),
}

/// The expected N+1 binding context for a candidate (its own canonical fields). The predicate
/// pins the candidate to this + `verify_canonical_hash` (tamper guard) + the selected point.
fn candidate_bindings(c: &EpochConsensusView) -> ViewBindings {
    ViewBindings {
        network_magic: c.network_magic,
        era: c.era,
        epoch: c.epoch,
        source_point: c.source_point.clone(),
        checkpoint_commitment: c.checkpoint_commitment.clone(),
        nonce: c.nonce.clone(),
        snapshot_phase: c.snapshot_phase,
        protocol_params_commitment: c.protocol_params_commitment.clone(),
    }
}

/// Orchestrate the boundary activation. `wal_write` writes the activation record durably and
/// returns whether it is durable (the ONLY durability gate — publication follows ONLY a
/// durable write). `active_view` is atomically promoted on success. Returns `Promoted` /
/// `NotYet` on a clean outcome, or a TERMINAL [`EpochViewActivationError`] the caller must
/// halt on.
#[allow(clippy::too_many_arguments)]
pub fn activate_at_boundary(
    window: &ActivationSourceWindow,
    window_blocks: &[SourceWindowBlock],
    checkpoint: &ReducedUtxoCheckpoint,
    bootstrap_state: &LedgerState,
    blocks: &[ShelleyBlock],
    era: CardanoEra,
    network_magic: u32,
    nonce: Hash32,
    profile: &CandidateProfile,
    selected_point: &Point,
    transition_eligible: bool,
    active_view: &mut ActiveEpochView,
    wal_write: impl FnOnce(&WalEntry) -> bool,
) -> Result<BoundaryActivationOutcome, EpochViewActivationError> {
    // 1. validate the durable source window (DC-EPOCH-08). A corrupt/forked/incomplete
    //    window is a TERMINAL activation failure -- the durable chain cannot be trusted.
    validate_source_window(window, window_blocks)
        .map_err(|_| EpochViewActivationError::EpochViewActivationFailed)?;

    // 2. derive the candidate (DC-EPOCH-09). A derivation failure is TERMINAL.
    let candidate = derive_candidate(
        window, checkpoint, bootstrap_state, blocks, era, network_magic, nonce, profile,
    )
    .map_err(|_| EpochViewActivationError::EpochViewActivationFailed)?;

    // 3. the activation predicate (DC-EPOCH-05/07), BEFORE the WAL: transition eligible +
    //    bindings verify + the candidate's point IS the selected-chain point. `wal_durable`
    //    is passed `true` here (the intent); the REAL durability gate is step 5. A decline is
    //    NotYet (seed stays), NOT terminal.
    let bindings = candidate_bindings(&candidate);
    match activation_predicate(&candidate, &bindings, selected_point, transition_eligible, true) {
        ActivationOutcome::Promote => {}
        ActivationOutcome::NoPromotion(reject) => {
            return Ok(BoundaryActivationOutcome::NotYet(reject))
        }
    }

    // 4. write the durable WAL activation record (DC-EPOCH-06: durable BEFORE visible).
    let record = activation_record_for(&candidate);
    let durable = wal_write(&record);

    // 5. publish ONLY if the write is durable -- else TERMINAL EpochViewActivationFailed.
    let published = activate_durable_before_visible(candidate, durable)?;
    let view = match published {
        ActiveEpochView::Promoted(v) => v,
        // unreachable: activate_durable_before_visible returns Promoted or Err.
        ActiveEpochView::Seed => return Err(EpochViewActivationError::EpochViewActivationFailed),
    };

    // 6. atomically promote the active view (a differing already-active view -> TERMINAL
    //    conflict; the same view is idempotent).
    active_view.promote(view)?;
    Ok(BoundaryActivationOutcome::Promoted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epoch_source_window::target_epoch_for_source;
    use ade_ledger::reduced_snapshot::SnapshotPhase;
    use ade_ledger::reduced_utxo::ReducedStakeRef;
    use ade_types::primitives::SlotNo;
    use ade_types::shelley::cert::StakeCredential;
    use ade_types::tx::{Coin, PoolId, TxIn};
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_types::{EpochNo, Hash28};
    use std::collections::BTreeMap;

    const RAW_CONWAY_BLOCK: &[u8] = include_bytes!("../tests/fixtures/raw_era_block_conway.cbor");

    fn conway_block() -> ShelleyBlock {
        let env = ade_codec::cbor::envelope::decode_block_envelope(RAW_CONWAY_BLOCK).expect("env");
        let inner = &RAW_CONWAY_BLOCK[env.block_start..env.block_end];
        ade_codec::conway::decode_conway_block(inner).expect("decode").decoded().clone()
    }

    fn checkpoint() -> (ReducedUtxoCheckpoint, tempfile::TempDir, LedgerState) {
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
        (cp, dir, state)
    }

    fn window(pin: Hash32) -> ActivationSourceWindow {
        ActivationSourceWindow {
            source_epoch: EpochNo(575),
            source_window_start: SlotNo(0),
            source_window_end: SlotNo(1000),
            snapshot_phase: SnapshotPhase::Set,
            target_epoch: target_epoch_for_source(EpochNo(575), SnapshotPhase::Set).unwrap(),
            source_window_anchor: Hash32([0x00; 32]),
            lineage_pin: pin,
        }
    }

    fn window_blocks() -> Vec<SourceWindowBlock> {
        vec![SourceWindowBlock {
            slot: SlotNo(190),
            hash: Hash32([0xab; 32]),
            prev_hash: Hash32([0x00; 32]),
        }]
    }

    // the candidate's source_point is {source_window_end, lineage_pin}; the selected point
    // must equal it for the predicate to pass.
    fn selected_point() -> Point {
        Point { slot: SlotNo(1000), hash: Hash32([0xab; 32]) }
    }
    fn profile() -> CandidateProfile {
        CandidateProfile {
            slots_per_epoch: 432_000,
            genesis_hash: Hash32([0x91; 32]),
            protocol_params_hash: Hash32([0x92; 32]),
            asc: ActiveSlotsCoeff { numer: 1, denom: 20 },
        }
    }

    #[test]
    fn happy_path_promotes_after_durable_wal() {
        let (cp, _d, state) = checkpoint();
        let mut active = ActiveEpochView::new();
        let mut written: Option<WalEntry> = None;
        let out = activate_at_boundary(
            &window(Hash32([0xab; 32])),
            &window_blocks(),
            &cp,
            &state,
            std::slice::from_ref(&conway_block()),
            CardanoEra::Conway,
            2,
            Hash32([0x42; 32]),
            &profile(),
            &selected_point(),
            true,
            &mut active,
            |rec| {
                written = Some(rec.clone());
                true // durable
            },
        )
        .expect("no terminal");
        assert_eq!(out, BoundaryActivationOutcome::Promoted);
        assert!(active.is_promoted());
        assert!(matches!(written, Some(WalEntry::EpochConsensusViewActivated { .. })));
    }

    #[test]
    fn non_durable_wal_is_terminal_and_does_not_publish() {
        let (cp, _d, state) = checkpoint();
        let mut active = ActiveEpochView::new();
        let r = activate_at_boundary(
            &window(Hash32([0xab; 32])),
            &window_blocks(),
            &cp,
            &state,
            std::slice::from_ref(&conway_block()),
            CardanoEra::Conway,
            2,
            Hash32([0x42; 32]),
            &profile(),
            &selected_point(),
            true,
            &mut active,
            |_rec| false, // NOT durable
        );
        assert_eq!(r, Err(EpochViewActivationError::EpochViewActivationFailed));
        assert!(!active.is_promoted(), "no publication on a non-durable WAL write");
    }

    #[test]
    fn not_eligible_transition_is_not_yet_not_terminal() {
        let (cp, _d, state) = checkpoint();
        let mut active = ActiveEpochView::new();
        let mut wrote = false;
        let out = activate_at_boundary(
            &window(Hash32([0xab; 32])),
            &window_blocks(),
            &cp,
            &state,
            std::slice::from_ref(&conway_block()),
            CardanoEra::Conway,
            2,
            Hash32([0x42; 32]),
            &profile(),
            &selected_point(),
            false, // transition NOT eligible
            &mut active,
            |_rec| {
                wrote = true;
                true
            },
        )
        .expect("not terminal");
        assert_eq!(out, BoundaryActivationOutcome::NotYet(ActivationReject::TransitionIneligible));
        assert!(!active.is_promoted(), "seed stays authoritative");
        assert!(!wrote, "no WAL write when the predicate declines");
    }

    #[test]
    fn invalid_window_is_terminal_before_any_wal() {
        let (cp, _d, state) = checkpoint();
        let mut active = ActiveEpochView::new();
        let mut wrote = false;
        // a window whose blocks do not pin to the lineage tip -> validate fails -> terminal.
        let r = activate_at_boundary(
            &window(Hash32([0x99; 32])), // lineage_pin != the block's hash (0xab)
            &window_blocks(),
            &cp,
            &state,
            std::slice::from_ref(&conway_block()),
            CardanoEra::Conway,
            2,
            Hash32([0x42; 32]),
            &profile(),
            &Point { slot: SlotNo(1000), hash: Hash32([0x99; 32]) },
            true,
            &mut active,
            |_rec| {
                wrote = true;
                true
            },
        );
        assert_eq!(r, Err(EpochViewActivationError::EpochViewActivationFailed));
        assert!(!wrote, "no WAL write when the window is invalid");
        assert!(!active.is_promoted());
    }

    // selected-point mismatch (the candidate's point != the live selected tip) -> NotYet.
    #[test]
    fn selected_point_mismatch_declines() {
        let (cp, _d, state) = checkpoint();
        let mut active = ActiveEpochView::new();
        let out = activate_at_boundary(
            &window(Hash32([0xab; 32])),
            &window_blocks(),
            &cp,
            &state,
            std::slice::from_ref(&conway_block()),
            CardanoEra::Conway,
            2,
            Hash32([0x42; 32]),
            &profile(),
            &Point { slot: SlotNo(7), hash: Hash32([0xee; 32]) }, // not the candidate's point
            true,
            &mut active,
            |_rec| true,
        )
        .expect("not terminal");
        assert_eq!(out, BoundaryActivationOutcome::NotYet(ActivationReject::WrongSelectedPoint));
        assert!(!active.is_promoted());
    }
}
