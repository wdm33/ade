// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Conway certificate (stake registration, delegation, DRep registration, etc.).
/// Opaque in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConwayCert {
    pub raw: Vec<u8>,
}

/// Delegated representative (CIP-1694).
///
/// A credential can delegate its voting power to one of:
/// - A specific DRep (identified by key hash or script hash)
/// - AlwaysAbstain (voting power excluded from quorum)
/// - AlwaysNoConfidence (automatic no-confidence vote)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DRep {
    /// Delegate to a specific DRep identified by key hash.
    KeyHash(crate::Hash28),
    /// Delegate to a specific DRep identified by script hash.
    ScriptHash(crate::Hash28),
    /// Abstain from all governance votes. Stake excluded from quorum denominator.
    AlwaysAbstain,
    /// Automatic no-confidence in the constitutional committee.
    AlwaysNoConfidence,
}
