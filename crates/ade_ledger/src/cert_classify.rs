// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::conway::cert::{CertDisposition, CoinSource, ConwayCert, DepositEffect};

use crate::delegation::CertState;
use crate::error::UnsupportedStateDependentDepositAccounting;
use crate::pparams::ConwayDepositParams;

/// Classify a single decoded Conway certificate into its closed deposit-effect
/// disposition.
///
/// Total over [`ConwayCert`] (compiler-enforced exhaustive match). The three
/// failure classes are kept distinct: decode failures never reach here;
/// known-but-removed tags map to [`CertDisposition::NotValidInConway`]; a
/// state-dependent effect that cannot be resolved from `registration_state`
/// returns [`UnsupportedStateDependentDepositAccounting`] rather than guessing.
pub fn classify(
    cert: &ConwayCert,
    deposit_params: &ConwayDepositParams,
    registration_state: &CertState,
) -> Result<CertDisposition, UnsupportedStateDependentDepositAccounting> {
    let disposition = match cert {
        // tag 0 — legacy registration: implicit key deposit from canonical params.
        ConwayCert::AccountRegistration { .. } => CertDisposition::Accountable(
            DepositEffect::NewDeposit(CoinSource::DepositParam(deposit_params.key_deposit)),
        ),

        // tag 1 — legacy unregistration: refund equals the deposit recorded at
        // registration. State-dependent: resolved from the registrations map,
        // else a structured reject (never the key_deposit param, which can drift).
        ConwayCert::AccountUnregistration { credential } => {
            match registration_state.delegation.registrations.get(credential) {
                Some(recorded) => CertDisposition::Accountable(DepositEffect::Refund(
                    CoinSource::RegistrationState(*recorded),
                )),
                None => {
                    return Err(
                        UnsupportedStateDependentDepositAccounting::LegacyUnregistrationRefundUnresolved,
                    );
                }
            }
        }

        // tag 3 — pool registration: a new deposit only when the pool is new;
        // re-registration is an update with no tx-time deposit.
        ConwayCert::PoolRegistration(cert) => {
            if registration_state.pool.pools.contains_key(&cert.pool_id) {
                CertDisposition::Neutral
            } else {
                CertDisposition::Accountable(DepositEffect::NewDeposit(CoinSource::DepositParam(
                    deposit_params.pool_deposit,
                )))
            }
        }

        // Explicit-deposit registration variants — coin carried in the cert.
        ConwayCert::AccountRegistrationDeposit { deposit, .. }
        | ConwayCert::StakeRegistrationDelegation { deposit, .. }
        | ConwayCert::VoteRegistrationDelegation { deposit, .. }
        | ConwayCert::StakeVoteRegistrationDelegation { deposit, .. }
        | ConwayCert::DRepRegistration { deposit, .. } => CertDisposition::Accountable(
            DepositEffect::NewDeposit(CoinSource::ExplicitInCert(*deposit)),
        ),

        // Explicit-refund variants — coin carried in the cert.
        ConwayCert::AccountUnregistrationDeposit { refund, .. } => CertDisposition::Accountable(
            DepositEffect::Refund(CoinSource::ExplicitInCert(*refund)),
        ),
        ConwayCert::DRepUnregistration { refund, .. } => CertDisposition::Accountable(
            DepositEffect::Refund(CoinSource::ExplicitInCert(*refund)),
        ),

        // Neutral certs — no tx-time conservation effect.
        ConwayCert::StakeDelegation { .. }
        | ConwayCert::PoolRetirement { .. }
        | ConwayCert::VoteDelegation { .. }
        | ConwayCert::StakeVoteDelegation { .. }
        | ConwayCert::AuthCommitteeHot { .. }
        | ConwayCert::ResignCommitteeCold { .. }
        | ConwayCert::DRepUpdate { .. } => CertDisposition::Neutral,

        // Era-validity reject — known-but-removed tags (5/6).
        ConwayCert::RemovedInConway { .. } => CertDisposition::NotValidInConway,
    };

    Ok(disposition)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ade_types::conway::cert::DRep;
    use ade_types::shelley::cert::{PoolRegistrationCert, StakeCredential};
    use ade_types::tx::{Coin, PoolId};
    use ade_types::{EpochNo, Hash28};

    fn params() -> ConwayDepositParams {
        ConwayDepositParams {
            key_deposit: Coin(2_000_000),
            pool_deposit: Coin(500_000_000),
            drep_deposit: Coin(500_000_000),
            gov_action_deposit: Coin(100_000_000_000),
        }
    }

    fn cred(byte: u8) -> StakeCredential {
        StakeCredential(Hash28([byte; 28]))
    }

    fn pool(byte: u8) -> PoolId {
        PoolId(Hash28([byte; 28]))
    }

    fn preg(byte: u8) -> PoolRegistrationCert {
        PoolRegistrationCert {
            pool_id: pool(byte),
            vrf_hash: ade_types::Hash32([0u8; 32]),
            pledge: Coin(0),
            cost: Coin(0),
            margin: (0, 1),
            reward_account: vec![],
            owners: vec![],
        }
    }

    /// Every `ConwayCert` variant, enumerated. If a variant is added without a
    /// classifier arm, `classify` fails to compile (exhaustive match); if a
    /// variant is added without being listed here, this slice's totality test
    /// no longer covers it. The two together pin totality.
    fn all_variants() -> Vec<ConwayCert> {
        vec![
            ConwayCert::AccountRegistration { credential: cred(1) },
            ConwayCert::AccountUnregistration { credential: cred(1) },
            ConwayCert::StakeDelegation { credential: cred(1), pool_id: pool(9) },
            ConwayCert::PoolRegistration(preg(9)),
            ConwayCert::PoolRetirement { pool_id: pool(9), epoch: EpochNo(500) },
            ConwayCert::RemovedInConway { tag: 5 },
            ConwayCert::RemovedInConway { tag: 6 },
            ConwayCert::AccountRegistrationDeposit { credential: cred(1), deposit: Coin(2_000_000) },
            ConwayCert::AccountUnregistrationDeposit { credential: cred(1), refund: Coin(2_000_000) },
            ConwayCert::VoteDelegation { credential: cred(1), drep: DRep::AlwaysAbstain },
            ConwayCert::StakeVoteDelegation {
                credential: cred(1),
                pool_id: pool(9),
                drep: DRep::AlwaysAbstain,
            },
            ConwayCert::StakeRegistrationDelegation {
                credential: cred(1),
                pool_id: pool(9),
                deposit: Coin(2_000_000),
            },
            ConwayCert::VoteRegistrationDelegation {
                credential: cred(1),
                drep: DRep::AlwaysAbstain,
                deposit: Coin(2_000_000),
            },
            ConwayCert::StakeVoteRegistrationDelegation {
                credential: cred(1),
                pool_id: pool(9),
                drep: DRep::AlwaysAbstain,
                deposit: Coin(2_000_000),
            },
            ConwayCert::AuthCommitteeHot { cold_credential: cred(1), hot_credential: cred(2) },
            ConwayCert::ResignCommitteeCold { cold_credential: cred(1) },
            ConwayCert::DRepRegistration { drep_credential: cred(1), deposit: Coin(500_000_000) },
            ConwayCert::DRepUnregistration { drep_credential: cred(1), refund: Coin(500_000_000) },
            ConwayCert::DRepUpdate { drep_credential: cred(1) },
        ]
    }

    /// State pre-seeded so the state-dependent tags resolve: credential 1 has a
    /// recorded deposit (tag 1), and pool 9 is NOT registered (tag 3 is new).
    fn seeded_state() -> CertState {
        let mut state = CertState::new();
        state
            .delegation
            .registrations
            .insert(cred(1), Coin(2_000_000));
        state
    }

    #[test]
    fn class_mapping_is_total() {
        let params = params();
        let state = seeded_state();
        for cert in all_variants() {
            let result = classify(&cert, &params, &state);
            // Every variant resolves to a disposition with the seeded state;
            // the compiler-exhaustive match in `classify` guarantees no
            // unhandled variant. `RemovedInConway` is a disposition, not a
            // reject, so the only `Err` path here would be an unresolved
            // state-dependent case, which the seeded state precludes.
            assert!(
                result.is_ok(),
                "variant {cert:?} did not classify against seeded state"
            );
        }
    }

    #[test]
    fn legacy_unregistration_unresolved_is_unsupported_state_dependent() {
        let params = params();
        let empty = CertState::new();
        let cert = ConwayCert::AccountUnregistration { credential: cred(7) };
        let result = classify(&cert, &params, &empty);
        assert_eq!(
            result,
            Err(UnsupportedStateDependentDepositAccounting::LegacyUnregistrationRefundUnresolved)
        );
    }

    #[test]
    fn legacy_unregistration_resolves_recorded_deposit() {
        let params = params();
        let mut state = CertState::new();
        state
            .delegation
            .registrations
            .insert(cred(7), Coin(2_000_000));
        let cert = ConwayCert::AccountUnregistration { credential: cred(7) };
        let result = classify(&cert, &params, &state);
        assert_eq!(
            result,
            Ok(CertDisposition::Accountable(DepositEffect::Refund(
                CoinSource::RegistrationState(Coin(2_000_000))
            )))
        );
    }

    #[test]
    fn pool_reregistration_is_neutral() {
        use crate::delegation::PoolParams;
        let params = params();
        let mut state = CertState::new();
        state.pool.pools.insert(
            pool(9),
            PoolParams {
                pool_id: pool(9),
                vrf_hash: ade_types::Hash32([0u8; 32]),
                pledge: Coin(0),
                cost: Coin(0),
                margin: (0, 1),
                reward_account: vec![],
                owners: vec![],
            },
        );
        let cert = ConwayCert::PoolRegistration(preg(9));
        let result = classify(&cert, &params, &state);
        assert_eq!(result, Ok(CertDisposition::Neutral));
    }

    #[test]
    fn pool_new_registration_charges_pool_deposit() {
        let params = params();
        let state = CertState::new();
        let cert = ConwayCert::PoolRegistration(preg(9));
        let result = classify(&cert, &params, &state);
        assert_eq!(
            result,
            Ok(CertDisposition::Accountable(DepositEffect::NewDeposit(
                CoinSource::DepositParam(Coin(500_000_000))
            )))
        );
    }
}
