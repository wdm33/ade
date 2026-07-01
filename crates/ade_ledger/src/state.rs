// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::tx::Coin;
use ade_types::{CardanoEra, EpochNo, SlotNo};
use crate::delegation::CertState;
use crate::epoch::SnapshotState;
use crate::error::ValidationEnvironmentError;
use crate::pparams::{ConwayDepositParams, ConwayOnlyDepositParams, ProtocolParameters};
use crate::utxo::UTxOState;

/// Epoch state — tracks current epoch, slot, stake distribution snapshots,
/// reserves and treasury.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochState {
    pub epoch: EpochNo,
    pub slot: SlotNo,
    /// Stake distribution snapshots (mark/set/go pipeline).
    pub snapshots: SnapshotState,
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
            snapshots: SnapshotState::new(),
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
    /// Accumulated certificate state (delegations, pools, retirements).
    /// Populated during replay when track_utxo is true.
    pub cert_state: CertState,
    /// Maximum lovelace supply (from Shelley genesis). Default: 45B ADA.
    /// Used for `circulation = maxLovelaceSupply - reserves` in reward formula.
    pub max_lovelace_supply: u64,
    /// Conway governance state. None for pre-Conway eras.
    pub gov_state: Option<ConwayGovState>,
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
}

impl LedgerState {
    pub fn new(era: CardanoEra) -> Self {
        LedgerState {
            utxo_state: UTxOState::new(),
            epoch_state: EpochState::new(),
            protocol_params: ProtocolParameters::default(),
            era,
            track_utxo: false,
            cert_state: CertState::new(),
            max_lovelace_supply: 45_000_000_000_000_000, // 45B ADA mainnet default
            gov_state: None,
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

/// Compute the epoch number for a given slot (Shelley+ only).
///
/// Returns None for pre-Shelley slots.
pub fn slot_to_epoch(slot: SlotNo) -> Option<EpochNo> {
    if slot.0 < SHELLEY_START_SLOT {
        return None;
    }
    let offset = slot.0 - SHELLEY_START_SLOT;
    let epoch = SHELLEY_START_EPOCH + offset / SHELLEY_EPOCH_LENGTH;
    Some(EpochNo(epoch))
}

/// Check if a slot is the first slot of a new epoch relative to
/// the current epoch in the state.
///
/// Returns Some(new_epoch) if the slot crosses an epoch boundary,
/// None if it's still in the current epoch.
pub fn detect_epoch_transition(current_epoch: EpochNo, slot: SlotNo) -> Option<EpochNo> {
    let new_epoch = slot_to_epoch(slot)?;
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

    #[test]
    fn slot_to_epoch_shelley_start() {
        assert_eq!(slot_to_epoch(SlotNo(4_492_800)), Some(EpochNo(208)));
    }

    #[test]
    fn slot_to_epoch_mid_epoch() {
        // Slot 4,924,800 = start of epoch 209
        assert_eq!(slot_to_epoch(SlotNo(4_924_800)), Some(EpochNo(209)));
        // One slot before = still epoch 208
        assert_eq!(slot_to_epoch(SlotNo(4_924_799)), Some(EpochNo(208)));
    }

    #[test]
    fn slot_to_epoch_allegra() {
        // Allegra epoch 236 starts at 4,492,800 + 28*432,000 = 16,588,800
        assert_eq!(slot_to_epoch(SlotNo(16_588_800)), Some(EpochNo(236)));
    }

    #[test]
    fn slot_to_epoch_pre_shelley() {
        assert_eq!(slot_to_epoch(SlotNo(0)), None);
        assert_eq!(slot_to_epoch(SlotNo(4_492_799)), None);
    }

    #[test]
    fn detect_transition_same_epoch() {
        assert_eq!(
            detect_epoch_transition(EpochNo(208), SlotNo(4_500_000)),
            None
        );
    }

    #[test]
    fn detect_transition_new_epoch() {
        assert_eq!(
            detect_epoch_transition(EpochNo(208), SlotNo(4_924_800)),
            Some(EpochNo(209))
        );
    }

    #[test]
    fn detect_transition_skip_epoch() {
        // If a slot is 2 epochs ahead (shouldn't happen in practice but test the logic)
        assert_eq!(
            detect_epoch_transition(EpochNo(208), SlotNo(5_356_800)),
            Some(EpochNo(210))
        );
    }
}
