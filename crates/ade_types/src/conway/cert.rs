// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::shelley::cert::StakeCredential;
use crate::tx::{Coin, PoolId};

/// Closed Conway certificate grammar over CDDL tags `0..18`.
///
/// Only the deposit/refund-relevant fields are retained; every other field is
/// structurally consumed during decode and dropped. There is no catch-all
/// accept arm: unknown tags reject at decode and tags `5`/`6`
/// (genesis-key-delegation / MIR, removed in Conway) decode to
/// [`ConwayCert::RemovedInConway`] so the classifier maps them to a distinct
/// era-validity disposition rather than an accept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConwayCert {
    /// tag 0 — `account_registration_cert` (legacy, implicit key deposit).
    AccountRegistration { credential: StakeCredential },
    /// tag 1 — `account_unregistration_cert` (legacy, implicit refund).
    AccountUnregistration { credential: StakeCredential },
    /// tag 2 — `delegation_to_stake_pool_cert`.
    StakeDelegation,
    /// tag 3 — `pool_registration_cert` (new-vs-update is resolved against state).
    PoolRegistration { pool_id: PoolId },
    /// tag 4 — `pool_retirement_cert` (refund happens at POOLREAP, not tx-time).
    PoolRetirement,
    /// tags 5/6 — genesis-key-delegation / MIR, structurally removed in Conway.
    RemovedInConway { tag: u64 },
    /// tag 7 — `account_registration_deposit_cert` (explicit deposit).
    AccountRegistrationDeposit { deposit: Coin },
    /// tag 8 — `account_unregistration_deposit_cert` (explicit refund).
    AccountUnregistrationDeposit { refund: Coin },
    /// tag 9 — `delegation_to_drep_cert`.
    VoteDelegation,
    /// tag 10 — `delegation_to_stake_pool_and_drep_cert`.
    StakeVoteDelegation,
    /// tag 11 — `account_registration_delegation_to_stake_pool_cert` (explicit deposit).
    StakeRegistrationDelegation { deposit: Coin },
    /// tag 12 — `account_registration_delegation_to_drep_cert` (explicit deposit).
    VoteRegistrationDelegation { deposit: Coin },
    /// tag 13 — `account_registration_delegation_to_stake_pool_and_drep_cert` (explicit deposit).
    StakeVoteRegistrationDelegation { deposit: Coin },
    /// tag 14 — `committee_authorization_cert`.
    AuthCommitteeHot,
    /// tag 15 — `committee_resignation_cert`.
    ResignCommitteeCold,
    /// tag 16 — `drep_registration_cert` (explicit deposit).
    DRepRegistration { deposit: Coin },
    /// tag 17 — `drep_unregistration_cert` (explicit refund).
    DRepUnregistration { refund: Coin },
    /// tag 18 — `drep_update_cert`.
    DRepUpdate,
}

/// Closed disposition taxonomy for a single Conway certificate.
///
/// An era-grammar reject ([`CertDisposition::NotValidInConway`]) is deliberately
/// **not** a [`DepositEffect`] — era validity is not an accounting effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertDisposition {
    /// The certificate contributes a deposit or refund to value conservation.
    Accountable(DepositEffect),
    /// The certificate has no tx-time conservation effect.
    Neutral,
    /// A known-but-removed tag (5/6); not an accounting effect.
    NotValidInConway,
}

/// The deposit-side or refund-side conservation effect of an accountable cert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepositEffect {
    NewDeposit(CoinSource),
    Refund(CoinSource),
}

/// Where the coin amount for a deposit/refund effect comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoinSource {
    /// Conway explicit-deposit variants carry the coin in the certificate.
    ExplicitInCert(Coin),
    /// Legacy-implicit deposit, sourced from canonical `ConwayDepositParams`.
    DepositParam(Coin),
    /// Refund resolved from ledger registration state (deposit recorded at registration).
    RegistrationState(Coin),
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
