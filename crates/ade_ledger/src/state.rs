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
use crate::pparams::ProtocolParameters;
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
}

/// Conway governance state at the epoch boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConwayGovState {
    /// Active governance proposals.
    pub proposals: Vec<ade_types::conway::governance::GovActionState>,
    /// Committee members: credential hash → expiry epoch.
    pub committee: std::collections::BTreeMap<ade_types::Hash28, u64>,
    /// Committee quorum (numerator, denominator).
    pub committee_quorum: (u64, u64),
    /// DRep expiry epochs: credential hash → expiry epoch.
    pub drep_expiry: std::collections::BTreeMap<ade_types::Hash28, u64>,
    /// Governance action lifetime in epochs.
    pub gov_action_lifetime: u64,
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
        }
    }
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
