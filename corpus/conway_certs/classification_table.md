# Conway certificate classification table (§3.0 entry-proof)

This is the **proven** Conway certificate tag → deposit-effect mapping for
PHASE4-B3 slice B3-S2. It is the authoritative source for the
`conway_cert_classification` totality test, which reads the machine-checkable
companion fixture `tags.json` (never a hand-coded copy in test code).

## Source

Conway CDDL `certificate` grammar:
`eras/conway/impl/cddl/data/conway.cddl` (IntersectMBO/cardano-ledger). The
certificate sum type defines tags `0..18`. Tags `5`/`6` (Shelley
genesis-key-delegation / move-instantaneous-rewards) are **structurally absent**
from the Conway `certificate` rule — they are removed in Conway. Tags `>= 19`
are not defined by the grammar.

## Disposition taxonomy

- `NewDeposit` / `Refund` — `CertDisposition::Accountable(DepositEffect::…)`.
- `Neutral` — no tx-time conservation effect.
- `NotValidInConway` — known-but-removed tag (era-validity reject; **not** an
  accounting effect).

## Coin source

- `ExplicitInCert` — the deposit/refund coin is encoded in the certificate.
- `DepositParam` — legacy-implicit; sourced from canonical `ConwayDepositParams`.
- `RegistrationState` — refund resolved from the deposit recorded in ledger
  registration state (state-dependent).
- `none` — `Neutral` / `NotValidInConway` (no coin).

## Table

| tag | constructor | fields | disposition | coin source | state-dependent |
|-----|-------------|--------|-------------|-------------|-----------------|
| 0 | account_registration_cert | stake_credential | NewDeposit | DepositParam(key_deposit) | no |
| 1 | account_unregistration_cert | stake_credential | Refund | RegistrationState(recorded deposit) | yes |
| 2 | delegation_to_stake_pool_cert | stake_credential, pool_keyhash | Neutral | none | no |
| 3 | pool_registration_cert | pool_params | NewDeposit (pool new) else Neutral | DepositParam(pool_deposit) | yes |
| 4 | pool_retirement_cert | pool_keyhash, epoch | Neutral (refund at POOLREAP) | none | no |
| 5 | (removed in Conway) | — | NotValidInConway | none | no |
| 6 | (removed in Conway) | — | NotValidInConway | none | no |
| 7 | account_registration_deposit_cert | stake_credential, coin | NewDeposit | ExplicitInCert(coin) | no |
| 8 | account_unregistration_deposit_cert | stake_credential, coin | Refund | ExplicitInCert(coin) | no |
| 9 | delegation_to_drep_cert | stake_credential, drep | Neutral | none | no |
| 10 | delegation_to_stake_pool_and_drep_cert | stake_credential, pool_keyhash, drep | Neutral | none | no |
| 11 | account_registration_delegation_to_stake_pool_cert | stake_credential, pool_keyhash, coin | NewDeposit | ExplicitInCert(coin) | no |
| 12 | account_registration_delegation_to_drep_cert | stake_credential, drep, coin | NewDeposit | ExplicitInCert(coin) | no |
| 13 | account_registration_delegation_to_stake_pool_and_drep_cert | stake_credential, pool_keyhash, drep, coin | NewDeposit | ExplicitInCert(coin) | no |
| 14 | committee_authorization_cert | committee_cold_credential, committee_hot_credential | Neutral | none | no |
| 15 | committee_resignation_cert | committee_cold_credential, anchor/nil | Neutral | none | no |
| 16 | drep_registration_cert | drep_credential, coin, anchor/nil | NewDeposit | ExplicitInCert(coin) | no |
| 17 | drep_unregistration_cert | drep_credential, coin | Refund | ExplicitInCert(coin) | no |
| 18 | drep_update_cert | drep_credential, anchor/nil | Neutral | none | no |

Tags `>= 19`: unknown → `CodecError::UnknownCertTag { tag, offset }`.

## State-dependence boundary (D1)

The only state-dependent cases are tag 1 (legacy unregistration refund =
deposit recorded at registration) and tag 3 (pool registration new-vs-update).
`ade_ledger::delegation` tracks both:

- tag 1: `DelegationState.registrations: BTreeMap<StakeCredential, Coin>` records
  the per-credential deposit; absence of the credential →
  `UnsupportedStateDependentDepositAccounting::LegacyUnregistrationRefundUnresolved`
  (never the `key_deposit` param, which may have drifted since registration).
- tag 3: `PoolState.pools: BTreeMap<PoolId, PoolParams>` records registered
  pools; present → `Neutral` (re-registration update), absent → new
  `pool_deposit`.
