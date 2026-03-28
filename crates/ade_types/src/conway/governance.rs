// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::{Hash28, Hash32, EpochNo};
use crate::tx::Coin;

/// Governance action identifier: transaction hash + index within that transaction.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GovActionId {
    pub tx_hash: Hash32,
    pub index: u32,
}

/// Vote on a governance proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vote {
    No,
    Yes,
    Abstain,
}

/// Governance action type (CIP-1694).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovAction {
    ParameterChange { prev_action: Option<GovActionId>, update: Vec<u8>, policy_hash: Option<Hash28> },
    HardForkInitiation { prev_action: Option<GovActionId>, protocol_version: (u64, u64) },
    TreasuryWithdrawals { withdrawals: Vec<(Vec<u8>, Coin)>, policy_hash: Option<Hash28> },
    NoConfidence { prev_action: Option<GovActionId> },
    UpdateCommittee { prev_action: Option<GovActionId>, raw: Vec<u8> },
    NewConstitution { prev_action: Option<GovActionId>, raw: Vec<u8> },
    InfoAction,
}

/// The state of a governance proposal: votes collected, procedure, timing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovActionState {
    pub action_id: GovActionId,
    pub committee_votes: Vec<(Hash28, Vote)>,
    pub drep_votes: Vec<(Hash28, Vote)>,
    pub spo_votes: Vec<(Hash28, Vote)>,
    pub deposit: Coin,
    pub return_addr: Vec<u8>,
    pub gov_action: GovAction,
    pub proposed_in: EpochNo,
    pub expires_after: EpochNo,
}

/// Voting procedures in a transaction body. Opaque — parsed at validation time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VotingProcedures {
    pub raw: Vec<u8>,
}

/// Anchor (URL + content hash for off-chain metadata). Opaque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub raw: Vec<u8>,
}
