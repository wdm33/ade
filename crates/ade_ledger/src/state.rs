// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::tx::Coin;
use ade_types::{CardanoEra, EpochNo, SlotNo};
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
}

impl EpochState {
    pub fn new() -> Self {
        EpochState {
            epoch: EpochNo(0),
            slot: SlotNo(0),
            snapshots: SnapshotState::new(),
            reserves: Coin(0),
            treasury: Coin(0),
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
}

impl LedgerState {
    pub fn new(era: CardanoEra) -> Self {
        LedgerState {
            utxo_state: UTxOState::new(),
            epoch_state: EpochState::new(),
            protocol_params: ProtocolParameters::default(),
            era,
        }
    }
}
