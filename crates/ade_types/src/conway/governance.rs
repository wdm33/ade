// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::{Hash28, Hash32, EpochNo};
use crate::tx::Coin;
use crate::shelley::cert::StakeCredential;
use std::collections::{BTreeMap, BTreeSet};

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
    /// CIP-1694 `update_committee = (4, gov_action_id/null,
    /// set<committee_cold_credential>, { committee_cold_credential => epoch_no },
    /// unit_interval)`. Structured (not opaque bytes): the cold credentials are
    /// discriminated `StakeCredential` (DC-LEDGER-10) so committee-enactment
    /// write-back cannot re-collapse the key/script discriminant.
    UpdateCommittee {
        prev_action: Option<GovActionId>,
        /// Cold credentials to remove from the committee.
        removed: BTreeSet<StakeCredential>,
        /// Cold credentials to add, each with its term-expiry epoch.
        added: BTreeMap<StakeCredential, u64>,
        /// New committee quorum threshold (unit_interval numerator, denominator).
        threshold: (u64, u64),
    },
    NewConstitution { prev_action: Option<GovActionId>, raw: Vec<u8> },
    InfoAction,
}

/// The state of a governance proposal: votes collected, procedure, timing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovActionState {
    pub action_id: GovActionId,
    pub committee_votes: Vec<(StakeCredential, Vote)>,
    pub drep_votes: Vec<(StakeCredential, Vote)>,
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

/// CIP-1694 proposal_procedure = `[deposit, return_addr, gov_action, anchor]`.
///
/// Closed struct: `proposal_procedures` is no longer an opaque byte field
/// on the authoritative `ConwayTxBody` shape (DC-LEDGER-11). Construction
/// outside the closed decoder (`ade_codec::conway::governance::decode_proposal_procedures`)
/// is forbidden on the production path by CI; the testkit fixture
/// builders are the only sanctioned synthesis site.
///
/// `return_addr` carries reward-account bytes verbatim (OQ-4 — typed
/// `RewardAccount` is a separate fidelity decision; same shape as the
/// existing `TreasuryWithdrawals.withdrawals` element type).
/// `anchor` reuses the existing opaque `Anchor` struct (OQ-3-adjacent
/// — nested anchor opacity is not in this cluster's scope).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalProcedure {
    pub deposit: Coin,
    pub return_addr: Vec<u8>,
    pub gov_action: GovAction,
    pub anchor: Anchor,
}
