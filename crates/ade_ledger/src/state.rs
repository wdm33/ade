// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::tx::Coin;
use ade_types::{CardanoEra, EpochNo, SlotNo};
use crate::delegation::CertState;
use crate::error::ValidationEnvironmentError;
use crate::pparams::{ConwayDepositParams, ConwayOnlyDepositParams, ProtocolParameters};
use crate::utxo::UTxOState;

/// Epoch state — tracks current epoch, slot, stake distribution snapshots,
/// reserves and treasury.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochState {
    pub epoch: EpochNo,
    pub slot: SlotNo,
    /// Stake distribution snapshots (mark/set/go pipeline), capability-typed: `Authoritative` on the full path,
    /// `ReducedUnavailable` when a reduced follower crossed a boundary without stake authority (never fabricated).
    pub snapshots: crate::epoch::EpochStakeSnapshots,
    /// Ada reserves (un-minted lovelace).
    pub reserves: Coin,
    /// Treasury (accumulated from monetary expansion).
    pub treasury: Coin,
    /// Block production counts per pool for the previous epoch (nesBprev).
    /// Pools not in this map produced zero blocks → zero rewards.
    pub block_production: std::collections::BTreeMap<ade_types::tx::PoolId, u64>,
    /// Accumulated transaction fees from the epoch.
    /// Added to the reward pot at the epoch boundary.
    pub epoch_fees: Coin,
}

impl EpochState {
    pub fn new() -> Self {
        EpochState {
            epoch: EpochNo(0),
            slot: SlotNo(0),
            snapshots: crate::epoch::EpochStakeSnapshots::new(),
            reserves: Coin(0),
            treasury: Coin(0),
            block_production: std::collections::BTreeMap::new(),
            epoch_fees: Coin(0),
        }
    }
}

impl Default for EpochState {
    fn default() -> Self {
        Self::new()
    }
}

/// Capability-typed certificate state on a `LedgerState`. `Authoritative` carries the real `CertState` (the
/// full-ledger / accumulator-seed / window-replay path); `ReducedUnavailable` is a reduced follower
/// (`track_utxo=false`) that crossed a Conway boundary — it advanced NO certificate/pool lifecycle, so there is
/// NO `CertState` at all (never a cleared/empty stand-in). "Reduced follower + a normal `CertState` present" is
/// unrepresentable. A reader wanting cert authority is compiler-forced to go through `require_full` (fail-closed
/// `FullBoundaryStateRequired`) or `as_authoritative` — never a normal field access
/// (feedback_mechanical_not_review_enforced_authority).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertStateProjection {
    Authoritative(CertState),
    ReducedUnavailable,
}

impl CertStateProjection {
    /// A fresh authoritative (empty) cert state — the genuine "no certs yet" of a new/full ledger, NOT a reduced
    /// projection.
    pub fn new() -> Self {
        CertStateProjection::Authoritative(CertState::new())
    }
    pub fn as_authoritative(&self) -> Option<&CertState> {
        match self {
            CertStateProjection::Authoritative(c) => Some(c),
            CertStateProjection::ReducedUnavailable => None,
        }
    }
    pub fn as_authoritative_mut(&mut self) -> Option<&mut CertState> {
        match self {
            CertStateProjection::Authoritative(c) => Some(c),
            CertStateProjection::ReducedUnavailable => None,
        }
    }
    /// Authoritative cert state, or fail closed with `FullBoundaryStateRequired` (never a fabricated/empty stand-in).
    pub fn require_full(
        &self,
        boundary_point: SlotNo,
    ) -> Result<&CertState, crate::governance::GovernanceTerminal> {
        self.as_authoritative()
            .ok_or(crate::governance::GovernanceTerminal::FullBoundaryStateRequired { boundary_point })
    }
    pub fn is_reduced(&self) -> bool {
        matches!(self, CertStateProjection::ReducedUnavailable)
    }
}

impl Default for CertStateProjection {
    fn default() -> Self {
        Self::new()
    }
}

/// Capability-typed Conway governance state on a `LedgerState`. `Authoritative(Option<ConwayGovState>)` is the
/// full path (`Some` = live gov, `None` = pre-Conway / no gov); `ReducedUnavailable` is a reduced follower that
/// crossed a Conway boundary — it ratified/enacted no governance and retains NO gov state (distinct from a full
/// `None`). A reader is compiler-forced to `require_full` (fail-closed) or `as_authoritative`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovStateProjection {
    Authoritative(Option<ConwayGovState>),
    ReducedUnavailable,
}

impl GovStateProjection {
    /// A fresh authoritative (no-governance) state.
    pub fn new() -> Self {
        GovStateProjection::Authoritative(None)
    }
    pub fn as_authoritative(&self) -> Option<&Option<ConwayGovState>> {
        match self {
            GovStateProjection::Authoritative(g) => Some(g),
            GovStateProjection::ReducedUnavailable => None,
        }
    }
    pub fn as_authoritative_mut(&mut self) -> Option<&mut Option<ConwayGovState>> {
        match self {
            GovStateProjection::Authoritative(g) => Some(g),
            GovStateProjection::ReducedUnavailable => None,
        }
    }
    /// Authoritative gov state, or fail closed with `FullBoundaryStateRequired`.
    pub fn require_full(
        &self,
        boundary_point: SlotNo,
    ) -> Result<&Option<ConwayGovState>, crate::governance::GovernanceTerminal> {
        self.as_authoritative()
            .ok_or(crate::governance::GovernanceTerminal::FullBoundaryStateRequired { boundary_point })
    }
    pub fn is_reduced(&self) -> bool {
        matches!(self, GovStateProjection::ReducedUnavailable)
    }
}

impl Default for GovStateProjection {
    fn default() -> Self {
        Self::new()
    }
}

/// Top-level ledger state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerState {
    pub utxo_state: UTxOState,
    pub epoch_state: EpochState,
    pub protocol_params: ProtocolParameters,
    pub era: CardanoEra,
    /// When true, apply_block tracks UTxO and delegation/pool state.
    /// When false (default), state tracking is skipped for performance.
    /// Set to true when state is loaded from a snapshot for boundary replay.
    pub track_utxo: bool,
    /// Accumulated certificate state (delegations, pools, retirements), capability-typed: `Authoritative` on the
    /// full/replay path, `ReducedUnavailable` for a reduced follower that crossed a boundary (no lifecycle).
    pub cert_state: CertStateProjection,
    /// Maximum lovelace supply (from Shelley genesis). Default: 45B ADA.
    /// Used for `circulation = maxLovelaceSupply - reserves` in reward formula.
    pub max_lovelace_supply: u64,
    /// Conway governance state, capability-typed. `Authoritative(None)` for pre-Conway eras;
    /// `ReducedUnavailable` for a reduced follower that crossed a boundary.
    pub gov_state: GovStateProjection,
    /// Conway-only deposit parameters (`drep_deposit`, `gov_action_deposit`).
    /// `Some` iff `era == Conway`; `None` (structurally absent, not defaulted)
    /// for every other era.
    pub conway_deposit_params: Option<ConwayOnlyDepositParams>,
}

/// The Conway `numDormantEpochs` under a VERSIONED lineage. It is AUTHORITATIVE governance state — it
/// changes the active-DRep denominator (`drepExpiry + numDormant >= currentEpoch`), so two states that
/// differ in it MUST NOT share a governance fingerprint. There is NO default: a construction site must
/// declare whether the state predates the field (`Unversioned`, V1 — historical fingerprint unchanged) or
/// carries a value from a NAMED BOUND source (`Bound`, V2 — included in the canonical encoding + fingerprint).
/// A `Unversioned` state is NEVER silently promoted to `Bound(0)`; the DRep-expiry/ratification path REJECTS
/// `Unversioned` (fail-closed) rather than fabricate the offset. See
/// `feedback_versioned_authoritative_state_no_fabricated_default`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DormantEpochs {
    /// V1: `numDormantEpochs` was not part of this state's canonical encoding/fingerprint. Any path that
    /// needs the dormancy offset must fail-closed on this variant, never coerce it to 0.
    Unversioned,
    /// V2: the authoritative `numDormantEpochs`, from a named bound source (imported Conway/ChainDB state, a
    /// replay-derived epoch transition, or a verified migration input). Fingerprinted (V2).
    Bound(u64),
}

/// Conway governance state at the epoch boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConwayGovState {
    /// Active governance proposals.
    pub proposals: Vec<ade_types::conway::governance::GovActionState>,
    /// Committee members: discriminated cold credential → expiry epoch.
    pub committee: std::collections::BTreeMap<ade_types::shelley::cert::StakeCredential, u64>,
    /// Committee quorum (numerator, denominator).
    pub committee_quorum: (u64, u64),
    /// DRep expiry epochs: DRep credential → expiry epoch.
    pub drep_expiry: std::collections::BTreeMap<ade_types::shelley::cert::StakeCredential, u64>,
    /// Governance action lifetime in epochs.
    pub gov_action_lifetime: u64,
    /// Vote delegations: credential → DRep. Loaded from UMap.
    pub vote_delegations: std::collections::BTreeMap<ade_types::shelley::cert::StakeCredential, ade_types::conway::cert::DRep>,
    /// Pool voting thresholds: per-action-type rationals (num, den).
    pub pool_voting_thresholds: Vec<(u64, u64)>,
    /// DRep voting thresholds: per-action-type rationals (num, den).
    pub drep_voting_thresholds: Vec<(u64, u64)>,
    /// Committee hot→cold credential mapping (from VState).
    /// Used to resolve committee vote credentials (hot) to member credentials (cold).
    pub committee_hot_keys: std::collections::BTreeMap<
        ade_types::shelley::cert::StakeCredential,
        ade_types::shelley::cert::StakeCredential,
    >,
    /// `numDormantEpochs` under the versioned lineage (see [`DormantEpochs`]). AUTHORITATIVE: it shifts the
    /// active-DRep denominator. No default — every construction path declares its source (`Unversioned` for
    /// states predating the field, `Bound(n)` from a named source).
    pub num_dormant: DormantEpochs,
    /// The enacted previous-`ParameterChange` root (`prevGovActionIds.pgaPParamUpdate`) under the versioned
    /// lineage (see [`PreviousPParamAction`]). AUTHORITATIVE (CRE S4.3b, INERT): no default — `Unversioned`
    /// for states predating the field, `NoPreviousAction`/`Enacted` only from a decoded source fact.
    pub prev_pparam_action: PreviousPParamAction,
}

/// The enacted previous-`ParameterChange` action root (`prevGovActionIds.pgaPParamUpdate`) as VERSIONED
/// authoritative state (CRE S4.3b). `Unversioned` = a state predating this field's canonical
/// encoding/fingerprint (old snapshots) — the ratify-lineage path must fail-closed on it, NEVER fabricate a
/// root. `NoPreviousAction` = the certified source decoded an explicit `SNothing` (no root exists).
/// `Enacted(id)` = the source decoded `SJust id`. INERT in S4.3b (S4.3c's lineage check activates it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviousPParamAction {
    Unversioned,
    NoPreviousAction,
    Enacted(ade_types::conway::governance::GovActionId),
}

impl LedgerState {
    pub fn new(era: CardanoEra) -> Self {
        LedgerState {
            utxo_state: UTxOState::new(),
            epoch_state: EpochState::new(),
            protocol_params: ProtocolParameters::default(),
            era,
            track_utxo: false,
            cert_state: CertStateProjection::new(),
            max_lovelace_supply: 45_000_000_000_000_000, // 45B ADA mainnet default
            gov_state: GovStateProjection::new(),
            conway_deposit_params: None,
        }
    }

    /// Assemble the validator-boundary [`ConwayDepositParams`] view from the
    /// two canonical sources in this state.
    ///
    /// Fail-fast: if the Conway-only deposit params are absent, returns
    /// [`ValidationEnvironmentError::MissingConwayDepositParams`] — a
    /// validation-environment error, never a default substitution and never a
    /// tx-validity reject. Callers reach this only on the Conway path, where
    /// the params are required to be present.
    pub fn conway_deposit_view(&self) -> Result<ConwayDepositParams, ValidationEnvironmentError> {
        match &self.conway_deposit_params {
            Some(c) => Ok(ConwayDepositParams {
                key_deposit: self.protocol_params.key_deposit,
                pool_deposit: self.protocol_params.pool_deposit,
                drep_deposit: c.drep_deposit,
                gov_action_deposit: c.gov_action_deposit,
            }),
            None => Err(ValidationEnvironmentError::MissingConwayDepositParams),
        }
    }

    /// Assemble the governance-cert accumulation environment from this state's
    /// two canonical sources: the current epoch (`epoch_state.epoch`) and the
    /// Conway-only `drep_activity` parameter.
    ///
    /// Fail-fast: if the Conway-only deposit params are absent, returns
    /// [`ValidationEnvironmentError::MissingDRepActivityParam`] — never a
    /// default substitution. Callers reach this only on the Conway
    /// governance-cert accumulation path, where the param is required.
    pub fn gov_cert_env(&self) -> Result<GovCertEnv, ValidationEnvironmentError> {
        match &self.conway_deposit_params {
            Some(c) => Ok(GovCertEnv {
                current_epoch: self.epoch_state.epoch.0,
                drep_activity: c.drep_activity,
            }),
            None => Err(ValidationEnvironmentError::MissingDRepActivityParam),
        }
    }
}

/// Environment for Conway governance-certificate accumulation (PHASE4-B5).
///
/// The two canonical inputs a DRep-expiry mutation needs: the current epoch and
/// the `drep_activity` parameter. Constructed only via
/// [`LedgerState::gov_cert_env`] (fail-fast on absent param), never defaulted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GovCertEnv {
    pub current_epoch: u64,
    pub drep_activity: u64,
}

/// Mainnet epoch parameters for Shelley+ eras.
///
/// These are fixed by the Shelley genesis and do not change.
/// Byron uses a different epoch scheme (21,600 slots per epoch).
pub const SHELLEY_START_SLOT: u64 = 4_492_800;
pub const SHELLEY_START_EPOCH: u64 = 208;
pub const SHELLEY_EPOCH_LENGTH: u64 = 432_000;

/// PREPROD-ENTRY-AUTHORITY P3: the MAINNET Shelley schedule, built from the constants above.
///
/// These constants describe MAINNET and nothing else. They used to be applied to every venue by
/// `slot_to_epoch`, which silently produced a fictitious epoch off-mainnet. Boundary detection is now
/// bound to the caller's `EraSchedule`; this constructor exists so mainnet-shaped callers and the
/// existing mainnet test corpus keep byte-identical behaviour, and so the constants can only enter a
/// computation through an explicit, named mainnet schedule rather than by default.
pub fn mainnet_shelley_schedule() -> ade_core::consensus::era_schedule::EraSchedule {
    use ade_core::consensus::era_schedule::{BootstrapAnchorHash, EraSchedule, EraSummary};
    EraSchedule::new(
        BootstrapAnchorHash(ade_types::Hash32([0u8; 32])),
        SHELLEY_START_SLOT,
        vec![EraSummary {
            randomness_stabilisation_window_slots: None,
            era: CardanoEra::Shelley,
            start_slot: SlotNo(SHELLEY_START_SLOT),
            start_epoch: EpochNo(SHELLEY_START_EPOCH),
            slot_length_ms: 1_000,
            epoch_length_slots: SHELLEY_EPOCH_LENGTH as u32,
            safe_zone_slots: SHELLEY_EPOCH_LENGTH as u32,
        }],
    )
    .expect("mainnet shelley schedule is well-formed")
}

// PREPROD-ENTRY-AUTHORITY P5 (DC-LEDGER-13): `slot_to_epoch` USED TO LIVE HERE and is deliberately
// GONE. It applied the MAINNET constants above to whatever slot it was handed, which off-mainnet
// yields a fictitious epoch (preprod slot 130,046,891 -> 498 instead of 304; preview -> 473 instead of
// 1378). That is the P3 defect, and P4 (`e1de7a2e`) measured what it cost: a preview store whose
// ledger never advanced past its seed epoch for its entire life. Epoch derivation now goes through the
// caller's `EraSchedule`; a genuinely mainnet-shaped caller builds `mainnet_shelley_schedule()` and
// calls `locate`, which is the SOLE consumer of these constants.
// `ci/ci_check_venue_constant_containment.sh` enforces that mechanically.

/// The ledger epoch disagrees with the venue era schedule for a slot (P5, DC-EPOCH-36).
///
/// Carries both epochs and the slot so an operator can tell a STALE ledger (the P4 shape: the
/// boundary never fired, `ledger < schedule`) from a ledger AHEAD of the schedule (a wrong-venue or
/// wrong-geometry store) without re-deriving anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpochAgreementViolation {
    pub slot: SlotNo,
    pub ledger_epoch: EpochNo,
    pub schedule_epoch: EpochNo,
}

/// DC-EPOCH-36 — the epoch-agreement invariant.
///
/// After the epoch-boundary decision for a block at `slot`, the ledger's epoch MUST equal the venue
/// era schedule's epoch for that slot. A disagreement in EITHER direction is a durable-state
/// contradiction and fails closed.
///
/// This is strictly STRONGER than the detection it guards. [`detect_epoch_transition`] fires only on
/// `schedule > ledger`, so it is structurally blind to a ledger AHEAD of the schedule — equally a
/// contradiction, and equally silent. P4 (`e1de7a2e`) proved the cost of having neither check: a
/// preview store ran its entire life with `ledger_epoch=1375` against `schedule_epoch=1378`, and the
/// drift only surfaced — three epochs later, as an opaque recovery fingerprint mismatch — when a
/// binary that computed the epoch correctly replayed it.
///
/// An UNLOCATABLE slot (before the schedule's first era — the mainnet corpus has pre-Shelley slots)
/// makes the invariant unverifiable, NOT violated: `Ok(())`. This preserves `detect_epoch_transition`'s
/// pre-P5 behaviour exactly (it already returns `None` via `.ok()?` there). Turning an unlocatable
/// slot into an error would be an unrelated behaviour change and is deliberately not done.
pub fn check_epoch_agreement(
    ledger_epoch: EpochNo,
    slot: SlotNo,
    era_schedule: &ade_core::consensus::era_schedule::EraSchedule,
) -> Result<(), EpochAgreementViolation> {
    let Ok(location) = era_schedule.locate(slot) else {
        return Ok(());
    };
    if location.epoch.0 == ledger_epoch.0 {
        Ok(())
    } else {
        Err(EpochAgreementViolation {
            slot,
            ledger_epoch,
            schedule_epoch: location.epoch,
        })
    }
}

/// Check if a slot is the first slot of a new epoch relative to
/// the current epoch in the state.
///
/// Returns Some(new_epoch) if the slot crosses an epoch boundary,
/// None if it's still in the current epoch.
///
/// PREPROD-ENTRY-AUTHORITY P5: `pub(crate)`, NOT `pub`, with a SINGLE non-test caller —
/// `rules::cross_epoch_boundary_for_slot`, which pairs it with [`check_epoch_agreement`]. Detection
/// alone cannot see a ledger ahead of the schedule, so an open-coded detect-then-dispatch site is a
/// silent hole; routing every crossing through one function makes the pairing unforgettable rather
/// than remembered. `ci/ci_check_epoch_agreement.sh` enforces the single-caller invariant. Mirrors
/// the `block_validity_trusted_replay` containment pattern.
pub(crate) fn detect_epoch_transition(
    current_epoch: EpochNo,
    slot: SlotNo,
    era_schedule: &ade_core::consensus::era_schedule::EraSchedule,
) -> Option<EpochNo> {
    // PREPROD-ENTRY-AUTHORITY P3: the epoch MUST come from the VENUE's era schedule, never from the
    // mainnet constants in `slot_to_epoch`. Those constants yield a fictitious epoch off-mainnet
    // (preprod slot 130,046,891 -> 498 instead of 304), and because this is the trigger for EVERY
    // ledger boundary application, a fictitious epoch ABOVE the real one declares a phantom boundary,
    // routes through `apply_reduced_epoch_boundary`, and leaves cert/gov/snapshots permanently
    // `ReducedUnavailable`. Preview was unaffected only by numeric accident (its fictitious epoch sat
    // BELOW its real one, so the comparison never fired) -- luck, not correctness.
    let new_epoch = era_schedule.locate(slot).ok()?.epoch;
    if new_epoch.0 > current_epoch.0 {
        Some(new_epoch)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pparams::ConwayOnlyDepositParams;

    #[test]
    fn gov_cert_env_present_ok() {
        let mut state = LedgerState::new(CardanoEra::Conway);
        state.epoch_state.epoch = EpochNo(576);
        state.conway_deposit_params = Some(ConwayOnlyDepositParams {
            drep_deposit: ade_types::tx::Coin(500_000_000),
            gov_action_deposit: ade_types::tx::Coin(100_000_000_000),
            drep_activity: 20,
        });
        let env = state.gov_cert_env().unwrap();
        assert_eq!(env.current_epoch, 576);
        assert_eq!(env.drep_activity, 20);
    }

    #[test]
    fn gov_cert_env_missing_drep_activity_is_fail_fast() {
        // Conway state without conway_deposit_params: the env is unavailable and
        // must be a structured fail-fast, never a defaulted activity period.
        let state = LedgerState::new(CardanoEra::Conway);
        assert_eq!(state.conway_deposit_params, None);
        assert_eq!(
            state.gov_cert_env(),
            Err(ValidationEnvironmentError::MissingDRepActivityParam)
        );
    }

    /// P5: the mainnet slot->epoch mapping these tests cover is unchanged; only its ENTRY POINT moved.
    /// `slot_to_epoch` applied the mainnet constants to any venue; the schedule cannot, because a
    /// caller must name which schedule it means.
    fn mainnet_epoch_of(slot: SlotNo) -> Option<EpochNo> {
        mainnet_shelley_schedule().locate(slot).ok().map(|l| l.epoch)
    }

    #[test]
    fn mainnet_epoch_at_shelley_start() {
        assert_eq!(mainnet_epoch_of(SlotNo(4_492_800)), Some(EpochNo(208)));
    }

    #[test]
    fn mainnet_epoch_mid_epoch() {
        // Slot 4,924,800 = start of epoch 209
        assert_eq!(mainnet_epoch_of(SlotNo(4_924_800)), Some(EpochNo(209)));
        // One slot before = still epoch 208
        assert_eq!(mainnet_epoch_of(SlotNo(4_924_799)), Some(EpochNo(208)));
    }

    #[test]
    fn mainnet_epoch_allegra() {
        // Allegra epoch 236 starts at 4,492,800 + 28*432,000 = 16,588,800
        assert_eq!(mainnet_epoch_of(SlotNo(16_588_800)), Some(EpochNo(236)));
    }

    #[test]
    fn mainnet_epoch_pre_shelley() {
        assert_eq!(mainnet_epoch_of(SlotNo(0)), None);
        assert_eq!(mainnet_epoch_of(SlotNo(4_492_799)), None);
    }

    /// DC-EPOCH-36 CE-P5-1: a STALE ledger epoch (the P4 shape) is rejected.
    #[test]
    fn epoch_agreement_rejects_a_stale_ledger_epoch() {
        let err =
            check_epoch_agreement(EpochNo(208), SlotNo(4_924_800), &mainnet_shelley_schedule())
                .expect_err("ledger 208 vs schedule 209 must fail closed");
        assert_eq!(err.ledger_epoch, EpochNo(208));
        assert_eq!(err.schedule_epoch, EpochNo(209));
        assert_eq!(err.slot, SlotNo(4_924_800));
    }

    /// DC-EPOCH-36 CE-P5-1: the OTHER direction — a ledger AHEAD of the schedule. This is the case
    /// `detect_epoch_transition` is structurally blind to, and the reason the invariant is not just a
    /// restatement of the detection.
    #[test]
    fn epoch_agreement_rejects_a_ledger_ahead_of_the_schedule() {
        let sched = mainnet_shelley_schedule();
        assert_eq!(
            detect_epoch_transition(EpochNo(999), SlotNo(4_500_000), &sched),
            None,
            "detection cannot see a ledger ahead of the schedule"
        );
        let err = check_epoch_agreement(EpochNo(999), SlotNo(4_500_000), &sched)
            .expect_err("a ledger ahead of the schedule must fail closed");
        assert_eq!(err.ledger_epoch, EpochNo(999));
        assert_eq!(err.schedule_epoch, EpochNo(208));
    }

    /// DC-EPOCH-36 CE-P5-1: agreement passes.
    #[test]
    fn epoch_agreement_accepts_agreement() {
        assert!(check_epoch_agreement(
            EpochNo(208),
            SlotNo(4_500_000),
            &mainnet_shelley_schedule()
        )
        .is_ok());
    }

    /// DC-EPOCH-36 CE-P5-2: an UNLOCATABLE slot (pre-Shelley on the mainnet schedule) is
    /// unverifiable, NOT violated — behaviour identical to pre-P5.
    #[test]
    fn epoch_agreement_is_silent_on_an_unlocatable_slot() {
        assert!(
            check_epoch_agreement(EpochNo(1), SlotNo(0), &mainnet_shelley_schedule()).is_ok(),
            "a slot the schedule cannot locate makes the invariant unverifiable, not violated"
        );
    }

    #[test]
    fn detect_transition_same_epoch() {
        assert_eq!(
            detect_epoch_transition(EpochNo(208), SlotNo(4_500_000), &mainnet_shelley_schedule()),
            None
        );
    }

    #[test]
    fn detect_transition_new_epoch() {
        assert_eq!(
            detect_epoch_transition(EpochNo(208), SlotNo(4_924_800), &mainnet_shelley_schedule()),
            Some(EpochNo(209))
        );
    }

    #[test]
    fn detect_transition_skip_epoch() {
        // If a slot is 2 epochs ahead (shouldn't happen in practice but test the logic)
        assert_eq!(
            detect_epoch_transition(EpochNo(208), SlotNo(5_356_800), &mainnet_shelley_schedule()),
            Some(EpochNo(210))
        );
    }

    // -----------------------------------------------------------------------------------------
    // PREPROD-ENTRY-AUTHORITY P6-S5 — the MULTI-VENUE differential (DC-EPOCH-37).
    //
    // A green mainnet-shaped corpus is NOT evidence that preview/preprod semantics are correct.
    // P3 is the proof: `detect_epoch_transition` computed the epoch from hardcoded MAINNET constants,
    // the entire mainnet corpus stayed byte-identical, and the defect was fatal on preprod (a phantom
    // boundary) and silently corrosive on preview (the ledger epoch never advanced for a store's whole
    // life). No mainnet test could have caught it, because mainnet is the one venue where the wrong
    // formula is right.
    //
    // So every venue's geometry is exercised directly, and the anchors below are the values MEASURED
    // during P3/P4 rather than recomputed here — they are the regression this suite exists to pin.
    // -----------------------------------------------------------------------------------------

    /// (name, shelley_start_epoch, shelley_start_slot, epoch_length_slots).
    /// Mirrors `ade_node::native_firstrun::shelley_boundary_for_magic` +
    /// `ade_node::bootstrap_export::resolve_network_profile`;
    /// `ci/ci_check_venue_differential.sh` pins these against those authorities so they cannot drift.
    use ade_core::consensus::era_schedule::{BootstrapAnchorHash, EraSchedule, EraSummary};

    const VENUES: [(&str, u64, u64, u32); 3] = [
        ("mainnet", 208, 4_492_800, 432_000),
        ("preprod", 4, 86_400, 432_000),
        ("preview", 0, 0, 86_400),
    ];

    fn venue_schedule(start_epoch: u64, start_slot: u64, epoch_length: u32) -> EraSchedule {
        EraSchedule::new(
            BootstrapAnchorHash(ade_types::Hash32([0u8; 32])),
            start_slot,
            vec![EraSummary {
                randomness_stabilisation_window_slots: None,
                era: CardanoEra::Conway,
                start_slot: SlotNo(start_slot),
                start_epoch: EpochNo(start_epoch),
                slot_length_ms: 1_000,
                epoch_length_slots: epoch_length,
                safe_zone_slots: epoch_length,
            }],
        )
        .expect("venue geometry is well-formed")
    }

    /// `schedule.locate(slot).epoch`, unwrapped -- the operation every venue assertion below performs.
    fn epoch_at(sched: &EraSchedule, slot: u64, name: &str) -> u64 {
        sched.locate(SlotNo(slot)).expect(name).epoch.0
    }

    /// The MAINNET formula that P3 deleted, reproduced here ONLY to prove it is wrong off-mainnet.
    fn mainnet_formula_epoch(slot: u64) -> Option<u64> {
        (slot >= SHELLEY_START_SLOT)
            .then(|| SHELLEY_START_EPOCH + (slot - SHELLEY_START_SLOT) / SHELLEY_EPOCH_LENGTH)
    }

    /// Epoch derivation is correct at every boundary edge, for EVERY venue — not just the one whose
    /// constants happen to be compiled in.
    #[test]
    fn venue_differential_epoch_derivation_is_correct_for_every_venue() {
        for (name, start_epoch, start_slot, epoch_length) in VENUES {
            let sched = venue_schedule(start_epoch, start_slot, epoch_length);
            let len = u64::from(epoch_length);
            let at = |slot: u64| epoch_at(&sched, slot, name);

            assert_eq!(at(start_slot), start_epoch, "{name}: epoch start");
            assert_eq!(at(start_slot + len - 1), start_epoch, "{name}: before");
            assert_eq!(at(start_slot + len), start_epoch + 1, "{name}: after");
            assert_eq!(at(start_slot + 5 * len + 7), start_epoch + 5, "{name}: +5");
        }
    }

    /// The boundary fires EXACTLY once per venue epoch, at the first slot of the new epoch — never a
    /// slot early (a phantom boundary, the preprod failure) and never never (the preview failure).
    #[test]
    fn venue_differential_boundary_fires_exactly_at_each_venue_boundary() {
        for (name, start_epoch, start_slot, epoch_length) in VENUES {
            let sched = venue_schedule(start_epoch, start_slot, epoch_length);
            let len = u64::from(epoch_length);
            let here = EpochNo(start_epoch);

            assert_eq!(
                detect_epoch_transition(here, SlotNo(start_slot + len - 1), &sched),
                None,
                "{name}: the slot BEFORE the boundary must not declare a transition"
            );
            assert_eq!(
                detect_epoch_transition(here, SlotNo(start_slot + len), &sched),
                Some(EpochNo(start_epoch + 1)),
                "{name}: the first slot of the new epoch must declare exactly one transition"
            );
            assert_eq!(
                detect_epoch_transition(here, SlotNo(start_slot + len / 2), &sched),
                None,
                "{name}: mid-epoch must never declare a transition"
            );
        }
    }

    /// DC-EPOCH-36 holds per venue, in both directions.
    #[test]
    fn venue_differential_epoch_agreement_discriminates_for_every_venue() {
        for (name, start_epoch, start_slot, epoch_length) in VENUES {
            let sched = venue_schedule(start_epoch, start_slot, epoch_length);
            let mid = SlotNo(start_slot + u64::from(epoch_length) / 2);

            assert!(
                check_epoch_agreement(EpochNo(start_epoch), mid, &sched).is_ok(),
                "{name}: an agreeing ledger must pass"
            );
            let stale = check_epoch_agreement(EpochNo(start_epoch + 3), mid, &sched)
                .expect_err("a ledger ahead of the schedule must fail closed");
            assert_eq!(stale.schedule_epoch.0, start_epoch, "{name}");
        }
    }

    /// THE P3 REGRESSION PIN. The mainnet formula and the venue schedule disagree off-mainnet, and the
    /// numbers here are the ones MEASURED in P3/P4 — preprod 498-vs-304 (fatal: a phantom boundary
    /// declared, cert/gov/snapshots left ReducedUnavailable, node dead at exit 43) and preview
    /// 473-vs-1378 (silent: the fictitious epoch sits BELOW the real one, so `new > current` never
    /// fires and the ledger epoch NEVER advances).
    ///
    /// It also pins WHY a mainnet corpus cannot catch this: on mainnet the two agree exactly.
    #[test]
    fn venue_differential_mainnet_formula_is_wrong_off_mainnet() {
        // mainnet: the formula and the schedule agree -- which is precisely why a mainnet-only suite
        // stays green through this defect.
        let mainnet = venue_schedule(208, 4_492_800, 432_000);
        let mainnet_slot = 119_075_343u64;
        assert_eq!(
            mainnet_formula_epoch(mainnet_slot),
            Some(epoch_at(&mainnet, mainnet_slot, "mainnet")),
            "on mainnet the formula is correct -- this is the blind spot"
        );

        // preprod: FATAL direction. Fictitious epoch ABOVE the real one => phantom boundary.
        let preprod = venue_schedule(4, 86_400, 432_000);
        let preprod_slot = 130_046_891u64;
        let preprod_real = epoch_at(&preprod, preprod_slot, "preprod");
        assert_eq!(preprod_real, 304, "the real preprod epoch measured in P3");
        assert_eq!(
            mainnet_formula_epoch(preprod_slot),
            Some(498),
            "the fictitious epoch measured in P3"
        );
        assert!(498 > preprod_real, "ABOVE real => phantom boundary");

        // preview: SILENT direction. Fictitious epoch BELOW the real one => no boundary ever fires.
        let preview = venue_schedule(0, 0, 86_400);
        let preview_slot = 119_075_343u64;
        let preview_real = epoch_at(&preview, preview_slot, "preview");
        assert_eq!(preview_real, 1378, "the real preview epoch measured in P4");
        assert_eq!(
            mainnet_formula_epoch(preview_slot),
            Some(473),
            "the fictitious epoch measured in P4"
        );
        assert!(473 < preview_real, "BELOW real => no boundary ever fires");

        // And the invariant that would have caught BOTH on the first block:
        assert!(check_epoch_agreement(EpochNo(473), SlotNo(preview_slot), &preview).is_err());
        assert!(check_epoch_agreement(EpochNo(498), SlotNo(preprod_slot), &preprod).is_err());
    }
}
