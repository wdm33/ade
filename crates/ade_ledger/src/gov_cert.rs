// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! PHASE4-B5: native Conway governance-certificate accumulation.
//!
//! B4 classified every Conway cert into a closed owner-tagged disposition and
//! routed the governance-affecting ones (vote-delegation 9/10/12/13, committee
//! 14/15, DRep 16/17/18) to [`crate::state::ConwayGovState`] *out of mutation
//! scope* — observed, not applied. B5 applies them.
//!
//! [`apply_conway_gov_cert`] is a pure dispatch over the owner-complete
//! [`ConwayCert`], reading the governance payloads directly off the cert (the
//! native-dispatch resolution of OQ-1). It mutates **only** governance-owned
//! fields of `ConwayGovState`; it neither takes nor returns the B4-owned
//! `CertState`, so it cannot mutate delegation/pool state and cannot
//! double-apply the delegation/pool half of composite certs.

use ade_types::conway::cert::ConwayCert;

use crate::error::{LedgerError, ValidationEnvironmentError};
use crate::state::{ConwayGovState, GovCertEnv};

/// Apply one Conway governance certificate to [`ConwayGovState`].
///
/// - **Vote delegation** (tags 9/10/12/13): `vote_delegations[cred] = drep`. The
///   stake/pool/registration half of composites (10/12/13) is B4-owned and is
///   **not** applied here.
/// - **Committee hot-key auth** (tag 14): `committee_hot_keys[hot] = cold`.
/// - **Committee cold resignation** (tag 15): remove every `committee_hot_keys`
///   entry authorizing `cold` (clears that member's hot authorization).
/// - **DRep registration / update** (tags 16/18): `drep_expiry[cred] =
///   env.current_epoch + env.drep_activity`. Requires `env`; absent env is a
///   structured fail-fast ([`ValidationEnvironmentError::MissingDRepActivityParam`]),
///   never a defaulted expiry.
/// - **DRep unregistration** (tag 17): remove `drep_expiry[cred]`.
/// - **CertState-only certs** (0/2/3/4/7/8/11) and **removed tags** (5/6): no
///   governance mutation — `gov_state` is returned unchanged (B4 owns or rejects
///   these).
///
/// `env` is `None` when the state lacks `drep_activity`; only tags 16/18 consult
/// it, so env-free certs accumulate regardless.
pub fn apply_conway_gov_cert(
    gov_state: &ConwayGovState,
    cert: &ConwayCert,
    env: Option<&GovCertEnv>,
) -> Result<ConwayGovState, LedgerError> {
    let mut gov = gov_state.clone();

    match cert {
        // --- vote delegation: gov.vote_delegations[cred] = drep ---
        // Composites 10/12/13 carry a B4-owned delegation/pool/registration half
        // applied by apply_conway_cert; only the drep target is applied here.
        ConwayCert::VoteDelegation { credential, drep }
        | ConwayCert::StakeVoteDelegation { credential, drep, .. }
        | ConwayCert::VoteRegistrationDelegation { credential, drep, .. }
        | ConwayCert::StakeVoteRegistrationDelegation { credential, drep, .. } => {
            gov.vote_delegations.insert(credential.clone(), drep.clone());
        }

        // --- committee hot-key authorization: committee_hot_keys[hot] = cold ---
        ConwayCert::AuthCommitteeHot {
            cold_credential,
            hot_credential,
        } => {
            gov.committee_hot_keys
                .insert(hot_credential.clone(), cold_credential.clone());
        }

        // --- committee cold resignation: drop the member's hot authorization ---
        ConwayCert::ResignCommitteeCold { cold_credential } => {
            let cold = cold_credential.clone();
            gov.committee_hot_keys.retain(|_hot, c| *c != cold);
        }

        // --- DRep registration / update: env-driven expiry ---
        ConwayCert::DRepRegistration { drep_credential, .. }
        | ConwayCert::DRepUpdate { drep_credential } => {
            let env = env.ok_or(LedgerError::ValidationEnvironment(
                ValidationEnvironmentError::MissingDRepActivityParam,
            ))?;
            // Checked: a DRep-activity param large enough to overflow u64 is an
            // ill-formed environment — deterministic halt, never a silent wrap to
            // a wrong expiry (matches the ledger's checked/saturating arithmetic
            // convention for authoritative state).
            let expiry = env.current_epoch.checked_add(env.drep_activity).ok_or(
                LedgerError::ValidationEnvironment(
                    ValidationEnvironmentError::DRepActivityOverflow,
                ),
            )?;
            gov.drep_expiry.insert(drep_credential.clone(), expiry);
        }

        // --- DRep unregistration: remove expiry (env-free) ---
        ConwayCert::DRepUnregistration { drep_credential, .. } => {
            gov.drep_expiry.remove(drep_credential);
        }

        // --- no governance effect: B4-owned CertState certs + removed tags ---
        ConwayCert::AccountRegistration { .. }
        | ConwayCert::AccountUnregistration { .. }
        | ConwayCert::StakeDelegation { .. }
        | ConwayCert::PoolRegistration(_)
        | ConwayCert::PoolRetirement { .. }
        | ConwayCert::AccountRegistrationDeposit { .. }
        | ConwayCert::AccountUnregistrationDeposit { .. }
        | ConwayCert::StakeRegistrationDelegation { .. }
        | ConwayCert::RemovedInConway { .. } => {}
    }

    Ok(gov)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::delegation::{apply_conway_cert, CertState, ConwayCertEnv};
    use ade_types::conway::cert::{ConwayCert, DRep};
    use ade_types::shelley::cert::{PoolRegistrationCert, StakeCredential};
    use ade_types::tx::{Coin, PoolId};
    use ade_types::{EpochNo, Hash28, Hash32};
    use std::collections::BTreeMap;

    fn cred(b: u8) -> StakeCredential {
        StakeCredential::KeyHash(Hash28([b; 28]))
    }
    fn pool(b: u8) -> PoolId {
        PoolId(Hash28([b; 28]))
    }
    fn h(b: u8) -> Hash28 {
        Hash28([b; 28])
    }
    fn empty_gov() -> ConwayGovState {
        ConwayGovState {
            proposals: Vec::new(),
            committee: BTreeMap::new(),
            committee_quorum: (2, 3),
            drep_expiry: BTreeMap::new(),
            gov_action_lifetime: 6,
            vote_delegations: BTreeMap::new(),
            pool_voting_thresholds: Vec::new(),
            drep_voting_thresholds: Vec::new(),
            committee_hot_keys: BTreeMap::new(),
            num_dormant: crate::state::DormantEpochs::Unversioned,
        }
    }
    fn env() -> GovCertEnv {
        GovCertEnv {
            current_epoch: 576,
            drep_activity: 20,
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
            owners: vec![h(b)],
        }
    }

    /// Every one of the 18 Conway tags, applied to an empty gov-state with env
    /// present, produces exactly the cluster-doc table's mutation (or none).
    /// Compiler-exhaustive in `apply_conway_gov_cert`; this pins behavior.
    #[test]
    fn gov_apply_total_over_18_tags() {
        let g = empty_gov();
        let e = env();
        let drep = DRep::KeyHash(h(0xDD));

        // -- vote delegation (9/10/12/13): vote_delegations[cred] = drep --
        for cert in [
            ConwayCert::VoteDelegation { credential: cred(1), drep: drep.clone() },
            ConwayCert::StakeVoteDelegation { credential: cred(1), pool_id: pool(9), drep: drep.clone() },
            ConwayCert::VoteRegistrationDelegation { credential: cred(1), drep: drep.clone(), deposit: Coin(2_000_000) },
            ConwayCert::StakeVoteRegistrationDelegation { credential: cred(1), pool_id: pool(9), drep: drep.clone(), deposit: Coin(2_000_000) },
        ] {
            let out = apply_conway_gov_cert(&g, &cert, Some(&e)).unwrap();
            assert_eq!(out.vote_delegations.get(&cred(1)), Some(&drep), "{cert:?}");
            assert!(out.committee_hot_keys.is_empty() && out.drep_expiry.is_empty(), "{cert:?}");
        }

        // -- committee hot-key auth (14): committee_hot_keys[hot] = cold --
        let out = apply_conway_gov_cert(
            &g,
            &ConwayCert::AuthCommitteeHot { cold_credential: cred(0xC0), hot_credential: cred(0x40) },
            Some(&e),
        ).unwrap();
        assert_eq!(out.committee_hot_keys.get(&cred(0x40)), Some(&cred(0xC0)));
        assert!(out.vote_delegations.is_empty() && out.drep_expiry.is_empty());

        // -- committee cold resignation (15): remove entries authorizing cold --
        let mut g_auth = empty_gov();
        g_auth.committee_hot_keys.insert(cred(0x40), cred(0xC0));
        g_auth.committee_hot_keys.insert(cred(0x41), cred(0xC1));
        let out = apply_conway_gov_cert(
            &g_auth,
            &ConwayCert::ResignCommitteeCold { cold_credential: cred(0xC0) },
            Some(&e),
        ).unwrap();
        assert_eq!(out.committee_hot_keys.get(&cred(0x40)), None, "0xC0 authorization removed");
        assert_eq!(out.committee_hot_keys.get(&cred(0x41)), Some(&cred(0xC1)), "0xC1 untouched");

        // -- DRep registration/update (16/18): drep_expiry = epoch + activity --
        for cert in [
            ConwayCert::DRepRegistration { drep_credential: cred(0xAA), deposit: Coin(500_000_000) },
            ConwayCert::DRepUpdate { drep_credential: cred(0xAA) },
        ] {
            let out = apply_conway_gov_cert(&g, &cert, Some(&e)).unwrap();
            assert_eq!(out.drep_expiry.get(&cred(0xAA)), Some(&(576 + 20)), "{cert:?}");
        }

        // -- DRep unregistration (17): remove drep_expiry --
        let mut g_drep = empty_gov();
        g_drep.drep_expiry.insert(cred(0xAA), 600);
        let out = apply_conway_gov_cert(
            &g_drep,
            &ConwayCert::DRepUnregistration { drep_credential: cred(0xAA), refund: Coin(500_000_000) },
            Some(&e),
        ).unwrap();
        assert_eq!(out.drep_expiry.get(&cred(0xAA)), None);

        // -- CertState-only (0/2/3/4/7/8/11) + removed (5/6): gov unchanged --
        for cert in [
            ConwayCert::AccountRegistration { credential: cred(1) },
            ConwayCert::AccountUnregistration { credential: cred(1) },
            ConwayCert::StakeDelegation { credential: cred(1), pool_id: pool(9) },
            ConwayCert::PoolRegistration(preg(9)),
            ConwayCert::PoolRetirement { pool_id: pool(9), epoch: EpochNo(600) },
            ConwayCert::AccountRegistrationDeposit { credential: cred(1), deposit: Coin(2_000_000) },
            ConwayCert::AccountUnregistrationDeposit { credential: cred(1), refund: Coin(2_000_000) },
            ConwayCert::StakeRegistrationDelegation { credential: cred(1), pool_id: pool(9), deposit: Coin(2_000_000) },
            ConwayCert::RemovedInConway { tag: 5 },
            ConwayCert::RemovedInConway { tag: 6 },
        ] {
            let out = apply_conway_gov_cert(&g, &cert, Some(&e)).unwrap();
            assert_eq!(out, g, "CertState-only/removed must not touch gov: {cert:?}");
        }
    }

    /// A composite cert's B4 half (apply_conway_cert -> CertState) and B5 half
    /// (apply_conway_gov_cert -> ConwayGovState) touch disjoint domains: B5 never
    /// mutates CertState (it has none), and applies the gov half exactly once.
    #[test]
    fn composite_gov_half_applied_once_certstate_untouched_by_b5() {
        // tag 10: StakeVoteDelegation { credential, pool_id, drep }
        let drep = DRep::KeyHash(h(0xDD));
        let cert = ConwayCert::StakeVoteDelegation {
            credential: cred(1),
            pool_id: pool(9),
            drep: drep.clone(),
        };

        // B4 half: CertState mutated (credential -> pool), gov untouched (no gov param).
        let mut cs = CertState::new();
        cs.delegation.registrations.insert(cred(1), Coin(2_000_000));
        cs.pool.pools.insert(
            pool(9),
            crate::delegation::PoolParams {
                pool_id: pool(9),
                vrf_hash: Hash32([0u8; 32]),
                pledge: Coin(0),
                cost: Coin(0),
                margin: (0, 1),
                reward_account: vec![],
                owners: vec![],
            },
        );
        let cenv = ConwayCertEnv { key_deposit: Coin(2_000_000), cert_index: 0 };
        let b4 = apply_conway_cert(&cs, &cert, &cenv).unwrap();
        assert_eq!(b4.state.delegation.delegations.get(&cred(1)), Some(&pool(9)), "B4 applied pool half");

        // B5 half: gov.vote_delegations set; CertState is structurally inaccessible.
        let g = empty_gov();
        let b5 = apply_conway_gov_cert(&g, &cert, Some(&env())).unwrap();
        assert_eq!(b5.vote_delegations.get(&cred(1)), Some(&drep), "B5 applied drep half exactly once");
        // B5 output is a ConwayGovState only — no delegation/pool fields exist to double-apply.
    }

    #[test]
    fn drep_expiry_uses_epoch_plus_activity() {
        let g = empty_gov();
        let e = GovCertEnv { current_epoch: 100, drep_activity: 7 };
        let out = apply_conway_gov_cert(
            &g,
            &ConwayCert::DRepRegistration { drep_credential: cred(0xAA), deposit: Coin(500_000_000) },
            Some(&e),
        ).unwrap();
        assert_eq!(out.drep_expiry.get(&cred(0xAA)), Some(&107));
    }

    /// Env-free certs (here: vote delegation) apply with `env = None`.
    #[test]
    fn env_free_gov_certs_need_no_env() {
        let g = empty_gov();
        let drep = DRep::AlwaysAbstain;
        let out = apply_conway_gov_cert(
            &g,
            &ConwayCert::VoteDelegation { credential: cred(1), drep: drep.clone() },
            None,
        ).unwrap();
        assert_eq!(out.vote_delegations.get(&cred(1)), Some(&drep));

        // committee + DRep-unregistration are also env-free.
        let out = apply_conway_gov_cert(
            &g,
            &ConwayCert::AuthCommitteeHot { cold_credential: cred(0xC0), hot_credential: cred(0x40) },
            None,
        ).unwrap();
        assert_eq!(out.committee_hot_keys.get(&cred(0x40)), Some(&cred(0xC0)));
    }

    /// DRep register/update with env absent is a structured fail-fast — never a
    /// defaulted expiry.
    #[test]
    fn drep_register_missing_env_is_fail_fast() {
        let g = empty_gov();
        for cert in [
            ConwayCert::DRepRegistration { drep_credential: cred(0xAA), deposit: Coin(500_000_000) },
            ConwayCert::DRepUpdate { drep_credential: cred(0xAA) },
        ] {
            let err = apply_conway_gov_cert(&g, &cert, None).unwrap_err();
            assert!(matches!(
                err,
                LedgerError::ValidationEnvironment(
                    ValidationEnvironmentError::MissingDRepActivityParam
                )
            ), "{cert:?}");
        }
    }

    /// A `drep_activity` large enough to overflow the DRep-expiry sum is a
    /// deterministic fail-closed halt — never a silent wrap to a wrong expiry.
    #[test]
    fn drep_expiry_overflow_is_fail_closed() {
        let g = empty_gov();
        let e = GovCertEnv { current_epoch: 10, drep_activity: u64::MAX };
        for cert in [
            ConwayCert::DRepRegistration { drep_credential: cred(0xAA), deposit: Coin(500_000_000) },
            ConwayCert::DRepUpdate { drep_credential: cred(0xAA) },
        ] {
            let err = apply_conway_gov_cert(&g, &cert, Some(&e)).unwrap_err();
            assert!(matches!(
                err,
                LedgerError::ValidationEnvironment(
                    ValidationEnvironmentError::DRepActivityOverflow
                )
            ), "{cert:?}");
        }
    }

    #[test]
    fn gov_apply_is_deterministic() {
        let g = empty_gov();
        let cert = ConwayCert::VoteDelegation { credential: cred(1), drep: DRep::KeyHash(h(0xDD)) };
        let r1 = apply_conway_gov_cert(&g, &cert, Some(&env())).unwrap();
        let r2 = apply_conway_gov_cert(&g, &cert, Some(&env())).unwrap();
        assert_eq!(r1, r2);
    }
}
