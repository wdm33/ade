// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeMap;
use ade_types::tx::{Coin, PoolId};
use ade_types::{EpochNo, Hash32};
use ade_types::shelley::cert::{
    Certificate, PoolRegistrationCert, StakeCredential,
};

use crate::error::{CertFailureReason, CertificateError, LedgerError};

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
        owners: Vec::new(), // TODO: parse from registration cert
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_types::Hash28;
    use ade_types::shelley::cert::{MIRCert, MIRPot};

    fn make_cred(byte: u8) -> StakeCredential {
        StakeCredential(Hash28([byte; 28]))
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
