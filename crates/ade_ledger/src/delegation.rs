// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeMap;
use ade_types::conway::cert::ConwayCert;
use ade_types::tx::{Coin, PoolId};
use ade_types::{EpochNo, Hash32};
use ade_types::shelley::cert::{
    Certificate, PoolRegistrationCert, StakeCredential,
};

use crate::error::{
    CertFailureReason, CertificateError, EraInvalidCertificateError, LedgerError,
};

/// Aggregate certificate state: delegation state + pool state.
///
/// Passed into and returned from `apply_cert` as a pure value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertState {
    pub delegation: DelegationState,
    pub pool: PoolState,
}

impl CertState {
    pub fn new() -> Self {
        CertState {
            delegation: DelegationState::new(),
            pool: PoolState::new(),
        }
    }
}

impl Default for CertState {
    fn default() -> Self {
        Self::new()
    }
}

/// Delegation bookkeeping: registrations, active delegations, and reward balances.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegationState {
    /// Registered stake credentials and the deposit paid at registration.
    pub registrations: BTreeMap<StakeCredential, Coin>,
    /// Active delegations: credential -> pool.
    pub delegations: BTreeMap<StakeCredential, PoolId>,
    /// Accumulated reward balances per credential.
    pub rewards: BTreeMap<StakeCredential, Coin>,
}

impl DelegationState {
    pub fn new() -> Self {
        DelegationState {
            registrations: BTreeMap::new(),
            delegations: BTreeMap::new(),
            rewards: BTreeMap::new(),
        }
    }
}

impl Default for DelegationState {
    fn default() -> Self {
        Self::new()
    }
}

/// Pool registration and retirement tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolState {
    /// Registered pools and their parameters.
    pub pools: BTreeMap<PoolId, PoolParams>,
    /// Pools scheduled for retirement at a given epoch.
    pub retiring: BTreeMap<PoolId, EpochNo>,
}

impl PoolState {
    pub fn new() -> Self {
        PoolState {
            pools: BTreeMap::new(),
            retiring: BTreeMap::new(),
        }
    }
}

impl Default for PoolState {
    fn default() -> Self {
        Self::new()
    }
}

/// Minimal pool parameters stored in delegation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolParams {
    pub pool_id: PoolId,
    pub vrf_hash: Hash32,
    pub pledge: Coin,
    pub cost: Coin,
    pub margin: (u64, u64),
    pub reward_account: Vec<u8>,
    /// Pool owner key hashes (for pledge satisfaction check).
    /// If empty, pledge check is skipped.
    pub owners: Vec<ade_types::Hash28>,
}

/// Apply a single certificate to the certificate state.
///
/// Pure function: consumes a reference to the current state and certificate,
/// returns a new state or a typed error.
///
/// `key_deposit` is the protocol parameter controlling stake key deposits.
/// `cert_index` is the position of this certificate in the transaction's cert array
/// (used for error reporting).
pub fn apply_cert(
    cert_state: &CertState,
    cert: &Certificate,
    key_deposit: Coin,
    cert_index: u16,
) -> Result<CertState, LedgerError> {
    match cert {
        Certificate::StakeRegistration(cred) => {
            apply_stake_registration(cert_state, cred, key_deposit, cert_index)
        }
        Certificate::StakeDeregistration(cred) => {
            apply_stake_deregistration(cert_state, cred, cert_index)
        }
        Certificate::StakeDelegation {
            credential,
            pool_id,
        } => apply_stake_delegation(cert_state, credential, pool_id, cert_index),
        Certificate::PoolRegistration(pool_cert) => {
            apply_pool_registration(cert_state, pool_cert, cert_index)
        }
        Certificate::PoolRetirement { pool_id, epoch } => {
            apply_pool_retirement(cert_state, pool_id, *epoch, cert_index)
        }
        Certificate::GenesisKeyDelegation { .. } => {
            // Genesis key delegations are governance-only and do not mutate
            // the delegation or pool state. Pass through unchanged.
            Ok(cert_state.clone())
        }
        Certificate::MIRTransfer(_mir) => {
            // MIR certs affect reserves/treasury at the epoch boundary,
            // not delegation state. Pass through unchanged.
            Ok(cert_state.clone())
        }
    }
}

/// Register a staking credential.
fn apply_stake_registration(
    state: &CertState,
    cred: &StakeCredential,
    key_deposit: Coin,
    cert_index: u16,
) -> Result<CertState, LedgerError> {
    if state.delegation.registrations.contains_key(cred) {
        return Err(LedgerError::InvalidCertificate(CertificateError {
            cert_index,
            reason: CertFailureReason::StakeAlreadyRegistered,
        }));
    }

    let mut new_state = state.clone();
    new_state
        .delegation
        .registrations
        .insert(cred.clone(), key_deposit);
    new_state
        .delegation
        .rewards
        .insert(cred.clone(), Coin::ZERO);
    Ok(new_state)
}

/// Deregister a staking credential.
fn apply_stake_deregistration(
    state: &CertState,
    cred: &StakeCredential,
    cert_index: u16,
) -> Result<CertState, LedgerError> {
    if !state.delegation.registrations.contains_key(cred) {
        return Err(LedgerError::InvalidCertificate(CertificateError {
            cert_index,
            reason: CertFailureReason::StakeNotRegistered,
        }));
    }

    let mut new_state = state.clone();
    new_state.delegation.registrations.remove(cred);
    new_state.delegation.delegations.remove(cred);
    new_state.delegation.rewards.remove(cred);
    Ok(new_state)
}

/// Delegate stake from a credential to a pool.
fn apply_stake_delegation(
    state: &CertState,
    cred: &StakeCredential,
    pool_id: &PoolId,
    cert_index: u16,
) -> Result<CertState, LedgerError> {
    // Credential must be registered to delegate
    if !state.delegation.registrations.contains_key(cred) {
        return Err(LedgerError::InvalidCertificate(CertificateError {
            cert_index,
            reason: CertFailureReason::StakeNotRegistered,
        }));
    }

    // Target pool must be registered
    if !state.pool.pools.contains_key(pool_id) {
        return Err(LedgerError::InvalidCertificate(CertificateError {
            cert_index,
            reason: CertFailureReason::PoolNotRegistered,
        }));
    }

    let mut new_state = state.clone();
    new_state
        .delegation
        .delegations
        .insert(cred.clone(), pool_id.clone());
    Ok(new_state)
}

/// Register (or update) a stake pool.
fn apply_pool_registration(
    state: &CertState,
    pool_cert: &PoolRegistrationCert,
    _cert_index: u16,
) -> Result<CertState, LedgerError> {
    let params = PoolParams {
        pool_id: pool_cert.pool_id.clone(),
        vrf_hash: pool_cert.vrf_hash.clone(),
        pledge: pool_cert.pledge,
        cost: pool_cert.cost,
        margin: pool_cert.margin,
        reward_account: pool_cert.reward_account.clone(),
        owners: pool_cert.owners.clone(),
    };

    let mut new_state = state.clone();
    // Insert or update — pool re-registration is allowed and overwrites params.
    new_state
        .pool
        .pools
        .insert(pool_cert.pool_id.clone(), params);
    // If the pool was previously scheduled for retirement, cancel it.
    new_state.pool.retiring.remove(&pool_cert.pool_id);
    Ok(new_state)
}

/// Schedule a pool for retirement at a given epoch.
fn apply_pool_retirement(
    state: &CertState,
    pool_id: &PoolId,
    epoch: EpochNo,
    cert_index: u16,
) -> Result<CertState, LedgerError> {
    if !state.pool.pools.contains_key(pool_id) {
        return Err(LedgerError::InvalidCertificate(CertificateError {
            cert_index,
            reason: CertFailureReason::PoolNotRegistered,
        }));
    }

    let mut new_state = state.clone();
    new_state.pool.retiring.insert(pool_id.clone(), epoch);
    Ok(new_state)
}

/// Apply a sequence of certificates to the certificate state.
///
/// Processes certificates in order, threading state through each application.
pub fn apply_certs(
    cert_state: &CertState,
    certs: &[Certificate],
    key_deposit: Coin,
) -> Result<CertState, LedgerError> {
    let mut state = cert_state.clone();
    for (i, cert) in certs.iter().enumerate() {
        state = apply_cert(&state, cert, key_deposit, i as u16)?;
    }
    Ok(state)
}

// ===========================================================================
// PHASE4-B4: native owner-tagged Conway cert-state accumulation
// ===========================================================================

/// The authoritative state owner of a governance-affecting certificate effect.
///
/// B4 owns delegation/pool [`CertState`] only. Governance effects are
/// owner-tagged to [`GovernanceOwner::ConwayGovState`] and routed out of B4's
/// mutation scope — they are observed, never silently neutralized (the owner
/// exists) and never applied here. A future governance-accumulation cluster
/// (PHASE4-B5) applies them into `ConwayGovState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceOwner {
    ConwayGovState,
}

/// A governance certificate effect, owner-tagged to [`GovernanceOwner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceCertEffect {
    VoteDelegation,
    StakeVoteDelegation,
    CommitteeHotKeyAuthorization,
    CommitteeColdKeyResignation,
    DRepRegistration,
    DRepUnregistration,
    DRepUpdate,
}

/// A governance effect tagged with the state owner that applies it (PHASE4-B5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnerTaggedEffect {
    pub owner: GovernanceOwner,
    pub effect: GovernanceCertEffect,
}

/// Closed, owner-tagged classification of a Conway certificate's cert-state
/// effect. There is no `Neutral`: every defined Conway tag has an owner.
/// Composite certs (tags 10/12/13) carry both a B4-owned [`CertState`] mutation
/// **and** an owner-tagged governance effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConwayCertAction {
    /// Mutates only B4-owned delegation/pool [`CertState`].
    MutateCertState,
    /// Owner-tagged governance effect only — no [`CertState`] mutation.
    Governance(GovernanceCertEffect),
    /// Composite: mutates [`CertState`] **and** emits a governance effect.
    CertStateAndGovernance(GovernanceCertEffect),
    /// A Conway-removed tag (5/6) — deterministic era-validity reject.
    NotValidInEra,
}

/// Classify a Conway certificate into its closed, owner-tagged action.
/// Compiler-exhaustive over all 18 tags; the totality test pins this.
pub fn conway_cert_action(cert: &ConwayCert) -> ConwayCertAction {
    use GovernanceCertEffect::*;
    match cert {
        // Delegation/pool — B4-owned CertState.
        ConwayCert::AccountRegistration { .. }
        | ConwayCert::AccountUnregistration { .. }
        | ConwayCert::StakeDelegation { .. }
        | ConwayCert::PoolRegistration(_)
        | ConwayCert::PoolRetirement { .. }
        | ConwayCert::AccountRegistrationDeposit { .. }
        | ConwayCert::AccountUnregistrationDeposit { .. }
        | ConwayCert::StakeRegistrationDelegation { .. } => ConwayCertAction::MutateCertState,

        // Governance-only — owner-tagged to ConwayGovState.
        ConwayCert::VoteDelegation { .. } => ConwayCertAction::Governance(VoteDelegation),
        ConwayCert::AuthCommitteeHot { .. } => {
            ConwayCertAction::Governance(CommitteeHotKeyAuthorization)
        }
        ConwayCert::ResignCommitteeCold { .. } => {
            ConwayCertAction::Governance(CommitteeColdKeyResignation)
        }
        ConwayCert::DRepRegistration { .. } => ConwayCertAction::Governance(DRepRegistration),
        ConwayCert::DRepUnregistration { .. } => ConwayCertAction::Governance(DRepUnregistration),
        ConwayCert::DRepUpdate { .. } => ConwayCertAction::Governance(DRepUpdate),

        // Composite — CertState mutation + owner-tagged governance.
        ConwayCert::StakeVoteDelegation { .. } => {
            ConwayCertAction::CertStateAndGovernance(StakeVoteDelegation)
        }
        ConwayCert::VoteRegistrationDelegation { .. } => {
            ConwayCertAction::CertStateAndGovernance(VoteDelegation)
        }
        ConwayCert::StakeVoteRegistrationDelegation { .. } => {
            ConwayCertAction::CertStateAndGovernance(StakeVoteDelegation)
        }

        // Era-removed.
        ConwayCert::RemovedInConway { .. } => ConwayCertAction::NotValidInEra,
    }
}

/// Environment for owner-tagged Conway cert-state accumulation.
#[derive(Debug, Clone, Copy)]
pub struct ConwayCertEnv {
    /// Legacy implicit key deposit (tag 0). Explicit-deposit variants carry
    /// their own deposit in the certificate.
    pub key_deposit: Coin,
    /// Positional index of the cert within its tx (error reporting; parity with
    /// the Shelley apply path).
    pub cert_index: u16,
}

/// Outcome of applying one Conway certificate: the mutated B4-owned
/// [`CertState`] plus any owner-tagged governance effects observed. The effects
/// are routed out of B4's mutation scope (applied by PHASE4-B5), never swallowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConwayCertOutcome {
    pub state: CertState,
    pub owner_tagged: Vec<OwnerTaggedEffect>,
}

/// Apply one Conway certificate to the B4-owned [`CertState`] (delegation +
/// pool), consuming Conway-native meaning over the owner-complete `ConwayCert`.
///
/// - Delegation/pool certs mutate `CertState` via the shared apply helpers.
/// - Governance certs are owner-tagged to `ConwayGovState` and returned in
///   `owner_tagged` without mutating `CertState` (PHASE4-B5 applies them).
/// - Composite certs (10/12/13) do both.
/// - Removed tags (5/6) reject with [`LedgerError::EraInvalidCertificate`].
///
/// No Conway certificate is reduced into the Shelley `Certificate`, flattened to
/// neutral, or silently swallowed.
pub fn apply_conway_cert(
    state: &CertState,
    cert: &ConwayCert,
    env: &ConwayCertEnv,
) -> Result<ConwayCertOutcome, LedgerError> {
    use GovernanceCertEffect::*;
    let idx = env.cert_index;

    let tagged = |state: CertState, effect: GovernanceCertEffect| ConwayCertOutcome {
        state,
        owner_tagged: vec![OwnerTaggedEffect {
            owner: GovernanceOwner::ConwayGovState,
            effect,
        }],
    };
    let plain = |state: CertState| ConwayCertOutcome {
        state,
        owner_tagged: Vec::new(),
    };

    let outcome = match cert {
        // --- delegation/pool: B4-owned CertState ---
        ConwayCert::AccountRegistration { credential } => {
            plain(apply_stake_registration(state, credential, env.key_deposit, idx)?)
        }
        ConwayCert::AccountRegistrationDeposit { credential, deposit } => {
            plain(apply_stake_registration(state, credential, *deposit, idx)?)
        }
        ConwayCert::AccountUnregistration { credential } => {
            plain(apply_stake_deregistration(state, credential, idx)?)
        }
        ConwayCert::AccountUnregistrationDeposit { credential, .. } => {
            plain(apply_stake_deregistration(state, credential, idx)?)
        }
        ConwayCert::StakeDelegation { credential, pool_id } => {
            plain(apply_stake_delegation(state, credential, pool_id, idx)?)
        }
        ConwayCert::PoolRegistration(pool_cert) => {
            plain(apply_pool_registration(state, pool_cert, idx)?)
        }
        ConwayCert::PoolRetirement { pool_id, epoch } => {
            plain(apply_pool_retirement(state, pool_id, *epoch, idx)?)
        }
        ConwayCert::StakeRegistrationDelegation { credential, pool_id, deposit } => {
            let s = apply_stake_registration(state, credential, *deposit, idx)?;
            plain(apply_stake_delegation(&s, credential, pool_id, idx)?)
        }

        // --- composite: CertState mutation + owner-tagged governance ---
        ConwayCert::StakeVoteDelegation { credential, pool_id, .. } => {
            let s = apply_stake_delegation(state, credential, pool_id, idx)?;
            tagged(s, StakeVoteDelegation)
        }
        ConwayCert::VoteRegistrationDelegation { credential, deposit, .. } => {
            let s = apply_stake_registration(state, credential, *deposit, idx)?;
            tagged(s, VoteDelegation)
        }
        ConwayCert::StakeVoteRegistrationDelegation { credential, pool_id, deposit, .. } => {
            let s = apply_stake_registration(state, credential, *deposit, idx)?;
            let s = apply_stake_delegation(&s, credential, pool_id, idx)?;
            tagged(s, StakeVoteDelegation)
        }

        // --- governance-only: owner-tagged, CertState unchanged ---
        ConwayCert::VoteDelegation { .. } => tagged(state.clone(), VoteDelegation),
        ConwayCert::AuthCommitteeHot { .. } => tagged(state.clone(), CommitteeHotKeyAuthorization),
        ConwayCert::ResignCommitteeCold { .. } => {
            tagged(state.clone(), CommitteeColdKeyResignation)
        }
        ConwayCert::DRepRegistration { .. } => tagged(state.clone(), DRepRegistration),
        ConwayCert::DRepUnregistration { .. } => tagged(state.clone(), DRepUnregistration),
        ConwayCert::DRepUpdate { .. } => tagged(state.clone(), DRepUpdate),

        // --- era-removed: deterministic reject ---
        ConwayCert::RemovedInConway { tag } => {
            return Err(LedgerError::EraInvalidCertificate(EraInvalidCertificateError {
                cert_index: idx,
                removed_tag: *tag,
            }));
        }
    };

    Ok(outcome)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_types::Hash28;
    use ade_types::shelley::cert::{MIRCert, MIRPot};

    fn make_cred(byte: u8) -> StakeCredential {
        StakeCredential::KeyHash(Hash28([byte; 28]))
    }

    fn make_pool_id(byte: u8) -> PoolId {
        PoolId(Hash28([byte; 28]))
    }

    fn make_pool_cert(byte: u8) -> PoolRegistrationCert {
        PoolRegistrationCert {
            pool_id: make_pool_id(byte),
            vrf_hash: Hash32([byte; 32]),
            pledge: Coin(100_000_000),
            cost: Coin(340_000_000),
            margin: (1, 100),
            reward_account: vec![0xe0, byte],
            owners: vec![Hash28([byte; 28])],
        }
    }

    fn key_deposit() -> Coin {
        Coin(2_000_000)
    }

    // -----------------------------------------------------------------------
    // Stake registration
    // -----------------------------------------------------------------------

    #[test]
    fn stake_registration_succeeds() {
        let state = CertState::new();
        let cert = Certificate::StakeRegistration(make_cred(0x01));
        let new_state = apply_cert(&state, &cert, key_deposit(), 0).unwrap();

        assert!(new_state
            .delegation
            .registrations
            .contains_key(&make_cred(0x01)));
        assert_eq!(
            new_state.delegation.registrations[&make_cred(0x01)],
            key_deposit()
        );
        assert_eq!(
            new_state.delegation.rewards[&make_cred(0x01)],
            Coin::ZERO
        );
    }

    #[test]
    fn stake_registration_duplicate_fails() {
        let state = CertState::new();
        let cert = Certificate::StakeRegistration(make_cred(0x01));
        let state = apply_cert(&state, &cert, key_deposit(), 0).unwrap();

        let result = apply_cert(&state, &cert, key_deposit(), 1);
        assert!(matches!(
            result,
            Err(LedgerError::InvalidCertificate(CertificateError {
                cert_index: 1,
                reason: CertFailureReason::StakeAlreadyRegistered,
            }))
        ));
    }

    // -----------------------------------------------------------------------
    // Stake deregistration
    // -----------------------------------------------------------------------

    #[test]
    fn stake_deregistration_succeeds() {
        let state = CertState::new();
        let reg = Certificate::StakeRegistration(make_cred(0x02));
        let state = apply_cert(&state, &reg, key_deposit(), 0).unwrap();

        let dereg = Certificate::StakeDeregistration(make_cred(0x02));
        let new_state = apply_cert(&state, &dereg, key_deposit(), 1).unwrap();

        assert!(!new_state
            .delegation
            .registrations
            .contains_key(&make_cred(0x02)));
        assert!(!new_state
            .delegation
            .rewards
            .contains_key(&make_cred(0x02)));
    }

    #[test]
    fn stake_deregistration_unregistered_fails() {
        let state = CertState::new();
        let cert = Certificate::StakeDeregistration(make_cred(0x03));
        let result = apply_cert(&state, &cert, key_deposit(), 0);
        assert!(matches!(
            result,
            Err(LedgerError::InvalidCertificate(CertificateError {
                cert_index: 0,
                reason: CertFailureReason::StakeNotRegistered,
            }))
        ));
    }

    #[test]
    fn stake_deregistration_removes_delegation() {
        let state = CertState::new();

        // Register pool and stake
        let pool_cert = Certificate::PoolRegistration(make_pool_cert(0xaa));
        let state = apply_cert(&state, &pool_cert, key_deposit(), 0).unwrap();

        let reg = Certificate::StakeRegistration(make_cred(0x04));
        let state = apply_cert(&state, &reg, key_deposit(), 1).unwrap();

        let del = Certificate::StakeDelegation {
            credential: make_cred(0x04),
            pool_id: make_pool_id(0xaa),
        };
        let state = apply_cert(&state, &del, key_deposit(), 2).unwrap();
        assert!(state
            .delegation
            .delegations
            .contains_key(&make_cred(0x04)));

        let dereg = Certificate::StakeDeregistration(make_cred(0x04));
        let state = apply_cert(&state, &dereg, key_deposit(), 3).unwrap();
        assert!(!state
            .delegation
            .delegations
            .contains_key(&make_cred(0x04)));
    }

    // -----------------------------------------------------------------------
    // Stake delegation
    // -----------------------------------------------------------------------

    #[test]
    fn stake_delegation_succeeds() {
        let state = CertState::new();

        // Register pool first
        let pool_cert = Certificate::PoolRegistration(make_pool_cert(0xbb));
        let state = apply_cert(&state, &pool_cert, key_deposit(), 0).unwrap();

        // Register stake
        let reg = Certificate::StakeRegistration(make_cred(0x05));
        let state = apply_cert(&state, &reg, key_deposit(), 1).unwrap();

        // Delegate
        let del = Certificate::StakeDelegation {
            credential: make_cred(0x05),
            pool_id: make_pool_id(0xbb),
        };
        let new_state = apply_cert(&state, &del, key_deposit(), 2).unwrap();
        assert_eq!(
            new_state.delegation.delegations[&make_cred(0x05)],
            make_pool_id(0xbb)
        );
    }

    #[test]
    fn stake_delegation_unregistered_credential_fails() {
        let state = CertState::new();

        let pool_cert = Certificate::PoolRegistration(make_pool_cert(0xcc));
        let state = apply_cert(&state, &pool_cert, key_deposit(), 0).unwrap();

        let del = Certificate::StakeDelegation {
            credential: make_cred(0x06),
            pool_id: make_pool_id(0xcc),
        };
        let result = apply_cert(&state, &del, key_deposit(), 1);
        assert!(matches!(
            result,
            Err(LedgerError::InvalidCertificate(CertificateError {
                cert_index: 1,
                reason: CertFailureReason::StakeNotRegistered,
            }))
        ));
    }

    #[test]
    fn stake_delegation_to_unregistered_pool_fails() {
        let state = CertState::new();

        let reg = Certificate::StakeRegistration(make_cred(0x07));
        let state = apply_cert(&state, &reg, key_deposit(), 0).unwrap();

        let del = Certificate::StakeDelegation {
            credential: make_cred(0x07),
            pool_id: make_pool_id(0xdd),
        };
        let result = apply_cert(&state, &del, key_deposit(), 1);
        assert!(matches!(
            result,
            Err(LedgerError::InvalidCertificate(CertificateError {
                cert_index: 1,
                reason: CertFailureReason::PoolNotRegistered,
            }))
        ));
    }

    // -----------------------------------------------------------------------
    // Pool registration
    // -----------------------------------------------------------------------

    #[test]
    fn pool_registration_succeeds() {
        let state = CertState::new();
        let cert = Certificate::PoolRegistration(make_pool_cert(0xee));
        let new_state = apply_cert(&state, &cert, key_deposit(), 0).unwrap();

        assert!(new_state.pool.pools.contains_key(&make_pool_id(0xee)));
        assert_eq!(
            new_state.pool.pools[&make_pool_id(0xee)].pledge,
            Coin(100_000_000)
        );
    }

    #[test]
    fn pool_re_registration_updates_params() {
        let state = CertState::new();
        let cert = Certificate::PoolRegistration(make_pool_cert(0xff));
        let state = apply_cert(&state, &cert, key_deposit(), 0).unwrap();

        // Re-register with different pledge
        let mut updated_cert = make_pool_cert(0xff);
        updated_cert.pledge = Coin(200_000_000);
        let cert2 = Certificate::PoolRegistration(updated_cert);
        let new_state = apply_cert(&state, &cert2, key_deposit(), 1).unwrap();

        assert_eq!(
            new_state.pool.pools[&make_pool_id(0xff)].pledge,
            Coin(200_000_000)
        );
    }

    #[test]
    fn pool_re_registration_cancels_retirement() {
        let state = CertState::new();
        let cert = Certificate::PoolRegistration(make_pool_cert(0x11));
        let state = apply_cert(&state, &cert, key_deposit(), 0).unwrap();

        // Schedule retirement
        let retire = Certificate::PoolRetirement {
            pool_id: make_pool_id(0x11),
            epoch: EpochNo(10),
        };
        let state = apply_cert(&state, &retire, key_deposit(), 1).unwrap();
        assert!(state.pool.retiring.contains_key(&make_pool_id(0x11)));

        // Re-register cancels retirement
        let cert2 = Certificate::PoolRegistration(make_pool_cert(0x11));
        let new_state = apply_cert(&state, &cert2, key_deposit(), 2).unwrap();
        assert!(!new_state.pool.retiring.contains_key(&make_pool_id(0x11)));
    }

    // -----------------------------------------------------------------------
    // Pool retirement
    // -----------------------------------------------------------------------

    #[test]
    fn pool_retirement_succeeds() {
        let state = CertState::new();
        let cert = Certificate::PoolRegistration(make_pool_cert(0x22));
        let state = apply_cert(&state, &cert, key_deposit(), 0).unwrap();

        let retire = Certificate::PoolRetirement {
            pool_id: make_pool_id(0x22),
            epoch: EpochNo(15),
        };
        let new_state = apply_cert(&state, &retire, key_deposit(), 1).unwrap();
        assert_eq!(
            new_state.pool.retiring[&make_pool_id(0x22)],
            EpochNo(15)
        );
    }

    #[test]
    fn pool_retirement_unregistered_pool_fails() {
        let state = CertState::new();
        let retire = Certificate::PoolRetirement {
            pool_id: make_pool_id(0x33),
            epoch: EpochNo(20),
        };
        let result = apply_cert(&state, &retire, key_deposit(), 0);
        assert!(matches!(
            result,
            Err(LedgerError::InvalidCertificate(CertificateError {
                cert_index: 0,
                reason: CertFailureReason::PoolNotRegistered,
            }))
        ));
    }

    // -----------------------------------------------------------------------
    // Genesis key delegation / MIR pass-through
    // -----------------------------------------------------------------------

    #[test]
    fn genesis_key_delegation_passes_through() {
        let state = CertState::new();
        let cert = Certificate::GenesisKeyDelegation {
            genesis_hash: Hash28([0x44; 28]),
            delegate_hash: Hash28([0x55; 28]),
            vrf_hash: Hash32([0x66; 32]),
        };
        let new_state = apply_cert(&state, &cert, key_deposit(), 0).unwrap();
        assert_eq!(new_state, state);
    }

    #[test]
    fn mir_cert_passes_through() {
        let state = CertState::new();
        let cert = Certificate::MIRTransfer(MIRCert {
            pot: MIRPot::Reserves,
            rewards: BTreeMap::new(),
        });
        let new_state = apply_cert(&state, &cert, key_deposit(), 0).unwrap();
        assert_eq!(new_state, state);
    }

    // -----------------------------------------------------------------------
    // apply_certs batch
    // -----------------------------------------------------------------------

    #[test]
    fn apply_certs_batch() {
        let state = CertState::new();
        let certs = vec![
            Certificate::PoolRegistration(make_pool_cert(0xaa)),
            Certificate::StakeRegistration(make_cred(0x01)),
            Certificate::StakeDelegation {
                credential: make_cred(0x01),
                pool_id: make_pool_id(0xaa),
            },
        ];

        let new_state = apply_certs(&state, &certs, key_deposit()).unwrap();
        assert!(new_state.pool.pools.contains_key(&make_pool_id(0xaa)));
        assert!(new_state
            .delegation
            .registrations
            .contains_key(&make_cred(0x01)));
        assert_eq!(
            new_state.delegation.delegations[&make_cred(0x01)],
            make_pool_id(0xaa)
        );
    }

    #[test]
    fn apply_certs_batch_fails_on_invalid() {
        let state = CertState::new();
        let certs = vec![
            Certificate::StakeRegistration(make_cred(0x01)),
            // This delegation will fail because no pool is registered
            Certificate::StakeDelegation {
                credential: make_cred(0x01),
                pool_id: make_pool_id(0xbb),
            },
        ];

        let result = apply_certs(&state, &certs, key_deposit());
        assert!(matches!(
            result,
            Err(LedgerError::InvalidCertificate(CertificateError {
                cert_index: 1,
                reason: CertFailureReason::PoolNotRegistered,
            }))
        ));
    }

    // -----------------------------------------------------------------------
    // Determinism
    // -----------------------------------------------------------------------

    #[test]
    fn apply_cert_deterministic() {
        let state = CertState::new();
        let cert = Certificate::StakeRegistration(make_cred(0x99));

        let r1 = apply_cert(&state, &cert, key_deposit(), 0).unwrap();
        let r2 = apply_cert(&state, &cert, key_deposit(), 0).unwrap();
        assert_eq!(r1, r2);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod conway_apply {
    use super::*;
    use ade_types::conway::cert::{ConwayCert, DRep};
    use ade_types::Hash28;

    fn cred(b: u8) -> StakeCredential {
        StakeCredential::KeyHash(Hash28([b; 28]))
    }
    fn pool(b: u8) -> PoolId {
        PoolId(Hash28([b; 28]))
    }
    fn env() -> ConwayCertEnv {
        ConwayCertEnv {
            key_deposit: Coin(2_000_000),
            cert_index: 0,
        }
    }
    fn preg(b: u8) -> PoolRegistrationCert {
        PoolRegistrationCert {
            pool_id: pool(b),
            vrf_hash: Hash32([0u8; 32]),
            pledge: Coin(0),
            cost: Coin(0),
            margin: (0, 1),
            reward_account: vec![],
            owners: vec![Hash28([b; 28])],
        }
    }

    /// Every `ConwayCert` variant with its expected owner-tagged action. If a
    /// variant is added without updating this table, the totality test fails;
    /// if it is added without a `conway_cert_action` arm, the build fails.
    fn variants_with_action() -> Vec<(ConwayCert, ConwayCertAction)> {
        use ConwayCertAction::*;
        use GovernanceCertEffect as G;
        vec![
            (ConwayCert::AccountRegistration { credential: cred(1) }, MutateCertState),
            (ConwayCert::AccountUnregistration { credential: cred(1) }, MutateCertState),
            (ConwayCert::StakeDelegation { credential: cred(1), pool_id: pool(9) }, MutateCertState),
            (ConwayCert::PoolRegistration(preg(9)), MutateCertState),
            (ConwayCert::PoolRetirement { pool_id: pool(9), epoch: EpochNo(500) }, MutateCertState),
            (ConwayCert::RemovedInConway { tag: 5 }, NotValidInEra),
            (ConwayCert::RemovedInConway { tag: 6 }, NotValidInEra),
            (ConwayCert::AccountRegistrationDeposit { credential: cred(1), deposit: Coin(2_000_000) }, MutateCertState),
            (ConwayCert::AccountUnregistrationDeposit { credential: cred(1), refund: Coin(2_000_000) }, MutateCertState),
            (ConwayCert::VoteDelegation { credential: cred(1), drep: DRep::AlwaysAbstain }, Governance(G::VoteDelegation)),
            (ConwayCert::StakeVoteDelegation { credential: cred(1), pool_id: pool(9), drep: DRep::AlwaysAbstain }, CertStateAndGovernance(G::StakeVoteDelegation)),
            (ConwayCert::StakeRegistrationDelegation { credential: cred(1), pool_id: pool(9), deposit: Coin(2_000_000) }, MutateCertState),
            (ConwayCert::VoteRegistrationDelegation { credential: cred(1), drep: DRep::AlwaysAbstain, deposit: Coin(2_000_000) }, CertStateAndGovernance(G::VoteDelegation)),
            (ConwayCert::StakeVoteRegistrationDelegation { credential: cred(1), pool_id: pool(9), drep: DRep::AlwaysAbstain, deposit: Coin(2_000_000) }, CertStateAndGovernance(G::StakeVoteDelegation)),
            (ConwayCert::AuthCommitteeHot { cold_credential: cred(1), hot_credential: cred(2) }, Governance(G::CommitteeHotKeyAuthorization)),
            (ConwayCert::ResignCommitteeCold { cold_credential: cred(1) }, Governance(G::CommitteeColdKeyResignation)),
            (ConwayCert::DRepRegistration { drep_credential: cred(1), deposit: Coin(500_000_000) }, Governance(G::DRepRegistration)),
            (ConwayCert::DRepUnregistration { drep_credential: cred(1), refund: Coin(500_000_000) }, Governance(G::DRepUnregistration)),
            (ConwayCert::DRepUpdate { drep_credential: cred(1) }, Governance(G::DRepUpdate)),
        ]
    }

    /// Totality: every one of the 18 Conway tags classifies to its declared
    /// owner-tagged action. No variant is `Neutral` (the type has no such case);
    /// governance vs CertState vs composite vs era-reject is pinned per variant.
    #[test]
    fn conway_cert_action_total() {
        let table = variants_with_action();
        assert_eq!(table.len(), 19, "all tags incl. both removed (5/6) present");
        for (cert, expected) in table {
            assert_eq!(conway_cert_action(&cert), expected, "action for {cert:?}");
        }
    }

    /// `apply_conway_cert`'s outcome shape agrees with `conway_cert_action` for
    /// every variant (no swallow, no flatten): CertState mutations carry no
    /// owner-tag; governance certs leave CertState unchanged with one owner-tag;
    /// composites do both; removed tags reject.
    #[test]
    fn apply_outcome_agrees_with_action() {
        // State seeded so delegation/registration/retirement all succeed:
        // cred(1) NOT yet registered (so registration works), pool(9) present.
        for (cert, action) in variants_with_action() {
            let mut state = CertState::new();
            state.pool.pools.insert(
                pool(9),
                PoolParams {
                    pool_id: pool(9),
                    vrf_hash: Hash32([0u8; 32]),
                    pledge: Coin(0),
                    cost: Coin(0),
                    margin: (0, 1),
                    reward_account: vec![],
                    owners: vec![],
                },
            );
            // For deregistration/delegation variants the credential must exist.
            let needs_registered = matches!(
                cert,
                ConwayCert::AccountUnregistration { .. }
                    | ConwayCert::AccountUnregistrationDeposit { .. }
                    | ConwayCert::StakeDelegation { .. }
                    | ConwayCert::StakeVoteDelegation { .. }
            );
            if needs_registered {
                state.delegation.registrations.insert(cred(1), Coin(2_000_000));
            }

            let result = apply_conway_cert(&state, &cert, &env());
            match action {
                ConwayCertAction::NotValidInEra => {
                    assert!(result.is_err(), "removed tag must reject: {cert:?}");
                }
                ConwayCertAction::MutateCertState => {
                    let out = result.unwrap_or_else(|e| panic!("{cert:?}: {e:?}"));
                    assert!(out.owner_tagged.is_empty(), "CertState-only carries no owner tag: {cert:?}");
                }
                ConwayCertAction::Governance(effect) => {
                    let out = result.unwrap();
                    assert_eq!(out.state, state, "governance cert must not mutate CertState: {cert:?}");
                    assert_eq!(
                        out.owner_tagged,
                        vec![OwnerTaggedEffect { owner: GovernanceOwner::ConwayGovState, effect }]
                    );
                }
                ConwayCertAction::CertStateAndGovernance(effect) => {
                    let out = result.unwrap();
                    assert_ne!(out.state, state, "composite must mutate CertState: {cert:?}");
                    assert_eq!(
                        out.owner_tagged,
                        vec![OwnerTaggedEffect { owner: GovernanceOwner::ConwayGovState, effect }]
                    );
                }
            }
        }
    }

    #[test]
    fn pool_registration_populates_owners_from_cert() {
        let state = CertState::new();
        let cert = ConwayCert::PoolRegistration(preg(9));
        let out = apply_conway_cert(&state, &cert, &env()).unwrap();
        assert_eq!(out.state.pool.pools[&pool(9)].owners, vec![Hash28([9u8; 28])]);
    }

    #[test]
    fn removed_tag_rejects_as_era_invalid() {
        let state = CertState::new();
        let cert = ConwayCert::RemovedInConway { tag: 6 };
        let err = apply_conway_cert(&state, &cert, &env()).unwrap_err();
        assert!(matches!(err, LedgerError::EraInvalidCertificate(e) if e.removed_tag == 6));
    }

    #[test]
    fn drep_registration_is_owner_tagged_not_applied() {
        let state = CertState::new();
        let cert = ConwayCert::DRepRegistration { drep_credential: cred(1), deposit: Coin(500_000_000) };
        let out = apply_conway_cert(&state, &cert, &env()).unwrap();
        assert_eq!(out.state, state, "DRep registration does not touch B4-owned CertState");
        assert_eq!(
            out.owner_tagged,
            vec![OwnerTaggedEffect {
                owner: GovernanceOwner::ConwayGovState,
                effect: GovernanceCertEffect::DRepRegistration
            }]
        );
    }
}
