// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Voting procedures map. Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VotingProcedures {
    pub raw: Vec<u8>,
}

/// Proposal procedure (deposit + reward account + gov action + anchor). Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalProcedure {
    pub raw: Vec<u8>,
}

/// Governance action (parameter change, hard fork, treasury withdrawal, etc.).
/// Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovAction {
    pub raw: Vec<u8>,
}

/// Governance action identifier (tx hash + index). Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovActionId {
    pub raw: Vec<u8>,
}

/// Voter (constitutional committee, DRep, or SPO). Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voter {
    pub raw: Vec<u8>,
}

/// Vote (yes, no, abstain). Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vote {
    pub raw: Vec<u8>,
}

/// Anchor (URL + content hash for off-chain metadata). Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub raw: Vec<u8>,
}
